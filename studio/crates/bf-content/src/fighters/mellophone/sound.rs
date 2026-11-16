//! MELLOPHONE's part in the score (a from-scratch fighter, 2026-11-16): a drum-corps mellophone whose notes come back
//! off the far wall.
//!
//! Every SLAPBACK wave is a short bright chord tone; every echo of it is the same note again, quieter and darker, one
//! chord tone higher a generation (a real slapback: the part answers itself). HORNS UP is three hard stabs, PRESS BOX
//! a two-note fanfare, STADIUM ECHO a held top note and a held note on each lap. It falls apart as a smear down the
//! valves; the stinger is a held tonic with its echo, the solo its notes, the reassembly a run up the chord.

use bf_core::harmony::HarmonyError;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, Samples, event_is_str, event_number};
use bf_sound::synth::{self, THUD_SEED, clip};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "MELLOPHONE";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// An echo of a note: the far wall's copy, darker (band-passed) and quieter.
pub fn echo(x: &[f64]) -> Vec<f64> {
    synth::bp(x, 220.0, 2200.0, 2).iter().map(|v| v * 1.3).collect()
}

/// MELLOPHONE's part.
#[derive(Clone, Copy, Debug)]
pub struct MellophoneSound {
    k: usize,
}

impl MellophoneSound {
    /// The part of the fighter on side `k`.
    pub fn new(k: usize) -> MellophoneSound {
        MellophoneSound { k }
    }

    /// One MELLOPHONE note.
    fn horn(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64, cap: f64) -> Samples {
        let pp = ctx.cfg[self.k].pp.as_str();
        Ok(ctx.note(pp, f64::from(m), dur, cap, vel)?.to_vec())
    }

    /// `T.chord_notes(ch, lo, hi)`, refused when empty.
    fn tones(&self, ctx: &MusicCtx, ch: &str, lo: i32, hi: i32) -> Result<Vec<i32>, MusicError> {
        let v = ctx.harmony.chord_tones(lo, hi, ch).map_err(bad)?;
        if v.is_empty() {
            return Err(MusicError::new(
                H,
                format!("the chord {ch:?} has no tone from {lo} to {hi}"),
            ));
        }
        Ok(v)
    }

    /// The episode's first chord.
    fn first_chord<'a>(&self, ctx: &'a MusicCtx) -> Result<&'a str, MusicError> {
        ctx.harmony
            .chords
            .first()
            .map(String::as_str)
            .ok_or_else(|| MusicError::new(H, "the episode has no bars"))
    }
}

impl Sound for MellophoneSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.2
    }

    fn send(&self) -> f64 {
        0.35
    }

    fn band(&self) -> (f64, f64) {
        (200.0, 4000.0)
    }

    /// A rip up the chord into a held top note.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let ch = self.first_chord(ctx)?;
        let ms = self.tones(ctx, ch, 58, 79)?;
        let n = ms.len() as f64;
        for (j, &m) in ms.iter().enumerate() {
            let x = self.horn(ctx, m, 0.08, 0.7, 0.8)?;
            mix.place(&bus, &Sig::Mono(x), t - 0.15 + 0.15 * j as f64 / n, 0.14, 0.0);
        }
        let top = *ms.last().unwrap_or(&72);
        let x = self.horn(ctx, top, 0.6, 1.0, 1.2)?;
        mix.place(&bus, &Sig::Mono(x), t, 0.30, 0.0);
        Ok(())
    }

    /// A short call on each count.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let ch = match ctx.harmony.chords.get(1) {
            Some(c) => c.as_str(),
            None => self.first_chord(ctx)?,
        };
        let ms = self.tones(ctx, ch, 62, 79)?;
        for (j, (tc, _)) in ctx.show.grid.count.iter().take(3).enumerate() {
            let x = self.horn(ctx, ms[j % ms.len()], 0.2, 0.8, 1.0)?;
            mix.place(&bus, &Sig::Mono(x), *tc, 0.26, 0.0);
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _bed: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let cfg = ctx.cfg[k];
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if e.who() != Some(k) || !matches!(e.kind.as_str(), "slap" | "ability") {
                continue;
            }
            let tq = ctx.q16(t);
            let ch = ctx.harmony.chord_at(tq);
            if e.kind == "slap" {
                let pan = clip(event_number(H, e, 4)? / 3.5, -0.7, 0.7);
                let g = event_number(H, e, 9)?.max(0.0) as usize;
                if event_is_str(H, e, 8, "stab")? {
                    // HORNS UP: a hard, short stab high in the chord
                    let ms = self.tones(ctx, ch, 72, 84)?;
                    let x = self.horn(ctx, ms[0], s16 * 0.9, 1.0, 1.3)?;
                    mix.place(&bus, &Sig::Mono(x), tq, 0.26, pan);
                } else if event_is_str(H, e, 8, "stadium")? {
                    // STADIUM ECHO: a held top note each lap, its echo behind it
                    let ms = self.tones(ctx, ch, 65, 84)?;
                    let m = ms[(g + ms.len() - 1) % ms.len()];
                    let x = self.horn(ctx, m, beat * 1.5, 1.0, 1.3)?;
                    mix.place(&bus, &Sig::Mono(echo(&x)), tq + beat * 0.5, 0.16, -pan);
                    mix.place(&bus, &Sig::Mono(x), tq, 0.32, pan);
                } else {
                    // SLAPBACK: the bell's note, then each echo the same climb, darker and quieter
                    let ms = self.tones(ctx, ch, 60, 81)?;
                    let x = self.horn(ctx, ms[g.min(ms.len() - 1)], beat * 0.6, 0.85, 1.0)?;
                    if g == 0 {
                        mix.place(&bus, &Sig::Mono(x), tq, 0.26, pan);
                    } else {
                        mix.place(&bus, &Sig::Mono(echo(&x)), tq, 0.20 - 0.02 * g.min(4) as f64, pan);
                    }
                }
                continue;
            }
            let is = |name: &str| event_is_str(H, e, 3, name);
            if is(&cfg.unlock2.name)? {
                // PRESS BOX: a two-note fanfare, root then the fifth above, held
                let ms = self.tones(ctx, ch, 60, 84)?;
                let (lo, hi) = (ms[0], *ms.get(2).unwrap_or(&ms[0]));
                for (j, m) in [lo, hi].iter().enumerate() {
                    let x = self.horn(ctx, *m, beat * (0.5 + j as f64), 0.95, 1.2)?;
                    mix.place(&bus, &Sig::Mono(x), tq + j as f64 * beat * 0.5, 0.28, 0.0);
                }
            } else if is(&cfg.low.name)? {
                mix.place(
                    "fx",
                    &Sig::Mono(synth::thud(0.5, 110.0, 40.0, THUD_SEED)),
                    tq,
                    0.30,
                    0.0,
                );
            }
        }
        Ok(())
    }

    /// A smear down the valves.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let t_hs = ctx.show.t_hs;
        for j in 0..6 {
            let x = self.horn(ctx, 77 - 2 * j, 0.12, 0.7, 0.8)?;
            mix.place(
                &bus,
                &Sig::Mono(x),
                t_hs + 0.08 * j as f64,
                0.20,
                (j as f64 - 2.5) / 6.0,
            );
        }
        Ok(())
    }

    /// A held tonic and its echo off the far wall.
    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let ch = self.first_chord(ctx)?;
        let m = self.tones(ctx, ch, 65, 81)?[0];
        let x = self.horn(ctx, m, 0.9, 1.0, 1.3)?;
        mix.place("solo", &Sig::Mono(echo(&x)), tw + 0.25, 0.16, 0.3);
        mix.place("solo", &Sig::Mono(x), tw, 0.34, 0.0);
        Ok(())
    }

    /// Its solo, note by note; it stops at the reassembly.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts >= grid.t_reassemble - 0.05 {
                break;
            }
            let x = self.horn(ctx, m, (d * grid.beat * 0.9).max(0.08), 0.95, 1.3)?;
            mix.place("solo", &Sig::Mono(x), ts, 0.40, 0.0);
        }
        Ok(())
    }

    /// Its pieces fly home: a run up the tonic chord.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let ch = self.first_chord(ctx)?;
        let mut run = self.tones(ctx, ch, 60, 84)?;
        run.truncate(7);
        let n = run.len() as f64;
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / n);
            let x = self.horn(ctx, m, 0.15, 0.7, 0.6)?;
            mix.place(&bus, &Sig::Mono(x), tj, 0.20, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::echo;

    #[test]
    fn an_echo_keeps_the_length() {
        let x: Vec<f64> = (0..4800).map(|i| bf_core::dmath::sin(i as f64 * 0.2) * 0.5).collect();
        let y = echo(&x);
        assert_eq!(y.len(), x.len());
        assert!(y.iter().all(|v| v.is_finite()));
    }
}
