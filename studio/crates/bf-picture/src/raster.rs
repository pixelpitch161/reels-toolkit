//! `reelkit/raster.py`: the frame every scene is drawn into, and the glow sprites.
//!
//! [`Frame`] is a colour picture and a depth buffer that every draw call keeps in step (painter's algorithm: the
//! caller sorts far to near, then draws), so the lens blur can use real depth. The drawing itself is Pillow's,
//! through [`crate::pil`]: a polygon or a line into the `RGB` picture, and the same shape with the depth as its
//! ink into the mode `F` buffer.
//!
//! How close to Python (`studio/reference/picture_kit/raster.npz`, all at the full 360 x 640): the pictures and
//! the depth buffers are equal byte for byte and bit for bit, the glow kernels and `apply_glows` bit for bit.
//! The screen points `poly` returns come from [`Cam::wp`] and agree to rounding (about 1e-15).
//!
//! Not ported: `painter` (a sort by the first element, far to near: `sort_by` on the depth, descending, stable).

use crate::camera::{Cam, V3, clip_near};
use crate::config::{BAYER, H, W};
use crate::img::Img;
use crate::{np, pil};
use bf_core::{dmath, py};
use std::sync::OnceLock;

/// `SKY_DEPTH`: the depth of a pixel nothing was drawn on.
pub const SKY_DEPTH: f64 = 1e6;

/// One glow, as `Frame.glow` files it: `(x, y, radius, colour, intensity)`, composited after the lens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glow {
    pub x: f64,
    pub y: f64,
    pub r: f64,
    pub col: [f64; 3],
    pub inten: f64,
}

/// `Frame`: the colour picture (`pil`, RGB), the depth buffer (`dep`, one `float32` per pixel) and the glows
/// filed for later.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub pil: Img<u8>,
    pub dep: Img<f32>,
    pub glows: Vec<Glow>,
}

/// `tuple(int(c) for c in col)` as a Pillow ink: each channel loses its fraction (toward zero) and Pillow holds
/// it to 0..=255. (`np.clip(col, 0, 255)` before the `int`, as `poly` has it, gives the same bytes.)
fn ink(col: [f64; 3]) -> [u8; 3] {
    [col[0] as u8, col[1] as u8, col[2] as u8]
}

impl Frame {
    /// `Frame(bg)` with an RGB tuple: the whole frame in that colour, the depth buffer at [`SKY_DEPTH`].
    pub fn new(bg: [f64; 3]) -> Frame {
        Frame {
            pil: Img::filled(W, H, &ink(bg)),
            dep: Img::filled(W, H, &[SKY_DEPTH as f32]),
            glows: Vec::new(),
        }
    }

    /// `Frame(bg)` with an `(H, W, 3)` array (a sky gradient): the picture is
    /// `np.clip(bg, 0, 255).astype(np.uint8)`.
    pub fn from_array(bg: &Img<f64>) -> Frame {
        assert_eq!(
            (bg.w, bg.h, bg.c),
            (W, H, 3),
            "Frame: the background must be {W} x {H} RGB"
        );
        let mut fr = Frame::new([0.0; 3]);
        fr.set_array(bg, None);
        fr
    }

    /// `poly(cam, P, col, depth=True)`: a world polygon, clipped at the camera's near plane, filled with `col`;
    /// with `depth` its mean camera depth goes into the depth buffer. Returns the screen points drawn, or `None`
    /// when the polygon is behind the near plane.
    pub fn poly(&mut self, cam: &Cam, p: &[V3], col: [f64; 3], depth: bool) -> Option<Vec<(f64, f64)>> {
        let mut pc = cam.to_cam(p);
        if pc.iter().any(|q| q[2] < cam.near) {
            if pc.iter().all(|q| q[2] < cam.near) {
                return None;
            }
            pc = clip_near(&pc, cam.near)?;
        }
        let pts = cam.proj(&pc);
        pil::polygon(&mut self.pil, &pts, &ink(col));
        if depth {
            pil::polygon(&mut self.dep, &pts, &[mean_depth(&pc)]);
        }
        Some(pts)
    }

    /// `screen_poly(pts, col, z=None)`: a polygon given in screen pixels; with `z` that depth goes into the
    /// depth buffer.
    pub fn screen_poly(&mut self, pts: &[(f64, f64)], col: [f64; 3], z: Option<f64>) {
        pil::polygon(&mut self.pil, pts, &ink(col));
        if let Some(z) = z {
            pil::polygon(&mut self.dep, pts, &[z as f32]);
        }
    }

    /// `line(cam, pts, col, width=1)`: a world polyline, in colour and (at its mean depth) in the depth buffer.
    /// Nothing is drawn if any point is behind the near plane.
    pub fn line(&mut self, cam: &Cam, pts: &[V3], col: [f64; 3], width: u32) {
        let pc = cam.to_cam(pts);
        if pc.iter().any(|q| q[2] < cam.near) {
            return;
        }
        let s = cam.proj(&pc);
        pil::line(&mut self.pil, &s, &ink(col), width);
        pil::line(&mut self.dep, &s, &[mean_depth(&pc)], width);
    }

    /// `disc(center, r, col, n=12, rot=0.0, z=None)`: a regular `n`-gon in screen pixels.
    pub fn disc(&mut self, center: (f64, f64), r: f64, col: [f64; 3], n: usize, rot: f64, z: Option<f64>) {
        let pts: Vec<(f64, f64)> = (0..n)
            .map(|k| {
                let a = rot + 2.0 * dmath::PI * k as f64 / n as f64;
                (center.0 + r * dmath::cos(a), center.1 + r * dmath::sin(a))
            })
            .collect();
        self.screen_poly(&pts, col, z);
    }

    /// `glow(x, y, r, col, inten)`: files a glow; [`apply_glows`] draws `frame.glows` after the lens.
    pub fn glow(&mut self, x: f64, y: f64, r: f64, col: [f64; 3], inten: f64) {
        self.glows.push(Glow { x, y, r, col, inten });
    }

    /// `array()`: the picture as `float32`, `(H, W, 3)`.
    pub fn array(&self) -> Img<f32> {
        Img {
            w: self.pil.w,
            h: self.pil.h,
            c: self.pil.c,
            data: self.pil.data.iter().map(|&v| f32::from(v)).collect(),
        }
    }

    /// `depth()`: a copy of the depth buffer.
    pub fn depth(&self) -> Img<f32> {
        self.dep.clone()
    }

    /// `set_array(arr, depth=None)`: replaces the pixels (after a per-pixel lighting pass, say) so drawing can go
    /// on over them: the picture becomes `np.clip(arr, 0, 255).astype(np.uint8)`, and the depth buffer `depth`
    /// if one is given. `arr` may be `float32` or `float64`.
    ///
    /// Panics if the picture and the depth buffer (the one given, or the one kept) would differ in size. Python
    /// takes any two arrays here and fails later, where the two are used together.
    pub fn set_array<T: Copy + Into<f64>>(&mut self, arr: &Img<T>, depth: Option<&Img<f32>>) {
        assert_eq!(arr.c, 3, "set_array: the picture must be RGB");
        let dep = depth.unwrap_or(&self.dep);
        assert_eq!(dep.c, 1, "set_array: the depth buffer has one channel");
        assert_eq!(
            (dep.w, dep.h),
            (arr.w, arr.h),
            "set_array: the depth buffer {} is {} x {}, the picture {} x {}",
            if depth.is_some() { "given" } else { "kept" },
            dep.w,
            dep.h,
            arr.w,
            arr.h
        );
        self.pil = Img {
            w: arr.w,
            h: arr.h,
            c: 3,
            data: arr.data.iter().map(|&v| np::clip_u8(v.into())).collect(),
        };
        if let Some(d) = depth {
            self.dep = d.clone();
        }
    }
}

/// `float(Pc[:, 2].mean())` as the ink of the depth buffer (a `float32`).
fn mean_depth(pc: &[V3]) -> f32 {
    let z: Vec<f64> = pc.iter().map(|q| q[2]).collect();
    np::mean(&z) as f32
}

// ---------------------------------------------------------------- glow sprites
/// The largest glow radius; `add_glow` holds a radius to `1..=GLOW_MAX`.
pub const GLOW_MAX: usize = 60;

/// `_glow_kernel(r)`: a round falloff over `(2 r + 1)` squared pixels, banded into fifths and dithered.
fn make_glow_kernel(r: usize) -> Img<f64> {
    let n = 2 * r + 1;
    let mut data = Vec::with_capacity(n * n);
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (x as f64 - r as f64, y as f64 - r as f64);
            let d = dmath::hypot(dx, dy) / (r as f64 + 0.5);
            let v = dmath::pow(np::clip(1.0 - d, 0.0, 1.0), 1.7);
            data.push(np::floor_dither(v, 5.0, BAYER[y % 4][x % 4], 0.999));
        }
    }
    Img::from_vec(n, n, 1, data)
}

/// `GLOW[r]` for `r` from 1 to [`GLOW_MAX`]: the kernel of radius `r`, each value a multiple of 0.2 from 0 to 1.
pub fn glow_kernel(r: usize) -> &'static Img<f64> {
    static TABLE: OnceLock<Vec<Img<f64>>> = OnceLock::new();
    assert!(
        (1..=GLOW_MAX).contains(&r),
        "glow_kernel: radius {r} is outside 1..={GLOW_MAX}"
    );
    &TABLE.get_or_init(|| (1..=GLOW_MAX).map(make_glow_kernel).collect())[r - 1]
}

/// A sample type [`apply_glows`] can add to: a `float32` or a `float64` picture.
///
/// After the lens the ring's picture is `float64`, unless nothing was out of focus: then it is still the
/// frame's `float32` (see [`crate::lens::dof`]) and numpy adds each glow in `float64` and rounds the sum back
/// to `float32`.
pub trait GlowSample: Copy {
    /// `self + x` as numpy's `img[...] += x` leaves it in this dtype (`x` is `float64`).
    fn add_f64(self, x: f64) -> Self;
}

impl GlowSample for f32 {
    fn add_f64(self, x: f64) -> f32 {
        (f64::from(self) + x) as f32
    }
}

impl GlowSample for f64 {
    fn add_f64(self, x: f64) -> f64 {
        self + x
    }
}

/// `add_glow(img, x, y, r, col, inten)`: adds the glow kernel of the rounded radius (held to 1..=60), times
/// `inten` and the colour, centred on the rounded `(x, y)` and clipped to the picture. `round` is Python's
/// (halves go to the even number). A position or radius that is not a number draws nothing (Python raises).
pub fn add_glow<T: GlowSample>(img: &mut Img<T>, x: f64, y: f64, r: f64, col: [f64; 3], inten: f64) {
    assert_eq!(img.c, 3, "add_glow: the picture must be RGB");
    if x.is_nan() || y.is_nan() || r.is_nan() {
        return;
    }
    // r = int(max(1, min(60, round(r))))
    let r = py::round_half_even(r).clamp(1.0, GLOW_MAX as f64) as i64;
    let kernel = glow_kernel(r as usize);
    let (x, y) = (py::round_half_even(x), py::round_half_even(y));
    let (w, h) = (img.w as f64, img.h as f64);
    let rf = r as f64;
    // if x1 <= 0 or y1 <= 0 or x0 >= W or y0 >= H: return
    if x + rf + 1.0 <= 0.0 || y + rf + 1.0 <= 0.0 || x - rf >= w || y - rf >= h {
        return;
    }
    let (x, y) = (x as i64, y as i64);
    let (x0, y0, x1, y1) = (x - r, y - r, x + r + 1, y + r + 1);
    let (kx0, ky0) = ((-x0).max(0), (-y0).max(0));
    let (px0, py0) = (x0.max(0), y0.max(0));
    let (px1, py1) = (x1.min(img.w as i64), y1.min(img.h as i64));
    for py_ in py0..py1 {
        let ky = (ky0 + (py_ - py0)) as usize;
        for px in px0..px1 {
            let kx = (kx0 + (px - px0)) as usize;
            // k[ky, kx] * col, with k = GLOW[r] * inten
            let k = kernel.data[ky * kernel.w + kx] * inten;
            for (s, c) in img.px_mut(px as usize, py_ as usize).iter_mut().zip(col) {
                *s = s.add_f64(k * c);
            }
        }
    }
}

/// `apply_glows(img, glows)`: every glow added in turn. The picture's own size is the frame (Python reads the
/// kit's `W` and `H`).
pub fn apply_glows<T: GlowSample>(img: &mut Img<T>, glows: &[Glow]) {
    for g in glows {
        add_glow(img, g.x, g.y, g.r, g.col, g.inten);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::NEAR;

    fn ring() -> Cam {
        Cam::new([0.12, 0.25, 0.0], 27.5, 0.0, 58.0, 36.0, Some(288.0), 0.0, NEAR)
    }

    #[test]
    fn a_new_frame_is_flat_and_far() {
        let fr = Frame::new([10.9, 300.0, -5.0]);
        assert_eq!((fr.pil.w, fr.pil.h, fr.pil.c), (W, H, 3));
        assert_eq!(fr.pil.px(7, 600), &[10, 255, 0]);
        assert_eq!(fr.depth().px(359, 639), &[1e6f32]);
        assert!(fr.glows.is_empty());
        assert_eq!(fr.array().px(0, 0), &[10.0f32, 255.0, 0.0]);
    }

    #[test]
    fn a_polygon_goes_into_colour_and_depth() {
        let mut fr = Frame::new([0.0; 3]);
        let cam = ring();
        let quad = [
            [-1.0, 0.25, -1.0],
            [1.24, 0.25, -1.0],
            [1.24, 0.25, 1.0],
            [-1.0, 0.25, 1.0],
        ];
        let pts = fr.poly(&cam, &quad, [200.7, 100.0, 50.0], true).expect("in view");
        assert_eq!(pts.len(), 4);
        assert_eq!(fr.pil.px(180, 288), &[200, 100, 50]);
        let z = fr.dep.px(180, 288)[0];
        assert!((z - 27.5).abs() < 1e-3, "{z}");
        // Without depth the buffer keeps what it had.
        fr.poly(&cam, &quad, [1.0, 2.0, 3.0], false);
        assert_eq!(fr.pil.px(180, 288), &[1, 2, 3]);
        assert_eq!(fr.dep.px(180, 288)[0], z);
        assert_eq!(fr.dep.px(5, 5), &[1e6f32]);
    }

    #[test]
    fn what_is_behind_the_camera_is_not_drawn() {
        let mut fr = Frame::new([0.0; 3]);
        let cam = ring();
        let behind: Vec<V3> = [[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
            .iter()
            .map(|p| [p[0] + cam.pos[0], p[1] + cam.pos[1] + 5.0, p[2] + cam.pos[2] + 5.0])
            .collect();
        assert_eq!(fr.poly(&cam, &behind, [255.0; 3], true), None);
        fr.line(&cam, &behind, [255.0; 3], 2);
        assert_eq!(fr, Frame::new([0.0; 3]));
        // A polygon across the near plane is cut, not dropped.
        let beside = [cam.target[0] + 1.0, cam.target[1], cam.target[2]];
        let across = [cam.target, beside, behind[0]];
        assert!(fr.poly(&cam, &across, [255.0; 3], true).is_some_and(|p| p.len() == 4));
    }

    #[test]
    fn set_array_clips_and_truncates() {
        let mut fr = Frame::new([0.0; 3]);
        let mut arr: Img<f64> = Img::new(W, H, 3);
        arr.px_mut(3, 4).copy_from_slice(&[-7.0, 127.99, 300.0]);
        let mut depth: Img<f32> = Img::filled(W, H, &[2.5]);
        depth.px_mut(3, 4)[0] = 9.0;
        fr.set_array(&arr, Some(&depth));
        assert_eq!(fr.pil.px(3, 4), &[0, 127, 255]);
        assert_eq!((fr.dep.px(3, 4)[0], fr.dep.px(0, 0)[0]), (9.0, 2.5));
        fr.set_array(&fr.array(), None);
        assert_eq!(fr.pil.px(3, 4), &[0, 127, 255]);
        assert_eq!(fr.dep.px(3, 4)[0], 9.0);
    }

    #[test]
    #[should_panic(expected = "the depth buffer given is 360 x 639, the picture 360 x 640")]
    fn set_array_refuses_a_depth_buffer_of_another_size() {
        let mut fr = Frame::new([0.0; 3]);
        let arr: Img<f64> = Img::new(W, H, 3);
        fr.set_array(&arr, Some(&Img::new(W, H - 1, 1)));
    }

    #[test]
    #[should_panic(expected = "the depth buffer kept is 360 x 640, the picture 180 x 320")]
    fn set_array_refuses_a_picture_the_kept_depth_does_not_fit() {
        let mut fr = Frame::new([0.0; 3]);
        let arr: Img<f32> = Img::new(W / 2, H / 2, 3);
        fr.set_array(&arr, None);
    }

    #[test]
    fn glow_kernels_are_banded_discs() {
        let k = glow_kernel(1);
        assert_eq!((k.w, k.h), (3, 3));
        // The centre is full; a fifth is the step.
        assert_eq!(k.px(1, 1), &[1.0]);
        let big = glow_kernel(60);
        assert_eq!((big.w, big.h), (121, 121));
        assert_eq!(big.px(60, 60), &[1.0]);
        assert_eq!(big.px(0, 0), &[0.0]);
        assert!(
            big.data
                .iter()
                .all(|&v| (v * 5.0).fract() == 0.0 && (0.0..=1.0).contains(&v))
        );
    }

    #[test]
    fn glows_add_light_clipped_to_the_picture() {
        let mut img: Img<f64> = Img::new(20, 10, 3);
        // Radius 0.2 is held to 1; the half positions round to the even pixel.
        add_glow(&mut img, 2.5, 3.5, 0.2, [10.0, 20.0, 30.0], 0.5);
        assert_eq!(img.px(2, 4), &[5.0, 10.0, 15.0]);
        assert_eq!(img.px(2, 7), &[0.0; 3]);
        // Off the picture: nothing. On its corner: the part inside.
        let before = img.clone();
        add_glow(&mut img, -30.0, 5.0, 3.0, [255.0; 3], 1.0);
        add_glow(&mut img, 5.0, f64::NAN, 3.0, [255.0; 3], 1.0);
        assert_eq!(img, before);
        add_glow(&mut img, 19.0, 9.0, 80.0, [1.0; 3], 1.0);
        assert_eq!(img.px(19, 9), &[1.0; 3]);
        // float32 pictures take the sum rounded to float32.
        let mut small: Img<f32> = Img::filled(4, 4, &[0.1f32; 3]);
        let mut frame = Frame::new([0.0; 3]);
        frame.glow(1.0, 1.0, 1.0, [0.3, 0.3, 0.3], 1.0);
        apply_glows(&mut small, &frame.glows);
        assert_eq!(small.px(1, 1)[0], (f64::from(0.1f32) + 0.3) as f32);
    }
}
