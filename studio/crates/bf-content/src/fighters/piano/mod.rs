//! PIANO: its look, instrument voice, part in the score and card (`kit/fighters/piano.py`), ported in milestone 3's
//! Task 7. The app does not offer it until a register task adds it to the registry's `PORTED`.
//!
//! PIANO is the one fighter whose music feeds its fight: the rows of its roll are its line (`kit/story.py`
//! `roll_rows`), so a change of key, chord loop or line changes what it fires ([`MUSIC_FEEDS_FIGHT`]).

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
pub use look::PianoLook;
pub use sound::PianoSound;
pub use voice::PianoVoice;

/// `fighters.look('PIANO', P, k)`.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(PianoLook::new()))
}

/// Its instrument voice (`pp_bridge`'s entry: the modern grand of `piano_lowpoly`).
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(PianoVoice))
}

/// `fighters.sound('PIANO', M, k)`: its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(PianoSound::new(k)))
}

/// Its model (`data/models/PIANO.json`, written by its export), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/PIANO.json"));

/// Its music settings (key, chord loop, its line) feed its fight: the roll's rows are its line.
pub const MUSIC_FEEDS_FIGHT: bool = true;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
