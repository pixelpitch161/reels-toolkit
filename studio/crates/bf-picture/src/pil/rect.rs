//! `ImageDraw.rectangle`: `ImagingDrawRectangle` of `Draw.c`.

use super::line::draw_line;
use super::{Plain, Sample, Surface, same_ink, truncated_box};
use crate::img::Img;

/// `ImagingDrawRectangle`: the pixel box `x0, y0, x1, y1` (both corners included), filled or as an outline
/// `width` thick.
pub(crate) fn draw_rectangle<S: Surface>(s: &mut S, x0: i32, y0: i32, x1: i32, y1: i32, fill: bool, width: i32) {
    let (mut y0, mut y1) = (y0, y1);
    if y0 > y1 {
        std::mem::swap(&mut y0, &mut y1);
    }

    if fill {
        let ysize = s.ysize();
        if y0 < 0 {
            y0 = 0;
        } else if y0 >= ysize {
            return;
        }
        if y1 < 0 {
            return;
        } else if y1 > ysize {
            y1 = ysize;
        }
        for y in y0..=y1 {
            s.hline(x0, y, x1);
        }
    } else {
        // outline
        let width = if width == 0 { 1 } else { width };
        for i in 0..width {
            s.hline(x0, y0.wrapping_add(i), x1);
            s.hline(x0, y1.wrapping_sub(i), x1);
            let (top, bottom) = (y0.wrapping_add(width), y1.wrapping_sub(width).wrapping_add(1));
            draw_line(s, x1.wrapping_sub(i), top, x1.wrapping_sub(i), bottom);
            draw_line(s, x0.wrapping_add(i), top, x0.wrapping_add(i), bottom);
        }
    }
}

/// `ImageDraw.Draw(im).rectangle([x0, y0, x1, y1], fill=fill, outline=outline)`: the box whose corners are
/// truncated to pixels and both included. The fill is drawn first; the 1 px outline follows unless its ink is the
/// fill's ink. Panics if `x1 < x0` or `y1 < y0`, as Pillow raises.
pub fn rectangle<T: Sample>(img: &mut Img<T>, bbox: (f64, f64, f64, f64), fill: Option<&[T]>, outline: Option<&[T]>) {
    let (x0, y0, x1, y1) = truncated_box(bbox);
    if let Some(fill) = fill {
        draw_rectangle(&mut Plain::new(img, fill), x0, y0, x1, y1, true, 0);
    }
    if let Some(outline) = outline
        && !fill.is_some_and(|f| same_ink(f, outline))
    {
        draw_rectangle(&mut Plain::new(img, outline), x0, y0, x1, y1, false, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(im: &Img<u8>) -> Vec<String> {
        (0..im.h)
            .map(|y| {
                im.row(y)
                    .iter()
                    .map(|&v| match v {
                        0 => '.',
                        1 => '#',
                        _ => 'o',
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn both_corners_are_inside() {
        let mut im: Img<u8> = Img::new(6, 5, 1);
        rectangle(&mut im, (1.9, 1.1, 4.2, 3.8), Some(&[1]), None);
        assert_eq!(rows(&im), ["......", ".####.", ".####.", ".####.", "......"]);
    }

    #[test]
    fn an_outline_goes_around_the_fill() {
        let mut im: Img<u8> = Img::new(6, 5, 1);
        rectangle(&mut im, (1.0, 0.0, 4.0, 4.0), Some(&[1]), Some(&[2]));
        assert_eq!(rows(&im), [".oooo.", ".o##o.", ".o##o.", ".o##o.", ".oooo."]);
    }

    #[test]
    fn a_flat_outline_spills_one_row_down() {
        // y0 == y1: the two side lines run from y0 + 1 back to y0 and leave a pixel on row y0 + 1.
        let mut im: Img<u8> = Img::new(6, 4, 1);
        rectangle(&mut im, (1.0, 1.0, 4.0, 1.0), None, Some(&[2]));
        assert_eq!(rows(&im), ["......", ".oooo.", ".o..o.", "......"]);
    }

    #[test]
    fn clipped_at_the_image_border() {
        let mut im: Img<u8> = Img::new(4, 3, 1);
        rectangle(&mut im, (-5.0, -5.0, 1.0, 9.0), Some(&[1]), None);
        assert_eq!(rows(&im), ["##..", "##..", "##.."]);
        let mut out: Img<u8> = Img::new(4, 3, 1);
        rectangle(&mut out, (0.0, 3.0, 3.0, 9.0), Some(&[1]), None);
        rectangle(&mut out, (0.0, -9.0, 3.0, -1.0), Some(&[1]), None);
        assert!(out.data.iter().all(|&v| v == 0));
    }

    #[test]
    #[should_panic(expected = "y1 must be greater than or equal to y0")]
    fn an_upside_down_box_is_refused() {
        let mut im: Img<u8> = Img::new(4, 3, 1);
        rectangle(&mut im, (0.0, 2.0, 3.0, 1.0), Some(&[1]), None);
    }

    /// A NaN corner is not refused (Pillow's `xy[2] < xy[0]` is false for it): it is the lowest `int`, and the
    /// rest of the box is drawn. These are the pictures Pillow 12.3.0 gives on a 16 x 12 `L` image (taken on the
    /// reference PC; the outlines with a NaN row take Pillow 5.4 s each, for the side lines' walk).
    #[test]
    fn a_nan_corner_draws_what_pillow_draws() {
        let (nan, inf) = (f64::NAN, f64::INFINITY);
        let draw = |bbox: (f64, f64, f64, f64), fill: bool| {
            let mut im: Img<u8> = Img::new(16, 12, 1);
            let started = std::time::Instant::now();
            if fill {
                rectangle(&mut im, bbox, Some(&[1]), None);
            } else {
                rectangle(&mut im, bbox, None, Some(&[1]));
            }
            assert!(started.elapsed().as_millis() < 100, "{bbox:?}: {:?}", started.elapsed());
            rows(&im).join("|")
        };
        let blank = "................";
        let picture = |lines: &[(usize, &str)]| {
            let mut out = [blank; 12];
            for &(y, row) in lines {
                out[y] = row;
            }
            out.join("|")
        };
        let span = |from: usize, to: usize, row: &'static str| (from..=to).map(|y| (y, row)).collect::<Vec<_>>();

        // Filled. x0 NaN: from the left edge. x1 NaN: nothing. y0 NaN: from the top. y1 NaN: the rows above y0.
        assert_eq!(
            draw((nan, 2.0, 5.0, 9.0), true),
            picture(&span(2, 9, "######.........."))
        );
        assert_eq!(draw((5.0, 2.0, nan, 9.0), true), picture(&[]));
        assert_eq!(
            draw((5.0, nan, 9.0, 9.0), true),
            picture(&span(0, 9, ".....#####......"))
        );
        assert_eq!(
            draw((5.0, 2.0, 9.0, nan), true),
            picture(&span(0, 2, ".....#####......"))
        );
        assert_eq!(draw((nan, nan, nan, nan), true), picture(&[]));
        assert_eq!(draw((5.0, 2.0, inf, 9.0), true), picture(&[]));
        assert_eq!(
            draw((5.0, 2.0, 9.0, inf), true),
            picture(&span(0, 2, ".....#####......"))
        );
        assert_eq!(
            draw((-inf, 2.0, 9.0, 9.0), true),
            picture(&span(2, 9, "##########......"))
        );

        // Outlined.
        let mut left_open = span(3, 8, ".....#..........");
        left_open.extend([(2, "######.........."), (9, "######..........")]);
        assert_eq!(draw((nan, 2.0, 5.0, 9.0), false), picture(&left_open));
        assert_eq!(
            draw((5.0, 2.0, nan, 9.0), false),
            picture(&span(3, 8, ".....#.........."))
        );
        assert_eq!(
            draw((5.0, 2.0, inf, 9.0), false),
            picture(&span(3, 8, ".....#.........."))
        );
        let mut from_above = span(10, 11, ".....#...#......");
        from_above.push((9, ".....#####......"));
        assert_eq!(draw((5.0, nan, 9.0, 9.0), false), picture(&from_above));
        let mut to_the_bottom = span(3, 11, ".....#...#......");
        to_the_bottom.push((2, ".....#####......"));
        assert_eq!(draw((5.0, 2.0, 9.0, nan), false), picture(&to_the_bottom));
        assert_eq!(draw((5.0, 2.0, 9.0, inf), false), picture(&to_the_bottom));
        assert_eq!(draw((nan, nan, nan, nan), false), picture(&[]));
        let mut wide = span(3, 8, ".........#......");
        wide.extend([(2, "##########......"), (9, "##########......")]);
        assert_eq!(draw((-inf, 2.0, 9.0, 9.0), false), picture(&wide));
    }
}
