//! `Image.resize(size, filter)` as Pillow 12.3.0 computes it.
//!
//! Ported from `Image.resize` (`Image.py`), `_resize` (`_imaging.c`), `Resample.c` and, for NEAREST,
//! `ImagingScaleAffine` (`Geometry.c`). The box is always the whole image and there is no `reducing_gap`.
//!
//! BILINEAR and BICUBIC are convolutions: for every output pixel a row of filter weights is computed in `double`,
//! normalised to sum to 1 and rounded to 22-bit fixed point. The horizontal pass runs first (every row to the new
//! width, and only the source rows the second pass will read), then the vertical pass on its 8-bit result; each
//! sum is rounded and clipped to 8 bits. A filter widens with the scale when the image shrinks, so every source
//! pixel counts. NEAREST copies one source pixel per output pixel, stepping through the source with a running
//! `double` sum.

use crate::img::Img;

/// The resampling filters of `Image.resize` that the Studio reproduces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resample {
    Nearest,
    Bilinear,
    Bicubic,
}

/// `Image.resize((w, h), how)` for an 8-bit image of 1, 3 or 4 channels (Pillow's `L`, `RGB` and `RGBA`).
///
/// A 4-channel image is `RGBA`: for every filter but NEAREST its colour is multiplied by its alpha before the
/// resize and divided by it afterwards, as Pillow does by converting to `RGBa` and back.
pub fn resize(src: &Img<u8>, w: usize, h: usize, how: Resample) -> Img<u8> {
    assert!(
        matches!(src.c, 1 | 3 | 4) && src.data.len() == src.w * src.h * src.c,
        "resize takes 1, 3 or 4 channels: got {} x {} x {} with {} samples",
        src.w,
        src.h,
        src.c,
        src.data.len()
    );
    // `Image.resize`: the same size is `self.copy()`.
    if (w, h) == (src.w, src.h) {
        return copy(src);
    }
    if src.c == 4 && how != Resample::Nearest {
        let premultiplied = resize_bands(&rgba_to_premultiplied(src), w, h, how);
        return premultiplied_to_rgba(&premultiplied);
    }
    resize_bands(src, w, h, how)
}

/// The rest of `Image.resize`, for an image whose channels are resized as they are (`L`, `RGB`, `RGBa`).
fn resize_bands(src: &Img<u8>, w: usize, h: usize, how: Resample) -> Img<u8> {
    // An image more than 100 times taller than wide that gets shorter is resized in two calls: to the new
    // height at its own width, then to the new width (the ordinary path does the width first).
    if src.h > src.w * 100 && h < src.h {
        let shorter = core_resize(src, src.w, h, how);
        core_resize(&shorter, w, h, how)
    } else {
        core_resize(src, w, h, how)
    }
}

fn copy(src: &Img<u8>) -> Img<u8> {
    Img {
        w: src.w,
        h: src.h,
        c: src.c,
        data: src.data.clone(),
    }
}

/// `ImagingCore.resize(size, filter, box)` with the box the whole image (`_resize` in `_imaging.c`).
fn core_resize(src: &Img<u8>, w: usize, h: usize, how: Resample) -> Img<u8> {
    assert!(w >= 1 && h >= 1, "height and width must be > 0");
    // A box of the requested size is a crop, and the whole image cropped is a copy.
    if (w, h) == (src.w, src.h) {
        return copy(src);
    }
    match how {
        Resample::Nearest => scale_nearest(src, w, h),
        Resample::Bilinear => resample(src, w, h, &BILINEAR),
        Resample::Bicubic => resample(src, w, h, &BICUBIC),
    }
}

// ---------------------------------------------------------------- NEAREST

/// `COORD` of `Geometry.c`: the pixel a source position falls in; anything left of 0 is outside.
fn coord(v: f64) -> i32 {
    if v < 0.0 { -1 } else { v as i32 }
}

/// `ImagingScaleAffine` as `_resize` calls it: the transform `a` is a pure scale, the output starts as zeros
/// (`fill`), and the source position is a sum that grows by the scale at every pixel, not a product.
fn scale_nearest(src: &Img<u8>, w: usize, h: usize) -> Img<u8> {
    let c = src.c;
    // `_resize` keeps the box as C floats: `a[0] = (double)(box[2] - box[0]) / xsize`, and the same for y.
    let (box0, box1, box2, box3) = (0.0f32, 0.0f32, src.w as f32, src.h as f32);
    let scale_x = f64::from(box2 - box0) / w as f64;
    let scale_y = f64::from(box3 - box1) / h as f64;
    // The centre of the first output pixel: `a[2] + a[0] * 0.5`, with `a[2]` the box's left edge.
    let mut xo = f64::from(box0) + scale_x * 0.5;
    let mut yo = f64::from(box1) + scale_y * 0.5;

    // The source column of every output column; the columns that have one are `xmin..xmax`.
    let mut xintab = vec![0usize; w];
    let (mut xmin, mut xmax) = (w, 0);
    for (x, slot) in xintab.iter_mut().enumerate() {
        let xin = coord(xo);
        if xin >= 0 && (xin as usize) < src.w {
            xmax = x + 1;
            xmin = xmin.min(x);
            *slot = xin as usize;
        }
        xo += scale_x;
    }

    let columns = xmin..xmax.max(xmin);

    let mut data = vec![0u8; w * h * c];
    for row in data.chunks_exact_mut(w * c) {
        let yi = coord(yo);
        if yi >= 0 && (yi as usize) < src.h {
            let line = &src.data[yi as usize * src.w * c..][..src.w * c];
            let pixels = row[columns.start * c..columns.end * c].chunks_exact_mut(c);
            for (px, &xin) in pixels.zip(&xintab[columns.clone()]) {
                px.copy_from_slice(&line[xin * c..][..c]);
            }
        }
        yo += scale_y;
    }
    Img { w, h, c, data }
}

// ---------------------------------------------------------------- BILINEAR and BICUBIC

/// A resampling filter of `Resample.c`: the weight at a distance from the centre, and the distance past which
/// it is zero (at a scale of 1).
struct Filter {
    weight: fn(f64) -> f64,
    support: f64,
}

const BILINEAR: Filter = Filter {
    weight: bilinear_filter,
    support: 1.0,
};
const BICUBIC: Filter = Filter {
    weight: bicubic_filter,
    support: 2.0,
};

fn bilinear_filter(x: f64) -> f64 {
    let x = if x < 0.0 { -x } else { x };
    if x < 1.0 { 1.0 - x } else { 0.0 }
}

/// The cubic convolution kernel with `a = -0.5`, in the operation order of `bicubic_filter`.
fn bicubic_filter(x: f64) -> f64 {
    const A: f64 = -0.5;
    let x = if x < 0.0 { -x } else { x };
    if x < 1.0 {
        ((A + 2.0) * x - (A + 3.0)) * x * x + 1.0
    } else if x < 2.0 {
        (((x - 5.0) * x + 8.0) * x - 4.0) * A
    } else {
        0.0
    }
}

/// The fixed-point scale of the weights: 8 bits for the sample and 2 spare, because a filter with negative
/// lobes can push a sum below 0 or above 255.
const PRECISION_BITS: u32 = 32 - 8 - 2;

/// 1.0 in that fixed point.
const ONE: f64 = (1u32 << PRECISION_BITS) as f64;

/// `clip8`: a fixed-point sum as a byte, clipped to 0..=255 (the shift floors, as C's does on this platform).
fn clip8(sum: i32) -> u8 {
    (sum >> PRECISION_BITS).clamp(0, 255) as u8
}

/// The weights of one pass: output pixel `i` reads `bounds[i].1` source pixels starting at `bounds[i].0`, with
/// the weights `kk[i * ksize..]`.
struct Coeffs {
    ksize: usize,
    bounds: Vec<(usize, usize)>,
    kk: Vec<i32>,
}

/// `precompute_coeffs` followed by `normalize_coeffs_8bpc`, for a box that is the whole line: `in_size` source
/// pixels become `out_size`.
fn precompute_coeffs(in_size: usize, out_size: usize, filter: &Filter) -> Coeffs {
    // The box ends are C floats (`float box[4]`), widened to double after the subtraction.
    let (in0, in1) = (0.0f32, in_size as f32);
    let scale = f64::from(in1 - in0) / out_size as f64;
    let filterscale = if scale < 1.0 { 1.0 } else { scale };
    // The length of the resampling filter, and the most weights a pixel can have.
    let support = filter.support * filterscale;
    let ksize = support.ceil() as usize * 2 + 1;
    let inv_filterscale = 1.0 / filterscale;

    let mut bounds = Vec::with_capacity(out_size);
    let mut kk = vec![0i32; out_size * ksize];
    let mut k = vec![0.0f64; ksize];
    for (xx, fixed) in kk.chunks_exact_mut(ksize).enumerate() {
        let center = f64::from(in0) + (xx as f64 + 0.5) * scale;
        // Both ends are rounded by adding 0.5 and truncating toward zero.
        let xmin = ((center - support + 0.5) as i32).max(0);
        let xmax = ((center + support + 0.5) as i32).min(in_size as i32);
        let count = (xmax - xmin).max(0) as usize;
        let mut ww = 0.0;
        for (x, w) in k[..count].iter_mut().enumerate() {
            *w = (filter.weight)((f64::from(x as i32 + xmin) - center + 0.5) * inv_filterscale);
            ww += *w;
        }
        if ww != 0.0 {
            for w in &mut k[..count] {
                *w /= ww;
            }
        }
        // To fixed point, rounding half away from zero.
        for (out, &w) in fixed.iter_mut().zip(&k[..count]) {
            *out = if w < 0.0 {
                (-0.5 + w * ONE) as i32
            } else {
                (0.5 + w * ONE) as i32
            };
        }
        bounds.push((xmin as usize, count));
    }
    Coeffs { ksize, bounds, kk }
}

/// `ImagingResample` with the box the whole image: the horizontal pass first (when the width changes), then the
/// vertical pass (when the height changes).
fn resample(src: &Img<u8>, w: usize, h: usize, filter: &Filter) -> Img<u8> {
    let mut vert = precompute_coeffs(src.h, h, filter);
    if w == src.w {
        return resample_vertical(src, h, &vert);
    }
    // The first and one past the last source row that the vertical pass reads: the horizontal pass makes only
    // those, and the vertical bounds move up to match.
    let ybox_first = vert.bounds[0].0;
    let ybox_last = vert.bounds[h - 1].0 + vert.bounds[h - 1].1;
    let horiz = precompute_coeffs(src.w, w, filter);
    for b in &mut vert.bounds {
        b.0 -= ybox_first;
    }
    let temp = resample_horizontal(src, ybox_first, w, ybox_last - ybox_first, &horiz);
    if h == src.h {
        return temp;
    }
    resample_vertical(&temp, h, &vert)
}

/// `ImagingResampleHorizontal_8bpc`: rows `offset..offset + h` of `src`, each resampled to `w` pixels.
fn resample_horizontal(src: &Img<u8>, offset: usize, w: usize, h: usize, co: &Coeffs) -> Img<u8> {
    let c = src.c;
    let mut data = vec![0u8; w * h * c];
    for (yy, line_out) in data.chunks_exact_mut(w * c).enumerate() {
        let line_in = &src.data[(yy + offset) * src.w * c..][..src.w * c];
        for (xx, px) in line_out.chunks_exact_mut(c).enumerate() {
            let (xmin, count) = co.bounds[xx];
            let k = &co.kk[xx * co.ksize..][..count];
            for (ch, out) in px.iter_mut().enumerate() {
                let mut ss = 1i32 << (PRECISION_BITS - 1);
                for (x, &kx) in k.iter().enumerate() {
                    ss += i32::from(line_in[(x + xmin) * c + ch]) * kx;
                }
                *out = clip8(ss);
            }
        }
    }
    Img { w, h, c, data }
}

/// `ImagingResampleVertical_8bpc`: every column of `src` resampled to `h` pixels.
fn resample_vertical(src: &Img<u8>, h: usize, co: &Coeffs) -> Img<u8> {
    let (w, c) = (src.w, src.c);
    let stride = w * c;
    let mut data = vec![0u8; stride * h];
    for (yy, line_out) in data.chunks_exact_mut(stride).enumerate() {
        let (ymin, count) = co.bounds[yy];
        let k = &co.kk[yy * co.ksize..][..count];
        for (i, out) in line_out.iter_mut().enumerate() {
            let mut ss = 1i32 << (PRECISION_BITS - 1);
            for (y, &ky) in k.iter().enumerate() {
                ss += i32::from(src.data[(y + ymin) * stride + i]) * ky;
            }
            *out = clip8(ss);
        }
    }
    Img { w, h, c, data }
}

// ---------------------------------------------------------------- RGBA <-> RGBa

/// `rgbA2rgba` (`Convert.c`): colour times alpha over 255, rounded by `MULDIV255`.
fn rgba_to_premultiplied(src: &Img<u8>) -> Img<u8> {
    let mut data = src.data.clone();
    for [r, g, b, a] in data.as_chunks_mut::<4>().0 {
        let alpha = u32::from(*a);
        for v in [r, g, b] {
            let tmp = u32::from(*v) * alpha + 128;
            *v = (((tmp >> 8) + tmp) >> 8) as u8;
        }
    }
    Img {
        w: src.w,
        h: src.h,
        c: 4,
        data,
    }
}

/// `rgba2rgbA` (`Convert.c`): colour times 255 over alpha, truncated and clipped; an alpha of 0 or 255 leaves
/// the colour as it is.
fn premultiplied_to_rgba(src: &Img<u8>) -> Img<u8> {
    let mut data = src.data.clone();
    for [r, g, b, a] in data.as_chunks_mut::<4>().0 {
        let alpha = u32::from(*a);
        if alpha != 255 && alpha != 0 {
            for v in [r, g, b] {
                *v = (255 * u32::from(*v) / alpha).min(255) as u8;
            }
        }
    }
    Img {
        w: src.w,
        h: src.h,
        c: 4,
        data,
    }
}
