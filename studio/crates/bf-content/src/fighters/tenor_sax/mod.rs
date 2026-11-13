//! TENOR SAX: its look, instrument voice, part in the score and card, ported in milestone 3's Task 14
//! (`kit/fighters/tenor_sax.py`). The app offers it once the registry's `PORTED` names it (only a register task
//! changes that).
//!
//! This folder holds `look.rs`, `sound.rs`, `voice.rs`, `card.rs`, `state.rs` (what the look and the part both derive
//! from the recording) and the entry points below. Nothing outside it names TENOR SAX's types.

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
pub use look::TenorSaxLook;
pub use sound::TenorSaxSound;
pub use voice::TenorSaxVoice;

/// `fighters.look('TENOR SAX', P, k)`.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(TenorSaxLook::default()))
}

/// Its instrument voice (`pp_bridge`'s entry).
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(TenorSaxVoice))
}

/// `fighters.sound('TENOR SAX', M, k)`: its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(TenorSaxSound::new(k)))
}

/// Its model (`data/models/TENOR SAX.json`, written by its export), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/TENOR SAX.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
