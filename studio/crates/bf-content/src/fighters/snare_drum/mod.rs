//! SNARE DRUM: a Studio fighter (no Python kit reference; spec `docs/superpowers/specs/2026-11-16-snare-drum-fighter-design.md`).
//! Its weapon PARADIDDLE jabs two drumsticks at the enemy on a paradiddle sticking; FLAM, RIMSHOT and BUZZ ROLL are
//! its moves (`bf_sim::kinds::sticks`).
//!
//! `look.rs` is its `Look` (the model as the ball, the sticks' jabs, the flams, the rimshot crack, the buzz disc, the
//! shards), `sound.rs` its part in the score, `voice.rs` its snare voice, `card.rs` its card, `state.rs` what the
//! look and the part both read from the recording, `personality.rs` its musical personality. Its model was built from
//! web photos of real snare drums (`studio/reference/models/snare_drum/`).

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
pub use look::SnareDrumLook;
pub use sound::SnareDrumSound;
pub use voice::SnareDrumVoice;

/// Its look.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(SnareDrumLook::default()))
}

/// Its instrument voice.
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(SnareDrumVoice))
}

/// Its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(SnareDrumSound::new(k)))
}

/// Its model (`data/models/SNARE DRUM.json`, made by `studio/scripts/models/snare_drum_model.py`), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/SNARE DRUM.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
