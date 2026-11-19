//! `ImageDraw.ellipse`: the integer ellipse of `Draw.c` (`quarter_*`, `ellipse_*`, `ellipseNew`).
//!
//! `Draw.c` works on a grid of half pixels: for a box `a` pixels wide and `b` high it walks the top right quarter
//! of the ellipse with semi-axes `a` and `b` over the points `(2x + a % 2, 2y + b % 2)`, always stepping to the
//! neighbour that lies closest to the curve, and mirrors it into horizontal spans. An outline of width `w` is the
//! band between that ellipse and the one `2 (w - 1)` smaller; a fill is an outline of width `a + b`.

use super::line::{steps_inside, wrap};
use super::{Plain, Sample, Surface, same_ink, truncated_box};
use crate::img::Img;

/// The most steps the two quarter walks of one ellipse may take together. An ellipse needs about
/// `(a + b) / 2` steps per walk, so nothing up to a million pixels across comes near this. Past it the
/// ellipse is left as far as it has been drawn: Pillow needs from seconds to for ever there (see
/// [`draw_ellipse`]).
const MAX_WALK_STEPS: u64 = 1 << 21;

/// A walk skips its long level runs (see [`Quarter::run_end`]) when the ellipse is wider than this. Below it the
/// runs are short and the plain walk is quicker.
const RUN_SKIP_ABOVE: i32 = 4096;

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
    /// Whether long runs to the left are taken in one step (not in `Draw.c`: see [`Quarter::run_end`]).
    skip: bool,
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
            skip: false,
        }
    }

    /// Where the run of steps to the left that starts here ends, if there is one to skip. Only for a walk whose
    /// arithmetic is exact (and `a > 0`). With `f(x, y) = a2 y^2 + b2 x^2 - a2 b2` and `L = (cx - 2, cy)`, the
    /// other two candidates lie at `f(L) + g` and more, `g = a2 (4 cy + 4) > 0`. `quarter_next` takes `L` when it
    /// is strictly the nearest of the three, which works out to `2 f(L) + g > 0`: for `f(L) >= 0` it always is the
    /// nearest; for `f(L) < 0` it is while `-f(L) < f(L) + g`. As an inequality in x that is
    /// `b2 (cx - 2)^2 > a2 (b2 - (cy + 1)^2 - 1)`, so the walk goes left exactly as long as the point two to the
    /// left is at or beyond the smallest `x` with `b2 x^2` above that bound, and it stops at the smallest such `x`
    /// of its parity. Nothing reads the points in between: the outer walk only wants the first point of each row,
    /// the inner one the last.
    fn run_end(&self) -> Option<i32> {
        if !self.skip || self.cx <= 3 {
            return None;
        }
        let (a2, b2, cy) = (i128::from(self.a2), i128::from(self.b2), i128::from(self.cy));
        let bound = a2 * (b2 - (cy + 1) * (cy + 1) - 1);
        // The smallest x >= 0 with b2 x^2 > bound.
        let edge = if bound < 0 || b2 == 0 {
            0
        } else {
            let squared = bound / b2;
            let mut root = (squared as f64).sqrt() as i128;
            while root * root > squared {
                root -= 1;
            }
            while (root + 1) * (root + 1) <= squared {
                root += 1;
            }
            (root + 1) as i64
        };
        let cx = i64::from(self.cx);
        let target = edge + ((cx - edge) & 1);
        (target <= cx - 2).then_some(target as i32)
    }

    /// `quarter_delta`: how far the point is from the curve (the point put into the ellipse equation). For a
    /// huge ellipse the 64-bit sums overflow; they wrap here as they do in the reference build.
    fn delta(&self, x: i32, y: i32) -> i64 {
        let (x, y) = (i64::from(x), i64::from(y));
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
        } else if let Some(target) = self.run_end() {
            self.cx = target;
        } else {
            // Bresenham's algorithm, possible optimization: only consider 2 of 3 next points depending on
            // current slope
            let (left, up) = (self.cx.wrapping_sub(2), self.cy.wrapping_add(2));
            let mut nx = self.cx;
            let mut ny = up;
            let mut ndelta = self.delta(nx, ny);
            if nx > 1 {
                let newdelta = self.delta(left, up);
                if ndelta > newdelta {
                    nx = left;
                    ny = up;
                    ndelta = newdelta;
                }
                let newdelta = self.delta(left, self.cy);
                if ndelta > newdelta {
                    nx = left;
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
    /// Steps taken so far by both walks (not in `Draw.c`: see [`MAX_WALK_STEPS`]).
    steps: u64,
}

impl EllipseState {
    /// `ellipse_init`. `skip_above` is not in `Draw.c`: when the arithmetic is exact, the width above which a
    /// walk skips its level runs.
    fn new(a: i32, b: i32, w: i32, skip_above: Option<i32>) -> EllipseState {
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
            steps: 0,
        };
        if w < 1 {
            return s;
        }
        s.st_o.skip = skip_above.is_some_and(|limit| a > limit);
        if let Some((pr, py)) = s.st_o.next() {
            s.pr = pr;
            s.py = py;
            s.finished = false;
            let inner_a = a.wrapping_sub(2i32.wrapping_mul(w - 1));
            s.st_i = Quarter::new(inner_a, b.wrapping_sub(2i32.wrapping_mul(w - 1)));
            s.st_i.skip = skip_above.is_some_and(|limit| inner_a > limit);
            s.pl = s.leftmost;
        }
        s
    }

    /// Counts one more step of a walk; `false` once the walks have used up [`MAX_WALK_STEPS`].
    fn step(&mut self) -> bool {
        self.steps += 1;
        if self.steps > MAX_WALK_STEPS {
            self.finished = true;
        }
        !self.finished
    }

    /// `ellipse_next`: the next span, or `None` when the ellipse is complete (or the walks have used up
    /// [`MAX_WALK_STEPS`]).
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
                if !self.step() {
                    return None;
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
                if !self.step() {
                    return None;
                }
            }
            // A new row is a step too, so that a walk that only climbs is counted as well.
            if !self.step() {
                return None;
            }
            match outer {
                None => self.finished = true,
                Some((cx, cy)) => {
                    self.pr = cx;
                    self.py = cy;
                }
            }
            self.pl = inner.unwrap_or(self.leftmost);

            let (minus_y, minus_l, minus_r) = (y.wrapping_neg(), l.wrapping_neg(), r.wrapping_neg());
            if (l > 0 || l < r) && y > 0 {
                self.push(if l == 0 { 2 } else { l }, y, r);
            }
            if y > 0 {
                self.push(minus_r, y, minus_l);
            }
            if l > 0 || l < r {
                self.push(if l == 0 { 2 } else { l }, minus_y, r);
            }
            self.push(minus_r, minus_y, minus_l);
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

/// Whether C's results for the ellipse with semi-axes `a`, `b` and width `w` are the true ones: no `int` sum
/// overflows (`2 * (w - 1)`, `X + a`, `cy + 2`) and every value `quarter_delta` returns fits 64 bits. Those values
/// lie between `-a^2 b^2` (the centre) and `a^2 (b + 2)^2` (the corner candidate above the top row); a sum that
/// wraps on the way to such a value still arrives at it. Then the walk is the true nearest-point walk: it stays
/// inside the box, climbs one row after the other and ends.
fn arithmetic_is_exact(a: i32, b: i32, w: i32) -> bool {
    const LIMIT: i64 = 1 << 30;
    if i64::from(a) + i64::from(b) >= LIMIT || i64::from(w) > LIMIT {
        return false;
    }
    let (a, b) = (a as u128, b as u128);
    a * a * (b + 2) * (b + 2) <= i64::MAX as u128
}

/// Whether a span coordinate `start + (X + extent) / 2`, for some `X` from `-extent` to `extent`, can be inside
/// `0..size`. The offsets are `0..=extent` when `X + extent` cannot overflow, else somewhere in `-2^30..2^30`.
fn can_touch(start: i32, extent: i32, size: i32) -> bool {
    const QUARTER: i64 = 1 << 30;
    let (from, n) = if i64::from(extent) < QUARTER {
        (i64::from(start), i64::from(extent) + 1)
    } else {
        (wrap(i64::from(start) - QUARTER, 32), 2 * QUARTER)
    };
    steps_inside(32, i64::from(size), from, 1, n).is_some()
}

/// `ellipseNew`: the ellipse inscribed in the pixel box `x0, y0, x1, y1`, filled or as an outline `width` thick.
///
/// Every span Pillow draws is drawn, in Pillow's order. Three things keep an absurd box (a NaN corner arrives as
/// `i32::MIN`) from walking for minutes, as it does in Pillow:
///
/// - If no span can touch the image the ellipse is skipped. Every span is
///   `hline(x0 + (X0 + a) / 2, y0 + (Y + b) / 2, x0 + (X1 + a) / 2)` with `X0` and `X1` from `-a` to `a` (the walks
///   never leave `0..=a`), so its columns are known without walking; so are its rows when the arithmetic is exact.
/// - When the arithmetic is exact the rows move away from the middle one step after the other, so the walk stops
///   once the rows on both sides have left the image; and a wide ellipse's long level runs are taken in one step
///   each ([`Quarter::run_end`]).
/// - The walks stop after [`MAX_WALK_STEPS`] steps, and what has been drawn until then stays. This is the one
///   place where pixels can be missing that Pillow would draw in the end. It takes an ellipse millions of pixels
///   across whose spans reach the image late: one whose 64-bit sums overflow, or a tall one whose middle is
///   millions of rows away. Measured in Pillow on a 16 x 12 image: 1.6 s for a box 4 x 10^8 wide and 6 high,
///   8.6 s for `[5, 2, nan, 2]`, 37 s for `[-1e9, 3, 12, 9]` outlined; a disc of radius 10^6 around the image,
///   and most outlines with one NaN corner, had not returned after 25 to 40 s.
pub(crate) fn draw_ellipse<S: Surface>(s: &mut S, x0: i32, y0: i32, x1: i32, y1: i32, fill: bool, width: i32) {
    let a = x1.wrapping_sub(x0);
    let b = y1.wrapping_sub(y0);
    if a < 0 || b < 0 {
        return;
    }
    let width = if fill { a.wrapping_add(b) } else { width };
    if width < 1 {
        return;
    }
    let exact = arithmetic_is_exact(a, b, width);
    // The inner walk starts at `a - 2 (width - 1)`, which is at most `a` unless that product overflows.
    let columns_known = i64::from(width) <= 1 << 30;
    if (columns_known && !can_touch(x0, a, s.xsize())) || (exact && !can_touch(y0, b, s.ysize())) {
        return;
    }
    let ysize = i64::from(s.ysize());
    let mut st = EllipseState::new(a, b, width, exact.then_some(RUN_SKIP_ABOVE));
    while let Some((sx0, sy, sx1)) = st.next() {
        s.hline(
            x0.wrapping_add(sx0.wrapping_add(a) / 2),
            y0.wrapping_add(sy.wrapping_add(b) / 2),
            x0.wrapping_add(sx1.wrapping_add(a) / 2),
        );
        if exact {
            // The rows of this pair of spans, above and below the middle; every later pair lies further out.
            let y = i64::from(sy).abs();
            let above = i64::from(y0) + (i64::from(b) - y) / 2;
            let below = i64::from(y0) + (i64::from(b) + y) / 2;
            if above < 0 && below >= ysize {
                break;
            }
        }
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

    #[test]
    #[should_panic(expected = "y1 must be greater than or equal to y0")]
    fn an_upside_down_box_is_refused() {
        let mut im: Img<u8> = Img::new(3, 3, 1);
        ellipse(&mut im, (0.0, 2.0, 1.0, 1.0), Some(&[1]), None, 1);
    }

    /// A 16 x 12 `L` image after one call, as rows of `#` and `.` joined by `|`, and how long the call took.
    fn drawn(bbox: (f64, f64, f64, f64), fill: bool, width: u32) -> (String, std::time::Duration) {
        let mut im: Img<u8> = Img::new(16, 12, 1);
        let started = std::time::Instant::now();
        if fill {
            ellipse(&mut im, bbox, Some(&[1]), None, 1);
        } else {
            ellipse(&mut im, bbox, None, Some(&[1]), width);
        }
        (rows(&im).join("|"), started.elapsed())
    }

    const BLANK: &str = "................";

    fn picture(lines: &[(usize, &str)]) -> String {
        let mut out = [BLANK; 12];
        for &(y, row) in lines {
            out[y] = row;
        }
        out.join("|")
    }

    /// A NaN corner is not refused (Pillow's `xy[2] < xy[0]` is false for it): it is the lowest `int`. Every box
    /// here makes Pillow 12.3.0 return at once with an untouched image (checked on the reference PC); the box
    /// wraps to a negative size, or the fill's width `a + b` wraps below 1.
    #[test]
    fn a_nan_corner_draws_what_pillow_draws() {
        let (nan, inf) = (f64::NAN, f64::INFINITY);
        let nothing = picture(&[]);
        for fill in [true, false] {
            for bbox in [
                (nan, 2.0, 5.0, 9.0),
                (-5.0, 2.0, nan, 9.0),
                (5.0, nan, 9.0, 9.0),
                (5.0, -2.0, 9.0, nan),
                (nan, nan, nan, nan),
                (nan, 2.0, nan, 9.0),
                (5.0, nan, 9.0, nan),
                (-inf, 2.0, 5.0, 9.0),
                (nan, nan, 5.0, 9.0),
            ] {
                assert_eq!(drawn(bbox, fill, 1).0, nothing, "{bbox:?} fill {fill}");
            }
        }
        // Filled, the remaining slots give nothing too: the width a + b wraps.
        for bbox in [
            (nan, 2.0, -5.0, 9.0),
            (5.0, 2.0, nan, 9.0),
            (5.0, nan, 9.0, -3.0),
            (5.0, 2.0, 9.0, nan),
            (5.0, 2.0, inf, 9.0),
            (5.0, 2.0, nan, nan),
        ] {
            assert_eq!(drawn(bbox, true, 1).0, nothing, "{bbox:?}");
        }
    }

    /// Outlines of boxes with one NaN corner make Pillow walk without end (no result after 40 s on the reference
    /// PC). Where the box cannot touch the image the port returns at once, the image untouched; the others are
    /// cut off by the step bound.
    #[test]
    fn absurd_ellipses_return_at_once() {
        let (nan, inf) = (f64::NAN, f64::INFINITY);
        let nothing = picture(&[]);
        // No span can touch the image: the columns end left of it, or the rows above it.
        for bbox in [(nan, 2.0, -5.0, 9.0), (-inf, 2.0, -1.0, 9.0), (-3.0e9, 2.0, -7.0, 9.0)] {
            let (got, took) = drawn(bbox, false, 1);
            assert_eq!(got, nothing, "{bbox:?}");
            assert!(took.as_millis() < 5, "{bbox:?}: {took:?}");
        }
        // The rest stop at the step bound, wherever their walk has got to.
        for bbox in [
            (5.0, 2.0, nan, 9.0),
            (5.0, nan, 9.0, -3.0),
            (5.0, 2.0, 9.0, nan),
            (5.0, 2.0, inf, 9.0),
            (5.0, 2.0, nan, nan),
            (-1.0e9, -1.0e9, 1.0e9, 1.0e9),
            (-5.0e8, -5.0e8, 5.0e8, 5.0e8),
            (3.0, 3.0, 1.0e9, 9.0),
        ] {
            for (fill, width) in [(true, 1), (false, 1), (false, 3)] {
                let (_, took) = drawn(bbox, fill, width);
                assert!(took.as_secs() < 2, "{bbox:?} fill {fill} width {width}: {took:?}");
            }
        }
        // A frame's worth of them still costs little.
        let mut im: Img<u8> = Img::new(360, 640, 3);
        let started = std::time::Instant::now();
        for k in 0..20 {
            let v = f64::from(k);
            ellipse(&mut im, (v, 20.0, nan, 600.0), None, Some(&[1, 2, 3]), 1);
            ellipse(&mut im, (v, nan, 300.0, nan), Some(&[1, 2, 3]), None, 1);
            ellipse(&mut im, (nan, nan, nan, nan), Some(&[1, 2, 3]), Some(&[3, 2, 1]), 1);
        }
        assert!(started.elapsed().as_secs() < 20, "{:?}", started.elapsed());
    }

    /// Ellipses far larger than the image, as Pillow 12.3.0 draws them on a 16 x 12 `L` image (taken on the
    /// reference PC; the largest of each kind takes Pillow 0.6 to 8.5 s). The port goes straight to the rows and
    /// runs that matter.
    #[test]
    fn giant_ellipses_draw_what_pillow_draws() {
        let full = picture(&(0..12).map(|y| (y, "################")).collect::<Vec<_>>());
        let band = picture(&(3..10).map(|y| (y, "################")).collect::<Vec<_>>());
        let rails = picture(&[(3, "################"), (9, "################")]);
        let pillar = picture(&(0..12).map(|y| (y, ".....#######....")).collect::<Vec<_>>());
        let posts = picture(&(0..12).map(|y| (y, ".....#.....#....")).collect::<Vec<_>>());
        let nothing = picture(&[]);
        // A disc centred on the image covers it; its outline is far away.
        for r in [2.0e4, 2.7e4, 3.0e4, 6.0e4, 1.0e5, 3.0e5] {
            let bbox = (8.0 - r, 6.0 - r, 8.0 + r, 6.0 + r);
            assert_eq!(drawn(bbox, true, 1).0, full, "disc {r}");
            assert_eq!(drawn(bbox, false, 1).0, nothing, "circle {r}");
            // A disc that starts at the image's corner misses it.
            let corner = (3.0, 2.0, 3.0 + 2.0 * r, 2.0 + 2.0 * r);
            assert_eq!(drawn(corner, true, 1).0, nothing, "corner disc {r}");
            assert_eq!(drawn(corner, false, 1).0, nothing, "corner circle {r}");
        }
        // Flat: rows 3 to 9, any width. Tall: columns 5 to 11, any height.
        for r in [2.0e4, 1.0e5, 1.0e6, 3.0e6, 1.0e7, 3.0e7, 1.0e8] {
            let flat = (8.0 - r, 3.0, 8.0 + r, 9.0);
            let (got, took) = drawn(flat, true, 1);
            assert_eq!(got, band, "flat {r}");
            assert!(took.as_millis() < 500, "flat {r}: {took:?}");
            assert_eq!(drawn(flat, false, 1).0, rails, "flat outline {r}");
            let tall = (5.0, 6.0 - r, 11.0, 6.0 + r);
            let (got, took) = drawn(tall, true, 1);
            assert_eq!(got, pillar, "tall {r}");
            assert!(took.as_millis() < 500, "tall {r}: {took:?}");
            assert_eq!(drawn(tall, false, 1).0, posts, "tall outline {r}");
        }
        // A box 10^9 wide and one row high: 3.7 s in Pillow.
        let thin = picture(&[(3, "...#############"), (4, "...#############")]);
        assert_eq!(drawn((3.0, 3.0, 1.0e9, 4.0), true, 1).0, thin);
        assert_eq!(drawn((3.0, 3.0, 1.0e9, 4.0), false, 1).0, thin);
    }

    /// The spans of an ellipse, in order, with and without the run skip.
    fn spans(a: i32, b: i32, w: i32, skip_above: Option<i32>) -> Vec<(i32, i32, i32)> {
        let mut st = EllipseState::new(a, b, w, skip_above);
        std::iter::from_fn(|| st.next()).collect()
    }

    #[test]
    fn skipping_level_runs_changes_no_span() {
        // Every small ellipse, filled and outlined at several widths, with the skip forced on from width 0.
        let mut compared = 0u32;
        for a in 0..=48 {
            for b in 0..=48 {
                for w in [a + b, 1, 2, 3, 7] {
                    assert!(arithmetic_is_exact(a, b, w));
                    assert_eq!(spans(a, b, w, Some(0)), spans(a, b, w, None), "a {a} b {b} w {w}");
                    compared += 1;
                }
            }
        }
        assert_eq!(compared, 49 * 49 * 5);
        // Larger ones of every proportion, where the runs are long.
        for (a, b) in [
            (4001, 6),
            (6000, 1),
            (5000, 0),
            (2500, 2500),
            (7, 5000),
            (9000, 33),
            (30001, 2),
        ] {
            for w in [a + b, 1, 4] {
                assert!(arithmetic_is_exact(a, b, w));
                assert_eq!(spans(a, b, w, Some(0)), spans(a, b, w, None), "a {a} b {b} w {w}");
            }
        }
    }

    #[test]
    fn exact_arithmetic_has_its_limits() {
        assert!(arithmetic_is_exact(46_000, 46_000, 92_000));
        assert!(arithmetic_is_exact(1_000_000_000, 1, 1_000_000_001));
        // a (b + 2) above about 3.04e9: the 64-bit sums overflow.
        assert!(!arithmetic_is_exact(60_000, 60_000, 120_000));
        assert!(!arithmetic_is_exact(400_000_000, 6, 1));
        // An int sum would overflow.
        assert!(!arithmetic_is_exact(1 << 30, 0, 1));
        assert!(!arithmetic_is_exact(10, 10, i32::MAX));
    }

    #[test]
    fn spans_that_cannot_touch_the_image_are_known_without_walking() {
        assert!(can_touch(5, 10, 16));
        assert!(can_touch(-10, 10, 16));
        assert!(!can_touch(-11, 10, 16));
        assert!(can_touch(15, 0, 16));
        assert!(!can_touch(16, 1000, 16));
        // From the lowest int, a box 2^31 - 5 wide ends at column -5.
        assert!(!can_touch(i32::MIN, i32::MAX - 4, 16));
        // From column 5, a box that wide reaches every column to the right.
        assert!(can_touch(5, i32::MAX - 4, 16));
    }
}
