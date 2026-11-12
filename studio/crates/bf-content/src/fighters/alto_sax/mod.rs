//! ALTO SAX: its look, instrument voice, part in the score and card (`kit/fighters/alto_sax.py`), ported in
//! milestone 3's Task 15. The app offers it once a register task adds it to the registry's `PORTED`.
//!
//! Nothing outside this folder names its types.

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
pub use look::AltoSaxLook;
pub use sound::AltoSaxSound;
pub use voice::AltoSaxVoice;

/// `fighters.look('ALTO SAX', P, k)`.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(AltoSaxLook::default()))
}

/// Its instrument voice (`pp_bridge`'s entry: the rebuilt alto of `altosax_lowpoly`).
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(AltoSaxVoice))
}

/// `fighters.sound('ALTO SAX', M, k)`: its part on side `k`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(AltoSaxSound::new(k)))
}

/// Its model (`data/models/ALTO SAX.json`, written by its export), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/ALTO SAX.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
