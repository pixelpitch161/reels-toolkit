//! The run as the renderer reads it, and the picture state of a fighter at a time: `kit/picture.py`'s run access
//! (`sim_i`, `hp_at`, `rev_offset`, `theta_open`, `theta_tail`), `_glide_arcs`, `fighter`, the camera and the
//! lights.

use std::collections::BTreeMap;
use std::sync::Arc;

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py::{self, py_max, py_min};
use bf_core::recording::{Array, Recording};

use super::PictureCtx;
use crate::camera::{Cam, Key, V3, interp_keys};
use crate::config::{ease, ss};
use crate::lighting::Lights;
use crate::look::Phase;
use crate::np;
use crate::scene::{self, SPOT, SPOT_AIM};
use crate::v3;

/// `FPS` as a float.
const FPS: f64 = crate::config::FPS as f64;

/// One expanding ring of `RUN['WAVES']`: `[owner, centre x, centre z, radius now, final radius, kind, time it
/// started]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Wave {
    pub owner: usize,
    pub cx: f64,
    pub cz: f64,
    pub r: f64,
    pub rmax: f64,
    pub kind: String,
    pub t0: f64,
}

/// The run (`RUN`): the recording's per-frame arrays as the renderer indexes them.
#[derive(Clone, Debug)]
pub struct Run {
    rec: Arc<Recording>,
    /// `NSIM = len(RUN['HP'])`.
    pub nsim: usize,
    waves: Vec<Vec<Wave>>,
}

/// A number of a recording's JSON-like value.
fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Float(x) => Some(*x),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

impl Run {
    /// The run of a recording, after checking the arrays every frame reads: `HP`, `P`, `TH`, `BL` and `FR` (two
    /// values a frame per fighter) and `WAVES`. The error says, in words, what is missing or misshapen.
    pub fn new(rec: Arc<Recording>) -> Result<Run, String> {
        let nsim = match rec.arrays.get("HP") {
            Some(Array::F64 { shape, .. }) if shape.len() == 2 && shape[1] == 2 && shape[0] >= 1 => shape[0],
            _ => return Err("the recording has no HP array of two numbers a frame".into()),
        };
        match rec.arrays.get("P") {
            Some(Array::F64 { shape, data }) if shape[..] == [nsim, 2, 2] && data.len() == nsim * 4 => {}
            _ => return Err("the recording has no P array of two positions a frame".into()),
        }
        for key in ["TH", "BL", "FR"] {
            match rec.arrays.get(key) {
                Some(Array::I64 { shape, data }) if shape[..] == [nsim, 2] && data.len() == nsim * 2 => {}
                Some(Array::F64 { shape, data }) if shape[..] == [nsim, 2] && data.len() == nsim * 2 => {}
                _ => return Err(format!("the recording has no {key} array of two numbers a frame")),
            }
        }
        let frames = match rec.arrays.get("WAVES") {
            Some(Array::Nested(Value::List(frames))) if frames.len() == nsim => frames,
            _ => return Err("the recording has no WAVES list with one entry a frame".into()),
        };
        let mut waves = Vec::with_capacity(nsim);
        for (i, frame) in frames.iter().enumerate() {
            let Value::List(rows) = frame else {
                return Err(format!("WAVES, frame {i}: not a list"));
            };
            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                out.push(wave(row).ok_or_else(|| {
                    format!("WAVES, frame {i}: a wave is not [owner 0 or 1, x, z, radius, final radius, kind, start]")
                })?);
            }
            waves.push(out);
        }
        Ok(Run { rec, nsim, waves })
    }

    /// `key in RUN`.
    pub fn has(&self, key: &str) -> bool {
        self.rec.arrays.contains_key(key)
    }

    /// `float(RUN[key][i, k])` for an array of two numbers a frame; `None` when the recording has no such array
    /// or it does not reach frame `i`.
    pub fn f(&self, key: &str, i: usize, k: usize) -> Option<f64> {
        match self.rec.arrays.get(key)? {
            Array::F64 { shape, data } if shape.len() == 2 && shape[1] == 2 => data.get(i * 2 + k).copied(),
            Array::I64 { shape, data } if shape.len() == 2 && shape[1] == 2 => data.get(i * 2 + k).map(|&v| v as f64),
            _ => None,
        }
    }

    /// `int(RUN[key][i, k])`: as [`Run::f`], a float cut toward zero.
    pub fn i(&self, key: &str, i: usize, k: usize) -> Option<i64> {
        match self.rec.arrays.get(key)? {
            Array::I64 { shape, data } if shape.len() == 2 && shape[1] == 2 => data.get(i * 2 + k).copied(),
            Array::F64 { shape, data } if shape.len() == 2 && shape[1] == 2 => {
                data.get(i * 2 + k).map(|&v| np::f64_to_i64(v))
            }
            _ => None,
        }
    }

    /// `RUN['P'][i, k]`: fighter `k`'s position at recording frame `i` (held to the last frame).
    pub fn p(&self, i: usize, k: usize) -> [f64; 2] {
        let i = i.min(self.nsim - 1);
        match self.rec.arrays.get("P") {
            Some(Array::F64 { data, .. }) => [data[(i * 2 + k) * 2], data[(i * 2 + k) * 2 + 1]],
            _ => [0.0, 0.0], // (checked in `new`: cannot happen)
        }
    }

    /// The last recording frame, Python's index `-1`.
    pub fn last(&self) -> usize {
        self.nsim - 1
    }

    /// `RUN['WAVES'][i]`.
    pub fn waves(&self, i: usize) -> &[Wave] {
        self.waves.get(i).map_or(&[], Vec::as_slice)
    }

    /// The projectiles in flight at recording frame `i`: the `(x, y)` of every row of `RUN['K'][i]` whose first
    /// value is a number (an unused slot is NaN). Empty when the recording has no `K`.
    pub fn projectiles(&self, i: usize) -> Vec<[f64; 2]> {
        match self.rec.arrays.get("K") {
            Some(Array::F64 { shape, data }) if shape.len() == 3 && shape[2] >= 2 => {
                let (slots, width) = (shape[1], shape[2]);
                (0..slots)
                    .filter_map(|j| data.get((i * slots + j) * width..(i * slots + j) * width + 2))
                    .filter(|row| !row[0].is_nan())
                    .map(|row| [row[0], row[1]])
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    /// `RUN[key][i]` of a ragged array (`SLAMS`, `COB`), or `None` when the recording has none.
    pub fn nested(&self, key: &str, i: usize) -> Option<&Value> {
        match self.rec.arrays.get(key)? {
            Array::Nested(Value::List(frames)) => frames.get(i),
            _ => None,
        }
    }
}

/// One row of `WAVES`.
fn wave(row: &Value) -> Option<Wave> {
    let Value::List(v) = row else { return None };
    if v.len() < 7 {
        return None;
    }
    let owner = match v[0] {
        Value::Int(n @ 0..=1) => n as usize,
        _ => return None,
    };
    let Value::Str(kind) = &v[5] else { return None };
    Some(Wave {
        owner,
        cx: num(&v[1])?,
        cz: num(&v[2])?,
        r: num(&v[3])?,
        rmax: num(&v[4])?,
        kind: kind.clone(),
        t0: num(&v[6])?,
    })
}

/// The picture state of a fighter at a time: the dict `fighter(k, t)` returns. The fields the shared renderer
/// reads are named; a look keeps fields of its own in `extra`.
#[derive(Clone, Debug, PartialEq)]
pub struct BallState {
    /// `p`: the ball's centre in the ring frame.
    pub p: [f64; 2],
    pub vis: bool,
    pub shield: f64,
    pub haste: f64,
    pub glow: f64,
    pub scale: f64,
    pub th: f64,
    pub blen: f64,
    pub frets: i64,
    pub level: i64,
    pub shattered: bool,
    pub om: f64,
    pub sd: f64,
    pub ext: f64,
    pub slow: f64,
    pub thr_k: f64,
    pub lift: f64,
    /// `st.get('sink', 0.0)`: a look may sink the ball into shadow.
    pub sink: f64,
    /// `st['sx']`, when a look squashes or stretches the ball.
    pub sx: Option<f64>,
    /// `st['sy']`.
    pub sy: Option<f64>,
    /// `st.get('air_bias', 0.0)`: an airborne ball is drawn over what it hops past.
    pub air_bias: f64,
    /// `st['paint_h']`, when a look sets the height the ball is sorted at.
    pub paint_h: Option<f64>,
    /// Any other field a look adds for itself.
    pub extra: BTreeMap<String, f64>,
}

/// `(a + math.pi) % (2 * math.pi) - math.pi`.
fn wrap(a: f64) -> f64 {
    py::wrap_pi(a)
}

impl PictureCtx {
    /// `sim_i(t)`: the recording frame at `t`, `int(min(NSIM - 1, max(0, round((t - T.T_FIGHT) * FPS))))`.
    pub fn sim_i(&self, t: f64) -> usize {
        let r = py::round_half_even((t - self.show.grid.t_fight) * FPS);
        // max(0, r), then min(NSIM - 1, ...): a time that is not a number takes frame 0 (Python raises)
        if r > 0.0 {
            (r as usize).min(self.run.nsim - 1)
        } else {
            0
        }
    }

    /// `hp_at(k, t)`.
    pub fn hp_at(&self, k: usize, t: f64) -> f64 {
        let g = &self.show.grid;
        if t < g.t_fight {
            return 100.0;
        }
        if t < self.t_ko {
            return self.run.f("HP", self.sim_i(t), k).unwrap_or(0.0);
        }
        let end = if k == self.win {
            self.run.f("HP", self.run.last(), k).unwrap_or(0.0)
        } else {
            0.0
        };
        end + (100.0 - end) * ease((t - g.t_glide) / (g.t_rev - g.t_glide))
    }

    /// `rev_offset(k, t)`: the corner rev, a fast small shake and a slow sway, the same at both ends of the file.
    pub fn rev_offset(&self, k: usize, t: f64) -> [f64; 2] {
        let (p1, p2) = (61.0 / 419.0, 61.0 / 353.0);
        let a = 0.035;
        let k = k as f64;
        [
            a * dmath::sin(2.0 * dmath::PI * t / p1 + k),
            a * dmath::cos(2.0 * dmath::PI * t / p2 + 2.0 * k),
        ]
    }

    /// `theta_open(k, t)`: the blade angle while revving.
    pub fn theta_open(&self, k: usize, t: f64) -> f64 {
        self.th0[k] + self.sd0[k] * self.omega[k] * (t - self.show.grid.t_fight)
    }

    /// `theta_tail(k, t)`.
    pub fn theta_tail(&self, k: usize, t: f64) -> f64 {
        self.theta_open(k, t - self.show.grid.dur)
    }

    /// `solo_env(t)`: a pulse on each note of the winner's solo.
    pub fn solo_env(&self, t: f64) -> f64 {
        // past = [s for s in SOLO_T if s <= t]; past[-1]
        match self.solo_t.iter().rev().find(|&&s| s <= t) {
            Some(&last) if t < self.show.grid.t_reassemble + 0.3 => dmath::exp(-(t - last) / 0.16),
            _ => 0.0,
        }
    }

    /// `fx_clock(t)`: the effects clock: real time in the fight, held through the hit-stop, then running on.
    pub fn fx_clock(&self, t: f64) -> f64 {
        if t < self.t_ko {
            t
        } else if t < self.t_hs {
            self.t_ko
        } else {
            t - bf_core::show::HITSTOP
        }
    }

    /// `fighter(k, t)`: the picture state of fighter `k` at time `t`; each look adds its own fields.
    pub fn fighter(&self, k: usize, t: f64) -> BallState {
        let g = &self.show.grid;
        let mut st = BallState {
            p: [0.0, 0.0],
            vis: true,
            shield: 0.0,
            haste: 0.0,
            glow: 0.0,
            scale: 1.0,
            th: 0.0,
            blen: self.base_len[k],
            frets: self.frets0[k],
            level: 1,
            shattered: false,
            om: self.omega[k],
            sd: self.sd0[k],
            ext: self.rest[k],
            slow: 0.0,
            thr_k: 0.0,
            lift: 0.0,
            sink: 0.0,
            sx: None,
            sy: None,
            air_bias: 0.0,
            paint_h: None,
            extra: BTreeMap::new(),
        };
        if t < g.t_fight {
            let ro = self.rev_offset(k, t);
            st.p = [self.corner0[k][0] + ro[0], self.corner0[k][1] + ro[1]];
            st.th = self.theta_open(k, t);
            st.glow = 0.25 * g.beat_pulse(t, 0.12, 0.12);
            for l in &self.looks {
                l.state(self, k, &mut st, t, Phase::Rev, 0.0);
            }
            return st;
        }
        let run = &self.run;
        if t < self.t_hs {
            let i = self.sim_i(py_min(t, self.t_ko));
            st.p = run.p(i, k);
            st.th = run.f("TH", i, k).unwrap_or(0.0);
            // for key, rk, cast in _KEYS_FIGHT: if rk in RUN: st[key] = cast(RUN[rk][i, k])
            if let Some(v) = run.f("BL", i, k) {
                st.blen = v;
            }
            if let Some(v) = run.i("FR", i, k) {
                st.frets = v;
            }
            if let Some(v) = run.f("SH", i, k) {
                st.shield = v;
            }
            if let Some(v) = run.f("HA", i, k) {
                st.haste = v;
            }
            if let Some(v) = run.i("LV", i, k) {
                st.level = v;
            }
            if let Some(v) = run.f("OM", i, k) {
                st.om = v;
            }
            if let Some(v) = run.f("SPIN", i, k) {
                st.sd = v;
            }
            if let Some(v) = run.f("EXT", i, k) {
                st.ext = v;
            }
            if let Some(v) = run.f("SLW", i, k) {
                st.slow = v;
            }
            for l in &self.looks {
                l.state(self, k, &mut st, t, Phase::Fight, 0.0);
            }
            if t >= self.t_ko && k == self.lose {
                st.glow = 1.0;
            }
            return st;
        }
        let last = run.last();
        let blen_end = run.f("BL", last, k).unwrap_or(0.0);
        let fr_end = run.i("FR", last, k).unwrap_or(0);
        let gl = ease((t - g.t_glide) / (g.t_rev - g.t_glide));
        let ro = self.rev_offset(k, t);
        let rk = ss(g.t_rev - 0.3, g.t_rev + 0.2, t);
        let corner = [self.corner0[k][0] + ro[0] * rk, self.corner0[k][1] + ro[1] * rk];
        let rev_glow = 0.25 * g.beat_pulse(t, 0.12, 0.12) * ss(g.t_rev, g.t_rev + 0.3, t);
        let (arc_n, arc_a) = self.glide_arc[k];
        let arc_s = dmath::sin(dmath::PI * gl);
        let p_ko = self.p_ko[k];
        if k == self.win {
            let u = ease((t - self.t_hs) / 1.8);
            let bob = 0.06
                * dmath::sin(2.0 * dmath::PI * (t - self.t_hs) / (2.0 * g.bar))
                * ss(self.t_hs, self.t_hs + 1.5, t);
            // p = P_KO[k] + (centre - P_KO[k]) * u + np.array([bob, -bob])
            let p = [
                p_ko[0] + (0.0 - p_ko[0]) * u + bob,
                p_ko[1] + (0.0 - p_ko[1]) * u + -bob,
            ];
            st.p = [
                p[0] + (corner[0] - p[0]) * gl + arc_n[0] * arc_a * arc_s,
                p[1] + (corner[1] - p[1]) * gl + arc_n[1] * arc_a * arc_s,
            ];
            let spin_end = if run.has("SPIN") {
                run.f("SPIN", last, k).unwrap_or(1.0)
            } else {
                1.0
            };
            let th_cont = run.f("TH", last, k).unwrap_or(0.0) + spin_end * self.omega[k] * (t - self.t_hs);
            let tail = self.theta_tail(k, t);
            st.th = tail + wrap(th_cont - tail) * (1.0 - gl);
            st.blen = blen_end + (self.base_len[k] - blen_end) * gl;
            // int(round(fr_end + (FRETS0[k] - fr_end) * gl))
            st.frets = np::f64_to_i64(py::round_half_even(
                fr_end as f64 + (self.frets0[k] - fr_end) as f64 * gl,
            ));
            st.glow = py_max(
                rev_glow,
                (0.35 + 0.5 * self.solo_env(t)) * ss(self.t_hs, self.t_hs + 0.6, t) * (1.0 - gl),
            );
            st.level = if t < g.t_glide { 3 } else { 1 };
            st.sd = if gl < 0.5 { spin_end } else { self.sd0[k] };
            for l in &self.looks {
                l.state(self, k, &mut st, t, Phase::Tail, gl);
            }
            return st;
        }
        // the loser: shattered until the reassembly, then glides home
        st.p = [
            p_ko[0] + (corner[0] - p_ko[0]) * gl + arc_n[0] * arc_a * arc_s,
            p_ko[1] + (corner[1] - p_ko[1]) * gl + arc_n[1] * arc_a * arc_s,
        ];
        st.th = self.theta_tail(k, t);
        st.shattered = t < g.t_glide;
        st.vis = t >= g.t_glide - 0.12;
        st.glow = py_max(rev_glow, 0.9 * (1.0 - ss(g.t_glide - 0.12, g.t_glide + 0.5, t)));
        st.scale = 0.75 + 0.25 * ss(g.t_glide - 0.12, g.t_glide + 0.12, t);
        for l in &self.looks {
            l.state(self, k, &mut st, t, Phase::Tail, gl);
        }
        st
    }

    /// `camera(t)`: the camera between its keys, with the shake's offset.
    pub fn camera(&self, t: f64) -> Cam {
        let mut cam = interp_keys(&self.keys, t, None);
        let (dx, dy) = self.shk.offset(t);
        cam.cx += dx;
        cam.cy += dy;
        cam
    }

    /// `win_k(t)`.
    pub fn win_k(&self, t: f64) -> f64 {
        let g = &self.show.grid;
        ss(self.t_hs + 0.4, self.t_solo, t) * (1.0 - ss(g.t_glide, g.t_rev, t))
    }

    /// `spot_aim(t)`: the spot follows the champion during the victory lap.
    pub fn spot_aim(&self, t: f64) -> V3 {
        let g = &self.show.grid;
        let k = ss(self.t_hs, self.t_hs + 1.2, t) * (1.0 - ss(g.t_glide, g.t_rev, t));
        if k <= 0.0 {
            return SPOT_AIM;
        }
        let p = self.fighter(self.win, t).p;
        let w = scene::to_world(p[0], p[1], 0.0);
        // S.SPOT_AIM + (w * 0.55 - S.SPOT_AIM) * k
        std::array::from_fn(|c| SPOT_AIM[c] + (w[c] * 0.55 - SPOT_AIM[c]) * k)
    }

    /// `lights(t, cam)`: the spot, the two corner lights, the attack flares and the looks' own lights.
    pub fn lights(&self, t: f64, cam: &Cam) -> Lights {
        let mut l = Lights::new([0.36, 0.33, 0.44], [0.10, 0.08, 0.14]);
        l.view = cam.view();
        let d = v3::sub(SPOT, self.spot_aim(t));
        l.key("spot", v3::div(d, v3::norm(d)), [1.05, 1.0, 0.96]);
        let rk = 1.0 + 0.25 * self.show.grid.beat_pulse(t, 0.2, 0.12);
        l.point(self.scene.red_light, self.light_rgb[0].map(|c| c * 1.1 * rk), 2.6);
        l.point(self.scene.blue_light, self.light_rgb[1].map(|c| c * 1.1 * rk), 2.6);
        self.fx.fl.apply(&mut l, self.fx_clock(t));
        for look in &self.looks {
            look.light(self, &mut l, t);
        }
        l
    }
}

/// `AIM`: the camera's target.
const AIM: V3 = [0.12, 0.25, 0.0];
/// `SY`: the screen row of the optical centre.
const SY: f64 = 288.0;

/// `KEYS`: a gentle drift, a slight push-in for the final exchange, back for the win.
pub(super) fn camera_keys(t_ko: f64) -> Vec<Key> {
    [
        (0.0, 27.5, 0.0, 58.0),
        (8.0, 27.6, -0.8, 58.4),
        (20.0, 27.4, 0.7, 57.7),
        (32.0, 27.5, -0.6, 58.2),
        (44.0, 26.9, 0.0, 58.0),
        (t_ko, 26.7, 0.3, 58.0),
        (t_ko + 2.5, 27.3, 0.4, 58.4),
        (58.0, 27.4, 0.2, 58.2),
        (61.0, 27.5, 0.0, 58.0),
    ]
    .into_iter()
    .map(|(t, dist, yaw, pitch)| Key {
        t,
        target: AIM,
        dist,
        yaw,
        pitch,
        fov: 36.0,
        sy: SY,
    })
    .collect()
}

/// `_glide_arcs()`: when the straight glides home would cross, each bows sideways: the smallest bow that keeps
/// the balls `2 R + 0.15` apart. Returns `(normal, a)` for each fighter.
pub(super) fn glide_arcs(p_ko: [[f64; 2]; 2], corner0: [[f64; 2]; 2], win: usize) -> [([f64; 2], f64); 2] {
    let lose = 1 - win;
    let (pl, cl, cw) = (p_ko[lose], corner0[lose], corner0[win]);
    let dl = [cl[0] - pl[0], cl[1] - pl[1]];
    let dw = [cw[0] - 0.0, cw[1] - 0.0];
    let norm2 = |d: [f64; 2]| (d[0] * d[0] + d[1] * d[1]).sqrt();
    let perp = |d: [f64; 2]| {
        let n = norm2(d) + 1e-9;
        [-d[1] / n, d[0] / n]
    };
    let (mut nl, mut nw) = (perp(dl), perp(dw));
    let ml = [(pl[0] + cl[0]) / 2.0, (pl[1] + cl[1]) / 2.0];
    let mw = [cw[0] / 2.0, cw[1] / 2.0];
    if nl[0] * (ml[0] - mw[0]) + nl[1] * (ml[1] - mw[1]) < 0.0 {
        nl = [-nl[0], -nl[1]];
    }
    if nw[0] * (mw[0] - ml[0]) + nw[1] * (mw[1] - ml[1]) < 0.0 {
        nw = [-nw[0], -nw[1]];
    }
    // gs = np.linspace(0, 1, 201)
    let step = 1.0 / 200.0;
    let gs: Vec<f64> = (0..201)
        .map(|i| if i == 200 { 1.0 } else { i as f64 * step + 0.0 })
        .collect();
    let sin: Vec<f64> = gs.iter().map(|&g| dmath::sin(dmath::PI * g)).collect();
    let apart = 2.0 * scene::R_BALL + 0.15;
    let mut a = 0.0;
    // for a in np.arange(0.0, 2.51, 0.05): 51 values, 0.0 + i * 0.05
    for i in 0..51 {
        a = 0.0 + i as f64 * 0.05;
        let mut least = f64::INFINITY;
        for (&g, &s) in gs.iter().zip(&sin) {
            let (bow_l, bow_w) = (a * s, 0.5 * a * s);
            let bl = [pl[0] + dl[0] * g + nl[0] * bow_l, pl[1] + dl[1] * g + nl[1] * bow_l];
            let bw = [0.0 + dw[0] * g + nw[0] * bow_w, 0.0 + dw[1] * g + nw[1] * bow_w];
            least = np::minimum(least, np::hypot(bl[0] - bw[0], bl[1] - bw[1]));
        }
        if least >= apart {
            break;
        }
    }
    let mut out = [([0.0; 2], 0.0); 2];
    out[lose] = (nl, a);
    out[win] = (nw, 0.5 * a);
    out
}
