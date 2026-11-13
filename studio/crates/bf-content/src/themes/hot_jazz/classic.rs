//! The hot jazz's classic arrangement: the `Bed` class of `projects/ballfight/kit/moods/hot_jazz.py`, section by
//! section as `kit.music.score()` calls them, each sound placed in the kit's order (sounds add up on a bus in that
//! order, buses come into being in it, and the cache below fills in it).
//!
//! **The kit's cache.** The kit's bed keeps every instrument's sound in a dict (`self._c`) under a key made when it
//! is first asked for, and hands that sound back for every later call with the same key. A duration in a key is
//! rounded to 3 decimals, and a press roll's crescendo is keyed by `round(v0 / v, 3)`, so a later call that differs
//! only past the rounding gets the first call's sound: the cache is part of the music. [`HotJazzClassic`] keeps the
//! same cache under the same keys, filled in the same order (one bed per build, as the kit makes one bed per
//! soundtrack).
//!
//! **The tuba** is the TUBA's Pixel Pitch voice through the music context's `note` (which has its own cache and the
//! voice's level), `M.note('TUBA', int(m), round(dur, 3), round(dur + 0.12, 2), round(vel, 1))`.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::dmath::exp;
use bf_core::harmony::HarmonyError;
use bf_core::py::{py_max, round_nd};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, THUNDER_CRACK, noise_seed, riser, thunder};
use bf_sound::synth::{self, FILTER_O, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED, clip, noise};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{CHOP_DUR, PLUCK_RING, SWING, choke, fifth, kick_felt, pluck, root, voicing};

/// The name the bed's errors carry.
const H: &str = "hot_jazz";

/// The TUBA's Pixel Pitch voice id, which the bed plays itself.
const TUBA: &str = "TUBA";

/// A key of the kit's `self._c`: the method's tag and its key fields, a duration as `round(dur, 3)` (by its bits;
/// a rounded zero is +0.0, as Python's `-0.0 == 0.0`), other floats as they are.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// `('pl', int(m), round(dur, 3), seed, bright, ring)`
    Pluck(i32, u64, i64, u64, u64),
    /// `('ch', ch, ghost, round(dur, 3))`
    Chop(String, bool, u64),
    /// `('sr', ch, round(dur, 3))`
    StrumRing(String, u64),
    /// `('k',)`
    Kick,
    /// `('cy', seed)`
    Cymbal(u64),
    /// `('br', seed)`
    Brush(u64),
    /// `('pr', round(dur, 3), seed, None or round(v0 / max(v, 1e-6), 3))`
    Press(u64, u64, Option<u64>),
    /// `('wb', hi)`
    Woodblock(bool),
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

/// The hot jazz's bed as the kit plays it, with the kit's cache of sounds.
#[derive(Debug, Default)]
pub struct HotJazzClassic {
    cache: Mutex<BTreeMap<Key, Arc<Vec<f64>>>>,
}

impl HotJazzClassic {
    /// The bed, its cache empty.
    pub fn new() -> HotJazzClassic {
        HotJazzClassic::default()
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

    /// `self.tuba(m, dur, vel)`: one note of the TUBA's voice, `M.note('TUBA', int(m), round(dur, 3),
    /// round(dur + 0.12, 2), round(vel, 1))`.
    fn tuba(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64) -> Result<Vec<f64>, MusicError> {
        let x = ctx.note(
            TUBA,
            f64::from(m),
            round_nd(dur, 3),
            round_nd(dur + 0.12, 2),
            round_nd(vel, 1),
        )?;
        Ok(x.to_vec())
    }

    /// `self.pluck(m, dur, seed, bright, ring)`.
    fn pluck(&self, m: i32, dur: f64, seed: i64, bright: f64, ring: f64) -> Result<Arc<Vec<f64>>, MusicError> {
        self.cached(Key::Pluck(m, r3(dur), seed, bright.to_bits(), ring.to_bits()), || {
            Ok(pluck(m, dur, seed, bright, ring))
        })
    }

    /// `self.chop(ch, v, ghost, dur)`: the rhythm guitar's chop: a fast down-strum of the voicing, choked by the
    /// fretting hand after about 0.12 s (a ghost stroke: shorter, darker, mostly the pick's scratch).
    fn chop(&self, ctx: &MusicCtx, ch: &str, v: f64, ghost: bool, dur: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Chop(ch.to_string(), ghost, r3(dur)), || {
            let vo = voicing(&ctx.harmony, ch).map_err(bad)?;
            let d = length(0.006 * SRF);
            let n = length((dur + 0.05) * SRF) + d * vo.len();
            let mut x = vec![0.0; n];
            for (j, &m) in vo.iter().enumerate() {
                let bright = if ghost { 0.35 } else { 0.55 };
                let s = self.pluck(m, dur + 0.05, j as i64, bright, PLUCK_RING)?;
                let g = 0.85 + 0.05 * j as f64;
                add_at(&mut x, j * d, s.iter().map(|v| v * g));
            }
            let t = synth::tt(n);
            let cut = if ghost { 0.06 } else { dur };
            let k = vo.len() as f64;
            let xs: Vec<f64> = x.iter().map(|v| v / k).collect();
            let low = synth::lp(&xs, 3800.0, FILTER_O);
            let scratch = synth::bp(&noise(77, n), 1800.0, 6500.0, FILTER_O);
            let sg = if ghost { 0.9 } else { 0.35 };
            let last = if ghost { 0.45 } else { 1.0 };
            let held = exp(-cut / 0.10);
            Ok((0..n)
                .map(|i| {
                    let t = t[i];
                    let fall = if t < cut {
                        exp(-t / 0.10)
                    } else {
                        held * exp(-(t - cut) / 0.018)
                    };
                    let env = clip(t / 0.002, 0.0, 1.0) * fall;
                    let scratch = scratch[i] * exp(-t / 0.012) * sg;
                    (low[i] * env + scratch) * last
                })
                .collect())
        })?;
        Ok(scaled(&x, v))
    }

    /// `self.strum_ring(ch, dur, v)`: a full strum left ringing (the band hits): the voicing and a fourth above its
    /// top note.
    fn strum_ring(&self, ctx: &MusicCtx, ch: &str, dur: f64, v: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::StrumRing(ch.to_string(), r3(dur)), || {
            let mut vo = voicing(&ctx.harmony, ch).map_err(bad)?;
            let top = *vo.last().expect("a voicing has four notes");
            vo.push(top + 5);
            let d = length(0.010 * SRF);
            let n = length((dur + 0.1) * SRF) + d * vo.len();
            let mut x = vec![0.0; n];
            for (j, &m) in vo.iter().enumerate() {
                let s = self.pluck(m, dur + 0.1, 10 + j as i64, 0.7, 0.997)?;
                add_at(&mut x, j * d, s.iter().copied());
            }
            let k = vo.len() as f64;
            let xs: Vec<f64> = x.iter().map(|v| v / k).collect();
            let y = synth::lp(&xs, 4500.0, FILTER_O);
            let dec = py_max(0.3, dur * 0.6);
            let t = synth::tt(n);
            Ok(y.iter()
                .zip(&t)
                .map(|(y, &t)| y * exp(-t / dec) * clip(t / 0.002, 0.0, 1.0))
                .collect())
        })?;
        Ok(scaled(&x, v))
    }

    /// `self.tremolo(ch, dur, v)` (never cached): the guitar's tremolo, down-up strums on the 32nds (the card's
    /// held chord).
    fn tremolo(&self, ctx: &MusicCtx, ch: &str, dur: f64, v: f64) -> Result<Vec<f64>, MusicError> {
        let n = length((dur + 0.2) * SRF);
        let mut out = vec![0.0; n];
        let step = ctx.show.grid.s16 / 2.0;
        let mut j = 0usize;
        while (j as f64) * step < dur - 1e-6 {
            let vel = 0.55 + 0.15 * if j.is_multiple_of(2) { 1.0 } else { 0.0 };
            let s = self.chop(ctx, ch, vel, false, 0.10)?;
            let a = length(j as f64 * step * SRF);
            assert!(a <= n, "{H}: a tremolo stroke at {a} past {n} samples");
            let b = n.min(a + s.len());
            add_at(&mut out, a, s[..b - a].iter().copied());
            j += 1;
        }
        Ok(scaled(&out, v))
    }

    /// `self.kick(v)`: a felt bass drum.
    fn kick(&self, v: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Kick, || Ok(kick_felt()))?;
        Ok(scaled(&x, v))
    }

    /// `self.cymbal(v, seed)`.
    fn cymbal(&self, v: f64, seed: u64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Cymbal(seed), || Ok(super::cymbal(seed)))?;
        Ok(scaled(&x, v))
    }

    /// `self.brush(v, seed)`.
    fn brush(&self, v: f64, seed: u64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Brush(seed), || Ok(super::brush(seed)))?;
        Ok(scaled(&x, v))
    }

    /// `self.press(dur, v, seed, v0)`.
    fn press(&self, dur: f64, v: f64, seed: u64, v0: Option<f64>) -> Result<Vec<f64>, MusicError> {
        let ratio = v0.map(|v0| r3(v0 / py_max(v, 1e-6)));
        let x = self.cached(Key::Press(r3(dur), seed, ratio), || Ok(super::press(dur, v, seed, v0)))?;
        Ok(scaled(&x, v))
    }

    /// `self.woodblock(v, hi)`.
    fn woodblock(&self, v: f64, hi: bool) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Woodblock(hi), || Ok(super::woodblock(hi)))?;
        Ok(scaled(&x, v))
    }

    fn root(&self, ctx: &MusicCtx, ch: &str) -> Result<i32, MusicError> {
        root(&ctx.harmony, ch).map_err(bad)
    }

    /// `Bed.bar(mix, b, intensity, light)`: one bar of the two-beat (bar index `b`): the tuba on 1 and 3, the
    /// guitar's chop on 2 and 4, brushed cymbals on every beat with the swung skip, the felt bass drum on 1 and 3,
    /// brush taps on 2 and 4; the back half (`intensity` above 1.25) adds snare press rolls on 2 and 4, ghost chops
    /// on 1 and 3, woodblock off-beats and tuba pickups. Python's defaults are `intensity = 1.0` and
    /// `light = False`.
    ///
    /// # Errors
    /// Where Python raises: a chord the harmony does not hold, a tuba note the voice cannot make.
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
        put(mix, "tuba", self.tuba(ctx, r, 0.72 * beat, 0.95)?, t0, 0.62, 0.0);
        put(
            mix,
            "tuba",
            self.tuba(ctx, f5, 0.72 * beat, 0.85)?,
            t0 + 2.0 * beat,
            0.56,
            0.0,
        );
        if big {
            let nr = self.root(ctx, nxt)?;
            let pk = if nxt != ch {
                nr - 1
            } else {
                f5 + if f5 + 2 <= 50 { 2 } else { -2 }
            };
            put(
                mix,
                "tuba",
                self.tuba(ctx, pk, 0.32 * beat, 0.8)?,
                t0 + (3.0 + SWING) * beat,
                0.42,
                0.0,
            );
        }
        for bt in 0..4usize {
            let tb = t0 + bt as f64 * beat;
            if bt == 0 || bt == 2 {
                let g = if light { 0.40 } else { 0.50 };
                put(mix, "drums", self.kick(1.0)?, tb, g, 0.0);
                if big {
                    put(mix, "guitar", self.chop(ctx, ch, 1.0, true, CHOP_DUR)?, tb, 0.30, 0.18);
                }
            } else {
                put(mix, "guitar", self.chop(ctx, ch, 1.0, false, CHOP_DUR)?, tb, 0.46, 0.18);
                if big {
                    let s = self.press(0.20, 1.0, (b * 4 + bt) as u64, None)?;
                    put(mix, "drums", s, tb, 0.13, -0.12);
                } else {
                    put(mix, "drums", self.brush(1.0, (b + bt) as u64)?, tb, 0.16, -0.12);
                }
            }
            let cv = if bt % 2 == 0 { 0.85 } else { 1.0 };
            put(mix, "drums", self.cymbal(cv, (bt % 2) as u64)?, tb, 0.055, 0.32);
            if bt == 1 || bt == 3 {
                put(mix, "drums", self.cymbal(0.6, 2)?, tb + SWING * beat, 0.040, 0.32);
            }
            if big && b % 2 == 1 && (bt == 1 || bt == 3) {
                put(
                    mix,
                    "drums",
                    self.woodblock(1.0, bt == 1)?,
                    tb + SWING * beat,
                    0.07,
                    -0.3,
                );
            }
        }
        let ls = ctx.harmony.loop_start;
        if b >= ls && (b - ls) % 4 == 3 && !light {
            let s = self.press(beat * 0.95, 1.0, 500 + b as u64, Some(0.35))?;
            put(mix, "drums", s, t0 + 3.0 * beat, 0.16, -0.12);
        }
        Ok(())
    }
}

impl Bed for HotJazzClassic {
    /// The mood's class defines `boom` and `hit`, none of the other optional hooks.
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom, BedHook::Hit]
    }

    /// `buses = {'drums': (1.45, 0.14), 'tuba': (1.75, 0.12), 'guitar': (1.6, 0.28)}`.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &[("drums", (1.45, 0.14)), ("tuba", (1.75, 0.12)), ("guitar", (1.6, 0.28))]
    }

    /// A low-ceilinged club.
    fn reverb(&self) -> (f64, f64, f64) {
        (1.4, 0.45, 0.16)
    }

    fn comp(&self) -> (f64, f64) {
        (0.18, 0.55)
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

    /// A big drum hit (a slam landing): the kick, and the tuba's root of the chord at `t` an octave down where it
    /// can go (not below A1).
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        put(mix, "drums", self.kick(1.0)?, t, g, 0.0);
        let r = self.root(ctx, ctx.harmony.chord_at(t))?;
        let m = if r - 12 >= 33 { r - 12 } else { r };
        put(mix, "tuba", self.tuba(ctx, m, 0.5, 1.0)?, t, 0.4 * g, 0.0);
        Ok(())
    }

    /// A full band hit: the tuba's root an octave down where it can go and at pitch, the guitar's strum ringing,
    /// the kick and a choked crash.
    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, ring: f64) -> Played {
        let r = self.root(ctx, ch)?;
        let low = if r - 12 >= 33 { r - 12 } else { r };
        put(mix, "tuba", self.tuba(ctx, low, ring, 1.0)?, t, 0.60 * g, 0.0);
        put(mix, "tuba", self.tuba(ctx, r, ring, 0.9)?, t, 0.30 * g, 0.0);
        put(mix, "guitar", self.strum_ring(ctx, ch, ring, 1.0)?, t, 0.55 * g, 0.18);
        put(mix, "drums", self.kick(1.0)?, t, 0.62 * g, 0.0);
        put(mix, "drums", choke(1.0, noise_seed(H, t * 10.0)?), t, 0.16 * g, 0.0);
        Ok(())
    }

    /// `mix.place('fx', self.woodblock(1.0, hi=False), t, 0.05)`.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", self.woodblock(1.0, false)?, t, 0.05, 0.0);
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

    /// The held chord under the card (never quiet): the guitar's tremolo, a long tuba root, the cymbal swelling, a
    /// press roll on beat 4 as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let beat = ctx.show.grid.beat;
        let ch = chord(ctx, 0)?;
        put(
            mix,
            "guitar",
            self.tremolo(ctx, ch, 3.0 * beat - 0.1, 1.0)?,
            0.08,
            0.36,
            0.18,
        );
        let r = self.root(ctx, ch)?;
        put(mix, "tuba", self.tuba(ctx, r, 2.9 * beat, 0.8)?, 0.05, 0.40, 0.0);
        let n = length(3.0 * beat * SRF);
        let swell = dsp::linspace(0.2, 1.0, n);
        let sw = synth::hp(&noise(5, n), 5000.0, FILTER_O)
            .iter()
            .zip(&swell)
            .map(|(x, l)| x * (l * l))
            .collect();
        put(mix, "drums", sw, 0.0, 0.035, 0.0);
        let s = self.press(beat * 0.95, 1.0, 7, Some(0.3))?;
        put(mix, "drums", s, 3.0 * beat, 0.20, -0.12);
        Ok(())
    }

    /// Bar 1: the band keeps the two-beat on the count-in chord (the groove from the card on), a band stab on each
    /// count, a press roll and a tuba walk-up into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        self.bar(ctx, mix, 1, 1.4, false)?;
        let ch = chord(ctx, 1)?;
        for (tc, _) in &grid.count[..3] {
            self.hit(ctx, mix, *tc, ch, 0.55, 0.30)?;
        }
        let t4 = bar + 3.0 * beat;
        let s = self.press(beat * 0.98, 1.0, 9, Some(0.4))?;
        put(mix, "drums", s, t4, 0.26, -0.12);
        let walk = self.root(ctx, chord(ctx, 2)?)? - 2;
        put(mix, "tuba", self.tuba(ctx, walk, 0.3 * beat, 0.85)?, t4, 0.40, 0.0);
        Ok(())
    }

    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_fight = ctx.show.grid.t_fight;
        put(mix, "drums", synth::crash(1.8), t_fight, 0.12, 0.0);
        put(mix, "fx", synth::thud(0.7, 110.0, 34.0, THUD_SEED), t_fight, 0.34, 0.0);
        Ok(())
    }

    /// The two-beat from FIGHT! to the K.O. (denser as it goes), a long press roll and a riser into the back half,
    /// crashes on the phrase lines, a tape stop at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, 1.0 + 0.6 * (b as f64 / bar_ko as f64), false)?;
        }
        for b in (2..=bar_ko).step_by(8) {
            put(mix, "drums", synth::crash(1.6), b as f64 * bar, 0.08, 0.0);
        }
        let t_half = (ctx.harmony.loop_start + 2 * ctx.harmony.loop_.len()) as f64 * bar;
        let s = self.press(2.0 * beat, 1.0, 33, Some(0.3))?;
        put(mix, "drums", s, t_half - 2.0 * beat, 0.18, -0.12);
        let rise = riser(bar, 0.0, 12)?.iter().map(|v| v * 0.6).collect();
        put(mix, "fx", rise, t_half - bar, 0.08, 0.0);
        put(mix, "drums", synth::crash(2.0), t_half, 0.12, 0.0);
        for bus in ["drums", "tuba", "guitar"] {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, ctx.show.t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        put(mix, "drums", synth::crash(2.4), t_ko, 0.16, 0.0);
        put(mix, "fx", synth::thud(0.9, 120.0, 30.0, THUD_SEED), t_ko, 0.6, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        self.hit(ctx, mix, tw, chord(ctx, 0)?, 1.0, 0.8)?;
        put(mix, "drums", synth::crash(1.8), tw, 0.10, 0.0);
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

    /// The band plays on under the solo at full tilt (a victory, never a quiet stretch), up to the build.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(1) {
            self.bar(ctx, mix, b, 1.4, false)?;
        }
        Ok(())
    }

    /// The last bar builds into the title slam on sample 0: a press roll crescendo right up to the seam, the tuba
    /// walking up chromatically into the tonic, the guitar chopping every beat, the bass drum on every beat, a riser.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, dur) = (grid.beat, grid.bar, grid.dur);
        let t_last = (grid.n_bars as f64 - 1.0) * bar;
        let ch = chord(ctx, ctx.harmony.chords.len().saturating_sub(1))?;
        let s = self.press(dur - t_last + 0.012, 1.0, 99, Some(0.35))?;
        put(mix, "drums", s, t_last, 0.22, -0.12);
        let tgt = self.root(ctx, chord(ctx, 0)?)?;
        for bt in 0..4i32 {
            let tb = t_last + f64::from(bt) * beat;
            put(
                mix,
                "tuba",
                self.tuba(ctx, tgt - 4 + bt, 0.6 * beat, 0.9)?,
                tb,
                0.58,
                0.0,
            );
            put(mix, "guitar", self.chop(ctx, ch, 1.0, false, CHOP_DUR)?, tb, 0.44, 0.18);
            put(mix, "drums", self.kick(0.95)?, tb, 0.50, 0.0);
            put(mix, "drums", self.cymbal(0.9, (bt % 2) as u64)?, tb, 0.06, 0.32);
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
        // round(dur, 3): 0.1234 and 0.1226 share a key; -0.0001 rounds to the key of 0
        assert_eq!(r3(0.1234), r3(0.1226));
        assert_eq!(r3(-0.0001), r3(0.0));
        // a press roll's crescendo is keyed by round(v0 / v, 3): the first call's sound for both
        let bed = HotJazzClassic::new();
        let first = bed.press(0.2, 1.0, 5, Some(0.35)).unwrap();
        assert_eq!(first, super::super::press(0.2, 1.0, 5, Some(0.35)));
        assert_eq!(bed.press(0.2, 1.0, 5, Some(0.3502)).unwrap(), first);
        assert_ne!(bed.press(0.2, 1.0, 5, None).unwrap(), first);
        // a cymbal is keyed by its whole seed, scaled after the cache
        let c = bed.cymbal(1.0, 2).unwrap();
        assert_eq!(bed.cymbal(0.6, 2).unwrap(), scaled(&c, 0.6));
    }

    #[test]
    fn the_bed_has_boom_and_hit() {
        let bed = HotJazzClassic::new();
        assert_eq!(bed.hooks(), &[BedHook::Boom, BedHook::Hit]);
        assert!(bed.has(BedHook::Boom) && !bed.has(BedHook::PressRoll));
    }
}
