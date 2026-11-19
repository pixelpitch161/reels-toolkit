//! The card's effects, all in screen space and all a pure function of the time: the crack `VS` runs along the
//! divider, and over the closed card each slam's local light, motion streaks, dust burst and the sparks of `VS`.
//!
//! The port of `_rng`, `streaks`, `dust`, `light`, `crack_pts`, `draw_crack` and `sparks`. Every random number is
//! numpy's `default_rng` with a fixed seed, drawn in the order the Python draws it (`bf_core::rng`). Python takes
//! `exp`, `sin`, `cos` and the dust's `** 1.5` from the C runtime of the reference PC, the Studio from
//! `bf_core::dmath`; a result may differ in its last bit, which reaches a pixel only where a position lies within
//! about 1e-13 of a whole pixel or a mixed colour within 1e-13 of a whole level.

use bf_core::dmath;
use bf_core::grid::Grid;
use bf_core::py::{py_max, py_min};
use bf_core::rng::Generator;

use super::layout;
use super::{GOLD, INK, over};
use crate::config::{self, H, W};
use crate::img::{Img, RgbF};
use crate::{np, pil};

/// `_rng(j)`: `np.random.default_rng(3000 + j)`.
fn rng(j: u64) -> Generator {
    Generator::new(3000 + j)
}

/// One frame in seconds, `1.0 / T.FPS`: the effects that start "the frame after the slam" count from here.
fn frame(grid: &Grid) -> f64 {
    1.0 / f64::from(grid.fps)
}

/// Python's `int(x)` for a pixel coordinate.
fn int(x: f64) -> i32 {
    np::f64_to_i32(x)
}

/// `light(img, t, j, cx, cy, col)`: a soft local light in the word's colour behind word `j`, for half a second
/// from its slam (it rises from nothing on the slam frame itself).
pub fn light(img: &mut RgbF, grid: &Grid, t: f64, j: usize, cx: f64, cy: f64, col: [f64; 3]) {
    let u = t - grid.title[j];
    if !(0.0..0.5).contains(&u) {
        return;
    }
    let k = py_min(1.0, u / 0.06) * dmath::exp(-py_max(0.0, u - 0.06) / 0.14) * 0.55;
    let r = 110.0;
    let ry = r * 0.6;
    let (x0i, x1i) = (int(py_max(0.0, cx - r)), int(py_min(W as f64, cx + r)));
    let (y0i, y1i) = (int(py_max(0.0, cy - ry)), int(py_min(H as f64, cy + ry)));
    for y in y0i..y1i {
        for x in x0i..x1i {
            // dd = ((xx - cx) / r) ** 2 + ((yy - cy) / (r * 0.6)) ** 2
            let (ax, ay) = ((f64::from(x) - cx) / r, (f64::from(y) - cy) / ry);
            let dd = ax * ax + ay * ay;
            // img += (np.clip(1 - dd, 0, 1) ** 2 * k)[..., None] * col[None, None] * 0.6
            let c = np::clip(1.0 - dd, 0.0, 1.0);
            let g = c * c * k;
            for (p, cc) in img.px_mut(x as usize, y as usize).iter_mut().zip(col) {
                *p += g * cc * 0.6;
            }
        }
    }
}

/// `streaks(img, t, j, cx, cy, w, h, col)`: fourteen motion streaks either side of a slamming word (`w x h` at
/// rest), shooting outward and fading over a quarter of a second, from the frame after the slam.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn streaks(img: &mut RgbF, grid: &Grid, t: f64, j: usize, cx: f64, cy: f64, w: i32, h: i32, col: [f64; 3]) {
    streaks_at(img, grid, t, j, cx, cy, w, h, col, 0.0);
}

/// `np.arange(start, stop, step)` for `float64`: `ceil((stop - start) / step)` values, the first two `start` and
/// `start + step`, then `start + i * delta` with `delta` the difference of those two (numpy's `fill`).
pub(crate) fn arange(start: f64, stop: f64, step: f64) -> Vec<f64> {
    let len = ((stop - start) / step).ceil();
    if len.is_nan() || len <= 0.0 {
        return Vec::new();
    }
    let n = np::f64_to_i64(len).max(0) as usize;
    let mut out = Vec::with_capacity(n);
    out.push(start);
    if n > 1 {
        let second = start + step;
        let delta = second - start;
        out.push(second);
        out.extend((2..n).map(|i| start + i as f64 * delta));
    }
    out
}

/// `streaks(img, t, j, cx, cy, w, h, col, ang)`: [`streaks`], and for an angle other than 0 (Python's `if ang:`)
/// `_streaks_turned`: the streaks shoot out of both ends of the turned word along its direction, one pixel every
/// 0.7 px of their length.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn streaks_at(
    img: &mut RgbF,
    grid: &Grid,
    t: f64,
    j: usize,
    cx: f64,
    cy: f64,
    w: i32,
    h: i32,
    col: [f64; 3],
    ang: f64,
) {
    let u = (t - grid.title[j] - frame(grid)) / 0.25;
    if !(0.0..1.0).contains(&u) {
        return;
    }
    if ang != 0.0 {
        streaks_turned(img, u, j, cx, cy, w, h, col, ang);
        return;
    }
    let mut rg = rng(j as u64);
    let (w, h) = (f64::from(w), f64::from(h));
    let hot = col.map(|c| np::clip(c * 0.5 + 140.0, 0.0, 255.0));
    for i in 0..14 {
        let y = int(cy + rg.uniform(-0.6, 0.6) * h);
        let side = if i % 2 == 1 { 1.0 } else { -1.0 };
        let x_in = cx + side * (w / 2.0 + 4.0 + 40.0 * u);
        let len = rg.uniform(30.0, 80.0) * (1.0 - u);
        // xa, xb = sorted((int(x_in), int(x_in + side * L)));  xa, xb = max(0, xa), min(W, xb)
        let (p, q) = (int(x_in), int(x_in + side * len));
        let (xa, xb) = (p.min(q).max(0), p.max(q).min(W as i32));
        if 0 <= y && y < H as i32 && xb > xa {
            for x in xa..xb {
                // img[y, xa:xb] * (0.35 + 0.65 * u) + np.clip(col * 0.5 + 140, 0, 255) * (0.65 * (1 - u))
                for (v, hc) in img.px_mut(x as usize, y as usize).iter_mut().zip(hot) {
                    *v = *v * (0.35 + 0.65 * u) + hc * (0.65 * (1.0 - u));
                }
            }
        }
    }
}

/// The direction along a word turned `ang` degrees and the normal to it, `D = (cos a, -sin a)` and
/// `Nn = (sin a, cos a)`.
fn turned_axes(ang: f64) -> ([f64; 2], [f64; 2]) {
    let a = layout::radians(ang);
    let (c, s) = (dmath::cos(a), dmath::sin(a));
    ([c, -s], [s, c])
}

/// `_streaks_turned(img, u, j, cx, cy, w, h, col, ang)`.
#[allow(clippy::too_many_arguments)] // the Python signature
fn streaks_turned(img: &mut RgbF, u: f64, j: usize, cx: f64, cy: f64, w: i32, h: i32, col: [f64; 3], ang: f64) {
    let (d, nn) = turned_axes(ang);
    let mut rg = rng(j as u64);
    let (w, h) = (f64::from(w), f64::from(h));
    let hot = col.map(|c| np::clip(c * 0.5 + 140.0, 0.0, 255.0));
    for i in 0..14 {
        let off = rg.uniform(-0.6, 0.6) * h;
        let side = if i % 2 == 1 { 1.0 } else { -1.0 };
        let s_in = w / 2.0 + 4.0 + 40.0 * u;
        let len = rg.uniform(30.0, 80.0) * (1.0 - u);
        for s in arange(s_in, s_in + len, 0.7) {
            // x, y = (c + D * side * s_ + Nn * off).astype(int)
            let x = np::f64_to_i64(cx + d[0] * side * s + nn[0] * off);
            let y = np::f64_to_i64(cy + d[1] * side * s + nn[1] * off);
            if (0..W as i64).contains(&x) && (0..H as i64).contains(&y) {
                for (v, hc) in img.px_mut(x as usize, y as usize).iter_mut().zip(hot) {
                    *v = *v * (0.35 + 0.65 * u) + hc * (0.65 * (1.0 - u));
                }
            }
        }
    }
}

/// The colour of the dust, `np.array([236, 226, 214])`.
const DUST: [f64; 3] = [236.0, 226.0, 214.0];

/// `dust(img, t, j, cx, cy, w, h)`: a dust burst along the base of a slamming word: twelve puffs rolling outward,
/// growing and fading over 0.6 s, each a dithered disc mixed into the picture.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn dust(img: &mut RgbF, grid: &Grid, t: f64, j: usize, cx: f64, cy: f64, w: i32, h: i32) {
    dust_at(img, grid, t, j, cx, cy, w, h, 0.0);
}

/// `dust(img, t, j, cx, cy, w, h, ang)`: [`dust`], and for an angle other than 0 each puff in the turned word's
/// own frame, along its base line.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn dust_at(img: &mut RgbF, grid: &Grid, t: f64, j: usize, cx: f64, cy: f64, w: i32, h: i32, ang: f64) {
    let u = (t - grid.title[j] - frame(grid)) / 0.6;
    if !(0.0..1.0).contains(&u) {
        return;
    }
    let mut rg = rng(j as u64 + 10);
    let (d, nn) = turned_axes(ang);
    let (w, h) = (f64::from(w), f64::from(h));
    for _ in 0..12 {
        let x0 = cx + rg.uniform(-0.55, 0.55) * w;
        let vx = (x0 - cx) / (w / 2.0 + 1e-6) * rg.uniform(40.0, 90.0);
        // (1 - u) ** 2 is a Python float squared: a product here
        let x = x0 + vx * (1.0 - (1.0 - u) * (1.0 - u)) * 0.8;
        let y = cy + h * 0.5 + rg.uniform(-4.0, 4.0) - 18.0 * u;
        let (x, y) = if ang != 0.0 {
            // x, y = np.array([cx, cy]) + D * (x - cx) + Nn * (y - cy)
            (
                cx + d[0] * (x - cx) + nn[0] * (y - cy),
                cy + d[1] * (x - cx) + nn[1] * (y - cy),
            )
        } else {
            (x, y)
        };
        let rad = 3.0 + 12.0 * u * rg.uniform(0.7, 1.2);
        let a = 0.55 * dmath::pow(1.0 - u, 1.5);
        let (x0i, x1i) = (int(py_max(0.0, x - rad)), int(py_min(W as f64, x + rad + 1.0)));
        let (y0i, y1i) = (int(py_max(0.0, y - rad)), int(py_min(H as f64, y + rad + 1.0)));
        if x1i <= x0i || y1i <= y0i {
            continue;
        }
        for yy in y0i..y1i {
            for xx in x0i..x1i {
                // dd = ((xx - x) ** 2 + (yy - y) ** 2) / (rad * rad)
                let (ex, ey) = (f64::from(xx) - x, f64::from(yy) - y);
                let dd = (ex * ex + ey * ey) / (rad * rad);
                let m = np::clip(1.0 - dd, 0.0, 1.0);
                // m = np.floor(m * 3 + BAYER_FULL[y0i:y1i, x0i:x1i] * 0.9) / 3
                let m = (m * 3.0 + config::BAYER[yy as usize % 4][xx as usize % 4] * 0.9).floor() / 3.0;
                // img += (np.array([236, 226, 214]) - img) * (m * a)[..., None]
                for (v, d) in img.px_mut(xx as usize, yy as usize).iter_mut().zip(DUST) {
                    *v += (d - *v) * (m * a);
                }
            }
        }
    }
}

/// `sparks(img, t)`: twenty gold sparks flying out from `VS` (at (180, 300)) and falling, for 0.45 s from its
/// slam; each is five pixels along its last 0.03 s of flight.
pub fn sparks(img: &mut RgbF, grid: &Grid, t: f64) {
    sparks_from(img, grid, t, (180.0, 300.0));
}

/// `sparks(img, t)` with the sparks flying out from `p0`: `(180, 300)` for `VS`, the centre of a turned middle
/// word (`WORDS['VS']['cx'], ['cy']` when it has an angle).
pub fn sparks_from(img: &mut RgbF, grid: &Grid, t: f64, p0: (f64, f64)) {
    let u = t - grid.title[1];
    if !(0.0..0.45).contains(&u) {
        return;
    }
    let mut rg = rng(99);
    for _ in 0..20 {
        let a = rg.uniform(0.0, 2.0 * dmath::PI);
        let v = rg.uniform(90.0, 220.0);
        let dir = [dmath::cos(a), dmath::sin(a)];
        // p = p0 + v * u * [cos, sin] + [0, 160 * u * u];  q = p - v * 0.03 * [cos, sin]
        let p = [p0.0 + v * u * dir[0] + 0.0, p0.1 + v * u * dir[1] + 160.0 * u * u];
        let q = [p[0] - v * 0.03 * dir[0], p[1] - v * 0.03 * dir[1]];
        let k = 1.0 - u / 0.45;
        // np.linspace(0, 1, 5)
        for s in [0.0, 0.25, 0.5, 0.75, 1.0] {
            // x, y = (q + (p - q) * s).astype(int)
            let x = np::f64_to_i64(q[0] + (p[0] - q[0]) * s);
            let y = np::f64_to_i64(q[1] + (p[1] - q[1]) * s);
            if 0 <= x && x < W as i64 && 0 <= y && y < H as i64 {
                for (c, g) in img.px_mut(x as usize, y as usize).iter_mut().zip(GOLD) {
                    *c = over(*c, g, k);
                }
            }
        }
    }
}

/// `crack_pts()`: the crack along the divider from `VS` (at (180, 320)) out to both edges of the frame: two
/// jagged polylines and the short branches off them, in the order they are drawn (each branch before the rest
/// of its line is finished, the line itself last). The same for every card: the seed is 77.
pub fn crack_pts() -> Vec<Vec<(f64, f64)>> {
    let mut rg = Generator::new(77);
    let (dd, n) = (layout::dd(), layout::normal());
    let c = [180.0, 320.0];
    let mut lines = Vec::new();
    for sgn in [1.0, -1.0] {
        let mut p = c;
        let mut pts = vec![(c[0], c[1])];
        while 0.0 <= p[0] && p[0] <= W as f64 {
            // p = p + DD * sgn * rg.uniform(10, 18) + N * rg.uniform(-4, 4)
            let (along, across) = (rg.uniform(10.0, 18.0), rg.uniform(-4.0, 4.0));
            p = [0, 1].map(|i| p[i] + dd[i] * sgn * along + n[i] * across);
            pts.push((p[0], p[1]));
            if rg.random() < 0.18 {
                // a branch: three more steps, each to one side or the other
                let mut q = p;
                let mut br = vec![(p[0], p[1])];
                for _ in 0..3 {
                    // q = q + DD * sgn * rg.uniform(5, 9) + N * rg.choice([-1, 1]) * rg.uniform(5, 10)
                    let along = rg.uniform(5.0, 9.0);
                    let turn = [-1.0, 1.0][rg.choice_index(2)];
                    let across = rg.uniform(5.0, 10.0);
                    q = [0, 1].map(|i| q[i] + dd[i] * sgn * along + n[i] * turn * across);
                    br.push((q[0], q[1]));
                }
                lines.push(br);
            }
        }
        lines.push(pts);
    }
    lines
}

/// `draw_crack(layer, alpha, t)`: `VS` cracks the divider: the crack runs out from the centre over 0.12 s, white
/// gold at first and cooling to ink, opaque. `crack` is [`crack_pts`]; each polyline is a Pillow line of width 2.
pub fn draw_crack(layer: &mut RgbF, alpha: &mut Img<f64>, grid: &Grid, t: f64, crack: &[Vec<(f64, f64)>]) {
    let u = t - grid.title[1];
    if u < 0.0 {
        return;
    }
    let grow = py_min(1.0, u / 0.12);
    let mut im: Img<u8> = Img::new(layer.w, layer.h, 1);
    for pts in crack {
        // n = max(2, int(len(pts) * grow));  d.line(pts[:n], fill=255, width=2)
        let n = (np::f64_to_i64(pts.len() as f64 * grow).max(2) as usize).min(pts.len());
        pil::line(&mut im, &pts[..n], &[255], 2);
    }
    let hot = dmath::exp(-u / 0.12);
    let c = [0, 1, 2].map(|i| INK[i] * (1.0 - hot) + GOLD[i] * hot);
    for (i, &v) in im.data.iter().enumerate() {
        // m = np.asarray(im, np.float32) / 255.0 > 0.5
        if f32::from(v) / 255.0 > 0.5 {
            layer.data[i * 3..i * 3 + 3].copy_from_slice(&c);
            alpha.data[i] = 1.0;
        }
    }
}
