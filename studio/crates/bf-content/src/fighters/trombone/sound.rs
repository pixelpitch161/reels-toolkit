//! The TROMBONE's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/trombone.py` (#41).
//!
//! The circus bass and the smears (role `bass`): every thrust is the bass line (the root on beats 1 and 3, the
//! fifth on 2 and 4) with a smear up into it, POWER SLIDE walks 8ths, a landed lance smears up the chord, SAD
//! TROMBONE is the clown's plunger wah-wah down to the tonic, PEDAL TONE the pedal tone under the hovering
//! bell. Opt-in (#48), role `riff`: over someone else's bass, every thrust is a punchy stab of its riff line
//! (the fight file's line for it), smeared up into it; the rest as before.
//!
//! The trombone's voice draws no noise, but its notes are still asked for in the kit's order: the first of them
//! measures the voice's level.

use bf_core::event::Event;
use bf_core::harmony::HarmonyError;
use bf_core::py::{imod_py, py_max, py_min, round_half_even, round_nd};
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    CLANK_DECAYS, CLANK_DUR, ColourArgs, MusicCtx, MusicError, Samples, Thinner, clank, event_int, event_is,
    event_is_str, event_number, gliss, noise_seed, vibrato, wah,
};
use bf_sound::synth::{self, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED, clip};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "TROMBONE";

/// `bone`'s default held length, in seconds.
pub const BONE_DUR: f64 = 0.16;
/// `bone`'s default velocity.
pub const BONE_VEL: f64 = 0.8;
/// `bone`'s default slide: the semitones it glides up into the pitch.
pub const BONE_GL: f64 = 2.0;
/// `bone`'s default slide time, in seconds.
pub const BONE_GL_T: f64 = 0.035;
/// `title_blast`'s notes when the fight file gives none: the low E and the E an octave up.
pub const TITLE_HITS: [i32; 2] = [40, 52];

/// The TROMBONE's part.
#[derive(Clone, Copy, Debug)]
pub struct TromboneSound {
    k: usize,
    /// Its role is `riff` (#48), read from the fight file once, when the part is made: the one source of that
    /// fact for the part (`band` has no context to read the role in; `play` and `solo` read this too).
    riff: bool,
}

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// What a fight file can change when one of its chords is too small for the TROMBONE's bass.
const CHORD_FIELDS: &str = "music.chords_intro or music.chord_loop";

/// `r + ((pcs[1] - pcs[0]) % 12)`: the third of `chord` (pitch classes `pcs`, root first) above its root `r`, for
/// the walking bass. A chord of fewer than two notes has no third: Python raises an `IndexError` there, this
/// says which chord it is. (Every chord the kit names has three notes or more; this guards a later chord type.)
fn third_above(chord: &str, pcs: &[i32], r: i32) -> Result<i32, MusicError> {
    match pcs {
        [root, second, ..] => Ok(r + (second - root).rem_euclid(12)),
        _ => Err(MusicError::new(
            H,
            format!(
                "the chord {chord:?} has {} note(s), and the walking bass needs a second one above the root: use a \
                 chord of three notes or more in {CHORD_FIELDS}",
                pcs.len()
            ),
        )),
    }
}

/// `climb[bt % len(climb)]`: beat `bt` of the climb up the last chord `chord` into the title. An empty climb (no
/// note of the chord within the octave from its root) has nothing to play: Python divides by zero there, this
/// says which chord it is. (The root is always a note of its chord, so no chord the kit names gives one.)
fn climb_note(chord: &str, climb: &[i32], bt: usize) -> Result<i32, MusicError> {
    if climb.is_empty() {
        return Err(MusicError::new(
            H,
            format!(
                "the last chord {chord:?} has no note within an octave of its root for the bass to climb into the \
                 title: change the chord that falls on the last bar in {CHORD_FIELDS}"
            ),
        ));
    }
    Ok(climb[bt % climb.len()])
}

/// `bone(m, dur, vel, gl, gl_t, cap)`: a trombone note gliding up `gl` semitones into the pitch (the slide)
/// (#34): `M.note('TROMBONE', m, round(dur, 3), cap or round(dur + 0.3, 2), round(vel, 1))`, through
/// `M.gliss(x, gl, gl_t)` unless `gl` is 0. Python's defaults are [`BONE_DUR`], [`BONE_VEL`], [`BONE_GL`],
/// [`BONE_GL_T`] and no `cap` (a `cap` of 0 counts as none, as in Python).
///
/// # Errors
/// As [`MusicCtx::note`] and [`gliss`].
pub fn bone(ctx: &MusicCtx, m: i32, dur: f64, vel: f64, gl: f64, gl_t: f64, cap: Option<f64>) -> Samples {
    let cap = match cap {
        Some(c) if c != 0.0 => c,
        _ => round_nd(dur + 0.3, 2),
    };
    let x = ctx.note("TROMBONE", f64::from(m), round_nd(dur, 3), cap, round_nd(vel, 1))?;
    if gl != 0.0 { gliss(&x, gl, gl_t) } else { Ok(x.to_vec()) }
}

impl TromboneSound {
    /// The part of the fighter on side `k` (0 is the left corner); `role` is its role in the fight file
    /// (`T.ROLES.get(name, 'melody')`), which sets its band and whether it plays the bass line or the riff.
    pub fn new(k: usize, role: &str) -> TromboneSound {
        TromboneSound {
            k,
            riff: role == "riff",
        }
    }

    /// `self.M.CFG[self.k]['register']`.
    fn register(&self, ctx: &MusicCtx) -> (i32, i32) {
        ctx.cfg[self.k].register
    }

    /// `root(chord)`: the chord's root in the trombone's register (the lowest one at or above its bottom).
    ///
    /// # Errors
    /// `chord` is not a chord name.
    pub fn root(&self, ctx: &MusicCtx, chord: &str) -> Result<i32, MusicError> {
        ctx.harmony.root_above(chord, self.register(ctx).0).map_err(bad)
    }

    /// `fifth(chord)`: the chord's fifth under its root, or above it when that would leave the register.
    ///
    /// # Errors
    /// `chord` is not a chord name.
    pub fn fifth(&self, ctx: &MusicCtx, chord: &str) -> Result<i32, MusicError> {
        let r = self.root(ctx, chord)?;
        Ok(if r - 5 >= self.register(ctx).0 { r - 5 } else { r + 7 })
    }

    /// `bass_note(t)`: the circus bass on the beat: the root on beats 1 and 3, the fifth on 2 and 4.
    ///
    /// # Errors
    /// `t` is not a time.
    pub fn bass_note(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let c = ctx.harmony.chord_at(t);
        let b = imod_py(slot(t / ctx.show.grid.beat)?, 4);
        if b == 0 || b == 2 {
            self.root(ctx, c)
        } else {
            self.fifth(ctx, c)
        }
    }

    /// `walk_note(t)`: POWER SLIDE walks 8ths: root, third, fifth, a chromatic approach into the next bar's root
    /// (8 per bar).
    ///
    /// # Errors
    /// `t` is not a time, or the chord there has no second note above its root.
    pub fn walk_note(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let grid = &ctx.show.grid;
        let (lo, hi) = self.register(ctx);
        let c = ctx.harmony.chord_at(t);
        let i = imod_py(slot(t / (grid.beat / 2.0))?, 8) as usize;
        let r = self.root(ctx, c)?;
        let nxt = if i >= 6 {
            self.root(ctx, ctx.harmony.chord_at(t + grid.bar * 0.51))?
        } else {
            r
        };
        let pcs = ctx.harmony.chord_pcs(c).map_err(bad)?;
        let third = third_above(c, pcs, r)?;
        let fifth = r + 7;
        let approach = if nxt + 2 <= hi { nxt + 2 } else { nxt - 1 };
        let table = [r, r, third, third, fifth, fifth, approach, nxt + 1];
        // max(lo, min(hi, table[i]))
        let note = if table[i] < hi { table[i] } else { hi };
        Ok(if note > lo { note } else { lo })
    }

    /// `self.colour(x, t, drive_k)`, or `self.colour(x, t, drive_k, low=True)` for a bass line.
    fn tone(
        &self,
        ctx: &MusicCtx,
        x: &[f64],
        t: f64,
        drive_k: f64,
        low: bool,
    ) -> Result<(Vec<f64>, f64, f64), MusicError> {
        let args = ColourArgs {
            low,
            ..self.colour_args()
        };
        self.colour(ctx, x, t, drive_k, &args)
    }

    /// The first loop of `play`: the bass line, a smear into every thrust (trombone.py 486-519).
    fn line(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let k = self.k;
        let (name, bus) = (self.name(ctx), self.bus(ctx));
        let s16 = ctx.show.grid.s16;
        let riff = self.riff;
        let mut thin = Thinner::new(7.0);
        let mut last_slot = None;
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if !(e.kind == "thrust" && event_is(H, e, 2, k)?) {
                continue;
            }
            let tq = ctx.q16(t);
            // slot = round(tq / T.S16): one note a 16th at most.
            let slot = slot(tq / s16)?;
            if last_slot == Some(slot) {
                continue;
            }
            last_slot = Some(slot);
            let hp = ctx.hp_at(k, t) / 100.0;
            if !thin.keep(slot as f64, hp) {
                continue;
            }
            let hasted = ctx.run_at("HA", t, k)? > 0.0;
            let on_beat = imod_py(slot, 4) == 0;
            let slide = if on_beat { 3.0 } else { 1.5 };
            if riff {
                // #48 (opt-in): a riff stab, smeared up into it.
                let m = ctx.line_at(name, tq)?.ok_or_else(|| {
                    MusicError::new(
                        H,
                        format!("{name}'s line has a rest at {tq} s, where a thrust needs a note"),
                    )
                })?;
                let dur = if hasted { 0.15 } else { 0.24 };
                let x = bone(ctx, m, dur, if on_beat { 0.9 } else { 0.75 }, slide, 0.05, None)?;
                let (sig, g, lead) = self.tone(ctx, &x, t, 0.8, false)?;
                mix.place(&bus, &Sig::Mono(sig), tq, (if on_beat { 0.42 } else { 0.34 }) * g, 0.15);
                if lead > 0.04 && on_beat {
                    // The leader: doubled an octave down, fuller.
                    let low = bone(ctx, m - 12, dur, 0.6, 2.0, 0.05, None)?;
                    let double = self.tone(ctx, &low, t, 1.0, true)?.0;
                    mix.place(&bus, &Sig::Mono(double), tq, 0.14 * g, 0.2);
                }
                continue;
            }
            let m = if hasted {
                self.walk_note(ctx, tq)?
            } else {
                self.bass_note(ctx, tq)?
            };
            let dur = if hasted { 0.17 } else { 0.30 };
            let x = bone(ctx, m, dur, if on_beat { 0.85 } else { 0.7 }, slide, 0.05, None)?;
            let (sig, g, lead) = self.tone(ctx, &x, t, 0.8, true)?;
            mix.place(&bus, &Sig::Mono(sig), tq, (if on_beat { 0.44 } else { 0.34 }) * g, 0.15);
            if lead > 0.04 && on_beat {
                // The leader: doubled an octave up, fuller.
                let high = bone(ctx, m + 12, dur, 0.6, 2.0, 0.05, None)?;
                let double = self.tone(ctx, &high, t, 1.0, false)?.0;
                mix.place(&bus, &Sig::Mono(double), tq, 0.14 * g, 0.2);
            }
        }
        Ok(())
    }

    /// The second loop of `play`: landed lances, the shockwave, the abilities and the stomp (trombone.py
    /// 520-557).
    fn events(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let beat = ctx.show.grid.beat;
        let cfg = ctx.cfg[k];
        let hit = |e: &Event, how: &str| -> Result<bool, MusicError> {
            Ok(e.kind == "hit" && event_is(H, e, 2, k)? && event_is_str(H, e, 7, how)?)
        };
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if hit(e, "blade")? {
                // A landed lance: a big smear up the chord.
                let tq = ctx.q16(t);
                let ct = ctx
                    .harmony
                    .chord_tones(52, 62, ctx.harmony.chord_at(tq + 0.01))
                    .map_err(bad)?;
                if ct.is_empty() {
                    return Err(MusicError::new(H, "the chord has no tone from 52 to 62"));
                }
                let m = ct[imod_py(event_int(H, e, 8)?, ct.len() as i64) as usize];
                let x = bone(ctx, m, 0.22, 1.0, 7.0, 0.11, None)?;
                let (sig, g, _) = self.tone(ctx, &x, t, 1.0, false)?;
                let pan = clip(event_number(H, e, 5)? / 3.5, -0.6, 0.6);
                mix.place(&bus, &Sig::Mono(sig), tq, 0.26 * g, pan);
                let thump = synth::thud(0.22, 150.0, 70.0, noise_seed(H, t * 7.0)?);
                mix.place("fx", &Sig::Mono(thump), tq, 0.08, 0.0);
            } else if hit(e, "shockwave")? {
                let thump = synth::thud(0.4, 120.0, 45.0, THUD_SEED);
                mix.place("fx", &Sig::Mono(thump), ctx.q16(t), 0.22, 0.0);
            } else if e.kind == "ability" && event_is(H, e, 2, k)? {
                let tq = ctx.q16(t);
                let is = |name: &str| event_is_str(H, e, 3, name);
                if is(&cfg.unlock1.name)? {
                    // SAD TROMBONE: wah wah wah waaah, down to the tonic.
                    for (j, m) in [55, 54, 53, 52].into_iter().enumerate() {
                        let d = if j < 3 { 0.5 * beat } else { 1.6 * beat };
                        let gl = if j != 0 { 1.0 } else { 0.0 };
                        let mut x = bone(ctx, m, d, 0.9, gl, 0.04, Some(round_nd(d + 0.35, 2)))?;
                        if j == 3 {
                            x = vibrato(&x, 0.5, 5.0, 0.2)?;
                        }
                        let x = wah(&x, &[(0.0, py_min(0.18, d * 0.6))])?;
                        let sig = self.tone(ctx, &x, t, 1.0, false)?.0;
                        mix.place(&bus, &Sig::Mono(sig), tq + j as f64 * 0.5 * beat, 0.42, 0.1);
                    }
                    let fall = synth::whoosh(0.6, 200.0, 1500.0, false, WHOOSH_SEED);
                    mix.place("fx", &Sig::Mono(fall), tq, 0.08, 0.0);
                } else if is(&cfg.unlock2.name)? {
                    // POWER SLIDE: a long rip up two octaves into the walk.
                    let x = bone(ctx, 64, 0.45, 1.0, 24.0, 0.32, None)?;
                    let sig = self.tone(ctx, &x, t, 1.0, false)?.0;
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.30, 0.0);
                } else if is(&cfg.low.name)? {
                    // PEDAL TONE: the pedal E under the hovering bell.
                    let cap = Some(round_nd(2.0 * beat + 0.3, 2));
                    let pedal = bone(ctx, 28, 2.0 * beat, 1.0, 2.0, 0.1, cap)?;
                    mix.place(&bus, &Sig::Mono(pedal), tq, 0.40, 0.0);
                    let octave = bone(ctx, 40, 2.0 * beat, 0.9, 0.0, BONE_GL_T, cap)?;
                    mix.place(&bus, &Sig::Mono(octave), tq, 0.30, 0.0);
                    let air = synth::whoosh(2.0 * beat, 150.0, 1800.0, WHOOSH_RISE, WHOOSH_SEED);
                    mix.place("fx", &Sig::Mono(air), tq, 0.10, 0.0);
                }
            } else if e.kind == "slam" && event_is(H, e, 2, k)? {
                // PEDAL TONE: the bell stomps.
                let ts = e.t;
                bed.boom(ctx, mix, ts, 0.8)?;
                mix.place("fx", &Sig::Mono(synth::thud(0.8, 90.0, 30.0, THUD_SEED)), ts, 0.5, 0.0);
                mix.place(
                    "fx",
                    &Sig::Mono(clank(64.0, 9, CLANK_DUR, &CLANK_DECAYS)?),
                    ts,
                    0.10,
                    0.0,
                );
                let x = bone(ctx, 40, 0.5, 1.0, 0.0, BONE_GL_T, None)?;
                mix.place(&bus, &Sig::Mono(x), ts, 0.36, 0.0);
                mix.place("drums", &Sig::Mono(synth::crash(1.6)), ts, 0.10, 0.0);
            }
        }
        Ok(())
    }
}

/// Python's `int(round(x))` for a count of beats or 16ths.
fn slot(x: f64) -> Result<i64, MusicError> {
    let r = round_half_even(x);
    if r.abs() < 9.0e15 {
        Ok(r as i64)
    } else {
        Err(MusicError::new(H, format!("{x} is not a place on the grid")))
    }
}

impl Sound for TromboneSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.8
    }

    fn send(&self) -> f64 {
        0.2
    }

    /// The bass line E2 to C3 with its brass harmonics and smears; as a riff (#48), the stabs E3 to E4 with
    /// theirs.
    fn band(&self) -> (f64, f64) {
        if self.riff { (120.0, 1500.0) } else { (70.0, 700.0) }
    }

    /// The low note and a big smear up.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let hits = ctx
            .ff
            .music
            .title_hits
            .get(self.name(ctx))
            .map_or(&TITLE_HITS[..], Vec::as_slice);
        let (Some(&low), Some(&high)) = (hits.first(), hits.last()) else {
            return Err(MusicError::new(H, "the title hits are an empty list"));
        };
        let x = bone(ctx, low, 0.7, 1.0, 0.0, BONE_GL_T, Some(1.0))?;
        mix.place(&bus, &Sig::Mono(x), t, 0.42, 0.0);
        let x = bone(ctx, high, 0.6, 1.0, 12.0, 0.26, Some(0.95))?;
        mix.place(&bus, &Sig::Mono(x), t, 0.34, 0.0);
        Ok(())
    }

    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let ch = ctx
            .harmony
            .chords
            .get(1)
            .ok_or_else(|| MusicError::new(H, "the episode has no bar 1 to count in on"))?;
        let notes = [self.root(ctx, ch)?, self.fifth(ctx, ch)?, self.root(ctx, ch)?];
        for (j, (tc, _)) in grid.count[..3].iter().enumerate() {
            let x = bone(ctx, notes[j], 0.25, 0.95, 3.0, 0.05, None)?;
            mix.place(&bus, &Sig::Mono(x), *tc, 0.40, 0.0);
        }
        // The pickup smear.
        let x = bone(ctx, self.root(ctx, ch)?, 0.4, 1.0, 7.0, 0.3, None)?;
        mix.place(&bus, &Sig::Mono(x), grid.bar + 3.0 * grid.beat, 0.36, 0.0);
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        self.line(ctx, mix)?;
        self.events(ctx, bed, mix)
    }

    /// The slide falls apart: a falling wail, clattering tubing.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t_hs = ctx.show.t_hs;
        let x = bone(ctx, 52, 0.9, 1.0, 0.0, BONE_GL_T, Some(1.3))?;
        mix.place(&self.bus(ctx), &Sig::Mono(gliss(&x, -10.0, 0.9)?), t_hs, 0.26, 0.0);
        for (j, m) in [80.0, 75.0, 85.0, 71.0].into_iter().enumerate() {
            let tube = clank(m, 30 + j as u64, CLANK_DUR, &CLANK_DECAYS)?;
            mix.place(
                "fx",
                &Sig::Mono(tube),
                t_hs + 0.1 * j as f64,
                0.08,
                (j as f64 - 1.5) / 4.0,
            );
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let x = bone(ctx, 40, 1.0, 1.0, 0.0, BONE_GL_T, Some(1.5))?;
        mix.place("solo", &Sig::Mono(x), tw, 0.40, 0.0);
        for (m, g) in [(52, 0.30), (55, 0.24), (59, 0.20)] {
            let x = bone(ctx, m, 1.0, 1.0, 5.0, 0.12, Some(1.5))?;
            mix.place("solo", &Sig::Mono(x), tw, g, 0.0);
        }
        Ok(())
    }

    /// The solo on the slide (each note glides from the last), over its own bass line on the beats that follow
    /// the solo, up to the last bar.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        let name = self.name(ctx);
        let riff = self.riff;
        let notes = ctx.solo(name)?;
        // t_end = t0 + max(bt + d for bt, m, d in notes) * T.BEAT
        let last = notes
            .iter()
            .map(|&(bt, _, d)| bt + d)
            .reduce(py_max)
            .ok_or_else(|| MusicError::new(H, format!("{name}'s solo has no notes")))?;
        let t_end = t0 + last * beat;
        // range(int(math.ceil(t_end / T.BEAT)), int((T.N_BARS - 1) * T.BAR / T.BEAT + 0.5))
        let from = slot((t_end / beat).ceil())?;
        let to = slot(((grid.n_bars as f64 - 1.0) * bar / beat + 0.5).trunc())?;
        for j in from..to {
            let tb = j as f64 * beat;
            // #48: back to its riff.
            let m = if riff {
                ctx.line_at(name, tb)?
                    .ok_or_else(|| MusicError::new(H, format!("{name}'s line has a rest at {tb} s")))?
            } else {
                self.bass_note(ctx, tb)?
            };
            let x = bone(ctx, m, 0.3, 0.85, 3.0, 0.05, None)?;
            mix.place("solo", &Sig::Mono(x), tb, 0.40, 0.0);
        }
        let mut prev: Option<i32> = None;
        for (bt, m, d) in notes {
            let ts = t0 + bt * beat;
            // A late K.O. leaves less room: stop at the reassembly.
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let gl = match prev {
                Some(p) => clip(f64::from(m - p), -9.0, 9.0),
                None => 5.0,
            };
            let cap = Some(round_nd(d * beat + 0.35, 2));
            let x = bone(ctx, m, d * beat * 0.95, 0.95, gl, 0.08, cap)?;
            mix.place("solo", &Sig::Mono(x), ts, 0.50, 0.0);
            prev = Some(m);
        }
        Ok(())
    }

    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        for (j, m) in [40, 45, 47, 52, 55, 59].into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / 6.0);
            let x = bone(ctx, m, 0.12, 0.7, BONE_GL, BONE_GL_T, None)?;
            mix.place(&bus, &Sig::Mono(x), tj, 0.20, 0.0);
        }
        Ok(())
    }

    /// The bass climbs the last chord (the dominant) into the title slam.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        if self.role(ctx) != "bass" {
            return Ok(());
        }
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let t_last = (grid.n_bars as f64 - 1.0) * grid.bar;
        let ch = ctx
            .harmony
            .chords
            .last()
            .ok_or_else(|| MusicError::new(H, "the episode has no bars"))?;
        let r = self.root(ctx, ch)?;
        let mut climb = ctx.harmony.chord_tones(r, r + 11, ch).map_err(bad)?;
        climb.truncate(4);
        for bt in 0..4usize {
            let x = bone(ctx, climb_note(ch, &climb, bt)?, 0.35, 0.9, 3.0, 0.06, None)?;
            mix.place(&bus, &Sig::Mono(x), t_last + bt as f64 * grid.beat, 0.42, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chord_without_a_third_is_an_error() {
        // E minor: E G B, the third is three semitones up.
        assert_eq!(third_above("Em", &[4, 7, 11], 40), Ok(43));
        // Python: `pcs[1]` raises an IndexError.
        assert_eq!(
            third_above("E1", &[4], 40).unwrap_err().to_string(),
            "TROMBONE: the chord \"E1\" has 1 note(s), and the walking bass needs a second one above the root: use a \
             chord of three notes or more in music.chords_intro or music.chord_loop"
        );
        assert!(third_above("", &[], 40).is_err());
    }

    #[test]
    fn an_empty_climb_is_an_error() {
        let climb = [47, 51, 54, 57];
        let got: Vec<i32> = (0..4).map(|bt| climb_note("B7", &climb, bt).unwrap()).collect();
        assert_eq!(got, climb);
        // Fewer than four notes go round again, as `bt % len(climb)` does.
        assert_eq!(climb_note("B7", &[47, 51], 3), Ok(51));
        // Python: `bt % len(climb)` divides by zero.
        assert_eq!(
            climb_note("B7", &[], 0).unwrap_err().to_string(),
            "TROMBONE: the last chord \"B7\" has no note within an octave of its root for the bass to climb into the \
             title: change the chord that falls on the last bar in music.chords_intro or music.chord_loop"
        );
    }
}
