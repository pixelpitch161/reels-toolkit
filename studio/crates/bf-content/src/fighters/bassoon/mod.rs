//! BASSOON: its look, instrument voice, part in the score and card (`kit/fighters/bassoon.py`, ported in milestone
//! 3's Task 19). The app does not offer it until a register task adds it to the registry's `PORTED`.
//!
//! Nothing outside this folder names BASSOON's types, so nothing outside it changes.

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
pub use look::BassoonLook;
pub use sound::BassoonSound;
pub use voice::BassoonVoice;

/// `fighters.look('BASSOON', P, k)`.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(BassoonLook::default()))
}

/// Its instrument voice (`pp_bridge`'s entry: the double reed of `bassoon_lowpoly`).
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(BassoonVoice))
}

/// `fighters.sound('BASSOON', M, k)`: its part on side `k`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(BassoonSound::new(k)))
}

/// Its model (`data/models/BASSOON.json`, written by its export), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/BASSOON.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
