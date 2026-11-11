//! The show timings and caption windows: what picture and sound both need from a fight, derived once.
//!
//! A port of the module-level derivations at the top of `projects/ballfight/kit/picture.py` (`T_KO`, `WIN`, `LOSE`,
//! `NAMES`, `HITSTOP`, `T_HS`, `T_SOLO`, `SOLO_T`) and of its `_caps()`, with `kit/story.py`'s `captions()` (the
//! first use of each ability, kept greedily when at least `CAP_GAP` seconds after the previous kept one), and of
//! `reelkit/ui.py`'s `impact_landings` (when each caption line lands: the picture drops the lines then and the
//! sound puts a thud on each). Every time here is a sum or a product in Python's operation order, so it equals the
//! kit's bit for bit.

use std::fmt;

use crate::event::{Event, Value};
use crate::fightfile::FightFile;
use crate::grid::Grid;
use crate::py::round_half_even;
use crate::recording::Recording;

/// The timeline's `CAP_GAP`: the least time between two kept ability captions (the series default, 4.0 s).
pub const CAP_GAP: f64 = 4.0;

/// picture.py `HITSTOP`: the freeze after the K.O. hit, 0.35 s.
pub const HITSTOP: f64 = 0.35;

/// How long an ability caption stays up (`(t, t + 2.4, [name])` in `_caps()`).
const ABILITY_HOLD: f64 = 2.4;

/// `reelkit/ui.py` `IMPACT_STAGGER`: the seconds between one caption line landing and the next.
pub const IMPACT_STAGGER: f64 = 0.22;

/// `reelkit/ui.py` `IMPACT_FALL`: the seconds a caption line falls before it lands.
pub const IMPACT_FALL: f64 = 0.10;

/// One caption window, a `(t0, t1, lines)` tuple of `_caps()`: drawn while `t0 <= t < t1` (`reelkit/ui.py`
/// `draw_captions`; a window with `t1 <= t0` is never drawn). A line may carry `*...*` highlight marks.
#[derive(Clone, Debug, PartialEq)]
pub struct Cap {
    pub t0: f64,
    pub t1: f64,
    pub lines: Vec<String>,
}

impl Cap {
    fn new(t0: f64, t1: f64, lines: &[&str]) -> Cap {
        Cap {
            t0,
            t1,
            lines: lines.iter().map(|l| l.to_string()).collect(),
        }
    }
}

/// What picture and sound both derive from a recording and its fight file.
#[derive(Clone, Debug, PartialEq)]
pub struct Show {
    /// The bar and beat grid at the fight file's tempo.
    pub grid: Grid,
    /// `NAMES`: the recording's fighters, left corner first.
    pub names: [String; 2],
    /// `WIN`: `int(RUN['winner'])`.
    pub win: usize,
    /// `LOSE`: `1 - WIN`.
    pub lose: usize,
    /// `T_KO`: `float(RUN['t_ko'])`.
    pub t_ko: f64,
    /// `T_HS`: `T_KO + HITSTOP`, the end of the hit-stop.
    pub t_hs: f64,
    /// `T_SOLO`: `T.next_bar(T_KO + 1.6)`, the winner's solo starts on the first bar line after the K.O. moment.
    pub t_solo: f64,
    /// `CAP_ABIL`: per fighter, `(t, t + 2.4, [name])` for each of its kept ability captions, in time order.
    pub caps_ability: [Vec<Cap>; 2],
    /// `CAP_KO`: `[(T_KO, T_KO + 1.25, ['*K.O.*'])]`.
    pub cap_ko: Vec<Cap>,
    /// `CAP_WINS`: `[(T_KO + 1.3, T_KO + 3.55, [NAMES[WIN], 'WINS!'])]`.
    pub cap_wins: Vec<Cap>,
    /// `CAP_COMMENT`: `[(T_KO + 3.65, T.T_REASSEMBLE, ['COMMENT THE', '*NEXT MATCH-UP*'])]`. After a late K.O.
    /// `t0` can pass `t1` (seed 500: 59.54 against 57.41), and the window is then never drawn, as in the kit.
    pub cap_comment: Vec<Cap>,
    /// `CAPTIONS`: `CAP_TITLE + CAP_ABIL[0] + CAP_ABIL[1] + CAP_KO + CAP_WINS + CAP_COMMENT`, the list the audio
    /// reads for its caption thuds. `CAP_TITLE` is always empty (the VS title card replaces the top caption).
    pub captions: Vec<Cap>,
}

/// Why a [`Show`] cannot be made from a recording.
#[derive(Clone, Debug, PartialEq)]
pub enum ShowError {
    /// The fight has no K.O. (no winner or no K.O. time): there is no show to make.
    NoKo,
    /// The recording names a winner that is not fighter 0 or 1.
    Winner(usize),
    /// The recording does not have two fighters, or not the fight file's two in its order.
    Names {
        recording: Vec<String>,
        fight_file: Vec<String>,
    },
    /// An `ability` event the captions read is malformed (`index` counts from 0 in the event list).
    Event { index: usize, problem: String },
    /// The recording was not fought on the fight file's grid (its tempo, its beat, the time the fight starts or
    /// the video frame of its first frame differ), or does not say what it was fought at: the whole sentence.
    Tempo(String),
}

impl fmt::Display for ShowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShowError::NoKo => f.write_str("the fight has no K.O. (no winner and K.O. time)"),
            ShowError::Winner(w) => write!(f, "the recording names winner {w}, not fighter 0 or 1"),
            ShowError::Names { recording, fight_file } => write!(
                f,
                "the recording's fighters {recording:?} are not the fight file's {fight_file:?}"
            ),
            ShowError::Event { index, problem } => write!(f, "event {index} (ability): {problem}"),
            ShowError::Tempo(problem) => f.write_str(problem),
        }
    }
}

impl std::error::Error for ShowError {}

/// `kit/story.py` `captions(events, gap)`, reporting a malformed event instead of panicking.
fn try_captions(events: &[Event], gap: f64) -> Result<Vec<(f64, usize, String)>, ShowError> {
    let mut out: Vec<(f64, usize, String)> = Vec::new();
    for (index, e) in events.iter().enumerate() {
        if e.kind != "ability" {
            continue;
        }
        let bad = |problem: String| ShowError::Event { index, problem };
        // e[5]: 1 for the first use of that ability
        let first = e
            .fields
            .get(3)
            .ok_or_else(|| bad(format!("no element 5 (it has {})", 2 + e.fields.len())))?;
        if !first.truthy() {
            continue;
        }
        let keep = match out.last() {
            None => true,
            Some(last) => e.t - last.0 >= gap,
        };
        if keep {
            // (e[0], e[2], e[3]); e[2] indexes CAP_ABIL, so it must be fighter 0 or 1
            let who = e
                .who()
                .filter(|&w| w < 2)
                .ok_or_else(|| bad(format!("element 2 is {:?}, not fighter 0 or 1", e.fields.first())))?;
            let name = match e.fields.get(1) {
                Some(Value::Str(s)) => s.clone(),
                other => return Err(bad(format!("element 3 is {other:?}, not the ability's name"))),
            };
            out.push((e.t, who, name));
        }
    }
    Ok(out)
}

/// `kit/story.py` `captions(events, gap)`: the first use of each ability (an `ability` event whose element 5 is
/// true), kept when it comes at least `gap` seconds after the previous kept one, as `(t, who, name)`.
///
/// # Panics
/// When a first-use `ability` event lacks an element the kit reads, or names no fighter 0 or 1 (Python raises
/// there too). [`Show::new`] reports the same as [`ShowError::Event`] instead.
pub fn captions(events: &[Event], gap: f64) -> Vec<(f64, usize, String)> {
    try_captions(events, gap).unwrap_or_else(|e| panic!("captions: {e}"))
}

/// One row of `impact_landings`, Python's `(t_land, k, len(_plain(line)))`: when a caption line lands, which line
/// of its caption it is (from 0) and how many characters it has without its `*` and `~` marks (Python's
/// characters: Unicode code points).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Landing {
    /// When the line lands, in seconds.
    pub t: f64,
    /// Which line of its caption it is, from 0.
    pub line: usize,
    /// Its characters without the marks.
    pub chars: usize,
}

/// `reelkit/ui.py` `impact_landings(caps, stagger, fall)`: when each line of the caption windows `caps` lands in
/// the `impact` style, in the order of `caps`: line `k` of a window lands at `t0 + k * stagger + fall`, and a
/// line that would land at or after the window's end is left out. Python's defaults are [`IMPACT_STAGGER`] and
/// [`IMPACT_FALL`]. The picture drops the lines at these times; the sound puts a soft thud on each.
pub fn impact_landings(caps: &[Cap], stagger: f64, fall: f64) -> Vec<Landing> {
    let mut out = Vec::new();
    for cap in caps {
        for (k, line) in cap.lines.iter().enumerate() {
            let t = cap.t0 + k as f64 * stagger + fall;
            if t < cap.t1 {
                let chars = line.chars().filter(|c| !matches!(c, '*' | '~')).count();
                out.push(Landing { t, line: k, chars });
            }
        }
    }
    out
}

/// Two numbers as text: with `short` decimals, or in full where that would make them look the same.
fn two(a: f64, b: f64, short: usize) -> (String, String) {
    let (x, y) = (format!("{a:.short$}"), format!("{b:.short$}"));
    if x == y {
        (format!("{a:?}"), format!("{b:?}"))
    } else {
        (x, y)
    }
}

/// That the recording `rec` was fought on `grid`, the grid of the fight file whose tempo is `tempo_bpm`.
///
/// The recording's `rules.bpm`, `rules.beat` and `rules.t_fight` are `T.BPM`, `T.BEAT` and `T.T_FIGHT` of the
/// tempo it was fought at, the same expressions in the same order as [`Grid`]'s (the kit's `build_timeline`, and
/// the Studio's fight engine writes the grid's own values), and `f0` is `round(t_fight * fps)`. So the values
/// of the same tempo are equal to the bit, and they are compared exactly: there is no tolerance to choose. Two
/// tempos that fit 61 s in the same number of bars give the same grid, and are the same fight.
fn check_grid(rec: &Recording, grid: &Grid, tempo_bpm: f64) -> Result<(), ShowError> {
    let rule = |key: &str| match rec.rules.get(key) {
        Some(Value::Float(x)) => Ok(*x),
        Some(_) => Err(ShowError::Tempo(format!(
            "the recording does not say its tempo: rules.{key} is not a number"
        ))),
        None => Err(ShowError::Tempo(format!(
            "the recording does not say its tempo: rules.{key} is missing"
        ))),
    };
    let (bpm, beat, t_fight) = (rule("bpm")?, rule("beat")?, rule("t_fight")?);
    if bpm != grid.bpm {
        let (a, b) = two(bpm, grid.bpm, 2);
        return Err(ShowError::Tempo(format!(
            "the recording was fought at {a} bpm; the fight file's grid is {b} bpm (tempo_bpm = {tempo_bpm}, \
             fitted to whole bars in 61 s): fight this fight file again, or set its tempo back"
        )));
    }
    if beat != grid.beat {
        return Err(ShowError::Tempo(format!(
            "the recording's beat is {beat:?} s; the fight file's grid has {:?} s",
            grid.beat
        )));
    }
    if t_fight != grid.t_fight {
        let (a, b) = two(t_fight, grid.t_fight, 3);
        return Err(ShowError::Tempo(format!(
            "the recording's fight starts at {a} s; the fight file's grid starts it at {b} s (bar {})",
            grid.fight_bar
        )));
    }
    // f0 = round(rules.t_fight * fps)
    let f0 = round_half_even(grid.t_fight * f64::from(rec.fps));
    if rec.f0 as f64 != f0 {
        return Err(ShowError::Tempo(format!(
            "the recording's first frame is video frame {}; the fight file's grid starts the fight on frame {f0}",
            rec.f0
        )));
    }
    Ok(())
}

impl Show {
    /// The show of a fight: picture.py's module-level values and `_caps()` for this recording, on the fight file's
    /// grid, with the caption gap [`CAP_GAP`]. A fight without a K.O. is [`ShowError::NoKo`]; a recording fought
    /// on another grid than the fight file's (another tempo) is [`ShowError::Tempo`].
    pub fn new(rec: &Recording, ff: &FightFile) -> Result<Show, ShowError> {
        // T_KO = float(RUN['t_ko']) and WIN = int(RUN['winner']): Python raises on None
        let (Some(t_ko), Some(win)) = (rec.t_ko, rec.winner) else {
            return Err(ShowError::NoKo);
        };
        if win >= 2 {
            return Err(ShowError::Winner(win));
        }
        let names: [String; 2] = match <[String; 2]>::try_from(rec.names.clone()) {
            Ok(n) if n[..] == ff.fighters[..] => n,
            _ => {
                return Err(ShowError::Names {
                    recording: rec.names.clone(),
                    fight_file: ff.fighters.clone(),
                });
            }
        };
        let lose = 1 - win;
        let grid = ff.grid();
        check_grid(rec, &grid, ff.tempo_bpm)?;
        let t_hs = t_ko + HITSTOP;
        let t_solo = grid.next_bar(t_ko + 1.6);

        let mut caps_ability: [Vec<Cap>; 2] = [Vec::new(), Vec::new()];
        for (t, who, name) in try_captions(&rec.events, CAP_GAP)? {
            caps_ability[who].push(Cap::new(t, t + ABILITY_HOLD, &[&name]));
        }
        let cap_ko = vec![Cap::new(t_ko, t_ko + 1.25, &["*K.O.*"])];
        let cap_wins = vec![Cap::new(t_ko + 1.3, t_ko + 3.55, &[&names[win], "WINS!"])];
        let cap_comment = vec![Cap::new(
            t_ko + 3.65,
            grid.t_reassemble,
            &["COMMENT THE", "*NEXT MATCH-UP*"],
        )];
        let captions = caps_ability
            .iter()
            .flatten()
            .chain(&cap_ko)
            .chain(&cap_wins)
            .chain(&cap_comment)
            .cloned()
            .collect();
        Ok(Show {
            grid,
            names,
            win,
            lose,
            t_ko,
            t_hs,
            t_solo,
            caps_ability,
            cap_ko,
            cap_wins,
            cap_comment,
            captions,
        })
    }

    /// picture.py `SOLO_T`: the start of each note of a solo `[(beat, midi, beats)]` (the winner's, from the fight
    /// file or the kit's default), `T_SOLO + beat * BEAT`.
    pub fn solo_times(&self, solo: &[(f64, i32, f64)]) -> Vec<f64> {
        solo.iter()
            .map(|&(bt, _, _)| self.t_solo + bt * self.grid.beat)
            .collect()
    }
}
