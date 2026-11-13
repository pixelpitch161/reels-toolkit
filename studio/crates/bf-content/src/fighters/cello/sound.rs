//! CELLO's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/cello.py` (#47).
//!
//! The Pixel Pitch cello sings the tango's melody (the fight file's line, a note a beat; repeated notes tie into
//! long bows, up to four beats), fuller as it leads, thinner and dirtier as it loses (`colour`, and `Thinner(47 +
//! k)` dropping notes); a rush slides up into the next note, a grab swells it with a double stop, the hold bows with
//! a deeper vibrato, a throw is a marcato stab and a rope slam a crunching double stop: the band stops for that beat
//! ([`Sound::stops`]). VIBRATO trembles on the 16ths; PRELUDE plays Bach's rocking figure over the chords (a note per
//! note-wave); SWAN SONG plays The Swan's opening phrase in the episode's key.
//!
//! Every note is `bow(m, dur, vel, vib)`: the CELLO's voice through `M.note`, with `M.vibrato` when `vib` is above
//! 0. The voice draws one number a note from the violin module's generator, so the notes are asked for in the kit's
//! order. The part reads nothing off its look.

use bf_core::dmath::{self, PI};
use bf_core::event::Event;
use bf_core::harmony::HarmonyError;
use bf_core::py::{self, py_max, round_nd};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    ColourArgs, MusicCtx, MusicError, Samples, Thinner, bend, event_int, event_is_str, gliss, noise_seed, vibrato,
};
use bf_sound::synth::{self, FILTER_O, WHOOSH_RISE, WHOOSH_SEED, clip};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "CELLO";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// `PRELUDE_FIG`: Bach, Cello Suite No. 1, Prelude, bar 1 (public domain), as chord degrees over whatever chord is
/// playing: `(chord tone index, octave above the root)`, `None` for the scale step under the third.
const PRELUDE_FIG: [(Option<usize>, i32); 8] = [
    (Some(0), 0),
    (Some(2), 0),
    (Some(1), 1),
    (None, 1),
    (Some(1), 1),
    (Some(2), 0),
    (Some(1), 1),
    (Some(2), 0),
];

/// `SWAN`: Saint-Saens, The Swan (public domain), the cello's opening phrase as `(beat, semitones above the key's
/// tonic, beats)`.
const SWAN: [(f64, i32, f64); 10] = [
    (0.0, 12, 1.5),
    (1.5, 11, 0.5),
    (2.0, 4, 2.0),
    (4.0, 9, 1.5),
    (5.5, 7, 0.5),
    (6.0, 0, 2.0),
    (8.0, 2, 0.5),
    (8.5, 4, 0.5),
    (9.0, 5, 1.0),
    (10.0, 4, 2.0),
];

/// Python's `int(x * SR)` for a length in samples, cut toward zero.
fn samples(seconds: f64) -> Result<usize, MusicError> {
    let n = seconds * SRF;
    if n.is_finite() && n > -1.0 && n < 1.0e9 {
        Ok(n as usize)
    } else {
        Err(MusicError::new(H, format!("{seconds} s is not a length")))
    }
}

/// The error for a rest in its line where the part needs a note (Python compares `None` with a number).
fn rest(name: &str, t: f64) -> MusicError {
    MusicError::new(
        H,
        format!("{name}'s line has a rest at {t} s: this part plays a note on every beat"),
    )
}

/// `x * np.clip(np.linspace(a, b, len(x)), lo, hi)`.
fn ramp(x: &[f64], a: f64, b: f64, lo: f64, hi: f64) -> Vec<f64> {
    let r = dsp::linspace(a, b, x.len());
    x.iter().zip(&r).map(|(v, &g)| v * clip(g, lo, hi)).collect()
}

/// `any(a <= t < b for a, b in spans)`.
fn in_spans(spans: &[(f64, f64)], t: f64) -> bool {
    spans.iter().any(|&(a, b)| a <= t && t < b)
}

/// The CELLO's part.
#[derive(Clone, Copy, Debug)]
pub struct CelloSound {
    k: usize,
}

impl CelloSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> CelloSound {
        CelloSound { k }
    }

    /// `self.pan`: a little left in the left corner, right in the right.
    fn pan(&self) -> f64 {
        if self.k == 0 { -0.18 } else { 0.18 }
    }

    /// `(self.lo, self.hi)`: its roster `register`.
    fn reg(&self, ctx: &MusicCtx) -> (i32, i32) {
        ctx.cfg[self.k].register
    }

    /// The events of its own side: `[e for e in M.EV if e[1] == kind and e[2] == k]`.
    fn mine<'c>(&self, ctx: &'c MusicCtx, kind: &'static str) -> impl Iterator<Item = &'c Event> + 'c {
        let k = self.k;
        ctx.rec
            .events
            .iter()
            .filter(move |e| e.kind == kind && e.who() == Some(k))
    }

    /// `bow(m, dur, vel=0.85, vib=0.0)`: one bowed note in its voice, with a vibrato when `vib` is above 0:
    ///
    /// ```python
    /// x = self.M.note(self.pp, int(m), round(max(0.08, dur), 3), round(max(0.3, dur + 0.25), 2), round(vel, 2))
    /// if vib > 0:
    ///     x = self.M.vibrato(x, vib, 5.6, 0.06)
    /// ```
    fn bow(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64, vib: f64) -> Samples {
        let pp = &ctx.cfg[self.k].pp;
        let x = ctx.note(
            pp,
            f64::from(m),
            round_nd(py_max(0.08, dur), 3),
            round_nd(py_max(0.3, dur + 0.25), 2),
            round_nd(vel, 2),
        )?;
        if vib > 0.0 {
            vibrato(&x, vib, 5.6, 0.06)
        } else {
            Ok(x.to_vec())
        }
    }

    /// `fit_reg(m, lo=None, hi=None)`: `m` moved by octaves into `lo..hi` (its register by default; Python's `or`
    /// takes the default for a 0 too).
    fn fit_reg(&self, ctx: &MusicCtx, m: i32, lo: Option<i32>, hi: Option<i32>) -> i32 {
        let (rl, rh) = self.reg(ctx);
        let lo = lo.filter(|&v| v != 0).unwrap_or(rl);
        let hi = hi.filter(|&v| v != 0).unwrap_or(rh);
        let mut m = m;
        while m > hi {
            m -= 12;
        }
        while m < lo {
            m += 12;
        }
        m
    }

    /// `T.line_at(self.name, t)` where the part needs a note.
    fn line(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let name = self.name(ctx);
        ctx.line_at(name, t)?.ok_or_else(|| rest(name, t))
    }

    /// `root_of(t)`: the root of the chord playing at `t`, at or above C3 (48).
    fn root_of(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        ctx.harmony.root_above(ctx.harmony.chord_at(t), 48).map_err(bad)
    }

    /// `self.colour(x, t)` (and with `drive_k`).
    fn col(&self, ctx: &MusicCtx, x: &[f64], t: f64, drive_k: f64) -> Result<(Vec<f64>, f64, f64), MusicError> {
        self.colour(ctx, x, t, drive_k, &self.colour_args())
    }

    /// `prelude_note(t, j)`: bar 1 of Bach's prelude as degrees of the chord playing at `t`.
    fn prelude_note(&self, ctx: &MusicCtx, t: f64, j: i64) -> Result<i32, MusicError> {
        let h = &ctx.harmony;
        let ch = h.chord_at(t);
        let pcs = h.chord_pcs(ch).map_err(bad)?;
        let r = h.root_above(ch, 43).map_err(bad)?;
        let (deg, octv) = PRELUDE_FIG[py::imod_py(j, 8) as usize];
        let pc = |i: usize| {
            pcs.get(i).copied().ok_or_else(|| {
                MusicError::new(
                    H,
                    format!(
                        "PRELUDE reads tone {} of the chord {ch:?}, which has {}",
                        i + 1,
                        pcs.len()
                    ),
                )
            })
        };
        match deg {
            None => {
                // the step under the third (+8ve): a scale tone
                let third = r + 12 + (pc(1)? - pc(0)?).rem_euclid(12);
                let below = h.scale_notes(third - 3, third - 1, Some(ch)).map_err(bad)?;
                Ok(below.last().copied().unwrap_or(third - 2))
            }
            Some(d) => {
                let up = if d != 0 { (pc(d)? - pc(0)?).rem_euclid(12) } else { 0 };
                Ok(r + 12 * octv + up)
            }
        }
    }

    /// `swan_notes(t0)`: The Swan's opening phrase from `t0` in the episode's key (its scale decides the third and
    /// the sixth): `(time, midi, seconds)`.
    fn swan_notes(&self, ctx: &MusicCtx, t0: f64) -> Vec<(f64, i32, f64)> {
        let h = &ctx.harmony;
        let beat = ctx.show.grid.beat;
        let (lo, hi) = self.reg(ctx);
        let tonic = 48 + (h.key_pc - 48).rem_euclid(12);
        let in_scale = |semi: i32| {
            h.scale
                .iter()
                .any(|&s| (h.key_pc + s).rem_euclid(12) == (h.key_pc + semi).rem_euclid(12))
        };
        SWAN.iter()
            .map(|&(bt, semi, d)| {
                let mut m = tonic + semi;
                if semi.rem_euclid(12) == 4 && !in_scale(4) {
                    m -= 1; // minor third
                }
                if semi.rem_euclid(12) == 9 && !in_scale(9) {
                    m -= 1; // minor sixth
                }
                (
                    t0 + bt * beat,
                    self.fit_reg(ctx, m + 12, Some(lo + 7), Some(hi + 2)),
                    d * beat,
                )
            })
            .collect()
    }

    /// `melody()`: the melody's notes over the fight, `(t, midi, beats)`, with repeats tied (up to four beats) and
    /// gaps for PRELUDE and SWAN SONG (twelve beats from its trigger).
    fn melody(&self, ctx: &MusicCtx) -> Result<Vec<(f64, i32, i64)>, MusicError> {
        let beat = ctx.show.grid.beat;
        let mut cut: Vec<(f64, f64)> = self.mine(ctx, "prelude").map(|e| (e.t, e.f(3))).collect();
        cut.extend(self.mine(ctx, "swan").map(|e| (e.t, e.t + 12.0 * beat)));
        let mut notes: Vec<(f64, i32, i64)> = Vec::new();
        // b = int(math.ceil(T.T_FIGHT / T.BEAT - 1e-6))
        let mut b = (ctx.show.grid.t_fight / beat - 1e-6).ceil() as i64;
        while b as f64 * beat < ctx.show.t_ko - 0.05 {
            let t = b as f64 * beat;
            if !cut.iter().any(|&(a, c)| a - 1e-6 <= t && t < c) {
                let m = self.fit_reg(ctx, self.line(ctx, t)?, None, None);
                match notes.last_mut() {
                    Some(last) if last.1 == m && (last.0 + last.2 as f64 * beat - t).abs() < 1e-6 && last.2 < 4 => {
                        last.2 += 1;
                    }
                    _ => notes.push((t, m, 1)),
                }
            }
            b += 1;
        }
        Ok(notes)
    }
}

impl Sound for CelloSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.55
    }

    fn send(&self) -> f64 {
        0.38
    }

    /// Its singing register, above the bandoneon's stabs.
    fn band(&self) -> (f64, f64) {
        (180.0, 1400.0)
    }

    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 80.0,
            cut_span: 480.0,
            drive: 3.5,
            ..ColourArgs::default()
        }
    }

    /// `stops()`: the band's tango stops (a beat's cut) on its rope slams up to the K.O.
    fn stops(&self, ctx: &MusicCtx) -> Vec<f64> {
        self.mine(ctx, "ropeslam")
            .filter(|e| e.t <= ctx.show.t_ko)
            .map(|e| ctx.q16(e.t))
            .collect()
    }

    /// A cello swell: a bowed double stop crescendo (the fight file's title hits: its notes, else root and fifth).
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let beat = ctx.show.grid.beat;
        let hits = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(h) if !h.is_empty() => h.clone(),
            _ => {
                let r = self.root_of(ctx, 0.0)?;
                vec![r, r + 7]
            }
        };
        // the first named: the swell holds through VS
        let dur = beat * if t < beat * 0.5 { 1.95 } else { 0.95 };
        for (j, &m) in hits.iter().take(2).enumerate() {
            let x = self.bow(ctx, m, dur, 1.0, 0.25)?;
            let r = dsp::linspace(0.0, 1.0, x.len());
            let x: Vec<f64> = x
                .iter()
                .zip(&r)
                .map(|(v, &u)| v * clip(0.55 + 0.6 * dmath::pow(u, 0.7), 0.0, 1.1))
                .collect();
            mix.place(&bus, &Sig::Mono(x), t, 0.42 - 0.08 * j as f64, pan);
        }
        for (j, &m) in hits.iter().skip(2).enumerate() {
            let x = self.bow(ctx, m, 0.12, 0.9, 0.0)?;
            mix.place(&bus, &Sig::Mono(x), t + 0.05 + 0.05 * j as f64, 0.24, pan);
        }
        Ok(())
    }

    /// A long bow on the dominant's fifth under the count, the count on root, fifth and octave, and a slide into
    /// the first note of the fight.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let r = self.root_of(ctx, grid.bar)?;
        let ch1 = ctx
            .harmony
            .chords
            .get(1)
            .ok_or_else(|| MusicError::new(H, "the episode has no bar 1 to count in on"))?;
        let hold = self.fit_reg(ctx, ctx.harmony.root_above(ch1, 48).map_err(bad)? + 7, None, None);
        let x = self.bow(ctx, hold, 3.0 * grid.beat * 0.97, 0.85, 0.25)?;
        mix.place(&bus, &Sig::Mono(ramp(&x, 0.7, 1.05, 0.0, 1.05)), grid.bar, 0.24, pan);
        for (j, (tc, _)) in grid.count.iter().take(3).enumerate() {
            let x = self.bow(ctx, r + [0, 7, 12][j], 0.2, 0.9, 0.0)?;
            mix.place(&bus, &Sig::Mono(x), *tc, 0.34, pan);
        }
        let m = self.fit_reg(ctx, self.line(ctx, grid.t_fight)?, None, None);
        let x = gliss(&self.bow(ctx, m, grid.beat * 0.9, 0.95, 0.0)?, 7.0, 0.25)?;
        mix.place(&bus, &Sig::Mono(x), grid.bar + 3.0 * grid.beat, 0.30, pan);
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let (lo, hi) = self.reg(ctx);
        let mut th = Thinner::new(f64::from(47 + k as i32));
        let holds: Vec<(f64, f64)> = self.mine(ctx, "hold").map(|e| (e.t, e.f(5))).collect();
        let vib: Vec<(f64, f64)> = self.mine(ctx, "vibrato").map(|e| (e.t, e.f(3))).collect();
        let rushes: Vec<f64> = self.mine(ctx, "rush").map(|e| e.t).collect();
        // the singing line
        for (t, m, beats) in self.melody(ctx)? {
            let hp01 = ctx.hp_at(k, t) / 100.0;
            if !th.keep(py::round_half_even(t / s16), hp01) {
                continue;
            }
            let beats = beats as f64;
            let dur = beats * beat * 0.97;
            let mut v = if in_spans(&holds, t) { 0.32 } else { 0.18 };
            if in_spans(&vib, t) {
                v = 0.6;
            }
            let mut x = self.bow(ctx, m, dur, 0.8 + 0.05 * beats.min(3.0), v)?;
            if rushes.iter().any(|&r| 0.0 <= t - r && t - r < beat) {
                // a rush slides up into the note
                x = gliss(&x, 4.0, 0.12)?;
            }
            let (sig, g, _) = self.col(ctx, &x, t, 1.0)?;
            mix.place(&bus, &Sig::Mono(sig), t, 0.34 * g, pan);
        }
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
                "grab" if event_int(H, e, 3)? == 1 => {
                    // the embrace: a swelling double stop
                    let m = self.fit_reg(ctx, self.line(ctx, tq)?, None, None);
                    let low = lo.max(m - 9);
                    let x = self.bow(ctx, low, beat * 0.9, 0.9, 0.3)?;
                    let x = ramp(&x, 0.6, 1.1, 0.0, 1.1);
                    let sig = self.col(ctx, &x, t, 1.0)?.0;
                    let g = self.col(ctx, &x, t, 1.0)?.1;
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.24 * g, pan);
                }
                "throw" => {
                    // marcato stab
                    let r = self.root_of(ctx, tq)?;
                    let x = self.bow(ctx, r + 12, 0.16, 1.0, 0.0)?;
                    mix.place(&bus, &Sig::Mono(self.col(ctx, &x, t, 1.0)?.0), tq, 0.36, pan);
                    let whoosh = synth::whoosh(0.3, 500.0, 4000.0, WHOOSH_RISE, WHOOSH_SEED);
                    mix.place("fx", &Sig::Mono(whoosh), tq, 0.08, pan);
                }
                "ropeslam" => {
                    // a crunching double stop on the band's stop
                    let r = self.root_of(ctx, tq)?;
                    for (m, gg) in [(r, 0.36), (r + 7, 0.28), (r + 12, 0.18)] {
                        let x = self.col(ctx, &self.bow(ctx, m, 0.35, 1.0, 0.0)?, t, 2.0)?.0;
                        mix.place(&bus, &Sig::Mono(x), tq, gg, pan);
                    }
                    let thud = synth::thud(0.5, 140.0, 40.0, noise_seed(H, t * 9.0)?);
                    mix.place("fx", &Sig::Mono(thud), tq, 0.32, 0.0);
                }
                "hit" if event_is_str(H, e, 7, "vibrato")? => {
                    // VIBRATO: a bowed tremolo shake on the 16ths
                    let m = self.fit_reg(ctx, self.line(ctx, tq)?, None, None);
                    mix.place(&bus, &Sig::Mono(self.bow(ctx, m, 0.07, 0.75, 0.0)?), tq, 0.12, pan);
                }
                "prelude_note" => {
                    // PRELUDE: Bach's rocking figure, a note per wave
                    let j = event_int(H, e, 3)?;
                    let m = self.prelude_note(ctx, tq, j)?;
                    let x = self.bow(ctx, self.fit_reg(ctx, m, Some(lo - 5), Some(hi)), s16 * 1.6, 0.85, 0.0)?;
                    let (sig, g, _) = self.col(ctx, &x, t, 1.0)?;
                    let gain = if py::imod_py(j, 8) == 0 { 0.34 } else { 0.27 };
                    mix.place(&bus, &Sig::Mono(sig), tq, gain * g, pan);
                }
                "ability" if event_is_str(H, e, 4, "swan")? => {
                    // SWAN SONG: The Swan's opening phrase
                    for (ts, m, d) in self.swan_notes(ctx, ctx.q16(t)) {
                        if ts > ctx.show.t_ko + 0.05 {
                            break;
                        }
                        let mut x = self.bow(ctx, m, d * 0.98, 0.95, 0.35)?;
                        if d >= beat {
                            x = gliss(&x, 1.0, 0.08)?;
                        }
                        let (sig, g, _) = self.col(ctx, &x, ts, 1.0)?;
                        mix.place(&bus, &Sig::Mono(sig), ts, 0.42 * g, pan * 0.5);
                    }
                }
                "swan_land" => {
                    // the dive into the lake
                    mix.place("fx", &Sig::Mono(synth::thud(0.8, 120.0, 30.0, 7)), tq, 0.4, 0.0);
                    let n = samples(0.8)?;
                    let tt = synth::tt(n);
                    let splash: Vec<f64> = synth::bp(&synth::noise(5, n), 600.0, 7000.0, FILTER_O)
                        .iter()
                        .zip(&tt)
                        .map(|(v, &t)| v * dmath::exp(-t / 0.18))
                        .collect();
                    mix.place("fx", &Sig::Mono(splash), tq, 0.22, 0.0);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// The strings let go: a falling glissando and a snapped string.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t0 = ctx.show.t_hs;
        let r = self.root_of(ctx, t0)?;
        let x = bend(&self.bow(ctx, r + 12, 1.4, 1.0, 0.3)?, -14.0, 0.05, 1.3)?;
        mix.place(&self.bus(ctx), &Sig::Mono(x), t0, 0.40, self.pan());
        let n = samples(0.5)?;
        let w = 2.0 * PI * 1800.0;
        let snap: Vec<f64> = synth::tt(n)
            .iter()
            .map(|&t| dmath::sin(w * t * dmath::exp(-t * 3.0)) * dmath::exp(-t / 0.05))
            .collect();
        mix.place("fx", &Sig::Mono(snap), t0 + 0.08, 0.12, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let r = self.root_of(ctx, 0.0)?;
        for (m, g) in [(r, 0.4), (r + 7, 0.32), (r + 12, 0.24)] {
            mix.place("solo", &Sig::Mono(self.bow(ctx, m, 1.1, 1.0, 0.3)?), tw, g, 0.0);
        }
        Ok(())
    }

    /// The fight file's solo: the cello's tango cadenza, long notes slid into and bowed with a deep vibrato.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let vib = if d >= 1.0 { 0.4 } else { 0.15 };
            let mut x = self.bow(ctx, m, py_max(0.1, d * grid.beat * 0.96), 0.95, vib)?;
            if d >= 1.0 {
                x = gliss(&x, 2.0, 0.1)?;
            }
            mix.place("solo", &Sig::Mono(x), ts, 0.44, 0.0);
        }
        Ok(())
    }

    /// Its pieces fly home as rising pizzicato.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let r = self.root_of(ctx, 0.0)?;
        let (lo, _) = self.reg(ctx);
        let run = [-12, -5, 0, 3, 7, 12, 15, 19].map(|i| r + i);
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            let x = self.bow(ctx, self.fit_reg(ctx, m, Some(lo - 12), None), 0.12, 0.85, 0.0)?;
            mix.place("solo", &Sig::Mono(x), tj, 0.32, 0.0);
        }
        Ok(())
    }

    /// Into the title slam: a tremolo swell on the dominant's fifth.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let t_last = (grid.n_bars as f64 - 1.0) * grid.bar;
        let ch = ctx
            .harmony
            .chords
            .last()
            .ok_or_else(|| MusicError::new(H, "the episode has no bars"))?;
        let m = self.fit_reg(ctx, ctx.harmony.root_above(ch, 48).map_err(bad)? + 7, None, None);
        let n16 = py::round_half_even((grid.dur - t_last) / (grid.s16 / 2.0)) as i64;
        for j in 0..n16 {
            let tj = t_last + j as f64 * grid.s16 / 2.0;
            let x = self.bow(ctx, m, grid.s16 * 0.45, 0.8, 0.0)?;
            mix.place(&bus, &Sig::Mono(x), tj, 0.10 + 0.10 * j as f64 / n16.max(1) as f64, pan);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_figures_are_the_kits() {
        assert_eq!(PRELUDE_FIG.iter().filter(|f| f.0.is_none()).count(), 1);
        assert_eq!(SWAN.iter().map(|s| s.2).sum::<f64>(), 12.0);
        assert!(in_spans(&[(1.0, 2.0)], 1.0) && !in_spans(&[(1.0, 2.0)], 2.0));
        assert_eq!(ramp(&[2.0, 2.0], 0.6, 1.2, 0.0, 1.1), [1.2, 2.2]);
    }
}
