//! The story of one recorded fight: a port of `projects/ballfight/kit/story.py` `story()`, with `fight.py`
//! `lead_changes()` and `story.captions()` (`bf_core::show::captions`).
//!
//! Every number is a sum or a difference in the kit's order (no transcendental), so a story equals the kit's bit
//! for bit.

use std::collections::BTreeSet;

use bf_core::event::Event;
use bf_core::grid::Grid;
use bf_core::py::py_max;
use bf_core::recording::Recording;
use bf_core::show::captions;

use crate::score::BeatSheet;

/// `fight.py` `lead_changes()` defaults: every fighter starts on 100 HP, a lead needs more than 5 HP, and a flip
/// before 4 s does not count.
const HP0: f64 = 100.0;
const LEAD_HYST: f64 = 5.0;
const LEAD_FROM: f64 = 4.0;

/// `max(gaps) if gaps else 99`: the longest gap of a fight without a hit.
const NO_HIT_GAP: f64 = 99.0;

/// What happened in one fight: the kit's story dict, plus the lead ribbon's data.
#[derive(Clone, Debug, PartialEq)]
pub struct Story {
    /// `run['seed']`.
    pub seed: u64,
    /// `run['winner']`: the fighter left standing; `None` without a K.O.
    pub winner: Option<usize>,
    /// `run['t_ko']`: the K.O. time on the video clock; `None` without a K.O.
    pub t_ko: Option<f64>,
    /// `lead_changes(ev)[0]`: how often the HP lead flipped (5 HP hysteresis, from 4 s).
    pub lead_changes: u32,
    /// `lead_changes(ev)[1]`: when each counted flip happened.
    pub lc_t: Vec<f64>,
    /// `max(0.0, hp[w])` (hit points from the `hit` events alone) when there is a winner, else 100.0.
    pub winner_hp: f64,
    /// Every first use of an ability, `(t, who, name)`, in event order.
    pub firsts: Vec<(f64, usize, String)>,
    /// `captions(ev)`: the first uses kept at least `cap_gap` apart, `(t, who, name)`.
    pub caps: Vec<(f64, usize, String)>,
    /// The longest time without a hit, from FIGHT! to the first hit included; 99.0 when no hit landed.
    pub max_gap: f64,
    /// The desperation moves (low abilities) that fired, sorted by name.
    pub lows: Vec<String>,
    /// The lead ribbon: `(t, hp[0] - hp[1])` after each `hit` event, each side's HP floored at 0 as the
    /// recording's `HP` array holds it. HP here follows `lead_changes()`: hits take away, `heal` events give back.
    /// Not in the kit's story dict.
    pub lead: Vec<(f64, f64)>,
}

/// A fighter index element (`hp[e[3]]`); Python's negative indices never occur for a fighter.
fn fighter(e: &Event, i: usize) -> usize {
    let n = e.i(i);
    usize::try_from(n)
        .ok()
        .filter(|&k| k < 2)
        .unwrap_or_else(|| panic!("event {:?} at {}: element {i} is {n}, not fighter 0 or 1", e.kind, e.t))
}

/// `fight.py` `lead_changes(events)`, plus the lead ribbon: `(n, times, lead)`.
fn lead_changes(events: &[Event]) -> (u32, Vec<f64>, Vec<(f64, f64)>) {
    let mut hp = [HP0, HP0];
    let (mut lead, mut n, mut times) = (0i32, 0u32, Vec::new());
    let mut ribbon = Vec::new();
    for e in events {
        if e.kind == "heal" {
            // ability kind 'rewind' gives back damage taken
            hp[fighter(e, 2)] += e.f(3);
            continue;
        }
        if e.kind == "hit" {
            hp[fighter(e, 3)] -= e.f(4);
            let d = hp[0] - hp[1];
            ribbon.push((e.t, py_max(0.0, hp[0]) - py_max(0.0, hp[1])));
            let new = if d > LEAD_HYST {
                1
            } else if d < -LEAD_HYST {
                -1
            } else {
                lead
            };
            if new != lead && new != 0 {
                if lead != 0 && e.t >= LEAD_FROM {
                    n += 1;
                    times.push(e.t);
                }
                lead = new;
            }
        }
    }
    (n, times, ribbon)
}

/// `kit/story.py` `story(run)` for a recording: `grid` gives `T.T_FIGHT` (the start of the first gap), `sheet` the
/// caption gap, and `low_names` the two fighters' low ability names in the recording's order (`story.lows()`).
///
/// # Panics
/// On a malformed `hit`, `heal` or `ability` event (an element missing or of the wrong type), as the kit raises
/// there. Recordings made by the engine or read from a file never have one.
pub fn story(rec: &Recording, grid: &Grid, sheet: &BeatSheet, low_names: [&str; 2]) -> Story {
    story_at(rec, grid.t_fight, sheet, low_names)
}

/// [`story`] for a caller that has the rules of the fight and no grid (the seed search): `t_fight` is `T.T_FIGHT`,
/// which the rules hold as `rules.t_fight`.
///
/// # Panics
/// As [`story`].
pub fn story_at(rec: &Recording, t_fight: f64, sheet: &BeatSheet, low_names: [&str; 2]) -> Story {
    let ev = &rec.events;
    let mut hp = [HP0, HP0];
    let mut hits_t = Vec::new();
    for e in ev {
        if e.kind == "hit" {
            hp[fighter(e, 3)] -= e.f(4);
            hits_t.push(e.t);
        }
    }
    let (lead_changes, lc_t, lead) = lead_changes(ev);
    let firsts = ev
        .iter()
        .filter(|e| e.kind == "ability" && e.at(5).truthy())
        .map(|e| (e.t, fighter(e, 2), e.s(3).to_string()))
        .collect();
    let caps = captions(ev, sheet.cap_gap);
    let winner_hp = match rec.winner {
        Some(w) => py_max(0.0, hp[w]),
        None => 100.0,
    };
    // gaps = [b - a for a, b in zip([T.T_FIGHT] + hits_t, hits_t)]
    let mut max_gap: Option<f64> = None;
    let mut a = t_fight;
    for &b in &hits_t {
        let g = b - a;
        max_gap = Some(max_gap.map_or(g, |m| py_max(m, g)));
        a = b;
    }
    let lows: BTreeSet<String> = ev
        .iter()
        .filter(|e| e.kind == "ability" && low_names.contains(&e.s(3)))
        .map(|e| e.s(3).to_string())
        .collect();
    Story {
        seed: rec.seed,
        winner: rec.winner,
        t_ko: rec.t_ko,
        lead_changes,
        lc_t,
        winner_hp,
        firsts,
        caps,
        max_gap: max_gap.unwrap_or(NO_HIT_GAP),
        lows: lows.into_iter().collect(),
        lead,
    }
}
