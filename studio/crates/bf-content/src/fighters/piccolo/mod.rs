//! PICCOLO: a Studio fighter (no Python kit reference; spec `docs/superpowers/specs/2026-11-16-piccolo-fighter-design.md`).
//! Its weapon PIERCING NOTE fires very fast needles that ricochet off the ropes and pierce through the enemy; TRILL,
//! OVER THE BAND and GLASS SHATTER are its moves (`bf_sim::kinds::needle`).
//!
//! `look.rs` is its `Look` (the model as the ball, the needles' streaks, the bounces, the pierces, the shards),
//! `sound.rs` its part in the score, `voice.rs` its piccolo voice, `card.rs` its card, `state.rs` what the look and the
//! part both read from the recording, `personality.rs` its musical personality. Its model was built from web photos
//! of real piccolos (`studio/reference/models/piccolo/`).

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
pub use look::PiccoloLook;
pub use sound::PiccoloSound;
pub use voice::PiccoloVoice;

/// Its look.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(PiccoloLook::default()))
}

/// Its instrument voice.
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(PiccoloVoice))
}

/// Its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(PiccoloSound::new(k)))
}

/// Its model (`data/models/PICCOLO.json`, made by `studio/scripts/models/piccolo_model.py`), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/PICCOLO.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
