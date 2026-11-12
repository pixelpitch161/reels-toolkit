//! CONTRABASS: its look, instrument voice, part in the score and card (`kit/fighters/contrabass.py`, ported in
//! milestone 3's Task 18). The app does not offer it until a register task adds it to the registry's `PORTED`.
//!
//! This folder holds everything of it outside the engine: `look.rs`, `sound.rs`, `voice.rs`, `card.rs`, `state.rs`
//! (what the look derives from the recording) and the entry points below. Nothing outside it names CONTRABASS's
//! types.

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
pub use look::ContrabassLook;
pub use sound::ContrabassSound;
pub use voice::ContrabassVoice;

/// `fighters.look('CONTRABASS', P, k)`.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(ContrabassLook::new()))
}

/// Its instrument voice (`pp_bridge`'s entry).
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(ContrabassVoice))
}

/// `fighters.sound('CONTRABASS', M, k)`: its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(ContrabassSound::new(k)))
}

/// Its model (`data/models/CONTRABASS.json`, written by its export), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/CONTRABASS.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
