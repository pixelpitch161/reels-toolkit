//! TARGET DUMMY: a developer fighter, never offered in the app (it is not in `PORTED` and has no place in the score
//! order), for example renders of another fighter's moves. A fight file names it like any fighter
//! (`fights/dev/trumpet_vs_target_dummy.toml`). It has no weapon and no abilities (kind `dummy`,
//! [`bf_core::roster::DUMMY`]): its ball drifts and bounces round the ring and never attacks, and its enemy can still
//! knock it out, so its fights end and render. Engine 2's dummy was the idea; this one is drawn the Engine 1 way.
//!
//! `look.rs` is its `Look` (the model as the ball, a bullseye flash where it is hit, its shatter of straw, burlap and
//! wood), `sound.rs` its part (a woody knock on every hit it takes), `card.rs` its card, `personality.rs` its musical
//! personality. It plays MARIMBA's voice. Its model is a straw training dummy
//! (`studio/scripts/models/target_dummy_model.py`).

use std::sync::Arc;

use bf_core::fightfile::FightFile;
use bf_picture::look::Look;
use bf_sound::traits::Sound;
use bf_sound::voice::Voice;

pub mod card;
pub mod look;
pub mod personality;
pub mod sound;

pub use card::{CARD, card};
pub use look::TargetDummyLook;
pub use sound::TargetDummySound;

/// Its look.
pub fn look() -> Option<Box<dyn Look>> {
    Some(Box::new(TargetDummyLook::default()))
}

/// Its instrument voice: MARIMBA's (the roster's `pp`).
pub fn voice() -> Option<Arc<dyn Voice>> {
    crate::fighters::marimba::voice()
}

/// Its part on side `k` of the fight `ff`.
pub fn sound(k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    Some(Box::new(TargetDummySound::new(k)))
}

/// Its model (`data/models/TARGET DUMMY.json`, made by `studio/scripts/models/target_dummy_model.py`), compiled in.
pub const MESH: Option<&str> = Some(include_str!("../../../data/models/TARGET DUMMY.json"));

/// Its music does not feed its fight.
pub const MUSIC_FEEDS_FIGHT: bool = false;

/// No extra lines.
pub const EXTRA_LINES: &[&str] = &[];
