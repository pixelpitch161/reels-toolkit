//! TROMBONE: the fighter's instrument voice, its part in the score and its look.

use std::sync::Arc;

use bf_core::fightfile::FightFile;
use bf_picture::look::Look;
use bf_sound::traits::Sound;
use bf_sound::voice::Voice;

use crate::cards::FighterCard;

pub mod look;
pub mod sound;
pub mod voice;

pub use look::TromboneLook;
pub use sound::TromboneSound;
pub use voice::TromboneVoice;

/// `fighters.look('TROMBONE', P, k)`.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(TromboneLook::default()))
}

/// Its instrument voice.
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(TromboneVoice))
}

/// `fighters.sound('TROMBONE', M, k)`: its role (`T.ROLES.get(name, 'melody')`) sets the band it is checked in.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let role = ff.music.roles.get("TROMBONE").map_or("melody", String::as_str);
    Some(Box::new(TromboneSound::new(k, role)))
}

/// Its card (written with milestone 2's interface, in `cards.rs`).
pub fn card() -> Option<&'static FighterCard> {
    Some(&crate::cards::TROMBONE)
}

/// Its model, compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/TROMBONE.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
