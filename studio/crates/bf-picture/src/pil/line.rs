//! `ImageDraw.line`: the Bresenham line of `Draw.c` (`line8`, `line32`, `line32rgba`), the wide line
//! (`ImagingDrawWideLine`) and the polyline loop of `_imaging.c` (`_draw_lines`).

use super::polygon::{Edge, polygon_generic};
use super::{Plain, Sample, Surface, c_int, round_down_f64, round_up_f64};
use crate::img::Img;
use bf_core::dmath;

/// `line8` / `line32` / `line32rgba`: the pixels from `(x0, y0)` towards `(x1, y1)`, WITHOUT the last one. (The
/// caller draws the end point of a polyline; a polygon outline never needs it.)
pub(crate) fn draw_line<S: Surface>(s: &mut S, x0: i32, y0: i32, x1: i32, y1: i32) {
    let (mut x0, mut y0) = (x0, y0);
    // The differences and the error term are 64-bit so that absurd coordinates cannot overflow; for every line
    // that fits a C int they hold the same values as Pillow's.
    // normalize coordinates
    let mut dx = i64::from(x1) - i64::from(x0);
    let xs = if dx < 0 {
        dx = -dx;
        -1
    } else {
        1
    };
    let mut dy = i64::from(y1) - i64::from(y0);
    let ys = if dy < 0 {
        dy = -dy;
        -1
    } else {
        1
    };

    if dx == 0 {
        // vertical
        for _ in 0..dy {
            s.point(x0, y0);
            y0 = y0.wrapping_add(ys);
        }
    } else if dy == 0 {
        // horizontal
        for _ in 0..dx {
            s.point(x0, y0);
            x0 = x0.wrapping_add(xs);
        }
    } else if dx > dy {
        // bresenham, horizontal slope
        let n = dx;
        dy += dy;
        let mut e = dy - dx;
        dx += dx;
        for _ in 0..n {
            s.point(x0, y0);
            if e >= 0 {
                y0 = y0.wrapping_add(ys);
                e -= dx;
            }
            e += dy;
            x0 = x0.wrapping_add(xs);
        }
    } else {
        // bresenham, vertical slope
        let n = dy;
        dx += dx;
        let mut e = dx - dy;
        dy += dy;
        for _ in 0..n {
            s.point(x0, y0);
            if e >= 0 {
                x0 = x0.wrapping_add(xs);
                e -= dy;
            }
            e += dx;
            y0 = y0.wrapping_add(ys);
        }
    }
}

/// `ImagingDrawWideLine`: one segment of a line wider than 1 px, as a filled quadrilateral around it.
pub(crate) fn draw_wide_line<S: Surface>(s: &mut S, x0: i32, y0: i32, x1: i32, y1: i32, width: i32) {
    let dx = x1.wrapping_sub(x0);
    let dy = y1.wrapping_sub(y0);
    if dx == 0 && dy == 0 {
        s.point(x0, y0);
        return;
    }

    let big_hypotenuse = dmath::hypot(f64::from(dx), f64::from(dy));
    let small_hypotenuse = f64::from(width - 1) / 2.0;
    let ratio_max = f64::from(round_up_f64(small_hypotenuse)) / big_hypotenuse;
    let ratio_min = f64::from(round_down_f64(small_hypotenuse)) / big_hypotenuse;

    let dxmin = round_down_f64(ratio_min * f64::from(dy));
    let dxmax = round_down_f64(ratio_max * f64::from(dy));
    let dymin = round_down_f64(ratio_min * f64::from(dx));
    let dymax = round_down_f64(ratio_max * f64::from(dx));
    let vertices = [
        (x0.wrapping_sub(dxmin), y0.wrapping_add(dymax)),
        (x1.wrapping_sub(dxmin), y1.wrapping_add(dymax)),
        (x1.wrapping_add(dxmax), y1.wrapping_sub(dymin)),
        (x0.wrapping_add(dxmax), y0.wrapping_sub(dymin)),
    ];
    let edge = |a: usize, b: usize| Edge::new(vertices[a].0, vertices[a].1, vertices[b].0, vertices[b].1);
    polygon_generic(s, &[edge(0, 1), edge(1, 2), edge(2, 3), edge(3, 0)]);
}

/// `_draw_lines`: the polyline through `pts`, each coordinate truncated to a C `int` on the way.
pub(crate) fn draw_lines<S: Surface>(s: &mut S, pts: &[(f64, f64)], width: i32) {
    if width == 1 {
        for p in pts.windows(2) {
            draw_line(s, c_int(p[0].0), c_int(p[0].1), c_int(p[1].0), c_int(p[1].1));
        }
        if let [.., last] = pts
            && pts.len() >= 2
        {
            // draw last point
            s.point(c_int(last.0), c_int(last.1));
        }
    } else {
        for p in pts.windows(2) {
            draw_wide_line(s, c_int(p[0].0), c_int(p[0].1), c_int(p[1].0), c_int(p[1].1), width);
        }
    }
}

/// `ImageDraw.line`'s own part: nothing at all for width 0, else [`draw_lines`].
pub(crate) fn line_on<S: Surface>(s: &mut S, pts: &[(f64, f64)], width: u32) {
    if width != 0 {
        draw_lines(s, pts, i32::try_from(width).expect("a line width fits a C int"));
    }
}

/// `ImageDraw.Draw(im).line(pts, fill=fill, width=width)`: the polyline through `pts`. Width 1 (what a call
/// without `width=` draws) is a Bresenham line through the truncated points; a wider line is one filled
/// quadrilateral per segment, with no joints. Width 0 draws nothing; fewer than two points draw nothing.
pub fn line<T: Sample>(img: &mut Img<T>, pts: &[(f64, f64)], fill: &[T], width: u32) {
    line_on(&mut Plain::new(img, fill), pts, width);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(im: &Img<u8>) -> Vec<String> {
        (0..im.h)
            .map(|y| im.row(y).iter().map(|&v| if v != 0 { '#' } else { '.' }).collect())
            .collect()
    }

    #[test]
    fn a_thin_line_reaches_both_ends() {
        let mut im: Img<u8> = Img::new(6, 4, 1);
        line(&mut im, &[(0.0, 0.0), (5.0, 2.0)], &[1], 1);
        assert_eq!(rows(&im), ["##....", "..##..", "....##", "......"]);
    }

    /// A surface that counts how often each pixel is written.
    struct Counter {
        w: i32,
        h: i32,
        hits: Vec<u32>,
    }

    impl Surface for Counter {
        const BLENDS: bool = false;

        fn xsize(&self) -> i32 {
            self.w
        }

        fn ysize(&self) -> i32 {
            self.h
        }

        fn point(&mut self, x: i32, y: i32) {
            if x >= 0 && x < self.w && y >= 0 && y < self.h {
                self.hits[(y * self.w + x) as usize] += 1;
            }
        }

        fn hline(&mut self, x0: i32, y: i32, x1: i32) {
            for x in x0..=x1 {
                self.point(x, y);
            }
        }
    }

    #[test]
    fn a_polyline_draws_each_vertex_once() {
        let mut im: Img<u8> = Img::new(5, 5, 1);
        let pts = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
        line(&mut im, &pts, &[7], 1);
        assert_eq!(rows(&im), ["#####", "....#", "....#", "....#", "#####"]);
        // Each segment stops before its end and the last point is drawn once: no pixel is written twice.
        let mut count = Counter {
            w: 5,
            h: 5,
            hits: vec![0; 25],
        };
        draw_lines(&mut count, &pts, 1);
        assert_eq!(count.hits.iter().sum::<u32>(), 13);
        assert!(count.hits.iter().all(|&n| n <= 1), "{:?}", count.hits);
        // A closed loop writes its first pixel twice: once as the start, once as the last point.
        let mut closed = Counter {
            w: 5,
            h: 5,
            hits: vec![0; 25],
        };
        draw_lines(&mut closed, &[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 0.0)], 1);
        assert_eq!(closed.hits[0], 2);
        assert_eq!(closed.hits.iter().filter(|&&n| n > 1).count(), 1);
    }

    #[test]
    fn a_zero_length_line_is_one_pixel_at_any_width() {
        for width in [1, 2, 5] {
            let mut im: Img<u8> = Img::new(5, 5, 1);
            line(&mut im, &[(2.7, 2.2), (2.1, 2.9)], &[1], width);
            assert_eq!(
                rows(&im),
                [".....", ".....", "..#..", ".....", "....."],
                "width {width}"
            );
        }
    }

    #[test]
    fn width_zero_and_single_points_draw_nothing() {
        let mut im: Img<u8> = Img::new(5, 5, 1);
        line(&mut im, &[(0.0, 0.0), (4.0, 4.0)], &[1], 0);
        line(&mut im, &[(2.0, 2.0)], &[1], 1);
        line(&mut im, &[(2.0, 2.0)], &[1], 3);
        line(&mut im, &[], &[1], 1);
        assert!(im.data.iter().all(|&v| v == 0));
    }

    #[test]
    fn a_wide_horizontal_line_is_a_band() {
        let mut im: Img<u8> = Img::new(7, 6, 1);
        line(&mut im, &[(1.0, 2.0), (5.0, 2.0)], &[1], 3);
        assert_eq!(
            rows(&im),
            [".......", ".#####.", ".#####.", ".#####.", ".......", "......."]
        );
    }
}
