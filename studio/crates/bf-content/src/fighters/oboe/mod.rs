//! OBOE: its look, instrument voice, part in the score and card (`kit/fighters/oboe.py`), ported in milestone 3's
//! Task 11. The app does not offer it until a register task adds it to the registry's `PORTED`.
//!
//! Nothing outside this folder names OBOE's types: every suite and the registry reach it through the entry points
//! below.

use std::sync::Arc;

use bf_core::fightfile::FightFile;
use bf_picture::look::Look;
use bf_sound::traits::Sound;
use bf_sound::voice::Voice;

pub mod card;
pub mod look;
pub mod sound;
pub mod state;
pub mod voice;

pub use card::{CARD, card};
pub use look::OboeLook;
pub use sound::OboeSound;
pub use voice::OboeVoice;

/// `fighters.look('OBOE', P, k)`: its look.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(OboeLook::default()))
}

/// Its instrument voice (`pp_bridge`'s entry).
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(OboeVoice))
}

/// `fighters.sound('OBOE', M, k)`: its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(OboeSound::new(k)))
}

/// Its model (`data/models/OBOE.json`, written by its export), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/OBOE.json"));

/// Whether its music settings (key, chord loop, its line) feed its fight. True only for PIANO, once ported.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// The extra lines its part declares beside its own (HORN: `HORN THEME`).
pub const EXTRA_LINES: &[&str] = &[];
