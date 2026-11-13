//! TURNTABLES's part in the score: the `Sound` class of its reference kit module
//! (`tools/studio_export/older/fighters/turntables.py`, #31 v3's `audio.py`, the decks' part).
//!
//! Every scratch IS the floor's motion: the record position under the hand is the picture's scratch curve
//! ([`scratch_q`]), sampled at audio rate and played through a chopped F-minor stab ([`src`]), with a thump under each
//! jerk; every rope slam is an organ stab chord in the decks' Pixel Pitch voice (the root doubled an octave down when
//! leading); SCRATCH is a scribble, REWIND a backspin with the tape whirring backwards, BASS DROP a whoosh and then
//! the sub drop, a thud, a stab and a crash; the title blast is a flare over a stab chord, the count-in a chirp a
//! count; it falls apart as a tape-stopped baby scratch; the stinger is a backspin into a stab chord, the solo a stab
//! a note (the fight file's solos hold plain notes; #31's scratch routine needs its own solo entries, ruling M4-R5),
//! the reassembly a rising run of stabs. As its HP drops its sounds thin and dirty (`colour`).
//!
//! The scratches are the deck's own motion curves (`turntables60_lowpoly.qmotion`, [`qmotion`]) and draw no noise
//! from the series' streams: each has its own seeded generator.

use std::collections::BTreeSet;

use bf_core::dmath::{self, PI};
use bf_core::event::Event;
use bf_core::py::{py_max, py_min, round_half_even, round_nd};
use bf_core::roster::AbilityKind;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, Samples, event_is_str, event_number, noise_seed};
use bf_sound::synth::{self, clip, noise};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "TURNTABLES";
/// The series' Butterworth order.
const O: usize = 2;
/// `ANG_PER_SEC`: record audio seconds to floor angle (33 1/3 rpm: 1.8 s a turn).
const ANG_PER_SEC: f64 = 2.0 * PI / 1.8;
/// `SCR_AMP`: the floor angle (rad) of a scratch at strength 1.3.
const SCR_AMP: f64 = 0.40;
/// `turntables60_lowpoly.BEAT_S`: the deck's scene tempo, 130 BPM.
const BEAT_S: f64 = 60.0 / 130.0;

/// A scratch of `turntables60_lowpoly` (its MIDI codes `BABY` .. `BACKSPIN`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `BABY`.
    Baby,
    /// `CHIRP`.
    Chirp,
    /// `TRANS`.
    Trans,
    /// `FWD`.
    Fwd,
    /// `SCRIB`.
    Scrib,
    /// `TEAR`.
    Tear,
    /// `FLARE`.
    Flare,
    /// `BACKSPIN`.
    Backspin,
}

impl Kind {
    /// #31's names: `baby`, `chirp`, `trans`, `flare`, `tear`, `scrib`, `back`, `fwd`.
    pub fn named(name: &str) -> Option<Kind> {
        Some(match name {
            "baby" => Kind::Baby,
            "chirp" => Kind::Chirp,
            "trans" => Kind::Trans,
            "flare" => Kind::Flare,
            "tear" => Kind::Tear,
            "scrib" => Kind::Scrib,
            "back" => Kind::Backspin,
            "fwd" => Kind::Fwd,
            _ => return None,
        })
    }

    /// `SCR_PAR[kind]`: cycles a beat, amplitude (seconds of record).
    fn par(self) -> (f64, f64) {
        match self {
            Kind::Baby | Kind::Chirp => (2.0, 0.13),
            Kind::Trans | Kind::Flare => (1.0, 0.2),
            Kind::Fwd => (2.0, 0.12),
            Kind::Scrib => (8.0, 0.022),
            Kind::Tear => (1.0, 0.18),
            Kind::Backspin => (0.0, 0.0),
        }
    }
}

/// `np_ease(x)`: `x = clip(x, 0, 1); x * x * (3 - 2 * x)`.
fn np_ease(x: f64) -> f64 {
    let x = clip(x, 0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// `x % 1` of numpy for a number that is never negative.
fn frac(x: f64) -> f64 {
    x % 1.0
}

/// `qmotion(kind, t, d)` (`turntables60_lowpoly`): the record position under the hand (seconds of audio from the
/// hold point) and the crossfader gate, at the times `t` of a scratch `d` seconds long.
pub fn qmotion(kind: Kind, t: &[f64], d: f64) -> (Vec<f64>, Vec<f64>) {
    if kind == Kind::Backspin {
        // -1.7 * np.clip(t / d, 0, 1) ** 1.6, np.ones_like(t)
        let q = t
            .iter()
            .map(|&t| -1.7 * dmath::pow(clip(t / d, 0.0, 1.0), 1.6))
            .collect();
        return (q, vec![1.0; t.len()]);
    }
    let (cpb, a) = kind.par();
    // n = max(1, round(d / BEAT_S * cpb))
    let n = py_max(1.0, round_half_even(d / BEAT_S * cpb));
    let mut q = Vec::with_capacity(t.len());
    let mut g = Vec::with_capacity(t.len());
    for &t in t {
        let fr = frac(t * n / d);
        q.push(if kind == Kind::Tear {
            a * if fr < 0.5 {
                if fr < 0.2 {
                    0.5 * np_ease(fr / 0.2)
                } else if fr < 0.3 {
                    0.5
                } else {
                    0.5 + 0.5 * np_ease((fr - 0.3) / 0.2)
                }
            } else {
                1.0 - np_ease((fr - 0.5) / 0.5)
            }
        } else {
            a * (1.0 - dmath::cos(2.0 * PI * fr)) / 2.0
        });
        let hs = frac(2.0 * fr);
        let on = match kind {
            Kind::Chirp => hs < 0.5,
            Kind::Trans => (t / (BEAT_S / 8.0)).floor() % 2.0 == 0.0,
            Kind::Fwd => fr < 0.5,
            Kind::Flare => !(hs > 0.38 && hs < 0.62),
            _ => true,
        };
        g.push(if on { 1.0 } else { 0.0 });
    }
    (q, g)
}

/// `np.gradient(q, h)` with a uniform spacing: second-order central differences inside, one-sided at the ends.
fn gradient(q: &[f64], h: f64) -> Vec<f64> {
    let n = q.len();
    assert!(n >= 2, "np.gradient needs at least two samples");
    let mut out = vec![0.0; n];
    for i in 1..n - 1 {
        out[i] = (q[i + 1] - q[i - 1]) / (2.0 * h);
    }
    out[0] = (q[1] - q[0]) / h;
    out[n - 1] = (q[n - 1] - q[n - 2]) / h;
    out
}

/// `np.interp(idx, np.arange(len(src)), src, left=0.0, right=0.0)`.
fn play_at(idx: &[f64], src: &[f64]) -> Vec<f64> {
    let xp: Vec<f64> = (0..src.len()).map(|i| i as f64).collect();
    dsp::interp_lr(idx, &xp, src, Some(0.0), Some(0.0))
}

/// `y[-fd:] *= np.linspace(1, 0, fd)`.
fn fade_out(y: &mut [f64], fd: usize) {
    let fd = fd.min(y.len());
    let n = y.len();
    for (o, w) in y[n - fd..].iter_mut().zip(dsp::linspace(1.0, 0.0, fd)) {
        *o *= w;
    }
}

/// `np.minimum(a, b)` of two numbers.
fn np_minimum(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if b < a {
        b
    } else {
        a
    }
}

/// `_stab_src()`: the 'record' under the needle: a chopped F-minor synth-brass stab, then a held tail (long enough to
/// backspin), normalised.
pub fn src() -> Vec<f64> {
    let n = (2.4 * SRF) as usize;
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    for m in [53.0, 56.0, 60.0, 65.0, 68.0] {
        for (o, s) in x.iter_mut().zip(synth::saw(synth::mtof(m), n, 0.004, 3)) {
            *o += s;
        }
    }
    // env = np.exp(-t / 0.5) * np.clip(t / 0.004, 0, 1); y = A.lp(x * env, 2600) + 0.6 * A.bp(x * np.exp(-t / 0.08), ..)
    let xe: Vec<f64> = (0..n)
        .map(|i| x[i] * (dmath::exp(-t[i] / 0.5) * clip(t[i] / 0.004, 0.0, 1.0)))
        .collect();
    let xd: Vec<f64> = (0..n).map(|i| x[i] * dmath::exp(-t[i] / 0.08)).collect();
    let low = synth::lp(&xe, 2600.0, O);
    let band = synth::bp(&xd, 1500.0, 5000.0, O);
    let y: Vec<f64> = (0..n).map(|i| low[i] + 0.6 * band[i]).collect();
    let peak = y.iter().fold(0.0f64, |m, v| if v.abs() > m { v.abs() } else { m });
    y.iter().map(|v| v / (peak + 1e-9)).collect()
}

/// `scratch(kind, d, seed=seed)`: the record pushed back and forth under the needle (the deck's motion curves,
/// crossfader cuts included) playing the stab; backspins start further into the record.
pub fn scratch(src: &[f64], kind: Kind, d: f64, seed: u64) -> Vec<f64> {
    let (s0, friction) = (if kind == Kind::Backspin { 1.75 } else { 0.06 }, 0.05);
    let n = ((d + 0.03) * SRF) as usize;
    let t: Vec<f64> = synth::tt(n).iter().map(|&t| np_minimum(t, d)).collect();
    let (q, g) = qmotion(kind, &t, d);
    let vel = gradient(&q, 1.0 / SRF);
    let idx: Vec<f64> = q.iter().map(|&q| (s0 + q) * SRF).collect();
    let rec = play_at(&idx, src);
    let nz = synth::bp(&noise(seed + 11, n), 400.0, 3000.0, O);
    let mut y: Vec<f64> = (0..n)
        .map(|i| {
            let v = vel[i].abs();
            let y = rec[i] * dmath::pow(clip(v, 0.0, 1.6), 0.7);
            y + nz[i] * clip(v, 0.0, 4.0) * friction
        })
        .collect();
    // a = math.exp(-1 / (0.0025 * SR)); y *= lfilter([1 - a], [1, -a], g)
    let a = dmath::exp(-1.0 / (0.0025 * SRF));
    let gate = dsp::lfilter(&[1.0 - a], &[1.0, -a], &g);
    for (o, w) in y.iter_mut().zip(&gate) {
        *o *= w;
    }
    fade_out(&mut y, (0.015 * SRF) as usize);
    y
}

/// The scratches of side `k` from the run: `(t0, length, strength, direction)`.
fn scratches(ctx: &MusicCtx, k: usize) -> Result<Vec<(f64, f64, f64, f64)>, MusicError> {
    ctx.rec
        .events
        .iter()
        .filter(|e| e.kind == "scratch" && e.who() == Some(k))
        .map(|e| {
            Ok((
                e.t,
                event_number(H, e, 3)?,
                event_number(H, e, 4)?,
                event_number(H, e, 5)?,
            ))
        })
        .collect()
}

/// `fx_clock(t)`: real time in the fight, held through the hit-stop, then running on.
fn fx_clock(ctx: &MusicCtx, t: f64) -> f64 {
    let (t_ko, t_hs) = (ctx.show.t_ko, ctx.show.t_hs);
    if t < t_ko {
        t
    } else if t < t_hs {
        t_ko
    } else {
        t - (t_hs - t_ko)
    }
}

/// `scratch_q(t)` (#31 render.py, the module's `Record.scratch_q`): the record's position under the DJ's hand
/// (seconds of record audio) from the fight's scratches: a baby scratch, forward then back, over each one's
/// length; 0 between them.
pub fn scratch_q(ctx: &MusicCtx, scr: &[(f64, f64, f64, f64)], t: f64) -> f64 {
    let tc = fx_clock(ctx, t);
    // j = np.searchsorted(SCR_T, tc, 'right') - 1
    let after = scr.partition_point(|s| s.0 <= tc);
    let Some(j) = after.checked_sub(1) else {
        return 0.0;
    };
    if tc >= ctx.show.t_ko + 0.01 && t >= ctx.show.t_hs {
        return 0.0;
    }
    let (t0, d, stg, sgn) = scr[j];
    let u = (tc - t0) / d;
    if !(0.0..1.0).contains(&u) {
        return 0.0;
    }
    let a = SCR_AMP * stg / 1.3 / ANG_PER_SEC;
    sgn * a * (1.0 - dmath::cos(2.0 * PI * u)) / 2.0
}

/// `floor_scratch(t0, d)`: the scratch the picture shows, the floor's own curve sampled at audio rate and played
/// through the stab; backward-first scratches hold further into the record.
fn floor_scratch(ctx: &MusicCtx, src: &[f64], scr: &[(f64, f64, f64, f64)], t0: f64, d: f64) -> Samples {
    let step = 1.0 / SRF;
    let n = ((d + 0.03) * SRF) as usize;
    let q: Vec<f64> = (0..n).map(|i| scratch_q(ctx, scr, t0 + i as f64 * step)).collect();
    let qmin = q.iter().fold(f64::INFINITY, |m, &v| if v < m { v } else { m });
    let s0 = 0.03 + py_max(0.0, -qmin);
    let vel = gradient(&q, step);
    let idx: Vec<f64> = q.iter().map(|&q| (s0 + q) * SRF).collect();
    let rec = play_at(&idx, src);
    let nz = synth::bp(&noise(noise_seed(H, t0 * 100.0)?, n), 400.0, 3000.0, O);
    let mut y: Vec<f64> = (0..n)
        .map(|i| {
            let v = vel[i].abs();
            rec[i] * dmath::pow(clip(v, 0.0, 1.6), 0.7) + nz[i] * clip(v, 0.0, 4.0) * 0.05
        })
        .collect();
    fade_out(&mut y, (0.012 * SRF) as usize);
    Ok(y)
}

/// `tick(seed)`: a vinyl tick, a ricochet off the rope.
fn tick(seed: u64) -> Vec<f64> {
    let n = (0.03 * SRF) as usize;
    let t = synth::tt(n);
    let x = synth::bp(&noise(seed, n), 2500.0, 9000.0, O);
    x.iter().zip(&t).map(|(x, &t)| x * dmath::exp(-t / 0.004)).collect()
}

/// `sub_drop(dur, f0, f1)`: a falling sub sine, driven.
fn sub_drop(dur: f64, f0: f64, f1: f64) -> Vec<f64> {
    let n = (dur * SRF) as usize;
    let t = synth::tt(n);
    let f: Vec<f64> = t.iter().map(|&t| f1 + (f0 - f1) * dmath::exp(-t / 0.25)).collect();
    let ph = dsp::cumsum(&f);
    (0..n)
        .map(|i| {
            let x = dmath::sin(2.0 * PI * ph[i] / SRF);
            dmath::np_tanh(2.2 * x) * dmath::exp(-t[i] / (dur * 0.6)) * clip(t[i] / 0.002, 0.0, 1.0)
        })
        .collect()
}

/// `rewind_whirr(dur)`: tape whirring backwards: a chirpy, falling-then-rising band of noise and a pitched squeal.
fn rewind_whirr(dur: f64) -> Vec<f64> {
    let n = (dur * SRF) as usize;
    let t = synth::tt(n);
    let x = noise(31, n);
    let mut out = vec![0.0; n];
    let nb = 16;
    for k in 0..nb {
        let (a, b) = (k * n / nb, (k + 1) * n / nb);
        let fc = 900.0 + 5000.0 * (1.0 - (2.0 * (k as f64 + 0.5) / nb as f64 - 1.0).abs());
        let band = synth::bp(&x[a.saturating_sub(1024)..b], fc * 0.6, py_min(fc * 1.5, 20000.0), O);
        out[a..b].copy_from_slice(&band[band.len() - (b - a)..]);
    }
    let f: Vec<f64> = t
        .iter()
        .map(|&t| {
            let u = t / dur;
            600.0 + 2400.0 * (1.0 - (2.0 * u - 1.0).abs()) + 300.0 * dmath::sin(2.0 * PI * 23.0 * t)
        })
        .collect();
    let ph = dsp::cumsum(&f);
    (0..n)
        .map(|i| {
            let u = t[i] / dur;
            let sq = dmath::sin(2.0 * PI * ph[i] / SRF) * 0.4;
            (out[i] * 0.6 + sq) * dmath::pow(dmath::sin(PI * clip(u, 0.0, 1.0)), 0.5)
        })
        .collect()
}

/// `round(x, 3)` of a scratch's length.
fn r3(x: f64) -> f64 {
    round_nd(x, 3)
}

fn mono(x: Vec<f64>) -> Sig {
    Sig::Mono(x)
}

/// TURNTABLES's part.
#[derive(Clone, Copy, Debug)]
pub struct TurntablesSound {
    k: usize,
}

impl TurntablesSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> TurntablesSound {
        TurntablesSound { k }
    }

    /// `stab(m, vel, dur)`: an organ stab in the decks' voice, `note(m, dur, 0.4, vel)`.
    fn stab(&self, ctx: &MusicCtx, m: i32, vel: f64, dur: f64) -> Samples {
        let pp = ctx.cfg[self.k].pp.as_str();
        // (#31's note fits its notes with a 30 ms fade, the kit's 40 ms: the note helper with a fade of its own
        // comes with the shared note accounting, Task 8's NEEDS_CONTEXT item 4b)
        Ok(ctx.note(pp, f64::from(m), dur, 0.4, vel)?.to_vec())
    }
}

impl Sound for TurntablesSound {
    fn k(&self) -> usize {
        self.k
    }

    /// #31's decks bus.
    fn bus_gain(&self) -> f64 {
        3.2
    }

    fn send(&self) -> f64 {
        0.3
    }

    /// The DJ's blast: a scratch flare over an organ-stab chord.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        mix.place(&bus, &mono(scratch(&src(), Kind::Flare, 0.4, 11)), t, 0.40, -0.2);
        for m in [53, 60, 65, 68] {
            mix.place(&bus, &mono(self.stab(ctx, m, 1.0, 0.35)?), t, 0.22, -0.15);
        }
        Ok(())
    }

    /// A chirp on each count.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let src = src();
        for (j, (tc, _)) in ctx.show.grid.count.iter().take(3).enumerate() {
            mix.place(&bus, &mono(scratch(&src, Kind::Chirp, 0.22, j as u64)), *tc, 0.30, -0.2);
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (k, bus) = (self.k, self.bus(ctx));
        let t_ko = ctx.show.t_ko;
        let grid = &ctx.show.grid;
        let src = src();
        let scr = scratches(ctx, k)?;
        let events: &[Event] = &ctx.rec.events;
        // the scratch IS the floor's motion; a thump under each jerk
        for e in events {
            if e.t > t_ko + 0.01 || e.who() != Some(k) {
                continue;
            }
            if e.kind == "scratch" {
                let (t0, d) = (e.t, event_number(H, e, 3)?);
                let x = floor_scratch(ctx, &src, &scr, t0, d)?;
                let (sig, g, _) = self.colour(ctx, &x, t0, 0.8, &self.colour_args())?;
                mix.place(&bus, &mono(sig), t0, 0.34 * g, -0.2);
            } else if e.kind == "jerk" {
                let thump = synth::thud(0.18, 110.0, 50.0, noise_seed(H, e.t * 13.0)?);
                let gain = 0.07 * py_min(1.4, event_number(H, e, 6)? / 1.3);
                mix.place("fx", &mono(thump), e.t, gain, 0.0);
            }
        }
        // a rope slam: an organ stab on the chord
        let mut slots = BTreeSet::new();
        for e in events {
            if e.t > t_ko + 0.01 {
                break;
            }
            if !(e.kind == "hit" && e.who() == Some(k) && event_is_str(H, e, 7, "scratch")?) {
                continue;
            }
            let tq = ctx.q16(e.t);
            let slot = round_half_even(tq / grid.s16) as i64;
            if !slots.insert(slot) {
                continue;
            }
            let ch = ctx.harmony.chord_at(tq).to_string();
            let bad = |e: bf_core::harmony::HarmonyError| MusicError::new(H, e.to_string());
            let pcs = ctx.harmony.chord_pcs(&ch).map_err(bad)?.to_vec();
            let tones: Vec<i32> = ctx
                .harmony
                .scale_notes(60, 79, Some(&ch))
                .map_err(bad)?
                .into_iter()
                .filter(|m| pcs.contains(&m.rem_euclid(12)))
                .collect();
            let root = *tones
                .first()
                .ok_or_else(|| MusicError::new(H, format!("the chord {ch:?} has no tone from C4 to G5")))?;
            let pan = clip(event_number(H, e, 5)? / 3.5, -0.6, 0.6);
            let (mut g, mut lead) = (1.0, 0.0);
            for &m in tones.iter().take(3) {
                let x = self.stab(ctx, m, 0.95, 0.12)?;
                let (sig, g_, lead_) = self.colour(ctx, &x, e.t, 1.0, &self.colour_args())?;
                (g, lead) = (g_, lead_);
                mix.place(&bus, &mono(sig), tq, 0.16 * g, pan);
            }
            if lead > 0.04 {
                // the leader: the root doubled an octave down
                let x = self.stab(ctx, root - 12, 0.8, 0.12)?;
                let sig = self.colour(ctx, &x, e.t, 1.0, &self.colour_args())?.0;
                mix.place(&bus, &mono(sig), tq, 0.12 * g, pan);
            }
            let thud = synth::thud(0.3, 130.0, 55.0, noise_seed(H, e.t * 7.0)?);
            mix.place("fx", &mono(thud), tq, 0.14, 0.0);
            let seed = noise_seed(H, e.t * 100.0)? % 50;
            mix.place("fx", &mono(tick(seed)), tq, 0.08, pan);
        }
        // its abilities and its slam
        let cfg = ctx.cfg[k];
        let rewind = [&cfg.unlock1, &cfg.unlock2, &cfg.low]
            .into_iter()
            .find(|a| a.kind == AbilityKind::Rewind);
        for e in events {
            let t = e.t;
            if t > t_ko + 0.01 {
                break;
            }
            if e.who() != Some(k) {
                continue;
            }
            if e.kind == "ability" {
                let tq = ctx.q16(t);
                if event_is_str(H, e, 3, &cfg.unlock1.name)? && event_is_str(H, e, 4, "haste")? {
                    // SCRATCH: a scribble
                    mix.place(&bus, &mono(scratch(&src, Kind::Scrib, 0.4, 3)), tq, 0.20, -0.2);
                } else if let (true, Some(ab)) = (event_is_str(H, e, 4, "rewind")?, rewind) {
                    // REWIND: a backspin, the tape whirring backwards
                    let zip = ab
                        .get("zip_beats")
                        .ok_or_else(|| MusicError::new(H, "REWIND has no zip_beats"))?;
                    let d = grid.beat * zip;
                    mix.place(
                        &bus,
                        &mono(scratch(&src, Kind::Backspin, r3(d + 0.1), 7)),
                        t,
                        0.40,
                        -0.1,
                    );
                    mix.place("fx", &mono(rewind_whirr(d + 0.1)), t, 0.20, 0.0);
                } else if event_is_str(H, e, 4, "slam")? {
                    // BASS DROP: the record falls
                    let w = synth::whoosh(2.0 * grid.beat, 6000.0, 300.0, false, 6);
                    mix.place("fx", &mono(w), tq, 0.12, 0.0);
                }
            } else if e.kind == "slam" {
                mix.place("bass", &mono(sub_drop(1.3, 95.0, 28.0)), t, 0.62, 0.0);
                mix.place("fx", &mono(synth::thud(0.6, 90.0, 32.0, 7)), t, 0.50, 0.0);
                mix.place(&bus, &mono(self.stab(ctx, 53, 1.0, 0.3)?), t, 0.24, 0.0);
                mix.place("fx", &mono(synth::crash(1.5)), t, 0.10, 0.0);
            }
        }
        Ok(())
    }

    /// The decks run down: a baby scratch, tape-stopped.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let x = scratch(&src(), Kind::Baby, 1.2, 9);
        let st: Vec<[f64; 2]> = x.iter().map(|&v| [v, v]).collect();
        let stopped: Vec<f64> = synth::tape_stop(&st, 0.0, 1.0).iter().map(|v| v[0]).collect();
        mix.place(&self.bus(ctx), &mono(stopped), ctx.show.t_hs, 0.3, 0.0);
        Ok(())
    }

    /// A backspin into a stab chord on "(WINNER) WINS".
    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        mix.place(
            "solo",
            &mono(scratch(&src(), Kind::Backspin, 0.6, 2)),
            tw - 0.6,
            0.30,
            0.0,
        );
        for m in [53, 60, 65, 68] {
            mix.place("solo", &mono(self.stab(ctx, m, 1.0, 0.4)?), tw, 0.20, 0.0);
        }
        Ok(())
    }

    /// A stab a solo note (the fight file's solos hold plain notes).
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let beat = ctx.show.grid.beat;
        for (bt, m, ln) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * beat;
            let x = self.stab(ctx, m, 0.95, py_min(0.3, ln * beat))?;
            mix.place("solo", &mono(x), ts, 0.16, 0.0);
        }
        Ok(())
    }

    /// Its pieces fly home as a rising run of stabs.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let run = [65, 68, 72, 75, 77, 80, 84, 89];
        for (j, m) in run.into_iter().enumerate() {
            let at = grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64;
            let tj = ctx.q16(at);
            mix.place(&bus, &mono(self.stab(ctx, m - 12, 0.7, 0.12)?), tj, 0.16, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_deck_curves_are_its_modules() {
        // a baby scratch over a beat at 130 BPM: two cycles, up 0.13 and back
        let t: Vec<f64> = (0..5).map(|i| f64::from(i) * BEAT_S / 4.0).collect();
        let (q, g) = qmotion(Kind::Baby, &t, BEAT_S);
        assert!(q[0].abs() < 1e-12 && (q[1] - 0.13).abs() < 1e-12 && q[2].abs() < 1e-12);
        assert_eq!(g, vec![1.0; 5]);
        // a backspin: -1.7 at the end
        let (q, _) = qmotion(Kind::Backspin, &[0.0, 0.5, 1.0], 1.0);
        assert_eq!((q[0], q[2]), (-0.0, -1.7));
        assert_eq!(Kind::named("back"), Some(Kind::Backspin));
        assert_eq!(Kind::named("stab"), None);
    }

    #[test]
    fn the_gradient_is_numpys() {
        assert_eq!(gradient(&[1.0, 2.0, 4.0, 7.0], 0.5), vec![2.0, 3.0, 5.0, 6.0]);
    }
}
