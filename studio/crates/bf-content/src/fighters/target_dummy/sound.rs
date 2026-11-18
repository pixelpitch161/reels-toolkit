//! TARGET DUMMY's part in the score: a dry wooden knock (a short MARIMBA note on a chord tone, over a soft thud) on
//! every hit it takes, a little higher as its hit points drop; it falls apart as a tumble of falling knocks.

use bf_core::harmony::HarmonyError;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, Samples, event_is};
use bf_sound::synth::{self, THUD_SEED};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "TARGET DUMMY";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// TARGET DUMMY's part.
#[derive(Clone, Copy, Debug)]
pub struct TargetDummySound {
    k: usize,
}

impl TargetDummySound {
    /// The part of the fighter on side `k`.
    pub fn new(k: usize) -> TargetDummySound {
        TargetDummySound { k }
    }

    /// One knock: a short MARIMBA note.
    fn knock(&self, ctx: &MusicCtx, m: i32, vel: f64) -> Samples {
        let pp = ctx.cfg[self.k].pp.as_str();
        Ok(ctx.note(pp, f64::from(m), 0.18, 0.3, vel)?.to_vec())
    }
}

impl Sound for TargetDummySound {
    fn k(&self) -> usize {
        self.k
    }

    fn band(&self) -> (f64, f64) {
        (300.0, 3000.0)
    }

    fn play(&self, ctx: &MusicCtx, _bed: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let mut hits = 0usize;
        for e in &ctx.rec.events {
            if e.t > ctx.show.t_ko + 0.01 {
                break;
            }
            if e.kind != "hit" || e.who() == Some(k) || !event_is(H, e, 3, k)? {
                continue;
            }
            hits += 1;
            let tq = ctx.q16(e.t);
            let ch = ctx.harmony.chord_at(tq);
            let tones = ctx.harmony.chord_tones(64, 84, ch).map_err(bad)?;
            let Some(&m) = tones.get((hits / 3) % tones.len().max(1)) else {
                continue;
            };
            let pan = if k == 0 { -0.25 } else { 0.25 };
            mix.place(&bus, &Sig::Mono(self.knock(ctx, m, 1.0)?), tq, 0.6, pan);
            // the thud is on its own bus: the fx bus counts as the bed in the band check
            mix.place(
                &bus,
                &Sig::Mono(synth::thud(0.10, 520.0, 220.0, THUD_SEED)),
                tq,
                0.2,
                pan,
            );
        }
        Ok(())
    }

    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let t_hs = ctx.show.t_hs;
        for j in 0..5 {
            let x = self.knock(ctx, 79 - 3 * j, 0.8)?;
            mix.place(
                &bus,
                &Sig::Mono(x),
                t_hs + 0.09 * j as f64,
                0.26,
                (j as f64 - 2.0) / 5.0,
            );
        }
        Ok(())
    }
}
