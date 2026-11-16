//! CONTRABASS CLARINET: the first fighter made in the Studio (design `docs/studio/2026-11-16-contrabass-clarinet-
//! design.md`). No kit module or episode is behind it: its roster entry is `data/roster_new.json`, its model is
//! made by `tools/studio_export/new_models/contrabass_clarinet.py`, and its look, voice, part and motifs are its own.
//!
//! Its weapon is the FOGHORN (weapon kind `foghorn`, `bf-sim/src/kinds/foghorn.rs`); its abilities reuse the
//! engine's `shield` (FOG BANK), `haste` (MOTORBOAT) and `beam` (SHIP'S HORN).

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

/// Its look.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(look::ContraLook::default()))
}

/// Its instrument voice.
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(voice::ContraVoice))
}

/// Its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(sound::ContraSound::new(k)))
}

/// Its model, compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/CONTRABASS CLARINET.json"));

/// Its music settings do not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// The extra lines its part declares beside its own: none.
pub const EXTRA_LINES: &[&str] = &[];

/// Its musical personality.
pub mod personality;
