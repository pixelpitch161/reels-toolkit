//! EUPHONIUM: its look, instrument voice, part in the score and card (`kit/fighters/euphonium.py`, ported in milestone
//! 3's Task 26). The app offers it once a register task adds it to the registry's `PORTED`.
//!
//! `look.rs` is its `Look` (with IDENTITY CRISIS's copy of the enemy's look), `sound.rs` its `Sound`, `voice.rs` its
//! Pixel Pitch voice and the synth its part plays directly, `card.rs` its card; `state.rs` holds its salvos and
//! landings, which its look and its part both derive from the recording.

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

pub use look::EuphoniumLook;
pub use sound::EuphoniumSound;
pub use voice::EuphoniumVoice;

/// `fighters.look('EUPHONIUM', P, k)`: its look.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(EuphoniumLook::default()))
}

/// Its instrument voice (`pp_bridge`'s entry).
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(EuphoniumVoice))
}

/// `fighters.sound('EUPHONIUM', M, k)`: its part on side `k` of the fight `ff` (the melody whatever its role: the
/// kit's part never reads it).
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(EuphoniumSound::new(k)))
}

/// Its model (`data/models/EUPHONIUM.json`, written by its export), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/EUPHONIUM.json"));

/// Whether its music settings (key, chord loop, its line) feed its fight. True only for PIANO, once ported.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// The extra lines its part declares beside its own (HORN: `HORN THEME`).
pub const EXTRA_LINES: &[&str] = &[];
