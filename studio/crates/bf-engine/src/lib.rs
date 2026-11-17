//! The one place a fight file becomes a fight (Engine 2 design, `docs/superpowers/specs/2026-11-17-engine2-design.md`
//! section 1).
//!
//! Every part of the Studio that fights a fight file (search, balance, takes, render jobs, the preview bench and the
//! command line) asks here, never `bf_sim::setup_from` directly, so the engine a file names decides how it is fought.
//! Milestone E0: a current-engine file is set up exactly as before; an Engine 2 file is refused in words. E1 adds
//! [`fight`], which fights a file on the engine it names: Engine 2 fights its `bf_content::fighters2` definitions.

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use bf_core::Engine;
use bf_core::fightfile::FightFile;
use bf_export::{ExportError, JobEvent, JobOpts};
use bf_sim::{Setup, SimError};
use bf_story::{BeatSheet, Candidate};
use bf2_sim::{EvKind, Recording2, Rules2, Setup2, SimError2};

/// Why a fight file could not be set up.
#[derive(Clone, Debug, PartialEq)]
pub enum EngineError {
    /// The file is for an engine this version of the app cannot fight yet.
    NotYet(Engine),
    /// The current engine refused the set-up.
    Sim(SimError),
    /// Engine 2 refused the set-up.
    Sim2(SimError2),
    /// The render job did not make the video.
    Export(ExportError),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::NotYet(e) => {
                write!(
                    f,
                    "this fight is made for {e}, which this version of the app cannot fight yet"
                )
            }
            EngineError::Sim(e) => write!(f, "{e}"),
            EngineError::Sim2(e) => write!(f, "{e}"),
            EngineError::Export(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<SimError> for EngineError {
    fn from(e: SimError) -> Self {
        EngineError::Sim(e)
    }
}

impl From<SimError2> for EngineError {
    fn from(e: SimError2) -> Self {
        EngineError::Sim2(e)
    }
}

/// One fought fight, from the engine the file names.
#[derive(Clone, Debug)]
pub enum Fought {
    /// The current engine's recording.
    One(bf_core::recording::Recording),
    /// Engine 2's recording.
    Two(Recording2),
}

/// Fights `seed` under `ff` on the file's engine; `record` keeps every frame or step (the outcome and events are
/// kept either way). The current engine runs [`classic_setup`] then `bf_sim::run`. Engine 2 fights the two
/// `bf_content::fighters2` definitions the file names (a name with none is [`EngineError::NotYet`]) under
/// `Rules2::new` at the file's tempo grid, with the file's damage scale and handicap.
pub fn fight(ff: &FightFile, seed: u64, record: bool) -> Result<Fought, EngineError> {
    match ff.engine {
        Engine::One => Ok(Fought::One(bf_sim::run(&classic_setup(ff, seed)?, record)?)),
        Engine::Two => Ok(Fought::Two(bf2_sim::run(&engine_two_setup(ff, seed)?, record)?)),
    }
}

impl From<ExportError> for EngineError {
    fn from(e: ExportError) -> Self {
        EngineError::Export(e)
    }
}

/// Fights `seed` under `ff` on the file's engine and renders the video into a new folder under `opts.out_root`:
/// the current engine through `bf_export::run_job` as today, Engine 2 through `bf_export::job2::run_job2` (with
/// the ffmpeg `bf_export::find_ffmpeg` finds). Gives the video's folder.
///
/// # Errors
/// The set-up's or the job's sentence.
pub fn render(
    ff: &FightFile,
    seed: u64,
    opts: &JobOpts,
    on: &(dyn Fn(JobEvent) + Sync),
    cancel: &AtomicBool,
) -> Result<PathBuf, EngineError> {
    match fight(ff, seed, true)? {
        Fought::One(rec) => Ok(bf_export::run_job(ff, Arc::new(rec), opts, on, cancel)?),
        Fought::Two(rec) => {
            let ffmpeg = bf_export::find_ffmpeg()?;
            Ok(bf_export::job2::run_job2(&ffmpeg, ff, Arc::new(rec), opts, on, cancel)?)
        }
    }
}

/// The series beat sheet of an Engine 2 file: `BeatSheet::series` with its two fighters' Engine 2 roster entries
/// (`bf_content::fighters2::cfg`), so the caption targets name their own moves. A fighter with none is
/// [`EngineError::NotYet`].
pub fn beat_sheet2(ff: &FightFile) -> Result<BeatSheet, EngineError> {
    let cfg = |i: usize| {
        ff.fighters
            .get(i)
            .and_then(|n| bf_content::fighters2::cfg(n))
            .ok_or(EngineError::NotYet(Engine::Two))
    };
    Ok(BeatSheet::series([cfg(0)?, cfg(1)?]))
}

/// One Engine 2 seed, read and judged for the seed search, on the video clock: the fight without its steps, the
/// time map from its events (the default `DirectCfg`), the video-clock shadow (events only, no `HP`), then
/// `bf_story::story_at` and `bf_story::score`. Every window of the beat sheet (the K.O. 49 to 53 s, the caption
/// targets, the longest gap) is read in video seconds, as the video shows the fight. A fight with no K.O. is a
/// candidate that does not fit.
///
/// # Errors
/// What [`fight`]'s Engine 2 set-up or run refuses.
pub fn candidate2(ff: &FightFile, seed: u64, sheet: &BeatSheet) -> Result<Candidate, EngineError> {
    let setup = engine_two_setup(ff, seed)?;
    let mut rec = bf2_sim::run(&setup, false)?;
    let map = bf2_direct::TimeMap::from_recording(&rec, &bf2_direct::DirectCfg::default());
    let defs = [setup.fighters[0].as_ref(), setup.fighters[1].as_ref()];
    let grid = ff.grid();
    // Only the events the story reads go through the shadow (`hit`, `ability`; the K.O. is the map's), with those
    // they are read from (a projectile's move, a ProjHit's Contact or Block): the shadow writes every time it keeps
    // as Python's `round(t, 5)` does, through text, which would otherwise cost the search more than the story.
    rec.events.retain(|e| {
        matches!(
            e.kind,
            EvKind::Contact { .. }
                | EvKind::ProjHit { .. }
                | EvKind::Block { .. }
                | EvKind::Active { .. }
                | EvKind::Spawn { .. }
                | EvKind::Ability { .. }
        )
    });
    let sv = bf2_direct::shadow(&rec, defs, &map, bf2_direct::Clock::Video, &grid);
    let lows = [bf2_direct::low_name(defs[0]), bf2_direct::low_name(defs[1])];
    let story = bf_story::story_at(&sv, grid.t_fight, sheet, lows);
    let verdict = bf_story::score(&story, sheet);
    Ok(Candidate { seed, story, verdict })
}

/// Engine 2's set-up of `seed` under `ff`.
fn engine_two_setup(ff: &FightFile, seed: u64) -> Result<Setup2, EngineError> {
    let def = |i: usize| {
        ff.fighters
            .get(i)
            .and_then(|n| bf_content::fighters2::def(n))
            .ok_or(EngineError::NotYet(Engine::Two))
    };
    if ff.fighters.len() != 2 {
        return Err(SimError2::new("fighters", format!("two fighters, this file has {}", ff.fighters.len())).into());
    }
    let fighters: [Arc<bf2_sim::FighterDef>; 2] = [def(0)?, def(1)?];
    if ff.handicap.len() != 2 {
        return Err(SimError2::new(
            "handicap",
            format!("one entry per fighter (2), this file has {}", ff.handicap.len()),
        )
        .into());
    }
    let mut rules = Rules2::new(ff.grid().t_fight);
    rules.dmg_scale = ff.dmg_scale;
    rules.dmg_side = [ff.handicap[0], ff.handicap[1]];
    Ok(Setup2 { fighters, rules, seed })
}

/// The current engine's set-up of `seed` under `ff`: exactly `bf_sim::setup_from(ff, bf_content::roster(), seed)`
/// for a current-engine file; an Engine 2 file is refused before the roster is read.
pub fn classic_setup(ff: &FightFile, seed: u64) -> Result<Setup<'static>, EngineError> {
    match ff.engine {
        Engine::One => Ok(bf_sim::setup_from(ff, bf_content::roster(), seed)?),
        other => Err(EngineError::NotYet(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../../../fights/flute_vs_trombone.toml");

    fn example() -> FightFile {
        FightFile::parse(EXAMPLE).expect("the example parses")
    }

    #[test]
    fn current_engine_matches_bf_sim() {
        let ff = example();
        let ours = classic_setup(&ff, 7).expect("sets up");
        let theirs = bf_sim::setup_from(&ff, bf_content::roster(), 7).expect("sets up");
        assert_eq!(format!("{ours:?}"), format!("{theirs:?}"));
    }

    #[test]
    fn engine_two_is_refused_in_words() {
        let ff = FightFile {
            engine: Engine::Two,
            ..example()
        };
        let e = classic_setup(&ff, 7).expect_err("refused").to_string();
        assert_eq!(
            e,
            "this fight is made for Engine 2, which this version of the app cannot fight yet"
        );
        assert!(!e.contains('\n'));
    }

    fn dummy_file() -> FightFile {
        FightFile {
            engine: Engine::Two,
            fighters: vec!["TEST DUMMY".into(), "TEST DUMMY".into()],
            ..example()
        }
    }

    fn two(f: Fought) -> bf2_sim::Recording2 {
        match f {
            Fought::Two(r) => r,
            Fought::One(_) => panic!("an Engine 2 file fights on Engine 2"),
        }
    }

    #[test]
    fn engine_two_dummy_fights_reproducibly() {
        use bf2_sim::EvKind;
        let ff = dummy_file();
        let a = two(fight(&ff, 5, true).expect("fights"));
        let b = two(fight(&ff, 5, true).expect("fights"));
        assert!(a == b, "the same seed gives the same recording");
        assert!(!a.steps.is_empty());
        assert_eq!(a.t_fight, ff.grid().t_fight);
        let (mut contact, mut launch, mut grabbed, mut spawn, mut expire) = (0, 0, 0, 0, 0);
        for seed in 1..=20 {
            let r = two(fight(&ff, seed, false).expect("fights"));
            for e in &r.events {
                match e.kind {
                    EvKind::Contact { .. } => contact += 1,
                    EvKind::Launch { .. } => launch += 1,
                    EvKind::Grabbed { .. } => grabbed += 1,
                    EvKind::Spawn { .. } => spawn += 1,
                    EvKind::Expire { .. } => expire += 1,
                    _ => {}
                }
            }
        }
        assert!(
            contact > 0 && launch > 0 && grabbed > 0 && spawn > 0,
            "{contact} {launch} {grabbed} {spawn}"
        );
        assert_eq!(spawn, expire, "every projectile expires");
    }

    #[test]
    fn engine_two_takes_the_files_damage() {
        let ff = FightFile {
            dmg_scale: 0.0,
            ..dummy_file()
        };
        let r = two(fight(&ff, 3, false).expect("fights"));
        assert_eq!(r.outcome.damage, [0.0, 0.0]);
        assert!(r.outcome.by_decision);
        let ff = FightFile {
            handicap: vec![1.0, 0.0],
            ..dummy_file()
        };
        let r = two(fight(&ff, 3, false).expect("fights"));
        assert_eq!(r.outcome.damage[1], 0.0);
        assert!(r.outcome.damage[0] > 0.0);
    }

    #[test]
    fn engine_two_unknown_fighter_is_not_yet() {
        let ff = FightFile {
            fighters: vec!["TEST DUMMY".into(), "FLUTE".into()],
            ..dummy_file()
        };
        assert_eq!(fight(&ff, 1, false).unwrap_err(), EngineError::NotYet(Engine::Two));
    }

    /// The TEST DUMMY's own fight file (the dummy against TEST DUMMY 2).
    fn dummy_toml() -> FightFile {
        FightFile::parse(include_str!("../../../fights/engine2/test_dummy.toml")).expect("the dummy's file")
    }

    fn slows_before_ko(r: &bf2_sim::Recording2) -> bool {
        use bf2_sim::{Cue, EvKind};
        let ko = r
            .events
            .iter()
            .find(|e| matches!(e.kind, EvKind::Ko { .. }))
            .map(|e| e.step);
        ko.is_some_and(|ko| {
            r.events
                .iter()
                .any(|e| e.step < ko && matches!(e.kind, EvKind::Cue(Cue::SlowMo { .. })))
        })
    }

    #[test]
    fn beat_sheet2_names_the_dummys_moves() {
        let sheet = beat_sheet2(&dummy_toml()).expect("a sheet");
        assert_eq!(sheet.cap_targets.get("DUMMY BUSTER"), Some(&12.0));
        assert_eq!(sheet.cap_targets.get("SECOND WIND"), Some(&40.0));
        assert_eq!(sheet.t_ko, (49.0, 53.0));
        assert_eq!(beat_sheet2(&example()).unwrap_err(), EngineError::NotYet(Engine::Two));
    }

    #[test]
    fn candidate2_times_are_video_times() {
        let ff = dummy_toml();
        let sheet = beat_sheet2(&ff).unwrap();
        let seed = (1..=200)
            .find(|&s| slows_before_ko(&two(fight(&ff, s, false).unwrap())))
            .expect("a seed with a slow-mo before its K.O.");
        let rec = two(fight(&ff, seed, false).unwrap());
        let map = bf2_direct::TimeMap::from_recording(&rec, &bf2_direct::DirectCfg::default());
        let c = candidate2(&ff, seed, &sheet).expect("a candidate");
        assert_eq!(c.seed, seed);
        assert_eq!(c.story.t_ko, map.ko_video_t());
        assert!(c.story.t_ko.unwrap() > rec.outcome.t_ko.unwrap());
        assert_eq!(c.verdict, bf_story::score(&c.story, &sheet));
    }

    #[test]
    fn candidate2_matches_recorded() {
        let ff = dummy_toml();
        let sheet = beat_sheet2(&ff).unwrap();
        for seed in 1..=12 {
            let c = candidate2(&ff, seed, &sheet).unwrap();
            let rec = two(fight(&ff, seed, true).unwrap());
            let map = bf2_direct::TimeMap::from_recording(&rec, &bf2_direct::DirectCfg::default());
            let defs = [0, 1].map(|k| bf_content::fighters2::def(&ff.fighters[k]).unwrap());
            let sv = bf2_direct::shadow(&rec, [&defs[0], &defs[1]], &map, bf2_direct::Clock::Video, &ff.grid());
            let lows = [bf2_direct::low_name(&defs[0]), bf2_direct::low_name(&defs[1])];
            let recorded = bf_story::story_at(&sv, ff.grid().t_fight, &sheet, lows);
            assert_eq!(c.story, recorded, "seed {seed}");
        }
    }

    /// Review Focus 4: a fight with no K.O. is a misfit, never a panic.
    #[test]
    fn candidate2_without_ko_does_not_fit() {
        let ff = FightFile {
            dmg_scale: 0.01,
            ..dummy_toml()
        };
        let sheet = beat_sheet2(&ff).unwrap();
        let c = candidate2(&ff, 3, &sheet).unwrap();
        assert_eq!(c.story.t_ko, None);
        assert!(!c.verdict.fits);
    }

    /// The story costs little next to the fight: over 200 seeds, at most 1.1x `fight(.., false)` alone (release; the
    /// best of three rounds each).
    #[test]
    #[cfg_attr(debug_assertions, ignore = "a timing, for release builds")]
    fn candidate2_is_cheap() {
        use std::time::Instant;
        let ff = dummy_toml();
        let sheet = beat_sheet2(&ff).unwrap();
        // the best of three rounds of each, taken in turn (the first round also warms the caches)
        let (mut alone, mut with) = (f64::INFINITY, f64::INFINITY);
        for _ in 0..3 {
            let t = Instant::now();
            for seed in 1..=200 {
                fight(&ff, seed, false).unwrap();
            }
            alone = alone.min(t.elapsed().as_secs_f64());
            let t = Instant::now();
            for seed in 1..=200 {
                candidate2(&ff, seed, &sheet).unwrap();
            }
            with = with.min(t.elapsed().as_secs_f64());
        }
        assert!(with <= 1.1 * alone, "{with} s against {alone} s");
    }

    #[test]
    fn engine_one_fights_as_bf_sim() {
        let ff = example();
        let ours = match fight(&ff, 7, false).expect("fights") {
            Fought::One(r) => r,
            Fought::Two(_) => panic!("a current-engine file fights on the current engine"),
        };
        let theirs = bf_sim::run(&classic_setup(&ff, 7).unwrap(), false).unwrap();
        assert_eq!(format!("{ours:?}"), format!("{theirs:?}"));
    }
}
