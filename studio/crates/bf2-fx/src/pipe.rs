//! Pipeline building shared by the effect passes: they all draw into the scene's multisampled HDR colour,
//! depth tested against it and never writing it.

use bf2_render::Gpu;
use bf2_render::target::{DEPTH_FORMAT, HDR_FORMAT, SAMPLES};

/// How an effect adds to the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blend {
    /// Light: adds its HDR colour.
    Add,
    /// Premultiplied over: covers what is behind by its alpha (smoke, decals, debris).
    Over,
}

impl Blend {
    fn state(self) -> wgpu::BlendState {
        match self {
            Blend::Add => wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Zero,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            },
            Blend::Over => wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
        }
    }
}

/// A WGSL module: the shared hash and noise, then `body`.
pub fn module(gpu: &Gpu, label: &str, body: &str) -> wgpu::ShaderModule {
    gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(format!("{}\n{body}", include_str!("shaders/hash.wgsl")).into()),
    })
}

/// A uniform or storage binding visible to both stages.
pub fn entry(binding: u32, ty: wgpu::BufferBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

pub const UNIFORM: wgpu::BufferBindingType = wgpu::BufferBindingType::Uniform;
pub const STORAGE: wgpu::BufferBindingType = wgpu::BufferBindingType::Storage { read_only: true };

/// An effect pipeline: `vs_main`/`fs_main` of `m`, no vertex buffers unless given.
pub fn pipeline(
    gpu: &Gpu,
    label: &str,
    layout: &wgpu::PipelineLayout,
    m: &wgpu::ShaderModule,
    topology: wgpu::PrimitiveTopology,
    blend: Blend,
    buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
) -> wgpu::RenderPipeline {
    pipeline_entries(gpu, label, layout, m, "vs_main", "fs_main", topology, blend, buffers)
}

/// An effect pipeline with named entry points.
#[allow(clippy::too_many_arguments)]
pub fn pipeline_entries(
    gpu: &Gpu,
    label: &str,
    layout: &wgpu::PipelineLayout,
    m: &wgpu::ShaderModule,
    vs: &str,
    fs: &str,
    topology: wgpu::PrimitiveTopology,
    blend: Blend,
    buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
) -> wgpu::RenderPipeline {
    gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: m,
            entry_point: Some(vs),
            compilation_options: Default::default(),
            buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: SAMPLES,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: m,
            entry_point: Some(fs),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: HDR_FORMAT,
                blend: Some(blend.state()),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// A storage buffer that grows (to the next power of two) and is never shrunk, with its bind group remade when
/// it grows.
pub struct Grow {
    pub buf: wgpu::Buffer,
    pub cap: u64,
    usage: wgpu::BufferUsages,
    label: &'static str,
}

impl Grow {
    pub fn new(gpu: &Gpu, label: &'static str, usage: wgpu::BufferUsages, cap: u64) -> Grow {
        let usage = usage | wgpu::BufferUsages::COPY_DST;
        Grow {
            buf: gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: cap,
                usage,
                mapped_at_creation: false,
            }),
            cap,
            usage,
            label,
        }
    }

    /// Writes `bytes`; returns true when the buffer was remade (so bind groups on it must be too).
    pub fn write(&mut self, gpu: &Gpu, bytes: &[u8]) -> bool {
        let mut grew = false;
        if bytes.len() as u64 > self.cap {
            self.cap = (bytes.len() as u64).next_power_of_two();
            self.buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: self.cap,
                usage: self.usage,
                mapped_at_creation: false,
            });
            grew = true;
        }
        if !bytes.is_empty() {
            gpu.queue.write_buffer(&self.buf, 0, bytes);
        }
        grew
    }
}
