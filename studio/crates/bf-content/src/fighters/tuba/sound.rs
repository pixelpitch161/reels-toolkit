//! TUBA's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/tuba.py` (#36).
//!
//! The bass line (role `bass`): every oom holds the bar's root into the pah (the air sucked into the bell under
//! it, and a sub-octave under it for the leader), every pah is the fifth as a short blast (gone below 25 HP unless
//! it spits), a spit adds the octave and the cannonball's whoosh; a gulp is a low bubble pop and a growl; the
//! cannonball into the ropes is a thud, a crash and the bed's boom. OOMPAH rips up an octave, BRASS WALL is a
//! buzzing root under a metallic shimmer, SUBSONIC a sub growl climbing into the slam, which lands on the lowest
//! note, a sub rumble, a thud and the bed's boom.
//!
//! Every note is `tuba(m, beats, vel)`: the TUBA's voice through `M.note`, and for a note held over 0.36 s a soft
//! buzzing sustain under it. The voice draws no noise, but its notes are asked for in the kit's order all the
//! same: the first of them measures the voice's level. The Python part reads nothing off its look.

use bf_core::dmath::{self, PI};
use bf_core::harmony::HarmonyError;
use bf_core::py::{py_max, py_min, round_nd};
use bf_core::roster::AbilityCfg;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    CLANK_DECAYS, CLANK_DUR, ColourArgs, MusicCtx, MusicError, Samples, clank, event_int, event_is, event_is_str,
    event_number, gliss, noise_seed,
};
use bf_sound::synth::{self, FILTER_O, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED, clip};
use bf_sound::traits::{Bed, BedHook, Played, Sound};

/// The name the part's errors carry.
const H: &str = "TUBA";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// Python's `int(x * SR)` for a length in samples, cut toward zero.
fn samples(seconds: f64) -> Result<usize, MusicError> {
    let n = seconds * SRF;
    if n.is_finite() && n > -1.0 && n < 1.0e9 {
        Ok(n as usize)
    } else {
        Err(MusicError::new(H, format!("{seconds} s is not a length")))
    }
}

/// `tuba(m, beats, vel)`: a tuba note held for `beats` (its Pixel Pitch voice, stretched with a soft buzzing
/// sustain under it):
///
/// ```python
/// dur = beats * T.BEAT * 0.92
/// x = M.note('TUBA', m, round(min(dur, 0.4), 3), round(dur + 0.2, 2), round(vel, 1)).copy()
/// if dur > 0.36:
///     n = len(x); tt = A.tt(n); f = A.mtof(m); ph = (f * tt) % 1.0
///     sus = A.lp(0.6 * (2 * ph - 1) + 0.4 * np.sin(2 * np.pi * f * tt), 700) * 0.22 * vel
///     env = np.clip((tt - 0.12) / 0.15, 0, 1) * np.clip((dur - tt) / 0.06, 0, 1)
///     x += sus * env * M.inst_gain('TUBA') * 0.5
/// ```
///
/// # Errors
/// As [`MusicCtx::note`].
pub fn tuba(ctx: &MusicCtx, m: f64, beats: f64, vel: f64) -> Samples {
    let dur = beats * ctx.show.grid.beat * 0.92;
    let x = ctx.note(
        "TUBA",
        m,
        round_nd(py_min(dur, 0.4), 3),
        round_nd(dur + 0.2, 2),
        round_nd(vel, 1),
    )?;
    let mut x = x.to_vec();
    if dur > 0.36 {
        let n = x.len();
        let tt = synth::tt(n);
        let f = synth::mtof(m);
        let w = 2.0 * PI * f;
        let raw: Vec<f64> = tt
            .iter()
            .map(|&t| {
                let ph = (f * t) % 1.0;
                0.6 * (2.0 * ph - 1.0) + 0.4 * dmath::sin(w * t)
            })
            .collect();
        let g = ctx.inst_gain("TUBA")?;
        for ((v, s), &t) in x.iter_mut().zip(synth::lp(&raw, 700.0, FILTER_O)).zip(&tt) {
            let sus = s * 0.22 * vel;
            let env = clip((t - 0.12) / 0.15, 0.0, 1.0) * clip((dur - t) / 0.06, 0.0, 1.0);
            *v += sus * env * g * 0.5;
        }
    }
    Ok(x)
}

/// `sub_drone(dur, f0, f1)`: SUBSONIC's telegraph, a sub-bass growl climbing from `f0` toward `f1` Hz.
///
/// # Errors
/// `dur` is not a length.
pub fn sub_drone(dur: f64, f0: f64, f1: f64) -> Samples {
    let n = samples(dur)?;
    let tt = synth::tt(n);
    // u = tt / dur; f = f0 + (f1 - f0) * u ** 1.5; ph = 2 * np.pi * np.cumsum(f) / SR
    let u: Vec<f64> = tt.iter().map(|t| t / dur).collect();
    let f: Vec<f64> = u.iter().map(|&u| f0 + (f1 - f0) * dmath::pow(u, 1.5)).collect();
    let ph = dsp::cumsum(&f);
    let wobble = 2.0 * PI * 9.0;
    Ok((0..n)
        .map(|i| {
            let ph = 2.0 * PI * ph[i] / SRF;
            // x = np.sin(ph) + 0.4 * np.sin(2 * ph) * (0.5 + 0.5 * np.sin(2 * np.pi * 9 * tt))
            let x = dmath::sin(ph) + 0.4 * dmath::sin(2.0 * ph) * (0.5 + 0.5 * dmath::sin(wobble * tt[i]));
            dmath::tanh(2.0 * x) * clip(tt[i] / 0.1, 0.0, 1.0) * (0.4 + 0.6 * u[i])
        })
        .collect())
}

/// `np.sin(2 * np.pi * f * tt)` for `n` samples.
fn sine(f: f64, n: usize) -> Vec<f64> {
    let w = 2.0 * PI * f;
    synth::tt(n).iter().map(|&t| dmath::sin(w * t)).collect()
}

/// The TUBA's part.
#[derive(Clone, Copy, Debug)]
pub struct TubaSound {
    k: usize,
}

impl TubaSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> TubaSound {
        TubaSound { k }
    }

    /// `root(chord)`: the oom, the chord's root from the bottom of its register.
    ///
    /// # Errors
    /// `chord` is not a chord name.
    pub fn root(&self, ctx: &MusicCtx, chord: &str) -> Result<i32, MusicError> {
        ctx.harmony.root_above(chord, ctx.cfg[self.k].register.0).map_err(bad)
    }

    /// `self.root(T.chord_at(tq + 0.01))`.
    fn root_at(&self, ctx: &MusicCtx, tq: f64) -> Result<i32, MusicError> {
        self.root(ctx, ctx.harmony.chord_at(tq + 0.01))
    }

    /// `self.colour(x, t, drive_k, low=True)`.
    fn col(&self, ctx: &MusicCtx, x: &[f64], t: f64, drive_k: f64) -> Result<(Vec<f64>, f64, f64), MusicError> {
        let args = ColourArgs {
            low: true,
            ..self.colour_args()
        };
        self.colour(ctx, x, t, drive_k, &args)
    }

    /// The first loop of `play`: the bass line, the gulps and the cannonball (tuba.py 533-575).
    fn line(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let beat = ctx.show.grid.beat;
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            let kind = e.kind.as_str();
            if kind == "oom" && event_is(H, e, 2, k)? {
                let tq = ctx.q16(t);
                let t_in = event_number(H, e, 3)?;
                let root = self.root_at(ctx, tq)?;
                let beats = py_max(1.0, (ctx.q16(t_in) - tq) / beat + 0.4);
                let (sig, g, lead) = self.col(ctx, &tuba(ctx, f64::from(root), beats, 0.9)?, t, 0.8)?;
                mix.place(&bus, &Sig::Mono(sig), tq, 0.62 * g, 0.12);
                // the inhale: air sucked in, rising into the bell
                let n = samples(t_in - t + 0.1)?;
                if n > 400 {
                    let tt = synth::tt(n);
                    let last = tt[n - 1];
                    let air = synth::bp(&synth::noise(noise_seed(H, t * 10.0)?, n), 300.0, 2400.0, FILTER_O);
                    // (tt_ / tt_[-1]) ** 2: numpy squares
                    let air: Vec<f64> = air
                        .iter()
                        .zip(&tt)
                        .map(|(a, &t)| {
                            let r = t / last;
                            a * (r * r)
                        })
                        .collect();
                    mix.place("fx", &Sig::Mono(air), tq, 0.05, 0.1);
                }
                if lead > 0.04 {
                    // the leader: a sub-octave under the oom
                    let n = samples(beats * beat * 0.9)?;
                    let env = synth::adsr(n, 0.02, 0.2, 0.7, 0.1);
                    let sub: Vec<f64> = sine(synth::mtof(f64::from(root - 12)), n)
                        .iter()
                        .zip(&env)
                        .map(|(s, e)| s * e)
                        .collect();
                    mix.place(&bus, &Sig::Mono(sub), tq, 0.22 * py_min(1.0, lead * 4.0), 0.0);
                }
            } else if kind == "pah" && event_is(H, e, 2, k)? {
                let tq = ctx.q16(t);
                let hp = ctx.hp_at(k, t);
                let caught = event_int(H, e, 3)? != 0;
                let root = self.root_at(ctx, tq)?;
                if hp > 25.0 || caught {
                    // pah: the fifth as a short blast (gone below 25 HP)
                    let x = tuba(ctx, f64::from(root + 7), 0.6, if caught { 0.95 } else { 0.75 })?;
                    let (sig, g, _) = self.col(ctx, &x, t, 0.8)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, (if caught { 0.62 } else { 0.45 }) * g, 0.12);
                }
                if caught {
                    // the spit: an octave blast and the cannonball's whoosh
                    let x = tuba(ctx, f64::from(root + 12), 0.5, 1.0)?;
                    mix.place(&bus, &Sig::Mono(self.col(ctx, &x, t, 0.8)?.0), tq, 0.34, 0.0);
                    let whoosh = synth::whoosh(0.6, 500.0, 7000.0, WHOOSH_RISE, WHOOSH_SEED);
                    mix.place("fx", &Sig::Mono(whoosh), tq, 0.16, 0.0);
                }
            } else if kind == "gulp" && event_is(H, e, 2, k)? {
                // gulp: a low bubble pop and a growl from the bell
                let tq = ctx.q16(t);
                let n = samples(0.22)?;
                let tt = synth::tt(n);
                // np.sin(2 * np.pi * np.cumsum(180 * np.exp(-tt_ / 0.06) + 50) / SR) * np.exp(-tt_ / 0.08)
                let f: Vec<f64> = tt.iter().map(|&t| 180.0 * dmath::exp(-t / 0.06) + 50.0).collect();
                let pop: Vec<f64> = dsp::cumsum(&f)
                    .iter()
                    .zip(&tt)
                    .map(|(&c, &t)| dmath::sin(2.0 * PI * c / SRF) * dmath::exp(-t / 0.08))
                    .collect();
                mix.place("fx", &Sig::Mono(pop), tq, 0.32, 0.0);
                let x = tuba(ctx, f64::from(ctx.harmony.key_root - 12), 0.5, 0.9)?;
                mix.place(&bus, &Sig::Mono(self.col(ctx, &x, t, 1.2)?.0), tq, 0.30, 0.0);
            } else if kind == "hit" && event_is(H, e, 2, k)? && event_is_str(H, e, 7, "cannon")? {
                // into the ropes
                let tq = ctx.q16(t);
                let thud = synth::thud(0.5, 120.0, 40.0, noise_seed(H, t * 7.0)?);
                mix.place("fx", &Sig::Mono(thud), tq, 0.42, 0.0);
                mix.place("drums", &Sig::Mono(synth::crash(1.2)), tq, 0.08, 0.0);
                bed.boom(ctx, mix, tq, 0.30)?;
            }
        }
        Ok(())
    }

    /// The second loop of `play`: its abilities and SUBSONIC's landing (tuba.py 576-605).
    fn abilities(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let beat = ctx.show.grid.beat;
        let ab = ctx.cfg[k];
        let beats_of = |slot: &AbilityCfg, key: &str| {
            slot.get(key)
                .ok_or_else(|| MusicError::new(H, format!("the ability {} has no {key}", slot.name)))
        };
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if e.kind == "ability" && event_is(H, e, 2, k)? {
                let tq = ctx.q16(t);
                let is = |name: &str| event_is_str(H, e, 3, name);
                if is(&ab.unlock1.name)? {
                    // OOMPAH: rips up an octave into the double time
                    let x = tuba(ctx, f64::from(self.root_at(ctx, tq)? + 12), 0.8, 1.0)?;
                    let x = gliss(&x, 12.0, 0.22)?;
                    mix.place(&bus, &Sig::Mono(self.col(ctx, &x, t, 0.8)?.0), tq, 0.36, 0.0);
                } else if is(&ab.unlock2.name)? {
                    // BRASS WALL: a buzzing root under a metallic shimmer
                    let dur = beats_of(&ab.unlock2, "dur_beats")? * beat;
                    let x = tuba(ctx, f64::from(self.root_at(ctx, tq)?), dur / beat, 0.8)?;
                    mix.place(&bus, &Sig::Mono(self.col(ctx, &x, t, 0.6)?.0), tq, 0.20, 0.0);
                    let n = samples(dur)?;
                    let r = ctx.harmony.key_root;
                    let tones: Vec<Vec<f64>> = [r + 48, r + 55, r + 60]
                        .iter()
                        .map(|&m| sine(synth::mtof(f64::from(m)), n))
                        .collect();
                    let wave = sine(6.0, n);
                    let env = synth::adsr(n, 0.05, 0.3, 0.6, 0.5);
                    let shim: Vec<f64> = (0..n)
                        .map(|i| {
                            // sum(...) / 3: Python's sum starts from 0
                            let s = 0.0 + tones[0][i] + tones[1][i] + tones[2][i];
                            s / 3.0 * (0.6 + 0.4 * wave[i]) * env[i]
                        })
                        .collect();
                    mix.place("fx", &Sig::Mono(shim), tq, 0.025, 0.0);
                    mix.place("drums", &Sig::Mono(synth::crash(1.2)), tq, 0.08, 0.0);
                } else if is(&ab.low.name)? {
                    // SUBSONIC: a sub growl climbing toward the slam
                    let tele = beats_of(&ab.low, "tele")? * beat;
                    mix.place(&bus, &Sig::Mono(sub_drone(tele + 0.05, 30.0, 48.0)?), tq, 0.40, 0.0);
                }
            } else if e.kind == "slam" && event_is(H, e, 2, k)? {
                // SUBSONIC lands: a ring-shaking low note
                let ts = e.t;
                let x = tuba(ctx, f64::from(ctx.harmony.key_root - 12), 2.5, 1.0)?;
                mix.place(&bus, &Sig::Mono(x), ts, 0.70, 0.0);
                mix.place(&bus, &Sig::Mono(sub_drone(1.2, 26.0, 30.0)?), ts, 0.30, 0.0);
                mix.place("fx", &Sig::Mono(synth::thud(0.9, 90.0, 28.0, THUD_SEED)), ts, 0.65, 0.0);
                bed.boom(ctx, mix, ts, 0.6)?;
            }
        }
        Ok(())
    }
}

impl Sound for TubaSound {
    fn k(&self) -> usize {
        self.k
    }

    /// The cannonball and SUBSONIC's landing call `M.bed.boom` without asking (`tuba.py` 575, 605).
    fn bed_calls(&self) -> &'static [BedHook] {
        &[BedHook::Boom]
    }

    /// #36's tuba bus.
    fn bus_gain(&self) -> f64 {
        2.4
    }

    fn send(&self) -> f64 {
        0.08
    }

    /// The oom-pah, F1 to C3 (#36's band check).
    fn band(&self) -> (f64, f64) {
        (40.0, 260.0)
    }

    /// #36's `colour()`; every call passes `low=True` for the bass.
    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 70.0,
            cut_span: 420.0,
            drive: 4.5,
            low_cut: (30.0, 90.0),
            ..ColourArgs::default()
        }
    }

    /// `voice(m, dur, vel=0.85)`: `self.tuba(m, dur / self.T.BEAT, vel)`.
    fn voice(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<std::sync::Arc<[f64]>, MusicError> {
        Ok(tuba(ctx, m, dur / ctx.show.grid.beat, vel)?.into())
    }

    /// A low blast, the low hit under the high one (F1 under F2 by default).
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let r = ctx.harmony.key_root;
        let hits = ctx
            .ff
            .music
            .title_hits
            .get(self.name(ctx))
            .cloned()
            .unwrap_or_else(|| vec![r - 12, r]);
        let (Some(&low), Some(&high)) = (hits.first(), hits.last()) else {
            return Err(MusicError::new(H, "the title hits are an empty list"));
        };
        mix.place(&bus, &Sig::Mono(tuba(ctx, f64::from(low), 2.0, 1.0)?), t, 0.55, 0.0);
        mix.place(&bus, &Sig::Mono(tuba(ctx, f64::from(high), 2.0, 1.0)?), t, 0.50, 0.0);
        Ok(())
    }

    /// The count-in on bar 1's root.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let ch = ctx
            .harmony
            .chords
            .get(1)
            .ok_or_else(|| MusicError::new(H, "the episode has no bar 1 to count in on"))?;
        let r = self.root(ctx, ch)?;
        for (tc, _) in ctx.show.grid.count.iter().take(3) {
            mix.place(&bus, &Sig::Mono(tuba(ctx, f64::from(r), 0.8, 0.95)?), *tc, 0.55, 0.0);
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        self.line(ctx, bed, mix)?;
        self.abilities(ctx, bed, mix)
    }

    /// The tuba collapses: a falling groan, valves rattling.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t_hs = ctx.show.t_hs;
        let x = tuba(ctx, f64::from(ctx.harmony.key_root), 2.0, 1.0)?;
        let fade = dsp::linspace(1.0, 0.3, x.len());
        let groan: Vec<f64> = gliss(&x, -7.0, 0.9)?.iter().zip(&fade).map(|(a, b)| a * b).collect();
        mix.place(&self.bus(ctx), &Sig::Mono(groan), t_hs, 0.40, 0.0);
        for (j, m) in [77.0, 72.0, 82.0, 69.0, 75.0].into_iter().enumerate() {
            let valve = clank(m, 20 + j as u64, CLANK_DUR, &CLANK_DECAYS)?;
            mix.place(
                "fx",
                &Sig::Mono(valve),
                t_hs + 0.09 * j as f64,
                0.08,
                (j as f64 - 2.0) / 5.0,
            );
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let r = f64::from(ctx.harmony.key_root);
        mix.place("solo", &Sig::Mono(tuba(ctx, r - 12.0, 2.0, 1.0)?), tw, 0.55, 0.0);
        mix.place("solo", &Sig::Mono(tuba(ctx, r, 2.0, 1.0)?), tw, 0.50, 0.0);
        mix.place("solo", &Sig::Mono(tuba(ctx, r + 7.0, 2.0, 0.9)?), tw, 0.30, 0.0);
        Ok(())
    }

    /// The strutting solo (the fight file's), up to the reassembly.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            // a late K.O. leaves less room: stop at the reassembly
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            mix.place(
                "solo",
                &Sig::Mono(tuba(ctx, f64::from(m), d * 0.95, 0.95)?),
                ts,
                0.62,
                0.0,
            );
        }
        Ok(())
    }

    /// Its pieces fly home as a rising run.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let r = ctx.harmony.key_root;
        let run = [-12, -7, -5, 0, 3, 7].map(|i| r + i);
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            mix.place(&bus, &Sig::Mono(tuba(ctx, f64::from(m), 0.35, 0.7)?), tj, 0.32, 0.0);
        }
        Ok(())
    }

    /// The bass climbs the last chord (root, 3rd, 5th, 7th) into the title slam.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        if self.role(ctx) != "bass" {
            return Ok(());
        }
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let t_last = (grid.n_bars as f64 - 1.0) * grid.bar;
        let ch = ctx
            .harmony
            .chords
            .last()
            .ok_or_else(|| MusicError::new(H, "the episode has no bars"))?;
        let r = self.root(ctx, ch)?;
        let pcs = ctx.harmony.chord_pcs(ch).map_err(bad)?;
        let [p0, p1, p2, ..] = pcs else {
            return Err(MusicError::new(
                H,
                format!(
                    "the last chord {ch:?} has {} note(s), and the bass climbs its root, third and fifth: use a \
                     chord of three notes or more on the last bar",
                    pcs.len()
                ),
            ));
        };
        let climb = [r, r + (p1 - p0).rem_euclid(12), r + (p2 - p0).rem_euclid(12), r + 10];
        for (bt, m) in climb.into_iter().enumerate() {
            let x = tuba(ctx, f64::from(m), 0.9, 0.85)?;
            mix.place(&bus, &Sig::Mono(x), t_last + bt as f64 * grid.beat, 0.50, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sub_drone_climbs_and_swells() {
        let x = sub_drone(0.5, 30.0, 48.0).unwrap();
        assert_eq!(x.len(), 24000);
        assert_eq!(x[0], 0.0);
        assert!(x.iter().all(|v| v.abs() <= 1.0));
        // it swells: louder at the end than just after the fade-in
        let peak = |a: &[f64]| a.iter().fold(0.0f64, |p, v| p.max(v.abs()));
        assert!(peak(&x[20000..]) > peak(&x[4800..9600]));
        assert!(sub_drone(f64::NAN, 30.0, 48.0).is_err());
    }
}
