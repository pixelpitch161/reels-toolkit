//! MARIMBA: a Studio fighter (no Python kit reference; spec `docs/superpowers/specs/2026-11-16-marimba-fighter-design.md`).
//! Its weapon MALLET RUN throws ricochet mallets that climb the scale with every wall bounce; TREMOLO, DEAD STROKE and
//! FIVE OCTAVES are its moves (`bf_sim::kinds::mallets`).
//!
//! `look.rs` is its `Look` (the model as the ball, the mallets in flight, the bounces, the pin, the shards),
//! `sound.rs` its part in the score, `voice.rs` its marimba voice, `card.rs` its card, `state.rs` what the look and the
//! part both read from the recording, `personality.rs` its musical personality. Its model was built from web photos
//! of real concert marimbas (`studio/reference/models/marimba/`).

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
pub use look::MarimbaLook;
pub use sound::MarimbaSound;
pub use voice::MarimbaVoice;

/// Its look.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(MarimbaLook::default()))
}

/// Its instrument voice.
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(MarimbaVoice))
}

/// Its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(MarimbaSound::new(k)))
}

/// Its model (`data/models/MARIMBA.json`, made by `studio/scripts/models/marimba_model.py`), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/MARIMBA.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
