//! ACOUSTIC: its look, instrument voice, part in the score and card (`kit/fighters/acoustic.py`, ported in milestone
//! 3's Task 22). The app does not offer it until a register task adds it to the registry's `PORTED`.
//!
//! This folder holds everything of it outside the engine: `look.rs`, `sound.rs`, `voice.rs`, `card.rs`, `state.rs`
//! (what the look derives from the recording's events) and the entry points below. Nothing outside it names
//! ACOUSTIC's types.

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
pub use look::AcousticLook;
pub use sound::AcousticSound;
pub use voice::AcousticVoice;

/// `fighters.look('ACOUSTIC', P, k)`.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(AcousticLook::new()))
}

/// Its instrument voice (`pp_bridge`'s entry); also the stadium rock bed's unplugged strums.
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(AcousticVoice))
}

/// `fighters.sound('ACOUSTIC', M, k)`: its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(AcousticSound::new(k)))
}

/// Its model (`data/models/ACOUSTIC.json`, written by its export), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/ACOUSTIC.json"));

/// Whether its music settings (key, chord loop, its line) feed its fight. True only for PIANO, once ported.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// The extra lines its part declares beside its own (HORN: `HORN THEME`).
pub const EXTRA_LINES: &[&str] = &[];
