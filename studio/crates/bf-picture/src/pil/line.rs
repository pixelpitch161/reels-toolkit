//! `ImageDraw.line`: the Bresenham line of `Draw.c` (`line8`, `line32`, `line32rgba`), the wide line
//! (`ImagingDrawWideLine`) and the polyline loop of `_imaging.c` (`_draw_lines`).
//!
//! # Lines with absurd ends
//!
//! `line8` visits every pixel between its two ends, on the image or not, with C `int` arithmetic. An end that is
//! NaN or infinite arrives here as `i32::MIN`, and then the differences overflow; on the reference build they wrap,
//! and the walk runs for up to 2^31 steps (Pillow needs 1.5 to 2.7 s for such a segment) and still draws pixels:
//! `line([(5, 5), (nan, 7)])` paints row 5 from column 5 to the right edge. [`walk_line`] gives the same pixels
//! without the wait. It does `line8`'s arithmetic in wrapping integers and only steps through the part of the walk
//! that is on the image: the major coordinate moves one pixel per step, so it is inside the image during one
//! stretch of steps at most, and the state at the start of that stretch is computed, not walked to (a closed
//! form when nothing overflowed, else whole runs of steps at a time). One case is cut short: when both
//! differences overflow, the walk starts far from the image and its error term changes sign more than
//! [`MAX_RUNS`] times before reaching it, the segment draws nothing (Pillow walks for seconds there).

use super::polygon::{Edge, polygon_generic};
use super::{Plain, Sample, Surface, c_int, round_down_f64, round_up_f64};
use crate::img::Img;
use bf_core::dmath;

/// The width of a C `int`. The walk takes the width as a parameter so that its tests can compare it with the
/// plain loop exhaustively on a narrow integer.
const INT_BITS: u32 = 32;

/// The most stretches of equal sign the error term may have in the skipped part of a walk that overflowed.
const MAX_RUNS: u32 = 1 << 16;

/// `v` as a two's-complement integer of `bits` bits: what a C `int` of that width holds after an overflow.
pub(super) fn wrap(v: i64, bits: u32) -> i64 {
    let shift = 64 - bits;
    (v << shift) >> shift
}

/// A coordinate that starts at `start` and moves by `dir` (+1 or -1) per step, wrapping as a `bits`-bit integer:
/// the steps `first..end` (within `0..n`) at which it is inside `0..size`, or `None` if it never is. There is
/// one such stretch at most, because `n` is less than a full turn minus the image.
pub(super) fn steps_inside(bits: u32, size: i64, start: i64, dir: i64, n: i64) -> Option<(i64, i64)> {
    if size <= 0 || n <= 0 {
        return None;
    }
    let turn = 1i64 << bits;
    let (first, end) = if (0..size).contains(&start) {
        (0, if dir > 0 { size - start } else { start + 1 })
    } else {
        // The steps needed to reach the near edge of the image, going round if need be.
        let d = if dir > 0 {
            (-start).rem_euclid(turn)
        } else {
            (start - (size - 1)).rem_euclid(turn)
        };
        (d, d + size)
    };
    let end = end.min(n);
    (first < end).then_some((first, end))
}

/// A straight part of `line8` (`for (i = 0; i < n; i++) { point(...); c += dir; }`): the coordinates on the
/// image, in order.
fn straight(bits: u32, size: i64, start: i64, dir: i64, n: i64, mut put: impl FnMut(i64)) {
    if let Some((first, end)) = steps_inside(bits, size, start, dir, n) {
        for k in first..end {
            put(wrap(start + dir * k, bits));
        }
    }
}

/// One sloped walk of `line8` after its set-up: `n` steps; each draws `(major, minor)`, then
/// `if (e >= 0) { minor += smin; e -= sub; } e += add; major += smaj;`, all as `bits`-bit integers.
struct Walk {
    major: i64,
    minor: i64,
    e: i64,
    smaj: i64,
    smin: i64,
    add: i64,
    sub: i64,
    n: i64,
}

impl Walk {
    /// The state after the first `k` steps when nothing in `line8` overflowed. `big` and `small` are the two
    /// differences (`big >= small >= 1`, `n == big`, `e` starts at `2 small - big`). The minor coordinate has
    /// then moved `floor((2 k small + big) / (2 big))` times.
    fn skip_exact(&mut self, bits: u32, k: i64, big: i64, small: i64) {
        let moved = (2 * k * small + big) / (2 * big);
        self.e = 2 * small * (k + 1) - big - moved * 2 * big;
        self.minor = wrap(self.minor + self.smin * moved, bits);
        self.major = wrap(self.major + self.smaj * k, bits);
    }

    /// The state after the first `k` steps in wrapping arithmetic, a run of steps with the same sign of `e` at a
    /// time. `false` if there are more than [`MAX_RUNS`] runs.
    fn skip_wrapped(&mut self, bits: u32, k: i64) -> bool {
        let half = 1i64 << (bits - 1);
        let (int_min, int_max) = (-half, half - 1);
        // What a step adds to `e`: `add - sub` when it starts with `e >= 0` (and the minor coordinate moves),
        // `add` otherwise.
        let on_plus = wrap(self.add - self.sub, bits);
        let on_minus = self.add;
        let mut left = k;
        let mut runs = 0;
        while left > 0 {
            runs += 1;
            if runs > MAX_RUNS {
                return false;
            }
            let plus = self.e >= 0;
            let step = if plus { on_plus } else { on_minus };
            // How many steps keep the sign: until `e` crosses zero or wraps round.
            let run = match (plus, step) {
                (_, 0) => left,
                (true, 1..) => (int_max - self.e) / step + 1,
                (true, _) => self.e / -step + 1,
                (false, 1..) => (-1 - self.e) / step + 1,
                (false, _) => (self.e - int_min) / -step + 1,
            }
            .min(left);
            self.e = wrap(self.e + run * step, bits);
            if plus {
                self.minor = wrap(self.minor + self.smin * run, bits);
            }
            left -= run;
        }
        self.major = wrap(self.major + self.smaj * k, bits);
        true
    }

    /// Hands every point of the walk whose major coordinate is inside `0..size` to `put`, as `(major, minor)`.
    /// `exact` holds the two differences when nothing overflowed.
    fn run(mut self, bits: u32, size: i64, exact: Option<(i64, i64)>, mut put: impl FnMut(i64, i64)) {
        let Some((first, end)) = steps_inside(bits, size, self.major, self.smaj, self.n) else {
            return;
        };
        if first > 0 {
            match exact {
                Some((big, small)) => self.skip_exact(bits, first, big, small),
                None => {
                    if !self.skip_wrapped(bits, first) {
                        return;
                    }
                }
            }
        }
        for _ in first..end {
            put(self.major, self.minor);
            if self.e >= 0 {
                self.minor = wrap(self.minor + self.smin, bits);
                self.e = wrap(self.e - self.sub, bits);
            }
            self.e = wrap(self.e + self.add, bits);
            self.major = wrap(self.major + self.smaj, bits);
        }
    }
}

/// `line8` / `line32` / `line32rgba` for C ints of `bits` bits and an image of `size` (`xsize`, `ysize`): hands
/// `put` the pixels from `(x0, y0)` towards `(x1, y1)`, WITHOUT the last one, in the order C draws them, leaving
/// out pixels whose major coordinate is off the image (C's `point` would drop them). See the module text.
fn walk_line(bits: u32, size: (i64, i64), [x0, y0, x1, y1]: [i64; 4], mut put: impl FnMut(i64, i64)) {
    // normalize coordinates
    let mut dx = wrap(x1 - x0, bits);
    let xs = if dx < 0 {
        dx = wrap(-dx, bits);
        -1
    } else {
        1
    };
    let mut dy = wrap(y1 - y0, bits);
    let ys = if dy < 0 {
        dy = wrap(-dy, bits);
        -1
    } else {
        1
    };
    // Nothing overflows when both true differences double within the int: then the wrapped walk is the plain one.
    let quarter = 1i64 << (bits - 2);
    let no_overflow = (x1 - x0).abs() < quarter && (y1 - y0).abs() < quarter;

    if dx == 0 {
        // vertical
        straight(bits, size.1, y0, ys, dy, |y| put(x0, y));
    } else if dy == 0 {
        // horizontal
        straight(bits, size.0, x0, xs, dx, |x| put(x, y0));
    } else if dx > dy {
        // bresenham, horizontal slope: n = dx; dy += dy; e = dy - dx; dx += dx;
        let add = wrap(dy + dy, bits);
        let walk = Walk {
            major: x0,
            minor: y0,
            e: wrap(add - dx, bits),
            smaj: xs,
            smin: ys,
            add,
            sub: wrap(dx + dx, bits),
            n: dx,
        };
        walk.run(bits, size.0, no_overflow.then_some((dx, dy)), put);
    } else {
        // bresenham, vertical slope: n = dy; dx += dx; e = dx - dy; dy += dy;
        let add = wrap(dx + dx, bits);
        let walk = Walk {
            major: y0,
            minor: x0,
            e: wrap(add - dy, bits),
            smaj: ys,
            smin: xs,
            add,
            sub: wrap(dy + dy, bits),
            n: dy,
        };
        walk.run(bits, size.1, no_overflow.then_some((dy, dx)), |y, x| put(x, y));
    }
}

/// `line8` / `line32` / `line32rgba`: the pixels from `(x0, y0)` towards `(x1, y1)`, WITHOUT the last one. (The
/// caller draws the end point of a polyline; a polygon outline never needs it.)
pub(crate) fn draw_line<S: Surface>(s: &mut S, x0: i32, y0: i32, x1: i32, y1: i32) {
    let size = (i64::from(s.xsize()), i64::from(s.ysize()));
    walk_line(INT_BITS, size, [x0, y0, x1, y1].map(i64::from), |x, y| {
        s.point(x as i32, y as i32)
    });
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
/// quadrilateral per segment, with no joints. Width 0 draws nothing; fewer than two points draw nothing. A NaN or
/// infinite coordinate does not fail: it draws what Pillow draws for it, at once.
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

    /// `line8` as C writes it, one step at a time, every operation wrapped to `bits` bits: what [`walk_line`]
    /// must equal. `put` gets every point, on the image or not.
    fn plain_line(bits: u32, [x0, y0, x1, y1]: [i64; 4], mut put: impl FnMut(i64, i64)) {
        let w = |v: i64| wrap(v, bits);
        let (mut x0, mut y0) = (x0, y0);
        let mut dx = w(x1 - x0);
        let xs = if dx < 0 {
            dx = w(-dx);
            -1
        } else {
            1
        };
        let mut dy = w(y1 - y0);
        let ys = if dy < 0 {
            dy = w(-dy);
            -1
        } else {
            1
        };
        if dx == 0 {
            for _ in 0..dy {
                put(x0, y0);
                y0 = w(y0 + ys);
            }
        } else if dy == 0 {
            for _ in 0..dx {
                put(x0, y0);
                x0 = w(x0 + xs);
            }
        } else if dx > dy {
            let n = dx;
            dy = w(dy + dy);
            let mut e = w(dy - dx);
            dx = w(dx + dx);
            for _ in 0..n {
                put(x0, y0);
                if e >= 0 {
                    y0 = w(y0 + ys);
                    e = w(e - dx);
                }
                e = w(e + dy);
                x0 = w(x0 + xs);
            }
        } else {
            let n = dy;
            dx = w(dx + dx);
            let mut e = w(dx - dy);
            dy = w(dy + dy);
            for _ in 0..n {
                put(x0, y0);
                if e >= 0 {
                    x0 = w(x0 + xs);
                    e = w(e - dy);
                }
                e = w(e + dx);
                y0 = w(y0 + ys);
            }
        }
    }

    /// The points drawn on an image, in order.
    type Points = Vec<(i64, i64)>;

    /// The points on a `w x h` image, in order, from the walk under test and from the plain loop.
    fn both(bits: u32, (w, h): (i64, i64), ends: [i64; 4]) -> (Points, Points) {
        let inside = |x: i64, y: i64| (0..w).contains(&x) && (0..h).contains(&y);
        let (mut fast, mut plain) = (Vec::new(), Vec::new());
        walk_line(bits, (w, h), ends, |x, y| {
            if inside(x, y) {
                fast.push((x, y));
            }
        });
        plain_line(bits, ends, |x, y| {
            if inside(x, y) {
                plain.push((x, y));
            }
        });
        (fast, plain)
    }

    #[test]
    fn the_walk_equals_the_plain_loop_for_every_5_bit_line() {
        // Every pair of ends a 5-bit int can hold, on four images: every overflow a wider int can have happens
        // here too (differences that wrap, doubled terms that wrap, a walk that goes round and comes back).
        let bits = 5;
        let mut lines = 0u32;
        for size in [(6, 5), (16, 16), (1, 1), (16, 3)] {
            for x0 in -16..16 {
                for y0 in -16..16 {
                    for x1 in -16..16 {
                        for y1 in -16..16 {
                            let (fast, plain) = both(bits, size, [x0, y0, x1, y1]);
                            assert_eq!(fast, plain, "{size:?} ({x0}, {y0}) to ({x1}, {y1})");
                            lines += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(lines, 4 << 20);
    }

    #[test]
    fn the_walk_equals_the_plain_loop_for_sampled_8_bit_lines() {
        // A fixed stream of pseudo-random ends (a 64-bit linear congruential generator).
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 33) % 256) as i64 - 128
        };
        for size in [(20, 13), (128, 128), (7, 100)] {
            for _ in 0..200_000 {
                let ends = [next(), next(), next(), next()];
                let (fast, plain) = both(8, size, ends);
                assert_eq!(fast, plain, "{size:?} {ends:?}");
            }
        }
    }

    #[test]
    fn steps_inside_finds_the_one_stretch() {
        // Inside from the start, leaving upwards and downwards.
        assert_eq!(steps_inside(32, 10, 3, 1, 100), Some((0, 7)));
        assert_eq!(steps_inside(32, 10, 3, -1, 100), Some((0, 4)));
        assert_eq!(steps_inside(32, 10, 3, 1, 2), Some((0, 2)));
        // Outside, approaching and never arriving.
        assert_eq!(steps_inside(32, 10, -5, 1, 100), Some((5, 15)));
        assert_eq!(steps_inside(32, 10, 14, -1, 100), Some((5, 15)));
        assert_eq!(steps_inside(32, 10, -5, 1, 5), None);
        assert_eq!(steps_inside(32, 10, 12, 1, i64::from(i32::MAX)), None);
        assert_eq!(steps_inside(32, 10, -1, -1, 1000), None);
        // Going down from the lowest int wraps to the highest and reaches the image from above at the very end.
        let low = i64::from(i32::MIN);
        assert_eq!(
            steps_inside(32, 12, low + 1, -1, (1 << 31) - 8),
            Some(((1 << 31) - 10, (1 << 31) - 8))
        );
        assert_eq!(steps_inside(32, 10, low + 1, -1, (1 << 31) - 8), None);
    }

    /// The pictures Pillow 12.3.0 draws on a 16 x 12 `L` image for lines with a non-finite or absurd end (taken
    /// on the reference PC, where each thin one takes Pillow 1.2 to 2.7 s). Here they are immediate.
    #[test]
    fn non_finite_ends_draw_what_pillow_draws() {
        let (nan, inf) = (f64::NAN, f64::INFINITY);
        let draw = |pts: &[(f64, f64)], width: u32| {
            let mut im: Img<u8> = Img::new(16, 12, 1);
            line(&mut im, pts, &[255], width);
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
        // The end's x is NaN: the difference wraps to a positive number, and the walk runs right.
        assert_eq!(draw(&[(5.0, 5.0), (nan, 7.0)], 1), picture(&[(5, ".....###########")]));
        assert_eq!(draw(&[(5.0, 5.0), (inf, 5.0)], 1), picture(&[(5, ".....###########")]));
        assert_eq!(draw(&[(5.0, 5.0), (-inf, 5.0)], 1), picture(&[(5, ".....###########")]));
        // The start's x is NaN on a level line: the walk comes round and arrives from the right.
        assert_eq!(draw(&[(nan, 5.0), (9.0, 5.0)], 1), picture(&[(5, ".........#######")]));
        // A sloped walk that comes round arrives far off the image: only the last point shows.
        let last_point = picture(&[(7, ".........#......")]);
        assert_eq!(draw(&[(nan, 5.0), (9.0, 7.0)], 1), last_point);
        assert_eq!(draw(&[(5.0, nan), (9.0, 7.0)], 1), last_point);
        assert_eq!(draw(&[(nan, nan), (9.0, 7.0)], 1), last_point);
        // Both of the end's coordinates NaN: a diagonal to the corner.
        let diagonal: Vec<(usize, &str)> = vec![
            (5, ".....#.........."),
            (6, "......#........."),
            (7, ".......#........"),
            (8, "........#......."),
            (9, ".........#......"),
            (10, "..........#....."),
            (11, "...........#...."),
        ];
        assert_eq!(draw(&[(5.0, 5.0), (nan, nan)], 1), picture(&diagonal));
        // The end's y is NaN: straight down.
        let down: Vec<(usize, &str)> = (5..12).map(|y| (y, ".....#..........")).collect();
        assert_eq!(draw(&[(5.0, 5.0), (9.0, nan)], 1), picture(&down));
        assert_eq!(draw(&[(5.0, 5.0), (5.0, nan)], 1), picture(&down));
        // From column 0 the difference is the lowest int itself, which stays negative: a short vertical line.
        assert_eq!(
            draw(&[(0.0, 5.0), (nan, 7.0)], 1),
            picture(&[(5, "#..............."), (6, "#...............")])
        );
        assert_eq!(draw(&[(-3.0, 5.0), (nan, 7.0)], 1), picture(&[]));
        // Finite but absurd ends.
        assert_eq!(draw(&[(5.0, 5.0), (1e9, 7.0)], 1), picture(&[(5, ".....###########")]));
        assert_eq!(draw(&[(1e9, 5.0), (5.0, 7.0)], 1), picture(&[(7, ".....###########")]));
        assert_eq!(
            draw(&[(1e9, 9e8), (5.0, 7.0)], 1),
            picture(&[
                (7, ".....#.........."),
                (8, "......#........."),
                (9, ".......#........"),
                (10, "........#......."),
                (11, ".........##.....")
            ])
        );
        assert_eq!(
            draw(&[(5.0, 5.0), (1.5e9, 1.2e9)], 1),
            picture(&[
                (5, ".....#.........."),
                (6, "......#........."),
                (7, ".......##......."),
                (8, ".........#......"),
                (9, "..........#....."),
                (10, "...........#...."),
                (11, "............##..")
            ])
        );
        assert_eq!(
            draw(&[(1.5e9, 1.2e9), (5.0, 5.0)], 1),
            picture(&[(5, ".....#..........")])
        );
        // A polyline through a NaN point.
        let through: Vec<(usize, &str)> = vec![
            (2, "..#............."),
            (3, "...#............"),
            (4, "....#..........."),
            (5, ".....#.........."),
            (6, "......#........."),
            (7, ".......#........"),
            (8, "........#......."),
            (9, ".........#......"),
            (10, "..........#....."),
            (11, "...........#...."),
        ];
        assert_eq!(draw(&[(2.0, 2.0), (nan, nan), (9.0, 9.0)], 1), picture(&through));
        // Wide lines are quadrilaterals around the wrapped ends.
        assert_eq!(
            draw(&[(5.0, 5.0), (nan, 7.0)], 3),
            picture(&[
                (4, "######.........."),
                (5, "######.........."),
                (6, "######.........."),
                (7, "################")
            ])
        );
        assert_eq!(
            draw(&[(nan, nan), (9.0, 7.0)], 3),
            picture(&[
                (0, "#..............."),
                (6, "#######........."),
                (7, "########........"),
                (8, "#########.......")
            ])
        );
        assert_eq!(
            draw(&[(5.0, 5.0), (9.0, nan)], 3),
            picture(&(0..6).map(|y| (y, "....###.........")).collect::<Vec<_>>())
        );
    }

    #[test]
    fn absurd_lines_return_at_once() {
        // Each of these makes Pillow walk a thousand million pixels or more. Thousands of them must cost nothing.
        let (nan, inf) = (f64::NAN, f64::INFINITY);
        let mut im: Img<u8> = Img::new(360, 640, 3);
        let started = std::time::Instant::now();
        for k in 0..2000 {
            let v = f64::from(k);
            line(&mut im, &[(nan, v), (v, 300.0)], &[1, 2, 3], 1);
            line(&mut im, &[(v, 20.0), (inf, -inf)], &[1, 2, 3], 1);
            line(&mut im, &[(1.9e9, -2.0e9), (v, 100.0)], &[1, 2, 3], 1);
            line(&mut im, &[(-1.5e9 - v, 1.2e9), (1.4e9, -1.3e9 + v)], &[1, 2, 3], 1);
            line(&mut im, &[(nan, nan), (v, v)], &[1, 2, 3], 4);
        }
        assert!(started.elapsed().as_secs() < 20, "{:?}", started.elapsed());
    }
}
