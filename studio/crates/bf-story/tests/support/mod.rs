//! What the search, balance and statistics tests share: the roster, the example fight file and its set-up.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use bf_core::fightfile::FightFile;
use bf_core::roster::{FighterCfg, parse_roster};
use bf_sim::{Setup, setup_from};
use bf_story::BeatSheet;

pub fn studio() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn roster() -> Vec<FighterCfg> {
    let text = fs::read_to_string(studio().join("crates/bf-content/data/roster.json")).unwrap();
    parse_roster(&text).unwrap()
}

/// The example fight file: FLUTE against TROMBONE, handicap 2.0 / 0.4, the episode the reference statistics are of.
pub fn fight_file() -> FightFile {
    let text = fs::read_to_string(studio().join("fights/flute_vs_trombone.toml")).unwrap();
    FightFile::parse(&text).unwrap()
}

/// The example fight's set-up with seed 0: the tests' closures copy it and put in their seed (and handicap).
pub fn base(roster: &[FighterCfg]) -> Setup<'_> {
    setup_from(&fight_file(), roster, 0).unwrap()
}

/// The example episode's beat sheet.
pub fn sheet(base: &Setup) -> BeatSheet {
    BeatSheet::series(base.fighters)
}

/// The set-up of one seed.
pub fn with_seed<'a>(base: &Setup<'a>, seed: u64) -> Setup<'a> {
    Setup { seed, ..base.clone() }
}

/// The set-up of one seed under a handicap.
pub fn with_handicap<'a>(base: &Setup<'a>, handicap: [f64; 2], seed: u64) -> Setup<'a> {
    let mut setup = with_seed(base, seed);
    setup.rules.dmg_side = Some(handicap);
    setup
}
