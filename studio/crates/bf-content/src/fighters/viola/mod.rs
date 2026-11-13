//! VIOLA: one of the older fighters (milestone 4), ported in its Task 11 against its reference kit module
//! (`tools/studio_export/older/fighters/viola.py`, the reference kit module moved from
//! `projects/ballfight_viola_vs_violin/` (#42)): its look, voice, part and card. The app does not offer it until a
//! register task adds it to the registry's `PORTED`.
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
    Some(look::look())
}

/// Its instrument voice (`pp_bridge`'s entry: `viola_lowpoly`'s bowed viola).
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(voice::ViolaVoice))
}

/// `fighters.sound('VIOLA', M, k)`: its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(sound::ViolaSound::new(k)))
}

/// Its model (`data/models/VIOLA.json`, written by the export tool's `models` for one roster name), compiled in:
/// `Some(include_str!("../../../data/models/VIOLA.json"))` once ported, `None` until then.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/VIOLA.json"));

/// Whether its music settings (key, chord loop, its line) feed its fight: false for every older fighter.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// The extra lines its part declares beside its own (HORN: `HORN THEME`).
pub const EXTRA_LINES: &[&str] = &[];

/// The voices its part plays besides its own (`M.note('CELLO', ...)`: the crush's cello, ruling M4-R10). The kit's
/// `M.note` reaches every voice of the series; the Studio's music context knows only the fighters' and the theme's,
/// so a soundtrack with the VIOLA needs the CELLO's voice added to it (a shared change, reported NEEDS_CONTEXT).
pub const PART_VOICES: &[&str] = &["CELLO"];
