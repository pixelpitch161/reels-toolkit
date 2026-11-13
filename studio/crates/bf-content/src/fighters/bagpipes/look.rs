//! BAGPIPES's look: the port of `Look` in its reference kit module (`tools/studio_export/older/fighters/bagpipes.py`,
//! #30 v3's `render.py` layers moved into the kit's hooks) and of the #30 `styles.py` draw code it calls
//! (`tartan_rgb`, `draw_trail`, `tartan_ring`), hook by hook.
//!
//! What it draws: the ball at the run's own radius (`RAD`) with the bag's tartan stretched over the glass while a
//! cycle runs (opt-in `ball_glass`), a shiver just before it lets go and its glow through the swell and the flight;
//! the flight's ribbon trail behind the ball; WAR CRY's tartan band rolling out on the canvas (`canvas_wave`, the band
//! painted over the lit canvas by opt-in `canvas_paint`) and BAG BURST's gust, with their heat shimmer; its hits'
//! sparks, impact points, shakes, flashbulbs, glows and #30's own chromatic fringes (0.1 s, in `after_grade`); the
//! effects of its hits, releases, WAR CRY and BAG BURST, the flight's puff trail and its K.O. shreds; its shards
//! (drone pipes, tartan, reeds). Its VS card panel is the card's own (`_tartan`).
//!
//! The look reads no roster number (`self.cfg`): `cfg_summary` is `None`.

use bf_core::dmath::{self, PI};
use bf_core::event::{Event, Value};
use bf_core::py::{self, fmod_py, py_max, py_min};
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config;
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DefaultLook, DrawnBall, FrameCtx, Fx, GlassPx, Look, Payload, Phase, Pic, Spark};
use bf_picture::raster::Frame;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{self, PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::np;

use crate::fighters::shared::older::{blend_into, mask_of_lines};
use crate::fighters::shared::reeds::Num;

/// `TARTAN['dk']`.
const TARTAN_DK: V3 = [26.0, 96.0, 46.0];
/// `TARTAN['lt']`.
const TARTAN_LT: V3 = [84.0, 206.0, 98.0];
/// `TARTAN['red']`.
const TARTAN_RED: V3 = [214.0, 48.0, 44.0];
/// `TARTAN['gold']`.
const TARTAN_GOLD: V3 = [246.0, 204.0, 72.0];

/// #30's hit kinds with the big spark spray (`BIG`).
const BIG: [&str; 5] = ["flight", "bodyslam", "clamp", "wave", "shockwave"];

/// The `phase` key of its picture state (`st['phase']`): `extra["phase"]` is 0 for `None`, 1 while it swells,
/// 2 while it flies.
const SWELL: f64 = 1.0;
/// See [`SWELL`].
const FLY: f64 = 2.0;

/// `e[7]`, the `how` of a `hit` event.
fn how(e: &Event) -> &str {
    match e.fields.get(5) {
        Some(Value::Str(s)) => s,
        _ => "",
    }
}

/// `int(x)` of a positive float, as a seed.
fn seed_of(x: f64) -> u64 {
    x.trunc().max(0.0) as u64
}

/// A colour times a scalar.
fn sc(a: V3, k: f64) -> V3 {
    a.map(|v| v * k)
}

/// `np.linalg.norm(v)` of a 2-vector: the root of numpy's dot product, `sqrt(fma(v1, v1, v0 * v0))` (ruling M3-R39).
fn norm2(v: [f64; 2]) -> f64 {
    dmath::fma(v[1], v[1], v[0] * v[0]).sqrt()
}

/// `tartan_rgb(dx, dy, stretch)` (#30 `styles.py` 83) at one pixel: green blocks, a red over-check, gold pin lines;
/// the sett stretches as the bag swells.
pub fn tartan_rgb(dx: f64, dy: f64, stretch: f64) -> V3 {
    let k = 2.6 / stretch;
    let (u, v) = (dx * k + 0.37, dy * k + 0.11);
    let (fu, fv) = (fmod_py(u, 1.0), fmod_py(v, 1.0));
    // blk = ((np.floor(u * 2) + np.floor(v * 2)) % 2).astype(float)
    let blk = fmod_py((u * 2.0).floor() + (v * 2.0).floor(), 2.0);
    let mut c = [0.0; 3];
    for ch in 0..3 {
        c[ch] = TARTAN_DK[ch] * (1.0 - blk) + TARTAN_LT[ch] * blk;
    }
    if (fu - 0.5).abs() < 0.07 || (fv - 0.5).abs() < 0.07 {
        c = TARTAN_RED;
    }
    if (fu - 0.12).abs() < 0.03 || (fv - 0.12).abs() < 0.03 {
        c = TARTAN_GOLD;
    }
    c
}

/// `tartan_ring(sxp, szp, bayer, cx, cz, r, width, k, t)` (#30 `styles.py` 286) at one canvas pixel: `(paint,
/// weight)` of a tartan band of radius `r` around `(cx, cz)`.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn tartan_ring(sx: f64, sz: f64, bayer: f64, cx: f64, cz: f64, r: f64, width: f64, k: f64, t: f64) -> (V3, f64) {
    let rd = np::hypot(sx - cx, sz - cz);
    let u = (rd - (r - width)) / width;
    let on = (0.0..=1.0).contains(&u);
    let ang = dmath::atan2(sz - cz, sx - cx);
    let s = fmod_py(ang * py_max(r, 0.3) / 0.24 + 0.5 * t, 1.0);
    let mut c = if s < 0.5 { TARTAN_DK } else { TARTAN_LT };
    if (s - 0.25).abs() < 0.06 || (u - 0.5).abs() < 0.10 {
        c = TARTAN_RED;
    }
    if (s - 0.75).abs() < 0.035 {
        c = TARTAN_GOLD;
    }
    // edge = np.clip(1 - np.abs(u - 0.5) * 2, 0, 1) ** 0.3
    let edge = dmath::pow(np::clip(1.0 - (u - 0.5).abs() * 2.0, 0.0, 1.0), 0.3);
    let wgt = ((edge * k * 4.0 + bayer * 0.9).floor() / 4.0) * if on { 1.0 } else { 0.0 };
    (c, np::clip(wgt, 0.0, 1.0))
}

/// `draw_trail(arr, cam, pts_ring, col, width)` (#30 `styles.py` 98): the flight's ribbon, a fading polyline through
/// the recent positions (ring frame, oldest first), one segment at a time.
pub fn draw_trail(arr: &mut Img<f32>, cam: &Cam, pts: &[[f64; 2]], col: V3, width: f64) {
    if pts.len() < 2 {
        return;
    }
    let world: Vec<V3> = pts.iter().map(|p| scene::to_world(p[0], p[1], R * 0.9)).collect();
    let sp = cam.wp(&world);
    let n = sp.len();
    for j in 0..n - 1 {
        let u = (j + 1) as f64 / n as f64;
        // width=max(1, int(round(width * u)))
        let w = (py::round_half_even(width * u) as i64).max(1) as u32;
        let m = mask_of_lines(&[vec![sp[j], sp[j + 1]]], Some(w));
        let c = col.map(|v| np::clip(v * 0.45 + 140.0 * u + 20.0, 0.0, 255.0));
        blend_into(arr, &m, c, Num::Py(0.9 * dmath::pow(u, 0.7)));
    }
}

/// A BAG INFLATE cycle as the look rebuilds it from the run (#30 render.py `_cycles`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cycle {
    t0: f64,
    s: f64,
    t_rel: f64,
    t_land: f64,
}

/// What it queues in the painter pass.
enum Item {
    /// The flight's ribbon trail.
    Trail,
}

/// BAGPIPES's look: `Look` of its reference kit module.
#[derive(Clone, Debug, Default)]
pub struct BagpipesLook {
    /// Its side.
    k: usize,
    /// `RUN['RAD']`: both balls' radius per recording frame.
    rad: Vec<[f64; 2]>,
    /// Its cycles.
    cycles: Vec<Cycle>,
    /// `cyc_t`: when each cycle started.
    cyc_t: Vec<f64>,
}

impl BagpipesLook {
    /// `cycle_at(t)`: the cycle running at `t` and whether it swells or flies.
    fn cycle_at(&self, t: f64) -> Option<(Cycle, f64)> {
        // j = np.searchsorted(self.cyc_t, t, 'right') - 1
        let j = self.cyc_t.partition_point(|&v| v <= t).checked_sub(1)?;
        let c = self.cycles[j];
        if t < c.t_rel {
            Some((c, SWELL))
        } else if t < c.t_land {
            Some((c, FLY))
        } else {
            None
        }
    }

    /// `float(self.rad[i, kk])`.
    fn rad(&self, i: usize, kk: usize) -> f64 {
        self.rad.get(i).map_or(R, |r| r[kk])
    }
}

impl Look for BagpipesLook {
    fn shards(&self) -> Vec<String> {
        // ('pipe',) * 5 + ('tartan',) * 7 + ('reed',) * 2
        [("pipe", 5), ("tartan", 7), ("reed", 2)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_flash(&self) -> &[&str] {
        &["flight", "bodyslam"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["flight", "bodyslam", "wave", "shockwave"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &[]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`: the run's radii and its cycles (it reads no roster number).
    fn setup_as(&mut self, p: &PictureCtx, k: usize, _cfg: &FighterCfg) {
        self.k = k;
        let num = |x: &Value| match x {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f64,
            _ => R,
        };
        self.rad = (0..p.run.nsim)
            .map(|i| match p.run.nested("RAD", i) {
                Some(Value::List(v)) if v.len() == 2 => [num(&v[0]), num(&v[1])],
                _ => [R, R],
            })
            .collect();
        let mut out: Vec<(f64, f64, Option<f64>, Option<f64>)> = Vec::new();
        for e in &p.rec.events {
            if e.who() != Some(k) {
                continue;
            }
            match e.kind.as_str() {
                "inflate" => out.push((e.t, e.f(3), None, None)),
                "release" => {
                    if let Some(c) = out.last_mut() {
                        c.2 = Some(e.t);
                    }
                }
                "land" => {
                    if let Some(c) = out.last_mut() {
                        c.3 = Some(e.t);
                    }
                }
                _ => {}
            }
        }
        // a cycle the K.O. interrupted ends there
        self.cycles = out
            .into_iter()
            .map(|(t0, s, r, l)| Cycle {
                t0,
                s,
                t_rel: r.unwrap_or(p.t_ko + 10.0),
                t_land: l.unwrap_or(p.t_ko + 10.0),
            })
            .collect();
        self.cyc_t = self.cycles.iter().map(|c| c.t0).collect();
    }

    // ------------------------------------------------------------ the ball
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if kk != self.k || phase != Phase::Fight {
            return;
        }
        let i = p.sim_i(py_min(t, p.t_ko));
        let tc = py_min(t, p.t_ko);
        st.scale = self.rad(i, kk) / R;
        let Some((c, ph)) = self.cycle_at(tc) else {
            st.extra.insert("phase".into(), 0.0);
            return;
        };
        st.extra.insert("phase".into(), ph);
        st.extra.insert("infl".into(), (st.scale - 1.0) / py_max(1e-6, c.s - 1.0));
        if ph == SWELL {
            // a shiver just before it lets go
            let u = (tc - c.t0) / py_max(1e-6, c.t_rel - c.t0);
            if u > 0.8 {
                let w = 0.025 * dmath::sin(tc * 2.0 * PI * 31.0);
                st.p = [st.p[0] + w * 1.0, st.p[1] + w * -0.6];
            }
            st.glow = 0.15 + 0.25 * u;
        } else {
            st.glow = 0.55 * dmath::exp(-(tc - c.t_rel) / 0.25) + 0.15;
        }
    }

    /// (opt-in) BAG INFLATE: the bag's tartan stretched over the glass as the ball swells.
    fn ball_glass(&self, _p: &PictureCtx, kk: usize, st: &BallState, _t: f64, px: &GlassPx, inner: &mut V3) {
        if kk != self.k {
            return;
        }
        let on = st.extra.get("phase").is_some_and(|&ph| ph != 0.0);
        let infl = if on { st.extra.get("infl").copied().unwrap_or(0.0) } else { 0.0 };
        if infl > 0.02 {
            let tart = sc(tartan_rgb(px.dx, px.dy, 1.0 + 0.5 * infl), 0.62 + 0.42 * px.band);
            let w_t = np::clip(infl * 1.15, 0.0, 0.92) * (0.75 + 0.25 * px.nz);
            for ch in 0..3 {
                inner[ch] = inner[ch] * (1.0 - w_t) + tart[ch] * w_t;
            }
        }
    }

    // ------------------------------------------------------------ events -> camera, sparks, glows
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "hit" && e.who() == Some(self.k) {
            let a = match how(e) {
                "bite" => Some(0.7),
                "flight" => Some(1.3),
                "bodyslam" => Some(1.6),
                "clamp" => Some(2.5),
                "wave" => Some(2.2),
                "shockwave" => Some(1.4),
                _ => None,
            };
            if a.is_some() {
                return a;
            }
        }
        DefaultLook.shake(p, e)
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let h = how(e);
        let (n, sp, life) = if BIG.contains(&h) {
            (16, 3.2, 0.3)
        } else if h == "bite" {
            (8, 2.2, 0.22)
        } else {
            (3, 1.4, 0.16)
        };
        let col = p.col[ring::owner(e).unwrap_or(self.k)];
        (col.map(|c| c * 0.6 + 100.0), n, sp, life)
    }

    /// #30: halved (the bursts carry the impacts).
    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        let h = how(e);
        if BIG.contains(&h) || h == "bite" {
            let big = h != "bite";
            Some((if big { 12.0 } else { 7.0 }, if big { 0.35 } else { 0.22 }))
        } else {
            None
        }
    }

    // ------------------------------------------------------------ effects (reelkit.fx), as #30 built them
    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.who() != Some(self.k) {
            return;
        }
        let t0 = e.t;
        let k = self.k;
        match e.kind.as_str() {
            "hit" => {
                let (x, z) = (e.f(5), e.f(6));
                let pos = scene::to_world(x, z, R);
                let c = p.col[k];
                match how(e) {
                    h @ ("flight" | "bodyslam") => {
                        fx.bu.add(t0, pos, c, if h == "flight" { 1.3 } else { 1.5 }, None, true);
                        fx.fl.add(t0, scene::to_world(x, z, 1.0), c, 1.8, 0.9, 0.03, 0.28, 0.0);
                        let puffs = Emit {
                            n: Some(10),
                            speed: Some((1.4, 2.8)),
                            ..Emit::default()
                        };
                        fx.pa.emit(t0, pos, "puffs", &puffs);
                        let shreds = Emit {
                            n: Some(if h == "flight" { 4 } else { 6 }),
                            speed: Some((1.0, 2.6)),
                            ..Emit::default()
                        };
                        fx.pa.emit(t0, pos, "shreds", &shreds);
                        fx.mk.add(
                            t0,
                            Mark {
                                size: if h == "flight" { 0.45 } else { 0.65 },
                                n: 4,
                                life: 4.0,
                                seed: seed_of(t0 * 100.0),
                                ..Mark::new(MarkKind::Crack, x, z)
                            },
                        );
                    }
                    "wave" => {
                        fx.bu.add(t0, pos, c, 1.6, None, true);
                        fx.fl.add(t0, scene::to_world(x, z, 1.0), c, 2.0, 1.0, 0.03, 0.35, 0.0);
                    }
                    "shockwave" => {
                        fx.bu.add(t0, pos, c, 1.0, None, true);
                        fx.fl.add(t0, scene::to_world(x, z, 1.0), sc(c, 0.9), 1.6, 0.7, 0.03, 0.3, 0.0);
                    }
                    _ => {}
                }
            }
            "release" => {
                // the bag lets go: a blast of air out of the nozzle
                let puffs = Emit {
                    n: Some(10),
                    spread: Some(60.0),
                    speed: Some((1.6, 3.0)),
                    ..Emit::default()
                };
                fx.pa.emit(t0, scene::to_world(e.f(5), e.f(6), R), "puffs", &puffs);
                let at = scene::to_world(e.f(5), e.f(6), 1.0);
                fx.fl.add(t0, at, sc(p.col[k], 0.9), 1.6, 0.6, 0.03, 0.25, 0.0);
            }
            "ability" => {
                // i = int(min(NSIM - 1, max(0, round((t0 - T.T_FIGHT) * FPS))))
                let q = p.run.p(p.sim_i(t0), k);
                if e.s(3) == "BAG BURST" {
                    // the bag pops: a tartan-shred explosion
                    let shreds = Emit {
                        n: Some(48),
                        spread: Some(180.0),
                        speed: Some((2.0, 4.8)),
                        scale: 1.2,
                        ..Emit::default()
                    };
                    fx.pa.emit(t0, scene::to_world(q[0], q[1], R), "shreds", &shreds);
                    let puffs = Emit {
                        n: Some(16),
                        spread: Some(180.0),
                        speed: Some((1.5, 3.0)),
                        scale: 1.4,
                        ..Emit::default()
                    };
                    fx.pa.emit(t0, scene::to_world(q[0], q[1], R), "puffs", &puffs);
                    fx.fl.add(t0, scene::to_world(q[0], q[1], 1.2), p.col[k], 2.6, 1.1, 0.03, 0.5, 0.0);
                    fx.mk.add(
                        t0,
                        Mark {
                            size: 1.6,
                            life: 5.0,
                            seed: 47,
                            strength: 0.7,
                            ..Mark::new(MarkKind::SmokeRing, q[0], q[1])
                        },
                    );
                } else if e.s(4) == "shockwave" {
                    fx.fl.add(t0, scene::to_world(q[0], q[1], 1.0), p.col[k], 2.2, 0.8, 0.03, 0.4, 0.0);
                    let shreds = Emit {
                        n: Some(10),
                        spread: Some(180.0),
                        speed: Some((1.5, 3.0)),
                        ..Emit::default()
                    };
                    fx.pa.emit(t0, scene::to_world(q[0], q[1], R), "shreds", &shreds);
                }
            }
            _ => {}
        }
    }

    /// The flight's puff trail: one puff every other frame (after the event pass, as #30).
    fn fx_after(&self, p: &PictureCtx, fx: &mut Fx) {
        let k = self.k;
        for c in &self.cycles {
            if c.t_land > p.t_ko + 1.0 {
                continue;
            }
            let (j0, j1) = (p.sim_i(c.t_rel), p.sim_i(c.t_land));
            for j in (j0..j1).step_by(2) {
                let q = p.run.p(j, k);
                let v = p.run.row("V", j, k).map_or([0.0, 0.0], |r| [r[0], r[1]]);
                let tj = p.show.grid.t_fight + j as f64 / config::FPS as f64;
                // back = p - v / (np.linalg.norm(v) + 1e-9) * float(RUN['RAD'][j, k]) * 0.9
                let nv = norm2(v) + 1e-9;
                let rad = self.rad(j, k);
                let back = [q[0] - v[0] / nv * rad * 0.9, q[1] - v[1] / nv * rad * 0.9];
                // vel=S.to_world(*(-0.4 * v), 0.0) - S.to_world(0, 0, 0)
                let a = scene::to_world(-0.4 * v[0], -0.4 * v[1], 0.0);
                let b = scene::to_world(0.0, 0.0, 0.0);
                let puff = Emit {
                    n: Some(1),
                    vel: [a[0] - b[0], a[1] - b[1], a[2] - b[2]],
                    ..Emit::default()
                };
                fx.pa.emit(tj, scene::to_world(back[0], back[1], R * 0.8), "puffs", &puff);
            }
        }
    }

    /// The bag bursts at its K.O.: tartan shreds.
    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let shreds = Emit {
            n: Some(36),
            spread: Some(180.0),
            speed: Some((1.5, 4.0)),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "shreds", &shreds);
    }

    // ------------------------------------------------------------ the canvas: WAR CRY's band, BAG BURST's gust
    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        let (cx, cz, r, rmax) = (w.cx, w.cz, w.r, w.rmax);
        let k_ = 1.0 - r / rmax;
        if w.kind == "shockwave" {
            // WAR CRY: a tartan ring rolling out (its paint is `canvas_paint`'s)
            let tint = p.col[w.owner].map(|v| v / 255.0 + 0.15);
            for (j, light) in ic.iter_mut().enumerate() {
                let rd = np::hypot(c.sxp[j] - cx, c.szp[j] - cz);
                let band = np::clip(1.0 - (rd - r).abs() / 0.3, 0.0, 1.0);
                let v = band * k_ * 0.5;
                for ch in 0..3 {
                    light[ch] += v * tint[ch];
                }
            }
            return;
        }
        // BAG BURST: a thick gust with puffs of air
        let bayer = config::bayer_full();
        let tint = p.col[w.owner].map(|v| v / 255.0 * 1.4 + 0.35);
        for (j, light) in ic.iter_mut().enumerate() {
            let (dx, dz) = (c.sxp[j] - cx, c.szp[j] - cz);
            let ang = dmath::atan2(dz, dx);
            let rd = np::hypot(dx, dz)
                + 0.08 * dmath::sin(5.0 * ang + 9.0 * c.t)
                + 0.05 * dmath::sin(13.0 * ang - 17.0 * c.t);
            let band = np::clip(1.0 - (rd - r).abs() / 0.36, 0.0, 1.0);
            let b = bayer.px(c.xs[j], c.ys[j])[0];
            let band = (band * 3.0 + b * 0.9).floor() / 3.0;
            let v = band * k_ * 1.2;
            for ch in 0..3 {
                light[ch] += v * tint[ch];
            }
        }
    }

    /// (opt-in) WAR CRY's tartan bands, lit by the spot like the canvas: one per WAR CRY ring of its in the frame's
    /// row of `WAVES`, in the row's order (the order `canvas_wave` met them in).
    fn canvas_paint(&self, p: &PictureCtx, col: &mut [V3], ic: &[V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let bayer = config::bayer_full();
        for w in p.run.waves(c.i).iter().filter(|w| w.owner == self.k && w.kind == "shockwave") {
            let k_ = 1.0 - w.r / w.rmax;
            for (j, out) in col.iter_mut().enumerate() {
                let b = bayer.px(c.xs[j], c.ys[j])[0];
                let (paint, dw) = tartan_ring(c.sxp[j], c.szp[j], b, w.cx, w.cz, w.r, 0.26, 0.95 * k_ + 0.05, c.t);
                // lit = np.clip(Ic.mean(1), 0.45, 1.6)[:, None] * 0.82
                let lit = np::clip(np::mean(&ic[j]), 0.45, 1.6) * 0.82;
                for ch in 0..3 {
                    out[ch] = out[ch] * (1.0 - dw) + paint[ch] * lit * dw;
                }
            }
        }
    }

    // ------------------------------------------------------------ the painter: the flight's ribbon trail
    fn items_ball(&self, _p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        if st.extra.get("phase") == Some(&FLY) && !st.shattered {
            let dz = f.cam.depth(&[scene::to_world(st.p[0], st.p[1], R)])[0];
            let trail: Payload = Box::new(Item::Trail);
            return vec![(dz + 0.05, trail)];
        }
        Vec::new()
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        if let Some(Item::Trail) = payload.downcast_ref::<Item>() {
            let tc = py_min(f.t, p.t_ko);
            let j1 = p.sim_i(tc) as i64;
            // j0 = max(sim_i(c_['t_rel']) if c_ else j1 - 12, j1 - 14)
            let first = match self.cycle_at(tc) {
                Some((c, _)) => p.sim_i(c.t_rel) as i64,
                None => j1 - 12,
            };
            let j0 = first.max(j1 - 14);
            let pts: Vec<[f64; 2]> = (j0..=j1).map(|j| p.run.p(j.max(0) as usize, self.k)).collect();
            let (mut arr, depth) = (fr.array(), fr.depth());
            draw_trail(&mut arr, f.cam, &pts, p.col[self.k], 7.0);
            fr.set_array(&arr, Some(&depth));
        }
    }

    /// #30's shards: a drone pipe (black wood, an ivory ferrule), a cane reed, a chip of tartan bag cloth.
    fn draw_shard(
        &self,
        p: &PictureCtx,
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
        let on_screen = |pts: &[[f64; 2]]| -> Vec<(f64, f64)> {
            let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], pos[1])).collect();
            screen(cam, &world)
        };
        // np.array([sx_, sz_]) + d * Lk / 2 * a1 + nn * w * a2
        let quad = |l: f64, w: f64, corners: [(f64, f64); 4]| -> Vec<[f64; 2]> {
            corners
                .iter()
                .map(|&(a1, a2)| {
                    [
                        sx + d[0] * l / 2.0 * a1 + nn[0] * w * a2,
                        sz + d[1] * l / 2.0 * a1 + nn[1] * w * a2,
                    ]
                })
                .collect()
        };
        match kind {
            "pipe" | "reed" => {
                let (l, wk, c) = if kind == "pipe" {
                    (0.48, 0.07, [34.0, 28.0, 30.0])
                } else {
                    (0.22, 0.03, [222.0, 196.0, 120.0])
                };
                // nn * Wk / 2 * a2
                let q4: Vec<[f64; 2]> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                    .iter()
                    .map(|&(a1, a2)| {
                        [
                            sx + d[0] * l / 2.0 * a1 + nn[0] * wk / 2.0 * a2,
                            sz + d[1] * l / 2.0 * a1 + nn[1] * wk / 2.0 * a2,
                        ]
                    })
                    .collect();
                layer_blend(arr, &[on_screen(&q4)], c, 1.0, None);
                if kind == "pipe" {
                    // the ivory ferrule at one end
                    let q2 = quad(l, wk * 0.7, [(0.75, -1.0), (1.0, -1.0), (1.0, 1.0), (0.75, 1.0)]);
                    layer_blend(arr, &[on_screen(&q2)], [240.0, 232.0, 214.0], 1.0, None);
                }
            }
            "tartan" => {
                // tumbling flat chips of the bag cloth
                let col = if (size * 100.0) as i64 % 3 != 0 {
                    sc(p.col[p.lose], 0.8)
                } else {
                    [196.0, 40.0, 44.0]
                };
                let s = size;
                let tri = [
                    [sx + d[0] * s, sz + d[1] * s],
                    [sx - d[0] * s * 0.7 + nn[0] * s * 0.8, sz - d[1] * s * 0.7 + nn[1] * s * 0.8],
                    [sx - d[0] * s * 0.4 - nn[0] * s * 0.7, sz - d[1] * s * 0.4 - nn[1] * s * 0.7],
                ];
                let shade_k = 0.75 + 0.25 * dmath::cos(rot * 2.0);
                let col = col.map(|c| np::clip(c * shade_k, 0.0, 255.0));
                layer_blend(arr, &[on_screen(&tri)], col, 1.0, None);
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ lens, post
    /// Heat shimmer on WAR CRY / BAG BURST fronts.
    fn shimmer(&self, _p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, w: &Wave) {
        let wave = w.kind == "wave";
        let c = scene::to_world(w.cx, w.cz, 0.05);
        img.shimmer_world(
            cam,
            c,
            w.r,
            if wave { 0.4 } else { 0.25 },
            if wave { 3.0 } else { 2.0 },
            t,
            1.0 - w.r / w.rmax,
            w.owner as f64,
        );
    }

    /// Chromatic fringing on its big hits (local), as #30: 0.1 s, (30, 2.6) on BAG BURST, (20, 1.8) on a flight or
    /// a body slam (before the K.O.'s fringe, the ability moments and the bursts, as #30's renderer).
    fn after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, _drawn: &[DrawnBall]) {
        let t = f.t;
        for i in p.events_between(t - 0.1, t, &["hit"]) {
            let e = &p.rec.events[i];
            let h = how(e);
            if e.who() == Some(self.k) && matches!(h, "flight" | "bodyslam" | "clamp" | "wave") {
                let s = f.cam.wp(&[scene::to_world(e.f(5), e.f(6), R)])[0];
                let big = matches!(h, "clamp" | "wave");
                img.fringe(
                    s.0,
                    s.1,
                    if big { 30.0 } else { 20.0 },
                    if big { 2.6 } else { 1.8 },
                    1.0 - (t - e.t) / 0.1,
                );
            }
        }
    }
}
