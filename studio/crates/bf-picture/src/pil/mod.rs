//! Pillow 12.3.0's drawing primitives, pixel for pixel: `ImageDraw.polygon`, `line`, `ellipse`, `rectangle` and
//! `point`, ported from `src/libImaging/Draw.c`, `src/_imaging.c` and `src/PIL/ImageDraw.py`.
//!
//! The kit draws every frame with these calls, so every function here gives the bytes Pillow gives.
//! `tests/pillow_vectors.rs` holds them to that on the cases of `studio/reference/pillow` (2,447 of them).
//!
//! # The API
//!
//! One function per `ImageDraw` method, in two families.
//!
//! **Plain drawing**, `ImageDraw.Draw(im)`: the ink replaces the pixel. The functions are generic over the sample
//! type ([`Sample`]): `Img<u8>` with 1 channel (Pillow's mode `L`), 3 (`RGB`) or 4 (`RGBA`), and `Img<f32>` with
//! 1 channel (mode `F`, the depth buffer). The ink is a slice with one sample per channel.
//!
//! ```
//! use bf_picture::img::Img;
//! use bf_picture::pil;
//! let mut rgb: Img<u8> = Img::filled(64, 48, &[20, 20, 30]); // Image.new('RGB', (64, 48), (20, 20, 30))
//! let mut mask: Img<u8> = Img::new(64, 48, 1); //                Image.new('L', (64, 48), 0)
//! let mut depth: Img<f32> = Img::filled(64, 48, &[1e6]); //      Image.new('F', (64, 48), 1e6)
//! let mut patch: Img<u8> = Img::new(21, 21, 4); //               Image.new('RGBA', (21, 21), (0, 0, 0, 0))
//! let quad = [(3.2, 4.9), (40.0, 6.5), (44.7, 30.1), (5.5, 28.0)];
//! // draw.polygon(quad, fill=(200, 120, 40))
//! pil::polygon(&mut rgb, &quad, &[200, 120, 40]);
//! // ddraw.polygon(quad, fill=float(z)): a Python float becomes float32, as in Pillow
//! let z: f64 = 7.25;
//! pil::polygon(&mut depth, &quad, &[z as f32]);
//! // draw.polygon(quad, fill=255) on a mask
//! pil::polygon(&mut mask, &quad, &[255]);
//! // d.polygon(pts, fill=core + (a,), outline=ink + (a,)) and d.polygon(pts, outline=...)
//! let star = [(2.0, 2.0), (18.0, 4.0), (10.0, 19.0)];
//! pil::polygon_outline(&mut patch, &star, Some(&[255, 240, 200, 255]), &[12, 16, 30, 255]);
//! pil::polygon_outline(&mut patch, &star, None, &[12, 16, 30, 255]);
//! // draw.line(pts, fill=(255, 255, 255), width=3); a call without width= is width 1
//! pil::line(&mut rgb, &[(1.0, 1.0), (30.5, 12.0), (60.0, 40.0)], &[255, 255, 255], 3);
//! pil::line(&mut depth, &[(1.0, 1.0), (30.5, 12.0)], &[z as f32], 2);
//! // draw.ellipse([x0, y0, x1, y1], fill=(90, 80, 70)); outline=... with the default width=1
//! pil::ellipse(&mut rgb, (10.4, 10.4, 20.6, 18.0), Some(&[90, 80, 70]), None, 1);
//! pil::ellipse(&mut rgb, (10.4, 10.4, 20.6, 18.0), None, Some(&[0, 0, 0]), 1);
//! // draw.rectangle([x0, y0, x1, y1], fill=..., outline=...)
//! pil::rectangle(&mut rgb, (2.0, 2.0, 9.9, 5.0), Some(&[1, 2, 3]), Some(&[9, 9, 9]));
//! // draw.point((x, y), fill=...) and draw.point([(x, y), ...], fill=...)
//! pil::point(&mut rgb, &[(5.9, 5.1)], &[255, 0, 0]);
//! ```
//!
//! **Blended drawing**, `ImageDraw.Draw(rgb_image, 'RGBA')`: every pixel the shape lands on becomes the blend of
//! the pixel and the ink by the ink's alpha. The image is RGB (3 channels), the ink `[r, g, b, a]`. The same
//! names, in [`blend`].
//!
//! ```
//! use bf_picture::img::Img;
//! use bf_picture::pil;
//! let mut rgb: Img<u8> = Img::filled(64, 48, &[20, 20, 30]);
//! // d = ImageDraw.Draw(base, 'RGBA')
//! // d.line([p, q], fill=col + (a,), width=2)
//! pil::blend::line(&mut rgb, &[(4.0, 4.0), (40.2, 20.7)], [255, 240, 200, 128], 2);
//! // d.ellipse([x - r, y - r, x + r, y + r], fill=c + (a,))
//! pil::blend::ellipse(&mut rgb, (10.0, 10.0, 16.5, 16.5), Some([255, 200, 90, 63]), None, 1);
//! // d.point((x, y), fill=c + (a,))
//! pil::blend::point(&mut rgb, &[(12.3, 9.8)], [255, 255, 240, 200]);
//! // d.polygon(pts, fill=c + (a,))
//! pil::blend::polygon(&mut rgb, &[(1.0, 1.0), (9.0, 2.0), (5.0, 9.0)], [90, 60, 40, 180]);
//! // d.rectangle([x0, y0, x1, y1], fill=c + (a,))
//! pil::blend::rectangle(&mut rgb, (30.0, 30.0, 36.0, 33.0), Some([1, 2, 3, 99]), None);
//! ```
//!
//! Conventions, the same for every function:
//!
//! - **Coordinates** are `f64`, exactly the Python floats the kit passes; a point is `(x, y)`, a box
//!   `(x0, y0, x1, y1)`. Truncation to pixels happens inside (see the quirks). Do not round or truncate first.
//! - **Inks** are in the image's own sample type, one per channel, and the slice length must equal `img.c`
//!   (a panic otherwise). The kit's `tuple(int(c) for c in col)` is `[col[0] as u8, col[1] as u8, col[2] as u8]`
//!   for `f64` channels: Python's `int()` truncates toward zero and Pillow clamps an ink to 0..=255 (`CLIP8` in
//!   `getink`), which together is exactly Rust's float-to-`u8` cast. `fill=float(z)` on the depth buffer is
//!   `&[z as f32]`.
//! - **Nothing is returned and nothing fails for geometry**: shapes outside the image are clipped; a degenerate
//!   shape draws what Pillow draws for it. The only panics are the calls Pillow itself refuses (a box with
//!   `x1 < x0` or `y1 < y0`, a polygon of fewer than two points) and a wrong ink length or channel count.
//! - **NaN and infinite coordinates are not refused and cannot stall a frame.** Pillow does not refuse them either
//!   (its box check `x1 < x0` is false for a NaN); it truncates them to the lowest `int` and draws with that. So
//!   does this port, with the wrapping arithmetic of the reference build, and it gives the same pixels: usually
//!   none, sometimes a run to the image's edge. Where Pillow would walk thousands of millions of pixels to get
//!   there (a thin line: 1.5 to 2.7 s each; an ellipse outline: more than 40 s, or for ever) the port goes
//!   straight to the part on the image. See quirk 10.
//! - **Widths**: `line(.., width)` takes the `width=` keyword (pass 1 when the Python call has none); `ellipse`
//!   takes the outline width (Pillow's default is 1); `rectangle` and `polygon_outline` draw 1 px outlines, the
//!   only width the kit uses.
//!
//! # Pillow quirks reproduced (and where each comes from)
//!
//! 1. **Coordinates truncate toward zero** to a C `int` (`_imaging.c`, `_draw_polygon`, `_draw_lines`,
//!    `_draw_ellipse`, `_draw_rectangle`, `_draw_points`: `(int)xy[i]`): -0.7 is 0, 10.99 is 10. A value no `int`
//!    holds becomes `i32::MIN`, as the x86 conversion gives ([`crate::np::f64_to_i32`]).
//! 2. **The polygon filler** (`Draw.c`, `ImagingDrawPolygon` and `polygon_generic`) is an edge-list scan fill.
//!    Per scan line it collects each edge's x as a C `float` (`(y - y0) * dx + x0`, with `dx` a `float` slope),
//!    sorts them and fills between pairs from `ROUND_UP(left)` to `ROUND_DOWN(right)`. `ROUND_UP` is the nearest
//!    whole number with halves away from zero (`floor(f + 0.5F)`), `ROUND_DOWN` with halves toward zero
//!    (`ceil(f - 0.5F)`); for `f >= 0` the sum is a `float` sum, for `f < 0` a `double` one. (The float sum shows
//!    for exactly one value: 0.49999997, the largest float below one half, plus 0.5 is 1.0, so a span whose left
//!    edge sits there starts at column 1. An edge from column 0 with slope 1/82 does that on row 41.) Details
//!    that matter: the lower end of an edge is entered twice unless it is the polygon's last row ("Needed to draw
//!    consistent polygons"); where the end of a sloped edge meets an earlier edge at the same rounded x and both
//!    run more than a pixel away on the adjacent row, the crossing moves to one pixel beside the nearer of them
//!    ("Connect discontiguous corners"); horizontal edges are drawn on their own, and a horizontal edge that
//!    continues the previous one in the same direction is merged into it; the scan runs to row `ysize` inclusive.
//!    A polygon needs at least two points; two points give a line, one repeated point a dot.
//! 3. **A blended fill blends each pixel once per polygon** (`polygon_generic`, the `hasAlpha` path with
//!    `draw_horizontal_lines`): spans are clipped against the position already drawn on the row, so a
//!    self-intersecting polygon does not darken where it overlaps itself.
//! 4. **A line of width 1** (`_draw_lines` then `ImagingDrawLine`, `line8`/`line32`/`line32rgba`) is Bresenham
//!    from vertex to vertex, each segment drawn without its last pixel, and then the last point of the whole
//!    polyline once. So a blended polyline blends a shared vertex once, but a pixel it passes twice is blended
//!    twice, and so is the first pixel of a closed loop (it is also the last point).
//! 5. **A wider line** (`ImagingDrawWideLine`) is one filled quadrilateral per segment, no joints and no caps: the
//!    corners are the truncated end points moved by rounded shares of `(width - 1) / 2` across the line (`hypot`
//!    and four `ROUND_DOWN`s in `double`), filled by the polygon filler without the horizontal-edge merge. A
//!    segment of zero length is one point. Blended, overlapping segments blend again.
//! 6. **An outline is drawn after the fill and only if its ink differs** from the fill's (`ImageDraw.polygon`,
//!    `ellipse`, `rectangle`: `if ink is not None and ink != fill_ink`). A polygon outline (`ImagingDrawPolygon`
//!    with `fill == 0`) is the Bresenham line of every edge including the closing one, each without its last
//!    pixel: a polygon whose points are all the same point has no outline at all.
//! 7. **The ellipse** (`ellipseNew`, `ellipse_init`, `ellipse_next`, `quarter_next`) is an integer algorithm on a
//!    grid of half pixels: for the box `x0, y0, x1, y1` (truncated) it walks a quarter of the ellipse with
//!    semi-axes `x1 - x0` and `y1 - y0` and emits horizontal spans. A filled ellipse is an outline of width
//!    `a + b`: so a filled ellipse whose box is a single point draws nothing, while its outline draws that point.
//! 8. **The rectangle** (`ImagingDrawRectangle`): filled, rows `y0` to `y1` inclusive, columns `x0` to `x1`
//!    inclusive. Outlined, two horizontal spans and two vertical Bresenham lines from `y0 + 1` towards `y1`; for a
//!    box of zero height those two lines run backwards and put a pixel on row `y0 + 1`, and blended the two spans
//!    blend the same row twice.
//! 9. **Inks per mode** (`_imaging.c`, `getink`): an 8-bit ink is stored as it is; an `F` ink is the Python float
//!    converted to `float32`; on an RGB image only the three colour bytes are visible. Through
//!    `Draw(rgb, 'RGBA')` each colour byte becomes `BLEND(alpha, pixel, ink)` (`ImagingUtils.h`):
//!    `t = pixel * (255 - alpha) + ink * alpha + 128; ((t >> 8) + t) >> 8`. Alpha 255 gives the ink, alpha 0
//!    leaves the pixel.
//! 10. **Coordinates no `int` holds** (NaN, an infinity, anything beyond 2^31). C leaves `(int)` of such a value
//!     undefined, and the overflowing sums that follow; the reference build gives the lowest `int` and wraps, and
//!     Pillow draws accordingly. The port does the same in every primitive, and `studio/reference/pillow/nonfinite`
//!     holds 201 such calls as Pillow drew them: `rectangle([nan, 2, 5, 9], fill)` fills from the left edge to
//!     column 5; `line([(5, 5), (nan, 7)])` paints row 5 from column 5 to the right edge; a polygon with a NaN
//!     vertex is the polygon with that vertex at the lowest `int`; an ellipse with a NaN corner draws nothing
//!     when its box wraps to a negative size or its fill width `a + b` wraps below 1, which covers every box of
//!     the form `[x - r, y - r, x + r, y + r]`. Pillow's loops over pixels far outside the image are not walked:
//!     the thin line (`line.rs`) and the ellipse (`ellipse.rs`) go straight to the part on the image, with the
//!     same pixels. Two bounds remain, both past anything Pillow finishes in good time: a thin line whose two
//!     differences both overflow and whose error term changes sign more than 65,536 times before the walk
//!     reaches the image draws nothing; and an ellipse is drawn as far as 2^21 steps of its walk get, which only
//!     one millions of pixels across can exceed: one whose 64-bit sums overflow (`a (b + 2)` above about
//!     3.04e9), or a tall one whose middle lies millions of rows from the image (Pillow: a second to never).
//!
//! Not ported, because the kit never calls them: `arc`, `chord`, `pieslice`, `rounded_rectangle`, `bitmap`,
//! `text`, `line(joint="curve")`, rectangle and polygon outlines wider than 1 px (Pillow draws the polygon's
//! through a mask), string colour names, and palette images.
//!
//! # How the port is laid out
//!
//! `Draw.c` has three sets of pixel writers (`draw8`, `draw32`, `draw32rgba`) behind one `DRAW` table. Here that
//! table is the crate-private `Surface` trait: an image bound to one ink, with `point` and `hline`. `Plain` is
//! `draw8` and `draw32` (copy the ink), `blend::Blended` is `draw32rgba`. Every algorithm (`polygon_generic`, the
//! Bresenham line, the wide line, the ellipse, the rectangle) is written once over `Surface`; the public functions
//! do what `ImageDraw.py` and `_imaging.c` do before calling C (truncate, order fill and outline) and pick the
//! surface.

use crate::img::Img;
use crate::np;

pub mod blend;
mod ellipse;
mod line;
mod point;
mod polygon;
mod rect;

pub use self::ellipse::ellipse;
pub use self::line::line;
pub use self::point::point;
pub use self::polygon::{polygon, polygon_outline};
pub use self::rect::rectangle;

mod sealed {
    pub trait Sealed {}
    impl Sealed for u8 {}
    impl Sealed for f32 {}
}

/// The sample types Pillow draws on: `u8` (modes `L`, `RGB`, `RGBA`) and `f32` (mode `F`).
pub trait Sample: Copy + sealed::Sealed {
    /// Whether Pillow has an image mode with `c` channels of this type.
    #[doc(hidden)]
    fn has_mode(c: usize) -> bool;
    /// Whether two samples are the same ink (the same bits): what `ink != fill_ink` compares in `ImageDraw.py`.
    #[doc(hidden)]
    fn same_ink(a: Self, b: Self) -> bool;
}

impl Sample for u8 {
    fn has_mode(c: usize) -> bool {
        matches!(c, 1 | 3 | 4)
    }

    fn same_ink(a: u8, b: u8) -> bool {
        a == b
    }
}

impl Sample for f32 {
    fn has_mode(c: usize) -> bool {
        c == 1
    }

    fn same_ink(a: f32, b: f32) -> bool {
        a.to_bits() == b.to_bits()
    }
}

/// `Draw.c`'s `DRAW` table bound to an image and an ink: the two pixel writers every primitive is built from.
/// Both clip to the image.
pub(crate) trait Surface {
    /// True for the blending writers (`draw32rgba`): `polygon_generic` then takes its `hasAlpha` path.
    const BLENDS: bool;
    /// `im->xsize`.
    fn xsize(&self) -> i32;
    /// `im->ysize`.
    fn ysize(&self) -> i32;
    /// `point8` / `point32` / `point32rgba`: one pixel, if it is inside the image.
    fn point(&mut self, x: i32, y: i32);
    /// `hline8` / `hline32` / `hline32rgba`: the pixels of row `y` from `x0` to `x1` inclusive, clipped.
    fn hline(&mut self, x0: i32, y: i32, x1: i32);
}

/// The columns `x0..=x1` of a row of `xsize` pixels after `hline*`'s clipping, or `None` if nothing is left.
pub(crate) fn clip_span(x0: i32, x1: i32, xsize: i32) -> Option<(usize, usize)> {
    if x0 >= xsize || x1 < 0 {
        return None;
    }
    let (x0, x1) = (x0.max(0), x1.min(xsize - 1));
    (x0 <= x1).then_some((x0 as usize, x1 as usize))
}

/// `draw8` and `draw32`: the ink replaces the pixel.
pub(crate) struct Plain<'a, T> {
    img: &'a mut Img<T>,
    ink: &'a [T],
}

impl<'a, T: Sample> Plain<'a, T> {
    /// Panics unless the image is one of Pillow's modes for `T` and the ink has one sample per channel.
    pub(crate) fn new(img: &'a mut Img<T>, ink: &'a [T]) -> Self {
        assert!(
            T::has_mode(img.c),
            "Pillow has no image mode with {} channel(s) of this sample type",
            img.c
        );
        assert_eq!(
            ink.len(),
            img.c,
            "the ink has {} sample(s), the image {} channel(s)",
            ink.len(),
            img.c
        );
        Plain { img, ink }
    }
}

impl<T: Sample> Surface for Plain<'_, T> {
    const BLENDS: bool = false;

    fn xsize(&self) -> i32 {
        self.img.w as i32
    }

    fn ysize(&self) -> i32 {
        self.img.h as i32
    }

    fn point(&mut self, x: i32, y: i32) {
        if x >= 0 && x < self.xsize() && y >= 0 && y < self.ysize() {
            self.img.px_mut(x as usize, y as usize).copy_from_slice(self.ink);
        }
    }

    fn hline(&mut self, x0: i32, y: i32, x1: i32) {
        if y < 0 || y >= self.ysize() {
            return;
        }
        if let Some((a, b)) = clip_span(x0, x1, self.xsize()) {
            let c = self.img.c;
            let row = self.img.row_mut(y as usize);
            for px in row[a * c..(b + 1) * c].chunks_exact_mut(c) {
                px.copy_from_slice(self.ink);
            }
        }
    }
}

/// Whether two plain inks are the same ink (`ink == fill_ink` in `ImageDraw.py`).
pub(crate) fn same_ink<T: Sample>(a: &[T], b: &[T]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(&x, &y)| T::same_ink(x, y))
}

/// `(int)x` on a C `double` as the kit's PCs do it: truncation toward zero.
pub(crate) fn c_int(x: f64) -> i32 {
    np::f64_to_i32(x)
}

/// Every point truncated to pixels: what `_imaging.c` hands to `Draw.c`.
pub(crate) fn truncated(pts: &[(f64, f64)]) -> Vec<(i32, i32)> {
    pts.iter().map(|&(x, y)| (c_int(x), c_int(y))).collect()
}

/// A box truncated to pixels, after the two checks `_draw_ellipse` and `_draw_rectangle` make on the floats:
/// `if (xy[2] < xy[0])` and `if (xy[3] < xy[1])` raise. Both are false when a corner is NaN, so a NaN box is not
/// refused: Pillow goes on to draw it with `(int)NaN`, the lowest `int`, and so does the port.
pub(crate) fn truncated_box(bbox: (f64, f64, f64, f64)) -> (i32, i32, i32, i32) {
    let (x0, y0, x1, y1) = bbox;
    if x1 < x0 {
        panic!("x1 must be greater than or equal to x0 (x0 {x0}, x1 {x1})");
    }
    if y1 < y0 {
        panic!("y1 must be greater than or equal to y0 (y0 {y0}, y1 {y1})");
    }
    (c_int(x0), c_int(y0), c_int(x1), c_int(y1))
}

/// `ROUND_UP` of `Draw.c` on a C `float`: half away from zero. For `f >= 0` the sum `f + 0.5F` is a `float` sum;
/// for `f < 0` `fabs` has already made it a `double`.
pub(crate) fn round_up_f32(f: f32) -> i32 {
    if f >= 0.0 {
        c_int(f64::from(f + 0.5f32).floor())
    } else {
        c_int(-(f64::from(f).abs() + 0.5).floor())
    }
}

/// `ROUND_DOWN` of `Draw.c` on a C `float`: half toward zero, with the same `float` / `double` split.
pub(crate) fn round_down_f32(f: f32) -> i32 {
    if f >= 0.0 {
        c_int(f64::from(f - 0.5f32).ceil())
    } else {
        c_int(-(f64::from(f).abs() - 0.5).ceil())
    }
}

/// `ROUND_UP` on a C `double`.
pub(crate) fn round_up_f64(f: f64) -> i32 {
    if f >= 0.0 {
        c_int((f + 0.5).floor())
    } else {
        c_int(-(f.abs() + 0.5).floor())
    }
}

/// `ROUND_DOWN` on a C `double`.
pub(crate) fn round_down_f64(f: f64) -> i32 {
    if f >= 0.0 {
        c_int((f - 0.5).ceil())
    } else {
        c_int(-(f.abs() - 0.5).ceil())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_macros() {
        // Half away from zero and half toward zero.
        assert_eq!((round_up_f32(2.5), round_down_f32(2.5)), (3, 2));
        assert_eq!((round_up_f32(-2.5), round_down_f32(-2.5)), (-3, -2));
        assert_eq!((round_up_f32(2.4), round_down_f32(2.6)), (2, 3));
        assert_eq!((round_up_f32(0.0), round_down_f32(0.0)), (0, 0));
        assert_eq!((round_up_f32(-0.0), round_down_f32(-0.0)), (0, 0));
        assert_eq!((round_up_f64(2.5), round_down_f64(2.5)), (3, 2));
        assert_eq!((round_up_f64(-2.5), round_down_f64(-2.5)), (-3, -2));
        assert_eq!((round_up_f64(1.5), round_down_f64(1.5)), (2, 1));
        // The float sum: the largest float below 0.5 plus 0.5 rounds to 1.0 in float, so ROUND_UP gives 1 where
        // the same value as a double gives 0.
        let just_below_half = f32::from_bits(0.5f32.to_bits() - 1);
        assert_eq!(round_up_f32(just_below_half), 1);
        assert_eq!(round_up_f64(f64::from(just_below_half)), 0);
        // The negative branch is a double sum: no such step.
        assert_eq!(round_up_f32(-just_below_half), 0);
    }

    #[test]
    fn spans_clip_like_hline() {
        assert_eq!(clip_span(-5, 3, 10), Some((0, 3)));
        assert_eq!(clip_span(4, 99, 10), Some((4, 9)));
        assert_eq!(clip_span(10, 12, 10), None);
        assert_eq!(clip_span(-9, -1, 10), None);
        assert_eq!(clip_span(5, 4, 10), None);
        assert_eq!(clip_span(0, 0, 10), Some((0, 0)));
    }

    #[test]
    fn plain_surface_writes_inside_only() {
        let mut im: Img<u8> = Img::new(4, 3, 3);
        let mut s = Plain::new(&mut im, &[1, 2, 3]);
        s.point(-1, 0);
        s.point(4, 0);
        s.point(0, 3);
        s.point(3, 2);
        s.hline(-2, 1, 1);
        s.hline(0, -1, 3);
        s.hline(0, 3, 3);
        assert_eq!(im.px(3, 2), &[1, 2, 3]);
        assert_eq!(im.row(1), &[1, 2, 3, 1, 2, 3, 0, 0, 0, 0, 0, 0]);
        assert_eq!(im.data.iter().filter(|&&v| v != 0).count(), 9);
    }

    #[test]
    #[should_panic(expected = "the ink has 4 sample(s), the image 3 channel(s)")]
    fn the_ink_must_fit_the_image() {
        let mut im: Img<u8> = Img::new(2, 2, 3);
        polygon(&mut im, &[(0.0, 0.0), (1.0, 1.0)], &[1, 2, 3, 4]);
    }

    #[test]
    #[should_panic(expected = "no image mode with 3 channel(s)")]
    fn float_images_have_one_channel() {
        let mut im: Img<f32> = Img::new(2, 2, 3);
        point(&mut im, &[(0.0, 0.0)], &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn inks_compare_by_bits() {
        assert!(same_ink(&[1u8, 2, 3], &[1, 2, 3]));
        assert!(!same_ink(&[1u8, 2, 3], &[1, 2, 4]));
        assert!(same_ink(&[f32::NAN], &[f32::NAN]));
        assert!(!same_ink(&[0.0f32], &[-0.0]));
    }
}
