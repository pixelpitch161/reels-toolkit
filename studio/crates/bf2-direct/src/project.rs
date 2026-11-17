//! The camera pose and a CPU projection that matches the renderer's (`bf2_render::scene::Camera::project`), in
//! `f64` on `dmath`, for the framing logic and the tests. This crate does not depend on `bf2-render` (the renderer
//! depends on it), so the pose is a plain struct the renderer converts.

use bf_core::dmath;

pub type P3 = [f64; 3];

/// The video's picture in pixels: shake offsets and the safe zone are in these.
pub const W: f64 = 1080.0;
pub const H: f64 = 1920.0;

/// Today's safe zone `(20, 96, 340, 500)` of 360 x 640 (`bf-picture/src/config.rs`), at 1080 x 1920:
/// `(x0, y0, x1, y1)`.
pub const SAFE: [f64; 4] = [60.0, 288.0, 1020.0, 1500.0];

/// The whole picture as a rectangle.
pub const FRAME: [f64; 4] = [0.0, 0.0, W, H];

/// A ball's radius (`Rules2::r`, `bf2_render::stage::BALL_R`).
pub const BALL_R: f64 = 0.6;

/// A camera in render space (Y up). `bf2-render` turns it into its `Camera`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CamPose {
    pub eye: P3,
    pub target: P3,
    /// Radians about the view direction, as `Camera::roll`.
    pub roll: f64,
    /// The vertical field of view.
    pub fov_deg: f64,
}

/// A sim point (x, y on the floor, z height) in render space (Y up): (x, z, −y), as `bf2_render::to_render`.
pub fn to_render(v: P3) -> P3 {
    [v[0], v[2], -v[1]]
}

/// A ball's centre in render space: the sim position is its bottom (as `bf2_render::stage::ball_centre`).
pub fn ball_centre(pos: P3) -> P3 {
    let r = to_render(pos);
    [r[0], r[1] + BALL_R, r[2]]
}

pub(crate) fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(crate) fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(a: P3) -> P3 {
    let l = dot(a, a).sqrt();
    if l > 0.0 { [a[0] / l, a[1] / l, a[2] / l] } else { a }
}

/// The view basis of `look_at`: side, up and forward.
fn basis(c: &CamPose) -> (P3, P3, P3) {
    let f = normalize(sub(c.target, c.eye));
    let mut s = normalize(cross(f, [0.0, 1.0, 0.0]));
    let mut u = cross(s, f);
    if c.roll != 0.0 {
        let (sn, cs) = (dmath::sin(c.roll), dmath::cos(c.roll));
        let s2 = [s[0] * cs + u[0] * sn, s[1] * cs + u[1] * sn, s[2] * cs + u[2] * sn];
        u = [u[0] * cs - s[0] * sn, u[1] * cs - s[1] * sn, u[2] * cs - s[2] * sn];
        s = s2;
    }
    (s, u, f)
}

/// The focal factor `1 / tan(fov / 2)`.
fn focal(c: &CamPose) -> f64 {
    1.0 / dmath::tan(c.fov_deg.to_radians() / 2.0)
}

/// Where `p_render` lands on a `w x h` picture, in pixels from the top left; `None` behind the camera.
pub fn project(c: &CamPose, p_render: P3, w: f64, h: f64) -> Option<(f64, f64)> {
    let (s, u, f) = basis(c);
    let d = sub(p_render, c.eye);
    let depth = dot(f, d);
    if depth <= 0.0 {
        return None;
    }
    let k = focal(c);
    let x = k / (w / h) * dot(s, d) / depth;
    let y = k * dot(u, d) / depth;
    Some(((x + 1.0) / 2.0 * w, (1.0 - y) / 2.0 * h))
}

/// The screen box of a ball centred at `centre` (render space) on the `W x H` picture, or `None` when any of it is
/// behind the camera. A sphere seen off axis is an ellipse a little longer than its on-axis circle, so the radius
/// gets 8 % on top.
pub fn ball_box(c: &CamPose, centre: P3) -> Option<[f64; 4]> {
    let (s, u, f) = basis(c);
    let d = sub(centre, c.eye);
    let depth = dot(f, d);
    if depth <= BALL_R * 1.5 {
        return None;
    }
    let k = focal(c);
    let x = k / (W / H) * dot(s, d) / depth;
    let y = k * dot(u, d) / depth;
    let (px, py) = ((x + 1.0) / 2.0 * W, (1.0 - y) / 2.0 * H);
    let r = 1.08 * BALL_R * k * (H / 2.0) / depth;
    Some([px - r, py - r, px + r, py + r])
}

/// Whether box `b` lies inside rectangle `r`.
pub fn inside(b: [f64; 4], r: [f64; 4]) -> bool {
    b[0] >= r[0] && b[1] >= r[1] && b[2] <= r[2] && b[3] <= r[3]
}

/// `r` shrunk by `share` of its width and height on every side.
pub fn inset(r: [f64; 4], share: f64) -> [f64; 4] {
    let (dx, dy) = ((r[2] - r[0]) * share, (r[3] - r[1]) * share);
    [r[0] + dx, r[1] + dy, r[2] - dx, r[3] - dy]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `bf2_render::scene::Camera::default_ring`: from 27.5 away, 58 degrees down, a 36 degree field of view.
    fn default_ring() -> CamPose {
        let (d, p) = (27.5, 58f64.to_radians());
        CamPose {
            eye: [0.0, d * dmath::sin(p), d * dmath::cos(p)],
            target: [0.0; 3],
            roll: 0.0,
            fov_deg: 36.0,
        }
    }

    /// `bf2-render` is not a dependency (it will depend on this crate), so its values are hard-coded here: they
    /// are its `to_render` (`to_render_swaps_axes`), its `ball_centre`, and its `Camera::project` worked through
    /// `maths::look_at` and `maths::perspective` by hand, for the default ring camera and a rolled one.
    #[test]
    fn to_render_matches_bf2_render() {
        assert_eq!(to_render([1.0, 2.0, 3.0]), [1.0, 3.0, -2.0]);
        assert_eq!(ball_centre([1.0, 2.0, 0.0]), [1.0, 0.6, -2.0]);
        let rolled = CamPose {
            eye: [3.0, 10.0, 12.0],
            target: [1.0, 0.5, -1.0],
            roll: 0.3,
            fov_deg: 40.0,
        };
        let cases = [
            (default_ring(), [0.0, 0.0, 0.0], (540.0, 960.0)),
            (default_ring(), [1.0, 0.6, -2.0], (645.328693347061, 747.8629820640904)),
            (
                default_ring(),
                [-2.5, 0.6, 2.5],
                (252.21331138293485, 1167.4560629117286),
            ),
            (default_ring(), [0.0, 3.0, 0.0], (540.0, 771.785234475053)),
            (rolled, [1.0, 0.6, -2.0], (592.8897854347097, 869.0056995768209)),
            (rolled, [-2.5, 0.6, 2.5], (-281.4264873903941, 1032.7212865031015)),
            (rolled, [0.0, 3.0, 0.0], (421.02029587329486, 621.8852359893226)),
        ];
        for (c, p, want) in cases {
            let got = project(&c, p, 1080.0, 1920.0).unwrap();
            assert!(
                (got.0 - want.0).abs() < 1e-6 && (got.1 - want.1).abs() < 1e-6,
                "{p:?}: {got:?}"
            );
        }
        // behind the camera
        assert_eq!(project(&default_ring(), [0.0, 30.0, 40.0], 1080.0, 1920.0), None);
        // sim +y (away from the camera) is up the picture
        let far = project(&default_ring(), to_render([0.0, 3.0, 0.0]), W, H).unwrap();
        assert!(far.1 < 960.0);
    }

    #[test]
    fn ball_box_is_about_the_ball() {
        let c = default_ring();
        let b = ball_box(&c, [0.0, 0.0, 0.0]).unwrap();
        assert!((0.5 * (b[0] + b[2]) - 540.0).abs() < 1e-9);
        let r = 0.5 * (b[2] - b[0]);
        // 0.6 at 27.5 with a 36 degree fov on 1920 rows: about 66 px
        assert!(r > 60.0 && r < 75.0, "{r}");
        assert!(inside(b, SAFE));
        assert!(!inside([0.0, 0.0, 70.0, 70.0], SAFE));
        assert_eq!(inset([0.0, 0.0, 100.0, 200.0], 0.1), [10.0, 20.0, 90.0, 180.0]);
    }
}
