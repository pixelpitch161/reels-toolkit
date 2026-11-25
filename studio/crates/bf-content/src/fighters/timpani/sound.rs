//! TIMPANI's part in the score (a Studio fighter: written for it, no kit part to match). What you see is what you
//! hear: every time a kettle booms it plays that kettle's note on that 16th: the low kettle the chord's root, the
//! high kettle its fifth, FULL SET's kettles the octave and the low fifth. The interval hit sounds root and fifth at
//! once with a crack; PEDAL SLIDE is a pedal glissando up a fourth; CROSS-STICKING's fast hops play themselves as
//! quick alternating strokes. Notes come from its own voice ([`stroke`]) through a cache of its own.
//!
//! Under the Composed arrangement (sound upgrade, 2026-11-25) every boom plays the written line's note of its 16th
//! instead of the kettle's own, and the interval hit stands on the line's note. PEDAL SLIDE, FULL SET's fanfare and
//! the endings keep their notes. Classic is the part above.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::round_nd;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{ColourArgs, MusicCtx, MusicError};
use bf_sound::synth;
use bf_sound::traits::{Bed, Played, Sound};

use super::state;
use super::voice::stroke;

/// The name the part's errors carry.
const H: &str = "TIMPANI";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// A note of the cache: `(round(m, 2), round(dur, 3), round(vel, 2), round(gliss, 2))` by their bits.
type Key = (u64, u64, u64, u64);

fn bits(x: f64, nd: i32) -> u64 {
    (round_nd(x, nd) + 0.0).to_bits()
}

/// TIMPANI's part.
#[derive(Debug)]
pub struct TimpaniSound {
    k: usize,
    notes: Mutex<BTreeMap<Key, Arc<[f64]>>>,
}

impl TimpaniSound {
    /// The part of the fighter on side `k`.
    pub fn new(k: usize) -> TimpaniSound {
        TimpaniSound {
            k,
            notes: Mutex::new(BTreeMap::new()),
        }
    }

    /// One stroke, kept by its rounded key.
    ///
    /// # Errors
    /// None in practice (the cache's lock).
    pub fn tim(&self, m: f64, dur: f64, vel: f64, gliss: f64) -> Result<Arc<[f64]>, MusicError> {
        let key = (bits(m, 2), bits(dur, 3), bits(vel, 2), bits(gliss, 2));
        let mut notes = self
            .notes
            .lock()
            .map_err(|_| MusicError::new(H, "an earlier note panicked"))?;
        if let Some(x) = notes.get(&key) {
            return Ok(Arc::clone(x));
        }
        let x: Arc<[f64]> = stroke(m, vel, dur, gliss).into();
        notes.insert(key, Arc::clone(&x));
        Ok(x)
    }

    /// The low kettle's note at `t`: the chord's root in the lower part of its register.
    fn low(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let lo = ctx.cfg[self.k].register.0;
        ctx.harmony.root_above(ctx.harmony.chord_at(t), lo).map_err(bad)
    }

    /// Kettle `i`'s note at `t`: low = root, high = fifth, then the octave and the fifth below.
    fn kettle_note(&self, ctx: &MusicCtx, i: i64, t: f64) -> Result<i32, MusicError> {
        let r = self.low(ctx, t)?;
        Ok(match i {
            0 => r,
            1 => r + 7,
            2 => r + 12,
            _ => r - 5,
        })
    }

    /// The written line's note for a stroke at `tq` (sound upgrade: under the Composed arrangement the kettles play
    /// the timpani's line). `None` under Classic and on a rest of the line: the kettle's own note stands then.
    fn written(&self, ctx: &MusicCtx, tq: f64) -> Result<Option<i32>, MusicError> {
        if ctx.composed_on() {
            ctx.play_line(self.name(ctx), tq)
        } else {
            Ok(None)
        }
    }

    /// The octave of `m` that a second kettle sounds with it: the one below while it is inside the register, else
    /// the one above.
    fn octave_of(&self, ctx: &MusicCtx, m: i32) -> i32 {
        if m - 12 >= ctx.cfg[self.k].register.0 {
            m - 12
        } else {
            m + 12
        }
    }

    /// The second note of an interval hit on `m` at `t`: the tone of the bar's chord nearest the fifth above it (a
    /// third to a sixth above), so the two kettles stay in the harmony whatever note the line has; the plain fifth
    /// where the chord has no tone there.
    fn fifth_over(&self, ctx: &MusicCtx, m: i32, t: f64) -> Result<i32, MusicError> {
        let tones = ctx
            .harmony
            .chord_tones(m + 3, m + 9, ctx.harmony.chord_at(t))
            .map_err(bad)?;
        Ok(tones.into_iter().min_by_key(|c| (c - (m + 7)).abs()).unwrap_or(m + 7))
    }

    /// `x` through its tone at `t` on its bus.
    fn put(&self, ctx: &MusicCtx, mix: &mut Mix, x: &[f64], t: f64, gain: f64) -> Played {
        let (sig, g, _) = self.colour(ctx, x, t, 1.0, &self.colour_args())?;
        mix.place(&self.bus(ctx), &Sig::Mono(sig), t, gain * g, 0.1);
        Ok(())
    }

    /// The tonic in the middle of its register.
    fn home(&self, ctx: &MusicCtx) -> i32 {
        let lo = ctx.cfg[self.k].register.0;
        lo + (ctx.harmony.key_pc - lo).rem_euclid(12)
    }
}

impl Sound for TimpaniSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.5
    }

    fn send(&self) -> f64 {
        0.35
    }

    /// The kettles' low notes and their felt thump.
    fn band(&self) -> (f64, f64) {
        (55.0, 1200.0)
    }

    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 40.0,
            cut_span: 250.0,
            drive: 1.3,
            ..ColourArgs::default()
        }
    }

    fn voice(&self, _ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.tim(m, dur, vel, 0.0)
    }

    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let home = self.home(ctx);
        let hits = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(h) => h.clone(),
            None => vec![home, home + 7, home - 5, home + 12],
        };
        for (j, (m, g)) in hits.iter().zip([0.55, 0.42, 0.36, 0.30, 0.26]).enumerate() {
            let x = self.tim(f64::from(*m), 1.4, 1.0, 0.0)?;
            mix.place(&bus, &Sig::Mono(x.to_vec()), t + j as f64 * 0.012, g, 0.1);
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _bed: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let ev = &ctx.rec.events;
        let t_ko = ctx.show.t_ko + 0.01;
        let beat = ctx.show.grid.beat;
        // every boom: that kettle's note, louder for the low kettle
        let mut last_tq = None;
        for b in state::booms(ev, k) {
            if b.t > t_ko {
                break;
            }
            let tq = ctx.q16(b.t);
            // Composed: the line's note on that 16th, whichever kettle booms; a second kettle booming on the same
            // 16th sounds its octave instead of doubling it
            let m = match self.written(ctx, tq)? {
                Some(m) if last_tq == Some(tq) => self.octave_of(ctx, m),
                Some(m) => m,
                None => self.kettle_note(ctx, b.kettle, tq)?,
            };
            last_tq = Some(tq);
            let (vel, gain) = match b.kettle {
                0 => (0.95, 0.46),
                1 => (0.85, 0.36),
                _ => (0.8, 0.30),
            };
            self.put(ctx, mix, &self.tim(f64::from(m), 1.2 * beat, vel, 0.0)?, tq, gain)?;
        }
        // the interval hit: root and fifth together with a crack
        for (t, _, _) in state::hits(ev, k, "interval") {
            if t > t_ko {
                break;
            }
            let tq = ctx.q16(t);
            // Composed: the interval stands on the line's note, its upper kettle on the chord
            let (r, fifth) = match self.written(ctx, tq)? {
                Some(m) => (m, self.fifth_over(ctx, m, tq)?),
                None => {
                    let r = self.low(ctx, tq)?;
                    (r, r + 7)
                }
            };
            for (m, g) in [(r, 0.34), (fifth, 0.30), (r + 12, 0.18)] {
                self.put(ctx, mix, &self.tim(f64::from(m), beat, 1.0, 0.0)?, tq, g)?;
            }
            let crack = synth::thud(0.08, 1800.0, 900.0, 29);
            mix.place("fx", &Sig::Mono(crack), tq, 0.07, 0.0);
        }
        // PEDAL SLIDE: a glissando up a fourth into the root
        for (t0, t1) in state::slides(ev, k) {
            if t0 > t_ko {
                break;
            }
            let tq = ctx.q16(t0);
            let r = self.low(ctx, tq)?;
            let dur = (t1.min(t_ko) - tq).max(0.2);
            let x = self.tim(f64::from(r - 5), dur, 0.9, 5.0)?;
            self.put(ctx, mix, &x, tq, 0.40)?;
        }
        // a squeeze: a dull thud of copper
        for (t, _, _) in state::hits(ev, k, "squeeze") {
            if t > t_ko {
                break;
            }
            mix.place(
                "fx",
                &Sig::Mono(synth::thud(0.2, 160.0, 60.0, 31)),
                ctx.q16(t),
                0.16,
                0.0,
            );
        }
        // FULL SET: a four-drum fanfare as the new kettles land
        if let Some(t) = state::full_set(ev, k).filter(|&t| t <= t_ko) {
            let tq = ctx.q16(t);
            let r = self.low(ctx, tq)?;
            for (j, d) in [0, 7, 12, 7, 0].into_iter().enumerate() {
                let x = self.tim(f64::from(r + d), 0.5 * beat, 1.0, 0.0)?;
                self.put(ctx, mix, &x, tq + j as f64 * beat / 4.0, 0.34)?;
            }
        }
        Ok(())
    }

    /// The kettles fall over: a long glissando down on the low drum.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let home = self.home(ctx);
        let x = self.tim(f64::from(home + 7), 1.2, 1.0, -9.0)?;
        mix.place(&self.bus(ctx), &Sig::Mono(x.to_vec()), ctx.show.t_hs, 0.40, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let home = f64::from(self.home(ctx));
        for (j, d) in [0.0, 7.0, 0.0, 12.0].into_iter().enumerate() {
            let x = self.tim(home + d, 0.9, 1.0, 0.0)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), tw + j as f64 * 0.09, 0.40, 0.0);
        }
        Ok(())
    }

    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let x = self.tim(f64::from(m), d * grid.beat, 0.95, 0.0)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), ts, 0.52, 0.0);
            if d * grid.beat > 0.4 {
                // a long note is rolled, as a timpanist holds one
                let n = (d * 4.0) as usize;
                for i in 1..n {
                    let xi = self.tim(f64::from(m), 0.1, 0.5, 0.0)?;
                    mix.place(
                        "solo",
                        &Sig::Mono(xi.to_vec()),
                        ts + i as f64 * grid.beat / 4.0,
                        0.20,
                        0.0,
                    );
                }
            }
        }
        Ok(())
    }

    /// A rising roll as it reassembles.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let home = self.home(ctx);
        let n = 12;
        for j in 0..n {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * f64::from(j) / f64::from(n));
            let m = if j % 2 == 0 { home } else { home + 7 };
            let x = self.tim(f64::from(m), 0.2, 0.45 + 0.04 * f64::from(j), 0.0)?;
            mix.place(&self.bus(ctx), &Sig::Mono(x.to_vec()), tj, 0.24, 0.0);
        }
        Ok(())
    }

    /// Root and fifth in quarters into the frame-0 slam.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let t30 = (grid.n_bars as f64 - 2.0) * grid.bar;
        for j in 1..8 {
            let tb = t30 + f64::from(j) * grid.beat;
            let r = self.low(ctx, tb)?;
            let m = if j % 2 == 0 { r + 7 } else { r };
            let x = self.tim(f64::from(m), 0.8 * grid.beat, 0.85, 0.0)?;
            mix.place(&self.bus(ctx), &Sig::Mono(x.to_vec()), tb, 0.34, 0.1);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_is_kept_by_its_rounded_key() {
        let part = TimpaniSound::new(1);
        let x = part.tim(45.0, 0.3, 0.85, 0.0).unwrap();
        assert!(Arc::ptr_eq(&x, &part.tim(45.001, 0.3004, 0.851, 0.0).unwrap()));
        assert!(!Arc::ptr_eq(&x, &part.tim(47.0, 0.3, 0.85, 0.0).unwrap()));
        assert!(!Arc::ptr_eq(&x, &part.tim(45.0, 0.3, 0.85, 5.0).unwrap()));
        assert_eq!((part.k(), part.enemy()), (1, 0));
    }
}
