//! `reelkit/lens.py`: depth of field with an octagonal (8-blade) aperture.
//!
//! The focus is locked on the subject for the whole video:
//!
//! ```text
//! zf  = the subject's camera depth              cam.depth(&[subject])[0]
//! coc = coc_map(&frame.depth(), zf, K, sharp)   the blur radius of every pixel, in pixels
//! img = dof(&frame.array(), &coc, &LEVELS)      the blurred picture
//! ```
//!
//! [`dof`] is a stack of masked, normalised blurs, one per radius in [`LEVELS`]: each level blurs only the pixels
//! out of focus enough to belong to it, so a sharp foreground never bleeds into a blurred background. A level's
//! blur is `scipy.signal.fftconvolve` with the octagon of that radius, which is [`crate::fft`].
//!
//! How close to Python (`studio/reference/picture_kit/lens.npz`): [`oct_kernel`] and [`coc_map`] are equal to the
//! bit. [`dof`] carries the rounding of a `float32` FFT (about 1e-5 of a level on a 0..=255 picture, the same in
//! Python and here) and is held to 1e-9 of the picture's largest value; it agrees to about 1e-15.
//!
//! Not ported (the BALLFIGHT renderer does not call it): `add_bokeh`.

use crate::fft::Convolver;
use crate::img::Img;
use crate::np;
use bf_core::{dmath, py};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

/// `LEVELS`: the blur radii of the stack, in pixels; the first is "in focus".
pub const LEVELS: [f64; 7] = [0.0, 1.0, 2.0, 3.4, 5.2, 7.6, 10.5];

/// `oct_kernel(r)`: an octagon whose corners are `r` pixels from the centre (its flat sides face up, down, left
/// and right), widened by 0.35 pixels, as weights that add up to 1: `(2 ceil(r) + 1)` squared values, each 0 or
/// one over the number of pixels inside.
pub fn oct_kernel(r: f64) -> Img<f64> {
    let n = r.ceil() as i64;
    let side = (2 * n + 1) as usize;
    let (eighth, quarter) = (dmath::PI / 8.0, dmath::PI / 4.0);
    let reach = r * dmath::cos(eighth);
    let mut inside = Vec::with_capacity(side * side);
    for y in -n..=n {
        for x in -n..=n {
            let (xx, yy) = (x as f64, y as f64);
            let ang = dmath::atan2(yy, xx);
            // The distance to the octagon's edge in this direction.
            let rad = reach / dmath::cos(py::fmod_py(ang + eighth, quarter) - eighth);
            inside.push(dmath::hypot(xx, yy) <= rad + 0.35);
        }
    }
    let count = inside.iter().filter(|&&b| b).count() as f64;
    Img::from_vec(
        side,
        side,
        1,
        inside.iter().map(|&b| if b { 1.0 } else { 0.0 } / count).collect(),
    )
}

/// The octagon of radius `r` ready to blur planes of `w` x `h`: Python's `kernel(r)` cache, with the kernel's
/// spectrum kept too (scipy works it out again on every call; the result is the same).
fn lens_for(r: f64, w: usize, h: usize) -> Arc<Convolver> {
    type Made = Mutex<BTreeMap<(u64, usize, usize), Arc<Convolver>>>;
    static MADE: OnceLock<Made> = OnceLock::new();
    let mut held = MADE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    held.entry((r.to_bits(), w, h))
        .or_insert_with(|| Arc::new(Convolver::new(&oct_kernel(r), w, h)))
        .clone()
}

/// `np.maximum(depth, 1e-3)` for a `float32` depth: the Python float becomes a `float32`.
fn floor_depth(d: f32) -> f32 {
    let floor = 1e-3f32;
    if d.is_nan() || d > floor { d } else { floor }
}

/// `coc_map(depth, zf, K, sharp=0.05, levels=LEVELS)`: the circle of confusion of every pixel, in pixels, from
/// its depth, the focus depth `zf` and the lens strength `K`; `sharp` is the band around the focus plane that
/// stays fully crisp. `depth` is the frame's `float32` buffer and `zf` a numpy `float64` (what `Cam.depth` gives),
/// so the result is `float64`. It never exceeds the last level.
pub fn coc_map(depth: &Img<f32>, zf: f64, k: f64, sharp: f64, levels: &[f64]) -> Img<f64> {
    assert_eq!(depth.c, 1, "coc_map: the depth buffer has one channel");
    let top = levels[levels.len() - 1];
    let data = depth
        .data
        .iter()
        .map(|&d| {
            let off = (1.0 - zf / f64::from(floor_depth(d))).abs() - sharp;
            // np.maximum(0.0, off): 0.0 unless off is larger or not a number
            let off = if off.is_nan() || off > 0.0 { off } else { 0.0 };
            np::clip(k * off, 0.0, top)
        })
        .collect();
    Img::from_vec(depth.w, depth.h, 1, data)
}

/// `dof(img, coc, levels=LEVELS)`: the picture (`float32` RGB, as `Frame::array` gives it) blurred by the
/// circle-of-confusion map `coc`.
///
/// Returns `None` when no pixel is out of focus (`coc` nowhere above the first level): Python then returns the
/// picture as it was, still `float32`, and what follows (`apply_glows`) keeps working in `float32`. Otherwise the
/// result is `float64`.
pub fn dof(img: &Img<f32>, coc: &Img<f64>, levels: &[f64]) -> Option<Img<f64>> {
    assert_eq!(img.c, 3, "dof: the picture must be RGB");
    assert_eq!(
        (coc.w, coc.h, coc.c),
        (img.w, img.h, 1),
        "dof: coc must be one value per pixel"
    );
    let (w, h, n) = (img.w, img.h, img.w * img.h);
    let widened = || -> Vec<f64> { img.data.iter().map(|&v| f64::from(v)).collect() };
    // `out` and `prev` start as the picture; they become float64 with the first level that runs.
    let mut out: Option<Vec<f64>> = None;
    let mut prev: Option<Vec<f64>> = None;
    let mut plane = vec![0.0f32; n];
    for i in 1..levels.len() {
        let (r, lo) = (levels[i], levels[i - 1]);
        if !coc.data.iter().any(|&c| c > lo) {
            break;
        }
        // The pixels blurred enough to take part in this level.
        let mask: Vec<f32> = coc.data.iter().map(|&c| if c >= r * 0.7 { 1.0 } else { 0.0 }).collect();
        let lens = lens_for(r, w, h);
        let den = lens.convolve(&mask);
        let mut blur = vec![0.0f64; n * 3];
        for ch in 0..3 {
            for (p, (px, &m)) in plane.iter_mut().zip(img.data.as_chunks::<3>().0.iter().zip(&mask)) {
                *p = px[ch] * m;
            }
            let num = lens.convolve(&plane);
            for (j, (&nm, &d)) in num.iter().zip(&den).enumerate() {
                // np.where(den > 1e-3, num / np.maximum(den, 1e-3), img)
                blur[j * 3 + ch] = if d > 1e-3 {
                    nm / np::maximum(d, 1e-3)
                } else {
                    f64::from(img.data[j * 3 + ch])
                };
            }
        }
        let mut mixed = out.take().unwrap_or_else(widened);
        let before = prev.take().unwrap_or_else(widened);
        for (j, &c) in coc.data.iter().enumerate() {
            if c > lo {
                let wgt = np::clip((c - lo) / (r - lo), 0.0, 1.0);
                for ch in 0..3 {
                    let at = j * 3 + ch;
                    mixed[at] = before[at] * (1.0 - wgt) + blur[at] * wgt;
                }
            }
        }
        out = Some(mixed);
        prev = Some(blur);
    }
    out.map(|data| Img::from_vec(w, h, 3, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(k: &Img<f64>) -> Vec<String> {
        (0..k.h)
            .map(|y| k.row(y).iter().map(|&v| if v > 0.0 { '#' } else { '.' }).collect())
            .collect()
    }

    #[test]
    fn the_small_kernels_are_octagons() {
        let k1 = oct_kernel(1.0);
        assert_eq!(rows(&k1), [".#.", "###", ".#."]);
        assert_eq!(k1.px(1, 1), &[0.2]);
        let k2 = oct_kernel(2.0);
        assert_eq!(rows(&k2), [".###.", "#####", "#####", "#####", ".###."]);
        assert_eq!(k2.px(2, 2), &[1.0 / 21.0]);
        // A radius between whole pixels takes the next size up.
        assert_eq!(oct_kernel(3.4).w, 9);
        for r in LEVELS.iter().skip(1) {
            let total: f64 = oct_kernel(*r).data.iter().sum();
            assert!((total - 1.0).abs() < 1e-12, "radius {r}");
        }
    }

    #[test]
    fn the_circle_of_confusion_grows_away_from_the_focus() {
        let depth = Img::from_vec(6, 1, 1, vec![27.5f32, 26.0, 20.0, 1e6, 1e-9, 13.75]);
        let coc = coc_map(&depth, 27.5, 26.0, 0.17, &LEVELS);
        // In focus, and inside the sharp band.
        assert_eq!(&coc.data[..2], &[0.0, 0.0]);
        // |1 - 27.5 / 20| = 0.375: 26 * (0.375 - 0.17).
        assert!((coc.data[2] - 26.0 * 0.205).abs() < 1e-12);
        // The sky: 26 * (1 - 0.17), capped by the last level. A depth of nothing is held at a thousandth.
        assert_eq!(&coc.data[3..5], &[10.5, 10.5]);
        assert_eq!(coc.data[5], 10.5);
    }

    #[test]
    fn nothing_out_of_focus_is_no_blur() {
        let img: Img<f32> = Img::filled(12, 10, &[10.0, 20.0, 30.0]);
        let coc: Img<f64> = Img::new(12, 10, 1);
        assert_eq!(dof(&img, &coc, &LEVELS), None);
    }

    #[test]
    fn the_blur_keeps_sharp_pixels_and_mixes_the_rest() {
        // Left half sharp and dark, right half far out of focus and bright.
        let (w, h) = (24, 16);
        let mut img: Img<f32> = Img::new(w, h, 3);
        let mut coc: Img<f64> = Img::new(w, h, 1);
        for y in 0..h {
            for x in w / 2..w {
                img.px_mut(x, y).copy_from_slice(&[200.0, 100.0, 50.0]);
                coc.px_mut(x, y)[0] = 10.5;
            }
        }
        // One bright dot in the sharp half and one dark dot in a corner of the blurred half.
        img.px_mut(3, 8).copy_from_slice(&[255.0; 3]);
        img.px_mut(22, 14).copy_from_slice(&[0.0; 3]);
        let out = dof(&img, &coc, &LEVELS).expect("the right half is out of focus");
        // Sharp pixels are untouched, to the bit.
        for y in 0..h {
            for x in 0..w / 2 {
                let want: Vec<f64> = img.px(x, y).iter().map(|&v| f64::from(v)).collect();
                assert_eq!(out.px(x, y), want.as_slice(), "({x}, {y})");
            }
        }
        // The dark dot is smeared away, and the sharp half does not bleed into the blurred one: at the border
        // the blurred side keeps its own colour.
        assert!(out.px(22, 14)[0] > 150.0, "{:?}", out.px(22, 14));
        let edge = out.px(w / 2, 1);
        assert!(
            (edge[0] - 200.0).abs() < 1e-3 && (edge[2] - 50.0).abs() < 1e-3,
            "{edge:?}"
        );
    }
}
