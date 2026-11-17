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
use bf2_sim::{Recording2, Rules2, Setup2, SimError2};

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
