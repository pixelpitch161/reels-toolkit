//! ELECTRIC's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/electric.py` (#44).
//!
//! Its Pixel Pitch voice through a fuzz and a cab (`fuzz`: a tanh on the high-passed note, two low-passes): every
//! bolt is a riff note (its line, eight a bar); a landed bolt hits it full, with a power-chord stab on the riff's
//! first note or a forked bolt, and a zap; a miss is palm-muted and fizzles; a hurt fighter's riff loses notes
//! (`Thinner`). A post strike adds the ropes' buzz, DISTORTION doubles the riff an octave up through more gain and
//! starts with a squeal, TAPPING plays tapped notes high up the chord (a hammer-on flourish to start it), the smash
//! a huge chord crash, feedback while it lies in pieces and a pickup slide as it snaps back. The part reads the
//! run's events, nothing of its look.
//!
//! Notes are kept as the kit keeps them (`self._fz`): the first note made under `(int(m), round(dur, 3),
//! round(cap, 2), round(vel, 2), round(gain, 1), int(cab))` is every later one's.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::dmath::{self, PI};
use bf_core::harmony::HarmonyError;
use bf_core::py::{imod_py, py_max, round_half_even, round_nd};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    ColourArgs, MusicCtx, MusicError, Samples, Thinner, bend, event_int, event_is, event_is_str, event_number, gliss,
    vibrato,
};
use bf_sound::synth::{self, FILTER_O};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "ELECTRIC";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// `round(x, nd)` as a key (+ 0.0: a rounded -0.0 is the key 0.0, as Python's 0.0 == -0.0).
fn bits(x: f64, nd: i32) -> u64 {
    (round_nd(x, nd) + 0.0).to_bits()
}

/// A note of `self._fz`: `(int(m), round(dur, 3), round(cap, 2), round(vel, 2), round(gain, 1), int(cab))`, the
/// rounded numbers by their bits.
type Key = (i64, u64, u64, u64, u64, i64);

/// Python's `int(x * SR)` for a length in samples, cut toward zero.
fn samples(seconds: f64) -> Result<usize, MusicError> {
    let n = seconds * SRF;
    if n.is_finite() && n > -1.0 && n < 1.0e9 {
        Ok(n as usize)
    } else {
        Err(MusicError::new(H, format!("{seconds} s is not a length")))
    }
}

/// `np.sign(x)`.
fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

/// `zap(dur=0.09, seed=0, f0=3800, f1=500)`: a lightning zap: a crackle burst over a falling chirp.
fn zap(dur: f64, seed: u64, f0: f64, f1: f64) -> Samples {
    let n = samples(dur)?;
    let t = synth::tt(n);
    // f = f1 + (f0 - f1) * np.exp(-t / (dur * 0.3))
    let f: Vec<f64> = t
        .iter()
        .map(|&t| f1 + (f0 - f1) * dmath::exp(-t / (dur * 0.3)))
        .collect();
    // chirp = np.sign(np.sin(2 * np.pi * np.cumsum(f) / SR)) * np.exp(-t / (dur * 0.4))
    let chirp: Vec<f64> = dsp::cumsum(&f)
        .iter()
        .zip(&t)
        .map(|(&c, &t)| sign(dmath::sin(2.0 * PI * c / SRF)) * dmath::exp(-t / (dur * 0.4)))
        .collect();
    // crk = A.bp(np.random.default_rng(seed).normal(0, 1, n), 2500, 11000) * np.exp(-t / 0.012)
    let crk = synth::bp(&synth::noise(seed, n), 2500.0, 11000.0, FILTER_O);
    let lp = synth::lp(&chirp, 7000.0, FILTER_O);
    Ok(lp
        .iter()
        .zip(&crk)
        .zip(&t)
        .map(|((&c, &k), &t)| (0.35 * c + k * dmath::exp(-t / 0.012)) * 0.6)
        .collect())
}

/// `buzz(dur=0.18, seed=0)`: the ropes buzzing: a mains-hum saw with a crackle.
fn buzz(dur: f64, seed: u64) -> Samples {
    let n = samples(dur)?;
    let t = synth::tt(n);
    // saw = 2 * ((t * 120) % 1.0) - 1
    let saw: Vec<f64> = t
        .iter()
        .map(|&t| 2.0 * bf_core::py::fmod_py(t * 120.0, 1.0) - 1.0)
        .collect();
    // A.bp(rng(seed + 9).normal(0, 1, n), 1500, 7000) * (rng(seed).random(n) < 0.02)
    let crk = synth::bp(&synth::noise(seed + 9, n), 1500.0, 7000.0, FILTER_O);
    let mut gate = bf_core::rng::Generator::new(seed);
    let lp = synth::lp(&saw, 2500.0, FILTER_O);
    Ok(lp
        .iter()
        .zip(&crk)
        .zip(&t)
        .map(|((&s, &c), &t)| {
            let on = if gate.random() < 0.02 { 1.0 } else { 0.0 };
            // (A.lp(saw, 2500) * 0.5 + crk) * np.clip(t / 0.01, 0, 1) * np.exp(-t / (dur * 0.5))
            (s * 0.5 + c * on) * synth::clip(t / 0.01, 0.0, 1.0) * dmath::exp(-t / (dur * 0.5))
        })
        .collect())
}

/// The ELECTRIC's part.
#[derive(Debug)]
pub struct ElectricSound {
    k: usize,
    /// `self._fz`: every fuzzed note made so far, by its key.
    fz: Mutex<BTreeMap<Key, Arc<[f64]>>>,
}

impl ElectricSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> ElectricSound {
        ElectricSound {
            k,
            fz: Mutex::new(BTreeMap::new()),
        }
    }

    /// `self.pan`: -0.22 on the left, 0.22 on the right.
    fn pan(&self) -> f64 {
        if self.k == 0 { -0.22 } else { 0.22 }
    }

    /// `self.lo, self.hi`: its roster register.
    fn register(&self, ctx: &MusicCtx) -> (i32, i32) {
        ctx.cfg[self.k].register
    }

    /// `M.note(self.pp, int(m), dur, cap, vel)`.
    fn raw(&self, ctx: &MusicCtx, m: f64, dur: f64, cap: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        let pp = ctx.cfg[self.k].pp.as_str();
        ctx.note(pp, m.trunc(), dur, cap, vel)
    }

    /// `fuzz(x, gain=6.0, cab=4800)`: the fuzz and the cab: `np.tanh(gain * A.hp(x, 120) / (np.abs(x).max() +
    /// 1e-9))`, then `A.lp(A.lp(y, cab), cab * 1.3) * 0.32`.
    fn fuzz(x: &[f64], gain: f64, cab: f64) -> Samples {
        // np.abs(x).max(): raises on an empty note
        let top = x
            .iter()
            .map(|v| v.abs())
            .reduce(|a, b| if b > a || b.is_nan() { b } else { a })
            .ok_or_else(|| MusicError::new(H, "the fuzz of an empty note"))?;
        let den = top + 1e-9;
        let y: Vec<f64> = synth::hp(x, 120.0, FILTER_O)
            .iter()
            .map(|&v| dmath::np_tanh(gain * v / den))
            .collect();
        let y = synth::lp(&synth::lp(&y, cab, FILTER_O), cab * 1.3, FILTER_O);
        Ok(y.iter().map(|v| v * 0.32).collect())
    }

    /// `note(m, dur=0.22, cap=0.6, vel=0.85, gain=6.0, cab=4800)`: a note of its voice through the fuzz, kept by
    /// `(int(m), round(dur, 3), round(cap, 2), round(vel, 2), round(gain, 1), int(cab))`.
    #[allow(clippy::too_many_arguments)] // the Python signature
    fn note(
        &self,
        ctx: &MusicCtx,
        m: f64,
        dur: f64,
        cap: f64,
        vel: f64,
        gain: f64,
        cab: f64,
    ) -> Result<Arc<[f64]>, MusicError> {
        let key = (
            m.trunc() as i64,
            bits(dur, 3),
            bits(cap, 2),
            bits(vel, 2),
            bits(gain, 1),
            cab.trunc() as i64,
        );
        let mut fz = self
            .fz
            .lock()
            .map_err(|_| MusicError::new(H, "an earlier note panicked"))?;
        if let Some(x) = fz.get(&key) {
            return Ok(Arc::clone(x));
        }
        let x: Arc<[f64]> = Self::fuzz(&self.raw(ctx, m, dur, cap, vel)?, gain, cab)?.into();
        fz.insert(key, Arc::clone(&x));
        Ok(x)
    }

    /// `power(root, dur=0.3, cap=0.8, vel=0.95, gain=7.0)`: a power chord (root, fifth, octave) through the fuzz.
    fn power(&self, ctx: &MusicCtx, root: i32, dur: f64, cap: f64, vel: f64, gain: f64) -> Samples {
        // x = sum(M.note(...) for m in (root, root + 7, root + 12)): from the int 0
        let mut x: Vec<f64> = Vec::new();
        for (j, m) in [root, root + 7, root + 12].into_iter().enumerate() {
            let y = self.raw(ctx, f64::from(m), dur, cap, vel)?;
            if j == 0 {
                x = y.iter().map(|v| 0.0 + v).collect();
            } else {
                if y.len() != x.len() {
                    return Err(MusicError::new(H, "the power chord's notes differ in length"));
                }
                for (a, b) in x.iter_mut().zip(y.iter()) {
                    *a += b;
                }
            }
        }
        Self::fuzz(&x, gain, 4200.0)
    }

    /// `root_of(t)`: the chord's root at `t`, from MIDI 40 up.
    fn root_of(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        ctx.harmony.root_above(ctx.harmony.chord_at(t), 40).map_err(bad)
    }

    /// `riff_note(t)`: its line's note at `t`, folded by octaves into its register.
    fn riff_note(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let (lo, hi) = self.register(ctx);
        let mut m = ctx
            .line_at(self.name(ctx), t)?
            .ok_or_else(|| MusicError::new(H, format!("its line rests at {t} s (Python compares None)")))?;
        while m > hi {
            m -= 12;
        }
        while m < lo {
            m += 12;
        }
        Ok(m)
    }

    /// `tap_note(t, j)`: tone `j` of the chord's tones from `hi - 7` to `hi + 7` (`[hi]` when there is none).
    fn tap_note(&self, ctx: &MusicCtx, t: f64, j: i64) -> Result<i32, MusicError> {
        let hi = self.register(ctx).1;
        let mut tones = ctx
            .harmony
            .chord_tones(hi - 7, hi + 7, ctx.harmony.chord_at(t))
            .map_err(bad)?;
        if tones.is_empty() {
            tones = vec![hi];
        }
        Ok(tones[imod_py(j, tones.len() as i64) as usize])
    }

    /// `self.colour(x, t)` with its colour arguments: `(signal, gain, lead)`.
    fn col(&self, ctx: &MusicCtx, x: &[f64], t: f64) -> Result<(Vec<f64>, f64, f64), MusicError> {
        self.colour(ctx, x, t, 1.0, &self.colour_args())
    }
}

impl Sound for ElectricSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.5
    }

    fn send(&self) -> f64 {
        0.3
    }

    /// Its riff's bite (the fuzz above the bed's chug).
    fn band(&self) -> (f64, f64) {
        (1200.0, 6000.0)
    }

    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 110.0,
            cut_span: 500.0,
            drive: 3.0,
            ..ColourArgs::default()
        }
    }

    /// `voice(m, dur, vel=0.85)`: `self.note(m, dur, max(0.4, dur + 0.25), vel)`.
    fn voice(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.note(ctx, m, dur, py_max(0.4, dur + 0.25), vel, 6.0, 4800.0)
    }

    /// A distorted power-chord blast with a crackle of lightning (the fight file's title hits: its notes, else
    /// the tonic).
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let hits = ctx
            .ff
            .music
            .title_hits
            .get(self.name(ctx))
            .filter(|h| !h.is_empty())
            .cloned();
        let r = match &hits {
            Some(h) => h[0],
            None => self.root_of(ctx, 0.0)?,
        };
        let bus = self.bus(ctx);
        let pan = self.pan();
        mix.place(&bus, &Sig::Mono(self.power(ctx, r, 0.9, 1.6, 1.0, 8.0)?), t, 0.55, pan);
        for j in 0..3u64 {
            let jf = j as f64;
            mix.place(
                "fx",
                &Sig::Mono(zap(0.12, 3 + j, 6000.0, 400.0)?),
                t + jf * 0.045,
                0.22,
                pan * (1.0 - jf),
            );
        }
        if let Some(h) = hits {
            for (j, &m) in h[1..].iter().enumerate() {
                let x = self.note(ctx, f64::from(m), 0.12, 0.4, 0.9, 7.0, 4800.0)?;
                mix.place(&bus, &Sig::Mono(x.to_vec()), t + 0.09 + j as f64 * 0.06, 0.30, pan);
            }
        }
        Ok(())
    }

    /// Power chords on the count, palm-muted chugs into FIGHT!.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let r = self.root_of(ctx, grid.bar)?;
        let bus = self.bus(ctx);
        let pan = self.pan();
        for &(tc, _) in grid.count.iter().take(3) {
            mix.place(
                &bus,
                &Sig::Mono(self.power(ctx, r, 0.18, 0.5, 0.95, 7.0)?),
                tc,
                0.36,
                pan,
            );
        }
        for st in 12..16 {
            let x = synth::lp(&self.power(ctx, r, 0.07, 0.25, 0.8, 7.0)?, 1500.0, FILTER_O);
            mix.place(&bus, &Sig::Mono(x), grid.bar + f64::from(st) * grid.s16, 0.24, pan);
        }
        Ok(())
    }

    /// LIGHTNING RIFF: every bolt a riff note; DISTORTION, TAPPING and the smash.
    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let grid = &ctx.show.grid;
        let bus = self.bus(ctx);
        let pan = self.pan();
        let (_, hi) = self.register(ctx);
        let mut th = Thinner::new(41.0 + k as f64);
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            let kind = e.kind.as_str();
            if kind == "bolt" && event_is(H, e, 2, k)? {
                let tq = ctx.q16(t);
                let hit = event_int(H, e, 9)? != 0;
                let src = event_int(H, e, 4)?;
                let fork = event_int(H, e, 11)? != 0;
                let tap = event_int(H, e, 12)? != 0;
                let hp01 = ctx.hp_at(k, t) / 100.0;
                if tap {
                    // TAPPING: tapped notes high up the chord
                    let j = round_half_even(tq / (grid.beat / 2.0)) as i64;
                    let m = self.tap_note(ctx, tq, j)?;
                    let x = self.note(ctx, f64::from(m), 0.16, 0.4, 0.8, 5.0, 6000.0)?;
                    let (sig, g, _) = self.col(ctx, &x, t)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, if hit { 0.30 } else { 0.22 } * g, pan * 0.6);
                    if hit {
                        let seed = (t * 31.0) as u64;
                        mix.place("fx", &Sig::Mono(zap(0.05, seed, 5000.0, 1500.0)?), tq, 0.08, -pan);
                    }
                    continue;
                }
                // a hurt fighter's riff loses notes
                if !th.keep(round_half_even(tq / grid.s16), hp01) {
                    continue;
                }
                let m = self.riff_note(ctx, tq)?;
                let g = if hit {
                    let x = self.note(ctx, f64::from(m), 0.22, 0.6, 0.95, if fork { 9.5 } else { 7.0 }, 4800.0)?;
                    let (sig, g, _) = self.col(ctx, &x, t)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.40 * g, pan);
                    if event_int(H, e, 3)? == 0 || fork {
                        // the riff's first note / a forked bolt: a stab
                        let stab = self.power(ctx, self.root_of(ctx, tq)?, 0.18, 0.5, 0.95, 7.0)?;
                        mix.place(&bus, &Sig::Mono(self.col(ctx, &stab, t)?.0), tq, 0.22 * g, pan);
                    }
                    let seed = (t * 37.0) as u64;
                    mix.place("fx", &Sig::Mono(zap(0.09, seed, 3800.0, 500.0)?), tq, 0.10, -pan * 0.5);
                    g
                } else {
                    // a miss: palm-muted, the bolt fizzles
                    let x = self.note(ctx, f64::from(m), 0.10, 0.3, 0.75, 6.0, 4800.0)?;
                    let (sig, g, _) = self.col(ctx, &synth::lp(&x, 2200.0, FILTER_O), t)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.30 * g, pan);
                    let seed = (t * 41.0) as u64;
                    mix.place("fx", &Sig::Mono(zap(0.06, seed, 2400.0, 900.0)?), tq, 0.05, pan);
                    g
                };
                if fork {
                    // DISTORTION: the riff doubled an octave up
                    let x = self.note(ctx, f64::from(m + 12), 0.18, 0.5, 0.8, 10.0, 6500.0)?;
                    mix.place(
                        &bus,
                        &Sig::Mono(self.col(ctx, &x, t)?.0),
                        tq + 0.004,
                        0.18 * g,
                        -pan * 0.4,
                    );
                }
                if src >= 0 {
                    // a post strike: the ropes buzz into it
                    let x0 = event_number(H, e, 5)?;
                    let p = synth::clip(x0 / 4.0, -0.6, 0.6);
                    let seed = (t * 13.0) as u64;
                    mix.place("fx", &Sig::Mono(buzz(0.16, seed)?), tq - 0.10, 0.07, p);
                }
            } else if kind == "ability" && event_is(H, e, 2, k)? {
                let tq = ctx.q16(t);
                if event_is_str(H, e, 4, "fork")? {
                    // DISTORTION: the gain kicks in with a squeal
                    let x = self.note(ctx, f64::from(hi + 7), 0.6, 1.0, 1.0, 12.0, 7000.0)?;
                    mix.place(&bus, &Sig::Mono(bend(&x, 2.0, 0.1, 0.5)?), tq, 0.22, pan);
                    mix.place("fx", &Sig::Mono(zap(0.2, 77, 7000.0, 300.0)?), tq, 0.14, 0.0);
                } else if event_is_str(H, e, 4, "tapping")? {
                    // TAPPING: a quick hammer-on flourish to start it
                    for j in 0..4i64 {
                        let m = self.tap_note(ctx, tq, j)?;
                        let x = self.note(ctx, f64::from(m), 0.08, 0.25, 0.9, 5.0, 6000.0)?;
                        mix.place(&bus, &Sig::Mono(x.to_vec()), tq + j as f64 * grid.s16 / 2.0, 0.20, pan);
                    }
                }
            } else if kind == "smash" && event_is(H, e, 2, k)? {
                // the smash: the crash, feedback, the snap back
                let tq = ctx.q16(t);
                let r = self.root_of(ctx, tq)?;
                let low = if r - 12 >= 40 { r - 12 } else { r };
                mix.place(
                    &bus,
                    &Sig::Mono(self.power(ctx, low, 1.2, 1.8, 1.0, 12.0)?),
                    tq,
                    0.55,
                    pan,
                );
                let thud = synth::thud(0.9, 110.0, 30.0, (t * 7.0) as u64);
                mix.place("fx", &Sig::Mono(thud), tq, 0.7, 0.0);
                mix.place("fx", &Sig::Mono(synth::crash(1.6)), tq, 0.16, 0.0);
                mix.place("fx", &Sig::Mono(zap(0.35, 5, 9000.0, 120.0)?), tq, 0.25, 0.0);
                // feedback while it lies in pieces
                let rebuild = event_number(H, e, 6)?;
                let n = samples(rebuild)?;
                let f = synth::mtof(f64::from(hi + 12));
                let fb: Vec<f64> = synth::tt(n)
                    .iter()
                    .map(|&tt| {
                        let ph = 2.0 * PI * f * tt + 3.0 * dmath::sin(2.0 * PI * 6.0 * tt);
                        dmath::sin(ph) * synth::clip(tt / 0.15, 0.0, 1.0)
                    })
                    .collect();
                let fb: Vec<f64> = Self::fuzz(&fb, 4.0, 6000.0)?
                    .iter()
                    .zip(dsp::linspace(0.6, 1.0, n))
                    .map(|(v, s)| v * s)
                    .collect();
                mix.place(&bus, &Sig::Mono(fb), tq + 0.05, 0.16, pan);
                // the pickup slide up as it snaps back together
                let te = ctx.q16(t + rebuild);
                for (j, m) in [r, r + 7, r + 12, r + 19].into_iter().enumerate() {
                    let x = self.note(ctx, f64::from(m), 0.06, 0.2, 0.9, 8.0, 4800.0)?;
                    mix.place(&bus, &Sig::Mono(x.to_vec()), te - (4 - j) as f64 * 0.035, 0.22, pan);
                }
                mix.place(
                    &bus,
                    &Sig::Mono(self.power(ctx, r, 0.25, 0.6, 1.0, 9.0)?),
                    te,
                    0.36,
                    pan,
                );
            }
        }
        Ok(())
    }

    /// The amp dies: a dive-bombed chord, crackle and the hum fading.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t0 = ctx.show.t_hs;
        let x = self.power(ctx, self.root_of(ctx, t0)?, 1.4, 1.6, 1.0, 10.0)?;
        mix.place(
            &self.bus(ctx),
            &Sig::Mono(bend(&x, -12.0, 0.05, 1.2)?),
            t0,
            0.40,
            self.pan(),
        );
        for j in 0..5u64 {
            let jf = j as f64;
            let z = zap(0.08, 60 + j, 5000.0 - 700.0 * jf, 400.0)?;
            mix.place("fx", &Sig::Mono(z), t0 + 0.07 * jf, 0.10, (jf - 2.0) / 5.0);
        }
        mix.place("fx", &Sig::Mono(buzz(1.2, 61)?), t0 + 0.2, 0.10, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let r = self.root_of(ctx, 0.0)?;
        let x = self.power(ctx, r, 1.0, 1.8, 1.0, 9.0)?;
        mix.place("solo", &Sig::Mono(x), tw, 0.42, 0.0);
        let y = self.note(ctx, f64::from(r + 24), 0.8, 1.2, 1.0, 10.0, 6500.0)?;
        mix.place("solo", &Sig::Mono(bend(&y, -0.5, 0.4, 1.0)?), tw + 0.02, 0.18, 0.0);
        mix.place("fx", &Sig::Mono(zap(0.18, 88, 8000.0, 300.0)?), tw, 0.14, 0.0);
        Ok(())
    }

    /// The fight file's solo: a shredded line through the fuzz (bends into the long notes).
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let dur = py_max(0.08, d * grid.beat * 0.95);
            let cap = py_max(0.3, d * grid.beat + 0.2);
            let mut x = self.note(ctx, f64::from(m), dur, cap, 0.95, 8.0, 5500.0)?.to_vec();
            if d >= 1.0 {
                x = vibrato(&gliss(&x, 1.0, 0.06)?, 0.4, 6.0, 0.12)?;
            }
            mix.place("solo", &Sig::Mono(x), ts, 0.42, 0.0);
        }
        Ok(())
    }

    /// Its pieces fly home as a rising slide, the amp humming back on.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let r = self.root_of(ctx, 0.0)?;
        let run = [r, r + 3, r + 5, r + 7, r + 10, r + 12, r + 15, r + 19];
        for (j, &m) in run.iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            let x = self.note(ctx, f64::from(m), 0.14, 0.4, 0.7, 6.0, 4800.0)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), tj, 0.22, 0.0);
        }
        let hum = buzz(grid.t_glide - grid.t_reassemble, 62)?;
        mix.place("fx", &Sig::Mono(hum), grid.t_reassemble, 0.05, 0.0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The zap and the buzz are their lengths, and the gate of the buzz's crackle comes from its own generator.
    #[test]
    fn the_zap_and_the_buzz() {
        assert_eq!(zap(0.09, 0, 3800.0, 500.0).unwrap().len(), (0.09 * SRF) as usize);
        let b = buzz(0.18, 3).unwrap();
        assert_eq!(b.len(), (0.18 * SRF) as usize);
        assert_eq!(b[0], 0.0, "the buzz fades in from silence");
        assert!(ElectricSound::fuzz(&[], 6.0, 4800.0).is_err());
    }
}
