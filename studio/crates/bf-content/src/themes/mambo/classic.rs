//! The mambo's classic arrangement: the `Bed` class of `projects/ballfight/kit/moods/mambo.py`, section by section
//! as `kit.music.score()` calls them, each sound placed in the kit's order (sounds add up on a bus in that order,
//! buses come into being in it, and the cache below fills in it).
//!
//! **The kit's cache.** The kit's bed keeps every percussion sound and the sax pad in a dict (`self._c`) under a key
//! made when it is first asked for, and hands that sound back for every later call with the same key. The pad's key
//! holds the duration rounded to 3 decimals, so a later pad that differs only past the rounding gets the first
//! one's sound: the cache is part of the music. [`MamboClassic`] keeps the same cache under the same keys, filled in
//! the same order (one bed per build, as the kit makes one bed per soundtrack).
//!
//! **The voices** go through the music context's `note` (which has its own cache and each voice's level): the
//! piano `M.note('PIANO', int(m), round(dur, 3), round(dur + 0.18, 2), round(vel, 1))`, the bass
//! `M.note('CONTRABASS', int(m), round(min(dur, 0.45), 3), round(dur + 0.12, 2), round(vel, 1))` and the sax
//! `M.note(ALTO, int(m), round(dur, 3), round(dur + 0.2, 2), round(vel, 1))`.
//!
//! **The alto's generator and the ALTO SAX.** The sax notes' breath comes from the old alto module's generator,
//! which the ALTO SAX's voice and part draw from too. The kit makes the fighters' parts before the score starts, and
//! the ALTO SAX's part measures its voice's level then (`self.g0 = M.inst_gain(M.CFG[k]['pp'])`, a note of the
//! ALTO SAX's voice drawn from that generator), so in a fight with the ALTO SAX the generator has moved on by that
//! note before the bed's first sax note. The Studio's build does the same: it makes both parts (`Sound::made`)
//! before any bed method, so this bed draws in the kit's order without knowing who fights.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::{py_max, py_min, round_half_even, round_nd};
use bf_sound::SRF;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, THUNDER_CRACK, noise_seed, riser, thunder};
use bf_sound::synth::{self, FILTER_O, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED, clip};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{ALTO, CASCARA, CLAVE, CONGA, Conga, MONTUNO, fifth, root, tones_from};

/// The name the bed's errors carry.
const H: &str = "mambo";

/// The PIANO's and the CONTRABASS's Pixel Pitch voice ids, which the bed plays itself.
const PIANO: &str = "PIANO";
const CONTRABASS: &str = "CONTRABASS";

/// A key of the kit's `self._c`: the method's tag and its key fields, a duration as `round(dur, 3)` (by its bits;
/// a rounded zero is +0.0, as Python's `-0.0 == 0.0`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// `('pad', ch, round(dur, 3))`
    Pad(String, u64),
    /// `('cg', kind, low)`
    Conga(Conga, bool),
    /// `('tb', hi, rim)`
    Timbale(bool, bool),
    /// `('sh',)`
    Shell,
    /// `('bl', mouth)`
    Bell(bool),
    /// `('cl',)`
    Clave,
    /// `('bo',)`
    Bombo,
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

/// `self._pad(x, n)`: `x` cut to `n` samples, or padded with zeros to them.
fn fit_to(x: &[f64], n: usize) -> Vec<f64> {
    let mut out: Vec<f64> = x[..n.min(x.len())].to_vec();
    out.resize(n, 0.0);
    out
}

/// `a + b` of two numpy arrays, which numpy refuses when their lengths differ.
fn add(a: &[f64], b: impl ExactSizeIterator<Item = f64>) -> Vec<f64> {
    assert_eq!(a.len(), b.len(), "{H}: operands could not be broadcast together");
    a.iter().zip(b).map(|(a, b)| a + b).collect()
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

/// The mambo's bed as the kit plays it, with the kit's cache of sounds.
#[derive(Debug, Default)]
pub struct MamboClassic {
    cache: Mutex<BTreeMap<Key, Arc<Vec<f64>>>>,
}

impl MamboClassic {
    /// The bed, its cache empty.
    pub fn new() -> MamboClassic {
        MamboClassic::default()
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

    // ------------------------------------------------------------ pitched instruments (Pixel Pitch voices)

    /// `self.piano(m, dur, vel)`.
    fn piano(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        ctx.note(
            PIANO,
            f64::from(m),
            round_nd(dur, 3),
            round_nd(dur + 0.18, 2),
            round_nd(vel, 1),
        )
    }

    /// `self.octave(m, dur, vel, top)`: a montuno hit: the note in octaves (and a third octave on top in the back
    /// half).
    fn octave(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64, top: bool) -> Result<Vec<f64>, MusicError> {
        let a = self.piano(ctx, m, dur, vel)?;
        let b = self.piano(ctx, m - 12, dur, vel)?;
        let mut x = add(&a, b.iter().map(|v| 0.8 * v));
        if top {
            let c = fit_to(&self.piano(ctx, m + 12, dur, vel)?, x.len());
            x = add(&x, c.iter().map(|v| 0.45 * v));
        }
        Ok(scaled(&x, 0.6))
    }

    /// `self.chord(ch, dur, vel)`: a piano stab: the tones of the chord from G3 plus the root an octave down.
    fn chord(&self, ctx: &MusicCtx, ch: &str, dur: f64, vel: f64) -> Result<Vec<f64>, MusicError> {
        let ts = tones_from(&ctx.harmony, ch, 55).map_err(bad)?;
        let mut x = self.piano(ctx, ts[0] - 12, dur, vel)?.to_vec();
        for &m in &ts {
            let s = fit_to(&self.piano(ctx, m, dur, vel)?, x.len());
            x = add(&x, s.into_iter());
        }
        let k = 0.6 + 0.4 * ts.len() as f64;
        Ok(x.iter().map(|v| v / k).collect())
    }

    /// `self.bass(m, dur, vel)`: the CONTRABASS's voice, its held length at most 0.45 s (a pizzicato).
    fn bass(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64) -> Result<Vec<f64>, MusicError> {
        let x = ctx.note(
            CONTRABASS,
            f64::from(m),
            round_nd(py_min(dur, 0.45), 3),
            round_nd(dur + 0.12, 2),
            round_nd(vel, 1),
        )?;
        Ok(x.to_vec())
    }

    /// `self.sax(m, dur, vel)`: the old alto's voice.
    fn sax(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        ctx.note(
            ALTO,
            f64::from(m),
            round_nd(dur, 3),
            round_nd(dur + 0.2, 2),
            round_nd(vel, 1),
        )
    }

    /// `self.pad(ch, dur)`: the sax section: three tones of the chord around A3-F4, soft attack, a slow fade at the
    /// end.
    fn pad(&self, ctx: &MusicCtx, ch: &str, dur: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Pad(ch.to_string(), r3(dur)), || {
            let mut ts = tones_from(&ctx.harmony, ch, 57).map_err(bad)?;
            ts.truncate(3);
            let n = length((dur + 0.2) * SRF);
            let mut x = vec![0.0; n];
            for (j, &m) in ts.iter().enumerate() {
                let s = self.sax(ctx, m, dur, 0.55)?;
                let g = 1.0 - 0.12 * j as f64;
                for (o, v) in x.iter_mut().zip(s.iter().take(n)) {
                    *o += v * g;
                }
            }
            let k = ts.len() as f64;
            let xs: Vec<f64> = x.iter().map(|v| v / k).collect();
            let y = synth::lp(&xs, 2600.0, FILTER_O);
            let t = synth::tt(n);
            Ok(y.iter().zip(&t).map(|(y, &t)| y * clip(t / 0.09, 0.0, 1.0)).collect())
        })?;
        Ok(x.to_vec())
    }

    // ------------------------------------------------------------ percussion (synthesised)

    /// `self.conga(kind, v, low)`.
    fn conga(&self, kind: Conga, v: f64, low: bool) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Conga(kind, low), || Ok(super::conga(kind, low)))?;
        Ok(scaled(&x, v))
    }

    /// `self.timbale(v, hi, rim)`.
    fn timbale(&self, v: f64, hi: bool, rim: bool) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Timbale(hi, rim), || Ok(super::timbale(hi, rim)))?;
        Ok(scaled(&x, v))
    }

    /// `self.shell(v)`.
    fn shell(&self, v: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Shell, || Ok(super::shell()))?;
        Ok(scaled(&x, v))
    }

    /// `self.bell(v, mouth)`.
    fn bell(&self, v: f64, mouth: bool) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Bell(mouth), || Ok(super::bell(mouth)))?;
        Ok(scaled(&x, v))
    }

    /// `self.clave(v)`.
    fn clave(&self, v: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Clave, || Ok(super::clave()))?;
        Ok(scaled(&x, v))
    }

    /// `self.bombo(v)`.
    fn bombo(&self, v: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Bombo, || Ok(super::bombo()))?;
        Ok(scaled(&x, v))
    }

    /// `self.roll(mix, t0, dur, g0, g1, seed)`: an abanico: a timbale roll on the 32nds, a crescendo from `g0` to
    /// `g1`, ending on a rimshot (placed to finish on the target).
    #[allow(clippy::too_many_arguments)]
    fn roll(&self, ctx: &MusicCtx, mix: &mut Mix, t0: f64, dur: f64, g0: f64, g1: f64, seed: i64) -> Played {
        let step = ctx.show.grid.s16 / 2.0;
        let k = (round_half_even(dur / step) as i64).max(1);
        for j in 0..k {
            let a = j as f64 / (k - 1).max(1) as f64;
            let odd = j % 2 == 1;
            let s = self.timbale(0.8 + 0.2 * (j % 2) as f64, (j + seed).rem_euclid(3) != 2, false)?;
            let pan = if odd { -0.25 } else { -0.1 };
            put(mix, "perc", s, t0 + j as f64 * step, g0 + (g1 - g0) * a, pan);
        }
        put(
            mix,
            "perc",
            self.timbale(1.0, true, true)?,
            t0 + k as f64 * step,
            g1 * 1.05,
            -0.18,
        );
        Ok(())
    }

    fn root(&self, ctx: &MusicCtx, ch: &str) -> Result<i32, MusicError> {
        root(&ctx.harmony, ch).map_err(bad)
    }

    /// `Bed.bar(mix, b, intensity, light)`: one bar of the mambo (bar index `b`): montuno, tumbao, congas, cascara
    /// (front) or mambo bell (back), clave, the sax pad; the back half (`intensity` above 1.25) adds the octave-up
    /// montuno, bass ghosts and extra slaps. Python's defaults are `intensity = 1.0` and `light = False`.
    ///
    /// # Errors
    /// Where Python raises: a chord the harmony does not hold, a note a voice cannot make.
    pub fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, light: bool) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        let t0 = b as f64 * bar;
        let e8 = beat / 2.0;
        let nb = grid.n_bars;
        let ch = chord(ctx, b % nb)?;
        let nxt = chord(ctx, (b + 1) % nb)?;
        let big = intensity > 1.25 && !light;
        // the montuno
        let side = b % 2;
        let ts = tones_from(&ctx.harmony, ch, 67).map_err(bad)?;
        let tn = tones_from(&ctx.harmony, nxt, 67).map_err(bad)?;
        for (j, &e) in MONTUNO[side].iter().enumerate() {
            let last = side == 1 && e == 7;
            let m = if last { tn[0] } else { ts[(j + 2 * side) % ts.len()] };
            let dur = if e == 3 || e == 7 { 0.18 } else { 0.14 };
            let vel = if e == 0 || e == 3 || e == 7 { 0.9 } else { 0.75 };
            put(
                mix,
                "piano",
                self.octave(ctx, m, dur, vel, big)?,
                t0 + e as f64 * e8,
                0.34,
                0.22,
            );
        }
        // the tumbao
        let f5 = fifth(&ctx.harmony, ch).map_err(bad)?;
        put(
            mix,
            "bass",
            self.bass(ctx, f5, 0.9 * beat, 0.85)?,
            t0 + 3.0 * e8,
            0.62,
            0.0,
        );
        let rn = self.root(ctx, nxt)?;
        put(
            mix,
            "bass",
            self.bass(ctx, rn, 1.9 * beat, 0.95)?,
            t0 + 6.0 * e8,
            0.68,
            0.0,
        );
        if big {
            let m = if rn + 12 <= 55 { rn + 12 } else { rn };
            put(
                mix,
                "bass",
                self.bass(ctx, m, 0.3 * beat, 0.6)?,
                t0 + 7.0 * e8,
                0.30,
                0.0,
            );
        }
        // congas
        for (e, &k) in CONGA.iter().enumerate() {
            let g = match k {
                Conga::Muff => 0.10,
                Conga::Slap => 0.22,
                Conga::Open => 0.26,
            };
            let te = t0 + e as f64 * e8;
            put(
                mix,
                "perc",
                self.conga(k, 1.0, k == Conga::Open && e == 7)?,
                te,
                g,
                0.35,
            );
            if big && e == 5 {
                put(mix, "perc", self.conga(Conga::Slap, 0.8, false)?, te, 0.14, 0.40);
            }
        }
        // timbales: the cascara on the shell (front) / the mambo bell on the quarters (back), the clave
        for e in 0..8usize {
            let j = side * 8 + e;
            let te = t0 + e as f64 * e8;
            if CASCARA[j] && !big {
                let v = 0.9 + 0.1 * if e % 2 == 0 { 1.0 } else { 0.0 };
                put(mix, "perc", self.shell(v)?, te, 0.10, -0.22);
            }
            if CLAVE[j] {
                put(mix, "perc", self.clave(1.0)?, te, 0.035, 0.05);
            }
        }
        for bt in 0..4usize {
            let tb = t0 + bt as f64 * beat;
            if big {
                put(mix, "perc", self.bell(1.0, bt % 2 == 0)?, tb, 0.13, -0.28);
            } else {
                put(mix, "perc", self.bell(0.8, true)?, tb, 0.07, -0.28);
            }
        }
        if big {
            put(mix, "perc", self.timbale(1.0, false, true)?, t0 + 3.0 * e8, 0.09, -0.2);
        }
        // the sax pad, one chord a bar, low
        put(mix, "sax", self.pad(ctx, ch, 0.94 * bar)?, t0 + 0.01, 0.30, -0.05);
        // an abanico into every phrase
        let ls = ctx.harmony.loop_start;
        if b >= ls && (b - ls) % 4 == 3 && !light {
            self.roll(
                ctx,
                mix,
                t0 + 3.5 * beat,
                0.5 * beat - grid.s16 / 2.0,
                0.08,
                0.22,
                b as i64,
            )?;
        }
        Ok(())
    }

    /// `Bed.ability_fills(mix)`: a timbale fill into every ability the fighters fire (the fill's rimshot lands on the
    /// ability, on the 16th), each at most once per half bar, up to the K.O.
    ///
    /// # Errors
    /// As the roll's sounds.
    pub fn ability_fills(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let s16 = grid.s16;
        let mut last = -9.0;
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if e.kind != "ability" || t - last < grid.bar / 2.0 {
                continue;
            }
            last = t;
            let tq = ctx.q16(t);
            let seed = noise_seed(H, t * 7.0)? as i64;
            self.roll(ctx, mix, tq - 3.0 * s16, 3.0 * s16 - s16 / 2.0, 0.10, 0.24, seed)?;
            put(mix, "perc", self.bell(1.0, true)?, tq, 0.10, -0.28);
        }
        Ok(())
    }
}

impl Bed for MamboClassic {
    /// The mood's class defines `boom` and `hit`, none of the other optional hooks.
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom, BedHook::Hit]
    }

    /// `buses = {'perc': (1.35, 0.12), 'bass': (1.7, 0.08), 'piano': (1.25, 0.22), 'sax': (0.85, 0.35)}`.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &[
            ("perc", (1.35, 0.12)),
            ("bass", (1.7, 0.08)),
            ("piano", (1.25, 0.22)),
            ("sax", (0.85, 0.35)),
        ]
    }

    /// A dance hall.
    fn reverb(&self) -> (f64, f64, f64) {
        (1.6, 0.5, 0.17)
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
        (91.0, 98.0, 0.10)
    }

    /// A big drum hit (a slam landing): the bombo, a low rimshot, the bass's root of the chord at `t`.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        put(mix, "perc", self.bombo(1.0)?, t, g, 0.0);
        put(mix, "perc", self.timbale(1.0, false, true)?, t, 0.5 * g, 0.0);
        let r = self.root(ctx, ctx.harmony.chord_at(t))?;
        put(mix, "bass", self.bass(ctx, r, 0.5, 1.0)?, t, 0.45 * g, 0.0);
        Ok(())
    }

    /// A full band hit: piano stab, bass root, sax chord, timbale rimshot, bombo, a conga slap.
    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, ring: f64) -> Played {
        put(mix, "piano", self.chord(ctx, ch, ring, 0.95)?, t, 0.52 * g, 0.22);
        let r = self.root(ctx, ch)?;
        put(mix, "bass", self.bass(ctx, r, ring, 1.0)?, t, 0.70 * g, 0.0);
        put(mix, "sax", self.pad(ctx, ch, ring)?, t, 0.36 * g, -0.05);
        put(mix, "perc", self.timbale(1.0, true, true)?, t, 0.22 * g, -0.18);
        put(mix, "perc", self.bombo(1.0)?, t, 0.55 * g, 0.0);
        put(mix, "perc", self.conga(Conga::Slap, 1.0, false)?, t, 0.20 * g, 0.35);
        Ok(())
    }

    /// `mix.place('fx', self.conga('s', 1.0), t, 0.06)`.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", self.conga(Conga::Slap, 1.0, false)?, t, 0.06, 0.0);
        Ok(())
    }

    /// A full band hit under each word of the VS card.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let ch = chord(ctx, 0)?;
        for tw in times {
            self.hit(ctx, mix, tw, ch, 1.0, 0.45)?;
            put(mix, "perc", synth::crash(1.0), tw, 0.07, 0.0);
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

    /// Under the card (never quiet): the piano rolling the chord in 8ths, the sax pad held, the bass root, conga open
    /// tones, the cascara, an abanico on beat 4 as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let beat = grid.beat;
        let ch = chord(ctx, 0)?;
        let ts = tones_from(&ctx.harmony, ch, 67).map_err(bad)?;
        for e in 1..7usize {
            let x = self.octave(ctx, ts[e % ts.len()], 0.14, 0.8, false)?;
            put(mix, "piano", x, e as f64 * beat / 2.0, 0.30, 0.22);
        }
        put(mix, "sax", self.pad(ctx, ch, 3.0 * beat)?, 0.05, 0.34, -0.05);
        let r = self.root(ctx, ch)?;
        put(mix, "bass", self.bass(ctx, r, 2.0 * beat, 0.9)?, 0.05, 0.50, 0.0);
        for (e, &on) in CASCARA[..8].iter().enumerate() {
            let te = e as f64 * beat / 2.0;
            if on {
                put(mix, "perc", self.shell(1.0)?, te, 0.10, -0.22);
            }
            if e == 6 || e == 7 {
                put(mix, "perc", self.conga(Conga::Open, 1.0, e == 7)?, te, 0.24, 0.35);
            }
        }
        self.roll(ctx, mix, 3.0 * beat, beat - grid.s16 / 2.0, 0.10, 0.26, 7)
    }

    /// Bar 1: the band keeps the mambo on the count-in chord (the groove from the card on), a band stab on each
    /// count, a timbale roll and a bass walk-up into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        self.bar(ctx, mix, 1, 1.4, true)?;
        let ch = chord(ctx, 1)?;
        for (tc, _) in &grid.count[..3] {
            self.hit(ctx, mix, *tc, ch, 0.55, 0.30)?;
        }
        let t4 = bar + 3.0 * beat;
        self.roll(ctx, mix, t4, beat - grid.s16 / 2.0, 0.12, 0.28, 9)?;
        let walk = self.root(ctx, chord(ctx, 2)?)? - 2;
        put(mix, "bass", self.bass(ctx, walk, 0.3 * beat, 0.85)?, t4, 0.40, 0.0);
        Ok(())
    }

    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_fight = ctx.show.grid.t_fight;
        put(mix, "perc", synth::crash(1.8), t_fight, 0.12, 0.0);
        put(mix, "fx", synth::thud(0.7, 110.0, 34.0, THUD_SEED), t_fight, 0.34, 0.0);
        Ok(())
    }

    /// The mambo from FIGHT! to the K.O. (denser as it goes), timbale fills into the abilities, a long roll and a
    /// riser into the back half, crashes on the phrase lines, a tape stop at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, 1.0 + 0.6 * (b as f64 / bar_ko as f64), false)?;
        }
        for b in (2..=bar_ko).step_by(8) {
            put(mix, "perc", synth::crash(1.6), b as f64 * bar, 0.08, 0.0);
        }
        self.ability_fills(ctx, mix)?;
        let t_half = (ctx.harmony.loop_start + 2 * ctx.harmony.loop_.len()) as f64 * bar;
        self.roll(
            ctx,
            mix,
            t_half - 2.0 * beat,
            2.0 * beat - grid.s16 / 2.0,
            0.08,
            0.26,
            33,
        )?;
        let rise = riser(bar, 0.0, 12)?.iter().map(|v| v * 0.6).collect();
        put(mix, "fx", rise, t_half - bar, 0.08, 0.0);
        put(mix, "perc", synth::crash(2.0), t_half, 0.12, 0.0);
        for bus in ["perc", "bass", "piano", "sax"] {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, ctx.show.t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        put(mix, "perc", synth::crash(2.4), t_ko, 0.16, 0.0);
        put(mix, "fx", synth::thud(0.9, 120.0, 30.0, THUD_SEED), t_ko, 0.6, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        self.hit(ctx, mix, tw, chord(ctx, 0)?, 1.0, 0.8)?;
        put(mix, "perc", synth::crash(1.8), tw, 0.10, 0.0);
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

    /// The last bar builds into the title slam on sample 0: a timbale roll right up to the seam, the bass walking up
    /// chromatically into the tonic, piano stabs and the mambo bell on every beat, conga slaps, a riser.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, dur) = (grid.beat, grid.bar, grid.dur);
        let t_last = (grid.n_bars as f64 - 1.0) * bar;
        let ch = chord(ctx, ctx.harmony.chords.len().saturating_sub(1))?;
        let tgt = self.root(ctx, chord(ctx, 0)?)?;
        for bt in 0..4i32 {
            let tb = t_last + f64::from(bt) * beat;
            put(
                mix,
                "bass",
                self.bass(ctx, tgt - 4 + bt, 0.6 * beat, 0.9)?,
                tb,
                0.62,
                0.0,
            );
            put(mix, "piano", self.chord(ctx, ch, 0.22, 0.9)?, tb, 0.40, 0.22);
            put(mix, "perc", self.bell(1.0, bt % 2 == 0)?, tb, 0.13, -0.28);
            let kind = if bt % 2 == 1 { Conga::Slap } else { Conga::Open };
            put(mix, "perc", self.conga(kind, 1.0, false)?, tb + beat / 2.0, 0.20, 0.35);
            put(mix, "perc", self.bombo(0.9)?, tb, 0.40, 0.0);
        }
        put(mix, "sax", self.pad(ctx, ch, dur - t_last)?, t_last, 0.34, -0.05);
        let roll = dur - t_last - 2.0 * beat - grid.s16 / 2.0 + 0.012;
        self.roll(ctx, mix, t_last + 2.0 * beat, roll, 0.10, 0.30, 99)?;
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
        // a percussion sound is keyed by its kind, scaled after the cache
        let bed = MamboClassic::new();
        let c = bed.conga(Conga::Slap, 1.0, false).unwrap();
        assert_eq!(c, super::super::conga(Conga::Slap, false));
        assert_eq!(bed.conga(Conga::Slap, 0.8, false).unwrap(), scaled(&c, 0.8));
        assert_ne!(bed.conga(Conga::Slap, 1.0, true).unwrap(), c);
        assert_eq!(
            bed.timbale(0.5, true, true).unwrap(),
            scaled(&super::super::timbale(true, true), 0.5)
        );
    }

    #[test]
    fn the_bed_has_boom_and_hit() {
        let bed = MamboClassic::new();
        assert_eq!(bed.hooks(), &[BedHook::Boom, BedHook::Hit]);
        assert!(bed.has(BedHook::Boom) && !bed.has(BedHook::PressRoll));
    }

    #[test]
    fn a_short_sound_is_padded_and_a_long_one_cut() {
        assert_eq!(fit_to(&[1.0, 2.0], 4), [1.0, 2.0, 0.0, 0.0]);
        assert_eq!(fit_to(&[1.0, 2.0, 3.0], 2), [1.0, 2.0]);
    }
}
