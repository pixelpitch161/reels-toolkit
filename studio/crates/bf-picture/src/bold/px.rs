//! Crisp pixel drawing for the bold effects: a few hard-edged shapes (no anti-aliasing, as everything in the
//! ring is pixel art) on any of the renderer's pictures. The shapes are given in screen pixels (the 360 x 640
//! frame); every one is cut by the picture's edge, and a shape with a NaN or an empty size draws nothing.
//!
//! The functions with `ink_` in front draw the shape once in the dark ink, a pixel bigger all round, and then in
//! its colour: the "dark outline" every bold weapon and mark has, so it reads on any background.

use bf_core::dmath;

use super::INK;
use crate::camera::V3;
use crate::img::Img;
use crate::look::Pic;

/// A picture the shapes can be drawn on: the `float32` and `float64` frames, the byte frame and [`Pic`].
pub trait Surface {
    /// Its width and height in pixels.
    fn size(&self) -> (usize, usize);
    /// Mixes `col` (0 to 255 a channel) into the pixel with strength `a` (0 to 1); off the picture: nothing.
    fn mix(&mut self, x: i32, y: i32, col: V3, a: f64);
    /// Adds `col * a` to the pixel as light; off the picture: nothing.
    fn add(&mut self, x: i32, y: i32, col: V3, a: f64);
    /// Multiplies the pixel by `k` (below 1 darkens it); off the picture: nothing.
    fn dim(&mut self, x: i32, y: i32, k: f64);
    /// Drains the pixel's colour: mixes it toward its own grey with strength `k` (0 to 1); off the picture:
    /// nothing.
    fn grey(&mut self, x: i32, y: i32, k: f64);
}

/// A pixel's grey (its luma).
fn luma(r: f64, g: f64, b: f64) -> f64 {
    0.299 * r + 0.587 * g + 0.114 * b
}

/// Whether a size is no size at all: zero, negative or not a number.
pub(super) fn bad(x: f64) -> bool {
    x.is_nan() || x <= 0.0
}

fn inside(w: usize, h: usize, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h
}

impl Surface for Img<f32> {
    fn size(&self) -> (usize, usize) {
        (self.w, self.h)
    }
    fn mix(&mut self, x: i32, y: i32, col: V3, a: f64) {
        if !inside(self.w, self.h, x, y) || self.c < 3 {
            return;
        }
        let px = self.px_mut(x as usize, y as usize);
        for c in 0..3 {
            px[c] = (f64::from(px[c]) * (1.0 - a) + col[c] * a) as f32;
        }
    }
    fn add(&mut self, x: i32, y: i32, col: V3, a: f64) {
        if !inside(self.w, self.h, x, y) || self.c < 3 {
            return;
        }
        let px = self.px_mut(x as usize, y as usize);
        for c in 0..3 {
            px[c] = (f64::from(px[c]) + col[c] * a).min(255.0) as f32;
        }
    }
    fn dim(&mut self, x: i32, y: i32, k: f64) {
        if !inside(self.w, self.h, x, y) || self.c < 3 {
            return;
        }
        for v in self.px_mut(x as usize, y as usize).iter_mut().take(3) {
            *v = (f64::from(*v) * k) as f32;
        }
    }
    fn grey(&mut self, x: i32, y: i32, k: f64) {
        if !inside(self.w, self.h, x, y) || self.c < 3 {
            return;
        }
        let px = self.px_mut(x as usize, y as usize);
        let g = luma(f64::from(px[0]), f64::from(px[1]), f64::from(px[2]));
        for v in px.iter_mut().take(3) {
            *v = (f64::from(*v) * (1.0 - k) + g * k) as f32;
        }
    }
}

impl Surface for Img<f64> {
    fn size(&self) -> (usize, usize) {
        (self.w, self.h)
    }
    fn mix(&mut self, x: i32, y: i32, col: V3, a: f64) {
        if !inside(self.w, self.h, x, y) || self.c < 3 {
            return;
        }
        let px = self.px_mut(x as usize, y as usize);
        for c in 0..3 {
            px[c] = px[c] * (1.0 - a) + col[c] * a;
        }
    }
    fn add(&mut self, x: i32, y: i32, col: V3, a: f64) {
        if !inside(self.w, self.h, x, y) || self.c < 3 {
            return;
        }
        let px = self.px_mut(x as usize, y as usize);
        for c in 0..3 {
            px[c] = (px[c] + col[c] * a).min(255.0);
        }
    }
    fn dim(&mut self, x: i32, y: i32, k: f64) {
        if !inside(self.w, self.h, x, y) || self.c < 3 {
            return;
        }
        for v in self.px_mut(x as usize, y as usize).iter_mut().take(3) {
            *v *= k;
        }
    }
    fn grey(&mut self, x: i32, y: i32, k: f64) {
        if !inside(self.w, self.h, x, y) || self.c < 3 {
            return;
        }
        let px = self.px_mut(x as usize, y as usize);
        let g = luma(px[0], px[1], px[2]);
        for v in px.iter_mut().take(3) {
            *v = *v * (1.0 - k) + g * k;
        }
    }
}

impl Surface for Img<u8> {
    fn size(&self) -> (usize, usize) {
        (self.w, self.h)
    }
    fn mix(&mut self, x: i32, y: i32, col: V3, a: f64) {
        if !inside(self.w, self.h, x, y) || self.c < 3 {
            return;
        }
        let px = self.px_mut(x as usize, y as usize);
        for c in 0..3 {
            px[c] = (f64::from(px[c]) * (1.0 - a) + col[c] * a).clamp(0.0, 255.0) as u8;
        }
    }
    fn add(&mut self, x: i32, y: i32, col: V3, a: f64) {
        if !inside(self.w, self.h, x, y) || self.c < 3 {
            return;
        }
        let px = self.px_mut(x as usize, y as usize);
        for c in 0..3 {
            px[c] = (f64::from(px[c]) + col[c] * a).clamp(0.0, 255.0) as u8;
        }
    }
    fn dim(&mut self, x: i32, y: i32, k: f64) {
        if !inside(self.w, self.h, x, y) || self.c < 3 {
            return;
        }
        for v in self.px_mut(x as usize, y as usize).iter_mut().take(3) {
            *v = (f64::from(*v) * k).clamp(0.0, 255.0) as u8;
        }
    }
    fn grey(&mut self, x: i32, y: i32, k: f64) {
        if !inside(self.w, self.h, x, y) || self.c < 3 {
            return;
        }
        let px = self.px_mut(x as usize, y as usize);
        let g = luma(f64::from(px[0]), f64::from(px[1]), f64::from(px[2]));
        for v in px.iter_mut().take(3) {
            *v = (f64::from(*v) * (1.0 - k) + g * k).clamp(0.0, 255.0) as u8;
        }
    }
}

impl Surface for Pic {
    fn size(&self) -> (usize, usize) {
        match self {
            Pic::F32(a) => (a.w, a.h),
            Pic::F64(a) => (a.w, a.h),
        }
    }
    fn mix(&mut self, x: i32, y: i32, col: V3, a: f64) {
        match self {
            Pic::F32(p) => p.mix(x, y, col, a),
            Pic::F64(p) => p.mix(x, y, col, a),
        }
    }
    fn add(&mut self, x: i32, y: i32, col: V3, a: f64) {
        match self {
            Pic::F32(p) => p.add(x, y, col, a),
            Pic::F64(p) => p.add(x, y, col, a),
        }
    }
    fn dim(&mut self, x: i32, y: i32, k: f64) {
        match self {
            Pic::F32(p) => p.dim(x, y, k),
            Pic::F64(p) => p.dim(x, y, k),
        }
    }
    fn grey(&mut self, x: i32, y: i32, k: f64) {
        match self {
            Pic::F32(p) => p.grey(x, y, k),
            Pic::F64(p) => p.grey(x, y, k),
        }
    }
}

/// The pixel columns and rows a shape inside `(x0, y0)..(x1, y1)` can touch, cut to the picture; `None` when
/// that is nothing (or a bound is not a number).
fn span<S: Surface + ?Sized>(s: &S, x0: f64, y0: f64, x1: f64, y1: f64) -> Option<(i32, i32, i32, i32)> {
    if !(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite()) {
        return None;
    }
    let (w, h) = s.size();
    let xa = (x0.floor().max(0.0)) as i32;
    let ya = (y0.floor().max(0.0)) as i32;
    let xb = (x1.ceil().min(w as f64 - 1.0)) as i32;
    let yb = (y1.ceil().min(h as f64 - 1.0)) as i32;
    if xb < xa || yb < ya {
        return None;
    }
    Some((xa, ya, xb, yb))
}

/// A filled disc of radius `r` round `c`.
pub fn disc<S: Surface + ?Sized>(s: &mut S, c: (f64, f64), r: f64, col: V3, a: f64) {
    if bad(r) || a <= 0.0 {
        return;
    }
    let Some((xa, ya, xb, yb)) = span(s, c.0 - r, c.1 - r, c.0 + r, c.1 + r) else {
        return;
    };
    for y in ya..=yb {
        for x in xa..=xb {
            let (dx, dy) = (f64::from(x) + 0.5 - c.0, f64::from(y) + 0.5 - c.1);
            if dx * dx + dy * dy <= r * r {
                s.mix(x, y, col, a);
            }
        }
    }
}

/// A ring of radius `r` and line width `w` round `c`.
pub fn ring<S: Surface + ?Sized>(s: &mut S, c: (f64, f64), r: f64, w: f64, col: V3, a: f64) {
    arc(s, c, r, w, 0.0, dmath::TAU, col, a);
}

/// The part of a ring from angle `a0` to `a1` (radians, 0 is to the right and the angle grows clockwise on the
/// screen); `a1 - a0` of a full turn or more is the whole ring.
#[allow(clippy::too_many_arguments)]
pub fn arc<S: Surface + ?Sized>(s: &mut S, c: (f64, f64), r: f64, w: f64, a0: f64, a1: f64, col: V3, a: f64) {
    if bad(r) || bad(w) || a <= 0.0 || bad(a1 - a0) {
        return;
    }
    let ro = r + w / 2.0;
    let ri = (r - w / 2.0).max(0.0);
    let Some((xa, ya, xb, yb)) = span(s, c.0 - ro, c.1 - ro, c.0 + ro, c.1 + ro) else {
        return;
    };
    let whole = a1 - a0 >= dmath::TAU - 1e-9;
    for y in ya..=yb {
        for x in xa..=xb {
            let (dx, dy) = (f64::from(x) + 0.5 - c.0, f64::from(y) + 0.5 - c.1);
            let d2 = dx * dx + dy * dy;
            if d2 > ro * ro || d2 < ri * ri {
                continue;
            }
            if !whole {
                let ang = (dmath::atan2(dy, dx) - a0).rem_euclid(dmath::TAU);
                if ang > a1 - a0 {
                    continue;
                }
            }
            s.mix(x, y, col, a);
        }
    }
}

/// A straight line from `p` to `q`, `w` pixels wide, with round ends.
pub fn line<S: Surface + ?Sized>(s: &mut S, p: (f64, f64), q: (f64, f64), w: f64, col: V3, a: f64) {
    if bad(w) || a <= 0.0 {
        return;
    }
    let h = w / 2.0;
    let Some((xa, ya, xb, yb)) = span(
        s,
        p.0.min(q.0) - h,
        p.1.min(q.1) - h,
        p.0.max(q.0) + h,
        p.1.max(q.1) + h,
    ) else {
        return;
    };
    let (vx, vy) = (q.0 - p.0, q.1 - p.1);
    let len2 = vx * vx + vy * vy;
    for y in ya..=yb {
        for x in xa..=xb {
            let (px, py) = (f64::from(x) + 0.5 - p.0, f64::from(y) + 0.5 - p.1);
            let u = if len2 > 0.0 {
                ((px * vx + py * vy) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let (dx, dy) = (px - u * vx, py - u * vy);
            if dx * dx + dy * dy <= h * h {
                s.mix(x, y, col, a);
            }
        }
    }
}

/// The lines through `pts` in order (not closed).
pub fn polyline<S: Surface + ?Sized>(s: &mut S, pts: &[(f64, f64)], w: f64, col: V3, a: f64) {
    for pair in pts.windows(2) {
        line(s, pair[0], pair[1], w, col, a);
    }
}

/// A filled polygon (any shape; a point is inside by the even-odd rule).
pub fn poly<S: Surface + ?Sized>(s: &mut S, pts: &[(f64, f64)], col: V3, a: f64) {
    if pts.len() < 3 || a <= 0.0 {
        return;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in pts {
        x0 = x0.min(p.0);
        y0 = y0.min(p.1);
        x1 = x1.max(p.0);
        y1 = y1.max(p.1);
    }
    let Some((xa, ya, xb, yb)) = span(s, x0, y0, x1, y1) else {
        return;
    };
    for y in ya..=yb {
        let cy = f64::from(y) + 0.5;
        for x in xa..=xb {
            let cx = f64::from(x) + 0.5;
            let mut odd = false;
            let mut j = pts.len() - 1;
            for i in 0..pts.len() {
                let (pi, pj) = (pts[i], pts[j]);
                if (pi.1 > cy) != (pj.1 > cy) && cx < (pj.0 - pi.0) * (cy - pi.1) / (pj.1 - pi.1) + pi.0 {
                    odd = !odd;
                }
                j = i;
            }
            if odd {
                s.mix(x, y, col, a);
            }
        }
    }
}

/// The corners of a star of `n` points round `c`: outer radius `r_out`, inner `r_in`, turned by `rot` radians.
pub fn star_points(c: (f64, f64), r_out: f64, r_in: f64, n: usize, rot: f64) -> Vec<(f64, f64)> {
    let n = n.max(2);
    (0..2 * n)
        .map(|i| {
            let r = if i % 2 == 0 { r_out } else { r_in };
            let ang = rot + i as f64 * dmath::PI / n as f64;
            (c.0 + r * dmath::cos(ang), c.1 + r * dmath::sin(ang))
        })
        .collect()
}

/// A filled star (see [`star_points`]).
#[allow(clippy::too_many_arguments)]
pub fn star<S: Surface + ?Sized>(s: &mut S, c: (f64, f64), r_out: f64, r_in: f64, n: usize, rot: f64, col: V3, a: f64) {
    poly(s, &star_points(c, r_out, r_in, n, rot), col, a);
}

/// An X of half-size `r` and line width `w`.
pub fn cross<S: Surface + ?Sized>(s: &mut S, c: (f64, f64), r: f64, w: f64, col: V3, a: f64) {
    line(s, (c.0 - r, c.1 - r), (c.0 + r, c.1 + r), w, col, a);
    line(s, (c.0 - r, c.1 + r), (c.0 + r, c.1 - r), w, col, a);
}

/// An arrow from `p` to `q`: a line `w` wide with a filled head `head` pixels long.
pub fn arrow<S: Surface + ?Sized>(s: &mut S, p: (f64, f64), q: (f64, f64), w: f64, head: f64, col: V3, a: f64) {
    let (vx, vy) = (q.0 - p.0, q.1 - p.1);
    let n = dmath::hypot(vx, vy);
    if bad(n) {
        return;
    }
    let (ux, uy) = (vx / n, vy / n);
    let base = (q.0 - ux * head, q.1 - uy * head);
    line(s, p, base, w, col, a);
    let hw = head * 0.6;
    poly(
        s,
        &[
            q,
            (base.0 - uy * hw, base.1 + ux * hw),
            (base.0 + uy * hw, base.1 - ux * hw),
        ],
        col,
        a,
    );
}

/// [`disc`] with the dark outline.
pub fn ink_disc<S: Surface + ?Sized>(s: &mut S, c: (f64, f64), r: f64, col: V3, a: f64) {
    disc(s, c, r + 1.0, INK, a);
    disc(s, c, r, col, a);
}

/// [`ring`] with the dark outline on both sides.
pub fn ink_ring<S: Surface + ?Sized>(s: &mut S, c: (f64, f64), r: f64, w: f64, col: V3, a: f64) {
    ring(s, c, r, w + 2.0, INK, a);
    ring(s, c, r, w, col, a);
}

/// [`arc`] with the dark outline on both sides.
#[allow(clippy::too_many_arguments)]
pub fn ink_arc<S: Surface + ?Sized>(s: &mut S, c: (f64, f64), r: f64, w: f64, a0: f64, a1: f64, col: V3, a: f64) {
    arc(s, c, r, w + 2.0, a0, a1, INK, a);
    arc(s, c, r, w, a0, a1, col, a);
}

/// [`line`] with the dark outline.
pub fn ink_line<S: Surface + ?Sized>(s: &mut S, p: (f64, f64), q: (f64, f64), w: f64, col: V3, a: f64) {
    line(s, p, q, w + 2.0, INK, a);
    line(s, p, q, w, col, a);
}

/// [`polyline`] with the dark outline (the whole outline first, so the joints stay clean).
pub fn ink_polyline<S: Surface + ?Sized>(s: &mut S, pts: &[(f64, f64)], w: f64, col: V3, a: f64) {
    polyline(s, pts, w + 2.0, INK, a);
    polyline(s, pts, w, col, a);
}

/// [`poly`] with a 1 px dark outline round it.
pub fn ink_poly<S: Surface + ?Sized>(s: &mut S, pts: &[(f64, f64)], col: V3, a: f64) {
    if pts.len() < 3 {
        return;
    }
    let mut closed = pts.to_vec();
    closed.push(pts[0]);
    polyline(s, &closed, 2.6, INK, a);
    poly(s, pts, col, a);
}

/// [`star`] with the dark outline.
#[allow(clippy::too_many_arguments)]
pub fn ink_star<S: Surface + ?Sized>(
    s: &mut S,
    c: (f64, f64),
    r_out: f64,
    r_in: f64,
    n: usize,
    rot: f64,
    col: V3,
    a: f64,
) {
    ink_poly(s, &star_points(c, r_out, r_in, n, rot), col, a);
}

/// [`cross`] with the dark outline.
pub fn ink_cross<S: Surface + ?Sized>(s: &mut S, c: (f64, f64), r: f64, w: f64, col: V3, a: f64) {
    cross(s, c, r, w + 2.0, INK, a);
    cross(s, c, r, w, col, a);
}

/// [`arrow`] with the dark outline.
pub fn ink_arrow<S: Surface + ?Sized>(s: &mut S, p: (f64, f64), q: (f64, f64), w: f64, head: f64, col: V3, a: f64) {
    let (vx, vy) = (q.0 - p.0, q.1 - p.1);
    let n = dmath::hypot(vx, vy);
    if bad(n) {
        return;
    }
    let (ux, uy) = (vx / n, vy / n);
    arrow(
        s,
        (p.0 - ux, p.1 - uy),
        (q.0 + ux * 1.5, q.1 + uy * 1.5),
        w + 2.0,
        head + 2.0,
        INK,
        a,
    );
    arrow(s, p, q, w, head, col, a);
}

/// A four-point glint (a thin plus with a bright centre) of half-size `r`: the "watch out" sparkle of a wind-up.
pub fn glint<S: Surface + ?Sized>(s: &mut S, c: (f64, f64), r: f64, col: V3, a: f64) {
    star(s, c, r, r * 0.22, 4, 0.0, col, a);
    disc(s, c, (r * 0.3).max(1.0), [255.0, 255.0, 255.0], a);
}

/// Drains the colour out of the whole picture with strength `k` (0: untouched, 1: black and white), for a
/// moment when time stops or the lights go.
pub fn wash<S: Surface + ?Sized>(s: &mut S, k: f64) {
    if bad(k) {
        return;
    }
    let (w, h) = s.size();
    let k = k.min(1.0);
    for y in 0..h {
        for x in 0..w {
            s.grey(x as i32, y as i32, k);
        }
    }
}

/// Four corner brackets round the square of half-size `r` at `c`: a lock-on, a freeze, a camera frame.
pub fn brackets<S: Surface + ?Sized>(s: &mut S, c: (f64, f64), r: f64, w: f64, col: V3, a: f64) {
    let arm = r * 0.45;
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let corner = (c.0 + sx * r, c.1 + sy * r);
        ink_polyline(
            s,
            &[(corner.0 - sx * arm, corner.1), corner, (corner.0, corner.1 - sy * arm)],
            w,
            col,
            a,
        );
    }
}

/// Darkens everything outside the circle of radius `r` round `c` to `k` of its brightness, with a soft edge
/// `soft` pixels wide (dithered by `dither(x, y)`, 0 to 1, so the edge is pixel art and not a gradient).
pub fn dim_outside<S: Surface + ?Sized>(
    s: &mut S,
    c: (f64, f64),
    r: f64,
    soft: f64,
    k: f64,
    dither: &dyn Fn(usize, usize) -> f64,
) {
    if bad(1.0 - k) {
        return;
    }
    let (w, h) = s.size();
    for y in 0..h {
        for x in 0..w {
            let d = dmath::hypot(x as f64 + 0.5 - c.0, y as f64 + 0.5 - c.1);
            let mut u = if soft > 0.0 {
                ((d - r) / soft).clamp(0.0, 1.0)
            } else if d > r {
                1.0
            } else {
                0.0
            };
            if u <= 0.0 {
                continue;
            }
            // four steps, dithered
            u = ((u * 4.0 + dither(x, y) * 0.9).floor() / 4.0).min(1.0);
            if u > 0.0 {
                s.dim(x as i32, y as i32, 1.0 - (1.0 - k) * u);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(img: &Img<f64>) -> usize {
        (0..img.w * img.h).filter(|&i| img.data[i * 3] > 0.0).count()
    }

    #[test]
    fn shapes_are_cut_by_the_edge_and_ignore_nan() {
        let mut img: Img<f64> = Img::new(20, 20, 3);
        disc(&mut img, (-2.0, 3.0), 6.0, [255.0; 3], 1.0);
        line(&mut img, (-50.0, 10.0), (50.0, 10.0), 3.0, [255.0; 3], 1.0);
        ring(&mut img, (f64::NAN, 3.0), 6.0, 2.0, [255.0; 3], 1.0);
        poly(&mut img, &[(0.0, 0.0), (f64::NAN, 5.0), (5.0, 5.0)], [255.0; 3], 1.0);
        assert!(lit(&img) > 0);
    }

    #[test]
    fn a_ring_is_hollow_and_an_arc_is_part_of_it() {
        let mut a: Img<f64> = Img::new(40, 40, 3);
        ring(&mut a, (20.0, 20.0), 10.0, 2.0, [255.0; 3], 1.0);
        assert_eq!(a.px(20, 20)[0], 0.0);
        assert!(a.px(30, 20)[0] > 0.0);
        let mut b: Img<f64> = Img::new(40, 40, 3);
        arc(&mut b, (20.0, 20.0), 10.0, 2.0, 0.0, 1.0, [255.0; 3], 1.0);
        assert!(lit(&b) > 0 && lit(&b) < lit(&a));
    }

    #[test]
    fn a_star_has_twice_its_points_as_corners() {
        assert_eq!(star_points((0.0, 0.0), 5.0, 2.0, 4, 0.0).len(), 8);
        let mut img: Img<f64> = Img::new(30, 30, 3);
        ink_star(&mut img, (15.0, 15.0), 8.0, 3.0, 4, 0.0, [255.0, 240.0, 200.0], 1.0);
        assert!(img.px(15, 15)[0] > 200.0);
    }

    #[test]
    fn the_dim_leaves_the_circle_alone() {
        let mut img: Img<f64> = Img::filled(30, 30, &[100.0, 100.0, 100.0]);
        dim_outside(&mut img, (15.0, 15.0), 6.0, 4.0, 0.4, &|_, _| 0.5);
        assert_eq!(img.px(15, 15)[0], 100.0);
        assert!(img.px(0, 0)[0] < 50.0);
    }
}
