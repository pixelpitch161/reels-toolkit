//! ACCORDION: one of the older fighters (milestone 4), quick-ported (ruling M4-R16) from its episode
//! `projects/ballfight_bagpipes_vs_accordion/` (#30): BELLOWS BITE's jaws, FULL BELLOWS's clamp across the ring,
//! SQUEEZEBOX's collar, and its reed chords from its own synth. The app offers it once the registry's `PORTED` lists
//! it (a register task's change).
//!
//! This folder holds `look.rs`, `sound.rs`, `voice.rs`, `card.rs`, `state.rs` and the entry points below. Nothing
//! outside it names ACCORDION's types. Its music settings do not feed its fight (`MUSIC_FEEDS_FIGHT` stays false: none of
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

/// `fighters.look('ACCORDION', P, k)`: its look, or `None` while it is not ported (the bare ball is drawn then).
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(look::AccordionLook::default()))
}

/// Its instrument voice (its `pp_bridge` entry, or its episode's own synth), or `None` while it is not ported.
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(voice::AccordionVoice))
}

/// `fighters.sound('ACCORDION', M, k)`: its part on side `k` of the fight `ff`, or `None` while it is not ported.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(sound::AccordionSound::new(k)))
}

/// Its model (`data/models/ACCORDION.json`, written by the export tool's `models` for one roster name), compiled in:
/// `Some(include_str!("../../../data/models/ACCORDION.json"))` once ported, `None` until then.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/ACCORDION.json"));

/// Whether its music settings (key, chord loop, its line) feed its fight: false for every older fighter.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// The extra lines its part declares beside its own (HORN: `HORN THEME`).
pub const EXTRA_LINES: &[&str] = &[];

/// Its musical personality (milestone 5).
pub mod personality;
