//! The anime layer of the Punchy look: every hit gets a local impact frame and a speed-line burst; abilities get
//! afterimages, shields and auras; each fighter's low-HP move becomes a special with dramatic lighting (the arena
//! drops to dark, god rays, rim light), a cut-in banner and its own big moment. The six close-quarters fighters
//! (Violin, Trombone, Accordion, Tuba, Cello, Bari Sax) also get their own weapon marks and specials.
//!
//! Everything is drawn on the graded picture (0..255) from the recording's events, in screen space.

use bf_core::dmath;

use super::{Punch, PunchHit};
use crate::camera::{Cam, V3};
use crate::config::ss;
use crate::img::{Img, Rgb8};
use crate::look::DrawnBall;
use crate::ring::PictureCtx;
use crate::scene::{self, R_BALL as R};
use crate::ui::DrawLog;

/// A fighter's anime style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Violin,
    Trombone,
    Accordion,
    Tuba,
    Cello,
    BariSax,
    /// Everyone else: the shared layer only.
    Plain,
}

impl Style {
    /// The style of the fighter named `name`.
    pub fn of(name: &str) -> Style {
        match name {
            "VIOLIN" => Style::Violin,
            "TROMBONE" => Style::Trombone,
            "ACCORDION" => Style::Accordion,
            "TUBA" => Style::Tuba,
            "CELLO" => Style::Cello,
            "BARI SAX" => Style::BariSax,
            _ => Style::Plain,
        }
    }
}

/// A low-HP special: from its ability event `t0` to its impact `t_hit` at `at` (ring frame).
#[derive(Clone, Debug, PartialEq)]
pub struct Special {
    pub k: usize,
    pub t0: f64,
    pub t_hit: f64,
    pub at: [f64; 2],
    pub name: String,
}

/// A stretch of time an ability lasts: `(fighter, start, end)`.
pub type Span = (usize, f64, f64);

/// What the anime layer reads, built with the [`Punch`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Anime {
    pub style: [Option<Style>; 2],
    pub specials: Vec<Special>,
    pub hastes: Vec<Span>,
    pub shields: Vec<Span>,
    /// Other abilities: an aura burst at `(fighter, time)`.
    pub pops: Vec<(usize, f64)>,
    pub names: [String; 2],
}

fn num(v: &serde_json::Value, key: &str) -> Option<f64> {
    v.get(key).and_then(serde_json::Value::as_f64)
}

impl Anime {
    /// The anime layer's data from the picture `p` and the Punchy hits.
    pub fn new(p: &PictureCtx, hits: &[PunchHit]) -> Anime {
        let beat = p.show.grid.beat;
        let mut a = Anime {
            style: [Some(Style::of(&p.names[0])), Some(Style::of(&p.names[1]))],
            names: p.names.clone(),
            ..Anime::default()
        };
        for e in &p.rec.events {
            if e.kind != "ability" || e.t >= p.t_ko {
                continue;
            }
            let Some(k) = crate::ring::owner(e) else { continue };
            let name = e.s(3).to_string();
            let abil = p.cfg[k].raw.get("abilities");
            let slot = abil.and_then(|ab| {
                ["unlock1", "unlock2", "low"]
                    .into_iter()
                    .find(|s| ab.get(*s).and_then(|x| x.get("name")).and_then(|n| n.as_str()) == Some(&name))
                    .map(|s| (s, &ab[s]))
            });
            let kind = e.s(4);
            let dur = |v: &serde_json::Value, default: f64| {
                num(v, "dur")
                    .or_else(|| num(v, "dur_beats").map(|b| b * beat))
                    .unwrap_or(default)
            };
            match slot {
                Some(("low", _)) => {
                    let hit = hits.iter().find(|h| h.att == k && h.t > e.t && h.t <= e.t + 4.0);
                    let (t_hit, at) = match hit {
                        Some(h) => (h.t, [h.x, h.z]),
                        None => {
                            let t = (e.t + 0.8).min(p.t_ko);
                            (t, p.run.p(p.sim_i(t), 1 - k))
                        }
                    };
                    a.specials.push(Special {
                        k,
                        t0: e.t,
                        t_hit,
                        at,
                        name,
                    });
                }
                Some((_, v)) if kind == "haste" => a.hastes.push((k, e.t, e.t + dur(v, 3.0))),
                Some((_, v)) if kind == "shield" => a.shields.push((k, e.t, e.t + dur(v, 6.0 * beat))),
                _ => a.pops.push((k, e.t)),
            }
        }
        a
    }

    /// The special at work at `t` (from 0.15 s before its event to 1.4 s after its impact), with its envelope.
    pub fn special_at(&self, t: f64) -> Option<(&Special, f64)> {
        self.specials
            .iter()
            .filter(|s| t >= s.t0 - 0.15 && t < s.t_hit + 1.4)
            .map(|s| {
                (
                    s,
                    ss(s.t0 - 0.15, s.t0 + 0.2, t) * (1.0 - ss(s.t_hit + 0.8, s.t_hit + 1.4, t)),
                )
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
    }
}

// ---------------------------------------------------------------- drawing kit (graded picture, 0..255)

fn hyp(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

fn mix(a: V3, b: V3, k: f64) -> V3 {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * k)
}

const WHITE: V3 = [255.0, 252.0, 245.0];

/// Adds `col * k` at `(x, y)` with a bilinear footprint.
fn add(img: &mut Img<f64>, x: f64, y: f64, col: V3, k: f64) {
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
            p[c] += col[c] * k * w;
        }
    }
}

/// A soft additive disc.
fn disc(img: &mut Img<f64>, x: f64, y: f64, r: f64, col: V3, k: f64) {
    if k <= 0.003 || r <= 0.0 {
        return;
    }
    let (xa, xb) = (
        (x - r).floor().max(0.0) as i64,
        (x + r).ceil().min(img.w as f64 - 1.0) as i64,
    );
    let (ya, yb) = (
        (y - r).floor().max(0.0) as i64,
        (y + r).ceil().min(img.h as f64 - 1.0) as i64,
    );
    for py in ya..=yb {
        for px in xa..=xb {
            let d = hyp(px as f64 - x, py as f64 - y) / r;
            if d < 1.0 {
                let w = (1.0 - d) * (1.0 - d);
                let p = img.px_mut(px as usize, py as usize);
                for c in 0..3 {
                    p[c] += col[c] * k * w;
                }
            }
        }
    }
}

/// An additive line of width `w`, fading from `k0` at `a` to `k1` at `b`.
fn line(img: &mut Img<f64>, a: (f64, f64), b: (f64, f64), col: V3, k0: f64, k1: f64, w: f64) {
    if k0.max(k1) <= 0.003 {
        return;
    }
    let len = hyp(b.0 - a.0, b.1 - a.1);
    let n = (len * 1.5).ceil().max(1.0) as usize;
    let (nx, ny) = if len > 1e-9 {
        (-(b.1 - a.1) / len, (b.0 - a.0) / len)
    } else {
        (0.0, 0.0)
    };
    let wn = (w.ceil() as i64).max(1);
    for i in 0..=n {
        let f = i as f64 / n as f64;
        let (x, y) = (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f);
        let k = k0 + (k1 - k0) * f;
        for j in 0..wn {
            let o = j as f64 - (wn - 1) as f64 / 2.0;
            add(
                img,
                x + nx * o,
                y + ny * o,
                col,
                k / (1.0 + 0.15 * (wn - 1) as f64) * 0.9,
            );
        }
    }
}

/// An additive ellipse outline (`jag` px of wobble at `teeth` per turn), squashed like the floor.
#[allow(clippy::too_many_arguments)]
fn ring(img: &mut Img<f64>, x: f64, y: f64, r: f64, sq: f64, col: V3, k: f64, w: f64, jag: f64, teeth: f64, ph: f64) {
    if k <= 0.003 || r <= 0.5 {
        return;
    }
    let n = ((r * 0.9).ceil() as usize).clamp(24, 240);
    let pt = |i: usize| {
        let a = i as f64 / n as f64 * 2.0 * dmath::PI;
        let rr = r + jag * dmath::sin(a * teeth + ph);
        (x + rr * dmath::cos(a), y + rr * sq * dmath::sin(a))
    };
    for i in 0..n {
        line(img, pt(i), pt(i + 1), col, k, k, w);
    }
}

/// A darkening multiply of the whole picture by `1 - k`, kept off soft discs `(x, y, r)`.
fn dim(img: &mut Img<f64>, k: f64, keep: &[(f64, f64, f64)]) {
    if k <= 0.003 {
        return;
    }
    for py in 0..img.h {
        for px in 0..img.w {
            let mut a = k;
            for &(x, y, r) in keep {
                let d = hyp(px as f64 - x, py as f64 - y);
                a *= ss(r * 0.7, r * 1.6, d);
            }
            if a <= 0.0 {
                continue;
            }
            for v in img.px_mut(px, py).iter_mut().take(3) {
                *v *= 1.0 - a;
            }
        }
    }
}

/// God rays: soft wedges of light from the top of the frame down onto `(x, y)`.
fn god_rays(img: &mut Img<f64>, x: f64, y: f64, col: V3, k: f64) {
    if k <= 0.01 {
        return;
    }
    for i in 0..7 {
        let off = (i as f64 - 3.0) * 26.0;
        let top = (x + off * 2.2, -10.0);
        let w = 10.0 + 6.0 * f64::from(i % 3);
        let n = 60;
        for j in 0..n {
            let f = j as f64 / n as f64;
            let (cx, cy) = (top.0 + (x + off * 0.3 - top.0) * f, top.1 + (y - top.1) * f);
            disc(img, cx, cy, w * (0.4 + 0.6 * f), col, 0.035 * k * f);
        }
    }
}

/// A local impact frame: inside a disc around `(x, y)` the picture turns to inked, inverted black and white,
/// with a dark rim. `k` 0..1.
fn impact_frame(img: &mut Img<f64>, x: f64, y: f64, r: f64, k: f64) {
    if k <= 0.01 {
        return;
    }
    let (xa, xb) = (
        (x - r - 3.0).floor().max(0.0) as usize,
        (x + r + 3.0).ceil().min(img.w as f64 - 1.0) as usize,
    );
    let (ya, yb) = (
        (y - r - 3.0).floor().max(0.0) as usize,
        (y + r + 3.0).ceil().min(img.h as f64 - 1.0) as usize,
    );
    for py in ya..=yb {
        for px in xa..=xb {
            let d = hyp(px as f64 - x, py as f64 - y);
            let p = img.px_mut(px, py);
            if d < r {
                let l = 0.3 * p[0] + 0.59 * p[1] + 0.11 * p[2];
                // posterised, inverted ink
                let inv = if l > 150.0 {
                    12.0
                } else if l > 70.0 {
                    120.0
                } else {
                    248.0
                };
                for v in p.iter_mut().take(3) {
                    *v = *v * (1.0 - k) + inv * k;
                }
            } else if d < r + 3.0 {
                for v in p.iter_mut().take(3) {
                    *v *= 1.0 - 0.9 * k;
                }
            }
        }
    }
}

/// Radial anime speed lines from `(x, y)`.
fn speed_lines(img: &mut Img<f64>, x: f64, y: f64, r0: f64, k: f64, seed: u64, col: V3) {
    if k <= 0.01 {
        return;
    }
    let mut s = Mix(seed | 1);
    for _ in 0..24 {
        let a = s.next() * 2.0 * dmath::PI;
        let r_a = r0 * (1.0 + 0.8 * s.next());
        let r_b = r_a + 50.0 + 110.0 * s.next();
        let w = 1.0 + 1.5 * s.next() * k;
        let (ca, sa) = (dmath::cos(a), dmath::sin(a));
        line(
            img,
            (x + ca * r_a, y + sa * r_a),
            (x + ca * r_b, y + sa * r_b),
            col,
            0.45 * k,
            0.0,
            w,
        );
    }
}

/// Jagged glowing floor cracks from `(x, y)`.
fn cracks(img: &mut Img<f64>, x: f64, y: f64, len: f64, k: f64, col: V3, seed: u64) {
    if k <= 0.01 {
        return;
    }
    let mut s = Mix(seed | 1);
    for _ in 0..9 {
        let mut a = s.next() * 2.0 * dmath::PI;
        let (mut px, mut py) = (x, y);
        for _ in 0..4 {
            a += (s.next() - 0.5) * 0.9;
            let l = len * (0.15 + 0.2 * s.next()) * k;
            let (qx, qy) = (px + dmath::cos(a) * l, py + dmath::sin(a) * l * 0.55);
            darken_line(img, (px, py), (qx, qy), 0.75);
            line(img, (px, py), (qx, qy), col, 0.7 * k, 0.5 * k, 1.0);
            (px, py) = (qx, qy);
        }
    }
}

fn darken_line(img: &mut Img<f64>, a: (f64, f64), b: (f64, f64), k: f64) {
    let n = (hyp(b.0 - a.0, b.1 - a.1) * 1.5).ceil().max(1.0) as usize;
    for i in 0..=n {
        let f = i as f64 / n as f64;
        let (x, y) = ((a.0 + (b.0 - a.0) * f).round(), (a.1 + (b.1 - a.1) * f + 1.0).round());
        if x >= 0.0 && y >= 0.0 && (x as usize) < img.w && (y as usize) < img.h {
            for v in img.px_mut(x as usize, y as usize).iter_mut().take(3) {
                *v *= 1.0 - k;
            }
        }
    }
}

/// A crescent smear: an arc of radius `r` around `(x, y)` from angle `a0` to `a1`, brighter and thicker at its
/// head (`a1`).
fn smear(img: &mut Img<f64>, x: f64, y: f64, r: f64, a0: f64, a1: f64, col: V3, k: f64, w: f64) {
    if k <= 0.01 {
        return;
    }
    let n = 28;
    for i in 0..n {
        let f = i as f64 / n as f64;
        let (aa, ab) = (a0 + (a1 - a0) * f, a0 + (a1 - a0) * (f + 1.0 / n as f64));
        let pa = (x + r * dmath::cos(aa), y + r * 0.8 * dmath::sin(aa));
        let pb = (x + r * dmath::cos(ab), y + r * 0.8 * dmath::sin(ab));
        line(img, pa, pb, mix(col, WHITE, f * 0.6), k * f, k * f, 1.0 + w * f);
    }
}

/// Zigzag jaws closing on `(x, y)` from the direction `ang`: `open` 1 wide open, 0 shut.
fn jaws(img: &mut Img<f64>, x: f64, y: f64, ang: f64, size: f64, open: f64, col: V3, k: f64) {
    for s in [-1.0, 1.0] {
        let a = ang + s * open * 0.75;
        let (ca, sa) = (dmath::cos(a), dmath::sin(a));
        let (nx, ny) = (-sa * s, ca * s);
        let base = (x - ca * size, y - sa * size);
        let mut prev = base;
        for i in 1..=8 {
            let f = i as f64 / 8.0;
            let tooth = if i % 2 == 1 { 7.0 } else { 0.0 };
            let q = (base.0 + ca * size * f + nx * tooth, base.1 + sa * size * f + ny * tooth);
            line(img, prev, q, mix(col, WHITE, 0.5), k, k, 2.0);
            prev = q;
        }
    }
}

/// A small deterministic generator.
struct Mix(u64);
impl Mix {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ---------------------------------------------------------------- the layer

impl Punch {
    fn screen(&self, cam: &Cam, x: f64, z: f64, y: f64) -> (f64, f64) {
        cam.wp(&[scene::to_world(x, z, y)])[0]
    }

    /// The anime layer on the graded picture at `t` (after the K.O. grade, before the crisp overlays).
    pub fn anime(&self, p: &PictureCtx, img: &mut Img<f64>, cam: &Cam, t: f64, drawn: &[DrawnBall]) {
        let an = &self.anime;
        let ball = |k: usize| drawn.iter().find(|d| d.k == k).map(|d| (d.s, d.rp));
        // ---- dramatic lighting under a special
        let special = an.special_at(t);
        if let Some((sp, env)) = special {
            let keep: Vec<(f64, f64, f64)> = drawn.iter().map(|d| (d.s.0, d.s.1, d.rp * 2.4)).collect();
            dim(img, 0.6 * env, &keep);
            let at = self.screen(cam, sp.at[0], sp.at[1], R);
            let col = mix(self.col[sp.k], WHITE, 0.35);
            god_rays(img, at.0, at.1, col, env);
            // rim light on both balls in the attacker's colour
            for d in drawn {
                ring(
                    img,
                    d.s.0,
                    d.s.1,
                    d.rp + 1.5,
                    1.0,
                    mix(self.col[sp.k], WHITE, 0.5),
                    0.9 * env,
                    2.0,
                    0.0,
                    0.0,
                    0.0,
                );
            }
        }
        // ---- abilities: afterimages, shields, aura pops
        for &(k, t0, t1) in &an.hastes {
            if t < t0 || t >= t1 || t >= self.t_ko {
                continue;
            }
            let Some((_, rp)) = ball(k) else { continue };
            let fade = ss(t0, t0 + 0.2, t) * (1.0 - ss(t1 - 0.3, t1, t));
            for (j, dt) in [0.05, 0.1, 0.15, 0.2].into_iter().enumerate() {
                let q = p.run.p(p.sim_i((t - dt).max(t0)), k);
                let s = self.screen(cam, q[0], q[1], R);
                let col = mix(self.col[k], WHITE, 0.2);
                disc(img, s.0, s.1, rp * 1.05, col, 0.32 * fade * (1.0 - j as f64 * 0.2));
            }
        }
        for &(k, t0, t1) in &an.shields {
            if t < t0 || t >= t1 || t >= self.t_ko {
                continue;
            }
            let Some((s, rp)) = ball(k) else { continue };
            let fade = ss(t0, t0 + 0.15, t) * (1.0 - ss(t1 - 0.25, t1, t));
            let r = rp * (1.55 + 0.06 * dmath::sin(t * 9.0));
            let col = mix(self.col[k], WHITE, 0.45);
            let n = 6;
            for i in 0..n {
                let a0 = i as f64 / n as f64 * 2.0 * dmath::PI + t * 0.8;
                let a1 = (i + 1) as f64 / n as f64 * 2.0 * dmath::PI + t * 0.8;
                let pa = (s.0 + r * dmath::cos(a0), s.1 + r * dmath::sin(a0));
                let pb = (s.0 + r * dmath::cos(a1), s.1 + r * dmath::sin(a1));
                line(img, pa, pb, col, 0.8 * fade, 0.8 * fade, 2.0);
                line(img, (s.0, s.1), pa, col, 0.12 * fade, 0.25 * fade, 1.0);
            }
            disc(img, s.0, s.1, r, self.col[k], 0.12 * fade);
            // blocks crack it with sparks
            for e in p.events_between(t - 0.25, t, &["block"]) {
                let e = &p.rec.events[e];
                if crate::ring::owner(e) == Some(k) {
                    let u = t - e.t;
                    ring(
                        img,
                        s.0,
                        s.1,
                        r + u * 120.0,
                        1.0,
                        WHITE,
                        1.0 - u * 4.0,
                        2.0,
                        3.0,
                        9.0,
                        0.0,
                    );
                }
            }
        }
        for &(k, t0) in &an.pops {
            let u = t - t0;
            if !(0.0..0.6).contains(&u) {
                continue;
            }
            let Some((s, rp)) = ball(k) else { continue };
            let col = mix(self.col[k], WHITE, 0.3);
            ring(
                img,
                s.0,
                s.1,
                rp + u * 160.0,
                1.0,
                col,
                1.0 - u / 0.6,
                3.0,
                4.0,
                12.0,
                u * 20.0,
            );
            speed_lines(img, s.0, s.1, rp * 1.6, 0.7 * (1.0 - u / 0.6), 77 + k as u64, col);
        }
        // ---- the weapons' own marks, around every hit
        let lo = self.hits.partition_point(|h| h.t < t - 0.7);
        let hi = self.hits.partition_point(|h| h.t <= t + 0.3);
        for (i, h) in self.hits[lo..hi].iter().enumerate() {
            let u = t - h.t;
            let st = an.style[h.att].unwrap_or(Style::Plain);
            let c = self.screen(cam, h.x, h.z, R);
            let qa = p.run.p(p.sim_i(h.t), h.att);
            let a = self.screen(cam, qa[0], qa[1], R);
            let col = self.col[h.att];
            let ang = dmath::atan2(c.1 - a.1, c.0 - a.0);
            // wind-up glint
            if st != Style::Plain && (-0.1..0.0).contains(&u) {
                let g = 1.0 + u / 0.1;
                disc(
                    img,
                    a.0 + dmath::cos(ang) * 10.0,
                    a.1 + dmath::sin(ang) * 10.0 - 6.0,
                    4.0 + 6.0 * g,
                    WHITE,
                    0.9 * g,
                );
            }
            self.weapon_mark(img, st, h, u, a, c, ang, col, (lo + i) as u64);
            if u >= 0.0 && h.s >= 0.45 {
                // the shared layer: speed lines and an impact frame on every hit
                speed_lines(
                    img,
                    c.0,
                    c.1,
                    22.0,
                    (1.0 - u / 0.3).max(0.0) * (0.45 + 0.55 * h.s),
                    (lo + i) as u64 * 31 + 7,
                    WHITE,
                );
            }
        }
        // ---- the specials' own moments
        if let Some((sp, env)) = special {
            self.special(p, img, cam, t, sp, env, &ball);
        }
        // impact frames last: they ink what is under them
        for h in &self.hits[lo..hi] {
            let u = t - h.t;
            if (0.0..0.012).contains(&u) && h.s >= 0.4 {
                let c = self.screen(cam, h.x, h.z, R);
                impact_frame(img, c.0, c.1, 34.0 + 40.0 * h.s, 1.0);
            }
        }
        for sp in &an.specials {
            let u = t - sp.t_hit;
            if (0.0..0.03).contains(&u) {
                let c = self.screen(cam, sp.at[0], sp.at[1], R);
                impact_frame(img, c.0, c.1, 95.0, if u < 0.015 { 1.0 } else { 0.6 });
            }
        }
        for v in &mut img.data {
            *v = v.clamp(0.0, 255.0);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn weapon_mark(
        &self,
        img: &mut Img<f64>,
        st: Style,
        h: &PunchHit,
        u: f64,
        a: (f64, f64),
        c: (f64, f64),
        ang: f64,
        col: V3,
        seed: u64,
    ) {
        let hot = mix(col, WHITE, 0.45);
        let s = 0.6 + 0.4 * h.s;
        match st {
            Style::Violin => {
                // a crescent stroke through the victim, then a crossed X that hangs
                if (-0.06..0.25).contains(&u) {
                    let k = if u < 0.0 { 1.0 + u / 0.06 } else { 1.0 - u / 0.25 };
                    let dir = if seed % 2 == 0 { 1.0 } else { -1.0 };
                    smear(img, c.0, c.1, 26.0 * s, ang - dir * 2.2, ang + dir * 0.3, col, k, 6.0);
                }
                if (0.0..0.35).contains(&u) {
                    let k = 1.0 - u / 0.35;
                    let r = 22.0 * s;
                    line(img, (c.0 - r, c.1 - r), (c.0 + r, c.1 + r), hot, k, k, 3.0);
                    line(img, (c.0 + r, c.1 - r), (c.0 - r, c.1 + r), hot, k, k, 3.0);
                }
            }
            Style::Trombone => {
                // a motion-blurred thrust from the bell, then a brass halo
                if (-0.06..0.15).contains(&u) {
                    let k = if u < 0.0 { 1.0 + u / 0.06 } else { 1.0 - u / 0.15 };
                    for j in 0..5 {
                        line(img, a, c, col, 0.06 * k, 0.25 * k, 14.0 - j as f64 * 3.0);
                    }
                    line(img, a, c, WHITE, 0.3 * k, 0.9 * k, 2.0);
                }
                if (0.0..0.4).contains(&u) {
                    let v = u / 0.4;
                    ring(
                        img,
                        c.0,
                        c.1,
                        10.0 + 70.0 * s * v,
                        1.0,
                        [255.0, 214.0, 120.0],
                        1.0 - v,
                        3.0,
                        0.0,
                        0.0,
                        0.0,
                    );
                }
            }
            Style::Accordion => {
                if (-0.12..0.05).contains(&u) {
                    let open = if u < 0.0 { -u / 0.12 } else { 0.0 };
                    jaws(img, c.0, c.1, ang, 30.0 * s, open, col, 1.0 - (u.max(0.0) / 0.05));
                }
                if (0.0..0.35).contains(&u) {
                    let v = u / 0.35;
                    ring(
                        img,
                        c.0,
                        c.1,
                        8.0 + 60.0 * s * v,
                        0.6,
                        hot,
                        1.0 - v,
                        3.0,
                        2.5,
                        16.0,
                        0.0,
                    );
                }
            }
            Style::Tuba => {
                // suck lines into the bell before, a boom of rings after
                if (-0.25..0.0).contains(&u) {
                    let f = 1.0 + u / 0.25;
                    let mut m = Mix(seed * 13 + 5);
                    for _ in 0..14 {
                        let aa = m.next() * 2.0 * dmath::PI;
                        let d = 60.0 * (1.0 - ((f + m.next()) % 1.0)) + 8.0;
                        let p0 = (a.0 + dmath::cos(aa) * d, a.1 + dmath::sin(aa) * d);
                        let p1 = (
                            a.0 + dmath::cos(aa + 0.4) * (d + 14.0),
                            a.1 + dmath::sin(aa + 0.4) * (d + 14.0),
                        );
                        line(img, p1, p0, col, 0.0, 0.7 * f, 1.5);
                    }
                }
                if (0.0..0.5).contains(&u) {
                    for j in 0..3 {
                        let v = (u - j as f64 * 0.07) / 0.43;
                        if (0.0..1.0).contains(&v) {
                            ring(
                                img,
                                c.0,
                                c.1,
                                10.0 + 80.0 * s * v,
                                0.6,
                                hot,
                                1.0 - v,
                                3.0,
                                0.0,
                                0.0,
                                0.0,
                            );
                        }
                    }
                }
            }
            Style::Cello => {
                // four humming strings from the cello to the victim, then a snap
                if (-0.22..0.05).contains(&u) {
                    let k = if u < 0.0 { 1.0 + u / 0.22 } else { 1.0 - u / 0.05 };
                    for j in 0..4 {
                        let off = (j as f64 - 1.5) * 3.0;
                        let (nx, ny) = (-dmath::sin(ang) * off, dmath::cos(ang) * off);
                        let n = 16;
                        let mut prev = (a.0 + nx, a.1 + ny);
                        for i in 1..=n {
                            let f = i as f64 / n as f64;
                            let wob = dmath::sin(f * 18.0 + u * 90.0 + j as f64) * 2.5 * (1.0 - f) * f * 4.0;
                            let q = (
                                a.0 + (c.0 - a.0) * f + nx - dmath::sin(ang) * wob,
                                a.1 + (c.1 - a.1) * f + ny + dmath::cos(ang) * wob,
                            );
                            line(img, prev, q, hot, 0.8 * k, 0.8 * k, 1.0);
                            prev = q;
                        }
                    }
                }
                if (0.0..0.3).contains(&u) {
                    let v = u / 0.3;
                    for j in 0..6 {
                        let aa = ang + (j as f64 - 2.5) * 0.5;
                        let r = 10.0 + 70.0 * v;
                        line(
                            img,
                            c,
                            (c.0 + dmath::cos(aa) * r, c.1 + dmath::sin(aa) * r * 0.8),
                            [255.0, 220.0, 150.0],
                            1.0 - v,
                            0.0,
                            1.5,
                        );
                    }
                }
            }
            Style::BariSax => {
                // a heavy arc of the wrecking ball into the victim, then a crater
                if (-0.14..0.06).contains(&u) {
                    let k = if u < 0.0 { 1.0 + u / 0.14 } else { 1.0 - u / 0.06 };
                    let r = hyp(c.0 - a.0, c.1 - a.1).max(20.0);
                    smear(img, a.0, a.1, r, ang - 1.6, ang, col, k, 12.0);
                }
                if (0.0..0.7).contains(&u) {
                    let v = u / 0.7;
                    cracks(
                        img,
                        c.0,
                        c.1 + 6.0,
                        70.0 * s,
                        (u / 0.1).min(1.0) * (1.0 - v),
                        [255.0, 150.0, 80.0],
                        seed + 3,
                    );
                    ring(
                        img,
                        c.0,
                        c.1 + 6.0,
                        10.0 + 90.0 * s * v,
                        0.55,
                        [255.0, 190.0, 120.0],
                        1.0 - v,
                        3.0,
                        0.0,
                        0.0,
                        0.0,
                    );
                }
            }
            Style::Plain => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn special(
        &self,
        p: &PictureCtx,
        img: &mut Img<f64>,
        cam: &Cam,
        t: f64,
        sp: &super::anime::Special,
        env: f64,
        ball: &dyn Fn(usize) -> Option<((f64, f64), f64)>,
    ) {
        let st = self.anime.style[sp.k].unwrap_or(Style::Plain);
        let col = self.col[sp.k];
        let hot = mix(col, WHITE, 0.5);
        let at = self.screen(cam, sp.at[0], sp.at[1], R);
        let own = ball(sp.k).map_or(at, |b| b.0);
        let build = ((t - sp.t0) / (sp.t_hit - sp.t0).max(0.05)).clamp(0.0, 1.0);
        let u = t - sp.t_hit;
        let seed = (sp.t0 * 1000.0) as u64;
        // the build-up
        if u < 0.0 {
            match st {
                Style::Trombone => {
                    // a pillar of light gathering on the target
                    for j in 0..6 {
                        let w = 26.0 - j as f64 * 4.0;
                        line(img, (at.0, -10.0), at, col, 0.05 * build * env, 0.12 * build * env, w);
                    }
                    line(img, (at.0, -10.0), at, WHITE, 0.4 * build, 0.6 * build, 2.0);
                }
                Style::Accordion => {
                    // the inhale: streaks pulled into the bellows
                    let mut m = Mix(seed + 11);
                    for _ in 0..40 {
                        let aa = m.next() * 2.0 * dmath::PI;
                        let d = 160.0 * (1.0 - ((t * 1.6 + m.next()) % 1.0)) + 10.0;
                        let p0 = (own.0 + dmath::cos(aa) * d, own.1 + dmath::sin(aa) * d);
                        let p1 = (own.0 + dmath::cos(aa) * (d + 18.0), own.1 + dmath::sin(aa) * (d + 18.0));
                        line(img, p1, p0, hot, 0.0, 0.8 * env, 1.5);
                    }
                }
                Style::Tuba => {
                    // purple lightning crackling round the tuba
                    let mut m = Mix(seed + (t * 20.0) as u64);
                    for _ in 0..4 {
                        let aa = m.next() * 2.0 * dmath::PI;
                        let mut q = own;
                        for _ in 0..7 {
                            let n = (
                                q.0 + dmath::cos(aa) * 10.0 + (m.next() - 0.5) * 16.0,
                                q.1 + dmath::sin(aa) * 10.0 + (m.next() - 0.5) * 16.0,
                            );
                            line(img, q, n, hot, 0.9 * env, 0.9 * env, 1.5);
                            q = n;
                        }
                    }
                }
                Style::Cello => {
                    // a warm spotlight and feathers drifting up
                    god_rays(img, own.0, own.1, [255.0, 225.0, 170.0], env);
                    feathers(img, own, t - sp.t0, seed, env);
                }
                Style::BariSax => {
                    // the meteor: the wrecking ball falls from the sky in the last 0.25 s
                    let f = ((t - (sp.t_hit - 0.25)) / 0.25).clamp(0.0, 1.0);
                    if f > 0.0 {
                        let y = -20.0 + (at.1 + 20.0) * f;
                        for j in 0..10 {
                            disc(
                                img,
                                at.0,
                                y - j as f64 * 14.0,
                                14.0 - j as f64,
                                mix(col, [255.0, 230.0, 120.0], j as f64 / 10.0),
                                0.35,
                            );
                        }
                        disc(img, at.0, y, 12.0, WHITE, 0.8);
                    }
                    line(
                        img,
                        (at.0, -10.0),
                        (at.0, at.1 - 160.0 * (1.0 - build)),
                        [230.0, 220.0, 210.0],
                        0.5 * env,
                        0.5 * env,
                        1.0,
                    );
                }
                Style::Violin | Style::Plain => {
                    let r = 14.0 + 30.0 * build;
                    ring(img, own.0, own.1, r, 1.0, hot, 0.7 * env, 2.0, 3.0, 10.0, t * 30.0);
                }
            }
            return;
        }
        // the impact
        let v = u / 1.2;
        match st {
            Style::Violin => {
                for j in 0..4 {
                    let uu = u - j as f64 * 0.16;
                    if uu <= 0.0 {
                        continue;
                    }
                    let r = uu * 230.0;
                    for (ci, c) in [[255.0, 70.0, 130.0], [120.0, 255.0, 255.0], col]
                        .into_iter()
                        .enumerate()
                    {
                        ring(
                            img,
                            own.0,
                            own.1,
                            r + ci as f64 * 3.0,
                            0.62,
                            c,
                            (1.0 - uu / 1.1).max(0.0) * 0.8,
                            2.0,
                            7.0,
                            14.0,
                            uu * 30.0,
                        );
                    }
                }
                cracks(
                    img,
                    own.0,
                    own.1 + 14.0,
                    170.0,
                    (u / 0.5).min(1.0) * (1.0 - v).max(0.0),
                    col,
                    seed,
                );
            }
            Style::Trombone => {
                cracks(
                    img,
                    at.0,
                    at.1 + 8.0,
                    200.0,
                    (u / 0.3).min(1.0) * (1.0 - v).max(0.0),
                    col,
                    seed,
                );
                ring(
                    img,
                    at.0,
                    at.1 + 8.0,
                    30.0 + u * 280.0,
                    0.55,
                    col,
                    (1.0 - u / 0.9).max(0.0),
                    5.0,
                    0.0,
                    0.0,
                    0.0,
                );
                ring(
                    img,
                    at.0,
                    at.1 + 8.0,
                    10.0 + u * 170.0,
                    0.55,
                    WHITE,
                    (1.0 - u / 0.5).max(0.0),
                    3.0,
                    0.0,
                    0.0,
                    0.0,
                );
                line(img, (at.0, -10.0), at, col, 0.0, 0.5 * (1.0 - u / 0.5).max(0.0), 30.0);
            }
            Style::Accordion => {
                if u < 0.18 {
                    let ang = dmath::atan2(at.1 - own.1, at.0 - own.0);
                    jaws(img, at.0, at.1, ang, 70.0, (0.18 - u) / 0.18 * 0.2, col, 1.0 - u / 0.18);
                }
                ring(
                    img,
                    at.0,
                    at.1,
                    20.0 + u * 260.0,
                    0.6,
                    col,
                    (1.0 - u / 0.9).max(0.0),
                    5.0,
                    3.0,
                    22.0,
                    0.0,
                );
                ring(
                    img,
                    at.0,
                    at.1,
                    10.0 + u * 160.0,
                    0.6,
                    WHITE,
                    (1.0 - u / 0.4).max(0.0),
                    3.0,
                    0.0,
                    0.0,
                    0.0,
                );
            }
            Style::Tuba => {
                // the dark dome, and the picture bending in waves
                let r = 20.0 + u * 300.0;
                let k = (1.0 - u / 1.0).max(0.0);
                for j in 0..6 {
                    ring(
                        img,
                        at.0,
                        at.1,
                        r - j as f64 * 4.0,
                        0.6,
                        col,
                        k * (0.6 - j as f64 * 0.09),
                        3.0,
                        0.0,
                        0.0,
                        0.0,
                    );
                }
                ring(img, at.0, at.1, r + 3.0, 0.6, WHITE, k, 2.0, 0.0, 0.0, 0.0);
                bend(img, u, k);
                cracks(img, at.0, at.1 + 8.0, 210.0, (u / 0.3).min(1.0) * k, col, seed);
            }
            Style::Cello => {
                feathers(img, own, t - sp.t0, seed, env);
                for j in 0..8 {
                    let aa = j as f64 * 0.8 + 0.3;
                    let r = 10.0 + u * 300.0;
                    line(
                        img,
                        at,
                        (at.0 + dmath::cos(aa) * r, at.1 + dmath::sin(aa) * r * 0.8),
                        [255.0, 220.0, 150.0],
                        (1.0 - u / 0.4).max(0.0),
                        0.0,
                        2.0,
                    );
                }
                ring(
                    img,
                    at.0,
                    at.1,
                    20.0 + u * 240.0,
                    0.6,
                    [255.0, 220.0, 160.0],
                    (1.0 - u / 0.8).max(0.0),
                    4.0,
                    0.0,
                    0.0,
                    0.0,
                );
            }
            Style::BariSax => {
                cracks(
                    img,
                    at.0,
                    at.1 + 10.0,
                    220.0,
                    (u / 0.3).min(1.0) * (1.0 - v).max(0.0),
                    [255.0, 140.0, 70.0],
                    seed,
                );
                ring(
                    img,
                    at.0,
                    at.1 + 10.0,
                    30.0 + u * 300.0,
                    0.55,
                    col,
                    (1.0 - u / 0.9).max(0.0),
                    5.0,
                    0.0,
                    0.0,
                    0.0,
                );
                disc(
                    img,
                    at.0,
                    at.1,
                    60.0 * (1.0 - u).max(0.0),
                    [255.0, 150.0, 60.0],
                    0.5 * (1.0 - u / 0.6).max(0.0),
                );
            }
            Style::Plain => {
                ring(
                    img,
                    at.0,
                    at.1,
                    20.0 + u * 260.0,
                    0.6,
                    hot,
                    (1.0 - u / 0.9).max(0.0),
                    4.0,
                    0.0,
                    0.0,
                    0.0,
                );
            }
        }
        speed_lines(
            img,
            at.0,
            at.1,
            40.0,
            (1.0 - u / 0.6).max(0.0),
            seed + 1,
            mix(col, WHITE, 0.7),
        );
        let _ = p;
    }

    /// The cut-in banner of a special: a slanted band in the fighter's colour slashes across the upper ring with
    /// the fighter's name and the move's name (on the finished bytes, before the HUD).
    pub fn cut_in(&self, img: &mut Rgb8, log: &mut DrawLog, t: f64) {
        for sp in &self.anime.specials {
            let u = (t - sp.t0) / 1.3;
            if !(0.0..1.0).contains(&u) {
                continue;
            }
            let w = img.w as f64;
            let slide = if u < 0.12 {
                (1.0 - u / 0.12) * -w * 1.2
            } else if u > 0.85 {
                (u - 0.85) / 0.15 * w * 1.2
            } else {
                0.0
            };
            let (y, h) = (168.0, 52.0);
            let col = self.col[sp.k];
            let ink = |c: V3, a: u8| {
                [
                    c[0].clamp(0.0, 255.0) as u8,
                    c[1].clamp(0.0, 255.0) as u8,
                    c[2].clamp(0.0, 255.0) as u8,
                    a,
                ]
            };
            let band = |y0: f64, y1: f64, tilt: f64| {
                vec![
                    (slide - 20.0, y0 + tilt),
                    (slide + w + 20.0, y0 - tilt),
                    (slide + w + 20.0, y1 - tilt),
                    (slide - 20.0, y1 + tilt),
                ]
            };
            crate::pil::blend::polygon(img, &band(y, y + h, 10.0), ink([10.0, 8.0, 18.0], 225));
            crate::pil::blend::polygon(img, &band(y - 5.0, y + 1.0, 10.0), ink(col, 255));
            crate::pil::blend::polygon(img, &band(y + h - 1.0, y + h + 5.0, 10.0), ink(col, 255));
            // streaks inside the band
            let mut m = Mix(sp.t0.to_bits() | 1);
            for _ in 0..10 {
                let yy = y + 8.0 + m.next() * (h - 16.0);
                let x0 = slide + m.next() * w * 0.5;
                crate::pil::blend::line(
                    img,
                    &[(x0, yy), (x0 + 40.0 + m.next() * 120.0, yy - 6.0)],
                    ink(mix(col, WHITE, 0.5), 90),
                    1,
                );
            }
            if slide == 0.0 {
                let name = self.anime.names[sp.k].as_str();
                crate::ui::label(
                    img,
                    log,
                    name,
                    40,
                    y as i32 + 9,
                    [255, 255, 255],
                    1,
                    Some(crate::font::OUTLINE),
                );
                let mc = ink(mix(col, WHITE, 0.35), 255);
                let word = if sp.name.chars().count() > 12 { 2 } else { 3 };
                crate::ui::label(
                    img,
                    log,
                    &sp.name,
                    40,
                    y as i32 + 22,
                    [mc[0], mc[1], mc[2]],
                    word,
                    Some(crate::font::OUTLINE),
                );
            }
        }
    }
}

/// White feathers swirling up from `c`.
fn feathers(img: &mut Img<f64>, c: (f64, f64), u: f64, seed: u64, k: f64) {
    let mut m = Mix(seed + 99);
    for _ in 0..26 {
        let a = m.next() * 2.0 * dmath::PI;
        let sp = 30.0 + 70.0 * m.next();
        let ph = m.next() * 6.0;
        let x = c.0 + dmath::cos(a) * sp * u + dmath::sin(u * 3.0 + ph) * 10.0;
        let y = c.1 - u * 50.0 * (0.5 + m.next()) + dmath::sin(a) * sp * u * 0.4;
        let tilt = a + u * 2.0;
        let (dx, dy) = (dmath::cos(tilt) * 5.0, dmath::sin(tilt) * 2.0);
        line(img, (x - dx, y - dy), (x + dx, y + dy), WHITE, 0.7 * k, 0.7 * k, 2.0);
    }
}

/// The Subsonic bend: rows of the picture pushed sideways in a travelling wave.
fn bend(img: &mut Img<f64>, u: f64, k: f64) {
    if k <= 0.02 {
        return;
    }
    let (w, h) = (img.w, img.h);
    let src = img.data.clone();
    for y in 0..h {
        let off = (dmath::sin(y as f64 * 0.05 - u * 18.0) * 5.0 * k).round() as i64;
        if off == 0 {
            continue;
        }
        for x in 0..w {
            let sx = (x as i64 - off).clamp(0, w as i64 - 1) as usize;
            for c in 0..3 {
                img.data[(y * w + x) * 3 + c] = src[(y * w + sx) * 3 + c];
            }
        }
    }
}
