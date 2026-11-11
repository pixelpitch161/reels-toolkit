//! OBOE: its look, instrument voice, part in the score and card, ported in milestone 3's Task 11
//! (`kit/fighters/oboe.py`). Until then every entry point here says it has none: the picture draws OBOE as the
//! bare ball, the score has no part for it, the app has no card for it, and the app does not offer it (the
//! registry's `PORTED`, which only a register task changes).
//!
//! The port fills this folder only: `look.rs`, `sound.rs`, `voice.rs`, `card.rs`, `state.rs` (what the look and
//! the part both derive from the recording) and the entry points below. Nothing outside it names OBOE's
//! types, so nothing outside it changes.

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

/// `fighters.look('OBOE', P, k)`: its look, or `None` while it is not ported (the bare ball is drawn then).
pub fn look() -> Option<Box<dyn Look>> {
    None
}

/// Its instrument voice (`pp_bridge`'s entry), or `None` while it is not ported.
pub fn voice() -> Option<Arc<dyn Voice>> {
    None
}

/// `fighters.sound('OBOE', M, k)`: its part on side `k` of the fight `ff`, or `None` while it is not ported.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = (k, ff);
    None
}

/// Its model (`data/models/OBOE.json`, written by the export tool's `models` for one roster name), compiled in:
/// `Some(include_str!("../../../data/models/OBOE.json"))` once ported, `None` until then.
pub const MESH: Option<&str> = None;

/// Whether its music settings (key, chord loop, its line) feed its fight. True only for PIANO, once ported.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// The extra lines its part declares beside its own (HORN: `HORN THEME`).
pub const EXTRA_LINES: &[&str] = &[];
