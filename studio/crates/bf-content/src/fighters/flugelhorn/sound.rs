//! FLUGELHORN's part in the score: written for the Studio (design `docs/studio/2026-11-16-flugelhorn-design.md`), on
//! the same music context as every other part.
//!
//! The mellow horn holds the tune softly on every other beat, thinner as it loses. Every WINGBEAT clap is the flank
//! call: the root, then up a sixth; FLANKING CALL is a quick rising call up the chord; FEATHER SHOWER a fluttering
//! trill; TAKE WING a long swell that breaks into a held high chord on the giant clap. It falls apart in a soft
//! falling sigh; the solo glides in long notes.

use bf_core::harmony::HarmonyError;
use bf_core::py::{self, py_max, round_nd};
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, Samples, Thinner, event_is_str};
use bf_sound::synth::{self, FILTER_O};
use bf_sound::traits::{Bed, Played, Sound};

use super::state::claps;

/// The name the part's errors carry.
const H: &str = "FLUGELHORN";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// `int(x * SR)`.
fn samples(seconds: f64) -> usize {
    (py_max(0.0, seconds) * bf_sound::SRF) as usize
}

/// A soft rush of air (the wings): band-passed noise swelling in and out over `dur` seconds.
fn whoosh(dur: f64, seed: u64, lo: f64, hi: f64) -> Vec<f64> {
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

/// FLUGELHORN's part.
#[derive(Clone, Copy, Debug)]
pub struct FlugelSound {
    k: usize,
}

impl FlugelSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> FlugelSound {
        FlugelSound { k }
    }

    /// A little to its own side.
    fn pan(&self) -> f64 {
        if self.k == 0 { -0.2 } else { 0.2 }
    }

    /// A horn note held for `n16` sixteenths.
    fn horn(&self, ctx: &MusicCtx, m: i32, n16: f64, vel: f64) -> Samples {
        let dur = round_nd(py_max(0.07, n16 * ctx.show.grid.s16 * 0.95), 3);
        let x = ctx.note(
            &ctx.cfg[self.k].pp,
            f64::from(Self::fit(m, 52, 77)),
            dur,
            round_nd(dur + 0.2, 2),
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

    /// The tune's note at `t`: the line when it has one there, else the chord's third, in the middle.
    fn line(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        match ctx.line_at(self.name(ctx), t)? {
            Some(m) => Ok(Self::fit(m, 55, 74)),
            None => Ok(self.root(ctx, t)? + 4),
        }
    }

    /// The note of the tune it sounds at `t`: [`FlugelSound::line`] for a note that really plays, asked for once the
    /// note is known to sound. Under the Composed arrangement the context keeps what was played, and a moving melody
    /// style never hands back the note before it (sound upgrade); under Classic it is `line`.
    fn sounded(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        match ctx.play_line(self.name(ctx), t)? {
            Some(m) => Ok(Self::fit(m, 55, 74)),
            None => Ok(self.root(ctx, t)? + 4),
        }
    }

    /// The note of a flank call at `t`: the chord's root and `up` semitones. Under the Composed arrangement it is
    /// the written line's note there instead (sound upgrade: every pitched fighter plays its line), the root's own
    /// on a rest of the line.
    fn call(&self, ctx: &MusicCtx, t: f64, up: i32) -> Result<i32, MusicError> {
        if ctx.composed_on()
            && let Some(m) = ctx.play_line(self.name(ctx), t)?
        {
            return Ok(Self::fit(m, 55, 74));
        }
        Ok(self.root(ctx, t)? + up)
    }

    /// The chord's root at `t` from G3 up.
    fn root(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        ctx.harmony.root_above(ctx.harmony.chord_at(t), 55).map_err(bad)
    }
}

impl Sound for FlugelSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.3
    }

    fn send(&self) -> f64 {
        0.30
    }

    /// The warm middle of the band.
    fn band(&self) -> (f64, f64) {
        (160.0, 3200.0)
    }

    /// The flank call under the VS card, with a rush of wings.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let r = self.root(ctx, 0.0)?;
        let s16 = ctx.show.grid.s16;
        mix.place(&bus, &Sig::Mono(self.horn(ctx, r, 3.0, 0.9)?), t, 0.36, pan);
        mix.place(
            &bus,
            &Sig::Mono(self.horn(ctx, r + 9, 6.0, 1.0)?),
            t + 3.0 * s16,
            0.34,
            pan,
        );
        mix.place("fx", &Sig::Mono(whoosh(0.7, 7, 400.0, 3000.0)), t, 0.08, pan);
        Ok(())
    }

    /// The count: three soft roots, then the sixth into the fight.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let r = self.root(ctx, grid.bar)?;
        for (tc, _) in grid.count.iter().take(3) {
            mix.place(&bus, &Sig::Mono(self.horn(ctx, r, 2.0, 0.7)?), *tc, 0.24, pan);
        }
        mix.place(
            &bus,
            &Sig::Mono(self.horn(ctx, r + 9, 2.0, 0.8)?),
            grid.t_fight - 2.0 * grid.s16,
            0.24,
            pan,
        );
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let (bus, pan) = (self.bus(ctx), self.pan());
        let (s16, t_ko) = (ctx.show.grid.s16, ctx.show.t_ko);
        let t0 = ctx.show.grid.t_fight;
        // the tune, held softly on every other beat, thinned as it loses
        let mut thin = Thinner::new(3.0);
        // sound upgrade (M5): under Composed a horn sounds one note at a time, so a new note ends the one before
        let one = ctx.one_voice(self.name(ctx));
        let mut slot: i64 = py::round_half_even(ctx.q16(t0) / s16) as i64;
        while slot as f64 * s16 < t_ko {
            let tq = slot as f64 * s16;
            let hp = ctx.hp_at(k, tq) / 100.0;
            if thin.keep(slot as f64, hp) {
                let m = self.sounded(ctx, tq)?;
                let (sig, g, _) = self.colour(ctx, &self.horn(ctx, m, 7.0, 0.7)?, tq, 1.0, &self.colour_args())?;
                mix.note(&bus, &Sig::Mono(sig), tq, 0.22 * g, pan, one);
            }
            slot += 8;
        }
        // its claps: the flank call, the root and then up a sixth, with a rush of wings (Composed: the line's note
        // at each)
        for c in claps(&ctx.rec.events, k).into_iter().filter(|c| c.t0 < t_ko) {
            let tq = ctx.q16(c.t0);
            let r = self.call(ctx, tq, 0)?;
            let (a, g, _) = self.colour(ctx, &self.horn(ctx, r, 1.0, 0.8)?, tq, 1.0, &self.colour_args())?;
            mix.note(&bus, &Sig::Mono(a), tq, 0.22 * g, pan, one);
            let sixth = if ctx.composed_on() {
                self.call(ctx, tq + s16, 9)?
            } else {
                r + 9
            };
            let (b, _, _) = self.colour(ctx, &self.horn(ctx, sixth, 2.0, 0.9)?, tq, 1.0, &self.colour_args())?;
            mix.note(&bus, &Sig::Mono(b), tq + s16, 0.20 * g, pan, one);
            let seed = (c.t0 * 100.0) as u64;
            mix.place("fx", &Sig::Mono(whoosh(0.22, seed, 600.0, 4000.0)), tq - 0.1, 0.05, pan);
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
                "ability" if event_is_str(H, e, 3, "FLANKING CALL")? => {
                    // a quick rising call up the chord
                    let r = self.root(ctx, tq)?;
                    for (j, d) in [0, 4, 7, 9].into_iter().enumerate() {
                        let tj = tq + j as f64 * s16;
                        mix.place(&bus, &Sig::Mono(self.horn(ctx, r + d, 1.0, 0.9)?), tj, 0.22, pan);
                    }
                    mix.place("fx", &Sig::Mono(whoosh(0.6, 13, 500.0, 3500.0)), tq, 0.07, pan);
                }
                "ability" if event_is_str(H, e, 3, "FEATHER SHOWER")? => {
                    // the flutter: a trill on the third and the fourth, on 16ths for a bar
                    let r = self.root(ctx, tq)?;
                    for j in 0..8usize {
                        let tj = tq + j as f64 * s16;
                        if tj >= t_ko {
                            break;
                        }
                        let m = if j % 2 == 0 { r + 4 } else { r + 5 };
                        mix.place(&bus, &Sig::Mono(self.horn(ctx, m, 1.0, 0.6)?), tj, 0.14, pan);
                    }
                }
                "take_wing" => {
                    // the wings spread: a long swell of air under a held root
                    let r = self.root(ctx, tq)?;
                    mix.place(&bus, &Sig::Mono(self.horn(ctx, r, 4.0, 0.7)?), tq, 0.20, pan);
                    mix.place("fx", &Sig::Mono(whoosh(0.9, 21, 300.0, 2500.0)), tq, 0.10, pan);
                }
                "wing_clap" => {
                    // the giant clap: a thump and a held high chord
                    let r = self.root(ctx, tq)?;
                    mix.place("fx", &Sig::Mono(synth::thud(0.4, 140.0, 60.0, 17)), tq, 0.30, 0.0);
                    for (j, (m, g)) in [(r + 12, 0.26), (r + 16, 0.18), (r + 21, 0.16)].into_iter().enumerate() {
                        mix.place(
                            &bus,
                            &Sig::Mono(self.horn(ctx, m, 10.0, 1.0)?),
                            tq + 0.01 * j as f64,
                            g,
                            pan,
                        );
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// The horn sighs apart: three soft falling notes and a puff of air.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t0 = ctx.show.t_hs;
        let (bus, pan) = (self.bus(ctx), self.pan());
        let m0 = self.line(ctx, t0)? + 7;
        for j in 0..3u32 {
            let m = m0 - 3 * j as i32;
            mix.place(
                &bus,
                &Sig::Mono(self.horn(ctx, m, 2.0, 0.7)?),
                t0 + 0.16 * f64::from(j),
                0.20,
                pan,
            );
        }
        mix.place("fx", &Sig::Mono(whoosh(0.6, 41, 800.0, 5000.0)), t0, 0.08, pan);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let r = self.root(ctx, 0.0)?;
        let s16 = ctx.show.grid.s16;
        mix.place("solo", &Sig::Mono(self.horn(ctx, r, 3.0, 0.9)?), tw, 0.36, 0.0);
        mix.place(
            "solo",
            &Sig::Mono(self.horn(ctx, r + 9, 10.0, 1.0)?),
            tw + 3.0 * s16,
            0.34,
            0.0,
        );
        Ok(())
    }

    /// The solo, on the horn, a little longer than written: it glides.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let n16 = py_max(1.0, (d * 4.0).round() + 1.0);
            mix.place("solo", &Sig::Mono(self.horn(ctx, m, n16, 0.85)?), ts, 0.44, 0.0);
        }
        Ok(())
    }

    /// Its pieces come home on the flank call, an octave up.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let r = self.root(ctx, grid.t_reassemble)?;
        let t = ctx.q16(grid.t_reassemble);
        mix.place("solo", &Sig::Mono(self.horn(ctx, r + 12, 3.0, 0.7)?), t, 0.18, 0.0);
        mix.place(
            "solo",
            &Sig::Mono(self.horn(ctx, r + 21, 4.0, 0.6)?),
            t + 3.0 * grid.s16,
            0.15,
            0.0,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whoosh_has_its_length_and_fades() {
        let x = whoosh(0.5, 3, 400.0, 3000.0);
        assert_eq!(x.len(), samples(0.5));
        assert!(x.iter().all(|v| v.is_finite()));
        assert!(x[0].abs() < 1e-9);
        assert_eq!(FlugelSound::fit(90, 52, 77), 66);
    }
}
