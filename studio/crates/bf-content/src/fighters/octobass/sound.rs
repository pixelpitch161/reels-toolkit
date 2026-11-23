//! OCTOBASS's part in the score: written for the Studio (design
//! `docs/superpowers/specs/2026-11-21-octobass-fighter-design.md`), on the same music context as every other part.
//!
//! The lowest strings in the roster play the bass line an octave down, one long bowed note a beat, thinner as it
//! loses. Each lever punch is a wooden clunk, a landed one a felt thud; FOOT PEDAL is a metal snap; BELOW HEARING
//! cuts the whole band to near silence (every bus but its own) while a sub rumble swells, then one huge boom and
//! the band is back.

use bf_core::dmath;
use bf_core::harmony::HarmonyError;
use bf_core::py::{self, py_max, round_nd};
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, Samples, Thinner, event_is_str};
use bf_sound::synth::{self, FILTER_O};
use bf_sound::traits::{Bed, Played, Sound};
use bf_sound::{SR, SRF};

use super::state;

/// The name the part's errors carry.
const H: &str = "OCTOBASS";

/// How low the band goes during BELOW HEARING.
const HUSH: f64 = 0.06;

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// `int(x * SR)`.
fn samples(seconds: f64) -> usize {
    (py_max(0.0, seconds) * SRF) as usize
}

/// A sub rumble: very low band-passed noise swelling in over `dur` seconds.
fn rumble(dur: f64, seed: u64) -> Vec<f64> {
    let n = samples(dur);
    let x = synth::bp(&synth::noise(seed, n), 25.0, 110.0, FILTER_O);
    synth::tt(n)
        .iter()
        .zip(&x)
        .map(|(&t, v)| v * dmath::pow((t / dur.max(1e-6)).clamp(0.0, 1.0), 1.5))
        .collect()
}

/// The gain the band has at `t` around a silence `(a, b)`: down over 0.25 s, back at once on the boom.
fn hush_gain(t: f64, a: f64, b: f64) -> f64 {
    if t < a || t >= b {
        1.0
    } else {
        let u = ((t - a) / 0.25).clamp(0.0, 1.0);
        1.0 - (1.0 - HUSH) * u
    }
}

/// OCTOBASS's part.
#[derive(Clone, Copy, Debug)]
pub struct OctobassSound {
    k: usize,
}

impl OctobassSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> OctobassSound {
        OctobassSound { k }
    }

    /// A little to its own side.
    fn pan(&self) -> f64 {
        if self.k == 0 { -0.15 } else { 0.15 }
    }

    /// A string note held for `n16` sixteenths.
    fn note(&self, ctx: &MusicCtx, m: i32, n16: f64, vel: f64) -> Samples {
        let dur = round_nd(py_max(0.12, n16 * ctx.show.grid.s16 * 0.95), 3);
        let x = ctx.note(
            &ctx.cfg[self.k].pp,
            f64::from(m),
            dur,
            round_nd(dur + 0.3, 2),
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

    /// The bass line's note at `t`, an octave down: the line when it has one there, else the chord's root.
    fn line(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        match ctx.line_at(self.name(ctx), t)? {
            Some(m) => Ok(Self::fit(m - 12, 24, 40)),
            None => ctx.harmony.root_above(ctx.harmony.chord_at(t), 24).map_err(bad),
        }
    }

    /// The chord's root at `t` from C1 up.
    fn root(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        ctx.harmony.root_above(ctx.harmony.chord_at(t), 24).map_err(bad)
    }

    /// BELOW HEARING: every bus but its own falls to near silence through each of its silences.
    fn hush(&self, ctx: &MusicCtx, mix: &mut Mix) {
        let quiet = state::silences(&ctx.rec.events, self.k, ctx.show.t_ko);
        if quiet.is_empty() {
            return;
        }
        let own = self.bus(ctx);
        let names: Vec<String> = mix.bus_names().iter().map(|s| s.to_string()).collect();
        for name in names.iter().filter(|n| **n != own) {
            let b = mix.bus_mut(name);
            for &(a, z) in &quiet {
                let (i0, i1) = ((a * SRF) as usize, ((z * SRF) as usize).min(b.len()));
                for (i, s) in b.iter_mut().enumerate().take(i1).skip(i0) {
                    let g = hush_gain(i as f64 / SR as f64, a, z);
                    s[0] *= g;
                    s[1] *= g;
                }
            }
        }
    }
}

impl Sound for OctobassSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.5
    }

    fn send(&self) -> f64 {
        0.2
    }

    /// The very bottom of the band.
    fn band(&self) -> (f64, f64) {
        (25.0, 900.0)
    }

    /// One huge low note under the VS card.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let r = self.root(ctx, 0.0)?;
        mix.place(&bus, &Sig::Mono(self.note(ctx, r, 8.0, 1.0)?), t, 0.45, pan);
        mix.place(&bus, &Sig::Mono(synth::thud(0.6, 80.0, 30.0, 5)), t, 0.30, pan);
        Ok(())
    }

    /// The count: three low plucks, then the fifth into the fight.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let r = self.root(ctx, grid.bar)?;
        for (tc, _) in grid.count.iter().take(3) {
            mix.place(&bus, &Sig::Mono(self.note(ctx, r, 1.5, 0.9)?), *tc, 0.32, pan);
        }
        let m = self.line(ctx, grid.t_fight)?;
        mix.place(
            &bus,
            &Sig::Mono(self.note(ctx, m + 7, 2.0, 0.9)?),
            grid.t_fight - 2.0 * grid.s16,
            0.28,
            pan,
        );
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let (bus, pan) = (self.bus(ctx), self.pan());
        let (s16, t_ko) = (ctx.show.grid.s16, ctx.show.t_ko);
        let t0 = ctx.show.grid.t_fight;
        let quiet = state::silences(&ctx.rec.events, k, t_ko);
        // the bass line an octave down, a long bowed note every beat, thinned as it loses
        let mut thin = Thinner::new(3.0);
        let mut slot: i64 = py::round_half_even(ctx.q16(t0) / s16) as i64;
        while slot as f64 * s16 < t_ko {
            let tq = slot as f64 * s16;
            let hp = ctx.hp_at(k, tq) / 100.0;
            if thin.keep(slot as f64, hp) && !state::within(&quiet, tq) {
                let m = self.line(ctx, tq)?;
                let (sig, g, _) = self.colour(ctx, &self.note(ctx, m, 4.0, 0.8)?, tq, 1.0, &self.colour_args())?;
                mix.place(&bus, &Sig::Mono(sig), tq, 0.30 * g, pan);
            }
            slot += 4;
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
                // a lever swinging out: a dry wooden clunk
                "lever" => {
                    let seed = (e.t * 1000.0) as u64 % 997;
                    mix.place("fx", &Sig::Mono(synth::thud(0.09, 260.0, 120.0, seed)), e.t, 0.10, pan);
                }
                // a fist stopping on the enemy: a felt thud
                "hit" if event_is_str(H, e, 7, "lever")? || event_is_str(H, e, 7, "seven")? => {
                    mix.place("fx", &Sig::Mono(synth::thud(0.22, 140.0, 48.0, 13)), e.t, 0.22, pan);
                }
                // the pedal snapping up: a metal snap and a low knock
                "pedal_snap" => {
                    mix.place("fx", &Sig::Mono(synth::hat(0.09, false)), e.t, 0.18, pan);
                    mix.place("fx", &Sig::Mono(synth::thud(0.3, 180.0, 60.0, 17)), e.t, 0.24, pan);
                }
                // BELOW HEARING: a sub rumble swelling under the silence
                "below_start" => {
                    let end = quiet
                        .iter()
                        .find(|w| (w.0 - e.t).abs() < 1e-6)
                        .map_or(e.t + 1.0, |w| w.1);
                    let r = self.root(ctx, tq)?;
                    let dur = py_max(0.3, end - e.t);
                    mix.place(&bus, &Sig::Mono(rumble(dur, 23)), e.t, 0.5, pan);
                    let n16 = dur / s16;
                    mix.place(&bus, &Sig::Mono(self.note(ctx, r, n16, 0.7)?), e.t, 0.22, pan);
                }
                // the boom: the band comes back on it
                "below_hit" => {
                    let r = self.root(ctx, tq)?;
                    mix.place(&bus, &Sig::Mono(synth::thud(1.4, 90.0, 26.0, 29)), e.t, 0.6, 0.0);
                    mix.place(&bus, &Sig::Mono(synth::kick(1.0, 0.5, 120.0, 34.0)), e.t, 0.4, 0.0);
                    for (j, (m, g)) in [(r, 0.36), (r + 7, 0.22), (r + 12, 0.24)].into_iter().enumerate() {
                        mix.place(
                            &bus,
                            &Sig::Mono(self.note(ctx, m, 8.0, 1.0)?),
                            e.t + 0.01 * j as f64,
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

    /// The strings snap: three falling plucks and a deep knock. The band's silences are cut in first (every part
    /// has played by now).
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        self.hush(ctx, mix);
        let t0 = ctx.show.t_hs;
        let (bus, pan) = (self.bus(ctx), self.pan());
        let m0 = self.line(ctx, t0)? + 12;
        for j in 0..3u32 {
            let m = m0 - 5 * j as i32;
            mix.place(
                &bus,
                &Sig::Mono(self.note(ctx, m, 1.5, 0.8)?),
                t0 + 0.14 * f64::from(j),
                0.26,
                pan,
            );
        }
        mix.place("fx", &Sig::Mono(synth::thud(0.5, 120.0, 40.0, 41)), t0, 0.3, pan);
        Ok(())
    }

    /// The winner's stinger; the band's silences are cut in first (every part has played by now).
    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        self.hush(ctx, mix);
        let r = self.root(ctx, 0.0)?;
        let s16 = ctx.show.grid.s16;
        mix.place("solo", &Sig::Mono(self.note(ctx, r, 4.0, 1.0)?), tw, 0.42, 0.0);
        mix.place(
            "solo",
            &Sig::Mono(self.note(ctx, r + 7, 8.0, 1.0)?),
            tw + 4.0 * s16,
            0.36,
            0.0,
        );
        Ok(())
    }

    /// The solo, on its strings (the octave above, so the tune reads).
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
                &Sig::Mono(self.note(ctx, Self::fit(m, 34, 58), n16, 0.9)?),
                ts,
                0.50,
                0.0,
            );
        }
        Ok(())
    }

    /// Its pieces come home on two slow bowed notes.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let r = self.root(ctx, grid.t_reassemble)?;
        let t = ctx.q16(grid.t_reassemble);
        mix.place("solo", &Sig::Mono(self.note(ctx, r + 12, 4.0, 0.7)?), t, 0.22, 0.0);
        mix.place(
            "solo",
            &Sig::Mono(self.note(ctx, r + 19, 4.0, 0.6)?),
            t + 4.0 * grid.s16,
            0.18,
            0.0,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rumble_swells_and_the_band_hushes() {
        let x = rumble(0.5, 3);
        assert_eq!(x.len(), samples(0.5));
        assert!(x.iter().all(|v| v.is_finite()));
        assert!(x[0].abs() < 1e-9);
        assert_eq!(hush_gain(0.5, 1.0, 2.0), 1.0);
        assert!(hush_gain(1.5, 1.0, 2.0) < 0.1);
        assert_eq!(hush_gain(2.0, 1.0, 2.0), 1.0);
        assert_eq!(OctobassSound::fit(70, 24, 40), 34);
    }
}
