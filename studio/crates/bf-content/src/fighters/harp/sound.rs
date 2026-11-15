//! HARP's part in the score: #35's `audio.py`, the harp's part only (`harp`, `snap` and its events).
//!
//! Every string it lays is a rising plucked arpeggio on the bar's chord over the next two beats (thinner below 60 and
//! 30 HP; the leader gets an octave double on its top note); every crossing rings that string high with a snap;
//! a PLUCK dart that lands is a high pluck; PLUCK itself a rising 7-note pluck, ANGEL WINGS a shimmer of harmonics,
//! 47 STRINGS a huge glissando down the scale. It falls apart as its strings snapping top to bottom; the stinger is a
//! rolled chord, the solo its notes, the reassembly a rising run an octave up.

use bf_core::dmath::{self, PI};
use bf_core::harmony::HarmonyError;
use bf_core::rng::Generator;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, Samples, event_is_str, event_number};
use bf_sound::synth::{self, THUD_SEED, clip};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "HARP";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// `snap(m, vel)` (#35): a harp string snapping, a sharp pluck bent down from a quarter sharp with a whip of noise,
/// 0.5 s at 48 kHz.
pub fn snap(m: f64, vel: f64) -> Vec<f64> {
    let n = (0.5 * bf_sound::SRF) as usize;
    let t = synth::tt(n);
    let mut rng = Generator::new(m.max(0.0) as u64);
    let raw: Vec<f64> = (0..n).map(|_| rng.normal(0.0, 1.0)).collect();
    let hiss = synth::bp(&raw, 2000.0, 9000.0, 4);
    let mut ph = 0.0;
    t.iter()
        .zip(&hiss)
        .map(|(&t, &h)| {
            let f = synth::mtof(m) * (1.0 + 0.25 * dmath::exp(-t / 0.03));
            ph += f;
            let x =
                dmath::sin(2.0 * PI * ph / bf_sound::SRF) * dmath::exp(-t / 0.09) + 0.5 * h * dmath::exp(-t / 0.015);
            (2.0 * x).tanh() * vel
        })
        .collect()
}

/// HARP's part.
#[derive(Clone, Copy, Debug)]
pub struct HarpSound {
    k: usize,
}

impl HarpSound {
    /// The part of the fighter on side `k`.
    pub fn new(k: usize) -> HarpSound {
        HarpSound { k }
    }

    /// `harp(m, vel, cap)`: one Pixel Pitch harp note held 0.3 s.
    fn harp(&self, ctx: &MusicCtx, m: i32, vel: f64, cap: f64) -> Samples {
        let pp = ctx.cfg[self.k].pp.as_str();
        Ok(ctx.note(pp, f64::from(m), 0.3, cap, vel)?.to_vec())
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

    /// The key's scale from `lo` to `hi`.
    fn scale(&self, ctx: &MusicCtx, lo: i32, hi: i32) -> Vec<i32> {
        let key = ctx.harmony.key_pc;
        let pcs: Vec<i32> = ctx.harmony.scale.iter().map(|s| (key + s).rem_euclid(12)).collect();
        (lo..=hi).filter(|m| pcs.contains(&m.rem_euclid(12))).collect()
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

impl Sound for HarpSound {
    fn k(&self) -> usize {
        self.k
    }

    /// #35's harp bus.
    fn bus_gain(&self) -> f64 {
        0.85
    }

    fn send(&self) -> f64 {
        0.5
    }

    fn band(&self) -> (f64, f64) {
        (250.0, 3000.0)
    }

    /// A sweep up the strings into a big rolled chord.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let ch = self.first_chord(ctx)?;
        let sweep = self.tones(ctx, ch, 50, 86)?;
        let n = sweep.len() as f64;
        for (j, &m) in sweep.iter().enumerate() {
            let x = self.harp(ctx, m, 0.75, 0.8)?;
            let jf = j as f64;
            mix.place(
                &bus,
                &Sig::Mono(x),
                t - 0.12 + 0.12 * jf / n,
                0.12,
                0.5 * dmath::sin(jf * 0.6),
            );
        }
        for (j, &m) in sweep.iter().rev().take(4).enumerate() {
            let x = self.harp(ctx, m, 1.0, 1.4)?;
            mix.place(&bus, &Sig::Mono(x), t + j as f64 * 0.012, 0.22, -0.3 + 0.1 * j as f64);
        }
        Ok(())
    }

    /// A pluck on each count, up the chord.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let ch = match ctx.harmony.chords.get(1) {
            Some(c) => c.as_str(),
            None => self.first_chord(ctx)?,
        };
        let ms = self.tones(ctx, ch, 69, 93)?;
        for (j, (tc, _)) in ctx.show.grid.count.iter().take(3).enumerate() {
            let x = self.harp(ctx, ms[j % ms.len()], 0.9, 1.0)?;
            mix.place(&bus, &Sig::Mono(x), *tc, 0.30, 0.0);
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
            if e.who() != Some(k) || !matches!(e.kind.as_str(), "string" | "hit" | "ability") {
                continue;
            }
            let tq = ctx.q16(t);
            let ch = ctx.harmony.chord_at(tq);
            match e.kind.as_str() {
                "string" => {
                    // a new string = a rising arpeggio over two beats
                    let hp = ctx.hp_at(k, t) / 100.0;
                    let notes = self.tones(ctx, ch, 50, 81)?;
                    let mut arp: Vec<i32> = if notes.len() > 5 {
                        vec![notes[0], notes[2], notes[3], notes[5]]
                    } else {
                        notes.iter().copied().take(4).collect()
                    };
                    if hp < 0.6 {
                        arp = arp.into_iter().step_by(2).collect();
                    }
                    if hp < 0.3 {
                        arp.truncate(1);
                    }
                    let lead = (ctx.hp_at(k, t) - ctx.hp_at(1 - k, t)) / 100.0;
                    let n = arp.len();
                    let gap = if n < 4 { (4 / n.max(1)) as f64 } else { 1.0 };
                    for (j, &m) in arp.iter().enumerate() {
                        let tj = tq + j as f64 * 2.0 * s16 * gap;
                        let x = self.harp(ctx, m, 0.8 + 0.05 * j as f64, 1.0)?;
                        let (sig, g, _) = self.tone(ctx, &x, t, 1.2)?;
                        mix.place(&bus, &Sig::Mono(sig), tj, 0.30 * g, 0.2);
                        if lead > 0.04 && j == n - 1 {
                            let x = self.harp(ctx, m + 12, 0.7, 1.0)?;
                            let sig = self.tone(ctx, &x, t, 1.0)?.0;
                            mix.place(&bus, &Sig::Mono(sig), tj + s16, 0.16 * g, 0.25);
                        }
                    }
                }
                "hit" => {
                    let pan = clip(event_number(H, e, 5)? / 3.5, -0.6, 0.6);
                    let sub = event_number(H, e, 8).unwrap_or(0.0).max(0.0) as usize;
                    if event_is_str(H, e, 7, "string")? {
                        // a crossing rings the string
                        let hi = self.tones(ctx, ch, 67, 88)?;
                        let m = hi[sub % hi.len()];
                        let x = self.harp(ctx, m, 1.0, 1.2)?;
                        let (sig, g, _) = self.tone(ctx, &x, t, 1.0)?;
                        mix.place(&bus, &Sig::Mono(sig), tq, 0.34 * g, pan);
                        mix.place(&bus, &Sig::Mono(snap(f64::from(m + 12), 0.25)), tq, 0.05, pan);
                    } else if event_is_str(H, e, 7, "key")? {
                        // a PLUCK dart lands
                        let ms = self.tones(ctx, ch, 69, 93)?;
                        let x = self.harp(ctx, ms[sub % ms.len().min(5)], 0.9, 0.8)?;
                        let sig = self.tone(ctx, &x, t, 1.0)?.0;
                        mix.place(&bus, &Sig::Mono(sig), tq, 0.26, pan);
                    } else if event_is_str(H, e, 7, "wave")? {
                        mix.place(
                            "fx",
                            &Sig::Mono(synth::thud(0.45, 110.0, 40.0, THUD_SEED)),
                            tq,
                            0.36,
                            0.0,
                        );
                    }
                }
                "ability" => {
                    let is = |name: &str| event_is_str(H, e, 3, name);
                    if is(&cfg.unlock1.name)? {
                        // PLUCK: the fan of darts, a rising 7-note pluck
                        let ms = self.tones(ctx, ch, 62, 93)?;
                        for j in 0..7 {
                            let x = self.harp(ctx, ms[j % ms.len()], 0.85, 1.0)?;
                            let sig = self.tone(ctx, &x, t, 1.0)?.0;
                            let tj = ctx.q16(t + j as f64 * beat / 6.0);
                            mix.place(&bus, &Sig::Mono(sig), tj, 0.24, -0.3 + 0.1 * j as f64);
                        }
                    } else if is(&cfg.unlock2.name)? {
                        // ANGEL WINGS: a shimmer of harmonics high up
                        let ms = self.tones(ctx, ch, 74, 98)?;
                        for j in 0..8 {
                            let x = self.harp(ctx, ms[(j * 2) % ms.len()], 0.6, 1.4)?;
                            let jf = j as f64;
                            mix.place(&bus, &Sig::Mono(x), tq + jf * s16 / 2.0, 0.13, 0.4 * dmath::sin(jf));
                        }
                    } else if is(&cfg.low.name)? {
                        // 47 STRINGS: every string at once, a huge glissando down
                        let ms: Vec<i32> = self.scale(ctx, 43, 96).into_iter().rev().collect();
                        let n = ms.len() as f64;
                        for (j, &m) in ms.iter().enumerate() {
                            let jf = j as f64;
                            let mut x = self.harp(ctx, m, 0.8, 0.9)?;
                            for (o, s) in x.iter_mut().zip(snap(f64::from(m), 0.6)) {
                                *o = *o * 0.6 + s * 0.5;
                            }
                            mix.place(&bus, &Sig::Mono(x), tq + 1.2 * jf / n, 0.16, 0.6 * dmath::sin(jf * 0.7));
                        }
                        mix.place(
                            "fx",
                            &Sig::Mono(synth::thud(0.6, 100.0, 35.0, THUD_SEED)),
                            tq,
                            0.25,
                            0.0,
                        );
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Its strings snapping, top to bottom.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let t_hs = ctx.show.t_hs;
        let ch = self.first_chord(ctx)?;
        let ms: Vec<i32> = self.tones(ctx, ch, 55, 88)?.into_iter().rev().take(7).collect();
        for (j, &m) in ms.iter().enumerate() {
            let jf = j as f64;
            mix.place(
                &bus,
                &Sig::Mono(snap(f64::from(m), 1.0)),
                t_hs + 0.07 * jf,
                0.20,
                (jf - 3.0) / 6.0,
            );
        }
        Ok(())
    }

    /// A rolled chord up the strings.
    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let ch = self.first_chord(ctx)?;
        let s16 = ctx.show.grid.s16;
        for (j, &m) in self.tones(ctx, ch, 62, 86)?.iter().take(6).enumerate() {
            let x = self.harp(ctx, m, 1.0, 1.4)?;
            mix.place("solo", &Sig::Mono(x), tw + j as f64 * s16 / 2.0, 0.30, 0.0);
        }
        Ok(())
    }

    /// Its solo, note by note; it stops at the reassembly.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, _d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts >= grid.t_reassemble - 0.05 {
                break;
            }
            let x = self.harp(ctx, m, 0.95, 1.4)?;
            mix.place("solo", &Sig::Mono(x), ts, 0.40, 0.0);
        }
        Ok(())
    }

    /// Its pieces fly home: a rising run up the tonic chord, an octave up.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let ch = self.first_chord(ctx)?;
        let mut run = self.tones(ctx, ch, 62, 96)?;
        run.truncate(8);
        let n = run.len() as f64;
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / n);
            let x = self.harp(ctx, m, 0.7, 0.6)?;
            mix.place(&bus, &Sig::Mono(x), tj, 0.20, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::snap;

    #[test]
    fn a_snap_is_half_a_second_seeded_by_its_note() {
        let a = snap(74.0, 1.0);
        assert_eq!(a.len(), 24_000);
        assert_eq!(a, snap(74.0, 1.0));
        assert!(a.iter().all(|v| v.abs() <= 1.0));
    }
}
