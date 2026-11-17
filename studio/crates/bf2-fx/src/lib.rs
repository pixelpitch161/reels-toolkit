//! Engine 2's effects toolkit, built on `bf2-render` (Engine 2 design,
//! `docs/superpowers/specs/2026-11-17-engine2-design.md` section 3).
//!
//! GPU particles (100k+ live, seeded), ribbons and trails on weapon bones, mesh effects, decals, screen effects
//! (motion blur, radial blur, shockwaves, bloom, flashes, speed lines, grading), emissive effect lights and the hit
//! presets. Every effect has an exit: nothing just disappears. Milestone E3 builds it.

pub mod decal;
pub mod frame;
pub mod hash;
pub mod lights;
pub mod meshfx;
pub mod particles;
pub(crate) mod pipe;
pub mod presets;
pub mod record;
pub mod screen;
#[cfg(test)]
mod testkit;
pub mod timeline;
pub mod trail;
pub mod wire;

pub use record::{Anchor, Env, Exit, FxError, FxKind, FxRecord, envelope};
pub use timeline::FxTimeline;
