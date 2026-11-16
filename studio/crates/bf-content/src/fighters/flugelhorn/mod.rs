//! FLUGELHORN: a fighter made in the Studio (design `docs/studio/2026-11-16-flugelhorn-design.md`). No kit module
//! or episode is behind it: its roster entry is `data/roster_new.json`, its model is made from photos by
//! `studio/tools/models/flugelhorn_model.py`, and its look, voice, part and motifs are its own.
//!
//! Its weapon is the WINGBEAT (weapon kind `wingbeat`, `bf-sim/src/kinds/wingbeat.rs`, which also runs its
//! FLANKING CALL and TAKE WING); FEATHER SHOWER reuses the engine's `arc`.

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
    Some(Box::new(look::FlugelLook::default()))
}

/// Its instrument voice.
pub fn voice() -> Option<Arc<dyn Voice>> {
    Some(Arc::new(voice::FlugelVoice))
}

/// Its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(sound::FlugelSound::new(k)))
}

/// Its model, compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/FLUGELHORN.json"));

/// Its music settings do not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// The extra lines its part declares beside its own: none.
pub const EXTRA_LINES: &[&str] = &[];

/// Its musical personality.
pub mod personality;
