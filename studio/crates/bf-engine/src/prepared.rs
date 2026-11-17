//! A fight file made ready once: [`prepare`] sets the file up on its engine, and [`Prepared::candidate`] turns a
//! seed (and a handicap) into a `bf_story::Candidate`, the row the seed search, the stats and the balance read.
//!
//! The current engine's candidate is exactly `bf_story::fight` of the file's set-up at that seed. Engine 2's is
//! judged on the video clock: the fight is run without its steps, the time map is built from its events, and the
//! shadow recording on the video clock (`bf2_direct::shadow`) goes through `bf_story::story_at` and
//! `bf_story::score`, so every beat-sheet window (the K.O. 49 to 53 s, the caption targets, the longest gap) is in
//! the seconds the video shows.

use std::collections::BTreeMap;
use std::sync::Arc;

use bf_core::Engine;
use bf_core::fightfile::FightFile;
use bf_core::grid::Grid;
use bf_sim::Setup;
use bf_story::{BeatSheet, Candidate};
use bf2_direct::shadow::{Clock, low_name, shadow};
use bf2_direct::{DirectCfg, TimeMap};
use bf2_sim::{EvKind, FighterDef, Recording2, Setup2, Tag};

use crate::{EngineError, Fought, classic_setup, engine_two_setup};

/// A fight file set up once, ready to fight any seed on its engine. It is `Send + Sync`, so the search's workers
/// share one.
pub struct Prepared {
    inner: Inner,
    sheet: BeatSheet,
}

enum Inner {
    One(Setup<'static>),
    Two { setup: Setup2, grid: Grid },
}

/// Sets `ff` up on the engine it names: the current engine's set-up (`classic_setup`) and the series beat sheet of
/// its two roster entries, or Engine 2's (its `bf_content::fighters2` definitions) and [`sheet2`].
///
/// # Errors
/// The set-up's refusal, as [`crate::fight`] gives it.
pub fn prepare(ff: &FightFile) -> Result<Prepared, EngineError> {
    match ff.engine {
        Engine::One => {
            let setup = classic_setup(ff, 0)?;
            let sheet = BeatSheet::series(setup.fighters);
            Ok(Prepared {
                inner: Inner::One(setup),
                sheet,
            })
        }
        Engine::Two => {
            let setup = engine_two_setup(ff, 0)?;
            let sheet = sheet2([&setup.fighters[0], &setup.fighters[1]]);
            Ok(Prepared {
                inner: Inner::Two { setup, grid: ff.grid() },
                sheet,
            })
        }
    }
}

/// Engine 2's beat sheet for two definitions (left first): the series windows of `BeatSheet::series` (the K.O. 49
/// to 53 s, 2 lead changes, the winner at most 30 HP, captions 4 s apart, no gap over 4 s) with the caption targets
/// read from the moves: a fighter's first special move (a `Tag::Special` move without `Tag::Low`) at 12 s, its
/// second at 24 s and its low move at 40 s (a name met twice keeps the later value, as the series sheet does). It
/// asks for all but one of the captions the two fighters can make (each captions each of its ability moves, the
/// special and low ones, once), at most 5: the series' 5 of 6 for two fighters of three abilities each.
pub fn sheet2(defs: [&FighterDef; 2]) -> BeatSheet {
    let mut cap_targets = BTreeMap::new();
    let mut abilities = 0;
    for def in defs {
        let specials = def.moves.iter().filter(|m| m.has(Tag::Special) && !m.has(Tag::Low));
        for (m, t) in specials.zip([12.0, 24.0]) {
            cap_targets.insert(m.name.to_string(), t);
        }
        let low = low_name(def);
        if !low.is_empty() {
            cap_targets.insert(low.to_string(), 40.0);
        }
        abilities += def
            .moves
            .iter()
            .filter(|m| m.has(Tag::Special) || m.has(Tag::Low))
            .count();
    }
    BeatSheet {
        t_ko: (49.0, 53.0),
        min_lead_changes: 2,
        max_winner_hp: 30.0,
        cap_gap: 4.0,
        max_gap: 4.0,
        min_caps: abilities.saturating_sub(1).min(5),
        cap_targets,
    }
}

/// The time map of an Engine 2 fight from its events alone (a fight fought without its steps): it covers the
/// fight up to its last event, which holds the K.O. When the steps were kept, it is the render job's map.
pub(crate) fn event_map(rec: &Recording2) -> TimeMap {
    let cfg = DirectCfg::default();
    if !rec.steps.is_empty() {
        return TimeMap::from_recording(rec, &cfg);
    }
    let ko = rec
        .events
        .iter()
        .find(|e| matches!(e.kind, EvKind::Ko { .. }))
        .map(|e| e.step);
    let steps = rec.events.last().map_or(0, |e| e.step + 1);
    TimeMap::build(&rec.events, steps, rec.dt, rec.t_fight, ko, &cfg)
}

/// The K.O. of an Engine 2 fight on the video clock, `None` without one.
pub(crate) fn ko_video(rec: &Recording2) -> Option<f64> {
    rec.outcome.t_ko?;
    event_map(rec).ko_video_t()
}

/// Engine 2's candidate of the fight `rec` (fought by `defs`) under `sheet`, on the video clock.
fn candidate_two(rec: &Recording2, defs: [&FighterDef; 2], grid: &Grid, sheet: &BeatSheet) -> Candidate {
    let map = event_map(rec);
    let mut sh = shadow(rec, defs, &map, Clock::Video, grid);
    // A decision has no K.O., whatever the map says.
    if rec.outcome.t_ko.is_none() {
        sh.t_ko = None;
    }
    let lows = [low_name(defs[0]), low_name(defs[1])];
    let story = bf_story::story_at(&sh, grid.t_fight, sheet, lows);
    let verdict = bf_story::score(&story, sheet);
    Candidate {
        seed: rec.seed,
        story,
        verdict,
    }
}

impl Prepared {
    /// The engine the file names.
    pub fn engine(&self) -> Engine {
        match self.inner {
            Inner::One(_) => Engine::One,
            Inner::Two { .. } => Engine::Two,
        }
    }

    /// The beat sheet the candidates are judged by.
    pub fn sheet(&self) -> &BeatSheet {
        &self.sheet
    }

    /// The two fighters' colours (sRGB), left first.
    pub fn colours(&self) -> [[u8; 3]; 2] {
        match &self.inner {
            Inner::One(s) => s.fighters.map(|f| f.color),
            Inner::Two { setup, .. } => [setup.fighters[0].colour, setup.fighters[1].colour],
        }
    }

    /// Judges a fight of this file fought elsewhere (the take's, recorded): the same candidate
    /// [`Prepared::candidate`] gives for its seed without a handicap. A fight of the other engine is refused in
    /// words.
    ///
    /// # Errors
    /// The fight is not on this file's engine.
    pub fn judge(&self, fought: &Fought) -> Result<Candidate, String> {
        match (&self.inner, fought) {
            (Inner::One(base), Fought::One(rec)) => {
                let lows = [base.fighters[0].low.name.as_str(), base.fighters[1].low.name.as_str()];
                let story = bf_story::story_at(rec, base.rules.t_fight, &self.sheet, lows);
                let verdict = bf_story::score(&story, &self.sheet);
                Ok(Candidate {
                    seed: rec.seed,
                    story,
                    verdict,
                })
            }
            (Inner::Two { setup, grid }, Fought::Two(rec)) => Ok(candidate_two(
                rec,
                [&setup.fighters[0], &setup.fighters[1]],
                grid,
                &self.sheet,
            )),
            _ => Err(format!(
                "a fight on another engine than this file's ({})",
                self.engine()
            )),
        }
    }

    /// Fights `seed` (with `handicap` as the damage handicap when given, else the file's) and judges it. The
    /// current engine: `bf_story::fight` of the set-up at `seed`. Engine 2: the fight without its steps, judged on
    /// the video clock (see the module).
    ///
    /// # Errors
    /// The engine's refusal of the fight, in words.
    pub fn candidate(&self, seed: u64, handicap: Option<[f64; 2]>) -> Result<Candidate, String> {
        match &self.inner {
            Inner::One(base) => {
                let mut s = Setup { seed, ..base.clone() };
                if let Some(h) = handicap {
                    s.rules.dmg_side = Some(h);
                }
                bf_story::fight(&s, &self.sheet).map_err(|e| e.to_string())
            }
            Inner::Two { setup, grid } => {
                let mut s = Setup2 {
                    fighters: [Arc::clone(&setup.fighters[0]), Arc::clone(&setup.fighters[1])],
                    rules: setup.rules.clone(),
                    seed,
                };
                if let Some(h) = handicap {
                    s.rules.dmg_side = h;
                }
                let rec = bf2_sim::run(&s, false).map_err(|e| e.to_string())?;
                Ok(candidate_two(&rec, [&s.fighters[0], &s.fighters[1]], grid, &self.sheet))
            }
        }
    }
}
