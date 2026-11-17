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
    /// The attacker's position at the impact (ring frame).
    pub own: [f64; 2],
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
    /// PEDAL TONE bells that burst where they would vanish.
    pub bells: Vec<Bell>,
    /// Full-screen flashes: `(time, fighter, strength)`, at least 0.5 s apart.
    pub flashes: Vec<(f64, usize, f64)>,
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
                    let own = p.run.p(p.sim_i(t_hit), k);
                    a.specials.push(Special {
                        k,
                        t0: e.t,
                        t_hit,
                        at,
                        own,
                        name,
                    });
                }
                Some((_, v)) if kind == "haste" => a.hastes.push((k, e.t, e.t + dur(v, 3.0))),
                Some((_, v)) if kind == "shield" => a.shields.push((k, e.t, e.t + dur(v, 6.0 * beat))),
                _ => a.pops.push((k, e.t)),
            }
        }
        // full-screen flashes on the specials' impacts and the heaviest hits, spaced out
        let mut cand: Vec<(f64, usize, f64)> = a.specials.iter().map(|s| (s.t_hit, s.k, 0.5)).collect();
        cand.extend(hits.iter().filter(|h| h.s >= 0.9).map(|h| (h.t, h.att, 0.3)));
        cand.sort_by(|x, y| x.0.total_cmp(&y.0).then(y.2.total_cmp(&x.2)));
        for c in cand {
            if a.flashes.last().is_none_or(|l| c.0 - l.0 >= 0.5) {
                a.flashes.push(c);
            }
        }
        a.bells = bells(p);
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

/// The PEDAL TONE bells of the recording (`RUN['SLAMS']`) that leave the picture while still standing.
fn bells(p: &PictureCtx) -> Vec<Bell> {
    use bf_core::event::Value;
    let num = |v: &Value| match *v {
        Value::Float(f) => Some(f),
        Value::Int(i) => Some(i as f64),
        _ => None,
    };
    // (owner, x, z, slam time, radius) -> the last frame it shows
    let mut seen: Vec<((usize, f64, f64, f64, f64), usize)> = Vec::new();
    for i in 0..p.run.nsim {
        let Some(Value::List(rows)) = p.run.nested("SLAMS", i) else {
            continue;
        };
        for row in rows {
            let Value::List(v) = row else { continue };
            let (Some(Value::Int(o)), Some(x), Some(z), Some(ts), Some(r)) = (
                v.first(),
                v.get(1).and_then(num),
                v.get(2).and_then(num),
                v.get(4).and_then(num),
                v.get(5).and_then(num),
            ) else {
                continue;
            };
            let key = (*o as usize, x, z, ts, r);
            match seen.iter_mut().find(|(k, _)| *k == key) {
                Some(e) => e.1 = i,
                None => seen.push((key, i)),
            }
        }
    }
    let fr = 1.0 / f64::from(crate::config::FPS);
    seen.into_iter()
        .filter_map(|((k, x, z, ts, rad), i)| {
            let t = p.show.grid.t_fight + (i + 1) as f64 * fr;
            // the bell's own height then: hovering, slamming, sitting; gone already once it flies off
            let h = if t < ts - 0.12 {
                3.2
            } else if t < ts {
                let u = (t - (ts - 0.12)) / 0.12;
                3.2 - 3.1 * u * u
            } else if t < ts + 0.3 {
                0.1
            } else {
                return None;
            };
            (k < 2 && t < p.t_ko).then_some(Bell {
                t,
                k,
                at: [x, z],
                h,
                rad,
            })
        })
        .collect()
}

// ---------------------------------------------------------------- drawing kit (graded picture, 0..255)

/// Smoothstep of `x` clamped to 0..1.
fn ease(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

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
        // ---- motion blur on fast balls (a ghost trail over the last 1.5 frames)
        if t >= self.t_fight && t < self.t_ko {
            for d in drawn {
                let st = p.fighter(d.k, t - 0.05);
                let pw = scene::to_world(st.p[0], st.p[1], st.paint_h.unwrap_or(R + st.lift));
                let prev = cam.wp(&[pw])[0];
                motion_blur(img, d.s, prev, d.rp);
            }
        }
        // ---- the weapons' own marks, around every hit (some last on the canvas for a while)
        let lo = self.hits.partition_point(|h| h.t < t - 2.6);
        let hi = self.hits.partition_point(|h| h.t <= t + 0.7);
        let mut posts: Vec<Post> = Vec::new();
        let ctx = |h: &PunchHit| {
            let c = self.screen(cam, h.x, h.z, R);
            let qa = p.run.p(p.sim_i(h.t), h.att);
            let a = self.screen(cam, qa[0], qa[1], R);
            let ang = dmath::atan2(c.1 - a.1, c.0 - a.0);
            (a, c, ang)
        };
        // warps first: they bend the scene under the marks
        for h in &self.hits[lo..hi] {
            let u = t - h.t;
            let (a, c, ang) = ctx(h);
            if an.style[h.att] == Some(Style::Tuba) {
                if (0.0..1.6).contains(&u) {
                    ripple(img, c, u, 1.5 + 5.0 * h.s);
                }
                if h.s >= 0.55 && (0.0..1.0).contains(&u) {
                    let life = 0.45 + 0.5 * h.s;
                    let r = (30.0 + 100.0 * h.s) * ease((u / (life * 0.65)).min(1.0));
                    let k = (1.0 - ((u - life * 0.65) / (life * 0.35)).max(0.0)).max(0.0);
                    if u < life {
                        dome_bend(img, c, r, (4.0 + 9.0 * h.s) * k);
                    }
                }
                if (-0.3..0.2).contains(&u)
                    && let Some((vs, rp)) = ball(h.vic)
                {
                    let env = if u < 0.0 { ease((u + 0.3) / 0.3) } else { 1.0 - u / 0.2 };
                    stretch(img, vs, rp, (a.0 - vs.0, a.1 - vs.1), (0.25 + 0.9 * h.s) * env);
                }
            }
            let _ = ang;
        }
        for (i, h) in self.hits[lo..hi].iter().enumerate() {
            let u = t - h.t;
            let st = an.style[h.att].unwrap_or(Style::Plain);
            let (a, c, ang) = ctx(h);
            let col = self.col[h.att];
            self.weapon_mark(
                p,
                img,
                cam,
                st,
                h,
                u,
                a,
                c,
                ang,
                col,
                (lo + i) as u64,
                &mut posts,
                &ball,
            );
            if (0.0..0.3).contains(&u) && h.s >= 0.45 {
                // the shared layer: speed lines on harder hits
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
            if h.s >= 0.85 && (0.0..0.15).contains(&u) {
                posts.push(Post::Radial(c, 1.0 - u / 0.15));
            }
        }
        // ---- PEDAL TONE's bell bursts where it would vanish
        for b in &an.bells {
            let u = t - b.t;
            if (0.0..1.3).contains(&u) {
                self.bell_burst(img, cam, b, u);
            }
        }
        // ---- the specials' own moments
        if let Some((sp, env)) = special {
            self.special(p, img, cam, t, sp, env, &ball);
        }
        for sp in &an.specials {
            let u = t - sp.t_hit;
            if (0.0..0.25).contains(&u) {
                posts.push(Post::Radial(
                    self.screen(cam, sp.at[0], sp.at[1], R),
                    1.4 * (1.0 - u / 0.25),
                ));
            }
        }
        // ---- the picture-wide passes: the Violin's slice, radial blurs, flashes
        for post in &posts {
            match *post {
                Post::Slice(c, ang, d) => slice(img, c, ang, d),
                Post::Radial(c, k) => radial_blur(img, c, k),
            }
        }
        for &(tf, k, s) in &an.flashes {
            let u = t - tf;
            if (0.0..0.1).contains(&u) {
                let tint = mix(self.col[k], WHITE, 0.6);
                let a = s * (1.0 - u / 0.1);
                for px in img.data.chunks_exact_mut(3) {
                    for c in 0..3 {
                        px[c] = px[c] * (1.0 - a) + tint[c] * a;
                    }
                }
            }
        }
        for v in &mut img.data {
            *v = v.clamp(0.0, 255.0);
        }
    }

    /// The camera's roll through a special: the picture turned by [`Punch::tilt`] about its centre, zoomed just
    /// enough to keep the corners filled.
    pub fn roll(&self, img: &mut Img<f64>, t: f64) {
        let a = self.tilt(t);
        if a.abs() < 0.002 {
            return;
        }
        let (w, h) = (img.w as f64, img.h as f64);
        let (cx, cy) = (w / 2.0, h / 2.0);
        let z = 1.0 + 1.9 * a.abs();
        let (ca, sa) = (dmath::cos(a), dmath::sin(a));
        remap(img, 0.0, w, 0.0, h, |x, y| {
            let (dx, dy) = ((x - cx) / z, (y - cy) / z);
            Some((cx + dx * ca - dy * sa, cy + dx * sa + dy * ca))
        });
    }

    /// The burst of a PEDAL TONE bell: brass shards, a dust cloud and a flash where the bell stood.
    fn bell_burst(&self, img: &mut Img<f64>, cam: &Cam, b: &Bell, u: f64) {
        let c = self.screen(cam, b.at[0], b.at[1], b.h + 0.4);
        let f = self.screen(cam, b.at[0], b.at[1], 0.0);
        let rpx = (self.screen(cam, b.at[0] + b.rad, b.at[1], 0.0).0 - f.0)
            .abs()
            .max(20.0);
        let brass = [236.0, 198.0, 108.0];
        let col = self.col[b.k];
        if u < 0.1 {
            disc(img, c.0, c.1, rpx * 1.3, mix(col, WHITE, 0.6), 1.0 - u / 0.1);
        }
        // dust
        let mut m = Mix(b.t.to_bits() | 3);
        for _ in 0..10 {
            let a = m.next() * 2.0 * dmath::PI;
            let d = rpx * (0.3 + 0.7 * m.next()) + u * 70.0;
            let r = 14.0 + u * 50.0;
            disc(
                img,
                f.0 + dmath::cos(a) * d,
                f.1 + dmath::sin(a) * d * 0.6,
                r,
                [190.0, 180.0, 160.0],
                0.3 * (1.0 - u / 1.1).max(0.0),
            );
        }
        // shards: brass triangles flung out and falling
        let mut m = Mix(b.t.to_bits() | 5);
        for _ in 0..34 {
            let a = m.next() * 2.0 * dmath::PI;
            let sp = 140.0 + 220.0 * m.next();
            let sz = 3.0 + 6.0 * m.next();
            let spin = (m.next() - 0.5) * 16.0;
            let life = 0.7 + 0.5 * m.next();
            let go = (1.0 - dmath::exp(-u * 2.4)) / 2.4;
            let x = c.0 + dmath::cos(a) * (rpx * 0.4 + sp * go);
            let y = c.1 + dmath::sin(a) * (rpx * 0.25 + sp * 0.6 * go) + 120.0 * u * u;
            let k = (1.3 - u / life).clamp(0.0, 1.0);
            let r = spin * u + a;
            let pts: Vec<(f64, f64)> = [(-1.0, -0.4), (1.0, -0.2), (0.0, 0.7), (-1.0, -0.4)]
                .iter()
                .map(|&(px, py)| {
                    (
                        x + (px * dmath::cos(r) - py * dmath::sin(r)) * sz,
                        y + (px * dmath::sin(r) + py * dmath::cos(r)) * sz,
                    )
                })
                .collect();
            let shade = mix(brass, WHITE, 0.4 * m.next());
            for w in pts.windows(2) {
                line(img, w[0], w[1], shade, 0.9 * k, 0.9 * k, 2.0);
            }
            disc(img, x, y, sz * 0.8, shade, 0.5 * k);
        }
        burst(
            img,
            c,
            u,
            b.t.to_bits() | 7,
            60,
            mix(col, WHITE, 0.5),
            300.0,
            80.0,
            0.8,
            None,
        );
        ring(
            img,
            f.0,
            f.1,
            rpx + u * 200.0,
            0.55,
            [220.0, 210.0, 190.0],
            (1.0 - u / 0.7).max(0.0),
            4.0,
            0.0,
            0.0,
            0.0,
        );
        cracks(
            img,
            f.0,
            f.1,
            rpx * 2.0,
            (u / 0.2).min(1.0) * (1.0 - u / 1.3).max(0.0),
            col,
            b.t.to_bits() | 9,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn weapon_mark(
        &self,
        p: &PictureCtx,
        img: &mut Img<f64>,
        cam: &Cam,
        st: Style,
        h: &PunchHit,
        u: f64,
        a: (f64, f64),
        c: (f64, f64),
        ang: f64,
        col: V3,
        seed: u64,
        posts: &mut Vec<Post>,
        ball: &dyn Fn(usize) -> Option<((f64, f64), f64)>,
    ) {
        let hot = mix(col, WHITE, 0.45);
        let s = h.s;
        let (dx, dy) = (dmath::cos(ang), dmath::sin(ang));
        let (nx, ny) = (-dy, dx);
        let dist = hyp(c.0 - a.0, c.1 - a.1);
        // a quick glint on the weapon before it fires
        if st != Style::Plain && (-0.25..-0.04).contains(&u) {
            let g = dmath::sin(dmath::PI * (u + 0.25) / 0.21);
            star(
                img,
                (a.0 + dx * 10.0, a.1 + dy * 10.0 - 6.0),
                6.0 + 8.0 * s,
                hot,
                0.9 * g,
                0.3,
            );
        }
        match st {
            Style::Violin => {
                // the rapier crescent: razor thin, white-hot, sweeping through the contact
                if (-0.03..0.5).contains(&u) {
                    let pr = ease((u + 0.03) / 0.06);
                    let len = 1.2 + 2.0 * s;
                    let r = 18.0 + 36.0 * s;
                    let ctr = (c.0 - dx * r * 0.45, c.1 - dy * r * 0.45);
                    let dir = if seed % 2 == 0 { 1.0 } else { -1.0 };
                    let a0 = ang - dir * len / 2.0;
                    let pts = arc_pts(ctr, r, a0, a0 + dir * len * pr, 0.75, 36);
                    let k = (1.0 - (u - 0.06) / 0.44).clamp(0.0, 1.0);
                    taper(img, &pts, col, k, 2.0 + 4.0 * s);
                    if s >= 0.6 && (0.03..0.25).contains(&u) {
                        posts.push(Post::Slice(c, ang + dir * 1.1, 7.0 * s * (1.0 - (u - 0.03) / 0.22)));
                    }
                }
                // the flèche: a blurred lunge streak from where the bow was a moment ago
                if (-0.08..0.06).contains(&u) {
                    let q0 = p.run.p(p.sim_i(h.t - 0.14), h.att);
                    let b0 = self.screen(cam, q0[0], q0[1], R);
                    let k = if u < 0.0 { 1.0 + u / 0.08 } else { 1.0 - u / 0.06 };
                    for j in 0..4 {
                        line(img, b0, c, col, 0.0, 0.16 * k, 4.0 + 6.0 * s - j as f64 * 2.0);
                    }
                    line(img, b0, c, WHITE, 0.0, 0.7 * k, 1.5);
                }
                if (0.0..0.09).contains(&u) {
                    star(img, c, 12.0 + 40.0 * s, col, 1.0 - u / 0.09, ang);
                }
                burst(
                    img,
                    a,
                    u,
                    seed * 7 + 1,
                    (6.0 + 20.0 * s) as usize,
                    [235.0, 222.0, 196.0],
                    90.0 + 90.0 * s,
                    30.0,
                    0.7,
                    Some((ang + dmath::PI, 0.9)),
                );
                burst(
                    img,
                    c,
                    u,
                    seed * 7 + 2,
                    (6.0 + 26.0 * s) as usize,
                    [230.0, 212.0, 255.0],
                    150.0 + 110.0 * s,
                    40.0,
                    0.6,
                    None,
                );
                // the string: a light hit twangs it, a heavy one snaps it
                let half = 32.0 + 24.0 * s;
                let (p0, p1) = ((c.0 - nx * half, c.1 - ny * half), (c.0 + nx * half, c.1 + ny * half));
                if s < 0.6 && seed % 2 == 0 && (0.0..0.7).contains(&u) {
                    let amp = 6.0 * dmath::exp(-u * 4.0);
                    let pts: Vec<(f64, f64)> = (0..=30)
                        .map(|i| {
                            let f = f64::from(i) / 30.0;
                            let o = amp * dmath::sin(dmath::PI * f) * dmath::sin(u * 70.0 + f * 3.0);
                            (p0.0 + (p1.0 - p0.0) * f + dx * o, p0.1 + (p1.1 - p0.1) * f + dy * o)
                        })
                        .collect();
                    taper(img, &pts, col, (1.0 - u / 0.7).max(0.0), 1.6);
                } else if s >= 0.6 && (0.0..1.0).contains(&u) {
                    let fe = ease(u / 0.35);
                    for (end, sg) in [(p0, -1.0), (p1, 1.0)] {
                        let pts: Vec<(f64, f64)> = (0..=24)
                            .map(|i| {
                                let f = f64::from(i) / 24.0; // 0 at the end, 1 at the break
                                let mut x = end.0 + (c.0 - end.0) * f * (1.0 - fe);
                                let mut y = end.1 + (c.1 - end.1) * f * (1.0 - fe);
                                let curl = 18.0 * dmath::exp(-u * 2.2) * f * f * f;
                                x += dmath::cos(f * 9.0 + u * 12.0) * curl * sg;
                                y += dmath::sin(f * 9.0 + u * 12.0) * curl;
                                (x, y)
                            })
                            .collect();
                        taper(img, &pts, col, (1.0 - u).max(0.0), 1.8);
                    }
                    burst(
                        img,
                        c,
                        u,
                        seed * 7 + 3,
                        30,
                        [230.0, 210.0, 255.0],
                        240.0,
                        90.0,
                        0.8,
                        None,
                    );
                }
            }
            Style::Trombone => {
                // the piston thrust: a ghost slide fires out, overshoots and springs back
                let lmax = (dist - 6.0).max(10.0);
                let ov = 4.0 + 12.0 * s;
                let len = |u: f64| -> f64 {
                    if u < -0.05 {
                        0.0
                    } else if u < 0.0 {
                        lmax * ((u + 0.05) / 0.05) * ((u + 0.05) / 0.05)
                    } else {
                        lmax + ov * dmath::exp(-u * 7.0) * dmath::cos(u * 34.0)
                    }
                };
                if (-0.05..0.45).contains(&u) {
                    let k = 1.0 - ss(0.25, 0.45, u);
                    let brass = mix([236.0, 198.0, 108.0], col, 0.3);
                    let draw = |img: &mut Img<f64>, l: f64, kk: f64| {
                        for o in [-3.0, 3.0] {
                            let s0 = (a.0 + nx * o, a.1 + ny * o);
                            line(img, s0, (s0.0 + dx * l, s0.1 + dy * l), brass, 0.6 * kk, 0.9 * kk, 2.0);
                        }
                        ring(
                            img,
                            a.0 + dx * l,
                            a.1 + dy * l,
                            3.5,
                            1.0,
                            mix(brass, WHITE, 0.4),
                            kk,
                            2.0,
                            0.0,
                            0.0,
                            0.0,
                        );
                    };
                    if u < 0.03 {
                        for j in 1..6 {
                            draw(img, len(u - j as f64 * 0.007), 0.14 * k);
                        }
                        line(img, a, (a.0 + dx * len(u), a.1 + dy * len(u)), col, 0.0, 0.5 * k, 12.0);
                    }
                    draw(img, len(u), k);
                }
                // the brass halo: one ring for a tap, up to four for a heavy hit
                let n = 1 + ((s - 0.25) / 0.75 * 3.0).round().clamp(0.0, 3.0) as usize;
                for j in 0..n {
                    let uj = u - j as f64 * 0.075;
                    if !(0.0..0.7).contains(&uj) {
                        continue;
                    }
                    let r = 8.0 + (30.0 + 16.0 * j as f64) * ease((uj / 0.45).min(1.0)) * (0.7 + 0.5 * s);
                    let k = (1.0 - uj / 0.7) * (1.0 - j as f64 * 0.12);
                    let jag = r * 0.14 * dmath::exp(-uj * 5.0);
                    let tooth = 6.0 + j as f64;
                    ring(
                        img,
                        c.0,
                        c.1,
                        r,
                        0.62,
                        [255.0, 220.0, 140.0],
                        k,
                        2.5,
                        jag,
                        tooth,
                        uj * 55.0,
                    );
                    ring(img, c.0, c.1, r, 0.62, col, 0.35 * k, 5.0, jag, tooth, uj * 55.0);
                }
                // shock diamonds in a cone of air from the bell
                if s >= 0.45 {
                    let life = 0.25 + 0.4 * s;
                    if (0.0..life).contains(&u) {
                        let k = dmath::pow(dmath::sin(dmath::PI * u / life).max(0.0), 0.6);
                        let reach = (dist + 20.0) * ease((u / 0.08).min(1.0));
                        for j in -3i32..=3 {
                            let aa = ang + f64::from(j) * 0.07 * (0.6 + s);
                            line(
                                img,
                                a,
                                (a.0 + dmath::cos(aa) * reach, a.1 + dmath::sin(aa) * reach),
                                col,
                                0.22 * k,
                                0.0,
                                3.0,
                            );
                        }
                        let nd = 2 + (3.0 * s).round() as usize;
                        for i in 0..nd {
                            let d = 14.0 + i as f64 * (dist / (nd as f64 + 0.5)).max(12.0);
                            if d > reach {
                                break;
                            }
                            let q = (a.0 + dx * d, a.1 + dy * d);
                            let sz = (7.0 - i as f64 * 0.8) * (0.8 + 0.25 * dmath::sin(u * 90.0 + i as f64 * 2.0));
                            let pts = [
                                (q.0 - dx * sz * 1.5, q.1 - dy * sz * 1.5),
                                (q.0 + nx * sz * 0.7, q.1 + ny * sz * 0.7),
                                (q.0 + dx * sz * 1.5, q.1 + dy * sz * 1.5),
                                (q.0 - nx * sz * 0.7, q.1 - ny * sz * 0.7),
                                (q.0 - dx * sz * 1.5, q.1 - dy * sz * 1.5),
                            ];
                            for w in pts.windows(2) {
                                line(img, w[0], w[1], mix(col, WHITE, 0.6), k, k, 1.5);
                            }
                            disc(img, q.0, q.1, sz * 1.2, WHITE, 0.6 * k);
                        }
                    }
                }
                if (0.0..0.08).contains(&u) {
                    star(img, c, 12.0 + 30.0 * s, col, 1.0 - u / 0.08, 0.0);
                }
                burst(
                    img,
                    c,
                    u,
                    seed * 7 + 4,
                    (8.0 + 30.0 * s) as usize,
                    [200.0, 255.0, 220.0],
                    140.0 + 120.0 * s,
                    60.0,
                    0.6,
                    Some((ang, 2.2)),
                );
            }
            Style::Accordion => {
                // gouges left by heavy bites: they glow, cool and fade (on the canvas, so they stay put)
                if s >= 0.6 && (0.0..2.2).contains(&u) {
                    let k = (1.0 - u / 2.2).max(0.0);
                    let heat = (1.0 - u / 0.9).max(0.0);
                    let base = [h.x + h.dir[0] * 0.15, h.z + h.dir[1] * 0.15];
                    for i in 0..5 {
                        let aa = -0.75 + 0.37 * i as f64;
                        let (ca, sa) = (dmath::cos(aa), dmath::sin(aa));
                        let d = [h.dir[0] * ca - h.dir[1] * sa, h.dir[0] * sa + h.dir[1] * ca];
                        let q = [base[0] + d[0] * 0.5, base[1] + d[1] * 0.5];
                        let q0 = self.screen(cam, q[0] - d[0] * 0.12, q[1] - d[1] * 0.12, 0.0);
                        let q1 = self.screen(cam, q[0] + d[0] * 0.12, q[1] + d[1] * 0.12, 0.0);
                        darken_line(img, q0, q1, 0.6 * k);
                        darken_line(img, (q0.0, q0.1 - 1.0), (q1.0, q1.1 - 1.0), 0.5 * k);
                        line(img, q0, q1, col, 0.9 * heat, 0.4 * heat, 1.2);
                    }
                }
                // the chomp: pleated jaws gape, slam shut past each other, bounce open
                if (-0.15..0.4).contains(&u) {
                    let open = if u < 0.0 {
                        let f = (u + 0.15) / 0.15;
                        if f < 0.85 {
                            0.15 + 0.9 * ease(f / 0.85)
                        } else {
                            1.05 - 1.19 * ((f - 0.85) / 0.15) * ((f - 0.85) / 0.15)
                        }
                    } else if u < 0.08 {
                        -0.14 + 0.26 * ease(u / 0.08)
                    } else {
                        0.12 * dmath::exp(-(u - 0.08) * 6.0)
                    };
                    let k = 1.0 - ss(0.25, 0.4, u);
                    let size = 24.0 + 22.0 * s;
                    jaws(img, c.0, c.1, ang, size, open, col, k);
                    jaws(img, c.0 + nx, c.1 + ny, ang, size, open, WHITE, 0.6 * k);
                }
                // inhale, exhale
                if (-0.5..0.0).contains(&u) {
                    let inh = ease((u + 0.5) / 0.5);
                    let mut m = Mix(seed * 13 + 9);
                    for _ in 0..(8.0 + 16.0 * s) as usize {
                        let aa = m.next() * 2.0 * dmath::PI;
                        let f = (u * 1.8 + m.next()).rem_euclid(1.0);
                        let d = 100.0 * (1.0 - f) + 14.0;
                        let p0 = (a.0 + dmath::cos(aa) * d, a.1 + dmath::sin(aa) * d * 0.7);
                        let p1 = (
                            a.0 + dmath::cos(aa) * (d + 16.0),
                            a.1 + dmath::sin(aa) * (d + 16.0) * 0.7,
                        );
                        line(img, p1, p0, hot, 0.0, 0.7 * inh * dmath::sin(dmath::PI * f), 1.4);
                    }
                }
                if (0.0..0.7).contains(&u) {
                    let r = (30.0 + 100.0 * s) * ease((u / 0.35).min(1.0));
                    ring(
                        img,
                        a.0,
                        a.1,
                        r,
                        0.62,
                        hot,
                        1.0 - u / 0.7,
                        2.0 + 4.0 * s,
                        2.0,
                        9.0,
                        u * 20.0,
                    );
                    ring(
                        img,
                        a.0,
                        a.1,
                        r * 0.75,
                        0.62,
                        col,
                        0.5 * (1.0 - u / 0.5).max(0.0),
                        3.0 + 6.0 * s,
                        0.0,
                        0.0,
                        0.0,
                    );
                }
                // button shrapnel, ricocheting off the ropes
                if (0.0..1.1).contains(&u) {
                    let n = 3 + (7.0 * s).round() as usize;
                    let mut m = Mix(seed * 13 + 17);
                    for _ in 0..n {
                        let aa = (m.next() - 0.5) * (1.0 + 1.8 * s);
                        let sp = 4.5 + 2.5 * m.next();
                        let (ca, sa) = (dmath::cos(aa), dmath::sin(aa));
                        let mut v = [
                            (h.dir[0] * ca - h.dir[1] * sa) * sp,
                            (h.dir[0] * sa + h.dir[1] * ca) * sp,
                        ];
                        let mut q = [h.x, h.z];
                        let mut trail: Vec<[f64; 2]> = Vec::new();
                        let mut sparks: Vec<([f64; 2], f64)> = Vec::new();
                        let dt = 0.01;
                        let mut tt = 0.0;
                        while tt < u {
                            q = [q[0] + v[0] * dt, q[1] + v[1] * dt];
                            v = [v[0] * 0.993, v[1] * 0.993];
                            for ax in 0..2 {
                                if q[ax].abs() > scene::A - 0.1 && q[ax] * v[ax] > 0.0 {
                                    v[ax] = -v[ax] * 0.7;
                                    sparks.push((q, u - tt));
                                }
                            }
                            trail.push(q);
                            tt += dt;
                        }
                        let k = (1.0 - (u - 0.6) / 0.5).clamp(0.0, 1.0);
                        let tl = &trail[trail.len().saturating_sub(12)..];
                        let sc: Vec<(f64, f64)> = tl.iter().map(|q| self.screen(cam, q[0], q[1], R * 0.5)).collect();
                        for (j, w) in sc.windows(2).enumerate() {
                            let f = (j + 1) as f64 / sc.len() as f64;
                            line(img, w[0], w[1], col, 0.8 * k * f, 0.8 * k * f, 2.0 * f + 0.5);
                        }
                        if let Some(&e) = sc.last() {
                            disc(img, e.0, e.1, 4.0, [250.0, 245.0, 235.0], k);
                        }
                        for (sq, age) in sparks {
                            if age < 0.2 {
                                let ss_ = self.screen(cam, sq[0], sq[1], R * 0.5);
                                star(img, ss_, 10.0, col, 1.0 - age / 0.2, age * 10.0);
                            }
                        }
                    }
                }
                if (0.0..0.07).contains(&u) {
                    star(img, c, 12.0 + 30.0 * s, col, 1.0 - u / 0.07, 0.4);
                }
            }
            Style::Tuba => {
                // suction lines into the bell before the gulp
                if (-0.35..0.0).contains(&u) {
                    let f0 = ease((u + 0.35) / 0.35);
                    let mut m = Mix(seed * 13 + 5);
                    for _ in 0..(8.0 + 12.0 * s) as usize {
                        let off = (m.next() - 0.5) * 40.0;
                        let f = (u * 2.4 + m.next()).rem_euclid(1.0);
                        let x = c.0 + dx * 20.0 + (a.0 - c.0 - dx * 20.0) * f + nx * off * (1.0 - f);
                        let y = c.1 + dy * 20.0 + (a.1 - c.1 - dy * 20.0) * f + ny * off * (1.0 - f);
                        line(
                            img,
                            (x - dx * 16.0, y - dy * 16.0),
                            (x, y),
                            hot,
                            0.0,
                            0.8 * f0 * dmath::sin(dmath::PI * f),
                            1.3,
                        );
                    }
                }
                // the spit: a dark purple puff out of the bell
                let su = u - 0.12;
                if (0.0..0.7).contains(&su) {
                    ring(
                        img,
                        a.0,
                        a.1,
                        10.0 + su * (70.0 + 100.0 * s),
                        0.62,
                        mix(col, [20.0, 10.0, 40.0], 0.5),
                        (1.0 - su / 0.6).max(0.0),
                        4.0 + 5.0 * s,
                        0.0,
                        0.0,
                        0.0,
                    );
                    ring(
                        img,
                        a.0,
                        a.1,
                        6.0 + su * (55.0 + 80.0 * s),
                        0.62,
                        hot,
                        (1.0 - su / 0.5).max(0.0),
                        2.0,
                        0.0,
                        0.0,
                        0.0,
                    );
                    burst(
                        img,
                        a,
                        su,
                        seed * 7 + 5,
                        (10.0 + 30.0 * s) as usize,
                        [170.0, 160.0, 255.0],
                        120.0 + 120.0 * s,
                        30.0,
                        0.6,
                        Some((ang, 1.4)),
                    );
                }
                // the sonic boom dome
                if s >= 0.55 && (0.0..1.0).contains(&u) {
                    let life = 0.45 + 0.5 * s;
                    if u < life {
                        let r = (30.0 + 100.0 * s) * ease((u / (life * 0.65)).min(1.0));
                        let k = (1.0 - ((u - life * 0.65) / (life * 0.35)).max(0.0)).max(0.0);
                        darken_ellipse(img, c, r, r * 0.66, 0.35 * k);
                        ring(
                            img,
                            c.0,
                            c.1,
                            r,
                            0.66,
                            mix(col, WHITE, 0.5),
                            0.9 * k,
                            2.0,
                            0.0,
                            0.0,
                            0.0,
                        );
                        ring(img, c.0, c.1, r, 0.66, col, 0.45 * k, 7.0, 0.0, 0.0, 0.0);
                    }
                }
                if (0.0..0.08).contains(&u) {
                    star(img, c, 12.0 + 26.0 * s, col, 1.0 - u / 0.08, 0.0);
                }
            }
            Style::Cello => {
                // string tension: four strings pull taut and quiver, then snap (one for a tap, all four hard)
                if (-0.6..1.0).contains(&u) {
                    let tens = if u < 0.0 { ease((u + 0.6) / 0.6) } else { 1.0 };
                    let n_snap = 1 + ((s - 0.25) / 0.75 * 3.0).round().clamp(0.0, 3.0) as usize;
                    for j in 0..4 {
                        let off = (j as f64 - 1.5) * 3.0;
                        let p0 = (a.0 + nx * off, a.1 + ny * off);
                        let p1 = (c.0 + nx * off * 0.6, c.1 + ny * off * 0.6);
                        let us = u - j as f64 * 0.05;
                        let snapped = j < n_snap && us >= 0.0;
                        if u < 0.0 || (!snapped && u < 0.25) {
                            let k = if u < 0.0 { 0.5 + 0.5 * tens } else { 1.0 - u / 0.25 };
                            let sag = 14.0 * (1.0 - tens) + if u >= 0.0 { 10.0 * u / 0.25 } else { 0.0 };
                            let pts: Vec<(f64, f64)> = (0..=24)
                                .map(|i| {
                                    let f = f64::from(i) / 24.0;
                                    let o = sag * dmath::sin(dmath::PI * f)
                                        + 1.6
                                            * tens
                                            * dmath::sin(f * 40.0 + u * 160.0 + j as f64)
                                            * dmath::sin(dmath::PI * f);
                                    (p0.0 + (p1.0 - p0.0) * f, p0.1 + (p1.1 - p0.1) * f + o)
                                })
                                .collect();
                            taper(
                                img,
                                &pts,
                                mix(col, [255.0, 170.0, 80.0], 0.4 + 0.6 * tens),
                                k,
                                1.2 + 1.4 * tens,
                            );
                        } else if snapped && us < 1.0 {
                            let fe = ease(us / 0.3);
                            let mid = ((p0.0 + p1.0) / 2.0, (p0.1 + p1.1) / 2.0);
                            for (end, sg) in [(p0, -1.0), (p1, 1.0)] {
                                let pts: Vec<(f64, f64)> = (0..=20)
                                    .map(|i| {
                                        let f = f64::from(i) / 20.0;
                                        let mut x = end.0 + (mid.0 - end.0) * f * (1.0 - fe * 0.8);
                                        let mut y = end.1 + (mid.1 - end.1) * f * (1.0 - fe * 0.8);
                                        let curl = 22.0 * dmath::exp(-us * 2.4) * dmath::pow(f, 2.5);
                                        x += dmath::cos(f * 8.0 + us * 14.0 + j as f64) * curl * sg;
                                        y += dmath::sin(f * 8.0 + us * 14.0 + j as f64) * curl;
                                        (x, y)
                                    })
                                    .collect();
                                taper(img, &pts, [255.0, 170.0, 80.0], (1.0 - us).max(0.0), 2.0);
                            }
                            if us < 0.06 {
                                star(img, mid, 22.0, [255.0, 170.0, 80.0], 1.0 - us / 0.06, 0.0);
                            }
                            burst(
                                img,
                                mid,
                                us,
                                seed * 7 + 6 + j as u64,
                                14,
                                [255.0, 210.0, 140.0],
                                200.0,
                                90.0,
                                0.6,
                                None,
                            );
                        }
                    }
                }
                // the heavy bow arc: one thick, painterly brush stroke of amber light, leaving embers
                let sweep = 0.13;
                let uu = u + sweep * 0.7;
                if (0.0..0.9).contains(&uu) {
                    let pr = ease(uu / sweep);
                    let k = (1.0 - (uu - sweep - 0.1) / 0.6).clamp(0.0, 1.0);
                    let r = 40.0 + 34.0 * s;
                    let w = 6.0 + 14.0 * s;
                    let ctr = (c.0, c.1 + r * 0.55);
                    let len = 1.6 + 0.9 * s;
                    let a0 = -dmath::PI / 2.0 - len / 2.0;
                    let mut m = Mix(seed * 13 + 23);
                    for j in 0..9 {
                        let off = (j as f64 / 8.0 - 0.5) * w;
                        let al = 0.35 + 0.65 * m.next();
                        let cc = mix([255.0, 170.0, 80.0], [255.0, 236.0, 190.0], m.next());
                        let pts = arc_pts(
                            ctr,
                            r + off,
                            a0 + m.next() * 0.08,
                            a0 + len * pr - m.next() * 0.12,
                            0.62,
                            30,
                        );
                        for (i, wv) in pts.windows(2).enumerate() {
                            let f = i as f64 / (pts.len() - 1) as f64;
                            let ww = (w / 9.0 + 1.2) * dmath::sin(dmath::PI * f).max(0.0).sqrt() * (0.5 + 0.5 * f);
                            line(
                                img,
                                wv[0],
                                wv[1],
                                cc,
                                k * al * (0.25 + 0.75 * f),
                                k * al * (0.25 + 0.75 * f),
                                ww,
                            );
                        }
                    }
                    let mut m = Mix(seed * 13 + 29);
                    for _ in 0..(14.0 + 26.0 * s) as usize {
                        let f = m.next();
                        let aa = a0 + len * f;
                        let age = uu - sweep * f;
                        if age < 0.0 {
                            continue;
                        }
                        let x = ctr.0 + dmath::cos(aa) * (r + (m.next() - 0.5) * w) + (m.next() - 0.5) * age * 30.0;
                        let y = ctr.1 + dmath::sin(aa) * r * 0.62 - age * 30.0 - age * age * 20.0;
                        disc(img, x, y, 2.2, [255.0, 200.0, 120.0], (1.0 - age / 0.8).max(0.0));
                    }
                }
                // the suplex slam (the lift is drawn by the ball itself)
                if s >= 0.7 && (0.0..0.9).contains(&u) {
                    let f = self.screen(cam, h.x + h.dir[0] * 0.3, h.z + h.dir[1] * 0.3, 0.0);
                    ring(
                        img,
                        f.0,
                        f.1,
                        20.0 + u * 200.0,
                        0.5,
                        [220.0, 210.0, 190.0],
                        (1.0 - u / 0.7).max(0.0),
                        6.0,
                        0.0,
                        0.0,
                        0.0,
                    );
                    cracks(
                        img,
                        f.0,
                        f.1,
                        150.0,
                        ease((u / 0.25).min(1.0)) * (1.0 - u / 0.9),
                        [255.0, 170.0, 80.0],
                        seed + 41,
                    );
                    burst(
                        img,
                        f,
                        u,
                        seed * 7 + 11,
                        50,
                        [230.0, 215.0, 190.0],
                        240.0,
                        40.0,
                        0.9,
                        None,
                    );
                    if u < 0.08 {
                        star(img, f, 56.0, [255.0, 170.0, 80.0], 1.0 - u / 0.08, 0.0);
                    }
                }
                if (0.0..0.07).contains(&u) {
                    star(img, c, 12.0 + 28.0 * s, [255.0, 170.0, 80.0], 1.0 - u / 0.07, 0.2);
                }
            }
            Style::BariSax => {
                // the crater: on the canvas, it stays a while; its cracks glow and cool
                if (0.0..2.5).contains(&u) {
                    let fade = (1.0 - (u - 2.1) / 0.4).clamp(0.0, 1.0);
                    let heat = (1.0 - u / 1.3).max(0.0);
                    let f = self.screen(cam, h.x, h.z, 0.0);
                    let rr = 0.12 + 0.35 * s;
                    let rx = (self.screen(cam, h.x + rr, h.z, 0.0).0 - f.0).abs().max(4.0);
                    let rz = (self.screen(cam, h.x, h.z + rr, 0.0).1 - f.1).abs().max(2.5);
                    let grow = ease((u / 0.08).min(1.0));
                    darken_ellipse(img, f, rx * grow * 1.6, rz.max(rx * 0.5) * grow * 1.6, 0.6 * fade);
                    ring(
                        img,
                        f.0,
                        f.1 - 1.0,
                        rx * grow * 1.6,
                        rz.max(rx * 0.5) / rx,
                        [255.0, 230.0, 210.0],
                        0.25 * fade,
                        1.5,
                        0.0,
                        0.0,
                        0.0,
                    );
                    let cc = mix([110.0, 100.0, 110.0], [255.0, 150.0, 60.0], heat);
                    cracks(
                        img,
                        f.0,
                        f.1,
                        (50.0 + 90.0 * s) * ease((u / 0.15).min(1.0)),
                        fade,
                        cc,
                        seed + 3,
                    );
                    // rubble bouncing out
                    let mut m = Mix(seed * 13 + 31);
                    for _ in 0..(5.0 + 9.0 * s) as usize {
                        let aa = m.next() * 2.0 * dmath::PI;
                        let d = (rx * 1.2 + 10.0 + m.next() * (20.0 + 30.0 * s)) * (1.0 - dmath::exp(-u * 5.0));
                        let h0 = (12.0 + 18.0 * s) * (0.5 + m.next());
                        let tb = 0.32 + 0.2 * m.next();
                        let hh = h0 * dmath::sin(dmath::PI * u / tb).abs() * dmath::exp(-u * 2.6);
                        let (sx, sy) = (f.0 + dmath::cos(aa) * d, f.1 + dmath::sin(aa) * d * 0.55);
                        let sz = 1.5 + 2.5 * m.next();
                        darken_ellipse(img, (sx, sy + 2.0), sz * 1.2, sz * 0.6, 0.3 * fade);
                        disc(
                            img,
                            sx,
                            sy - hh,
                            sz * 1.5,
                            mix([96.0, 84.0, 110.0], [255.0, 150.0, 80.0], heat * 0.6),
                            0.9 * fade,
                        );
                    }
                }
                // the pendulum smear: a heavy hot arc through the swing into the victim
                if (-0.16..0.05).contains(&u) {
                    let k = if u < 0.0 { 1.0 + u / 0.16 } else { 1.0 - u / 0.05 };
                    let r = dist.max(20.0);
                    smear(img, a.0, a.1, r, ang - 1.6, ang, col, k, 10.0 + 18.0 * s);
                    smear(
                        img,
                        a.0,
                        a.1,
                        r,
                        ang - 1.2,
                        ang,
                        [255.0, 190.0, 110.0],
                        0.6 * k,
                        4.0 + 8.0 * s,
                    );
                }
                if (0.0..0.6).contains(&u) {
                    let v = u / 0.6;
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
                    burst(
                        img,
                        c,
                        u,
                        seed * 7 + 8,
                        (14.0 + 40.0 * s) as usize,
                        [255.0, 160.0, 90.0],
                        160.0 + 160.0 * s,
                        140.0,
                        0.8,
                        None,
                    );
                }
                if (0.0..0.08).contains(&u) {
                    star(img, c, 16.0 + 36.0 * s, col, 1.0 - u / 0.08, 0.0);
                }
            }
            Style::Plain => {}
        }
        let _ = ball;
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

// ---------------------------------------------------------------- the round-2 kit: picture-wide passes and strokes

/// A pass over the whole picture after the marks.
#[derive(Clone, Copy, Debug)]
pub enum Post {
    /// The Violin's slice: the side of the line through `c` at `ang` slides `d` px along it.
    Slice((f64, f64), f64, f64),
    /// A radial blur rushing to `c`, strength `k`.
    Radial((f64, f64), f64),
}

/// A PEDAL TONE bell that leaves the picture while it is still standing: it bursts at `t`.
#[derive(Clone, Debug, PartialEq)]
pub struct Bell {
    pub t: f64,
    pub k: usize,
    pub at: [f64; 2],
    pub h: f64,
    pub rad: f64,
}

/// The picture's colour at `(x, y)`, bilinear, clamped to the edges.
fn sample(src: &[f64], w: usize, h: usize, x: f64, y: f64) -> [f64; 3] {
    let x = x.clamp(0.0, w as f64 - 1.001);
    let y = y.clamp(0.0, h as f64 - 1.001);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (fx, fy) = (x - x0 as f64, y - y0 as f64);
    let at = |xx: usize, yy: usize, c: usize| src[(yy * w + xx) * 3 + c];
    std::array::from_fn(|c| {
        let a = at(x0, y0, c) * (1.0 - fx) + at(x0 + 1, y0, c) * fx;
        let b = at(x0, y0 + 1, c) * (1.0 - fx) + at(x0 + 1, y0 + 1, c) * fx;
        a * (1.0 - fy) + b * fy
    })
}

/// Remaps the pixels in the box `(xa..xb, ya..yb)`: `f(x, y)` gives the source point, or `None` to keep it.
fn remap(img: &mut Img<f64>, xa: f64, xb: f64, ya: f64, yb: f64, f: impl Fn(f64, f64) -> Option<(f64, f64)>) {
    let (w, h) = (img.w, img.h);
    let xa = xa.floor().max(0.0) as usize;
    let xb = (xb.ceil().max(0.0) as usize).min(w - 1);
    let ya = ya.floor().max(0.0) as usize;
    let yb = (yb.ceil().max(0.0) as usize).min(h - 1);
    if xa > xb || ya > yb {
        return;
    }
    let src = img.data.clone();
    for y in ya..=yb {
        for x in xa..=xb {
            if let Some((sx, sy)) = f(x as f64, y as f64) {
                let c = sample(&src, w, h, sx, sy);
                img.data[(y * w + x) * 3..(y * w + x) * 3 + 3].copy_from_slice(&c);
            }
        }
    }
}

/// Motion blur: a ball drawn at `now` that was at `prev` 1.5 frames ago leaves a ghost trail behind it.
fn motion_blur(img: &mut Img<f64>, now: (f64, f64), prev: (f64, f64), rp: f64) {
    let v = (now.0 - prev.0, now.1 - prev.1);
    let len = hyp(v.0, v.1);
    if len < 4.0 || len > 200.0 {
        return;
    }
    let (w, h) = (img.w, img.h);
    let src = img.data.clone();
    let n = 8;
    let (xa, xb) = (now.0.min(prev.0) - rp - 1.0, now.0.max(prev.0) + rp + 1.0);
    let (ya, yb) = (now.1.min(prev.1) - rp - 1.0, now.1.max(prev.1) + rp + 1.0);
    let xa = xa.floor().max(0.0) as usize;
    let xb = (xb.ceil().max(0.0) as usize).min(w - 1);
    let ya = ya.floor().max(0.0) as usize;
    let yb = (yb.ceil().max(0.0) as usize).min(h - 1);
    for y in ya..=yb {
        for x in xa..=xb {
            let (px, py) = (x as f64, y as f64);
            if hyp(px - now.0, py - now.1) < rp {
                continue;
            }
            let mut acc = [0.0; 3];
            let mut hits = 0.0;
            for j in 1..=n {
                let f = j as f64 / n as f64;
                let (qx, qy) = (px + v.0 * f, py + v.1 * f);
                if hyp(qx - now.0, qy - now.1) < rp - 0.5 {
                    let c = sample(&src, w, h, qx, qy);
                    for i in 0..3 {
                        acc[i] += c[i] * (1.0 - f * 0.6);
                    }
                    hits += 1.0 - f * 0.6;
                }
            }
            if hits <= 0.0 {
                continue;
            }
            let cover = (hits / n as f64 * 1.4).min(0.85);
            let o = (y * w + x) * 3;
            for i in 0..3 {
                img.data[o + i] = img.data[o + i] * (1.0 - cover) + acc[i] / hits * cover;
            }
        }
    }
}

/// The Tuba's drumhead floor: a ripple from `c`, `u` s after the hit, up to `amp` px.
fn ripple(img: &mut Img<f64>, c: (f64, f64), u: f64, amp: f64) {
    let front = u * 260.0;
    let env = dmath::exp(-u * 1.7);
    if env * amp < 0.25 {
        return;
    }
    let reach = front + 10.0;
    remap(
        img,
        c.0 - reach,
        c.0 + reach,
        c.1 - reach / 1.6,
        c.1 + reach / 1.6,
        |x, y| {
            let d = hyp(x - c.0, (y - c.1) * 1.6);
            if d > reach {
                return None;
            }
            let e = env * dmath::exp(-d / 150.0) * ((front - d) / 30.0 + 1.0).clamp(0.0, 1.0);
            Some((x, y + amp * e * dmath::sin(d * 0.11 - u * 26.0)))
        },
    );
}

/// The Tuba's sonic boom: everything near the dome's edge (radius `r` around `c`) bends outward by up to `b` px.
fn dome_bend(img: &mut Img<f64>, c: (f64, f64), r: f64, b: f64) {
    if b < 0.3 || r < 2.0 {
        return;
    }
    let reach = r + 40.0;
    remap(
        img,
        c.0 - reach,
        c.0 + reach,
        c.1 - reach / 1.5,
        c.1 + reach / 1.5,
        |x, y| {
            let (dx, dy) = (x - c.0, (y - c.1) * 1.5);
            let d = hyp(dx, dy) + 1e-6;
            let q = (d - r) / 14.0;
            let e = dmath::exp(-(q * q));
            if e < 0.01 {
                return None;
            }
            Some((x - dx / d * b * e, y - dy / d * b * e / 1.5))
        },
    );
}

/// The Tuba's gulp: the ball at `c` (radius `rp`) stretched toward `dir` by `amt` (0 = round).
fn stretch(img: &mut Img<f64>, c: (f64, f64), rp: f64, dir: (f64, f64), amt: f64) {
    let n = hyp(dir.0, dir.1);
    if amt < 0.03 || n < 1e-6 {
        return;
    }
    let (ux, uy) = (dir.0 / n, dir.1 / n);
    let st = 1.0 + amt;
    let reach = rp * st * 1.6 + 2.0;
    remap(img, c.0 - reach, c.0 + reach, c.1 - reach, c.1 + reach, |x, y| {
        let (dx, dy) = (x - c.0, y - c.1);
        let along = dx * ux + dy * uy;
        let side = -dx * uy + dy * ux;
        // the stretch leans toward the bell: the far side grows more
        let s_al = if along > 0.0 { st * 1.6 } else { st };
        let (sa, ss_) = (along / s_al, side * st.sqrt());
        if hyp(sa, ss_) > rp + 1.0 {
            return None;
        }
        Some((c.0 + sa * ux - ss_ * uy, c.1 + sa * uy + ss_ * ux))
    });
}

/// The Violin's slice: the half of the picture on one side of the line through `c` at `ang` slides `d` px along
/// the line, with a bright seam.
fn slice(img: &mut Img<f64>, c: (f64, f64), ang: f64, d: f64) {
    if d.abs() < 0.3 {
        return;
    }
    let (lx, ly) = (dmath::cos(ang), dmath::sin(ang));
    let (w, h) = (img.w as f64, img.h as f64);
    remap(img, 0.0, w, 0.0, h, |x, y| {
        let side = -(x - c.0) * ly + (y - c.1) * lx;
        if side <= 0.0 {
            None
        } else {
            Some((x - lx * d, y - ly * d))
        }
    });
    let far = 900.0;
    line(
        img,
        (c.0 - lx * far, c.1 - ly * far),
        (c.0 + lx * far, c.1 + ly * far),
        WHITE,
        0.7 * (d / 7.0).min(1.0),
        0.7 * (d / 7.0).min(1.0),
        1.0,
    );
}

/// A radial blur rushing to `c`: each pixel averages the picture along the line toward `c`.
fn radial_blur(img: &mut Img<f64>, c: (f64, f64), k: f64) {
    if k < 0.03 {
        return;
    }
    let (w, h) = (img.w, img.h);
    let src = img.data.clone();
    let n = 8;
    let reach = 0.09 * k;
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = (x as f64 - c.0, y as f64 - c.1);
            let mut acc = [0.0; 3];
            for j in 0..n {
                let f = 1.0 - reach * j as f64 / (n - 1) as f64;
                let s = sample(&src, w, h, c.0 + dx * f, c.1 + dy * f);
                for i in 0..3 {
                    acc[i] += s[i];
                }
            }
            let o = (y * w + x) * 3;
            for i in 0..3 {
                img.data[o + i] = acc[i] / n as f64;
            }
        }
    }
}

/// Darkens a soft ellipse at `c` (radii `rx`, `ry`) by up to `k`.
fn darken_ellipse(img: &mut Img<f64>, c: (f64, f64), rx: f64, ry: f64, k: f64) {
    if k <= 0.01 || rx < 0.5 || ry < 0.5 {
        return;
    }
    let (xa, xb) = (
        (c.0 - rx).floor().max(0.0) as i64,
        (c.0 + rx).ceil().min(img.w as f64 - 1.0) as i64,
    );
    let (ya, yb) = (
        (c.1 - ry).floor().max(0.0) as i64,
        (c.1 + ry).ceil().min(img.h as f64 - 1.0) as i64,
    );
    for y in ya..=yb {
        for x in xa..=xb {
            let d = hyp((x as f64 - c.0) / rx, (y as f64 - c.1) / ry);
            if d < 1.0 {
                let a = k * (1.0 - d * d);
                for v in img.px_mut(x as usize, y as usize).iter_mut().take(3) {
                    *v *= 1.0 - a;
                }
            }
        }
    }
}

/// Points on an ellipse arc around `c` (radius `r`, squashed by `sq`) from `a0` to `a1`.
fn arc_pts(c: (f64, f64), r: f64, a0: f64, a1: f64, sq: f64, n: usize) -> Vec<(f64, f64)> {
    (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f64 / n as f64;
            (c.0 + dmath::cos(a) * r, c.1 + dmath::sin(a) * r * sq)
        })
        .collect()
}

/// A glowing stroke through `pts` that tapers at both ends: a wide coloured glow, a bright core, a white centre.
fn taper(img: &mut Img<f64>, pts: &[(f64, f64)], col: V3, k: f64, w: f64) {
    if k <= 0.01 || pts.len() < 2 {
        return;
    }
    let n = pts.len() - 1;
    for (mul, c, al) in [(2.6, col, 0.35), (1.0, mix(col, WHITE, 0.55), 0.8), (0.4, WHITE, 0.9)] {
        for i in 0..n {
            let f = (i as f64 + 0.5) / n as f64;
            let ww = (w * dmath::sin(dmath::PI * f) * mul).max(0.6);
            line(img, pts[i], pts[i + 1], c, k * al, k * al, ww);
        }
    }
}

/// A four-point star flash at `c`: a soft core and two crossed spikes, `r` px, turned by `rot`.
fn star(img: &mut Img<f64>, c: (f64, f64), r: f64, col: V3, k: f64, rot: f64) {
    if k <= 0.01 {
        return;
    }
    disc(img, c.0, c.1, r * 0.55, mix(col, WHITE, 0.6), 0.9 * k);
    disc(img, c.0, c.1, r * 0.25, WHITE, k);
    for j in 0..2 {
        let a = rot + j as f64 * dmath::PI / 2.0;
        let (ca, sa) = (dmath::cos(a) * r, dmath::sin(a) * r);
        line(img, (c.0 - ca, c.1 - sa), c, mix(col, WHITE, 0.7), 0.0, k, 2.0);
        line(img, c, (c.0 + ca, c.1 + sa), mix(col, WHITE, 0.7), k, 0.0, 2.0);
    }
}

/// A closed-form spray of `n` streaks from `c`, `u` s after it started: speed `speed` px/s, gravity `grav`, life
/// `life` s, inside a cone `(angle, spread)` or all round.
#[allow(clippy::too_many_arguments)]
fn burst(
    img: &mut Img<f64>,
    c: (f64, f64),
    u: f64,
    seed: u64,
    n: usize,
    col: V3,
    speed: f64,
    grav: f64,
    life: f64,
    cone: Option<(f64, f64)>,
) {
    if !(0.0..life).contains(&u) {
        return;
    }
    let mut m = Mix(seed.wrapping_mul(2654435761) | 1);
    for _ in 0..n {
        let a = match cone {
            Some((ang, sp)) => ang + (m.next() - 0.5) * sp,
            None => m.next() * 2.0 * dmath::PI,
        };
        let sp = speed * (0.3 + 0.9 * m.next());
        let lf = life * (0.5 + 0.5 * m.next());
        let white = m.next() < 0.3;
        if u > lf {
            continue;
        }
        let (ca, sa) = (dmath::cos(a), dmath::sin(a));
        let x = c.0 + ca * sp * u;
        let y = c.1 + sa * sp * u * 0.7 + grav * u * u;
        let k = 1.0 - u / lf;
        let tail = (x - ca * sp * 0.035, y - (sa * sp * 0.7 * 0.035 + grav * u * 0.07));
        line(
            img,
            tail,
            (x, y),
            if white { WHITE } else { col },
            0.0,
            k,
            1.0 + 1.2 * k,
        );
    }
}
