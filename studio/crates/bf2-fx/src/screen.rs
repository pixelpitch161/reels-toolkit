//! Screen effects and motion blur.
//!
//! The chain runs each frame in HDR, between the scene (with its effects) and E2's tone map, in a fixed order:
//! shockwave refraction, radial blur, bloom (a dual-filter 6-level pyramid with procedural lens dirt), chroma,
//! speed lines, flash and invert, grade and desaturate, vignette, grain, letterbox. Three full-screen passes
//! carry it (warp, the bloom pyramid, composite); every one is stateless, so scrubbing is exact.
//!
//! Motion blur is sub-frame accumulation ([`subframes`], [`shutter_times`] and
//! [`bf2_render::Renderer::render_accum`]); camera shake moves the camera ([`shake_camera`]), not the image.

use bf2_render::maths::{self, V3};
use bf2_render::target::HDR_FORMAT;
use bf2_render::{Camera, FxCamera, Gpu, PostChain, PostCtx, to_render};
use bf2_sim::{Pose, Segment, solve};

use crate::hash::h3;
use crate::meshfx::grow;
use crate::pipe;
use crate::record::{Anchor, FxKind, FxRecord};
use crate::timeline::FxTimeline;
use crate::trail::{TrailSource, partition, state_at};

/// One screen effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScreenFx {
    /// The whole picture towards an HDR colour (no cap: `amount` 1 is solid colour).
    Flash {
        colour: [f32; 3],
    },
    Invert,
    /// 16 taps toward the point.
    RadialBlur {
        centre: Anchor,
    },
    /// A refraction ring from the point, growing `speed` picture heights per second.
    Shockwave {
        centre: Anchor,
        speed: f32,
    },
    /// Colour fringes growing away from the point.
    Chroma {
        centre: Anchor,
    },
    /// Streaks radiating from the point, fresh each frame.
    SpeedLines {
        centre: Anchor,
        colour: [f32; 3],
    },
    /// Bars top and bottom.
    Letterbox,
    /// The camera shakes by up to `amount` ring units ([`shake_camera`]).
    Shake,
    Grade {
        lift: [f32; 3],
        gamma: [f32; 3],
        gain: [f32; 3],
        saturation: f32,
    },
    /// Grey everything except hues near `keep`.
    Desaturate {
        keep: [f32; 3],
    },
    /// More bloom: `amount` adds to the bloom's intensity.
    Bloom,
    /// A cue for motion blur: `amount` 1 asks for all 8 sub-frames ([`subframes`]).
    MotionBlur,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScreenDef {
    pub effect: ScreenFx,
    /// Scales the effect (and is scaled by the record's envelope).
    pub amount: f32,
}

impl Default for ScreenDef {
    fn default() -> Self {
        ScreenDef {
            effect: ScreenFx::Flash { colour: [6.0; 3] },
            amount: 1.0,
        }
    }
}

/// The chain's standing look, with no effect live.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenLook {
    pub bloom: f32,
    pub threshold: f32,
    pub knee: f32,
    /// How much the lens dirt lights up in the bloom.
    pub dirt: f32,
    pub vignette: f32,
    pub grain: f32,
}

impl Default for ScreenLook {
    fn default() -> Self {
        ScreenLook {
            bloom: 0.1,
            threshold: 1.0,
            knee: 0.25,
            dirt: 0.9,
            // the tone map's grade vignettes the picture as the current engine does (`Post::ring`)
            vignette: 0.0,
            grain: 0.018,
        }
    }
}

/// How fast a shock ring's refraction is per unit of its `distort`.
const RING_PUSH: f32 = 0.03;
/// How hard a `Shockwave` pushes at `amount` 1.
const SHOCK_PUSH: f32 = 0.045;
const BLOOM_LEVELS: usize = 6;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenU {
    size: [f32; 4],
    flash: [f32; 4],
    misc: [f32; 4],
    radial: [f32; 4],
    chroma: [f32; 4],
    lines: [f32; 4],
    lines_col: [f32; 4],
    grade_lift: [f32; 4],
    grade_gamma: [f32; 4],
    grade_gain: [f32; 4],
    desat: [f32; 4],
    bloom: [f32; 4],
    shock: [[f32; 4]; 4],
    frame: [u32; 4],
}

/// Where a live effect sits: a sim-space point, or the picture centre.
#[derive(Clone, Copy, Debug)]
enum At {
    World([f64; 3]),
    Centre,
}

#[derive(Clone, Copy, Debug)]
enum Live {
    Fx {
        fx: ScreenFx,
        a: f32,
        age: f32,
        at: At,
    },
    /// A shock ring with `distort`: its world radius and push.
    Ring {
        at: [f64; 3],
        r: f32,
        push: f32,
    },
}

struct Level {
    w: u32,
    h: u32,
    view: wgpu::TextureView,
}

struct Sized {
    w: u32,
    h: u32,
    warped: wgpu::TextureView,
    levels: Vec<Level>,
    /// Prefilter (warped → level 0), then the downs (i−1 → i), then the ups (i → i−1).
    prefilter: wgpu::BindGroup,
    down: Vec<wgpu::BindGroup>,
    up: Vec<wgpu::BindGroup>,
    composite: wgpu::BindGroup,
}

/// The screen effect chain: a [`PostChain`]. [`Screen::set`] the live records, then render through it.
pub struct Screen {
    pub look: ScreenLook,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    blank: wgpu::TextureView,
    warp: wgpu::RenderPipeline,
    prefilter: wgpu::RenderPipeline,
    down: wgpu::RenderPipeline,
    up: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    sized: Option<Sized>,
    live: Vec<Live>,
    t: f64,
}

fn texture(gpu: &Gpu, label: &str, w: u32, h: u32) -> wgpu::TextureView {
    gpu.device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: HDR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

fn fullscreen(
    gpu: &Gpu,
    label: &str,
    layout: &wgpu::PipelineLayout,
    m: &wgpu::ShaderModule,
    fs: &str,
    additive: bool,
) -> wgpu::RenderPipeline {
    let add = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    };
    gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: m,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: m,
            entry_point: Some(fs),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: HDR_FORMAT,
                blend: additive.then_some(wgpu::BlendState { color: add, alpha: add }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn tex_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn pass<'e>(
    enc: &'e mut wgpu::CommandEncoder,
    label: &str,
    view: &wgpu::TextureView,
    clear: bool,
) -> wgpu::RenderPass<'e> {
    enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: if clear {
                    wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                } else {
                    wgpu::LoadOp::Load
                },
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

impl Screen {
    pub fn new(gpu: &Gpu) -> Screen {
        let src = [
            include_str!("shaders/screen_common.wgsl"),
            include_str!("shaders/shock.wgsl"),
            include_str!("shaders/radial.wgsl"),
            include_str!("shaders/bloom.wgsl"),
            include_str!("shaders/chroma.wgsl"),
            include_str!("shaders/lines.wgsl"),
            include_str!("shaders/flash.wgsl"),
            include_str!("shaders/grade.wgsl"),
        ]
        .join("\n");
        let m = pipe::module(gpu, "screen", &src);
        let layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("screen"),
            entries: &[
                pipe::entry(0, pipe::UNIFORM),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                tex_entry(2),
                tex_entry(3),
            ],
        });
        let pl = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("screen"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("screen"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniform = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screen"),
            size: std::mem::size_of::<ScreenU>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Screen {
            look: ScreenLook::default(),
            warp: fullscreen(gpu, "screen warp", &pl, &m, "fs_warp", false),
            prefilter: fullscreen(gpu, "bloom prefilter", &pl, &m, "fs_prefilter", false),
            down: fullscreen(gpu, "bloom down", &pl, &m, "fs_down", false),
            up: fullscreen(gpu, "bloom up", &pl, &m, "fs_up", true),
            composite: fullscreen(gpu, "screen composite", &pl, &m, "fs_composite", false),
            blank: texture(gpu, "screen blank", 1, 1),
            layout,
            sampler,
            uniform,
            sized: None,
            live: Vec::new(),
            t: 0.0,
        }
    }

    /// Takes the live screen records at video time `t` (each with its record anchor resolved, sim space) and
    /// the shock rings that distort; `resolve` places an effect's own centre. Only what is live at `t` counts.
    pub fn set(&mut self, recs: &[(&FxRecord, [f64; 3])], resolve: &dyn Fn(&Anchor) -> [f64; 3], t: f64) {
        self.live.clear();
        self.t = t;
        let at = |a: &Anchor| match a {
            Anchor::Screen => At::Centre,
            a => At::World(resolve(a)),
        };
        for (r, pos) in recs {
            if !r.live_at(t) {
                continue;
            }
            let (env, age) = (r.env_at(t), r.age(t));
            match &r.kind {
                FxKind::Screen(d) => {
                    let centre = match &d.effect {
                        ScreenFx::RadialBlur { centre }
                        | ScreenFx::Shockwave { centre, .. }
                        | ScreenFx::Chroma { centre }
                        | ScreenFx::SpeedLines { centre, .. } => at(centre),
                        _ => At::Centre,
                    };
                    self.live.push(Live::Fx {
                        fx: d.effect,
                        a: d.amount * env,
                        age,
                        at: centre,
                    });
                }
                FxKind::Ring(d) if d.distort > 0.0 => self.live.push(Live::Ring {
                    at: *pos,
                    r: grow(d.r, r.life, age, r.exit, env),
                    push: RING_PUSH * d.distort * env,
                }),
                _ => {}
            }
        }
    }

    fn uniform(&self, cam: &FxCamera) -> ScreenU {
        let [w, h] = cam.size;
        let (wf, hf) = (w as f32, h as f32);
        let lk = &self.look;
        let mut u = ScreenU {
            size: [wf, hf, 1.0 / wf, 1.0 / hf],
            misc: [0.0, 0.0, lk.vignette, lk.grain],
            bloom: [lk.bloom, lk.threshold, lk.knee, lk.dirt],
            grade_gamma: [1.0, 1.0, 1.0, 1.0],
            grade_gain: [1.0; 4],
            frame: [(self.t * 30.0).round().max(0.0) as u32, 0, 0, 0],
            ..ScreenU::default()
        };
        let uv = |a: At| -> Option<[f32; 2]> {
            match a {
                At::Centre => Some([0.5, 0.5]),
                At::World(p) => {
                    let c = maths::transform_point(&cam.view_proj, to_render(p));
                    (c[3] > 1e-4).then(|| [c[0] / c[3] * 0.5 + 0.5, 0.5 - c[1] / c[3] * 0.5])
                }
            }
        };
        // Where several of one kind are live, the strongest sets the centre and the amounts add (capped).
        let mut best = [0.0f32; 4];
        let mut flash_w = 0.0f32;
        let mut flash_c = [0.0f32; 3];
        let mut grade_w = 0.0f32;
        let mut shocks: Vec<[f32; 4]> = Vec::new();
        for l in &self.live {
            match *l {
                Live::Ring { at, r, push } => {
                    let p = to_render(at);
                    let dist = maths::length(maths::sub(p, cam.eye)).max(cam.near);
                    if let Some(c) = uv(At::World(at)) {
                        let r_pic = r / (dist * 2.0 * (cam.fov_y / 2.0).tan());
                        shocks.push([c[0], c[1], r_pic, push]);
                    }
                }
                Live::Fx { fx, a, age, at } => {
                    let a = a.max(0.0);
                    let point = |slot: &mut [f32; 4], k: usize, best: &mut [f32; 4]| {
                        if let Some(c) = uv(at) {
                            if a > best[k] {
                                best[k] = a;
                                slot[0] = c[0];
                                slot[1] = c[1];
                            }
                            slot[2] = (slot[2] + a).min(2.0);
                        }
                    };
                    match fx {
                        ScreenFx::Flash { colour } => {
                            flash_w += a;
                            for (fc, c) in flash_c.iter_mut().zip(colour) {
                                *fc += c * a;
                            }
                        }
                        ScreenFx::Invert => u.misc[0] = (u.misc[0] + a).min(1.0),
                        ScreenFx::RadialBlur { .. } => point(&mut u.radial, 0, &mut best),
                        ScreenFx::Chroma { .. } => point(&mut u.chroma, 1, &mut best),
                        ScreenFx::SpeedLines { colour, .. } => {
                            let before = best[2];
                            point(&mut u.lines, 2, &mut best);
                            if best[2] > before {
                                u.lines_col = [colour[0], colour[1], colour[2], 0.0];
                            }
                        }
                        ScreenFx::Shockwave { speed, .. } => {
                            if let Some(c) = uv(at) {
                                shocks.push([c[0], c[1], speed * age, SHOCK_PUSH * a]);
                            }
                        }
                        ScreenFx::Letterbox => u.misc[1] = u.misc[1].max(a),
                        ScreenFx::Shake | ScreenFx::MotionBlur => {}
                        ScreenFx::Grade {
                            lift,
                            gamma,
                            gain,
                            saturation,
                        } => {
                            if a > grade_w {
                                grade_w = a;
                                u.grade_lift = [lift[0], lift[1], lift[2], a.min(1.0)];
                                u.grade_gamma = [gamma[0], gamma[1], gamma[2], saturation];
                                u.grade_gain = [gain[0], gain[1], gain[2], 1.0];
                            }
                        }
                        ScreenFx::Desaturate { keep } => {
                            if a > best[3] {
                                best[3] = a;
                                u.desat = [keep[0], keep[1], keep[2], a.min(1.0)];
                            }
                        }
                        ScreenFx::Bloom => u.bloom[0] += a * lk.bloom,
                    }
                }
            }
        }
        if flash_w > 0.0 {
            let c = flash_c.map(|x| x / flash_w);
            u.flash = [c[0], c[1], c[2], flash_w.min(1.0)];
        }
        // The strongest four shockwaves.
        shocks.sort_by(|a, b| b[3].total_cmp(&a[3]));
        for (slot, s) in u.shock.iter_mut().zip(&shocks) {
            *slot = *s;
        }
        u.frame[1] = shocks.len().min(4) as u32;
        u
    }

    fn bind(&self, gpu: &Gpu, a: &wgpu::TextureView, b: &wgpu::TextureView) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("screen"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(a),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(b),
                },
            ],
        })
    }

    fn ensure_size(&mut self, gpu: &Gpu, w: u32, h: u32) {
        if self.sized.as_ref().is_some_and(|s| s.w == w && s.h == h) {
            return;
        }
        let warped = texture(gpu, "screen warped", w, h);
        let mut levels = Vec::with_capacity(BLOOM_LEVELS);
        let (mut lw, mut lh) = (w, h);
        for _ in 0..BLOOM_LEVELS {
            lw = lw.div_ceil(2).max(1);
            lh = lh.div_ceil(2).max(1);
            levels.push(Level {
                w: lw,
                h: lh,
                view: texture(gpu, "bloom level", lw, lh),
            });
        }
        let prefilter = self.bind(gpu, &warped, &self.blank);
        let down = (1..BLOOM_LEVELS)
            .map(|i| self.bind(gpu, &levels[i - 1].view, &self.blank))
            .collect();
        let up = (1..BLOOM_LEVELS)
            .map(|i| self.bind(gpu, &levels[i].view, &self.blank))
            .collect();
        let composite = self.bind(gpu, &warped, &levels[0].view);
        self.sized = Some(Sized {
            w,
            h,
            warped,
            levels,
            prefilter,
            down,
            up,
            composite,
        });
    }
}

impl PostChain for Screen {
    fn run(&mut self, ctx: &mut PostCtx<'_>) {
        let [w, h] = ctx.camera.size;
        self.ensure_size(ctx.gpu, w, h);
        let u = self.uniform(&ctx.camera);
        ctx.gpu.queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&u));
        // The input differs between plain and accumulated frames, so its bind group is made per frame.
        let input = self.bind(ctx.gpu, ctx.input, &self.blank);
        let s = self.sized.as_ref().expect("sized above");
        let draw = |enc: &mut wgpu::CommandEncoder,
                    label: &str,
                    view: &wgpu::TextureView,
                    pipe: &wgpu::RenderPipeline,
                    bind: &wgpu::BindGroup,
                    clear: bool| {
            let mut p = pass(enc, label, view, clear);
            p.set_pipeline(pipe);
            p.set_bind_group(0, bind, &[]);
            p.draw(0..3, 0..1);
        };
        // 1–2: shockwave refraction and radial blur.
        draw(ctx.encoder, "screen warp", &s.warped, &self.warp, &input, true);
        // 3: bloom, down then up.
        draw(
            ctx.encoder,
            "bloom prefilter",
            &s.levels[0].view,
            &self.prefilter,
            &s.prefilter,
            true,
        );
        for i in 1..BLOOM_LEVELS {
            draw(
                ctx.encoder,
                "bloom down",
                &s.levels[i].view,
                &self.down,
                &s.down[i - 1],
                true,
            );
        }
        for i in (1..BLOOM_LEVELS).rev() {
            debug_assert!(s.levels[i].w <= s.levels[i - 1].w && s.levels[i].h <= s.levels[i - 1].h);
            draw(
                ctx.encoder,
                "bloom up",
                &s.levels[i - 1].view,
                &self.up,
                &s.up[i - 1],
                false,
            );
        }
        // 4–10: chroma, speed lines, flash and invert, grade and desaturate, vignette, grain, letterbox.
        draw(
            ctx.encoder,
            "screen composite",
            ctx.output,
            &self.composite,
            &s.composite,
            true,
        );
    }
}

/// `cam` moved by the live `Shake` records at `t`: a smooth, seeded wobble across the picture of up to `amount`
/// ring units (times the envelope). Eye and target move together, so the view turns no further.
pub fn shake_camera(cam: &Camera, recs: &[&FxRecord], t: f64) -> Camera {
    let axes = FxCamera::of(cam, 1, 1);
    let mut off: V3 = [0.0; 3];
    for r in recs {
        let FxKind::Screen(ScreenDef {
            effect: ScreenFx::Shake,
            amount,
        }) = &r.kind
        else {
            continue;
        };
        if !r.live_at(t) {
            continue;
        }
        let a = amount * r.env_at(t);
        let age = r.age(t);
        let wob = |k: u32| -> f32 {
            // Three detuned sines with hashed phases, about 9–23 Hz.
            (0..3u32)
                .map(|j| {
                    let f = 9.0 + 7.0 * j as f32 + 3.0 * h3(r.seed, k * 3 + j, 0);
                    let ph = std::f32::consts::TAU * h3(r.seed, k * 3 + j, 1);
                    (std::f32::consts::TAU * f * age + ph).sin() / (1.0 + j as f32 * 0.6)
                })
                .sum::<f32>()
                / 1.9
        };
        off = maths::add(off, maths::scale(axes.right, a * wob(0)));
        off = maths::add(off, maths::scale(axes.up, a * wob(1)));
    }
    Camera {
        eye: maths::add(cam.eye, off),
        target: maths::add(cam.target, off),
        ..*cam
    }
}

/// The video frame rate the blur measures speed over.
const FRAME: f64 = 1.0 / 30.0;
/// A body or bone tip moving this far in a frame gets all 8 sub-frames.
const FAST: f64 = 0.4;

/// Fighter `k`'s ball centre and every bone tip at video time `t` (sim space).
fn points_at(src: &TrailSource<'_>, k: usize, t: f64, segs: &mut Vec<Segment>) -> Vec<[f64; 3]> {
    let n = src.rec.steps.len();
    if n == 0 {
        return Vec::new();
    }
    let s = partition(n, |i| (src.step_to_t)(i as u32) <= t);
    let (i, alpha) = if s == 0 {
        (0, 0.0)
    } else if s >= n {
        (n - 1, 0.0)
    } else {
        let (ta, tb) = ((src.step_to_t)((s - 1) as u32), (src.step_to_t)(s as u32));
        (s - 1, if tb > ta { (t - ta) / (tb - ta) } else { 0.0 })
    };
    let (pos, facing, joints) = state_at(src.rec, k, i, alpha);
    let r = src.r_ball[k];
    let centre = [pos[0], pos[1], pos[2] + r];
    solve(src.rigs[k], &Pose { joints }, centre, r, facing, segs);
    std::iter::once(centre).chain(segs.iter().map(|g| g.b)).collect()
}

/// How many sub-frames frame `t` needs: 1 when nothing moves fast, rising to 8 when a body or bone tip moves
/// [`FAST`] ring units or more over the frame, or as a live `MotionBlur` cue asks.
pub fn subframes(src: &TrailSource<'_>, t: f64, timeline: &FxTimeline) -> u32 {
    let mut segs = Vec::new();
    let mut d = 0.0f64;
    for k in 0..2 {
        let a = points_at(src, k, t - FRAME, &mut segs);
        let b = points_at(src, k, t, &mut segs);
        for (p, q) in a.iter().zip(&b) {
            let dd = ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt();
            d = d.max(dd);
        }
    }
    let mut n = 1 + (7.0 * (d / FAST).min(1.0)).floor() as u32;
    let mut live = Vec::new();
    timeline.active(t, &mut live);
    for &i in &live {
        let r = &timeline.records[i];
        if let FxKind::Screen(ScreenDef {
            effect: ScreenFx::MotionBlur,
            amount,
        }) = &r.kind
        {
            // A cue holds for its whole life (no envelope: sub-frames don't fade).
            let cue = 1 + (7.0 * amount.clamp(0.0, 1.0)).round() as u32;
            n = n.max(cue);
        }
    }
    n.clamp(1, 8)
}

/// The `n` sub-frame times of frame `t`: a 180° shutter (half of a 1/30 s frame) centred on `t`.
pub fn shutter_times(t: f64, n: u32) -> Vec<f64> {
    let n = n.max(1);
    let open = FRAME / 2.0;
    (0..n)
        .map(|i| t + (f64::from(i) + 0.5) / f64::from(n) * open - open / 2.0)
        .collect()
}

#[cfg(test)]
#[path = "screen_tests.rs"]
mod tests;
