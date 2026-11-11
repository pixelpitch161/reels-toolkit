//! `Image.filter(ImageFilter.GaussianBlur(radius))` as Pillow 12.3.0 computes it.
//!
//! Ported from `ImageFilter.GaussianBlur.filter`, `_gaussian_blur` (`_imaging.c`) and `BoxBlur.c`. Pillow has no
//! Gaussian kernel: it runs three passes of an "extended box blur" along the rows and three more along the columns.
//! One pass averages a box of `2 * radius + 1` whole pixels plus a fraction of the pixel just outside each end, in
//! 8.24 fixed point; past an edge the box reads the edge pixel again. The box radius comes from the Gaussian
//! radius in C `float` arithmetic, and every channel is blurred on its own with the same integer arithmetic,
//! so the port keeps the C types: `f32` where C has `float`, wrapping `u32` where C has `UINT32`.

use crate::img::Img;

/// The number of box passes per direction (`passes = 3` in `_gaussian_blur`; the Python layer never changes it).
const PASSES: usize = 3;

/// `Image.filter(ImageFilter.GaussianBlur(radius))` for an 8-bit image; every channel is blurred alike (Pillow's
/// modes `L`, `RGB` and `RGBA` are 1, 3 and 4 channels).
///
/// A radius of exactly 0 gives a copy. The radius is read as a C `float`, and only its square is used, so the
/// sign does not matter.
pub fn gaussian_blur(src: &Img<u8>, radius: f64) -> Img<u8> {
    assert!(
        src.c >= 1 && src.data.len() == src.w * src.h * src.c,
        "a {} x {} image of {} channel(s) cannot hold {} samples",
        src.w,
        src.h,
        src.c,
        src.data.len()
    );
    let (w, h, c) = (src.w, src.h, src.c);
    let copy = || Img {
        w,
        h,
        c,
        data: src.data.clone(),
    };
    // `GaussianBlur.filter`: `if xy == 0: return image.copy()`. `ImagingBoxBlur` returns at once for an image
    // without pixels.
    if radius == 0.0 || w == 0 || h == 0 {
        return copy();
    }
    // `_gaussian_blur` parses the radius as a C float. `ImagingBoxBlur` refuses a negative box radius; one that
    // is not a number or does not fit the C ints is refused here too (C leaves that undefined).
    let box_radius = gaussian_box_radius(radius as f32);
    assert!(
        box_radius >= 0.0 && box_radius < (1u32 << 30) as f32,
        "the blur radius {radius} is out of range"
    );
    // Both box radii 0 (a radius whose square is 0 as a float): `ImagingBoxBlur` copies.
    if box_radius == 0.0 {
        return copy();
    }
    let blur = BoxLine::new(box_radius);
    let mut data = vec![0u8; src.data.len()];
    // One channel at a time: rows first, then the same passes down the columns (Pillow transposes the image,
    // blurs its rows and transposes it back). `a` holds the lines being read, `b` the lines being written.
    let mut a = vec![0u8; w * h];
    let mut b = vec![0u8; w * h];
    for ch in 0..c {
        for (v, px) in a.iter_mut().zip(src.data.chunks_exact(c)) {
            *v = px[ch];
        }
        for _ in 0..PASSES {
            for (line_out, line_in) in b.chunks_exact_mut(w).zip(a.chunks_exact(w)) {
                blur.run(line_out, line_in);
            }
            std::mem::swap(&mut a, &mut b);
        }
        transpose(&mut b, &a, w, h);
        std::mem::swap(&mut a, &mut b);
        for _ in 0..PASSES {
            for (line_out, line_in) in b.chunks_exact_mut(h).zip(a.chunks_exact(h)) {
                blur.run(line_out, line_in);
            }
            std::mem::swap(&mut a, &mut b);
        }
        transpose(&mut b, &a, h, w);
        std::mem::swap(&mut a, &mut b);
        for (px, &v) in data.chunks_exact_mut(c).zip(&a) {
            px[ch] = v;
        }
    }
    Img { w, h, c, data }
}

/// `_gaussian_blur_radius(radius, 3)`: the radius of the extended box whose three passes have the variance of a
/// Gaussian of standard deviation `radius`. The intermediates are C `float`s; the square root and the floor run
/// in `double` and are stored back into a `float`.
fn gaussian_box_radius(radius: f32) -> f32 {
    let sigma2 = radius * radius / PASSES as f32;
    // Box length, then the whole part of the box radius.
    let length = (12.0 * f64::from(sigma2) + 1.0).sqrt() as f32;
    let l = ((f64::from(length) - 1.0) / 2.0).floor() as f32;
    // The fractional part of the box radius.
    let mut a = (2.0 * l + 1.0) * (l * (l + 1.0) - 3.0 * sigma2);
    a /= 6.0 * (sigma2 - (l + 1.0) * (l + 1.0));
    l + a
}

/// One extended box blur of a line (`ImagingHorizontalBoxBlur` with `ImagingLineBoxBlur8`), set up for a radius.
struct BoxLine {
    /// The whole pixels on each side of the centre.
    radius: i32,
    /// The weight of a whole pixel, 8.24 fixed point.
    ww: u32,
    /// The weight of the pixel just past each end of the box, 8.24 fixed point.
    fw: u32,
}

impl BoxLine {
    fn new(float_radius: f32) -> BoxLine {
        let radius = float_radius as i32;
        // `(UINT32)(1 << 24) / (floatRadius * 2 + 1)` divides as floats and truncates to `UINT32`.
        let ww = ((1u32 << 24) as f32 / (float_radius * 2.0 + 1.0)) as u32;
        let fw = (1u32 << 24).wrapping_sub(((radius * 2 + 1) as u32).wrapping_mul(ww)) / 2;
        BoxLine { radius, ww, fw }
    }

    /// Blurs `line_in` into `line_out` (the same length, at least one pixel). The running sum `acc` covers the
    /// whole pixels of the box, with the first and the last pixel standing in for everything past the ends.
    fn run(&self, line_out: &mut [u8], line_in: &[u8]) {
        let (radius, ww, fw) = (self.radius, self.ww, self.fw);
        let xsize = line_in.len() as i32;
        let lastx = xsize - 1;
        let edge_a = (radius + 1).min(xsize);
        let edge_b = (xsize - radius - 1).max(0);
        let at = |i: i32| u32::from(line_in[i as usize]);

        // The sum for the pixel before the line: `radius + 1` copies of the first pixel, then the pixels up to
        // `radius - 1`, the last pixel standing in for those past the end.
        let mut acc = at(0).wrapping_mul((radius + 1) as u32);
        for x in 0..edge_a - 1 {
            acc = acc.wrapping_add(at(x));
        }
        acc = acc.wrapping_add(at(lastx).wrapping_mul((radius - edge_a + 1) as u32));

        // Moves the box one pixel (drop `subtract`, take `add`), adds the two fractional ends (`left`, `right`)
        // and rounds to 8 bits.
        let mut step = |x: i32, subtract: i32, add: i32, left: i32, right: i32| {
            acc = acc.wrapping_add(at(add)).wrapping_sub(at(subtract));
            let bulk = acc
                .wrapping_mul(ww)
                .wrapping_add((at(left) + at(right)).wrapping_mul(fw));
            line_out[x as usize] = (bulk.wrapping_add(1 << 23) >> 24) as u8;
        };
        if edge_a <= edge_b {
            // The box hangs over the left end, then lies inside the line, then hangs over the right end.
            for x in 0..edge_a {
                step(x, 0, x + radius, 0, x + radius + 1);
            }
            for x in edge_a..edge_b {
                step(x, x - radius - 1, x + radius, x - radius - 1, x + radius + 1);
            }
            for x in edge_b..=lastx {
                step(x, x - radius - 1, lastx, x - radius - 1, lastx);
            }
        } else {
            // The box is wider than the line: in the middle stretch it hangs over both ends.
            for x in 0..edge_b {
                step(x, 0, x + radius, 0, x + radius + 1);
            }
            for x in edge_b..edge_a {
                step(x, 0, lastx, 0, lastx);
            }
            for x in edge_a..=lastx {
                step(x, x - radius - 1, lastx, x - radius - 1, lastx);
            }
        }
    }
}

/// Writes the transpose of the `w` x `h` plane `src` into `dst` (`h` x `w`: its rows are the columns of `src`).
fn transpose(dst: &mut [u8], src: &[u8], w: usize, h: usize) {
    for (x, column) in dst.chunks_exact_mut(h).enumerate() {
        for (y, v) in column.iter_mut().enumerate() {
            *v = src[y * w + x];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The box each radius of the reference vectors turns into: the box radius as float bits, its whole part,
    /// and the two fixed-point weights. Worked out with numpy `float32` arithmetic following
    /// `_gaussian_blur_radius` and `ImagingHorizontalBoxBlur` step by step. In `double` arithmetic 0.7 (the
    /// shadow masks) would get `ww` 14036937 and 1.0 a box radius of exactly 0.25.
    #[test]
    fn the_box_of_each_radius() {
        let table: [(f32, u32, i32, u32, u32); 12] = [
            (0.7, 0x3dc7_e784, 0, 14_036_938, 1_370_139),
            (3.0, 0x401a_aaab, 2, 2_876_094, 1_198_373),
            (0.3, 0x3c7d_5c5f, 0, 16_273_900, 251_658),
            (0.5, 0x3d3a_2e8c, 0, 15_379_114, 699_051),
            (1.0, 0x3e80_0001, 0, 11_184_811, 2_796_202),
            (1.5, 0x3f84_ec4f, 1, 5_452_595, 209_715),
            (2.0, 0x3fb0_0000, 1, 4_473_924, 1_677_722),
            (2.5, 0x4001_ed7e, 2, 3_315_497, 99_865),
            (4.0, 0x405c_0000, 3, 2_130_440, 932_068),
            (5.0, 0x408e_6666, 4, 1_694_668, 762_602),
            (8.0, 0x40ef_0000, 7, 1_052_688, 493_448),
            (0.05, 0x39da_a2b3, 0, 16_763_236, 6_990),
        ];
        for (radius, bits, whole, ww, fw) in table {
            let box_radius = gaussian_box_radius(radius);
            assert_eq!(box_radius.to_bits(), bits, "radius {radius}: box radius {box_radius}");
            let line = BoxLine::new(box_radius);
            assert_eq!((line.radius, line.ww, line.fw), (whole, ww, fw), "radius {radius}");
            // The weights of a whole box add up to 1.0 in 8.24 fixed point, less at most the rounding.
            let total = (2 * whole as u32 + 1) * ww + 2 * fw;
            assert!((1 << 24) - total <= 1, "radius {radius}: the weights sum to {total}");
        }
    }

    /// A radius whose square is 0 as a C float gives a box radius of 0, which Pillow answers with a copy.
    #[test]
    fn a_vanishing_radius_is_a_copy() {
        assert_eq!(gaussian_box_radius(1e-30), 0.0);
        let src = Img {
            w: 3,
            h: 1,
            c: 1,
            data: vec![0, 255, 0],
        };
        assert_eq!(gaussian_blur(&src, 1e-30).data, [0, 255, 0]);
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn a_radius_that_is_not_a_number_is_refused() {
        let src = Img {
            w: 1,
            h: 1,
            c: 1,
            data: vec![7],
        };
        gaussian_blur(&src, f64::NAN);
    }
}
