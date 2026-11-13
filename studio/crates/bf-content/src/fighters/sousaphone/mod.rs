//! SOUSAPHONE: its look, instrument voice, part in the score and card (`kit/fighters/sousaphone.py`, ported in
//! milestone 3's Task 12). The app does not offer it until a register task adds it to the registry's `PORTED`.
//!
//! This folder holds everything of it outside the engine: `look.rs`, `sound.rs`, `voice.rs`, `card.rs`, `state.rs`
//! (what the look and the part both derive from the recording) and the entry points below. Nothing outside it
//! names SOUSAPHONE's types.

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
pub use look::SousaphoneLook;
pub use sound::SousaphoneSound;
pub use voice::SousaphoneVoice;

/// `fighters.look('SOUSAPHONE', P, k)`.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(SousaphoneLook::new()))
}

/// Its instrument voice (`pp_bridge`'s entry).
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(SousaphoneVoice))
}

/// `fighters.sound('SOUSAPHONE', M, k)`: its part on side `k` of the fight `ff`. (A fight file has no field for the
/// kit's opt-in `OBBLIGATO`, which no kit config sets: the part is made without one.)
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(SousaphoneSound::new(k)))
}

/// Its model (`data/models/SOUSAPHONE.json`, written by its export), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/SOUSAPHONE.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
