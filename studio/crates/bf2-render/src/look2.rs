//! How content draws a fighter: the [`Look2`] trait, and the frame of the fight it draws from.

use crate::gpu::Gpu;
use crate::mesh::MeshId;
use crate::renderer::Renderer;
use crate::scene::Scene;

/// One video frame of the fight, as the stage and the looks read it.
///
/// PLACEHOLDER: this is the minimal shape E2 Task 5 draws from. Task 6 replaces it with the interpolated frame of
/// a `bf2_sim::Recording2` (`FightFrame<'a> { rec, t_video, t_fight, step, alpha, snap }`); the stage and the looks
/// then read the same facts from `snap`. Positions are sim space (x, y the floor, z height; z = 0 is a ball
/// resting on the canvas).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FightFrame {
    /// Seconds of video.
    pub t_video: f64,
    /// Each ball's position (its bottom touches the canvas at z = 0).
    pub pos: [[f64; 3]; 2],
    /// Each ball's facing, radians in the floor plane.
    pub facing: [f64; 2],
    pub hp: [f64; 2],
    /// Live projectiles: id and position.
    pub projs: Vec<(u32, [f64; 3])>,
}

/// A fighter's look: its meshes, made once, and what it adds to each frame.
pub trait Look2: Send + Sync {
    /// Makes the look's meshes (once, before the first frame).
    fn setup(&mut self, r: &mut Renderer, gpu: &Gpu);

    /// Adds fighter `k`'s weapon, its instrument (inside the glass) and any lights to `out`. Returns the ids of the
    /// projectiles it drew; the stage draws a default glowing sphere for every other one.
    fn draw(&self, f: &FightFrame, k: usize, out: &mut Scene) -> Vec<u32>;

    /// The instrument's mesh, if the look has one.
    fn instrument(&self) -> Option<MeshId> {
        None
    }
}
