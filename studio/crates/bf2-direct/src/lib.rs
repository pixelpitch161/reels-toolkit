//! Engine 2's director: the time map and the camera (Engine 2 design,
//! `docs/superpowers/specs/2026-11-17-engine2-design.md` section 4).
//!
//! The one map from video time to fight time (hitstop, slow-mo ramps, the K.O. cinematic) that picture, sound,
//! music and HUD all read; the camera rig and its shot rules (framing in the safe zone, punch-ins, tilts, whip
//! pans, shake); the arena's reactions. Milestone E4 builds it.
//!
//! Everything is precomputed once per fight from the `Recording2`, so any frame can be drawn in any order, and it
//! is pure CPU on `bf_core::dmath`: the same recording gives the same tracks on every PC.

#![forbid(unsafe_code)]

pub mod camera;
pub mod cfg;
pub mod project;
pub mod shots;
#[cfg(test)]
mod testkit;
pub mod time;

pub use camera::CameraTrack;
pub use cfg::{DirectCfg, FPS, FRAMES};
pub use project::{CamPose, project, to_render};
pub use shots::{Priority, Shot, ShotKind};
pub use time::TimeMap;
