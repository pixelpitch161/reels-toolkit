//! CORNET's part in the score (a from-scratch fighter, 2026-11-16): hot-jazz cornet with a plunger.
//!
//! Every cup that sticks is a short "pop" and a held chord tone; every wah is a plunger-muted note that opens and
//! closes ("wa-ah"), climbing the chord as the wahs go on; WA-WA a quick wah-wah trill; DERBY HAT a falling bend and,
//! on a landing, a muffled thump; each STOP-TIME stop a two-note band stab and the break a rising run. It falls apart
//! as a squeezed half-valve smear; the stinger is a held tonic, the solo its notes, the reassembly a run up the chord.

use bf_core::dmath::{self, PI};
use bf_core::harmony::HarmonyError;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, Samples, event_is_str, event_number};
use bf_sound::synth::{self, THUD_SEED, clip};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "CORNET";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// A plunger wah over a note: the note blended from a dark band-passed copy to its open self and back, opening over
/// `open` seconds then closing ("wa-ah").
pub fn wah(x: &[f64], open: f64) -> Vec<f64> {
    let dark = synth::bp(x, 180.0, 900.0, 2);
    let n = x.len();
    let t = synth::tt(n);
    (0..n)
        .map(|i| {
            let u = (t[i] / (2.0 * open.max(1e-3))).min(1.0);
            let a = dmath::sin(PI * u).max(0.0);
            dark[i] * (1.0 - a) * 1.6 + x[i] * a
        })
        .collect()
}

/// CORNET's part.
#[derive(Clone, Copy, Debug)]
pub struct CornetSound {
    k: usize,
}

impl CornetSound {
    /// The part of the fighter on side `k`.
    pub fn new(k: usize) -> CornetSound {
        CornetSound { k }
    }

    /// One CORNET note.
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

    /// The note of an ordinary cup or wah at `t`. Under the Composed arrangement it is the written line's note there
    /// (sound upgrade: every pitched fighter plays its line); under Classic, and on a rest of the line, the kit's
    /// own chord tone `kit`.
    fn line(&self, ctx: &MusicCtx, t: f64, kit: i32) -> Result<i32, MusicError> {
        if ctx.composed_on()
            && let Some(m) = ctx.play_line(self.name(ctx), t)?
        {
            return Ok(m);
        }
        Ok(kit)
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

impl Sound for CornetSound {
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

    /// A muted note on each count.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let ch = match ctx.harmony.chords.get(1) {
            Some(c) => c.as_str(),
            None => self.first_chord(ctx)?,
        };
        let ms = self.tones(ctx, ch, 62, 79)?;
        for (j, (tc, _)) in ctx.show.grid.count.iter().take(3).enumerate() {
            let x = wah(&self.horn(ctx, ms[j % ms.len()], 0.2, 0.8, 1.0)?, 0.08);
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
        // Composed (sound upgrade): a horn sounds one note at a time, so an ordinary note ends the one before it
        let one = ctx.one_voice(self.name(ctx));
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if e.who() != Some(k) {
                continue;
            }
            let kind = e.kind.as_str();
            if !matches!(
                kind,
                "plunger_stick" | "wah" | "ability" | "derby_land" | "stop" | "break" | "hit"
            ) {
                continue;
            }
            let tq = ctx.q16(t);
            let ch = ctx.harmony.chord_at(tq);
            match kind {
                "plunger_stick" => {
                    // the cup bites: a pop, then a muted chord tone
                    mix.place(
                        "fx",
                        &Sig::Mono(synth::thud(0.12, 420.0, 160.0, THUD_SEED)),
                        tq,
                        0.18,
                        0.0,
                    );
                    // (Composed: the line's note in place of the chord tone)
                    let ms = self.tones(ctx, ch, 60, 74)?;
                    let m = self.line(ctx, tq, ms[0])?;
                    let x = wah(&self.horn(ctx, m, beat * 0.9, 0.8, 1.0)?, beat * 0.3);
                    mix.note(&bus, &Sig::Mono(x), tq, 0.24, 0.1, one);
                }
                "wah" => {
                    let n = event_number(H, e, 3)?.max(1.0) as usize;
                    let ms = self.tones(ctx, ch, 62, 81)?;
                    // (Composed: the line's note in place of the climb up the chord)
                    let m = self.line(ctx, tq, ms[(n - 1) % ms.len()])?;
                    let x = wah(&self.horn(ctx, m, beat * 0.7, 0.9, 1.1)?, beat * 0.22);
                    mix.note(&bus, &Sig::Mono(x), tq, 0.30, 0.15, one);
                }
                "derby_land" => {
                    if event_number(H, e, 5)? > 0.5 {
                        mix.place(
                            "fx",
                            &Sig::Mono(synth::thud(0.3, 140.0, 60.0, THUD_SEED)),
                            tq,
                            0.28,
                            0.0,
                        );
                    }
                }
                "stop" => {
                    // a band stab: root and fifth, short and hard
                    let ms = self.tones(ctx, ch, 58, 77)?;
                    for (j, &m) in ms.iter().take(2).enumerate() {
                        let x = self.horn(ctx, m, s16 * 1.5, 1.0, 1.3)?;
                        mix.place(&bus, &Sig::Mono(x), tq, 0.28, -0.2 + 0.4 * j as f64);
                    }
                }
                "break" => {
                    let ms = self.tones(ctx, ch, 62, 86)?;
                    for (j, &m) in ms.iter().take(5).enumerate() {
                        let x = self.horn(ctx, m, s16 * 1.2, 0.95, 1.2)?;
                        mix.place(&bus, &Sig::Mono(x), tq + j as f64 * s16 * 0.5, 0.26, 0.0);
                    }
                    mix.place(
                        "fx",
                        &Sig::Mono(synth::thud(0.5, 110.0, 40.0, THUD_SEED)),
                        tq,
                        0.30,
                        0.0,
                    );
                }
                "hit" => {
                    if event_is_str(H, e, 7, "stoptime")? {
                        let pan = clip(event_number(H, e, 5)? / 3.5, -0.6, 0.6);
                        mix.place(
                            "fx",
                            &Sig::Mono(synth::thud(0.2, 300.0, 90.0, THUD_SEED)),
                            tq,
                            0.22,
                            pan,
                        );
                    }
                }
                "ability" => {
                    let is = |name: &str| event_is_str(H, e, 3, name);
                    if is(&cfg.unlock1.name)? {
                        // WA-WA: a wah-wah trill
                        let ms = self.tones(ctx, ch, 65, 81)?;
                        for j in 0..4 {
                            let m = ms[j % 2 % ms.len()];
                            let x = wah(&self.horn(ctx, m, beat * 0.45, 0.85, 1.0)?, beat * 0.12);
                            mix.place(&bus, &Sig::Mono(x), tq + j as f64 * beat * 0.5, 0.26, 0.0);
                        }
                    } else if is(&cfg.unlock2.name)? {
                        // DERBY HAT: a falling bend as the hat flies
                        let ms = self.tones(ctx, ch, 62, 81)?;
                        let (hi, lo) = (*ms.last().unwrap_or(&74), ms[0]);
                        for (j, m) in [hi, hi - 1, hi - 3, lo].iter().enumerate() {
                            let x = self.horn(ctx, *m, s16 * 1.4, 0.8, 1.0)?;
                            mix.place(&bus, &Sig::Mono(x), tq + j as f64 * s16, 0.22, 0.1);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// A squeezed half-valve smear down.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let t_hs = ctx.show.t_hs;
        for j in 0..6 {
            let x = wah(&self.horn(ctx, 74 - 2 * j, 0.12, 0.7, 0.8)?, 0.05);
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

    /// A held tonic, opened from the plunger.
    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let ch = self.first_chord(ctx)?;
        let m = self.tones(ctx, ch, 67, 79)?[0];
        let x = wah(&self.horn(ctx, m, 0.9, 1.0, 1.3)?, 0.3);
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
    use super::wah;

    #[test]
    fn a_wah_keeps_the_length_and_opens_in_the_middle() {
        let x: Vec<f64> = (0..4800).map(|i| bf_core::dmath::sin(i as f64 * 0.2) * 0.5).collect();
        let y = wah(&x, 0.05);
        assert_eq!(y.len(), x.len());
        assert!(y.iter().all(|v| v.is_finite()));
    }
}
