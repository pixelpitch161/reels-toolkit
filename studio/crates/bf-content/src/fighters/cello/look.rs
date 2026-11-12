//! CELLO's look: the port of `Look` in `projects/ballfight/kit/fighters/cello.py`, hook by hook.
//!
//! What it draws: the Pixel Pitch cello, a little smaller in its glass (`sprite_k` 0.74). The bow is its arm: on a
//! rush it reaches out ahead (stick, ivory hair, ebony frog); on a grab it wraps round the far side of the enemy,
//! which is swept round off its feet with a rust motion trail and flung away with rosin dust; a rope slam rings the
//! ropes. VIBRATO: the held enemy trembles inside rust zig-zags. PRELUDE: thin amber rings rock across the canvas,
//! eighth notes riding each one. SWAN SONG: it lifts off on two white wings, glides, folds them and dives into the
//! canvas like into a lake (a splash ring, foam and feathers). Its shards are the cello's body, neck, bow and strings.
//!
//! Its panel on the VS card is its own `title_pattern` (bow-hair stripes along the divider with rows of f-holes).
//!
//! The kit class reads nothing from `self.cfg`, so `cfg_summary` is the default `None` and `setup_as` takes only
//! the side's events.

use bf_core::dmath::{self, PI};
use bf_core::event::Event;
use bf_core::py::{self, py_max, py_min};
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{H, W, bayer_full};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::{Img, RgbF};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Fx, Look, Phase, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::title::layout;

use crate::fighters::shared::reeds::{self, INK, Num, Px};

/// `STICK`: the pernambuco bow stick.
const STICK: V3 = [92.0, 52.0, 30.0];
/// `HAIR`: the bow's ivory hair.
const HAIR: V3 = [240.0, 230.0, 206.0];
/// `FROG`: the ebony frog.
const FROG: V3 = [24.0, 20.0, 22.0];
/// `RUST_DUST`: rosin dust.
const RUST_DUST: [V3; 3] = [[240.0, 220.0, 170.0], [220.0, 190.0, 140.0], [255.0, 246.0, 222.0]];
/// `WING`: the swan's wings.
const WING: V3 = [250.0, 250.0, 246.0];
/// `WING_D`: their feather lines.
const WING_D: V3 = [196.0, 200.0, 214.0];
/// `WATER`: the lake's splash.
const WATER: V3 = [196.0, 230.0, 255.0];
/// `BODY`: the cello's varnish (the Pixel Pitch model's).
const BODY: V3 = [188.0, 84.0, 40.0];
/// `EBONY`: the neck's shard.
const EBONY: V3 = [30.0, 24.0, 26.0];
/// The feathers of take-off and the dive: `[(250, 250, 246), (220, 224, 236)]`.
const FEATHERS: [V3; 2] = [[250.0, 250.0, 246.0], [220.0, 224.0, 236.0]];

/// `np.array([math.cos(a), math.sin(a)])`.
fn unit(a: f64) -> [f64; 2] {
    [dmath::cos(a), dmath::sin(a)]
}

/// `e[7]` of a hit (what hit), or `""` for an event without one.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(bf_core::event::Value::Str(s))) => s,
        _ => "",
    }
}

/// `np.linspace(a, b, n)`: `i * step + a`, `step = (b - a) / (n - 1)`, the last value `b` itself.
fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    let step = (b - a) / (n - 1) as f64;
    (0..n)
        .map(|i| if i == n - 1 { b } else { i as f64 * step + a })
        .collect()
}

/// `_seg_pts(cam, P)`: ring-frame points `(x, z, height)` on the screen.
fn seg_pts(cam: &Cam, pts: &[V3]) -> Vec<(f64, f64)> {
    let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], q[2])).collect();
    cam.wp(&world)
}

/// `img[y0:y1, x0:x1] = img[y0:y1, x0:x1] * (1 - k) + col * k` (callers keep the box inside the picture): the
/// picture's dtype times a Python float, plus a `float64` colour, stored back.
fn paint_box<T: Px>(img: &mut Img<T>, ys: std::ops::Range<i64>, xs: std::ops::Range<i64>, col: V3, k: f64) {
    for y in ys {
        for x in xs.clone() {
            if (0..H as i64).contains(&y) && (0..W as i64).contains(&x) {
                for (v, c) in img.px_mut(x as usize, y as usize).iter_mut().zip(col) {
                    *v = T::set(v.times(1.0 - k) + c * k);
                }
            }
        }
    }
}

/// `draw_bow(img, cam, P, k, frog, col)`: a bow along ring-frame points `P`: an inked stick, the ivory hair beside
/// it, the ebony frog at `P[0]`.
fn draw_bow(img: &mut Img<f32>, cam: &Cam, pts: &[V3], k: f64, frog: bool) {
    if k <= 0.02 || pts.len() < 2 {
        return;
    }
    let sp = seg_pts(cam, pts);
    reeds::blend(
        img,
        &reeds::mask(&[], std::slice::from_ref(&sp), 4),
        INK,
        Num::Py(0.6 * k),
    );
    reeds::blend(img, &reeds::mask(&[], std::slice::from_ref(&sp), 2), STICK, Num::Py(k));
    // P + np.array([0.0, 0.0, -0.05])
    let hair: Vec<V3> = pts.iter().map(|q| [q[0] + 0.0, q[1] + 0.0, q[2] + -0.05]).collect();
    let hp = seg_pts(cam, &hair);
    reeds::blend(img, &reeds::mask(&[], &[hp], 1), HAIR, Num::Py(0.95 * k));
    if frog {
        let f = sp[0];
        let (x, y) = (py::round_half_even(f.0) as i64, py::round_half_even(f.1) as i64);
        if 2 <= x && x < W as i64 - 2 && 2 <= y && y < H as i64 - 2 {
            paint_box(img, y - 2..y + 2, x - 2..x + 2, FROG, k);
        }
    }
}

/// `arm_wrap(me, en, h)`: the bow arm hugging the enemy: from the cello's surface toward it, round the enemy's far
/// side (`rw = R * 1.1`, `open_ = 1.25`, 18 points round).
fn arm_wrap(me: [f64; 2], en: [f64; 2], h: f64) -> Vec<V3> {
    let (rw, open, n) = (R * 1.1, 1.25, 18);
    let d = [en[0] - me[0], en[1] - me[1]];
    let phi = dmath::atan2(d[1], d[0]);
    let th = linspace(phi + PI - open, phi - PI + open, n);
    let start = [
        me[0] + dmath::cos(phi) * R * 0.85,
        me[1] + dmath::sin(phi) * R * 0.85,
        h,
    ];
    let mut out = vec![start];
    out.extend(
        th.iter()
            .map(|&a| [en[0] + rw * dmath::cos(a), en[1] + rw * dmath::sin(a), h]),
    );
    out
}

/// `wing_poly(cx, cy, side, span, flap, fold)`: one swan wing on the screen: a fan of three feathers from the
/// ball's shoulder.
fn wing_poly(cx: f64, cy: f64, side: f64, span: f64, flap: f64, fold: f64) -> Vec<(f64, f64)> {
    let mut pts = vec![(cx + side * 3.0, cy - 2.0)];
    // radians above horizontal
    let base = -0.25 + 0.75 * flap * (1.0 - fold);
    for (ln, da) in [(1.0, 0.0), (0.85, -0.35), (0.62, -0.7)] {
        let a = base + da * (1.0 - 0.6 * fold) + fold * 1.1;
        let l = span * ln * (1.0 - 0.45 * fold);
        pts.push((cx + side * (3.0 + l * dmath::cos(a)), cy - 2.0 - l * dmath::sin(a)));
    }
    pts.push((cx + side * 3.0, cy + 3.0));
    pts
}

/// `title_pattern(base)`: CELLO's VS-card panel: bow-hair stripes along the divider with rows of f-holes between
/// them. `base` is the panel's gradient, changed in place.
pub fn title_pattern(base: &mut RgbF) {
    let dd = layout::dd();
    let side = layout::side();
    let da = layout::DA;
    // HAIR * 0.55, np.array([30, 14, 10]) * 0.75
    let hair = HAIR.map(|c| c * 0.55);
    let ink = [30.0 * 0.75, 14.0 * 0.75, 10.0 * 0.75];
    for y in 0..base.h {
        for x in 0..base.w {
            let u = (x as f64 - da[0]) * dd[0] + (y as f64 - da[1]) * dd[1];
            let av = side.px(x, y)[0].abs();
            // stripe = np.abs(((np.abs(v) + 30) % 84.0) - 42.0) < 1.3
            if (py::fmod_py(av + 30.0, 84.0) - 42.0).abs() < 1.3 {
                for (ch, b) in base.px_mut(x, y).iter_mut().enumerate() {
                    *b = *b * 0.45 + hair[ch];
                }
            }
            let (cu, cv) = (py::fmod_py(u, 70.0) - 35.0, py::fmod_py(av, 84.0) - 42.0);
            let f = (cv - 0.5 * dmath::sin(cu / 6.0) * 6.0 - cu * 0.25).abs() < 2.2 && cu.abs() < 15.0;
            let (e0, e1) = ((cu - 14.0, cv - 3.0), (cu + 14.0, cv + 11.0));
            let eyes = e0.0 * e0.0 + e0.1 * e0.1 < 10.0 || e1.0 * e1.0 + e1.1 * e1.1 < 10.0;
            if f || eyes {
                for (ch, b) in base.px_mut(x, y).iter_mut().enumerate() {
                    *b = *b * 0.25 + ink[ch];
                }
            }
        }
    }
}

/// One `throw` event of its own: `dict(t=e[0], a=e[3], power=e[4], x=e[5], y=e[6])` (the drawing reads `t`, `x`,
/// `y`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Throw {
    t: f64,
    x: f64,
    y: f64,
}

/// One `swan` event of its own (the drawing reads the times, the take-off spot and the height).
#[derive(Clone, Copy, Debug, PartialEq)]
struct SwanFlight {
    t0: f64,
    tg: f64,
    td: f64,
    tl: f64,
    p0: [f64; 2],
    h: f64,
}

/// One `prelude_note` event of its own: `dict(t=e[0], j=e[3], x=e[4], y=e[5], rmax=e[6])`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Note {
    t: f64,
    x: f64,
    y: f64,
    rmax: f64,
}

/// `grab_at(t)`: `(state, arm angle, t0, t1, reach)` of its grapple from the record.
type Grab = (i64, f64, f64, f64, f64);

// ---------------------------------------------------------------- the look
/// CELLO's look: `Look` of `kit/fighters/cello.py`. (`HOLDS` and `RELEASES`, which the kit's `setup` also lists,
/// are read by no hook.)
#[derive(Clone, Debug, Default)]
pub struct CelloLook {
    /// Its side.
    k: usize,
    /// `THROWS`.
    throws: Vec<Throw>,
    /// `SLAMS`: `(t, x, y)` of its rope slams.
    slams: Vec<(f64, f64, f64)>,
    /// `GRABS`: `(t, how, x, y)` of its rushes' ends.
    grabs: Vec<(f64, i64, f64, f64)>,
    /// `VIB`: `(t0, t1)` of its VIBRATOs.
    vib: Vec<(f64, f64)>,
    /// `NOTES`: PRELUDE's note-waves.
    notes: Vec<Note>,
    /// `SWANS`: its SWAN SONGs.
    swans: Vec<SwanFlight>,
    /// `LANDS`: `(t, x, y, hit)` of its landings.
    lands: Vec<(f64, f64, f64, i64)>,
}

impl CelloLook {
    /// A look before `setup`.
    pub fn new() -> CelloLook {
        CelloLook::default()
    }

    /// `show(t)`: the bow, rings and wings show from FIGHT! to just after the shatter.
    fn show(&self, p: &PictureCtx, t: f64) -> bool {
        p.show.grid.t_fight <= t && t < p.t_hs + 0.12
    }

    /// `ta(t)`: the attack clock, frozen at the K.O.
    fn ta(&self, p: &PictureCtx, t: f64) -> f64 {
        py_min(t, p.t_ko)
    }

    /// `grab_at(t)`: `(state, arm angle, t0, t1, reach)` of its grapple at `t` (the run's record), or `None`.
    fn grab_at(&self, p: &PictureCtx, t: f64) -> Option<Grab> {
        if !(p.show.grid.t_fight <= t && t < p.t_hs) || !p.run.has("GRAB") {
            return None;
        }
        let g = p.run.row("GRAB", p.sim_i(py_min(t, p.t_ko)), self.k)?;
        // (int(g[0]), ...) if g[0] > 0: int() of a float, toward zero
        (g[0] > 0.0).then(|| (g[0] as i64, g[1], g[2], g[3], g[4]))
    }

    /// `vib_on(t)`: VIBRATO is on.
    fn vib_on(&self, t: f64) -> bool {
        self.vib.iter().any(|&(t0, t1)| t0 <= t && t < t1)
    }

    /// `swan_at(t)`: `(swan, height, phase 0 rise / 1 glide / 2 dive, u)` while airborne (attack clock), else
    /// `None`.
    fn swan_at(&self, p: &PictureCtx, t: f64) -> Option<(SwanFlight, f64, i64, f64)> {
        let tc = p.fx_clock(t);
        for sw in &self.swans {
            if sw.t0 <= tc && tc < sw.tl {
                let h = sw.h;
                if tc < sw.tg {
                    let u = (tc - sw.t0) / (sw.tg - sw.t0);
                    return Some((*sw, h * (u * u * (3.0 - 2.0 * u)), 0, u));
                }
                if tc < sw.td {
                    let u = (tc - sw.tg) / (sw.td - sw.tg);
                    return Some((*sw, h * (1.0 + 0.08 * dmath::sin(2.0 * PI * u * 2.0)), 1, u));
                }
                let u = (tc - sw.td) / (sw.tl - sw.td);
                return Some((*sw, h * (1.0 - u * u), 2, u));
            }
        }
        None
    }
}

/// `st['glow'] = max(st['glow'], x)` with `x` a Python float: it is the glow when it is greater (and then a Python
/// float, ruling M3-R37; `glow_types.py` finds no numpy glow in CELLO's left fight).
fn raise_glow(st: &mut BallState, x: f64) {
    if x > st.glow {
        st.glow = x;
        st.glow_np = false;
    }
}

impl Look for CelloLook {
    /// `['body'] * 5 + ['neck'] * 2 + ['bow'] * 2 + ['string'] * 4`.
    fn shards(&self) -> Vec<String> {
        [("body", 5), ("neck", 2), ("bow", 2), ("string", 4)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["ropeslam", "swan", "throw"]
    }

    fn spark_flash(&self) -> &[&str] {
        &["grab"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["ropeslam", "swan"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["ropeslam", "swan"]
    }

    fn quiet_hits(&self) -> &[&str] {
        &["vibrato", "prelude"]
    }

    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[]
    }

    /// A little small in its glass: the double bass towers over it.
    fn sprite_k(&self) -> f64 {
        0.74
    }

    fn title_pattern(&self, base: &mut RgbF) {
        title_pattern(base);
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`: the look reads no roster number, only side `k`'s events.
    fn setup_as(&mut self, p: &PictureCtx, k: usize, _cfg: &FighterCfg) {
        self.k = k;
        let ev = &p.rec.events;
        let mine = |kind: &'static str| ev.iter().filter(move |e| e.kind == kind && e.who() == Some(k));
        self.throws = mine("throw")
            .map(|e| Throw {
                t: e.t,
                x: e.f(5),
                y: e.f(6),
            })
            .collect();
        self.slams = mine("ropeslam").map(|e| (e.t, e.f(3), e.f(4))).collect();
        self.grabs = mine("grab").map(|e| (e.t, e.i(3), e.f(4), e.f(5))).collect();
        self.vib = mine("vibrato").map(|e| (e.t, e.f(3))).collect();
        self.notes = mine("prelude_note")
            .map(|e| Note {
                t: e.t,
                x: e.f(4),
                y: e.f(5),
                rmax: e.f(6),
            })
            .collect();
        self.swans = mine("swan")
            .map(|e| SwanFlight {
                t0: e.f(3),
                tg: e.f(4),
                td: e.f(5),
                tl: e.f(6),
                p0: [e.f(7), e.f(8)],
                h: e.f(13),
            })
            .collect();
        self.lands = mine("swan_land").map(|e| (e.t, e.f(3), e.f(4), e.i(5))).collect();
    }

    // ------------------------------------------------------------ state
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if phase != Phase::Fight {
            return;
        }
        let held = self.grab_at(p, t).filter(|g| g.0 == 2);
        if kk == self.k {
            if let Some(sw) = self.swan_at(p, t) {
                st.lift = sw.1;
                raise_glow(st, 0.35);
            }
            if held.is_some() {
                raise_glow(st, 0.25 + 0.25 * f64::from(u8::from(self.vib_on(self.ta(p, t)))));
            }
            return;
        }
        if let Some(g) = held {
            // the held enemy: swept off its feet, trembling
            let u = (self.ta(p, t) - g.2) / py_max(1e-6, g.3 - g.2);
            st.lift = 0.16 * dmath::sin(PI * py_min(1.0, u));
            if self.vib_on(self.ta(p, t)) {
                // f = int(round(t * 30)); st['p'] + 0.05 * np.array([math.sin(f * 2.7), math.cos(f * 3.9)])
                let f = py::round_half_even(t * 30.0);
                let d = [0.05 * dmath::sin(f * 2.7), 0.05 * dmath::cos(f * 3.9)];
                st.p = [st.p[0] + d[0], st.p[1] + d[1]];
            }
        }
    }

    // ------------------------------------------------------------ events -> camera, ropes, sparks, glows
    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match e.kind.as_str() {
            "ropeslam" => Some(1.8),
            "swan_land" => Some(if e.at(5).truthy() { 2.4 } else { 1.4 }),
            "throw" => Some(0.8),
            "hit" if how(e) == "grab" => Some(0.5),
            _ => None,
        }
    }

    fn rope(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match e.kind.as_str() {
            "ropeslam" => Some(1.0),
            "swan_land" => Some(0.6),
            _ => None,
        }
    }

    fn spark(&self, _p: &PictureCtx, e: &Event) -> Spark {
        match how(e) {
            "ropeslam" => ([255.0, 190.0, 140.0], 18, 3.4, 0.34),
            "swan" => (WATER, 20, 3.2, 0.4),
            "vibrato" | "prelude" => ([255.0, 200.0, 140.0], 3, 1.2, 0.14),
            _ => ([255.0, 196.0, 150.0], 8, 2.2, 0.2),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "ropeslam" | "swan" => Some((16.0, 0.42)),
            "grab" | "throw" => Some((11.0, 0.32)),
            _ => None,
        }
    }

    // ------------------------------------------------------------ effects (reelkit.fx)
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let k = self.k;
        let col = p.fl_col[k];
        let emit = |n: i64, cols: &[V3], speed: Option<(f64, f64)>| Emit {
            n: Some(n),
            col: Some(cols.to_vec()),
            speed,
            ..Emit::default()
        };
        for &(t0, how, x, y) in &self.grabs {
            if t0 > p.t_ko {
                break;
            }
            let pos = up(x, y, R);
            if how == 1 {
                // the embrace: rosin dust off the bow
                fx.pa.emit(t0, pos, "rosin", &emit(10, &RUST_DUST, None));
                fx.fl.add(t0, pos, col, 1.2, 0.4, 0.03, 0.2, 0.0);
            } else if how == 2 {
                // it slipped off a walking bass: a puff
                fx.pa.emit(t0, pos, "puffs", &emit(4, &RUST_DUST, None));
            }
        }
        for th in &self.throws {
            if th.t > p.t_ko {
                break;
            }
            let pos = up(th.x, th.y, R);
            fx.bu.add(th.t, pos, p.col[k], 0.9, None, true);
            fx.pa.emit(th.t, pos, "rosin", &emit(14, &RUST_DUST, Some((0.6, 1.6))));
            fx.fl.add(th.t, pos, col, 1.6, 0.6, 0.03, 0.22, 0.0);
        }
        for &(t0, x, y) in &self.slams {
            if t0 > p.t_ko {
                break;
            }
            let pos = up(x, y, R * 0.9);
            fx.bu.add(t0, pos, p.col[k], 1.5, None, true);
            fx.fl.add(t0, pos, col, 2.0, 0.9, 0.03, 0.3, 0.0);
            let sparks = [[255.0, 210.0, 160.0], [255.0, 160.0, 100.0], [255.0, 246.0, 222.0]];
            fx.pa.emit(t0, pos, "sparks", &emit(16, &sparks, None));
            // seed=int(t0 * 100) % 997
            let seed = py::imod_py(np::f64_to_i64(t0 * 100.0), 997) as u64;
            fx.mk.add(
                t0,
                Mark {
                    size: 0.5,
                    life: 4.0,
                    seed,
                    strength: 0.5,
                    ..Mark::new(MarkKind::Claw, x, y)
                },
            );
        }
        // PRELUDE: each note-wave lights the cello
        for nt in &self.notes {
            if nt.t > p.t_ko {
                break;
            }
            fx.fl
                .add(nt.t, up(nt.x, nt.y, R), col, 1.0 + 0.5 * nt.rmax, 0.25, 0.03, 0.12, 0.0);
        }
        // SWAN SONG: feathers on take-off
        for sw in &self.swans {
            if sw.t0 > p.t_ko {
                break;
            }
            fx.pa.emit(
                sw.t0 + 0.1,
                up(sw.p0[0], sw.p0[1], R),
                "shreds",
                &emit(8, &FEATHERS, None),
            );
        }
        // the dive: a splash into the canvas
        for &(t0, x, y, _hit) in &self.lands {
            if t0 > p.t_ko {
                break;
            }
            let pos = up(x, y, 0.1);
            fx.bu.add(t0, pos, [210.0, 236.0, 255.0], 1.8, None, true);
            fx.fl
                .add(t0, up(x, y, R), [190.0, 220.0, 255.0], 2.6, 1.0, 0.03, 0.4, 0.0);
            let foam = [[230.0, 246.0, 255.0], [190.0, 225.0, 255.0], [255.0, 255.0, 255.0]];
            fx.pa.emit(t0, pos, "foam", &emit(18, &foam, None));
            fx.pa.emit(t0, up(x, y, R), "shreds", &emit(10, &FEATHERS, None));
            fx.mk.add(
                t0,
                Mark {
                    size: 1.2,
                    life: 5.0,
                    seed: 17,
                    strength: 0.8,
                    ..Mark::new(MarkKind::Water, x, y)
                },
            );
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.kind != "hit" {
            return;
        }
        let pos = scene::to_world(e.f(5), e.f(6), R);
        match how(e) {
            "grab" => {
                let att = e.who().filter(|&k| k < 2).unwrap_or(0);
                fx.bu.add(e.t, pos, p.col[att], 0.6, None, false);
            }
            "vibrato" => {
                let emit = Emit {
                    n: Some(2),
                    col: Some(RUST_DUST.to_vec()),
                    speed: Some((0.3, 0.8)),
                    ..Emit::default()
                };
                fx.pa.emit(e.t, pos, "rosin", &emit);
            }
            _ => {}
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let sparks = Emit {
            n: Some(26),
            spread: Some(180.0),
            speed: Some((0.8, 2.4)),
            col: Some(vec![[255.0, 200.0, 150.0], [240.0, 230.0, 206.0]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
        let chips = Emit {
            n: Some(10),
            col: Some(vec![BODY, [30.0, 24.0, 26.0], [240.0, 230.0, 206.0]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "chips", &chips);
    }

    // ------------------------------------------------------------ the canvas: PRELUDE's rings, the splash
    fn canvas_wave(&self, _p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind != "prelude" || w.owner != self.k {
            return;
        }
        let fade = dmath::pow(1.0 - w.r / w.rmax, 0.6);
        let tint = [255.0 / 255.0, 196.0 / 255.0, 110.0 / 255.0];
        let bayer = bayer_full();
        for (j, light) in ic.iter_mut().enumerate() {
            let rd = np::hypot(c.sxp[j] - w.cx, c.szp[j] - w.cz);
            let band = np::clip(1.0 - (rd - w.r).abs() / 0.09, 0.0, 1.0) * fade;
            let band = (band * 3.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 3.0;
            let v = band * 0.85;
            for ch in 0..3 {
                light[ch] += v * tint[ch];
            }
        }
    }

    fn canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let tc = p.fx_clock(c.t);
        let tint = WATER.map(|v| v / 255.0);
        let bayer = bayer_full();
        // the splash ring spreading like water
        for &(t0, x, y, _hit) in &self.lands {
            let age = tc - t0;
            if !(0.0..0.9).contains(&age) {
                continue;
            }
            for j in 0..2 {
                let jf = f64::from(j);
                let rr = 0.3 + (2.2 - 0.5 * jf) * (1.0 - dmath::pow(1.0 - py_min(1.0, age / 0.9), 2.0));
                let kk = 0.9 - 0.3 * jf;
                for (i, light) in ic.iter_mut().enumerate() {
                    let rd = np::hypot(c.sxp[i] - x, c.szp[i] - y);
                    let band = np::clip(1.0 - (rd - rr).abs() / 0.07, 0.0, 1.0) * (1.0 - age / 0.9);
                    let band = (band * 3.0 + bayer.px(c.xs[i], c.ys[i])[0] * 0.9).floor() / 3.0;
                    let v = band * kk;
                    for ch in 0..3 {
                        light[ch] += v * tint[ch];
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------ over the ring: the bow arm, rings, wings
    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        let (cam, k, t) = (f.cam, self.k, f.t);
        let col = p.col[k];
        if !self.show(p, t) {
            return;
        }
        let (st, en_st) = (&f.fs[k], &f.fs[1 - k]);
        let (me, en) = (st.p, en_st.p);
        let g = self.grab_at(p, t);
        let sw = self.swan_at(p, t);
        if let (Some((state, a, t0, t1, reach)), None, true) = (g, sw, st.vis) {
            if state == 1 {
                // the rush: the bow arm reaches out ahead
                let u = py_min(1.0, (self.ta(p, t) - t0) / 0.08);
                let d = unit(a);
                let l = R * 0.6 + (reach + 0.45) * u;
                let p0 = [me[0] + d[0] * R * 0.7, me[1] + d[1] * R * 0.7, R * 1.15];
                let p1 = [me[0] + d[0] * l, me[1] + d[1] * l, R * 1.05];
                let pts: Vec<V3> = linspace(0.0, 1.0, 6)
                    .iter()
                    .map(|&s| std::array::from_fn(|c| p0[c] + (p1[c] - p0[c]) * s))
                    .collect();
                draw_bow(arr, cam, &pts, 1.0, true);
            } else if state == 2 {
                // the embrace: the arm round the far side
                let h = R * 1.05 + en_st.lift * 0.6;
                let ta = self.ta(p, t);
                let u = (ta - t0) / py_max(1e-6, t1 - t0);
                // the swing's trail: where the enemy was over the last 0.12 s
                let trail: Vec<V3> = linspace(0.12, 0.0, 7)
                    .iter()
                    .map(|&dt| {
                        let q = p.fighter(1 - k, py_max(t0, ta - dt)).p;
                        [q[0], q[1], R * 0.9]
                    })
                    .collect();
                let sp = seg_pts(cam, &trail);
                let m3 = py_min(1.0, u * 3.0);
                let c1 = col.map(|c| c * 0.6 + 40.0);
                reeds::add(arr, &reeds::mask(&[], std::slice::from_ref(&sp), 7), c1, 0.22 * m3);
                let c2 = col.map(|c| np::clip(c * 0.8 + 60.0, 0.0, 255.0));
                reeds::blend(arr, &reeds::mask(&[], &[sp], 3), c2, Num::Py(0.5 * m3));
                draw_bow(arr, cam, &arm_wrap(me, en, h), 1.0, true);
                if self.vib_on(ta) {
                    // VIBRATO: zig-zags trembling round it
                    let c3 = col.map(|c| np::clip(c * 0.8 + 70.0, 0.0, 255.0));
                    for sgn in [-1.0, 1.0] {
                        let d = [en[0] - me[0], en[1] - me[1]];
                        let phi = dmath::atan2(d[1], d[0]) + sgn * 1.2;
                        let th = linspace(phi - 0.5, phi + 0.5, 12);
                        let z: Vec<V3> = th
                            .iter()
                            .enumerate()
                            .map(|(i, &a)| {
                                let rr = R * 1.45 + 0.05 * dmath::sin(i as f64 * 2.6 + t * 60.0);
                                [en[0] + rr * dmath::cos(a), en[1] + rr * dmath::sin(a), R]
                            })
                            .collect();
                        let sp = seg_pts(cam, &z);
                        reeds::blend(arr, &reeds::mask(&[], std::slice::from_ref(&sp), 3), INK, Num::Py(0.5));
                        reeds::blend(arr, &reeds::mask(&[], &[sp], 1), c3, Num::Py(1.0));
                    }
                }
            }
        }
        // a grab that slipped off a walking bass: a quick scrape
        let tc = self.ta(p, t);
        for &(t0, how, x, y) in &self.grabs {
            if how == 2 && 0.0 <= tc - t0 && tc - t0 < 0.18 {
                let kk = 1.0 - (tc - t0) / 0.18;
                let d = [x - me[0], y - me[1]];
                let nd = np::hypot(d[0], d[1]) + 1e-9;
                let n = [d[0] / nd, d[1] / nd];
                let nn = [-n[1], n[0]];
                let z: Vec<V3> = [-1.0, 1.0]
                    .iter()
                    .map(|&s| [x + nn[0] * s * 0.3, y + nn[1] * s * 0.3, R])
                    .collect();
                reeds::blend(arr, &reeds::mask(&[], &[seg_pts(cam, &z)], 2), HAIR, Num::Py(0.8 * kk));
            }
        }
        // PRELUDE: eighth notes riding the rings
        if f.in_fight {
            let c = col.map(|c| np::clip(c * 0.6 + 110.0, 0.0, 255.0));
            for w in p.run.waves(f.i) {
                if w.kind != "prelude" || w.owner != k {
                    continue;
                }
                let kk = dmath::pow(1.0 - w.r / w.rmax, 0.6);
                for j in 0..4 {
                    let a = f64::from(j) * PI / 2.0 + w.t0 * 3.1 + w.r * 0.9;
                    let at = scene::to_world(w.cx + w.r * dmath::cos(a), w.cz + w.r * dmath::sin(a), 0.12);
                    let q = cam.wp(&[at])[0];
                    let (x, y) = (py::round_half_even(q.0) as i64, py::round_half_even(q.1) as i64);
                    if 3 <= x && x < W as i64 - 3 && 6 <= y && y < H as i64 - 3 {
                        paint_box(arr, y - 1..y + 2, x - 2..x + 1, c, kk);
                        // the stem
                        paint_box(arr, y - 5..y, x..x + 1, c, kk);
                        // the flag
                        paint_box(arr, y - 5..y - 4, x + 1..x + 2, c, kk);
                    }
                }
            }
        }
        // SWAN SONG: white wings
        if let (Some((swd, h, ph, u)), true) = (sw, st.vis) {
            let lifted = scene::to_world(me[0], me[1], R + h);
            let c = cam.wp(&[lifted])[0];
            let rp = R * cam.f / cam.depth(&[lifted])[0];
            let rate = if ph == 0 { 2.2 } else { 1.3 };
            let mut flap = 0.5 + 0.5 * dmath::sin(2.0 * PI * (t - swd.t0) * rate);
            let fold = if ph < 2 { 0.0 } else { py_min(1.0, u * 1.6) };
            if ph == 0 {
                flap = 0.3 + 0.7 * flap;
            }
            for side in [-1.0, 1.0] {
                let wp = wing_poly(c.0, c.1, side, rp * 1.9, flap, fold);
                reeds::blend(arr, &reeds::mask(std::slice::from_ref(&wp), &[], 1), INK, Num::Py(0.7));
                let inner: Vec<(f64, f64)> = wp
                    .iter()
                    .map(|&(x, y)| (c.0 + (x - c.0) * 0.9, c.1 + (y - c.1) * 0.9))
                    .collect();
                reeds::blend(arr, &reeds::mask(&[inner], &[], 1), WING, Num::Py(1.0));
                // feather lines
                for &(x, y) in &wp[1..4] {
                    let line = vec![(c.0 + side * 3.0, c.1 - 1.0), (x, y)];
                    reeds::blend(arr, &reeds::mask(&[], &[line], 1), WING_D, Num::Py(0.8));
                }
            }
            if ph == 2 {
                // the dive's streak
                let top = cam.wp(&[scene::to_world(me[0], me[1], R + h + 0.9)])[0];
                let line = vec![(c.0, c.1 - rp), (top.0, top.1)];
                reeds::add(
                    arr,
                    &reeds::mask(&[], &[line], 3),
                    WING.map(|v| v * 0.6),
                    0.5 * (1.0 - u * 0.5),
                );
            }
        }
    }

    fn glows(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, _drawn: &[DrawnBall]) {
        let tc = self.ta(p, f.t);
        for th in &self.throws {
            if 0.0 <= tc - th.t && tc - th.t < 0.15 {
                let s = f.cam.wp(&[scene::to_world(th.x, th.y, R)])[0];
                let col = p.col[self.k].map(|c| c * 0.8 + 50.0);
                fr.glow(s.0, s.1, 14.0, col, 0.5 * (1.0 - (tc - th.t) / 0.15));
            }
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
                // c + d * (j - 2) * 0.12 + nn * curl * 0.06 * math.sin(j * 1.7)
                let pts: Vec<[f64; 2]> = (0..5)
                    .map(|j| {
                        let (jj, s) = (f64::from(j - 2), dmath::sin(f64::from(j) * 1.7));
                        std::array::from_fn(|i| c[i] + d[i] * jj * 0.12 + nn[i] * curl * 0.06 * s)
                    })
                    .collect();
                let sp = on_screen(&pts);
                reeds::blend(arr, &reeds::mask(&[], &[sp], 1), [214.0, 210.0, 200.0], Num::Py(0.9));
            }
            "bow" => {
                // c + d * (j - 2) * 0.16 + nn * curl * 0.03 * j
                let z: Vec<V3> = (0..5)
                    .map(|j| {
                        let jj = f64::from(j - 2);
                        let q: [f64; 2] =
                            std::array::from_fn(|i| c[i] + d[i] * jj * 0.16 + nn[i] * curl * 0.03 * f64::from(j));
                        [q[0], q[1], pos[1]]
                    })
                    .collect();
                draw_bow(arr, cam, &z, 0.9, false);
            }
            "neck" => {
                let (ln, wd) = (0.28, 0.045);
                let q4: Vec<[f64; 2]> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                    .iter()
                    .map(|&(a1, a2)| std::array::from_fn(|i| c[i] + d[i] * ln * a1 + nn[i] * wd * a2))
                    .collect();
                layer_blend(arr, &[on_screen(&q4)], EBONY, 1.0, None);
            }
            _ => {
                let sz = size * 1.5;
                let tri: Vec<[f64; 2]> = vec![
                    std::array::from_fn(|i| c[i] + d[i] * sz),
                    std::array::from_fn(|i| c[i] - d[i] * sz * 0.7 + nn[i] * sz * 0.8),
                    std::array::from_fn(|i| c[i] - d[i] * sz * 0.4 - nn[i] * sz * 0.7),
                ];
                let sp = on_screen(&tri);
                let shade = 0.75 + 0.35 * dmath::cos(rot).abs();
                layer_blend(arr, std::slice::from_ref(&sp), BODY.map(|v| v * shade), 1.0, None);
                let mut closed = sp.clone();
                closed.push(sp[0]);
                reeds::blend(arr, &reeds::mask(&[], &[closed], 1), [60.0, 26.0, 14.0], Num::Py(0.8));
            }
        }
    }

    fn focus_points(&self, _p: &PictureCtx, _t: f64, st: &BallState) -> Vec<(V3, String)> {
        if st.lift > 0.0 {
            let p = st.p;
            return vec![(scene::to_world(p[0], p[1], R + st.lift), "cello in flight".to_string())];
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// numpy's `linspace`: `i * step + start`, the last point the stop itself.
    #[test]
    fn linspace_is_numpys() {
        assert_eq!(linspace(0.0, 1.0, 6), [0.0, 0.2, 0.4, 0.6000000000000001, 0.8, 1.0]);
        let back = linspace(0.12, 0.0, 7);
        assert_eq!((back[0], back[6]), (0.12, 0.0));
    }

    /// A wing is the shoulder, three feather tips and the armpit.
    #[test]
    fn a_wing_has_three_feathers() {
        let open = wing_poly(100.0, 200.0, 1.0, 20.0, 1.0, 0.0);
        assert_eq!(open.len(), 5);
        assert_eq!((open[0], open[4]), ((103.0, 198.0), (103.0, 203.0)));
        let left = wing_poly(100.0, 200.0, -1.0, 20.0, 1.0, 0.0);
        assert_eq!(left[1].0 - 100.0, -(open[1].0 - 100.0));
    }
}
