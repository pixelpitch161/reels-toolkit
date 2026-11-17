//! The base shot: the current engine's camera, in Engine 2's render space (E7, "same style, higher fidelity").
//!
//! The current engine (`bf_picture::ring::state::camera_keys`) frames the whole ring from 27.5 away, 58 degrees
//! down, a 36 degree field of view, its optical centre 32 rows above the middle of the 360 x 640 picture, with a
//! gentle drift over the minute and a slight push-in for the final exchange. Its world is Engine 2's render space
//! turned 45 degrees about Y (`bf_picture::scene::to_world`): the ring stands as a diamond, fighter 0's corner on
//! the left, fighter 1's on the right. Engine 2's director shots are offsets around this pose.
//!
//! The off-centre optical centre has no place in a `CamPose`, so the pose looks down that much further instead
//! (the ray through the middle of the picture becomes the view axis): the same picture but for a keystone of a
//! fraction of a pixel.

use bf_core::dmath;
use bf_picture::camera::{Cam, interp_keys};
use bf_picture::ring::state::camera_keys;

use crate::project::{CamPose, P3};

/// The current engine's picture height in its own pixels (`bf_picture::config::H`).
const CUR_H: f64 = bf_picture::config::H as f64;

/// A current-engine world point in Engine 2's render space (`bf_picture::scene::to_ring` on x and z).
pub fn from_world(p: P3) -> P3 {
    let (x, z) = bf_picture::scene::to_ring(p[0], p[2]);
    [x, p[1], z]
}

/// The current engine's camera at video time `t` of a fight whose K.O. is at video time `t_ko`, unshaken.
pub fn current_cam(t: f64, t_ko: f64) -> Cam {
    interp_keys(&camera_keys(t_ko), t, None)
}

/// The base shot as the director's rig numbers: (target, distance, pitch and yaw in degrees), in render space.
/// Yaw is measured as the director's `Rig` measures it (the eye at `sin yaw, cos yaw` from the target).
pub fn base_rig(t: f64, t_ko: f64) -> (P3, f64, f64, f64) {
    let cam = current_cam(t, t_ko);
    let eye = from_world(cam.pos);
    // The ray through the middle row (the optical centre is `cy`): camera-space (0, -(H/2 - cy) / f, 1).
    let dy = -(CUR_H / 2.0 - cam.cy) / cam.f;
    let m = cam.m;
    let ray = [0, 1, 2].map(|j| m[1][j] * dy + m[2][j]);
    let fwd = from_world(ray);
    let len = (fwd[0] * fwd[0] + fwd[1] * fwd[1] + fwd[2] * fwd[2]).sqrt();
    let fwd = fwd.map(|c| c / len);
    let d = cam
        .pos
        .iter()
        .zip(cam.target)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt();
    let target = [0, 1, 2].map(|k| eye[k] + fwd[k] * d);
    let pitch = dmath::asin(-fwd[1]).to_degrees();
    let yaw = dmath::atan2(-fwd[0], -fwd[2]).to_degrees();
    (target, d, pitch, yaw)
}

/// The base shot as a pose.
pub fn base_pose(t: f64, t_ko: f64) -> CamPose {
    let (target, d, pitch, yaw) = base_rig(t, t_ko);
    let (p, y) = (pitch.to_radians(), yaw.to_radians());
    CamPose {
        eye: [
            target[0] + d * dmath::cos(p) * dmath::sin(y),
            target[1] + d * dmath::sin(p),
            target[2] + d * dmath::cos(p) * dmath::cos(y),
        ],
        target,
        roll: 0.0,
        fov_deg: 36.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{H, W, project};
    use bf_picture::scene::{AP, CORNERS, POST_H, to_world};

    /// Every corner of the canvas and every post top lands where the current engine draws it (its pixels times
    /// 3), within 3 pixels of 1080 x 1920 (the keystone of looking down instead of shifting the lens), at several
    /// moments of the minute.
    #[test]
    fn base_pose_draws_the_ring_where_the_current_engine_does() {
        let t_ko = 47.0;
        for t in [0.0, 10.0, 30.0, 46.0, 48.5, 60.0] {
            let cam = current_cam(t, t_ko);
            let pose = base_pose(t, t_ko);
            let mut pts = Vec::new();
            for sx in [-AP, AP] {
                for sz in [-AP, AP] {
                    pts.push(to_world(sx, sz, 0.0));
                }
            }
            for c in CORNERS {
                pts.push(to_world(c[0], c[1], POST_H));
            }
            pts.push([0.0, 0.0, 0.0]);
            for p in pts {
                let (cx, cy) = cam.wp(&[p])[0];
                let (ex, ey) = project(&pose, from_world(p), W, H).expect("in front");
                assert!(
                    (ex - 3.0 * cx).abs() < 3.0 && (ey - 3.0 * cy).abs() < 3.0,
                    "t {t} {p:?}: current ({}, {}) engine 2 ({ex}, {ey})",
                    3.0 * cx,
                    3.0 * cy
                );
            }
        }
    }

    /// The diamond: fighter 0's corner (sim −x, −y) on the left of the picture, fighter 1's on the right, the
    /// two at the same height.
    #[test]
    fn fighters_corners_are_left_and_right() {
        let pose = base_pose(0.0, 47.0);
        let corner = |s: f64| project(&pose, crate::project::to_render([s * 3.24, s * 3.24, 0.0]), W, H).unwrap();
        let (a, b) = (corner(-1.0), corner(1.0));
        assert!(a.0 < 0.15 * W && b.0 > 0.85 * W, "{a:?} {b:?}");
        assert!((a.1 - b.1).abs() < 2.0, "{a:?} {b:?}");
    }
}
