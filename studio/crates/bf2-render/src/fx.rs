//! The effects hook: after the glass, an [`FxPass`] draws into the frame's multisampled HDR colour, depth tested
//! against the scene (read-only), before the resolve. `bf2-fx` implements it.

use crate::gpu::Gpu;
use crate::maths::{self, M4, V3};
use crate::scene::{Camera, NEAR, Scene};

/// What an effect pass draws with, per frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FxCamera {
    pub view_proj: M4,
    pub eye: V3,
    /// The camera's right, up and forward in render space.
    pub right: V3,
    pub up: V3,
    pub forward: V3,
    /// The vertical field of view, radians.
    pub fov_y: f32,
    /// The target's width and height in pixels.
    pub size: [u32; 2],
    pub near: f32,
    /// Output pixels per art pixel when the pixel pass is on ([`crate::post::Post::pixel`]), else 1: effects with
    /// a pixel look draw hard shapes and dither their fades on this grid ([`FxCamera::pixel_look`]).
    pub art: f32,
}

impl FxCamera {
    pub fn of(c: &Camera, w: u32, h: u32) -> FxCamera {
        let view = c.view();
        // The view matrix's rows are the camera axes (forward is −Z).
        let row = |r: usize| [view[0][r], view[1][r], view[2][r]];
        FxCamera {
            view_proj: c.view_proj(w as f32 / h as f32),
            eye: c.eye,
            right: row(0),
            up: row(1),
            forward: maths::scale(row(2), -1.0),
            fov_y: c.fov_deg.to_radians(),
            size: [w, h],
            near: NEAR,
            art: 1.0,
        }
    }

    /// This camera with the art pixel size of `post` at its target's width (as the tone map counts it).
    pub fn with_art(mut self, post: &crate::post::Post) -> FxCamera {
        self.art = crate::post::art_scale(post, self.size[0]);
        self
    }

    /// Whether the pixel pass is on, so effects draw their pixel look.
    pub fn pixel_look(&self) -> bool {
        self.art > 1.0
    }

    /// World units per pixel at distance 1 from the eye.
    pub fn pixel_per_distance(&self) -> f32 {
        2.0 * (self.fov_y / 2.0).tan() / self.size[1] as f32
    }
}

/// An effect pass's view of the frame: record into `encoder`, through [`FxCtx::begin_pass`].
pub struct FxCtx<'a> {
    pub gpu: &'a Gpu,
    pub encoder: &'a mut wgpu::CommandEncoder,
    /// The multisampled HDR colour ([`crate::target::HDR_FORMAT`], [`crate::target::SAMPLES`] samples).
    pub colour: &'a wgpu::TextureView,
    /// The multisampled depth ([`crate::target::DEPTH_FORMAT`]); passes test against it, never write it.
    pub depth: &'a wgpu::TextureView,
    pub camera: FxCamera,
    pub scene: &'a Scene,
    /// Which sub-frame of an accumulated frame this is (0 for a plain frame).
    pub sub: usize,
}

impl FxCtx<'_> {
    /// A render pass that keeps what the scene drew and tests against its depth, read-only.
    pub fn begin_pass(&mut self, label: &str) -> wgpu::RenderPass<'_> {
        self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self.colour,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.depth,
                depth_ops: None,
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        })
    }
}

/// Something drawn after the scene, into the same HDR frame.
pub trait FxPass {
    fn draw(&mut self, ctx: &mut FxCtx<'_>);
}

/// No effects.
pub struct NoFx;

impl FxPass for NoFx {
    fn draw(&mut self, _ctx: &mut FxCtx<'_>) {}
}

/// What a post chain reads and writes: the frame's HDR colour in, the HDR colour the tone map reads out.
pub struct PostCtx<'a> {
    pub gpu: &'a Gpu,
    pub encoder: &'a mut wgpu::CommandEncoder,
    /// Single-sample HDR, bindable as a texture ([`crate::target::HDR_FORMAT`]).
    pub input: &'a wgpu::TextureView,
    /// Single-sample HDR render target of the same size; the chain must cover every pixel.
    pub output: &'a wgpu::TextureView,
    pub camera: FxCamera,
}

/// Screen effects between the scene and the tone map, in HDR.
pub trait PostChain {
    fn run(&mut self, ctx: &mut PostCtx<'_>);
}
