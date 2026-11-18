//! TRUMPET's Engine 2 part in the score (E5 shape, after `test_dummy/sound.rs`): it plays its own current-engine
//! voice (`fighters/trumpet/voice.rs`) and reads only the shadow recording (`bf2_direct::shadow`). Every jab it lands
//! plays its line's note on that 16th, louder the harder the hit; the chord of VALVE COMBO+ adds the key's third
//! and fifth above it; a TA-KA COMBO jab gets a grace a 32nd before it ("ta-ka"). HERALD LINE is a rising call on
//! the key's triad, DOUBLE TONGUE+ a run of ta-ka graces, PISTON ROCKETS three rising notes as they set off.
//! The impacts themselves are the shared Engine 2 effects (`bf_sound::music::v2`).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::round_nd;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError};
use bf_sound::traits::{Bed, Played, Sound};

use bf_sound::music::event_is_str;
use bf_sound::voice::pp;

use crate::fighters::trumpet::voice::synth;

/// The name the part's errors carry.
const H: &str = "TRUMPET";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// A note of the cache: `(round(m, 2), round(dur, 3), round(vel, 2))` by their bits.
type Key = (u64, u64, u64);

fn bits(x: f64, nd: i32) -> u64 {
    (round_nd(x, nd) + 0.0).to_bits()
}

/// TRUMPET's Engine 2 part.
#[derive(Debug)]
pub struct TrumpetSound {
    k: usize,
    notes: Mutex<BTreeMap<Key, Arc<[f64]>>>,
}

impl TrumpetSound {
    /// The part of the fighter on side `k`.
    pub fn new(k: usize) -> TrumpetSound {
        TrumpetSound {
            k,
            notes: Mutex::new(BTreeMap::new()),
        }
    }

    /// One note of the trumpet's voice, at 48 kHz, kept by its rounded key.
    fn mar(&self, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        let key = (bits(m, 2), bits(dur, 3), bits(vel, 2));
        let mut notes = self
            .notes
            .lock()
            .map_err(|_| MusicError::new(H, "an earlier note panicked"))?;
        if let Some(x) = notes.get(&key) {
            return Ok(Arc::clone(x));
        }
        let x: Arc<[f64]> = pp::to_48k(&synth(m, vel, dur.max(0.05))).into();
        notes.insert(key, Arc::clone(&x));
        Ok(x)
    }

    /// The key's tonic at the top of its register.
    fn top(&self, ctx: &MusicCtx) -> i32 {
        let hi = ctx.cfg[self.k].register.1;
        hi - (hi - ctx.harmony.key_pc).rem_euclid(12)
    }

    /// The key's tonic an octave under the top: where its line and its figures sit.
    fn home(&self, ctx: &MusicCtx) -> i32 {
        self.top(ctx) - 12
    }

    /// Its line's note at `t`, or `last` on a rest.
    fn line(&self, ctx: &MusicCtx, t: f64, last: i32) -> Result<i32, MusicError> {
        Ok(ctx.line_at(self.name(ctx), t)?.unwrap_or(last))
    }

    /// `x` through its tone at `t` on its bus.
    fn put(&self, ctx: &MusicCtx, mix: &mut Mix, x: &[f64], t: f64, gain: f64, pan: f64) -> Played {
        let (sig, g, _) = self.colour(ctx, x, t, 1.0, &self.colour_args())?;
        mix.place(&self.bus(ctx), &Sig::Mono(sig), t, gain * g, pan);
        Ok(())
    }
}

impl Sound for TrumpetSound {
    fn k(&self) -> usize {
        self.k
    }

    /// The current engine's (`fighters/trumpet/sound.rs`).
    fn bus_gain(&self) -> f64 {
        1.8
    }

    fn send(&self) -> f64 {
        0.5
    }

    fn band(&self) -> (f64, f64) {
        (300.0, 5000.0)
    }

    fn colour_args(&self) -> bf_sound::music::ColourArgs {
        bf_sound::music::ColourArgs {
            cut0: 90.0,
            cut_span: 520.0,
            drive: 4.0,
            ..bf_sound::music::ColourArgs::default()
        }
    }

    fn voice(&self, _ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.mar(m, dur, vel)
    }

    /// The voice's chord on the title hits: the fight file's hits are the chord's intervals above its home tonic
    /// (`[0, 4, 7]`), struck together and rolled up by 30 ms; without any, the tonic triad of the key.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let home = self.home(ctx);
        let hits = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(h) => h.clone(),
            None => vec![0, 3, 7],
        };
        for (j, d) in hits.iter().enumerate() {
            let x = self.mar(f64::from(home + d), 1.2, 1.0)?;
            mix.place(&self.bus(ctx), &Sig::Mono(x.to_vec()), t + 0.03 * j as f64, 0.36, 0.15);
        }
        Ok(())
    }

    /// One line note per count, the last an octave up into FIGHT!.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let beat = ctx.show.grid.beat;
        let mut last = self.home(ctx);
        for (j, &(tc, _)) in ctx.show.grid.count[..3].iter().enumerate() {
            let m = self.line(ctx, tc, last)?;
            last = m;
            let m = if j == 2 { m + 12 } else { m };
            self.put(ctx, mix, &self.mar(f64::from(m), 0.5 * beat, 0.7)?, tc, 0.26, 0.15)?;
        }
        Ok(())
    }

    /// Each of its own hits (the shadow's `hit`, `e[2]` its side) plays the line's note on that 16th, with
    /// `vel = 0.5 + 0.5·strength` (`e[9]`); the combo's chord (strength 0.4 and up) adds the key's third and fifth
    /// above it, and a TA-KA COMBO hit a grace a 32nd before. Its abilities (`e[3]` the move) play their calls.
    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko + 0.01;
        let beat = ctx.show.grid.beat;
        let home = self.home(ctx);
        let mut last = home;
        let scale = ctx.harmony.scale_notes(home - 12, home + 24, None).map_err(bad)?;
        // the scale note `steps` scale degrees above `m` (or `m` itself off the scale)
        let above = |m: i32, steps: usize| -> i32 {
            scale
                .iter()
                .position(|&n| n >= m)
                .and_then(|i| scale.get(i + steps))
                .copied()
                .unwrap_or(m)
        };
        for e in &ctx.rec.events {
            if e.t > t_ko {
                break;
            }
            if e.who() != Some(self.k) {
                continue;
            }
            if e.kind == "ability" {
                let call: &[(f64, usize)] = if event_is_str(H, e, 3, "HERALD LINE")? {
                    &[(0.0, 0), (0.5, 2), (1.0, 4), (1.5, 7)]
                } else if event_is_str(H, e, 3, "DOUBLE TONGUE+")? {
                    &[(0.0, 4), (0.25, 4), (0.5, 4), (0.75, 4), (1.0, 7)]
                } else if event_is_str(H, e, 3, "PISTON ROCKETS")? {
                    &[(0.7, 0), (1.2, 2), (1.7, 4)]
                } else {
                    &[]
                };
                let t0 = ctx.q16(e.t);
                for (j, &(at, deg)) in call.iter().enumerate() {
                    let m = above(home, deg);
                    let long = j + 1 == call.len();
                    let d = if long { 0.9 * beat } else { 0.22 * beat };
                    self.put(ctx, mix, &self.mar(f64::from(m), d, 0.8)?, t0 + at * beat, 0.28, 0.15)?;
                }
                continue;
            }
            if e.kind != "hit" {
                continue;
            }
            let strength = if e.fields.len() > 7 {
                e.f(9).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let tq = ctx.q16(e.t);
            let m = self.line(ctx, tq, last)?;
            last = m;
            let vel = 0.5 + 0.5 * strength;
            if strength < 0.1 {
                // a ghost jab: only the grace, soft
                self.put(ctx, mix, &self.mar(f64::from(m), 0.1 * beat, 0.45)?, tq, 0.18, 0.15)?;
                continue;
            }
            if event_is_str(H, e, 7, "TA-KA COMBO")? {
                let g = self.mar(f64::from(above(m, 1)), 0.1 * beat, 0.5)?;
                self.put(ctx, mix, &g, tq - beat / 8.0, 0.2, 0.15)?;
            }
            self.put(ctx, mix, &self.mar(f64::from(m), 0.6 * beat, vel)?, tq, 0.30, 0.15)?;
            if strength >= 0.4 {
                for (j, steps) in [2usize, 4].into_iter().enumerate() {
                    let x = self.mar(f64::from(above(m, steps)), 0.6 * beat, vel * 0.85)?;
                    self.put(ctx, mix, &x, tq + 0.012 * (j + 1) as f64, 0.22, 0.15)?;
                }
            }
        }
        Ok(())
    }

    /// It lost: a falling run down an octave of the scale.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let top = self.top(ctx);
        let notes = ctx.harmony.scale_notes(top - 12, top, None).map_err(bad)?;
        for (j, m) in notes.iter().rev().enumerate() {
            let x = self.mar(f64::from(*m), 0.1, 0.8 - 0.04 * j as f64)?;
            mix.place(
                &self.bus(ctx),
                &Sig::Mono(x.to_vec()),
                ctx.show.t_hs + j as f64 * 0.05,
                0.24,
                0.0,
            );
        }
        Ok(())
    }

    /// It won: the tonic, struck in two octaves.
    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let home = f64::from(self.home(ctx));
        for (j, m) in [home, home + 12.0].into_iter().enumerate() {
            let x = self.mar(m, 0.8, 1.0)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), tw + j as f64 * 0.03, 0.36, 0.0);
        }
        Ok(())
    }

    /// It won: the solo (the composer's, or the fight file's), each chord's notes together, stopping at the
    /// reassembly.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, notes, d) in ctx.solo_entries(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            for &m in notes.notes() {
                let x = self.mar(f64::from(m), d * grid.beat, 0.95)?;
                mix.place("solo", &Sig::Mono(x.to_vec()), ts, 0.46, 0.0);
            }
        }
        Ok(())
    }

    /// It lost: a run up the scale as it reassembles.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let home = self.home(ctx);
        let run = ctx.harmony.scale_notes(home, home + 12, None).map_err(bad)?;
        for (j, m) in run.iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            let x = self.mar(f64::from(*m), 0.14, 0.7)?;
            mix.place(&self.bus(ctx), &Sig::Mono(x.to_vec()), tj, 0.20, 0.0);
        }
        Ok(())
    }

    /// Its line in quarters into the frame-0 slam.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let t30 = (grid.n_bars as f64 - 2.0) * grid.bar;
        let mut last = self.home(ctx);
        for j in 1..8 {
            let tb = t30 + f64::from(j) * grid.beat;
            let m = self.line(ctx, tb, last)?;
            last = m;
            let x = self.mar(f64::from(m), 0.5 * grid.beat, 0.8)?;
            mix.place(&self.bus(ctx), &Sig::Mono(x.to_vec()), tb, 0.26, 0.15);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_is_kept_by_its_rounded_key() {
        let part = TrumpetSound::new(1);
        let x = part.mar(72.0, 0.3, 0.85).unwrap();
        assert!(Arc::ptr_eq(&x, &part.mar(72.001, 0.3004, 0.851).unwrap()));
        assert_eq!((part.k(), part.enemy()), (1, 0));
    }
}
