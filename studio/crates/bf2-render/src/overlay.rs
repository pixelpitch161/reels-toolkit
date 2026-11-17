//! The UI overlay (Engine 2 E5): the current engine's CPU UI layer (HUD, captions, count-in, VS card) drawn at
//! 360 x 640, turned into a premultiplied alpha matte, scaled up exactly 3x with nearest sampling and composited
//! over the GPU frame after the tone map.
//!
//! The UI is drawn twice, once over black (`B`) and once over white (`Wt`); per pixel `α = 1 − mean_c(Wt_c −
//! B_c)/255`, clamped to [0, 1], with `B` the premultiplied colour ([`matte`]). That recovers every draw that is a
//! linear blend of the background with ink, which is what the UI layer's draws are. A frame the VS card fills
//! is taken whole over black ([`opaque`]).

use bf_picture::img::Rgb8;

use crate::gpu::Gpu;
use crate::target::{LDR_FORMAT, Targets};

/// A premultiplied RGBA UI layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overlay {
    /// Its width (360 for the video).
    pub w: u32,
    /// Its height (640 for the video).
    pub h: u32,
    /// Premultiplied colour and alpha, row by row.
    pub rgba: Vec<[u8; 4]>,
}

impl Overlay {
    /// A transparent layer.
    pub fn empty(w: u32, h: u32) -> Overlay {
        Overlay {
            w,
            h,
            rgba: vec![[0; 4]; (w * h) as usize],
        }
    }

    /// Whether every pixel is fully transparent.
    pub fn is_clear(&self) -> bool {
        self.rgba.iter().all(|p| *p == [0; 4])
    }
}

/// The matte of a UI layer drawn over black (`over_black`) and over white (`over_white`): `α = 1 − mean_c(Wt_c −
/// B_c)/255` clamped to [0, 1] (rounded to a byte), with the colour over black as the premultiplied colour.
///
/// # Panics
/// The two pictures are not the same size, or not RGB.
pub fn matte(over_black: &Rgb8, over_white: &Rgb8) -> Overlay {
    assert_eq!(
        (over_black.w, over_black.h),
        (over_white.w, over_white.h),
        "two pictures of one size"
    );
    assert!(over_black.c == 3 && over_white.c == 3, "RGB pictures");
    let rgba = over_black
        .data
        .as_chunks::<3>()
        .0
        .iter()
        .zip(over_white.data.as_chunks::<3>().0.iter())
        .map(|(b, wt)| {
            let through: i32 = (0..3).map(|c| i32::from(wt[c]) - i32::from(b[c])).sum();
            // α·255 = 255 − mean(Wt − B), rounded half up in integers: (3·255 − Σ + 1) div 3
            let a = ((3 * 255 - through).clamp(0, 3 * 255) + 1) / 3;
            [b[0], b[1], b[2], a as u8]
        })
        .collect();
    Overlay {
        w: over_black.w as u32,
        h: over_black.h as u32,
        rgba,
    }
}

/// `img` as a layer that covers everything (α 255).
pub fn opaque(img: &Rgb8) -> Overlay {
    assert_eq!(img.c, 3, "an RGB picture");
    Overlay {
        w: img.w as u32,
        h: img.h as u32,
        rgba: img
            .data
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| [p[0], p[1], p[2], 255])
            .collect(),
    }
}

/// The integer scale of a `fw x fh` frame over `ov`.
///
/// # Panics
/// The frame is not an exact integer multiple of the layer, the same on both axes.
fn scale_of(fw: u32, fh: u32, ov: &Overlay) -> u32 {
    let s = fw / ov.w.max(1);
    assert!(
        s >= 1 && fw == s * ov.w && fh == s * ov.h,
        "a {fw} x {fh} frame is not a whole multiple of the {} x {} overlay",
        ov.w,
        ov.h
    );
    s
}

/// `ov` over `frame` on the CPU, as the GPU pass does it: nearest-sampled at the frame's integer scale, `out =
/// ov.rgb + round((255 − α)·dst/255)`, saturating.
///
/// # Panics
/// As [`scale_of`].
pub fn composite_cpu(frame: &mut Rgb8, ov: &Overlay) {
    let s = scale_of(frame.w as u32, frame.h as u32, ov) as usize;
    let w = frame.w;
    for (i, px) in frame.data.as_chunks_mut::<3>().0.iter_mut().enumerate() {
        let (x, y) = (i % w / s, i / w / s);
        let o = ov.rgba[y * ov.w as usize + x];
        let keep = 255 - u32::from(o[3]);
        for c in 0..3 {
            let d = (keep * u32::from(px[c]) + 127) / 255;
            px[c] = (u32::from(o[c]) + d).min(255) as u8;
        }
    }
}

/// `OverlayU` in overlay.wgsl.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct OverlayU {
    scale: [u32; 2],
    _pad: [u32; 2],
}

/// The layer's texture on the GPU.
struct Uploaded {
    tex: wgpu::Texture,
    bind: wgpu::BindGroup,
    w: u32,
    h: u32,
}

/// The overlay pass: the uploaded layer, if any, drawn over the LDR target after the tone map.
pub(crate) struct OverlayPass {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    uploaded: Option<Uploaded>,
    on: bool,
}

impl OverlayPass {
    pub(crate) fn new(gpu: &Gpu) -> OverlayPass {
        let device = &gpu.device;
        let m = device.create_shader_module(wgpu::include_wgsl!("shaders/overlay.wgsl"));
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty,
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("overlay"),
            entries: &[
                entry(
                    0,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    1,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("overlay"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("overlay"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &m,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &m,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: LDR_FORMAT,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::COLOR,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("overlay"),
            size: std::mem::size_of::<OverlayU>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        OverlayPass {
            pipeline,
            layout,
            uniform,
            uploaded: None,
            on: false,
        }
    }

    /// Uploads `ov` (the texture is made once per layer size and kept), or turns the pass off.
    pub(crate) fn set(&mut self, gpu: &Gpu, ov: Option<&Overlay>) {
        let Some(ov) = ov else {
            self.on = false;
            return;
        };
        if self.uploaded.as_ref().is_none_or(|u| (u.w, u.h) != (ov.w, ov.h)) {
            let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("overlay"),
                size: wgpu::Extent3d {
                    width: ov.w,
                    height: ov.h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("overlay"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.uniform.as_entire_binding(),
                    },
                ],
            });
            self.uploaded = Some(Uploaded {
                tex,
                bind,
                w: ov.w,
                h: ov.h,
            });
        }
        let u = self.uploaded.as_ref().expect("just made");
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &u.tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&ov.rgba),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * ov.w),
                rows_per_image: Some(ov.h),
            },
            wgpu::Extent3d {
                width: ov.w,
                height: ov.h,
                depth_or_array_layers: 1,
            },
        );
        self.on = true;
    }

    /// Records the layer over `t.ldr`, when one is set.
    ///
    /// # Panics
    /// The targets are not a whole multiple of the layer.
    pub(crate) fn encode(&self, gpu: &Gpu, enc: &mut wgpu::CommandEncoder, t: &Targets) {
        let (true, Some(u)) = (self.on, self.uploaded.as_ref()) else {
            return;
        };
        let probe = Overlay {
            w: u.w,
            h: u.h,
            rgba: Vec::new(),
        };
        let s = scale_of(t.w, t.h, &probe);
        let uni = OverlayU {
            scale: [s, s],
            _pad: [0; 2],
        };
        gpu.queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uni));
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("overlay"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &t.ldr,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &u.bind, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
#[path = "overlay_tests.rs"]
mod tests;
