//! BASS GUITAR's part in the score (a from-scratch fighter, 2026-11-21): a rock bassist with a pick.
//!
//! Every throw of PICK TOSS is a picked root note low in the chord (a second and third pick add the fifth and the
//! octave just after it). FRET SLIDE is a slide up the neck while the line lights, then a hard root and octave when it
//! lands; HEADSTOCK SWING lands as a big low power chord and a thud; SLINGSHOT is the string stretched sharp, a note
//! climbing for as long as it draws back, then the lowest root and a thud when it lands. It falls apart as a slide
//! down the neck; the stinger is a held low root, the solo its notes, the reassembly a run up the chord.

use bf_core::harmony::HarmonyError;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, Samples, event_number};
use bf_sound::synth::{self, THUD_SEED};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "BASS GUITAR";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// BASS GUITAR's part.
#[derive(Clone, Copy, Debug)]
pub struct BassGuitarSound {
    k: usize,
}

impl BassGuitarSound {
    /// The part of the fighter on side `k`.
    pub fn new(k: usize) -> BassGuitarSound {
        BassGuitarSound { k }
    }

    /// One BASS GUITAR note.
    fn bass(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64, cap: f64) -> Samples {
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

    fn thud(&self, mix: &mut Mix, t: f64, gain: f64) {
        mix.place("fx", &Sig::Mono(synth::thud(0.45, 90.0, 38.0, THUD_SEED)), t, gain, 0.0);
    }
}

impl Sound for BassGuitarSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        2.4
    }

    fn send(&self) -> f64 {
        0.2
    }

    fn band(&self) -> (f64, f64) {
        (40.0, 2500.0)
    }

    /// A slide up to the root and a held low note.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let ch = self.first_chord(ctx)?;
        let root = self.tones(ctx, ch, 28, 40)?[0];
        for j in 0..5 {
            let x = self.bass(ctx, root - 5 + j, 0.05, 0.6, 0.8)?;
            mix.place(&bus, &Sig::Mono(x), t - 0.2 + 0.04 * f64::from(j), 0.16, 0.0);
        }
        let x = self.bass(ctx, root, 0.8, 1.0, 1.3)?;
        mix.place(&bus, &Sig::Mono(x), t, 0.36, 0.0);
        Ok(())
    }

    /// A picked root on each count.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let ch = match ctx.harmony.chords.get(1) {
            Some(c) => c.as_str(),
            None => self.first_chord(ctx)?,
        };
        let ms = self.tones(ctx, ch, 28, 52)?;
        for (j, (tc, _)) in ctx.show.grid.count.iter().take(3).enumerate() {
            let x = self.bass(ctx, ms[(j * 2) % ms.len()], 0.25, 0.85, 1.0)?;
            mix.place(&bus, &Sig::Mono(x), *tc, 0.30, 0.0);
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _bed: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if e.who() != Some(k) || !matches!(e.kind.as_str(), "pick" | "fret_slide" | "headstock" | "slingshot") {
                continue;
            }
            let tq = ctx.q16(t);
            let ch = ctx.harmony.chord_at(tq);
            let ms = self.tones(ctx, ch, 28, 52)?;
            let root = ms[0];
            if e.kind == "pick" {
                // PICK TOSS: the root, then the fifth and the octave for the second and third picks
                let n = event_number(H, e, 3)?.clamp(1.0, 3.0) as usize;
                for j in 0..n {
                    let m = match j {
                        0 => root,
                        1 => *ms.get(2).unwrap_or(&(root + 7)),
                        _ => root + 12,
                    };
                    let x = self.bass(ctx, m, beat * 0.8, 0.9 - 0.1 * j as f64, 1.0)?;
                    mix.place(&bus, &Sig::Mono(x), tq + j as f64 * s16, 0.30, 0.0);
                }
                continue;
            }
            let landed = event_number(H, e, 3)? >= 1.0;
            match (e.kind.as_str(), landed) {
                ("fret_slide", false) => {
                    // a slide up the neck while the line lights
                    for j in 0..6 {
                        let x = self.bass(ctx, root + 2 * j, s16 * 0.6, 0.7, 0.9)?;
                        mix.place(&bus, &Sig::Mono(x), tq + f64::from(j) * s16 * 0.33, 0.22, 0.0);
                    }
                }
                ("fret_slide", true) => {
                    for m in [root, root + 12] {
                        let x = self.bass(ctx, m, beat * 0.5, 1.0, 1.2)?;
                        mix.place(&bus, &Sig::Mono(x), tq, 0.30, 0.0);
                    }
                }
                ("headstock", true) => {
                    // a big low power chord: root, fifth, octave
                    for m in [root, root + 7, root + 12] {
                        let x = self.bass(ctx, m, beat * 1.5, 1.0, 1.3)?;
                        mix.place(&bus, &Sig::Mono(x), tq, 0.28, 0.0);
                    }
                    self.thud(mix, tq, 0.30);
                }
                ("slingshot", false) => {
                    // the string stretched sharp: a note climbing for two beats
                    for j in 0..8 {
                        let x = self.bass(ctx, root + j, beat * 0.25, 0.5 + 0.05 * f64::from(j), 0.9)?;
                        mix.place(&bus, &Sig::Mono(x), tq + f64::from(j) * beat * 0.25, 0.20, 0.0);
                    }
                }
                ("slingshot", true) => {
                    let x = self.bass(ctx, root, beat * 2.0, 1.0, 1.4)?;
                    mix.place(&bus, &Sig::Mono(x), tq, 0.36, 0.0);
                    self.thud(mix, tq, 0.36);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// A slide down the neck.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let t_hs = ctx.show.t_hs;
        for j in 0..8 {
            let x = self.bass(ctx, 52 - 3 * j, 0.1, 0.7, 0.8)?;
            mix.place(&bus, &Sig::Mono(x), t_hs + 0.07 * f64::from(j), 0.22, 0.0);
        }
        Ok(())
    }

    /// A held low root.
    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let ch = self.first_chord(ctx)?;
        let m = self.tones(ctx, ch, 28, 40)?[0];
        let x = self.bass(ctx, m, 1.0, 1.0, 1.4)?;
        mix.place("solo", &Sig::Mono(x), tw, 0.40, 0.0);
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
            let x = self.bass(ctx, m, (d * grid.beat * 0.9).max(0.1), 0.95, 1.3)?;
            mix.place("solo", &Sig::Mono(x), ts, 0.42, 0.0);
        }
        Ok(())
    }

    /// Its pieces fly home: a run up the tonic chord.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let ch = self.first_chord(ctx)?;
        let mut run = self.tones(ctx, ch, 28, 55)?;
        run.truncate(7);
        let n = run.len() as f64;
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / n);
            let x = self.bass(ctx, m, 0.18, 0.75, 0.7)?;
            mix.place(&bus, &Sig::Mono(x), tj, 0.24, 0.0);
        }
        Ok(())
    }
}
