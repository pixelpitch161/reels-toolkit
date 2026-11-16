//! MARIMBA's part in the score (a Studio fighter: written for it, no kit part to match). What you see is what you
//! hear: every mallet it throws plays its line's note on that 16th, every wall bounce the next note up the key's
//! scale from it (the mallet run), and a mallet that lands a chord tone under the line with a wooden knock. TREMOLO
//! rolls the line's note in 32nds; DEAD STROKE's pin is a dead, muted stroke and a thud; FIVE OCTAVES is one note per
//! mallet, rising through the whole instrument. Notes come from its own voice ([`bar`]) through a cache of its own.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::round_nd;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{ColourArgs, MusicCtx, MusicError};
use bf_sound::synth;
use bf_sound::traits::{Bed, Played, Sound};

use super::state;
use super::voice::bar;

/// The name the part's errors carry.
const H: &str = "MARIMBA";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// A note of the cache: `(round(m, 2), round(dur, 3), round(vel, 2))` by their bits.
type Key = (u64, u64, u64);

fn bits(x: f64, nd: i32) -> u64 {
    (round_nd(x, nd) + 0.0).to_bits()
}

/// MARIMBA's part.
#[derive(Debug)]
pub struct MarimbaSound {
    k: usize,
    notes: Mutex<BTreeMap<Key, Arc<[f64]>>>,
}

impl MarimbaSound {
    /// The part of the fighter on side `k`.
    pub fn new(k: usize) -> MarimbaSound {
        MarimbaSound {
            k,
            notes: Mutex::new(BTreeMap::new()),
        }
    }

    /// One struck bar, kept by its rounded key.
    ///
    /// # Errors
    /// None in practice (the cache's lock).
    pub fn mar(&self, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        let key = (bits(m, 2), bits(dur, 3), bits(vel, 2));
        let mut notes = self
            .notes
            .lock()
            .map_err(|_| MusicError::new(H, "an earlier note panicked"))?;
        if let Some(x) = notes.get(&key) {
            return Ok(Arc::clone(x));
        }
        let x: Arc<[f64]> = bar(m, vel, dur).into();
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

    /// The `b`th note of the key's scale above `m` (the run up the walls), kept inside its register.
    fn step_up(&self, ctx: &MusicCtx, m: i32, b: i64, t: f64) -> Result<i32, MusicError> {
        let hi = ctx.cfg[self.k].register.1 + 12;
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

impl Sound for MarimbaSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.4
    }

    fn send(&self) -> f64 {
        0.3
    }

    /// The bars from C4 up and their bright overtone.
    fn band(&self) -> (f64, f64) {
        (250.0, 4000.0)
    }

    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 60.0,
            cut_span: 400.0,
            drive: 1.5,
            ..ColourArgs::default()
        }
    }

    fn voice(&self, _ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.mar(m, dur, vel)
    }

    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let top = self.top(ctx);
        let hits = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(h) => h.clone(),
            None => vec![top - 12, top - 5, top - 24, top],
        };
        for (m, g) in hits.iter().zip([0.50, 0.36, 0.30, 0.26, 0.22]) {
            let x = self.mar(f64::from(*m), 1.2, 1.0)?;
            mix.place(&bus, &Sig::Mono(x.to_vec()), t, g, 0.15);
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _bed: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let ev = &ctx.rec.events;
        let t_ko = ctx.show.t_ko + 0.01;
        let beat = ctx.show.grid.beat;
        let lo = ctx.cfg[k].register.0;
        let mut base: BTreeMap<i64, i32> = BTreeMap::new();
        let mut last = self.top(ctx) - 12;
        // the throws: the line's note (FIVE OCTAVES: one note per mallet, rising through the instrument)
        let throws = state::throws(ev, k);
        let n_oct = throws.iter().filter(|w| w.octave).count().max(1);
        let span = ctx
            .harmony
            .scale_notes(lo.min(60), self.top(ctx) + 12, None)
            .map_err(bad)?;
        let mut j_oct = 0usize;
        for w in &throws {
            if w.t > t_ko {
                break;
            }
            if w.octave {
                let i = (j_oct * span.len().saturating_sub(1)) / n_oct;
                j_oct += 1;
                let m = span.get(i).copied().unwrap_or(last);
                base.insert(w.id, m);
                self.put(ctx, mix, &self.mar(f64::from(m), 0.25, 0.7)?, w.t, 0.16)?;
                continue;
            }
            let tq = ctx.q16(w.t);
            let m = self.line(ctx, tq, last)?;
            last = m;
            base.insert(w.id, m);
            self.put(ctx, mix, &self.mar(f64::from(m), 0.6 * beat, 0.85)?, tq, 0.36)?;
        }
        // the bounces: up the scale, a wall at a time
        for b in state::bounces(ev, k) {
            if b.t > t_ko {
                break;
            }
            let Some(&m) = base.get(&b.id) else { continue };
            let tq = ctx.q16(b.t);
            let n = self.step_up(ctx, m, b.b, tq)?;
            self.put(ctx, mix, &self.mar(f64::from(n), 0.3 * beat, 0.6 + 0.08 * b.b.min(4) as f64)?, tq, 0.22)?;
        }
        // the landings: a chord tone under the line and a wooden knock
        for e in state::ends(ev, k) {
            if e.t > t_ko {
                break;
            }
            if !e.hit {
                continue;
            }
            let tq = ctx.q16(e.t);
            let m = base.get(&e.id).copied().unwrap_or(last);
            let tones = ctx
                .harmony
                .chord_tones(m - 9, m - 3, ctx.harmony.chord_at(tq))
                .map_err(bad)?;
            if let Some(&u) = tones.last() {
                self.put(ctx, mix, &self.mar(f64::from(u), 0.5 * beat, 1.0)?, tq, 0.30)?;
            }
            let knock = synth::thud(0.12, 420.0, 180.0, (e.id.rem_euclid(97) + 3) as u64);
            mix.place("fx", &Sig::Mono(knock), tq, 0.08, 0.0);
        }
        // TREMOLO: the line's note rolled in 32nds
        for (t0, t1) in state::windows(ev, k, "tremolo") {
            if t0 > t_ko {
                break;
            }
            let tq = ctx.q16(t0);
            let m = self.line(ctx, tq, last)?;
            let n = ((t1.min(t_ko) - tq) / (beat / 8.0)).floor().max(0.0) as usize;
            let x = self.mar(f64::from(m), 0.05, 0.55)?;
            let x5 = self.mar(f64::from(m + 7), 0.05, 0.5)?;
            for i in 0..n {
                let ti = tq + i as f64 * beat / 8.0;
                self.put(ctx, mix, if i % 2 == 0 { &x } else { &x5 }, ti, 0.12)?;
            }
        }
        // DEAD STROKE: armed with a dry click; a pin is a dead stroke and a thud
        for (t0, _) in state::windows(ev, k, "dead_stroke") {
            if t0 > t_ko {
                break;
            }
            let click = synth::thud(0.06, 900.0, 600.0, 11);
            mix.place("fx", &Sig::Mono(click), ctx.q16(t0), 0.06, 0.0);
        }
        for (t, _) in state::pins(ev, k) {
            if t > t_ko {
                break;
            }
            let m = f64::from(self.top(ctx) - 24);
            let dead: Vec<f64> = self.mar(m, 0.0, 1.0)?.iter().take((0.09 * 48_000.0) as usize).copied().collect();
            self.put(ctx, mix, &dead, t, 0.45)?;
            mix.place("fx", &Sig::Mono(synth::thud(0.3, 140.0, 55.0, 23)), t, 0.22, 0.0);
        }
        Ok(())
    }

    /// A falling glissando down the bars.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let top = self.top(ctx);
        let notes = ctx.harmony.scale_notes(top - 24, top, None).map_err(bad)?;
        for (j, m) in notes.iter().rev().enumerate() {
            let x = self.mar(f64::from(*m), 0.1, 0.8 - 0.03 * j as f64)?;
            mix.place(&self.bus(ctx), &Sig::Mono(x.to_vec()), ctx.show.t_hs + j as f64 * 0.04, 0.26, 0.0);
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let top = f64::from(self.top(ctx));
        for (j, d) in [-12.0, -8.0, -5.0, 0.0].into_iter().enumerate() {
            let x = self.mar(top + d, 0.8, 1.0)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), tw + j as f64 * 0.03, 0.34, 0.0);
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
            let x = self.mar(f64::from(m), d * grid.beat, 0.95)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), ts, 0.50, 0.0);
            if d * grid.beat > 0.3 {
                // a long note is rolled, as a marimba holds one
                let n = (d * 4.0) as usize;
                for i in 1..n {
                    let xi = self.mar(f64::from(m), 0.05, 0.6)?;
                    mix.place("solo", &Sig::Mono(xi.to_vec()), ts + i as f64 * grid.beat / 4.0, 0.22, 0.0);
                }
            }
        }
        Ok(())
    }

    /// A run up the bars as it reassembles.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let top = self.top(ctx);
        let run = ctx.harmony.scale_notes(top - 12, top, None).map_err(bad)?;
        for (j, m) in run.iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            let x = self.mar(f64::from(*m), 0.14, 0.7)?;
            mix.place(&self.bus(ctx), &Sig::Mono(x.to_vec()), tj, 0.22, 0.0);
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
            let x = self.mar(f64::from(m), 0.4 * grid.beat, 0.8)?;
            mix.place(&self.bus(ctx), &Sig::Mono(x.to_vec()), tb, 0.30, 0.15);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_is_kept_by_its_rounded_key() {
        let part = MarimbaSound::new(1);
        let x = part.mar(72.0, 0.3, 0.85).unwrap();
        assert!(Arc::ptr_eq(&x, &part.mar(72.001, 0.3004, 0.851).unwrap()));
        assert!(!Arc::ptr_eq(&x, &part.mar(74.0, 0.3, 0.85).unwrap()));
        assert_eq!((part.k(), part.enemy()), (1, 0));
        assert_eq!(part.band(), (250.0, 4000.0));
    }
}
