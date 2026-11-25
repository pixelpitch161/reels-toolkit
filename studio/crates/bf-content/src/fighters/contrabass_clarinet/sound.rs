//! CONTRABASS CLARINET's part in the score: written for the Studio (design
//! `docs/studio/2026-11-16-contrabass-clarinet-design.md`), on the same music context as every other part.
//!
//! The lowest voice in the roster plays the bass line on the beats, thinner as it loses. Every FOGHORN blast is the
//! foghorn call (the root, then the fifth above it, long, with a breath of air); a wall slam is a sub boom; FOG BANK
//! is a swell of dark noise; MOTORBOAT rattles the root in "putt-putt" 16ths; SHIP'S HORN is one huge held chord.

use bf_core::harmony::HarmonyError;
use bf_core::py::{self, py_max, round_nd};
use bf_sound::SRF;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, Samples, Thinner, event_is_str};
use bf_sound::synth::{self, FILTER_O};
use bf_sound::traits::{Bed, Played, Sound};

use super::state::horns;

/// The name the part's errors carry.
const H: &str = "CONTRABASS CLARINET";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// `int(x * SR)`.
fn samples(seconds: f64) -> usize {
    (py_max(0.0, seconds) * SRF) as usize
}

/// A breath of dark air: band-passed noise swelling in and out over `dur` seconds.
fn air(dur: f64, seed: u64, lo: f64, hi: f64) -> Vec<f64> {
    let n = samples(dur);
    let x = synth::bp(&synth::noise(seed, n), lo, hi, FILTER_O);
    synth::tt(n)
        .iter()
        .zip(&x)
        .map(|(&t, v)| {
            let u = (t / dur.max(1e-6)).clamp(0.0, 1.0);
            v * bf_core::dmath::sin(bf_core::dmath::PI * u)
        })
        .collect()
}

/// CONTRABASS CLARINET's part.
#[derive(Clone, Copy, Debug)]
pub struct ContraSound {
    k: usize,
}

impl ContraSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> ContraSound {
        ContraSound { k }
    }

    /// A little to its own side.
    fn pan(&self) -> f64 {
        if self.k == 0 { -0.15 } else { 0.15 }
    }

    /// A reed note held for `n16` sixteenths.
    fn reed(&self, ctx: &MusicCtx, m: i32, n16: f64, vel: f64) -> Samples {
        let dur = round_nd(py_max(0.07, n16 * ctx.show.grid.s16 * 0.95), 3);
        let x = ctx.note(
            &ctx.cfg[self.k].pp,
            f64::from(m),
            dur,
            round_nd(dur + 0.25, 2),
            round_nd(vel, 1),
        )?;
        Ok(x.to_vec())
    }

    /// `m` moved by octaves into `lo..=hi`.
    fn fit(m: i32, lo: i32, hi: i32) -> i32 {
        let mut m = m;
        while m > hi {
            m -= 12;
        }
        while m < lo {
            m += 12;
        }
        m
    }

    /// The bass line's note at `t`: the line when it has one there, else the chord's root, low.
    fn line(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        match ctx.line_at(self.name(ctx), t)? {
            Some(m) => Ok(Self::fit(m, 28, 45)),
            None => ctx.harmony.root_above(ctx.harmony.chord_at(t), 28).map_err(bad),
        }
    }

    /// The note of the bass line it sounds at `t`: [`ContraSound::line`] for a note that really plays, asked for once
    /// the note is known to sound. Under the Composed arrangement the context keeps what was played, and a moving
    /// melody style never hands back the note before it (sound upgrade); under Classic it is `line`.
    fn sounded(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        match ctx.play_line(self.name(ctx), t)? {
            Some(m) => Ok(Self::fit(m, 28, 45)),
            None => ctx.harmony.root_above(ctx.harmony.chord_at(t), 28).map_err(bad),
        }
    }

    /// The note of a foghorn blast at `t`: the chord's root and `up` semitones. Under the Composed arrangement it
    /// is the written bass line's note there instead (sound upgrade: every pitched fighter plays its line), the
    /// root's own on a rest of the line.
    fn call(&self, ctx: &MusicCtx, t: f64, up: i32) -> Result<i32, MusicError> {
        if ctx.composed_on()
            && let Some(m) = ctx.play_line(self.name(ctx), t)?
        {
            return Ok(Self::fit(m, 28, 45));
        }
        Ok(self.root(ctx, t)? + up)
    }

    /// The chord's root at `t` from D1 up.
    fn root(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        ctx.harmony.root_above(ctx.harmony.chord_at(t), 26).map_err(bad)
    }
}

impl Sound for ContraSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.5
    }

    fn send(&self) -> f64 {
        0.22
    }

    /// The very bottom of the band.
    fn band(&self) -> (f64, f64) {
        (30.0, 900.0)
    }

    /// The foghorn call under the VS card.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let r = self.root(ctx, 0.0)?;
        mix.place(&bus, &Sig::Mono(self.reed(ctx, r, 6.0, 1.0)?), t, 0.42, pan);
        mix.place(&bus, &Sig::Mono(self.reed(ctx, r + 12, 6.0, 0.8)?), t + 0.02, 0.18, pan);
        mix.place("fx", &Sig::Mono(air(1.0, 7, 120.0, 900.0)), t, 0.10, pan);
        Ok(())
    }

    /// The count: three low root notes, then the fifth into the fight.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let r = self.root(ctx, grid.bar)?;
        for (tc, _) in grid.count.iter().take(3) {
            mix.place(&bus, &Sig::Mono(self.reed(ctx, r, 2.0, 0.9)?), *tc, 0.30, pan);
        }
        let m = self.line(ctx, grid.t_fight)?;
        mix.place(
            &bus,
            &Sig::Mono(self.reed(ctx, m + 7, 2.0, 0.9)?),
            grid.t_fight - 2.0 * grid.s16,
            0.26,
            pan,
        );
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let (bus, pan) = (self.bus(ctx), self.pan());
        let (s16, t_ko) = (ctx.show.grid.s16, ctx.show.t_ko);
        let t0 = ctx.show.grid.t_fight;
        // the bass line, a note on every beat, thinned as it loses
        let mut thin = Thinner::new(3.0);
        // sound upgrade (M5): under Composed a clarinet sounds one note at a time, so a new note ends the one before
        let one = ctx.one_voice(self.name(ctx));
        let mut slot: i64 = py::round_half_even(ctx.q16(t0) / s16) as i64;
        while slot as f64 * s16 < t_ko {
            let tq = slot as f64 * s16;
            let hp = ctx.hp_at(k, tq) / 100.0;
            if thin.keep(slot as f64, hp) {
                let m = self.sounded(ctx, tq)?;
                let (sig, g, _) = self.colour(ctx, &self.reed(ctx, m, 3.0, 0.8)?, tq, 1.0, &self.colour_args())?;
                mix.note(&bus, &Sig::Mono(sig), tq, 0.30 * g, pan, one);
            }
            slot += 4;
        }
        // its blasts: the foghorn call, the root and then the fifth, long (Composed: the line's note at each)
        for h in horns(&ctx.rec.events, k).into_iter().filter(|h| h.t0 < t_ko) {
            let tq = ctx.q16(h.t0);
            let r = self.call(ctx, tq, 0)?;
            let (a, g, _) = self.colour(ctx, &self.reed(ctx, r, 4.0, 1.0)?, tq, 1.0, &self.colour_args())?;
            mix.note(&bus, &Sig::Mono(a), tq, 0.36 * g, pan, one);
            let fifth = if ctx.composed_on() {
                self.call(ctx, tq + 4.0 * s16, 7)?
            } else {
                r + 7
            };
            let (b, _, _) = self.colour(ctx, &self.reed(ctx, fifth, 4.0, 0.9)?, tq, 1.0, &self.colour_args())?;
            mix.note(&bus, &Sig::Mono(b), tq + 4.0 * s16, 0.28 * g, pan, one);
            let seed = (h.t0 * 100.0) as u64;
            mix.place("fx", &Sig::Mono(air(h.dur + 0.3, seed, 90.0, 700.0)), tq, 0.08, pan);
        }
        let mine = ctx
            .rec
            .events
            .iter()
            .take_while(|e| e.t <= t_ko + 0.01)
            .filter(|e| e.who() == Some(k));
        for e in mine {
            let tq = ctx.q16(e.t);
            match e.kind.as_str() {
                "hit" if event_is_str(H, e, 7, "wall")? => {
                    // the wall slam: a sub boom
                    mix.place("fx", &Sig::Mono(synth::thud(0.6, 90.0, 32.0, 11)), tq, 0.40, 0.0);
                }
                "ability" if event_is_str(H, e, 3, "FOG BANK")? => {
                    mix.place("fx", &Sig::Mono(air(2.4, 21, 60.0, 500.0)), tq, 0.16, pan);
                }
                "ability" if event_is_str(H, e, 3, "MOTORBOAT")? => {
                    let beats = ctx.cfg[k].unlock2.get("dur_beats").unwrap_or(8.0);
                    let n = (beats * 4.0) as usize;
                    for j in 0..n {
                        // putt-putt: the offbeat rattle cell
                        if ![0, 3, 6, 8, 11, 14].contains(&(j % 16)) {
                            continue;
                        }
                        let tj = tq + j as f64 * s16;
                        if tj >= t_ko {
                            break;
                        }
                        let r = self.root(ctx, tj)?;
                        mix.place(&bus, &Sig::Mono(self.reed(ctx, r, 1.0, 0.7)?), tj, 0.16, pan);
                    }
                }
                "beam" => {
                    // SHIP'S HORN: one huge held chord
                    let r = self.root(ctx, tq)?;
                    for (j, (m, g)) in [(r, 0.40), (r + 7, 0.26), (r + 12, 0.22)].into_iter().enumerate() {
                        mix.place(
                            &bus,
                            &Sig::Mono(self.reed(ctx, m, 12.0, 1.0)?),
                            tq + 0.01 * j as f64,
                            g,
                            pan,
                        );
                    }
                    mix.place("fx", &Sig::Mono(air(1.6, 31, 70.0, 1200.0)), tq, 0.14, pan);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// The tube coughs apart: three falling low notes and a puff of air.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t0 = ctx.show.t_hs;
        let (bus, pan) = (self.bus(ctx), self.pan());
        let m0 = self.line(ctx, t0)? + 12;
        for j in 0..3u32 {
            let m = m0 - 5 * j as i32;
            mix.place(
                &bus,
                &Sig::Mono(self.reed(ctx, m, 2.0, 0.8)?),
                t0 + 0.14 * f64::from(j),
                0.24,
                pan,
            );
        }
        mix.place("fx", &Sig::Mono(air(0.8, 41, 200.0, 2000.0)), t0, 0.10, pan);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let r = self.root(ctx, 0.0)?;
        let s16 = ctx.show.grid.s16;
        mix.place("solo", &Sig::Mono(self.reed(ctx, r, 4.0, 1.0)?), tw, 0.40, 0.0);
        mix.place(
            "solo",
            &Sig::Mono(self.reed(ctx, r + 7, 8.0, 1.0)?),
            tw + 4.0 * s16,
            0.34,
            0.0,
        );
        Ok(())
    }

    /// The solo, on the reed.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let n16 = py_max(1.0, (d * 4.0).round());
            mix.place(
                "solo",
                &Sig::Mono(self.reed(ctx, Self::fit(m, 26, 58), n16, 0.9)?),
                ts,
                0.50,
                0.0,
            );
        }
        Ok(())
    }

    /// Its pieces come home on a slow foghorn call.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let r = self.root(ctx, grid.t_reassemble)?;
        let t = ctx.q16(grid.t_reassemble);
        mix.place("solo", &Sig::Mono(self.reed(ctx, r + 12, 4.0, 0.7)?), t, 0.20, 0.0);
        mix.place(
            "solo",
            &Sig::Mono(self.reed(ctx, r + 19, 4.0, 0.6)?),
            t + 4.0 * grid.s16,
            0.16,
            0.0,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn air_has_its_length_and_fades() {
        let x = air(0.5, 3, 100.0, 800.0);
        assert_eq!(x.len(), samples(0.5));
        assert!(x.iter().all(|v| v.is_finite()));
        assert!(x[0].abs() < 1e-9);
        assert_eq!(ContraSound::fit(70, 26, 58), 58);
    }
}
