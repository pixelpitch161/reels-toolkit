//! How content draws a fighter: the [`Look2`] trait.

pub use crate::fight_frame::FightFrame;
use crate::gpu::Gpu;
use crate::mesh::MeshId;
use crate::renderer::Renderer;
use crate::scene::Scene;

/// A fighter's look: its meshes, made once, and what it adds to each frame.
pub trait Look2: Send + Sync {
    /// Makes the look's meshes (once, before the first frame).
    fn setup(&mut self, r: &mut Renderer, gpu: &Gpu);

    /// Adds fighter `k`'s weapon, its instrument (inside the glass) and any lights to `out`. Returns the ids of the
    /// projectiles it drew; the stage draws a default glowing sphere for every other one.
    fn draw(&self, f: &FightFrame<'_>, k: usize, out: &mut Scene) -> Vec<u32>;

    /// The instrument's mesh, if the look has one.
    fn instrument(&self) -> Option<MeshId> {
        None
    }
}
