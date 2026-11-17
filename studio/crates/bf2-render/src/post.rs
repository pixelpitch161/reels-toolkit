//! The post stack: exposure, the tone map (ACES fitted, Narkowicz 2015), a neutral grade hook and the sRGB
//! encode, from the resolved HDR colour into the 8-bit picture. Bloom, flashes and the rest are E3.

use crate::gpu::Gpu;
use crate::target::{LDR_FORMAT, Targets};

/// The post settings of one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Post {
    /// Multiplies the HDR colour before the tone map.
    pub exposure: f32,
}

impl Default for Post {
    fn default() -> Self {
        Post { exposure: 1.0 }
    }
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
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tonemap"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&t.hdr_resolved),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.uniform.as_entire_binding(),
                },
            ],
        })
    }

    /// Records the tone map from `t.hdr_resolved` into `t.ldr`. The uniform is written now, so one `PostPass`
    /// serves one frame per submit.
    pub fn encode(&self, gpu: &Gpu, enc: &mut wgpu::CommandEncoder, t: &Targets, bind: &wgpu::BindGroup, post: &Post) {
        let u = PostU {
            exposure: post.exposure,
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
