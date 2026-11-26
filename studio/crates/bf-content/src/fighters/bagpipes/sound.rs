//! BAGPIPES's part in the score: the `Sound` class of its reference kit module
//! (`tools/studio_export/older/fighters/bagpipes.py`, #30 v3's `audio.py` BAGPIPES part moved with its names).
//!
//! The Highland pipe's own synth (`voice.rs`: `wt`, `chanter`, the drones), no Pixel Pitch voice call. The drone is
//! the bag's air (A2 + two A3 drones, whole cycles in the file): low at rest, swelling as the bag fills, souring as
//! its HP drops, stopping at its K.O.; each flight squeals up about an octave; a landed flight is a grace-note squeal
//! and a thud, a body slam a drone blat, a stomp and a thud; WAR CRY skirls up to high A, HIGHLAND FLING throws a reel
//! flourish (and the choir's "oh"), BAG BURST pops and deflates; it falls apart deflating; its stinger and solo on the
//! chanter; its pieces fly home as a chanter run.
//!
//! Buses (ruling M4-R9): its own `bagpipes` (#30's `chanter`) and `drone`, its own `bagpipes stomp` and `bagpipes
//! choir` (#30's bed buses `stomp` and `choir`, kept apart so no theme's bed level reaches them), and the shared
//! `fx` and `solo`. Its own noise: every random draw is a generator of its own seed; the series' stream is untouched.

use bf_core::dmath::{self, PI};
use bf_core::event::{Event, Value};
use bf_core::py::{py_min, round_half_even, round_nd};
use bf_core::recording::Array;
use bf_picture::scene::R_BALL as R;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, noise_seed};
use bf_sound::synth::{self, FILTER_O, THUD_SEED, WHOOSH_SEED, clip};
use bf_sound::traits::{Bed, Played, Sound};
use bf_sound::voice::pp;
use bf_sound::voice::real::{self, RealNotes};

use super::voice::{self as bag, GHB_DRONES, HIGHLAND, look_up};

/// The name the part's errors carry.
const H: &str = "BAGPIPES";
/// Its copy of #30's bed bus `stomp`.
pub const STOMP: &str = "bagpipes stomp";
/// Its copy of #30's bed bus `choir`.
pub const CHOIR: &str = "bagpipes choir";

/// `int(seconds * SR)` at 48 kHz.
fn n_of(seconds: f64) -> usize {
    (seconds * SRF) as usize
}

/// `_fit(x, cap)`: `x` cut or padded to `cap` seconds, the last 30 ms (or a third) faded out.
fn fit(x: &[f64], cap: f64) -> Vec<f64> {
    let n = n_of(cap);
    let mut y: Vec<f64> = x.iter().take(n).copied().collect();
    y.resize(n, 0.0);
    let fd = (y.len() / 3).min(n_of(0.03));
    if fd > 0 {
        let fade = dsp::linspace(1.0, 0.0, fd);
        let at = y.len() - fd;
        for (v, w) in y[at..].iter_mut().zip(&fade) {
            *v *= w;
        }
    }
    y
}

/// The chanter's level (`_G['ch']`): `min(6.0, 0.16 / (rms + 1e-6))` of its C5, 0.2 s at 0.85, high-passed at
/// 120 Hz, at 48 kHz.
fn chanter_gain() -> f64 {
    let r = pp::to_48k(&bag::chanter(pp::midi_hz(72.0), 0.2, 0.85, &HIGHLAND, None, 1));
    let h = synth::hp(&r, 120.0, FILTER_O);
    let sq: Vec<f64> = h.iter().map(|v| v * v).collect();
    py_min(6.0, 0.16 / (dsp::mean(&sq).sqrt() + 1e-6))
}

/// `chanter(m, dur, vel, grace)`: one chanter note at 48 kHz (the Highland pipe's own synth), optionally with the
/// high-G grace squeal in front, level-matched.
pub fn chanter(m: f64, dur: f64, vel: f64, grace: bool) -> Vec<f64> {
    let g = if grace { Some(pp::midi_hz(79.0)) } else { None };
    // seed=int(m) * 7
    let x = bag::chanter(pp::midi_hz(m), dur, vel, &HIGHLAND, g, (m as i64 * 7) as u64);
    let x = fit(&pp::to_48k(&x), dur + 0.12);
    let gain = chanter_gain();
    x.into_iter().map(|v| v * gain).collect()
}

/// [`chanter`]'s note as the Composed arrangement plays it (sound upgrade, stage 3, S1): reworked toward the real
/// bagpipe ([`REWORK`](super::voice::REWORK)), the `variant`th of its repeats ([`real::rework`]).
pub fn chanter_real(m: f64, dur: f64, vel: f64, grace: bool, variant: u64) -> Vec<f64> {
    real::rework(
        &chanter(m, dur, vel, grace),
        m,
        dur,
        SRF,
        &super::voice::REWORK,
        variant,
    )
}

/// `drone_layer(ms_amps, n)`: the Highland drones as wavetables at frequencies nudged (< 0.01 Hz) to whole cycles
/// in `n` samples, so the drone at the file's last sample flows into its first; 48 kHz.
pub fn drone_layer(ms_amps: &[(f64, f64, f64)], n: usize) -> Vec<f64> {
    let dc = &GHB_DRONES;
    let l = n as f64 / SRF;
    let mut x = vec![0.0; n];
    for (j, &(m, a, cents)) in ms_amps.iter().enumerate() {
        let f = pp::midi_hz(m) * dmath::pow(2.0, cents / 1200.0);
        let fq = round_half_even(f * l) / l;
        let tb = bag::wt(f, dc.tilt, dc.form, dc.fbw, dc.fgain, dc.odd);
        let off = 0.137 * (j + 1) as f64;
        for (i, o) in x.iter_mut().enumerate() {
            let t = i as f64 / SRF;
            // ph = (fq * t + 0.137 * (j + 1)) % 1.0
            let ph = bf_core::py::fmod_py(fq * t + off, 1.0);
            *o += a * look_up(&tb, ph);
        }
    }
    synth::lp(&x, dc.lpf, FILTER_O).into_iter().map(|v| v * 0.3).collect()
}

/// `stomp(vel, seed)`: a boot on a wooden deck (#30's bed synth: a body slam stamps it).
pub fn stomp(vel: f64, seed: u64) -> Vec<f64> {
    let n = n_of(0.35);
    let t = synth::tt(n);
    let f: Vec<f64> = t.iter().map(|&t| 52.0 + 70.0 * dmath::exp(-t / 0.03)).collect();
    let cf = dsp::cumsum(&f);
    let knock = synth::bp(&synth::noise(seed, n), 180.0, 1400.0, FILTER_O);
    (0..n)
        .map(|i| {
            let body = dmath::sin(2.0 * PI * cf[i] / SRF) * dmath::exp(-t[i] / 0.13);
            let kn = knock[i] * dmath::exp(-t[i] / 0.022);
            (dmath::np_tanh(1.8 * body) * 0.9 + 0.55 * kn) * vel
        })
        .collect()
}

/// `choir(ms, dur, att, rel, vowel)` with vowel 'o' (#30's bed synth: HIGHLAND FLING's "oh").
pub fn choir_o(ms: &[f64], dur: f64, att: f64, rel: f64) -> Vec<f64> {
    let n = n_of(dur + rel);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    for &m in ms {
        let f = synth::mtof(m);
        for (d, ph0) in [(-0.004, 0.1), (0.0, 0.5), (0.005, 0.8)] {
            let rate = 4.6 + 3.0 * d * 100.0;
            let fr: Vec<f64> = t
                .iter()
                .map(|&t| f * (1.0 + d) * (1.0 + 0.004 * dmath::sin(2.0 * PI * rate * t + ph0 * 6.0)))
                .collect();
            let cs = dsp::cumsum(&fr);
            for (o, &c) in x.iter_mut().zip(&cs) {
                let ph = bf_core::py::fmod_py(c / SRF + ph0, 1.0);
                *o += 2.0 * ph - 1.0;
            }
        }
    }
    let div = (3 * ms.len().max(1)) as f64;
    for v in &mut x {
        *v /= div;
    }
    let a = synth::bp(&x, 300.0, 650.0, FILTER_O);
    let b = synth::bp(&x, 750.0, 1250.0, FILTER_O);
    let y: Vec<f64> = a.iter().zip(&b).map(|(a, b)| a * 1.0 + b * 0.45).collect();
    synth::lp(&y, 1600.0, FILTER_O)
        .into_iter()
        .zip(&t)
        .map(|(v, &t)| v * (clip(t / att, 0.0, 1.0) * clip((dur + rel - t) / rel, 0.0, 1.0)))
        .collect()
}

/// `deflate(dur, f0, f1, seed)`: the bag giving out: the chanter's reed sliding down and choking, the air hissing
/// out.
pub fn deflate(dur: f64, f0: f64, f1: f64, seed: u64) -> Vec<f64> {
    let n = n_of(dur);
    let t = synth::tt(n);
    let tn = &HIGHLAND;
    let f: Vec<f64> = t
        .iter()
        .map(|&t| {
            let u = t / dur;
            f0 * dmath::pow(f1 / f0, dmath::pow(u, 0.7)) * (1.0 + 0.03 * dmath::sin(2.0 * PI * 7.0 * t))
        })
        .collect();
    let tb = bag::wt(f0, tn.tilt, tn.form, tn.fbw, tn.fgain, tn.odd);
    let ph = dsp::cumsum(&f);
    let x: Vec<f64> = (0..n)
        .map(|i| look_up(&tb, ph[i] / SRF) * dmath::pow(1.0 - t[i] / dur, 1.2))
        .collect();
    let air = synth::bp(&synth::noise(seed, n), 1500.0, 7000.0, FILTER_O);
    synth::lp(&x, 4500.0, FILTER_O)
        .into_iter()
        .zip(air.iter().zip(&t))
        .map(|(v, (a, &t))| v + a * dmath::exp(-(t / dur) * 2.5) * 0.5)
        .collect()
}

/// `squeal(dur, f0, t0)`: the released balloon: the chanter's reed squealing up about an octave over the flight,
/// wobbling hard at every zig-zag, the air hissing out as it empties.
pub fn squeal(dur: f64, f0: f64, t0: f64) -> Result<Vec<f64>, MusicError> {
    let n = n_of(dur + 0.06);
    let t = synth::tt(n);
    let tn = &HIGHLAND;
    let u: Vec<f64> = t.iter().map(|&t| clip(t / dur, 0.0, 1.0)).collect();
    let f: Vec<f64> = (0..n)
        .map(|i| {
            f0 * dmath::pow(2.0, 0.95 * dmath::pow(u[i], 0.8))
                * (1.0 + 0.035 * dmath::sin(2.0 * PI * (7.0 + 9.0 * u[i]) * t[i]))
        })
        .collect();
    let tb = bag::wt(f0, tn.tilt, tn.form, tn.fbw, tn.fgain, tn.odd);
    let ph = dsp::cumsum(&f);
    let env: Vec<f64> = (0..n)
        .map(|i| clip(t[i] / 0.02, 0.0, 1.0) * (1.0 - 0.55 * u[i]) * clip((dur + 0.05 - t[i]) / 0.05, 0.0, 1.0))
        .collect();
    let xe: Vec<f64> = (0..n).map(|i| look_up(&tb, ph[i] / SRF) * env[i]).collect();
    // air = A.bp(np.random.default_rng(int(t0 * 100)).normal(0, 1, n), 2000, 8000) * 0.35 * u
    let air = synth::bp(&synth::noise(noise_seed(H, t0 * 100.0)?, n), 2000.0, 8000.0, FILTER_O);
    Ok(synth::lp(&xe, 5200.0, FILTER_O)
        .into_iter()
        .enumerate()
        .map(|(i, v)| v + air[i] * 0.35 * u[i] * env[i])
        .collect())
}

/// A BAG INFLATE cycle as the part rebuilds it from the run (#30 render.py `_cycles`).
#[derive(Clone, Copy, Debug)]
struct Cycle {
    t0: f64,
    dur_i: f64,
    t_rel: f64,
    t_land: f64,
}

/// BAGPIPES's part.
#[derive(Debug)]
pub struct BagpipesSound {
    k: usize,
    /// The Composed arrangement's chanter notes, reworked toward the real bagpipe (sound upgrade, stage 3, S1).
    real: RealNotes,
}

impl BagpipesSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> BagpipesSound {
        BagpipesSound {
            k,
            real: RealNotes::default(),
        }
    }

    /// A chanter note as the arrangement plays it: [`chanter`]'s own under Classic; under Composed (sound upgrade,
    /// stage 3, S1) reworked toward the real bagpipe ([`chanter_real`]), its repeats taking turns.
    fn ch(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64, grace: bool) -> Vec<f64> {
        if !ctx.composed_on() {
            return chanter(m, dur, vel, grace);
        }
        let [a, b, c] = [m, dur, vel].map(|v| (round_nd(v, 3) + 0.0).to_bits());
        self.real
            .get(&[a, b, c, u64::from(grace)], |v| chanter_real(m, dur, vel, grace, v))
            .to_vec()
    }

    /// `_cycles()`: its cycles from the run's events, a cycle the K.O. interrupted ending 10 s after it.
    fn cycles(&self, ctx: &MusicCtx) -> Vec<Cycle> {
        let mut out: Vec<(f64, f64, Option<f64>, Option<f64>)> = Vec::new();
        for e in &ctx.rec.events {
            if e.who() != Some(self.k) {
                continue;
            }
            match e.kind.as_str() {
                "inflate" => out.push((e.t, e.f(4), None, None)),
                "release" => {
                    if let Some(c) = out.last_mut() {
                        c.2 = Some(e.t);
                    }
                }
                "land" => {
                    if let Some(c) = out.last_mut() {
                        c.3 = Some(e.t);
                    }
                }
                _ => {}
            }
        }
        let t_ko = ctx.show.t_ko;
        out.into_iter()
            .map(|(t0, dur_i, r, l)| Cycle {
                t0,
                dur_i,
                t_rel: r.unwrap_or(t_ko + 10.0),
                t_land: l.unwrap_or(t_ko + 10.0),
            })
            .collect()
    }

    /// `melody_at(t)`: the note of its line the chanter sounds at `t` (a flight's squeal, a landed flight). Under
    /// Classic it is the line's note (`T.line_at`); under the Composed arrangement the context keeps what was
    /// played, and a moving melody style never hands back the note before it (sound upgrade).
    fn melody_at(&self, ctx: &MusicCtx, t: f64) -> Result<f64, MusicError> {
        match ctx.play_line(self.name(ctx), t)? {
            Some(m) => Ok(f64::from(m)),
            None => Err(MusicError::new(
                H,
                format!("its line has a rest at {t} s (the chanter has no rest)"),
            )),
        }
    }

    /// `colour(x, t, drive_k)`: its tone at `t` (the base colour arguments).
    fn tone(&self, ctx: &MusicCtx, x: &[f64], t: f64, drive_k: f64) -> Result<(Vec<f64>, f64, f64), MusicError> {
        self.colour(ctx, x, t, drive_k, &self.colour_args())
    }

    /// `RUN['RAD'][:, k]`: its radius at every recorded frame.
    fn radii(&self, ctx: &MusicCtx) -> Result<Vec<f64>, MusicError> {
        let Some(Array::Nested(Value::List(frames))) = ctx.rec.arrays.get("RAD") else {
            return Err(MusicError::new(H, "the recording has no RAD (an inflater's radius)"));
        };
        frames
            .iter()
            .map(|row| match row {
                Value::List(v) => match v.get(self.k) {
                    Some(Value::Float(f)) => Ok(*f),
                    Some(Value::Int(n)) => Ok(*n as f64),
                    _ => Err(MusicError::new(H, "a RAD row without its radius")),
                },
                _ => Err(MusicError::new(H, "a RAD row that is not a list")),
            })
            .collect()
    }
}

/// `e[7] in kinds` for a `hit`.
fn how(e: &Event) -> &str {
    match e.fields.get(5) {
        Some(Value::Str(s)) => s,
        _ => "",
    }
}

impl Sound for BagpipesSound {
    fn k(&self) -> usize {
        self.k
    }

    /// #30's chanter bus (`BUS_GAIN`, its reverb send).
    fn bus_gain(&self) -> f64 {
        1.0
    }

    fn send(&self) -> f64 {
        0.45
    }

    /// A drone blast (all three drones + the chanter's high A with its grace note).
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let blast: Vec<[f64; 2]> = pp::to_48k(&bag::drone_voice(0.75))
            .into_iter()
            .map(|v| {
                let v = v * 3.2;
                [v, v]
            })
            .collect();
        mix.place("drone", &Sig::Stereo(blast), t, 0.55, 0.0);
        mix.place(&bus, &Sig::Mono(self.ch(ctx, 81.0, 0.62, 1.0, true)), t, 0.46, 0.0);
        mix.place(&bus, &Sig::Mono(self.ch(ctx, 69.0, 0.62, 0.9, false)), t, 0.26, 0.0);
        Ok(())
    }

    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        for (j, &(tc, _)) in ctx.show.grid.count[..3].iter().enumerate() {
            let m = [71.0, 74.0, 76.0][j];
            mix.place(&bus, &Sig::Mono(self.ch(ctx, m, 0.30, 0.9, true)), tc, 0.40, 0.0);
        }
        Ok(())
    }

    /// The drone is the bag's air; the flight squeals; its hits and abilities (#30 `main()`, in its order).
    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (k, bus) = (self.k, self.bus(ctx));
        let name = self.name(ctx);
        // sound upgrade (M5): under Composed the chanter sounds one note at a time, so a landed flight's note ends
        // the squeal of the flight it lands in
        let one = ctx.one_voice(name);
        let grid = &ctx.show.grid;
        let (t_ko, lose) = (ctx.show.t_ko, ctx.show.lose);
        let rad = self.radii(ctx)?;
        let n = bf_sound::NA;
        let tt: Vec<f64> = (0..n).map(|i| i as f64 / SRF).collect();
        let drone = drone_layer(&[(57.0, 1.0, -1.5), (57.0, 1.0, 1.5), (45.0, 1.1, 0.0)], n);
        let tframes: Vec<f64> = (0..rad.len())
            .map(|i| grid.t_fight + i as f64 / f64::from(grid.fps))
            .collect();
        // how full the bag is (0 at rest .. 1)
        let full: Vec<f64> = rad.iter().map(|&r| clip((r / R - 1.0) / 0.6, 0.0, 1.0)).collect();
        let full_t = dsp::interp_lr(&tt, &tframes, &full, Some(0.0), Some(0.0));
        let full_t = dsp::lfilter(&[0.003], &[1.0, -0.997], &full_t);
        // the drone stops at a bagpipes K.O. and comes back with the reassembly
        let alive = if lose == k {
            let a: Vec<f64> = tt
                .iter()
                .map(|&t| {
                    let on = if t < t_ko { 1.0 } else { 0.0 };
                    let back = if t >= grid.t_glide { 1.0 } else { 0.0 };
                    on + clip((t - grid.t_glide) / 0.6, 0.0, 1.0) * back
                })
                .collect();
            dsp::lfilter(&[0.002], &[1.0, -0.998], &a)
        } else {
            vec![1.0; n]
        };
        let xs = dsp::linspace(0.0, grid.dur, 400);
        let hp_p: Vec<f64> = xs.iter().map(|&x| ctx.hp_at(k, x) / 100.0).collect();
        let hp_curve = dsp::interp(&tt, &xs, &hp_p);
        let not_lost = if lose != k { 1.0 } else { 0.0 };
        let sig: Vec<f64> = (0..n)
            .map(|i| {
                let t = tt[i];
                let ko = if t >= t_ko { 1.0 } else { 0.0 };
                let champ = ko * clip((grid.t_rev - t) / (grid.t_rev - grid.t_glide), 0.0, 1.0) * not_lost;
                let g = (0.11 + 0.10 * champ + 0.42 * full_t[i]) * alive[i];
                let sour = dmath::sin(2.0 * PI * 0.37 * t) * 0.5 + 0.5;
                drone[i] * g * (1.0 - 0.35 * (1.0 - hp_curve[i]) * sour)
            })
            .collect();
        {
            let b = mix.bus_mut("drone");
            for (frame, &v) in b.iter_mut().zip(&sig) {
                frame[0] += v * 0.95;
                frame[1] += v * 1.0;
            }
        }
        for c in self.cycles(ctx) {
            if c.t_rel > t_ko {
                continue;
            }
            // the bag filling
            let w: Vec<f64> = synth::whoosh(c.dur_i, 200.0, 1400.0, true, WHOOSH_SEED)
                .into_iter()
                .map(|v| v * 0.6)
                .collect();
            mix.place("fx", &Sig::Mono(w), c.t0, 0.05, 0.0);
            let t1 = py_min(c.t_land, t_ko);
            let d = t1 - c.t_rel;
            if d <= 0.05 {
                continue;
            }
            let m0 = self.melody_at(ctx, ctx.q16(c.t_rel))?;
            let s = squeal(d, pp::midi_hz(m0), c.t_rel)?;
            let (sg, gg, _) = self.tone(ctx, &s, c.t_rel, 1.0)?;
            // (sound upgrade, M3: a top note of the chanter plays softer under Composed; 1 under Classic)
            let soft = ctx.soften(name, m0 as i32);
            mix.note(&bus, &Sig::Mono(sg), c.t_rel, 0.24 * gg * soft, -0.15, one);
        }
        for e in &ctx.rec.events {
            let t = e.t;
            if t > t_ko + 0.01 {
                break;
            }
            if e.kind == "hit" && e.who() == Some(k) && matches!(how(e), "flight" | "bodyslam") {
                let tq = ctx.q16(t);
                let pan = clip(e.f(5) / 3.5, -0.5, 0.5);
                if how(e) == "flight" {
                    // a squealing grace note + the hit
                    let ml = self.melody_at(ctx, tq)?;
                    let m = if ml < 72.0 { ml + 12.0 } else { ml };
                    let (sig, gg, _) = self.tone(ctx, &self.ch(ctx, m, 0.22, 1.0, true), tq, 1.0)?;
                    let soft = ctx.soften(name, m as i32);
                    mix.note(&bus, &Sig::Mono(sig), tq, 0.34 * gg * soft, pan, one);
                    let th = synth::thud(0.32, 150.0, 55.0, noise_seed(H, t * 7.0)?);
                    mix.place("fx", &Sig::Mono(th), tq, 0.18, 0.0);
                } else {
                    // a body slam: a drone blat + a stomp
                    // (sound upgrade: under Composed the blat is the written line's note there, brought down by
                    // octaves to the blat's own depth, the octave around the drones' A3; A3 itself on a rest)
                    let low = if ctx.composed_on() {
                        ctx.play_line(name, tq)?.map_or(57.0, |mut m| {
                            while m > 62 {
                                m -= 12;
                            }
                            while m < 51 {
                                m += 12;
                            }
                            f64::from(m)
                        })
                    } else {
                        57.0
                    };
                    let blat = self.tone(ctx, &self.ch(ctx, low, 0.3, 1.0, false), tq, 1.0)?.0;
                    mix.note(&bus, &Sig::Mono(synth::lp(&blat, 1800.0, FILTER_O)), tq, 0.30, pan, one);
                    mix.place(STOMP, &Sig::Mono(stomp(1.0, noise_seed(H, t * 3.0)?)), tq, 0.40, 0.0);
                    let th = synth::thud(0.4, 110.0, 40.0, noise_seed(H, t * 5.0)?);
                    mix.place("fx", &Sig::Mono(th), tq, 0.22, 0.0);
                }
            }
        }
        let (s16, beat) = (grid.s16, grid.beat);
        for e in &ctx.rec.events {
            let t = e.t;
            if t > t_ko + 0.01 {
                break;
            }
            if e.who() != Some(k) {
                continue;
            }
            if e.kind == "hit" && matches!(how(e), "shockwave" | "wave") {
                mix.place(
                    "fx",
                    &Sig::Mono(synth::thud(0.45, 110.0, 40.0, THUD_SEED)),
                    ctx.q16(t),
                    0.30,
                    0.0,
                );
            } else if e.kind == "ability" {
                let tq = ctx.q16(t);
                match e.s(3) {
                    "WAR CRY" => {
                        // the skirl: a run up to high A, held
                        for (j, m) in [69.0, 71.0, 72.0, 74.0, 76.0, 79.0].into_iter().enumerate() {
                            let x = self.tone(ctx, &self.ch(ctx, m, s16 * 0.9, 1.0, j == 0), t, 1.0)?.0;
                            mix.place(&bus, &Sig::Mono(x), tq + j as f64 * s16 / 2.0, 0.26, 0.0);
                        }
                        let x = self.tone(ctx, &self.ch(ctx, 81.0, 2.0 * beat, 1.0, true), t, 1.0)?.0;
                        mix.place(&bus, &Sig::Mono(x), tq + 3.0 * s16, 0.34, 0.0);
                    }
                    "HIGHLAND FLING" => {
                        // a reel flourish
                        for (j, m) in [76.0, 74.0, 72.0, 74.0, 76.0, 79.0, 81.0, 79.0].into_iter().enumerate() {
                            let x = self
                                .tone(ctx, &self.ch(ctx, m, s16 * 0.85, 0.95, j % 4 == 0), t, 1.0)?
                                .0;
                            mix.place(&bus, &Sig::Mono(x), tq + j as f64 * s16, 0.22, 0.0);
                        }
                        mix.place(
                            CHOIR,
                            &Sig::Mono(choir_o(&[57.0, 60.0, 64.0], 0.18, 0.01, 0.12)),
                            tq,
                            0.14,
                            0.0,
                        );
                    }
                    "BAG BURST" => {
                        // the bag pops, then deflates
                        let np_ = n_of(0.12);
                        let tp = synth::tt(np_);
                        let pop: Vec<f64> = synth::bp(&synth::noise(29, np_), 300.0, 9000.0, FILTER_O)
                            .into_iter()
                            .zip(&tp)
                            .map(|(v, &t)| v * dmath::exp(-t / 0.02))
                            .collect();
                        mix.place("fx", &Sig::Mono(pop), tq, 0.55, 0.0);
                        let df = deflate(1.8, pp::midi_hz(81.0), 160.0, 21);
                        mix.place(&bus, &Sig::Mono(df), tq + 0.03, 0.34, 0.0);
                        mix.place(
                            "fx",
                            &Sig::Mono(synth::thud(0.7, 100.0, 32.0, THUD_SEED)),
                            tq,
                            0.45,
                            0.0,
                        );
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// The bag gives out: the chanter sliding down and choking.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let x = deflate(2.2, pp::midi_hz(76.0), 120.0, 27);
        mix.place(&bus, &Sig::Mono(x), ctx.show.t_hs, 0.36, 0.0);
        Ok(())
    }

    /// On "(WINNER) WINS" at the kit's `tw` (ruling M4-R11).
    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        mix.place("solo", &Sig::Mono(self.ch(ctx, 81.0, 1.2, 1.0, true)), tw, 0.36, 0.0);
        mix.place("solo", &Sig::Mono(self.ch(ctx, 69.0, 1.2, 0.8, false)), tw, 0.18, 0.0);
        Ok(())
    }

    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts >= grid.t_reassemble {
                break;
            }
            let dur = d * grid.beat * 0.95;
            mix.place(
                "solo",
                &Sig::Mono(self.ch(ctx, f64::from(m), dur, 0.95, d >= 0.75)),
                ts,
                0.46,
                0.0,
            );
        }
        Ok(())
    }

    /// Its pieces fly home as a rising chanter run.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let run = [57.0, 60.0, 62.0, 64.0, 67.0, 69.0, 72.0, 76.0];
        let n = run.len() as f64;
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / n);
            let x = self.ch(ctx, if m < 67.0 { m + 12.0 } else { m }, grid.s16 * 1.6, 0.6, false);
            mix.place(&bus, &Sig::Mono(x), tj, 0.16, 0.0);
        }
        Ok(())
    }
}
