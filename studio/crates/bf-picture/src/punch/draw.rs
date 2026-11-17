//! The Punchy look's drawing: shockwaves on the floor, impact glows and the floor's bar pools, cracks on a low
//! ball, the K.O.'s grey-out and speed lines, the heartbeat on a low fighter's HP bar.

use bf_core::dmath;

use super::{LOW_HP, Punch};
use crate::camera::{Cam, V3};
use crate::config::ss;
use crate::img::{Img, Rgb8};
use crate::look::DrawnBall;
use crate::raster::Frame;
use crate::scene::{self, R_BALL as R};
use crate::ui::hud::PxBox;

/// Adds `col * k` to the pixel at `(x, y)` with a soft 2x2 footprint.
fn splat<T: Copy + Into<f64> + FromF64>(img: &mut Img<T>, x: f64, y: f64, col: V3, k: f64) {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    for (dx, dy, w) in [
        (0, 0, (1.0 - fx) * (1.0 - fy)),
        (1, 0, fx * (1.0 - fy)),
        (0, 1, (1.0 - fx) * fy),
        (1, 1, fx * fy),
    ] {
        let (px, py) = (x0 as i64 + dx, y0 as i64 + dy);
        if px < 0 || py < 0 || px >= img.w as i64 || py >= img.h as i64 {
            continue;
        }
        let p = img.px_mut(px as usize, py as usize);
        for c in 0..3 {
            p[c] = T::from_f64(p[c].into() + col[c] * k * w);
        }
    }
}

/// The sample types the drawing here writes.
pub trait FromF64 {
    fn from_f64(v: f64) -> Self;
}
impl FromF64 for f32 {
    fn from_f64(v: f64) -> f32 {
        v as f32
    }
}
impl FromF64 for f64 {
    fn from_f64(v: f64) -> f64 {
        v
    }
}

/// A small deterministic generator for the cracks' shapes.
struct Mix(u64);
impl Mix {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

impl Punch {
    /// The shockwave rings on the floor at the contact of each recent hit (after the painter, before the
    /// sparks): an ellipse in the attacker's colour that grows and fades over 0.35 s.
    pub fn shockwaves(&self, arr: &mut Img<f32>, cam: &Cam, t: f64) {
        for (h, u) in self.recent(t, 0.35) {
            let v = u / 0.35;
            let k = (1.0 - v) * (1.0 - v) * (0.45 + 0.55 * h.s);
            if k < 0.02 {
                continue;
            }
            let r = R * (0.7 + (1.6 + 1.6 * h.s) * (1.0 - (1.0 - v) * (1.0 - v)));
            let n = 72;
            let pts: Vec<V3> = (0..n)
                .map(|i| {
                    let a = f64::from(i) / f64::from(n) * 2.0 * dmath::PI;
                    scene::to_world(h.x + r * dmath::cos(a), h.z + r * dmath::sin(a), 0.03)
                })
                .collect();
            let col = self.col[h.att].map(|c| c * 0.6 + 100.0);
            let sp = cam.wp(&pts);
            for i in 0..sp.len() {
                let (a, b) = (sp[i], sp[(i + 1) % sp.len()]);
                let steps = (hyp(b.0 - a.0, b.1 - a.1)).ceil().max(1.0) as usize;
                for s in 0..steps {
                    let f = s as f64 / steps as f64;
                    splat(arr, a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f, col, 0.85 * k);
                }
            }
        }
    }

    /// The impact glows of the recent hits and the floor's pools that bloom on each bar (into the glow list).
    pub fn glows(&self, fr: &mut Frame, cam: &Cam, t: f64, bar: f64, lights: [V3; 2]) {
        for (h, u) in self.recent(t, 0.2) {
            let k = (1.0 - u / 0.2) * (0.25 + 0.4 * h.s);
            let s = cam.wp(&[scene::to_world(h.x, h.z, R)])[0];
            fr.glow(s.0, s.1, 14.0 + 20.0 * h.s, self.col[h.att].map(|c| c * 0.6 + 100.0), k);
        }
        let bp = self.bar_pulse(t, bar);
        if bp > 0.03 {
            for (k, lp) in lights.iter().enumerate() {
                let s = cam.wp(&[*lp])[0];
                fr.glow(s.0, s.1, 26.0, self.col[k], 0.22 * bp);
            }
        }
    }

    /// The cracks on a low fighter's ball: lines from near its centre out to its rim, more with every hit taken
    /// under [`LOW_HP`], each dark with a thin light edge.
    pub fn cracks(&self, arr: &mut Img<f32>, drawn: &[DrawnBall], t: f64, seed: u64) {
        for d in drawn {
            let Some(n) = self.low(d.k, t) else { continue };
            let lines = (1 + 2 * n).min(14);
            let mut rng = Mix(seed ^ (0x9E37_79B9_7F4A_7C15u64.wrapping_mul(d.k as u64 + 1)) | 1);
            for _ in 0..lines {
                let a0 = rng.next() * 2.0 * dmath::PI;
                let r0 = 0.15 + 0.25 * rng.next();
                let mut p = (d.s.0 + d.rp * r0 * dmath::cos(a0), d.s.1 + d.rp * r0 * dmath::sin(a0));
                let mut a = a0 + (rng.next() - 0.5) * 0.6;
                let segs = 3 + (rng.next() * 3.0) as usize;
                for _ in 0..segs {
                    let len = d.rp * (0.18 + 0.14 * rng.next());
                    let q = (p.0 + len * dmath::cos(a), p.1 + len * dmath::sin(a));
                    let steps = (len.ceil() as usize).max(1) * 2;
                    for s in 0..steps {
                        let f = s as f64 / steps as f64;
                        let (x, y) = (p.0 + (q.0 - p.0) * f, p.1 + (q.1 - p.1) * f);
                        if hyp(x - d.s.0, y - d.s.1) > d.rp * 0.93 {
                            break;
                        }
                        darken(arr, x, y, 0.55);
                        splat(arr, x + 0.6, y - 0.6, [255.0, 250.0, 240.0], 0.18);
                    }
                    p = q;
                    a += (rng.next() - 0.5) * 1.1;
                }
            }
        }
    }

    /// The K.O.: everything but a disc around the loser goes grey through the hit-stop and the slow-mo, and speed
    /// lines burst out of the K.O. point. On the graded picture (0..255).
    pub fn ko_grade(&self, img: &mut Img<f64>, cam: &Cam, t: f64) {
        let end = self.t_hs + 0.9;
        if t < self.t_ko || t >= end + 0.6 {
            return;
        }
        let amt = 0.85 * ss(self.t_ko, self.t_ko + 0.12, t) * (1.0 - ss(end, end + 0.6, t));
        let c = cam.wp(&[scene::to_world(self.ko_pt[0], self.ko_pt[1], R)])[0];
        let keep_r = 46.0 + 50.0 * self.ko_zoom(t);
        if amt > 0.01 {
            for y in 0..img.h {
                for x in 0..img.w {
                    let d = hyp(x as f64 - c.0, y as f64 - c.1);
                    let a = amt * ss(keep_r * 0.8, keep_r * 1.6, d);
                    if a <= 0.0 {
                        continue;
                    }
                    let p = img.px_mut(x, y);
                    let l = 0.3 * p[0] + 0.59 * p[1] + 0.11 * p[2];
                    let l = l * 0.85;
                    for v in p.iter_mut().take(3) {
                        *v = *v * (1.0 - a) + l * a;
                    }
                }
            }
        }
        // speed lines for the first 0.9 s
        let u = t - self.t_ko;
        if u < 0.9 {
            let k = (1.0 - u / 0.9) * 0.55;
            let n = 30;
            for i in 0..n {
                let a = f64::from(i) / f64::from(n) * 2.0 * dmath::PI + 0.11 * f64::from(i % 3);
                let r0 = keep_r * (1.1 + 0.25 * f64::from(i % 4));
                let r1 = r0 + 90.0 + 160.0 * u + 30.0 * f64::from(i % 5);
                let steps = (r1 - r0).ceil() as usize;
                for s in 0..steps {
                    let r = r0 + s as f64;
                    let fade = 1.0 - s as f64 / steps as f64;
                    splat(
                        img,
                        c.0 + r * dmath::cos(a),
                        c.1 + r * dmath::sin(a),
                        [255.0, 245.0, 225.0],
                        k * fade,
                    );
                }
            }
            for v in &mut img.data {
                *v = v.clamp(0.0, 255.0);
            }
        }
    }

    /// The heartbeat on a low fighter's HP bar: a "lub-dub" glow around the bar, faster as its HP falls.
    pub fn heartbeat(&self, img: &mut Rgb8, bars: &[PxBox; 2], hp: [f64; 2], t: f64) {
        for k in 0..2 {
            if self.low(k, t).is_none() {
                continue;
            }
            let bpm = 70.0 + 60.0 * (1.0 - (hp[k] / LOW_HP).clamp(0.0, 1.0));
            let ph = (t * bpm / 60.0).fract();
            let lub = dmath::exp(-ph / 0.06);
            let dub = if ph > 0.22 {
                0.7 * dmath::exp(-(ph - 0.22) / 0.07)
            } else {
                0.0
            };
            let k_ = 0.75 * lub.max(dub);
            if k_ < 0.03 {
                continue;
            }
            let (xa, xb, y0, y1) = bars[k];
            let red = [255.0, 70.0, 60.0];
            for y in (y0 - 2)..(y1 + 2) {
                for x in (xa - 2)..(xb + 2) {
                    let inside = x >= xa && x < xb && y >= y0 && y < y1;
                    if x < 0 || y < 0 || x >= img.w as i32 || y >= img.h as i32 {
                        continue;
                    }
                    let a = if inside { 0.25 * k_ } else { k_ };
                    let p = img.px_mut(x as usize, y as usize);
                    for c in 0..3 {
                        p[c] = (f64::from(p[c]) * (1.0 - a) + red[c] * a).round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
}

fn hyp(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// Darkens the pixel at `(x, y)` by `k` (a crack's line).
fn darken(arr: &mut Img<f32>, x: f64, y: f64, k: f64) {
    let (px, py) = (x.round() as i64, y.round() as i64);
    if px < 0 || py < 0 || px >= arr.w as i64 || py >= arr.h as i64 {
        return;
    }
    for v in arr.px_mut(px as usize, py as usize).iter_mut().take(3) {
        *v = (f64::from(*v) * (1.0 - k)) as f32;
    }
}
