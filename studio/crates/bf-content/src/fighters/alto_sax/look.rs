//! ALTO SAX's look: the port of `Look` in `projects/ballfight/kit/fighters/alto_sax.py`, hook by hook.
//!
//! What it draws: each light as a cone from a lamp high in the rafters down to a warm pool on the canvas (the pool
//! brightens the balls it passes over, a point light; the arena dims only in a ring around the pool); OCTAVE KEY's
//! lights tinted orchid, ALTISSIMO's a narrow white-hot pool with a hard core; scorch marks and embers where the
//! light burned; ENCORE's chase light, and the phones as white pinpricks on the canvas with faint rays back to the
//! stands; its bell glowing while a light runs; brass tube pieces, key cups and pearls when it shatters. Its VS
//! card panel: spotlight rays fanning down.
//!
//! The lamp cone's outline is a `ConvexHull` call (`bf_picture::hull::convex_hull`). Where scipy raises (flat or
//! too few points) the kit's `try / except` skips that light's cone and lamp glow: so does this. Where Qhull would
//! merge, the hull module answers with its plain scan and counts the fallback (ruling M3-R4): the look draws that
//! answer, and the gates stop on the count.
//!
//! The kit's look reads no roster number (`self.cfg`): everything comes from the run (the `spot`, `burn`, `encore`
//! and `phone_burn` events and the `SPOT` rows).

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{py_max, py_min};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{self, H, W};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::hull::{self, Unusual};
use bf_picture::img::{Img, RgbF};
use bf_picture::lighting::Lights;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Fx, Look, Phase, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::state::BallState;
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};

use crate::fighters::shared::reeds::{self, Num, WHITE};

/// `WARM`: the pool's light (a tungsten follow-spot).
const WARM: V3 = [1.0, 0.93, 0.80];
/// `HOT`: ALTISSIMO, white-hot.
const HOT: V3 = [1.05, 1.05, 1.12];
/// `BRASS`.
const BRASS: V3 = [232.0, 182.0, 70.0];
/// `KEYG`.
const KEYG: V3 = [255.0, 214.0, 110.0];
/// `PEARL`.
const PEARL: V3 = [250.0, 246.0, 236.0];
/// `LAMP_H`: the rafters, the lamps' height above the canvas.
const LAMP_H: f64 = 7.6;
/// `LAMP_BACK`: the lamps are set back from the pool (away from the camera), so the cone reads.
const LAMP_BACK: f64 = 1.6;

/// A numpy boolean as a factor: `x * (a > b)`.
fn gate(on: bool) -> f64 {
    if on { 1.0 } else { 0.0 }
}

/// `e[7]` of a hit (what hit), or `""` for an event without one.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// A number of the recording's ragged arrays.
fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Float(x) => Some(*x),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

/// `np.linspace(0, 2 * math.pi, n + 1)[:-1]`: `i * step` for `i` below `n`, the step `2 pi / n`.
fn circle(n: usize) -> Vec<f64> {
    let step = (2.0 * dmath::PI - 0.0) / n as f64;
    (0..n).map(|i| i as f64 * step + 0.0).collect()
}

/// `title_pattern(base)`: spotlight rays fanning down from a lamp above the panel. `base` is the panel's
/// gradient, changed in place.
pub fn title_rays(base: &mut RgbF) {
    for y in 0..base.h {
        for x in 0..base.w {
            let (xx, yy) = (x as f64, y as f64);
            // a = np.arctan2(XX - 90.0, YY + 60.0)
            let a = dmath::atan2(xx - 90.0, yy + 60.0);
            // ray = (np.floor((a + 1.2) / 0.16) % 2 == 0) & (np.abs(a) < 0.95)
            let ray = ((a + 1.2) / 0.16).floor().rem_euclid(2.0) == 0.0 && a.abs() < 0.95;
            if !ray {
                continue;
            }
            // k = np.clip(1 - (YY + 60.0) / 700.0, 0.25, 1.0)
            let k = np::clip(1.0 - (yy + 60.0) / 700.0, 0.25, 1.0);
            // base * (1 - 0.35 * k) + np.array([255, 236, 210]) * 0.35 * k
            for (p, c) in base.px_mut(x, y).iter_mut().zip([255.0, 236.0, 210.0]) {
                *p = *p * (1.0 - 0.35 * k) + c * 0.35 * k;
            }
        }
    }
}

/// One sweep, rebuilt from its `spot` event (`self.SPOTS`): what the state reads.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sweep {
    /// `e[0]`: the event's time.
    t0: f64,
    /// `e[9]`: when the light goes out.
    t1: f64,
    /// `e[11]`: 0 a sweep, 1 the octave's second light.
    kind: i64,
}

/// A row of `RUN['SPOT'][i]`: `(owner, x, z, radius, kind, start, end, hot, id)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Light {
    cx: f64,
    cz: f64,
    r: f64,
    /// 0 a sweep, 1 the octave's light, 2 ENCORE's chase light, 3 a phone light.
    kind: i64,
    t0: f64,
    t1: f64,
    hot: bool,
}

impl Light {
    /// `min(1.0, age / 0.08) * (min(1.0, max(0.0, (t1 - t) / 0.1)) if kind != 2 else 1.0)`: the light fades in
    /// over 0.08 s and (but for the chase light) out over its last 0.1 s.
    fn fade(&self, t: f64) -> f64 {
        let age = t - self.t0;
        let out = if self.kind != 2 {
            py_min(1.0, py_max(0.0, (self.t1 - t) / 0.1))
        } else {
            1.0
        };
        py_min(1.0, age / 0.08) * out
    }
}

/// `[s for s in RUN['SPOT'][i] if s[0] == k]`: side `k`'s lights at recording frame `i` (none when the recording
/// has no `SPOT`; a row that is not the nine values above is left out).
fn lights(p: &PictureCtx, i: usize, k: usize) -> Vec<Light> {
    let Some(Value::List(rows)) = p.run.nested("SPOT", i) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            let Value::List(v) = row else { return None };
            let Value::Int(owner) = *v.first()? else { return None };
            if owner != k as i64 {
                return None;
            }
            let Value::Int(kind) = *v.get(4)? else { return None };
            Some(Light {
                cx: num(v.get(1)?)?,
                cz: num(v.get(2)?)?,
                r: num(v.get(3)?)?,
                kind,
                t0: num(v.get(5)?)?,
                t1: num(v.get(6)?)?,
                // `if hot`, `if s[7]`: an int, 0 or 1
                hot: v.get(7)?.truthy(),
            })
        })
        .collect()
}

/// `pts[ConvexHull(pts).vertices]`, or `None` where scipy raises (points on one line or in one place, fewer than
/// three, a coordinate that is not finite), which the kit's `try / except` turns into `continue`. A hull the port
/// cannot order as Qhull does (a merge) is returned as the hull module's plain scan and counted there.
fn hull_of(pts: &[(f64, f64)]) -> Option<Vec<(f64, f64)>> {
    let found = hull::convex_hull(pts);
    match found.fallback {
        Some(Unusual::Flat | Unusual::NotEnoughPoints) => None,
        None | Some(Unusual::Merge) => Some(found.vertices.iter().map(|&i| pts[i]).collect()),
    }
}

/// The screen points of a light's cone: the lamp first, then 24 points round the pool.
fn cone_points(cam: &Cam, s: &Light) -> Vec<(f64, f64)> {
    let base_w = scene::to_world(s.cx, s.cz, 0.02);
    // lamp = base_w + np.array([0.0, LAMP_H, -LAMP_BACK])
    let mut world = vec![[base_w[0] + 0.0, base_w[1] + LAMP_H, base_w[2] + -LAMP_BACK]];
    // base_w + [r * math.cos(a), 0.0, r * math.sin(a)] for a in np.linspace(0, 2 * math.pi, 25)[:-1]
    world.extend(circle(24).iter().map(|&a| {
        [
            base_w[0] + s.r * dmath::cos(a),
            base_w[1] + 0.0,
            base_w[2] + s.r * dmath::sin(a),
        ]
    }));
    cam.wp(&world)
}

/// ALTO SAX's look: `Look` of `kit/fighters/alto_sax.py`.
#[derive(Clone, Debug, Default)]
pub struct AltoSaxLook {
    /// Its side.
    k: usize,
    /// `SPOTS`: its sweeps.
    spots: Vec<Sweep>,
    /// `BURNS`: `(time, x, y)` of each burn.
    burns: Vec<(f64, f64, f64)>,
    /// `ENC`: `(start, end)` of each ENCORE.
    enc: Vec<(f64, f64)>,
    /// `PHONE_BURN`: `(time, x, y)` of each phone light that burned.
    phone_burn: Vec<(f64, f64, f64)>,
}

impl AltoSaxLook {
    /// `lights_at(F_or_C)`: its lights at a frame in the fight, none outside it.
    fn lights_at(&self, p: &PictureCtx, in_fight: bool, i: usize) -> Vec<Light> {
        if !in_fight {
            return Vec::new();
        }
        lights(p, i, self.k)
    }
}

impl Look for AltoSaxLook {
    fn shards(&self) -> Vec<String> {
        // ['brass'] * 6 + ['key'] * 4 + ['pearl'] * 3
        [("brass", 6), ("key", 4), ("pearl", 3)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["phone"]
    }

    fn spark_flash(&self) -> &[&str] {
        &[]
    }

    fn flash_hits(&self) -> &[&str] {
        &[]
    }

    fn fringe_hits(&self) -> &[&str] {
        &[]
    }

    /// The burns are a sting on every 16th: no HP-bar flash.
    fn quiet_hits(&self) -> &[&str] {
        &["burn", "phone"]
    }

    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[]
    }

    fn sprite_k(&self) -> f64 {
        0.86
    }

    fn title_pattern(&self, base: &mut RgbF) {
        title_rays(base);
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.k = k;
        let mine = |kind: &'static str| {
            p.rec
                .events
                .iter()
                .filter(move |e| e.kind == kind && e.who() == Some(k))
        };
        self.spots = mine("spot")
            .map(|e| Sweep {
                t0: e.t,
                t1: e.f(9),
                kind: e.i(11),
            })
            .collect();
        self.burns = mine("burn").map(|e| (e.t, e.f(4), e.f(5))).collect();
        self.enc = mine("encore").map(|e| (e.t, e.f(3))).collect();
        self.phone_burn = mine("phone_burn").map(|e| (e.t, e.f(3), e.f(4))).collect();
    }

    // ------------------------------------------------------------ state: its bell glows while a light runs
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if kk != self.k || phase != Phase::Fight {
            return;
        }
        let tc = py_min(t, p.t_ko);
        for s in &self.spots {
            if s.t0 <= tc && tc < s.t1 {
                // max(st['glow'], 0.25 + 0.15 * (s['kind'] == 0))
                st.glow = py_max(st.glow, 0.25 + 0.15 * gate(s.kind == 0));
            }
        }
        if self.enc.iter().any(|&(a, b)| a <= tc && tc < b) {
            st.glow = py_max(st.glow, 0.5);
        }
    }

    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "encore" { Some(0.8) } else { None }
    }

    fn spark(&self, _p: &PictureCtx, e: &Event) -> Spark {
        match how(e) {
            "burn" => ([255.0, 220.0, 170.0], 4, 1.2, 0.22),
            "phone" => ([255.0, 255.0, 255.0], 6, 1.6, 0.2),
            _ => ([230.0, 190.0, 255.0], 5, 1.8, 0.2),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        if how(e) == "burn" { Some((8.0, 0.3)) } else { None }
    }

    /// The lights brighten whatever passes under them (the balls): a point light over each pool.
    fn light(&self, p: &PictureCtx, l: &mut Lights, t: f64) {
        if !(p.show.grid.t_fight <= t && t < p.t_ko) {
            return;
        }
        let mut n = 0;
        for s in lights(p, p.sim_i(t), self.k) {
            if s.kind == 3 || n >= 2 {
                continue;
            }
            // (HOT * 1.4) if s[7] else (WARM * (1.1 if s[4] != 1 else 0.9))
            let col = if s.hot {
                HOT.map(|c| c * 1.4)
            } else {
                let g = if s.kind != 1 { 1.1 } else { 0.9 };
                WARM.map(|c| c * g)
            };
            l.point(scene::to_world(s.cx, s.cz, 1.6), col, 1.4 + s.r);
            n += 1;
        }
    }

    // ------------------------------------------------------------ effects
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let k = self.k;
        let col = p.fl_col[k];
        let mut last = -9.0;
        for &(t0, x, y) in &self.burns {
            if t0 > p.t_ko {
                break;
            }
            let emit = Emit {
                n: Some(2),
                col: Some(vec![
                    [255.0, 220.0, 160.0],
                    [236.0, 140.0, 255.0],
                    [255.0, 250.0, 230.0],
                ]),
                ..Emit::default()
            };
            fx.pa.emit(t0, scene::to_world(x, y, 0.15), "embers", &emit);
            if t0 - last > 0.35 {
                // a scorch where the light caught it (not every 16th)
                fx.mk.add(
                    t0,
                    Mark {
                        size: 0.35,
                        life: 4.0,
                        col: [130.0, 60.0, 150.0],
                        // seed=int(t0 * 71) % 991
                        seed: ((t0 * 71.0) as i64).rem_euclid(991) as u64,
                        strength: 0.5,
                        ..Mark::new(MarkKind::Scorch, x, y)
                    },
                );
                last = t0;
            }
        }
        for &(t0, x, y) in &self.phone_burn {
            if t0 > p.t_ko {
                break;
            }
            let emit = Emit {
                n: Some(4),
                col: Some(vec![[255.0, 255.0, 255.0], [220.0, 230.0, 255.0]]),
                ..Emit::default()
            };
            fx.pa.emit(t0, scene::to_world(x, y, 0.1), "sparks", &emit);
        }
        for &(t0, _) in &self.enc {
            // p = P.RUN['P'][P.sim_i(t0), k]
            let at = p.run.p(p.sim_i(t0), k);
            fx.fl
                .add(t0, scene::to_world(at[0], at[1], R), col, 2.6, 0.9, 0.03, 0.4, 0.0);
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let brass = Emit {
            n: Some(18),
            col: Some(vec![BRASS, KEYG, [255.0, 240.0, 200.0]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "brass", &brass);
        let sparks = Emit {
            n: Some(20),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            col: Some(vec![[236.0, 170.0, 255.0], [255.0, 236.0, 255.0]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
    }

    // ------------------------------------------------------------ the canvas: the pools of light, the dimmed ring
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let bayer = config::bayer_full();
        let own = p.col[self.k].map(|v| v / 255.0);
        for s in self.lights_at(p, c.in_fight, c.i) {
            if s.kind == 3 {
                // a phone light: a tiny white pinprick
                for (j, light) in ic.iter_mut().enumerate() {
                    let rd = np::hypot(c.sxp[j] - s.cx, c.szp[j] - s.cz);
                    let m = np::clip(1.0 - rd / 0.13, 0.0, 1.0);
                    let b = bayer.px(c.xs[j], c.ys[j])[0];
                    // (np.floor(m * 3 + BAYER * 0.9) / 3 * 1.4)[:, None] * WHITE[None] / 255.0
                    let v = (m * 3.0 + b * 0.9).floor() / 3.0 * 1.4;
                    for ch in 0..3 {
                        light[ch] += v * WHITE[ch] / 255.0;
                    }
                }
                continue;
            }
            let fade = s.fade(c.t);
            let edge = if s.hot { 0.05 } else { 0.10 };
            let (tint, gain): (V3, f64) = if s.hot {
                (HOT, 0.75)
            } else if s.kind == 1 {
                // WARM * 0.7 + self.col / 255.0 * 0.35
                (std::array::from_fn(|ch| WARM[ch] * 0.7 + own[ch] * 0.35), 0.34)
            } else {
                (WARM, if s.kind == 0 { 0.40 } else { 0.48 })
            };
            for (j, light) in ic.iter_mut().enumerate() {
                let rd = np::hypot(c.sxp[j] - s.cx, c.szp[j] - s.cz);
                let b = bayer.px(c.xs[j], c.ys[j])[0];
                let core = np::clip((s.r - rd) / edge, 0.0, 1.0);
                let mut m = (core * 4.0 + b * 0.9).floor() / 4.0;
                if s.hot {
                    m += 0.5 * gate(rd < s.r * 0.45);
                }
                // Ic += (m * gain * fade)[:, None] * tint[None]
                let v = m * gain * fade;
                for ch in 0..3 {
                    light[ch] += v * tint[ch];
                }
                // rim = np.clip(1 - np.abs(rd - r) / 0.05, 0, 1) * fade
                let rim = np::clip(1.0 - (rd - s.r).abs() / 0.05, 0.0, 1.0) * fade;
                let w = (rim * 2.0 + b * 0.9).floor() / 2.0 * 0.35;
                for ch in 0..3 {
                    light[ch] += w * own[ch];
                }
            }
        }
    }

    /// The arena dims only in a ring around each pool (the follow-spot's contrast), never the whole canvas.
    fn canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let bayer = config::bayer_full();
        for s in self.lights_at(p, c.in_fight, c.i) {
            if s.kind == 3 {
                continue;
            }
            for (j, light) in ic.iter_mut().enumerate() {
                let rd = np::hypot(c.sxp[j] - s.cx, c.szp[j] - s.cz);
                let b = bayer.px(c.xs[j], c.ys[j])[0];
                // ring = np.clip(1 - (rd - r) / 0.9, 0, 1) * (rd > r)
                let ring = np::clip(1.0 - (rd - s.r) / 0.9, 0.0, 1.0) * gate(rd > s.r);
                let ring = (ring * 3.0 + b * 0.9).floor() / 3.0;
                let dim = 1.0 - 0.30 * ring;
                for v in light.iter_mut() {
                    *v *= dim;
                }
            }
        }
    }

    // ------------------------------------------------------------ the cones from the rafters, the phones' rays
    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        let cam = f.cam;
        let bayer = config::bayer_full();
        for s in self.lights_at(p, f.in_fight, f.i) {
            if s.kind == 3 {
                // a faint ray back to the phone in the stands
                let w = scene::to_world(s.cx, s.cz, 0.05);
                // d = np.array([w[0], 0, w[2]]);  d = d / (np.linalg.norm(d) + 1e-9)
                let d = [w[0], 0.0, w[2]];
                let n = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() + 1e-9;
                let d = d.map(|v| v / n);
                let src = [d[0] * 6.6, -0.2, d[2] * 6.6];
                let sp = cam.wp(&[w, src]);
                reeds::add(arr, &reeds::mask(&[], &[sp], 1), [255.0, 250.0, 230.0], 0.10);
                let g = cam.wp(&[w])[0];
                fr.glow(g.0, g.1, 5.0, [255.0, 255.0, 255.0], 0.6);
                continue;
            }
            let fade = s.fade(f.t);
            let pts = cone_points(cam, &s);
            // try: hull = pts[ConvexHull(pts).vertices]  except Exception: continue
            let Some(outline) = hull_of(&pts) else { continue };
            let mk = reeds::mask(&[outline], &[], 1);
            // fall = np.clip((pts[1:, 1].max() - np.arange(H)[:, None] + 40) / 420.0, 0.15, 1.0): brighter near
            // the floor
            let y_max = pts[1..].iter().map(|q| q.1).fold(f64::NEG_INFINITY, np::maximum);
            // mk = np.floor(mk * fall * 6 + BAYER_FULL * 0.9) / 6: the float32 mask times the float64 fall
            let mut lit: Img<f64> = Img::new(W, H, 1);
            for y in 0..H {
                let fall = np::clip((y_max - y as f64 + 40.0) / 420.0, 0.15, 1.0);
                for x in 0..W {
                    let m = f64::from(mk.px(x, y)[0]);
                    lit.px_mut(x, y)[0] = (m * fall * 6.0 + bayer.px(x, y)[0] * 0.9).floor() / 6.0;
                }
            }
            let col = if s.hot {
                [255.0, 255.0, 255.0]
            } else if s.kind != 1 {
                [255.0, 236.0, 200.0]
            } else {
                [255.0, 220.0, 255.0]
            };
            reeds::add(arr, &lit, col, (if s.hot { 0.20 } else { 0.15 }) * fade);
            let g = pts[0];
            if -20.0 < g.1 && g.1 < H as f64 {
                fr.glow(g.0, g.1, 9.0, [255.0, 244.0, 220.0], 0.7 * fade);
            }
        }
    }

    fn glows(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, _drawn: &[DrawnBall]) {
        for s in self.lights_at(p, f.in_fight, f.i) {
            if s.kind == 3 {
                continue;
            }
            let g = f.cam.wp(&[scene::to_world(s.cx, s.cz, 0.05)])[0];
            let col = if s.hot {
                [255.0, 255.0, 255.0]
            } else {
                [255.0, 232.0, 200.0]
            };
            fr.glow(g.0, g.1, 22.0 * s.r / 0.5, col, 0.22);
        }
    }

    // ------------------------------------------------------------ its parts (K.O. shatter)
    fn draw_shard(
        &self,
        _p: &PictureCtx,
        arr: &mut Img<f32>,
        cam: &Cam,
        kind: &str,
        pos: V3,
        rot: f64,
        size: f64,
        curl: f64,
    ) {
        let d = [dmath::cos(rot), dmath::sin(rot)];
        let nn = [-d[1], d[0]];
        let c = [pos[0], pos[2]];
        let on_screen = |pts: &[[f64; 2]]| -> Vec<(f64, f64)> {
            let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], pos[1])).collect();
            screen(cam, &world)
        };
        if kind == "pearl" {
            let sp = on_screen(&[c])[0];
            // x, y = int(sp[0]), int(sp[1])
            let (x, y) = (sp.0 as i64, sp.1 as i64);
            if 1 <= x && x < W as i64 - 1 && 1 <= y && y < H as i64 - 1 {
                // arr[y - 1:y + 1, x - 1:x + 1] = PEARL
                for yy in y - 1..y + 1 {
                    for xx in x - 1..x + 1 {
                        for (a, &v) in arr.px_mut(xx as usize, yy as usize).iter_mut().zip(&PEARL) {
                            *a = v as f32;
                        }
                    }
                }
            }
            return;
        }
        if kind == "key" {
            // c + np.array([math.cos(a), math.sin(a)]) * size * 0.9 for a in np.linspace(0, 2 * math.pi, 9)[:-1]
            let ring: Vec<[f64; 2]> = circle(8)
                .iter()
                .map(|&a| [c[0] + dmath::cos(a) * size * 0.9, c[1] + dmath::sin(a) * size * 0.9])
                .collect();
            let sp = on_screen(&ring);
            let k = 0.8 + 0.3 * dmath::cos(rot).abs();
            layer_blend(arr, std::slice::from_ref(&sp), KEYG.map(|v| v * k), 1.0, None);
            // _mask([], [np.vstack([sp, sp[:1]])], 1): the cup's outline, closed
            let mut closed = sp.clone();
            closed.push(sp[0]);
            reeds::blend(arr, &reeds::mask(&[], &[closed], 1), [90.0, 50.0, 20.0], Num::Py(0.7));
            return;
        }
        // a piece of brass tube: c - d * size * 1.8 + nn * curl * 0.04, c + d * size * 1.8 - nn * curl * 0.04
        let pts = [
            [
                c[0] - d[0] * size * 1.8 + nn[0] * curl * 0.04,
                c[1] - d[1] * size * 1.8 + nn[1] * curl * 0.04,
            ],
            [
                c[0] + d[0] * size * 1.8 - nn[0] * curl * 0.04,
                c[1] + d[1] * size * 1.8 - nn[1] * curl * 0.04,
            ],
        ];
        let sp = on_screen(&pts);
        reeds::blend(
            arr,
            &reeds::mask(&[], std::slice::from_ref(&sp), 3),
            [110.0, 70.0, 20.0],
            Num::Py(0.9),
        );
        let k = 0.8 + 0.35 * dmath::cos(rot).abs();
        reeds::blend(
            arr,
            &reeds::mask(&[], std::slice::from_ref(&sp), 2),
            BRASS.map(|v| v * k),
            Num::Py(1.0),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{Light, circle, title_rays};
    use bf_picture::img::Img;

    #[test]
    fn a_circle_is_numpys_linspace_without_its_last_value() {
        let a = circle(24);
        assert_eq!(a.len(), 24);
        assert_eq!(a[0], 0.0);
        assert_eq!(a[6], 6.0 * (2.0 * std::f64::consts::PI / 24.0));
        assert!(a[23] < 2.0 * std::f64::consts::PI);
    }

    #[test]
    fn a_light_fades_in_and_out_but_the_chase_light_only_in() {
        let mut s = Light {
            cx: 0.0,
            cz: 0.0,
            r: 0.42,
            kind: 0,
            t0: 10.0,
            t1: 11.0,
            hot: false,
        };
        assert_eq!(s.fade(10.04), (10.04f64 - 10.0) / 0.08);
        assert_eq!(s.fade(10.5), 1.0);
        assert_eq!(s.fade(10.95), (11.0 - 10.95f64) / 0.1);
        assert_eq!(s.fade(11.2), 0.0);
        s.kind = 2;
        assert_eq!(s.fade(11.2), 1.0);
    }

    #[test]
    fn the_rays_light_alternate_wedges_under_the_lamp_and_leave_the_rest() {
        let mut base: Img<f64> = Img::from_vec(360, 640, 3, vec![100.0; 360 * 640 * 3]);
        title_rays(&mut base);
        // straight under the lamp (x = 90): a = 0, floor(1.2 / 0.16) = 7, odd: no ray
        assert_eq!(base.px(90, 300), [100.0, 100.0, 100.0]);
        // far to the right at the top: |a| is 0.95 or more, no ray
        assert_eq!(base.px(359, 0), [100.0, 100.0, 100.0]);
        // a = atan2(30, 360) = 0.0831..., floor(1.2831 / 0.16) = 8, even: a ray; k = clip(1 - 360 / 700, 0.25, 1)
        let k = 1.0 - 360.0 / 700.0;
        let want = |c: f64| 100.0 * (1.0 - 0.35 * k) + c * 0.35 * k;
        assert_eq!(base.px(120, 300), [want(255.0), want(236.0), want(210.0)]);
    }
}
