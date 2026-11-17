//! The renderer: the passes that turn a [`Scene`] into a picture.
//!
//! In order, each frame:
//! 1. the spot's shadow map (2048², depth only, the Toon instances), when the spot casts one;
//! 2. the scene pass into the 4x MSAA HDR target: opaque instances (Toon, Emissive, Unlit), one draw per mesh,
//!    then glass instances back to front, each its far side then its near side, depth tested but not written;
//!    the pass resolves into the single-sample HDR target as it ends;
//! 3. the tone map into the 8-bit picture, then the read-back.
//!
//! Every pipeline, the shadow map, and the frame, light and instance buffers are made once in [`Renderer::new`]
//! (the instance buffer grows when a frame needs more room); the targets are made once per frame size. A frame
//! only writes buffers and records passes.

use bf_picture::img::Rgb8;

use crate::gpu::Gpu;
use crate::maths::{self, M4};
use crate::mesh::{MeshId, Meshes, Vertex};
use crate::post::{Post, PostPass};
use crate::scene::{MAX_LIGHTS, Material, Scene};
use crate::target::{DEPTH_FORMAT, H, HDR_FORMAT, SAMPLES, Targets, W, read_back};

/// The spot's shadow map, in texels a side.
pub const SHADOW_SIZE: u32 = 2048;

const KIND_TOON: u32 = 0;
const KIND_GLASS: u32 = 1;
const KIND_EMISSIVE: u32 = 2;
const KIND_UNLIT: u32 = 3;

/// `Frame` in common.wgsl.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FrameU {
    view_proj: M4,
    spot_vp: M4,
    eye: [f32; 4],
    ambient: [f32; 4],
    spot_pos: [f32; 4],
    spot_dir: [f32; 4],
    spot_col: [f32; 4],
    info: [u32; 4],
}

/// `LightG` in common.wgsl.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct LightG {
    pos_range: [f32; 4],
    colour: [f32; 4],
}

/// `InstanceG` in common.wgsl.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceG {
    model: M4,
    nrm: [[f32; 4]; 3],
    colour: [f32; 4],
    mat_a: [f32; 4],
    mat_b: [f32; 4],
    kind: [u32; 4],
}

impl InstanceG {
    fn new(i: &crate::scene::Instance) -> InstanceG {
        let (kind, bands, mat_a, mat_b) = match i.material {
            Material::Toon { bands, rim, rim_power } => {
                (KIND_TOON, bands, [rim[0], rim[1], rim[2], rim_power], [0.0; 4])
            }
            Material::Glass { tint, opacity, fresnel } => (
                KIND_GLASS,
                0,
                [tint[0], tint[1], tint[2], opacity],
                [fresnel, 0.0, 0.0, 0.0],
            ),
            Material::Emissive { strength } => (KIND_EMISSIVE, 0, [strength, 0.0, 0.0, 0.0], [0.0; 4]),
            Material::Unlit => (KIND_UNLIT, 0, [0.0; 4], [0.0; 4]),
        };
        InstanceG {
            model: i.model,
            nrm: maths::normal_matrix(&i.model),
            colour: i.colour,
            mat_a,
            mat_b,
            kind: [kind, bands, 0, 0],
        }
    }
}

/// Whether an instance of `m` is drawn into the spot's shadow map: lit opaque surfaces only.
fn casts_shadow(m: &Material) -> bool {
    matches!(m, Material::Toon { .. })
}

/// One draw: `count` instances of `mesh` from `first` in the instance buffer.
#[derive(Clone, Copy)]
struct Draw {
    mesh: MeshId,
    first: u32,
    count: u32,
}

/// The targets of the current frame size and the tone map's bind group on them.
struct Sized {
    targets: Targets,
    post_bind: wgpu::BindGroup,
}

pub struct Renderer {
    pub meshes: Meshes,
    opaque: wgpu::RenderPipeline,
    glass_back: wgpu::RenderPipeline,
    glass_front: wgpu::RenderPipeline,
    shadow: wgpu::RenderPipeline,
    frame_buf: wgpu::Buffer,
    lights_buf: wgpu::Buffer,
    frame_bind: wgpu::BindGroup,
    shadow_frame_bind: wgpu::BindGroup,
    shadow_view: wgpu::TextureView,
    inst_layout: wgpu::BindGroupLayout,
    inst_buf: wgpu::Buffer,
    inst_cap: usize,
    inst_bind: wgpu::BindGroup,
    post: PostPass,
    sized: Option<Sized>,
    // Reused every frame.
    inst_data: Vec<InstanceG>,
    opaque_draws: Vec<Draw>,
    /// How many of `opaque_draws`, from the start, cast the spot's shadow.
    caster_draws: usize,
    glass_draws: Vec<Draw>,
}

const COMMON: &str = include_str!("shaders/common.wgsl");

fn module(gpu: &Gpu, label: &str, body: &str) -> wgpu::ShaderModule {
    gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(format!("{COMMON}\n{body}").into()),
    })
}

fn layout_entry(binding: u32, visibility: wgpu::ShaderStages, ty: wgpu::BindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty,
        count: None,
    }
}

fn buffer_ty(ty: wgpu::BufferBindingType) -> wgpu::BindingType {
    wgpu::BindingType::Buffer {
        ty,
        has_dynamic_offset: false,
        min_binding_size: None,
    }
}

fn new_instance_buffer(gpu: &Gpu, layout: &wgpu::BindGroupLayout, cap: usize) -> (wgpu::Buffer, wgpu::BindGroup) {
    let buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("instances"),
        size: (cap * std::mem::size_of::<InstanceG>()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("instances"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buf.as_entire_binding(),
        }],
    });
    (buf, bind)
}

impl Renderer {
    pub fn new(gpu: &Gpu) -> Renderer {
        let device = &gpu.device;
        let vf = wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT;
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame"),
            entries: &[
                layout_entry(0, vf, buffer_ty(wgpu::BufferBindingType::Uniform)),
                layout_entry(
                    1,
                    wgpu::ShaderStages::FRAGMENT,
                    buffer_ty(wgpu::BufferBindingType::Storage { read_only: true }),
                ),
                layout_entry(
                    2,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                layout_entry(
                    3,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                ),
            ],
        });
        // The shadow pass renders into the shadow map, so its frame group holds only the uniform.
        let shadow_frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow frame"),
            entries: &[layout_entry(
                0,
                wgpu::ShaderStages::VERTEX,
                buffer_ty(wgpu::BufferBindingType::Uniform),
            )],
        });
        let inst_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("instances"),
            entries: &[layout_entry(
                0,
                vf,
                buffer_ty(wgpu::BufferBindingType::Storage { read_only: true }),
            )],
        });

        let frame_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame"),
            size: std::mem::size_of::<FrameU>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let lights_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lights"),
            size: (MAX_LIGHTS * std::mem::size_of::<LightG>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow map"),
            size: wgpu::Extent3d {
                width: SHADOW_SIZE,
                height: SHADOW_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_cmp = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow compare"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let frame_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame"),
            layout: &frame_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: frame_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: lights_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&shadow_cmp),
                },
            ],
        });
        let shadow_frame_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow frame"),
            layout: &shadow_frame_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame_buf.as_entire_binding(),
            }],
        });

        let scene_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene"),
            bind_group_layouts: &[Some(&frame_layout), Some(&inst_layout)],
            immediate_size: 0,
        });
        let shadow_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow"),
            bind_group_layouts: &[Some(&shadow_frame_layout), Some(&inst_layout)],
            immediate_size: 0,
        });
        let lit = module(gpu, "lit", include_str!("shaders/lit.wgsl"));
        let glass = module(gpu, "glass", include_str!("shaders/glass.wgsl"));
        let shadow_mod = module(gpu, "shadow", include_str!("shaders/shadow.wgsl"));

        let msaa = wgpu::MultisampleState {
            count: SAMPLES,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };
        let primitive = |cull| wgpu::PrimitiveState {
            cull_mode: Some(cull),
            ..Default::default()
        };
        let depth = |write: bool| wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(write),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        };
        let scene_pipeline = |label, m: &wgpu::ShaderModule, fs, cull, blend, write| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&scene_layout),
                vertex: wgpu::VertexState {
                    module: m,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(Vertex::LAYOUT)],
                },
                primitive: primitive(cull),
                depth_stencil: Some(depth(write)),
                multisample: msaa,
                fragment: Some(wgpu::FragmentState {
                    module: m,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: HDR_FORMAT,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let premultiplied = Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING);
        let opaque = scene_pipeline("opaque", &lit, "fs_main", wgpu::Face::Back, None, true);
        let glass_back = scene_pipeline(
            "glass back",
            &glass,
            "fs_glass",
            wgpu::Face::Front,
            premultiplied,
            false,
        );
        let glass_front = scene_pipeline(
            "glass front",
            &glass,
            "fs_glass",
            wgpu::Face::Back,
            premultiplied,
            false,
        );
        let shadow = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow"),
            layout: Some(&shadow_layout),
            vertex: wgpu::VertexState {
                module: &shadow_mod,
                entry_point: Some("vs_shadow"),
                compilation_options: Default::default(),
                buffers: &[Some(Vertex::LAYOUT)],
            },
            primitive: primitive(wgpu::Face::Back),
            depth_stencil: Some(wgpu::DepthStencilState {
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
                ..depth(true)
            }),
            multisample: Default::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });

        let inst_cap = 256;
        let (inst_buf, inst_bind) = new_instance_buffer(gpu, &inst_layout, inst_cap);
        Renderer {
            meshes: Meshes::default(),
            opaque,
            glass_back,
            glass_front,
            shadow,
            frame_buf,
            lights_buf,
            frame_bind,
            shadow_frame_bind,
            shadow_view,
            inst_layout,
            inst_buf,
            inst_cap,
            inst_bind,
            post: PostPass::new(gpu),
            sized: None,
            inst_data: Vec::new(),
            opaque_draws: Vec::new(),
            caster_draws: 0,
            glass_draws: Vec::new(),
        }
    }

    /// Draws `scene` at the video's size, 1080 x 1920.
    pub fn render(&mut self, gpu: &Gpu, scene: &Scene, post: &Post) -> Rgb8 {
        self.render_sized(gpu, scene, post, W, H)
    }

    /// Draws `scene` at `w x h`. The targets are made on the first frame of a size and kept.
    pub fn render_sized(&mut self, gpu: &Gpu, scene: &Scene, post: &Post, w: u32, h: u32) -> Rgb8 {
        self.ensure_size(gpu, w, h);
        let aspect = w as f32 / h as f32;
        let shadowed = self.write_frame(gpu, scene, aspect);
        self.write_instances(gpu, scene);

        let mut enc = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        if shadowed {
            self.encode_shadow(&mut enc);
        }
        let sized = self.sized.as_ref().expect("targets were just made");
        self.encode_scene(&mut enc, &sized.targets);
        self.post.encode(gpu, &mut enc, &sized.targets, &sized.post_bind, post);
        gpu.queue.submit([enc.finish()]);
        read_back(gpu, &sized.targets)
    }

    fn ensure_size(&mut self, gpu: &Gpu, w: u32, h: u32) {
        if self
            .sized
            .as_ref()
            .is_some_and(|s| s.targets.w == w && s.targets.h == h)
        {
            return;
        }
        let targets = Targets::new(gpu, w, h);
        let post_bind = self.post.bind(gpu, &targets);
        self.sized = Some(Sized { targets, post_bind });
    }

    /// Writes the frame uniform and the lights; returns whether the spot casts a shadow this frame.
    fn write_frame(&self, gpu: &Gpu, scene: &Scene, aspect: f32) -> bool {
        let lights = scene.lights_for_gpu();
        let gpu_lights: Vec<LightG> = lights
            .iter()
            .map(|l| LightG {
                pos_range: [l.pos[0], l.pos[1], l.pos[2], l.range.max(1e-3)],
                colour: [l.colour[0], l.colour[1], l.colour[2], 0.0],
            })
            .collect();
        if !gpu_lights.is_empty() {
            gpu.queue
                .write_buffer(&self.lights_buf, 0, bytemuck::cast_slice(&gpu_lights));
        }
        let cam = &scene.camera;
        let mut u = FrameU {
            view_proj: cam.view_proj(aspect),
            spot_vp: maths::IDENTITY,
            eye: [cam.eye[0], cam.eye[1], cam.eye[2], 1.0],
            ambient: [scene.ambient[0], scene.ambient[1], scene.ambient[2], 0.0],
            spot_pos: [0.0; 4],
            spot_dir: [0.0, -1.0, 0.0, 1.0],
            spot_col: [0.0, 0.0, 0.0, 1.0],
            info: [gpu_lights.len() as u32, 0, SHADOW_SIZE, 0],
        };
        let mut shadowed = false;
        if let Some(s) = &scene.spot {
            let dir = maths::normalize(maths::sub(s.target, s.pos));
            let half = s.angle_deg.clamp(1.0, 80.0).to_radians();
            u.spot_pos = [s.pos[0], s.pos[1], s.pos[2], 1.0];
            u.spot_dir = [dir[0], dir[1], dir[2], half.cos()];
            u.spot_col = [s.colour[0], s.colour[1], s.colour[2], (half * 0.8).cos()];
            if s.shadow {
                let up = if dir[1].abs() > 0.99 {
                    [0.0, 0.0, -1.0]
                } else {
                    [0.0, 1.0, 0.0]
                };
                let reach = maths::length(maths::sub(s.target, s.pos));
                let proj = maths::perspective(2.0 * half * 1.05, 1.0, 0.5, (reach * 3.0).max(50.0));
                u.spot_vp = maths::mul(&proj, &maths::look_at(s.pos, s.target, up, 0.0));
                u.info[1] = 1;
                shadowed = true;
            }
        }
        gpu.queue.write_buffer(&self.frame_buf, 0, bytemuck::bytes_of(&u));
        shadowed
    }

    /// Orders the instances (opaque grouped by mesh, then glass far to near), writes them and plans the draws.
    fn write_instances(&mut self, gpu: &Gpu, scene: &Scene) {
        self.inst_data.clear();
        self.opaque_draws.clear();
        self.caster_draws = 0;
        self.glass_draws.clear();

        let mut opaque: Vec<usize> = Vec::with_capacity(scene.instances.len());
        let mut glass: Vec<(f32, usize)> = Vec::new();
        let view = scene.camera.view();
        for (k, inst) in scene.instances.iter().enumerate() {
            if self.meshes.get(inst.mesh).count == 0 {
                continue;
            }
            if inst.material.is_transparent() {
                let centre = maths::transform_point(&inst.model, self.meshes.get(inst.mesh).centre);
                // View space looks down −Z: the farther, the more negative.
                let z = maths::transform_point(&view, [centre[0], centre[1], centre[2]])[2];
                glass.push((z, k));
            } else {
                opaque.push(k);
            }
        }
        // Shadow casters (Toon) first, so the shadow pass draws a prefix of the opaque draws; glowing and unlit
        // things cast no shadow. Within each, grouped by mesh.
        opaque.sort_by_key(|&k| (!casts_shadow(&scene.instances[k].material), scene.instances[k].mesh));
        glass.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));

        for (n, &k) in opaque.iter().enumerate() {
            let inst = &scene.instances[k];
            let first = self.inst_data.len() as u32;
            self.inst_data.push(InstanceG::new(inst));
            let caster = casts_shadow(&inst.material);
            let same_run = n > 0 && casts_shadow(&scene.instances[opaque[n - 1]].material) == caster;
            match self.opaque_draws.last_mut() {
                Some(d) if same_run && d.mesh == inst.mesh => d.count += 1,
                _ => {
                    self.opaque_draws.push(Draw {
                        mesh: inst.mesh,
                        first,
                        count: 1,
                    });
                    if caster {
                        self.caster_draws = self.opaque_draws.len();
                    }
                }
            }
        }
        for &(_, k) in &glass {
            let inst = &scene.instances[k];
            self.glass_draws.push(Draw {
                mesh: inst.mesh,
                first: self.inst_data.len() as u32,
                count: 1,
            });
            self.inst_data.push(InstanceG::new(inst));
        }

        if self.inst_data.len() > self.inst_cap {
            self.inst_cap = self.inst_data.len().next_power_of_two();
            (self.inst_buf, self.inst_bind) = new_instance_buffer(gpu, &self.inst_layout, self.inst_cap);
        }
        if !self.inst_data.is_empty() {
            gpu.queue
                .write_buffer(&self.inst_buf, 0, bytemuck::cast_slice(&self.inst_data));
        }
    }

    fn draw<'p>(&self, pass: &mut wgpu::RenderPass<'p>, d: &Draw) {
        let m = self.meshes.get(d.mesh);
        pass.set_vertex_buffer(0, m.verts.slice(..));
        pass.set_index_buffer(m.idx.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..m.count, 0, d.first..d.first + d.count);
    }

    fn encode_shadow(&self, enc: &mut wgpu::CommandEncoder) {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.shadow_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.shadow);
        pass.set_bind_group(0, &self.shadow_frame_bind, &[]);
        pass.set_bind_group(1, &self.inst_bind, &[]);
        for d in &self.opaque_draws[..self.caster_draws] {
            self.draw(&mut pass, d);
        }
    }

    fn encode_scene(&self, enc: &mut wgpu::CommandEncoder, t: &Targets) {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &t.hdr_msaa,
                depth_slice: None,
                resolve_target: Some(&t.hdr_resolved),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &t.depth_msaa,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.frame_bind, &[]);
        pass.set_bind_group(1, &self.inst_bind, &[]);
        pass.set_pipeline(&self.opaque);
        for d in &self.opaque_draws {
            self.draw(&mut pass, d);
        }
        for d in &self.glass_draws {
            pass.set_pipeline(&self.glass_back);
            self.draw(&mut pass, d);
            pass.set_pipeline(&self.glass_front);
            self.draw(&mut pass, d);
        }
    }
}

#[cfg(test)]
#[path = "renderer_tests.rs"]
mod tests;
