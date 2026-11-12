//! The dark tango's classic arrangement: the `Bed` class of `projects/ballfight/kit/moods/dark_tango.py`, section by
//! section as `kit.music.score()` calls them, each sound placed in the kit's order (sounds add up on a bus in that
//! order, buses come into being in it, and the cache below fills in it).
//!
//! **The kit's cache.** The kit's bed keeps every instrument's sound in a dict (`self._c`) under a key made when it
//! is first asked for, and hands that sound back (times the velocity) for every later call with the same key. A
//! duration is keyed by `round(dur, 3)`, so the first call's exact duration wins. The stab is built from the cached
//! reeds, the bellows from the cached stab and the chop from the cached strings, so a reed first made for a stab is
//! the one a later lone reed with the same key gets. [`DarkTangoClassic`] keeps the same cache under the same keys,
//! filled in the same order (one bed per build, as the kit makes one bed per soundtrack).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::{py_max, round_half_even, round_nd};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, THUNDER_CRACK, noise_seed, riser, thunder};
use bf_sound::synth::{self, FILTER_O, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{
    CHOP_DUR, PLUCK_RING, REED_ATT, REED_BARK, ROOT_LO, chicharra, chop_voicing, golpe, latigo, pluck, reed, root,
    voicing,
};

/// The name the bed's errors carry.
const H: &str = "dark_tango";

/// The bed's own buses, in the order the mood lists them (`for k in self.buses` cuts and tape-stops them in it).
const BUSES: [(&str, (f64, f64)); 3] = [
    ("bandoneon", (1.55, 0.26)),
    ("guitar", (1.25, 0.20)),
    ("perc", (1.3, 0.12)),
];

/// A key of the kit's `self._c`: the method's tag and its key fields, a duration as `round(dur, 3)` and a float
/// argument by its bits (a rounded zero is +0.0, as Python's `-0.0 == 0.0`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// `('reed', int(m), round(dur, 3), round(att, 3), bark)`
    Reed(i32, u64, u64, u64),
    /// `('stab', ch, round(dur, 3), low)`
    Stab(String, u64, bool),
    /// `('shake', ch, round(dur, 3))`
    Shake(String, u64),
    /// `('pl', int(m), round(dur, 3), seed, bright, ring)`
    Pluck(i32, u64, i64, u64, u64),
    /// `('ch', ch, ghost, round(dur, 3))`
    Chop(String, bool, u64),
    /// `('golpe', seed)`
    Golpe(u64),
    /// `('chi', seed)`
    Chicharra(u64),
    /// `('lat',)`
    Latigo,
}

/// A float as a key (by its bits; -0.0 as +0.0).
fn fk(x: f64) -> u64 {
    if x == 0.0 { 0.0f64.to_bits() } else { x.to_bits() }
}

/// `round(x, 3)` as a key.
fn r3(x: f64) -> u64 {
    fk(round_nd(x, 3))
}

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

fn scaled(x: &[f64], v: f64) -> Vec<f64> {
    x.iter().map(|a| a * v).collect()
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

/// Python's `int(x)` of a time in samples (`int((ts + 0.16) * SR)`): cut toward zero.
fn int(x: f64) -> i64 {
    x as i64
}

/// `g[lo:hi] = np.minimum(g[lo:hi], ramp)`: numpy's slice (cut at the end of `g`) must hold `ramp` exactly, or
/// numpy raises (a stop so near the end of the soundtrack that its fade does not fit).
fn ramp_min(g: &mut [f64], lo: i64, ramp: &[f64]) -> Result<(), MusicError> {
    let n = g.len() as i64;
    let (a, b) = (lo.clamp(0, n) as usize, (lo + ramp.len() as i64).clamp(0, n) as usize);
    if b - a != ramp.len() || lo < 0 {
        return Err(MusicError::new(
            H,
            format!("a stop's fade at sample {lo} does not fit in the bus's {n} samples"),
        ));
    }
    for (v, &r) in g[a..b].iter_mut().zip(ramp) {
        // np.minimum: the smaller (a NaN wins; there is none)
        if r < *v {
            *v = r;
        }
    }
    Ok(())
}

/// The dark tango's bed as the kit plays it, with the kit's cache of sounds.
#[derive(Debug, Default)]
pub struct DarkTangoClassic {
    cache: Mutex<BTreeMap<Key, Arc<Vec<f64>>>>,
}

impl DarkTangoClassic {
    /// The bed, its cache empty.
    pub fn new() -> DarkTangoClassic {
        DarkTangoClassic::default()
    }

    /// `self._cache(key, fn)`: the sound under `key`, made by `make` the first time. The lock is not held while
    /// it is made: a stab asks the cache for its reeds, a chop for its strings.
    fn cached(
        &self,
        key: Key,
        make: impl FnOnce() -> Result<Vec<f64>, MusicError>,
    ) -> Result<Arc<Vec<f64>>, MusicError> {
        let hit = self.cache.lock().expect("the cache").get(&key).cloned();
        Ok(match hit {
            Some(x) => x,
            None => {
                let x = Arc::new(make()?);
                Arc::clone(self.cache.lock().expect("the cache").entry(key).or_insert(x))
            }
        })
    }

    /// `self.reed(m, dur, vel, att, bark)`.
    fn reed(&self, m: i32, dur: f64, vel: f64, att: f64, bark: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Reed(m, r3(dur), r3(att), fk(bark)), || Ok(reed(m, dur, att, bark)))?;
        Ok(scaled(&x, vel))
    }

    /// `self.stab(ch, dur, vel, low)`: the right hand's four reeds averaged (Python's `sum` from 0), and with `low`
    /// the left hand's root from A2 up (its bark at 0.6, at 0.8).
    fn stab(&self, ctx: &MusicCtx, ch: &str, dur: f64, vel: f64, low: bool) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Stab(ch.to_string(), r3(dur), low), || {
            let vo = voicing(&ctx.harmony, ch).map_err(bad)?;
            let mut x: Vec<f64> = Vec::new();
            for (j, &m) in vo.iter().enumerate() {
                let r = self.reed(m, dur, 1.0, REED_ATT, REED_BARK)?;
                if j == 0 {
                    x = r.iter().map(|v| 0.0 + v).collect();
                } else if r.len() == x.len() {
                    for (a, b) in x.iter_mut().zip(&r) {
                        *a += b;
                    }
                } else {
                    return Err(MusicError::new(H, "a stab's reeds have different lengths"));
                }
            }
            let k = vo.len() as f64;
            let mut x: Vec<f64> = x.iter().map(|v| v / k).collect();
            if low {
                let r = root(&ctx.harmony, ch, 45).map_err(bad)?;
                let lo: Vec<f64> = scaled(&self.reed(r, dur, 1.0, REED_ATT, 0.6)?, 0.8);
                let n = x.len().max(lo.len());
                x.resize(n, 0.0);
                for (a, b) in x.iter_mut().zip(lo.iter().chain(std::iter::repeat(&0.0))) {
                    *a += b;
                }
            }
            Ok(x)
        })?;
        Ok(scaled(&x, vel))
    }

    /// `self.bellows_shake(ch, dur, vel)`: the held chord with its low octave, the bellows shaken at 7.2 Hz.
    fn bellows_shake(&self, ctx: &MusicCtx, ch: &str, dur: f64, vel: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Shake(ch.to_string(), r3(dur)), || {
            let x = self.stab(ctx, ch, dur, 1.0, true)?;
            let t = synth::tt(x.len());
            Ok(x.iter()
                .zip(&t)
                .map(|(v, &t)| v * (0.75 + 0.25 * bf_core::dmath::sin(2.0 * bf_core::dmath::PI * 7.2 * t)))
                .collect())
        })?;
        Ok(scaled(&x, vel))
    }

    /// `self.pluck(m, dur, seed, bright, ring)` (no velocity: the kit returns the cached sound itself).
    fn pluck(&self, m: i32, dur: f64, seed: i64, bright: f64, ring: f64) -> Result<Arc<Vec<f64>>, MusicError> {
        self.cached(Key::Pluck(m, r3(dur), seed, fk(bright), fk(ring)), || {
            Ok(pluck(m, dur, seed, bright, ring))
        })
    }

    /// `self.chop(ch, v, ghost, dur)`: the chop chord's strings strummed down 5 ms apart, choked at `dur` (a ghost at
    /// 40 ms, darker, mostly the scratch of seed 78's noise).
    fn chop(&self, ctx: &MusicCtx, ch: &str, v: f64, ghost: bool, dur: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Chop(ch.to_string(), ghost, r3(dur)), || {
            let vo = chop_voicing(&ctx.harmony, ch).map_err(bad)?;
            let d = (0.005 * SRF) as usize;
            let n = ((dur + 0.05) * SRF) as usize + d * vo.len();
            let mut x = vec![0.0; n];
            let bright = if ghost { 0.25 } else { 0.4 };
            for (j, &m) in vo.iter().enumerate() {
                let s = self.pluck(m, dur + 0.05, j as i64, bright, PLUCK_RING)?;
                // `x[j * d:j * d + len(s)] += s`
                let lo = (j * d).min(n);
                let hi = (j * d + s.len()).min(n);
                if hi - lo != s.len() {
                    return Err(MusicError::new(H, "a chop's string does not fit its strum"));
                }
                for (a, b) in x[lo..hi].iter_mut().zip(s.iter()) {
                    *a += b;
                }
            }
            let t = synth::tt(n);
            let cut = if ghost { 0.04 } else { dur };
            let k = vo.len().max(1) as f64;
            let low: Vec<f64> = x.iter().map(|v| v / k).collect();
            let low = synth::lp(&low, 3200.0, FILTER_O);
            let scratch = synth::bp(&synth::noise(78, n), 1500.0, 6000.0, FILTER_O);
            let (sg, out) = if ghost { (0.9, 0.45) } else { (0.4, 1.0) };
            Ok((0..n)
                .map(|i| {
                    let t = t[i];
                    let fall = if t < cut {
                        bf_core::dmath::exp(-t / 0.07)
                    } else {
                        bf_core::dmath::exp(-cut / 0.07) * bf_core::dmath::exp(-(t - cut) / 0.012)
                    };
                    let env = synth::clip(t / 0.002, 0.0, 1.0) * fall;
                    let scratch = scratch[i] * bf_core::dmath::exp(-t / 0.01) * sg;
                    (low[i] * env + scratch) * out
                })
                .collect())
        })?;
        Ok(scaled(&x, v))
    }

    /// `self.golpe(v, seed)`.
    fn golpe(&self, v: f64, seed: u64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Golpe(seed), || Ok(golpe(seed)))?;
        Ok(scaled(&x, v))
    }

    /// `self.chicharra(v, seed)`.
    fn chicharra(&self, v: f64, seed: u64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Chicharra(seed), || Ok(chicharra(seed)))?;
        Ok(scaled(&x, v))
    }

    /// `self.latigo(v)`.
    fn latigo(&self, v: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Latigo, || Ok(latigo()))?;
        Ok(scaled(&x, v))
    }

    fn voicing(&self, ctx: &MusicCtx, ch: &str) -> Result<Vec<i32>, MusicError> {
        voicing(&ctx.harmony, ch).map_err(bad)
    }

    /// `Bed.bar(mix, b, intensity, light)`: one bar of the marcato (bar index `b`): bandoneon stabs on every beat (1
    /// and 3 accented, the low octave on 1), the guitar's chop on 2 and 4, the golpe on 1 and 3; the back half
    /// (`intensity` above 1.25 and not `light`) plays the sincopa on odd bars and adds the chicharra on the
    /// off-beats of 2 and 4 and ghost chops on 1 and 3; and, unless `light`, an arrastre into every phrase (the last
    /// bar of each four of the loop). Python's defaults are `intensity = 1.0` and `light = False`.
    ///
    /// # Errors
    /// Where Python raises: a chord the harmony does not hold.
    pub fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, light: bool) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, nb) = (grid.beat, grid.bar, grid.n_bars);
        let t0 = b as f64 * bar;
        let ch = chord(ctx, b % nb)?;
        let big = intensity > 1.25 && !light;
        let sincopa = big && b % 2 == 1;
        let hits: &[(f64, f64)] = if sincopa {
            &[(0.0, 1.0), (1.5, 0.75), (3.0, 0.85)]
        } else {
            &[(0.0, 1.0), (1.0, 0.62), (2.0, 0.9), (3.0, 0.62)]
        };
        for &(bt, v) in hits {
            let dur = if v < 0.8 { 0.15 } else { 0.19 };
            let x = self.stab(ctx, ch, dur, 1.0, bt == 0.0)?;
            put(mix, "bandoneon", x, t0 + bt * beat, 0.34 * v, -0.12);
        }
        for bt in 0..4u64 {
            let tb = t0 + bt as f64 * beat;
            if bt == 0 || bt == 2 {
                let g = if light { 0.24 } else { 0.30 };
                put(mix, "perc", self.golpe(1.0, b as u64 * 4 + bt)?, tb, g, 0.0);
                if big {
                    put(mix, "guitar", self.chop(ctx, ch, 1.0, true, CHOP_DUR)?, tb, 0.28, 0.2);
                }
            } else {
                put(mix, "guitar", self.chop(ctx, ch, 1.0, false, CHOP_DUR)?, tb, 0.44, 0.2);
            }
            if big && (bt == 1 || bt == 3) {
                put(
                    mix,
                    "perc",
                    self.chicharra(1.0, b as u64 + bt)?,
                    tb + beat / 2.0,
                    0.11,
                    0.3,
                );
            }
        }
        let ls = ctx.harmony.loop_start;
        if b >= ls && (b - ls) % 4 == 3 && !light {
            // the arrastre into the phrase
            let nxt = chord(ctx, (b + 1) % nb)?;
            for m in self.voicing(ctx, nxt)? {
                let x = synth::lp(&self.reed(m - 1, beat * 0.45, 1.0, 0.08, REED_BARK)?, 4000.0, FILTER_O);
                put(mix, "bandoneon", x, t0 + 3.5 * beat, 0.07, -0.12);
            }
        }
        Ok(())
    }

    /// `Bed.stop_times()`: every stop the fighters' parts ask for (their `stops()`, side by side as the score
    /// gathered them), those from FIGHT! up to (not at) the K.O., sorted.
    ///
    /// # Errors
    /// A voice panicked earlier (the context's lock).
    pub fn stop_times(&self, ctx: &MusicCtx) -> Result<Vec<f64>, MusicError> {
        let (t_fight, t_ko) = (ctx.show.grid.t_fight, ctx.show.t_ko);
        let mut out: Vec<f64> = ctx
            .fighter_stops()?
            .into_iter()
            .filter(|&t| t_fight <= t && t < t_ko)
            .collect();
        // Python's sorted: stable, by `<` (no NaN passes the filter)
        out.sort_by(|a, b| a.partial_cmp(b).expect("a stop time"));
        Ok(out)
    }

    /// `Bed.apply_stops(mix, stops)`: each stop, the band hits on it (ringing 0.18 s); then each of the bed's own
    /// buses the mix has is cut from 0.16 s after the stop to 12 ms before the next beat (12 ms fades, a fade never
    /// raising a gain another stop lowered), unless that leaves under 50 ms.
    ///
    /// # Errors
    /// Where Python raises: a chord the harmony does not hold, or a fade past the end of the bus.
    pub fn apply_stops(&self, ctx: &MusicCtx, mix: &mut Mix, stops: &[f64]) -> Played {
        let beat = ctx.show.grid.beat;
        for &ts in stops {
            let ch = ctx.harmony.chord_at(ts).to_string();
            self.hit(ctx, mix, ts, &ch, 1.0, 0.18)?;
        }
        let f = (0.012 * SRF) as usize;
        let fall = dsp::linspace(1.0, 0.0, f);
        let rise = dsp::linspace(0.0, 1.0, f);
        let shortest = int(0.05 * SRF);
        for (bus, _) in BUSES {
            let Some(x) = mix.bus(bus) else {
                continue;
            };
            let n = x.len();
            let mut g = vec![1.0; n];
            for &ts in stops {
                let a = int((ts + 0.16) * SRF);
                // `math.ceil(ts / T.BEAT + 1e-6) * T.BEAT`
                let nb = (ts / beat + 1e-6).ceil() * beat;
                let b = int((nb - 0.012) * SRF);
                if b - a < shortest {
                    continue;
                }
                let fi = f as i64;
                ramp_min(&mut g, a, &fall)?;
                // `g[a + f:b - f] = 0.0`: numpy's slice, cut at the end
                let (lo, hi) = (
                    (a + fi).clamp(0, n as i64) as usize,
                    (b - fi).clamp(0, n as i64) as usize,
                );
                if lo < hi {
                    g[lo..hi].fill(0.0);
                }
                ramp_min(&mut g, b - fi, &rise)?;
            }
            let cut: Vec<[f64; 2]> = x.iter().zip(&g).map(|(fr, &g)| [fr[0] * g, fr[1] * g]).collect();
            *mix.bus_mut(bus) = cut;
        }
        Ok(())
    }

    /// `Bed.theme()`: the tango's theme, the line of the first fighter (left corner first) whose role is `melody`
    /// and who has a line in the fight file; `None` when there is none.
    pub fn theme<'a>(&self, ctx: &'a MusicCtx) -> Option<&'a str> {
        let music = &ctx.ff.music;
        ctx.show
            .names
            .iter()
            .find(|n| {
                music.roles.get(n.as_str()).is_some_and(|r| r == "melody") && music.lines.contains_key(n.as_str())
            })
            .map(String::as_str)
    }
}

impl Bed for DarkTangoClassic {
    /// The mood's class defines `boom` and `hit`.
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom, BedHook::Hit]
    }

    /// `buses = {'bandoneon': (1.55, 0.26), 'guitar': (1.25, 0.20), 'perc': (1.3, 0.12)}`.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &BUSES
    }

    /// A milonga hall.
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
        (93.0, 98.0, 0.10)
    }

    /// A band hit on the chord at `t` (0.8 of `g`, ringing 0.4 s) and a thud of seed `int(t * 7)`.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        let ch = ctx.harmony.chord_at(t).to_string();
        self.hit(ctx, mix, t, &ch, 0.8 * g, 0.4)?;
        let seed = noise_seed(H, t * 7.0)?;
        put(mix, "fx", synth::thud(0.6, 110.0, 32.0, seed), t, 0.4 * g, 0.0);
        Ok(())
    }

    /// A full band hit: the bandoneon's chord with its low octave (ringing `ring` s), the guitar's chop (0.2 s), the
    /// golpe of seed `int(t * 10)`.
    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, ring: f64) -> Played {
        put(
            mix,
            "bandoneon",
            self.stab(ctx, ch, ring, 1.0, true)?,
            t,
            0.46 * g,
            -0.12,
        );
        put(mix, "guitar", self.chop(ctx, ch, 1.0, false, 0.2)?, t, 0.5 * g, 0.2);
        let seed = noise_seed(H, t * 10.0)?;
        put(mix, "perc", self.golpe(1.0, seed)?, t, 0.42 * g, 0.0);
        Ok(())
    }

    /// A soft golpe (seed `int(t * 31) % 50`).
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        let seed = noise_seed(H, t * 31.0)? % 50;
        put(mix, "perc", self.golpe(0.8, seed)?, t, 0.10, 0.0);
        Ok(())
    }

    /// A full band hit under each word of the VS card, a thud and a latigo just before it.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let ch = chord(ctx, 0)?;
        for tw in times {
            self.hit(ctx, mix, tw, ch, 1.0, 0.4)?;
            let seed = noise_seed(H, tw * 50.0)? + 3;
            put(mix, "fx", synth::thud(0.8, 120.0, 34.0, seed), tw, 0.30, 0.0);
            put(mix, "perc", self.latigo(1.0)?, tw - 0.09, 0.10, 0.0);
        }
        Ok(())
    }

    /// VS: a thunder crack where it splits the divider.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", thunder(1.4, 91, THUNDER_CRACK)?, t, 0.40, 0.0);
        Ok(())
    }

    /// The held chord under the card (never quiet): the bellows shaking, the golpe on each beat after the first, a
    /// chop on their ands, a latigo and an arrastre on beat 4 as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let beat = ctx.show.grid.beat;
        let ch = chord(ctx, 0)?;
        put(
            mix,
            "bandoneon",
            self.bellows_shake(ctx, ch, 3.0 * beat - 0.1, 1.0)?,
            0.08,
            0.46,
            -0.12,
        );
        for bt in 1..4u64 {
            let tb = bt as f64 * beat;
            put(mix, "perc", self.golpe(0.9, bt)?, tb, 0.30, 0.0);
            put(
                mix,
                "guitar",
                self.chop(ctx, ch, 1.0, false, CHOP_DUR)?,
                tb + beat / 2.0,
                0.36,
                0.2,
            );
        }
        put(mix, "perc", self.latigo(1.0)?, 3.5 * beat, 0.10, 0.0);
        for m in self.voicing(ctx, chord(ctx, 1)?)? {
            let x = self.reed(m - 1, beat * 0.45, 1.0, 0.08, REED_BARK)?;
            put(mix, "bandoneon", x, 3.5 * beat, 0.08, -0.12);
        }
        Ok(())
    }

    /// Bar 1: the full marcato on the count-in chord (intensity 1.4: sincopa, chicharra, ghosts), a band hit on each
    /// count, a latigo into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        self.bar(ctx, mix, 1, 1.4, false)?;
        let ch = chord(ctx, 1)?;
        for (tc, _) in &ctx.show.grid.count[..3] {
            self.hit(ctx, mix, *tc, ch, 0.5, 0.2)?;
        }
        put(mix, "perc", self.latigo(1.0)?, ctx.show.grid.t_fight - 0.1, 0.12, 0.0);
        Ok(())
    }

    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_fight = ctx.show.grid.t_fight;
        self.hit(ctx, mix, t_fight, chord(ctx, 2)?, 0.9, 0.3)?;
        put(mix, "fx", synth::thud(0.7, 110.0, 34.0, THUD_SEED), t_fight, 0.34, 0.0);
        Ok(())
    }

    /// The marcato from FIGHT! to the K.O. (denser as it goes), latigos on the phrase lines, a riser into the second
    /// pass of the loop, the tango stops, a tape stop of the bed's buses at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let bar = ctx.show.grid.bar;
        let t_ko = ctx.show.t_ko;
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, 1.0 + 0.6 * (b as f64 / bar_ko.max(1) as f64), false)?;
        }
        for b in (2..=bar_ko).step_by(4) {
            put(mix, "perc", self.latigo(1.0)?, b as f64 * bar - 0.1, 0.10, 0.0);
        }
        let t_half = (ctx.harmony.loop_start + ctx.harmony.loop_.len()) as f64 * bar;
        let rise = riser(bar, 0.0, 12)?.iter().map(|v| v * 0.6).collect();
        put(mix, "fx", rise, t_half - bar, 0.07, 0.0);
        let stops = self.stop_times(ctx)?;
        self.apply_stops(ctx, mix, &stops)?;
        for (bus, _) in BUSES {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        let ch = ctx.harmony.chord_at(t_ko).to_string();
        self.hit(ctx, mix, t_ko, &ch, 1.0, 0.5)?;
        put(mix, "fx", synth::thud(0.9, 120.0, 30.0, THUD_SEED), t_ko, 0.6, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        self.hit(ctx, mix, tw, chord(ctx, 0)?, 1.0, 0.7)?;
        put(mix, "perc", self.latigo(1.0)?, tw - 0.1, 0.12, 0.0);
        Ok(())
    }

    /// A whoosh from just after the stinger to the solo, and the light marcato in the bars between.
    fn after_ko(&self, ctx: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let bar = ctx.show.grid.bar;
        let dur = py_max(0.2, t_solo - tw - 0.1);
        put(
            mix,
            "fx",
            synth::whoosh(dur, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED),
            tw + 0.1,
            0.08,
            0.0,
        );
        // range(int(math.ceil((tw + 0.5) / T.BAR)), int(round(t_solo / T.BAR)))
        let from = ((tw + 0.5) / bar).ceil() as i64;
        let to = round_half_even(t_solo / bar) as i64;
        for b in from..to {
            let b = usize::try_from(b).map_err(|_| MusicError::new(H, format!("bar {b} before the start")))?;
            self.bar(ctx, mix, b, 1.0, true)?;
        }
        Ok(())
    }

    /// The band plays on under the solo at full tilt, up to the build: the bandoneon's left hand on the roots and
    /// fifths, the chop on the ands of 3 and 4, and the theme (the melody fighter's line) an octave up, legato,
    /// doubled an octave down in the reassembly bar.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, nb) = (grid.beat, grid.bar, grid.n_bars);
        let th = self.theme(ctx);
        for b in b_solo..nb.saturating_sub(1) {
            self.bar(ctx, mix, b, 1.4, false)?;
            let ch = chord(ctx, b % nb)?;
            for bt in [0u32, 2] {
                let m = root(&ctx.harmony, ch, ROOT_LO).map_err(bad)? + if bt == 0 { 0 } else { 7 };
                let x = self.reed(m, 0.5 * beat, 1.0, REED_ATT, 0.5)?;
                put(mix, "bandoneon", x, b as f64 * bar + f64::from(bt) * beat, 0.16, -0.12);
            }
            for bt in [2.5, 3.5] {
                let x = self.chop(ctx, ch, 0.8, false, CHOP_DUR)?;
                put(mix, "guitar", x, b as f64 * bar + bt * beat, 0.30, 0.2);
            }
            let Some(th) = th else {
                continue;
            };
            for bt in 0..4u32 {
                let tb = b as f64 * bar + f64::from(bt) * beat;
                let Some(mut m) = ctx.line_at(th, tb)? else {
                    return Err(MusicError::new(
                        H,
                        format!("{th}'s line rests at {tb} s, where the bandoneon plays the theme"),
                    ));
                };
                while m < 62 {
                    m += 12;
                }
                while m > 81 {
                    m -= 12;
                }
                // fuller as the loser's pieces fly home
                let g = if b + 2 < nb { 0.13 } else { 0.22 };
                put(mix, "bandoneon", self.reed(m, beat * 0.95, 1.0, 0.03, 0.4)?, tb, g, 0.1);
                if b + 2 == nb {
                    put(
                        mix,
                        "bandoneon",
                        self.reed(m - 12, beat * 0.95, 1.0, 0.03, 0.4)?,
                        tb,
                        0.12,
                        -0.1,
                    );
                }
            }
        }
        Ok(())
    }

    /// The last bar builds into the title slam on sample 0: stabs on every 8th, golpes on every 8th (the beats
    /// louder), a chicharra on the off-beats, the bandoneon's top line climbing chromatically into the tonic, a
    /// riser right up to the seam.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, dur, nb) = (grid.beat, grid.bar, grid.dur, grid.n_bars);
        let t_last = (nb as f64 - 1.0) * bar;
        let ch = chord(ctx, nb - 1)?;
        for j in 0..8u64 {
            let tj = t_last + j as f64 * beat / 2.0;
            let x = self.stab(ctx, ch, 0.14, 1.0, true)?;
            put(mix, "bandoneon", x, tj, 0.34 + 0.02 * j as f64, -0.12);
            put(mix, "guitar", self.chop(ctx, ch, 1.0, false, CHOP_DUR)?, tj, 0.40, 0.2);
            let g = if j % 2 == 0 { 0.30 } else { 0.18 };
            put(mix, "perc", self.golpe(1.0, 300 + j)?, tj, g, 0.0);
            if j % 2 == 1 {
                put(mix, "perc", self.chicharra(1.0, 400 + j)?, tj, 0.12, 0.3);
            }
        }
        let top = self
            .voicing(ctx, chord(ctx, 0)?)?
            .into_iter()
            .max()
            .ok_or_else(|| MusicError::new(H, "the first chord has no voicing"))?
            + 12;
        for j in 0..4i32 {
            let x = self.reed(top - 4 + j, beat * 0.9, 1.0, 0.02, REED_BARK)?;
            put(mix, "bandoneon", x, t_last + f64::from(j) * beat, 0.20, -0.12);
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
        assert_eq!(r3(0.1234), r3(0.1226));
        assert_eq!(r3(-0.0001), r3(0.0));
        let bed = DarkTangoClassic::new();
        // a duration is keyed by round(dur, 3): the first call's length wins, and the velocity scales it
        let a = bed.reed(62, 0.5004, 1.0, REED_ATT, REED_BARK).unwrap();
        assert_eq!(bed.reed(62, 0.4996, 1.0, REED_ATT, REED_BARK).unwrap(), a);
        assert_eq!(bed.reed(62, 0.5, 0.5, 0.0055, REED_BARK).unwrap(), scaled(&a, 0.5));
        assert_ne!(bed.reed(62, 0.5, 1.0, REED_ATT, 0.6).unwrap(), a);
        // a pluck is not scaled; its seed and brightness are in its key
        let p = bed.pluck(50, 0.17, 0, 0.4, PLUCK_RING).unwrap();
        assert_eq!(*p, pluck(50, 0.17, 0, 0.4, PLUCK_RING));
        assert_ne!(*bed.pluck(50, 0.17, 1, 0.4, PLUCK_RING).unwrap(), *p);
        assert_ne!(*bed.pluck(50, 0.17, 0, 0.25, PLUCK_RING).unwrap(), *p);
        // a golpe and a chicharra are keyed by their whole seed
        assert_ne!(bed.golpe(1.0, 3).unwrap(), bed.golpe(1.0, 53).unwrap());
        assert_eq!(bed.golpe(0.8, 3).unwrap(), scaled(&golpe(3), 0.8));
    }

    #[test]
    fn it_has_boom_and_hit() {
        let bed = DarkTangoClassic::new();
        assert!(bed.has(BedHook::Boom) && bed.has(BedHook::Hit) && !bed.has(BedHook::Erupt));
    }

    #[test]
    fn a_fade_must_fit_its_bus() {
        let mut g = vec![1.0; 10];
        ramp_min(&mut g, 2, &[1.0, 0.5, 0.0]).unwrap();
        assert_eq!(g[1..6], [1.0, 1.0, 0.5, 0.0, 1.0]);
        ramp_min(&mut g, 3, &[0.25, 1.0]).unwrap();
        assert_eq!(g[2..5], [1.0, 0.25, 0.0]);
        assert!(ramp_min(&mut g, 9, &[1.0, 0.0]).is_err());
        assert!(ramp_min(&mut g, 12, &[1.0]).is_err());
    }
}
