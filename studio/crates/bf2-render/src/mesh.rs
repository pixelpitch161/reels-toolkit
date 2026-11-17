//! Meshes: the vertex format, CPU-side mesh data, and the GPU copies, uploaded once and drawn by id.

use wgpu::util::DeviceExt;

use crate::gpu::Gpu;
use crate::maths::{self, V3};

/// `Vertex::flags` bit: the colour is used as it is, unshaded.
pub const FLAG_EMIT: u32 = 1;
/// `Vertex::flags` bit: the face is drawn from both sides (the mesh holds both windings).
pub const FLAG_TWO_SIDED: u32 = 2;

/// One vertex, as the shaders read it (68 bytes, no padding).
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
    /// Texture coordinates: where the albedo and normal maps are read (0..1 across a map; a material's
    /// `uv_scale` tiles them).
    pub uv: [f32; 2],
    /// The tangent (`xyz`, unit, at right angles to `nrm`, along +u) and its handedness `w` (±1): the bitangent,
    /// along +v, is `w * nrm x tan`. A normal map is read in this frame.
    pub tan: [f32; 4],
}

impl Vertex {
    /// A plain white vertex: uv 0, and some tangent at right angles to `nrm`.
    pub fn new(pos: V3, nrm: V3) -> Vertex {
        let t = any_tangent(nrm);
        Vertex {
            pos,
            nrm,
            rgb: [1.0; 3],
            flags: 0,
            spec: 0.0,
            uv: [0.0; 2],
            tan: [t[0], t[1], t[2], 1.0],
        }
    }

    /// The vertex buffer layout every scene pipeline uses.
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Uint32, 4 => Float32, 5 => Float32x2, 6 => Float32x4
        ],
    };
}

/// A unit vector at right angles to `n` (any one; `+X` for a zero `n`).
pub fn any_tangent(n: V3) -> V3 {
    let helper = if n[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let t = maths::sub(helper, maths::scale(n, maths::dot(helper, n)));
    if maths::length(t) < 1e-6 {
        [1.0, 0.0, 0.0]
    } else {
        maths::normalize(t)
    }
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

    /// The tangents worked out from the triangles' uvs (the usual per-triangle sum, then made at right angles to
    /// each vertex's normal). A vertex whose triangles give no tangent (no uv change) keeps one at right angles to
    /// its normal.
    pub fn with_tangents(mut self) -> MeshData {
        let n = self.verts.len();
        let mut tan = vec![[0.0f32; 3]; n];
        let mut bit = vec![[0.0f32; 3]; n];
        for t in self.idx.as_chunks::<3>().0 {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| i as usize);
            let (pa, pb, pc) = (self.verts[a].pos, self.verts[b].pos, self.verts[c].pos);
            let (ua, ub, uc) = (self.verts[a].uv, self.verts[b].uv, self.verts[c].uv);
            let (e1, e2) = (maths::sub(pb, pa), maths::sub(pc, pa));
            let (d1, d2) = ([ub[0] - ua[0], ub[1] - ua[1]], [uc[0] - ua[0], uc[1] - ua[1]]);
            let det = d1[0] * d2[1] - d2[0] * d1[1];
            if det.abs() < 1e-12 {
                continue;
            }
            let r = 1.0 / det;
            let sdir = maths::scale(maths::sub(maths::scale(e1, d2[1]), maths::scale(e2, d1[1])), r);
            let tdir = maths::scale(maths::sub(maths::scale(e2, d1[0]), maths::scale(e1, d2[0])), r);
            for i in [a, b, c] {
                tan[i] = maths::add(tan[i], sdir);
                bit[i] = maths::add(bit[i], tdir);
            }
        }
        for (i, v) in self.verts.iter_mut().enumerate() {
            let nv = v.nrm;
            let t = maths::sub(tan[i], maths::scale(nv, maths::dot(nv, tan[i])));
            let t = if maths::length(t) < 1e-8 {
                any_tangent(nv)
            } else {
                maths::normalize(t)
            };
            let w = if maths::dot(maths::cross(nv, t), bit[i]) < 0.0 {
                -1.0
            } else {
                1.0
            };
            v.tan = [t[0], t[1], t[2], w];
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
