//! VIOLA: one of the older fighters (milestone 4), ported in its Task 11 against its reference kit module
//! (`tools/studio_export/older/fighters/viola.py`, the reference kit module moved from
//! `projects/ballfight_viola_vs_violin/` (#42)). Until then every entry point here says it has none: the picture
//! draws VIOLA as the bare ball, the score has no part for it, the app has no card for it, and the app does not
//! offer it (the registry's `PORTED`, which only a register task changes).
//!
//! The port fills this folder only: `look.rs`, `sound.rs`, `voice.rs`, `card.rs`, `state.rs` (what the look and the
//! part both derive from the recording) and the entry points below. Nothing outside it names VIOLA's types, so
//! nothing outside it changes. Its music settings do not feed its fight (`MUSIC_FEEDS_FIGHT` stays false: none of
//! the older ten reads the music, checked against `kit/story.py`).

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

/// `fighters.look('VIOLA', P, k)`: its look, or `None` while it is not ported (the bare ball is drawn then).
pub fn look() -> Option<Box<dyn Look>> {
    None
}

/// Its instrument voice (its `pp_bridge` entry, or its episode's own synth), or `None` while it is not ported.
pub fn voice() -> Option<Arc<dyn Voice>> {
    None
}

/// `fighters.sound('VIOLA', M, k)`: its part on side `k` of the fight `ff`, or `None` while it is not ported.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = (k, ff);
    None
}

/// Its model (`data/models/VIOLA.json`, written by the export tool's `models` for one roster name), compiled in:
/// `Some(include_str!("../../../data/models/VIOLA.json"))` once ported, `None` until then.
pub const MESH: Option<&str> = None;

/// Whether its music settings (key, chord loop, its line) feed its fight: false for every older fighter.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// The extra lines its part declares beside its own (HORN: `HORN THEME`).
pub const EXTRA_LINES: &[&str] = &[];
