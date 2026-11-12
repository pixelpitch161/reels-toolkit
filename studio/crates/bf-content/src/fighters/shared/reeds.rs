//! `kit/fighters/_reeds.py`, whole: what the reed fighters share (screen masks, ribbons along ring-frame paths, the
//! dithered lane under a beam, the woodwind shatter parts), which nine looks import (CLARINET, OBOE, ALTO SAX,
//! TENOR SAX, CELLO, CONTRABASS, ELECTRIC, SOUSAPHONE, BASS DRUM); and the masks and blends other looks keep as
//! copies of the same code.
//!
//! numpy decides the arithmetic by the dtypes, so the functions take the picture's and the mask's sample type
//! ([`Px`]: `f32` or `f64`) and, where it matters, how the kit's number came ([`Num`]). What the kit passes, recorded
//! while each user drew a fight: the frame of `over_ring` and `draw_shard` is `float32`, a picture after the grade
//! (`shimmer`, `after_grade`) `float32` or `float64`; every mask is `float32`; every `alpha` a Python float.
//!
//! Masks are always the whole frame, 360 x 640 (`reelkit.config.W`, `H`).

use bf_core::{dmath, py};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{H, W, bayer_full};
use bf_picture::img::Img;
use bf_picture::look::Pic;
use bf_picture::ring::state::BallState;
use bf_picture::ring::{self, PictureCtx};
use bf_picture::{np, pil, scene};

/// `CANE`
pub const CANE: [f64; 3] = [232.0, 210.0, 152.0];
/// `CANE_D`
pub const CANE_D: [f64; 3] = [170.0, 140.0, 84.0];
/// `THREAD`
pub const THREAD: [f64; 3] = [60.0, 150.0, 86.0];
/// `INK`
pub const INK: [f64; 3] = [18.0, 10.0, 26.0];
/// `WHITE`
pub const WHITE: [f64; 3] = [255.0, 252.0, 240.0];
/// `WOOD`: a stick of the woodwind's body.
pub const WOOD: [f64; 3] = [34.0, 26.0, 30.0];
/// `NICKEL`: a key chip.
pub const NICKEL: [f64; 3] = [214.0, 216.0, 226.0];
/// `LAV_SPARK`: the CLARINET's motes and sparks.
pub const LAV_SPARK: [[f64; 3]; 3] = [[226.0, 214.0, 255.0], [190.0, 168.0, 255.0], [255.0, 250.0, 255.0]];
/// `SOUR`: the OBOE's sour motes.
pub const SOUR: [[f64; 3]; 3] = [[170.0, 190.0, 70.0], [210.0, 220.0, 120.0], [120.0, 140.0, 40.0]];
/// `KO_CHIPS`: the K.O. chips of the reeds.
pub const KO_CHIPS: [[f64; 3]; 3] = [[232.0, 210.0, 152.0], [214.0, 216.0, 226.0], [40.0, 32.0, 36.0]];

/// A sample of a picture or a mask as numpy computes with it: `float32` or `float64`.
pub trait Px: Copy {
    /// The sample widened to `float64` (exact).
    fn get(self) -> f64;
    /// A `float64` result stored back into the array (`float32`: rounded to nearest).
    fn set(x: f64) -> Self;
    /// The sample times a Python float: numpy keeps the array's dtype (NEP 50), so a `float32` mask is multiplied
    /// by the number rounded to `float32`, in `float32`.
    fn times(self, k: f64) -> f64;
}

impl Px for f32 {
    fn get(self) -> f64 {
        f64::from(self)
    }
    fn set(x: f64) -> f32 {
        x as f32
    }
    fn times(self, k: f64) -> f64 {
        f64::from(self * k as f32)
    }
}

impl Px for f64 {
    fn get(self) -> f64 {
        self
    }
    fn set(x: f64) -> f64 {
        x
    }
    fn times(self, k: f64) -> f64 {
        self * k
    }
}

/// A number the kit multiplies a mask by, as numpy sees it beside a `float32` array: a Python `float` or `int` keeps
/// the mask's dtype; a numpy `float64` scalar (a value that came out of a numpy call) makes the product `float64`.
/// For a `float64` mask the two are the same.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Num {
    /// A Python `float` or `int`.
    Py(f64),
    /// A numpy `float64` scalar.
    F64(f64),
}

impl Num {
    /// The number itself.
    pub fn value(self) -> f64 {
        match self {
            Num::Py(v) | Num::F64(v) => v,
        }
    }

    /// `mk * self` for one mask sample, as a `float64`.
    fn scale<M: Px>(self, m: M) -> f64 {
        match self {
            Num::Py(k) => m.times(k),
            Num::F64(k) => m.get() * k,
        }
    }
}

/// An `L` picture of the frame's size with the polygons (3 points or more) filled and the lines (2 points or more)
/// drawn at `width`, as `float32` 0..1: `np.asarray(m, np.float32) / 255.0`.
fn draw_mask(polys: &[Vec<(f64, f64)>], lines: &[Vec<(f64, f64)>], width: u32, any_polygon: bool) -> Img<f32> {
    let mut m: Img<u8> = Img::new(W, H, 1);
    for pts in polys {
        if any_polygon || pts.len() >= 3 {
            pil::polygon(&mut m, pts, &[255]);
        }
    }
    for pts in lines {
        if pts.len() >= 2 {
            pil::line(&mut m, pts, &[255], width);
        }
    }
    Img {
        w: m.w,
        h: m.h,
        c: 1,
        data: m.data.iter().map(|&b| f32::from(b) / 255.0f32).collect(),
    }
}

/// `_mask(polys, lines=(), width=1)`: the polygons of three points or more filled and the lines of two points or
/// more drawn `width` pixels wide, a `float32` mask of the frame (1 inside). Also the BASS CLARINET's, the
/// BASSOON's and the EUPHONIUM's `_mask`, ORGAN's `mask_of(polys)` (`mask(polys, &[], 1)`) and its
/// `line_mask(segs, width)` (`mask(&[], segs, width)`).
pub fn mask(polys: &[Vec<(f64, f64)>], lines: &[Vec<(f64, f64)>], width: u32) -> Img<f32> {
    draw_mask(polys, lines, width, false)
}

/// ACOUSTIC's and TRUMPET's `mask_of(polys, width=None)`: with a `width` (Python's `if width:`, so 0 is none)
/// every list is drawn as a line of that width (a line of fewer than two points draws nothing, as in Pillow);
/// without, the lists of three points or more are filled.
pub fn mask_of(polys: &[Vec<(f64, f64)>], width: Option<u32>) -> Img<f32> {
    match width {
        Some(w) if w != 0 => draw_mask(&[], polys, w, false),
        _ => draw_mask(polys, &[], 1, false),
    }
}

/// HORN's and TUBA's `mask_of(polys)`: every list filled, whatever its length.
///
/// # Panics
/// Where Pillow raises: a list of fewer than two points.
pub fn mask_all(polys: &[Vec<(f64, f64)>]) -> Img<f32> {
    draw_mask(polys, &[], 1, true)
}

/// `blend(img, mk, col, alpha)`: `img += (col - img) * (mk * alpha)[..., None]`, nothing at all when `alpha <= 0`
/// (a NaN alpha does blend, as Python's comparison lets it). The difference and the product are `float64`; the sum
/// is stored back in the picture's dtype.
///
/// # Panics
/// If the picture is not three channels or the mask not one channel of the picture's size (numpy cannot broadcast
/// them).
pub fn blend<T: Px, M: Px>(img: &mut Img<T>, mk: &Img<M>, col: [f64; 3], alpha: Num) {
    if alpha.value() <= 0.0 {
        return;
    }
    blend_into(img, mk, col, alpha);
}

/// TRUMPET's `blend(arr, m, col, a)` (and ACOUSTIC's, with the colour clipped to 0..255 first): [`blend`] without
/// its early return, so an `a` of 0 or below still runs the arithmetic.
pub fn blend_into<T: Px, M: Px>(img: &mut Img<T>, mk: &Img<M>, col: [f64; 3], alpha: Num) {
    check(img, mk);
    for (px, &m) in img.data.as_chunks_mut::<3>().0.iter_mut().zip(&mk.data) {
        let k = alpha.scale(m);
        for (v, &c) in px.iter_mut().zip(&col) {
            let a = v.get();
            *v = T::set(a + (c - a) * k);
        }
    }
}

/// `add(img, mk, col, k)`: `img += mk[..., None] * col * k`, in `float64` (the mask times the colour, then `k`),
/// stored back in the picture's dtype. Also the BASSOON's `add`.
///
/// # Panics
/// As [`blend`].
pub fn add<T: Px, M: Px>(img: &mut Img<T>, mk: &Img<M>, col: [f64; 3], k: f64) {
    check(img, mk);
    for (px, &m) in img.data.as_chunks_mut::<3>().0.iter_mut().zip(&mk.data) {
        let m = m.get();
        for (v, &c) in px.iter_mut().zip(&col) {
            *v = T::set(v.get() + m * c * k);
        }
    }
}

/// [`blend`] on a picture of either dtype (the hooks that get one after the grade).
pub fn blend_pic<M: Px>(img: &mut Pic, mk: &Img<M>, col: [f64; 3], alpha: Num) {
    match img {
        Pic::F32(a) => blend(a, mk, col, alpha),
        Pic::F64(a) => blend(a, mk, col, alpha),
    }
}

/// [`add`] on a picture of either dtype.
pub fn add_pic<M: Px>(img: &mut Pic, mk: &Img<M>, col: [f64; 3], k: f64) {
    match img {
        Pic::F32(a) => add(a, mk, col, k),
        Pic::F64(a) => add(a, mk, col, k),
    }
}

fn check<T, M>(img: &Img<T>, mk: &Img<M>) {
    assert!(
        img.c == 3 && mk.c == 1 && img.w == mk.w && img.h == mk.h,
        "operands could not be broadcast together: a {}x{}x{} picture and a {}x{}x{} mask",
        img.h,
        img.w,
        img.c,
        mk.h,
        mk.w,
        mk.c
    );
}

/// `ribbon(cam, P, hw)`: the screen polygon of a ribbon along the ring-frame points `p` (each `[x, z, y]`: ring x,
/// ring z, height, Python's `P` columns), `hw` world units either side of the path in the ring plane: one number for
/// every point, or one per point. The left edge from the first point to the last, then the right edge back.
///
/// The path's direction is numpy's `gradient` (one-sided at the ends, central inside), divided by its length plus
/// `1e-9`; the edges go through [`scene::to_world`] and the camera.
///
/// # Panics
/// Where numpy raises: fewer than two points, or an `hw` that is neither one number nor one per point.
pub fn ribbon(cam: &Cam, p: &[V3], hw: &[f64]) -> Vec<(f64, f64)> {
    let n = p.len();
    assert!(
        n >= 2,
        "Shape of array too small to calculate a numerical gradient, at least (edge_order + 1) elements are required."
    );
    assert!(
        hw.len() == 1 || hw.len() == n,
        "ribbon: {} widths for {n} points",
        hw.len()
    );
    let grad = |i: usize, c: usize| -> f64 {
        if i == 0 {
            (p[1][c] - p[0][c]) / 1.0
        } else if i == n - 1 {
            (p[n - 1][c] - p[n - 2][c]) / 1.0
        } else {
            (p[i + 1][c] - p[i - 1][c]) / 2.0
        }
    };
    let (mut left, mut right) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for (i, q) in p.iter().enumerate() {
        let (dx, dz) = (grad(i, 0), grad(i, 1));
        // d /= np.linalg.norm(d, axis=1, keepdims=True) + 1e-9
        let len = (dx * dx + dz * dz).sqrt() + 1e-9;
        let (dx, dz) = (dx / len, dz / len);
        // n = stack([-d[:, 1], d[:, 0]]); L = P[:, :2] + n * hw; Rr = P[:, :2] - n * hw
        let (nx, nz) = (-dz, dx);
        let w = if hw.len() == 1 { hw[0] } else { hw[i] };
        left.push(scene::to_world(q[0] + nx * w, q[1] + nz * w, q[2]));
        right.push(scene::to_world(q[0] - nx * w, q[1] - nz * w, q[2]));
    }
    let mut out = cam.wp(&left);
    let back = cam.wp(&right);
    out.extend(back.iter().rev());
    out
}

/// `ring_exit(o, d, A=None)`: how far from `o` along `d` the ropes are (the ring is `|x|, |z| <= A`, `S.A` when
/// `a` is `None`): beams stop there. 99 when `d` is (nearly) zero; never below 0.
pub fn ring_exit(o: [f64; 2], d: [f64; 2], a: Option<f64>) -> f64 {
    let a = a.unwrap_or(scene::A);
    let mut out = 99.0;
    for i in 0..2 {
        if d[i].abs() > 1e-9 {
            out = py::py_min(out, (a.copysign(d[i]) - o[i]) / d[i]);
        }
    }
    py::py_max(0.0, out)
}

/// `beam_lane(sxp, szp, o, aim, length, width, ys, xs)`: the light on each canvas pixel (`sxp`, `szp` its ring-frame
/// position, `ys`, `xs` its screen row and column) under a beam from `o` at angle `aim`, `length` long (stopped at
/// the ropes) and `width` wide: a lit lane, dithered to thirds with the Bayer table.
///
/// # Panics
/// If the four pixel lists are not of one length.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn beam_lane(
    sxp: &[f64],
    szp: &[f64],
    o: [f64; 2],
    aim: f64,
    length: f64,
    width: f64,
    ys: &[usize],
    xs: &[usize],
) -> Vec<f64> {
    assert!(
        szp.len() == sxp.len() && ys.len() == sxp.len() && xs.len() == sxp.len(),
        "beam_lane: pixel lists differ"
    );
    let d = [dmath::cos(aim), dmath::sin(aim)];
    let length = py::py_min(length, ring_exit(o, d, None));
    let bayer = bayer_full();
    let span = 0.16 + width;
    (0..sxp.len())
        .map(|i| {
            let (rx, rz) = (sxp[i] - o[0], szp[i] - o[1]);
            let along = rx * d[0] + rz * d[1];
            let across = (-rx * d[1] + rz * d[0]).abs();
            // clip(1 - across / (0.16 + width), 0, 1) * (along > 0) * (along < length)
            let lane = np::clip(1.0 - across / span, 0.0, 1.0)
                * f64::from(u8::from(along > 0.0))
                * f64::from(u8::from(along < length));
            (lane * 3.0 + bayer.px(xs[i], ys[i])[0] * 0.9).floor() / 3.0
        })
        .collect()
}

/// `draw_woodwind_shard(P, arr, cam, kind, p, rot, size)`: one woodwind shatter part on the `float32` frame:
/// `"wood"` (a stick of the body) or `"reed"` (a cane blade) as a turned quadrilateral; any other kind as a
/// triangle chip shaded by its turn, `"key"` a nickel one, the rest (`"bell"`) black wood. `p` is
/// `[x, height, z]` (ring frame); `rot` the part's turn in the ring plane, `size` its size for a chip.
///
/// Python passes `P` only for `P.screen` and `P.layer_blend`, which are [`ring::screen`] and [`ring::layer_blend`].
pub fn draw_woodwind_shard(arr: &mut Img<f32>, cam: &Cam, kind: &str, p: V3, rot: f64, size: f64) {
    let (sx, sz) = (p[0], p[2]);
    let d = [dmath::cos(rot), dmath::sin(rot)];
    let nn = [-d[1], d[0]];
    let at = |q: [f64; 2]| scene::to_world(q[0], q[1], p[1]);
    if kind == "wood" || kind == "reed" {
        let (ln, wd) = if kind == "wood" { (0.26, 0.035) } else { (0.12, 0.03) };
        // q4 = [s + d * ln_ * a1 + nn * wd_ * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
        let q4: Vec<V3> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .iter()
            .map(|&(a1, a2)| {
                at([
                    sx + d[0] * ln * a1 + nn[0] * wd * a2,
                    sz + d[1] * ln * a1 + nn[1] * wd * a2,
                ])
            })
            .collect();
        let sp = ring::screen(cam, &q4);
        ring::layer_blend(arr, &[sp], if kind == "wood" { WOOD } else { CANE }, 1.0, None);
        return;
    }
    let key = kind == "key";
    let col = if key { NICKEL } else { WOOD.map(|c| c * 1.3) };
    let sz_ = size * if key { 0.7 } else { 1.5 };
    // tri = [s + d * sz, s - d * sz * 0.7 + nn * sz * 0.8, s - d * sz * 0.4 - nn * sz * 0.7]
    let tri: Vec<V3> = [
        [sx + d[0] * sz_, sz + d[1] * sz_],
        [
            sx - d[0] * sz_ * 0.7 + nn[0] * sz_ * 0.8,
            sz - d[1] * sz_ * 0.7 + nn[1] * sz_ * 0.8,
        ],
        [
            sx - d[0] * sz_ * 0.4 - nn[0] * sz_ * 0.7,
            sz - d[1] * sz_ * 0.4 - nn[1] * sz_ * 0.7,
        ],
    ]
    .iter()
    .map(|&q| at(q))
    .collect();
    let sp = ring::screen(cam, &tri);
    let shade_k = 0.75 + 0.25 * dmath::cos(rot * 2.0);
    let col = col.map(|c| np::clip(c * shade_k, 0.0, 255.0));
    ring::layer_blend(arr, &[sp], col, 1.0, None);
}

/// `tail_theta(P, k, st, t, gl)`: on the glide home the aim eases from where it ended to its opening aim:
/// `st['th'] = th_end + P._wrap(P.TH0[k] - th_end) * gl`, `th_end` the last recorded `TH` of fighter `k`.
///
/// # Panics
/// If the recording has no `TH` (every recording the renderer takes has one).
pub fn tail_theta(p: &PictureCtx, k: usize, st: &mut BallState, gl: f64) {
    let th_end = p
        .run
        .f("TH", p.run.last(), k)
        .expect("tail_theta: the recording has no TH");
    st.th = tail_theta_of(th_end, p.th0[k], gl);
}

/// The arithmetic of [`tail_theta`]: `th_end + wrap(th0 - th_end) * gl`, `wrap` Python's
/// `(a + pi) % (2 pi) - pi`.
pub fn tail_theta_of(th_end: f64, th0: f64, gl: f64) -> f64 {
    th_end + py::wrap_pi(th0 - th_end) * gl
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mask_is_the_frame_with_ones_inside() {
        let m = mask(&[vec![(10.0, 10.0), (20.0, 10.0), (20.0, 20.0), (10.0, 20.0)]], &[], 1);
        assert_eq!((m.w, m.h, m.c), (W, H, 1));
        assert_eq!(m.data[15 * W + 15], 1.0);
        assert_eq!(m.data[5 * W + 5], 0.0);
        // a polygon of two points and a line of one draw nothing
        let none = mask(&[vec![(1.0, 1.0), (30.0, 30.0)]], &[vec![(4.0, 4.0)]], 3);
        assert!(none.data.iter().all(|&v| v == 0.0));
        // mask_of with a width draws every list as a line
        let l = mask_of(&[vec![(0.0, 5.0), (50.0, 5.0)]], Some(1));
        assert_eq!(l.data[5 * W + 20], 1.0);
        assert_eq!(mask_of(&[vec![(0.0, 5.0), (50.0, 5.0)]], Some(0)).data[5 * W + 20], 0.0);
    }

    #[test]
    fn blend_and_add_follow_numpys_dtypes() {
        let mut img: Img<f32> = Img::filled(1, 1, &[100.0, 0.1, 255.0]);
        let mk: Img<f32> = Img::filled(1, 1, &[0.3]);
        blend(&mut img, &mk, [200.0, 0.0, 0.0], Num::Py(0.7));
        let k = f64::from(0.3f32 * 0.7f32);
        assert_eq!(img.data[0], (100.0 + (200.0 - 100.0) * k) as f32);
        assert_eq!(img.data[1], (f64::from(0.1f32) + (0.0 - f64::from(0.1f32)) * k) as f32);
        // a numpy float64 alpha widens the mask first
        let mut wide: Img<f32> = Img::filled(1, 1, &[100.0, 0.1, 255.0]);
        blend(&mut wide, &mk, [200.0, 0.0, 0.0], Num::F64(0.7));
        assert_eq!(wide.data[0], (100.0 + 100.0 * (f64::from(0.3f32) * 0.7)) as f32);
        // no alpha, no blend; blend_into blends anyway
        let mut same = img.clone();
        blend(&mut same, &mk, [0.0; 3], Num::Py(0.0));
        assert_eq!(same, img);
        blend_into(&mut same, &mk, [0.0; 3], Num::Py(-0.5));
        assert_ne!(same, img);
        let mut a: Img<f64> = Img::filled(1, 1, &[1.0, 2.0, 3.0]);
        add(&mut a, &mk, [10.0, 20.0, 30.0], 0.5);
        assert_eq!(a.data[0], 1.0 + f64::from(0.3f32) * 10.0 * 0.5);
    }

    #[test]
    fn ring_exit_and_the_glide_home() {
        assert_eq!(ring_exit([0.0, 0.0], [1.0, 0.0], None), scene::A);
        assert_eq!(ring_exit([0.0, 0.0], [0.0, 0.0], None), 99.0);
        assert_eq!(ring_exit([4.0, 0.0], [1.0, 0.0], None), 0.0);
        assert_eq!(tail_theta_of(1.0, 1.0, 0.5), 1.0);
        assert_eq!(tail_theta_of(0.0, 1.0, 0.5), 0.5);
    }
}
