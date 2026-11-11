//! Blended drawing: `ImageDraw.Draw(rgb_image, 'RGBA')`, the kit's way of drawing translucent shapes straight
//! into an RGB picture (`reelkit.fx`: particles, speed lines).
//!
//! The image is RGB (`c == 3`) and the ink is `[r, g, b, alpha]`. Every pixel the shape lands on becomes
//! `BLEND(alpha, pixel, ink)` per colour byte (`Draw.c`, `point32rgba` and `hline32rgba`; `ImagingUtils.h`), once
//! per time it is drawn:
//!
//! - one filled polygon blends each of its pixels once, however its edges cross;
//! - a width-1 polyline blends each shared vertex once, but a pixel it passes twice is blended twice;
//! - the segments of a wide line are separate quadrilaterals and blend again where they overlap;
//! - a point given twice is blended twice; so are the two spans of a flat rectangle outline.
//!
//! The functions mirror the plain ones of [`super`], with the same truncation and the same Pillow quirks.

use super::ellipse::{draw_ellipse, ellipse_width};
use super::line::line_on;
use super::point::draw_points;
use super::polygon::{fill_polygon, outline_polygon, polygon_points};
use super::rect::draw_rectangle;
use super::{Surface, clip_span, truncated_box};
use crate::img::Img;

/// `BLEND(mask, in1, in2, tmp)` of `ImagingUtils.h`: `in1 * (255 - mask) + in2 * mask`, divided by 255 with
/// rounding (`DIV255`: add 128, then `((t >> 8) + t) >> 8`).
fn blend_byte(alpha: u8, under: u8, ink: u8) -> u8 {
    let (alpha, under, ink) = (u32::from(alpha), u32::from(under), u32::from(ink));
    let t = under * (255 - alpha) + ink * alpha + 128;
    (((t >> 8) + t) >> 8) as u8
}

/// `draw32rgba`: the ink is blended into an RGB image by its alpha.
pub(crate) struct Blended<'a> {
    img: &'a mut Img<u8>,
    ink: [u8; 4],
}

impl<'a> Blended<'a> {
    /// Panics unless the image is RGB: Pillow allows the `'RGBA'` drawing mode on RGB images only.
    pub(crate) fn new(img: &'a mut Img<u8>, ink: [u8; 4]) -> Self {
        assert_eq!(
            img.c, 3,
            "blended drawing needs an RGB image, this one has {} channel(s)",
            img.c
        );
        Blended { img, ink }
    }

    fn blend_px(px: &mut [u8], ink: [u8; 4]) {
        px[0] = blend_byte(ink[3], px[0], ink[0]);
        px[1] = blend_byte(ink[3], px[1], ink[1]);
        px[2] = blend_byte(ink[3], px[2], ink[2]);
    }
}

impl Surface for Blended<'_> {
    const BLENDS: bool = true;

    fn xsize(&self) -> i32 {
        self.img.w as i32
    }

    fn ysize(&self) -> i32 {
        self.img.h as i32
    }

    fn point(&mut self, x: i32, y: i32) {
        if x >= 0 && x < self.xsize() && y >= 0 && y < self.ysize() {
            Self::blend_px(self.img.px_mut(x as usize, y as usize), self.ink);
        }
    }

    fn hline(&mut self, x0: i32, y: i32, x1: i32) {
        if y < 0 || y >= self.ysize() {
            return;
        }
        if let Some((a, b)) = clip_span(x0, x1, self.xsize()) {
            let ink = self.ink;
            let row = self.img.row_mut(y as usize);
            for px in row[a * 3..(b + 1) * 3].as_chunks_mut::<3>().0 {
                Self::blend_px(px, ink);
            }
        }
    }
}

/// `ImageDraw.Draw(rgb, 'RGBA').polygon(pts, fill=fill)`: a filled polygon blended into the picture. Each pixel
/// is blended once, even where the polygon crosses itself. Panics on fewer than two points.
pub fn polygon(img: &mut Img<u8>, pts: &[(f64, f64)], fill: [u8; 4]) {
    let xy = polygon_points(pts);
    fill_polygon(&mut Blended::new(img, fill), &xy);
}

/// `ImageDraw.Draw(rgb, 'RGBA').polygon(pts, fill=fill, outline=outline)` (and `outline` alone, with `fill`
/// `None`): the blended fill, then a blended 1 px outline unless its ink is the fill's ink.
pub fn polygon_outline(img: &mut Img<u8>, pts: &[(f64, f64)], fill: Option<[u8; 4]>, outline: [u8; 4]) {
    let xy = polygon_points(pts);
    if let Some(fill) = fill {
        fill_polygon(&mut Blended::new(img, fill), &xy);
        if fill == outline {
            return;
        }
    }
    outline_polygon(&mut Blended::new(img, outline), &xy);
}

/// `ImageDraw.Draw(rgb, 'RGBA').line(pts, fill=fill, width=width)`: a blended polyline; pass width 1 when the
/// Python call has no `width=`. See [`super::line()`].
pub fn line(img: &mut Img<u8>, pts: &[(f64, f64)], fill: [u8; 4], width: u32) {
    line_on(&mut Blended::new(img, fill), pts, width);
}

/// `ImageDraw.Draw(rgb, 'RGBA').ellipse([x0, y0, x1, y1], fill=fill, outline=outline, width=width)`: a blended
/// ellipse. See [`super::ellipse()`].
pub fn ellipse(
    img: &mut Img<u8>,
    bbox: (f64, f64, f64, f64),
    fill: Option<[u8; 4]>,
    outline: Option<[u8; 4]>,
    width: u32,
) {
    let (x0, y0, x1, y1) = truncated_box(bbox);
    if let Some(fill) = fill {
        draw_ellipse(&mut Blended::new(img, fill), x0, y0, x1, y1, true, 0);
    }
    if let Some(outline) = outline
        && fill != Some(outline)
        && width != 0
    {
        draw_ellipse(
            &mut Blended::new(img, outline),
            x0,
            y0,
            x1,
            y1,
            false,
            ellipse_width(width),
        );
    }
}

/// `ImageDraw.Draw(rgb, 'RGBA').rectangle([x0, y0, x1, y1], fill=fill, outline=outline)`: a blended box. See
/// [`super::rectangle`].
pub fn rectangle(img: &mut Img<u8>, bbox: (f64, f64, f64, f64), fill: Option<[u8; 4]>, outline: Option<[u8; 4]>) {
    let (x0, y0, x1, y1) = truncated_box(bbox);
    if let Some(fill) = fill {
        draw_rectangle(&mut Blended::new(img, fill), x0, y0, x1, y1, true, 0);
    }
    if let Some(outline) = outline
        && fill != Some(outline)
    {
        draw_rectangle(&mut Blended::new(img, outline), x0, y0, x1, y1, false, 1);
    }
}

/// `ImageDraw.Draw(rgb, 'RGBA').point(pts, fill=fill)`: each point's pixel blended with the ink; a point listed
/// twice is blended twice.
pub fn point(img: &mut Img<u8>, pts: &[(f64, f64)], fill: [u8; 4]) {
    draw_points(&mut Blended::new(img, fill), pts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_byte_is_pillows_div255() {
        // Alpha 255 gives the ink, alpha 0 keeps the pixel.
        for (under, ink) in [(0u8, 255u8), (255, 0), (17, 200), (128, 128)] {
            assert_eq!(blend_byte(255, under, ink), ink);
            assert_eq!(blend_byte(0, under, ink), under);
        }
        // (10 * 127 + 200 * 128 + 128) = 26998; ((26998 >> 8) + 26998) >> 8 = 105.
        assert_eq!(blend_byte(128, 10, 200), 105);
        // The rounded division by 255, checked against exact arithmetic over every input.
        for alpha in 0..=255u32 {
            for under in (0..=255u32).step_by(5) {
                for ink in (0..=255u32).step_by(3) {
                    let exact = (under * (255 - alpha) + ink * alpha + 127) / 255;
                    assert_eq!(u32::from(blend_byte(alpha as u8, under as u8, ink as u8)), exact);
                }
            }
        }
    }

    #[test]
    fn a_point_given_twice_blends_twice() {
        let mut once: Img<u8> = Img::filled(2, 1, &[100, 100, 100]);
        let mut twice = once.clone();
        point(&mut once, &[(0.5, 0.5)], [200, 0, 100, 128]);
        point(&mut twice, &[(0.5, 0.5), (0.9, 0.1)], [200, 0, 100, 128]);
        assert_eq!(once.px(0, 0), &[150, 50, 100]);
        assert_eq!(twice.px(0, 0), &[175, 25, 100]);
        assert_eq!(once.px(1, 0), &[100, 100, 100]);
    }

    #[test]
    fn a_self_crossing_polygon_blends_each_pixel_once() {
        // A bow tie: the two triangles share the centre pixel.
        let mut im: Img<u8> = Img::filled(9, 9, &[0, 0, 0]);
        polygon(
            &mut im,
            &[(0.0, 0.0), (8.0, 8.0), (8.0, 0.0), (0.0, 8.0)],
            [255, 255, 255, 128],
        );
        assert!(im.data.iter().all(|&v| v == 0 || v == 128), "{:?}", im.data);
        assert_eq!(im.px(4, 4), &[128, 128, 128]);
    }

    #[test]
    fn a_polyline_blends_shared_vertices_once() {
        let mut im: Img<u8> = Img::filled(4, 4, &[0, 0, 0]);
        line(&mut im, &[(0.0, 0.0), (3.0, 0.0), (3.0, 3.0)], [255, 255, 255, 128], 1);
        assert_eq!(im.px(3, 0), &[128, 128, 128]);
        assert_eq!(im.px(3, 3), &[128, 128, 128]);
        assert_eq!(im.px(0, 0), &[128, 128, 128]);
    }

    #[test]
    #[should_panic(expected = "blended drawing needs an RGB image")]
    fn only_rgb_images_blend() {
        let mut im: Img<u8> = Img::new(2, 2, 4);
        point(&mut im, &[(0.0, 0.0)], [1, 2, 3, 4]);
    }
}
