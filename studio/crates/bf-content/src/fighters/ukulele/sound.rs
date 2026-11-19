//! UKULELE's part in the score, ported close enough for the ear (ruling M4-R16) from #33's `audio.py` (`uke_part`
//! and the UKULELE's branches of `abilities`): every SURF'S UP wave is a strum in the uke's own voice on a re-entrant
//! GCEA shape of the bar's chord (down on its own grid, up on SPEED STRUM's extra waves), with the swell's whoosh;
//! the leader adds an off-beat up-strum; it loses strings (4 down to 2) as it gets hurt; a wave that lands is a
//! picked note climbing the scale with a crash of water and a thud, one that breaks on the ropes a softer crash;
//! CHUNK a muted chunk strum and a splash; SPEED STRUM a tremolo flourish; UKE-NADO a whirl of strums spiralling up
//! and the waterspout's wind for as long as it lives.

use bf_core::dmath;
use bf_core::fightfile::SoloNote;
use bf_core::harmony::parse_chord;
use bf_core::py::{py_max, round_nd};
use bf_sound::SRF;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, Samples, event_is, event_is_str, noise_seed};
use bf_sound::synth::{self, FILTER_O, WHOOSH_RISE};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "UKULELE";

/// The open strings, re-entrant GCEA.
const GCEA: [i32; 4] = [67, 60, 64, 69];

/// The chord as a re-entrant GCEA shape: on each string the lowest chord tone at or above it (Am: A C E A, G: G D G
/// B, as #33's `UKE_CHORD`; its E was B E G# B).
///
/// # Errors
/// `ch` is not a chord name.
pub fn uke_chord(ch: &str) -> Result<Vec<i32>, MusicError> {
    let pcs = parse_chord(ch).map_err(|e| MusicError::new(H, e.to_string()))?.pcs();
    Ok(GCEA
        .iter()
        .map(|&o| (o..o + 12).find(|m| pcs.contains(&m.rem_euclid(12))).unwrap_or(o))
        .collect())
}

/// `uke_strings(hp)`: it loses strings as it gets hurt.
pub fn uke_strings(hp: f64) -> usize {
    if hp > 50.0 {
        4
    } else if hp > 25.0 {
        3
    } else {
        2
    }
}

fn samples(seconds: f64) -> Result<usize, MusicError> {
    let n = seconds * SRF;
    if n.is_finite() && n > -1.0 && n < 1.0e9 {
        Ok(n as usize)
    } else {
        Err(MusicError::new(H, format!("{seconds} s is not a length")))
    }
}

/// `wave_crash(dur, seed, big)`: a wave breaking: a low rush that swells and dies, a hiss of spray on top.
fn wave_crash(dur: f64, seed: u64, big: f64) -> Result<Vec<f64>, MusicError> {
    let n = samples(dur)?;
    let x = synth::noise(seed, n);
    let rush = synth::lp(&x, 900.0, FILTER_O);
    let spray = synth::bp(&x, 2500.0, 7000.0, FILTER_O);
    Ok(synth::tt(n)
        .into_iter()
        .enumerate()
        .map(|(i, t)| {
            let env = (t / 0.03).clamp(0.0, 1.0) * dmath::exp(-t / (dur * 0.35));
            let senv = (t / 0.06).clamp(0.0, 1.0) * dmath::exp(-t / (dur * 0.25));
            (rush[i] * env + spray[i] * senv * 0.35) * big
        })
        .collect())
}

/// `water_wind(dur, seed)`: the waterspout: a swirling band of wind and water, its pitch wheeling round.
fn water_wind(dur: f64, seed: u64) -> Result<Vec<f64>, MusicError> {
    let n = samples(dur)?;
    let x = synth::noise(seed, n);
    let mut out = vec![0.0; n];
    let nb = 24;
    for k in 0..nb {
        let (a, b) = (k * n / nb, (k + 1) * n / nb);
        if b <= a {
            continue;
        }
        let fc = 700.0 + 500.0 * dmath::sin(2.0 * dmath::PI * 3.0 * (a as f64 / SRF));
        let lo = a.saturating_sub(2048);
        let y = synth::bp(&x[lo..b], fc * 0.6, fc * 1.6, FILTER_O);
        out[a..b].copy_from_slice(&y[y.len() - (b - a)..]);
    }
    let low = synth::lp(&x, 300.0, FILTER_O);
    Ok(synth::tt(n)
        .into_iter()
        .enumerate()
        .map(|(i, t)| {
            let env = (t / 0.3).clamp(0.0, 1.0) * ((dur - t) / 0.4).clamp(0.0, 1.0);
            (out[i] + 0.4 * low[i]) * env * (1.0 + 0.4 * dmath::sin(2.0 * dmath::PI * 6.0 * t))
        })
        .collect())
}

/// The UKULELE's part.
#[derive(Clone, Copy, Debug)]
pub struct UkuleleSound {
    k: usize,
}

impl UkuleleSound {
    /// The part of the fighter on side `k`.
    pub fn new(k: usize) -> UkuleleSound {
        UkuleleSound { k }
    }

    fn pan(&self) -> f64 {
        if self.k == 0 { -0.25 } else { 0.25 }
    }

    fn note(&self, ctx: &MusicCtx, m: f64, dur: f64, cap: f64, vel: f64) -> Samples {
        let pp = ctx.cfg[self.k].pp.as_str();
        Ok(ctx.note(pp, m.trunc(), dur, cap, vel)?.to_vec())
    }

    /// `strum(notes, vel, spread, up, dur, cap)`: the chord's strings a few ms apart.
    #[allow(clippy::too_many_arguments)] // the Python signature
    fn strum(&self, ctx: &MusicCtx, notes: &[i32], vel: f64, spread: f64, up: bool, dur: f64, cap: f64) -> Samples {
        let mut ns = notes.to_vec();
        if up {
            ns.reverse();
        }
        let d = samples(spread)?;
        let mut out = vec![0.0; samples(cap)? + d * ns.len()];
        for (j, &m) in ns.iter().enumerate() {
            let v = round_nd(vel * if j == 0 { 1.0 } else { 0.9 }, 2);
            let x = self.note(ctx, f64::from(m), dur, cap, v)?;
            for (o, s) in out[j * d..].iter_mut().zip(&x) {
                *o += s;
            }
        }
        let root = (ns.len().max(1) as f64).sqrt();
        Ok(out.iter().map(|v| v / root * 1.5).collect())
    }

    fn shape_at(&self, ctx: &MusicCtx, t: f64) -> Result<Vec<i32>, MusicError> {
        uke_chord(ctx.harmony.chord_at(t))
    }
}

impl Sound for UkuleleSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.55
    }

    fn send(&self) -> f64 {
        0.25
    }

    fn band(&self) -> (f64, f64) {
        (300.0, 6000.0)
    }

    fn voice(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<std::sync::Arc<[f64]>, MusicError> {
        Ok(self.note(ctx, m, dur, py_max(0.6, dur + 0.3), vel)?.into())
    }

    /// The A minor shape rung out high, then an up-strum (`main()`'s title strum).
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let hits = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(h) => h.clone(),
            None => vec![69, 72, 76, 81],
        };
        let bus = self.bus(ctx);
        let s16 = ctx.show.grid.beat / 4.0;
        mix.place(
            &bus,
            &Sig::Mono(self.strum(ctx, &hits, 1.0, 0.008, false, 0.6, 1.2)?),
            t,
            0.50,
            self.pan(),
        );
        mix.place(
            &bus,
            &Sig::Mono(self.strum(ctx, &hits, 0.8, 0.006, true, 0.3, 0.8)?),
            t + s16,
            0.30,
            self.pan(),
        );
        Ok(())
    }

    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        for (tc, _) in ctx.show.grid.count.iter().take(3) {
            let ch = self.shape_at(ctx, *tc)?;
            mix.place(
                &bus,
                &Sig::Mono(self.strum(ctx, &ch, 0.9, 0.011, false, 0.40, 1.0)?),
                *tc,
                0.30,
                self.pan(),
            );
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let pan = self.pan();
        let beat = ctx.show.grid.beat;
        let s16 = beat / 4.0;
        let climb = ctx
            .harmony
            .scale_notes(72, 93, None)
            .map_err(|e| MusicError::new(H, e.to_string()))?;
        let mut n_landed = 0usize;
        let ev = &ctx.rec.events;
        for e in ev {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            let kind = e.kind.as_str();
            if kind == "surf" && event_is(H, e, 2, k)? {
                let tq = ctx.q16(t);
                let hp = ctx.hp_at(k, t);
                let full = self.shape_at(ctx, tq)?;
                let vo = &full[..uke_strings(hp).min(full.len())];
                let main = e.i(3) != 0;
                let sig = if main {
                    self.strum(ctx, vo, 0.9, 0.010, false, 0.40, 1.0)?
                } else {
                    self.strum(ctx, vo, 0.7, 0.008, true, 0.40, 1.0)?
                };
                let (y, g, lead) = self.colour(ctx, &sig, t, 0.8, &self.colour_args())?;
                mix.place(&bus, &Sig::Mono(y), tq, if main { 0.36 } else { 0.26 } * g, pan);
                let whoosh = synth::whoosh(0.28, 300.0, 2400.0, WHOOSH_RISE, noise_seed(H, t * 3.0)?);
                mix.place("fx", &Sig::Mono(whoosh), tq, 0.05, pan);
                if main && lead > 0.04 {
                    let up = self.strum(ctx, vo, 0.6, 0.008, true, 0.40, 1.0)?;
                    let y2 = self.colour(ctx, &up, t, 0.8, &self.colour_args())?.0;
                    mix.place(&bus, &Sig::Mono(y2), tq + 2.0 * s16, 0.18 * g, pan);
                }
            } else if kind == "surf_end" && event_is(H, e, 2, k)? && e.i(5) != 0 {
                let crash = wave_crash(0.45, noise_seed(H, t * 31.0)?, 0.7)?;
                mix.place("fx", &Sig::Mono(crash), t, 0.07, (e.f(3) / 3.5).clamp(-0.6, 0.6));
            } else if kind == "hit"
                && event_is(H, e, 2, k)?
                && (event_is_str(H, e, 7, "surf")? || event_is_str(H, e, 7, "spout")?)
            {
                let tq = ctx.q16(t);
                let m = climb[n_landed % climb.len().max(1)];
                n_landed += 1;
                let x = self.note(ctx, f64::from(m), 0.25, 0.6, 1.0)?;
                let (y, g, _) = self.colour(ctx, &x, t, 0.6, &self.colour_args())?;
                mix.place(&bus, &Sig::Mono(y), tq, 0.30 * g, pan);
                let surf = event_is_str(H, e, 7, "surf")?;
                let crash = wave_crash(0.55, noise_seed(H, t * 37.0)?, 1.0)?;
                mix.place("fx", &Sig::Mono(crash), tq, if surf { 0.13 } else { 0.09 }, -pan);
                mix.place(
                    "fx",
                    &Sig::Mono(synth::thud(0.3, 140.0, 60.0, noise_seed(H, t * 7.0)?)),
                    tq,
                    0.12,
                    0.0,
                );
            } else if kind == "spout" && event_is(H, e, 2, k)? {
                let end = ev
                    .iter()
                    .find(|x| x.kind == "spout_end" && x.who() == Some(k) && x.t >= t)
                    .map_or(ctx.show.t_ko, |x| x.t.min(ctx.show.t_ko));
                let wind = water_wind(end - t + 0.4, noise_seed(H, t * 3.0)?)?;
                mix.place("fx", &Sig::Mono(wind), t, 0.10, pan);
            } else if kind == "ability" && event_is(H, e, 2, k)? {
                let tq = ctx.q16(t);
                if event_is_str(H, e, 4, "shockwave")? {
                    // CHUNK: the percussive chunk strum and a splash
                    let vo = self.shape_at(ctx, tq)?;
                    let chunk = synth::lp(&self.strum(ctx, &vo, 1.0, 0.004, false, 0.05, 0.12)?, 3000.0, FILTER_O);
                    mix.place(&bus, &Sig::Mono(chunk), tq, 0.5, pan);
                    let n = samples(0.07)?;
                    let click: Vec<f64> = synth::bp(&synth::noise(5, n), 600.0, 3500.0, FILTER_O)
                        .iter()
                        .zip(synth::tt(n))
                        .map(|(v, t)| v * dmath::exp(-t / 0.012))
                        .collect();
                    mix.place("fx", &Sig::Mono(click), tq, 0.22, 0.0);
                    mix.place("fx", &Sig::Mono(synth::thud(0.4, 130.0, 45.0, 3)), tq, 0.3, 0.0);
                    mix.place(
                        "fx",
                        &Sig::Mono(wave_crash(0.5, noise_seed(H, t * 13.0)?, 0.8)?),
                        tq,
                        0.08,
                        0.0,
                    );
                } else if event_is_str(H, e, 4, "haste")? {
                    // SPEED STRUM: a quick tremolo-strum flourish
                    let vo = self.shape_at(ctx, tq)?;
                    for j in 0..4 {
                        let x = self.strum(ctx, &vo, 0.7, 0.005, j % 2 == 1, 0.12, 0.4)?;
                        mix.place(&bus, &Sig::Mono(x), tq + f64::from(j) * s16 / 2.0, 0.22, pan);
                    }
                } else if event_is_str(H, e, 4, "spout")? {
                    // UKE-NADO: a whirl of strums spiralling up as the spout rises
                    for j in 0..16 {
                        let tj = tq + f64::from(j) * s16 / 2.0;
                        let vo: Vec<i32> = self.shape_at(ctx, tj)?.iter().map(|m| m + 12 * (j / 8)).collect();
                        let x = self.strum(ctx, &vo, 0.6 + 0.025 * f64::from(j), 0.004, j % 2 == 1, 0.1, 0.35)?;
                        mix.place(&bus, &Sig::Mono(x), tj, 0.20, 0.6 * dmath::sin(f64::from(j) * 1.3));
                    }
                    let whoosh = synth::whoosh(2.0 * beat, 300.0, 6000.0, WHOOSH_RISE, 6);
                    mix.place("fx", &Sig::Mono(whoosh), tq, 0.16, 0.0);
                }
            }
        }
        Ok(())
    }

    /// The four strings snapping, top to bottom, and the little body cracking.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t_hs = ctx.show.t_hs;
        for (j, &m) in [69, 64, 60, 67].iter().enumerate() {
            let j_ = j as f64;
            let x = synth::hp(&self.note(ctx, f64::from(m), 0.2, 0.5, 1.0)?, 200.0, FILTER_O);
            mix.place("solo", &Sig::Mono(x), t_hs + 0.09 * j_, 0.30, (j_ - 2.0) / 4.0);
        }
        let n = samples(0.25)?;
        let crack: Vec<f64> = synth::bp(&synth::noise(78, n), 250.0, 2000.0, FILTER_O)
            .iter()
            .zip(synth::tt(n))
            .map(|(v, t)| v * dmath::exp(-t / 0.04))
            .collect();
        mix.place("fx", &Sig::Mono(crack), t_hs, 0.30, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let ch = self.shape_at(ctx, 0.0)?;
        mix.place(
            "solo",
            &Sig::Mono(self.strum(ctx, &ch, 1.0, 0.012, false, 0.9, 1.8)?),
            tw,
            0.46,
            0.0,
        );
        Ok(())
    }

    /// The fight file's solo: a chord is a strum (up on the off-beats), a number a picked note.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo_entries(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            match m {
                SoloNote::Chord(ch) => {
                    let x = self.strum(ctx, &ch, 0.9, 0.008, (bt % 1.0) > 0.1, py_max(0.12, d * grid.beat), 1.0)?;
                    mix.place("solo", &Sig::Mono(x), ts, 0.40, 0.0);
                }
                SoloNote::Note(m) => {
                    let x = self.note(ctx, f64::from(m), py_max(0.1, d * grid.beat * 0.9), 0.8, 0.9)?;
                    mix.place("solo", &Sig::Mono(x), ts, 0.48, 0.0);
                }
            }
        }
        Ok(())
    }

    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let run = [60, 64, 67, 69, 72, 76, 79, 81];
        for (j, &m) in run.iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            mix.place(
                "solo",
                &Sig::Mono(self.note(ctx, f64::from(m), 0.2, 0.5, 0.7)?),
                tj,
                0.22,
                0.0,
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shapes_are_the_episodes() {
        assert_eq!(uke_chord("Am").unwrap(), vec![69, 60, 64, 69]);
        assert_eq!(uke_chord("G").unwrap(), vec![67, 62, 67, 71]);
        assert_eq!(uke_chord("F").unwrap(), vec![69, 60, 65, 69]);
        // E: the lowest chord tones (the episode played B E G# B)
        assert_eq!(uke_chord("E").unwrap(), vec![68, 64, 64, 71]);
        assert_eq!((uke_strings(80.0), uke_strings(30.0), uke_strings(10.0)), (4, 3, 2));
    }
}
