//! Engine 2's fight engine: headless, on the CPU, deterministic (Engine 2 design,
//! `docs/superpowers/specs/2026-11-17-engine2-design.md` section 2).
//!
//! The ring floor plus a height axis; each fighter a ball with an articulated weapon rig; moves in phases (wind-up,
//! strike, recovery) with hitboxes on the rig's bones; hit results with freeze-on-hit, knockback, launches, juggles,
//! slams and bounces; behaviour profiles that pick moves from the seeded generator; a recording of every step and
//! the typed events v2 that the director, effects, sound, captions and HUD read. Milestone E1 builds it.

#![forbid(unsafe_code)]

pub mod body;
pub mod brain;
pub mod event;
pub mod fighter;
pub mod hits;
pub mod moves;
pub mod physics;
pub mod proj;
pub mod record;
pub mod rig;
pub mod rules;
pub mod run;
#[cfg(test)]
mod testkit;
pub mod v3;
pub mod world;

pub use body::Body;
pub use brain::{Brain, Choice, think};
pub use event::{Ev, EvKind, Events, Surface};
pub use fighter::{FighterDef, Setup2};
pub use moves::{Cue, CueAt, Ease, Grab, HitDef, Key, MoveDef, MoveState, Phase, ProjDef, Spawn, Tag, ease};
pub use proj::Proj;
pub use record::{Outcome, Recording2, StepSnap};
pub use rig::{Bone, Joint, Pose, Rig, Segment, solve};
pub use rules::{Rules2, SimError2};
pub use run::run;
pub use v3::V3;
pub use world::{World, step};
