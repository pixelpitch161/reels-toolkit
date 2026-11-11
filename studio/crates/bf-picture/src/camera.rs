//! `reelkit/camera.py`: the look-at perspective camera. World: y up; a camera with yaw 0 sits on +z looking toward
//! -z.
//!
//! Every method keeps the Python's operation order. Points are rows `[x, y, z]`; a screen position is `(x, y)` in
//! internal pixels, unrounded (the drawing calls of [`crate::pil`] truncate them, as Pillow does).
//!
//! How close to Python: `clip_near` and the interpolation in `interp_keys` are plain arithmetic and exact. The
//! rest goes through a sine, a norm or a matrix product, so a result can differ from Python's in its last bit
//! (`math.sin` on the reference PC is the Windows C runtime, numpy's products are fused, see [`crate::np`]);
//! `studio/reference/picture_kit/camera.npz` holds every value to 1e-9 of itself, or to 1e-12 of the largest
//! coordinate of its point where that is wider; they agree to about 1e-15 of the point's size.
//! One consequence to know: a component that is zero by geometry (the camera-space x of the look-at point) comes
//! out as rounding noise of either sign, here and in Python, and not the same noise.

use crate::config::{self, H, W};
use crate::np;
use crate::v3;
use bf_core::dmath;

pub use crate::v3::V3;

/// `NEAR`: the default near plane, in world units in front of the camera.
pub const NEAR: f64 = 0.1;

/// `math.radians(x)`: CPython multiplies by the constant `pi / 180`.
fn radians(x: f64) -> f64 {
    x * (dmath::PI / 180.0)
}

/// `Cam`: a camera looking at a point.
///
/// The fields are public as in Python: the ring's camera shake adds to `cx` and `cy` after the camera is built.
#[derive(Clone, Debug, PartialEq)]
pub struct Cam {
    /// The camera's position in the world.
    pub pos: V3,
    /// `M`: the rows are the camera's right, up and forward unit vectors.
    pub m: [V3; 3],
    /// The focal length in pixels.
    pub f: f64,
    /// The screen x of the optical centre.
    pub cx: f64,
    /// The screen y of the optical centre.
    pub cy: f64,
    /// The near plane.
    pub near: f64,
    /// The look-at point.
    pub target: V3,
}

impl Cam {
    /// `Cam(target, dist, yaw, pitch, fov, sy=None, roll=0.0, near=NEAR)`: `yaw` and `pitch` in degrees (pitch
    /// above 0 looks down from above), `fov` the vertical field of view in degrees, `sy` the screen y of the
    /// optical centre (`None`: the middle of the frame). The Python defaults are `None`, `0.0` and [`NEAR`].
    ///
    /// A camera looking straight down or up (pitch 90 or -90) has no right vector: its matrix is NaN, as in
    /// Python.
    #[allow(clippy::too_many_arguments)] // the Python signature
    pub fn new(target: V3, dist: f64, yaw: f64, pitch: f64, fov: f64, sy: Option<f64>, roll: f64, near: f64) -> Cam {
        let (yr, pr) = (radians(yaw), radians(pitch));
        let offset = [
            dmath::sin(yr) * dmath::cos(pr),
            dmath::sin(pr),
            dmath::cos(yr) * dmath::cos(pr),
        ];
        let pos = v3::add(target, v3::scale(offset, dist));
        let fwd = v3::sub(target, pos);
        let fwd = v3::div(fwd, v3::norm(fwd));
        let right = v3::cross(fwd, [0.0, 1.0, 0.0]);
        let mut right = v3::div(right, v3::norm(right));
        let mut up = v3::cross(right, fwd);
        if roll != 0.0 {
            let (c, s) = (dmath::cos(radians(roll)), dmath::sin(radians(roll)));
            // right, up = right * c + up * s, -right * s + up * c
            let rolled = v3::add(v3::scale(right, c), v3::scale(up, s));
            up = v3::add(v3::scale(v3::neg(right), s), v3::scale(up, c));
            right = rolled;
        }
        let half = H as f64 / 2.0;
        Cam {
            pos,
            m: [right, up, fwd],
            f: half / dmath::tan(radians(fov) / 2.0),
            cx: W as f64 / 2.0,
            cy: sy.unwrap_or(half),
            near,
            target,
        }
    }

    /// `cam.view`: the unit vector from the scene toward the camera, `-M[2]`.
    pub fn view(&self) -> V3 {
        v3::neg(self.m[2])
    }

    /// `to_cam(P)`: world points into camera space (x right, y up, z forward), `(P - pos) @ M.T`.
    pub fn to_cam(&self, p: &[V3]) -> Vec<V3> {
        p.iter()
            .map(|&p| {
                let d = v3::sub(p, self.pos);
                [v3::dot(d, self.m[0]), v3::dot(d, self.m[1]), v3::dot(d, self.m[2])]
            })
            .collect()
    }

    /// `proj(Pc)`: camera-space points onto the screen. The depth is held at 1e-3 or more, so a point at or
    /// behind the camera gives a far-off but finite position.
    pub fn proj(&self, pc: &[V3]) -> Vec<(f64, f64)> {
        pc.iter()
            .map(|p| {
                let z = np::maximum(p[2], 1e-3);
                (self.cx + self.f * p[0] / z, self.cy - self.f * p[1] / z)
            })
            .collect()
    }

    /// `wp(P)`: world points onto the screen.
    pub fn wp(&self, p: &[V3]) -> Vec<(f64, f64)> {
        self.proj(&self.to_cam(p))
    }

    /// `depth(P)`: the camera-space z of world points (`cam.depth(p)[0]` in Python is `cam.depth(&[p])[0]`).
    pub fn depth(&self, p: &[V3]) -> Vec<f64> {
        self.to_cam(p).iter().map(|c| c[2]).collect()
    }

    /// `pixel_rays(xs, ys)`: for pixel coordinates (`float64`, as `kit.picture` passes them), the world-space
    /// direction of the ray through each, scaled so its camera-z component is 1. `xs` and `ys` run together.
    pub fn pixel_rays(&self, xs: &[f64], ys: &[f64]) -> Vec<V3> {
        assert_eq!(xs.len(), ys.len(), "pixel_rays: xs and ys differ in length");
        let m = &self.m;
        xs.iter()
            .zip(ys)
            .map(|(&x, &y)| {
                let d = [(x - self.cx) / self.f, -(y - self.cy) / self.f, 1.0];
                // d @ M: column j of M against d.
                [
                    d[0] * m[0][0] + d[1] * m[1][0] + d[2] * m[2][0],
                    d[0] * m[0][1] + d[1] * m[1][1] + d[2] * m[2][1],
                    d[0] * m[0][2] + d[1] * m[1][2] + d[2] * m[2][2],
                ]
            })
            .collect()
    }

    /// `ground_hit(xs, ys, y0=0.0)`: where the rays through the pixels meet the plane `y = y0`: `(X, Z, t)`, the
    /// world x and z and the camera depth of each hit. A ray that does not point down is treated as pointing
    /// down very slightly (its y held at -1e-6 or less), which puts its hit far away.
    pub fn ground_hit(&self, xs: &[f64], ys: &[f64], y0: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let rays = self.pixel_rays(xs, ys);
        let (mut gx, mut gz, mut gt) = (
            Vec::with_capacity(rays.len()),
            Vec::with_capacity(rays.len()),
            Vec::with_capacity(rays.len()),
        );
        for dw in rays {
            let t = (y0 - self.pos[1]) / np::minimum(dw[1], -1e-6);
            gx.push(self.pos[0] + t * dw[0]);
            gz.push(self.pos[2] + t * dw[2]);
            gt.push(t);
        }
        (gx, gz, gt)
    }
}

/// `clip_near(Pc, near=NEAR)`: a camera-space polygon cut against the plane `z = near` (Sutherland-Hodgman).
/// `None` when fewer than three corners are left (the polygon is behind the plane).
pub fn clip_near(pc: &[V3], near: f64) -> Option<Vec<V3>> {
    let n = pc.len();
    let mut out = Vec::with_capacity(n + 2);
    for k in 0..n {
        let (a, b) = (pc[k], pc[(k + 1) % n]);
        let (ia, ib) = (a[2] >= near, b[2] >= near);
        if ia {
            out.push(a);
        }
        if ia != ib {
            let u = (near - a[2]) / (b[2] - a[2]);
            out.push(v3::add(a, v3::scale(v3::sub(b, a), u)));
        }
    }
    (out.len() >= 3).then_some(out)
}

/// One camera keyframe of `interp_keys`: `(t, target, dist, yaw, pitch, fov, sy)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    pub t: f64,
    pub target: V3,
    pub dist: f64,
    pub yaw: f64,
    pub pitch: f64,
    pub fov: f64,
    pub sy: f64,
}

/// `interp_keys(keys, t, ease_fn=None)`: the camera at time `t` between keyframes (sorted by time): the first
/// key before it, the last after it, and in between each value eased from one key to the next. `ease_fn` `None`
/// is `config::ease`. Panics on an empty list.
pub fn interp_keys(keys: &[Key], t: f64, ease_fn: Option<fn(f64) -> f64>) -> Cam {
    let (first, last) = (keys[0], keys[keys.len() - 1]);
    // (A `t` that is not a number takes the last key here; Python raises.)
    let mut k = if t <= first.t { first } else { last };
    if t > first.t && t < last.t {
        for pair in keys.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if t <= b.t {
                let u = ease_fn.unwrap_or(config::ease)((t - a.t) / (b.t - a.t));
                let mix = |x: f64, y: f64| x + (y - x) * u;
                k = Key {
                    t,
                    target: [
                        mix(a.target[0], b.target[0]),
                        mix(a.target[1], b.target[1]),
                        mix(a.target[2], b.target[2]),
                    ],
                    dist: mix(a.dist, b.dist),
                    yaw: mix(a.yaw, b.yaw),
                    pitch: mix(a.pitch, b.pitch),
                    fov: mix(a.fov, b.fov),
                    sy: mix(a.sy, b.sy),
                };
                break;
            }
        }
    }
    Cam::new(k.target, k.dist, k.yaw, k.pitch, k.fov, Some(k.sy), 0.0, NEAR)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring() -> Cam {
        Cam::new([0.12, 0.25, 0.0], 27.5, 0.0, 58.0, 36.0, Some(288.0), 0.0, NEAR)
    }

    #[test]
    fn a_camera_with_yaw_zero_looks_along_minus_z() {
        let cam = ring();
        // Yaw 0 leaves no rounding in the right vector: it is the world's x axis exactly.
        assert_eq!(cam.m[0], [1.0, 0.0, 0.0]);
        assert_eq!((cam.cx, cam.cy), (180.0, 288.0));
        assert!(cam.pos[2] > 0.0 && cam.m[2][2] < 0.0 && cam.m[2][1] < 0.0);
        assert_eq!(cam.view(), [-cam.m[2][0], -cam.m[2][1], -cam.m[2][2]]);
        // The look-at point lands on the optical centre, 27.5 away.
        let (x, y) = cam.wp(&[cam.target])[0];
        assert_eq!(x, 180.0);
        assert!((y - 288.0).abs() < 1e-9);
        assert!((cam.depth(&[cam.target])[0] - 27.5).abs() < 1e-12);
        // 36 degrees over 640 rows.
        assert!((cam.f - 984.8587).abs() < 1e-3, "{}", cam.f);
    }

    #[test]
    fn the_optical_centre_defaults_to_the_middle() {
        let cam = Cam::new([0.0; 3], 10.0, 30.0, 20.0, 50.0, None, 0.0, NEAR);
        assert_eq!((cam.cx, cam.cy), (180.0, 320.0));
    }

    #[test]
    fn projection_holds_the_depth_above_a_thousandth() {
        let cam = ring();
        let p = cam.proj(&[[1.0, 1.0, 1e-3], [1.0, 1.0, -5.0], [0.0, 0.0, 0.0]]);
        assert_eq!(p[0], p[1]);
        assert_eq!(p[2], (cam.cx, cam.cy));
        assert_eq!(p[0], (cam.cx + cam.f * 1.0 / 1e-3, cam.cy - cam.f * 1.0 / 1e-3));
    }

    #[test]
    fn rays_through_the_centre_hit_the_ground_under_the_target() {
        let cam = ring();
        let (x, z, t) = cam.ground_hit(&[cam.cx], &[cam.cy], 0.25);
        assert!((x[0] - 0.12).abs() < 1e-12 && z[0].abs() < 1e-9 && (t[0] - 27.5).abs() < 1e-9);
        // A ray above the horizon is held just below it: its hit is very far, never behind.
        let level = Cam::new([0.0, 1.0, 0.0], 10.0, 0.0, 0.0, 50.0, None, 0.0, NEAR);
        let (_, _, t) = level.ground_hit(&[180.0], &[0.0], 0.0);
        assert_eq!(t[0], (0.0 - level.pos[1]) / -1e-6);
    }

    #[test]
    fn clipping_keeps_the_part_in_front() {
        let tri = [[0.0, 0.0, 1.0], [1.0, 0.0, -1.0], [-1.0, 0.0, -1.0]];
        let cut = clip_near(&tri, 0.0).expect("a triangle in front");
        assert_eq!(cut, vec![[0.0, 0.0, 1.0], [0.5, 0.0, 0.0], [-0.5, 0.0, 0.0]]);
        assert_eq!(
            clip_near(&[[0.0, 0.0, 2.0], [1.0, 0.0, 2.0], [0.0, 1.0, 3.0]], NEAR).map(|p| p.len()),
            Some(3)
        );
        assert_eq!(
            clip_near(&[[0.0, 0.0, -2.0], [1.0, 0.0, -2.0], [0.0, 1.0, -3.0]], NEAR),
            None
        );
        // A corner on the plane counts as in front: what is left is that corner three times over.
        assert_eq!(clip_near(&tri, 1.0), Some(vec![[0.0, 0.0, 1.0]; 3]));
    }

    #[test]
    fn keys_hold_at_both_ends_and_ease_between() {
        let key = |t: f64, yaw: f64| Key {
            t,
            target: [0.0, 0.0, 0.0],
            dist: 10.0,
            yaw,
            pitch: 30.0,
            fov: 40.0,
            sy: 300.0,
        };
        let keys = [key(1.0, 0.0), key(3.0, 20.0), key(4.0, 40.0)];
        let at = |yaw: f64| Cam::new([0.0; 3], 10.0, yaw, 30.0, 40.0, Some(300.0), 0.0, NEAR);
        assert_eq!(interp_keys(&keys, 0.0, None), at(0.0));
        assert_eq!(interp_keys(&keys, 1.0, None), at(0.0));
        assert_eq!(interp_keys(&keys, 2.0, None), at(10.0));
        assert_eq!(interp_keys(&keys, 3.0, None), at(20.0));
        assert_eq!(interp_keys(&keys, 9.0, None), at(40.0));
        // ease(0.25) is far below a quarter; a straight line is a quarter.
        assert_eq!(interp_keys(&keys, 1.5, Some(|u| u)), at(5.0));
        assert_eq!(interp_keys(&keys, 1.5, None), at(20.0 * config::ease(0.25)));
    }
}
