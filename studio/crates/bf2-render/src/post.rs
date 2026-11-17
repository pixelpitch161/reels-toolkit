//! The post stack: exposure, the tone map (ACES fitted, Narkowicz 2015), the sRGB encode, then the current
//! engine's grade on the encoded picture (contrast, saturation, vignette, top scrim; `bf_picture::post`'s
//! `grade`, `vignette` and `top_scrim` without their dither), from the resolved HDR colour into the 8-bit picture.
//! Bloom, flashes and the rest are E3.

use crate::gpu::Gpu;
use crate::target::{LDR_FORMAT, Targets};

/// The post settings of one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Post {
    /// Multiplies the HDR colour before the tone map.
    pub exposure: f32,
    /// The grade's S-curve: `c + contrast * (smoothstep(c) - c)` on the encoded 0..1 value.
    pub contrast: f32,
    /// The grade's saturation about the channels' mean (1: none).
    pub saturation: f32,
    /// The vignette's strength (0: none): `1 - min(strength * clamp(r - 0.55, 0, 1), 0.5)`, `r` the distance from
    /// (0.5 w, 0.47 h) in units of 0.62 of the width and height.
    pub vignette: f32,
    /// The top scrim (0: none): darkens by this much above 100/640 of the height, fading out over 90/640.
    pub scrim: f32,
    /// The curve: 0 is ACES fitted and the sRGB encode; above 0, the current engine's straight light (no curve
    /// below this knee, a soft shoulder from it to white) and its 2.2 gamma, so a colour lit to `(c / 255)^2.2`
    /// comes out as `c` (E7).
    pub knee: f32,
    /// The pixel pass (E8): output pixels per art pixel across a 1080-wide frame, so 3 draws the current engine's
    /// 360 x 640 grid. Each art pixel takes the mean HDR colour of the output pixels it covers (a box filter: thin
    /// lines stay whole, as faint lines), and every output pixel inside it is that one
    /// colour (nearest neighbour), whatever came before: the scene, the effects' glow and the screen chain all
    /// land on the grid. A narrower frame keeps the same art grid (1.5 output pixels each at 540 wide), never
    /// finer than its own pixels. 0 or 1: off.
    pub pixel: u32,
    /// The value steps after the grade, on 0..255 (0: none): `floor(v / step + bayer) * step` with the 4 x 4
    /// Bayer matrix on the art grid, the current engine's `np.floor(col / 5 + BAYER_FULL) * 5`.
    pub step: f32,
}

impl Default for Post {
    /// Neutral: the tone map alone.
    fn default() -> Self {
        Post {
            exposure: 1.0,
            contrast: 0.0,
            saturation: 1.0,
            vignette: 0.0,
            scrim: 0.0,
            knee: 0.0,
            pixel: 1,
            step: 0.0,
        }
    }
}

impl Post {
    /// The fight's look: the current engine's grade (`grade(contrast=0.18, sat=1.18)`, `vignette(0.45)`,
    /// `top_scrim(0.45, 100, 90)` of `bf_picture::ring::frame`) after the tone map, at the exposure the stage is
    /// lit for.
    pub fn ring() -> Post {
        Post {
            exposure: RING_EXPOSURE,
            contrast: 0.18,
            saturation: 1.18,
            vignette: 0.45,
            scrim: 0.45,
            knee: RING_KNEE,
            pixel: RING_PIXEL,
            step: RING_STEP,
        }
    }
}

/// Where [`Post::ring`]'s shoulder starts.
pub const RING_KNEE: f32 = 0.7;

/// The exposure of [`Post::ring`].
pub const RING_EXPOSURE: f32 = 1.0;

/// [`Post::ring`]'s pixel size: 1080 x 1920 is drawn as 360 x 640 art pixels, as the current engine draws it.
pub const RING_PIXEL: u32 = 3;

/// [`Post::ring`]'s value step: the current engine's canvas and model shading step of 5.
pub const RING_STEP: f32 = 5.0;

/// The CPU reference of the pass's value steps: `v` an encoded 0..1 value at art pixel `q`, stepped by `step`
/// (on 0..255) with the 4 x 4 Bayer matrix (`bf_picture::config::BAYER`), 0..1. No step leaves it as it is.
pub fn dither_cpu(v: f32, step: f32, q: [u32; 2]) -> f32 {
    if step <= 0.0 {
        return v;
    }
    let b = bf_picture::config::BAYER[(q[1] % 4) as usize][(q[0] % 4) as usize] as f32;
    ((v * 255.0 / step + b).floor() * step).clamp(0.0, 255.0) / 255.0
}

/// The CPU reference of the grade the shader applies after the encode: `c` an encoded 0..1 colour at pixel
/// `(x, y)` (centres) of a `w x h` picture.
pub fn grade_cpu(c: [f32; 3], x: f32, y: f32, w: f32, h: f32, post: &Post) -> [f32; 3] {
    let s = c.map(|v| {
        let v = v.clamp(0.0, 1.0);
        v + post.contrast * (v * v * (3.0 - 2.0 * v) - v)
    });
    let g = (s[0] + s[1] + s[2]) / 3.0;
    let r = ((x - 0.5 * w) / (0.62 * w)).hypot((y - 0.47 * h) / (0.62 * h));
    let vig = 1.0 - ((r - 0.55).clamp(0.0, 1.0) * post.vignette).min(0.5);
    let scrim = 1.0 - (1.0 - (y - h * 100.0 / 640.0) / (h * 90.0 / 640.0)).clamp(0.0, 1.0) * post.scrim;
    s.map(|v| ((g + (v - g) * post.saturation).clamp(0.0, 1.0) * vig * scrim).clamp(0.0, 1.0))
}

/// The CPU reference of the shader's curve for one channel: exposure, ACES fitted, sRGB encode, to 0..255.
pub fn tonemap_cpu(hdr: f32, exposure: f32) -> u8 {
    let v = srgb_encode(aces((hdr * exposure).max(0.0)));
    (v * 255.0).round() as u8
}

/// The CPU reference of the shader's curve and encode for one channel (before the grade), 0..1.
pub fn curve_cpu(hdr: f32, post: &Post) -> f32 {
    let x = (hdr * post.exposure).max(0.0);
    if post.knee <= 0.0 {
        return srgb_encode(aces(x));
    }
    shoulder(x, post.knee).powf(1.0 / 2.2)
}

/// Straight up to `k`, then rolling off to 1.
fn shoulder(x: f32, k: f32) -> f32 {
    if x <= k {
        x
    } else {
        k + (1.0 - k) * (1.0 - (-(x - k) / (1.0 - k)).exp())
    }
}

/// Narkowicz's fit of the ACES curve, clamped to 0..1.
fn aces(x: f32) -> f32 {
    ((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)).clamp(0.0, 1.0)
}

fn srgb_encode(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// The uniform the shader reads (`PostU`).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PostU {
    exposure: f32,
    contrast: f32,
    saturation: f32,
    vignette: f32,
    scrim: f32,
    knee: f32,
    pixel: f32,
    step: f32,
}

/// The tone map pass: built once, its uniform rewritten each frame.
pub struct PostPass {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
}

impl PostPass {
    pub fn new(gpu: &Gpu) -> PostPass {
        let device = &gpu.device;
        let module = device.create_shader_module(wgpu::include_wgsl!("shaders/tonemap.wgsl"));
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tonemap"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tonemap"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("tonemap"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(LDR_FORMAT.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tonemap uniform"),
            size: std::mem::size_of::<PostU>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        PostPass {
            pipeline,
            layout,
            uniform,
        }
    }

    /// The bind group reading `t`'s resolved HDR colour. Made once per `Targets`.
    pub fn bind(&self, gpu: &Gpu, t: &Targets) -> wgpu::BindGroup {
        self.bind_view(gpu, &t.hdr_resolved)
    }

    /// The bind group reading any single-sample HDR view of the targets' size.
    pub fn bind_view(&self, gpu: &Gpu, view: &wgpu::TextureView) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tonemap"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.uniform.as_entire_binding(),
                },
            ],
        })
    }

    /// Records the tone map from the view `bind` reads into `t.ldr`. The uniform is written now, so one `PostPass`
    /// serves one frame per submit.
    pub fn encode(&self, gpu: &Gpu, enc: &mut wgpu::CommandEncoder, t: &Targets, bind: &wgpu::BindGroup, post: &Post) {
        let u = PostU {
            exposure: post.exposure,
            contrast: post.contrast,
            saturation: post.saturation,
            vignette: post.vignette,
            scrim: post.scrim,
            knee: post.knee,
            pixel: post.pixel as f32,
            step: post.step,
        };
        gpu.queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&u));
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("tonemap"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &t.ldr,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::read_back;

    fn clear_hdr(gpu: &Gpu, t: &Targets, c: [f64; 3]) -> wgpu::CommandEncoder {
        let mut enc = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &t.hdr_msaa,
                depth_slice: None,
                resolve_target: Some(&t.hdr_resolved),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: c[0],
                        g: c[1],
                        b: c[2],
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        enc
    }

    #[test]
    fn clear_colour_reads_back() {
        let gpu = Gpu::headless().unwrap();
        let t = Targets::new(&gpu, 270, 480);
        let pass = PostPass::new(&gpu);
        let bind = pass.bind(&gpu, &t);
        // Exactly representable in f16, so the HDR target stores them as they are.
        let c = [0.25, 0.75, 3.5];
        let post = Post::default();
        let mut enc = clear_hdr(&gpu, &t, c);
        pass.encode(&gpu, &mut enc, &t, &bind, &post);
        gpu.queue.submit([enc.finish()]);
        let img = read_back(&gpu, &t);
        assert_eq!((img.w, img.h, img.c), (270, 480, 3));
        let want: Vec<u8> = c.iter().map(|&v| tonemap_cpu(v as f32, 1.0)).collect();
        for (i, px) in img.data.as_chunks::<3>().0.iter().enumerate() {
            for k in 0..3 {
                assert!(
                    px[k].abs_diff(want[k]) <= 1,
                    "pixel {i} channel {k}: {} against {}",
                    px[k],
                    want[k]
                );
            }
        }
    }

    #[test]
    fn neutral_grade_changes_nothing() {
        let p = Post::default();
        for c in [[0.0, 0.5, 1.0], [0.2, 0.3, 0.9]] {
            for (x, y) in [(0.5, 0.5), (539.5, 959.5), (1079.5, 1919.5)] {
                let g = grade_cpu(c, x, y, 1080.0, 1920.0, &p);
                assert!(g.iter().zip(c).all(|(a, b)| (a - b).abs() < 1e-6), "{g:?} {c:?}");
            }
        }
    }

    #[test]
    fn ring_grade_is_the_current_engines() {
        let p = Post::ring();
        // the middle of the picture: only the curve and the saturation (bf_picture::post::grade)
        let c = [0.2f32, 0.4, 0.8];
        let got = grade_cpu(c, 540.0, 0.47 * 1920.0, 1080.0, 1920.0, &p);
        let img = bf_picture::img::Img::from_vec(1, 1, 3, c.iter().map(|&v| f64::from(v) * 255.0).collect());
        let want = bf_picture::post::grade(&img, 0.18, 1.18, 0.0, 1.0);
        for k in 0..3 {
            assert!(
                (f64::from(got[k]) * 255.0 - want.data[k]).abs() < 0.01,
                "{got:?} {:?}",
                want.data
            );
        }
        // a corner darkens by the vignette's cap, the top by the scrim
        let corner = grade_cpu([0.5; 3], 0.5, 1919.5, 1080.0, 1920.0, &p);
        assert!(corner[0] < 0.4, "{corner:?}");
        let top = grade_cpu([0.5; 3], 540.0, 10.0, 1080.0, 1920.0, &p);
        let mid = grade_cpu([0.5; 3], 540.0, 0.47 * 1920.0, 1080.0, 1920.0, &p);
        assert!(top[0] < 0.6 * mid[0], "{top:?} {mid:?}");
    }

    #[test]
    fn ring_curve_gives_back_the_current_engines_colours() {
        let p = Post::ring();
        // below the knee a colour lit to (c / 255)^2.2 comes out as c: the shadows are not crushed
        for c in [6.0f32, 30.0, 90.0, 180.0] {
            let lin = (c / 255.0).powf(2.2) / p.exposure;
            assert!((curve_cpu(lin, &p) * 255.0 - c).abs() < 0.05, "{c}");
        }
        // above it the shoulder rolls off to white, never past it, always rising
        let mut last = 0.0;
        for i in 0..=400 {
            let v = curve_cpu(i as f32 / 40.0, &p);
            assert!(v >= last && v <= 1.0, "{v} after {last}");
            last = v;
        }
        assert!(last > 0.99);
        // the default stays ACES
        assert_eq!(curve_cpu(0.5, &Post::default()), srgb_encode(aces(0.5)));
    }

    #[test]
    fn dither_lands_on_the_steps_and_keeps_the_mean() {
        // the current engine's np.floor(v / 5 + BAYER_FULL) * 5, on 0..1 values
        for v in [0.0f32, 0.2, 0.4, 0.5, 0.731, 0.99, 1.0] {
            let mut sum = 0.0;
            for y in 0..4 {
                for x in 0..4 {
                    let q = dither_cpu(v, 5.0, [x, y]);
                    let b = (q * 255.0).round();
                    assert!((q * 255.0 - b).abs() < 1e-3 && b as u32 % 5 == 0, "{v} -> {q}");
                    assert!((0.0..=1.0).contains(&q));
                    let want = ((f64::from(v) * 255.0 / 5.0 + bf_picture::config::BAYER[y as usize][x as usize])
                        .floor()
                        * 5.0)
                        .min(255.0);
                    assert_eq!(b, want as f32, "{v} at ({x}, {y})");
                    sum += q;
                }
            }
            // a 4 x 4 tile averages back to the value (to within a sixteenth of a step)
            if v < 0.98 {
                assert!((sum / 16.0 - v).abs() <= 5.0 / 255.0 / 2.0, "{v}: mean {}", sum / 16.0);
            }
        }
        // no steps: unchanged
        assert_eq!(dither_cpu(0.4321, 0.0, [1, 2]), 0.4321);
    }

    #[test]
    fn pixel_pass_dithers_a_flat_colour_in_the_bayer_pattern() {
        let gpu = Gpu::headless().unwrap();
        let t = Targets::new(&gpu, 1080, 48);
        let pass = PostPass::new(&gpu);
        let bind = pass.bind(&gpu, &t);
        // the straight curve, no grade: a flat value whose steps sit well clear of the dither's thresholds
        let post = Post {
            pixel: 3,
            step: 5.0,
            knee: 0.7,
            ..Post::default()
        };
        let c = 0.1875; // exact in f16
        let v = curve_cpu(c, &post);
        let frac = (v * 255.0 / 5.0).fract();
        assert!((0..16).all(|k| (frac + k as f32 / 16.0).fract() > 0.01 && (frac + k as f32 / 16.0).fract() < 0.99));
        let mut enc = clear_hdr(&gpu, &t, [c as f64; 3]);
        pass.encode(&gpu, &mut enc, &t, &bind, &post);
        gpu.queue.submit([enc.finish()]);
        let img = read_back(&gpu, &t);
        let mut seen = std::collections::BTreeSet::new();
        for y in 0..img.h {
            for x in 0..img.w {
                let q = [(x / 3) as u32, (y / 3) as u32];
                let want = (dither_cpu(v, 5.0, [q[0] % 4, q[1] % 4]) * 255.0).round() as u8;
                assert_eq!(img.px(x, y)[0], want, "pixel ({x}, {y}), art pixel {q:?}");
                seen.insert(want);
            }
        }
        assert_eq!(seen.len(), 2, "a flat value dithers between two neighbouring steps: {seen:?}");
    }

    /// One output column in three lit (`x % 3 == 1`, HDR 1.0), the rest black, drawn into the resolved HDR.
    fn thin_lines(gpu: &Gpu, t: &Targets) -> wgpu::CommandEncoder {
        let module = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("thin lines"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}
@fragment
fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let on = select(0.0, 1.0, u32(p.x) % 3u == 1u);
    return vec4<f32>(on, on, on, 1.0);
}
"#
                .into(),
            ),
        });
        let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("thin lines"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(crate::target::HDR_FORMAT.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let mut enc = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("thin lines"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.hdr_resolved,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.draw(0..3, 0..1);
        }
        enc
    }

    #[test]
    fn an_art_pixel_is_the_mean_of_what_it_covers() {
        let gpu = Gpu::headless().unwrap();
        let t = Targets::new(&gpu, 1080, 24);
        let pass = PostPass::new(&gpu);
        let bind = pass.bind(&gpu, &t);
        let post = Post {
            pixel: 3,
            knee: 0.7,
            ..Post::default()
        };
        let mut enc = thin_lines(&gpu, &t);
        pass.encode(&gpu, &mut enc, &t, &bind, &post);
        gpu.queue.submit([enc.finish()]);
        let img = read_back(&gpu, &t);
        // a line a third of an art pixel wide stays, as a third of its light: it does not break up or vanish
        let want = (curve_cpu(1.0 / 3.0, &post) * 255.0).round() as u8;
        for y in 0..img.h {
            for x in 0..img.w {
                let got = img.px(x, y)[0];
                assert!(got.abs_diff(want) <= 1, "({x}, {y}): {got} against {want}");
            }
        }
    }

    #[test]
    fn tonemap_is_monotonic() {
        let mut last = 0u8;
        assert_eq!(tonemap_cpu(0.0, 1.0), 0);
        for i in 0..=1600 {
            let v = tonemap_cpu(i as f32 / 100.0, 1.0);
            assert!(v >= last, "{v} after {last} at {}", i as f32 / 100.0);
            last = v;
        }
        assert!(last >= 254, "16.0 maps to {last}");
        assert!(tonemap_cpu(0.18, 1.0) > 60 && tonemap_cpu(0.18, 1.0) < 160);
    }
}
