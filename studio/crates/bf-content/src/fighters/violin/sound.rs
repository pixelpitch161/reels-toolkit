//! VIOLIN's part in the score, from #42's `audio.py` (its VIOLIN section, 425 to 502, and the helpers `bow_note`
//! 110, `scratch` 142, `pizz` 150, `squeal` 162, as #28 made them). A quick port (ruling M4-R16): the episode's
//! ideas on the Studio's music context, not its samples.
//!
//! The bow's turns play the line: every quarter turn of the spinning bow (`RUN['TH']`) is a bowed note on the next
//! 16th, held until the next one (1 to 8 sixteenths), thinner and dirtier as it loses, with rosin scratch when it is
//! hurt and a double stop a sixth below while it leads. A landed bow climbs the scale with an accent; a landed dart is
//! a pluck; PIZZICATO plucks a rising run on the 16ths; TREMOLO shivers in 32nds over the line; SCREECH is the
//! beginner-violin squeal with a high scraped note.

use bf_core::dmath::{self, PI};
use bf_core::event::Event;
use bf_core::harmony::HarmonyError;
use bf_core::py::{self, py_max, round_nd};
use bf_core::recording::Array;
use bf_sound::SRF;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, Samples, Thinner, event_int, event_is_str, noise_seed};
use bf_sound::synth::{self, FILTER_O, clip};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "VIOLIN";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// `int(x * SR)`.
fn samples(seconds: f64) -> usize {
    (py_max(0.0, seconds) * SRF) as usize
}

/// `scratch(dur, seed)`: bow scratch, rosin noise, band-limited, gritty.
fn scratch(dur: f64, seed: u64) -> Vec<f64> {
    let n = samples(dur);
    let x = synth::bp(&synth::noise(seed, n), 1800.0, 7000.0, FILTER_O);
    let env = synth::expenv(n, dur * 0.4, 0.004);
    let tt = synth::tt(n);
    x.iter()
        .zip(&env)
        .zip(&tt)
        .map(|((v, e), &t)| {
            let s = dmath::sin(2.0 * PI * 37.0 * t);
            v * (1.0 + 0.8 * s.signum() * f64::from(u8::from(s != 0.0))) * e
        })
        .collect()
}

/// `pizz(m, vel)`: a plucked string with a little wooden body.
fn pizz(m: i32, vel: f64) -> Vec<f64> {
    let n = samples(0.5);
    let f = synth::mtof(f64::from(m));
    let body = synth::bp(&synth::noise(m.unsigned_abs().into(), n), 800.0, 3000.0, FILTER_O);
    let x: Vec<f64> = synth::tt(n)
        .iter()
        .zip(&body)
        .map(|(&t, &b)| {
            let w = 2.0 * PI * f * t;
            (dmath::sin(w)
                + 0.5 * dmath::sin(2.0 * w) * dmath::exp(-t / 0.05)
                + 0.25 * dmath::sin(3.0 * w) * dmath::exp(-t / 0.03))
                * dmath::exp(-t / 0.16)
                + 0.15 * b * dmath::exp(-t / 0.01)
        })
        .collect();
    synth::lp(&x, 4500.0, FILTER_O).iter().map(|v| v * vel).collect()
}

/// `squeal(dur)`: SCREECH, a wobbling, scratchy whistle sliding up.
fn squeal(dur: f64) -> Vec<f64> {
    let n = samples(dur);
    let tt = synth::tt(n);
    let (mut p1, mut p2) = (0.0, 0.0);
    let x: Vec<f64> = tt
        .iter()
        .map(|&t| {
            let sq = dmath::sin(2.0 * PI * 3.1 * t);
            let f = 2300.0 + 500.0 * dmath::sin(2.0 * PI * 6.5 * t) + 900.0 * t + 300.0 * sq.signum();
            p1 += f;
            p2 += f * 2.02;
            let v = dmath::sin(2.0 * PI * p1 / SRF) + 0.35 * dmath::sin(2.0 * PI * p2 / SRF);
            dmath::tanh(2.8 * v) * clip(t / 0.03, 0.0, 1.0) * dmath::exp(-t / (dur * 0.55))
        })
        .collect();
    let s = scratch(dur, 13);
    synth::bp(&x, 700.0, 8000.0, FILTER_O)
        .iter()
        .zip(&s)
        .map(|(a, b)| a + 0.5 * b)
        .collect()
}

/// VIOLIN's part.
#[derive(Clone, Copy, Debug)]
pub struct ViolinSound {
    k: usize,
}

impl ViolinSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> ViolinSound {
        ViolinSound { k }
    }

    /// A little to its own side.
    fn pan(&self) -> f64 {
        if self.k == 0 { -0.2 } else { 0.2 }
    }

    /// `bow_note(m, n16, vel)`: a bowed note held for `n16` sixteenths.
    fn bow_note(&self, ctx: &MusicCtx, m: i32, n16: f64, vel: f64) -> Samples {
        let dur = round_nd(py_max(0.07, n16 * ctx.show.grid.s16 * 0.92), 3);
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

    /// The line's note at `t` in its register (the chord's tone near the middle of it on a rest).
    fn line(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let (lo, hi) = ctx.cfg[self.k].register;
        match ctx.line_at(self.name(ctx), t)? {
            Some(m) => Ok(Self::fit(m, lo, hi - 7)),
            None => {
                let tones = ctx
                    .harmony
                    .chord_tones(lo + 7, hi - 7, ctx.harmony.chord_at(t))
                    .map_err(bad)?;
                Ok(tones.first().copied().unwrap_or(lo + 12))
            }
        }
    }

    /// The note a bowed note sounds at `t`: as [`ViolinSound::line`], read through [`MusicCtx::play_line`] (sound
    /// upgrade: under Composed the note is counted there, and in a moving style it is never the note bowed before
    /// it).
    fn sounded(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let (lo, hi) = ctx.cfg[self.k].register;
        match ctx.play_line(self.name(ctx), t)? {
            Some(m) => Ok(Self::fit(m, lo, hi - 7)),
            None => self.line(ctx, t),
        }
    }

    /// The chord tones from A4 to A6 at `t` (the plucks' pitches).
    fn plucks(&self, ctx: &MusicCtx, t: f64) -> Result<Vec<i32>, MusicError> {
        let ms = ctx.harmony.chord_tones(69, 93, ctx.harmony.chord_at(t)).map_err(bad)?;
        if ms.is_empty() {
            return Err(MusicError::new(H, "the chord has no tones from A4 to A6"));
        }
        Ok(ms)
    }

    /// The times the bow passes each quarter turn, from `RUN['TH']` (its total turning, unwrapped).
    fn quarter_turns(&self, ctx: &MusicCtx) -> Vec<f64> {
        let Some(Array::F64 { shape, data }) = ctx.rec.arrays.get("TH") else {
            return Vec::new();
        };
        if shape.len() != 2 || shape[1] != 2 {
            return Vec::new();
        }
        let th: Vec<f64> = data.iter().skip(self.k).step_by(2).copied().collect();
        let (t0, fps) = (ctx.show.grid.t_fight, ctx.show.grid.fps);
        let mut out = Vec::new();
        let (mut phi, mut kq) = (0.0, 1.0);
        for i in 1..th.len() {
            let d = py::wrap_pi(th[i] - th[i - 1]).abs();
            let prev = phi;
            phi += d;
            while phi >= kq * PI / 2.0 {
                let u = (kq * PI / 2.0 - prev) / py_max(1e-9, phi - prev);
                out.push(t0 + (i as f64 - 1.0 + u) / f64::from(fps));
                kq += 1.0;
            }
        }
        out
    }

    /// Its own events up to the K.O.
    fn mine<'c>(&self, ctx: &'c MusicCtx) -> impl Iterator<Item = &'c Event> + 'c {
        let (k, t_ko) = (self.k, ctx.show.t_ko);
        ctx.rec
            .events
            .iter()
            .take_while(move |e| e.t <= t_ko + 0.01)
            .filter(move |e| e.who() == Some(k))
    }
}

impl Sound for ViolinSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.4
    }

    fn send(&self) -> f64 {
        0.34
    }

    /// Its singing register, high over the band.
    fn band(&self) -> (f64, f64) {
        (300.0, 3200.0)
    }

    /// A bowed double stop swelling under the VS card (the fight file's title hits, else the root and fifth).
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let hits = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(h) if !h.is_empty() => h.clone(),
            _ => {
                let r = ctx.harmony.root_above(ctx.harmony.chord_at(0.0), 67).map_err(bad)?;
                vec![r, r + 7]
            }
        };
        for (j, &m) in hits.iter().take(3).enumerate() {
            let x = self.bow_note(ctx, m, 6.0, 0.9)?;
            mix.place(&bus, &Sig::Mono(x), t + 0.04 * j as f64, 0.30 - 0.06 * j as f64, pan);
        }
        Ok(())
    }

    /// The count: three pizzicato plucks climbing, then a bowed pickup into the fight.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let ms = self.plucks(ctx, grid.bar)?;
        for (j, (tc, _)) in grid.count.iter().take(3).enumerate() {
            let m = ms[(j * 2).min(ms.len() - 1)];
            mix.place(&bus, &Sig::Mono(pizz(m, 0.9)), *tc, 0.2, pan);
        }
        let m = self.line(ctx, grid.t_fight)?;
        mix.place(
            &bus,
            &Sig::Mono(self.bow_note(ctx, m, 3.0, 0.9)?),
            grid.t_fight - 2.0 * grid.s16,
            0.22,
            pan,
        );
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let (bus, pan) = (self.bus(ctx), self.pan());
        let s16 = ctx.show.grid.s16;
        let t_ko = ctx.show.t_ko;
        // the bow's turns play the line
        let mut slots: Vec<i64> = self
            .quarter_turns(ctx)
            .into_iter()
            .filter(|&t| t < t_ko)
            .map(|t| py::round_half_even(ctx.q16(t) / s16) as i64)
            .collect();
        slots.sort_unstable();
        slots.dedup();
        let mut thin = Thinner::new(3.0);
        // sound upgrade: under Composed the bow sounds one note at a time (a double stop is one event), and its
        // notes are softer the higher they are above C6
        let name = self.name(ctx);
        let one = ctx.one_voice(name);
        for (j, &slot) in slots.iter().enumerate() {
            let tq = slot as f64 * s16;
            let n16 = slots.get(j + 1).map_or(4, |&nx| (nx - slot).clamp(1, 8)) as f64;
            let hp = ctx.hp_at(k, tq) / 100.0;
            if !thin.keep(slot as f64, hp) {
                continue;
            }
            let m = self.sounded(ctx, tq)?;
            let vel = 0.62 + 0.25 * (2.0 / n16).min(1.0);
            let (sig, g, lead) = self.colour(ctx, &self.bow_note(ctx, m, n16, vel)?, tq, 1.0, &self.colour_args())?;
            mix.note(&bus, &Sig::Mono(sig), tq, 0.34 * g * ctx.soften(name, m), pan, one);
            if 1.0 - hp > 0.25 {
                let x = scratch((n16 * s16).min(0.25), slot.unsigned_abs());
                mix.place(&bus, &Sig::Mono(x), tq, 0.09 * dmath::pow(1.0 - hp, 1.3), pan);
            }
            if lead > 0.04 {
                // the leader: a double stop a sixth below
                let under = ctx
                    .harmony
                    .chord_tones(m - 10, m - 7, ctx.harmony.chord_at(tq))
                    .map_err(bad)?;
                if let Some(&u) = under.first() {
                    let x = self
                        .colour(ctx, &self.bow_note(ctx, u, n16, 0.6)?, tq, 1.0, &self.colour_args())?
                        .0;
                    mix.note(&bus, &Sig::Mono(x), tq, 0.14 * g, pan, one);
                }
            }
        }
        let climb = ctx.harmony.scale_notes(74, 98, None).map_err(bad)?;
        for e in self.mine(ctx) {
            let (t, tq) = (e.t, ctx.q16(e.t));
            match e.kind.as_str() {
                "hit" if event_is_str(H, e, 7, "blade")? && !climb.is_empty() => {
                    // a landed bow: accents climbing the scale
                    let sub = event_int(H, e, 8)?;
                    let mut m = climb[py::imod_py(sub, climb.len() as i64) as usize];
                    if ctx.composed_on() {
                        // the written line's note at the hit, not the next step of the climb
                        m = self.sounded(ctx, tq)?;
                    }
                    let (sig, g, _) =
                        self.colour(ctx, &self.bow_note(ctx, m, 1.0, 1.0)?, t, 1.0, &self.colour_args())?;
                    let p = clip(e.f(5) / 3.5, -0.6, 0.6);
                    mix.note(&bus, &Sig::Mono(sig), tq, 0.30 * g * ctx.soften(name, m), p, one);
                    mix.place(&bus, &Sig::Mono(scratch(0.05, noise_seed(H, t * 11.0)?)), tq, 0.05, p);
                }
                "hit" if event_is_str(H, e, 7, "key")? => {
                    // a pizzicato dart lands
                    let ms = self.plucks(ctx, tq)?;
                    let sub = event_int(H, e, 8)?;
                    let m = ms[py::imod_py(sub * 2, ms.len() as i64) as usize];
                    let x = self.colour(ctx, &pizz(m, 0.9), t, 1.0, &self.colour_args())?.0;
                    mix.place(&bus, &Sig::Mono(x), tq, 0.30, clip(e.f(5) / 3.5, -0.6, 0.6));
                }
                "hit" if event_is_str(H, e, 7, "wave")? => {
                    mix.place("fx", &Sig::Mono(synth::thud(0.45, 110.0, 40.0, 7)), tq, 0.30, 0.0);
                }
                "ability" if event_is_str(H, e, 3, "PIZZICATO")? => {
                    let n = ctx.cfg[k].unlock1.get("n").unwrap_or(5.0) as usize;
                    let ms = self.plucks(ctx, tq)?;
                    for j in 0..n {
                        let m = ms[(j * 2) % ms.len()];
                        let x = self.colour(ctx, &pizz(m, 0.8), t, 1.0, &self.colour_args())?.0;
                        mix.place(&bus, &Sig::Mono(x), tq + j as f64 * s16, 0.24, pan);
                    }
                }
                "ability" if event_is_str(H, e, 3, "TREMOLO")? => {
                    let dur = ctx.cfg[k].unlock2.get("dur").unwrap_or(3.0);
                    let mut j = 0.0;
                    while j * s16 / 2.0 < dur {
                        let tk = tq + j * s16 / 2.0;
                        let m = self.line(ctx, tk)? + 12;
                        let x = self
                            .colour(ctx, &self.bow_note(ctx, m, 1.0, 0.5)?, t, 1.0, &self.colour_args())?
                            .0;
                        mix.place(&bus, &Sig::Mono(x), tk, 0.07 * (0.5 + 0.5 * j * s16 / 2.0 / dur), pan);
                        j += 1.0;
                    }
                }
                "ability" if event_is_str(H, e, 3, "SCREECH")? => {
                    mix.place(&bus, &Sig::Mono(squeal(1.9)), tq, 0.14, pan);
                    let x = self
                        .colour(ctx, &self.bow_note(ctx, 96, 6.0, 1.0)?, t, 2.0, &self.colour_args())?
                        .0;
                    mix.place(&bus, &Sig::Mono(x), tq, 0.16, pan);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// The bow scrapes off the strings: a falling run of short bowed notes with rosin scratch.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t0 = ctx.show.t_hs;
        let (bus, pan) = (self.bus(ctx), self.pan());
        let m0 = self.line(ctx, t0)? + 12;
        for j in 0..6u32 {
            let m = m0 - 2 * j as i32;
            let tj = t0 + 0.1 * f64::from(j);
            mix.place(&bus, &Sig::Mono(self.bow_note(ctx, m, 1.0, 0.8)?), tj, 0.20, pan);
            mix.place(
                &bus,
                &Sig::Mono(synth::hp(&scratch(0.06, 40 + u64::from(j)), 3000.0, FILTER_O)),
                tj,
                0.08,
                pan,
            );
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let r = ctx.harmony.root_above(ctx.harmony.chord_at(0.0), 67).map_err(bad)?;
        for (j, m) in [r, r + 4, r + 7, r + 12].into_iter().enumerate() {
            let n16 = if j < 3 { 1.0 } else { 8.0 };
            let x = self.bow_note(ctx, m, n16, 1.0)?;
            mix.place(
                "solo",
                &Sig::Mono(x),
                tw + j as f64 * ctx.show.grid.s16 / 2.0,
                0.30,
                0.0,
            );
        }
        Ok(())
    }

    /// The fight file's solo, bowed.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let n16 = py_max(1.0, (d * 4.0).round());
            mix.place("solo", &Sig::Mono(self.bow_note(ctx, m, n16, 0.9)?), ts, 0.46, 0.0);
        }
        Ok(())
    }

    /// Its pieces fly home as rising pizzicato.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let ms = self.plucks(ctx, grid.t_reassemble)?;
        let n = ms.len().clamp(1, 6);
        for (j, &m) in ms.iter().take(n).enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / n as f64);
            mix.place("solo", &Sig::Mono(pizz(m, 0.7)), tj, 0.18, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sounds_have_their_lengths_and_ring() {
        assert_eq!(pizz(76, 0.8).len(), samples(0.5));
        assert_eq!(squeal(1.9).len(), samples(1.9));
        assert_eq!(scratch(0.1, 5).len(), samples(0.1));
        assert!(pizz(76, 0.8).iter().all(|v| v.is_finite()));
        assert!(squeal(0.5).iter().any(|v| v.abs() > 0.01));
        assert_eq!(ViolinSound::fit(100, 62, 81), 76);
    }
}
