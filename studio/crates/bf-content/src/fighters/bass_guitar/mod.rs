//! BASS GUITAR: a Studio fighter (no Python kit reference; spec
//! `docs/superpowers/specs/2026-11-21-bass-guitar-fighter-design.md`). The rock bassist with a pick: its weapon PICK
//! TOSS throws big picks on a looping curve that come back to it; FRET SLIDE, HEADSTOCK SWING and SLINGSHOT are its
//! moves (`bf_sim::kinds::pick`).
//!
//! `look.rs` is its `Look` (the model as the ball, the picks and their trails, the fret marks of a slide, the neck of a
//! swing, the slingshot's strings), `sound.rs` its part in the score, `voice.rs` its picked electric bass, `card.rs`
//! its card, `state.rs` what the look and the part both read from the recording, `personality.rs` its musical
//! personality. Its model was built from web photos of real electric basses (`studio/reference/models/bass_guitar/`).

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
pub use look::BassGuitarLook;
pub use sound::BassGuitarSound;
pub use voice::BassGuitarVoice;

/// Its look.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(BassGuitarLook::default()))
}

/// Its instrument voice.
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(BassGuitarVoice))
}

/// Its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(BassGuitarSound::new(k)))
}

/// Its model (`data/models/BASS GUITAR.json`, made by `studio/scripts/models/bass_guitar_model.py`), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/BASS GUITAR.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
