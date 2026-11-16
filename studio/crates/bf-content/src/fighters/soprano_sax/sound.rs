//! SOPRANO SAX's part in the score (a from-scratch fighter, 2026-11-16): a bright, buzzing soprano that darts like a
//! hornet.
//!
//! Every launch is a quick two-note trill (one more note for every hornet sent), every sting a short, high stab on a
//! chord tone, every new hornet a grace note up; KICK THE HIVE a buzzing chromatic run, BUZZ OFF a tremolo between a
//! chord tone and its upper neighbour, QUEEN a proud descending call. It falls apart as a squealing chromatic fall; the
//! stinger is a held high tonic with a flutter, the solo its notes, the reassembly a run up the chord.

use bf_core::harmony::HarmonyError;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, Samples, event_is_str, event_number};
use bf_sound::synth::{self, THUD_SEED};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "SOPRANO SAX";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// SOPRANO SAX's part.
#[derive(Clone, Copy, Debug)]
pub struct SopranoSaxSound {
    k: usize,
}

impl SopranoSaxSound {
    /// The part of the fighter on side `k`.
    pub fn new(k: usize) -> SopranoSaxSound {
        SopranoSaxSound { k }
    }

    /// One SOPRANO SAX note.
    fn reed(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64, cap: f64) -> Samples {
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

impl Sound for SopranoSaxSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.1
    }

    fn send(&self) -> f64 {
        0.35
    }

    fn band(&self) -> (f64, f64) {
        (400.0, 7000.0)
    }

    /// A quick run up the chord into a held, squealing top note.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let ch = self.first_chord(ctx)?;
        let ms = self.tones(ctx, ch, 70, 88)?;
        let n = ms.len() as f64;
        for (j, &m) in ms.iter().enumerate() {
            let x = self.reed(ctx, m, 0.08, 0.7, 0.8)?;
            mix.place(&bus, &Sig::Mono(x), t - 0.15 + 0.15 * j as f64 / n, 0.14, 0.0);
        }
        let top = *ms.last().unwrap_or(&82);
        let x = self.reed(ctx, top, 0.6, 1.0, 1.2)?;
        mix.place(&bus, &Sig::Mono(x), t, 0.30, 0.0);
        Ok(())
    }

    /// A short buzzing note on each count.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let ch = match ctx.harmony.chords.get(1) {
            Some(c) => c.as_str(),
            None => self.first_chord(ctx)?,
        };
        let ms = self.tones(ctx, ch, 72, 86)?;
        for (j, (tc, _)) in ctx.show.grid.count.iter().take(3).enumerate() {
            let x = self.reed(ctx, ms[j % ms.len()], 0.12, 0.8, 1.0)?;
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
            if e.who() != Some(k) {
                continue;
            }
            let kind = e.kind.as_str();
            if !matches!(kind, "hornet" | "swarm_grow" | "ability" | "hit") {
                continue;
            }
            let tq = ctx.q16(t);
            let ch = ctx.harmony.chord_at(tq);
            match kind {
                "hornet" => {
                    // the launch: a quick trill, one more note for every hornet sent
                    let n = event_number(H, e, 3)?.clamp(1.0, 4.0) as usize;
                    let ms = self.tones(ctx, ch, 70, 86)?;
                    let m = ms[0];
                    for j in 0..(n + 1) {
                        let x = self.reed(ctx, m + (j % 2) as i32, s16 * 0.45, 0.7, 0.8)?;
                        mix.place(&bus, &Sig::Mono(x), tq + j as f64 * s16 * 0.5, 0.20, 0.25);
                    }
                }
                "swarm_grow" => {
                    let size = event_number(H, e, 3)?.max(1.0) as usize;
                    let ms = self.tones(ctx, ch, 72, 88)?;
                    let m = ms[size % ms.len()];
                    let x = self.reed(ctx, m, s16 * 0.6, 0.75, 0.8)?;
                    mix.place(&bus, &Sig::Mono(x), tq, 0.18, -0.2);
                }
                "hit" => {
                    let what = ["sting", "hive", "buzz", "queen"];
                    let mut stung = false;
                    for w in what {
                        stung |= event_is_str(H, e, 7, w)?;
                    }
                    if stung {
                        // a sting: a short, high stab and a tiny click
                        let ms = self.tones(ctx, ch, 74, 88)?;
                        let m = *ms.last().unwrap_or(&79);
                        let x = self.reed(ctx, m, s16 * 0.8, 0.95, 1.1)?;
                        mix.place(&bus, &Sig::Mono(x), tq, 0.24, 0.1);
                        mix.place(
                            "fx",
                            &Sig::Mono(synth::thud(0.05, 1800.0, 900.0, THUD_SEED)),
                            tq,
                            0.10,
                            0.0,
                        );
                    }
                }
                "ability" => {
                    let is = |name: &str| event_is_str(H, e, 3, name);
                    if is(&cfg.unlock1.name)? {
                        // KICK THE HIVE: a buzzing chromatic run up to a chord tone
                        let ms = self.tones(ctx, ch, 74, 86)?;
                        let top = ms[0];
                        for j in 0..8 {
                            let x = self.reed(ctx, top - 7 + j, s16 * 0.5, 0.85, 1.0)?;
                            mix.place(&bus, &Sig::Mono(x), tq + j as f64 * s16 * 0.5, 0.24, 0.0);
                        }
                    } else if is(&cfg.unlock2.name)? {
                        // BUZZ OFF: a tremolo between a chord tone and its upper neighbour, two beats
                        let ms = self.tones(ctx, ch, 70, 84)?;
                        let m = ms[0];
                        for j in 0..16 {
                            let x = self.reed(ctx, m + (j % 2) as i32 * 2, s16 * 0.45, 0.6, 0.7)?;
                            mix.place(&bus, &Sig::Mono(x), tq + j as f64 * beat / 8.0, 0.18, 0.0);
                        }
                    } else if is(&cfg.low.name)? {
                        // QUEEN: a proud descending call down the chord, the last note held
                        let ms = self.tones(ctx, ch, 66, 86)?;
                        let mut down: Vec<i32> = ms.iter().rev().take(4).copied().collect();
                        if down.is_empty() {
                            down.push(74);
                        }
                        let n = down.len();
                        for (j, m) in down.into_iter().enumerate() {
                            let d = if j + 1 == n { beat * 1.5 } else { s16 * 1.6 };
                            let x = self.reed(ctx, m, d, 1.0, 1.2)?;
                            mix.place(&bus, &Sig::Mono(x), tq + j as f64 * s16 * 2.0, 0.30, 0.0);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// A squealing chromatic fall.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let t_hs = ctx.show.t_hs;
        for j in 0..7 {
            let x = self.reed(ctx, 86 - 2 * j, 0.1, 0.75, 0.8)?;
            mix.place(
                &bus,
                &Sig::Mono(x),
                t_hs + 0.07 * j as f64,
                0.20,
                (j as f64 - 3.0) / 7.0,
            );
        }
        Ok(())
    }

    /// A held high tonic with a flutter.
    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let ch = self.first_chord(ctx)?;
        let m = self.tones(ctx, ch, 74, 86)?[0];
        for j in 0..3 {
            let x = self.reed(ctx, m + (j % 2) as i32, 0.06, 0.8, 0.9)?;
            mix.place("solo", &Sig::Mono(x), tw + 0.05 * j as f64, 0.26, 0.0);
        }
        let x = self.reed(ctx, m, 0.9, 1.0, 1.3)?;
        mix.place("solo", &Sig::Mono(x), tw + 0.15, 0.34, 0.0);
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
            let x = self.reed(ctx, m, (d * grid.beat * 0.9).max(0.08), 0.95, 1.3)?;
            mix.place("solo", &Sig::Mono(x), ts, 0.40, 0.0);
        }
        Ok(())
    }

    /// Its pieces fly home: a run up the tonic chord.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let ch = self.first_chord(ctx)?;
        let mut run = self.tones(ctx, ch, 70, 90)?;
        run.truncate(7);
        let n = run.len() as f64;
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / n);
            let x = self.reed(ctx, m, 0.15, 0.7, 0.6)?;
            mix.place(&bus, &Sig::Mono(x), tj, 0.20, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::SopranoSaxSound;
    use bf_sound::traits::Sound;

    #[test]
    fn the_part_sits_on_its_side() {
        assert_eq!(SopranoSaxSound::new(1).k(), 1);
    }
}
