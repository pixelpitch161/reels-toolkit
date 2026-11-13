//! FLUTE: the fighter's instrument voice, its part in the score and its look.

use std::sync::Arc;

use bf_core::fightfile::FightFile;
use bf_picture::look::Look;
use bf_sound::traits::Sound;
use bf_sound::voice::Voice;

use crate::cards::FighterCard;

pub mod look;
pub mod sound;
pub mod voice;

pub use look::{FluteLook, flute_keys, flute_sprite};
pub use sound::FluteSound;
pub use voice::FluteVoice;

/// `fighters.look('FLUTE', P, k)`.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(FluteLook::new()))
}

/// Its instrument voice.
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(FluteVoice))
}

/// `fighters.sound('FLUTE', M, k)`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(FluteSound::new(k)))
}

/// Its card (written with milestone 2's interface, in `cards.rs`).
pub fn card() -> Option<&'static FighterCard> {
    Some(&crate::cards::FLUTE)
}

/// Its model, compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/FLUTE.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
