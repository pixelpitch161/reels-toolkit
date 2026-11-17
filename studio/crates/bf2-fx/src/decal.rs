//! Decals: marks the fight leaves on the canvas and the ropes (scorches, cracks, splats, glows). Procedural from
//! the record's seed, long-lived, and they fade (or cool through red, with `Exit::Ember`) before they go.

use bf2_render::stage::{POST, ROPE_HEIGHTS, ROPE_R, ROPE_SAG, SIDES};
use bf2_render::{FxCamera, FxCtx, Gpu, geom, to_render};

use crate::pipe::{self, Blend, Grow};
use crate::record::{FxKind, FxRecord};

/// What mark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DecalKind {
    /// A charred patch with a hot rim that cools.
    #[default]
    Scorch,
    /// A web of cracks, glowing hot at first.
    Crack,
    /// A blot of paint (its colour).
    Splat,
    /// Light on the surface, covering nothing.
    Glow,
}

/// Where a decal is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Surface {
    /// On the canvas, at the anchor's floor position.
    #[default]
    Canvas,
    /// Around the three ropes of a side (`bf2_render::stage::SIDES` order: 0 is sim +y, then +x, −y, −x),
    /// at the anchor's position along it.
    Rope(u8),
}

/// A mark on the canvas or the ropes.
#[derive(Clone, Debug, PartialEq)]
pub struct DecalDef {
    /// Units.
    pub r: f32,
    pub kind: DecalKind,
    /// The mark's colour (linear; a glow's is HDR light).
    pub colour: [f32; 3],
    pub on: Surface,
}

impl Default for DecalDef {
    fn default() -> Self {
        DecalDef {
            r: 0.9,
            kind: DecalKind::Scorch,
            colour: [0.02, 0.015, 0.012],
            on: Surface::Canvas,
        }
    }
}

/// A decal's default life, seconds.
pub const DECAL_LIFE: f32 = 6.0;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DecG {
    centre: [f32; 4],
    colour: [f32; 4],
    axis: [f32; 4],
    extra: [f32; 4],
}

fn kind_code(k: DecalKind) -> f32 {
    match k {
        DecalKind::Scorch => 0.0,
        DecalKind::Crack => 1.0,
        DecalKind::Splat => 2.0,
        DecalKind::Glow => 3.0,
    }
}

/// The decal pass.
pub struct Decals {
    canvas: wgpu::RenderPipeline,
    rope: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    cam: wgpu::Buffer,
    decs: Grow,
    bind: wgpu::BindGroup,
    sleeve_vb: wgpu::Buffer,
    sleeve_ib: wgpu::Buffer,
    sleeve_count: u32,
    data: Vec<DecG>,
    n_canvas: u32,
}

impl Decals {
    pub fn new(gpu: &Gpu) -> Decals {
        use wgpu::util::DeviceExt;
        let device = &gpu.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("decals"),
            entries: &[pipe::entry(0, pipe::UNIFORM), pipe::entry(1, pipe::STORAGE)],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("decals"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let m = pipe::module(gpu, "decals", include_str!("shaders/decal.wgsl"));
        let canvas = pipe::pipeline_entries(
            gpu,
            "canvas decals",
            &pl,
            &m,
            "vs_canvas",
            "fs_main",
            wgpu::PrimitiveTopology::TriangleStrip,
            Blend::Over,
            &[],
        );
        let rope = pipe::pipeline_entries(
            gpu,
            "rope decals",
            &pl,
            &m,
            "vs_rope",
            "fs_main",
            wgpu::PrimitiveTopology::TriangleList,
            Blend::Over,
            &[Some(bf2_render::Vertex::LAYOUT)],
        );
        let cam = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("decals camera"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let decs = Grow::new(gpu, "decals", wgpu::BufferUsages::STORAGE, 64 * 64);
        let bind = Self::bind_group(gpu, &layout, &cam, &decs.buf);
        // An open sleeve: a cylinder's side only (the caps would sit inside the rope).
        let mut sleeve = geom::cylinder(1.0, 2.0, 24);
        sleeve.idx = sleeve
            .idx
            .as_chunks::<3>()
            .0
            .iter()
            .filter(|t| t.iter().all(|&i| sleeve.verts[i as usize].nrm[1].abs() < 0.5))
            .flatten()
            .copied()
            .collect();
        let sleeve_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sleeve"),
            contents: bytemuck::cast_slice(&sleeve.verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let sleeve_ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sleeve"),
            contents: bytemuck::cast_slice(&sleeve.idx),
            usage: wgpu::BufferUsages::INDEX,
        });
        Decals {
            canvas,
            rope,
            layout,
            cam,
            decs,
            bind,
            sleeve_vb,
            sleeve_ib,
            sleeve_count: sleeve.idx.len() as u32,
            data: Vec::new(),
            n_canvas: 0,
        }
    }

    fn bind_group(gpu: &Gpu, layout: &wgpu::BindGroupLayout, cam: &wgpu::Buffer, b: &wgpu::Buffer) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("decals"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cam.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: b.as_entire_binding(),
                },
            ],
        })
    }

    /// Uploads the live decals of `recs` (anchors resolved, sim space) at video time `t`.
    pub fn prepare(&mut self, gpu: &Gpu, camera: &FxCamera, recs: &[(&FxRecord, [f64; 3])], t: f64) {
        self.data.clear();
        let mut ropes = Vec::new();
        for (r, at) in recs {
            let FxKind::Decal(d) = &r.kind else { continue };
            if !r.live_at(t) {
                continue;
            }
            let extra = [
                r.age(t) / r.life,
                (r.seed & 0xFFFF) as f32,
                crate::particles::exit_code(r.exit) as f32,
                r.env_at(t),
            ];
            let colour = [d.colour[0], d.colour[1], d.colour[2], kind_code(d.kind)];
            let p = to_render(*at);
            match d.on {
                Surface::Canvas => self.data.push(DecG {
                    centre: [p[0], 0.004, p[2], d.r],
                    colour,
                    axis: [1.0, 0.0, 0.0, 0.0],
                    extra,
                }),
                Surface::Rope(side) => {
                    let (out, along) = SIDES[usize::from(side) % 4];
                    // The anchor's place along the side, kept between the posts.
                    let u = (p[0] * along[0] + p[2] * along[2]).clamp(-POST + d.r, POST - d.r);
                    let s = (u / POST + 1.0) / 2.0;
                    for h in ROPE_HEIGHTS {
                        let y = h - ROPE_SAG * 4.0 * s * (1.0 - s);
                        ropes.push(DecG {
                            centre: [out[0] * POST + along[0] * u, y, out[2] * POST + along[2] * u, d.r],
                            colour,
                            axis: [along[0], along[1], along[2], ROPE_R * 1.35],
                            extra,
                        });
                    }
                }
            }
        }
        self.n_canvas = self.data.len() as u32;
        self.data.extend(ropes);
        gpu.queue
            .write_buffer(&self.cam, 0, bytemuck::cast_slice(&camera.view_proj));
        if self.decs.write(gpu, bytemuck::cast_slice(&self.data)) {
            self.bind = Self::bind_group(gpu, &self.layout, &self.cam, &self.decs.buf);
        }
    }

    pub fn encode(&self, pass: &mut wgpu::RenderPass<'_>) {
        let n = self.data.len() as u32;
        if n == 0 {
            return;
        }
        pass.set_bind_group(0, &self.bind, &[]);
        if self.n_canvas > 0 {
            pass.set_pipeline(&self.canvas);
            pass.draw(0..4, 0..self.n_canvas);
        }
        if n > self.n_canvas {
            pass.set_pipeline(&self.rope);
            pass.set_vertex_buffer(0, self.sleeve_vb.slice(..));
            pass.set_index_buffer(self.sleeve_ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.sleeve_count, 0, self.n_canvas..n);
        }
    }

    pub fn draw(&mut self, ctx: &mut FxCtx<'_>, recs: &[(&FxRecord, [f64; 3])], t: f64) {
        self.prepare(ctx.gpu, &ctx.camera, recs, t);
        let mut pass = ctx.begin_pass("decals");
        self.encode(&mut pass);
    }
}

#[cfg(test)]
#[path = "decal_tests.rs"]
mod tests;
