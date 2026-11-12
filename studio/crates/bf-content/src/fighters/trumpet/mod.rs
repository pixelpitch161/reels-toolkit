//! TRUMPET: its look, instrument voice, part in the score and card (`kit/fighters/trumpet.py`), ported in
//! milestone 3's Task 8. The app does not offer it until a register task adds it to the registry's `PORTED`.

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
pub use look::TrumpetLook;
pub use sound::TrumpetSound;
pub use voice::TrumpetVoice;

/// `fighters.look('TRUMPET', P, k)`.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(TrumpetLook::default()))
}

/// Its instrument voice (`pp_bridge`'s entry).
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(TrumpetVoice))
}

/// `fighters.sound('TRUMPET', M, k)`: its part on side `k`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(TrumpetSound::new(k)))
}

/// Its model (`data/models/TRUMPET.json`, written by the export tool), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/TRUMPET.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
