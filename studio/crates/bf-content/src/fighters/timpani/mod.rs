//! TIMPANI: a Studio fighter (no Python kit reference; spec `docs/superpowers/specs/2026-11-16-timpani-fighter-design.md`).
//! Its weapon TWO KETTLES sets a low and a high kettle in the ring and hops between them, each landing booming that
//! kettle; PEDAL SLIDE, CROSS-STICKING and FULL SET are its moves (`bf_sim::kinds::kettles`).
//!
//! `look.rs` is its `Look` (the model as the ball, the kettles on the floor, their rings, the hops, the slides),
//! `sound.rs` its part in the score, `voice.rs` its timpani voice, `card.rs` its card, `state.rs` what the look and the
//! part both read from the recording, `personality.rs` its musical personality. Its model was built from web photos
//! of real concert pedal timpani (`studio/reference/models/timpani/`).

use std::sync::Arc;

use bf_core::fightfile::FightFile;
use bf_picture::look::Look;
use bf_sound::traits::Sound;
use bf_sound::voice::Voice;

pub mod card;
pub mod look;
pub mod personality;
pub mod sound;
pub mod state;
pub mod voice;

pub use card::{CARD, card};
pub use look::TimpaniLook;
pub use sound::TimpaniSound;
pub use voice::TimpaniVoice;

/// Its look.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(TimpaniLook::default()))
}

/// Its instrument voice.
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(TimpaniVoice))
}

/// Its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(TimpaniSound::new(k)))
}

/// Its model (`data/models/TIMPANI.json`, made by `studio/scripts/models/timpani_model.py`), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/TIMPANI.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
