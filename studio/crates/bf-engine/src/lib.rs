//! The one place a fight file becomes a fight (Engine 2 design, `docs/superpowers/specs/2026-11-17-engine2-design.md`
//! section 1).
//!
//! Every part of the Studio that fights a fight file (search, balance, takes, render jobs, the preview bench and the
//! command line) asks here, never `bf_sim::setup_from` directly, so the engine a file names decides how it is fought.
//! Milestone E0: a current-engine file is set up exactly as before; an Engine 2 file is refused in words. E1 adds
//! Engine 2's set-up and fight here.

use std::fmt;

use bf_core::Engine;
use bf_core::fightfile::FightFile;
use bf_sim::{Setup, SimError};

/// Why a fight file could not be set up.
#[derive(Clone, Debug, PartialEq)]
pub enum EngineError {
    /// The file is for an engine this version of the app cannot fight yet.
    NotYet(Engine),
    /// The current engine refused the set-up.
    Sim(SimError),
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
        }
    }
}

impl std::error::Error for EngineError {}

impl From<SimError> for EngineError {
    fn from(e: SimError) -> Self {
        EngineError::Sim(e)
    }
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
}
