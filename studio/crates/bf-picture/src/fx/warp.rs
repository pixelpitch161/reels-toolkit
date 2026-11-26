//! The local warps: heat shimmer on a shockwave's front and chromatic fringing around a big hit. Both only move
//! pixels inside a small box, so they take a picture of any sample type.
//!
//! Both are ported as the ring calls them: with a centre (and, for the shimmer, half-axes) that are numpy
//! `float64` scalars (what `cam.wp` and `ellipse_of` give), which makes every intermediate `float64`. Called with
//! plain Python floats numpy would compute the same expressions in `float32`; nothing in the kit does that.

use crate::camera::{Cam, V3};
use crate::img::Img;
use crate::np;
use bf_core::dmath;
use bf_core::py::{py_max, round_half_even};

use super::bbox;

/// `np.clip(np.round(v), 0, size - 1).astype(int)` as an index, for a `v` that is a number.
fn index(v: f64, size: usize) -> Option<usize> {
    let i = np::clip(round_half_even(v), 0.0, size as f64 - 1.0);
    i.is_finite().then_some(i as usize)
}

/// `shimmer(img, cx, cy, ax, ay, width, amp, t, k, seed)`: a heat-shimmer ripple on an elliptical front (screen
/// centre `cx`, `cy`, half-axes `ax`, `ay` pixels): pixels within `width` pixels of the front are pushed in or
/// out along the radius by up to `amp` pixels, with a wobbling, travelling profile. Only the front's box is
/// touched; nothing happens for `k <= 0.01` or `amp * k < 0.4`. The Python defaults are `t=0.0`, `k=1.0`,
/// `seed=0`.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn shimmer<T: Copy>(
    img: &mut Img<T>,
    cx: f64,
    cy: f64,
    ax: f64,
    ay: f64,
    width: f64,
    amp: f64,
    t: f64,
    k: f64,
    seed: f64,
) {
    if k <= 0.01 || amp * k < 0.4 {
        return;
    }
    let Some((x0, x1, y0, y1)) = bbox(cx, cy, ax + width + amp, ay + width + amp, img.w, img.h) else {
        return;
    };
    let (mx, my) = (py_max(ax, 1.0), py_max(ay, 1.0));
    let mut moved: Vec<(usize, usize)> = Vec::new();
    for yi in y0..y1 {
        for xi in x0..x1 {
            let (xx, yy) = (xi as f64, yi as f64);
            let (dx, dy) = (xx - cx, yy - cy);
            let (qx, qy) = (dx / mx, dy / my);
            let rho = (qx * qx + qy * qy).sqrt() + 1e-6;
            // signed pixels from the front
            let dist = (rho - 1.0) * 0.5 * (ax + ay);
            let c = np::clip(dist / width, -1.0, 1.0);
            let cosine = dmath::cos(c * dmath::PI / 2.0);
            let prof = cosine * cosine;
            // m = prof > 0.02 (false for a NaN)
            if prof.is_nan() || prof <= 0.02 {
                continue;
            }
            let ang = dmath::atan2(dy, dx);
            let wob = 1.0 + 0.45 * dmath::sin(ang * 9.0 + t * 31.0 + seed) * dmath::sin(ang * 4.0 - t * 17.0);
            let disp = amp * k * prof * wob * dmath::sin(c * dmath::PI);
            let sx = index(xx - dmath::cos(ang) * disp, img.w);
            let sy = index(yy - dmath::sin(ang) * disp, img.h);
            if let (Some(sx), Some(sy)) = (sx, sy) {
                moved.push((img.at(xi, yi), img.at(sx, sy)));
            }
        }
    }
    // reg[m] = img[sy[m], sx[m]]: every pixel is read from the picture as it was
    let src = img.data.clone();
    let c = img.c;
    for (to, from) in moved {
        img.data[to..to + c].copy_from_slice(&src[from..from + c]);
    }
}

/// `ellipse_of(cam, c, r)`: the screen centre and half-axes `(cx, cy, ax, ay)` of a horizontal world circle
/// (centre `c`, radius `r`), from eight points of it.
pub fn ellipse_of(cam: &Cam, c: V3, r: f64) -> (f64, f64, f64, f64) {
    #[allow(clippy::approx_constant)] // Python's own 0.7071, not the root of a half
    let d = r * 0.7071;
    let p = [
        [c[0] + r, c[1] + 0.0, c[2] + 0.0],
        [c[0] - r, c[1] - 0.0, c[2] - 0.0],
        [c[0] + 0.0, c[1] + 0.0, c[2] + r],
        [c[0] - 0.0, c[1] - 0.0, c[2] - r],
        [c[0] + d, c[1] + 0.0, c[2] + d],
        [c[0] - d, c[1] - 0.0, c[2] - d],
        [c[0] + d, c[1] + 0.0, c[2] + -d],
        [c[0] - d, c[1] - 0.0, c[2] - -d],
    ];
    let s = cam.wp(&p);
    let (mut xa, mut xb, mut ya, mut yb) = (s[0].0, s[0].0, s[0].1, s[0].1);
    for &(x, y) in &s[1..] {
        xa = np::minimum(xa, x);
        xb = np::maximum(xb, x);
        ya = np::minimum(ya, y);
        yb = np::maximum(yb, y);
    }
    ((xb + xa) / 2.0, (yb + ya) / 2.0, (xb - xa) / 2.0, (yb - ya) / 2.0)
}

/// `shimmer_world(img, cam, c, r, width, amp, t, k, seed)`: [`shimmer`] on a world circle on the floor (a
/// shockwave's front): `width` in world units, `amp` in pixels. The Python defaults are `t=0.0`, `k=1.0`,
/// `seed=0`.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn shimmer_world<T: Copy>(
    img: &mut Img<T>,
    cam: &Cam,
    c: V3,
    r: f64,
    width: f64,
    amp: f64,
    t: f64,
    k: f64,
    seed: f64,
) {
    let (cx, cy, ax, ay) = ellipse_of(cam, c, r);
    let wpx = py_max(2.0, width * ax / py_max(r, 1e-3));
    shimmer(img, cx, cy, ax, ay, wpx, amp, t, k, seed);
}

/// `fringe(img, cx, cy, r, px, k)`: chromatic fringing around a big hit. Inside a disc of radius `r` pixels red
/// is taken from further out and blue from further in (by up to `px` pixels), fading to nothing at the edge.
/// Nothing happens for `k <= 0.01`. The Python defaults are `px=2.0`, `k=1.0`.
///
/// # Panics
/// If the picture has fewer than three channels.
pub fn fringe<T: Copy>(img: &mut Img<T>, cx: f64, cy: f64, r: f64, px: f64, k: f64) {
    assert!(img.c >= 3, "fringe: a picture of {} channel(s)", img.c);
    if k <= 0.01 {
        return;
    }
    let Some((x0, x1, y0, y1)) = bbox(cx, cy, r, r, img.w, img.h) else {
        return;
    };
    // (the sample to write, the sample to read): red, then blue
    let mut moved: Vec<(usize, usize)> = Vec::new();
    for yi in y0..y1 {
        for xi in x0..x1 {
            let (xx, yy) = (xi as f64, yi as f64);
            let (dx, dy) = (xx - cx, yy - cy);
            let d = (dx * dx + dy * dy).sqrt() + 1e-6;
            let f = dmath::pow(np::clip(1.0 - d / r, 0.0, 1.0), 0.6) * np::clip(d / 3.0, 0.0, 1.0) * px * k;
            // m = f > 0.3 (false for a NaN)
            if f.is_nan() || f <= 0.3 {
                continue;
            }
            let (ux, uy) = (dx / d, dy / d);
            let at = img.at(xi, yi);
            if let (Some(sx), Some(sy)) = (index(xx - ux * f, img.w), index(yy - uy * f, img.h)) {
                moved.push((at, img.at(sx, sy)));
            }
            if let (Some(sx), Some(sy)) = (index(xx + ux * f, img.w), index(yy + uy * f, img.h)) {
                moved.push((at + 2, img.at(sx, sy) + 2));
            }
        }
    }
    let src = img.data.clone();
    for (to, from) in moved {
        img.data[to] = src[from];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture whose every sample says where it is.
    fn numbered(w: usize, h: usize) -> Img<f64> {
        Img::from_vec(w, h, 3, (0..w * h * 3).map(|i| i as f64).collect())
    }

    #[test]
    fn a_weak_shimmer_and_a_weak_fringe_do_nothing() {
        let before = numbered(60, 50);
        let mut img = before.clone();
        shimmer(&mut img, 30.0, 25.0, 12.0, 8.0, 3.0, 3.0, 0.4, 0.01, 0.0);
        shimmer(&mut img, 30.0, 25.0, 12.0, 8.0, 3.0, 1.9, 0.4, 0.2, 0.0);
        shimmer(&mut img, f64::NAN, 25.0, 12.0, 8.0, 3.0, 3.0, 0.4, 1.0, 0.0);
        fringe(&mut img, 30.0, 25.0, 12.0, 2.5, 0.01);
        fringe(&mut img, 30.0, 25.0, 12.0, 2.5, 0.1);
        fringe(&mut img, 200.0, 25.0, 12.0, 2.5, 1.0);
        assert_eq!(img, before);
    }

    #[test]
    fn a_fringe_moves_red_and_blue_only_and_only_in_its_disc() {
        let before = numbered(60, 50);
        let mut img = before.clone();
        fringe(&mut img, 30.0, 25.0, 12.0, 2.5, 1.0);
        let mut changed = 0;
        for y in 0..50 {
            for x in 0..60 {
                let (a, b) = (img.px(x, y), before.px(x, y));
                assert_eq!(a[1], b[1]);
                if a != b {
                    changed += 1;
                    let d = ((x as f64 - 30.0) * (x as f64 - 30.0) + (y as f64 - 25.0) * (y as f64 - 25.0)).sqrt();
                    assert!(d < 12.0, "({x}, {y})");
                }
            }
        }
        assert!(changed > 100, "{changed}");
        // every sample is one the picture already had, in its own channel
        assert!(img.data.iter().enumerate().all(|(i, &v)| v as usize % 3 == i % 3));
    }

    #[test]
    fn a_shimmer_moves_whole_pixels_near_its_front() {
        let before = numbered(60, 50);
        let mut img = before.clone();
        shimmer(&mut img, 30.0, 25.0, 12.0, 8.0, 3.0, 3.0, 0.4, 1.0, 1.0);
        assert_ne!(img, before);
        for y in 0..50 {
            for x in 0..60 {
                let p = img.px(x, y);
                assert!((p[0] as usize).is_multiple_of(3) && p[1] == p[0] + 1.0 && p[2] == p[0] + 2.0);
            }
        }
        // the centre is far from the front
        assert_eq!(img.px(30, 25), before.px(30, 25));
    }
}
