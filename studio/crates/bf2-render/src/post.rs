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
        }
    }
}

/// The exposure of [`Post::ring`].
pub const RING_EXPOSURE: f32 = 1.0;

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
    _pad: [f32; 3],
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
            _pad: [0.0; 3],
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
