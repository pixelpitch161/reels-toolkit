//! Mesh effects: shock rings (expanding annuli, flat on the canvas or tilted), domes (expanding fresnel shells) and
//! beams (jittering bands between two anchors). All emissive HDR, additive, depth tested, never depth written.

use bf2_render::{FxCamera, FxCtx, Gpu, geom, to_render};

use crate::hash::h3;
use crate::pipe::{self, Blend, Grow};
use crate::record::{Anchor, Exit, FxKind, FxRecord};
use crate::trail::TrailVertex;

/// An expanding ring.
#[derive(Clone, Debug, PartialEq)]
pub struct RingDef {
    /// Radius at birth and at the end, units.
    pub r: (f32, f32),
    pub thickness: f32,
    /// HDR.
    pub colour: [f32; 3],
    /// Radians from flat on the canvas, about the render X axis (toward the camera).
    pub tilt: f32,
    /// How hard it pushes the screen shockwave (Task 6).
    pub distort: f32,
}

impl Default for RingDef {
    fn default() -> Self {
        RingDef {
            r: (0.2, 2.5),
            thickness: 0.35,
            colour: [5.0, 3.0, 1.2],
            tilt: 0.0,
            distort: 0.0,
        }
    }
}

/// An expanding shell.
#[derive(Clone, Debug, PartialEq)]
pub struct DomeDef {
    pub r: (f32, f32),
    /// HDR.
    pub colour: [f32; 3],
    /// 0: evenly lit; 1: only its rim glows.
    pub fresnel: f32,
}

impl Default for DomeDef {
    fn default() -> Self {
        DomeDef {
            r: (0.3, 1.8),
            colour: [2.0, 3.0, 6.0],
            fresnel: 0.85,
        }
    }
}

/// A beam between two anchors.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamDef {
    pub from: Anchor,
    pub to: Anchor,
    pub width: f32,
    /// HDR.
    pub colour: [f32; 3],
    pub core: [f32; 3],
    /// Units of sideways jitter.
    pub jitter: f32,
}

impl Default for BeamDef {
    fn default() -> Self {
        BeamDef {
            from: Anchor::World([0.0; 3]),
            to: Anchor::World([0.0, 0.0, 2.0]),
            width: 0.3,
            colour: [2.0, 3.5, 8.0],
            core: [8.0, 8.0, 10.0],
            jitter: 0.08,
        }
    }
}

/// Eases out: fast at first, settling at the end.
fn ease_out(u: f32) -> f32 {
    let v = 1.0 - u.clamp(0.0, 1.0);
    1.0 - v * v * v
}

/// The ring's or dome's radius `age` seconds into a `life`.
pub fn grow(r: (f32, f32), life: f32, age: f32, exit: Exit, env: f32) -> f32 {
    let base = r.0 + (r.1 - r.0) * ease_out(age / life.max(1e-6));
    match exit {
        Exit::Shrink => base * env,
        Exit::Burst => base * (1.0 + 0.3 * (1.0 - env)),
        _ => base,
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CamU {
    view_proj: [[f32; 4]; 4],
    eye: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct InstG {
    centre: [f32; 4],
    colour: [f32; 4],
    axis: [f32; 4],
    extra: [f32; 4],
}

const RING_SEGMENTS: u32 = 128;
const BEAM_SEGMENTS: usize = 24;

/// The mesh effect pass.
pub struct MeshFx {
    ring: wgpu::RenderPipeline,
    dome: wgpu::RenderPipeline,
    beam: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    cam: wgpu::Buffer,
    insts: Grow,
    bind: wgpu::BindGroup,
    beam_bind: wgpu::BindGroup,
    ring_vb: wgpu::Buffer,
    dome_vb: wgpu::Buffer,
    dome_ib: wgpu::Buffer,
    dome_count: u32,
    beam_vb: Grow,
    data: Vec<InstG>,
    n_rings: u32,
    beam_verts: Vec<TrailVertex>,
}

fn buffer(gpu: &Gpu, label: &str, bytes: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytes,
        usage,
    })
}

impl MeshFx {
    pub fn new(gpu: &Gpu) -> MeshFx {
        let device = &gpu.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("meshfx"),
            entries: &[pipe::entry(0, pipe::UNIFORM), pipe::entry(1, pipe::STORAGE)],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("meshfx"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let m = pipe::module(gpu, "meshfx", include_str!("shaders/meshfx.wgsl"));
        let ring_layout = wgpu::VertexBufferLayout {
            array_stride: 12,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3],
        };
        let mk = |label, vs, fs, buffers: &[Option<wgpu::VertexBufferLayout<'_>>]| {
            pipe::pipeline_entries(
                gpu,
                label,
                &pl,
                &m,
                vs,
                fs,
                wgpu::PrimitiveTopology::TriangleStrip,
                Blend::Add,
                buffers,
            )
        };
        let ring = mk("rings", "vs_ring", "fs_ring", &[Some(ring_layout)]);
        let dome = pipe::pipeline_entries(
            gpu,
            "domes",
            &pl,
            &m,
            "vs_dome",
            "fs_dome",
            wgpu::PrimitiveTopology::TriangleList,
            Blend::Add,
            &[Some(bf2_render::Vertex::LAYOUT)],
        );
        // Beams are strips like trails: the trail shader draws them.
        let trail_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("beams"),
            entries: &[pipe::entry(0, pipe::UNIFORM)],
        });
        let trail_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("beams"),
            bind_group_layouts: &[Some(&trail_layout)],
            immediate_size: 0,
        });
        let tm = pipe::module(gpu, "beams", include_str!("shaders/trail.wgsl"));
        let beam = pipe::pipeline(
            gpu,
            "beams",
            &trail_pl,
            &tm,
            wgpu::PrimitiveTopology::TriangleList,
            Blend::Add,
            &[Some(TrailVertex::LAYOUT)],
        );
        let cam = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("meshfx camera"),
            size: std::mem::size_of::<CamU>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let insts = Grow::new(gpu, "meshfx instances", wgpu::BufferUsages::STORAGE, 64 * 64);
        let bind = Self::bind_group(gpu, &layout, &cam, &insts.buf);
        // The trail shader's camera is the first 64 bytes of ours.
        let beam_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("beams"),
            layout: &trail_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: cam.as_entire_binding(),
            }],
        });
        let mut ring_v: Vec<f32> = Vec::new();
        for s in 0..=RING_SEGMENTS {
            let a = s as f32 / RING_SEGMENTS as f32 * std::f32::consts::TAU;
            for edge in [0.0, 1.0] {
                ring_v.extend([a.cos(), a.sin(), edge]);
            }
        }
        let ring_vb = buffer(gpu, "ring", bytemuck::cast_slice(&ring_v), wgpu::BufferUsages::VERTEX);
        let sphere = geom::icosphere(1.0, 3);
        let dome_vb = buffer(
            gpu,
            "dome",
            bytemuck::cast_slice(&sphere.verts),
            wgpu::BufferUsages::VERTEX,
        );
        let dome_ib = buffer(
            gpu,
            "dome",
            bytemuck::cast_slice(&sphere.idx),
            wgpu::BufferUsages::INDEX,
        );
        MeshFx {
            ring,
            dome,
            beam,
            layout,
            cam,
            insts,
            bind,
            beam_bind,
            ring_vb,
            dome_vb,
            dome_ib,
            dome_count: sphere.idx.len() as u32,
            beam_vb: Grow::new(gpu, "beam vertices", wgpu::BufferUsages::VERTEX, 1 << 14),
            data: Vec::new(),
            n_rings: 0,
            beam_verts: Vec::new(),
        }
    }

    fn bind_group(gpu: &Gpu, layout: &wgpu::BindGroupLayout, cam: &wgpu::Buffer, b: &wgpu::Buffer) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("meshfx"),
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

    /// Uploads the live rings, domes and beams at video time `t`. `recs` carry each record's anchor resolved
    /// (sim space); `resolve` resolves a beam's two ends.
    pub fn prepare(
        &mut self,
        gpu: &Gpu,
        camera: &FxCamera,
        recs: &[(&FxRecord, [f64; 3])],
        resolve: &dyn Fn(&Anchor) -> [f64; 3],
        t: f64,
    ) {
        self.data.clear();
        self.beam_verts.clear();
        let mut domes = Vec::new();
        for (r, at) in recs {
            if !r.live_at(t) {
                continue;
            }
            let (env, age) = (r.env_at(t), r.age(t));
            let u = age / r.life;
            let c = to_render(*at);
            match &r.kind {
                FxKind::Ring(d) => {
                    let rad = grow(d.r, r.life, age, r.exit, env);
                    let thick = if r.exit == Exit::Shrink {
                        d.thickness * env
                    } else {
                        d.thickness
                    };
                    self.data.push(InstG {
                        centre: [c[0], c[1] + 0.01, c[2], rad],
                        colour: [d.colour[0] * env, d.colour[1] * env, d.colour[2] * env, thick],
                        axis: [0.0, d.tilt.cos(), d.tilt.sin(), 0.0],
                        extra: [u, (r.seed & 0xFFFF) as f32, 0.0, 0.0],
                    });
                }
                FxKind::Dome(d) => {
                    let rad = grow(d.r, r.life, age, r.exit, env);
                    domes.push(InstG {
                        centre: [c[0], c[1], c[2], rad],
                        colour: [d.colour[0] * env, d.colour[1] * env, d.colour[2] * env, 0.0],
                        axis: [0.0, 1.0, 0.0, d.fresnel],
                        extra: [u, (r.seed & 0xFFFF) as f32, 1.0, 0.0],
                    });
                }
                FxKind::Beam(d) => {
                    let (a, b) = (to_render(resolve(&d.from)), to_render(resolve(&d.to)));
                    beam_strip(d, a, b, camera.eye, env, r.seed, t, &mut self.beam_verts);
                }
                _ => {}
            }
        }
        self.n_rings = self.data.len() as u32;
        self.data.extend(domes);
        let cam = CamU {
            view_proj: camera.view_proj,
            eye: [camera.eye[0], camera.eye[1], camera.eye[2], 1.0],
        };
        gpu.queue.write_buffer(&self.cam, 0, bytemuck::bytes_of(&cam));
        if self.insts.write(gpu, bytemuck::cast_slice(&self.data)) {
            self.bind = Self::bind_group(gpu, &self.layout, &self.cam, &self.insts.buf);
        }
        self.beam_vb.write(gpu, bytemuck::cast_slice(&self.beam_verts));
    }

    pub fn encode(&self, pass: &mut wgpu::RenderPass<'_>) {
        let n = self.data.len() as u32;
        if n > 0 {
            pass.set_bind_group(0, &self.bind, &[]);
            if self.n_rings > 0 {
                pass.set_pipeline(&self.ring);
                pass.set_vertex_buffer(0, self.ring_vb.slice(..));
                pass.draw(0..(RING_SEGMENTS + 1) * 2, 0..self.n_rings);
            }
            if n > self.n_rings {
                pass.set_pipeline(&self.dome);
                pass.set_vertex_buffer(0, self.dome_vb.slice(..));
                pass.set_index_buffer(self.dome_ib.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.dome_count, 0, self.n_rings..n);
            }
        }
        if !self.beam_verts.is_empty() {
            pass.set_pipeline(&self.beam);
            pass.set_bind_group(0, &self.beam_bind, &[]);
            pass.set_vertex_buffer(0, self.beam_vb.buf.slice(..));
            pass.draw(0..self.beam_verts.len() as u32, 0..1);
        }
    }

    pub fn draw(
        &mut self,
        ctx: &mut FxCtx<'_>,
        recs: &[(&FxRecord, [f64; 3])],
        resolve: &dyn Fn(&Anchor) -> [f64; 3],
        t: f64,
    ) {
        self.prepare(ctx.gpu, &ctx.camera, recs, resolve, t);
        let mut pass = ctx.begin_pass("meshfx");
        self.encode(&mut pass);
    }
}

/// A beam from `a` to `b` (render space) as a camera-facing, jittering strip, tapered at both ends.
#[allow(clippy::too_many_arguments)]
pub fn beam_strip(
    d: &BeamDef,
    a: [f32; 3],
    b: [f32; 3],
    eye: [f32; 3],
    env: f32,
    seed: u32,
    t: f64,
    out: &mut Vec<TrailVertex>,
) {
    if env <= 0.0 {
        return;
    }
    let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, (a[2] + b[2]) / 2.0];
    let to_eye = [eye[0] - mid[0], eye[1] - mid[1], eye[2] - mid[2]];
    let mut side = [
        dir[1] * to_eye[2] - dir[2] * to_eye[1],
        dir[2] * to_eye[0] - dir[0] * to_eye[2],
        dir[0] * to_eye[1] - dir[1] * to_eye[0],
    ];
    let l = (side[0] * side[0] + side[1] * side[1] + side[2] * side[2]).sqrt();
    if l < 1e-6 {
        side = [1.0, 0.0, 0.0];
    } else {
        side = side.map(|x| x / l);
    }
    let tick = (t * 30.0).floor() as u32;
    let point = |i: usize| {
        let s = i as f32 / BEAM_SEGMENTS as f32;
        let taper = (s * (1.0 - s) * 4.0).sqrt();
        let j = if i == 0 || i == BEAM_SEGMENTS {
            0.0
        } else {
            (h3(seed ^ tick, i as u32, 0) - 0.5) * 2.0 * d.jitter
        };
        let c: [f32; 3] = std::array::from_fn(|k| a[k] + dir[k] * s + side[k] * j);
        let w = d.width * 0.5 * (0.25 + 0.75 * taper);
        (
            std::array::from_fn::<f32, 3, _>(|k| c[k] - side[k] * w),
            std::array::from_fn::<f32, 3, _>(|k| c[k] + side[k] * w),
        )
    };
    let v = |p: [f32; 3], across: f32| TrailVertex {
        pos: p,
        along: 0.0,
        colour: d.colour.map(|c| c * env),
        across,
        core: d.core.map(|c| c * env),
        style: 0.0,
    };
    for i in 0..BEAM_SEGMENTS {
        let (a0, a1) = point(i);
        let (b0, b1) = point(i + 1);
        out.extend([v(a0, 0.0), v(a1, 1.0), v(b1, 1.0), v(a0, 0.0), v(b1, 1.0), v(b0, 0.0)]);
    }
}

#[cfg(test)]
#[path = "meshfx_tests.rs"]
mod tests;
