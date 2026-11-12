//! CONTRABASS's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/contrabass.py` (#47).
//!
//! Its steps are the walking bass line: every footfall a plucked note of its line on the beat (DOUBLE TIME's off-8th
//! steps a chromatic approach note), with a low thud under it; a landed quake adds a ghost slap; SLAP BASS is a
//! slapped note with two ghost slaps after it (and an arco snarl when it broke a grab); THE ELEPHANT plays the
//! elephant's waltz tune (Saint-Saens, The Carnival of the Animals, 1886, public domain) bowed and heavy over the
//! stampede, its stomps and tramples thudding; the title card gets a slapped hit.
//!
//! Every note is one of its voice's three articulations (`voice::{bowed, pizz, slap}`), resampled and brought to the
//! series' level (`M.inst_gain`), and kept by `(articulation, note, round(dur, 3), round(vel, 2))` as the Python
//! part keeps them: two calls whose length differs only past the third decimal share the first one's samples.
//! The part reads nothing off its look.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::{fmod_py, imod_py, py_max, round_half_even, round_nd};
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{ColourArgs, MusicCtx, MusicError, Thinner, bend, noise_seed};
use bf_sound::synth;
use bf_sound::traits::{Bed, Played, Sound};
use bf_sound::voice::pp;

use super::voice::{bowed, midi_hz, pizz, slap};

/// The name the part's errors carry.
const H: &str = "CONTRABASS";

/// `ELEPHANT`: Saint-Saens, L'elephant (The Carnival of the Animals, 1886, public domain): the double bass's pompous
/// waltz tune, as `(beat, semitones above the key's tonic, beats)` in the bass's register.
pub const ELEPHANT: [(f64, i32, f64); 12] = [
    (0.0, -5, 0.75),
    (0.75, 0, 0.75),
    (1.5, 4, 0.75),
    (2.25, 7, 0.75),
    (3.0, 12, 1.5),
    (4.5, 11, 0.5),
    (5.0, 9, 0.5),
    (5.5, 7, 1.0),
    (6.5, 5, 0.5),
    (7.0, 4, 0.5),
    (7.5, 2, 0.5),
    (8.0, 0, 1.5),
];

/// One of the voice's articulations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum How {
    /// `'arco'`: bowed.
    Arco,
    /// `'pizz'`: plucked.
    Pizz,
    /// `'slap'`: slapped (with no note, the ghost slap).
    Slap,
}

/// The key `art` caches by: `(how, None or int(m), round(dur, 3), round(vel, 2))`, the floats by their bits.
type ArtKey = (How, Option<i32>, u64, u64);

fn bits(x: f64) -> u64 {
    if x == 0.0 { 0 } else { x.to_bits() }
}

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// `fit_reg(m, lo, hi)`: `m` moved by octaves into `lo..=hi` (down first, then up, as the Python loops do).
pub fn fit_reg(mut m: i32, lo: i32, hi: i32) -> i32 {
    while m > hi {
        m -= 12;
    }
    while m < lo {
        m += 12;
    }
    m
}

/// CONTRABASS's part.
#[derive(Debug)]
pub struct ContrabassSound {
    k: usize,
    /// `self._c`: the notes made so far.
    notes: Mutex<BTreeMap<ArtKey, Arc<[f64]>>>,
}

impl ContrabassSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> ContrabassSound {
        ContrabassSound {
            k,
            notes: Mutex::new(BTreeMap::new()),
        }
    }

    /// `self.pan`: a little to its own side.
    fn pan(&self) -> f64 {
        if self.k == 0 { -0.1 } else { 0.1 }
    }

    /// `self.lo, self.hi = M.CFG[k]['register']`.
    fn register(&self, ctx: &MusicCtx) -> (i32, i32) {
        ctx.cfg[self.k].register
    }

    /// `art(how, m, dur, vel=0.9)`: one note of its voice in the articulation `how` (`m` `None`: the unpitched
    /// ghost slap), at 48 kHz and at the series' level: `_resample(<how>(_midi_hz(int(m)), dur, vel)) * g0`, kept by
    /// `(how, int(m), round(dur, 3), round(vel, 2))`.
    ///
    /// # Errors
    /// An argument is not a number (Python raises there); the voice's level cannot be measured.
    pub fn art(&self, ctx: &MusicCtx, how: How, m: Option<i32>, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        for (what, v) in [("dur", dur), ("vel", vel)] {
            if !v.is_finite() {
                return Err(MusicError::new(H, format!("a note's {what} is {v}, not a number")));
            }
        }
        // self.g0 = M.inst_gain(M.CFG[k]['pp']), measured when the part is made: the voice draws no shared noise,
        // so measuring it here (once: the context keeps it) gives the same soundtrack
        let g0 = ctx.inst_gain(&ctx.cfg[self.k].pp)?;
        let key = (how, m, bits(round_nd(dur, 3)), bits(round_nd(vel, 2)));
        let mut notes = self
            .notes
            .lock()
            .map_err(|_| MusicError::new(H, "a note panicked earlier"))?;
        if let Some(x) = notes.get(&key) {
            return Ok(Arc::clone(x));
        }
        let f = m.map(midi_hz);
        let x = match (how, f) {
            (How::Arco, Some(f)) => bowed(f, dur, vel),
            (How::Slap, f) => slap(f, dur, vel),
            (How::Pizz, Some(f)) => pizz(f, dur, vel, 1.0),
            (_, None) => {
                return Err(MusicError::new(H, "only a slap has no note"));
            }
        };
        let out: Arc<[f64]> = pp::to_48k(&x).iter().map(|v| v * g0).collect();
        notes.insert(key, Arc::clone(&out));
        Ok(out)
    }

    /// [`ContrabassSound::art`] as a mono sound to place.
    fn sig(&self, ctx: &MusicCtx, how: How, m: Option<i32>, dur: f64, vel: f64) -> Result<Sig, MusicError> {
        Ok(Sig::Mono(self.art(ctx, how, m, dur, vel)?.to_vec()))
    }

    /// `walk_note(t)`: its line's note at `t`, in the lower two octaves of its register.
    ///
    /// # Errors
    /// The line has a rest there (Python cannot move `None` by octaves), or as [`MusicCtx::line_at`].
    fn walk_note(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let name = self.name(ctx);
        let lo = self.register(ctx).0;
        let m = ctx.line_at(name, t)?.ok_or_else(|| {
            MusicError::new(
                H,
                format!("{name}'s line has a rest at {t} s, where its walking bass needs a note"),
            )
        })?;
        Ok(fit_reg(m, lo, lo + 24))
    }

    /// `root_of(t)`: the root of the chord at `t`, the lowest at or above a major third over its bottom note.
    fn root_of(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let lo = self.register(ctx).0;
        ctx.harmony.root_above(ctx.harmony.chord_at(t), lo + 4).map_err(bad)
    }

    /// `col(x, t, **kw)`: `self.colour(x, t, low=True, **kw)`: a bass line keeps its fundamental.
    fn col(&self, ctx: &MusicCtx, x: &[f64], t: f64, drive_k: f64) -> Result<(Vec<f64>, f64), MusicError> {
        let args = ColourArgs {
            low: true,
            ..self.colour_args()
        };
        let (sig, g, _lead) = self.colour(ctx, x, t, drive_k, &args)?;
        Ok((sig, g))
    }

    /// `elephant_notes(t0)`: the waltz tune from `t0` as `(time, midi note, seconds)`: on the key's tonic (the root
    /// of the first chord), its 3rd and 6th lowered a semitone where the key's scale has them so, in the bass's
    /// register.
    fn elephant_notes(&self, ctx: &MusicCtx, t0: f64) -> Result<Vec<(f64, i32, f64)>, MusicError> {
        let beat = ctx.show.grid.beat;
        let lo = self.register(ctx).0;
        let tonic = self.root_of(ctx, 0.0)?;
        let pc = i64::from(ctx.harmony.key_pc);
        let scale: Vec<i64> = ctx.harmony.scale.iter().map(|&s| imod_py(pc + i64::from(s), 12)).collect();
        let in_scale = |semi: i64| scale.contains(&imod_py(pc + semi, 12));
        let mut out = Vec::new();
        for (bt, semi, d) in ELEPHANT {
            let mut m = tonic + semi;
            let s12 = imod_py(i64::from(semi), 12);
            if s12 == 4 && !in_scale(4) {
                m -= 1;
            }
            if s12 == 9 && !in_scale(9) {
                m -= 1;
            }
            out.push((t0 + bt * beat, fit_reg(m, lo, lo + 26), d * beat));
        }
        Ok(out)
    }
}

impl Sound for ContrabassSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.6
    }

    fn send(&self) -> f64 {
        0.18
    }

    /// The walking line's fundamentals (the bed has no bass).
    fn band(&self) -> (f64, f64) {
        (40.0, 260.0)
    }

    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 30.0,
            cut_span: 60.0,
            drive: 2.5,
            ..ColourArgs::default()
        }
    }

    /// `voice(m, dur, vel=0.85)`: `self.art('pizz' if dur < 0.5 else 'arco', m, dur, vel)`.
    fn voice(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        if !m.is_finite() {
            return Err(MusicError::new(H, format!("a note's midi is {m}, not a number")));
        }
        let how = if dur < 0.5 { How::Pizz } else { How::Arco };
        self.art(ctx, how, Some(m.trunc() as i32), dur, vel)
    }

    /// A slapped bass hit (the config's TITLE_HITS: its notes, else the tonic) with two ghost slaps after it, and
    /// it keeps walking under the card's last beat.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        // hits = self.T.TITLE_HITS.get(self.name) or (self.root_of(0.0),)
        let hits = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(h) if !h.is_empty() => h.clone(),
            _ => vec![self.root_of(ctx, 0.0)?],
        };
        let h0 = Some(hits[0]);
        mix.place(&bus, &self.sig(ctx, How::Slap, h0, 0.5, 1.0)?, t, 0.62, pan);
        mix.place(&bus, &self.sig(ctx, How::Arco, h0, 0.8, 0.9)?, t, 0.30, pan);
        for (j, &m) in hits.iter().skip(1).enumerate() {
            let at = t + 0.08 + 0.07 * j as f64;
            mix.place(&bus, &self.sig(ctx, How::Pizz, Some(m), 0.12, 0.9)?, at, 0.32, pan);
        }
        for j in [1.0, 2.0] {
            mix.place(&bus, &self.sig(ctx, How::Slap, None, 0.08, 0.8)?, t + j * grid.s16, 0.22, pan);
        }
        if t + grid.beat < grid.bar - 0.05 {
            // it keeps walking under the card's last beat
            let r = hits[0];
            let d = grid.beat * 0.45;
            mix.place(&bus, &self.sig(ctx, How::Pizz, Some(r + 7), d, 0.9)?, t + grid.beat, 0.42, pan);
            mix.place(&bus, &self.sig(ctx, How::Pizz, Some(r + 11), d, 0.9)?, t + 1.5 * grid.beat, 0.42, pan);
        }
        Ok(())
    }

    /// It walks the count-in bar (the groove from bar 1): root, third, fifth, then a chromatic step to FIGHT!'s root.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let (lo, hi) = self.register(ctx);
        let ch = ctx
            .harmony
            .chords
            .get(1)
            .ok_or_else(|| MusicError::new(H, "the fight has no second bar of chords"))?;
        let r = self.root_of(ctx, grid.bar)?;
        let pcs = ctx.harmony.chord_pcs(ch).map_err(bad)?;
        if pcs.len() < 3 {
            return Err(MusicError::new(H, format!("the chord {ch} has fewer than three notes")));
        }
        let nxt = self.walk_note(ctx, grid.t_fight)?;
        let step = |a: i32| imod_py(i64::from(a - pcs[0]), 12) as i32;
        let notes = [
            r,
            r + step(pcs[1]),
            r + step(pcs[2]),
            nxt + if nxt <= r + 6 { 1 } else { -1 },
        ];
        for (j, m) in notes.into_iter().enumerate() {
            let x = self.sig(ctx, How::Pizz, Some(fit_reg(m, lo, hi)), grid.beat * 0.9, 0.95)?;
            mix.place(&bus, &x, grid.bar + j as f64 * grid.beat, 0.5, pan);
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (k, bus, pan) = (self.k, self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let lo = self.register(ctx).0;
        let mut th = Thinner::new(470.0 + k as f64);
        let thud = |mix: &mut Mix, dur: f64, f0: f64, f1: f64, seed: f64, tq: f64, g: f64| -> Played {
            let x = synth::thud(dur, f0, f1, noise_seed(H, seed)?);
            mix.place("fx", &Sig::Mono(x), tq, g, 0.0);
            Ok(())
        };
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
                "step_land" => {
                    // a footfall = a note of the walking line
                    let hp01 = ctx.hp_at(k, t) / 100.0;
                    if !th.keep(round_half_even(tq / s16), hp01) {
                        continue;
                    }
                    let on_beat = (tq / beat - round_half_even(tq / beat)).abs() < 1e-6;
                    let (m, dur) = if on_beat {
                        let hasted = e.f(6) != 0.0;
                        (self.walk_note(ctx, tq)?, beat * if hasted { 0.45 } else { 0.9 })
                    } else {
                        // DOUBLE TIME's off-8th: a chromatic approach
                        let nxt = self.walk_note(ctx, tq + beat / 2.0)?;
                        (if nxt > lo { nxt - 1 } else { nxt + 1 }, beat * 0.45)
                    };
                    let x = self.art(ctx, How::Pizz, Some(m), dur, 0.95)?;
                    let (sig, g) = self.col(ctx, &x, t, 1.0)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.56 * g, pan);
                    thud(mix, 0.18, 70.0, 40.0, t * 13.0, tq, 0.07)?;
                }
                "hit" if matches!(e.fields.get(5), Some(bf_core::event::Value::Str(s)) if s == "step") => {
                    // a landed quake: a ghost slap on top
                    mix.place(&bus, &self.sig(ctx, How::Slap, None, 0.08, 0.7)?, tq, 0.16, pan);
                }
                "slap" => {
                    // SLAP BASS: slap-a-slap
                    let m = self.walk_note(ctx, tq)?;
                    mix.place(&bus, &self.sig(ctx, How::Slap, Some(m), 0.3, 1.0)?, tq, 0.52, pan);
                    for j in [1.0, 2.0] {
                        let at = tq + j * beat / 3.0;
                        mix.place(&bus, &self.sig(ctx, How::Slap, None, 0.08, 0.85)?, at, 0.24, pan);
                    }
                    if e.f(3) != 0.0 {
                        // it broke a grab: an arco snarl under it
                        let x = self.art(ctx, How::Arco, Some(m), 0.4, 1.0)?;
                        let (sig, _) = self.col(ctx, &x, t, 2.0)?;
                        mix.place(&bus, &Sig::Mono(sig), tq, 0.30, pan);
                    }
                }
                "ability" if matches!(e.fields.get(2), Some(bf_core::event::Value::Str(s)) if s == "stampede") => {
                    // THE ELEPHANT: the waltz tune, bowed and heavy
                    for (ts, m, d) in self.elephant_notes(ctx, tq)? {
                        if ts > ctx.show.t_ko + 0.05 {
                            break;
                        }
                        let x = self.art(ctx, How::Arco, Some(m), d * 0.95, 1.0)?;
                        let (sig, g) = self.col(ctx, &x, ts, 1.5)?;
                        mix.place(&bus, &Sig::Mono(sig), ts, 0.50 * g, pan * 0.5);
                    }
                }
                "stampede_stomp" => {
                    thud(mix, 0.5, 90.0, 28.0, t * 11.0, tq, 0.26)?;
                    mix.place(&bus, &self.sig(ctx, How::Slap, None, 0.08, 0.7)?, tq, 0.14, pan);
                }
                "trample" => thud(mix, 0.7, 110.0, 30.0, t * 17.0, tq, 0.36)?,
                _ => {}
            }
        }
        Ok(())
    }

    /// The strings give way: a sagging low pizz and a rattle down the fingerboard.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let t0 = ctx.show.t_hs;
        let r = self.root_of(ctx, t0)?;
        let x = self.art(ctx, How::Arco, Some(r), 1.3, 1.0)?;
        mix.place(&bus, &Sig::Mono(bend(&x, -7.0, 0.1, 1.2)?), t0, 0.5, pan);
        for j in 0..6 {
            let j = f64::from(j);
            let x = self.sig(ctx, How::Slap, None, 0.06, 0.7 - 0.08 * j)?;
            mix.place(&bus, &x, t0 + 0.05 + 0.06 * j, 0.14, pan);
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let r = self.root_of(ctx, 0.0)?;
        mix.place("solo", &self.sig(ctx, How::Slap, Some(r + 12), 0.4, 1.0)?, tw, 0.5, 0.0);
        mix.place("solo", &self.sig(ctx, How::Arco, Some(r), 1.2, 1.0)?, tw, 0.42, 0.0);
        Ok(())
    }

    /// The fight file's solo: a walking-bass solo, plucked, slapped on the off-beat accents, the long notes bowed;
    /// the octave rings under each plucked or slapped note.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        let lo = self.register(ctx).0;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let how = if d >= 1.5 {
                How::Arco
            } else if fmod_py(bt * 2.0, 2.0) == 1.0 {
                How::Slap
            } else {
                How::Pizz
            };
            let x = self.sig(ctx, how, Some(m), py_max(0.1, d * grid.beat * 0.95), 0.95)?;
            mix.place("solo", &x, ts, 0.62, 0.0);
            if how != How::Arco {
                // the octave ringing under the plucked line
                let under = if m - 12 >= lo { m - 12 } else { m };
                let x = self.sig(ctx, How::Pizz, Some(under), py_max(0.1, d * grid.beat), 0.8)?;
                mix.place("solo", &x, ts, 0.26, 0.0);
            }
        }
        Ok(())
    }

    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let lo = self.register(ctx).0;
        let r = self.root_of(ctx, 0.0)?;
        let run = [r, r + 3, r + 7, r + 10, r + 12, r + 15, r + 19, r + 24];
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            let x = self.sig(ctx, How::Pizz, Some(fit_reg(m, lo, lo + 30)), 0.12, 0.85)?;
            mix.place("solo", &x, tj, 0.3, 0.0);
        }
        Ok(())
    }

    /// The walking line climbs into the title slam: four steps up to the tonic on frame 0.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let lo = self.register(ctx).0;
        let t_last = (grid.n_bars as f64 - 1.0) * grid.bar;
        let tgt = fit_reg(self.root_of(ctx, 0.0)? + 12, lo + 5, lo + 20);
        for bt in 0..4 {
            let m = if bt != 0 { tgt - 4 + bt } else { tgt - 5 };
            let x = self.sig(ctx, How::Pizz, Some(m), grid.beat * 0.9, 0.95)?;
            mix.place(&bus, &x, t_last + f64::from(bt) * grid.beat, 0.52, pan);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_is_moved_into_the_register_by_octaves() {
        assert_eq!(fit_reg(36, 28, 52), 36);
        assert_eq!(fit_reg(72, 28, 52), 48);
        assert_eq!(fit_reg(12, 28, 52), 36);
        assert_eq!(fit_reg(63, 30, 42), 39);
    }

    #[test]
    fn the_elephant_tune_is_nine_and_a_half_beats_in_order() {
        let mut at = 0.0;
        for (bt, _semi, d) in ELEPHANT {
            assert_eq!(bt, at, "each note starts where the last ended");
            at += d;
        }
        assert_eq!(at, 9.5);
    }
}
