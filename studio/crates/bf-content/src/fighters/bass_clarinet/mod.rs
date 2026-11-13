//! BASS CLARINET: its look, instrument voice, part in the score and card (`kit/fighters/bass_clarinet.py`, ported in
//! milestone 3's Task 16). The app offers it once a register task adds it to the registry's `PORTED`.
//!
//! `look.rs` is its `Look`, `sound.rs` its `Sound`, `voice.rs` its Pixel Pitch voice and the reed synth its part plays
//! directly, `card.rs` its card; `state.rs` holds its pegs (`ANCH`), which its look and its part both derive from the
//! recording.

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

pub use look::BassClarinetLook;
pub use sound::BassClarinetSound;
pub use voice::BassClarinetVoice;

/// `fighters.look('BASS CLARINET', P, k)`: its look.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(BassClarinetLook::default()))
}

/// Its instrument voice (`pp_bridge`'s entry).
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(BassClarinetVoice))
}

/// `fighters.sound('BASS CLARINET', M, k)`: its part on side `k` of the fight `ff`: as the bass when the fight file
/// gives it the role `bass` (`T.ROLES.get(name, 'melody')`), an octave up otherwise.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let bass = ff
        .fighters
        .get(k)
        .and_then(|name| ff.music.roles.get(name))
        .is_some_and(|role| role == "bass");
    Some(Box::new(BassClarinetSound::new(k, bass)))
}

/// Its model (`data/models/BASS CLARINET.json`, written by its export), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/BASS CLARINET.json"));

/// Whether its music settings (key, chord loop, its line) feed its fight. True only for PIANO, once ported.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// The extra lines its part declares beside its own (HORN: `HORN THEME`).
pub const EXTRA_LINES: &[&str] = &[];
