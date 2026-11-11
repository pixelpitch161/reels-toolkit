//! `reelkit/post.py`: the effects on the float picture after the lens, in this order: glows
//! ([`crate::raster::apply_glows`]), [`bloom`], [`grade`], then the [`vignette`] and [`top_scrim`] tables
//! multiplied in.
//!
//! How close to Python (`studio/reference/picture_kit/post.npz`): `bloom`, `vignette`, `top_scrim` and `grade`
//! with `gamma` 1 are equal bit for bit. `grade` with another `gamma` takes a power (`bf_core::dmath` against
//! numpy's) and is held to 1e-9. `vignette` goes through a `float32` hypot whose last bit is not pinned by any
//! source the Studio can use; its 230,400 values are banded into fifths and all five reference tables are equal
//! to the bit.
//!
//! Not ported (the BALLFIGHT renderer does not call them): `god_rays`, `posterize`.

use crate::config::{H, W, bayer_full, xx, yy};
use crate::img::Img;
use crate::np;
use crate::pil::blur::gaussian_blur;
use crate::pil::resize::{Resample, resize};
use bf_core::dmath;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

/// `bloom(img, thr=200.0, k=0.4, exclude=None)`: a glow around bright pixels. What is brighter than `thr` (by its
/// largest channel) is shrunk to a quarter, blurred, blown up again and added back `k` times. `exclude` (one
/// channel, non-zero is set) marks pixels that must not bloom. The picture is `float64` RGB, as `kit.picture`
/// hands it over; its own size is the frame (Python reads the kit's `W` and `H`).
pub fn bloom(img: &Img<f64>, thr: f64, k: f64, exclude: Option<&Img<u8>>) -> Img<f64> {
    assert_eq!(img.c, 3, "bloom: the picture must be RGB");
    if let Some(e) = exclude {
        assert_eq!(
            (e.w, e.h, e.c),
            (img.w, img.h, 1),
            "bloom: exclude must be a mask of the picture's size"
        );
    }
    let mut src: Img<u8> = Img::new(img.w, img.h, 3);
    let pixels = img.data.as_chunks::<3>().0.iter().zip(src.data.as_chunks_mut::<3>().0);
    for (i, (px, out)) in pixels.enumerate() {
        let most = np::maximum(np::maximum(px[0], px[1]), px[2]);
        let mut b = np::maximum(most - thr, 0.0) / (255.0 - thr);
        if let Some(e) = exclude {
            b *= if e.data[i] != 0 { 0.0 } else { 1.0 };
        }
        for (o, &v) in out.iter_mut().zip(px) {
            *o = np::clip_u8(v * b);
        }
    }
    let small = resize(&src, img.w / 4, img.h / 4, Resample::Bilinear);
    let small = gaussian_blur(&small, 3.0);
    let up = resize(&small, img.w, img.h, Resample::Bilinear);
    // up is float32 and k a Python float: numpy multiplies in float32, and only the sum is float64.
    let k = k as f32;
    Img {
        w: img.w,
        h: img.h,
        c: 3,
        data: img
            .data
            .iter()
            .zip(&up.data)
            .map(|(&v, &u)| v + f64::from(f32::from(u) * k))
            .collect(),
    }
}

/// `x ** gamma` as numpy computes it for a `float64` array and a Python float: a square and a square root where
/// the exponent is 2 or 0.5, else the power function.
fn np_power(x: f64, gamma: f64) -> f64 {
    if gamma == 2.0 {
        x * x
    } else if gamma == 0.5 {
        x.sqrt()
    } else {
        dmath::pow(x, gamma)
    }
}

/// `grade(img, contrast=0.18, sat=1.1, black=0.0, gamma=1.0)`: a gentle S-curve and saturation on a `float64`
/// picture of 0..=255; `black` lifts the black point and `gamma` bends the curve (both off at their defaults).
pub fn grade(img: &Img<f64>, contrast: f64, sat: f64, black: f64, gamma: f64) -> Img<f64> {
    assert_eq!(img.c, 3, "grade: the picture must be RGB");
    let mut out = Vec::with_capacity(img.data.len());
    for px in img.data.as_chunks::<3>().0 {
        let mut x = [0.0; 3];
        for (o, &v) in x.iter_mut().zip(px) {
            let mut c = np::clip(v / 255.0, 0.0, 1.0);
            if black != 0.0 {
                c = np::clip((c - black) / (1.0 - black), 0.0, 1.0);
            }
            if gamma != 1.0 {
                c = np_power(c, gamma);
            }
            *o = c + contrast * (c * c * (3.0 - 2.0 * c) - c);
        }
        // x.mean(-1): the three channels added in order
        let g = (0.0 + x[0] + x[1] + x[2]) / 3.0;
        for c in x {
            out.push(np::clip(g + (c - g) * sat, 0.0, 1.0) * 255.0);
        }
    }
    Img {
        w: img.w,
        h: img.h,
        c: 3,
        data: out,
    }
}

/// A table per argument tuple, made once: Python's `_VIG` and `_SCRIM` dictionaries. The key is the arguments'
/// bits; a `BTreeMap`, so nothing depends on a hash.
type Tables<const N: usize> = Mutex<BTreeMap<[u64; N], Arc<Img<f64>>>>;

fn cached<const N: usize>(tables: &Tables<N>, key: [f64; N], make: impl FnOnce() -> Img<f64>) -> Arc<Img<f64>> {
    let key = key.map(f64::to_bits);
    let mut held = tables.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    held.entry(key).or_insert_with(|| Arc::new(make())).clone()
}

/// `np.hypot(a, b)` for `float32`: computed in double and rounded once.
fn hypot_f32(a: f32, b: f32) -> f32 {
    dmath::hypot(f64::from(a), f64::from(b)) as f32
}

/// `vignette(strength=0.9, start=0.55, cap=0.5, cy=0.47)`: the table that darkens the frame's corners, one
/// `float64` multiplier per pixel (`(H, W, 1)` in Python), banded into fifths and dithered. `start` is where the
/// darkening begins (in units of 0.62 of the frame), `cap` the most it takes away, `cy` the height of its centre.
/// Made once per argument tuple.
pub fn vignette(strength: f64, start: f64, cap: f64, cy: f64) -> Arc<Img<f64>> {
    static TABLES: OnceLock<Tables<4>> = OnceLock::new();
    cached(TABLES.get_or_init(Default::default), [strength, start, cap, cy], || {
        let (w, h) = (W as f64, H as f64);
        // XX and YY are float32, so every step up to the dither is float32 arithmetic with the Python floats
        // rounded to float32.
        let (half_w, across) = ((w / 2.0) as f32, (w * 0.62) as f32);
        let (mid_y, down) = ((h * cy) as f32, (h * 0.62) as f32);
        let (start32, strength32) = (start as f32, strength as f32);
        let (xs, ys, bayer) = (xx(), yy(), bayer_full());
        let data = (0..W * H)
            .map(|i| {
                let r = hypot_f32((xs.data[i] - half_w) / across, (ys.data[i] - mid_y) / down);
                let v = np::clip_f32(r - start32, 0.0, 1.0) * strength32;
                // np.floor(v * 5 + BAYER_FULL * 0.999) / 5: v * 5 is still float32
                let v = (f64::from(v * 5.0) + bayer.data[i] * 0.999).floor() / 5.0;
                1.0 - np::clip(v, 0.0, cap)
            })
            .collect();
        Img::from_vec(W, H, 1, data)
    })
}

/// `top_scrim(amount=0.58, top=110, span=120)`: the table that darkens the top of the frame so captions read:
/// the full `amount` above row `top`, fading out over `span` rows, banded into sixths and dithered. One
/// `float64` multiplier per pixel; made once per argument tuple.
pub fn top_scrim(amount: f64, top: f64, span: f64) -> Arc<Img<f64>> {
    static TABLES: OnceLock<Tables<3>> = OnceLock::new();
    cached(TABLES.get_or_init(Default::default), [amount, top, span], || {
        let (top32, span32, amount32) = (top as f32, span as f32, amount as f32);
        let (ys, bayer) = (yy(), bayer_full());
        let data = (0..W * H)
            .map(|i| {
                let v = np::clip_f32(1.0 - (ys.data[i] - top32) / span32, 0.0, 1.0) * amount32;
                let v = (f64::from(v * 6.0) + bayer.data[i] * 0.999).floor() / 6.0;
                1.0 - v
            })
            .collect();
        Img::from_vec(W, H, 1, data)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bloom_leaves_a_dim_picture_alone() {
        let img: Img<f64> = Img::filled(16, 24, &[100.5, 50.25, 199.0]);
        assert_eq!(bloom(&img, 200.0, 0.4, None), img);
    }

    #[test]
    fn bloom_spreads_light_from_bright_pixels() {
        let mut img: Img<f64> = Img::filled(64, 96, &[10.0, 10.0, 10.0]);
        let mut mask: Img<u8> = Img::new(64, 96, 1);
        for y in 68..76 {
            for x in 44..52 {
                img.px_mut(x, y).copy_from_slice(&[255.0, 255.0, 255.0]);
                mask.px_mut(x, y)[0] = 1;
            }
        }
        let out = bloom(&img, 200.0, 0.4, None);
        // The lamp gets brighter still, its surround is lit, the far corner is not.
        assert!(out.px(48, 72)[0] > 255.0);
        assert!(out.px(41, 72)[0] > 10.0);
        assert_eq!(out.px(0, 0), &[10.0, 10.0, 10.0]);
        // Excluding the lamp leaves nothing to bloom.
        assert_eq!(bloom(&img, 200.0, 0.4, Some(&mask)), img);
    }

    #[test]
    fn grade_keeps_black_grey_and_white() {
        let img = Img::from_vec(3, 1, 3, vec![0.0, 0.0, 0.0, 127.5, 127.5, 127.5, 255.0, 255.0, 255.0]);
        let out = grade(&img, 0.18, 1.1, 0.0, 1.0);
        assert_eq!(out.data, img.data);
        // Out of range is clipped first; saturation 0 is grey.
        let wild = Img::from_vec(1, 1, 3, vec![-50.0, 300.0, 127.5]);
        let grey = grade(&wild, 0.0, 0.0, 0.0, 1.0);
        assert_eq!(grey.data, vec![127.5; 3]);
        // The S-curve pushes a dark grey down and a light grey up.
        let two = Img::from_vec(2, 1, 3, vec![64.0, 64.0, 64.0, 191.0, 191.0, 191.0]);
        let out = grade(&two, 0.18, 1.0, 0.0, 1.0);
        assert!(out.data[0] < 64.0 && out.data[3] > 191.0);
        assert_eq!(np_power(9.0, 0.5), 3.0);
        assert_eq!(np_power(3.0, 2.0), 9.0);
        assert_eq!(np_power(2.0, 3.0), 8.0);
    }

    #[test]
    fn the_vignette_darkens_corners_only() {
        let v = vignette(0.9, 0.55, 0.5, 0.47);
        assert_eq!((v.w, v.h, v.c), (W, H, 1));
        assert_eq!(v.px(180, 300), &[1.0]);
        assert!(v.px(0, 0)[0] < 1.0 && v.px(0, 0)[0] >= 0.5);
        assert!(v.data.iter().all(|&m| (0.5..=1.0).contains(&m)));
        // The same arguments give the same table, not a new one.
        assert!(Arc::ptr_eq(&v, &vignette(0.9, 0.55, 0.5, 0.47)));
        assert!(!Arc::ptr_eq(&v, &vignette(0.45, 0.55, 0.5, 0.47)));
    }

    #[test]
    fn the_scrim_darkens_the_top() {
        let s = top_scrim(0.58, 110.0, 120.0);
        assert_eq!((s.w, s.h, s.c), (W, H, 1));
        // Full strength above the top row: 0.58 in sixths is 3 or 4 sixths, by the dither.
        let top: Vec<f64> = s.row(0).to_vec();
        assert!(
            top.iter().all(|&m| m == 1.0 - 3.0 / 6.0 || m == 1.0 - 4.0 / 6.0),
            "{:?}",
            &top[..8]
        );
        assert!(s.row(400).iter().all(|&m| m == 1.0));
        assert!(Arc::ptr_eq(&s, &top_scrim(0.58, 110.0, 120.0)));
    }
}
