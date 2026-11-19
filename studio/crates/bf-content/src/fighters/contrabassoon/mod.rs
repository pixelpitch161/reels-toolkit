//! CONTRABASSOON: the first fighter of the PLUMBING kind, made from scratch (Nov 16, 2026,
//! `docs/studio/2026-11-16-contrabassoon.md`). No kit module stands behind it: Rust is the source.
//!
//! Nothing outside this folder names CONTRABASSOON's types, so nothing outside it changes.

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

/// Its look: the folded tube in the ball and its pipes in the ring.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(look::ContrabassoonLook::default()))
}

/// Its instrument voice: the double reed, an octave below the BASSOON's, with a rattle.
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(voice::ContrabassoonVoice))
}

/// Its part on side `k`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(sound::ContrabassoonSound::new(k)))
}

/// Its model (`data/models/CONTRABASSOON.json`, made from photos by `tools/studio_export/contrabassoon_model.py`).
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/CONTRABASSOON.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];

/// Its musical personality.
pub mod personality;
