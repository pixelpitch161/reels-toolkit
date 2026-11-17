//! Engine 2's renderer: wgpu on the GPU, full 1080 x 1920 in HDR (Engine 2 design,
//! `docs/superpowers/specs/2026-11-17-engine2-design.md` section 3).
//!
//! The scene (ring, arena, crowd, the instruments in their glass balls), toon-banded and glass materials, many
//! dynamic lights and shadows, the post stack (tone map, grade), offscreen frames read back for ffmpeg, and the same
//! scene drawn into the app's preview. Milestone E2 builds it.

pub mod error;
pub mod gpu;

pub use error::RenderError;
pub use gpu::Gpu;
pub mod post;
pub mod target;

pub use post::{Post, PostPass};
pub use target::{H, Targets, W, read_back};
