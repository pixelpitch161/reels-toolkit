//! TENOR SAX's look: the port of `Look` in `projects/ballfight/kit/fighters/tenor_sax.py`, hook by hook.
//!
//! What it draws: a pose as a little hop with a glow and honk lines from the bell; a hype meter of pips arcing over
//! its ball; every throw arcing out of the crowd's hands in the front rows to its landing spot (a tomato, a rose or
//! a foam finger), its shadow growing on the canvas as it falls, and its splat; WALK THE BAR's catwalk across the
//! canvas with chaser bulbs along its edges, the honks as rings in its colour; STAGE DIVE's arc over the ropes into
//! the stands, the ball carried on raised hands, then the low cannonball arc with streaks and a cratering landing
//! ring; amber brass, key cups and pearls when it loses; a row of cheering crowd silhouettes on its VS card panel.
//!
//! During the dive it moves its ball's place and height itself (`state`) and hides its HP number (`hides_hp`).
//!
//! Every roster number comes from the `cfg` that `setup_as` is given (`self.cfg` in Python): the weapon's `n0` and
//! `n_max`.

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::roster::{FighterCfg, WeaponCfg};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{H, W, bayer_full};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::{Img, RgbF};
use bf_picture::look::{CanvasCtx, FrameCtx, Fx, Look, Phase, Pic, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::title::layout;

use super::state;
use crate::fighters::shared::reeds::{self, INK, Num};

/// `AMBER`: the lacquer.
const AMBER: V3 = [218.0, 156.0, 52.0];
/// `KEYG`: the bright keys.
const KEYG: V3 = [250.0, 196.0, 92.0];
/// `PEARL`.
const PEARL: V3 = [250.0, 246.0, 236.0];
/// `TOMATO`.
const TOMATO: V3 = [220.0, 44.0, 34.0];
/// `LEAF`.
const LEAF: V3 = [60.0, 150.0, 60.0];
/// `ROSE`.
const ROSE: V3 = [236.0, 60.0, 110.0];
/// `STAND_R`: how far from the centre the crowd's hands are (the front rows, world units).
const STAND_R: f64 = 6.55;
/// `HAND_Y`: their height.
const HAND_Y: f64 = -0.25;
/// `ITEM_COLS`: the splat of a tomato, the petals of a rose, the puffs of a foam finger.
const ITEM_COLS: [[V3; 3]; 3] = [
    [[230.0, 50.0, 40.0], [255.0, 120.0, 90.0], [120.0, 20.0, 16.0]],
    [[240.0, 90.0, 140.0], [255.0, 180.0, 200.0], [200.0, 40.0, 90.0]],
    [[230.0, 110.0, 30.0], [255.0, 190.0, 110.0], [255.0, 255.0, 255.0]],
];
/// `CATWALK`: the runway's light.
const CATWALK: V3 = [1.0, 0.86, 0.66];

/// `np.linalg.norm(v)` of a 3-vector: the root of `v.dot(v)`, whose products numpy adds fused on the reference PC
/// (`fma(v2, v2, fma(v1, v1, v0 * v0))`, as the sprite camera's dot product of two vectors, ruling M3-R30).
fn norm3(v: V3) -> f64 {
    dmath::fma(v[2], v[2], dmath::fma(v[1], v[1], v[0] * v[0])).sqrt()
}

/// `stand_point(x, z)`: the crowd's hands behind the ring-frame point `(x, z)`: straight out from the centre, in
/// the front rows.
pub fn stand_point(x: f64, z: f64) -> V3 {
    let w = scene::to_world(x, z, 0.0);
    let d = [w[0], 0.0, w[2]];
    let n = norm3(d) + 1e-9;
    let d = [d[0] / n, d[1] / n, d[2] / n];
    [d[0] * STAND_R, HAND_Y, d[2] * STAND_R]
}

/// `title_pattern(base)`: a row of cheering crowd silhouettes along the divider, hands up. `base` is the panel's
/// gradient, changed in place.
pub fn title_pattern(base: &mut RgbF) {
    let dd = layout::dd();
    let side = layout::side();
    let da = layout::DA;
    // np.array([40, 18, 6]) * 0.65
    let ink = [40.0 * 0.65, 18.0 * 0.65, 6.0 * 0.65];
    for y in 0..base.h {
        for x in 0..base.w {
            // u = (XX - TT.DA[0]) * TT.DD[0] + (YY - TT.DA[1]) * TT.DD[1]
            let u = (x as f64 - da[0]) * dd[0] + (y as f64 - da[1]) * dd[1];
            let v = side.px(x, y)[0].abs();
            let cu = py::fmod_py(u, 34.0) - 17.0;
            let band = py::fmod_py(v, 120.0);
            let head = cu * cu + (band - 44.0) * (band - 44.0) < 64.0;
            let body = cu.abs() < 11.0 && band > 50.0 && band < 80.0;
            let arms = (cu.abs() - 10.0 + (band - 40.0) * 0.25).abs() < 2.0 && band > 18.0 && band < 48.0;
            if head || body || arms {
                for (ch, v) in base.px_mut(x, y).iter_mut().enumerate() {
                    *v = *v * 0.35 + ink[ch];
                }
            }
        }
    }
}

/// `e[7]` of a hit (what hit), or `""` for an event without one.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// `[(c[0] + r * math.cos(a), c[1] + r * math.sin(a)) for a in np.linspace(0, 2 * math.pi, n + 1)[:-1]]`: `n`
/// points of a circle on the screen.
fn circle(c: (f64, f64), r: f64, n: usize) -> Vec<(f64, f64)> {
    // np.linspace(0, 2 * pi, n + 1): i * step, step = 2 * pi / n
    let step = (2.0 * dmath::PI - 0.0) / n as f64;
    (0..n)
        .map(|i| {
            let a = i as f64 * step + 0.0;
            (c.0 + r * dmath::cos(a), c.1 + r * dmath::sin(a))
        })
        .collect()
}

/// One of its poses: `(e[0], e[3], e[4])`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pose {
    t: f64,
    t_end: f64,
    hype: f64,
}

/// One of its throws: `dict(id, p0, p1, t0, t1, item)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Throw {
    id: i64,
    p0: [f64; 2],
    p1: [f64; 2],
    t0: f64,
    t1: f64,
    item: i64,
}

/// One of its runways: `(t0, a, b, t1)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Runway {
    t0: f64,
    a: [f64; 2],
    b: [f64; 2],
    t1: f64,
}

/// One of its stage dives.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Dive {
    t0: f64,
    out: [f64; 2],
    t_surf: f64,
    t_launch: f64,
    /// `(l0, l1, t_land)` of its `dive_launch` event, when the fight got that far.
    launch: Option<([f64; 2], [f64; 2], f64)>,
    /// The time of its `dive_land` event, when the fight got that far.
    landed: Option<f64>,
}

/// TENOR SAX's look: `Look` of `kit/fighters/tenor_sax.py`.
#[derive(Clone, Debug, Default)]
pub struct TenorSaxLook {
    /// Its side.
    k: usize,
    /// `POSES`.
    poses: Vec<Pose>,
    /// `THROWS`.
    throws: Vec<Throw>,
    /// `SPLAT`: by throw, `(time, x, y, hit)`, in event order (a later event of the same id replaces the earlier).
    splat: Vec<(i64, (f64, f64, f64, bool))>,
    /// `RUNWAY`.
    runway: Vec<Runway>,
    /// `DIVES`.
    dives: Vec<Dive>,
    /// `self.cfg['weapon']['n0']`: the hype outside the fight (`None` for a weapon without one).
    n0: Option<f64>,
    /// `self.cfg['weapon']['n_max']`: the pips of the hype meter.
    n_max: Option<f64>,
}

impl TenorSaxLook {
    /// `ta(t)`: the time, held at the K.O.
    fn ta(&self, p: &PictureCtx, t: f64) -> f64 {
        py_min(t, p.t_ko)
    }

    /// `pose_at(t)`: `(how far through the pose and its 0.12 s tail, the hype)`, or `None`.
    fn pose_at(&self, t: f64) -> Option<(f64, f64)> {
        // j = int(np.searchsorted(self.POSE_T, t, 'right')) - 1
        let after = self.poses.partition_point(|q| q.t <= t);
        let q = self.poses.get(after.checked_sub(1)?)?;
        if t < q.t_end + 0.12 {
            return Some(((t - q.t) / py_max(1e-6, q.t_end + 0.12 - q.t), q.hype));
        }
        None
    }

    /// `dive_at(t)`: the dive it is in at `t`: from the leap to the landing (a second past the planned launch for a
    /// dive the fight ended before).
    fn dive_at(&self, t: f64) -> Option<&Dive> {
        self.dives.iter().find(|d| {
            let end = match d.launch {
                Some((_, _, t_land)) => t_land,
                None => d.t_launch + 1.0,
            };
            d.t0 <= t && t < end
        })
    }

    /// `runway_at(t)`: the runway lit at `t`: from its start to a quarter of a second after its end.
    fn runway_at(&self, t: f64) -> Option<&Runway> {
        self.runway.iter().find(|r| r.t0 <= t && t < r.t1 + 0.25)
    }

    /// `hype(t)`: the recorded hype at `t` (`RUN['CROWD'][i][k][0]`), the roster's `n0` outside the fight.
    fn hype(&self, p: &PictureCtx, t: f64) -> f64 {
        if !p.run.has("CROWD") || !(p.show.grid.t_fight <= t && t < p.t_ko) {
            // float(self.cfg['weapon']['n0']) (Python raises for a weapon without one)
            return self.n0.unwrap_or(0.0);
        }
        let row = match p.run.nested("CROWD", p.sim_i(t)) {
            Some(Value::List(rows)) => rows.get(self.k),
            _ => None,
        };
        match row {
            Some(Value::List(v)) => match v.first() {
                Some(Value::Float(x)) => *x,
                Some(Value::Int(n)) => *n as f64,
                _ => 0.0,
            },
            _ => 0.0,
        }
    }

    /// `_p_at(t)`: where its ball was at `t` (`RUN['P'][sim_i(t), k]`).
    fn p_at(&self, p: &PictureCtx, t: f64) -> [f64; 2] {
        p.run.p(p.sim_i(t), self.k)
    }

    /// `_flying(t)`: its throws in the air at `t`.
    fn flying(&self, t: f64) -> impl Iterator<Item = &Throw> {
        self.throws.iter().filter(move |th| th.t0 <= t && t < th.t1)
    }
}

/// `w0 + (w1 - w0) * u`.
fn lerp3(w0: V3, w1: V3, u: f64) -> V3 {
    std::array::from_fn(|i| w0[i] + (w1[i] - w0[i]) * u)
}

impl Look for TenorSaxLook {
    /// `['brass'] * 6 + ['key'] * 4 + ['pearl'] * 3`.
    fn shards(&self) -> Vec<String> {
        [("brass", 6), ("key", 4), ("pearl", 3)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["throw", "dive", "strut"]
    }

    fn spark_flash(&self) -> &[&str] {
        &["dive"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["throw", "dive"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["dive"]
    }

    fn quiet_hits(&self) -> &[&str] {
        &[]
    }

    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[("stagedive", 1.0)]
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

    /// What `setup_as` took from `cfg`, by the keys `tenor_sax.py` reads from `self.cfg`.
    fn cfg_summary(&self) -> Option<String> {
        Some(format!("weapon.n0={:?}; weapon.n_max={:?}", self.n0, self.n_max))
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`: the hype's numbers come from `cfg`, the events from side
    /// `k`. For its own side `cfg` is `p.cfg[k]`, and this is `setup`.
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        (self.n0, self.n_max) = match &cfg.weapon {
            WeaponCfg::Crowd(w) => (Some(w.n0), Some(w.n_max)),
            _ => (None, None),
        };
        let ev = &p.rec.events;
        let mine = |kind: &'static str| ev.iter().filter(move |e| e.kind == kind && e.who() == Some(k));
        self.poses = mine("pose")
            .map(|e| Pose {
                t: e.t,
                t_end: e.f(3),
                hype: e.f(4),
            })
            .collect();
        self.throws = mine("throw")
            .map(|e| Throw {
                id: e.i(3),
                p0: [e.f(4), e.f(5)],
                p1: [e.f(6), e.f(7)],
                t0: e.f(8),
                t1: e.f(9),
                item: e.i(10),
            })
            .collect();
        self.splat = Vec::new();
        for e in mine("splat") {
            let entry = (e.t, e.f(4), e.f(5), e.at(6).truthy());
            match self.splat.iter_mut().find(|(id, _)| *id == e.i(3)) {
                Some(slot) => slot.1 = entry,
                None => self.splat.push((e.i(3), entry)),
            }
        }
        // (t0, a, b, min([t1] + [x for x in ends if x >= t0 - 1e-6])): ends = its strut_end and dive times
        self.runway = mine("runway")
            .map(|e| Runway {
                t0: e.t,
                a: [e.f(3), e.f(4)],
                b: [e.f(5), e.f(6)],
                t1: state::runway_end(ev, k, e.f(8), e.t - 1e-6),
            })
            .collect();
        self.dives = mine("dive")
            .map(|e| {
                let la = mine("dive_launch").find(|x| x.t >= e.t);
                let ld = mine("dive_land").find(|x| x.t >= e.t);
                Dive {
                    t0: e.t,
                    out: [e.f(3), e.f(4)],
                    t_surf: e.f(5),
                    t_launch: e.f(6),
                    launch: la.map(|la| ([la.f(3), la.f(4)], [la.f(5), la.f(6)], la.f(7))),
                    landed: ld.map(|ld| ld.t),
                }
            })
            .collect();
    }

    // ------------------------------------------------------------ state: the pose hop, the dive's flight path
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if kk != self.k || phase != Phase::Fight {
            return;
        }
        let tc = self.ta(p, t);
        if let Some((u, _)) = self.pose_at(tc) {
            st.lift = py_max(st.lift, 0.14 * dmath::sin(dmath::PI * py_min(1.0, u)));
            st.glow = py_max(st.glow, 0.4 * (1.0 - u));
        }
        if self.runway_at(tc).is_some() {
            st.glow = py_max(st.glow, 0.3 + 0.2 * p.show.grid.beat_pulse(t, 0.12, 0.12));
        }
        let Some(d) = self.dive_at(tc) else { return };
        st.glow = py_max(st.glow, 0.45);
        let at = st.p;
        let carried = HAND_Y + 0.15;
        let w = if tc < d.t_surf {
            // the leap: over the ropes, into the stands
            let u = (tc - d.t0) / py_max(1e-6, d.t_surf - d.t0);
            let from = self.p_at(p, d.t0);
            let w0 = scene::to_world(from[0], from[1], 0.0);
            let w1 = stand_point(d.out[0], d.out[1]);
            let ue = u * u * (3.0 - 2.0 * u);
            // 2.2 * math.sin(math.pi * u) + (HAND_Y + 0.15 - R * 0.0) * ue
            st.lift = 2.2 * dmath::sin(dmath::PI * u) + (carried - R * 0.0) * ue;
            lerp3(w0, w1, ue)
        } else if tc < d.t_launch || d.launch.is_none() {
            // crowd-surfing, carried on raised hands
            st.lift = carried + 0.07 * dmath::sin(2.0 * dmath::PI * (tc - d.t_surf) * 4.0);
            st.extra.insert("surf".to_string(), 1.0);
            stand_point(at[0], at[1])
        } else {
            // the cannonball: a low arc from the crowd
            let (l0, l1, t_land) = d.launch.expect("tested above");
            let u = (tc - d.t_launch) / py_max(1e-6, t_land - d.t_launch);
            let w0 = stand_point(l0[0], l0[1]);
            let w1 = scene::to_world(l1[0], l1[1], 0.0);
            st.lift = carried * (1.0 - u) + 1.3 * dmath::sin(dmath::PI * u);
            st.extra.insert("cannon".to_string(), u);
            lerp3(w0, w1, u)
        };
        let (x, z) = scene::to_ring(w[0], w[2]);
        st.p = [x, z];
    }

    fn hides_hp(&self, p: &PictureCtx, kk: usize, t: f64) -> bool {
        kk == self.k && self.dive_at(self.ta(p, t)).is_some()
    }

    // ------------------------------------------------------------ events -> camera, sparks
    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match e.kind.as_str() {
            "dive_land" => Some(2.2),
            "dive_hit" => Some(2.0),
            "pose" => Some(0.3),
            "strut_bump" => Some(1.0),
            _ => None,
        }
    }

    fn rope(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind != "ability" {
            return None;
        }
        // self.rope_kinds.get(e[4])
        match e.fields.get(2) {
            Some(Value::Str(kind)) if kind == "stagedive" => Some(1.0),
            _ => None,
        }
    }

    fn spark(&self, _p: &PictureCtx, e: &Event) -> Spark {
        match how(e) {
            "throw" => ([255.0, 140.0, 100.0], 10, 2.2, 0.28),
            "dive" => ([255.0, 190.0, 120.0], 18, 3.6, 0.36),
            _ => ([255.0, 170.0, 90.0], 8, 2.0, 0.24),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "dive" | "strut" => Some((16.0, 0.4)),
            "throw" => Some((9.0, 0.3)),
            _ => None,
        }
    }

    // ------------------------------------------------------------ effects
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let k = self.k;
        let col = p.fl_col[k];
        for th in &self.throws {
            // sp = self.SPLAT.get(Th['id'])
            let Some(&(_, (t0, x, y, hit))) = self.splat.iter().find(|(id, _)| *id == th.id) else {
                continue;
            };
            if t0 > p.t_ko {
                continue;
            }
            let pos = up(x, y, 0.08);
            match th.item {
                0 => {
                    // a tomato splats
                    let emit = Emit {
                        n: Some(if hit { 7 } else { 5 }),
                        col: Some(ITEM_COLS[0].to_vec()),
                        speed: Some((0.8, 2.0)),
                        ..Emit::default()
                    };
                    fx.pa.emit(t0, pos, "shreds", &emit);
                    // seed=int(t0 * 37) % 991
                    let seed = py::imod_py(np::f64_to_i64(t0 * 37.0), 991) as u64;
                    fx.mk.add(
                        t0,
                        Mark {
                            size: 0.30,
                            life: 4.0,
                            col: [200.0, 40.0, 30.0],
                            seed,
                            strength: 0.8,
                            ..Mark::new(MarkKind::Dust, x, y)
                        },
                    );
                }
                1 => {
                    // a rose bursts into petals
                    let emit = Emit {
                        n: Some(7),
                        col: Some(ITEM_COLS[1].to_vec()),
                        speed: Some((0.6, 1.6)),
                        ..Emit::default()
                    };
                    fx.pa.emit(t0, pos, "scales", &emit);
                }
                _ => {
                    // a foam finger bounces
                    let emit = Emit {
                        n: Some(4),
                        col: Some(ITEM_COLS[2].to_vec()),
                        ..Emit::default()
                    };
                    fx.pa.emit(t0, pos, "puffs", &emit);
                }
            }
            if hit {
                fx.fl.add(t0, up(x, y, R), col, 1.2, 0.4, 0.03, 0.15, 0.0);
            }
        }
        for q in &self.poses {
            if q.t > p.t_ko {
                break;
            }
            let at = self.p_at(p, q.t);
            fx.fl.add(q.t, up(at[0], at[1], R), col, 1.4, 0.35, 0.03, 0.18, 0.0);
        }
        for d in &self.dives {
            if let (Some(landed), Some((_, l1, _))) = (d.landed, d.launch)
                && landed <= p.t_ko
            {
                let pos = up(l1[0], l1[1], 0.1);
                let emit = Emit {
                    n: Some(14),
                    speed: Some((0.6, 1.6)),
                    ..Emit::default()
                };
                fx.pa.emit(landed, pos, "dust", &emit);
                fx.mk.add(
                    landed,
                    Mark {
                        size: 0.8,
                        life: 5.0,
                        seed: 17,
                        strength: 0.8,
                        ..Mark::new(MarkKind::Crack, l1[0], l1[1])
                    },
                );
                fx.bu.add(landed, up(l1[0], l1[1], R), p.col[k], 1.6, None, true);
                fx.fl.add(landed, pos, col, 2.6, 1.0, 0.03, 0.35, 0.0);
            }
            if let Some((l0, l1, _)) = d.launch
                && d.t_launch <= p.t_ko
            {
                let w0 = stand_point(l0[0], l0[1]);
                let w1 = scene::to_world(l1[0], l1[1], 0.0);
                let fly = [w1[0] - w0[0], w1[1] - w0[1], w1[2] - w0[2]];
                let n = norm3(fly) + 1e-9;
                let emit = Emit {
                    n: Some(10),
                    dir: [fly[0] / n, fly[1] / n, fly[2] / n],
                    col: Some(vec![[255.0, 200.0, 140.0], [255.0, 255.0, 230.0]]),
                    ..Emit::default()
                };
                fx.pa
                    .emit(d.t_launch, [w0[0] + 0.0, w0[1] + R, w0[2] + 0.0], "streaks", &emit);
            }
            if d.t0 <= p.t_ko {
                let s = stand_point(d.out[0], d.out[1]);
                let emit = Emit {
                    n: Some(10),
                    col: Some(vec![[255.0, 220.0, 160.0], [255.0, 255.0, 255.0], p.col[k]]),
                    ..Emit::default()
                };
                fx.pa
                    .emit(d.t_surf, [s[0] + 0.0, s[1] + 0.3, s[2] + 0.0], "foam", &emit);
            }
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if how(e) == "strut" {
            let who = bf_picture::ring::owner(e).unwrap_or(0);
            fx.bu
                .add(e.t, scene::to_world(e.f(5), e.f(6), R), p.col[who], 1.0, None, true);
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let brass = Emit {
            n: Some(18),
            col: Some(vec![AMBER, KEYG, [255.0, 230.0, 180.0]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "brass", &brass);
        let sparks = Emit {
            n: Some(20),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            col: Some(vec![[255.0, 160.0, 80.0], [255.0, 230.0, 200.0]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
    }

    // ------------------------------------------------------------ the canvas: the catwalk, the throws' shadows
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let tc = self.ta(p, c.t);
        let Some(rw) = self.runway_at(tc) else { return };
        let (t0, a, b, t1) = (rw.t0, rw.a, rw.b, rw.t1);
        let fade = py_min(1.0, (tc - t0) / 0.12) * py_min(1.0, py_max(0.0, (t1 + 0.25 - tc) / 0.25));
        let d = [b[0] - a[0], b[1] - a[1]];
        let len = np::hypot(d[0], d[1]) + 1e-9;
        let u = [d[0] / len, d[1] / len];
        let ext = 0.7;
        let bayer = bayer_full();
        for (j, light) in ic.iter_mut().enumerate() {
            let (rel_x, rel_z) = (c.sxp[j] - a[0], c.szp[j] - a[1]);
            let along = rel_x * u[0] + rel_z * u[1];
            let across = -rel_x * u[1] + rel_z * u[0];
            let inside = along > -ext && along < len + ext;
            // band = np.clip((0.42 - np.abs(across)) / 0.06, 0, 1) * inside
            let band = np::clip((0.42 - across.abs()) / 0.06, 0.0, 1.0) * f64::from(u8::from(inside));
            let band = (band * 3.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 3.0;
            let v = band * 0.42 * fade;
            for ch in 0..3 {
                light[ch] += v * CATWALK[ch];
            }
        }
        // chaser bulbs along both edges
        let step = 0.36;
        let ph = (tc - t0) / (p.show.grid.beat / 4.0);
        let bulb = [1.0, 0.8, 0.45];
        let count = ((len + 2.0 * ext) / step) as i64 + 1;
        for side in [-1.0, 1.0] {
            for j in 0..count {
                let s = -ext + j as f64 * step;
                let on = if py::imod_py(j + ph as i64, 3) == 0 { 1.0 } else { 0.35 };
                let bx = a[0] + u[0] * s - u[1] * side * 0.5;
                let bz = a[1] + u[1] * s + u[0] * side * 0.5;
                for (i, light) in ic.iter_mut().enumerate() {
                    let m = np::clip(1.0 - np::hypot(c.sxp[i] - bx, c.szp[i] - bz) / 0.07, 0.0, 1.0);
                    let v = m * on * 1.1 * fade;
                    for ch in 0..3 {
                        light[ch] += v * bulb[ch];
                    }
                }
            }
        }
    }

    fn canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let t = self.ta(p, c.t);
        let bayer = bayer_full();
        // each throw's shadow grows as it falls
        for th in self.flying(t) {
            let u = (t - th.t0) / py_max(1e-6, th.t1 - th.t0);
            let r = 0.06 + 0.12 * u;
            for (j, light) in ic.iter_mut().enumerate() {
                let m = np::clip(1.0 - np::hypot(c.sxp[j] - th.p1[0], c.szp[j] - th.p1[1]) / r, 0.0, 1.0);
                let m = (m * 2.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 2.0;
                let k = 1.0 - 0.45 * u * m;
                for v in light.iter_mut() {
                    *v *= k;
                }
            }
        }
    }

    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        let honk = w.kind == "honk";
        if w.owner != self.k || !(honk || w.kind == "dive") {
            return;
        }
        let wid = if honk { 0.10 } else { 0.16 };
        // (1 - r / rmax) ** 0.6
        let fade = dmath::pow(1.0 - w.r / w.rmax, 0.6);
        let gain = if honk { 0.9 } else { 1.2 };
        let tint = p.col[self.k].map(|v| v / 255.0 * 0.8 + 0.2);
        let bayer = bayer_full();
        for (j, light) in ic.iter_mut().enumerate() {
            let rd = np::hypot(c.sxp[j] - w.cx, c.szp[j] - w.cz);
            let band = np::clip(1.0 - (rd - w.r).abs() / wid, 0.0, 1.0) * fade;
            let band = (band * 3.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 3.0;
            let v = band * gain;
            for ch in 0..3 {
                light[ch] += v * tint[ch];
            }
        }
    }

    // ------------------------------------------------------------ over the ring: the throws in flight, honk
    // lines, the hype pips, the crowd's hands under a stage diver
    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        if !f.in_fight {
            return;
        }
        let (cam, k) = (f.cam, self.k);
        let col = p.col[k];
        let t = self.ta(p, f.t);
        let line = |a: (f64, f64), b: (f64, f64), width: u32| reeds::mask(&[], &[vec![a, b]], width);
        for th in self.flying(t) {
            let u = (t - th.t0) / py_max(1e-6, th.t1 - th.t0);
            let w0 = stand_point(th.p0[0], th.p0[1]);
            let w1 = scene::to_world(th.p1[0], th.p1[1], 0.12);
            // w = w0 + (w1 - w0) * u + np.array([0, 3.0 * 4 * u * (1 - u), 0])
            let mut w = lerp3(w0, w1, u);
            w[0] += 0.0;
            w[1] += 3.0 * 4.0 * u * (1.0 - u);
            w[2] += 0.0;
            let c = cam.wp(&[w])[0];
            let rp = py_max(2.0, 0.17 * cam.f / cam.depth(&[w])[0]);
            let spin = (t - th.t0) * 9.0 + th.id as f64;
            match th.item {
                0 => {
                    // tomato: red ball, green top
                    reeds::blend(arr, &reeds::mask(&[circle(c, rp * 1.25, 8)], &[], 1), INK, Num::Py(0.7));
                    reeds::blend(arr, &reeds::mask(&[circle(c, rp, 8)], &[], 1), TOMATO, Num::Py(1.0));
                    let top = line((c.0 - rp * 0.5, c.1 - rp * 0.9), (c.0 + rp * 0.5, c.1 - rp * 0.9), 1);
                    reeds::blend(arr, &top, LEAF, Num::Py(1.0));
                }
                1 => {
                    // rose: a pink-red bloom on a green stem
                    let e = (dmath::cos(spin), dmath::sin(spin));
                    let stem = line(c, (c.0 - e.0 * rp * 2.4, c.1 - e.1 * rp * 2.4), 1);
                    reeds::blend(arr, &stem, LEAF, Num::Py(1.0));
                    reeds::blend(arr, &reeds::mask(&[circle(c, rp * 1.1, 6)], &[], 1), ROSE, Num::Py(1.0));
                }
                _ => {
                    // foam finger: a big hand in its colour, finger up
                    let e = [dmath::cos(spin * 0.5), dmath::sin(spin * 0.5)];
                    let n = [-e[1], e[0]];
                    let c2 = [c.0, c.1];
                    // c + e * rp * 1.2 + n_ * rp, c + e * rp * 1.2 - n_ * rp, c - e * rp * 0.6 - n_ * rp,
                    // c - e * rp * 0.6 + n_ * rp
                    let corner = |along: f64, across: f64| -> [f64; 2] {
                        std::array::from_fn(|i| {
                            let q = if along > 0.0 {
                                c2[i] + e[i] * rp * along
                            } else {
                                c2[i] - e[i] * rp * -along
                            };
                            if across > 0.0 { q + n[i] * rp } else { q - n[i] * rp }
                        })
                    };
                    let hand = [
                        corner(1.2, 1.0),
                        corner(1.2, -1.0),
                        corner(-0.6, -1.0),
                        corner(-0.6, 1.0),
                    ];
                    let outline: Vec<(f64, f64)> = hand.iter().map(|q| (q[0], q[1])).collect();
                    reeds::blend(arr, &reeds::mask(&[outline], &[], 1), INK, Num::Py(0.6));
                    // [q * 0.86 + c * 0.14 for q in hand]
                    let inner: Vec<(f64, f64)> = hand
                        .iter()
                        .map(|q| (q[0] * 0.86 + c2[0] * 0.14, q[1] * 0.86 + c2[1] * 0.14))
                        .collect();
                    reeds::blend(arr, &reeds::mask(&[inner], &[], 1), col, Num::Py(1.0));
                    let finger = line(
                        (c2[0] + e[0] * rp * 1.1, c2[1] + e[1] * rp * 1.1),
                        (c2[0] + e[0] * rp * 3.0, c2[1] + e[1] * rp * 3.0),
                        2,
                    );
                    reeds::blend(arr, &finger, col, Num::Py(1.0));
                }
            }
        }
        let st = &f.fs[k];
        if !(st.vis && st.scale > 0.5) {
            return;
        }
        let at = scene::to_world(st.p[0], st.p[1], R + st.lift);
        let c = cam.wp(&[at])[0];
        let rp = R * cam.f / cam.depth(&[at])[0];
        let ps = self.pose_at(t);
        if let Some((u, _)) = ps
            && u < 0.9
        {
            // honk lines from the bell
            let kk = 1.0 - u;
            let ink = col.map(|v| np::clip(v * 0.6 + 110.0, 0.0, 255.0));
            for j in 0..3 {
                let a = -0.9 + 0.5 * f64::from(j);
                let (r0, r1) = (rp * (1.15 + 0.5 * u), rp * (1.55 + 0.7 * u));
                let seg = line(
                    (c.0 + r0 * dmath::cos(a), c.1 + r0 * dmath::sin(a)),
                    (c.0 + r1 * dmath::cos(a), c.1 + r1 * dmath::sin(a)),
                    2,
                );
                reeds::blend(arr, &seg, ink, Num::Py(kk));
            }
        }
        // the hype meter: pips over the ball
        let hy = self.hype(p, f.t);
        let n_on = (hy + 1e-9) as i64;
        // range(int(self.cfg['weapon']['n_max'])) (Python raises for a weapon without one)
        for j in 0..self.n_max.unwrap_or(0.0) as i64 {
            let a = -dmath::PI / 2.0 + (j as f64 - 2.5) * 0.30;
            let q = (c.0 + rp * 1.28 * dmath::cos(a), c.1 + rp * 1.28 * dmath::sin(a));
            let on = j < n_on;
            let mut colj = if on {
                col.map(|v| np::clip(v * 0.7 + 90.0, 0.0, 255.0))
            } else {
                [70.0, 60.0, 70.0]
            };
            if let Some((u, _)) = ps
                && on
            {
                colj = colj.map(|v| np::clip(v + 80.0 * (1.0 - u), 0.0, 255.0));
            }
            let (x, y) = (py::round_half_even(q.0) as i64, py::round_half_even(q.1) as i64);
            if 1 <= x && x < W as i64 - 1 && 1 <= y && y < H as i64 - 1 {
                // arr[y_ - 1:y_ + 1, x_ - 1:x_ + 1] = INK; arr[y_, x_] = colj
                for yy in y - 1..=y {
                    for xx in x - 1..=x {
                        for (v, c) in arr.px_mut(xx as usize, yy as usize).iter_mut().zip(INK) {
                            *v = c as f32;
                        }
                    }
                }
                for (v, c) in arr.px_mut(x as usize, y as usize).iter_mut().zip(colj) {
                    *v = c as f32;
                }
            }
        }
        if st.extra.get("surf").is_some_and(|&v| v != 0.0) {
            // the crowd's hands holding it up
            for j in 0..5 {
                let j = f64::from(j);
                let xx = c.0 - rp * 0.9 + j * rp * 0.45;
                let wob = dmath::sin(f.t * 9.0 + j * 1.7) * 1.5;
                let seg = line((xx, c.1 + rp * 0.9 + 6.0), (xx + wob, c.1 + rp * 0.7), 2);
                reeds::blend(arr, &seg, [60.0, 40.0, 52.0], Num::Py(0.9));
            }
        }
    }

    fn shimmer(&self, _p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, w: &Wave) {
        if w.kind == "dive" && w.owner == self.k {
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
        let d = [dmath::cos(rot), dmath::sin(rot)];
        let nn = [-d[1], d[0]];
        let c = [pos[0], pos[2]];
        let on_screen = |pts: &[[f64; 2]]| -> Vec<(f64, f64)> {
            let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], pos[1])).collect();
            screen(cam, &world)
        };
        match kind {
            "pearl" => {
                let sp = on_screen(&[c])[0];
                // x, y = int(sp[0]), int(sp[1])
                let (x, y) = (sp.0 as i64, sp.1 as i64);
                if 1 <= x && x < W as i64 - 1 && 1 <= y && y < H as i64 - 1 {
                    // arr[y - 1:y + 1, x - 1:x + 1] = PEARL
                    for yy in y - 1..=y {
                        for xx in x - 1..=x {
                            for (v, col) in arr.px_mut(xx as usize, yy as usize).iter_mut().zip(PEARL) {
                                *v = col as f32;
                            }
                        }
                    }
                }
            }
            "key" => {
                // ring = [c + np.array([math.cos(a), math.sin(a)]) * size * 1.0 for a in linspace(0, 2 pi, 9)[:-1]]
                let step = (2.0 * dmath::PI - 0.0) / 8.0;
                let ring: Vec<[f64; 2]> = (0..8)
                    .map(|i| {
                        let a = f64::from(i) * step + 0.0;
                        [c[0] + dmath::cos(a) * size * 1.0, c[1] + dmath::sin(a) * size * 1.0]
                    })
                    .collect();
                let sp = on_screen(&ring);
                let shade = 0.8 + 0.3 * dmath::cos(rot).abs();
                layer_blend(arr, std::slice::from_ref(&sp), KEYG.map(|v| v * shade), 1.0, None);
                let mut closed = sp.clone();
                closed.push(sp[0]);
                reeds::blend(arr, &reeds::mask(&[], &[closed], 1), [80.0, 40.0, 12.0], Num::Py(0.7));
            }
            _ => {
                // pts = [c - d * size * 2.0 + nn * curl * 0.05, c + d * size * 2.0 - nn * curl * 0.05]
                let pts = [
                    [
                        c[0] - d[0] * size * 2.0 + nn[0] * curl * 0.05,
                        c[1] - d[1] * size * 2.0 + nn[1] * curl * 0.05,
                    ],
                    [
                        c[0] + d[0] * size * 2.0 - nn[0] * curl * 0.05,
                        c[1] + d[1] * size * 2.0 - nn[1] * curl * 0.05,
                    ],
                ];
                let sp = on_screen(&pts);
                reeds::blend(
                    arr,
                    &reeds::mask(&[], std::slice::from_ref(&sp), 4),
                    [90.0, 50.0, 14.0],
                    Num::Py(0.9),
                );
                let shade = 0.8 + 0.35 * dmath::cos(rot).abs();
                reeds::blend(arr, &reeds::mask(&[], &[sp], 3), AMBER.map(|v| v * shade), Num::Py(1.0));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose(t: f64, t_end: f64, hype: f64) -> Pose {
        Pose { t, t_end, hype }
    }

    /// A pose shows from its event to 0.12 s after its end; the latest pose that has begun is the one asked.
    #[test]
    fn a_pose_runs_to_a_tail_after_its_end() {
        let look = TenorSaxLook {
            poses: vec![pose(2.0, 2.25, 3.0), pose(4.0, 4.25, 3.34)],
            ..TenorSaxLook::default()
        };
        assert_eq!(look.pose_at(1.9), None);
        assert_eq!(look.pose_at(2.0), Some((0.0, 3.0)));
        assert_eq!(look.pose_at(2.185), Some(((2.185 - 2.0) / (2.25 + 0.12 - 2.0), 3.0)));
        assert_eq!(look.pose_at(2.37), None);
        assert_eq!(look.pose_at(3.9), None);
        assert_eq!(look.pose_at(4.1).map(|q| q.1), Some(3.34));
    }

    /// A dive lasts from the leap to the landing; one the fight ended before, a second past its planned launch.
    #[test]
    fn a_dive_lasts_to_its_landing() {
        let dive = |t0: f64, launch: Option<f64>| Dive {
            t0,
            out: [0.0, 5.0],
            t_surf: t0 + 0.45,
            t_launch: t0 + 0.9,
            launch: launch.map(|t_land| ([0.0, 5.0], [1.0, 1.0], t_land)),
            landed: None,
        };
        let look = TenorSaxLook {
            dives: vec![dive(10.0, Some(11.2)), dive(20.0, None)],
            ..TenorSaxLook::default()
        };
        assert!(look.dive_at(9.99).is_none());
        assert_eq!(look.dive_at(10.0).map(|d| d.t0), Some(10.0));
        assert_eq!(look.dive_at(11.19).map(|d| d.t0), Some(10.0));
        assert!(look.dive_at(11.2).is_none());
        assert_eq!(look.dive_at(21.89).map(|d| d.t0), Some(20.0));
        assert!(look.dive_at(21.9).is_none());
    }

    /// A runway stays lit a quarter of a second after its end.
    #[test]
    fn a_runway_is_lit_a_little_past_its_end() {
        let look = TenorSaxLook {
            runway: vec![Runway {
                t0: 5.0,
                a: [0.0, 0.0],
                b: [1.0, 0.0],
                t1: 6.8,
            }],
            ..TenorSaxLook::default()
        };
        assert!(look.runway_at(4.99).is_none() && look.runway_at(7.05).is_none());
        assert!(look.runway_at(5.0).is_some() && look.runway_at(7.04).is_some());
    }

    /// The crowd's hands are straight out from the centre at the stands' radius and the hands' height.
    #[test]
    fn the_stands_are_straight_out_from_the_centre() {
        let s = stand_point(3.0, 3.0);
        // ring (3, 3) is world +z
        assert!(s[0].abs() < 1e-12 && (s[2] - STAND_R).abs() < 1e-8, "{s:?}");
        assert_eq!(s[1], HAND_Y);
        let s = stand_point(2.0, -1.0);
        assert!(((s[0] * s[0] + s[2] * s[2]).sqrt() - STAND_R).abs() < 1e-8);
        // the centre itself has no direction: the hands are at the centre (the 1e-9 keeps the division)
        assert_eq!(stand_point(0.0, 0.0), [0.0, HAND_Y, 0.0]);
    }
}
