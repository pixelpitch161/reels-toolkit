//! BASS DRUM's look: the port of `Look` in `projects/ballfight/kit/fighters/bass_drum.py`, hook by hook.
//!
//! What it draws: its own model with a wider crop (the padded sprite, in the ball and on the VS card). As it tips it
//! squashes and keels over; rolling, it turns to face the camera head-on and spins like a wheel (the model without
//! its mallet), drawn bigger as it grows. The line is chalked in dust before it goes (a dashed trail fading in while
//! it tips); a tread of churned canvas and dust puffs follow it; each BOOM is a flash, a burst, a dust ring and a
//! crack in the canvas, with sound rings over the ring. HEARTBEAT is two thick rings on the canvas; 1812 OVERTURE
//! marks each cannon's landing spot with a pulsing target, then a smoke-and-scorch blast. It shatters into pieces of
//! white head, dark hoop and chrome rods. VS card: the drum head's rings and the tension rods.
//!
//! The arithmetic keeps Python's order and dtypes; the sines, cosines, arctangents and hypotenuses are
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
use bf_picture::look::{BallSprite, CanvasCtx, FrameCtx, Fx, Look, Phase, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::sprite::Mesh;
use bf_picture::title::layout;

use super::state::{self, Seg};
use crate::fighters::shared::padded::padded_sprite;
use crate::fighters::shared::reeds::{self, INK, Num, WHITE};

/// `DUST`: the dust puffs' colours.
const DUST: [V3; 3] = [[176.0, 156.0, 126.0], [140.0, 122.0, 98.0], [214.0, 200.0, 172.0]];
/// `DUSTC`: the chalked line and the tread's edge.
const DUSTC: V3 = [196.0, 178.0, 146.0];
/// `HEADC`: the white head.
const HEADC: V3 = [238.0, 236.0, 228.0];
/// `HOOPC`: the dark hoop.
const HOOPC: V3 = [38.0, 38.0, 46.0];
/// `CHROME`: the tension rods.
const CHROME: V3 = [214.0, 218.0, 228.0];
/// `SMOKE`: a cannon's smoke.
const SMOKE: [V3; 3] = [[120.0, 118.0, 124.0], [90.0, 88.0, 96.0], [170.0, 166.0, 170.0]];
/// `PAD`: its sprite's crop margin (the bridge's x1.1 size, plus growth).
pub const PAD: f64 = 1.18;
/// `ROLL_Q`: the rolling sprite's spin in steps of this many degrees.
const ROLL_Q: i64 = 30;
/// The faces of the model without its mallet (`_nomallet_geo`): the first faces of its model file, to the bit (the
/// mallet is the last thing `bassdrum_lowpoly.drum` builds). `studio/reference/fighters/bass_drum/variant.json`
/// holds the export's proof and this count.
pub const NOMALLET_FACES: usize = 382;
/// The BASS DRUM's view in the kit's roster (yaw centre, yaw swing, roll), for a look that was never set up (the
/// VS card's close-up is asked of one).
const VIEW: (f64, f64, f64) = (0.0, 16.0, 0.0);

/// The BASS DRUM's model (`SP._geo(self.cfg['pp'])`), read once.
fn mesh() -> &'static Mesh {
    static MODEL: OnceLock<Mesh> = OnceLock::new();
    MODEL.get_or_init(|| {
        let json = super::MESH.expect("the BASS DRUM's model is compiled in");
        Mesh::from_json(json).expect("the BASS DRUM's model reads")
    })
}

/// `_nomallet_geo()`: the drum without its mallet, centred and scaled like the full model.
pub fn nomallet() -> &'static Mesh {
    static MODEL: OnceLock<Mesh> = OnceLock::new();
    MODEL.get_or_init(|| {
        let full = mesh();
        assert!(
            full.faces.len() > NOMALLET_FACES,
            "the model has its mallet after the drum"
        );
        Mesh {
            faces: full.faces[..NOMALLET_FACES].to_vec(),
        }
    })
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

/// `math.degrees(x)`: CPython multiplies by the constant `180 / pi`.
fn degrees(x: f64) -> f64 {
    x * (180.0 / dmath::PI)
}

/// `math.radians(x)`: CPython multiplies by the constant `pi / 180`.
fn radians(x: f64) -> f64 {
    x * (dmath::PI / 180.0)
}

/// `e[7]` of a hit (what hit), or `""` for an event without one.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// A number of a row of `RUN['SLAMS'][i]`.
fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Float(x) => Some(*x),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

/// One row of `RUN['SLAMS'][i]`: a cannon (or another slam) in the air or just landed.
#[derive(Clone, Copy, Debug)]
struct SlamRow {
    owner: i64,
    cx: f64,
    cz: f64,
    t0: f64,
    t_slam: f64,
    rad: f64,
}

/// `RUN['SLAMS'][i]`: the rows of recording frame `i` (none when the recording has no `SLAMS`; a row that is not
/// `[owner, x, z, start, slam, radius]` is left out where Python raises).
fn slams(p: &PictureCtx, i: usize) -> Vec<SlamRow> {
    let Some(Value::List(rows)) = p.run.nested("SLAMS", i) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            let Value::List(v) = row else { return None };
            let Value::Int(owner) = *v.first()? else { return None };
            Some(SlamRow {
                owner,
                cx: num(v.get(1)?)?,
                cz: num(v.get(2)?)?,
                t0: num(v.get(3)?)?,
                t_slam: num(v.get(4)?)?,
                rad: num(v.get(5)?)?,
            })
        })
        .collect()
}

/// `title_pattern(base_)`: the VS card's panel: a drum head's rings (a white disc with a dark hoop) and tension rods
/// radiating round it, repeated along the divider. `base` is the panel's gradient, changed in place.
pub fn title_pattern(base: &mut RgbF) {
    let dd = layout::dd();
    let side = layout::side();
    let da = layout::DA;
    // np.array([236, 232, 222]) * 0.55, np.array([30, 30, 40]) * 0.7, np.array([220, 224, 236]) * 0.7
    let head_col = [236.0 * 0.55, 232.0 * 0.55, 222.0 * 0.55];
    let hoop_col = [30.0 * 0.7, 30.0 * 0.7, 40.0 * 0.7];
    let rod_col = [220.0 * 0.7, 224.0 * 0.7, 236.0 * 0.7];
    for y in 0..base.h {
        for x in 0..base.w {
            // u = (XX - TT.DA[0]) * TT.DD[0] + (YY - TT.DA[1]) * TT.DD[1]
            let u = (x as f64 - da[0]) * dd[0] + (y as f64 - da[1]) * dd[1];
            let cu = py::fmod_py(u, 240.0) - 120.0;
            let cv = py::fmod_py(side.px(x, y)[0].abs(), 240.0) - 120.0;
            let r = dmath::np_hypot(cu, cv);
            let a = dmath::atan2(cv, cu);
            let head = r < 62.0;
            let hoop = (62.0..74.0).contains(&r);
            let rods = (74.0..96.0).contains(&r)
                && (py::fmod_py(a / (2.0 * dmath::PI) * 10.0, 1.0) - 0.5).abs() < 0.09;
            for (ch, v) in base.px_mut(x, y).iter_mut().enumerate() {
                let b = *v;
                let out = if head { b * 0.45 + head_col[ch] } else { b };
                let out = if hoop { b * 0.3 + hoop_col[ch] } else { out };
                *v = if rods { b * 0.3 + rod_col[ch] } else { out };
            }
        }
    }
}

/// The key of a drawn sprite: rolling (no mallet) or not, the radius, the yaw and the roll to the bit.
type SpriteKey = (bool, u32, u64, u64);

/// BASS DRUM's look: `Look` of `kit/fighters/bass_drum.py`.
#[derive(Debug)]
pub struct BassDrumLook {
    /// Its side.
    k: usize,
    /// `T.T_FIGHT`.
    t_fight: f64,
    /// `SEGS`: its rolls.
    segs: Vec<Seg>,
    /// `BOOMS`: `(t, x, y, kind)`.
    booms: Vec<(f64, f64, f64, i64)>,
    /// `BEATS`: `(t, i, x, y)`.
    beats: Vec<(f64, i64, f64, f64)>,
    /// `CANNONS`: `(t, i, x, y, t_land)`.
    cannons: Vec<(f64, i64, f64, f64, f64)>,
    /// `_spr`: the sprites drawn so far.
    spr: Mutex<BTreeMap<SpriteKey, Arc<Img<u8>>>>,
    /// `self.cfg['pp']`: the model's name.
    pp: String,
    /// `self.cfg['view']`: `(yaw centre, yaw swing, roll)`, `None` for an entry without one.
    view: Option<(f64, f64, f64)>,
    /// `self.cfg['weapon'].get('size_max')`: `None` when absent (the look takes 0.25 then) or the weapon is not a
    /// roll_charge.
    size_max: Option<f64>,
}

impl BassDrumLook {
    /// The look before a picture sets it up: its constants and its title-card hooks work (with the roster's
    /// view), and it has no rolls.
    pub fn new() -> BassDrumLook {
        BassDrumLook {
            k: 0,
            t_fight: 0.0,
            segs: Vec::new(),
            booms: Vec::new(),
            beats: Vec::new(),
            cannons: Vec::new(),
            spr: Mutex::new(BTreeMap::new()),
            pp: String::new(),
            view: Some(VIEW),
            size_max: Some(0.25),
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

    /// `sprite(r, yaw, roll)`, kept by its arguments: a roll of 1000 or more is the rolling drum (no mallet, at yaw
    /// 10, turned by `roll - 1000`), else the model at `yaw` and `roll`.
    fn drawn(&self, r: u32, yaw: f64, roll: f64) -> Arc<Img<u8>> {
        let rolling = roll >= 1000.0;
        let key = (rolling, r, yaw.to_bits(), roll.to_bits());
        let mut cache = self.spr.lock().unwrap_or_else(|e| e.into_inner());
        Arc::clone(cache.entry(key).or_insert_with(|| {
            Arc::new(if rolling {
                padded_sprite(nomallet(), r, 10.0, roll - 1000.0, PAD)
            } else {
                padded_sprite(mesh(), r, yaw, roll, PAD)
            })
        }))
    }

    /// `ta(t)`: the time, held at the K.O.
    fn ta(&self, p: &PictureCtx, t: f64) -> f64 {
        py_min(t, p.t_ko)
    }

    /// `row(t)`: its row of `RUN['RCHG']` at `t`, during the fight.
    fn row(&self, p: &PictureCtx, t: f64) -> Option<Vec<f64>> {
        if !p.run.has("RCHG") || !(self.t_fight <= t && t < p.t_ko) {
            return None;
        }
        p.run.row("RCHG", p.sim_i(t), self.k)
    }

    /// `size_k(t)`: how much bigger the drum is drawn as it grows: 1 + 0.16 x its size over `size_max`.
    fn size_k(&self, p: &PictureCtx, t: f64) -> f64 {
        let Some(rw) = self.row(p, t) else { return 1.0 };
        if rw[4].is_nan() {
            return 1.0;
        }
        let smax = py_max(1e-6, self.size_max.unwrap_or(0.25));
        1.0 + 0.16 * rw[4] / smax
    }
}

impl Default for BassDrumLook {
    fn default() -> BassDrumLook {
        BassDrumLook::new()
    }
}

impl Look for BassDrumLook {
    fn shards(&self) -> Vec<String> {
        // ['head'] * 6 + ['hoop'] * 4 + ['rod'] * 4
        [("head", 6), ("hoop", 4), ("rod", 4)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["roll", "drumroll", "cannon", "shockwave"]
    }

    fn spark_flash(&self) -> &[&str] {
        &["roll"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["roll", "cannon"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["roll", "cannon"]
    }

    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[("heartbeat", 0.7), ("drum_roll", 0.4), ("cannons", 0.8)]
    }

    fn sprite_k(&self) -> f64 {
        0.80
    }

    fn title_pattern(&self, base: &mut RgbF) {
        title_pattern(base);
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// What `setup_as` took from `cfg`, by the keys `bass_drum.py` reads from `self.cfg`.
    fn cfg_summary(&self) -> Option<String> {
        Some(format!(
            "pp={}; view={:?}; weapon.size_max={:?}",
            self.pp, self.view, self.size_max
        ))
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`: the model's name, the view and the drum's largest size
    /// come from `cfg`, the events from side `k`. For its own side `cfg` is `p.cfg[k]`, and this is `setup`. (The
    /// model drawn is the BASS DRUM's: the kit only ever makes this look with the BASS DRUM's own entry, which names
    /// it.)
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.t_fight = p.show.grid.t_fight;
        let ev = &p.rec.events;
        self.segs = state::segs(ev, k);
        self.booms = state::booms(ev, k);
        self.beats = state::beats(ev, k);
        self.cannons = state::cannons(ev, k);
        self.spr = Mutex::new(BTreeMap::new());
        self.pp = cfg.pp.clone();
        let num = |i: usize| cfg.raw.get("view").and_then(|v| v.get(i)).and_then(|x| x.as_f64());
        self.view = match (num(0), num(1), num(2)) {
            (Some(yc), Some(sw), Some(roll)) => Some((yc, sw, roll)),
            _ => None,
        };
        self.size_max = match &cfg.weapon {
            WeaponCfg::RollCharge(w) => w.size_max,
            _ => None,
        };
    }

    fn sprite(&self, r: u32, yaw: f64, roll: f64) -> Option<Img<u8>> {
        Some((*self.drawn(r, yaw, roll)).clone())
    }

    fn title_closeup(&self, r: u32) -> Option<Img<u8>> {
        let (yc, roll) = self.yaw_roll();
        Some((*self.drawn(r, yc, roll)).clone())
    }

    /// `ball_sprite(kk, st, t, rp, yaw)`: its own ball is always drawn by the look: rolling, the wheel at its spin;
    /// tipping, keeling over in 15-degree steps; else upright. Drawn bigger as it grows.
    fn ball_sprite<'a>(
        &'a self,
        p: &'a PictureCtx,
        kk: usize,
        st: &BallState,
        t: f64,
        rp: f64,
        yaw: f64,
    ) -> Option<BallSprite<'a>> {
        if kk != self.k || st.shattered {
            return None;
        }
        let sk = self.size_k(p, t);
        // int(np.clip(round(rp * self.sprite_k * sk), 8, 40))
        let r = py::round_half_even(rp * self.sprite_k() * sk).clamp(8.0, 40.0) as u32;
        let yk = (py::round_half_even(yaw / 6.0) as i64 * 6) as f64;
        let sprite = if let Some(&rc) = st.extra.get("drum_roll") {
            // rolling
            self.drawn(r, 0.0, 1000.0 + rc)
        } else {
            let tip = st.extra.get("drum_tip").copied().unwrap_or(0.0);
            if tip > 0.0 {
                // keeling over onto its side
                let roll = py::round_half_even(tip * 90.0 / 15.0) as i64 * 15;
                self.drawn(r, yk, roll as f64)
            } else {
                self.drawn(r, yk, 0.0)
            }
        };
        Some(BallSprite { sprite, tint: None })
    }

    // ------------------------------------------------------------ state
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if kk != self.k || phase != Phase::Fight {
            return;
        }
        let Some(rw) = self.row(p, t) else { return };
        if rw[0] == 1.0 {
            // tipping: squash and keel over
            let u = np::clip(rw[1], 0.0, 1.0);
            st.extra.insert("drum_tip".to_string(), u);
            st.sx = Some(1.0 + 0.10 * dmath::sin(dmath::PI * u));
            st.sy = Some(1.0 - 0.10 * dmath::sin(dmath::PI * u));
        } else if rw[0] == 2.0 || rw[6] == 1.0 {
            // rolling (or between DRUM ROLL legs)
            let ang = degrees(rw[3]);
            // spin direction: the way the wheel turns as seen head-on (rolling screen-right turns it clockwise)
            let ca = if rw[2].is_nan() { 1.0 } else { dmath::cos(rw[2]) };
            let spin = py::fmod_py(if ca >= 0.0 { -ang } else { ang }, 360.0);
            let q = py::round_half_even(spin / ROLL_Q as f64) as i64 * ROLL_Q;
            st.extra.insert("drum_roll".to_string(), py::imod_py(q, 360) as f64);
            st.lift += 0.012 * dmath::sin(radians(ang * 2.0)).abs();
        }
        for &(t0, _x, _y, _kind) in &self.booms {
            // a BOOM: the ball flashes
            if 0.0 <= t - t0 && t - t0 < 0.3 {
                st.glow = py_max(st.glow, 0.7 * (1.0 - (t - t0) / 0.3));
            }
        }
        for &(t0, _i, _x, _y) in &self.beats {
            if 0.0 <= t - t0 && t - t0 < 0.22 {
                st.glow = py_max(st.glow, 0.5 * (1.0 - (t - t0) / 0.22));
                st.scale *= 1.0 + 0.06 * (1.0 - (t - t0) / 0.22);
            }
        }
        // every glow and scale here is a Python float (unit_checks/glow_types.py: 1,830 frames of the left fight)
        st.glow_np = false;
    }

    // ------------------------------------------------------------ events -> camera, ropes, sparks
    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match e.kind.as_str() {
            "roll_boom" => Some(if e.i(5) == 0 { 1.0 } else { 0.5 }),
            "heartbeat" => Some(0.55),
            // a cannon landing
            "slam" => Some(1.1),
            "roll" => Some(0.15),
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
            "slam" => Some(0.6),
            _ => None,
        }
    }

    fn spark(&self, _p: &PictureCtx, e: &Event) -> Spark {
        match how(e) {
            "roll" | "cannon" => ([240.0, 236.0, 230.0], 18, 3.4, 0.32),
            "drumroll" => ([210.0, 200.0, 180.0], 9, 2.4, 0.24),
            "shockwave" => ([200.0, 200.0, 214.0], 10, 2.6, 0.26),
            _ => ([200.0, 196.0, 200.0], 6, 1.8, 0.2),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "roll" | "cannon" => Some((16.0, 0.45)),
            "drumroll" | "shockwave" => Some((11.0, 0.3)),
            _ => None,
        }
    }

    // ------------------------------------------------------------ effects
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let col = p.fl_col[self.k];
        let flare = col.map(|v| v * 0.6 + 100.0);
        let dust = || Some(DUST.to_vec());
        // seed=int(t * k) % m
        let seed = |t0: f64, k: f64, m: i64| py::imod_py(np::f64_to_i64(t0 * k), m) as u64;
        // dust puffs along every roll
        for s in &self.segs {
            if s.t0 > p.t_ko {
                break;
            }
            let n = ((s.t_end - s.t0) / 0.05) as i64;
            for j in 0..n.max(0) {
                let tj = s.t0 + j as f64 * 0.05;
                if tj > p.t_ko {
                    break;
                }
                let u = (tj - s.t0) / py_max(1e-9, s.t1 - s.t0);
                let q = [s.p0[0] + (s.p1[0] - s.p0[0]) * u, s.p0[1] + (s.p1[1] - s.p0[1]) * u];
                let emit = Emit {
                    n: Some(2),
                    col: dust(),
                    ..Emit::default()
                };
                fx.pa.emit(tj, up(q[0], q[1], 0.05), "dust", &emit);
                if j % 3 == 0 {
                    fx.mk.add(
                        tj,
                        Mark {
                            size: 0.26,
                            life: 2.0,
                            seed: seed(tj, 97.0, 991),
                            strength: 0.3,
                            col: [186.0, 168.0, 138.0],
                            ..Mark::new(MarkKind::Dust, q[0], q[1])
                        },
                    );
                }
            }
        }
        for &(t0, x, y, kind) in &self.booms {
            if t0 > p.t_ko {
                break;
            }
            let pos = up(x, y, R * 0.8);
            let big = kind == 0;
            let (radius, peak) = if big { (2.8, 1.1) } else { (2.0, 0.6) };
            fx.fl.add(t0, pos, flare, radius, peak, 0.03, 0.3, 0.0);
            fx.bu.add(t0, pos, [246.0, 244.0, 236.0], if big { 1.3 } else { 0.8 }, None, true);
            let emit = Emit {
                n: Some(if big { 14 } else { 6 }),
                col: dust(),
                speed: Some((0.6, 1.8)),
                ..Emit::default()
            };
            fx.pa.emit(t0, up(x, y, 0.08), "dust", &emit);
            if big {
                fx.mk.add(
                    t0,
                    Mark {
                        size: 0.4,
                        life: 4.0,
                        seed: seed(t0, 31.0, 997),
                        strength: 0.45,
                        ..Mark::new(MarkKind::Crack, x, y)
                    },
                );
            }
        }
        for &(t0, _i, x, y) in &self.beats {
            if t0 > p.t_ko {
                break;
            }
            fx.fl.add(t0, up(x, y, R), flare, 2.4, 0.8, 0.03, 0.22, 0.0);
            let emit = Emit {
                n: Some(8),
                col: dust(),
                speed: Some((0.8, 1.6)),
                ..Emit::default()
            };
            fx.pa.emit(t0, up(x, y, 0.06), "dust", &emit);
        }
        // the cannon landings (they come as 'slam' events too)
        for &(_t0, _i, x, y, t_land) in &self.cannons {
            if t_land > p.t_ko {
                break;
            }
            let pos = up(x, y, 0.2);
            fx.fl.add(t_land, pos, [255.0, 196.0, 120.0], 2.6, 1.1, 0.03, 0.32, 0.0);
            fx.bu.add(t_land, pos, [255.0, 232.0, 196.0], 1.3, None, true);
            let smoke = Emit {
                n: Some(10),
                col: Some(SMOKE.to_vec()),
                ..Emit::default()
            };
            fx.pa.emit(t_land, up(x, y, 0.1), "smoke", &smoke);
            let sparks = Emit {
                n: Some(12),
                col: Some(vec![[255.0, 210.0, 140.0], [255.0, 240.0, 210.0]]),
                ..Emit::default()
            };
            fx.pa.emit(t_land, up(x, y, 0.1), "sparks", &sparks);
            fx.mk.add(
                t_land,
                Mark {
                    size: 0.6,
                    life: 5.0,
                    seed: seed(t_land, 53.0, 991),
                    strength: 0.55,
                    ..Mark::new(MarkKind::Scorch, x, y)
                },
            );
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let sparks = Emit {
            n: Some(28),
            spread: Some(180.0),
            speed: Some((0.8, 2.4)),
            col: Some(vec![[240.0, 238.0, 230.0], [200.0, 204.0, 216.0]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
        let chips = Emit {
            n: Some(12),
            col: Some(vec![HEADC, HOOPC, CHROME]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "chips", &chips);
    }

    // ------------------------------------------------------------ the canvas: the dust line, tread, rings, targets
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let t = c.t;
        if !(self.t_fight <= t && t < p.t_ko + 0.6) {
            return;
        }
        let tc = self.ta(p, t);
        let dustc = DUSTC.map(|v| v / 255.0);
        for s in &self.segs {
            if !(s.t_tip <= tc && tc < s.t_end + 0.6) {
                continue;
            }
            let a = s.p0;
            let d = [s.p1[0] - a[0], s.p1[1] - a[1]];
            let len = dmath::np_hypot(d[0], d[1]) + 1e-9;
            let d = [d[0] / len, d[1] / len];
            let out = py_max(0.0, (tc - s.t_end) / 0.6);
            let telegraph = tc < s.t0 + 0.05 && s.kind == 0;
            let fin = py_min(1.0, (tc - s.t_tip) / py_max(1e-3, s.t0 - s.t_tip));
            let u = py_min(1.0, py_max(0.0, (py_min(tc, s.t_end) - s.t0) / py_max(1e-9, s.t1 - s.t0)));
            for (j, light) in ic.iter_mut().enumerate() {
                let (rx, rz) = (c.sxp[j] - a[0], c.szp[j] - a[1]);
                let along = rx * d[0] + rz * d[1];
                let off = (-rx * d[1] + rz * d[0]).abs();
                if telegraph {
                    // the telegraph: a dashed dust line, fading in
                    let dash = py::fmod_py(along / 0.24 - tc * 3.0, 1.0) < 0.55;
                    let lane = off < 0.06 && along > R * 0.6 && along < len + 0.15 && dash;
                    let v = gate(lane) * 0.5 * fin;
                    for ch in 0..3 {
                        light[ch] += v * dustc[ch];
                    }
                }
                if u > 0.0 {
                    // the tread behind it: churned canvas
                    let tread = off < R * 0.42 && along > 0.0 && along < len * u;
                    let k_ = 1.0 - gate(tread) * 0.18 * (1.0 - out);
                    for v in light.iter_mut() {
                        *v *= k_;
                    }
                    let edge = tread && off > R * 0.30;
                    let v = gate(edge) * 0.25 * (1.0 - out);
                    for ch in 0..3 {
                        light[ch] += v * dustc[ch];
                    }
                }
            }
        }
    }

    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind != "shockwave" {
            return;
        }
        // HEARTBEAT's thick rings
        let k_ = 1.0 - w.r / w.rmax;
        let tint = p.col[w.owner].map(|v| v / 255.0 * 1.2 + 0.3);
        for (j, light) in ic.iter_mut().enumerate() {
            let rd = dmath::np_hypot(c.sxp[j] - w.cx, c.szp[j] - w.cz);
            let band = np::clip(1.0 - (rd - w.r).abs() / 0.22, 0.0, 1.0);
            let band = (band * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
            let v = band * k_ * 1.0;
            for ch in 0..3 {
                light[ch] += v * tint[ch];
            }
        }
    }

    fn canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        // 1812 OVERTURE: the target, then the blast
        for s in slams(p, c.i) {
            if s.owner != self.k as i64 {
                continue;
            }
            if c.t < s.t_slam {
                let u = (c.t - s.t0) / py_max(1e-9, s.t_slam - s.t0);
                let dim = 1.0 - 0.4 * u;
                let on = 0.5 + 0.5 * gate(dmath::sin(c.t * 36.0) > 0.0);
                let k_ = 0.35 + 0.45 * on;
                let mark = [1.0 * k_, 0.72 * k_, 0.4 * k_];
                for (j, light) in ic.iter_mut().enumerate() {
                    let (sx, sz) = (c.sxp[j], c.szp[j]);
                    let rd = dmath::np_hypot(sx - s.cx, sz - s.cz);
                    if rd < s.rad {
                        for v in light.iter_mut() {
                            *v *= dim;
                        }
                    }
                    let ring = (rd - s.rad * (1.3 - 0.3 * u)).abs() < 0.05;
                    let cross = ((sx - s.cx).abs() < 0.03 || (sz - s.cz).abs() < 0.03) && rd < s.rad * 0.8;
                    if ring || cross {
                        for ch in 0..3 {
                            light[ch] += mark[ch];
                        }
                    }
                }
            } else {
                let u = (c.t - s.t_slam) / 0.5;
                if u < 1.0 {
                    let blast = [1.0, 0.8, 0.55];
                    for (j, light) in ic.iter_mut().enumerate() {
                        let rd = dmath::np_hypot(c.sxp[j] - s.cx, c.szp[j] - s.cz);
                        let band = np::clip(1.0 - (rd - s.rad * (0.6 + 0.6 * u)).abs() / 0.2, 0.0, 1.0) * (1.0 - u);
                        let v = band * 1.1;
                        for ch in 0..3 {
                            light[ch] += v * blast[ch];
                        }
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------ over the ring: the booms' sound rings
    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        let cam = f.cam;
        let t = f.t;
        if !(self.t_fight <= t && t < p.t_hs + 0.5) {
            return;
        }
        for &(t0, x, y, kind) in &self.booms {
            let age = t - t0;
            if !(0.0 <= age && age < 0.35) || t0 > p.t_ko {
                continue;
            }
            let kk = 1.0 - age / 0.35;
            for j in 0..(if kind == 0 { 2 } else { 1 }) {
                let rr = R * (1.1 + 3.0 * age / 0.35 + 0.5 * f64::from(j));
                let world: Vec<V3> = linspace(0.0, 2.0 * dmath::PI, 25)
                    .iter()
                    .map(|&th| scene::to_world(x + rr * dmath::cos(th), y + rr * dmath::sin(th), R * 0.9))
                    .collect();
                let sp = cam.wp(&world);
                reeds::blend(
                    arr,
                    &reeds::mask(&[], std::slice::from_ref(&sp), 3),
                    INK,
                    Num::Py(0.35 * kk),
                );
                reeds::blend(arr, &reeds::mask(&[], &[sp], 1), WHITE, Num::Py(0.85 * kk));
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
        let d = [dmath::cos(rot), dmath::sin(rot)];
        let nn = [-d[1], d[0]];
        let c = [pos[0], pos[2]];
        let on_screen = |pts: &[[f64; 2]]| -> Vec<(f64, f64)> {
            let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], pos[1])).collect();
            screen(cam, &world)
        };
        match kind {
            "rod" => {
                // a chrome tension rod
                let pts = [
                    [c[0] - d[0] * 0.07, c[1] - d[1] * 0.07],
                    [c[0] + d[0] * 0.07, c[1] + d[1] * 0.07],
                ];
                let sp = on_screen(&pts);
                reeds::blend(arr, &reeds::mask(&[], std::slice::from_ref(&sp), 3), INK, Num::Py(0.7));
                reeds::blend(arr, &reeds::mask(&[], &[sp], 1), CHROME, Num::Py(1.0));
            }
            "hoop" => {
                // an arc of dark hoop
                let pts: Vec<[f64; 2]> = (0..6)
                    .map(|j| {
                        let s = f64::from(j) - 2.5;
                        // c + d * (j - 2.5) * 0.05 + nn * curl * 0.025 * (j - 2.5) ** 2 (a Python float squared)
                        let s2 = dmath::pow(s, 2.0);
                        [
                            c[0] + d[0] * s * 0.05 + nn[0] * curl * 0.025 * s2,
                            c[1] + d[1] * s * 0.05 + nn[1] * curl * 0.025 * s2,
                        ]
                    })
                    .collect();
                let sp = on_screen(&pts);
                reeds::blend(arr, &reeds::mask(&[], std::slice::from_ref(&sp), 3), HOOPC, Num::Py(1.0));
                reeds::blend(arr, &reeds::mask(&[], &[sp], 1), [140.0, 140.0, 156.0], Num::Py(0.8));
            }
            _ => {
                // a torn piece of white head
                let sz = size * 1.9;
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
                let shade = 0.8 + 0.22 * dmath::cos(rot).abs();
                layer_blend(arr, std::slice::from_ref(&sp), HEADC.map(|v| v * shade), 1.0, None);
                // np.vstack([sp, sp[:1]]): the outline, closed
                let mut edge = sp;
                edge.push(edge[0]);
                reeds::blend(arr, &reeds::mask(&[], &[edge], 1), [110.0, 110.0, 124.0], Num::Py(0.8));
            }
        }
    }

    // ------------------------------------------------------------ checks
    fn focus_points(&self, _p: &PictureCtx, _t: f64, _st: &BallState) -> Vec<(V3, String)> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_shatter_is_fourteen_parts_of_three_kinds() {
        let parts = BassDrumLook::new().shards();
        let count = |kind: &str| parts.iter().filter(|p| *p == kind).count();
        assert_eq!(
            (parts.len(), count("head"), count("hoop"), count("rod")),
            (14, 6, 4, 4)
        );
    }

    /// The rolling drum is the model without its mallet: the first faces of the model file, fewer of them.
    #[test]
    fn the_rolling_model_is_the_drum_without_its_mallet() {
        assert_eq!(nomallet().faces.len(), NOMALLET_FACES);
        assert!(mesh().faces.len() > NOMALLET_FACES);
        assert_eq!(nomallet().faces[..], mesh().faces[..NOMALLET_FACES]);
        // a sprite of either is the padded size, 2 * int(round(r * 1.18)) + 7
        let look = BassDrumLook::new();
        let roll = look.sprite(20, 0.0, 1030.0).expect("its own sprite");
        let std = look.sprite(20, 12.0, 0.0).expect("its own sprite");
        assert_eq!((roll.w, roll.h, std.w), (2 * 24 + 7, 2 * 24 + 7, 2 * 24 + 7));
        assert_ne!(roll, std);
    }
}
