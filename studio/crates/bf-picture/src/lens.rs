//! `reelkit/lens.py`: depth of field with an octagonal (8-blade) aperture.
//!
//! The focus is locked on the subject for the whole video:
//!
//! ```text
//! zf  = the subject's camera depth              cam.depth(&[subject])[0]
//! coc = coc_map(&frame.depth(), zf, K, sharp)   the blur radius of every pixel, in pixels
//! img = dof(&frame.array(), &coc)               the blurred picture
//! ```
//!
//! [`dof`] is a stack of masked, normalised blurs, one per radius in [`LEVELS`]: each level blurs only the pixels
//! out of focus enough to belong to it, so a sharp foreground never bleeds into a blurred background. A level's
//! blur is `scipy.signal.fftconvolve` with the octagon of that radius, which is [`crate::fft`].
//!
//! Python's `coc_map` and `dof` take the levels as an argument (`levels=LEVELS`) that no caller in the kit
//! passes. Here they always work with [`LEVELS`], so no caller can hand over a list the blur is not made for (a
//! negative radius, a NaN, radii out of order).
//!
//! How close to Python (`studio/reference/picture_kit/lens.npz`): [`oct_kernel`] and [`coc_map`] are equal to the
//! bit. [`dof`] carries the rounding of a `float32` FFT (about 1e-5 of a level on a 0..=255 picture, the same in
//! Python and here) and is held to 1e-9 of each value, or 1e-12 of the picture's largest value where that is
//! wider; it agrees to about 1e-15 of the picture's largest value.
//!
//! Not ported (the BALLFIGHT renderer does not call it): `add_bokeh`.

use crate::fft::{self, Convolver};
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
///
/// The table lives as long as the process, is keyed by the radius and the picture size, and is never emptied.
/// The ring blurs one picture size with the six radii of [`LEVELS`]: six entries of about 2 MB. Blurring
/// pictures of ever new sizes would grow it without limit.
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

/// `coc_map(depth, zf, K, sharp=0.05)`: the circle of confusion of every pixel, in pixels, from its depth, the
/// focus depth `zf` and the lens strength `K`; `sharp` is the band around the focus plane that stays fully crisp.
/// `depth` is the frame's `float32` buffer and `zf` a numpy `float64` (what `Cam.depth` gives), so the result is
/// `float64`. It never exceeds the last of [`LEVELS`].
pub fn coc_map(depth: &Img<f32>, zf: f64, k: f64, sharp: f64) -> Img<f64> {
    assert_eq!(depth.c, 1, "coc_map: the depth buffer has one channel");
    let top = LEVELS[LEVELS.len() - 1];
    let data = depth
        .data
        .iter()
        .map(|&d| {
            // np.maximum(depth, 1e-3) is still float32: the Python float becomes a float32.
            let depth = f64::from(np::maximum_f32(d, 1e-3));
            let off = np::maximum(0.0, (1.0 - zf / depth).abs() - sharp);
            np::clip(k * off, 0.0, top)
        })
        .collect();
    Img::from_vec(depth.w, depth.h, 1, data)
}

/// `dof(img, coc)`: the picture (`float32` RGB, as `Frame::array` gives it) blurred by the circle-of-confusion
/// map `coc`, through the radii of [`LEVELS`].
///
/// Returns `None` when no pixel is out of focus (`coc` nowhere above the first level): Python then returns the
/// picture as it was, still `float32`, and what follows (`apply_glows`) keeps working in `float32`. Otherwise the
/// result is `float64`.
///
/// Panics, before any work, for a picture the FFT here does not take: one pixel wide or high, or so large that
/// a side padded for the widest kernel exceeds [`fft::MAX_LEN`] samples.
pub fn dof(img: &Img<f32>, coc: &Img<f64>) -> Option<Img<f64>> {
    let levels = &LEVELS;
    assert_eq!(img.c, 3, "dof: the picture must be RGB");
    assert_eq!(
        (coc.w, coc.h, coc.c),
        (img.w, img.h, 1),
        "dof: coc must be one value per pixel"
    );
    let (w, h, n) = (img.w, img.h, img.w * img.h);
    // The widest kernel pads the most (the padded size never shrinks as the kernel grows).
    let widest = 2 * (LEVELS[LEVELS.len() - 1].ceil() as usize) + 1;
    let padded = (fft::good_size_real(w + widest - 1), fft::good_size_real(h + widest - 1));
    assert!(
        w > 1 && h > 1 && padded.0 <= fft::MAX_LEN && padded.1 <= fft::MAX_LEN,
        "dof: a picture of {w} x {h} cannot be blurred here: it must be more than one pixel each way, and its \
         size padded for the FFT ({} x {}) at most {} a side",
        padded.0,
        padded.1,
        fft::MAX_LEN
    );
    let widened = || -> Vec<f64> { img.data.iter().map(|&v| f64::from(v)).collect() };
    // `out` and `prev` start as the picture; they become float64 with the first level that runs.
    let mut out: Option<Vec<f64>> = None;
    let mut prev: Option<Vec<f64>> = None;
    // The buffers are made once and used by every level (fresh memory from the system is slow here); each is
    // written whole before it is read.
    let mut plane = vec![0.0f32; n];
    let mut mask = vec![0.0f32; n];
    let (mut den, mut num) = (vec![0.0f64; n], vec![0.0f64; n]);
    let mut spare: Option<Vec<f64>> = None;
    let mut ws = fft::Workspace::default();
    for i in 1..levels.len() {
        let (r, lo) = (levels[i], levels[i - 1]);
        if !coc.data.iter().any(|&c| c > lo) {
            break;
        }
        // The pixels blurred enough to take part in this level.
        for (m, &c) in mask.iter_mut().zip(&coc.data) {
            *m = if c >= r * 0.7 { 1.0 } else { 0.0 };
        }
        let mut blur = spare.take().unwrap_or_else(|| vec![0.0f64; n * 3]);
        if mask.iter().all(|&m| m == 0.0) {
            // No pixel takes part (2 % of the levels of the example episode): `den` is the convolution of zeros,
            // every product and sum of which is a zero, so `den > 1e-3` holds nowhere and the blur is the picture
            // itself, to the bit. The four convolutions are left out.
            for (b, &v) in blur.iter_mut().zip(&img.data) {
                *b = f64::from(v);
            }
        } else {
            let lens = lens_for(r, w, h);
            lens.convolve_into(&mask, &mut ws, &mut den);
            for ch in 0..3 {
                for (p, (px, &m)) in plane.iter_mut().zip(img.data.as_chunks::<3>().0.iter().zip(&mask)) {
                    *p = px[ch] * m;
                }
                lens.convolve_into(&plane, &mut ws, &mut num);
                for (j, (&nm, &d)) in num.iter().zip(&den).enumerate() {
                    // np.where(den > 1e-3, num / np.maximum(den, 1e-3), img)
                    blur[j * 3 + ch] = if d > 1e-3 {
                        nm / np::maximum(d, 1e-3)
                    } else {
                        f64::from(img.data[j * 3 + ch])
                    };
                }
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
        // the blur before is not read again: the next level writes its blur into it
        spare = Some(before);
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
        let coc = coc_map(&depth, 27.5, 26.0, 0.17);
        // In focus, and inside the sharp band.
        assert_eq!(&coc.data[..2], &[0.0, 0.0]);
        // |1 - 27.5 / 20| = 0.375: 26 * (0.375 - 0.17).
        assert!((coc.data[2] - 26.0 * 0.205).abs() < 1e-12);
        // The sky: 26 * (1 - 0.17), capped by the last level. A depth of nothing is held at a thousandth.
        assert_eq!(&coc.data[3..5], &[10.5, 10.5]);
        assert_eq!(coc.data[5], 10.5);
    }

    #[test]
    fn a_level_no_pixel_takes_part_in_is_the_picture() {
        // Every circle of confusion is 0.5: the first level runs (0.5 > 0) but its mask (>= 0.7) is empty. Its
        // convolutions would be of zeros, which are zeros to the bit, so the blur is the picture.
        let (w, h) = (24, 16);
        let zeros = vec![0.0f32; w * h];
        assert!(lens_for(LEVELS[1], w, h).convolve(&zeros).iter().all(|&d| d == 0.0));
        let img: Img<f32> = Img::from_vec(w, h, 3, (0..w * h * 3).map(|i| (i % 251) as f32 * 0.75).collect());
        let coc: Img<f64> = Img::filled(w, h, &[0.5]);
        let out = dof(&img, &coc).expect("the first level runs");
        // mixed = before * (1 - wgt) + blur * wgt, with before and blur both the picture and wgt = 0.5
        let want: Vec<f64> = img
            .data
            .iter()
            .map(|&v| f64::from(v) * (1.0 - 0.5) + f64::from(v) * 0.5)
            .collect();
        assert_eq!(out.data, want);
    }

    #[test]
    fn nothing_out_of_focus_is_no_blur() {
        let img: Img<f32> = Img::filled(12, 10, &[10.0, 20.0, 30.0]);
        let coc: Img<f64> = Img::new(12, 10, 1);
        assert_eq!(dof(&img, &coc), None);
    }

    #[test]
    #[should_panic(expected = "a picture of 1 x 40 cannot be blurred here")]
    fn a_picture_one_pixel_wide_is_refused_at_once() {
        let _ = dof(&Img::new(1, 40, 3), &Img::new(1, 40, 1));
    }

    #[test]
    #[should_panic(expected = "a picture of 2 x 9990 cannot be blurred here")]
    fn a_picture_too_long_for_the_fft_is_refused_at_once() {
        // 9,990 + 22 rows pad to 10,125, above what the FFT here follows ducc0 for. Nothing is out of focus:
        // the size is checked before anything else.
        assert_eq!(fft::good_size_real(9_990 + 22), 10_125);
        let _ = dof(&Img::new(2, 9_990, 3), &Img::new(2, 9_990, 1));
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
        let out = dof(&img, &coc).expect("the right half is out of focus");
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
