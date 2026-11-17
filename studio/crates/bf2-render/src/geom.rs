//! Mesh builders: the primitives the stage and the looks are made of, and the existing instrument models.
//!
//! Everything is in render space (Y up), centred on the origin unless said otherwise, and winds counter-clockwise
//! seen from outside, so back faces can be culled.

use crate::maths::{self, V3};
use crate::mesh::{FLAG_EMIT, FLAG_TWO_SIDED, MeshData, Vertex};

/// A sphere of radius `r`: an icosahedron subdivided `subdiv` times, with smooth normals.
pub fn icosphere(r: f32, subdiv: u32) -> MeshData {
    let t = (1.0 + 5f64.sqrt()) / 2.0;
    let mut pts: Vec<[f64; 3]> = vec![
        [-1.0, t, 0.0],
        [1.0, t, 0.0],
        [-1.0, -t, 0.0],
        [1.0, -t, 0.0],
        [0.0, -1.0, t],
        [0.0, 1.0, t],
        [0.0, -1.0, -t],
        [0.0, 1.0, -t],
        [t, 0.0, -1.0],
        [t, 0.0, 1.0],
        [-t, 0.0, -1.0],
        [-t, 0.0, 1.0],
    ];
    let unit = |p: [f64; 3]| {
        let l = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        [p[0] / l, p[1] / l, p[2] / l]
    };
    for p in &mut pts {
        *p = unit(*p);
    }
    let mut tris: Vec<[u32; 3]> = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    for _ in 0..subdiv {
        let mut mid = std::collections::HashMap::new();
        let mut half = |a: u32, b: u32, pts: &mut Vec<[f64; 3]>| -> u32 {
            *mid.entry((a.min(b), a.max(b))).or_insert_with(|| {
                let (p, q) = (pts[a as usize], pts[b as usize]);
                pts.push(unit([p[0] + q[0], p[1] + q[1], p[2] + q[2]]));
                (pts.len() - 1) as u32
            })
        };
        let mut next = Vec::with_capacity(tris.len() * 4);
        for [a, b, c] in tris {
            let ab = half(a, b, &mut pts);
            let bc = half(b, c, &mut pts);
            let ca = half(c, a, &mut pts);
            next.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        tris = next;
    }
    let r = f64::from(r);
    MeshData {
        verts: pts
            .iter()
            .map(|p| {
                let n = [p[0] as f32, p[1] as f32, p[2] as f32];
                let mut v = Vertex::new([(p[0] * r) as f32, (p[1] * r) as f32, (p[2] * r) as f32], n);
                // uv by longitude and latitude (a seam at -X: an icosphere is for surfaces with no map); the
                // tangent runs east.
                v.uv = [
                    0.5 + n[2].atan2(n[0]) / std::f32::consts::TAU,
                    0.5 - n[1].clamp(-1.0, 1.0).asin() / std::f32::consts::PI,
                ];
                let east = [-n[2], 0.0, n[0]];
                let t = if maths::length(east) < 1e-6 {
                    [1.0, 0.0, 0.0]
                } else {
                    maths::normalize(east)
                };
                v.tan = [t[0], t[1], t[2], 1.0];
                v
            })
            .collect(),
        idx: tris.into_iter().flatten().collect(),
    }
}

/// One flat quad centred at `c`, spanning `±u` and `±v`, facing `u x v`.
fn quad(m: &mut MeshData, c: V3, u: V3, v: V3) {
    let n = maths::normalize(maths::cross(u, v));
    let t = maths::normalize(u);
    let base = m.verts.len() as u32;
    for (a, b) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let p = maths::add(c, maths::add(maths::scale(u, a), maths::scale(v, b)));
        let mut vx = Vertex::new(p, n);
        // u along `u`, v along `v`: the bitangent `n x t` is along `v`, so the handedness is +1.
        vx.uv = [(a + 1.0) / 2.0, (b + 1.0) / 2.0];
        vx.tan = [t[0], t[1], t[2], 1.0];
        m.verts.push(vx);
    }
    m.idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

/// A box of full extents `size`, flat-shaded.
pub fn box_(size: V3) -> MeshData {
    let [x, y, z] = maths::scale(size, 0.5);
    let mut m = MeshData::default();
    quad(&mut m, [x, 0.0, 0.0], [0.0, 0.0, -z], [0.0, y, 0.0]);
    quad(&mut m, [-x, 0.0, 0.0], [0.0, 0.0, z], [0.0, y, 0.0]);
    quad(&mut m, [0.0, y, 0.0], [x, 0.0, 0.0], [0.0, 0.0, -z]);
    quad(&mut m, [0.0, -y, 0.0], [x, 0.0, 0.0], [0.0, 0.0, z]);
    quad(&mut m, [0.0, 0.0, z], [x, 0.0, 0.0], [0.0, y, 0.0]);
    quad(&mut m, [0.0, 0.0, -z], [-x, 0.0, 0.0], [0.0, y, 0.0]);
    m
}

/// A plane `w` wide (X) and `d` deep (Z) at height 0, facing up.
pub fn plane(w: f32, d: f32) -> MeshData {
    let mut m = MeshData::default();
    quad(&mut m, [0.0; 3], [w / 2.0, 0.0, 0.0], [0.0, 0.0, -d / 2.0]);
    m
}

/// One ring of a surface of revolution about Y: radius, height and the normal's (radial, Y) parts.
struct Ring {
    rho: f32,
    y: f32,
    n: [f32; 2],
}

/// A surface of revolution about Y from `rings` (top to bottom) and `n` segments. Consecutive rings that
/// coincide (a crease) and zero-area triangles at the poles are skipped.
fn lathe(rings: &[Ring], n: u32) -> MeshData {
    let n = n.max(3);
    let mut m = MeshData::default();
    // v by the length along the profile, top (0) to bottom (1).
    let mut along = vec![0.0f32; rings.len()];
    for i in 1..rings.len() {
        let (a, b) = (&rings[i - 1], &rings[i]);
        along[i] = along[i - 1] + (b.rho - a.rho).hypot(b.y - a.y);
    }
    let total = along.last().copied().unwrap_or(0.0).max(1e-6);
    for (ring, &d) in rings.iter().zip(&along) {
        for j in 0..=n {
            let th = j as f32 / n as f32 * std::f32::consts::TAU;
            let (s, c) = th.sin_cos();
            let mut v = Vertex::new(
                [ring.rho * c, ring.y, ring.rho * s],
                maths::normalize([ring.n[0] * c, ring.n[1], ring.n[0] * s]),
            );
            v.uv = [j as f32 / n as f32, d / total];
            // u grows with the angle: along (-sin, 0, cos), which is at right angles to every normal of the ring.
            v.tan = [-s, 0.0, c, 1.0];
            m.verts.push(v);
        }
    }
    let row = n + 1;
    for (i, pair) in rings.windows(2).enumerate() {
        let (a, b) = (&pair[0], &pair[1]);
        if a.rho == b.rho && a.y == b.y {
            continue;
        }
        let i = i as u32;
        for j in 0..n {
            let (p, q) = (i * row + j, (i + 1) * row + j);
            if a.rho > 0.0 {
                m.idx.extend([p, p + 1, q]);
            }
            if b.rho > 0.0 {
                m.idx.extend([p + 1, q + 1, q]);
            }
        }
    }
    // the handedness: v runs down the profile
    for v in &mut m.verts {
        let b = maths::cross(v.nrm, [v.tan[0], v.tan[1], v.tan[2]]);
        v.tan[3] = if b[1] > 0.0 { -1.0 } else { 1.0 };
    }
    m
}

/// A sphere of radius `r` from `lon` segments round and `lat` from pole to pole, with uvs by longitude and
/// latitude (seamless: the seam's vertices are doubled) and tangents running east: for surfaces with a map.
pub fn uv_sphere(r: f32, lon: u32, lat: u32) -> MeshData {
    let lat = lat.max(2);
    let rings: Vec<Ring> = (0..=lat)
        .map(|k| {
            let phi = k as f32 / lat as f32 * std::f32::consts::PI;
            let (s, c) = phi.sin_cos();
            Ring {
                rho: if k == 0 || k == lat { 0.0 } else { r * s },
                y: r * c,
                n: [s, c],
            }
        })
        .collect();
    lathe(&rings, lon)
}

/// A closed cylinder of radius `r` and height `h` along Y, centred, with `n` sides (smooth sides, flat caps).
pub fn cylinder(r: f32, h: f32, n: u32) -> MeshData {
    let (t, b) = (h / 2.0, -h / 2.0);
    lathe(
        &[
            Ring {
                rho: 0.0,
                y: t,
                n: [0.0, 1.0],
            },
            Ring {
                rho: r,
                y: t,
                n: [0.0, 1.0],
            },
            Ring {
                rho: r,
                y: t,
                n: [1.0, 0.0],
            },
            Ring {
                rho: r,
                y: b,
                n: [1.0, 0.0],
            },
            Ring {
                rho: r,
                y: b,
                n: [0.0, -1.0],
            },
            Ring {
                rho: 0.0,
                y: b,
                n: [0.0, -1.0],
            },
        ],
        n,
    )
}

/// A capsule of radius `r` around the segment `a`..`b` (render space), `n` segments round.
pub fn capsule(a: V3, b: V3, r: f32, n: u32) -> MeshData {
    let axis = maths::sub(b, a);
    let len = maths::length(axis);
    let half = len / 2.0;
    let m_lat = (n / 4).max(2);
    let mut rings = Vec::with_capacity(2 * m_lat as usize + 2);
    for k in 0..=m_lat {
        let phi = k as f32 / m_lat as f32 * std::f32::consts::FRAC_PI_2;
        let (s, c) = phi.sin_cos();
        rings.push(Ring {
            rho: r * s,
            y: half + r * c,
            n: [s, c],
        });
    }
    for k in 0..=m_lat {
        let phi = std::f32::consts::FRAC_PI_2 + k as f32 / m_lat as f32 * std::f32::consts::FRAC_PI_2;
        let (s, c) = phi.sin_cos();
        rings.push(Ring {
            rho: r * s.max(0.0),
            y: -half + r * c,
            n: [s, c],
        });
    }
    // The last ring is the bottom pole.
    if let Some(last) = rings.last_mut() {
        last.rho = 0.0;
    }
    let mut m = lathe(&rings, n);
    // Local Y onto the axis: an orthonormal basis (x', y' = axis, z') with x' x y' = z'.
    let y = if len > 0.0 {
        maths::scale(axis, 1.0 / len)
    } else {
        [0.0, 1.0, 0.0]
    };
    let helper = if y[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let x = maths::normalize(maths::cross(y, helper));
    let z = maths::cross(x, y);
    let centre = maths::scale(maths::add(a, b), 0.5);
    let to = |p: V3| {
        maths::add(
            maths::add(maths::scale(x, p[0]), maths::scale(y, p[1])),
            maths::scale(z, p[2]),
        )
    };
    for v in &mut m.verts {
        v.pos = maths::add(centre, to(v.pos));
        v.nrm = to(v.nrm);
        let t = to([v.tan[0], v.tan[1], v.tan[2]]);
        v.tan = [t[0], t[1], t[2], v.tan[3]];
    }
    m
}

/// An instrument model: each face a fan from its first vertex, flat-shaded with the face's normal, its colour
/// (truncated to whole numbers, then over 255) and flags. Two-sided faces get both windings, so the renderer can
/// cull back faces everywhere.
pub fn from_sprite_mesh(m: &bf_picture::sprite::Mesh, scale: f32) -> MeshData {
    let mut out = MeshData::default();
    for face in &m.faces {
        if face.p.len() < 3 {
            continue;
        }
        let p: Vec<V3> = face
            .p
            .iter()
            .map(|q| [q[0] as f32 * scale, q[1] as f32 * scale, q[2] as f32 * scale])
            .collect();
        let n = maths::normalize([face.n[0] as f32, face.n[1] as f32, face.n[2] as f32]);
        // Newell's normal of the polygon as listed: if it disagrees with `n`, the fan is reversed.
        let mut newell = [0.0f32; 3];
        for (i, a) in p.iter().enumerate() {
            let b = p[(i + 1) % p.len()];
            newell = maths::add(newell, maths::cross(*a, b));
        }
        let flip = maths::dot(newell, n) < 0.0;
        let rgb = face.rgb.map(|c| (c.trunc() / 255.0) as f32);
        let flags = if face.emit { FLAG_EMIT } else { 0 } | if face.two { FLAG_TWO_SIDED } else { 0 };
        let mut emit_side = |n: V3, reverse: bool| {
            let base = out.verts.len() as u32;
            out.verts.extend(p.iter().map(|&pos| Vertex {
                rgb,
                flags,
                spec: face.spec as f32,
                ..Vertex::new(pos, n)
            }));
            for i in 1..p.len() as u32 - 1 {
                if reverse {
                    out.idx.extend([base, base + i + 1, base + i]);
                } else {
                    out.idx.extend([base, base + i, base + i + 1]);
                }
            }
        };
        emit_side(n, flip);
        if face.two {
            emit_side(maths::scale(n, -1.0), !flip);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use bf_picture::sprite::{Face, Mesh};

    /// Every triangle with area faces the way its vertices' normals point.
    fn winds_outward(m: &MeshData) -> bool {
        m.idx.as_chunks::<3>().0.iter().all(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| m.verts[i as usize]);
            let g = maths::cross(maths::sub(b.pos, a.pos), maths::sub(c.pos, a.pos));
            if maths::length(g) < 1e-9 {
                return true;
            }
            let n = maths::add(maths::add(a.nrm, b.nrm), c.nrm);
            maths::dot(g, n) > 0.0
        })
    }

    #[test]
    fn icosphere_is_closed_and_unit() {
        for subdiv in 0..4 {
            let m = icosphere(1.0, subdiv);
            assert_eq!(m.idx.len() % 3, 0);
            assert_eq!(m.idx.len() / 3, 20 * 4usize.pow(subdiv));
            for v in &m.verts {
                assert!((maths::length(v.pos) - 1.0).abs() <= 1e-6, "{:?}", v.pos);
            }
            // Closed: every edge is shared by exactly two triangles, once each way.
            let mut edges = std::collections::HashMap::new();
            for t in m.idx.as_chunks::<3>().0 {
                for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                    *edges.entry((a, b)).or_insert(0) += 1;
                }
            }
            for (&(a, b), &k) in &edges {
                assert_eq!(k, 1);
                assert_eq!(edges.get(&(b, a)), Some(&1), "edge {a}-{b} is open");
            }
            assert!(winds_outward(&m));
        }
    }

    #[test]
    fn primitives_wind_outward() {
        assert!(winds_outward(&box_([1.0, 2.0, 3.0])));
        assert!(winds_outward(&plane(4.0, 2.0)));
        assert!(winds_outward(&cylinder(0.5, 2.0, 12)));
        assert!(winds_outward(&capsule([0.0, 0.0, 0.0], [1.0, 2.0, -0.5], 0.3, 12)));
        assert!(winds_outward(&capsule([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], 0.3, 8)));
        // The plane faces up.
        assert!(plane(1.0, 1.0).verts.iter().all(|v| v.nrm == [0.0, 1.0, 0.0]));
    }

    /// Every vertex has a unit tangent at right angles to its normal and a handedness of ±1.
    fn tangents_ok(m: &MeshData) -> bool {
        m.verts.iter().all(|v| {
            let t = [v.tan[0], v.tan[1], v.tan[2]];
            (maths::length(t) - 1.0).abs() < 1e-3 && maths::dot(t, v.nrm).abs() < 1e-3 && v.tan[3].abs() == 1.0
        })
    }

    #[test]
    fn plane_maps_uv_along_x_and_z() {
        let m = plane(4.0, 2.0);
        for v in &m.verts {
            // u runs 0..1 along +X, v 0..1 along -Z (from the near edge to the far one)
            assert!((v.uv[0] - (v.pos[0] / 4.0 + 0.5)).abs() < 1e-6, "{v:?}");
            assert!((v.uv[1] - (0.5 - v.pos[2] / 2.0)).abs() < 1e-6, "{v:?}");
            assert_eq!(v.tan, [1.0, 0.0, 0.0, 1.0], "{v:?}");
        }
    }

    #[test]
    fn primitives_have_tangent_frames() {
        assert!(tangents_ok(&box_([1.0, 2.0, 3.0])));
        assert!(tangents_ok(&plane(4.0, 2.0)));
        assert!(tangents_ok(&cylinder(0.5, 2.0, 12)));
        assert!(tangents_ok(&capsule([0.0, 0.0, 0.0], [1.0, 2.0, -0.5], 0.3, 12)));
        assert!(tangents_ok(&uv_sphere(0.6, 32, 16)));
        assert!(tangents_ok(&icosphere(1.0, 2)));
    }

    #[test]
    fn uv_sphere_is_round_and_wraps_its_uvs() {
        let m = uv_sphere(0.6, 24, 12);
        assert!(winds_outward(&m));
        for v in &m.verts {
            assert!((maths::length(v.pos) - 0.6).abs() < 1e-5);
            assert!((0.0..=1.0).contains(&v.uv[0]) && (0.0..=1.0).contains(&v.uv[1]));
        }
        // The tangent runs east, the way u grows: around +Y.
        let eq = m
            .verts
            .iter()
            .find(|v| v.pos[1].abs() < 1e-4 && v.pos[2].abs() < 1e-4 && v.pos[0] > 0.0);
        let eq = eq.expect("a vertex on +X at the equator");
        assert!(eq.tan[2].abs() > 0.99, "{eq:?}");
    }

    #[test]
    fn compute_tangents_follows_the_uvs() {
        // A quad in the XY plane facing +Z with u along +Y: the tangent is +Y.
        let mut m = MeshData::default();
        for (p, uv) in [
            ([0.0, 0.0, 0.0], [0.0, 0.0]),
            ([0.0, 1.0, 0.0], [1.0, 0.0]),
            ([-1.0, 1.0, 0.0], [1.0, 1.0]),
        ] {
            let mut v = Vertex::new(p, [0.0, 0.0, 1.0]);
            v.uv = uv;
            m.verts.push(v);
        }
        m.idx = vec![0, 1, 2];
        let m = m.with_tangents();
        for v in &m.verts {
            assert!((v.tan[1] - 1.0).abs() < 1e-5 && v.tan[0].abs() < 1e-5, "{v:?}");
            // bitangent = w * n x t = w * (+Z x +Y) = w * (-X); v grows along -X, so w = +1
            assert_eq!(v.tan[3], 1.0);
        }
    }

    #[test]
    fn capsule_spans_its_segment() {
        let (a, b, r) = ([1.0, 0.0, 0.0], [1.0, 0.0, 4.0], 0.5);
        let m = capsule(a, b, r, 16);
        let zs: Vec<f32> = m.verts.iter().map(|v| v.pos[2]).collect();
        let lo = zs.iter().cloned().fold(f32::INFINITY, f32::min);
        let hi = zs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!((lo + 0.5).abs() < 1e-5 && (hi - 4.5).abs() < 1e-5, "{lo} {hi}");
    }

    fn face(k: usize, two: bool) -> Face {
        let p = (0..k)
            .map(|i| {
                let a = i as f64 / k as f64 * std::f64::consts::TAU;
                [a.cos(), a.sin(), 0.0]
            })
            .collect();
        Face {
            p,
            n: [0.0, 0.0, 1.0],
            rgb: [255.0, 127.9, 0.0],
            two,
            emit: true,
            gid: 0,
            spec: 0.5,
        }
    }

    #[test]
    fn sprite_mesh_triangulates_fans() {
        let m = from_sprite_mesh(
            &Mesh {
                faces: vec![face(5, false)],
            },
            2.0,
        );
        assert_eq!(m.idx.len(), 9, "a 5-vertex face gives 3 triangles");
        assert_eq!(m.verts.len(), 5);
        let v = m.verts[0];
        assert_eq!(v.pos, [2.0, 0.0, 0.0]);
        assert_eq!(v.nrm, [0.0, 0.0, 1.0]);
        assert_eq!(v.rgb, [1.0, 127.0 / 255.0, 0.0]);
        assert_eq!(v.flags, FLAG_EMIT);
        assert_eq!(v.spec, 0.5);
        assert!(winds_outward(&m));
        // Listed clockwise: still wound to face `n`.
        let mut cw = face(4, false);
        cw.p.reverse();
        assert!(winds_outward(&from_sprite_mesh(&Mesh { faces: vec![cw] }, 1.0)));
    }

    #[test]
    fn two_sided_faces_get_both_windings() {
        let m = from_sprite_mesh(
            &Mesh {
                faces: vec![face(4, true)],
            },
            1.0,
        );
        assert_eq!(m.idx.len(), 12);
        assert!(winds_outward(&m));
        assert!(m.verts.iter().all(|v| v.flags & FLAG_TWO_SIDED != 0));
        assert_eq!(m.verts[4].nrm, [0.0, 0.0, -1.0]);
    }
}
