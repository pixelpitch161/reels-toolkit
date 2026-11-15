//! DRUMS's part in the score: #28's `audio.py`, the drums' part only (the pulses are the kit), close to it by ear
//! (ruling M4-R16).
//!
//! Every main pulse is a drum on the beat: the kick on beats 1 and 3, the snare on 2 and 4 (the snare goes below
//! 25 HP, keeping only beat 4 to 12 HP, then a ghost of it); DRUM ROLL's extra 16th pulses are a soft snare roll; the
//! leader adds off-beat hats. Every landed pulse adds a tom fill (three toms above 45 HP, one to 25, none below).
//! DRUM ROLL ends on a crash and a kick, CRASH CYMBAL is a crash, DOUBLE KICK a whoosh, then the beater's tap an 8th
//! early and the stomp (kick, thud, crash). It falls apart as the kit collapsing; the stinger is crash, kick and
//! tom, the solo #28's drum solo, the reassembly a snare roll swelling.

use bf_core::dmath;
use bf_core::rng::Generator;
use bf_sound::MusicError;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, Samples, event_is_str};
use bf_sound::synth::{self, THUD_SEED};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "DRUMS";

/// `PIECE`: the kit's pieces as the voice's drums.
fn piece(pc: &str) -> f64 {
    match pc {
        "K" => 36.0,
        "S" => 38.0,
        "T1" => 48.0,
        "T2" => 45.0,
        "T3" => 41.0,
        _ => 49.0,
    }
}

/// `T.SOLO['DRUMS']`: #28's drum solo, `(beat, piece, vel)`.
const SOLO: [(f64, &str, f64); 43] = [
    (0.0, "C", 1.0),
    (0.0, "K", 1.0),
    (0.5, "S", 0.8),
    (0.75, "S", 0.6),
    (1.0, "K", 0.9),
    (1.5, "T1", 0.8),
    (1.75, "T1", 0.7),
    (2.0, "T2", 0.9),
    (2.25, "T2", 0.7),
    (2.5, "T3", 0.9),
    (2.75, "T3", 0.8),
    (3.0, "K", 1.0),
    (3.25, "S", 0.7),
    (3.5, "S", 0.8),
    (3.75, "S", 0.9),
    (4.0, "C", 0.9),
    (4.0, "K", 1.0),
    (4.5, "K", 0.8),
    (4.75, "S", 0.8),
    (5.0, "K", 0.9),
    (5.25, "T1", 0.7),
    (5.5, "T2", 0.8),
    (5.75, "T3", 0.9),
    (6.0, "S", 0.9),
    (6.25, "S", 0.5),
    (6.5, "S", 0.7),
    (6.75, "S", 0.9),
    (7.0, "K", 1.0),
    (7.25, "T3", 0.8),
    (7.5, "T2", 0.8),
    (7.75, "T1", 0.8),
    (8.0, "C", 1.0),
    (8.0, "K", 1.0),
    (8.5, "S", 0.9),
    (9.0, "K", 0.9),
    (9.25, "K", 0.8),
    (9.5, "S", 0.9),
    (10.0, "T1", 0.8),
    (10.25, "T2", 0.8),
    (10.5, "T3", 0.9),
    (10.75, "T3", 0.9),
    (11.0, "C", 1.0),
    (11.0, "K", 1.0),
];

/// `sticks(vel)`: the drummer clicking the sticks together for the count.
pub fn sticks(vel: f64) -> Vec<f64> {
    let n = (0.09 * bf_sound::SRF) as usize;
    let t = synth::tt(n);
    let mut rng = Generator::new(9);
    let raw: Vec<f64> = (0..n).map(|_| rng.normal(0.0, 1.0)).collect();
    let click = synth::bp(&raw, 1800.0, 6500.0, 2);
    (0..n)
        .map(|i| {
            let t = t[i];
            (click[i] * dmath::exp(-t / 0.012)
                + 0.6 * dmath::sin(2.0 * dmath::PI * 2350.0 * t) * dmath::exp(-t / 0.02)
                + 0.3 * dmath::sin(2.0 * dmath::PI * 3900.0 * t) * dmath::exp(-t / 0.01))
                * vel
        })
        .collect()
}

/// DRUMS's part.
#[derive(Clone, Copy, Debug)]
pub struct DrumsSound {
    k: usize,
}

impl DrumsSound {
    /// The part of the fighter on side `k`.
    pub fn new(k: usize) -> DrumsSound {
        DrumsSound { k }
    }

    /// `drum(piece, vel)`: a kit piece in the fighter's own voice.
    fn drum(&self, ctx: &MusicCtx, pc: &str, vel: f64) -> Samples {
        let pp = ctx.cfg[self.k].pp.as_str();
        let cap = if pc == "C" { 1.8 } else { 0.45 };
        let v = ((0.5 + 0.5 * vel.min(1.0)) * 10.0).round() / 10.0;
        let x = ctx.note(pp, piece(pc), 0.2, cap, v)?;
        Ok(x.iter().map(|s| s * (0.55 + 0.45 * vel)).collect())
    }

    /// `colour(x, k, t, drive_k)`: `(signal, gain)`.
    fn tone(&self, ctx: &MusicCtx, x: &[f64], t: f64, drive_k: f64) -> Result<(Vec<f64>, f64), MusicError> {
        let (y, g, _) = self.colour(ctx, x, t, drive_k, &self.colour_args())?;
        Ok((y, g))
    }
}

impl Sound for DrumsSound {
    fn k(&self) -> usize {
        self.k
    }

    /// #28's kit bus.
    fn bus_gain(&self) -> f64 {
        2.6
    }

    fn send(&self) -> f64 {
        0.25
    }

    fn band(&self) -> (f64, f64) {
        (60.0, 6000.0)
    }

    /// A crash and a kick.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        mix.place(&bus, &Sig::Mono(self.drum(ctx, "C", 1.0)?), t, 0.40, 0.0);
        mix.place(&bus, &Sig::Mono(self.drum(ctx, "K", 1.0)?), t, 0.50, 0.0);
        Ok(())
    }

    /// The sticks on each count, a snare pickup into FIGHT!.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let s16 = ctx.show.grid.s16;
        let count = ctx.show.grid.count;
        for (tc, _) in count.iter().take(3) {
            mix.place(&bus, &Sig::Mono(sticks(1.0)), *tc, 0.30, 0.0);
        }
        let t_fight = count[3].0;
        for j in 1..=2 {
            let x = self.drum(ctx, "S", 0.5 + 0.2 * f64::from(j))?;
            mix.place(&bus, &Sig::Mono(x), t_fight - f64::from(3 - j) * s16, 0.20, -0.15);
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
            if e.who() != Some(k) {
                continue;
            }
            let tq = ctx.q16(t);
            match e.kind.as_str() {
                "pulse" => {
                    let hp = ctx.hp_at(k, t);
                    let main = e.i(3) != 0;
                    let pos = ((tq / beat).round() as i64).rem_euclid(4);
                    let pan = -0.15;
                    if !main {
                        // DRUM ROLL: the 16th pulses are a snare roll
                        let (sig, g) = self.tone(ctx, &self.drum(ctx, "S", 0.5)?, t, 0.6)?;
                        mix.place(&bus, &Sig::Mono(sig), tq, 0.22 * g, pan);
                        continue;
                    }
                    let lead = (hp - ctx.hp_at(1 - k, t)) / 100.0;
                    if pos == 0 || pos == 2 {
                        let (sig, g) = self.tone(ctx, &self.drum(ctx, "K", 1.0)?, t, 0.4)?;
                        mix.place(&bus, &Sig::Mono(sig), tq, 0.52 * g, 0.0);
                    } else if hp > 25.0 || (hp > 12.0 && pos == 3) {
                        let (sig, g) = self.tone(ctx, &self.drum(ctx, "S", 0.9)?, t, 0.6)?;
                        mix.place(&bus, &Sig::Mono(sig), tq, 0.40 * g, pan);
                    } else if hp > 12.0 {
                        // a ghost of the snare
                        let ghost = synth::hp(&self.drum(ctx, "S", 0.3)?, 900.0, 2);
                        mix.place(&bus, &Sig::Mono(ghost), tq, 0.06, pan);
                    }
                    if lead > 0.04 {
                        // the leader: off-beat hats
                        let at = tq + beat / 2.0;
                        mix.place(&bus, &Sig::Mono(synth::hat(0.03, false)), at, 0.10, 0.3);
                        if pos == 3 {
                            mix.place(&bus, &Sig::Mono(synth::hat(0.03, true)), at, 0.05, 0.3);
                        }
                    }
                }
                "hit" => {
                    if !event_is_str(H, e, 7, "pulse")? {
                        continue;
                    }
                    // a landed pulse: a tom fill
                    let hp = ctx.hp_at(k, t);
                    let fill: &[&str] = if hp > 45.0 {
                        &["T1", "T2", "T3"]
                    } else if hp > 25.0 {
                        &["T3"]
                    } else {
                        &[]
                    };
                    for (j, pc) in fill.iter().enumerate() {
                        let jf = j as f64;
                        let (sig, g) = self.tone(ctx, &self.drum(ctx, pc, 0.85 - 0.1 * jf)?, t, 0.5)?;
                        mix.place(&bus, &Sig::Mono(sig), tq + (jf + 1.0) * s16, 0.30 * g, -0.3 + 0.3 * jf);
                    }
                    let thud = synth::thud(0.3, 130.0, 55.0, THUD_SEED);
                    mix.place("fx", &Sig::Mono(thud), tq, 0.12, 0.0);
                }
                "ability" => {
                    let is = |name: &str| event_is_str(H, e, 3, name);
                    if is(&cfg.unlock1.name)? {
                        // DRUM ROLL ends on a crash and a kick
                        let at = tq + 4.0 * beat;
                        mix.place(&bus, &Sig::Mono(self.drum(ctx, "C", 0.9)?), at, 0.28, 0.0);
                        mix.place(&bus, &Sig::Mono(self.drum(ctx, "K", 1.0)?), at, 0.40, 0.0);
                    } else if is(&cfg.unlock2.name)? {
                        // CRASH CYMBAL
                        mix.place(&bus, &Sig::Mono(self.drum(ctx, "C", 1.0)?), tq, 0.42, 0.0);
                    } else if is(&cfg.low.name)? {
                        // DOUBLE KICK: the beater swings up
                        let x = synth::whoosh(2.0 * beat, 200.0, 2500.0, true, synth::WHOOSH_SEED);
                        mix.place("fx", &Sig::Mono(x), tq, 0.10, 0.0);
                    }
                }
                "slam" => {
                    // DOUBLE KICK: the tap an 8th early, then the stomp
                    let tap = ctx.q16(t - beat / 2.0);
                    mix.place(&bus, &Sig::Mono(self.drum(ctx, "K", 0.75)?), tap, 0.40, 0.0);
                    mix.place(&bus, &Sig::Mono(self.drum(ctx, "K", 1.0)?), t, 0.62, 0.0);
                    let thud = synth::thud(0.6, 90.0, 32.0, THUD_SEED);
                    mix.place("fx", &Sig::Mono(thud), t, 0.55, 0.0);
                    mix.place(&bus, &Sig::Mono(self.drum(ctx, "C", 0.8)?), t, 0.16, 0.0);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// The kit collapsing, crash to kick.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let t_hs = ctx.show.t_hs;
        for (j, pc) in ["C", "T1", "T2", "T3", "S", "K"].iter().enumerate() {
            let jf = j as f64;
            mix.place(
                &bus,
                &Sig::Mono(self.drum(ctx, pc, 0.8)?),
                t_hs + 0.08 * jf,
                0.26,
                (jf - 2.5) / 6.0,
            );
        }
        Ok(())
    }

    /// Crash, kick and the floor tom, then the floor tom again.
    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        for (pc, v) in [("C", 1.0), ("K", 1.0), ("T3", 0.9)] {
            mix.place("solo", &Sig::Mono(self.drum(ctx, pc, v)?), tw, 0.40, 0.0);
        }
        let s16 = ctx.show.grid.s16;
        mix.place("solo", &Sig::Mono(self.drum(ctx, "T3", 0.8)?), tw + s16, 0.26, 0.0);
        Ok(())
    }

    /// #28's drum solo; it stops at the reassembly.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, pc, vel) in SOLO {
            let ts = t0 + bt * grid.beat;
            if ts >= grid.t_reassemble - 0.05 {
                break;
            }
            let pan = match pc {
                "T1" => -0.25,
                "T3" => 0.25,
                _ => 0.0,
            };
            let gain = if pc == "C" { 0.36 } else { 0.48 };
            mix.place("solo", &Sig::Mono(self.drum(ctx, pc, vel)?), ts, gain, pan);
        }
        Ok(())
    }

    /// Its pieces fly home: a snare roll swelling into the glide.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let (t0, t1) = (grid.t_reassemble, grid.t_glide);
        let step = grid.s16 / 2.0;
        let mut t = t0;
        while t < t1 - 1e-4 {
            let u = (t - t0) / (t1 - t0).max(1e-9);
            let vel = ((0.4 + 0.6 * u) * 10.0).round() / 10.0;
            mix.place(
                &bus,
                &Sig::Mono(self.drum(ctx, "S", vel)?),
                t,
                0.10 + 0.14 * u * u,
                -0.1,
            );
            t += step;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::sticks;

    #[test]
    fn the_sticks_click_short_and_the_same_each_time() {
        let a = sticks(1.0);
        assert_eq!(a.len(), (0.09 * bf_sound::SRF) as usize);
        assert_eq!(a, sticks(1.0));
    }
}
