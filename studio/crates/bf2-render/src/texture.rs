//! Material maps: albedo and tangent-space normal maps, in two texture arrays the scene shaders read by layer
//! (E7). A map is [`TEX_SIZE`] square, RGBA8 with a mip chain made here; an albedo map holds sRGB colour, a normal
//! map the tangent-space normal as `(n * 0.5 + 0.5) * 255` (x along the tangent, +u; y along the bitangent, +v;
//! z out of the surface). Maps are optional per instance ([`crate::scene::Surface`]); an instance with none
//! draws exactly as before.
//!
//! The procedural maps (the canvas weave, a hammered finish) are made on the CPU from tileable height fields, so
//! a map repeated by a material's `uv_scale` has no seams.

use crate::gpu::Gpu;

/// A map's size in texels a side.
pub const TEX_SIZE: u32 = 1024;
/// How many maps of each kind the renderer holds.
pub const TEX_LAYERS: u32 = 8;
/// Mip levels of a [`TEX_SIZE`] map (1024 down to 1).
pub const MIPS: u32 = 11;

/// A map's handle: its layer in its array.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TexId(pub(crate) u32);

impl TexId {
    /// The layer as the shaders read it: the layer plus one (0 is "no map").
    pub fn slot(self) -> u32 {
        self.0 + 1
    }
}

/// The mip chain of a square RGBA8 map of `size` texels a side (a power of two): `rgba` itself, then each level
/// the 2 x 2 average of the one above, down to 1 x 1. A normal map's averages are made unit length again
/// (re-encoded), so its far mips stay normals.
pub fn mip_chain(rgba: &[u8], size: u32, normal: bool) -> Vec<Vec<u8>> {
    assert!(size.is_power_of_two(), "a map's size is a power of two");
    assert_eq!(rgba.len(), (size * size * 4) as usize, "a {size} x {size} RGBA map");
    let mut out = vec![rgba.to_vec()];
    let mut s = size;
    while s > 1 {
        let prev = out.last().expect("the level above");
        let h = s / 2;
        let mut next = vec![0u8; (h * h * 4) as usize];
        for y in 0..h {
            for x in 0..h {
                let mut sum = [0.0f32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let i = (((2 * y + dy) * s + 2 * x + dx) * 4) as usize;
                    for c in 0..4 {
                        sum[c] += f32::from(prev[i + c]);
                    }
                }
                let mut px = sum.map(|v| v / 4.0);
                if normal {
                    let n = [0, 1, 2].map(|c| px[c] / 255.0 * 2.0 - 1.0);
                    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
                    for c in 0..3 {
                        px[c] = (n[c] / l * 0.5 + 0.5) * 255.0;
                    }
                }
                let o = ((y * h + x) * 4) as usize;
                for c in 0..4 {
                    next[o + c] = px[c].round().clamp(0.0, 255.0) as u8;
                }
            }
        }
        out.push(next);
        s = h;
    }
    out
}

/// The normal map of a tileable height field `h` (`size` x `size`, row by row; v grows down the rows): central
/// differences that wrap at the edges, scaled by `strength` (height units per texel), encoded as RGBA8.
pub fn normals_from_height(h: &[f32], size: u32, strength: f32) -> Vec<u8> {
    let s = size as usize;
    assert_eq!(h.len(), s * s, "a {size} x {size} height field");
    let at = |x: usize, y: usize| h[(y % s) * s + (x % s)];
    let mut out = Vec::with_capacity(s * s * 4);
    for y in 0..s {
        for x in 0..s {
            let dx = (at(x + 1, y) - at(x + s - 1, y)) * 0.5 * strength;
            let dy = (at(x, y + 1) - at(x, y + s - 1)) * 0.5 * strength;
            let n = [-dx, -dy, 1.0];
            let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            for c in n {
                out.push(((c / l * 0.5 + 0.5) * 255.0).round() as u8);
            }
            out.push(255);
        }
    }
    out
}

/// A plain-weave height field, `threads` threads each way across the map: over-and-under ridges, each thread
/// rounded across its width. Tileable.
pub fn weave_height(size: u32, threads: u32) -> Vec<f32> {
    let s = size as usize;
    let k = threads as f32 / size as f32;
    let mut out = Vec::with_capacity(s * s);
    for y in 0..s {
        for x in 0..s {
            let (u, v) = ((x as f32 + 0.5) * k, (y as f32 + 0.5) * k);
            let (iu, iv) = (u.floor() as i64, v.floor() as i64);
            let (fu, fv) = (u - u.floor(), v - v.floor());
            // the weft (along u) is on top where the cell is even, the warp where odd
            let weft_top = (iu + iv) % 2 == 0;
            let across = |f: f32| (std::f32::consts::PI * f).sin();
            let weft = across(fv) * (if weft_top { 1.0 } else { 0.55 });
            let warp = across(fu) * (if weft_top { 0.55 } else { 1.0 });
            out.push(weft.max(warp));
        }
    }
    out
}

/// A hammered (dimpled) height field: `cells` x `cells` jittered round dents, from `seed`. Tileable.
pub fn hammered_height(size: u32, cells: u32, seed: u64) -> Vec<f32> {
    let s = size as usize;
    let c = cells.max(1) as i64;
    let jitter = |i: i64, j: i64, axis: u64| {
        let (i, j) = (i.rem_euclid(c) as u64, j.rem_euclid(c) as u64);
        let mut z = seed ^ (i.wrapping_mul(0x9E37_79B9_7F4A_7C15)) ^ (j.wrapping_mul(0xBF58_476D_1CE4_E5B9)) ^ axis;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 40) as f32 / (1u64 << 24) as f32
    };
    let mut out = Vec::with_capacity(s * s);
    for y in 0..s {
        for x in 0..s {
            let (u, v) = (
                (x as f32 + 0.5) / size as f32 * c as f32,
                (y as f32 + 0.5) / size as f32 * c as f32,
            );
            let (iu, iv) = (u.floor() as i64, v.floor() as i64);
            let mut best = f32::INFINITY;
            for dj in -1..=1 {
                for di in -1..=1 {
                    let (ci, cj) = (iu + di, iv + dj);
                    let px = ci as f32 + 0.15 + 0.7 * jitter(ci, cj, 1);
                    let py = cj as f32 + 0.15 + 0.7 * jitter(ci, cj, 2);
                    best = best.min((u - px).hypot(v - py));
                }
            }
            // a dent: deepest at the cell's centre, a soft rim between dents
            out.push(-(1.0 - (best / 0.75).min(1.0)).powi(2));
        }
    }
    out
}

/// The flat normal map: every texel `(0, 0, 1)`.
pub fn flat_normals(size: u32) -> Vec<u8> {
    [128u8, 128, 255, 255].repeat((size * size) as usize)
}

/// The two map arrays and their sampler, made once with the renderer.
pub struct Textures {
    albedo: wgpu::Texture,
    normal: wgpu::Texture,
    pub(crate) albedo_view: wgpu::TextureView,
    pub(crate) normal_view: wgpu::TextureView,
    pub(crate) sampler: wgpu::Sampler,
    n_albedo: u32,
    n_normal: u32,
}

fn array(gpu: &Gpu, label: &str, format: wgpu::TextureFormat) -> (wgpu::Texture, wgpu::TextureView) {
    let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: TEX_SIZE,
            height: TEX_SIZE,
            depth_or_array_layers: TEX_LAYERS,
        },
        mip_level_count: MIPS,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    (tex, view)
}

impl Textures {
    pub(crate) fn new(gpu: &Gpu) -> Textures {
        let (albedo, albedo_view) = array(gpu, "albedo maps", wgpu::TextureFormat::Rgba8UnormSrgb);
        let (normal, normal_view) = array(gpu, "normal maps", wgpu::TextureFormat::Rgba8Unorm);
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("maps"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        Textures {
            albedo,
            normal,
            albedo_view,
            normal_view,
            sampler,
            n_albedo: 0,
            n_normal: 0,
        }
    }

    fn upload(gpu: &Gpu, tex: &wgpu::Texture, layer: u32, rgba: &[u8], normal: bool) {
        let mut size = TEX_SIZE;
        for (level, data) in mip_chain(rgba, TEX_SIZE, normal).iter().enumerate() {
            gpu.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: tex,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d { x: 0, y: 0, z: layer },
                    aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * size),
                    rows_per_image: Some(size),
                },
                wgpu::Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 1,
                },
            );
            size = (size / 2).max(1);
        }
    }

    /// Uploads an albedo map (sRGB RGBA8, [`TEX_SIZE`] square). `None` when every layer is taken.
    pub fn add_albedo(&mut self, gpu: &Gpu, rgba: &[u8]) -> Option<TexId> {
        if self.n_albedo >= TEX_LAYERS {
            return None;
        }
        Self::upload(gpu, &self.albedo, self.n_albedo, rgba, false);
        self.n_albedo += 1;
        Some(TexId(self.n_albedo - 1))
    }

    /// Uploads a tangent-space normal map (RGBA8, [`TEX_SIZE`] square). `None` when every layer is taken.
    pub fn add_normal(&mut self, gpu: &Gpu, rgba: &[u8]) -> Option<TexId> {
        if self.n_normal >= TEX_LAYERS {
            return None;
        }
        Self::upload(gpu, &self.normal, self.n_normal, rgba, true);
        self.n_normal += 1;
        Some(TexId(self.n_normal - 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(px: &[u8]) -> [f32; 3] {
        [0, 1, 2].map(|c| f32::from(px[c]) / 255.0 * 2.0 - 1.0)
    }

    #[test]
    fn mip_chain_halves_down_to_one_texel() {
        let size = 8;
        let rgba: Vec<u8> = (0..size * size)
            .flat_map(|i| [(i % 2 * 200) as u8, 10, 20, 255])
            .collect();
        let mips = mip_chain(&rgba, size, false);
        assert_eq!(mips.len(), 4);
        assert_eq!(mips.iter().map(Vec::len).collect::<Vec<_>>(), vec![256, 64, 16, 4]);
        // alternate columns of 0 and 200 average to 100
        assert_eq!(&mips[1][..4], &[100, 10, 20, 255]);
        assert_eq!(&mips[3][..], &[100, 10, 20, 255]);
        assert_eq!(MIPS as usize, mip_chain(&flat_normals(TEX_SIZE), TEX_SIZE, true).len());
    }

    #[test]
    fn normal_mips_stay_unit() {
        let h = hammered_height(64, 4, 9);
        let mips = mip_chain(&normals_from_height(&h, 64, 6.0), 64, true);
        for level in &mips {
            for px in level.chunks(4) {
                let n = decode(px);
                let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                assert!((l - 1.0).abs() < 0.02, "{n:?}");
                assert!(n[2] > 0.0);
            }
        }
    }

    #[test]
    fn a_flat_height_is_the_flat_normal() {
        assert_eq!(normals_from_height(&[0.3; 16], 4, 5.0), flat_normals(4));
    }

    #[test]
    fn a_slope_tilts_the_normal_against_it() {
        // height rising along u (x): the normal leans toward -u
        let h: Vec<f32> = (0..16).map(|i| (i % 4) as f32).collect();
        let n = normals_from_height(&h, 4, 1.0);
        let at = |x: usize, y: usize| decode(&n[(y * 4 + x) * 4..]);
        assert!(at(2, 1)[0] < -0.2, "{:?}", at(2, 1));
        assert!(at(2, 1)[1].abs() < 0.01);
    }

    #[test]
    fn procedural_heights_tile() {
        for h in [weave_height(64, 8), hammered_height(64, 4, 3)] {
            // the step across the wrap is no bigger than the steepest step inside the field
            let at = |x: usize, y: usize| h[y * 64 + x];
            let inner = (0..64)
                .flat_map(|y| (0..63).map(move |x| (y, x)))
                .map(|(y, x)| (at(x + 1, y) - at(x, y)).abs())
                .fold(0.0f32, f32::max);
            for y in 0..64 {
                assert!((at(0, y) - at(63, y)).abs() <= inner + 1e-6);
                assert!((at(y, 0) - at(y, 63)).abs() <= inner + 1e-6);
            }
        }
    }

    #[test]
    fn slots_count_from_one() {
        assert_eq!(TexId(0).slot(), 1);
        assert_eq!(TexId(7).slot(), 8);
    }
}
