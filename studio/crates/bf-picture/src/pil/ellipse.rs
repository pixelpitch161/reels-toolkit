//! `ImageDraw.ellipse`: the integer ellipse of `Draw.c` (`quarter_*`, `ellipse_*`, `ellipseNew`).
//!
//! `Draw.c` works on a grid of half pixels: for a box `a` pixels wide and `b` high it walks the top right quarter
//! of the ellipse with semi-axes `a` and `b` over the points `(2x + a % 2, 2y + b % 2)`, always stepping to the
//! neighbour that lies closest to the curve, and mirrors it into horizontal spans. An outline of width `w` is the
//! band between that ellipse and the one `2 (w - 1)` smaller; a fill is an outline of width `a + b`.

use super::{Plain, Sample, Surface, same_ink, truncated_box};
use crate::img::Img;

/// `quarter_state`: the walk along one quarter of an ellipse, from `(a, b % 2)` up to `(a % 2, b)`.
#[derive(Clone, Copy, Debug, Default)]
struct Quarter {
    cx: i32,
    cy: i32,
    ex: i32,
    ey: i32,
    a2: i64,
    b2: i64,
    a2b2: i64,
    finished: bool,
}

impl Quarter {
    /// `quarter_init`.
    fn new(a: i32, b: i32) -> Quarter {
        if a < 0 || b < 0 {
            return Quarter {
                finished: true,
                ..Quarter::default()
            };
        }
        let a2 = i64::from(a) * i64::from(a);
        let b2 = i64::from(b) * i64::from(b);
        Quarter {
            cx: a,
            cy: b % 2,
            ex: a % 2,
            ey: b,
            a2,
            b2,
            a2b2: a2.wrapping_mul(b2),
            finished: false,
        }
    }

    /// `quarter_delta`: how far the point is from the curve (the point put into the ellipse equation).
    fn delta(&self, x: i64, y: i64) -> i64 {
        self.a2
            .wrapping_mul(y)
            .wrapping_mul(y)
            .wrapping_add(self.b2.wrapping_mul(x).wrapping_mul(x))
            .wrapping_sub(self.a2b2)
            .wrapping_abs()
    }

    /// `quarter_next`: the current point, then a step to the next; `None` once the walk is over.
    fn next(&mut self) -> Option<(i32, i32)> {
        if self.finished {
            return None;
        }
        let ret = (self.cx, self.cy);
        if self.cx == self.ex && self.cy == self.ey {
            self.finished = true;
        } else {
            // Bresenham's algorithm, possible optimization: only consider 2 of 3 next points depending on
            // current slope
            let mut nx = self.cx;
            let mut ny = self.cy + 2;
            let mut ndelta = self.delta(i64::from(nx), i64::from(ny));
            if nx > 1 {
                let newdelta = self.delta(i64::from(self.cx - 2), i64::from(self.cy + 2));
                if ndelta > newdelta {
                    nx = self.cx - 2;
                    ny = self.cy + 2;
                    ndelta = newdelta;
                }
                let newdelta = self.delta(i64::from(self.cx - 2), i64::from(self.cy));
                if ndelta > newdelta {
                    nx = self.cx - 2;
                    ny = self.cy;
                }
            }
            self.cx = nx;
            self.cy = ny;
        }
        Some(ret)
    }
}

/// `ellipse_state`: joins the four quarters of an outer and an inner ellipse into horizontal spans
/// `(x0, y, x1)`, still on the grid of half pixels.
struct EllipseState {
    st_o: Quarter,
    st_i: Quarter,
    py: i32,
    pl: i32,
    pr: i32,
    cy: [i32; 4],
    cl: [i32; 4],
    cr: [i32; 4],
    bufcnt: usize,
    finished: bool,
    leftmost: i32,
}

impl EllipseState {
    /// `ellipse_init`.
    fn new(a: i32, b: i32, w: i32) -> EllipseState {
        let mut s = EllipseState {
            st_o: Quarter::new(a, b),
            st_i: Quarter::default(),
            py: 0,
            pl: 0,
            pr: 0,
            cy: [0; 4],
            cl: [0; 4],
            cr: [0; 4],
            bufcnt: 0,
            finished: true,
            leftmost: a % 2,
        };
        if w < 1 {
            return s;
        }
        if let Some((pr, py)) = s.st_o.next() {
            s.pr = pr;
            s.py = py;
            s.finished = false;
            s.st_i = Quarter::new(
                a.wrapping_sub(2i32.wrapping_mul(w - 1)),
                b.wrapping_sub(2i32.wrapping_mul(w - 1)),
            );
            s.pl = s.leftmost;
        }
        s
    }

    /// `ellipse_next`: the next span, or `None` when the ellipse is complete.
    fn next(&mut self) -> Option<(i32, i32, i32)> {
        if self.bufcnt == 0 {
            if self.finished {
                return None;
            }
            let y = self.py;
            let mut l = self.pl;
            let r = self.pr;
            // while ((next_ret = quarter_next(&s->st_o, &cx, &cy)) != -1 && cy <= y) {}
            let mut outer = None;
            while let Some((cx, cy)) = self.st_o.next() {
                if cy > y {
                    outer = Some((cx, cy));
                    break;
                }
            }
            match outer {
                None => self.finished = true,
                Some((cx, cy)) => {
                    self.pr = cx;
                    self.py = cy;
                }
            }
            // while ((next_ret = quarter_next(&s->st_i, &cx, &cy)) != -1 && cy <= y) { l = cx; }
            let mut inner = None;
            while let Some((cx, cy)) = self.st_i.next() {
                if cy > y {
                    inner = Some(cx);
                    break;
                }
                l = cx;
            }
            self.pl = inner.unwrap_or(self.leftmost);

            if (l > 0 || l < r) && y > 0 {
                self.push(if l == 0 { 2 } else { l }, y, r);
            }
            if y > 0 {
                self.push(-r, y, -l);
            }
            if l > 0 || l < r {
                self.push(if l == 0 { 2 } else { l }, -y, r);
            }
            self.push(-r, -y, -l);
        }
        self.bufcnt -= 1;
        Some((self.cl[self.bufcnt], self.cy[self.bufcnt], self.cr[self.bufcnt]))
    }

    fn push(&mut self, l: i32, y: i32, r: i32) {
        self.cl[self.bufcnt] = l;
        self.cy[self.bufcnt] = y;
        self.cr[self.bufcnt] = r;
        self.bufcnt += 1;
    }
}

/// `ellipseNew`: the ellipse inscribed in the pixel box `x0, y0, x1, y1`, filled or as an outline `width` thick.
pub(crate) fn draw_ellipse<S: Surface>(s: &mut S, x0: i32, y0: i32, x1: i32, y1: i32, fill: bool, width: i32) {
    let a = x1.wrapping_sub(x0);
    let b = y1.wrapping_sub(y0);
    if a < 0 || b < 0 {
        return;
    }
    let width = if fill { a.wrapping_add(b) } else { width };
    let mut st = EllipseState::new(a, b, width);
    while let Some((sx0, sy, sx1)) = st.next() {
        s.hline(
            x0.wrapping_add((sx0 + a) / 2),
            y0.wrapping_add((sy + b) / 2),
            x0.wrapping_add((sx1 + a) / 2),
        );
    }
}

/// An outline width as the C `int` `draw_ellipse` takes.
pub(crate) fn ellipse_width(width: u32) -> i32 {
    i32::try_from(width).expect("an outline width fits a C int")
}

/// `ImageDraw.Draw(im).ellipse([x0, y0, x1, y1], fill=fill, outline=outline, width=width)`: the ellipse inscribed
/// in the box, whose corners are truncated to pixels and both included. The fill is drawn first; the outline
/// (`width` pixels thick, Pillow's default being 1) follows unless its ink is the fill's ink or `width` is 0.
/// Panics if `x1 < x0` or `y1 < y0`, as Pillow raises.
pub fn ellipse<T: Sample>(
    img: &mut Img<T>,
    bbox: (f64, f64, f64, f64),
    fill: Option<&[T]>,
    outline: Option<&[T]>,
    width: u32,
) {
    let (x0, y0, x1, y1) = truncated_box(bbox);
    if let Some(fill) = fill {
        draw_ellipse(&mut Plain::new(img, fill), x0, y0, x1, y1, true, 0);
    }
    if let Some(outline) = outline
        && !fill.is_some_and(|f| same_ink(f, outline))
        && width != 0
    {
        draw_ellipse(
            &mut Plain::new(img, outline),
            x0,
            y0,
            x1,
            y1,
            false,
            ellipse_width(width),
        );
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
    fn a_small_disc() {
        let mut im: Img<u8> = Img::new(7, 7, 1);
        ellipse(&mut im, (1.0, 1.0, 5.0, 5.0), Some(&[1]), None, 1);
        assert_eq!(
            rows(&im),
            [
                ".......", "..###..", ".#####.", ".#####.", ".#####.", "..###..", "......."
            ]
        );
    }

    #[test]
    fn an_outline_and_a_fill_with_outline() {
        let mut im: Img<u8> = Img::new(7, 7, 1);
        ellipse(&mut im, (1.0, 1.0, 5.0, 5.0), None, Some(&[2]), 1);
        assert_eq!(
            rows(&im),
            [
                ".......", "..ooo..", ".o...o.", ".o...o.", ".o...o.", "..ooo..", "......."
            ]
        );
        let mut both: Img<u8> = Img::new(7, 7, 1);
        ellipse(&mut both, (1.0, 1.0, 5.0, 5.0), Some(&[1]), Some(&[2]), 1);
        assert_eq!(
            rows(&both),
            [
                ".......", "..ooo..", ".o###o.", ".o###o.", ".o###o.", "..ooo..", "......."
            ]
        );
    }

    #[test]
    fn a_point_box_fills_nothing_but_outlines_a_pixel() {
        let mut im: Img<u8> = Img::new(3, 3, 1);
        ellipse(&mut im, (1.2, 1.7, 1.9, 1.8), Some(&[1]), None, 1);
        assert!(im.data.iter().all(|&v| v == 0));
        ellipse(&mut im, (1.2, 1.7, 1.9, 1.8), None, Some(&[1]), 1);
        assert_eq!(rows(&im), ["...", ".#.", "..."]);
    }

    #[test]
    fn the_same_ink_or_width_zero_skips_the_outline() {
        let mut a: Img<u8> = Img::new(9, 9, 3);
        let mut b = a.clone();
        let mut c = a.clone();
        ellipse(&mut a, (1.0, 1.0, 7.0, 6.0), Some(&[5, 6, 7]), None, 1);
        ellipse(&mut b, (1.0, 1.0, 7.0, 6.0), Some(&[5, 6, 7]), Some(&[5, 6, 7]), 1);
        ellipse(&mut c, (1.0, 1.0, 7.0, 6.0), Some(&[5, 6, 7]), Some(&[9, 9, 9]), 0);
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    #[should_panic(expected = "x1 must be greater than or equal to x0")]
    fn a_backwards_box_is_refused() {
        let mut im: Img<u8> = Img::new(3, 3, 1);
        ellipse(&mut im, (2.0, 0.0, 1.0, 1.0), Some(&[1]), None, 1);
    }
}
