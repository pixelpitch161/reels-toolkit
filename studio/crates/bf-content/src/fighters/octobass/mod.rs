//! OCTOBASS: a Studio fighter (no Python kit reference; spec `docs/superpowers/specs/2026-11-21-octobass-fighter-design.md`).
//! Its weapon LEVER PUNCH punches with the octobass's string levers, each felt fist stopping dead on the enemy; FOOT
//! PEDAL, SEVEN LEVERS and BELOW HEARING are its moves (`bf_sim::kinds::levers`).
//!
//! `look.rs` is its `Look` (the model as the ball, the lever arms, the pedal plate, the dark and the slow ring of
//! BELOW HEARING, the pieces), `sound.rs` its part in the score, `voice.rs` its string voice, `card.rs` its card,
//! `state.rs` what the look and the part both read from the recording, `personality.rs` its musical personality.
//! Its model was built from web photos of real octobasses (`studio/reference/models/octobass/`).

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
pub use look::OctobassLook;
pub use sound::OctobassSound;
pub use voice::OctobassVoice;

/// Its look.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(OctobassLook::default()))
}

/// Its instrument voice.
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(OctobassVoice))
}

/// Its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(OctobassSound::new(k)))
}

/// Its model (`data/models/OCTOBASS.json`, made by `studio/scripts/models/octobass_model.py`), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/OCTOBASS.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
