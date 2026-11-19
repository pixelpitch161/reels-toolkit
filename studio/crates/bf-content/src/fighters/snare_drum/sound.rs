//! SNARE DRUM's part in the score (a Studio fighter: written for it, no kit part to match). What you see is what you
//! hear: every stroke of its paradiddle is a snare tap on that 16th (the accents louder, left and right panned a
//! little apart); under FLAM each stroke gets a quiet grace tap a hair early; RIMSHOT's arming is a stick click and a
//! landed rimshot a dry crack; BUZZ ROLL is a press roll as long as the disc. Notes come from its own voice ([`hit`],
//! [`roll`]) through a cache of its own.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::py::round_nd;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{ColourArgs, MusicCtx, MusicError};
use bf_sound::traits::{Bed, Played, Sound};

use super::state;
use super::voice::{Stroke, hit, roll};

/// The name the part's errors carry.
const H: &str = "SNARE DRUM";

/// The head's note.
const HEAD: f64 = 72.0;

/// A note of the cache: `(round(m, 2), round(dur, 3), round(vel, 2), rim)` by their bits.
type Key = (u64, u64, u64, bool);

fn bits(x: f64, nd: i32) -> u64 {
    (round_nd(x, nd) + 0.0).to_bits()
}

/// SNARE DRUM's part.
#[derive(Debug)]
pub struct SnareDrumSound {
    k: usize,
    notes: Mutex<BTreeMap<Key, Arc<[f64]>>>,
}

impl SnareDrumSound {
    /// The part of the fighter on side `k`.
    pub fn new(k: usize) -> SnareDrumSound {
        SnareDrumSound {
            k,
            notes: Mutex::new(BTreeMap::new()),
        }
    }

    /// One stroke (`dur` 0) or a press roll (`dur` > 0), kept by its rounded key.
    ///
    /// # Errors
    /// None in practice (the cache's lock).
    pub fn sn(&self, m: f64, dur: f64, vel: f64, rim: bool) -> Result<Arc<[f64]>, MusicError> {
        let key = (bits(m, 2), bits(dur, 3), bits(vel, 2), rim);
        let mut notes = self
            .notes
            .lock()
            .map_err(|_| MusicError::new(H, "an earlier note panicked"))?;
        if let Some(x) = notes.get(&key) {
            return Ok(Arc::clone(x));
        }
        let x: Arc<[f64]> = if dur > 0.0 {
            roll(m, vel, dur).into()
        } else {
            hit(m, vel, if rim { Stroke::Rim } else { Stroke::Tap }).into()
        };
        notes.insert(key, Arc::clone(&x));
        Ok(x)
    }

    /// `x` through its tone at `t` on its bus, panned `pan`.
    fn put(&self, ctx: &MusicCtx, mix: &mut Mix, x: &[f64], t: f64, gain: f64, pan: f64) -> Played {
        let (sig, g, _) = self.colour(ctx, x, t, 1.0, &self.colour_args())?;
        mix.place(&self.bus(ctx), &Sig::Mono(sig), t, gain * g, pan);
        Ok(())
    }

    /// A run of taps every `step` seconds from `t0` for `n` taps, swelling from `v0` to `v1`.
    #[allow(clippy::too_many_arguments)]
    fn taps(&self, ctx: &MusicCtx, mix: &mut Mix, bus: &str, t0: f64, step: f64, n: usize, v0: f64, v1: f64) -> Played {
        for i in 0..n {
            let s = if n > 1 { i as f64 / (n - 1) as f64 } else { 1.0 };
            let v = v0 + (v1 - v0) * s;
            let x = self.sn(HEAD, 0.0, round_nd(v, 2), false)?;
            let _ = ctx;
            mix.place(bus, &Sig::Mono(x.to_vec()), t0 + i as f64 * step, 0.30, 0.0);
        }
        Ok(())
    }
}

impl Sound for SnareDrumSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.2
    }

    fn send(&self) -> f64 {
        0.2
    }

    /// The head's thump to the wires' sizzle.
    fn band(&self) -> (f64, f64) {
        (150.0, 9000.0)
    }

    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 80.0,
            cut_span: 300.0,
            drive: 1.3,
            ..ColourArgs::default()
        }
    }

    fn voice(&self, _ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.sn(m, if dur > 0.3 { dur } else { 0.0 }, vel, false)
    }

    /// A flam and a rimshot.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let grace = self.sn(HEAD, 0.0, 0.4, false)?;
        mix.place(&bus, &Sig::Mono(grace.to_vec()), (t - 0.03).max(0.0), 0.25, 0.0);
        let rim = self.sn(HEAD, 0.0, 1.0, true)?;
        mix.place(&bus, &Sig::Mono(rim.to_vec()), t, 0.55, 0.0);
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _bed: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let ev = &ctx.rec.events;
        let t_ko = ctx.show.t_ko + 0.01;
        let flams = state::windows(ev, k, "flam");
        let buzz = state::windows(ev, k, "buzz_roll");
        // the paradiddle: one tap per stroke (a rest under BUZZ ROLL, where the roll plays)
        for s in state::strokes(ev, k) {
            if s.t > t_ko {
                break;
            }
            let tq = ctx.q16(s.t);
            let vel = if s.accent { 0.95 } else { 0.6 };
            let pan = if s.side == 0 { -0.2 } else { 0.2 };
            self.put(ctx, mix, &self.sn(HEAD, 0.0, vel, false)?, tq, 0.34, pan)?;
            if state::within(&flams, s.t) {
                let g = self.sn(HEAD, 0.0, 0.4, false)?;
                self.put(ctx, mix, &g, (tq - 0.028).max(0.0), 0.2, -pan)?;
            }
        }
        // RIMSHOT: armed with a stick click; landed, a dry crack
        for t in state::rim_armed(ev, k) {
            if t > t_ko {
                break;
            }
            let click = self.sn(HEAD + 12.0, 0.0, 0.35, false)?;
            self.put(ctx, mix, &click, ctx.q16(t), 0.12, 0.0)?;
        }
        for (t, _) in state::rimshots(ev, k) {
            if t > t_ko {
                break;
            }
            self.put(ctx, mix, &self.sn(HEAD, 0.0, 1.0, true)?, t, 0.6, 0.0)?;
        }
        // BUZZ ROLL: a press roll as long as the disc
        for (t0, t1) in buzz {
            if t0 > t_ko {
                break;
            }
            let tq = ctx.q16(t0);
            let dur = round_nd((t1.min(t_ko) - tq).max(0.1), 2);
            self.put(ctx, mix, &self.sn(HEAD, dur, 0.85, false)?, tq, 0.4, 0.0)?;
        }
        Ok(())
    }

    /// A roll dying away as it falls apart.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        self.taps(ctx, mix, &bus, ctx.show.t_hs, 0.05, 12, 0.8, 0.2)
    }

    /// A flam, a diddle and a rimshot.
    fn stinger(&self, _ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        for (dt, v) in [(-0.09, 0.4), (-0.06, 0.6), (-0.03, 0.4)] {
            let x = self.sn(HEAD, 0.0, v, false)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), (tw + dt).max(0.0), 0.3, 0.0);
        }
        let rim = self.sn(HEAD, 0.0, 1.0, true)?;
        mix.place("solo", &Sig::Mono(rim.to_vec()), tw, 0.5, 0.0);
        Ok(())
    }

    /// Its solo line as strokes: short notes are taps (shaded by pitch), long ones press rolls, the top note a
    /// rimshot.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        let notes = ctx.solo(self.name(ctx))?;
        let top = notes.iter().map(|n| n.1).max().unwrap_or(0);
        for (bt, m, d) in notes {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let x = if d * grid.beat > 0.3 {
                self.sn(f64::from(m), round_nd(d * grid.beat, 2), 0.85, false)?
            } else {
                self.sn(f64::from(m), 0.0, 0.85, m == top)?
            };
            mix.place("solo", &Sig::Mono(x.to_vec()), ts, 0.5, 0.0);
        }
        Ok(())
    }

    /// A crescendo roll as it reassembles.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let bus = self.bus(ctx);
        let span = (grid.t_glide - grid.t_reassemble).max(0.1);
        let n = ((span / (grid.beat / 4.0)).floor() as usize).clamp(2, 64);
        self.taps(ctx, mix, &bus, grid.t_reassemble, span / n as f64, n, 0.3, 0.9)
    }

    /// Sixteenth diddles into the frame-0 slam, an accent on every beat.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let t30 = (grid.n_bars as f64 - 2.0) * grid.bar;
        for j in 4..32 {
            let tb = t30 + f64::from(j) * grid.beat / 4.0;
            let v = if j % 4 == 0 { 0.9 } else { 0.5 };
            let x = self.sn(HEAD, 0.0, v, false)?;
            mix.place(&self.bus(ctx), &Sig::Mono(x.to_vec()), tb, 0.28, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_is_kept_by_its_rounded_key() {
        let part = SnareDrumSound::new(1);
        let x = part.sn(72.0, 0.0, 0.85, false).unwrap();
        assert!(Arc::ptr_eq(&x, &part.sn(72.001, 0.0, 0.851, false).unwrap()));
        assert!(!Arc::ptr_eq(&x, &part.sn(72.0, 0.0, 0.85, true).unwrap()));
        assert_eq!((part.k(), part.enemy()), (1, 0));
    }
}
