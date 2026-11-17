//! Trails and smears on weapon bones, from the recording: the bone's base and tip at every recorded step in a
//! window of history (120 Hz), plus the interpolated end. A smear fills the arc the bone swept (the anime swing
//! arc); a ribbon follows the tip; lightning is a jagged, flickering ribbon. The strip is rebuilt on the CPU each
//! frame into a reused vertex buffer and drawn additive in HDR, fading to nothing at its tail.

use bf2_render::{FxCamera, FxCtx, Gpu, to_render};
use bf2_sim::{Joint, Pose, Recording2, Rig, Segment, V3, solve};

use crate::hash::h3;
use crate::pipe::{self, Blend, Grow};
use crate::record::{FxKind, FxRecord};

/// How a trail is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrailStyle {
    /// A band behind the tip.
    #[default]
    Ribbon,
    /// The filled arc between the bone's base and tip across the samples.
    Smear,
    /// A jagged, flickering band behind the tip.
    Lightning,
}

/// The "bone" that is fighter `k`'s ball itself: a trail with it follows the ball's centre (a launch ribbon).
pub const BALL: usize = usize::MAX;

/// The first "bone" that is a projectile: `PROJ + id` follows projectile `id` (a thrown thing's ribbon).
pub const PROJ: usize = 1 << 40;

/// A trail on fighter `k`'s `bone` (or its ball, [`BALL`], or a projectile, [`PROJ`] + its id).
#[derive(Clone, Debug, PartialEq)]
pub struct TrailDef {
    pub k: usize,
    pub bone: usize,
    /// Units across, at the head and at the tail (a ribbon's width; a smear is as wide as the bone).
    pub width: (f32, f32),
    /// Seconds of history drawn.
    pub length_s: f32,
    /// HDR.
    pub colour: [f32; 3],
    /// The hot inner line, HDR.
    pub core: [f32; 3],
    pub style: TrailStyle,
}

impl Default for TrailDef {
    fn default() -> Self {
        TrailDef {
            k: 0,
            bone: 0,
            width: (0.18, 0.02),
            length_s: 0.15,
            colour: [2.5, 1.2, 0.4],
            core: [6.0, 5.0, 4.0],
            style: TrailStyle::Smear,
        }
    }
}

/// The state of fighter `k` at step `s` moved `alpha` toward step `s + 1`.
pub(crate) fn state_at(rec: &Recording2, k: usize, s: usize, alpha: f64) -> (V3, f64, Vec<Joint>) {
    let a = &rec.steps[s];
    let b = rec.steps.get(s + 1).unwrap_or(a);
    let lerp = |x: f64, y: f64| x + (y - x) * alpha;
    let pos = std::array::from_fn(|i| lerp(a.pos[k][i], b.pos[k][i]));
    let tau = std::f64::consts::TAU;
    let d = (b.facing[k] - a.facing[k]).rem_euclid(tau);
    let d = if d > std::f64::consts::PI { d - tau } else { d };
    let joints = a.joints[k]
        .iter()
        .enumerate()
        .map(|(i, j)| b.joints[k].get(i).map_or(*j, |jb| j.lerp(*jb, alpha)))
        .collect();
    (pos, a.facing[k] + d * alpha, joints)
}

fn bone_at(
    rig: &Rig,
    r_ball: f64,
    state: (V3, f64, Vec<Joint>),
    bone: usize,
    segs: &mut Vec<Segment>,
) -> Option<Segment> {
    let (pos, facing, joints) = state;
    let centre = [pos[0], pos[1], pos[2] + r_ball];
    if bone == BALL {
        return Some(Segment {
            a: centre,
            b: centre,
            radius: r_ball,
        });
    }
    solve(rig, &Pose { joints }, centre, r_ball, facing, segs);
    segs.get(bone).copied()
}

/// The first of `0..n` for which `pred` is false (`pred` true then false, as for a sorted slice).
pub(crate) fn partition(n: usize, pred: impl Fn(usize) -> bool) -> usize {
    let (mut lo, mut hi) = (0, n);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if pred(mid) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// The bone's (base, tip, age in seconds) at every recorded step whose video time (`step_to_t`) is within
/// `length_s` before `t_end`, oldest first, then the interpolated state at `t_end`. Sim space. `r_ball` is the
/// fighter's ball radius (the rig starts on its surface).
#[allow(clippy::too_many_arguments)]
pub fn trail_points(
    rec: &Recording2,
    rig: &Rig,
    r_ball: f64,
    k: usize,
    bone: usize,
    t_end: f64,
    length_s: f32,
    step_to_t: &dyn Fn(u32) -> f64,
) -> Vec<(V3, V3, f32)> {
    if (PROJ..BALL).contains(&bone) {
        return proj_points(rec, (bone - PROJ) as u32, t_end, length_s, step_to_t);
    }
    let n = rec.steps.len();
    let mut out = Vec::new();
    if n == 0 {
        return out;
    }
    let t_start = t_end - f64::from(length_s);
    let first = partition(n, |i| step_to_t(i as u32) < t_start);
    let mut segs = Vec::new();
    let mut last_t = f64::NEG_INFINITY;
    for i in first..n {
        let t = step_to_t(i as u32);
        if t > t_end {
            break;
        }
        if let Some(s) = bone_at(rig, r_ball, state_at(rec, k, i, 0.0), bone, &mut segs) {
            out.push((s.a, s.b, (t_end - t) as f32));
        }
        last_t = t;
    }
    // The end, between recorded steps.
    let s = partition(n, |i| step_to_t(i as u32) <= t_end);
    if s > 0 && s < n && t_end > last_t {
        let (ta, tb) = (step_to_t((s - 1) as u32), step_to_t(s as u32));
        let alpha = if tb > ta { (t_end - ta) / (tb - ta) } else { 0.0 };
        if let Some(seg) = bone_at(rig, r_ball, state_at(rec, k, s - 1, alpha), bone, &mut segs) {
            out.push((seg.a, seg.b, 0.0));
        }
    }
    out
}

/// Projectile `id`'s centre at every step within `length_s` before `t_end` (base = tip), then at `t_end`.
fn proj_points(
    rec: &Recording2,
    id: u32,
    t_end: f64,
    length_s: f32,
    step_to_t: &dyn Fn(u32) -> f64,
) -> Vec<(V3, V3, f32)> {
    let n = rec.steps.len();
    let at = |i: usize| rec.steps[i].projs.iter().find(|p| p.0 == id).map(|p| p.1);
    let t_start = t_end - f64::from(length_s);
    let first = partition(n, |i| step_to_t(i as u32) < t_start);
    let mut out = Vec::new();
    for i in first..n {
        let t = step_to_t(i as u32);
        if t > t_end {
            let (Some(a), Some(b)) = (i.checked_sub(1).and_then(at), at(i)) else {
                break;
            };
            let ta = step_to_t(i as u32 - 1);
            let u = if t > ta { (t_end - ta) / (t - ta) } else { 0.0 };
            let p = std::array::from_fn(|c| a[c] + (b[c] - a[c]) * u);
            out.push((p, p, 0.0));
            break;
        }
        if let Some(p) = at(i) {
            out.push((p, p, (t_end - t) as f32));
        }
    }
    out
}

/// One vertex of a trail strip (`TVert` in trail.wgsl).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TrailVertex {
    /// Render space.
    pub pos: [f32; 3],
    /// 0 at the head, 1 at the tail.
    pub along: f32,
    /// HDR body colour, times the envelope.
    pub colour: [f32; 3],
    /// 0..1 across the strip (a smear: 0 at the base, 1 at the tip).
    pub across: f32,
    /// HDR core colour, times the envelope.
    pub core: [f32; 3],
    /// The style's code (0 ribbon, 1 smear, 2 lightning).
    pub style: f32,
}

impl TrailVertex {
    pub(crate) const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<TrailVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3, 1 => Float32, 2 => Float32x3, 3 => Float32, 4 => Float32x3, 5 => Float32
        ],
    };
}

/// The triangle list of one trail, appended to `out`. `points` are newest last (as `trail_points` gives them);
/// `env` scales its colour; `t` drives lightning's flicker.
pub fn build_strip(
    def: &TrailDef,
    points: &[(V3, V3, f32)],
    eye: [f32; 3],
    env: f32,
    seed: u32,
    t: f64,
    out: &mut Vec<TrailVertex>,
) {
    if points.len() < 2 || env <= 0.0 {
        return;
    }
    let len = def.length_s.max(1e-3);
    let style = match def.style {
        TrailStyle::Ribbon => 0.0,
        TrailStyle::Smear => 1.0,
        TrailStyle::Lightning => 2.0,
    };
    // Each sample becomes a pair of points across the strip.
    let mut pairs: Vec<([f32; 3], [f32; 3], f32)> = Vec::with_capacity(points.len());
    let flick = (t * 30.0).floor() as u32;
    for (i, &(base, tip, age)) in points.iter().enumerate() {
        let along = (age / len).clamp(0.0, 1.0);
        let (b, tp) = (to_render(base), to_render(tip));
        match def.style {
            TrailStyle::Smear => pairs.push((b, tp, along)),
            TrailStyle::Ribbon | TrailStyle::Lightning => {
                // Across the path, facing the eye.
                let prev = to_render(points[i.saturating_sub(1)].1);
                let next = to_render(points[(i + 1).min(points.len() - 1)].1);
                let dir = sub(next, prev);
                let side = normalize(cross(dir, sub(eye, tp)));
                let w = def.width.0 + (def.width.1 - def.width.0) * along;
                let mut c = tp;
                if def.style == TrailStyle::Lightning && i + 1 < points.len() {
                    let j = (h3(seed ^ flick, i as u32, 0) - 0.5) * 2.0 * def.width.0 * 1.5;
                    c = add(c, scale(side, j));
                }
                pairs.push((add(c, scale(side, -w / 2.0)), add(c, scale(side, w / 2.0)), along));
            }
        }
    }
    let flicker = if def.style == TrailStyle::Lightning {
        0.6 + 0.4 * h3(seed, flick, 1)
    } else {
        1.0
    };
    let k = env * flicker;
    let v = |p: [f32; 3], along: f32, across: f32| TrailVertex {
        pos: p,
        along,
        colour: def.colour.map(|c| c * k),
        across,
        core: def.core.map(|c| c * k),
        style,
    };
    for w in pairs.windows(2) {
        let (a0, a1, ua) = w[0];
        let (b0, b1, ub) = w[1];
        out.extend([
            v(a0, ua, 0.0),
            v(a1, ua, 1.0),
            v(b1, ub, 1.0),
            v(a0, ua, 0.0),
            v(b1, ub, 1.0),
            v(b0, ub, 0.0),
        ]);
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn normalize(a: [f32; 3]) -> [f32; 3] {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if l > 1e-6 { scale(a, 1.0 / l) } else { [1.0, 0.0, 0.0] }
}

/// What the trail pass needs to rebuild trails at a time: the recording, each fighter's rig and ball radius,
/// and the step-to-video-time map.
pub struct TrailSource<'a> {
    pub rec: &'a Recording2,
    pub rigs: [&'a Rig; 2],
    pub r_ball: [f64; 2],
    pub step_to_t: &'a dyn Fn(u32) -> f64,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CamU {
    view_proj: [[f32; 4]; 4],
    /// w: output pixels per art pixel (`FxCamera::art`).
    art: [f32; 4],
}

/// The trail pass: one pipeline, a reused vertex buffer.
pub struct Trails {
    pipeline: wgpu::RenderPipeline,
    cam: wgpu::Buffer,
    bind: wgpu::BindGroup,
    verts: Grow,
    data: Vec<TrailVertex>,
}

impl Trails {
    pub fn new(gpu: &Gpu) -> Trails {
        let device = &gpu.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("trails"),
            entries: &[pipe::entry(0, pipe::UNIFORM)],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("trails"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let m = pipe::module(gpu, "trails", include_str!("shaders/trail.wgsl"));
        let pipeline = pipe::pipeline(
            gpu,
            "trails",
            &pl,
            &m,
            wgpu::PrimitiveTopology::TriangleList,
            Blend::Add,
            &[Some(TrailVertex::LAYOUT)],
        );
        let cam = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("trails camera"),
            size: std::mem::size_of::<CamU>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("trails"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: cam.as_entire_binding(),
            }],
        });
        Trails {
            pipeline,
            cam,
            bind,
            verts: Grow::new(gpu, "trail vertices", wgpu::BufferUsages::VERTEX, 1 << 16),
            data: Vec::new(),
        }
    }

    /// Rebuilds the live trail records of `recs` at video time `t`.
    pub fn prepare(&mut self, gpu: &Gpu, camera: &FxCamera, src: &TrailSource<'_>, recs: &[&FxRecord], t: f64) {
        self.data.clear();
        for r in recs {
            let FxKind::Trail(def) = &r.kind else { continue };
            if !r.live_at(t) || def.k > 1 {
                continue;
            }
            let pts = trail_points(
                src.rec,
                src.rigs[def.k],
                src.r_ball[def.k],
                def.k,
                def.bone,
                t,
                def.length_s,
                src.step_to_t,
            );
            build_strip(def, &pts, camera.eye, r.env_at(t), r.seed, t, &mut self.data);
        }
        gpu.queue.write_buffer(
            &self.cam,
            0,
            bytemuck::bytes_of(&CamU {
                view_proj: camera.view_proj,
                art: [0.0, 0.0, 0.0, camera.art],
            }),
        );
        self.verts.write(gpu, bytemuck::cast_slice(&self.data));
    }

    pub fn encode(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.data.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.set_vertex_buffer(0, self.verts.buf.slice(..));
        pass.draw(0..self.data.len() as u32, 0..1);
    }

    pub fn draw(&mut self, ctx: &mut FxCtx<'_>, src: &TrailSource<'_>, recs: &[&FxRecord], t: f64) {
        self.prepare(ctx.gpu, &ctx.camera, src, recs, t);
        let mut pass = ctx.begin_pass("trails");
        self.encode(&mut pass);
    }
}

#[cfg(test)]
#[path = "trail_tests.rs"]
mod tests;
