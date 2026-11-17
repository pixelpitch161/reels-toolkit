//! Meshes: the vertex format, CPU-side mesh data, and the GPU copies, uploaded once and drawn by id.

use wgpu::util::DeviceExt;

use crate::gpu::Gpu;
use crate::maths::{self, V3};

/// `Vertex::flags` bit: the colour is used as it is, unshaded.
pub const FLAG_EMIT: u32 = 1;
/// `Vertex::flags` bit: the face is drawn from both sides (the mesh holds both windings).
pub const FLAG_TWO_SIDED: u32 = 2;

/// One vertex, as the shaders read it (44 bytes, no padding).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub nrm: [f32; 3],
    /// Linear 0..1; the instance colour multiplies it.
    pub rgb: [f32; 3],
    /// [`FLAG_EMIT`], [`FLAG_TWO_SIDED`].
    pub flags: u32,
    /// The specular strength (0: none).
    pub spec: f32,
}

impl Vertex {
    /// A plain white vertex.
    pub fn new(pos: V3, nrm: V3) -> Vertex {
        Vertex {
            pos,
            nrm,
            rgb: [1.0; 3],
            flags: 0,
            spec: 0.0,
        }
    }

    /// The vertex buffer layout every scene pipeline uses.
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Uint32, 4 => Float32
        ],
    };
}

/// A triangle list on the CPU. Triangles wind counter-clockwise seen from the side they face.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshData {
    pub verts: Vec<Vertex>,
    pub idx: Vec<u32>,
}

impl MeshData {
    /// Adds `other`'s triangles to these.
    pub fn append(&mut self, other: &MeshData) {
        let base = self.verts.len() as u32;
        self.verts.extend_from_slice(&other.verts);
        self.idx.extend(other.idx.iter().map(|i| i + base));
    }

    /// Every vertex coloured `rgb` with `spec`.
    pub fn painted(mut self, rgb: [f32; 3], spec: f32) -> MeshData {
        for v in &mut self.verts {
            v.rgb = rgb;
            v.spec = spec;
        }
        self
    }

    /// The bounding sphere: the box centre and the farthest vertex from it.
    pub fn bounds(&self) -> (V3, f32) {
        if self.verts.is_empty() {
            return ([0.0; 3], 0.0);
        }
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        for v in &self.verts {
            for k in 0..3 {
                lo[k] = lo[k].min(v.pos[k]);
                hi[k] = hi[k].max(v.pos[k]);
            }
        }
        let c = maths::scale(maths::add(lo, hi), 0.5);
        let r = self
            .verts
            .iter()
            .map(|v| maths::length(maths::sub(v.pos, c)))
            .fold(0.0, f32::max);
        (c, r)
    }
}

/// A mesh's handle in [`Meshes`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MeshId(pub(crate) u32);

/// A mesh on the GPU (public so effect passes can draw the same meshes).
pub struct GpuMesh {
    pub verts: wgpu::Buffer,
    pub idx: wgpu::Buffer,
    pub count: u32,
    /// The bounding sphere in model space, for sorting glass.
    pub centre: V3,
    pub radius: f32,
}

/// Every mesh the renderer can draw, uploaded once.
#[derive(Default)]
pub struct Meshes {
    list: Vec<GpuMesh>,
}

impl Meshes {
    /// Uploads `m` and returns its id. An empty mesh is allowed and draws nothing.
    pub fn add(&mut self, gpu: &Gpu, m: MeshData) -> MeshId {
        let (centre, radius) = m.bounds();
        // A zero-size buffer can't be bound; an empty mesh gets one dummy vertex and index.
        let empty = m.idx.is_empty();
        let (verts, idx) = if empty {
            (vec![Vertex::new([0.0; 3], [0.0, 1.0, 0.0])], vec![0u32])
        } else {
            (m.verts, m.idx)
        };
        let id = MeshId(self.list.len() as u32);
        let label = format!("mesh {}", id.0);
        let verts_buf = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&label),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let idx_buf = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&label),
            contents: bytemuck::cast_slice(&idx),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.list.push(GpuMesh {
            verts: verts_buf,
            idx: idx_buf,
            count: if empty { 0 } else { idx.len() as u32 },
            centre,
            radius,
        });
        id
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn get(&self, id: MeshId) -> &GpuMesh {
        &self.list[id.0 as usize]
    }
}
