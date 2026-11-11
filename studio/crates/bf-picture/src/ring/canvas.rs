//! The two per-pixel passes of `kit/picture.py`'s `render` and what stands on them: the arena floor, the ring's
//! skirt, the canvas (the spot with real shadows, the corner lights, the looks' decals), `canvas_shadows`, and
//! the posts and ropes that are drawn as lit faces and lines.

use std::sync::OnceLock;

use bf_core::dmath;
use bf_core::py;

use super::state::BallState;
use super::{PictureCtx, bayer};
use crate::camera::{Cam, V3};
use crate::config::{H, W};
use crate::img::Img;
use crate::lighting::{Lights, shade};
use crate::look::{CanvasCtx, FrameCtx};
use crate::np;
use crate::raster::Frame;
use crate::scene::{self, AP, ARENA_FLOOR, CORNERS, FLOOR_Y, POST_H, R_BALL as R, ROPE_COL, ROPE_Y, canvas_albedo};
use crate::shadows::{MaskGrid, proj_point};
use crate::v3;

/// `YY_, XX_ = np.mgrid[0:H, 0:W].astype(np.float64)`: every pixel's column and row, row by row.
fn pixels() -> &'static (Vec<f64>, Vec<f64>) {
    static PX: OnceLock<(Vec<f64>, Vec<f64>)> = OnceLock::new();
    PX.get_or_init(|| {
        let (mut xs, mut ys) = (Vec::with_capacity(W * H), Vec::with_capacity(W * H));
        for y in 0..H {
            for x in 0..W {
                xs.push(x as f64);
                ys.push(y as f64);
            }
        }
        (xs, ys)
    })
}

/// `CANVAS_GRID = MaskGrid((-5.4, 5.4, -5.4, 5.4, 0.03))`.
pub(super) fn canvas_grid() -> MaskGrid {
    MaskGrid::new([-5.4, 5.4, -5.4, 5.4, 0.03])
}

/// `STATIC_CASTERS`: a thin upright quad at each corner post.
pub(super) fn static_casters() -> Vec<[V3; 4]> {
    CORNERS
        .iter()
        .map(|c| {
            let w = scene::to_world(c[0], c[1], 0.0);
            [
                [w[0] + -0.09, w[1] + 0.0, w[2] + 0.0],
                [w[0] + 0.09, w[1] + 0.0, w[2] + 0.0],
                [w[0] + 0.09, w[1] + POST_H, w[2] + 0.0],
                [w[0] + -0.09, w[1] + POST_H, w[2] + 0.0],
            ]
        })
        .collect()
}

/// `np.linspace(0, 2 * math.pi, n + 1)[:-1]`: `n` angles round the circle.
pub(super) fn circle_angles(n: usize) -> Vec<f64> {
    let step = (2.0 * dmath::PI - 0.0) / n as f64;
    (0..n).map(|k| k as f64 * step + 0.0).collect()
}

impl PictureCtx {
    /// The arena floor, per pixel: the spill under the ring and the corner lights, dithered in steps of 6. Fills
    /// the `float32` picture and the depth buffer (`arr[:] = ...`, `depth[:] = dd`).
    pub(super) fn floor(&self, cam: &Cam, l: &Lights, arr: &mut Img<f32>, depth: &mut Img<f32>) {
        let (xs, ys) = pixels();
        let (gx, gz, dd) = cam.ground_hit(xs, ys, FLOOR_Y);
        for i in 0..W * H {
            let (x, z) = (gx[i], gz[i]);
            let rr = np::hypot(x, z);
            // I = [0.20, 0.18, 0.26] + (np.exp(-np.maximum(rr - 4.6, 0) / 3.0) * 0.55) * [1.0, 0.95, 1.05]
            let e = dmath::exp(-np::maximum(rr - 4.6, 0.0) / 3.0) * 0.55;
            let mut light = [0.0 + 0.20 + e * 1.0, 0.0 + 0.18 + e * 0.95, 0.0 + 0.26 + e * 1.05];
            for (lp, lc, r_) in &l.points {
                let h = lp[1] - FLOOR_Y;
                let dist = ((lp[0] - x) * (lp[0] - x) + h * h + (lp[2] - z) * (lp[2] - z)).sqrt();
                let q = dist / (r_ * 1.8);
                let w = h / dist / (1.0 + q * q);
                for c in 0..3 {
                    light[c] += w * lc[c] * 0.9;
                }
            }
            // alb = S.ARENA_FLOOR * (1 + 0.06 * ((np.floor(X * 1.2) + np.floor(Z * 1.2)) % 2))
            let check = 1.0 + 0.06 * py::fmod_py((x * 1.2).floor() + (z * 1.2).floor(), 2.0);
            let b = bayer(i % W, i / W);
            for c in 0..3 {
                // arr[:] = np.floor(alb * I / 6 + BAYER_FULL[..., None]) * 6
                arr.data[i * 3 + c] = ((ARENA_FLOOR[c] * check * light[c] / 6.0 + b).floor() * 6.0) as f32;
            }
            depth.data[i] = dd[i] as f32;
        }
    }

    /// The ring's skirt: the faces that look at the camera, shaded and drawn.
    pub(super) fn skirt(&self, fr: &mut Frame, cam: &Cam, l: &Lights) {
        for f in &self.scene.skirt_faces {
            let ctr = v3::mean(&f.q);
            if v3::dot(f.n, v3::sub(cam.pos, ctr)) > 0.0 {
                let cc = shade(l, &[f.n], &[ctr], &[f.col], None, 0.3, 0.0, 5.0)[0];
                fr.poly(cam, &f.q, cc, true);
            }
        }
    }

    /// `draw_post(name)`: the faces of a corner post that look at the camera.
    pub(super) fn draw_post(&self, fr: &mut Frame, cam: &Cam, l: &Lights, corner: usize) {
        for f in &self.scene.posts[corner] {
            let ctr = v3::mean(&f.q);
            if v3::dot(f.n, v3::sub(cam.pos, ctr)) > 0.0 {
                let cc = shade(l, &[f.n], &[ctr], &[f.col], None, 0.6, 0.0, 5.0)[0];
                fr.poly(cam, &f.q, cc, true);
            }
        }
    }

    /// `draw_rope_side(s)`: the three ropes of side `s`; the middle one in its fighter's colour.
    pub(super) fn draw_rope_side(&self, fr: &mut Frame, cam: &Cam, l: &Lights, t: f64, s: usize) {
        // np.array([[0, 1.0, 0.3]]) / np.linalg.norm([0, 1, 0.3])
        let up = [0.0, 1.0, 0.3];
        let nrm = v3::div(up, v3::norm(up));
        for (j, &y) in ROPE_Y.iter().enumerate() {
            let pts = self.rope_points(s, y, t, 15);
            let base = if j != 1 {
                ROPE_COL[j]
            } else {
                self.col[if s == 0 || s == 3 { 0 } else { 1 }].map(|c| c * 0.92)
            };
            let c = shade(l, &[nrm], &[v3::mean(&pts)], &[base], None, 0.5, 0.0, 5.0)[0];
            fr.line(cam, &pts, c, 2);
        }
    }

    /// `canvas_shadows(F, spot)`: the shadow mask on the canvas, cast from the spot by the posts, the ropes,
    /// the balls and the looks' casters, blurred by 0.7 px.
    pub(super) fn canvas_shadows(&self, f: &FrameCtx<'_>, spot: V3) -> Img<f32> {
        let mut polys: Vec<Vec<(f64, f64)>> = Vec::new();
        for q in &self.static_casters {
            polys.push(proj_point(q, spot, 0.93));
        }
        for s in 0..4 {
            for y in ROPE_Y {
                let p = self.rope_points(s, y, f.t, 5);
                for ab in p.windows(2) {
                    let (a, b) = (ab[0], ab[1]);
                    let q = [
                        [a[0] + 0.0, a[1] + -0.02, a[2] + 0.0],
                        [b[0] + 0.0, b[1] + -0.02, b[2] + 0.0],
                        [b[0] + 0.0, b[1] + 0.02, b[2] + 0.0],
                        [a[0] + 0.0, a[1] + 0.02, a[2] + 0.0],
                    ];
                    polys.push(proj_point(&q, spot, 0.93));
                }
            }
        }
        let angles = circle_angles(12);
        for (k, st) in f.fs.iter().enumerate() {
            if !st.vis || st.scale < 0.3 {
                continue;
            }
            let pw = scene::to_world(st.p[0], st.p[1], R);
            // a lifted ball's shadow falls further off
            let hy = R + st.lift;
            let circ: Vec<V3> = angles
                .iter()
                .map(|&u| {
                    [
                        pw[0] + R * st.scale * dmath::cos(u),
                        hy,
                        pw[2] + R * st.scale * dmath::sin(u),
                    ]
                })
                .collect();
            polys.push(proj_point(&circ, spot, 0.93));
            polys.extend(self.looks[k].shadows(self, st, spot));
        }
        for l in &self.looks {
            polys.extend(l.shadows_free(self, f, spot));
        }
        // (Pillow raises for a polygon of fewer than two points; such a caster is left out)
        polys.retain(|q| q.len() >= 2);
        self.canvas_grid.raster(&polys, 0.7)
    }

    /// The canvas, per pixel: the albedo with its fading marks, the spot's cone with the shadow mask, the corner
    /// lights and the flares, then the looks' decals, dithered in steps of 5. Writes the canvas's pixels into the
    /// `float32` picture and the depth buffer.
    pub(super) fn canvas(
        &self,
        f: &FrameCtx<'_>,
        fs: &[BallState; 2],
        axis: V3,
        arr: &mut Img<f32>,
        depth: &mut Img<f32>,
    ) {
        let (cam, l, t, spot) = (f.cam, f.l, f.t, f.spot);
        let (px, py_) = pixels();
        let (gx, gz, dc) = cam.ground_hit(px, py_, 0.0);
        // on = (np.abs(sx) <= S.AP) & (np.abs(sz) <= S.AP); ys, xs = np.nonzero(on)
        let (mut ys, mut xs, mut xp, mut zp, mut sxp, mut szp) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for i in 0..W * H {
            let (sx, sz) = scene::to_ring(gx[i], gz[i]);
            if sx.abs() <= AP && sz.abs() <= AP {
                ys.push(i / W);
                xs.push(i % W);
                xp.push(gx[i]);
                zp.push(gz[i]);
                sxp.push(sx);
                szp.push(sz);
            }
        }
        if xs.is_empty() {
            return;
        }
        let n = xs.len();
        let mut alb: Vec<V3> = (0..n).map(|j| canvas_albedo(sxp[j], szp[j])).collect();
        let dither: Vec<f64> = (0..n).map(|j| bayer(xs[j], ys[j])).collect();
        // fading marks
        self.fx.mk.apply(&mut alb, &sxp, &szp, self.fx_clock(t), Some(&dither));
        let sh = self.canvas_grid.sample(&self.canvas_shadows(f, spot), &xp, &zp);
        let (cos0, cos1) = scene::spot_cos();
        let amb = l.amb.map(|c| c * 0.75);
        let mut ic: Vec<V3> = Vec::with_capacity(n);
        for j in 0..n {
            // v = [Xp - spot[0], -spot[1], Zp - spot[2]]
            let v = [xp[j] - spot[0], -spot[1] * 1.0, zp[j] - spot[2]];
            let dist = v3::norm(v);
            let cosang = v3::dot(v, axis) / dist;
            let cone = np::clip((cosang - cos0) / (cos1 - cos0), 0.0, 1.0);
            let lam = spot[1] / dist;
            // (1 - 0.85 * sh): sh is float32, and so is this
            let open = f64::from(1.0f32 - 0.85f32 * sh[j]);
            let k = open * cone * lam * 0.86;
            let mut light = [amb[0] + k * 1.0, amb[1] + k * 0.98, amb[2] + k * 0.95];
            for (lp, lc, r_) in &l.points {
                let dl = ((lp[0] - xp[j]) * (lp[0] - xp[j]) + lp[1] * lp[1] + (lp[2] - zp[j]) * (lp[2] - zp[j])).sqrt();
                let q = dl / r_;
                let w = lp[1] / dl / (1.0 + q * q);
                for c in 0..3 {
                    light[c] += w * lc[c] * 0.8;
                }
            }
            ic.push(light);
        }
        // decals: each fighter's canvas light, the rings (in the run's order), then after-decals (darkening)
        let c = CanvasCtx {
            t,
            i: f.i,
            in_fight: f.in_fight,
            fs,
            sxp: &sxp,
            szp: &szp,
            ys: &ys,
            xs: &xs,
        };
        for look in &self.looks {
            look.canvas_first(self, &mut ic, &c);
        }
        for look in &self.looks {
            look.canvas(self, &mut ic, &c);
        }
        if f.in_fight {
            for w in self.run.waves(f.i) {
                self.looks[w.owner].canvas_wave(self, &mut ic, &c, w);
            }
        }
        for look in &self.looks {
            look.canvas_after(self, &mut ic, &c);
        }
        for j in 0..n {
            let at = arr.at(xs[j], ys[j]);
            for ch in 0..3 {
                // arr[ys, xs] = np.floor(col / 5 + BAYER_FULL[ys, xs, None]) * 5
                arr.data[at + ch] = ((alb[j][ch] * ic[j][ch] / 5.0 + dither[j]).floor() * 5.0) as f32;
            }
            depth.data[ys[j] * W + xs[j]] = dc[ys[j] * W + xs[j]] as f32;
        }
    }
}
