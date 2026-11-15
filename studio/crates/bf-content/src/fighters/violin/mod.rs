//! VIOLIN: one of the older fighters (milestone 4), a quick port (ruling M4-R16) of #42 (`projects/ballfight_viola_vs_violin/`, as #28 drew it): its look, voice, part and card. The app offers it once the registry's `PORTED` names it.
//!
//! Nothing outside this folder names VIOLIN's types. Its music settings do not feed its fight (`MUSIC_FEEDS_FIGHT` stays false: none of
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

/// `fighters.look('VIOLIN', P, k)`.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(look::ViolinLook::default()))
}

/// Its instrument voice (its `pp_bridge` entry).
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(voice::ViolinVoice))
}

/// `fighters.sound('VIOLIN', M, k)`: its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(sound::ViolinSound::new(k)))
}

/// Its model (`data/models/VIOLIN.json`, written by the export tool's `models` for one roster name), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/VIOLIN.json"));

/// Whether its music settings (key, chord loop, its line) feed its fight: false for every older fighter.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// The extra lines its part declares beside its own (HORN: `HORN THEME`).
pub const EXTRA_LINES: &[&str] = &[];
