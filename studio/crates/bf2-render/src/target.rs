//! The frame's render targets and the read-back of the finished picture.
//!
//! One frame flows through: the multisampled HDR colour and depth (the scene passes draw here), the resolved HDR
//! colour (the tone map reads it), the 8-bit picture (the tone map writes it) and a mappable buffer the picture is
//! copied into. All of it is made once per size and reused for every frame.

use bf_picture::img::Rgb8;

use crate::gpu::Gpu;

/// The video's width and height.
pub const W: u32 = 1080;
pub const H: u32 = 1920;

/// MSAA samples for the scene passes: 1, none. The current engine draws hard-edged pixel art (Pillow polygons
/// with no smoothing); smoothed edges read as a 3D model (Aedan, 2026-11-17). With one sample the scene draws
/// straight into the resolved colour and there is nothing to resolve.
pub const SAMPLES: u32 = 1;
/// The scene's colour: linear HDR.
pub const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// The final picture. Not an sRGB format: the tone map shader encodes, so what is stored is what ffmpeg gets.
pub const LDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// The targets of one frame size.
pub struct Targets {
    pub w: u32,
    pub h: u32,
    pub hdr_msaa: wgpu::TextureView,
    pub hdr_resolved: wgpu::TextureView,
    /// Sub-frames are averaged here (motion blur).
    pub hdr_accum: wgpu::TextureView,
    /// The post chain's output, which the tone map then reads.
    pub hdr_post_tex: wgpu::Texture,
    pub hdr_post: wgpu::TextureView,
    pub depth_msaa: wgpu::TextureView,
    pub ldr_tex: wgpu::Texture,
    pub ldr: wgpu::TextureView,
    pub(crate) readback: wgpu::Buffer,
    /// Bytes per row in `readback`: `w * 4` rounded up to wgpu's 256-byte copy alignment.
    pub(crate) padded_row: u32,
}

impl Targets {
    /// What a pass drawing into [`Targets::hdr_msaa`] resolves into: the resolved colour, or nothing when the
    /// scene is not multisampled (it already draws there).
    pub fn resolve(&self) -> Option<&wgpu::TextureView> {
        (SAMPLES > 1).then_some(&self.hdr_resolved)
    }

    pub fn new(gpu: &Gpu, w: u32, h: u32) -> Targets {
        let tex = |label, format, samples, usage| {
            gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let view = |t: &wgpu::Texture| t.create_view(&wgpu::TextureViewDescriptor::default());
        let attach = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let hdr_resolved = tex(
            "hdr resolved",
            HDR_FORMAT,
            1,
            attach | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        // with one sample the scene's colour is the resolved colour itself
        let hdr_msaa = if SAMPLES > 1 {
            view(&tex("hdr msaa", HDR_FORMAT, SAMPLES, attach))
        } else {
            view(&hdr_resolved)
        };
        let sampled = attach | wgpu::TextureUsages::TEXTURE_BINDING;
        let hdr_accum = tex("hdr accum", HDR_FORMAT, 1, sampled);
        let hdr_post_tex = tex("hdr post", HDR_FORMAT, 1, sampled | wgpu::TextureUsages::COPY_SRC);
        let depth_msaa = tex("depth msaa", DEPTH_FORMAT, SAMPLES, attach);
        let ldr_tex = tex("ldr", LDR_FORMAT, 1, attach | wgpu::TextureUsages::COPY_SRC);
        let padded_row = (w * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(padded_row) * u64::from(h),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Targets {
            w,
            h,
            hdr_msaa,
            hdr_resolved: view(&hdr_resolved),
            hdr_accum: view(&hdr_accum),
            hdr_post: view(&hdr_post_tex),
            hdr_post_tex,
            depth_msaa: view(&depth_msaa),
            ldr: view(&ldr_tex),
            ldr_tex,
            readback,
            padded_row,
        }
    }
}

/// Copies the 8-bit picture to the CPU: `w x h x 3`, row padding and alpha removed.
pub fn read_back(gpu: &Gpu, t: &Targets) -> Rgb8 {
    let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("read back"),
    });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &t.ldr_tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &t.readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(t.padded_row),
                rows_per_image: Some(t.h),
            },
        },
        wgpu::Extent3d {
            width: t.w,
            height: t.h,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([enc.finish()]);
    t.readback
        .map_async(wgpu::MapMode::Read, .., |r| r.expect("map the read-back buffer"));
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("wait for the read-back");
    let (w, h) = (t.w as usize, t.h as usize);
    let mut data = Vec::with_capacity(w * h * 3);
    {
        let mapped = t.readback.get_mapped_range(..).expect("the read-back is mapped");
        for row in mapped.chunks_exact(t.padded_row as usize) {
            for px in row[..w * 4].as_chunks::<4>().0 {
                data.extend_from_slice(&px[..3]);
            }
        }
    }
    t.readback.unmap();
    Rgb8::from_vec(w, h, 3, data)
}

/// A half float's value.
fn half(h: u16) -> f32 {
    let sign = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = (h >> 10) & 0x1F;
    let man = f32::from(h & 0x3FF);
    sign * match exp {
        0 => man * (2f32).powi(-24),
        31 if man == 0.0 => f32::INFINITY,
        31 => f32::NAN,
        e => (1.0 + man / 1024.0) * (2f32).powi(i32::from(e) - 15),
    }
}

/// The post chain's HDR output (`hdr_post`) on the CPU, row-major RGBA. For tests and tools; it allocates.
pub fn read_back_hdr(gpu: &Gpu, t: &Targets) -> Vec<[f32; 4]> {
    let row = (t.w * 8).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hdr read-back"),
        size: u64::from(row) * u64::from(t.h),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("hdr read-back"),
    });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &t.hdr_post_tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(t.h),
            },
        },
        wgpu::Extent3d {
            width: t.w,
            height: t.h,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([enc.finish()]);
    buf.map_async(wgpu::MapMode::Read, .., |r| r.expect("map the HDR read-back"));
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("wait for the HDR read-back");
    let mapped = buf.get_mapped_range(..).expect("mapped");
    let mut out = Vec::with_capacity((t.w * t.h) as usize);
    for r in mapped.chunks_exact(row as usize) {
        for px in r[..t.w as usize * 8].as_chunks::<8>().0 {
            out.push(std::array::from_fn(|k| {
                half(u16::from_le_bytes([px[2 * k], px[2 * k + 1]]))
            }));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATTERN: &str = r#"
@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let x = u32(p.x);
    let y = u32(p.y);
    return vec4<f32>(f32(x % 256u), f32(y % 256u), f32(x / 256u), 255.0) / 255.0;
}
"#;

    #[test]
    fn readback_has_no_padding_garbage() {
        let gpu = Gpu::headless().unwrap();
        // 1080 x 4 = 4320 bytes a row: not a multiple of 256, so every row is padded.
        let t = Targets::new(&gpu, W, 24);
        let module = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pattern"),
            source: wgpu::ShaderSource::Wgsl(PATTERN.into()),
        });
        let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pattern"),
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
                targets: &[Some(LDR_FORMAT.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let mut enc = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pattern"),
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
            pass.set_pipeline(&pipeline);
            pass.draw(0..3, 0..1);
        }
        gpu.queue.submit([enc.finish()]);
        // Twice, so the buffer's map and unmap are shown to be reusable.
        for _ in 0..2 {
            let img = read_back(&gpu, &t);
            assert_eq!((img.w, img.h, img.c), (W as usize, 24, 3));
            for y in 0..img.h {
                for x in 0..img.w {
                    let want = [(x % 256) as u8, (y % 256) as u8, (x / 256) as u8];
                    assert_eq!(img.px(x, y), want, "pixel ({x}, {y})");
                }
            }
        }
    }
}
