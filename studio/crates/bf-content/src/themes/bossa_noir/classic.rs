//! The bossa noir's classic arrangement: the `Bed` class of `projects/ballfight/kit/moods/bossa_noir.py`, section by
//! section as `kit.music.score()` calls them, each sound placed in the kit's order (sounds add up on a bus in that
//! order, buses come into being in it, and the cache below fills in it).
//!
//! **The kit's cache.** The kit's bed keeps every instrument's sound in a dict (`self._c`) under a key made when it
//! is first asked for, and hands that sound back for every later call with the same key. The keys are loose on
//! purpose: a duration is rounded to 3 decimals, and the noise of a brush, a rim click and a shaker is keyed by its
//! seed modulo 8, 4 and 4. So `brush(seed=9)` sounds like whichever of seeds 1, 9, 17, ... was asked for first, and
//! the cache is part of the music. [`BossaNoirClassic`] keeps the same cache under the same keys, filled in the same
//! order (one bed per build, as the kit makes one bed per soundtrack).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::{py_max, round_nd};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, THUNDER_CRACK, noise_seed, riser, thunder};
use bf_sound::synth::{self, FILTER_O, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED, clip, noise};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{CLAVE, COMP_ROLL, PLUCK_RING, choke, fifth, kick_felt, pluck, root, voicing, voicing_from};

/// The name the bed's errors carry.
const H: &str = "bossa_noir";

/// The nylon strings' bus: the kit mood's own bus id, kept as the kit spells it (the master adds the buses' reverb
/// sends in the bed's order and the references name the buses by these ids). An internal id: no screen shows it.
const NYLON: &str = "guitar";

/// A key of the kit's `self._c`: the method's tag and its key fields, a duration as `round(dur, 3)` (by its bits;
/// a rounded zero is +0.0, as Python's `-0.0 == 0.0`), other floats as they are.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// `('pl', int(m), round(dur, 3), seed, bright, ring)`
    Pluck(i32, u64, i64, u64, u64),
    /// `('cc', ch, ghost, round(dur, 3), roll)`
    Comp(String, bool, u64, u64),
    /// `('th', int(m), round(dur, 3))`
    Thumb(i32, u64),
    /// `('rr', ch, round(dur, 3))`
    RollRing(String, u64),
    /// `('ub', int(m), round(dur, 3))`
    Upright(i32, u64),
    /// `('rh', ch, round(dur, 3), up)`
    Rhodes(String, u64, i32),
    /// `('k',)`
    Kick,
    /// `('br', seed % 8, long)`
    Brush(u64, bool),
    /// `('rim', seed % 4)`
    Rim(u64),
    /// `('sh', seed % 4, accent)`
    Shaker(u64, bool),
}

/// `round(x, 3)` as a key.
fn r3(x: f64) -> u64 {
    let r = round_nd(x, 3);
    if r == 0.0 { 0.0f64.to_bits() } else { r.to_bits() }
}

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

fn scaled(x: &[f64], v: f64) -> Vec<f64> {
    x.iter().map(|a| a * v).collect()
}

/// `x[a:a + len(s)] += s`, which numpy refuses when `s` does not fit.
fn add_at(x: &mut [f64], a: usize, s: impl ExactSizeIterator<Item = f64>) {
    let n = s.len();
    assert!(
        a + n <= x.len(),
        "{H}: a sound of {n} samples at {a} does not fit {} samples",
        x.len()
    );
    for (o, v) in x[a..a + n].iter_mut().zip(s) {
        *o += v;
    }
}

fn put(mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, gain: f64, pan: f64) {
    mix.place(bus, &Sig::Mono(x), t, gain, pan);
}

/// `T.CHORDS[b]`.
fn chord<'a>(ctx: &'a MusicCtx, b: usize) -> Result<&'a str, MusicError> {
    let chords = &ctx.harmony.chords;
    chords.get(b).map(String::as_str).ok_or_else(|| {
        MusicError::new(
            H,
            format!("bar {b} is past the episode's last bar ({} bars)", chords.len()),
        )
    })
}

/// The bossa noir's bed as the kit plays it, with the kit's cache of sounds.
#[derive(Debug, Default)]
pub struct BossaNoirClassic {
    cache: Mutex<BTreeMap<Key, Arc<Vec<f64>>>>,
}

impl BossaNoirClassic {
    /// The bed, its cache empty.
    pub fn new() -> BossaNoirClassic {
        BossaNoirClassic::default()
    }

    /// `self._cache(key, fn)`: the sound under `key`, made by `make` the first time.
    fn cached(
        &self,
        key: Key,
        make: impl FnOnce() -> Result<Vec<f64>, MusicError>,
    ) -> Result<Arc<Vec<f64>>, MusicError> {
        if let Some(x) = self.cache.lock().expect("the cache").get(&key) {
            return Ok(Arc::clone(x));
        }
        let x = Arc::new(make()?);
        Ok(Arc::clone(
            self.cache.lock().expect("the cache").entry(key).or_insert(x),
        ))
    }

    /// `self.pluck(m, dur, seed, bright, ring)`.
    fn pluck(&self, m: i32, dur: f64, seed: i64, bright: f64, ring: f64) -> Result<Arc<Vec<f64>>, MusicError> {
        self.cached(Key::Pluck(m, r3(dur), seed, bright.to_bits(), ring.to_bits()), || {
            Ok(pluck(m, dur, seed, bright, ring))
        })
    }

    /// `self.comp_chord(ch, v, dur, ghost, roll)`: the nylon strings' bossa stroke: the fingers pluck the voicing
    /// together (a hair rolled), damped after `dur`; a ghost stroke is short and soft.
    fn comp_chord(
        &self,
        ctx: &MusicCtx,
        ch: &str,
        v: f64,
        dur: f64,
        ghost: bool,
        roll: f64,
    ) -> Result<Vec<f64>, MusicError> {
        let key = Key::Comp(ch.to_string(), ghost, r3(dur), roll.to_bits());
        let x = self.cached(key, || {
            let vo = voicing(&ctx.harmony, ch).map_err(bad)?;
            let d = length(roll * SRF);
            let ln = dur + 0.08;
            let n = length(ln * SRF) + d * vo.len();
            let mut x = vec![0.0; n];
            for (j, &m) in vo.iter().enumerate() {
                let s = self.pluck(m, ln, j as i64, if ghost { 0.22 } else { 0.34 }, PLUCK_RING)?;
                let g = 0.85 + 0.06 * j as f64;
                add_at(&mut x, j * d, s.iter().map(|v| v * g));
            }
            let t = synth::tt(n);
            let cut = if ghost { 0.07 } else { dur };
            let k = vo.len() as f64;
            let xs: Vec<f64> = x.iter().map(|v| v / k).collect();
            let y = synth::lp(&synth::lp(&xs, 2200.0, FILTER_O), 2600.0, FILTER_O);
            let last = if ghost { 0.42 } else { 1.0 };
            Ok(y.iter()
                .zip(&t)
                .map(|(y, &t)| {
                    let env = if t < cut {
                        1.0
                    } else {
                        bf_core::dmath::exp(-(t - cut) / 0.03)
                    };
                    y * env * last
                })
                .collect())
        })?;
        Ok(scaled(&x, v))
    }

    /// `self.thumb(m, dur, v)`: the thumb on the nylon's bass string (soft, round).
    fn thumb(&self, m: i32, dur: f64, v: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Thumb(m, r3(dur)), || {
            Ok(synth::lp(&self.pluck(m, dur, 9, 0.18, 0.995)?, 1400.0, FILTER_O))
        })?;
        Ok(scaled(&x, v))
    }

    /// `self.roll_ring(ch, dur, v)`: a slow rolled chord left ringing (the band hits).
    fn roll_ring(&self, ctx: &MusicCtx, ch: &str, dur: f64, v: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::RollRing(ch.to_string(), r3(dur)), || {
            let mut vo = voicing(&ctx.harmony, ch).map_err(bad)?;
            let top = *vo.last().expect("a voicing has four notes");
            vo.push(top + 5);
            let d = length(0.022 * SRF);
            let n = length((dur + 0.15) * SRF) + d * vo.len();
            let mut x = vec![0.0; n];
            for (j, &m) in vo.iter().enumerate() {
                let s = self.pluck(m, dur + 0.15, 10 + j as i64, 0.45, 0.998)?;
                add_at(&mut x, j * d, s.iter().copied());
            }
            let k = vo.len() as f64;
            let xs: Vec<f64> = x.iter().map(|v| v / k).collect();
            let y = synth::lp(&xs, 4000.0, FILTER_O);
            let dec = py_max(0.35, dur * 0.7);
            let t = synth::tt(n);
            Ok(y.iter()
                .zip(&t)
                .map(|(y, &t)| y * bf_core::dmath::exp(-t / dec))
                .collect())
        })?;
        Ok(scaled(&x, v))
    }

    /// `self.tremolo(ch, dur, v)` (never cached): the nylon strings' tremolo, soft strokes on the 16ths.
    fn tremolo(&self, ctx: &MusicCtx, ch: &str, dur: f64, v: f64) -> Result<Vec<f64>, MusicError> {
        let n = length((dur + 0.3) * SRF);
        let mut out = vec![0.0; n];
        let step = ctx.show.grid.s16;
        let mut j = 0usize;
        while (j as f64) * step < dur - 1e-6 {
            let vel = 0.55 + 0.2 * if j.is_multiple_of(2) { 1.0 } else { 0.0 };
            let s = self.comp_chord(ctx, ch, vel, 0.12, false, COMP_ROLL)?;
            let a = length(j as f64 * step * SRF);
            assert!(a <= n, "{H}: a tremolo stroke at {a} past {n} samples");
            let b = n.min(a + s.len());
            add_at(&mut out, a, s[..b - a].iter().copied());
            j += 1;
        }
        Ok(scaled(&out, v))
    }

    /// `self.upright(m, dur, v)`: a soft upright-style bass: a plucked low string on a sine body, the finger's
    /// thump, dark.
    fn upright(&self, m: i32, dur: f64, v: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Upright(m, r3(dur)), || {
            let n = length((dur + 0.10) * SRF);
            let t = synth::tt(n);
            let f = synth::mtof(f64::from(m));
            let st = self.pluck(m, dur + 0.10, 3, 0.10, 0.993)?;
            assert!(st.len() >= n, "{H}: a cached string of {} samples under {n}", st.len());
            let thump = synth::lp(&noise(u64::try_from(m).expect("a noise seed"), n), 300.0, FILTER_O);
            use bf_core::dmath::{PI, exp, sin};
            let mixed: Vec<f64> = (0..n)
                .map(|i| {
                    let t = t[i];
                    let body = (sin(2.0 * PI * f * t) + 0.22 * sin(4.0 * PI * f * t) + 0.05 * sin(6.0 * PI * f * t))
                        * exp(-t / 0.55);
                    let thump = thump[i] * exp(-t / 0.015) * 0.6;
                    body * 0.9 + st[i] * 0.25 + thump
                })
                .collect();
            let y = synth::lp(&synth::lp(&mixed, 520.0, FILTER_O), 520.0, FILTER_O);
            Ok(y.iter()
                .zip(&t)
                .map(|(y, &t)| {
                    let rel = if t < dur { 1.0 } else { exp(-(t - dur) / 0.05) };
                    y * (clip(t / 0.006, 0.0, 1.0) * rel)
                })
                .collect())
        })?;
        Ok(scaled(&x, v))
    }

    /// `self.rhodes(ch, dur, v, up)`: a dark Rhodes pad: FM tines (the index falling from bright to round), a slow
    /// tremolo, low-passed.
    fn rhodes(&self, ctx: &MusicCtx, ch: &str, dur: f64, v: f64, up: i32) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Rhodes(ch.to_string(), r3(dur), up), || {
            use bf_core::dmath::{PI, exp, sin};
            let vo: Vec<i32> = voicing_from(&ctx.harmony, ch, 55)
                .map_err(bad)?
                .into_iter()
                .map(|m| m + 12 * up)
                .collect();
            let n = length((dur + 0.3) * SRF);
            let t = synth::tt(n);
            let mut x = vec![0.0; n];
            for (j, &m) in vo.iter().enumerate() {
                let f = synth::mtof(f64::from(m));
                let w = 1.0 - 0.08 * j as f64;
                for (o, &t) in x.iter_mut().zip(&t) {
                    let idx = 0.4 + 1.6 * exp(-t / 0.25);
                    let tine = sin(2.0 * PI * f * t + idx * sin(2.0 * PI * f * t + j as f64));
                    let bell = sin(2.0 * PI * f * 7.0 * t) * exp(-t / 0.03) * 0.15;
                    *o += (tine + bell) * w;
                }
            }
            let k = vo.len() as f64;
            let xs: Vec<f64> = x.iter().map(|v| v / k).collect();
            let y = synth::lp(&xs, if up == 0 { 2200.0 } else { 3000.0 }, FILTER_O);
            Ok(y.iter()
                .zip(&t)
                .map(|(y, &t)| {
                    let trem = 1.0 - 0.22 * (0.5 + 0.5 * sin(2.0 * PI * 4.6 * t));
                    let rel = if t < dur { 1.0 } else { exp(-(t - dur) / 0.08) };
                    let env = clip(t / 0.008, 0.0, 1.0) * exp(-t / (dur * 1.4)) * rel;
                    y * trem * env
                })
                .collect())
        })?;
        Ok(scaled(&x, v))
    }

    /// `self.kick(v)`: a felt bass drum.
    fn kick(&self, v: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Kick, || Ok(kick_felt()))?;
        Ok(scaled(&x, v))
    }

    /// `self.brush(v, seed, long)`.
    fn brush(&self, v: f64, seed: u64, long: bool) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Brush(seed % 8, long), || Ok(super::brush(seed, long)))?;
        Ok(scaled(&x, v))
    }

    /// `self.rim(v, seed)`.
    fn rim(&self, v: f64, seed: u64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Rim(seed % 4), || Ok(super::rim(seed)))?;
        Ok(scaled(&x, v))
    }

    /// `self.shaker(v, seed, accent)`.
    fn shaker(&self, v: f64, seed: u64, accent: bool) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Shaker(seed % 4, accent), || Ok(super::shaker(seed, accent)))?;
        Ok(scaled(&x, v))
    }

    fn root(&self, ctx: &MusicCtx, ch: &str) -> Result<i32, MusicError> {
        root(&ctx.harmony, ch).map_err(bad)
    }

    /// `Bed.bar(mix, b, intensity, light)`: one bar of the bossa (bar index `b`): the bass's dotted quarter and
    /// eighth, the nylon strings on the clave with the thumb on 1 and 3, the Rhodes holding the chord, brushes on the
    /// 8ths, the kick and rim clicks; the back half (`intensity` above 1.25) adds the shaker, ghost strokes, the
    /// Rhodes' upper voicing and bass pickups. Python's defaults are `intensity = 1.0` and `light = False`.
    ///
    /// # Errors
    /// Where Python raises: a chord the harmony does not hold.
    pub fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, light: bool) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        let t0 = b as f64 * bar;
        let nb = grid.n_bars;
        let ch = chord(ctx, b % nb)?;
        let nxt = chord(ctx, (b + 1) % nb)?;
        let big = intensity > 1.25 && !light;
        let r = self.root(ctx, ch)?;
        let f5 = fifth(&ctx.harmony, ch).map_err(bad)?;
        let e8 = beat / 2.0;
        put(mix, "bass", self.upright(r, 1.4 * beat, 1.0)?, t0, 0.62, 0.0);
        put(
            mix,
            "bass",
            self.upright(f5, 0.45 * beat, 0.8)?,
            t0 + 3.0 * e8,
            0.46,
            0.0,
        );
        put(
            mix,
            "bass",
            self.upright(f5, 1.4 * beat, 0.9)?,
            t0 + 2.0 * beat,
            0.54,
            0.0,
        );
        let nr = self.root(ctx, nxt)?;
        let pk = if big { if nxt != ch { nr - 1 } else { r } } else { r };
        let pk_gain = if big { 0.42 } else { 0.36 };
        put(
            mix,
            "bass",
            self.upright(pk, 0.45 * beat, 0.8)?,
            t0 + 7.0 * e8,
            pk_gain,
            0.0,
        );
        put(mix, NYLON, self.thumb(r + 12, 0.9 * beat, 1.0)?, t0, 0.36, 0.2);
        put(
            mix,
            NYLON,
            self.thumb(f5 + 12, 0.9 * beat, 1.0)?,
            t0 + 2.0 * beat,
            0.32,
            0.2,
        );
        let hits = CLAVE[b % 2];
        for &j in hits {
            let s = self.comp_chord(ctx, ch, 1.0, 0.30, false, COMP_ROLL)?;
            put(mix, NYLON, s, t0 + j as f64 * e8, 0.50, 0.2);
        }
        if big {
            for j in 0..8usize {
                if !hits.contains(&j) && j % 2 == 1 {
                    let s = self.comp_chord(ctx, ch, 1.0, super::COMP_DUR, true, COMP_ROLL)?;
                    put(mix, NYLON, s, t0 + j as f64 * e8, 0.34, 0.2);
                }
            }
        }
        let keys_gain = if light { 0.24 } else { 0.30 };
        put(
            mix,
            "keys",
            self.rhodes(ctx, ch, bar * 0.96, 1.0, 0)?,
            t0,
            keys_gain,
            -0.2,
        );
        if big {
            put(
                mix,
                "keys",
                self.rhodes(ctx, ch, bar * 0.9, 1.0, 1)?,
                t0 + 0.01,
                0.12,
                -0.28,
            );
        }
        for j in 0..8usize {
            let tj = t0 + j as f64 * e8;
            let seed = (b + j) as u64;
            if j % 2 == 0 {
                put(mix, "drums", self.brush(1.0, seed, true)?, tj, 0.065, -0.15);
            } else {
                let g = if j == 3 || j == 7 { 0.10 } else { 0.075 };
                put(mix, "drums", self.brush(1.0, seed, false)?, tj, g, -0.15);
            }
            if big {
                put(mix, "drums", self.shaker(1.0, j as u64, j % 2 == 1)?, tj, 0.045, 0.35);
            }
        }
        let kick_gain = if light { 0.34 } else { 0.42 };
        for j in [0usize, 3, 4, 7] {
            let v = if j == 0 || j == 4 { 1.0 } else { 0.7 };
            put(mix, "drums", self.kick(v)?, t0 + j as f64 * e8, kick_gain, 0.0);
        }
        for &j in CLAVE[b % 2] {
            put(mix, "drums", self.rim(1.0, j as u64)?, t0 + j as f64 * e8, 0.11, 0.12);
        }
        Ok(())
    }
}

impl Bed for BossaNoirClassic {
    /// The mood's class defines `boom` and `hit`, none of the other optional hooks.
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom, BedHook::Hit]
    }

    /// `buses = {'drums': (1.70, 0.14), 'bass': (1.9, 0.10), NYLON: (1.35, 0.26), 'keys': (0.95, 0.34)}`.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &[
            ("drums", (1.70, 0.14)),
            ("bass", (1.9, 0.10)),
            (NYLON, (1.35, 0.26)),
            ("keys", (0.95, 0.34)),
        ]
    }

    /// A small smoky club, a little velvet.
    fn reverb(&self) -> (f64, f64, f64) {
        (1.7, 0.5, 0.18)
    }

    fn comp(&self) -> (f64, f64) {
        (0.16, 0.55)
    }

    fn roar_gain(&self) -> f64 {
        0.16
    }

    fn doors_gain(&self) -> f64 {
        0.40
    }

    fn landing(&self) -> (f64, f64) {
        (0.07, 0.05)
    }

    fn pop(&self) -> (f64, f64, f64) {
        (93.0, 98.0, 0.10)
    }

    /// A big drum hit (a slam landing): the kick and the bass's root of the chord at `t`.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        put(mix, "drums", self.kick(1.0)?, t, g, 0.0);
        let r = self.root(ctx, ctx.harmony.chord_at(t))?;
        put(mix, "bass", self.upright(r, 0.5, 1.0)?, t, 0.45 * g, 0.0);
        Ok(())
    }

    /// A full band hit: the bass root, a rolled nylon chord ringing, the Rhodes, the kick and a choked crash.
    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, ring: f64) -> Played {
        let r = self.root(ctx, ch)?;
        put(mix, "bass", self.upright(r, ring, 1.0)?, t, 0.66 * g, 0.0);
        put(mix, NYLON, self.roll_ring(ctx, ch, ring, 1.0)?, t, 0.62 * g, 0.2);
        put(mix, "keys", self.rhodes(ctx, ch, ring, 1.0, 0)?, t, 0.34 * g, -0.2);
        put(mix, "drums", self.kick(1.0)?, t, 0.60 * g, 0.0);
        put(mix, "drums", choke(1.0, noise_seed(H, t * 10.0)?), t, 0.14 * g, 0.0);
        Ok(())
    }

    /// `mix.place('fx', self.rim(1.0, seed=3), t, 0.05)`.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", self.rim(1.0, 3)?, t, 0.05, 0.0);
        Ok(())
    }

    /// A full band hit under each word of the VS card.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let ch = chord(ctx, 0)?;
        for tw in times {
            self.hit(ctx, mix, tw, ch, 1.0, 0.45)?;
            put(mix, "drums", synth::crash(1.0), tw, 0.07, 0.0);
            let seed = noise_seed(H, tw * 50.0)? + 3;
            put(mix, "fx", synth::thud(0.8, 120.0, 34.0, seed), tw, 0.30, 0.0);
        }
        Ok(())
    }

    /// VS: a thunder crack where it splits the divider.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", thunder(1.4, 91, THUNDER_CRACK)?, t, 0.40, 0.0);
        Ok(())
    }

    /// The held chord under the card (never quiet): the nylon tremolo, the Rhodes, a long bass root, brushes
    /// swelling, rim clicks rolling on beat 4 as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let ch = chord(ctx, 0)?;
        put(
            mix,
            NYLON,
            self.tremolo(ctx, ch, 3.0 * beat - 0.1, 1.0)?,
            0.08,
            0.42,
            0.2,
        );
        put(mix, "keys", self.rhodes(ctx, ch, 3.0 * beat, 1.0, 0)?, 0.05, 0.30, -0.2);
        let r = self.root(ctx, ch)?;
        put(mix, "bass", self.upright(r, 2.9 * beat, 0.8)?, 0.05, 0.46, 0.0);
        let n = length(3.0 * beat * SRF);
        let swell = dsp::linspace(0.2, 1.0, n);
        let sw = synth::bp(&noise(5, n), 1800.0, 8000.0, FILTER_O)
            .iter()
            .zip(&swell)
            .map(|(x, l)| x * (l * l))
            .collect();
        put(mix, "drums", sw, 0.0, 0.05, 0.0);
        for k in 0..4u64 {
            let rim = self.rim(0.6 + 0.13 * k as f64, k)?;
            put(mix, "drums", rim, 3.0 * beat + k as f64 * s16, 0.13, 0.0);
        }
        Ok(())
    }

    /// Bar 1: the band keeps the bossa on the count-in chord, a band stab on each count, rim clicks and a bass
    /// walk-up into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        self.bar(ctx, mix, 1, 1.4, false)?;
        let ch = chord(ctx, 1)?;
        for (tc, _) in &grid.count[..3] {
            self.hit(ctx, mix, *tc, ch, 0.5, 0.28)?;
        }
        let t4 = bar + 3.0 * beat;
        for k in 0..4u64 {
            let rim = self.rim(0.6 + 0.13 * k as f64, k)?;
            put(mix, "drums", rim, t4 + k as f64 * s16, 0.14, 0.0);
        }
        let walk = self.root(ctx, chord(ctx, 2)?)? - 1;
        put(
            mix,
            "bass",
            self.upright(walk, 0.4 * beat, 0.85)?,
            t4 + 2.0 * s16,
            0.44,
            0.0,
        );
        Ok(())
    }

    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_fight = ctx.show.grid.t_fight;
        put(mix, "drums", synth::crash(1.8), t_fight, 0.11, 0.0);
        put(mix, "fx", synth::thud(0.7, 110.0, 34.0, THUD_SEED), t_fight, 0.34, 0.0);
        Ok(())
    }

    /// The bossa from FIGHT! to the K.O. (denser as it goes), a riser and a rim roll into the back half, soft
    /// crashes on the phrase lines, a tape stop at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, 1.0 + 0.6 * (b as f64 / bar_ko as f64), false)?;
        }
        for b in (2..=bar_ko).step_by(8) {
            put(mix, "drums", synth::crash(1.6), b as f64 * bar, 0.07, 0.0);
        }
        let t_half = (ctx.harmony.loop_start + 2 * ctx.harmony.loop_.len()) as f64 * bar;
        for k in 0..8u64 {
            let rim = self.rim(0.5 + 0.06 * k as f64, k)?;
            put(
                mix,
                "drums",
                rim,
                t_half - beat * 2.0 + k as f64 * beat / 4.0,
                0.12,
                0.0,
            );
        }
        let rise = riser(bar, 0.0, 12)?.iter().map(|v| v * 0.6).collect();
        put(mix, "fx", rise, t_half - bar, 0.08, 0.0);
        put(mix, "drums", synth::crash(2.0), t_half, 0.10, 0.0);
        for bus in ["drums", "bass", NYLON, "keys"] {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, ctx.show.t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        put(mix, "drums", synth::crash(2.4), t_ko, 0.15, 0.0);
        put(mix, "fx", synth::thud(0.9, 120.0, 30.0, THUD_SEED), t_ko, 0.6, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        self.hit(ctx, mix, tw, chord(ctx, 0)?, 1.0, 0.8)?;
        put(mix, "drums", synth::crash(1.8), tw, 0.09, 0.0);
        Ok(())
    }

    /// `A.whoosh(max(0.2, t_solo - tw - 0.1), 300, 5000)` from just after the stinger to the solo.
    fn after_ko(&self, _: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let dur = py_max(0.2, t_solo - tw - 0.1);
        put(
            mix,
            "fx",
            synth::whoosh(dur, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED),
            tw + 0.1,
            0.08,
            0.0,
        );
        Ok(())
    }

    /// The band plays on under the solo at full density, up to the build.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(1) {
            self.bar(ctx, mix, b, 1.4, false)?;
        }
        Ok(())
    }

    /// The last bar builds into the title slam on sample 0: rim clicks rolling up to the seam, the bass walking up
    /// chromatically into the tonic, the nylon strings on every 8th, the kick on every beat, the Rhodes, a riser.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, s16, dur) = (grid.beat, grid.bar, grid.s16, grid.dur);
        let t_last = (grid.n_bars as f64 - 1.0) * bar;
        let ch = chord(ctx, ctx.harmony.chords.len().saturating_sub(1))?;
        let tgt = self.root(ctx, chord(ctx, 0)?)?;
        put(mix, "keys", self.rhodes(ctx, ch, bar, 1.0, 0)?, t_last, 0.30, -0.2);
        put(mix, "keys", self.rhodes(ctx, ch, bar, 1.0, 1)?, t_last, 0.12, -0.28);
        for bt in 0..4i32 {
            let tb = t_last + f64::from(bt) * beat;
            put(mix, "bass", self.upright(tgt - 4 + bt, 0.6 * beat, 0.9)?, tb, 0.60, 0.0);
            put(mix, "drums", self.kick(0.95)?, tb, 0.46, 0.0);
            put(mix, "drums", self.brush(1.0, bt as u64, true)?, tb, 0.07, -0.15);
            for h in 0..2 {
                let s = self.comp_chord(ctx, ch, 1.0, 0.18, false, COMP_ROLL)?;
                put(mix, NYLON, s, tb + f64::from(h) * beat / 2.0, 0.46, 0.2);
            }
        }
        let n16 = length((dur - t_last) / s16 + 1e-6);
        let span = n16.saturating_sub(1).max(1) as f64;
        for k in 0..n16 {
            let rim = self.rim(0.45 + 0.55 * k as f64 / span, k as u64)?;
            put(mix, "drums", rim, t_last + k as f64 * s16, 0.13, 0.0);
        }
        put(
            mix,
            "fx",
            riser(dur - t_last, RISER_OVER, RISER_SEED)?,
            t_last,
            0.18,
            0.0,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cache_keys_are_the_kits() {
        // round(dur, 3): 0.1234 and 0.1226 share a key; 0.1225 rounds to 0.122 (its binary value is below the tie)
        assert_eq!(r3(0.1234), r3(0.1226));
        assert_ne!(r3(0.1225), r3(0.1226));
        assert_eq!(r3(-0.0001), r3(0.0));
        // a brush keyed by seed % 8 is the first seed's noise
        let bed = BossaNoirClassic::new();
        let first = bed.brush(1.0, 9, false).unwrap();
        assert_eq!(first, super::super::brush(9, false));
        assert_eq!(bed.brush(0.5, 1, false).unwrap(), scaled(&first, 0.5));
        assert_ne!(bed.brush(1.0, 1, true).unwrap(), super::super::brush(9, true));
    }
}
