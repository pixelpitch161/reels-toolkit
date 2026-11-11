//! `reelkit/geometry.py`: the low-poly builders the ring uses. Every builder returns faces: the corners and a
//! unit normal.
//!
//! How close to Python (`studio/reference/picture_kit/geometry.npz`): corners that involve no angle are exact
//! (`face`, `catenary`, a `box_` with `rot` 0); corners on a circle go through `math.cos` / `math.sin`, which on
//! the reference PC can differ from `bf_core::dmath` in the last bit; normals go through `np.linalg.norm`, which
//! is fused there (see [`crate::np`]). Those are held to 1e-9 of each value (or 1e-12 of the largest coordinate
//! of its point, where that is wider) and agree to about 1e-16.
//!
//! Not ported (the BALLFIGHT renderer does not call them): `icosa`, `gable`, `mound`, `ridge`, and the `Obj`
//! class with its occluder bookkeeping.

use crate::v3::{self, V3};
use bf_core::dmath;

/// A face: its corners in order, and its unit normal.
pub type Face = (Vec<V3>, V3);

/// `face(pts, center=None, up=False)`: the face of these corners with the normal of its first three. `up` turns
/// a normal that points down; `center` turns one that points toward that point (so the normal faces away from
/// the solid's middle).
pub fn face(pts: &[V3], center: Option<V3>, up: bool) -> Face {
    let q = pts.to_vec();
    let n = v3::cross(v3::sub(q[1], q[0]), v3::sub(q[2], q[0]));
    let mut n = v3::div(n, v3::norm(n) + 1e-12);
    if up && n[1] < 0.0 {
        n = v3::neg(n);
    }
    if let Some(c) = center
        && v3::dot(n, v3::sub(v3::mean(&q), c)) < 0.0
    {
        n = v3::neg(n);
    }
    (q, n)
}

/// `box(cx, cy, cz, hx, hy, hz, rot=0.0)`: an axis box with half sizes, turned by `rot` radians about y. Six
/// faces: the four sides, the top, the bottom.
pub fn box_(cx: f64, cy: f64, cz: f64, hx: f64, hy: f64, hz: f64, rot: f64) -> Vec<Face> {
    let (c, s) = (dmath::cos(rot), dmath::sin(rot));
    let p = |x: f64, y: f64, z: f64| [cx + x * c - z * s, cy + y, cz + x * s + z * c];
    let mut v = Vec::with_capacity(8);
    for sy in [-1.0, 1.0] {
        for (sx, sz) in [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)] {
            v.push(p(sx * hx, sy * hy, sz * hz));
        }
    }
    const QUADS: [[usize; 4]; 6] = [
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
        [4, 5, 6, 7],
        [0, 3, 2, 1],
    ];
    QUADS
        .iter()
        .map(|q| face(&q.map(|i| v[i]), Some([cx, cy, cz]), false))
        .collect()
}

/// The angles of an `n`-sided ring: `[rot + 2 * math.pi * k / n for k in range(n)]`.
fn ring_angles(n: usize, rot: f64) -> Vec<f64> {
    (0..n).map(|k| rot + 2.0 * dmath::PI * k as f64 / n as f64).collect()
}

/// `prism(cx, cz, r, y0, y1, n=8, r1=None, rot=0.0, tilt=(0.0, 0.0))`: an `n`-sided prism, or a frustum with
/// radius `r` at `y0` and `r1` at `y1` (`None`: the same radius); `tilt` slants the top. `n` side faces, then
/// the top cap.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn prism(
    cx: f64,
    cz: f64,
    r: f64,
    y0: f64,
    y1: f64,
    n: usize,
    r1: Option<f64>,
    rot: f64,
    tilt: (f64, f64),
) -> Vec<Face> {
    let r1 = r1.unwrap_or(r);
    let ang = ring_angles(n, rot);
    let top_y = |x: f64, z: f64| y1 + tilt.0 * (x - cx) + tilt.1 * (z - cz);
    let mut out = Vec::with_capacity(n + 1);
    for k in 0..n {
        let (a0, a1) = (ang[k], ang[(k + 1) % n]);
        let b0 = [cx + r * dmath::cos(a0), y0, cz + r * dmath::sin(a0)];
        let b1 = [cx + r * dmath::cos(a1), y0, cz + r * dmath::sin(a1)];
        let mut t1 = [cx + r1 * dmath::cos(a1), 0.0, cz + r1 * dmath::sin(a1)];
        let mut t0 = [cx + r1 * dmath::cos(a0), 0.0, cz + r1 * dmath::sin(a0)];
        t1[1] = top_y(t1[0], t1[2]);
        t0[1] = top_y(t0[0], t0[2]);
        out.push(face(&[b0, b1, t1, t0], Some([cx, (y0 + y1) / 2.0, cz]), false));
    }
    let cap: Vec<V3> = ang
        .iter()
        .rev()
        .map(|&a| {
            let (x, z) = (cx + r1 * dmath::cos(a), cz + r1 * dmath::sin(a));
            [x, top_y(x, z), z]
        })
        .collect();
    out.push(face(&cap, None, true));
    out
}

/// `lathe(cx, cz, prof, n=8, rot=0.0, cy=0.0)`: a surface of revolution from `(y, r)` pairs, bottom to top, with
/// a cap on the last pair if its radius is above 1e-3. A band whose two radii are both under 1e-4 is left out.
pub fn lathe(cx: f64, cz: f64, prof: &[(f64, f64)], n: usize, rot: f64, cy: f64) -> Vec<Face> {
    let ang = ring_angles(n, rot);
    let mut faces = Vec::new();
    for pair in prof.windows(2) {
        let ((y0, r0), (y1, r1)) = (pair[0], pair[1]);
        if r0 < 1e-4 && r1 < 1e-4 {
            continue;
        }
        for k in 0..n {
            let (a0, a1) = (ang[k], ang[(k + 1) % n]);
            let q = [
                [cx + r0 * dmath::cos(a0), cy + y0, cz + r0 * dmath::sin(a0)],
                [cx + r0 * dmath::cos(a1), cy + y0, cz + r0 * dmath::sin(a1)],
                [cx + r1 * dmath::cos(a1), cy + y1, cz + r1 * dmath::sin(a1)],
                [cx + r1 * dmath::cos(a0), cy + y1, cz + r1 * dmath::sin(a0)],
            ];
            // A band that starts at a point has no lower edge: its normal comes from the upper one.
            let nn = if r0 > 1e-4 {
                v3::cross(v3::sub(q[1], q[0]), v3::sub(q[3], q[0]))
            } else {
                v3::cross(v3::sub(q[2], q[3]), v3::sub(q[0], q[3]))
            };
            let mut nn = v3::div(nn, v3::norm(nn) + 1e-12);
            // (For the band's last face a1 is the ring's first angle, so `am` is on the far side of the axis and
            // the normal ends up pointing inward. Kept: the kit's pictures are made with it.)
            let am = (a0 + a1) / 2.0;
            if v3::dot(nn, [dmath::cos(am), 0.0, dmath::sin(am)]) < 0.0 {
                nn = v3::neg(nn);
            }
            faces.push((q.to_vec(), nn));
        }
    }
    if let Some(&(y, r)) = prof.last()
        && r > 1e-3
    {
        let cap = ang
            .iter()
            .rev()
            .map(|&a| [cx + r * dmath::cos(a), cy + y, cz + r * dmath::sin(a)])
            .collect();
        faces.push((cap, [0.0, 1.0, 0.0]));
    }
    faces
}

/// `np.linspace(0, 1, n)`: `k * (1 / (n - 1))`, with the last value set to 1 exactly.
fn linspace01(n: usize) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![0.0];
    }
    let step = 1.0 / (n - 1) as f64;
    let mut u: Vec<f64> = (0..n).map(|k| k as f64 * step + 0.0).collect();
    u[n - 1] = 1.0;
    u
}

/// `catenary(a, b, sag, n)`: `n` points along a string hanging between `a` and `b`, `sag` lower in the middle.
pub fn catenary(a: V3, b: V3, sag: f64, n: usize) -> Vec<V3> {
    let d = v3::sub(b, a);
    linspace01(n)
        .into_iter()
        .map(|u| {
            let mut p = v3::add(a, v3::scale(d, u));
            p[1] -= sag * 4.0 * u * (1.0 - u);
            p
        })
        .collect()
}

/// `thin_quad(a, b, w)`: a ribbon of width `w` between two points, lying across the z axis (stems, legs; also a
/// shadow caster). Two points above each other in z give a ribbon without width.
pub fn thin_quad(a: V3, b: V3, w: f64) -> [V3; 4] {
    let d = v3::sub(b, a);
    let p = v3::cross(d, [0.0, 0.0, 1.0]);
    // p / (norm + 1e-9) * w / 2
    let p = v3::div(v3::scale(v3::div(p, v3::norm(p) + 1e-9), w), 2.0);
    [v3::add(a, p), v3::add(b, p), v3::sub(b, p), v3::sub(a, p)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: V3, b: V3) -> bool {
        (0..3).all(|k| (a[k] - b[k]).abs() < 1e-9)
    }

    #[test]
    fn a_face_turns_its_normal_up_or_away() {
        let tri = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        // x cross z points down.
        assert!(near(face(&tri, None, false).1, [0.0, -1.0, 0.0]));
        assert!(near(face(&tri, None, true).1, [0.0, 1.0, 0.0]));
        assert!(near(face(&tri, Some([0.0, 5.0, 0.0]), false).1, [0.0, -1.0, 0.0]));
        assert!(near(face(&tri, Some([0.0, -5.0, 0.0]), false).1, [0.0, 1.0, 0.0]));
        assert_eq!(face(&tri, None, false).0, tri.to_vec());
    }

    #[test]
    fn a_box_has_six_faces_looking_out() {
        let faces = box_(1.0, 2.0, 3.0, 0.5, 1.0, 2.0, 0.0);
        assert_eq!(faces.len(), 6);
        let normals: Vec<V3> = faces.iter().map(|f| f.1).collect();
        for (n, want) in normals.iter().zip([
            [1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            [-1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
        ]) {
            assert!(near(*n, want), "{n:?} {want:?}");
        }
        assert_eq!(faces[0].0[0], [1.5, 1.0, 5.0]);
        assert_eq!(
            faces[4].0,
            vec![[1.5, 3.0, 5.0], [1.5, 3.0, 1.0], [0.5, 3.0, 1.0], [0.5, 3.0, 5.0]]
        );
    }

    #[test]
    fn a_prism_is_sides_and_a_cap() {
        let faces = prism(0.0, 0.0, 1.0, 0.0, 2.0, 6, None, 0.0, (0.0, 0.0));
        assert_eq!(faces.len(), 7);
        assert!(faces[..6].iter().all(|f| f.0.len() == 4 && f.1[1].abs() < 1e-12));
        assert_eq!(faces[6].0.len(), 6);
        assert!(near(faces[6].1, [0.0, 1.0, 0.0]));
        assert_eq!(faces[0].0[0], [1.0, 0.0, 0.0]);
        // A frustum's sides lean; a tilt lifts one side of the top.
        let cone = prism(0.0, 0.0, 1.0, 0.0, 1.0, 8, Some(0.5), 0.0, (0.0, 0.0));
        assert!(cone[0].1[1] > 0.1);
        let cut = prism(0.0, 0.0, 1.0, 0.0, 1.0, 4, None, 0.0, (0.5, 0.0));
        assert_eq!(cut[0].0[3], [1.0, 1.5, 0.0]);
    }

    #[test]
    fn a_lathe_skips_needles_and_caps_wide_tops() {
        let jar = lathe(0.0, 0.0, &[(0.0, 0.3), (0.5, 0.5), (1.0, 0.2)], 8, 0.0, 0.0);
        assert_eq!(jar.len(), 8 + 8 + 1);
        assert!(near(jar[16].1, [0.0, 1.0, 0.0]));
        let spindle = lathe(
            0.0,
            0.0,
            &[(0.0, 0.0), (0.1, 0.00005), (0.4, 0.3), (0.5, 0.0005)],
            5,
            0.0,
            0.0,
        );
        assert_eq!(spindle.len(), 5 + 5);
        // The side normals point away from the axis, but for the last face of each band: its two angles are
        // the ring's last and first, their mean lies across the axis, and Python turns the normal toward it.
        for (k, (q, n)) in jar[..16].iter().enumerate() {
            let mid = v3::mean(q);
            assert_eq!(n[0] * mid[0] + n[2] * mid[2] > 0.0, k % 8 != 7, "face {k}");
        }
    }

    #[test]
    fn a_catenary_hangs_between_its_ends() {
        let p = catenary([0.0, 1.0, 0.0], [2.0, 1.0, 4.0], 0.5, 5);
        assert_eq!(p.len(), 5);
        assert_eq!((p[0], p[4]), ([0.0, 1.0, 0.0], [2.0, 1.0, 4.0]));
        assert_eq!(p[2], [1.0, 0.5, 2.0]);
        assert_eq!(linspace01(1), [0.0]);
        assert_eq!(linspace01(3), [0.0, 0.5, 1.0]);
    }

    #[test]
    fn a_thin_quad_is_as_wide_as_asked() {
        let q = thin_quad([0.0, 0.0, 0.0], [0.0, 2.0, 0.0], 0.5);
        assert!(near(q[0], [0.25, 0.0, 0.0]) && near(q[3], [-0.25, 0.0, 0.0]));
        assert!(near(q[1], [0.25, 2.0, 0.0]) && near(q[2], [-0.25, 2.0, 0.0]));
        let flat = thin_quad([1.0, 1.0, 0.0], [1.0, 1.0, 3.0], 0.5);
        assert_eq!(flat[0], flat[3]);
    }
}
