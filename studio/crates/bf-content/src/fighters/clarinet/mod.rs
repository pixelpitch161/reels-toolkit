//! CLARINET: its look, instrument voice, part in the score and card (`kit/fighters/clarinet.py`, ported in milestone
//! 3's Task 10). The app offers it once a register task adds it to the registry's `PORTED`.
//!
//! `look.rs` is its `Look`, `sound.rs` its `Sound`, `voice.rs` its Pixel Pitch voice and the reed synth its part plays
//! directly, `card.rs` its card; `state.rs` holds nothing (its look and its part each read the run's `jump` and `stun`
//! events themselves, and neither reads the other).

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

pub use look::ClarinetLook;
pub use sound::ClarinetSound;
pub use voice::ClarinetVoice;

/// `fighters.look('CLARINET', P, k)`: its look.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(ClarinetLook::default()))
}

/// Its instrument voice (`pp_bridge`'s entry).
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(ClarinetVoice))
}

/// `fighters.sound('CLARINET', M, k)`: its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(ClarinetSound::new(k)))
}

/// Its model (`data/models/CLARINET.json`, written by its export), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/CLARINET.json"));

/// Whether its music settings (key, chord loop, its line) feed its fight. True only for PIANO, once ported.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// The extra lines its part declares beside its own (HORN: `HORN THEME`).
pub const EXTRA_LINES: &[&str] = &[];
