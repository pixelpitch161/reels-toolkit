//! HARP's look: #35's `render.py` layers for the harp and the `styles.py` draw code they call (`string_points`,
//! `string_twang`, `draw_string`, `draw_dart`, `wing_polys`, `draw_wings`, `draw_halo`), close to the episode by
//! eye (ruling M4-R16), not to the bit.
//!
//! What it draws: its strings across the ring, each stretched in from the end nearer the harp, plucked into a
//! decaying standing wave and glowing brighter while it rings after a crossing, fading when it is dropped and after
//! the K.O.; its PLUCK darts in flight (the run's projectiles it owns); ANGEL WINGS (feather fans flapping out from
//! behind the ball) with a gold halo above it while the shield is up; the ball thumping on every string it lays.

use bf_core::dmath::{self, PI};
use bf_core::event::Event;
use bf_core::py::{py_max, py_min};
use bf_core::roster::{AbilityKind, FighterCfg};
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_picture::look::{FrameCtx, Look, Payload, Phase, Spark};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::BallState;
use bf_picture::scene::{self, R_BALL as R};

use crate::fighters::shared::older::{
    Dart, PEARL_35 as PEARL, Poly, blend_into, draw_dart, mask_of_lines, mask_of_polys,
};
use crate::fighters::shared::reeds::Num;

/// The same moves with the bold effects.
#[path = "bold.rs"]
mod bold;

/// `STR_Y`: a string's height (just under the balls' centres).
const STR_Y: f64 = 0.50;
/// `DRAW_IN`: seconds to stretch a new string across.
const DRAW_IN: f64 = 0.16;
/// `FADE_OUT`: seconds a dropped string takes to fade.
const FADE_OUT: f64 = 0.35;
/// `FEATHER`.
const FEATHER: V3 = [250.0, 246.0, 236.0];
/// `FEATHER_DK`.
const FEATHER_DK: V3 = [206.0, 196.0, 182.0];
/// `GOLD`.
const GOLD: V3 = [255.0, 206.0, 110.0];

/// A string as the look rebuilds it from the run (#35 `sim.hazards`).
#[derive(Clone, Debug, PartialEq)]
struct Str {
    id: i64,
    t0: f64,
    /// Its ends, the one nearer the harp when it was laid first.
    a: [f64; 2],
    b: [f64; 2],
    t_off: Option<f64>,
    hits: Vec<f64>,
}

/// What the look draws in the painter's list.
enum Item {
    /// A string: its world points, alpha and twang.
    Str(Vec<V3>, f64, f64),
    /// A dart: `(x, z, ang)`.
    Dart(f64, f64, f64),
    /// ANGEL WINGS at strength `k`.
    Wings(f64),
    /// The halo at strength `k`.
    Halo(f64),
}

/// `smoothstep(a, b, x)`.
fn ss(a: f64, b: f64, x: f64) -> f64 {
    let u = ((x - a) / (b - a)).clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// `e[7]` of a hit.
fn how(e: &Event) -> &str {
    if e.fields.len() > 5 { e.s(7) } else { "" }
}

/// HARP's look.
#[derive(Clone, Debug, Default)]
pub struct HarpLook {
    k: usize,
    strings: Vec<Str>,
    /// `STR_T`: when each string was laid (the strings in that order).
    str_t: Vec<f64>,
    /// ANGEL WINGS's `(name, seconds)`.
    shield: Option<(String, f64)>,
}

impl HarpLook {
    /// `string_state(st, t)`: `(alpha, drawn fraction)`, or `None` when it is gone.
    fn string_state(s: &Str, t: f64) -> Option<(f64, f64)> {
        if t < s.t0 {
            return None;
        }
        let drawn = py_min(1.0, (t - s.t0) / DRAW_IN);
        let mut a = 1.0;
        if let Some(off) = s.t_off
            && t >= off
        {
            a = 1.0 - (t - off) / FADE_OUT;
            if a <= 0.0 {
                return None;
            }
        }
        Some((a, drawn))
    }

    /// `string_points(st, t)`: 17 world points, plucked where it was crossed, and its alpha.
    fn string_points(s: &Str, t: f64) -> Option<(Vec<V3>, f64)> {
        let (alpha, drawn) = Self::string_state(s, t)?;
        let (a, b) = (s.a, s.b);
        let d = [b[0] - a[0], b[1] - a[1]];
        let dl = dmath::hypot(d[0], d[1]) + 1e-9;
        let nrm = [-d[1] / dl, d[0] / dl];
        let n = 17;
        let pts = (0..n)
            .map(|j| {
                let u = j as f64 / (n - 1) as f64 * drawn;
                let mut disp = 0.0;
                for &th in &s.hits {
                    let dt = t - th;
                    if (0.0..0.9).contains(&dt) {
                        disp += 0.16 * dmath::exp(-dt / 0.22) * dmath::sin(2.0 * PI * 9.0 * dt) * dmath::sin(PI * u);
                    }
                }
                let x = a[0] + d[0] * u + nrm[0] * disp;
                let z = a[1] + d[1] * u + nrm[1] * disp;
                scene::to_world(x, z, STR_Y + 0.05 * disp.abs())
            })
            .collect();
        Some((pts, alpha))
    }

    /// `string_twang(st, t)`: 0..1, how hard the string is ringing.
    fn string_twang(s: &Str, t: f64) -> f64 {
        s.hits
            .iter()
            .filter(|&&th| (0.0..0.6).contains(&(t - th)))
            .fold(0.0, |k, &th| py_max(k, dmath::exp(-(t - th) / 0.2)))
    }

    /// `strings_at(t)`: the strings on screen, `(points, alpha, twang)`.
    fn strings_at(&self, p: &PictureCtx, t: f64) -> Vec<(Vec<V3>, f64, f64)> {
        let mut out = Vec::new();
        if self.strings.is_empty() {
            return out;
        }
        let tc = p.fx_clock(t);
        let fade = if t >= p.t_ko {
            1.0 - ss(p.t_hs + 0.5, p.t_hs + 1.1, t)
        } else {
            1.0
        };
        if fade <= 0.0 || tc < p.show.grid.t_fight {
            return out;
        }
        let j1 = self.str_t.partition_point(|&v| v <= tc);
        for s in &self.strings[j1.saturating_sub(12)..j1] {
            if let Some((pts, a)) = Self::string_points(s, tc) {
                out.push((pts, a * fade, Self::string_twang(s, tc)));
            }
        }
        out
    }

    /// `shield_k(st)`: ANGEL WINGS's strength, fading in over 0.12 s and out over the last 0.2 s.
    fn shield_k(&self, st: &BallState) -> f64 {
        let Some((_, dur)) = &self.shield else { return 0.0 };
        if st.shield <= 0.0 {
            return 0.0;
        }
        py_min(py_min(1.0, st.shield / 0.2), (dur - st.shield) / 0.12 + 0.2)
    }
}

/// `draw_string(arr, cam, P, alpha, col, twang)`: a 1-px bright core over a 3-px soft halo.
fn draw_string(arr: &mut Img<f32>, cam: &Cam, pts: &[V3], alpha: f64, col: V3, twang: f64) {
    let s: Poly = cam.wp(pts);
    let soft = col.map(|c| (c * 0.75 + 40.0).clamp(0.0, 255.0));
    let core = PEARL.map(|c| (c + 40.0 * twang).clamp(0.0, 255.0));
    for (width, k, c) in [(3, 0.30 + 0.35 * twang, soft), (1, 0.95, core)] {
        let m = mask_of_lines(std::slice::from_ref(&s), Some(width));
        blend_into(arr, &m, c, Num::Py(alpha * k));
    }
}

/// `wing_polys(cam, cx, cz, t, side, k)`: one wing (side -1 / +1, screen left / right) as six feather polygons
/// fanned up and out from behind the ball.
fn wing_polys(cam: &Cam, cx: f64, cz: f64, t: f64, side: f64, k: f64) -> Vec<(Poly, usize)> {
    let flap = 0.18 * dmath::sin(2.0 * PI * t / 0.95);
    let base = scene::to_world(cx, cz, R * 1.1);
    let bs = cam.wp(&[base])[0];
    let rp = R * cam.f / cam.depth(&[base])[0];
    (0..6)
        .map(|j| {
            let jf = j as f64;
            let a = (18.0 + 22.0 * jf).to_radians() + flap * (1.0 + 0.3 * jf);
            let l = rp * (2.2 - 0.18 * jf) * (0.4 + 0.6 * k);
            let w = rp * 0.36;
            let d = [side * dmath::cos(a), -dmath::sin(a)];
            let nn = [-d[1], d[0]];
            let o = (bs.0 + side * rp * 0.45, bs.1 - rp * 0.25);
            let tip = (o.0 + d[0] * l, o.1 + d[1] * l);
            let pt = |q: (f64, f64), n: f64, dd: f64| (q.0 + nn[0] * n + d[0] * dd, q.1 + nn[1] * n + d[1] * dd);
            (
                vec![
                    pt(o, w * 0.25, 0.0),
                    pt(o, -w * 0.25, 0.0),
                    pt(tip, -w * 0.5, -w * 0.4),
                    tip,
                    pt(tip, w * 0.5, -w * 0.4),
                ],
                j,
            )
        })
        .collect()
}

/// `draw_wings(arr, cam, cx, cz, t, k)`.
fn draw_wings(arr: &mut Img<f32>, cam: &Cam, cx: f64, cz: f64, t: f64, k: f64) {
    for side in [-1.0, 1.0] {
        for (pts, j) in wing_polys(cam, cx, cz, t, side, k) {
            let m = mask_of_polys(&[pts]);
            let c = if j % 2 == 0 { FEATHER } else { FEATHER_DK };
            blend_into(arr, &m, c, Num::Py(0.92 * k));
        }
    }
}

/// `draw_halo(arr, cam, cx, cz, t, k)`: a gold ring bobbing above the ball.
fn draw_halo(arr: &mut Img<f32>, cam: &Cam, cx: f64, cz: f64, t: f64, k: f64) {
    let y = R * 2.35 + 0.05 * dmath::sin(2.0 * PI * t / 0.95);
    let pts: Vec<V3> = (0..29)
        .map(|j| {
            let a = 2.0 * PI * j as f64 / 28.0;
            scene::to_world(cx + 0.42 * dmath::cos(a), cz + 0.42 * dmath::sin(a), y)
        })
        .collect();
    let m = mask_of_lines(&[cam.wp(&pts)], Some(2));
    blend_into(arr, &m, GOLD, Num::Py(0.95 * k));
}

impl Look for HarpLook {
    fn spark_big(&self) -> &[&str] {
        &["wave", "shockwave"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["wave", "shockwave"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["wave"]
    }

    /// A hit on ANGEL WINGS: a pearl spray.
    fn block_burst(&self) -> [f64; 3] {
        [255.0, 246.0, 222.0]
    }

    fn block_spark(&self) -> Spark {
        ([255.0, 236.0, 190.0], 12, 2.6, 0.3)
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        let ev = &p.rec.events;
        let mut strings: Vec<Str> = ev
            .iter()
            .filter(|e| e.kind == "string" && e.who() == Some(k))
            .map(|e| {
                let (a, b) = ([e.f(4), e.f(5)], [e.f(6), e.f(7)]);
                // it grows from the end nearer the harp as it stood when it laid the string
                let hp = p.fighter(k, e.t).p;
                let near_b = dmath::hypot(hp[0] - b[0], hp[1] - b[1]) < dmath::hypot(hp[0] - a[0], hp[1] - a[1]);
                let (a, b) = if near_b { (b, a) } else { (a, b) };
                Str {
                    id: e.i(3),
                    t0: e.t,
                    a,
                    b,
                    t_off: None,
                    hits: Vec::new(),
                }
            })
            .collect();
        for e in ev {
            if e.who() != Some(k) {
                continue;
            }
            if e.kind == "string_off" {
                let id = e.i(3);
                if let Some(s) = strings.iter_mut().find(|s| s.id == id) {
                    s.t_off = Some(e.t);
                }
            } else if e.kind == "hit" && how(e) == "string" {
                let id = e.i(8);
                if let Some(s) = strings.iter_mut().find(|s| s.id == id) {
                    s.hits.push(e.t);
                }
            }
        }
        self.str_t = strings.iter().map(|s| s.t0).collect();
        self.strings = strings;
        let sh = [&cfg.unlock1, &cfg.unlock2, &cfg.low]
            .into_iter()
            .find(|a| a.kind == AbilityKind::Shield);
        self.shield = sh.map(|a| (a.name.clone(), a.get("dur_beats").unwrap_or(6.0) * p.show.grid.beat));
    }

    /// The harp ball thumps on every string it lays.
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if kk != self.k || phase != Phase::Fight || self.str_t.is_empty() {
            return;
        }
        let tt = py_min(t, p.t_ko);
        let after = self.str_t.partition_point(|&v| v <= tt);
        if let Some(j) = after.checked_sub(1) {
            st.glow = 0.42 * dmath::exp(-(tt - self.str_t[j]) / 0.10);
        }
    }

    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &bf_picture::look::CanvasCtx<'_>) {
        if p.bold {
            self.bold_canvas(p, ic, c);
        }
    }

    fn canvas_wave(
        &self,
        p: &PictureCtx,
        ic: &mut [V3],
        c: &bf_picture::look::CanvasCtx<'_>,
        w: &bf_picture::ring::state::Wave,
    ) {
        if p.bold {
            self.bold_canvas_wave(p, ic, c, w);
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

    fn items_ball(&self, p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        if p.bold {
            return self.bold_items_ball(f, st);
        }
        let sk = self.shield_k(st);
        if sk <= 0.0 {
            return Vec::new();
        }
        let dz = f.cam.depth(&[scene::to_world(st.p[0], st.p[1], R)])[0];
        let wings: Payload = Box::new(Item::Wings(sk));
        let halo: Payload = Box::new(Item::Halo(sk));
        vec![(dz + 0.05, wings), (dz - 0.05, halo)]
    }

    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if p.bold {
            return self.bold_items_free(p, f);
        }
        let mut out: Vec<(f64, Payload)> = Vec::new();
        for (pts, a, tw) in self.strings_at(p, f.t) {
            let mut m = [0.0; 3];
            for q in &pts {
                for i in 0..3 {
                    m[i] += q[i] / pts.len() as f64;
                }
            }
            out.push((f.cam.depth(&[m])[0], Box::new(Item::Str(pts, a, tw))));
        }
        if f.in_fight
            && let Some(rows) = p.run.frame_values("K", f.i)
        {
            for row in rows.as_chunks::<7>().0 {
                if row[0].is_nan() || row[3] as i64 != self.k as i64 {
                    continue;
                }
                let d = f.cam.depth(&[scene::to_world(row[0], row[1], 0.45)])[0];
                out.push((d, Box::new(Item::Dart(row[0], row[1], row[2]))));
            }
        }
        out
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        if p.bold {
            return self.bold_draw_item(p, f, fr, payload);
        }
        let col = p.col[self.k];
        let at = f.fs[self.k].p;
        match payload.downcast_ref::<Item>() {
            Some(Item::Dart(x, z, ang)) => draw_dart(fr, f.cam, f.l, *x, *z, *ang, col, Dart::Harp),
            Some(item) => {
                let (mut arr, depth) = (fr.array(), fr.depth());
                match item {
                    Item::Str(pts, a, tw) => draw_string(&mut arr, f.cam, pts, *a, col, *tw),
                    Item::Wings(k) => draw_wings(&mut arr, f.cam, at[0], at[1], f.t, *k),
                    Item::Halo(k) => draw_halo(&mut arr, f.cam, at[0], at[1], f.t, *k),
                    Item::Dart(..) => {}
                }
                fr.set_array(&arr, Some(&depth));
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string() -> Str {
        Str {
            id: 0,
            t0: 1.0,
            a: [-3.5, 0.0],
            b: [3.5, 0.0],
            t_off: Some(3.0),
            hits: vec![2.0],
        }
    }

    #[test]
    fn a_string_stretches_in_rings_and_fades() {
        let s = string();
        assert_eq!(HarpLook::string_state(&s, 0.5), None);
        assert_eq!(HarpLook::string_state(&s, 1.5), Some((1.0, 1.0)));
        assert!(HarpLook::string_state(&s, 3.4).is_none());
        let (pts, a) = HarpLook::string_points(&s, 2.05).expect("up");
        assert_eq!((pts.len(), a), (17, 1.0));
        assert!(HarpLook::string_twang(&s, 2.05) > 0.7);
        assert_eq!(HarpLook::string_twang(&s, 2.7), 0.0);
    }
}
