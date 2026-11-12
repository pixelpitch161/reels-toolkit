//! HORN's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/horn.py` (#34 v5's `audio.py`,
//! the horn's part only).
//!
//! Every call is a round, mellow horn note of its line (one a beat; thinned when it is hurt); every echo repeats the
//! call it trails, quieter and darker each time, panned toward its ghost; a landed blast is a brighter stab up the
//! chord with a soft thud; the leader gets a horn a sixth below; a new ghost is a copper chime. STOPPED HORN is the
//! buzzing stopped tone (the fight file's `HORN THEME` line if given, else its own line), HORN RIP rips up the
//! harmonic series of the bar's root, FINAL FLICK a horn fanfare over a spring "boing", a slide whistle and a
//! whoosh, then a snap (and on a hit a heavy thud and the bed's heavy hit); flung into the ropes, the heavy one
//! again. The heavy hit is the bed's `trailer_hit` when the bed has one, else its `boom`. The title blast is a
//! hunting call in fifths, the count-in two horns on the A chord; it falls apart as a falling wail over clattering
//! tubing; the stinger, the solo, the reassembly run up the tonic.
//!
//! Its voice draws no noise; the part's own noise (the snap, the slide whistle, the boing, the stopped tone's buzz)
//! comes from generators of its own seeds, as in Python.

use bf_core::dmath;
use bf_core::harmony::HarmonyError;
use bf_core::py::{imod_py, round_half_even, round_nd};
use bf_sound::SRF;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    CLANK_DECAYS, CLANK_DUR, ColourArgs, MusicCtx, MusicError, Samples, clank, event_int, event_is_str, event_number,
    gliss, noise_seed, vibrato,
};
use bf_sound::synth::{self, FILTER_O, THUD_SEED, WHOOSH_SEED, clip};
use bf_sound::traits::{Bed, BedHook, Played, Sound};

use crate::fighters::shared::hash::h01;

/// The name the part's errors carry.
const H: &str = "HORN";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// A rest where the part needs a note (Python would pass `None` on and raise).
fn rest(what: &str, t: f64) -> MusicError {
    MusicError::new(H, format!("{what} at {t:.3} s is a rest: the horn needs a note there"))
}

/// `snap(A, SR, seed)`: a finger snap, the glove's flick: a dry click with a little low body.
pub fn snap(seed: u64) -> Vec<f64> {
    // n = int(0.06 * SR)
    let n = (0.06 * SRF) as usize;
    let t = synth::tt(n);
    let x = synth::bp(&synth::noise(seed, n), 900.0, 4500.0, FILTER_O);
    (0..n)
        .map(|i| {
            let click = x[i] * dmath::exp(-t[i] / 0.008);
            click + 0.5 * dmath::sin(2.0 * dmath::PI * 180.0 * t[i]) * dmath::exp(-t[i] / 0.015)
        })
        .collect()
}

/// `np.cumsum(f)`: the running sum, in order.
fn cumsum(f: &[f64]) -> Vec<f64> {
    let mut s = 0.0;
    f.iter()
        .map(|&v| {
            s += v;
            s
        })
        .collect()
}

/// `slide_whistle(A, SR, dur, f0=520.0, f1=2100.0)`: FINAL FLICK's wind-up, a slide whistle rising over the beat.
///
/// # Errors
/// Where Python raises: `dur` is not a number of samples.
pub fn slide_whistle(dur: f64) -> Samples {
    let (f0, f1) = (520.0, 2100.0);
    let n = samples("slide_whistle", dur)?;
    let t = synth::tt(n);
    // f = f0 * (f1 / f0) ** (t / dur) * (1 + 0.012 * np.sin(2 * np.pi * 6 * t))
    let f: Vec<f64> = t
        .iter()
        .map(|&t| f0 * dmath::pow(f1 / f0, t / dur) * (1.0 + 0.012 * dmath::sin(2.0 * dmath::PI * 6.0 * t)))
        .collect();
    let ph = cumsum(&f);
    let air = synth::bp(&synth::noise(5, n), 1500.0, 6000.0, FILTER_O);
    Ok((0..n)
        .map(|i| {
            let x = dmath::sin(2.0 * dmath::PI * ph[i] / SRF) + 0.15 * air[i];
            x * clip(t[i] / 0.04, 0.0, 1.0) * clip((dur - t[i]) / 0.02, 0.0, 1.0)
        })
        .collect())
}

/// `boing(A, SR, seed, f0=190.0, dur=0.45)`: a spring, the glove bursting out of the bell (a wobbling, falling
/// twang).
pub fn boing(seed: u64) -> Vec<f64> {
    let (f0, dur) = (190.0, 0.45);
    let n = (dur * SRF) as usize;
    let t = synth::tt(n);
    // f = f0 * (1 + 0.35 * np.exp(-t / 0.12) * np.sin(2 * np.pi * 18 * t)) * (1 - 0.25 * t / dur)
    let f: Vec<f64> = t
        .iter()
        .map(|&t| {
            f0 * (1.0 + 0.35 * dmath::exp(-t / 0.12) * dmath::sin(2.0 * dmath::PI * 18.0 * t)) * (1.0 - 0.25 * t / dur)
        })
        .collect();
    let f2: Vec<f64> = f.iter().map(|&v| v * 2.01).collect();
    let (ph, ph2) = (cumsum(&f), cumsum(&f2));
    let zing = synth::bp(&synth::noise(seed, n), 2000.0, 7000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let x = dmath::sin(2.0 * dmath::PI * ph[i] / SRF) + 0.3 * dmath::sin(2.0 * dmath::PI * ph2[i] / SRF);
            x * dmath::exp(-t[i] / 0.16) + 0.25 * zing[i] * dmath::exp(-t[i] / 0.01)
        })
        .collect()
}

/// `stopped(A, x)`: stopped horn, the hand closes the bell: thin, buzzing, brassy (cuivre).
pub fn stopped(x: &[f64]) -> Vec<f64> {
    let y = synth::hp(x, 500.0, FILTER_O);
    // np.abs(y).max() + 1e-9
    let top = y
        .iter()
        .fold(f64::NEG_INFINITY, |m, v| if v.abs() > m { v.abs() } else { m })
        + 1e-9;
    let y: Vec<f64> = y.iter().map(|&v| dmath::np_tanh(3.5 * v / top) * 0.5).collect();
    let band = synth::bp(&y, 1800.0, 4200.0, FILTER_O);
    y.iter().zip(&band).map(|(a, b)| a + 0.6 * b).collect()
}

/// `int(dur * SR)` samples, where numpy refuses a negative count or a number that is none.
fn samples(helper: &'static str, dur: f64) -> Result<usize, MusicError> {
    let n = dur * SRF;
    if n.is_finite() && n > -1.0 {
        Ok(n as usize)
    } else {
        Err(MusicError::new(helper, format!("{dur} s is not a length")))
    }
}

/// `float(np.clip(x / 3.5, -0.6, 0.6))`: a pan toward where something is in the ring.
fn pan_of(x: f64) -> f64 {
    clip(x / 3.5, -0.6, 0.6)
}

/// HORN's part.
#[derive(Clone, Copy, Debug)]
pub struct HornSound {
    k: usize,
}

impl HornSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> HornSound {
        HornSound { k }
    }

    /// `horn(m, dur=0.3, vel=0.8, cap=None)`: one note in the HORN's Pixel Pitch voice: `M.note('HORN', m,
    /// round(dur, 3), cap or round(dur + 0.35, 2), round(vel, 1))`.
    ///
    /// # Errors
    /// As [`MusicCtx::note`].
    pub fn horn(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64, cap: Option<f64>) -> Samples {
        let cap = match cap {
            Some(c) if c != 0.0 => c,
            _ => round_nd(dur + 0.35, 2),
        };
        Ok(ctx
            .note("HORN", f64::from(m), round_nd(dur, 3), cap, round_nd(vel, 1))?
            .to_vec())
    }

    /// `tonic(lo=60)`: the key's tonic at or above MIDI `lo` (D: D4 = 62, #34's calls).
    pub fn tonic(&self, ctx: &MusicCtx) -> i32 {
        let lo = 60;
        lo + (ctx.harmony.key_pc - lo).rem_euclid(12)
    }

    /// `theme_at(t)`: STOPPED HORN's note: the fight file's `LINES['HORN THEME']` (#34's `HORN_LINE`, a note per 2
    /// beats) if it has one, else its own line.
    ///
    /// # Errors
    /// As [`MusicCtx::line_at`]; a `HORN THEME` line shorter than the loop (Python divides by zero).
    pub fn theme_at(&self, ctx: &MusicCtx, t: f64) -> Result<Option<i32>, MusicError> {
        match ctx.ff.music.lines.get("HORN THEME") {
            Some(th) if !th.is_empty() => {
                // th[T.loop_index(t, len(th) // len(T.LOOP))]
                let per_bar = th.len() / ctx.harmony.loop_.len().max(1);
                if per_bar == 0 || ctx.harmony.loop_.is_empty() {
                    return Err(MusicError::new(
                        H,
                        "the HORN THEME line has fewer notes than the chord loop has bars",
                    ));
                }
                Ok(th[ctx.harmony.loop_index(t, per_bar)])
            }
            _ => ctx.line_at(self.name(ctx), t),
        }
    }

    /// `_hit(mix, t, taiko_g, seed, braam_dur, braam_g)`: a heavy hit from the bed: the trailer's taiko (and braam),
    /// or another mood's `boom`.
    #[allow(clippy::too_many_arguments)] // the Python signature
    fn heavy(
        &self,
        ctx: &MusicCtx,
        bed: &dyn Bed,
        mix: &mut Mix,
        t: f64,
        taiko_g: f64,
        seed: i64,
        braam: Option<f64>,
        braam_g: f64,
    ) -> Played {
        if bed.has(BedHook::TrailerHit) {
            bed.trailer_hit(ctx, mix, t, taiko_g, seed, braam, braam_g)
        } else {
            bed.boom(ctx, mix, t, taiko_g)
        }
    }

    /// `self.colour(x, t, drive_k)` with its own `colour_args`.
    fn tone(&self, ctx: &MusicCtx, x: &[f64], t: f64, drive_k: f64) -> Result<(Vec<f64>, f64, f64), MusicError> {
        self.colour(ctx, x, t, drive_k, &self.colour_args())
    }
}

impl Sound for HornSound {
    fn k(&self) -> usize {
        self.k
    }

    /// A heavy hit calls `M.bed.boom` without asking when the bed has no `trailer_hit` (`horn.py` 709 to 712).
    fn bed_calls(&self) -> &'static [BedHook] {
        &[BedHook::Boom]
    }

    /// STOPPED HORN plays the fight file's `HORN THEME` line when it has one.
    fn extra_lines(&self) -> &'static [&'static str] {
        &["HORN THEME"]
    }

    /// #34's horn bus.
    fn bus_gain(&self) -> f64 {
        1.25
    }

    fn send(&self) -> f64 {
        0.55
    }

    /// The calls D4 to D5 and their horn harmonics.
    fn band(&self) -> (f64, f64) {
        (250.0, 2500.0)
    }

    /// #34's `colour()`: `cut0=70, cut_span=420, drive=4.5`.
    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 70.0,
            cut_span: 420.0,
            drive: 4.5,
            ..ColourArgs::default()
        }
    }

    /// A hunting call: two horns in fifths, a leap up a fifth (D4 to A4 in D), or the fight file's title hits.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let r = self.tonic(ctx);
        let notes: Vec<(f64, i32, f64)> = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(hits) if !hits.is_empty() => hits
                .iter()
                .enumerate()
                .map(|(j, &m)| (0.15 * j as f64, m, if j + 1 < hits.len() { 0.14 } else { 0.42 }))
                .collect(),
            _ => vec![(0.0, r, 0.14), (0.15, r + 7, 0.42)],
        };
        for (dt, m, d) in notes {
            mix.place(&bus, &Sig::Mono(self.horn(ctx, m, d, 1.0, None)?), t + dt, 0.32, 0.0);
            let low = if m - 7 >= 55 { m - 7 } else { m - 5 };
            mix.place(&bus, &Sig::Mono(self.horn(ctx, low, d, 0.85, None)?), t + dt, 0.18, 0.0);
        }
        Ok(())
    }

    /// Two horns on each count, the root and the third of the count-in's chord (#34: A4 and C#5 over A).
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let ch = ctx
            .harmony
            .chords
            .get(1)
            .ok_or_else(|| MusicError::new(H, "the episode has no bar 1 to count in on"))?;
        let r = ctx.harmony.root_above(ch, 65).map_err(bad)?;
        let pcs = ctx.harmony.chord_pcs(ch).map_err(bad)?;
        let (Some(&p0), Some(&p1)) = (pcs.first(), pcs.get(1)) else {
            return Err(MusicError::new(H, format!("the chord {ch:?} has no third")));
        };
        let third = r + (p1 - p0).rem_euclid(12);
        for (tc, _) in ctx.show.grid.count.iter().take(3) {
            mix.place(&bus, &Sig::Mono(self.horn(ctx, r, 0.22, 1.0, None)?), *tc, 0.22, 0.0);
            mix.place(
                &bus,
                &Sig::Mono(self.horn(ctx, third, 0.22, 0.9, None)?),
                *tc,
                0.14,
                0.0,
            );
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let name = self.name(ctx).to_string();
        let cfg = ctx.cfg[k];
        // call id -> (time, midi): the echoes repeat it
        let mut call_note: std::collections::BTreeMap<i64, (f64, i32)> = std::collections::BTreeMap::new();
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            let mine = e.who() == Some(k);
            match e.kind.as_str() {
                "echo" if mine => {
                    let (j, call) = (event_int(H, e, 3)?, event_int(H, e, 4)?);
                    let tq = ctx.q16(t);
                    let pan = pan_of(event_number(H, e, 5)?);
                    if j == 0 {
                        // the call: a round, mellow horn note on the beat
                        let m = ctx.line_at(&name, tq)?.ok_or_else(|| rest("its line", tq))?;
                        call_note.insert(call, (tq, m));
                        let hp = ctx.hp_at(k, t) / 100.0;
                        // sparse when hurt
                        if h01(&[round_half_even(tq / s16), 3.0]) < 0.5 * dmath::pow(1.0 - hp, 1.6) {
                            continue;
                        }
                        let x = self.horn(ctx, m, 0.55 * beat, 0.8, None)?;
                        let (sig, g, lead) = self.tone(ctx, &x, t, 1.0)?;
                        mix.place(&bus, &Sig::Mono(sig), tq, 0.24 * g, pan);
                        if lead > 0.04 {
                            // the leader: a horn a sixth below (its own harmony)
                            let under = ctx
                                .harmony
                                .chord_tones(m - 9, m - 3, ctx.harmony.chord_at(tq))
                                .map_err(bad)?;
                            if let Some(&low) = under.last() {
                                let x = self.horn(ctx, low, 0.55 * beat, 0.65, None)?;
                                let sig = self.tone(ctx, &x, t, 1.0)?.0;
                                mix.place(&bus, &Sig::Mono(sig), tq, 0.10 * g, pan);
                            }
                        }
                    } else if let Some(&(_, m)) = call_note.get(&call) {
                        // an echo: the same note, quieter and darker each time
                        let x = self.horn(ctx, m, 0.4 * beat, 0.6, None)?;
                        let x = synth::lp(&x, 2600.0 / dmath::pow(1.55, j as f64), FILTER_O);
                        let sig = self.tone(ctx, &x, t, 0.5)?.0;
                        mix.place(&bus, &Sig::Mono(sig), tq, 0.26 * dmath::pow(0.56, j as f64), pan);
                    }
                }
                "hit" if mine && event_is_str(H, e, 7, "echo")? => {
                    // a blast lands: a brighter stab up the chord
                    let tq = ctx.q16(t);
                    let ct = ctx.harmony.chord_tones(65, 79, ctx.harmony.chord_at(tq)).map_err(bad)?;
                    if ct.is_empty() {
                        return Err(MusicError::new(H, "the chord has no tone from F4 to G5"));
                    }
                    let sub = event_int(H, e, 8)?;
                    // ct[(int(e[8]) + int(_h01_34(t, 5) * 3)) % len(ct)]
                    let at = imod_py(sub + (h01(&[t, 5.0]) * 3.0) as i64, ct.len() as i64);
                    let m = ct[at as usize];
                    let pan = pan_of(event_number(H, e, 5)?);
                    let call = sub == 0;
                    let sig = self.tone(ctx, &self.horn(ctx, m, 0.1, 0.9, None)?, t, 1.0)?.0;
                    mix.place(&bus, &Sig::Mono(sig), tq, if call { 0.12 } else { 0.08 }, pan);
                    let thump = synth::thud(0.2, 160.0, 70.0, noise_seed(H, t * 11.0)?);
                    mix.place("fx", &Sig::Mono(thump), tq, if call { 0.06 } else { 0.04 }, pan);
                }
                "echo_grow" if mine => {
                    // a new ghost peels off: a soft copper shimmer
                    let tq = ctx.q16(t);
                    let m = ctx.line_at(&name, tq)?.ok_or_else(|| rest("its line", tq))?;
                    mix.place("fx", &Sig::Mono(synth::chime(f64::from(m + 24), 0.6)), tq, 0.025, 0.0);
                }
                "flick_wind" if mine => {
                    // FINAL FLICK: the glove bursts out, winds up
                    let tq = ctx.q16(t);
                    let t_hit = event_number(H, e, 3)?;
                    mix.place("fx", &Sig::Mono(boing(noise_seed(H, t * 7.0)?)), tq, 0.16, 0.0);
                    mix.place("fx", &Sig::Mono(slide_whistle(t_hit - tq)?), tq, 0.035, 0.0);
                    let swish = synth::whoosh(t_hit - tq, 300.0, 4000.0, true, WHOOSH_SEED);
                    mix.place("fx", &Sig::Mono(swish), tq, 0.08, 0.0);
                }
                "flick" if mine => {
                    // ... and flicks: THWACK
                    let ts = e.t;
                    mix.place("fx", &Sig::Mono(snap(noise_seed(H, ts * 13.0)?)), ts, 0.30, 0.0);
                    let swish = synth::whoosh(0.25, 800.0, 7000.0, false, WHOOSH_SEED);
                    mix.place("fx", &Sig::Mono(swish), ts - 0.02, 0.14, 0.0);
                    if event_int(H, e, 3)? != 0 {
                        mix.place("fx", &Sig::Mono(synth::thud(0.7, 140.0, 34.0, THUD_SEED)), ts, 0.6, 0.0);
                        self.heavy(ctx, bed, mix, ts, 0.55, 81, Some(0.9), 0.22)?;
                    }
                }
                "hit" if mine && event_is_str(H, e, 7, "flick_rope")? => {
                    // flung into the ropes: the heavy one
                    let ts = e.t;
                    let pan = pan_of(event_number(H, e, 5)?);
                    mix.place("fx", &Sig::Mono(synth::thud(0.6, 85.0, 36.0, THUD_SEED)), ts, 0.55, pan);
                    self.heavy(ctx, bed, mix, ts, 0.45, 83, None, 0.0)?;
                    mix.place(
                        "fx",
                        &Sig::Mono(clank(70.0, 12, CLANK_DUR, &CLANK_DECAYS)?),
                        ts,
                        0.08,
                        0.0,
                    );
                }
                "ability" if mine => {
                    let tq = ctx.q16(t);
                    let is = |name: &str| event_is_str(H, e, 3, name);
                    if is(&cfg.unlock1.name)? {
                        // STOPPED HORN: the brassy, buzzing stopped tone
                        let m = self.theme_at(ctx, tq)?.ok_or_else(|| rest("its theme", tq))?;
                        let x = stopped(&self.horn(ctx, m, beat * 1.5, 1.0, None)?);
                        mix.place(&bus, &Sig::Mono(x), tq, 0.18, 0.0);
                        let beats = cfg.unlock1.get("dur_beats").ok_or_else(|| {
                            MusicError::new(H, "STOPPED HORN's ability has no dur_beats (Python raises)")
                        })?;
                        let n = samples(H, beats * beat)?;
                        let air = synth::bp(&synth::noise(4, n), 3000.0, 9000.0, FILTER_O);
                        let env = synth::adsr(n, 0.05, 0.2, 0.5, 0.4);
                        let buzz: Vec<f64> = air.iter().zip(&env).map(|(a, b)| a * b).collect();
                        mix.place("fx", &Sig::Mono(buzz), tq, 0.02, 0.0);
                    } else if is(&cfg.unlock2.name)? {
                        // HORN RIP: up the harmonic series of the bar's root (partials 4 5 6 7 8)
                        let root = ctx.harmony.root_above(ctx.harmony.chord_at(tq), 45).map_err(bad)?;
                        let mut ms: Vec<i32> = [24, 28, 31, 34, 36].iter().map(|iv| root + iv).collect();
                        while ms.iter().copied().max().is_some_and(|top| top > 77) {
                            ms.iter_mut().for_each(|m| *m -= 12);
                        }
                        let last_j = ms.len() - 1;
                        for (j, &m) in ms.iter().enumerate() {
                            let last = j == last_j;
                            let x = self.horn(ctx, m, if last { 0.6 } else { 0.07 }, 0.95, None)?;
                            let x = if last { vibrato(&x, 0.4, 5.5, 0.15)? } else { x };
                            let x = gliss(&x, 2.0, 0.03)?;
                            let sig = self.tone(ctx, &x, t, 1.0)?.0;
                            let at = tq + j as f64 * s16 / 2.0;
                            mix.place(&bus, &Sig::Mono(sig), at, if last { 0.26 } else { 0.17 }, 0.0);
                        }
                        let swish = synth::whoosh(0.35, 500.0, 6000.0, true, WHOOSH_SEED);
                        mix.place("fx", &Sig::Mono(swish), tq, 0.06, 0.0);
                    } else if is(&cfg.low.name)? {
                        // FINAL FLICK: a horn fanfare over the wind-up
                        let r = self.tonic(ctx);
                        for (bt, m, d) in [(0.0, r, 0.25), (0.25, r + 7, 0.25), (0.5, r + 12, 0.5)] {
                            let at = tq + bt * beat;
                            mix.place(&bus, &Sig::Mono(self.horn(ctx, m, d * beat, 1.0, None)?), at, 0.28, 0.0);
                            let low = if m - 5 >= 57 { m - 5 } else { m };
                            mix.place(
                                &bus,
                                &Sig::Mono(self.horn(ctx, low, d * beat, 0.8, None)?),
                                at,
                                0.13,
                                0.0,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// A falling wail over clattering tubing.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t_hs = ctx.show.t_hs;
        let x = self.horn(ctx, 69, 0.9, 1.0, Some(1.3))?;
        let x = gliss(&x, -12.0, 0.9)?;
        // * np.linspace(1, 0.3, len(x))
        let n = x.len();
        let step = if n > 1 { (0.3 - 1.0) / (n - 1) as f64 } else { 0.0 };
        let wail: Vec<f64> = x
            .iter()
            .enumerate()
            .map(|(i, &v)| v * if i + 1 == n { 0.3 } else { i as f64 * step + 1.0 })
            .collect();
        mix.place(&self.bus(ctx), &Sig::Mono(wail), t_hs, 0.22, 0.0);
        for (j, m) in [86, 81, 91, 77, 84, 89].into_iter().enumerate() {
            let jf = j as f64;
            let ring = clank(f64::from(m), 20 + j as u64, CLANK_DUR, &CLANK_DECAYS)?;
            let at = t_hs + 0.09 * jf + 0.04 * h01(&[jf, 4.0]);
            mix.place("fx", &Sig::Mono(ring), at, 0.08, (jf - 2.5) / 6.0);
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let r = self.tonic(ctx);
        for (m, g) in [(r, 0.26), (r + 7, 0.22), (r + 12, 0.2)] {
            mix.place("solo", &Sig::Mono(self.horn(ctx, m, 1.0, 1.0, Some(1.5))?), tw, g, 0.0);
        }
        Ok(())
    }

    /// Its solo (the fight file's): it stops at the reassembly.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            // a late K.O. leaves less room: stop at the reassembly
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let x = self.horn(ctx, m, d * grid.beat * 0.95, 0.95, None)?;
            mix.place("solo", &Sig::Mono(x), ts, 0.44, 0.0);
        }
        Ok(())
    }

    /// Its pieces fly home as a rising call.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let r = self.tonic(ctx);
        let run: Vec<i32> = [0, 3, 7, 8, 12, 15].iter().map(|iv| r + iv).collect();
        let n = run.len() as f64;
        for (j, &m) in run.iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / n);
            mix.place(&bus, &Sig::Mono(self.horn(ctx, m, 0.12, 0.7, None)?), tj, 0.14, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The part's own sounds: their lengths at 48 kHz, seeded where they hold noise.
    #[test]
    fn the_flicks_noises_have_their_lengths() {
        assert_eq!(snap(3).len(), 2880);
        assert_eq!(snap(3), snap(3));
        assert_ne!(snap(3), snap(4));
        assert_eq!(boing(7).len(), 21_600);
        let w = slide_whistle(0.45).expect("a whistle");
        assert_eq!(w.len(), 21_600);
        assert_eq!(w[0], 0.0, "it fades in");
        assert!(slide_whistle(f64::NAN).is_err());
        let x: Vec<f64> = (0..4800).map(|i| dmath::sin(i as f64 * 0.05)).collect();
        let s = stopped(&x);
        assert_eq!(s.len(), x.len());
        assert!(s.iter().all(|v| v.abs() < 2.0));
    }

    /// `pan_of` holds the pan to 0.6 either way.
    #[test]
    fn a_pan_is_held_to_six_tenths() {
        assert_eq!(pan_of(7.0), 0.6);
        assert_eq!(pan_of(-7.0), -0.6);
        assert_eq!(pan_of(1.75), 0.5);
    }
}
