//! Stateless GPU particles: a burst record says how many particles, how they fly and how they look; each
//! particle's state at any time is a closed form of the record, its index and the record's age (ballistic flight
//! with analytic drag, hashed variation and one analytic bounce off the canvas). Nothing is carried between
//! frames, so frames draw the same in any order.
//!
//! Drawn as camera-facing quads, one instanced draw per shape: sparks (velocity streaks), embers, notes (all
//! additive HDR, so they bloom), smoke and debris (premultiplied over, smoke sorted far to near by record).

use bf2_render::{FxCamera, FxCtx, Gpu, Renderer, to_render};

use crate::hash::h3;
use crate::pipe::{self, Blend, Grow};
use crate::record::{Exit, FxKind, FxRecord};

/// The most particles drawn in one frame. Over it, whole records are dropped, oldest first.
pub const MAX_LIVE: u32 = 262_144;

/// A particle's look.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Shape {
    /// A hot streak along its velocity.
    #[default]
    Spark,
    /// A soft, flickering glow.
    Ember,
    /// A billow of smoke (over-blended).
    Smoke,
    /// A tumbling shard (over-blended).
    Debris,
    /// A musical note.
    Note,
}

impl Shape {
    /// Draw order: the over-blended shapes first, so the glowing ones add on top of the smoke.
    pub const ALL: [Shape; 5] = [Shape::Smoke, Shape::Debris, Shape::Spark, Shape::Ember, Shape::Note];

    fn code(self) -> u32 {
        match self {
            Shape::Spark => 0,
            Shape::Ember => 1,
            Shape::Smoke => 2,
            Shape::Debris => 3,
            Shape::Note => 4,
        }
    }

    fn blend(self) -> Blend {
        match self {
            Shape::Smoke | Shape::Debris => Blend::Over,
            _ => Blend::Add,
        }
    }
}

/// A burst of particles from one point.
#[derive(Clone, Debug, PartialEq)]
pub struct BurstDef {
    pub count: u32,
    /// Units/s, min and max.
    pub speed: (f32, f32),
    /// The cone's half angle around `dir`, radians; π is a sphere.
    pub spread: f32,
    /// Render space (Y up).
    pub dir: [f32; 3],
    /// Downward acceleration, units/s².
    pub gravity: f32,
    /// 1/s.
    pub drag: f32,
    /// Units across, at birth and at death.
    pub size: (f32, f32),
    /// HDR.
    pub colour: [f32; 3],
    pub colour_end: [f32; 3],
    /// Seconds of velocity a streak is long.
    pub stretch: f32,
    /// Share of the speed kept off the canvas; 0 dies there.
    pub bounce: f32,
    pub shape: Shape,
    /// Seconds over which particles are born; 0 is all at once.
    pub emit_over: f32,
}

impl Default for BurstDef {
    fn default() -> Self {
        BurstDef {
            count: 1000,
            speed: (4.0, 12.0),
            spread: std::f32::consts::PI,
            dir: [0.0, 1.0, 0.0],
            gravity: 9.0,
            drag: 2.0,
            size: (0.06, 0.02),
            colour: [12.0, 6.0, 2.0],
            colour_end: [2.0, 0.3, 0.05],
            stretch: 0.03,
            bounce: 0.35,
            shape: Shape::Spark,
            emit_over: 0.0,
        }
    }
}

/// One particle at one time (CPU reference of the shader's closed form).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleState {
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    /// Share of its life gone.
    pub u: f32,
}

fn drift(d: f32, a: f32) -> f32 {
    if d < 1e-4 { a } else { (1.0 - (-d * a).exp()) / d }
}

fn fly(p0: [f32; 3], v0: [f32; 3], g: f32, d: f32, a: f32) -> [f32; 3] {
    let k = drift(d, a);
    [
        p0[0] + v0[0] * k,
        p0[1] + v0[1] * k - 0.5 * g * a * a,
        p0[2] + v0[2] * k,
    ]
}

fn fly_vel(v0: [f32; 3], g: f32, d: f32, a: f32) -> [f32; 3] {
    let e = (-d * a).exp();
    [v0[0] * e, v0[1] * e - g * a, v0[2] * e]
}

fn norm(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 0.0 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        [0.0, 1.0, 0.0]
    }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Particle `i` of a burst record (`seed`, `life`, render-space `origin`) when the record is `age` seconds old;
/// `None` before its birth, after its death or when it dies on the canvas. The shader's `particle`.
pub fn particle_state(
    def: &BurstDef,
    seed: u32,
    life: f32,
    origin: [f32; 3],
    i: u32,
    age: f32,
) -> Option<ParticleState> {
    let tb = def.emit_over * h3(seed, i, 0);
    let a = age - tb;
    let plife = (life - tb).max(0.0) * (0.45 + 0.55 * h3(seed, i, 4));
    if a <= 0.0 || a >= plife {
        return None;
    }
    let d = norm(def.dir);
    let helper = if d[1].abs() > 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let t = norm(cross(d, helper));
    let bt = cross(d, t);
    let cs = def.spread.cos();
    let c = cs + (1.0 - cs) * h3(seed, i, 1);
    let sn = (1.0 - c * c).max(0.0).sqrt();
    let phi = std::f32::consts::TAU * h3(seed, i, 2);
    let speed = def.speed.0 + (def.speed.1 - def.speed.0) * h3(seed, i, 3).sqrt();
    let v0: [f32; 3] = std::array::from_fn(|k| (t[k] * (phi.cos() * sn) + bt[k] * (phi.sin() * sn) + d[k] * c) * speed);
    let (g, dr) = (def.gravity, def.drag);
    let mut pos = fly(origin, v0, g, dr, a);
    let mut vel = fly_vel(v0, g, dr, a);
    if pos[1] < 0.0 && origin[1] >= 0.0 {
        let (mut lo, mut hi) = (0.0f32, a);
        for _ in 0..14 {
            let mid = 0.5 * (lo + hi);
            if fly(origin, v0, g, dr, mid)[1] > 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        if def.bounce <= 0.0 {
            return None;
        }
        let mut pb = fly(origin, v0, g, dr, hi);
        pb[1] = 0.0;
        let vb = fly_vel(v0, g, dr, hi);
        let vb = [vb[0] * 0.7, -vb[1] * def.bounce, vb[2] * 0.7];
        let rest = a - hi;
        pos = fly(pb, vb, g, dr, rest);
        vel = fly_vel(vb, g, dr, rest);
        if pos[1] < 0.0 {
            pos[1] = 0.0;
            vel[1] = 0.0;
        }
    }
    Some(ParticleState { pos, vel, u: a / plife })
}

/// `Cam` in particles.wgsl.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CamU {
    view_proj: [[f32; 4]; 4],
    eye: [f32; 4],
    right: [f32; 4],
    up: [f32; 4],
}

/// `Burst` in particles.wgsl.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BurstG {
    origin: [f32; 4],
    dir: [f32; 4],
    motion: [f32; 4],
    look: [f32; 4],
    colour: [f32; 4],
    colour_end: [f32; 4],
    envl: [f32; 4],
    info: [u32; 4],
    first: [u32; 4],
}

/// The particle pass: pipelines built once, the record buffer reused and grown.
pub struct Particles {
    add: wgpu::RenderPipeline,
    over: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    cam: wgpu::Buffer,
    bursts: Grow,
    bind: wgpu::BindGroup,
    dropped: u64,
    data: Vec<BurstG>,
    /// Per shape (in `Shape::ALL` order): the global particle range.
    ranges: [(u32, u32); 5],
}

impl Particles {
    pub fn new(gpu: &Gpu, _r: &Renderer) -> Particles {
        let device = &gpu.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particles"),
            entries: &[pipe::entry(0, pipe::UNIFORM), pipe::entry(1, pipe::STORAGE)],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particles"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let m = pipe::module(gpu, "particles", include_str!("shaders/particles.wgsl"));
        let strip = wgpu::PrimitiveTopology::TriangleStrip;
        let add = pipe::pipeline(gpu, "particles add", &pl, &m, strip, Blend::Add, &[]);
        let over = pipe::pipeline(gpu, "particles over", &pl, &m, strip, Blend::Over, &[]);
        let cam = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particles camera"),
            size: std::mem::size_of::<CamU>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bursts = Grow::new(
            gpu,
            "bursts",
            wgpu::BufferUsages::STORAGE,
            64 * std::mem::size_of::<BurstG>() as u64,
        );
        let bind = Self::bind_group(gpu, &layout, &cam, &bursts.buf);
        Particles {
            add,
            over,
            layout,
            cam,
            bursts,
            bind,
            dropped: 0,
            data: Vec::new(),
            ranges: [(0, 0); 5],
        }
    }

    fn bind_group(gpu: &Gpu, layout: &wgpu::BindGroupLayout, cam: &wgpu::Buffer, b: &wgpu::Buffer) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particles"),
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

    /// Records dropped so far because a frame was over [`MAX_LIVE`].
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Uploads the live burst records of `recs` (each with its anchor resolved, sim space) for video time `t`.
    pub fn prepare(&mut self, gpu: &Gpu, camera: &FxCamera, recs: &[(&FxRecord, [f64; 3])], t: f64) {
        let mut live: Vec<(&FxRecord, &BurstDef, [f32; 3])> = recs
            .iter()
            .filter_map(|(r, at)| match &r.kind {
                FxKind::Burst(b) if r.live_at(t) && b.count > 0 => Some((*r, b, to_render(*at))),
                _ => None,
            })
            .collect();
        // Over budget: drop whole records, oldest first.
        live.sort_by(|a, b| a.0.t0.total_cmp(&b.0.t0));
        let mut total: u64 = live.iter().map(|l| u64::from(l.1.count)).sum();
        let mut cut = 0;
        while total > u64::from(MAX_LIVE) && cut < live.len() {
            total -= u64::from(live[cut].1.count);
            cut += 1;
        }
        if cut > 0 {
            self.dropped += cut as u64;
            eprintln!("bf2-fx: {cut} particle burst(s) dropped at {t:.3} s: over the {MAX_LIVE}-particle budget");
            live.drain(..cut);
        }
        // Grouped by shape; smoke far to near so its over-blending layers right.
        let depth = |p: [f32; 3]| {
            let d = [p[0] - camera.eye[0], p[1] - camera.eye[1], p[2] - camera.eye[2]];
            d[0] * camera.forward[0] + d[1] * camera.forward[1] + d[2] * camera.forward[2]
        };
        let order = |s: Shape| Shape::ALL.iter().position(|&x| x == s).unwrap_or(0);
        live.sort_by(|a, b| {
            order(a.1.shape).cmp(&order(b.1.shape)).then_with(|| {
                if a.1.shape == Shape::Smoke {
                    depth(b.2).total_cmp(&depth(a.2))
                } else {
                    std::cmp::Ordering::Equal
                }
            })
        });

        self.data.clear();
        self.ranges = [(0, 0); 5];
        let mut first = 0u32;
        for (r, b, origin) in &live {
            let s = order(b.shape);
            if self.ranges[s] == (0, 0) || self.ranges[s].1 == 0 {
                self.ranges[s].0 = first;
            }
            let d = {
                let l = (b.dir[0] * b.dir[0] + b.dir[1] * b.dir[1] + b.dir[2] * b.dir[2]).sqrt();
                if l > 0.0 {
                    [b.dir[0] / l, b.dir[1] / l, b.dir[2] / l]
                } else {
                    [0.0, 1.0, 0.0]
                }
            };
            self.data.push(BurstG {
                origin: [origin[0], origin[1], origin[2], r.age(t)],
                dir: [d[0], d[1], d[2], b.spread.clamp(0.0, std::f32::consts::PI).cos()],
                motion: [b.speed.0, b.speed.1, b.gravity, b.drag.max(0.0)],
                look: [b.size.0, b.size.1, b.stretch.max(0.0), b.bounce],
                colour: [b.colour[0], b.colour[1], b.colour[2], b.emit_over.max(0.0)],
                colour_end: [b.colour_end[0], b.colour_end[1], b.colour_end[2], r.life],
                envl: [r.env_at(t), 0.0, 0.0, 0.0],
                info: [r.seed, b.count, b.shape.code(), exit_code(r.exit)],
                first: [first, 0, 0, 0],
            });
            first += b.count;
            self.ranges[s].1 = first;
        }
        let px = camera.pixel_per_distance();
        let cam = CamU {
            view_proj: camera.view_proj,
            eye: [camera.eye[0], camera.eye[1], camera.eye[2], px],
            right: [
                camera.right[0],
                camera.right[1],
                camera.right[2],
                self.data.len() as f32,
            ],
            up: [camera.up[0], camera.up[1], camera.up[2], t.rem_euclid(1000.0) as f32],
        };
        gpu.queue.write_buffer(&self.cam, 0, bytemuck::bytes_of(&cam));
        if self.bursts.write(gpu, bytemuck::cast_slice(&self.data)) {
            self.bind = Self::bind_group(gpu, &self.layout, &self.cam, &self.bursts.buf);
        }
    }

    /// Records the draws (after [`Particles::prepare`]) into an effect pass.
    pub fn encode(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.data.is_empty() {
            return;
        }
        pass.set_bind_group(0, &self.bind, &[]);
        for (k, shape) in Shape::ALL.iter().enumerate() {
            let (a, b) = self.ranges[k];
            if b <= a {
                continue;
            }
            pass.set_pipeline(match shape.blend() {
                Blend::Add => &self.add,
                Blend::Over => &self.over,
            });
            pass.draw(0..4, a..b);
        }
    }

    /// Prepares and draws in a pass of its own.
    pub fn draw(&mut self, ctx: &mut FxCtx<'_>, recs: &[(&FxRecord, [f64; 3])], t: f64) {
        self.prepare(ctx.gpu, &ctx.camera, recs, t);
        let mut pass = ctx.begin_pass("particles");
        self.encode(&mut pass);
    }
}

/// The shaders' exit codes (particles.wgsl's `EXIT_*`).
pub(crate) fn exit_code(e: Exit) -> u32 {
    match e {
        Exit::Fade => 0,
        Exit::Dissolve => 1,
        Exit::Burst => 2,
        Exit::Shrink => 3,
        Exit::FlyOff => 4,
        Exit::Ember => 5,
    }
}

#[cfg(test)]
#[path = "particles_tests.rs"]
mod tests;
