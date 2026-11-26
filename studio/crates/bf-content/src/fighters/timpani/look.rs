//! TIMPANI's look (a Studio fighter: no kit look to match). The ball is its model (a concert pedal timpani built from
//! web photos). On the floor stand its kettles, from the recorded `KETTLES`: copper bowls seen from above, a cream
//! head ringed by a chrome hoop with eight rod ticks (the high kettle smaller, FULL SET's two in between). Each boom
//! sends a copper ring across the floor from its kettle (`KRINGS`) and flashes the head; a hop lifts the ball in an
//! arc and squashes it on landing; PEDAL SLIDE leaves skid dust behind the gliding kettles; the interval hit is a
//! bright double burst; FULL SET drops two kettles in with a flare. It shatters into copper, chrome rods and head.

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{py_max, py_min};
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::fx::Emit;
use bf_picture::img::Img;
use bf_picture::look::{FrameCtx, Fx, Look, Phase, Spark};
use bf_picture::np;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::BallState;
use bf_picture::ring::{self, PictureCtx};
use bf_picture::scene::{R_BALL as R, to_world};

use super::state::{self, Boom, Hop};

/// The same moves with the bold effects.
#[path = "bold.rs"]
mod bold;

/// The kettle's copper.
const COPPER: V3 = [184.0, 100.0, 46.0];
/// Copper in shadow.
const COPPER_DK: V3 = [96.0, 40.0, 18.0];
/// The bright copper of a ring.
const COPPER_HI: V3 = [242.0, 167.0, 102.0];
/// The head.
const HEAD: V3 = [237.0, 230.0, 218.0];
/// The counterhoop and rods.
const CHROME: V3 = [222.0, 226.0, 234.0];
/// Skid dust.
const DUST: [V3; 2] = [[200.0, 186.0, 160.0], [150.0, 136.0, 116.0]];
/// The height of the kettles' heads above the floor.
const HEAD_Y: f64 = 0.03;

/// A hit's kind (`e[7]`), or `""`.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// The event's fighter as a side.
fn owner(e: &Event) -> usize {
    e.who().filter(|&k| k < 2).unwrap_or(0)
}

/// The numbers of a recorded row: `(owner, i, x, z, r, ...)`.
fn row(v: &Value) -> Option<Vec<f64>> {
    let Value::List(r) = v else { return None };
    r.iter()
        .map(|x| match x {
            Value::Float(x) => Some(*x),
            Value::Int(n) => Some(*n as f64),
            _ => None,
        })
        .collect()
}

/// A circle on the floor at height `y`, `n` points, on the screen.
fn circle(cam: &Cam, x: f64, z: f64, r: f64, y: f64, n: usize) -> Vec<(f64, f64)> {
    ring::screen(cam, &ring::ring_circle(x, z, r, y, n))
}

/// A band (annulus) from radius `r0` to `r1` round `(x, z)`, as quads.
fn band(cam: &Cam, x: f64, z: f64, r0: f64, r1: f64, y: f64, n: usize) -> Vec<Vec<(f64, f64)>> {
    (0..n)
        .map(|j| {
            let a0 = 2.0 * dmath::PI * j as f64 / n as f64;
            let a1 = 2.0 * dmath::PI * (j + 1) as f64 / n as f64;
            let p = |r: f64, a: f64| to_world(x + r * dmath::cos(a), z + r * dmath::sin(a), y);
            ring::screen(cam, &[p(r0, a0), p(r1, a0), p(r1, a1), p(r0, a1)])
        })
        .collect()
}

/// One kettle seen from above at `(x, z)`, radius `r`; `flash` (0 to 1) lights its head after a boom.
fn draw_kettle(arr: &mut Img<f32>, cam: &Cam, x: f64, z: f64, r: f64, flash: f64) {
    // the bowl's shadow side, then the bowl, the hoop and the head
    ring::layer_blend(
        arr,
        &[circle(cam, x + 0.03, z + 0.04, r * 1.04, 0.01, 32)],
        COPPER_DK,
        0.8,
        None,
    );
    ring::layer_blend(arr, &[circle(cam, x, z, r, HEAD_Y * 0.6, 32)], COPPER, 1.0, None);
    ring::layer_blend(arr, &[circle(cam, x, z, r * 0.9, HEAD_Y, 32)], CHROME, 1.0, None);
    let head: V3 = std::array::from_fn(|c| np::clip(HEAD[c] + (255.0 - HEAD[c]) * flash, 0.0, 255.0));
    ring::layer_blend(arr, &[circle(cam, x, z, r * 0.82, HEAD_Y, 32)], head, 1.0, None);
    // eight rod heads round the hoop
    let rods: Vec<Vec<(f64, f64)>> = (0..8)
        .map(|j| {
            let a = dmath::PI / 8.0 + dmath::PI * j as f64 / 4.0;
            let (cx, cz) = (x + r * 0.95 * dmath::cos(a), z + r * 0.95 * dmath::sin(a));
            let d = r * 0.07;
            ring::screen(
                cam,
                &[
                    to_world(cx - d, cz - d, HEAD_Y + 0.01),
                    to_world(cx + d, cz - d, HEAD_Y + 0.01),
                    to_world(cx + d, cz + d, HEAD_Y + 0.01),
                    to_world(cx - d, cz + d, HEAD_Y + 0.01),
                ],
            )
        })
        .collect();
    ring::layer_blend(arr, &rods, CHROME.map(|c| c * 0.8), 1.0, None);
}

/// `hop_state(u, L)`: the height of a hop of length `l` at progress `u` and its squash.
fn hop_state(u: f64, l: f64) -> (f64, f64, f64) {
    let hgt = (0.35 + 0.12 * py_min(3.0, l)) * 4.0 * u * (1.0 - u);
    let v = (1.0 - 2.0 * u).abs();
    let sy = 1.0 + 0.14 * v;
    (hgt, 1.0 / sy.sqrt(), sy)
}

/// The squash on landing.
fn land_squash(age: f64) -> (f64, f64) {
    if !(0.0..=0.2).contains(&age) {
        return (1.0, 1.0);
    }
    if age < 0.07 {
        let k = 1.0 - age / 0.07 * 0.3;
        return (1.0 + 0.2 * k, 1.0 - 0.22 * k);
    }
    let u = (age - 0.07) / 0.13;
    let s = 0.05 * dmath::sin(dmath::PI * u) * (1.0 - u);
    (1.0 - s, 1.0 + s)
}

/// TIMPANI's look.
#[derive(Default)]
pub struct TimpaniLook {
    k: usize,
    hops: Vec<Hop>,
    booms: Vec<Boom>,
    slides: Vec<(f64, f64)>,
    full: Option<f64>,
}

impl std::fmt::Debug for TimpaniLook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimpaniLook")
            .field("k", &self.k)
            .field("hops", &self.hops.len())
            .field("booms", &self.booms.len())
            .finish()
    }
}

impl TimpaniLook {
    /// Its rows of `key` in recording frame `i`.
    fn rows(&self, p: &PictureCtx, key: &str, i: usize) -> Vec<Vec<f64>> {
        if !p.run.has(key) {
            return Vec::new();
        }
        match p.run.nested(key, i) {
            Some(Value::List(rows)) => rows
                .iter()
                .filter_map(row)
                .filter(|r| r.first().is_some_and(|&o| o as usize == self.k))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The hop in the air at `t`, and how far along it is.
    fn hop_at(&self, t: f64) -> Option<(&Hop, f64)> {
        let j = self.hops.partition_point(|h| h.t <= t).checked_sub(1)?;
        let h = &self.hops[j];
        (t < h.t1).then(|| (h, (t - h.t) / py_max(1e-6, h.t1 - h.t)))
    }

    /// Seconds since kettle `i` last boomed (99 before its first).
    fn since_boom(&self, i: i64, t: f64) -> f64 {
        self.booms
            .iter()
            .rev()
            .find(|b| b.kettle == i && b.t <= t)
            .map_or(99.0, |b| t - b.t)
    }

    /// Seconds since the last landing.
    fn since_land(&self, t: f64) -> f64 {
        self.hops.iter().rev().find(|h| h.t1 <= t).map_or(99.0, |h| t - h.t1)
    }

    /// Where ball `kk` is at `t`, in the world, `y` up.
    fn at(&self, p: &PictureCtx, kk: usize, t: f64, y: f64) -> V3 {
        let q = p.run.p(p.sim_i(py_min(t, p.t_ko)), kk);
        to_world(q[0], q[1], y)
    }
}

impl Look for TimpaniLook {
    /// Its own panel on the VS card ([`crate::fighters::shared::panels::timpani`]).
    fn title_pattern(&self, base: &mut bf_picture::img::RgbF) {
        crate::fighters::shared::panels::timpani(base);
    }

    fn shards(&self) -> Vec<String> {
        [("copper", 8), ("rod", 4), ("head", 3)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["interval"]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    fn cfg_summary(&self) -> Option<String> {
        Some(String::new())
    }

    fn setup_as(&mut self, p: &PictureCtx, k: usize, _cfg: &FighterCfg) {
        self.k = k;
        let ev = &p.rec.events;
        self.hops = state::hops(ev, k);
        self.booms = state::booms(ev, k);
        self.slides = state::slides(ev, k);
        self.full = state::full_set(ev, k);
    }

    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if p.bold {
            return self.bold_state(p, kk, st, t, phase);
        }
        if kk != self.k || !matches!(phase, Phase::Fight) || t >= p.t_ko {
            return;
        }
        if let Some((h, u)) = self.hop_at(t) {
            let i0 = p.sim_i(h.t);
            let i1 = p.sim_i(py_min(h.t1, p.t_ko));
            let (a, b) = (p.run.p(i0, kk), p.run.p(i1, kk));
            let (lift, sx, sy) = hop_state(u, np::hypot(b[0] - a[0], b[1] - a[1]));
            st.lift = lift;
            st.sx = Some(sx);
            st.sy = Some(sy);
            if lift > 0.02 {
                st.air_bias = 0.6;
                st.paint_h = Some(R);
            }
        } else {
            let (sx, sy) = land_squash(self.since_land(t));
            st.sx = Some(sx);
            st.sy = Some(sy);
        }
    }

    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match (e.kind.as_str(), how(e)) {
            ("hit", "kettle") if owner(e) == self.k => Some(0.3),
            ("hit", "interval") if owner(e) == self.k => Some(0.7),
            ("full_set", _) if e.who() == Some(self.k) => Some(0.8),
            _ => None,
        }
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let col = p.col[owner(e)];
        match how(e) {
            "kettle" => (COPPER_HI.map(|c| c * 0.6 + 100.0), 6, 2.0, 0.2),
            "interval" => (CHROME.map(|c| c * 0.6 + 100.0), 10, 2.4, 0.24),
            _ => (col.map(|c| c * 0.6 + 100.0), 5, 1.8, 0.18),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "kettle" => Some((6.0, 0.18)),
            "interval" => Some((9.0, 0.24)),
            _ => None,
        }
    }

    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        // every boom: a warm flash over the head
        for b in &self.booms {
            if b.t > p.t_ko {
                break;
            }
            fx.fl.add(
                b.t,
                up(b.x, b.z, 0.2),
                COPPER_HI,
                1.0 + 0.3 * b.reach,
                0.5,
                0.01,
                0.18,
                0.0,
            );
        }
        // every landing: a puff of dust round the ball
        for h in &self.hops {
            if h.t1 > p.t_ko {
                break;
            }
            let dust = Emit {
                n: Some(4),
                spread: Some(180.0),
                col: Some(DUST.to_vec()),
                scale: 0.6,
                ..Emit::default()
            };
            fx.pa.emit(h.t1, self.at(p, self.k, h.t1, 0.05), "dust", &dust);
        }
        // PEDAL SLIDE: a flare on the ball and skid dust along the way
        for &(t0, t1) in &self.slides {
            if t0 > p.t_ko {
                break;
            }
            fx.fl
                .add(t0, self.at(p, self.k, t0, R), CHROME, 1.4, 0.5, 0.02, 0.25, 0.0);
            let n = ((t1.min(p.t_ko) - t0) / 0.1).floor().max(0.0) as usize;
            for j in 0..n {
                let t = t0 + j as f64 * 0.1;
                let dust = Emit {
                    n: Some(2),
                    col: Some(DUST.to_vec()),
                    scale: 0.5,
                    ..Emit::default()
                };
                fx.pa.emit(t, self.at(p, 1 - self.k, t, 0.05), "dust", &dust);
            }
        }
        // FULL SET: a flare and a burst of copper
        if let Some(t) = self.full.filter(|&t| t <= p.t_ko) {
            let pos = self.at(p, self.k, t, R);
            fx.fl.add(t, pos, COPPER_HI, 2.4, 0.9, 0.03, 0.5, 0.0);
            let sparks = Emit {
                n: Some(24),
                spread: Some(180.0),
                col: Some(vec![COPPER_HI, CHROME, HEAD]),
                ..Emit::default()
            };
            fx.pa.emit(t, pos, "sparks", &sparks);
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.kind == "hit" && owner(e) == self.k && how(e) == "interval" {
            let pos = up(e.f(5), e.f(6), R * 0.6);
            fx.bu.add(e.t, pos, COPPER_HI, 1.0, None, true);
            fx.bu.add(e.t + 0.05, pos, p.col[self.k], 0.8, None, true);
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let sparks = Emit {
            n: Some(26),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            col: Some(vec![COPPER, CHROME, HEAD, p.col[self.k]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
    }

    /// The kettles and their rings, on the floor under the balls.
    fn canvas_overlay(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        if p.bold {
            return self.bold_canvas_overlay(p, f, arr);
        }
        if !f.in_fight || f.t >= p.t_hs {
            return;
        }
        let i = p.sim_i(py_min(f.t, p.t_ko));
        for r in self.rows(p, "KRINGS", i) {
            let [_, _, x, z, rad, reach] = r[..] else { continue };
            let fade = py_max(0.0, 1.0 - rad / py_max(1e-6, reach));
            let w = 0.03 + 0.05 * fade;
            let quads = band(f.cam, x, z, py_max(0.0, rad - w), rad + w, 0.015, 40);
            ring::layer_blend(arr, &quads, COPPER_HI, 0.35 + 0.55 * fade, None);
        }
        for r in self.rows(p, "KETTLES", i) {
            let [_, ki, x, z, kr] = r[..] else { continue };
            let age = self.since_boom(ki as i64, f.t);
            let flash = if age < 0.25 { 1.0 - age / 0.25 } else { 0.0 };
            draw_kettle(arr, f.cam, x, z, kr, flash);
        }
    }

    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &bf_picture::look::CanvasCtx<'_>) {
        if p.bold {
            self.bold_canvas(p, ic, c);
        }
    }

    fn after_grade(
        &self,
        p: &PictureCtx,
        img: &mut bf_picture::look::Pic,
        f: &FrameCtx<'_>,
        drawn: &[bf_picture::look::DrawnBall],
    ) {
        if p.bold {
            self.bold_after_grade(p, img, f, drawn);
        }
    }

    fn bold_hit_size(
        &self,
        _p: &PictureCtx,
        e: &Event,
        by_damage: bf_picture::bold::HitSize,
    ) -> Option<bf_picture::bold::HitSize> {
        Self::bold_hit(e, by_damage)
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_shard(
        &self,
        _p: &PictureCtx,
        arr: &mut Img<f32>,
        cam: &Cam,
        kind: &str,
        pos: V3,
        rot: f64,
        size: f64,
        _curl: f64,
    ) {
        let (sx, sz) = (pos[0], pos[2]);
        let d = [dmath::cos(rot), dmath::sin(rot)];
        let nn = [-d[1], d[0]];
        let at = |q: [f64; 2]| to_world(q[0], q[1], pos[1]);
        let (ln, wd, col) = match kind {
            "copper" => (0.12, 0.09, COPPER),
            "rod" => (0.14, 0.02, CHROME),
            _ => (0.09, 0.08, HEAD),
        };
        let q4: Vec<V3> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .iter()
            .map(|&(a1, a2)| {
                at([
                    sx + d[0] * ln * a1 * size / 0.1 + nn[0] * wd * a2,
                    sz + d[1] * ln * a1 * size / 0.1 + nn[1] * wd * a2,
                ])
            })
            .collect();
        let sp = ring::screen(cam, &q4);
        let shade_k = 0.75 + 0.25 * dmath::cos(rot * 2.0);
        ring::layer_blend(arr, &[sp], col.map(|c| np::clip(c * shade_k, 0.0, 255.0)), 1.0, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hop_rises_and_falls() {
        let (h0, _, _) = hop_state(0.0, 2.0);
        let (hm, sx, sy) = hop_state(0.5, 2.0);
        let (h1, _, _) = hop_state(1.0, 2.0);
        assert_eq!((h0, h1), (0.0, 0.0));
        assert!(hm > 0.5, "{hm}");
        assert!((sx - 1.0).abs() < 1e-9 && (sy - 1.0).abs() < 1e-9);
        assert_eq!(land_squash(1.0), (1.0, 1.0));
        assert!(land_squash(0.01).0 > 1.0);
    }

    #[test]
    fn a_recorded_row_reads() {
        let v = Value::List(vec![
            Value::Int(1),
            Value::Int(0),
            Value::Float(0.5),
            Value::Float(-1.0),
            Value::Float(0.6),
        ]);
        assert_eq!(row(&v), Some(vec![1.0, 0.0, 0.5, -1.0, 0.6]));
        assert!(row(&Value::Int(3)).is_none());
    }
}
