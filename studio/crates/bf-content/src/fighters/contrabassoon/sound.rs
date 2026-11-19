//! CONTRABASSOON's part in the score (made from scratch, Nov 16, 2026).
//!
//! Every pipe it lays is a low staccato note on the bar's root with a hollow clank; every pipe bounce that lands
//! is a grumbling chord tone and a clank, higher with each one; a WATER KEY spray that lands a hiss. WATER KEY
//! itself is a gurgle of quick notes, HOT PIPES a riff stepping up in fourths, FIVE AND A HALF METRES a long run
//! down in fourths to the bottom with a thud. It falls apart as its pipes clanking down; the stinger is a low
//! chord, the solo its notes, the reassembly a rising run.

use bf_core::dmath::{self, PI};
use bf_core::harmony::HarmonyError;
use bf_core::rng::Generator;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, Samples, event_is_str, event_number};
use bf_sound::synth::{self, THUD_SEED, clip};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "CONTRABASSOON";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// A hollow metal clank: three inharmonic partials of a tube struck at `f` Hz, a short knock of noise, 0.35 s at
/// 48 kHz, seeded by its pitch.
pub fn clank(f: f64, vel: f64) -> Vec<f64> {
    let n = (0.35 * bf_sound::SRF) as usize;
    let t = synth::tt(n);
    let mut rng = Generator::new(f.max(0.0) as u64 + 11);
    let raw: Vec<f64> = (0..n).map(|_| rng.normal(0.0, 1.0)).collect();
    let knock = synth::bp(&raw, 600.0, 4000.0, 2);
    t.iter()
        .zip(&knock)
        .map(|(&t, &k)| {
            let ring = [(1.0, 1.0, 0.16), (2.76, 0.5, 0.09), (5.40, 0.25, 0.05)]
                .iter()
                .map(|&(m, a, d)| a * dmath::sin(2.0 * PI * f * m * t) * dmath::exp(-t / d))
                .sum::<f64>();
            dmath::tanh(1.5 * (ring + 0.6 * k * dmath::exp(-t / 0.01))) * vel
        })
        .collect()
}

/// A spray of water: band-passed noise that swells and fades, `dur` s at 48 kHz.
pub fn hiss(dur: f64, vel: f64, seed: u64) -> Vec<f64> {
    let n = (dur * bf_sound::SRF) as usize;
    let t = synth::tt(n);
    let mut rng = Generator::new(seed);
    let raw: Vec<f64> = (0..n).map(|_| rng.normal(0.0, 1.0)).collect();
    let h = synth::bp(&raw, 2500.0, 9000.0, 2);
    t.iter()
        .zip(&h)
        .map(|(&t, &x)| x * vel * (t / 0.03).min(1.0) * dmath::exp(-t / (0.4 * dur)))
        .collect()
}

/// CONTRABASSOON's part.
#[derive(Clone, Copy, Debug)]
pub struct ContrabassoonSound {
    k: usize,
}

impl ContrabassoonSound {
    /// The part of the fighter on side `k`.
    pub fn new(k: usize) -> ContrabassoonSound {
        ContrabassoonSound { k }
    }

    /// One note of its own voice held `dur` s.
    fn reed(&self, ctx: &MusicCtx, m: i32, vel: f64, dur: f64) -> Samples {
        let pp = ctx.cfg[self.k].pp.as_str();
        Ok(ctx.note(pp, f64::from(m), dur, 1.0, vel)?.to_vec())
    }

    /// The chord's tones from `lo` to `hi`, refused when empty.
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

    /// The key's scale from `lo` to `hi`.
    fn scale(&self, ctx: &MusicCtx, lo: i32, hi: i32) -> Vec<i32> {
        let key = ctx.harmony.key_pc;
        let pcs: Vec<i32> = ctx.harmony.scale.iter().map(|s| (key + s).rem_euclid(12)).collect();
        (lo..=hi).filter(|m| pcs.contains(&m.rem_euclid(12))).collect()
    }

    /// Steps of a fourth from `from` toward `to` (down when `to` is lower), each snapped into the key.
    fn fourths(&self, ctx: &MusicCtx, from: i32, to: i32) -> Vec<i32> {
        let sc = self.scale(ctx, 22, 80);
        let snap = |m: i32| *sc.iter().min_by_key(|s| (*s - m).abs()).unwrap_or(&m);
        let step = if to < from { -5 } else { 5 };
        let mut out = Vec::new();
        let mut m = from;
        while (step < 0 && m >= to) || (step > 0 && m <= to) {
            out.push(snap(m));
            m += step;
        }
        out
    }

    /// `self.colour(x, t, drive_k)`.
    fn tone(&self, ctx: &MusicCtx, x: &[f64], t: f64, drive_k: f64) -> Result<(Vec<f64>, f64, f64), MusicError> {
        self.colour(ctx, x, t, drive_k, &self.colour_args())
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

impl Sound for ContrabassoonSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        0.9
    }

    fn send(&self) -> f64 {
        0.35
    }

    fn band(&self) -> (f64, f64) {
        (60.0, 1800.0)
    }

    /// A low chord stab with a clank on top.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let ch = self.first_chord(ctx)?;
        for (j, &m) in self.tones(ctx, ch, 34, 58)?.iter().take(3).enumerate() {
            let x = self.reed(ctx, m, 1.0, 0.6)?;
            mix.place(&bus, &Sig::Mono(x), t + j as f64 * 0.01, 0.34, -0.2 + 0.2 * j as f64);
        }
        mix.place(&bus, &Sig::Mono(clank(220.0, 0.8)), t, 0.18, 0.0);
        Ok(())
    }

    /// A low staccato note on each count, up the chord.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let ch = match ctx.harmony.chords.get(1) {
            Some(c) => c.as_str(),
            None => self.first_chord(ctx)?,
        };
        let ms = self.tones(ctx, ch, 34, 58)?;
        for (j, (tc, _)) in ctx.show.grid.count.iter().take(3).enumerate() {
            let x = self.reed(ctx, ms[j % ms.len()], 0.9, 0.12)?;
            mix.place(&bus, &Sig::Mono(x), *tc, 0.36, 0.0);
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _bed: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let cfg = ctx.cfg[k];
        let mut landed = 0usize;
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if e.who() != Some(k) || !matches!(e.kind.as_str(), "pipe" | "hit" | "ability") {
                continue;
            }
            let tq = ctx.q16(t);
            let ch = ctx.harmony.chord_at(tq);
            match e.kind.as_str() {
                "pipe" => {
                    // a new pipe: the bar's lowest chord tone, short, and a clank
                    let m = self.tones(ctx, ch, 34, 46)?[0];
                    let x = self.reed(ctx, m, 0.85, 0.14)?;
                    let (sig, g, _) = self.tone(ctx, &x, t, 1.1)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.34 * g, 0.0);
                    mix.place(
                        "fx",
                        &Sig::Mono(clank(synth::mtof(f64::from(m + 36)), 0.6)),
                        tq,
                        0.08,
                        0.0,
                    );
                }
                "hit" => {
                    let pan = clip(event_number(H, e, 5)? / 3.5, -0.6, 0.6);
                    if event_is_str(H, e, 7, "pipe")? || event_is_str(H, e, 7, "pipe_box")? {
                        // a bounce that lands: a grumbling chord tone, a little higher each time
                        let ms = self.tones(ctx, ch, 34, 58)?;
                        let m = ms[landed % ms.len()];
                        landed += 1;
                        let x = self.reed(ctx, m, 1.0, 0.22)?;
                        let (sig, g, _) = self.tone(ctx, &x, t, 1.3)?;
                        mix.place(&bus, &Sig::Mono(sig), tq, 0.36 * g, pan);
                        let f = synth::mtof(f64::from(m + 31));
                        mix.place("fx", &Sig::Mono(clank(f, 0.9)), tq, 0.12, pan);
                    } else if event_is_str(H, e, 7, "spray")? {
                        mix.place("fx", &Sig::Mono(hiss(0.35, 0.7, 5)), tq, 0.14, pan);
                    }
                }
                "ability" => {
                    let is = |name: &str| event_is_str(H, e, 3, name);
                    if is(&cfg.unlock1.name)? {
                        // WATER KEY: a gurgle of quick notes over a hiss
                        let ms = self.tones(ctx, ch, 46, 62)?;
                        for j in 0..6 {
                            let x = self.reed(ctx, ms[(j * 2) % ms.len()], 0.7, 0.05)?;
                            let tj = tq + j as f64 * s16 / 2.0;
                            mix.place(&bus, &Sig::Mono(x), tj, 0.2, 0.3 * dmath::sin(j as f64));
                        }
                        mix.place("fx", &Sig::Mono(hiss(beat, 0.8, 9)), tq, 0.16, 0.0);
                    } else if is(&cfg.unlock2.name)? {
                        // HOT PIPES: up in fourths, one a sixteenth
                        let lo = self.tones(ctx, ch, 34, 46)?[0];
                        for (j, m) in self.fourths(ctx, lo, lo + 20).into_iter().enumerate() {
                            let x = self.reed(ctx, m, 0.95, 0.16)?;
                            mix.place(&bus, &Sig::Mono(x), tq + j as f64 * s16, 0.3, 0.0);
                        }
                    } else if is(&cfg.low.name)? {
                        // FIVE AND A HALF METRES: a long run down in fourths to the bottom, and a thud
                        let run = self.fourths(ctx, 62, 22);
                        let n = run.len() as f64;
                        for (j, &m) in run.iter().enumerate() {
                            let x = self.reed(ctx, m, 1.0, 0.25)?;
                            let tj = ctx.q16(t + 2.0 * beat * j as f64 / n);
                            mix.place(&bus, &Sig::Mono(x), tj, 0.32, 0.0);
                        }
                        let thud = synth::thud(0.6, 90.0, 30.0, THUD_SEED);
                        mix.place("fx", &Sig::Mono(thud), ctx.q16(t + 2.0 * beat), 0.3, 0.0);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Its pipes clanking down, one after another.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let t_hs = ctx.show.t_hs;
        for j in 0..6 {
            let jf = j as f64;
            let f = 440.0 * dmath::exp(-0.25 * jf);
            mix.place(
                &bus,
                &Sig::Mono(clank(f, 1.0)),
                t_hs + 0.09 * jf,
                0.16,
                (jf - 2.5) / 5.0,
            );
        }
        Ok(())
    }

    /// A low chord, rolled.
    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let ch = self.first_chord(ctx)?;
        let s16 = ctx.show.grid.s16;
        for (j, &m) in self.tones(ctx, ch, 34, 60)?.iter().take(4).enumerate() {
            let x = self.reed(ctx, m, 1.0, 0.8)?;
            mix.place("solo", &Sig::Mono(x), tw + j as f64 * s16 / 2.0, 0.32, 0.0);
        }
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
            let x = self.reed(ctx, m, 0.95, (d * grid.beat * 0.9).max(0.08))?;
            mix.place("solo", &Sig::Mono(x), ts, 0.42, 0.0);
        }
        Ok(())
    }

    /// Its pieces fly home: a rising run up the tonic chord.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let ch = self.first_chord(ctx)?;
        let mut run = self.tones(ctx, ch, 34, 62)?;
        run.truncate(8);
        let n = run.len() as f64;
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / n);
            let x = self.reed(ctx, m, 0.7, 0.12)?;
            mix.place(&bus, &Sig::Mono(x), tj, 0.24, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clank_and_a_hiss_are_short_and_loud_enough() {
        let c = clank(220.0, 1.0);
        assert_eq!(c.len(), (0.35 * bf_sound::SRF) as usize);
        assert!(c.iter().fold(0.0_f64, |a, v| a.max(v.abs())) > 0.3);
        let h = hiss(0.35, 0.7, 5);
        assert!(h.iter().fold(0.0_f64, |a, v| a.max(v.abs())) > 0.01);
    }
}
