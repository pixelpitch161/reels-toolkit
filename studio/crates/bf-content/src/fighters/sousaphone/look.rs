//! SOUSAPHONE's look: the port of `Look` in `projects/ballfight/kit/fighters/sousaphone.py`, hook by hook.
//!
//! What it draws: its own model with a wider crop (the padded sprite, in the ball and on the VS card); each drill
//! set's dashed chalk line on the canvas, a footprint and turf at every step, the ball bobbing on each footfall;
//! the bell blast as a cone of light on the canvas and, over the ring, a filled cone with sound-wave arcs rolling
//! out along it; turf and claw marks where it tramples, wicker chips where it tramples a basket; HORN FLASH's lit
//! lane sweeping round the ball; COMPANY FRONT's chalk front line and its row of seven translucent ghost
//! sousaphones, stepping on every beat; fiberglass bell pieces, brass tubing and valve caps when it loses; the
//! coil and the bell on its VS card panel.
//!
//! The arithmetic keeps Python's order and dtypes; the sines, cosines, arctangents and the power are
//! `bf_core::dmath`'s where Python has the C runtime's and numpy's.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::roster::{FighterCfg, WeaponCfg};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::BAYER;
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::{Img, RgbF};
use bf_picture::look::{CanvasCtx, FrameCtx, Fx, Look, Phase, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::BallState;
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::sprite::Mesh;
use bf_picture::title::layout;

use super::state::{self, Blast, Front, Set};
use crate::fighters::shared::padded::padded_sprite;
use crate::fighters::shared::reeds::{self, INK, Num, WHITE};

/// `CHALK`.
const CHALK: V3 = [238.0, 236.0, 226.0];
/// `TURF`.
const TURF: [V3; 3] = [[120.0, 150.0, 90.0], [90.0, 120.0, 70.0], [190.0, 180.0, 150.0]];
/// `FIBER`: the fiberglass bell.
const FIBER: V3 = [240.0, 238.0, 230.0];
/// `BRASS`.
const BRASS: V3 = [226.0, 180.0, 84.0];
/// `NICK`: the nickel valve caps.
const NICK: V3 = [214.0, 216.0, 226.0];
/// `PAD`: the sprite's crop margin (the bridge's x1.12 size).
pub const PAD: f64 = 1.22;
/// The ghosts of COMPANY FRONT's wall (`n = 7`, and the 7 dust puffs of `fx_setup`).
const GHOSTS: usize = 7;
/// The SOUSAPHONE's view in the kit's roster (yaw centre, yaw swing, roll), for a look that was never set up (the
/// VS card's close-up is asked of one).
const VIEW: (f64, f64, f64) = (20.0, 14.0, 0.0);

/// The SOUSAPHONE's model (`SP._geo(self.cfg['pp'])`), read once.
fn mesh() -> &'static Mesh {
    static MODEL: OnceLock<Mesh> = OnceLock::new();
    MODEL.get_or_init(|| {
        let json = super::MESH.expect("the SOUSAPHONE's model is compiled in");
        Mesh::from_json(json).expect("the SOUSAPHONE's model reads")
    })
}

/// `np.array([math.cos(a), math.sin(a)])`.
fn unit(a: f64) -> [f64; 2] {
    [dmath::cos(a), dmath::sin(a)]
}

/// A numpy boolean as a factor: `x * (a > b)`.
fn gate(on: bool) -> f64 {
    if on { 1.0 } else { 0.0 }
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

/// `-S.A + (j + 0.5) * 2 * S.A / n`: where ghost `j` of the wall's seven stands across the ring.
fn across(j: usize) -> f64 {
    -scene::A + (j as f64 + 0.5) * 2.0 * scene::A / GHOSTS as f64
}

/// `title_pattern(base_)`: the VS card's panel: the coiled wrap (rings round a centre) and the huge bell (a bright
/// disc with a dark throat), repeated along the divider. `base` is the panel's gradient, changed in place.
pub fn title_pattern(base: &mut RgbF) {
    let dd = layout::dd();
    let side = layout::side();
    let da = layout::DA;
    // np.array([60, 20, 26]) * 0.65 and np.array([255, 236, 230]) * 0.7
    let coil_col = [60.0 * 0.65, 20.0 * 0.65, 26.0 * 0.65];
    let bell_col = [255.0 * 0.7, 236.0 * 0.7, 230.0 * 0.7];
    for y in 0..base.h {
        for x in 0..base.w {
            // u = (XX - TT.DA[0]) * TT.DD[0] + (YY - TT.DA[1]) * TT.DD[1]
            let u = (x as f64 - da[0]) * dd[0] + (y as f64 - da[1]) * dd[1];
            let cu = py::fmod_py(u, 220.0) - 110.0;
            let cv = py::fmod_py(side.px(x, y)[0].abs(), 220.0) - 110.0;
            let r = dmath::np_hypot(cu, cv);
            let coil = (r - 70.0).abs() < 9.0 || (r - 50.0).abs() < 5.0;
            let rb = dmath::np_hypot(cu - 26.0, cv + 30.0);
            // (np.hypot(..) < 40) & ~(np.hypot(..) < 18)
            let bell = (18.0..40.0).contains(&rb);
            for (ch, v) in base.px_mut(x, y).iter_mut().enumerate() {
                let out = if coil { *v * 0.35 + coil_col[ch] } else { *v };
                *v = if bell { out * 0.3 + bell_col[ch] } else { out };
            }
        }
    }
}

/// `_ghost(spr, tint, alpha)` and `_paste(arr, rgb, a, cx, cy)` in one pass: the sprite as a translucent ghost in
/// the fighter's colour, pasted with its centre at `(cx, cy)` (the part of it on the frame).
fn paste_ghost(arr: &mut Img<f32>, spr: &Img<u8>, tint: V3, alpha: f64, cx: f64, cy: f64) {
    let (w, h) = (spr.w as i64, spr.h as i64);
    // x0, y0 = int(round(cx - w / 2)), int(round(cy - h / 2))
    let x0 = py::round_half_even(cx - w as f64 / 2.0) as i64;
    let y0 = py::round_half_even(cy - h as f64 / 2.0) as i64;
    let (xa, ya) = (x0.max(0), y0.max(0));
    let (xb, yb) = ((arr.w as i64).min(x0 + w), (arr.h as i64).min(y0 + h));
    if xa >= xb || ya >= yb {
        return;
    }
    for y in ya..yb {
        for x in xa..xb {
            let s = spr.px((x - x0) as usize, (y - y0) as usize);
            let rgb = [f64::from(s[0]), f64::from(s[1]), f64::from(s[2])];
            // lum = rgb.mean(-1, keepdims=True) / 255.0 (whole numbers: any order of the sum is exact)
            let lum = (rgb[0] + rgb[1] + rgb[2]) / 3.0 / 255.0;
            // a = spr[..., 3:4].astype(float) / 255.0 * alpha
            let sa = f64::from(s[3]) / 255.0 * alpha;
            for (v, &tn) in arr.px_mut(x as usize, y as usize).iter_mut().zip(&tint) {
                // col = np.clip(tint * (0.45 + 0.75 * lum), 0, 255)
                let col = np::clip(tn * (0.45 + 0.75 * lum), 0.0, 255.0);
                // arr += (rgb - arr) * sa: in float64, stored as float32
                let a = f64::from(*v);
                *v = (a + (col - a) * sa) as f32;
            }
        }
    }
}

/// SOUSAPHONE's look: `Look` of `kit/fighters/sousaphone.py`.
#[derive(Debug)]
pub struct SousaphoneLook {
    /// Its side.
    k: usize,
    /// `T.T_FIGHT`.
    t_fight: f64,
    /// `T.BEAT`.
    beat: f64,
    /// `SETS`: its drill sets.
    sets: Vec<Set>,
    /// `STEPS`: its footfalls as `(t, x, y)`.
    steps: Vec<(f64, f64, f64)>,
    /// `BLASTS`: its bell blasts.
    blasts: Vec<Blast>,
    /// `TRAMPLES`: `(t, x, y)`.
    tramples: Vec<(f64, f64, f64)>,
    /// `BASKETS`: the baskets it trampled, `(t, x, y)`.
    baskets: Vec<(f64, f64, f64)>,
    /// `FRONT`: its COMPANY FRONTs.
    fronts: Vec<Front>,
    /// `FSTEPS`: the wall's steps as `(t, position)`.
    fsteps: Vec<(f64, f64)>,
    /// `_spr`: the ghost sprites drawn so far, by radius.
    spr: Mutex<BTreeMap<u32, Arc<Img<u8>>>>,
    /// `self.cfg['pp']`: the model's name.
    pp: String,
    /// `self.cfg['view']`: `(yaw centre, yaw swing, roll)`, `None` for an entry without one.
    view: Option<(f64, f64, f64)>,
    /// `float(w.get('spread', w['width']))`: the blast's far width; `None` for a weapon that is not a drill.
    spread: Option<f64>,
    /// `w['width']` as the roster has it (what `spread` falls back on).
    width: Option<f64>,
    /// `float(self.cfg['abilities']['unlock1'].get('width', 0.5))`: HORN FLASH's lane width.
    fl_w: f64,
}

impl SousaphoneLook {
    /// The look before a picture sets it up: its constants and its title-card hooks work (with the roster's
    /// view), and it has no sets.
    pub fn new() -> SousaphoneLook {
        SousaphoneLook {
            k: 0,
            t_fight: 0.0,
            beat: 0.5,
            sets: Vec::new(),
            steps: Vec::new(),
            blasts: Vec::new(),
            tramples: Vec::new(),
            baskets: Vec::new(),
            fronts: Vec::new(),
            fsteps: Vec::new(),
            spr: Mutex::new(BTreeMap::new()),
            pp: String::new(),
            view: Some(VIEW),
            spread: None,
            width: None,
            fl_w: 0.5,
        }
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

    /// `ghost_sprite(r)`: its model at the card's view, `max(6, int(r))` pixels, kept by radius.
    fn ghost_sprite(&self, r: f64) -> Arc<Img<u8>> {
        let r = (r as i64).max(6) as u32;
        let (yc, roll) = self.yaw_roll();
        let mut cache = self.spr.lock().unwrap_or_else(|e| e.into_inner());
        Arc::clone(
            cache
                .entry(r)
                .or_insert_with(|| Arc::new(padded_sprite(mesh(), r, yc, roll, PAD))),
        )
    }

    /// `ta(t)`: the time, held at the K.O.
    fn ta(&self, p: &PictureCtx, t: f64) -> f64 {
        py_min(t, p.t_ko)
    }

    /// `row(t)`: its row of `RUN['DRILL']` at `t`, during the fight.
    fn row(&self, p: &PictureCtx, t: f64) -> Option<Vec<f64>> {
        if !p.run.has("DRILL") || !(self.t_fight <= t && t < p.t_ko) {
            return None;
        }
        p.run.row("DRILL", p.sim_i(t), self.k)
    }

    /// `set_at(t)`: the set on the canvas at `t`: from its start to 0.45 s after its blast (or last footfall).
    fn set_at(&self, t: f64) -> Option<&Set> {
        self.sets.iter().find(|s| s.t0 <= t && t < s.end() + 0.45)
    }
}

impl Default for SousaphoneLook {
    fn default() -> SousaphoneLook {
        SousaphoneLook::new()
    }
}

impl Look for SousaphoneLook {
    fn shards(&self) -> Vec<String> {
        // ['bell'] * 5 + ['tube'] * 4 + ['valve'] * 3
        [("bell", 5), ("tube", 4), ("valve", 3)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["blast", "trample", "front", "flash"]
    }

    fn spark_flash(&self) -> &[&str] {
        &["blast"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["blast", "front"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["blast", "front", "flash"]
    }

    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[("company_front", 1.0), ("flash_spin", 0.5)]
    }

    fn sprite_k(&self) -> f64 {
        0.86
    }

    fn title_pattern(&self, base: &mut RgbF) {
        title_pattern(base);
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// What `setup_as` took from `cfg`, by the keys `sousaphone.py` reads from `self.cfg`.
    fn cfg_summary(&self) -> Option<String> {
        Some(format!(
            "abilities.unlock1.width={:?}; pp={}; view={:?}; weapon.spread={:?}; weapon.width={:?}",
            self.fl_w, self.pp, self.view, self.spread, self.width
        ))
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`: the model's name, the view, the blast's far width and
    /// HORN FLASH's lane width come from `cfg`, the events from side `k`. For its own side `cfg` is `p.cfg[k]`,
    /// and this is `setup`. (The model drawn is the SOUSAPHONE's: the kit only ever makes this look with the
    /// SOUSAPHONE's own entry, which names it.)
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.t_fight = p.show.grid.t_fight;
        self.beat = p.show.grid.beat;
        let ev = &p.rec.events;
        let mine = |kind: &'static str| ev.iter().filter(move |e| e.kind == kind && e.who() == Some(k));
        self.sets = state::sets(ev, k);
        self.steps = mine("drill_step").map(|e| (e.t, e.f(4), e.f(5))).collect();
        self.blasts = state::blasts(ev, k);
        self.tramples = mine("drill_trample").map(|e| (e.t, e.f(3), e.f(4))).collect();
        self.baskets = mine("trample_basket").map(|e| (e.t, e.f(4), e.f(5))).collect();
        self.fronts = state::fronts(ev, k);
        self.fsteps = mine("front_step").map(|e| (e.t, e.f(4))).collect();
        self.spr = Mutex::new(BTreeMap::new());
        self.pp = cfg.pp.clone();
        let num = |i: usize| cfg.raw.get("view").and_then(|v| v.get(i)).and_then(|x| x.as_f64());
        self.view = match (num(0), num(1), num(2)) {
            (Some(yc), Some(sw), Some(roll)) => Some((yc, sw, roll)),
            _ => None,
        };
        // w = self.cfg['weapon']; self.spread = float(w.get('spread', w['width']))
        (self.spread, self.width) = match &cfg.weapon {
            WeaponCfg::Drill(w) => (Some(w.spread()), Some(w.width)),
            _ => (None, None),
        };
        self.fl_w = cfg.unlock1.get("width").unwrap_or(0.5);
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
        let Some(rw) = self.row(p, t) else { return };
        if rw[0] == 1.0 {
            // marching: a bob on every footfall
            if let Some(s) = self.set_at(t) {
                let st16 = (s.t1 - s.t0) / s.n.max(1) as f64;
                let fr = py::fmod_py((t - s.t0) / st16, 1.0);
                st.lift = 0.07 * dmath::sin(dmath::PI * fr);
            }
        }
        if !rw[4].is_nan() {
            // the blast: the ball glows from the bell
            st.glow = py_max(st.glow, 0.55 * (1.0 - rw[4]));
        }
        if !rw[7].is_nan() || !rw[9].is_nan() {
            st.glow = py_max(st.glow, 0.3);
        }
    }

    // ------------------------------------------------------------ events -> camera, ropes, sparks
    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match e.kind.as_str() {
            "blast" => Some(1.1),
            "drill_trample" => Some(1.3),
            "front_step" => Some(0.5),
            "front_hit" => Some(2.0),
            "drill_step" => Some(0.15),
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
            "front_step" => Some(0.25),
            _ => None,
        }
    }

    fn spark(&self, _p: &PictureCtx, e: &Event) -> Spark {
        match how(e) {
            "blast" | "flash" => ([255.0, 220.0, 210.0], 18, 3.6, 0.32),
            "trample" | "front" => ([200.0, 190.0, 170.0], 14, 2.8, 0.34),
            _ => ([230.0, 160.0, 170.0], 6, 1.8, 0.2),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "blast" | "front" => Some((16.0, 0.45)),
            "trample" | "flash" => Some((12.0, 0.35)),
            _ => None,
        }
    }

    // ------------------------------------------------------------ effects
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let col = p.fl_col[self.k];
        let turf = || Some(TURF.to_vec());
        // seed=int(t0 * k) % m
        let seed = |t0: f64, k: f64, m: i64| py::imod_py(np::f64_to_i64(t0 * k), m) as u64;
        // a footprint per step, alternating feet
        for &(t0, x, y) in &self.steps {
            if t0 > p.t_ko {
                break;
            }
            fx.mk.add(
                t0,
                Mark {
                    size: 0.18,
                    life: 2.2,
                    seed: seed(t0, 97.0, 991),
                    strength: 0.35,
                    col: [214.0, 206.0, 190.0],
                    ..Mark::new(MarkKind::Dust, x, y)
                },
            );
            let emit = Emit {
                n: Some(2),
                col: turf(),
                ..Emit::default()
            };
            fx.pa.emit(t0, up(x, y, 0.05), "dust", &emit);
        }
        for b in &self.blasts {
            if b.t0 > p.t_ko {
                break;
            }
            let pos = up(b.ox, b.oy, R);
            fx.fl.add(b.t0, pos, col, 2.6, 1.0, 0.03, 0.3, 0.0);
            fx.bu.add(b.t0, pos, [255.0, 236.0, 230.0], 1.2, None, true);
            let far = up(
                b.ox + dmath::cos(b.a) * b.ln * 0.6,
                b.oy + dmath::sin(b.a) * b.ln * 0.6,
                R,
            );
            fx.fl.add(b.t0 + 0.04, far, col, 2.0, 0.6, 0.03, 0.25, 0.0);
        }
        for &(t0, x, y) in &self.tramples {
            if t0 > p.t_ko {
                break;
            }
            let emit = Emit {
                n: Some(9),
                col: turf(),
                speed: Some((0.5, 1.4)),
                ..Emit::default()
            };
            fx.pa.emit(t0, up(x, y, 0.1), "dust", &emit);
            fx.mk.add(
                t0,
                Mark {
                    size: 0.3,
                    life: 3.0,
                    seed: seed(t0, 31.0, 997),
                    strength: 0.4,
                    ..Mark::new(MarkKind::Claw, x, y)
                },
            );
        }
        // a trampled basket: wicker chips
        for &(t0, x, y) in &self.baskets {
            if t0 > p.t_ko {
                break;
            }
            let emit = Emit {
                n: Some(10),
                col: Some(vec![[196.0, 150.0, 92.0], [120.0, 84.0, 46.0], [230.0, 200.0, 140.0]]),
                ..Emit::default()
            };
            fx.pa.emit(t0, up(x, y, 0.12), "chips", &emit);
            fx.mk.add(
                t0,
                Mark {
                    size: 0.3,
                    life: 3.0,
                    seed: seed(t0, 41.0, 997),
                    strength: 0.5,
                    col: [196.0, 150.0, 92.0],
                    ..Mark::new(MarkKind::Dust, x, y)
                },
            );
        }
        // COMPANY FRONT: a line of dust at every step
        for &(t0, pos) in &self.fsteps {
            if t0 > p.t_ko {
                break;
            }
            if let Some(fr) = self.fronts.iter().find(|fr| fr.tf <= t0 && t0 <= fr.t1 + 0.01) {
                for j in 0..GHOSTS {
                    let v = across(j);
                    let (x, y) = if fr.ax == 0 { (pos, v) } else { (v, pos) };
                    let emit = Emit {
                        n: Some(2),
                        col: turf(),
                        ..Emit::default()
                    };
                    fx.pa.emit(t0, up(x, y, 0.05), "dust", &emit);
                }
            }
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.kind != "hit" || e.who() != Some(self.k) {
            return;
        }
        let pos = scene::to_world(e.f(5), e.f(6), R);
        match how(e) {
            "blast" => fx.bu.add(e.t, pos, [255.0, 236.0, 230.0], 1.0, None, true),
            "front" => fx.bu.add(e.t, pos, p.col[self.k], 1.4, None, true),
            _ => {}
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let sparks = Emit {
            n: Some(28),
            spread: Some(180.0),
            speed: Some((0.8, 2.4)),
            col: Some(vec![[255.0, 200.0, 200.0], [240.0, 238.0, 230.0]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
        let chips = Emit {
            n: Some(12),
            col: Some(vec![FIBER, BRASS, NICK]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "chips", &chips);
    }

    // ------------------------------------------------------------ the canvas: chalk lines, the cone, the sweep
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let k = self.k;
        let t = c.t;
        let col = p.col[k].map(|v| v / 255.0);
        let chalk = CHALK.map(|v| v / 255.0);
        let tc = self.ta(p, t);
        let set = if self.t_fight <= t && t < p.t_ko {
            self.set_at(tc)
        } else {
            None
        };
        if let Some(s) = set {
            // the drill line: a dashed chalk yard line
            let a = s.p0;
            let d = [s.p1[0] - a[0], s.p1[1] - a[1]];
            let len = dmath::np_hypot(d[0], d[1]) + 1e-9;
            let d = [d[0] / len, d[1] / len];
            let end = s.end();
            let fade = py_min(1.0, (tc - s.t0) / 0.15) * (1.0 - py_max(0.0, (tc - end) / 0.45));
            for (j, light) in ic.iter_mut().enumerate() {
                let (rx, rz) = (c.sxp[j] - a[0], c.szp[j] - a[1]);
                let along = rx * d[0] + rz * d[1];
                let off = (-rx * d[1] + rz * d[0]).abs();
                let dash = py::fmod_py(along / 0.22, 1.0) < 0.6;
                let lane = off < 0.055 && along > -0.1 && along < len + 0.1 && dash;
                let v = gate(lane) * 0.55 * fade;
                for ch in 0..3 {
                    light[ch] += v * chalk[ch];
                }
            }
        }
        let Some(rw) = self.row(p, t) else { return };
        if !rw[4].is_nan() {
            // the bell blast: a cone of light
            let o = [rw[12], rw[13]];
            let d = unit(rw[2]);
            let (ln, w0) = (rw[5], rw[6]);
            // (Python raises for a weapon without a width; the look is only made with the SOUSAPHONE's entry)
            let spread = self.spread.unwrap_or(w0);
            let far = ln * (0.4 + 0.6 * py_min(1.0, rw[4] * 3.0));
            let k_ = dmath::pow(1.0 - rw[4], 0.6);
            let tint = col.map(|v| v * 0.8 + 0.35);
            for (j, light) in ic.iter_mut().enumerate() {
                let (rx, rz) = (c.sxp[j] - o[0], c.szp[j] - o[1]);
                let along = rx * d[0] + rz * d[1];
                let off = (-rx * d[1] + rz * d[0]).abs();
                let hw = 0.5 * (w0 + (spread - w0) * np::clip(along / py_max(ln, 1e-6), 0.0, 1.0));
                let band = np::clip(1.0 - off / (hw + 0.15), 0.0, 1.0) * gate(along > 0.0) * gate(along < far);
                let band = (band * 4.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 4.0;
                let v = band * 0.9 * k_;
                for ch in 0..3 {
                    light[ch] += v * tint[ch];
                }
            }
        }
        if !rw[7].is_nan() {
            // HORN FLASH: a lane sweeping round
            let st = &c.fs[k];
            let d = unit(rw[7]);
            let half = self.fl_w * 0.5 + 0.12;
            let tint = col.map(|v| v * 0.8 + 0.3);
            for (j, light) in ic.iter_mut().enumerate() {
                let (rx, rz) = (c.sxp[j] - st.p[0], c.szp[j] - st.p[1]);
                let along = rx * d[0] + rz * d[1];
                let off = (-rx * d[1] + rz * d[0]).abs();
                let band = np::clip(1.0 - off / half, 0.0, 1.0) * gate(along > R * 0.8) * gate(along < R + rw[8]);
                let band = (band * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
                let v = band * 0.8;
                for ch in 0..3 {
                    light[ch] += v * tint[ch];
                }
            }
        }
        if !rw[9].is_nan() {
            // COMPANY FRONT: the chalk front line, lit behind
            let (ax, sg, pos) = (rw[10] as i64, rw[11], rw[9]);
            for (j, light) in ic.iter_mut().enumerate() {
                let c_ = if ax == 0 { c.sxp[j] } else { c.szp[j] };
                let rel = (c_ - pos) * sg;
                let line = gate(rel.abs() < 0.04);
                let glow = np::clip(1.0 + rel / 0.9, 0.0, 1.0) * gate(rel < 0.0);
                let glow = (glow * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
                for ch in 0..3 {
                    light[ch] += line * 0.5 * chalk[ch] + glow * 0.35 * col[ch];
                }
            }
        }
    }

    // ------------------------------------------------------------ over the ring: sound waves, the ghost line
    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        let (cam, k) = (f.cam, self.k);
        let t = f.t;
        let col = p.col[k];
        let tc = self.ta(p, t);
        if self.t_fight <= t {
            // the bell blasts: a filled cone + sound-wave arcs
            for b in &self.blasts {
                let age = tc - b.t0;
                if !(0.0..0.45).contains(&age) {
                    continue;
                }
                let kk = 1.0 - age / 0.45;
                let reach = b.ln * py_min(1.0, 0.35 + age / 0.08);
                let hw0 = 0.5 * b.w0;
                let hw1 = 0.5 * (b.w0 + (b.w1 - b.w0) * reach / py_max(b.ln, 1e-6));
                let d = unit(b.a);
                let nn = [-d[1], d[0]];
                let o = [b.ox, b.oy];
                let on_screen = |quad: &[[f64; 2]; 4]| -> Vec<(f64, f64)> {
                    let world: Vec<V3> = quad.iter().map(|q| scene::to_world(q[0], q[1], R * 0.9)).collect();
                    cam.wp(&world)
                };
                // quad = [o + nn * hw0, o + d * reach + nn * hw1, o + d * reach - nn * hw1, o - nn * hw0]
                let quad = [
                    [o[0] + nn[0] * hw0, o[1] + nn[1] * hw0],
                    [o[0] + d[0] * reach + nn[0] * hw1, o[1] + d[1] * reach + nn[1] * hw1],
                    [o[0] + d[0] * reach - nn[0] * hw1, o[1] + d[1] * reach - nn[1] * hw1],
                    [o[0] - nn[0] * hw0, o[1] - nn[1] * hw0],
                ];
                let cone = col.map(|v| np::clip(v * 0.55 + 140.0, 0.0, 255.0));
                reeds::blend(arr, &reeds::mask(&[on_screen(&quad)], &[], 1), cone, Num::Py(0.42 * kk));
                // core = [o + nn * hw0 * 0.4, o + d * reach * 0.8 + nn * hw1 * 0.3, ..., o - nn * hw0 * 0.4]
                let core = [
                    [o[0] + nn[0] * hw0 * 0.4, o[1] + nn[1] * hw0 * 0.4],
                    [
                        o[0] + d[0] * reach * 0.8 + nn[0] * hw1 * 0.3,
                        o[1] + d[1] * reach * 0.8 + nn[1] * hw1 * 0.3,
                    ],
                    [
                        o[0] + d[0] * reach * 0.8 - nn[0] * hw1 * 0.3,
                        o[1] + d[1] * reach * 0.8 - nn[1] * hw1 * 0.3,
                    ],
                    [o[0] - nn[0] * hw0 * 0.4, o[1] - nn[1] * hw0 * 0.4],
                ];
                reeds::blend(
                    arr,
                    &reeds::mask(&[on_screen(&core)], &[], 1),
                    WHITE,
                    Num::Py(0.45 * kk),
                );
                // arcs rolling out along the cone
                for j in 0..3 {
                    let s = py::fmod_py(age / 0.45 * 1.6 + f64::from(j) / 3.0, 1.0) * reach;
                    let hw = hw0 + (hw1 - hw0) * s / py_max(reach, 1e-6);
                    let ang = dmath::atan2(hw, py_max(s, 0.05));
                    let rr = dmath::py_hypot(s, hw);
                    let world: Vec<V3> = linspace(-ang, ang, 9)
                        .iter()
                        .map(|&th| {
                            scene::to_world(o[0] + rr * dmath::cos(b.a + th), o[1] + rr * dmath::sin(b.a + th), R)
                        })
                        .collect();
                    let spa = cam.wp(&world);
                    reeds::blend(
                        arr,
                        &reeds::mask(&[], std::slice::from_ref(&spa), 3),
                        INK,
                        Num::Py(0.4 * kk),
                    );
                    reeds::blend(arr, &reeds::mask(&[], &[spa], 1), WHITE, Num::Py(0.95 * kk));
                }
            }
        }
        let mut rw = self.row(p, t);
        let mut fade = 1.0;
        if rw.is_none() && p.t_ko <= t && t < p.t_ko + 0.6 && p.run.has("DRILL") {
            // the wall holds at the K.O., fading
            rw = p.run.row("DRILL", p.sim_i(p.t_ko - 1e-3), k);
            fade = 1.0 - (t - p.t_ko) / 0.6;
        }
        let Some(rw) = rw else { return };
        if !rw[9].is_nan() {
            // the ghost sousaphones, a step a beat
            let (ax, sg, pos) = (rw[10] as i64, rw[11], rw[9]);
            let bob = dmath::sin(dmath::PI * ((py_min(t, p.t_ko) - self.t_fight) / self.beat)).abs();
            let mut items: Vec<(f64, V3)> = (0..GHOSTS)
                .map(|j| {
                    let v = across(j);
                    let (x, y) = if ax == 0 {
                        (pos - sg * 0.1, v)
                    } else {
                        (v, pos - sg * 0.1)
                    };
                    let w = scene::to_world(x, y, R + 0.05 * bob);
                    (cam.depth(&[w])[0], w)
                })
                .collect();
            // sorted(items, key=lambda q: -q[0]): a stable sort, far to near
            items.sort_by(|x, y| (-x.0 + 0.0).total_cmp(&(-y.0 + 0.0)));
            for (dz, w) in items {
                let c = cam.wp(&[w])[0];
                let rp = R * cam.f / dz;
                let al = 0.72 * py_min(1.0, rw[15] * 8.0) * py_min(1.0, (1.0 - rw[15]) * 8.0) * fade;
                let spr = self.ghost_sprite(rp * 1.9);
                paste_ghost(arr, &spr, col, al, c.0, c.1);
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
            "tube" => {
                // a curved length of brass tubing
                let pts: Vec<[f64; 2]> = (0..6)
                    .map(|j| {
                        let s = f64::from(j) - 2.5;
                        // c + d * (j - 2.5) * 0.06 + nn * curl * 0.03 * (j - 2.5) ** 2
                        [
                            c[0] + d[0] * s * 0.06 + nn[0] * curl * 0.03 * (s * s),
                            c[1] + d[1] * s * 0.06 + nn[1] * curl * 0.03 * (s * s),
                        ]
                    })
                    .collect();
                let sp = on_screen(&pts);
                let dark = [90.0, 60.0, 20.0];
                reeds::blend(arr, &reeds::mask(&[], std::slice::from_ref(&sp), 3), dark, Num::Py(0.8));
                reeds::blend(arr, &reeds::mask(&[], &[sp], 2), BRASS, Num::Py(1.0));
            }
            "valve" => {
                let pts = [
                    [c[0] - d[0] * 0.05, c[1] - d[1] * 0.05],
                    [c[0] + d[0] * 0.05, c[1] + d[1] * 0.05],
                ];
                reeds::blend(arr, &reeds::mask(&[], &[on_screen(&pts)], 3), NICK, Num::Py(1.0));
            }
            _ => {
                // a curved shard of the fiberglass bell
                let sz = size * 1.8;
                let tri = [
                    [c[0] + d[0] * sz, c[1] + d[1] * sz],
                    [
                        c[0] - d[0] * sz * 0.6 + nn[0] * sz * 0.9,
                        c[1] - d[1] * sz * 0.6 + nn[1] * sz * 0.9,
                    ],
                    [
                        c[0] - d[0] * sz * 0.5 - nn[0] * sz * 0.6,
                        c[1] - d[1] * sz * 0.5 - nn[1] * sz * 0.6,
                    ],
                ];
                let sp = on_screen(&tri);
                let shade = 0.8 + 0.25 * dmath::cos(rot).abs();
                layer_blend(arr, std::slice::from_ref(&sp), FIBER.map(|v| v * shade), 1.0, None);
                // np.vstack([sp, sp[:1]]): the outline, closed
                let mut edge = sp;
                edge.push(edge[0]);
                reeds::blend(arr, &reeds::mask(&[], &[edge], 1), [170.0, 40.0, 50.0], Num::Py(0.8));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wall_has_seven_ghosts_evenly_across_the_ring() {
        let at: Vec<f64> = (0..GHOSTS).map(across).collect();
        assert_eq!(at[3], 0.0);
        assert!(at[0] > -scene::A && at[6] < scene::A);
        for j in 0..GHOSTS - 1 {
            assert!((at[j + 1] - at[j] - 2.0 * scene::A / 7.0).abs() < 1e-12);
        }
    }

    /// A ghost off the frame pastes nothing; one across the left edge pastes only its part on the frame, in the
    /// tint, as translucent as its alpha says.
    #[test]
    fn a_ghost_is_pasted_where_it_is_on_the_frame() {
        let spr: Img<u8> = Img::filled(5, 5, &[255, 255, 255, 255]);
        let mut arr: Img<f32> = Img::new(8, 8, 3);
        paste_ghost(&mut arr, &spr, [100.0, 50.0, 200.0], 0.5, -20.0, 4.0);
        assert!(arr.data.iter().all(|&v| v == 0.0));
        paste_ghost(&mut arr, &spr, [100.0, 50.0, 200.0], 0.5, 0.0, 4.0);
        // x0 = round(0 - 2.5) = -2 (half to even): columns 0 to 2; y0 = round(1.5) = 2: rows 2 to 6
        assert_eq!(arr.px(0, 2), &[60.0f32, 30.0, 120.0]);
        assert_eq!(arr.px(2, 6), &[60.0f32, 30.0, 120.0]);
        assert_eq!(arr.px(3, 4), &[0.0f32; 3]);
        assert_eq!(arr.px(0, 1), &[0.0f32; 3]);
        assert_eq!(arr.px(0, 7), &[0.0f32; 3]);
    }

    #[test]
    fn its_shatter_is_twelve_parts_of_three_kinds() {
        let parts = SousaphoneLook::new().shards();
        let count = |kind: &str| parts.iter().filter(|p| *p == kind).count();
        assert_eq!(
            (parts.len(), count("bell"), count("tube"), count("valve")),
            (12, 5, 4, 3)
        );
    }
}
