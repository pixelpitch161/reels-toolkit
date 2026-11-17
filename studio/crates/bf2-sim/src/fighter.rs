//! Fighter definitions (content data; the engine owns no fighter) and a fight's setup.

use std::sync::Arc;

use crate::brain::Brain;
use crate::moves::MoveDef;
use crate::rig::Rig;
use crate::rules::Rules2;

/// One fighter as data.
#[derive(Clone, Debug, PartialEq)]
pub struct FighterDef {
    pub name: String,
    /// Ball radius (0.6).
    pub r: f64,
    pub mass: f64,
    /// HP multiplier on `Rules2::hp`.
    pub hp_scale: f64,
    pub rig: Rig,
    pub moves: Vec<MoveDef>,
    pub brain: Brain,
    pub colour: [u8; 3],
}

/// Everything one fight needs.
#[derive(Clone, Debug)]
pub struct Setup2 {
    pub fighters: [Arc<FighterDef>; 2],
    pub rules: Rules2,
    pub seed: u64,
}
