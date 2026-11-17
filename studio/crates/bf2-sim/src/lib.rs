//! Engine 2's fight engine: headless, on the CPU, deterministic (Engine 2 design,
//! `docs/superpowers/specs/2026-11-17-engine2-design.md` section 2).
//!
//! The ring floor plus a height axis; each fighter a ball with an articulated weapon rig; moves in phases (wind-up,
//! strike, recovery) with hitboxes on the rig's bones; hit results with freeze-on-hit, knockback, launches, juggles,
//! slams and bounces; behaviour profiles that pick moves from the seeded generator; a recording of every step and
//! the typed events v2 that the director, effects, sound, captions and HUD read. Milestone E1 builds it.

#![forbid(unsafe_code)]

pub mod rules;
pub mod v3;

pub use rules::{Rules2, SimError2};
pub use v3::V3;
