//! MELLOPHONE: a fighter made from scratch in the Studio (2026-11-16; design
//! `docs/superpowers/specs/2026-11-16-mellophone-fighter-design.md`). There is no Python kit module: its engine kinds
//! are `bf-sim/src/kinds/slapback.rs`, its roster entry is in `data/roster_new.json`, and this folder holds its look,
//! voice, part, card and musical personality.

use std::sync::Arc;

use bf_core::fightfile::FightFile;
use bf_picture::look::Look;
use bf_sound::traits::Sound;
use bf_sound::voice::Voice;

pub mod card;
pub mod look;
pub mod personality;
pub mod sound;
pub mod voice;

pub use card::{CARD, card};

/// Its look.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(look::MellophoneLook::default()))
}

/// Its instrument voice.
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(voice::MellophoneVoice))
}

/// Its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(sound::MellophoneSound::new(k)))
}

/// Its model (`data/models/MELLOPHONE.json`, written by `studio/tools/models/mellophone_model.py` from photos of a
/// Yamaha YMP-204MS), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/MELLOPHONE.json"));

/// Its music settings do not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
