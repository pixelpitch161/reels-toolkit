//! `projects/ballfight/kit/music.py`'s helpers, with the same names: the context a soundtrack is built in, the
//! fighters' cached, level-matched notes, the shaping of a note (slides, bends, vibrato, the plunger "wah", the
//! tone that thins and dirties as a fighter's HP drops), the note-thinning hashes, and the shared sound effects.
//!
//! Python's module-level state (the run, the timeline, the two fighters, the note cache `_VC`, the gains `_GAIN`,
//! the series' noise generator) is one [`MusicCtx`] here, made once per soundtrack build. What needs none of it
//! is a free function.
//!
//! **Notes are made one at a time, in the order they are first asked for.** The Pixel Pitch voices share one
//! noise stream ([`crate::voice::PpNoise`]), so the order of first-time notes is part of the sound.
//! [`MusicCtx::note`] holds the stream and the cache behind one lock, synthesises a note the first time its key
//! is asked for and hands out the same samples afterwards without drawing anything.
//!
//! **Defaults.** Rust has no default arguments: every function takes all of Python's arguments in Python's
//! order, and each default is a constant named `<FUNCTION>_<ARGUMENT>` (`NOTE_CAP`, `RISER_OVER`, ...).
//! `colour`'s keyword arguments are [`ColourArgs`], whose `Default` holds Python's.
//!
//! **Errors.** A helper that makes or shapes samples returns `Err` exactly where Python raises (an empty note
//! handed to a filter, a band of a sweep with no sample of its own, a cutoff outside the filter's range, a length
//! that is not a number), instead of reaching the panics of [`crate::dsp`]. One bound is this module's own: a
//! length over [`MAX_SECONDS`] is refused (Python would try to allocate it). [`MusicCtx::note`] refuses a few
//! values more, listed there. The number helpers (`h01`, `Thinner`, ...) panic where Python raises, as the rest
//! of the crate does. If a voice panics while a note is being made, the build's state is not handed out again:
//! every later call on that context is an error.
//!
//! Each sample is computed with the operations of the numpy expression it ports, in the same order; `sin`, `exp`,
//! `pow` and `tanh` come from `dmath`.
//!
//! The module is in parts: the context is here; [`MusicError`] in `error`; `q16` and the thinning hashes in
//! `thin`; `fit`, the warps, `wah` and `colour` in `shape`; the sound effects in `fx`; `shared_events` and the
//! reading of the run's events in `events`; the timeline's lines and solos and the run's per-frame values
//! (`line_at`, `solo`, `run_at`) in `timeline`; Python's number rules that they share in `num`. Everything public
//! is re-exported here.

mod error;
mod events;
mod fx;
mod num;
mod shape;
#[cfg(test)]
pub(crate) mod testkit;
#[cfg(test)]
mod tests;
mod thin;
mod timeline;

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};

use bf_core::fightfile::{FightFile, INTRO_BARS};
use bf_core::harmony::{self, Harmony};
use bf_core::py::{ifloor_div, round_half_even, round_nd};
use bf_core::recording::{Array, Recording};
use bf_core::roster::FighterCfg;
use bf_core::show::Show;

pub use self::error::{MusicError, Samples};
pub use self::events::{event_at, event_int, event_is, event_is_str, event_number, noise_seed, shared_events};
pub use self::fx::{
    CLANK_DECAYS, CLANK_DUR, CLANK_SEED, HISS_DEC, HISS_DUR, HISS_HI, HISS_LO, HISS_SEED, RISER_OVER, RISER_SEED,
    ROAR_SEED, THUNDER_CRACK, THUNDER_DUR, THUNDER_SEED, clank, hiss, riser, roar, thunder,
};
pub use self::num::MAX_SECONDS;
use self::num::int;
use self::shape::no_room;
pub use self::shape::{
    COLOUR_CUT_SPAN, COLOUR_CUT0, COLOUR_DRIVE, COLOUR_DRIVE_K, COLOUR_LOW, COLOUR_LOW_CUT, ColourArgs, FIT_FADE,
    VIBRATO_DELAY, VIBRATO_DEPTH, VIBRATO_RATE, bend, colour_hp, fit, gliss, vibrato, wah,
};
pub use self::thin::{Thinner, h01, h01_37, q16, thin_37};
use crate::SRF;
use crate::dsp;
use crate::synth::{self, FILTER_O};
use crate::voice::{PpNoise, Voice};
use bf_core::py::{py_max, py_min};

/// `note`'s default held length, in seconds.
pub const NOTE_DUR: f64 = 0.18;
/// `note`'s default `cap`: the length the note is cut or padded to, in seconds.
pub const NOTE_CAP: f64 = 0.6;
/// `note`'s default velocity.
pub const NOTE_VEL: f64 = 0.85;

/// One first-time note: the arguments `note` was called with when it synthesised it.
#[derive(Clone, Debug, PartialEq)]
pub struct NoteCall {
    /// The voice's roster name.
    pub name: String,
    /// The MIDI note, `int(midi)`.
    pub midi: i64,
    /// The held length, as passed.
    pub dur: f64,
    /// The length the note was cut or padded to, as passed.
    pub cap: f64,
    /// The velocity, as passed.
    pub vel: f64,
}

/// One window registered with [`MusicCtx::line_override`]: from `t0` to `t1` the fighter's line plays `notes`
/// instead, `(beat from tz, midi note or a rest, beats)`.
#[derive(Clone, Debug, PartialEq)]
pub struct LineOverride {
    pub t0: f64,
    pub t1: f64,
    pub tz: f64,
    pub notes: Vec<(f64, Option<i32>, f64)>,
}

/// The key `note` caches by: `(name, int(midi), round(dur, 3), cap, round(vel, 2))`, the floats by their bits
/// (`-0.0` as `0.0`, which Python's dict takes for the same key).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct NoteKey {
    name: String,
    midi: i64,
    dur: u64,
    cap: u64,
    vel: u64,
}

fn key_bits(x: f64) -> u64 {
    if x == 0.0 { 0 } else { x.to_bits() }
}

/// What a soundtrack build changes as it goes: Python's `drums60_lowpoly._rng`, `_VC`, `_GAIN` and
/// `T.LINE_OVERRIDE`.
struct State {
    noise: PpNoise,
    notes: BTreeMap<NoteKey, Arc<[f64]>>,
    gains: BTreeMap<String, f64>,
    first: Vec<NoteCall>,
    line_overrides: BTreeMap<String, Vec<LineOverride>>,
}

impl State {
    fn new(noise: PpNoise) -> State {
        State {
            noise,
            notes: BTreeMap::new(),
            gains: BTreeMap::new(),
            first: Vec::new(),
            line_overrides: BTreeMap::new(),
        }
    }
}

/// What `kit/music.py` knows while it builds a soundtrack: the show (`T`, `T_KO`, `WIN`, ...), the recording
/// (`RUN`, `EV`), the fight file, the harmony, the two fighters' roster entries (`CFG`) and voices, and, behind
/// one lock, the note cache, the instruments' gains and the Pixel Pitch noise stream.
///
/// Make one per soundtrack build. It can be shared between threads, but the notes of a build must be asked for
/// from one thread, in the score's order: the order of first-time notes is part of the sound.
pub struct MusicCtx<'a> {
    /// The show: the grid (`T`), `NAMES`, `WIN`, `LOSE`, `T_KO`, `T_HS`, `T_SOLO`, the captions.
    pub show: &'a Show,
    /// The recording: `RUN` and its events (`EV`).
    pub rec: &'a Recording,
    /// The fight file: the music settings (roles, lines, solos, title hits).
    pub ff: &'a FightFile,
    /// The timeline's harmony: the key, the scale, one chord per bar, the loop.
    pub harmony: Harmony,
    /// `CFG`: the two fighters' roster entries, left corner first.
    pub cfg: [&'a FighterCfg; 2],
    /// The two fighters' voices, in the same order. `note` finds a voice by its fighter's `pp` name.
    pub voices: [Arc<dyn Voice>; 2],
    /// `RUN['HP']`: both fighters' HP at every recorded frame, frame after frame.
    hp: &'a [f64],
    state: Mutex<State>,
}

impl fmt::Debug for MusicCtx<'_> {
    /// The fighters and their voices. What sits behind the lock is not printed: formatting a context must not
    /// wait for a note that is being made.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MusicCtx")
            .field("names", &self.show.names)
            .field("voices", &[self.voices[0].name(), self.voices[1].name()])
            .finish_non_exhaustive()
    }
}

/// `reelkit.config.ease(x)`: the quintic ease-in-out on [0, 1].
fn ease(x: f64) -> f64 {
    let x = py_min(1.0, py_max(0.0, x));
    x * x * x * (x * (6.0 * x - 15.0) + 10.0)
}

impl<'a> MusicCtx<'a> {
    /// The context of one soundtrack build: the harmony of the fight file (its key and scale, the intro chords,
    /// then the chord loop to the last bar), the noise stream at its start, nothing cached. `cfg` and `voices`
    /// are the two fighters' roster entries and voices, left corner first.
    ///
    /// # Errors
    /// The fight file's key, scale or chords do not parse; the recording has no `HP` array of two fighters with
    /// at least one frame; or `voices[k]` is not the voice fighter `k`'s roster entry names (its `pp`): swapped
    /// voices would play each part on the other's instrument.
    pub fn new(
        show: &'a Show,
        rec: &'a Recording,
        ff: &'a FightFile,
        cfg: [&'a FighterCfg; 2],
        voices: [Arc<dyn Voice>; 2],
    ) -> Result<MusicCtx<'a>, MusicError> {
        const H: &str = "music";
        let m = &ff.music;
        let bad = |e: harmony::HarmonyError| MusicError::new(H, e.to_string());
        let chords = harmony::chords_for(&m.chords_intro, &m.chord_loop, show.grid.n_bars);
        let harmony = Harmony::new(
            &show.grid,
            &m.key,
            harmony::scale(&m.scale).map_err(bad)?,
            chords,
            Some(m.chord_loop.clone()),
            INTRO_BARS,
        )
        .map_err(bad)?;
        let hp = match rec.arrays.get("HP") {
            Some(Array::F64 { shape, data }) if shape.len() == 2 && shape[1] == 2 && !data.is_empty() => {
                if data.len() != shape[0] * 2 {
                    return Err(MusicError::new(
                        H,
                        "the recording's HP does not have the length it states",
                    ));
                }
                data.as_slice()
            }
            _ => {
                return Err(MusicError::new(
                    H,
                    "the recording has no HP of two fighters at every frame",
                ));
            }
        };
        for (k, (voice, cfg)) in voices.iter().zip(cfg).enumerate() {
            if voice.name() != cfg.pp {
                return Err(MusicError::new(
                    H,
                    format!("voice {k} is {:?}, but {} plays {:?}", voice.name(), cfg.name, cfg.pp),
                ));
            }
        }
        Ok(MusicCtx {
            show,
            rec,
            ff,
            harmony,
            cfg,
            voices,
            hp,
            state: Mutex::new(State::new(PpNoise::start())),
        })
    }

    /// The lock on the build's state. If a voice panicked while a note was being made, the noise stream and the
    /// cache may be half-advanced, and later notes would be silently wrong: the state is not handed out again.
    ///
    /// # Errors
    /// The lock is poisoned.
    fn lock(&self) -> Result<MutexGuard<'_, State>, MusicError> {
        self.state.lock().map_err(|_| {
            MusicError::new(
                "music",
                "a voice panicked while a note was being made: the noise stream and the notes of this build cannot \
                 be trusted any more",
            )
        })
    }

    /// Forgets every cached note, every gain and the log of first-time notes, and puts the noise stream at
    /// `noise`: a new build in the same context (line overrides stay).
    ///
    /// # Errors
    /// A voice panicked earlier (as every method that reads or changes the build's state).
    pub fn restart(&self, noise: PpNoise) -> Result<(), MusicError> {
        let mut st = self.lock()?;
        let line_overrides = std::mem::take(&mut st.line_overrides);
        *st = State::new(noise);
        st.line_overrides = line_overrides;
        Ok(())
    }

    /// A copy of the noise stream where it stands now.
    pub fn noise(&self) -> Result<PpNoise, MusicError> {
        Ok(self.lock()?.noise.clone())
    }

    /// Every first-time note so far, in the order `note` synthesised them.
    pub fn first_notes(&self) -> Result<Vec<NoteCall>, MusicError> {
        Ok(self.lock()?.first.clone())
    }

    /// `q16(t)`: the nearest 16th of the episode's grid, see [`q16`].
    pub fn q16(&self, t: f64) -> f64 {
        q16(t, self.show.grid.s16)
    }

    /// The number of recorded frames, `NSIM`.
    fn nsim(&self) -> usize {
        self.hp.len() / 2
    }

    /// `P.sim_i(t)`: the recorded frame at `t`, `round((t - T_FIGHT) * FPS)` kept inside the recording. A `t`
    /// that is not a number gives frame 0 (Python raises).
    pub fn sim_i(&self, t: f64) -> usize {
        let grid = &self.show.grid;
        let frame = round_half_even((t - grid.t_fight) * f64::from(grid.fps));
        if frame > 0.0 {
            (frame as usize).min(self.nsim() - 1)
        } else {
            0
        }
    }

    /// `hp_at(k, t)`: fighter `k`'s HP at `t`, 0 to 100: 100 before FIGHT!, the recorded HP up to the K.O.,
    /// then the HP it ended on (0 for the loser), refilled over the glide to the corners at the end.
    ///
    /// # Panics
    /// When `k` is not 0 or 1.
    pub fn hp_at(&self, k: usize, t: f64) -> f64 {
        assert!(k < 2, "hp_at: fighter {k} (there are two, 0 and 1)");
        let grid = &self.show.grid;
        if t < grid.t_fight {
            return 100.0;
        }
        if t < self.show.t_ko {
            return self.hp[self.sim_i(t) * 2 + k];
        }
        let end = if k == self.show.win {
            self.hp[(self.nsim() - 1) * 2 + k]
        } else {
            0.0
        };
        end + (100.0 - end) * ease((t - grid.t_glide) / (grid.t_rev - grid.t_glide))
    }

    /// The fighter whose voice has the roster name `name` (`CFG[k]['pp']`).
    fn voice_of(&self, helper: &'static str, name: &str) -> Result<usize, MusicError> {
        (0..2).find(|&k| self.cfg[k].pp == name).ok_or_else(|| {
            MusicError::new(
                helper,
                format!(
                    "{name:?} is not the voice of a fighter of this fight ({:?} and {:?} are)",
                    self.cfg[0].pp, self.cfg[1].pp
                ),
            )
        })
    }

    /// `inst_gain(name)`: the factor that brings the voice `name` to the series' level, measured once: the note
    /// in the middle of the voice's register (`(lo + hi) // 2`, velocity 0.8, 0.2 s), its first 0.3 s high-passed
    /// at 120 Hz, `min(4.0, 0.16 / (rms + 1e-6))`. Measuring it synthesises that note, which draws from the
    /// noise stream if the voice has noise: `note` therefore measures it AFTER the first note's own voice call,
    /// as Python does, and calling this first gives a different soundtrack.
    ///
    /// # Errors
    /// `name` is not the voice of a fighter of this fight, or the voice gives no samples, or samples that are
    /// not numbers, for that note.
    pub fn inst_gain(&self, name: &str) -> Result<f64, MusicError> {
        self.gain(&mut *self.lock()?, "inst_gain", name)
    }

    fn gain(&self, st: &mut State, helper: &'static str, name: &str) -> Result<f64, MusicError> {
        if let Some(&g) = st.gains.get(name) {
            return Ok(g);
        }
        let k = self.voice_of(helper, name)?;
        let (lo, hi) = self.voices[k].register();
        let middle = ifloor_div(i64::from(lo) + i64::from(hi), 2) as f64;
        let x = self.voices[k].note(&mut st.noise, middle, 0.8, 0.2);
        let x = &x[..x.len().min((0.3 * SRF) as usize)];
        if x.is_empty() || !x.iter().all(|v| v.is_finite()) {
            return Err(MusicError::new(
                helper,
                format!("{name} gives no sound for MIDI note {middle}, the middle of its register"),
            ));
        }
        let squares: Vec<f64> = synth::hp(x, 120.0, FILTER_O).iter().map(|v| v * v).collect();
        let g = py_min(4.0, 0.16 / (dsp::mean(&squares).sqrt() + 1e-6));
        st.gains.insert(name.to_string(), g);
        Ok(g)
    }

    /// `note(name, midi, dur, cap, vel)`: one note in the Pixel Pitch voice `name` (a fighter's `pp` roster
    /// name), level-matched across instruments: `fit(voice(name, int(midi), vel, dur), cap) * inst_gain(name)`,
    /// `int(cap * SR)` samples long.
    ///
    /// A note is synthesised once and cached by `(name, int(midi), round(dur, 3), cap, round(vel, 2))`; later
    /// calls with the same key get the same samples and draw no noise. So two calls whose `dur` or `vel` differ
    /// only past the rounding share the first one's note, as in the kit. The first note of a voice also measures
    /// the voice's gain, after its own voice call (see [`MusicCtx::inst_gain`]).
    ///
    /// A `dur` a little below zero is a note that is its release alone, and a `cap` below zero cuts that much
    /// off the end of the note instead of setting its length (Python's slice), both as in the kit.
    ///
    /// # Errors
    /// Checked before anything is synthesised: an argument that is not a number (Python raises for `midi`,
    /// `dur` and `cap`, and plays NaN for `vel`); a `dur` the voice cannot hold (so far below zero that the note
    /// has a negative number of samples: Python raises) or over [`MAX_SECONDS`]; a `cap` of 1 or 2 samples
    /// (Python raises: no room for the fade); a `name` that is not the voice of a fighter of this fight (Python
    /// has every instrument of the series). And after the voice call: samples that are not numbers, which is
    /// what a MIDI note far outside what the voice can make gives (Python plays the NaN); and a negative `cap`
    /// that leaves 1 or 2 samples of the note (Python raises there too, after the voice has drawn its noise).
    pub fn note(&self, name: &str, midi: f64, dur: f64, cap: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        const H: &str = "note";
        for (what, v) in [("midi", midi), ("dur", dur), ("cap", cap), ("vel", vel)] {
            if !v.is_finite() {
                return Err(MusicError::new(H, format!("{what} is {v}, not a number")));
            }
        }
        if dur > MAX_SECONDS {
            return Err(MusicError::new(
                H,
                format!("a held length (dur) of {dur} s is longer than {MAX_SECONDS} s"),
            ));
        }
        let midi = midi.trunc();
        let key = NoteKey {
            name: name.to_string(),
            midi: midi as i64,
            dur: key_bits(round_nd(dur, 3)),
            cap: key_bits(cap),
            vel: key_bits(round_nd(vel, 2)),
        };
        let mut st = self.lock()?;
        if let Some(x) = st.notes.get(&key) {
            return Ok(Arc::clone(x));
        }
        let k = self.voice_of(H, name)?;
        // What fit refuses for a cap from zero up depends on the cap alone (the note is then `int(cap * SR)`
        // samples long whatever the voice gives): find out before the voice draws any noise.
        let n = int(H, "cap", cap * SRF)?;
        if n == 1 || n == 2 {
            return Err(MusicError::new(H, no_room(n as usize, FIT_FADE)));
        }
        if !self.voices[k].can_hold(dur) {
            return Err(MusicError::new(
                H,
                format!("{name} has no note for a held length (dur) of {dur} s: it would end before it starts"),
            ));
        }
        let x = self.voices[k].note(&mut st.noise, midi, vel, dur);
        if !x.iter().all(|v| v.is_finite()) {
            return Err(MusicError::new(
                H,
                format!(
                    "{name} gives samples that are not numbers for MIDI note {midi} (velocity {vel}, {dur} s): \
                     the note is outside what the voice can make"
                ),
            ));
        }
        let fitted = fit(&x, cap, FIT_FADE).map_err(|e| MusicError::new(H, e.problem))?;
        let gain = self.gain(&mut st, H, name)?;
        let out: Arc<[f64]> = fitted.iter().map(|v| v * gain).collect();
        st.notes.insert(key, Arc::clone(&out));
        st.first.push(NoteCall {
            name: name.to_string(),
            midi: midi as i64,
            dur,
            cap,
            vel,
        });
        Ok(out)
    }

    /// `colour(x, k, t, drive_k, ...)`: fighter `k`'s tone at time `t`: thinner (high-passed) and dirtier
    /// (driven) as its HP drops; returns `(signal, gain, lead)`: the leader plays up to +3.5 dB, the loser down
    /// to -4 dB. A bass line (`args.low`) keeps its fundamental. See [`colour_hp`].
    ///
    /// # Errors
    /// As [`colour_hp`], and when `k` is not 0 or 1.
    pub fn colour(
        &self,
        x: &[f64],
        k: usize,
        t: f64,
        drive_k: f64,
        args: &ColourArgs,
    ) -> Result<(Vec<f64>, f64, f64), MusicError> {
        if k > 1 {
            return Err(MusicError::new(
                "colour",
                format!("fighter {k} (there are two, 0 and 1)"),
            ));
        }
        colour_hp(x, self.hp_at(k, t), self.hp_at(1 - k, t), drive_k, args)
    }

    /// `line_override(k, t0, t1, tz, notes)` (#53, opt-in): from `t0` to `t1`, fighter `k`'s line plays `notes`
    /// instead: `(beat from tz, midi note or a rest, beats)`. A fighter registers it in its `prepare`;
    /// [`MusicCtx::line_at`] (`T.line_at`) then reads the windows before the line itself.
    ///
    /// # Errors
    /// `k` is not 0 or 1, or a voice panicked earlier.
    pub fn line_override(
        &self,
        k: usize,
        t0: f64,
        t1: f64,
        tz: f64,
        notes: &[(f64, Option<i32>, f64)],
    ) -> Result<(), MusicError> {
        let Some(name) = self.show.names.get(k) else {
            return Err(MusicError::new(
                "line_override",
                format!("fighter {k} (there are two, 0 and 1)"),
            ));
        };
        self.lock()?
            .line_overrides
            .entry(name.clone())
            .or_default()
            .push(LineOverride {
                t0,
                t1,
                tz,
                notes: notes.to_vec(),
            });
        Ok(())
    }

    /// `T.LINE_OVERRIDE.get(name, ())`: the windows registered for the fighter `name`, in the order they were
    /// registered.
    ///
    /// # Errors
    /// A voice panicked earlier.
    pub fn line_overrides(&self, name: &str) -> Result<Vec<LineOverride>, MusicError> {
        Ok(self.lock()?.line_overrides.get(name).cloned().unwrap_or_default())
    }
}
