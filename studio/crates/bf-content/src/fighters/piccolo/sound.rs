//! PICCOLO's part in the score (a Studio fighter: written for it, no kit part to match). What you see is what you
//! hear: every needle it fires plays its line's note on that 16th with a grace note a step above, every rope bounce
//! a quick high tick climbing the key's scale from it, and a needle through the enemy a bright chord tone above the
//! line. TRILL trills the line's note with its upper neighbour; OVER THE BAND holds a shrill top note; GLASS SHATTER is
//! a shower of glassy high notes falling one per shard. Notes come from its own voice ([`tone`]) through a cache of
//! its own.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::round_nd;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{ColourArgs, MusicCtx, MusicError};
use bf_sound::traits::{Bed, Played, Sound};

use super::state;
use super::voice::tone;

/// The name the part's errors carry.
const H: &str = "PICCOLO";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// A note of the cache: `(round(m, 2), round(dur, 3), round(vel, 2))` by their bits.
type Key = (u64, u64, u64);

fn bits(x: f64, nd: i32) -> u64 {
    (round_nd(x, nd) + 0.0).to_bits()
}

/// PICCOLO's part.
#[derive(Debug)]
pub struct PiccoloSound {
    k: usize,
    notes: Mutex<BTreeMap<Key, Arc<[f64]>>>,
}

impl PiccoloSound {
    /// The part of the fighter on side `k`.
    pub fn new(k: usize) -> PiccoloSound {
        PiccoloSound {
            k,
            notes: Mutex::new(BTreeMap::new()),
        }
    }

    /// One piccolo note, kept by its rounded key.
    ///
    /// # Errors
    /// None in practice (the cache's lock).
    pub fn pic(&self, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        let key = (bits(m, 2), bits(dur, 3), bits(vel, 2));
        let mut notes = self
            .notes
            .lock()
            .map_err(|_| MusicError::new(H, "an earlier note panicked"))?;
        if let Some(x) = notes.get(&key) {
            return Ok(Arc::clone(x));
        }
        let x: Arc<[f64]> = tone(m, vel, dur).into();
        notes.insert(key, Arc::clone(&x));
        Ok(x)
    }

    /// The key's tonic at the top of its register.
    fn top(&self, ctx: &MusicCtx) -> i32 {
        let hi = ctx.cfg[self.k].register.1;
        hi - (hi - ctx.harmony.key_pc).rem_euclid(12)
    }

    /// Its line's note at `t`, or the last note it played (a rest keeps the run going on that note).
    fn line(&self, ctx: &MusicCtx, t: f64, last: i32) -> Result<i32, MusicError> {
        Ok(ctx.line_at(self.name(ctx), t)?.unwrap_or(last))
    }

    /// `x` through its tone at `t` on its bus.
    fn put(&self, ctx: &MusicCtx, mix: &mut Mix, x: &[f64], t: f64, gain: f64) -> Played {
        let (sig, g, _) = self.colour(ctx, x, t, 1.0, &self.colour_args())?;
        mix.place(&self.bus(ctx), &Sig::Mono(sig), t, gain * g, 0.15);
        Ok(())
    }

    /// An ordinary note (a needle's note, a pierce): `x` through its tone at `t` on its bus. Under Composed the
    /// piccolo sounds one note at a time, so this note ends the one before it (sound upgrade, M5); under Classic it
    /// is [`PiccoloSound::put`].
    fn sing(&self, ctx: &MusicCtx, mix: &mut Mix, x: &[f64], t: f64, gain: f64) -> Played {
        let (sig, g, _) = self.colour(ctx, x, t, 1.0, &self.colour_args())?;
        let one = ctx.one_voice(self.name(ctx));
        mix.note(&self.bus(ctx), &Sig::Mono(sig), t, gain * g, 0.15, one);
        Ok(())
    }

    /// The `b`th note of the key's scale above `m` (the ticks up the ropes), kept inside its register.
    fn step_up(&self, ctx: &MusicCtx, m: i32, b: i64, t: f64) -> Result<i32, MusicError> {
        let hi = ctx.cfg[self.k].register.1;
        let up = ctx
            .harmony
            .scale_notes(m + 1, m + 36, Some(ctx.harmony.chord_at(t)))
            .map_err(bad)?;
        let mut n = up.get((b.max(1) - 1) as usize).copied().unwrap_or(m + 12);
        while n > hi {
            n -= 12;
        }
        Ok(n)
    }
}

impl Sound for PiccoloSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.1
    }

    fn send(&self) -> f64 {
        0.35
    }

    /// The top of the band and its breath.
    fn band(&self) -> (f64, f64) {
        (600.0, 8000.0)
    }

    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 60.0,
            cut_span: 400.0,
            drive: 1.2,
            ..ColourArgs::default()
        }
    }

    fn voice(&self, _ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.pic(m, dur, vel)
    }

    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let top = self.top(ctx);
        let hits = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(h) => h.clone(),
            None => vec![top - 12, top - 5, top - 24, top],
        };
        for (m, g) in hits.iter().zip([0.40, 0.30, 0.26, 0.22, 0.18]) {
            let x = self.pic(f64::from(*m), 1.0, 1.0)?;
            mix.place(&bus, &Sig::Mono(x.to_vec()), t, g, 0.15);
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _bed: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let ev = &ctx.rec.events;
        let t_ko = ctx.show.t_ko + 0.01;
        let beat = ctx.show.grid.beat;
        let name = self.name(ctx);
        // sound upgrade: under Composed a needle and a pierce sound the note `play_line` hands them, every ordinary
        // note is softer the higher it is above C6, and the bounce ticks play an octave lower (M3); GLASS SHATTER's
        // shards keep their own notes
        let composed = ctx.composed_on();
        let mut shards: BTreeSet<i64> = BTreeSet::new();
        let mut base: BTreeMap<i64, i32> = BTreeMap::new();
        let mut last = self.top(ctx) - 12;
        // the shots: a grace note a step above, then the line's note (GLASS SHATTER: one falling note per shard)
        let shots = state::shots(ev, k);
        let n_sh = shots.iter().filter(|w| w.shard).count().max(1);
        let top = self.top(ctx);
        let fall = ctx.harmony.scale_notes(top - 24, top, None).map_err(bad)?;
        let mut j_sh = 0usize;
        for w in &shots {
            if w.t > t_ko {
                break;
            }
            if w.shard {
                let i = fall.len().saturating_sub(1) - (j_sh * fall.len().saturating_sub(1)) / n_sh;
                j_sh += 1;
                let m = fall.get(i).copied().unwrap_or(last);
                base.insert(w.id, m);
                shards.insert(w.id);
                self.put(ctx, mix, &self.pic(f64::from(m), 0.08, 0.6)?, w.t, 0.12)?;
                continue;
            }
            let tq = ctx.q16(w.t);
            // the note it sounds (Classic: the line's, as `line` reads it)
            let m = ctx.play_line(name, tq)?.unwrap_or(last);
            last = m;
            base.insert(w.id, m);
            // the grace stays ahead of the note and does not end the note before it
            let grace = self.step_up(ctx, m, 1, tq)?;
            let x = self.pic(f64::from(grace), 0.03, 0.6)?;
            self.put(ctx, mix, &x, tq - 0.035, 0.16 * ctx.soften(name, grace))?;
            let x = self.pic(f64::from(m), 0.45 * beat, 0.85)?;
            self.sing(ctx, mix, &x, tq, 0.30 * ctx.soften(name, m))?;
        }
        // the bounces: quick high ticks up the scale, a rope at a time
        for b in state::bounces(ev, k) {
            if b.t > t_ko {
                break;
            }
            let Some(&m) = base.get(&b.id) else { continue };
            let mut n = self.step_up(ctx, m, b.b + 1, b.t)?;
            let mut soft = 1.0;
            if composed && !shards.contains(&b.id) {
                // the same climb an octave lower, out of the top of its register; a tick is too short to end a note
                n -= 12;
                soft = ctx.soften(name, n);
            }
            self.put(ctx, mix, &self.pic(f64::from(n), 0.02, 0.5)?, b.t, 0.07 * soft)?;
        }
        // a pierce: a bright chord tone above the line
        for h in state::pierces(ev, k) {
            if h.t > t_ko {
                break;
            }
            let tq = ctx.q16(h.t);
            let m = base.get(&h.id).copied().unwrap_or(last);
            let tones = ctx
                .harmony
                .chord_tones(m + 1, m + 9, ctx.harmony.chord_at(tq))
                .map_err(bad)?;
            if composed
                && !shards.contains(&h.id)
                && let Some(own) = ctx.play_line(name, tq)?
            {
                // the line's note at the pierce, not a chord tone above the needle's
                let x = self.pic(f64::from(own), 0.3 * beat, 1.0)?;
                self.sing(ctx, mix, &x, tq, 0.22 * ctx.soften(name, own))?;
            } else if let Some(&u) = tones.first() {
                let hi = ctx.cfg[k].register.1;
                let u = if u > hi { u - 12 } else { u };
                self.put(ctx, mix, &self.pic(f64::from(u), 0.3 * beat, 1.0)?, tq, 0.22)?;
            }
        }
        // TRILL: the line's note trilled with its upper neighbour in 32nds for a beat
        for t0 in state::trills(ev, k) {
            if t0 > t_ko {
                break;
            }
            let tq = ctx.q16(t0);
            let m = self.line(ctx, tq, last)?;
            let up = self.step_up(ctx, m, 1, tq)?;
            let (x, xu) = (self.pic(f64::from(m), 0.04, 0.6)?, self.pic(f64::from(up), 0.04, 0.6)?);
            for i in 0..8 {
                let ti = tq + i as f64 * beat / 8.0;
                self.put(ctx, mix, if i % 2 == 0 { &x } else { &xu }, ti, 0.12)?;
            }
        }
        // OVER THE BAND: a shrill top note held over everything for a bar
        for (t0, _) in state::over_band(ev, k) {
            if t0 > t_ko {
                break;
            }
            let tq = ctx.q16(t0);
            self.put(ctx, mix, &self.pic(f64::from(top), 4.0 * beat, 0.9)?, tq, 0.18)?;
        }
        Ok(())
    }

    /// A falling run, glassy and quick.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let top = self.top(ctx);
        let notes = ctx.harmony.scale_notes(top - 12, top, None).map_err(bad)?;
        for (j, m) in notes.iter().rev().enumerate() {
            let x = self.pic(f64::from(*m), 0.06, 0.8 - 0.04 * j as f64)?;
            mix.place(
                &self.bus(ctx),
                &Sig::Mono(x.to_vec()),
                ctx.show.t_hs + j as f64 * 0.05,
                0.22,
                0.0,
            );
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let top = f64::from(self.top(ctx));
        for (j, d) in [-12.0, -8.0, -5.0, 0.0].into_iter().enumerate() {
            let x = self.pic(top + d, if j == 3 { 0.8 } else { 0.08 }, 1.0)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), tw + j as f64 * 0.06, 0.28, 0.0);
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
            let x = self.pic(f64::from(m), d * grid.beat * 0.9, 0.95)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), ts, 0.40, 0.0);
        }
        Ok(())
    }

    /// A run up as it reassembles.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let top = self.top(ctx);
        let run = ctx.harmony.scale_notes(top - 12, top, None).map_err(bad)?;
        for (j, m) in run.iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            let x = self.pic(f64::from(*m), 0.12, 0.7)?;
            mix.place(&self.bus(ctx), &Sig::Mono(x.to_vec()), tj, 0.18, 0.0);
        }
        Ok(())
    }

    /// Its line in eighths into the frame-0 slam.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let t30 = (grid.n_bars as f64 - 2.0) * grid.bar;
        let mut last = self.top(ctx) - 12;
        for j in 2..16 {
            let tb = t30 + f64::from(j) * grid.beat / 2.0;
            let m = self.line(ctx, tb, last)?;
            last = m;
            let x = self.pic(f64::from(m), 0.4 * grid.beat, 0.8)?;
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
        let part = PiccoloSound::new(1);
        let x = part.pic(93.0, 0.3, 0.85).unwrap();
        assert!(Arc::ptr_eq(&x, &part.pic(93.001, 0.3004, 0.851).unwrap()));
        assert!(!Arc::ptr_eq(&x, &part.pic(95.0, 0.3, 0.85).unwrap()));
        assert_eq!((part.k(), part.enemy()), (1, 0));
        assert_eq!(part.band(), (600.0, 8000.0));
    }
}
