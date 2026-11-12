//! CONTRABASS's look: the port of `Look` in `projects/ballfight/kit/fighters/contrabass.py`, hook by hook.
//!
//! What it draws: its own double-bass model with a wider crop (the padded sprite, in the ball and on the VS card);
//! each heavy step lifts the ball a little and plants it with a dust puff, a footprint, a flare and a steel-blue quake
//! ring across the canvas (inked and jagged over the ring); SLAP BASS snaps a white string across its glass and
//! fires a hard white-blue ring; THE ELEPHANT grows grey ears that flap and a trunk that swings ahead, with dust
//! plumes, cracks and a heat shimmer at every stomp; it shatters into its wood, the scroll, the endpin and strings;
//! its VS card panel is rows of heavy footprints.
//!
//! The arithmetic keeps Python's order and dtypes; the sines, cosines, arctangents and powers are `bf_core::dmath`'s
//! where Python has the C runtime's and numpy's.

use bf_core::dmath::{self, PI};
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::rng::Generator;
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::BAYER;
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::{Img, RgbF};
use bf_picture::look::{CanvasCtx, FrameCtx, Fx, Look, Payload, Phase, Pic, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::sprite::Mesh;
use bf_picture::title::layout;

use std::sync::OnceLock;

use super::state::{self, Land, Step};
use crate::fighters::shared::padded::{PAD, padded_sprite};
use crate::fighters::shared::reeds::{self, INK, Num, WHITE};

/// `DUST`.
const DUST: [V3; 3] = [[206.0, 196.0, 180.0], [170.0, 160.0, 148.0], [232.0, 224.0, 210.0]];
/// `EAR`: elephant grey.
const EAR: V3 = [112.0, 108.0, 118.0];
/// `EAR_IN`.
const EAR_IN: V3 = [214.0, 150.0, 160.0];
/// `WOOD`.
const WOOD: V3 = [138.0, 60.0, 30.0];
/// `STEEL`.
const STEEL: V3 = [196.0, 200.0, 212.0];
/// The CONTRABASS's view in the kit's roster (yaw centre, yaw swing, roll), for a look that was never set up (the
/// VS card's close-up is asked of one).
const VIEW: (f64, f64, f64) = (16.0, 14.0, 14.0);

/// The CONTRABASS's model (`SP._geo(self.cfg['pp'])`), read once.
fn mesh() -> &'static Mesh {
    static MODEL: OnceLock<Mesh> = OnceLock::new();
    MODEL.get_or_init(|| {
        let json = super::MESH.expect("the CONTRABASS's model is compiled in");
        Mesh::from_json(json).expect("the CONTRABASS's model reads")
    })
}

/// `np.array([math.cos(a), math.sin(a)])`.
fn unit(a: f64) -> [f64; 2] {
    [dmath::cos(a), dmath::sin(a)]
}

/// `BAYER_FULL[y, x]`.
fn bayer(x: usize, y: usize) -> f64 {
    BAYER[y % 4][x % 4]
}

/// `np.linspace(a, b, n)`: `i * step + a`, the last value `b` itself.
fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    let step = (b - a) / (n - 1) as f64;
    (0..n)
        .map(|i| if i == n - 1 { b } else { i as f64 * step + a })
        .collect()
}

/// `e[7]` of a hit (what hit), or `""` for an event without one.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// `jag_ring(cx, cz, r, seed, y=0.05, n=56, amp=0.05)`: a jagged ring of `n + 1` ring-frame points `(x, z, y)`,
/// closed, its radius jittered by `default_rng(seed).uniform(-1, 1, n + 1)`.
pub fn jag_ring(cx: f64, cz: f64, r: f64, seed: u64, y: f64, n: usize, amp: f64) -> Vec<V3> {
    let a = linspace(0.0, 2.0 * PI, n + 1);
    let mut rng = Generator::new(seed);
    let mut jag: Vec<f64> = (0..=n).map(|_| r + amp * rng.uniform(-1.0, 1.0)).collect();
    jag[n] = jag[0];
    a.iter()
        .zip(&jag)
        .map(|(&a, &j)| [cx + j * dmath::cos(a), cz + j * dmath::sin(a), y])
        .collect()
}

/// `title_pattern(base)`: the VS card's panel: rows of heavy round footprints (an oval pad and three toes) marching
/// along the divider. `base` is the panel's gradient, changed in place.
pub fn title_pattern(base: &mut RgbF) {
    let dd = layout::dd();
    let side = layout::side();
    let da = layout::DA;
    // np.array([20, 26, 46]) * 0.7
    let ink = [20.0 * 0.7, 26.0 * 0.7, 46.0 * 0.7];
    for y in 0..base.h {
        for x in 0..base.w {
            let u = (x as f64 - da[0]) * dd[0] + (y as f64 - da[1]) * dd[1];
            let v = side.px(x, y)[0];
            let row = (v.abs() / 64.0).floor();
            let cu = py::fmod_py(u + 40.0 * py::fmod_py(row, 2.0), 80.0) - 40.0;
            let cv = py::fmod_py(v.abs(), 64.0) - 32.0;
            // cu ** 2 / 15.0 ** 2 + cv ** 2 / 11.0 ** 2 < 1
            let pad = cu * cu / 225.0 + cv * cv / 121.0 < 1.0;
            let toes = [-9.0, 0.0, 9.0].iter().any(|&dx: &f64| {
                let a = cu - dx;
                let b = cv + 15.0;
                a * a + b * b < 14.0
            });
            if pad || toes {
                for (ch, px) in base.px_mut(x, y).iter_mut().enumerate() {
                    *px = *px * 0.3 + ink[ch];
                }
            }
        }
    }
}

/// What the painter pass draws for it during THE ELEPHANT: the ears behind the ball, the trunk in front.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Item {
    Ears,
    Trunk,
}

/// CONTRABASS's look: `Look` of `kit/fighters/contrabass.py`.
#[derive(Debug)]
pub struct ContrabassLook {
    /// Its side.
    k: usize,
    /// `T.T_FIGHT`.
    t_fight: f64,
    /// `T.BEAT`.
    beat: f64,
    /// `STEPS`.
    steps: Vec<Step>,
    /// `STEP_T`: each step's start.
    step_t: Vec<f64>,
    /// `LAND`.
    land: Vec<Land>,
    /// `_LT`: each footfall's time.
    land_t: Vec<f64>,
    /// `SLAPS`: `(t, broke)`.
    slaps: Vec<(f64, bool)>,
    /// `STAMP`: `(t0, t1)`.
    stamp: Vec<(f64, f64)>,
    /// `STOMPS`: `(t, x, y, rmax)`.
    stomps: Vec<(f64, f64, f64, f64)>,
    /// `TRAMPLE`: `(t, x, y)`.
    trample: Vec<(f64, f64, f64)>,
    /// `self.cfg['pp']`: the model's name.
    pp: String,
    /// `self.cfg['view']`: `(yaw centre, yaw swing, roll)`, `None` for an entry without one.
    view: Option<(f64, f64, f64)>,
}

impl ContrabassLook {
    /// The look before a picture sets it up: its constants and its title-card hooks work (with the roster's
    /// view), and it has no steps.
    pub fn new() -> ContrabassLook {
        ContrabassLook {
            k: 0,
            t_fight: 0.0,
            beat: 0.5,
            steps: Vec::new(),
            step_t: Vec::new(),
            land: Vec::new(),
            land_t: Vec::new(),
            slaps: Vec::new(),
            stamp: Vec::new(),
            stomps: Vec::new(),
            trample: Vec::new(),
            pp: String::new(),
            view: Some(VIEW),
        }
    }

    /// `ta(t)`: the time, held at the K.O.
    fn ta(&self, p: &PictureCtx, t: f64) -> f64 {
        py_min(t, p.t_ko)
    }

    /// `step_at(t)`: the step under way at `t` (during the fight) and how far along it is, 0 to 1.
    fn step_at(&self, p: &PictureCtx, t: f64) -> Option<(&Step, f64)> {
        if self.step_t.is_empty() || !(self.t_fight <= t && t < p.t_ko) {
            return None;
        }
        let j = state::searchsorted_right(&self.step_t, t) as i64 - 1;
        if j >= 0 && t < self.steps[j as usize].t1 {
            let s = &self.steps[j as usize];
            return Some((s, (t - s.t0) / py_max(1e-6, s.t1 - s.t0)));
        }
        None
    }

    /// `stamp_at(t)`: THE ELEPHANT under way at `t` (held at the K.O.), as `(t0, t1)`.
    fn stamp_at(&self, p: &PictureCtx, t: f64) -> Option<(f64, f64)> {
        let tc = self.ta(p, t);
        self.stamp.iter().copied().find(|&(t0, t1)| t0 <= tc && tc < t1)
    }

    /// `(yc, roll)` of `self.cfg['view']`.
    ///
    /// # Panics
    /// When the roster entry it was set up with has no view (Python raises `KeyError`); the picture refuses such
    /// an entry before any look is made.
    fn yaw_roll(&self) -> (f64, f64) {
        let (yc, _sw, roll) = self
            .view
            .expect("a roster entry the picture accepted has a view (yaw, swing, roll)");
        (yc, roll)
    }
}

impl Default for ContrabassLook {
    fn default() -> ContrabassLook {
        ContrabassLook::new()
    }
}

impl Look for ContrabassLook {
    fn shards(&self) -> Vec<String> {
        // ['body'] * 6 + ['scroll'] * 1 + ['endpin'] * 2 + ['string'] * 4
        [("body", 6), ("scroll", 1), ("endpin", 2), ("string", 4)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["slap", "trample", "stampede"]
    }

    fn spark_flash(&self) -> &[&str] {
        &["slap"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["slap", "trample"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["slap", "trample"]
    }

    fn quiet_hits(&self) -> &[&str] {
        &[]
    }

    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[("slap", 0.7)]
    }

    fn sprite_k(&self) -> f64 {
        0.84
    }

    fn title_pattern(&self, base: &mut RgbF) {
        title_pattern(base);
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// What `setup_as` took from `cfg`, by the keys `contrabass.py` reads from `self.cfg`.
    fn cfg_summary(&self) -> Option<String> {
        Some(format!("pp={}; view={:?}", self.pp, self.view))
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`: the model's name and the view come from `cfg`, the events
    /// from side `k`. For its own side `cfg` is `p.cfg[k]`, and this is `setup`. (The model drawn is the
    /// CONTRABASS's: the kit only ever makes this look with the CONTRABASS's own entry, which names it.)
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.t_fight = p.show.grid.t_fight;
        self.beat = p.show.grid.beat;
        let ev = &p.rec.events;
        self.steps = state::steps(ev, k);
        self.step_t = self.steps.iter().map(|s| s.t0).collect();
        self.land = state::lands(ev, k);
        self.land_t = self.land.iter().map(|l| l.t).collect();
        self.slaps = state::slaps(ev, k);
        self.stamp = state::stamps(ev, k);
        self.stomps = state::stomps(ev, k);
        self.trample = state::tramples(ev, k);
        self.pp = cfg.pp.clone();
        let num = |i: usize| cfg.raw.get("view").and_then(|v| v.get(i)).and_then(|x| x.as_f64());
        self.view = match (num(0), num(1), num(2)) {
            (Some(yc), Some(sw), Some(roll)) => Some((yc, sw, roll)),
            _ => None,
        };
    }

    fn sprite(&self, r: u32, yaw: f64, roll: f64) -> Option<Img<u8>> {
        Some(padded_sprite(mesh(), r, yaw, roll, PAD))
    }

    fn title_closeup(&self, r: u32) -> Option<Img<u8>> {
        let (yc, roll) = self.yaw_roll();
        Some(padded_sprite(mesh(), r, yc, roll, PAD))
    }

    // ------------------------------------------------------------ state
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if kk != self.k || phase != Phase::Fight {
            return;
        }
        if let Some((s, u)) = self.step_at(p, t) {
            // a heavy step: a small lift, set down hard
            st.lift = 0.10 * dmath::sin(PI * u) * if s.kind != 0 { 0.7 } else { 1.0 };
        }
        let tc = self.ta(p, t);
        // self.LAND[max(0, self._land_i(tc) - 1):self._land_i(tc) + 1]
        let i = state::searchsorted_right(&self.land_t, tc);
        let lo = i.saturating_sub(1);
        let hi = (i + 1).min(self.land.len());
        for l in self.land.get(lo..hi).unwrap_or(&[]) {
            if 0.0 <= tc - l.t && tc - l.t < 0.12 {
                st.glow = py_max(st.glow, 0.35 * (1.0 - (tc - l.t) / 0.12));
            }
        }
        if let Some((t0, _t1)) = self.stamp_at(p, t) {
            st.glow = py_max(st.glow, 0.3);
            st.lift = py_max(st.lift, 0.05 * dmath::sin(2.0 * PI * (tc - t0) / self.beat).abs());
        }
    }

    // ------------------------------------------------------------ events -> camera, ropes, sparks
    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match e.kind.as_str() {
            "step_land" => Some(if e.f(6) == 0.0 { 0.35 } else { 0.25 }),
            "stampede_stomp" => Some(0.7),
            "slap" => Some(if e.f(3) != 0.0 { 1.2 } else { 0.9 }),
            "trample" => Some(1.8),
            _ => None,
        }
    }

    fn rope(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match e.kind.as_str() {
            // self.rope_kinds.get(e[4])
            "ability" => {
                let Some(Value::Str(kind)) = e.fields.get(2) else {
                    return None;
                };
                self.rope_kinds().iter().find(|(name, _)| name == kind).map(|&(_, s)| s)
            }
            "stampede_stomp" => Some(0.3),
            _ => None,
        }
    }

    fn spark(&self, _p: &PictureCtx, e: &Event) -> Spark {
        match how(e) {
            "slap" => ([210.0, 230.0, 255.0], 16, 3.4, 0.3),
            "trample" | "stampede" => ([200.0, 190.0, 176.0], 14, 2.8, 0.34),
            _ => ([170.0, 190.0, 230.0], 6, 1.8, 0.2),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "slap" | "trample" => Some((15.0, 0.4)),
            "step" => Some((9.0, 0.28)),
            _ => None,
        }
    }

    // ------------------------------------------------------------ effects
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let k = self.k;
        let col = p.fl_col[k];
        // seed=int(t0 * k) % m
        let seed = |t0: f64, k: f64, m: i64| py::imod_py(np::f64_to_i64(t0 * k), m) as u64;
        for l in &self.land {
            if l.t > p.t_ko {
                break;
            }
            let pos = up(l.x, l.y, 0.08);
            let emit = Emit {
                n: Some(if l.kind == 0 { 5 } else { 4 }),
                col: Some(DUST.to_vec()),
                ..Emit::default()
            };
            fx.pa.emit(l.t, pos, "dust", &emit);
            fx.mk.add(
                l.t,
                Mark {
                    size: 0.32,
                    life: 3.0,
                    seed: seed(l.t, 97.0, 991),
                    strength: 0.45,
                    ..Mark::new(MarkKind::Dust, l.x, l.y)
                },
            );
            fx.fl.add(l.t, up(l.x, l.y, R * 0.5), col, 1.0 + 0.4 * l.reach, 0.3, 0.03, 0.14, 0.0);
        }
        for &(t0, broke) in &self.slaps {
            if t0 > p.t_ko {
                break;
            }
            let at = p.run.p(p.sim_i(t0), k);
            let pos = up(at[0], at[1], R);
            fx.bu.add(t0, pos, [220.0, 236.0, 255.0], if broke { 1.3 } else { 1.0 }, None, true);
            fx.fl.add(t0, pos, col, 2.0, 0.8, 0.03, 0.22, 0.0);
            let emit = Emit {
                n: Some(14),
                col: Some(vec![[230.0, 240.0, 255.0], [150.0, 180.0, 240.0], [255.0, 255.0, 255.0]]),
                ..Emit::default()
            };
            fx.pa.emit(t0, pos, "sparks", &emit);
        }
        for &(t0, x, y, _rmax) in &self.stomps {
            if t0 > p.t_ko {
                break;
            }
            let pos = up(x, y, 0.1);
            let emit = Emit {
                n: Some(10),
                col: Some(DUST.to_vec()),
                speed: Some((0.5, 1.3)),
                ..Emit::default()
            };
            fx.pa.emit(t0, pos, "dust", &emit);
            fx.mk.add(
                t0,
                Mark {
                    size: 0.6,
                    life: 5.0,
                    seed: seed(t0, 53.0, 997),
                    strength: 0.7,
                    ..Mark::new(MarkKind::Crack, x, y)
                },
            );
            fx.fl.add(t0, up(x, y, R * 0.5), col, 1.8, 0.45, 0.03, 0.18, 0.0);
        }
        for &(t0, x, y) in &self.trample {
            if t0 > p.t_ko {
                break;
            }
            let pos = up(x, y, R);
            fx.bu.add(t0, pos, p.col[k], 1.5, None, true);
            fx.fl.add(t0, pos, col, 2.2, 0.9, 0.03, 0.3, 0.0);
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.kind != "hit" {
            return;
        }
        let pos = scene::to_world(e.f(5), e.f(6), R);
        match how(e) {
            "step" => {
                let who = e.who().expect("a hit names its attacker");
                fx.bu.add(e.t, pos, p.col[who], 0.55, None, false);
            }
            "slap" => fx.bu.add(e.t, pos, [230.0, 240.0, 255.0], 0.9, None, true),
            _ => {}
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let sparks = Emit {
            n: Some(28),
            spread: Some(180.0),
            speed: Some((0.8, 2.4)),
            col: Some(vec![[180.0, 200.0, 240.0], [230.0, 236.0, 250.0]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
        let chips = Emit {
            n: Some(12),
            col: Some(vec![WOOD, [28.0, 22.0, 24.0], STEEL]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "chips", &chips);
    }

    // ------------------------------------------------------------ the canvas: quake rings
    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.owner != self.k {
            return;
        }
        let (wid, gain) = match w.kind.as_str() {
            "step" => (0.12, 0.75),
            "stampede" => (0.18, 1.0),
            "slap" => (0.08, 1.2),
            _ => return,
        };
        let fade = dmath::pow(1.0 - w.r / w.rmax, 0.6);
        let tint = if w.kind != "slap" {
            p.col[self.k].map(|v| v / 255.0 * 0.8 + 0.2)
        } else {
            [0.85, 0.92, 1.0]
        };
        for (j, light) in ic.iter_mut().enumerate() {
            let rd = np::hypot(c.sxp[j] - w.cx, c.szp[j] - w.cz);
            let band = np::clip(1.0 - (rd - w.r).abs() / wid, 0.0, 1.0) * fade;
            let band = (band * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
            let v = band * gain;
            for ch in 0..3 {
                light[ch] += v * tint[ch];
            }
        }
    }

    // ------------------------------------------------------------ the painter pass: ears behind, trunk in front
    fn items_ball(&self, p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        if self.stamp_at(p, f.t).is_none() || !st.vis {
            return Vec::new();
        }
        let dz = f.cam.depth(&[scene::to_world(st.p[0], st.p[1], R + st.lift)])[0];
        let ears: Payload = Box::new(Item::Ears);
        let trunk: Payload = Box::new(Item::Trunk);
        vec![(dz + 0.02, ears), (dz - 0.02, trunk)]
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let Some(&what) = payload.downcast_ref::<Item>() else {
            return;
        };
        let cam = f.cam;
        let (mut arr, depth) = (fr.array(), fr.depth());
        let st = &f.fs[self.k];
        let at = st.p;
        let h = R + st.lift;
        let w = scene::to_world(at[0], at[1], h);
        let c = cam.wp(&[w])[0];
        let rp = R * cam.f / cam.depth(&[w])[0];
        let t = f.t;
        match what {
            Item::Ears => {
                // big flapping ears either side
                let flap = dmath::sin(2.0 * PI * t * 2.0);
                for side in [-1.0, 1.0] {
                    let a0 = side * (0.15 + 0.12 * flap);
                    let pts: Vec<(f64, f64)> = (0..9)
                        .map(|j| {
                            let th = -1.1 + 2.2 * f64::from(j) / 8.0;
                            let rr = rp * (1.55 + 0.3 * dmath::cos(th * 1.3));
                            (
                                c.0 + side * (rp * 0.6 + rr * 0.55 * dmath::cos(th + a0)),
                                c.1 + rr * 0.6 * dmath::sin(th),
                            )
                        })
                        .collect();
                    reeds::blend(&mut arr, &reeds::mask(std::slice::from_ref(&pts), &[], 1), INK, Num::Py(0.8));
                    let inner: Vec<(f64, f64)> = pts
                        .iter()
                        .map(|&(x, y)| (c.0 + (x - c.0) * 0.92, c.1 + (y - c.1) * 0.92))
                        .collect();
                    reeds::blend(&mut arr, &reeds::mask(&[inner], &[], 1), EAR, Num::Py(1.0));
                    let pink: Vec<(f64, f64)> = pts
                        .iter()
                        .map(|&(x, y)| (c.0 + (x - c.0) * 0.78 + side * 2.0, c.1 + (y - c.1) * 0.7))
                        .collect();
                    reeds::blend(&mut arr, &reeds::mask(&[pink], &[], 1), EAR_IN, Num::Py(0.6));
                }
            }
            Item::Trunk => {
                // the trunk swinging ahead, curling up at the end
                let v = p
                    .run
                    .row("V", p.sim_i(py_min(t, p.t_ko)), self.k)
                    .expect("the recording has velocities");
                let hd = dmath::atan2(v[1], v[0]);
                let swing = 0.5 * dmath::sin(2.0 * PI * t * 1.6);
                let curl = if swing >= 0.0 { 1.0 } else { -1.0 };
                let world: Vec<V3> = (0..12)
                    .map(|j| {
                        // out low in front, then curling up and back at the tip
                        let u = f64::from(j) / 11.0;
                        let a = hd + swing * u + 1.4 * dmath::pow(u, 3.0) * curl;
                        let d = R * 0.8 + 0.7 * u - 0.25 * dmath::pow(u, 4.0);
                        scene::to_world(
                            at[0] + d * dmath::cos(a),
                            at[1] + d * dmath::sin(a),
                            h - 0.25 * u + 0.6 * dmath::pow(u, 3.0),
                        )
                    })
                    .collect();
                let sp = cam.wp(&world);
                reeds::blend(&mut arr, &reeds::mask(&[], std::slice::from_ref(&sp), 9), INK, Num::Py(0.85));
                reeds::blend(&mut arr, &reeds::mask(&[], std::slice::from_ref(&sp), 6), EAR, Num::Py(1.0));
                let tip = sp[1..].to_vec();
                reeds::blend(&mut arr, &reeds::mask(&[], &[tip], 1), EAR.map(|v| v * 0.75), Num::Py(0.8));
            }
        }
        fr.set_array(&arr, Some(&depth));
    }

    // ------------------------------------------------------------ effects over the ring
    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        let (cam, k) = (f.cam, self.k);
        let t = f.t;
        let col = p.col[k];
        if f.in_fight {
            for w in p.run.waves(f.i) {
                if w.owner != k || !matches!(w.kind.as_str(), "step" | "stampede" | "slap") {
                    continue;
                }
                let kk = dmath::pow(1.0 - w.r / w.rmax, 0.7);
                // int(t0 * 1000) % 9973
                let seed = py::imod_py(np::f64_to_i64(w.t0 * 1000.0), 9973) as u64;
                let amp = if w.kind == "step" { 0.04 } else { 0.07 };
                let ring = jag_ring(w.cx, w.cz, w.r, seed, 0.05, 56, amp);
                let world: Vec<V3> = ring.iter().map(|q| scene::to_world(q[0], q[1], q[2])).collect();
                let sp = cam.wp(&world);
                reeds::blend(arr, &reeds::mask(&[], std::slice::from_ref(&sp), 3), INK, Num::Py(0.5 * kk));
                let lc = if w.kind != "slap" {
                    col.map(|v| np::clip(v * 0.6 + 100.0, 0.0, 255.0))
                } else {
                    [235.0, 245.0, 255.0]
                };
                reeds::blend(arr, &reeds::mask(&[], &[sp], 1), lc, Num::Py(kk));
            }
        }
        let tc = self.ta(p, t);
        let st = &f.fs[k];
        for &(t0, _broke) in &self.slaps {
            // SLAP BASS: a string snapping across the glass
            let age = tc - t0;
            if 0.0 <= age && age < 0.16 && st.vis {
                let at = st.p;
                let kk = 1.0 - age / 0.16;
                let wob = 0.12 * dmath::sin(age * 120.0) * kk;
                let z = [
                    scene::to_world(at[0] - R * 0.9, at[1] + wob, R * 1.2),
                    scene::to_world(at[0], at[1] - wob, R * 1.35),
                    scene::to_world(at[0] + R * 0.9, at[1] + wob, R * 1.2),
                ];
                let sp = cam.wp(&z);
                reeds::blend(arr, &reeds::mask(&[], std::slice::from_ref(&sp), 3), INK, Num::Py(0.6 * kk));
                reeds::blend(arr, &reeds::mask(&[], &[sp], 1), WHITE, Num::Py(kk));
            }
        }
    }

    fn shimmer(&self, _p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, w: &Wave) {
        if w.kind == "stampede" && w.owner == self.k {
            let c = scene::to_world(w.cx, w.cz, 0.05);
            img.shimmer_world(cam, c, w.r, 0.4, 2.2, t, 1.0 - w.r / w.rmax, w.owner as f64);
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
        let d = unit(rot);
        let nn = [-d[1], d[0]];
        let c = [pos[0], pos[2]];
        let on_screen = |pts: &[[f64; 2]]| -> Vec<(f64, f64)> {
            let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], pos[1])).collect();
            screen(cam, &world)
        };
        match kind {
            "string" => {
                let pts: Vec<[f64; 2]> = (0..5)
                    .map(|j| {
                        // c + d * (j - 2) * 0.14 + nn * curl * 0.07 * math.sin(j * 1.5)
                        let s = f64::from(j - 2);
                        let w = dmath::sin(f64::from(j) * 1.5);
                        [
                            c[0] + d[0] * s * 0.14 + nn[0] * curl * 0.07 * w,
                            c[1] + d[1] * s * 0.14 + nn[1] * curl * 0.07 * w,
                        ]
                    })
                    .collect();
                reeds::blend(arr, &reeds::mask(&[], &[on_screen(&pts)], 1), [206.0, 200.0, 188.0], Num::Py(0.9));
            }
            "endpin" => {
                let pts = [
                    [c[0] - d[0] * 0.16, c[1] - d[1] * 0.16],
                    [c[0] + d[0] * 0.16, c[1] + d[1] * 0.16],
                ];
                reeds::blend(arr, &reeds::mask(&[], &[on_screen(&pts)], 2), STEEL, Num::Py(1.0));
            }
            "scroll" => {
                let a = linspace(0.0, 4.0 * PI, 16);
                let pts: Vec<[f64; 2]> = a
                    .iter()
                    .map(|&x| {
                        let r_ = 0.02 + 0.012 * x;
                        let u = [dmath::cos(x + rot), dmath::sin(x + rot)];
                        [c[0] + u[0] * r_, c[1] + u[1] * r_]
                    })
                    .collect();
                reeds::blend(arr, &reeds::mask(&[], &[on_screen(&pts)], 2), [152.0, 92.0, 50.0], Num::Py(1.0));
            }
            _ => {
                let sz = size * 1.7;
                let tri = [
                    [c[0] + d[0] * sz, c[1] + d[1] * sz],
                    [
                        c[0] - d[0] * sz * 0.7 + nn[0] * sz * 0.8,
                        c[1] - d[1] * sz * 0.7 + nn[1] * sz * 0.8,
                    ],
                    [
                        c[0] - d[0] * sz * 0.4 - nn[0] * sz * 0.7,
                        c[1] - d[1] * sz * 0.4 - nn[1] * sz * 0.7,
                    ],
                ];
                let sp = on_screen(&tri);
                let shade = 0.75 + 0.35 * dmath::cos(rot).abs();
                layer_blend(arr, std::slice::from_ref(&sp), WOOD.map(|v| v * shade), 1.0, None);
                // np.vstack([sp, sp[:1]]): the outline, closed
                let mut edge = sp;
                edge.push(edge[0]);
                reeds::blend(arr, &reeds::mask(&[], &[edge], 1), [50.0, 22.0, 12.0], Num::Py(0.8));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_shatter_is_thirteen_parts_of_four_kinds() {
        let parts = ContrabassLook::new().shards();
        let count = |kind: &str| parts.iter().filter(|p| *p == kind).count();
        assert_eq!(
            (parts.len(), count("body"), count("scroll"), count("endpin"), count("string")),
            (13, 6, 1, 2, 4)
        );
    }

    #[test]
    fn a_jagged_ring_is_closed_and_near_its_radius() {
        let ring = jag_ring(0.5, -0.25, 1.0, 1234, 0.05, 56, 0.05);
        assert_eq!(ring.len(), 57);
        for q in &ring {
            let r = ((q[0] - 0.5).powi(2) + (q[1] + 0.25).powi(2)).sqrt();
            assert!((0.95 - 1e-12..=1.05 + 1e-12).contains(&r), "{r}");
            assert_eq!(q[2], 0.05);
        }
    }
}
