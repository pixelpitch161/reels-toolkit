//! After the grade: `post_fx` of `kit/picture.py` (chromatic fringing on big hits, the ability moments' dim and
//! speed lines, the impact bursts), and the picture of either dtype it works on.

use bf_core::dmath;
use bf_core::py::py_max;

use super::PictureCtx;
use super::state::BallState;
use crate::camera::Cam;
use crate::config::ss;
use crate::fx::{self, Px};
use crate::img::Img;
use crate::look::{DrawnBall, Pic};
use crate::scene::{self, R_BALL as R};

impl Pic {
    /// The picture's width and height.
    pub fn size(&self) -> (usize, usize) {
        match self {
            Pic::F32(a) => (a.w, a.h),
            Pic::F64(a) => (a.w, a.h),
        }
    }

    /// The picture as `float64` (a `float32` sample widens exactly).
    pub fn to_f64(&self) -> Img<f64> {
        match self {
            Pic::F32(a) => Img::from_vec(a.w, a.h, a.c, a.data.iter().map(|&v| f64::from(v)).collect()),
            Pic::F64(a) => a.clone(),
        }
    }

    /// Makes the picture `float64` in place, as numpy does when a `float64` term joins it.
    pub fn widen(&mut self) -> &mut Img<f64> {
        if let Pic::F32(_) = self {
            *self = Pic::F64(self.to_f64());
        }
        match self {
            Pic::F64(a) => a,
            Pic::F32(_) => unreachable!("the picture was just widened"),
        }
    }

    /// `np.clip(img, 0, 255).astype(np.uint8)`.
    pub fn to_bytes(&self) -> Img<u8> {
        match self {
            Pic::F32(a) => Img::from_vec(a.w, a.h, a.c, a.data.iter().map(|v| v.to_byte()).collect()),
            Pic::F64(a) => Img::from_vec(a.w, a.h, a.c, a.data.iter().map(|v| v.to_byte()).collect()),
        }
    }

    /// `fx.fringe(img, cx, cy, r, px, k)` on either dtype.
    pub fn fringe(&mut self, cx: f64, cy: f64, r: f64, px: f64, k: f64) {
        match self {
            Pic::F32(a) => fx::fringe(a, cx, cy, r, px, k),
            Pic::F64(a) => fx::fringe(a, cx, cy, r, px, k),
        }
    }

    /// `fx.shimmer_world(img, cam, c, r, width, amp, t, k, seed)` on either dtype.
    #[allow(clippy::too_many_arguments)] // the Python signature
    pub fn shimmer_world(&mut self, cam: &Cam, c: [f64; 3], r: f64, width: f64, amp: f64, t: f64, k: f64, seed: f64) {
        match self {
            Pic::F32(a) => fx::shimmer_world(a, cam, c, r, width, amp, t, k, seed),
            Pic::F64(a) => fx::shimmer_world(a, cam, c, r, width, amp, t, k, seed),
        }
    }

    /// `MO.apply_dim(img, t, keep)` as numpy computes it for either dtype: a dim of 0.005 or less leaves the
    /// picture as it is; with discs the factor is `float64` and so is the picture from then on; without discs
    /// the factor is a `float32` value, and a `float32` picture stays `float32`.
    pub fn apply_dim(&mut self, mo: &fx::Moments, t: f64, keep: &[(f64, f64, f64)]) {
        let k = mo.dim_k(t);
        if k <= 0.005 {
            return;
        }
        if let (Pic::F32(a), true) = (&mut *self, keep.is_empty()) {
            // img * (1 - k * (1 - lit)) with lit a float32 array of zeros: float32 times float32
            let factor = 1.0f32 - k as f32 * (1.0f32 - 0.0f32);
            for v in &mut a.data {
                *v *= factor;
            }
            return;
        }
        mo.apply_dim(self.widen(), t, keep);
    }

    /// `MO.speed_lines(img, t, key, x, y, r, direction)` on either dtype: a drawn picture is `float32`.
    #[allow(clippy::too_many_arguments)] // the Python signature
    pub fn speed_lines(&mut self, mo: &fx::Moments, t: f64, key: i64, x: f64, y: f64, r: f64, direction: (f64, f64)) {
        let new = match self {
            Pic::F32(a) => mo.speed_lines(a, t, key, x, y, r, direction, 16, (16.0, 46.0), 0),
            Pic::F64(a) => mo.speed_lines(a, t, key, x, y, r, direction, 16, (16.0, 46.0), 0),
        };
        if let Some(new) = new {
            *self = Pic::F32(new);
        }
    }

    /// `BU.draw(img, cam, t)` on either dtype.
    pub fn bursts(&mut self, bu: &fx::Bursts, cam: &Cam, t: f64) {
        match self {
            Pic::F32(a) => bu.draw(a, cam, t),
            Pic::F64(a) => bu.draw(a, cam, t),
        }
    }
}

impl PictureCtx {
    /// `post_fx(img, cam, t, fs, drawn)`: after the grade: chromatic fringing on big hits, the ability moments
    /// (dim and speed lines), the impact bursts.
    pub(super) fn post_fx(&self, img: &mut Pic, cam: &Cam, t: f64, _fs: &[BallState; 2], drawn: &[DrawnBall]) {
        for i in self.events_between(t - 0.12, t, &["hit"]) {
            let e = &self.rec.events[i];
            let att = super::owner(e).unwrap_or(0);
            if self.looks[att].fringe(self, e) {
                let s = cam.wp(&[scene::to_world(e.f(5), e.f(6), R)])[0];
                let (fr_r, fr_px) = self.looks[att].fringe_size(self, e);
                img.fringe(s.0, s.1, fr_r, fr_px, 1.0 - (t - e.t) / 0.12);
            }
        }
        let pk = self.p_ko[self.lose];
        if self.t_ko <= t && t < self.t_hs + 0.3 {
            let s = cam.wp(&[scene::to_world(pk[0], pk[1], R)])[0];
            img.fringe(s.0, s.1, 34.0, 3.0, 1.0 - ss(self.t_hs, self.t_hs + 0.3, t));
        }
        let keep: Vec<(f64, f64, f64)> = drawn.iter().map(|d| (d.s.0, d.s.1, d.rp * 1.7)).collect();
        img.apply_dim(&self.fx.mo, t, &keep);
        for d in drawn {
            let f1 = self.fighter(d.k, py_max(0.0, t - 0.06));
            let lift = if self.looks[d.k].speed_lift() { f1.lift } else { 0.0 };
            let s1 = cam.wp(&[scene::to_world(f1.p[0], f1.p[1], R + lift)])[0];
            let mut v = (d.s.0 - s1.0, d.s.1 - s1.1);
            // a teleport is not a dash: no speed lines
            if self.looks[d.k].teleports() && dmath::hypot(v.0, v.1) > 25.0 {
                v = (0.0, 0.0);
            }
            img.speed_lines(&self.fx.mo, t, d.k as i64, d.s.0, d.s.1, d.rp, v);
        }
        img.bursts(&self.fx.bu, cam, t);
    }
}
