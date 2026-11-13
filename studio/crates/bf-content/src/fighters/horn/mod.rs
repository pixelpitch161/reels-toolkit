//! HORN: its look, instrument voice, part in the score and card (`kit/fighters/horn.py`, ported in milestone 3's
//! Task 21). The app does not offer it until a register task adds it to the registry's `PORTED`.
//!
//! Nothing outside this folder names HORN's types, so nothing outside it changes.

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

pub use look::HornLook;
pub use sound::HornSound;
pub use voice::HornVoice;

/// `fighters.look('HORN', P, k)`.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(HornLook::default()))
}

/// Its instrument voice (`pp_bridge`'s entry: the modern double horn of `horn60_lowpoly`).
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(HornVoice))
}

/// `fighters.sound('HORN', M, k)`: its part on side `k`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(HornSound::new(k)))
}

/// Its model (`data/models/HORN.json`, written by its export), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/HORN.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// The extra line its part reads beside its own: STOPPED HORN's `HORN THEME`.
pub const EXTRA_LINES: &[&str] = &["HORN THEME"];
