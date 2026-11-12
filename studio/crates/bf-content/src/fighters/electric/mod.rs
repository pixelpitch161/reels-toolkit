//! ELECTRIC: its look, instrument voice, part in the score and card (`kit/fighters/electric.py`, ported in milestone
//! 3's Task 23). The app does not offer it until a register task adds it to the registry's `PORTED`.
//!
//! This folder holds everything of it outside the engine: `look.rs`, `sound.rs`, `voice.rs`, `card.rs`, `state.rs`
//! (what the part derives from the recording's events) and the entry points below. Nothing outside it names
//! ELECTRIC's types. Its roster voice id is an internal id that is never shown (ruling M3-R9): nothing here names it.

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
pub use look::ElectricLook;
pub use voice::ElectricVoice;

/// Its roster name.
pub const NAME: &str = "ELECTRIC";

/// `fighters.look('ELECTRIC', P, k)`.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(ElectricLook::new()))
}

/// Its instrument voice (`pp_bridge`'s entry).
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(ElectricVoice))
}

/// `fighters.sound('ELECTRIC', M, k)`: its part on side `k` of the fight `ff`, or `None` while it is not ported.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = (k, ff);
    None
}

/// Its model (`data/models/ELECTRIC.json`, written by its export), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/ELECTRIC.json"));

/// Whether its music settings (key, chord loop, its line) feed its fight. True only for PIANO, once ported.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// The extra lines its part declares beside its own (HORN: `HORN THEME`).
pub const EXTRA_LINES: &[&str] = &[];
