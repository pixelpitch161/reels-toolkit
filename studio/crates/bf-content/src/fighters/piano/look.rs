//! PIANO's look: the port of the `Look` half of `projects/ballfight/kit/fighters/piano.py` (v2, the PIANO ROLL).
//!
//! The cream paper roll on the apron (a canvas decal, so the ropes, shadows and attack lights sit on it) scrolling
//! toward the brass read bar, its coming rows of holes visible before they fire, holes flashing blue as they fire,
//! the supply spool turning; the note-shots as small glowing white and black keys (sharps are black) with short
//! trails; SUSTAIN's hanging keys pulsing over blue rings; 88 KEYS' full row and the paper running off in shreds,
//! then the new roll sliding in from the spool; key shards on hits; keys, hammers and paper in the K.O. shatter; the
//! VS card's panel (a keyboard and a punched roll).
//!
//! Every function is the Python one of the same name, with its operation order. Where the Python calls `math.sin`
//! or a numpy function the port calls `bf_core::dmath`; `np.hypot` is the plain root ([`bf_picture::np::hypot`]),
//! `math.hypot` is `dmath::py_hypot` (ruling M3-R35).
//!
//! The roster numbers the kit's look reads from `self.cfg` (the roll's range and margin, SUSTAIN's `out16` and
//! `hang_beats`, 88 KEYS' `reload_beats`) come from the `cfg` that `setup_as` is given, never from the picture's
//! own entry, so EUPHONIUM's copy can draw this look with PIANO's numbers on its own side.

use std::collections::BTreeMap;

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::roster::{FighterCfg, WeaponCfg};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{bayer_full, ease};
use bf_picture::fx::Emit;
use bf_picture::img::{Img, RgbF};
use bf_picture::look::{CanvasCtx, DefaultLook, DrawnBall, FrameCtx, Fx, Look, Payload, Spark};
use bf_picture::raster::Frame;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::BallState;
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{A, CANVAS_RIM, R_BALL as R, to_world};
use bf_picture::shadows::proj_point;
use bf_picture::{np, pil};

/// `PAPER`: cream punched paper.
const PAPER: V3 = [238.0, 228.0, 200.0];
/// `PAPER_DK`.
const PAPER_DK: V3 = [196.0, 184.0, 152.0];
/// `HOLE`.
const HOLE: V3 = [40.0, 34.0, 52.0];
/// `BRASS`.
const BRASS: V3 = [232.0, 188.0, 92.0];
/// `WOOD`.
const WOOD: V3 = [110.0, 64.0, 40.0];
/// `KEY_W`: ivory.
const KEY_W: V3 = [246.0, 242.0, 230.0];
/// `KEY_B`: ebony.
const KEY_B: V3 = [34.0, 30.0, 40.0];
/// `BLACK_PC`: the pitch classes of the black keys.
const BLACK_PC: [i64; 5] = [1, 3, 6, 8, 10];
/// `BAR`: the read bar: this far outside the fight wall (the apron).
const BAR: f64 = 0.05;
/// `DEPTH`: paper visible from the bar out to the spool.
const DEPTH: f64 = 0.40;
/// `SPOOL = BAR + DEPTH + 0.05`.
const SPOOL: f64 = BAR + DEPTH + 0.05;
/// `ROWGAP`: paper between two rows (one 8th of scroll).
const ROWGAP: f64 = 0.115;
/// `LAT = A - 0.18`: the paper's half-width along the side.
const LAT: f64 = A - 0.18;
/// `SHOT_Y`: note-shots fly at this height.
const SHOT_Y: f64 = 0.34;
/// `HOLE_L`: half the size of a punched hole along the scroll.
const HOLE_L: f64 = 0.036;
/// `HOLE_W`: half its size across.
const HOLE_W: f64 = 0.048;

/// `is_black(m)`: `int(m) % 12 in BLACK_PC`.
fn is_black(m: i64) -> bool {
    BLACK_PC.contains(&py::imod_py(m, 12))
}

/// What a volley of the piano is (`self.vols`' kind), or an arc's shot, whose volley the roll never listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Row,
    Runoff,
    Arc,
}

/// One live note-shot: a row of `shots()`: `(x, z, ang, pitch, sid, kind, age, hanging)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Shot {
    x: f64,
    z: f64,
    ang: f64,
    /// Its pitch (`None` for GLISSANDO's arc).
    m: Option<i64>,
    sid: i64,
    kind: Kind,
    age: f64,
    /// Seconds it has hung (a SUSTAIN mine), `None` while it flies.
    hang: Option<f64>,
}

/// A fired row of the roll: `self.rows[abs8] = (t, pitches, sustained)` (the flag is not read by any hook).
#[derive(Clone, Debug, PartialEq)]
struct Row {
    t: f64,
    pitches: Vec<i64>,
}

/// PIANO's look: `class Look(base.Look)` of `kit/fighters/piano.py`.
#[derive(Clone, Debug)]
pub struct PianoLook {
    /// Its side.
    k: usize,
    /// `T8 = T.BEAT / 2`.
    t8: f64,
    /// `reg = w.get('reg', cfg['register'])`.
    reg: (i64, i64),
    /// `margin = w.get('margin', 0.45)`.
    margin: f64,
    /// `ax`: the axis whose -A side holds the roll (the engine's).
    ax: usize,
    /// `out_t = ab1.get('out16', 2) * T.BEAT / 4` (set up as the kit does; no hook reads it).
    out_t: f64,
    /// `hang_t = ab1.get('hang_beats', 1) * T.BEAT` (likewise).
    hang_t: f64,
    /// `low.get('reload_beats', 0)`: the beats 88 KEYS leaves the roll bare.
    reload_beats: f64,
    /// What `setup_as` read of the roster, as the kit's keys name it (for [`Look::cfg_summary`]).
    read: String,
    /// `rows`: absolute 8th -> the row fired there.
    rows: BTreeMap<i64, Row>,
    /// `rows.values()` in the order the rows fired (the dict's order).
    fired: Vec<i64>,
    /// `vols`: volley -> `(t, kind)`.
    vols: BTreeMap<i64, (f64, Kind)>,
    /// `runoffs`: `(t, t_reload_end)` of each 88 KEYS.
    runoffs: Vec<(f64, f64)>,
    /// `row_t`: the rows' times, ascending.
    row_t: Vec<f64>,
}

impl Default for PianoLook {
    fn default() -> PianoLook {
        PianoLook::new()
    }
}

/// `e[i]` as a whole number, when the event has one there.
fn int(e: &Event, i: usize) -> Option<i64> {
    match e.fields.get(i.checked_sub(2)?)? {
        Value::Int(n) => Some(*n),
        Value::Float(x) => Some(np::f64_to_i64(*x)),
        Value::Bool(b) => Some(i64::from(*b)),
        _ => None,
    }
}

/// `e[i]` as a number.
fn num(e: &Event, i: usize) -> Option<f64> {
    match e.fields.get(i.checked_sub(2)?)? {
        Value::Float(x) => Some(*x),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

/// `e[i]` as text, or `""`.
fn text(e: &Event, i: usize) -> &str {
    match i.checked_sub(2).and_then(|j| e.fields.get(j)) {
        Some(Value::Str(s)) => s,
        _ => "",
    }
}

/// `tuple(e[i])`: the pitches of a `roll` or `runoff` event.
fn pitches(e: &Event, i: usize) -> Vec<i64> {
    match i.checked_sub(2).and_then(|j| e.fields.get(j)) {
        Some(Value::List(list)) => list
            .iter()
            .filter_map(|v| match v {
                Value::Int(m) => Some(*m),
                Value::Float(x) => Some(np::f64_to_i64(*x)),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// `tuple(int(v) for v in col)` as a Pillow ink.
fn ink(col: V3) -> [u8; 3] {
    [col[0] as u8, col[1] as u8, col[2] as u8]
}

/// `np.clip(col, 0, 255)`.
fn clip_col(c: V3) -> V3 {
    c.map(|v| np::clip(v, 0.0, 255.0))
}

/// `Q.mean(0)`: the rows added in order, then divided by their number.
fn mean(rows: &[V3]) -> V3 {
    let mut s = [0.0; 3];
    for r in rows {
        s = [s[0] + r[0], s[1] + r[1], s[2] + r[2]];
    }
    let n = rows.len() as f64;
    [s[0] / n, s[1] / n, s[2] / n]
}

/// `Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))`.
fn to_bytes(arr: &Img<f32>) -> Img<u8> {
    Img::from_vec(
        arr.w,
        arr.h,
        arr.c,
        arr.data.iter().map(|&v| np::clip_u8_f32(v)).collect(),
    )
}

/// `np.asarray(im).astype(np.float32)` back into `arr`.
fn from_bytes(arr: &mut Img<f32>, im: &Img<u8>) {
    for (v, &b) in arr.data.iter_mut().zip(&im.data) {
        *v = f32::from(b);
    }
}

/// `title_pattern(base)`: PIANO v2's VS-card panel: a keyboard along the top of the frame, below it a punched roll
/// (cream-tinted paper with rows of holes spelling a little melody, the holes lit blue). `base` is the panel's
/// gradient, changed in place.
pub fn title_pattern(base: &mut RgbF) {
    let kw = 20.0;
    // np.array([...]) * k, as numpy computes each first
    let ivory = [236.0 * 0.55, 232.0 * 0.55, 222.0 * 0.55];
    let cream = [238.0 * 0.12, 228.0 * 0.12, 200.0 * 0.12];
    let blue = [84.0 * 0.5, 156.0 * 0.5, 255.0 * 0.5];
    let lit = [150.0 * 0.85, 205.0 * 0.85, 255.0 * 0.85];
    let sharp = |c: f64| [0.0, 1.0, 3.0, 4.0, 5.0].contains(&py::fmod_py(c, 7.0));
    for y in 0..base.h {
        for x in 0..base.w {
            let (xx, yy) = (x as f64, y as f64);
            let kx = py::fmod_py(xx, kw);
            let col = (xx / kw).floor();
            let keys = yy < 96.0;
            let white = keys && kx > 1.5 && yy < 94.0;
            // C D F G A have a sharp to their right
            let (has_right, has_left) = (sharp(col), sharp(col - 1.0));
            let black = yy < 60.0 && ((kx > kw - 6.5 && has_right) || (kx < 6.5 && has_left));
            let paper = yy >= 100.0;
            let row = ((yy - 100.0) / 20.0).floor();
            let lane = (xx / 22.0).floor();
            // np.round(5 + 4 * np.sin(row * 0.9) + 2 * np.cos(row * 2.3)).astype(int)
            let melody = py::round_half_even(5.0 + 4.0 * dmath::sin(row * 0.9) + 2.0 * dmath::cos(row * 2.3));
            let pick = lane == melody
                || lane == melody + 3.0
                || (py::fmod_py(row, 2.0) == 0.0 && lane == melody - 4.0)
                || (py::fmod_py(row, 3.0) == 1.0 && lane == melody + 7.0);
            let hole = paper && py::fmod_py(yy - 100.0, 20.0) < 8.0 && py::fmod_py(xx, 22.0) < 11.0 && pick;
            let rim =
                paper && py::fmod_py(yy - 98.0, 20.0) < 12.0 && py::fmod_py(xx + 2.0, 22.0) < 15.0 && pick && !hole;
            for (ch, p) in base.px_mut(x, y).iter_mut().enumerate() {
                let b = *p;
                let mut out = if white { b * 0.45 + ivory[ch] } else { b };
                if black {
                    out = b * 0.2 + 12.0;
                }
                // the key bed's shadow line
                if keys && yy >= 94.0 {
                    out = b * 0.5;
                }
                if paper {
                    out = out * 0.80 + cream[ch];
                }
                if rim {
                    out = out * 0.5 + blue[ch];
                }
                if hole {
                    out = lit[ch] + out * 0.15;
                }
                *p = out;
            }
        }
    }
}

impl PianoLook {
    /// The look before a picture sets it up: its constants and its title-card hook work, and it has no rows.
    pub fn new() -> PianoLook {
        PianoLook {
            k: 0,
            t8: 0.0,
            reg: (48, 84),
            margin: 0.45,
            ax: 0,
            out_t: 0.0,
            hang_t: 0.0,
            reload_beats: 0.0,
            read: String::new(),
            rows: BTreeMap::new(),
            fired: Vec::new(),
            vols: BTreeMap::new(),
            runoffs: Vec::new(),
            row_t: Vec::new(),
        }
    }

    // ------------------------------------------------------------ geometry
    /// `lane(m, margin=None)`: where pitch `m` sits along the side.
    fn lane(&self, m: f64) -> f64 {
        let (lo, hi) = self.reg;
        let mg = self.margin;
        let u = (m - lo as f64) / py_max(1.0, (hi - lo) as f64);
        (A - mg) - 2.0 * (A - mg) * py_min(1.0, py_max(0.0, u))
    }

    /// `ring_pt(out, lat)`: (out: distance outside the fight wall, lat: along the side) -> ring frame `(sx, sz)`.
    fn ring_pt(&self, out: f64, lat: f64) -> (f64, f64) {
        if self.ax == 0 { (-A - out, lat) } else { (lat, -A - out) }
    }

    /// `P.up(*self.ring_pt(out, lat), y)`.
    fn ring_up(&self, out: f64, lat: f64, y: f64) -> V3 {
        let (sx, sz) = self.ring_pt(out, lat);
        up(sx, sz, y)
    }

    /// `local(sxp, szp)`: ring-frame pixels -> `(out, lat)` on the roll's side.
    fn local(&self, sxp: f64, szp: f64) -> (f64, f64) {
        if self.ax == 0 {
            (-(sxp + A), szp)
        } else {
            (-(szp + A), sxp)
        }
    }

    // ------------------------------------------------------------ the paper's state at t
    /// `paper(t)`: `(edge, gone)`: the paper's leading edge (out from the bar; 0 = threaded right up to the bar)
    /// and whether the old roll has run off (88 KEYS until the new one is threaded).
    fn paper(&self, p: &PictureCtx, t: f64) -> (f64, bool) {
        for &(t0, t1) in &self.runoffs {
            let tc = p.fx_clock(t);
            if t0 <= tc && tc < t1 {
                let u = (tc - t0) / py_max(1e-6, t1 - t0);
                if u < 0.35 {
                    // bare spool
                    return (DEPTH + 0.2, true);
                }
                // the new roll slides in from the spool
                return (DEPTH * (1.0 - ease((u - 0.35) / 0.65)), false);
            }
        }
        (0.0, false)
    }

    /// `visible_rows(t)`: `[(out, pitches, abs8, fired)]` of the rows on the visible paper at `t` (rows past the
    /// bar are under it).
    fn visible_rows(&self, p: &PictureCtx, t: f64) -> Vec<(f64, &[i64], i64, bool)> {
        let t8 = self.t8;
        let tc = p.fx_clock(t);
        let j0 = (tc / t8).floor() as i64 - 1;
        let mut out = Vec::new();
        // range(j0, j0 + int(DEPTH / ROWGAP) + 3)
        for j in j0..j0 + (DEPTH / ROWGAP) as i64 + 3 {
            let d = BAR + (j as f64 * t8 - tc) / t8 * ROWGAP;
            if d < BAR - ROWGAP * 0.6 || d > BAR + DEPTH {
                continue;
            }
            if let Some(r) = self.rows.get(&j) {
                out.push((d, r.pitches.as_slice(), j, r.t <= tc + 1e-6));
            }
        }
        out
    }

    // ------------------------------------------------------------ the shots at t (from the run)
    /// `shots(t, i)`: `[(x, z, ang, pitch, sid, kind, age, hanging)]` of the piano's live note-shots.
    fn shots(&self, p: &PictureCtx, t: f64, i: Option<usize>) -> Vec<Shot> {
        if !(p.show.grid.t_fight <= t && t < p.t_hs) {
            return Vec::new();
        }
        let i = i.unwrap_or_else(|| p.sim_i(t));
        let Some(k_rows) = p.run.frame_values("K", i) else {
            return Vec::new();
        };
        // held = {int(h[0]): h[1] for h in P.RUN['HOLD'][i]} if 'HOLD' in P.RUN else {}
        let mut held: BTreeMap<i64, f64> = BTreeMap::new();
        if let Some(Value::List(list)) = p.run.nested("HOLD", i) {
            for h in list {
                if let Value::List(h) = h
                    && let (Some(Value::Int(id)), Some(Value::Float(ts))) = (h.first(), h.get(1))
                {
                    held.insert(*id, *ts);
                }
            }
        }
        let tc = py_min(t, p.t_ko);
        let mut out = Vec::new();
        for row in k_rows.as_chunks::<7>().0 {
            if row[0].is_nan() || np::f64_to_i64(row[3]) != self.k as i64 {
                continue;
            }
            let (x, z, ang, sid, idx, vol) = (row[0], row[1], row[2], row[4], row[5], row[6]);
            let sid = np::f64_to_i64(sid);
            let Some(&(t0, kind)) = self.vols.get(&np::f64_to_i64(vol)) else {
                // GLISSANDO's arc (index = its step up the scale)
                out.push(Shot {
                    x,
                    z,
                    ang,
                    m: None,
                    sid,
                    kind: Kind::Arc,
                    age: 1.0,
                    hang: None,
                });
                continue;
            };
            // s it has hung (a SUSTAIN mine)
            let hang = match held.get(&sid) {
                Some(&ts) if tc >= ts - 1e-6 => Some(tc - ts),
                _ => None,
            };
            out.push(Shot {
                x,
                z,
                ang,
                m: Some(np::f64_to_i64(idx)),
                sid,
                kind,
                age: tc - t0,
                hang,
            });
        }
        out
    }

    /// `trail_pos(t, sid, back)`: where shot `sid` was `back` recording frames ago.
    fn trail_pos(&self, p: &PictureCtx, t: f64, sid: i64, back: usize) -> Option<(f64, f64)> {
        let j = p.sim_i(t).checked_sub(back)?;
        let k_rows = p.run.frame_values("K", j)?;
        // m = K[:, 4] == sid; r = K[np.argmax(m)]
        k_rows
            .as_chunks::<7>()
            .0
            .iter()
            .find(|row| row[4] == sid as f64)
            .map(|row| (row[0], row[1]))
    }

    // ------------------------------------------------------------ the note-shots
    /// `shot_y(kind, age, hang, t)`: the height a shot flies at.
    fn shot_y(&self, s: &Shot) -> f64 {
        if s.kind == Kind::Arc {
            return SHOT_Y;
        }
        let mut y = 0.05 + (SHOT_Y - 0.05) * py_min(1.0, py_max(0.0, s.age / 0.08));
        if let Some(hang) = s.hang {
            y += 0.03 * dmath::sin(2.0 * dmath::PI * 2.0 * hang);
        }
        y
    }

    /// `key_quad(x, z, ang, m, y)`: a key-shaped quad in the world.
    fn key_quad(&self, x: f64, z: f64, ang: f64, m: Option<i64>, y: f64) -> [V3; 4] {
        let black = m.is_some_and(is_black);
        let (lk, wk) = if black { (0.24, 0.09) } else { (0.30, 0.12) };
        let d = [dmath::cos(ang), dmath::sin(ang)];
        let n = [-d[1], d[0]];
        // c + d * Lk / 2 (+ or -) n * Wk / 2
        let pt = |sd: f64, sn: f64| {
            let q0 = x + sd * (d[0] * lk / 2.0) + sn * (n[0] * wk / 2.0);
            let q1 = z + sd * (d[1] * lk / 2.0) + sn * (n[1] * wk / 2.0);
            to_world(q0, q1, y)
        };
        [pt(1.0, 1.0), pt(1.0, -1.0), pt(-1.0, -1.0), pt(-1.0, 1.0)]
    }
}

impl Look for PianoLook {
    fn shards(&self) -> Vec<String> {
        // ['key'] * 8 + ['blackkey'] * 4 + ['hammer'] * 5 + ['paper'] * 4
        [("key", 8), ("blackkey", 4), ("hammer", 5), ("paper", 4)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_flash(&self) -> &[&str] {
        &[]
    }

    fn flash_hits(&self) -> &[&str] {
        &["slam", "wave", "shockwave"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &[]
    }

    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[("runoff", 0.8)]
    }

    /// v2's panel: `setup()` puts `title_pattern` into `title.PATTERNS` under the fighter's name.
    fn title_pattern(&self, base: &mut RgbF) {
        title_pattern(base);
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    fn cfg_summary(&self) -> Option<String> {
        Some(self.read.clone())
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`.
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        let beat = p.show.grid.beat;
        self.t8 = beat / 2.0;
        // w.get('reg', self.cfg['register']), w.get('margin', 0.45)
        let (reg, margin) = match &cfg.weapon {
            WeaponCfg::Roll(w) => (w.reg, Some(w.margin)),
            _ => (None, None),
        };
        let reg = reg.unwrap_or(cfg.register);
        self.reg = (i64::from(reg.0), i64::from(reg.1));
        self.margin = margin.unwrap_or(0.45);
        // the axis whose -A side holds the roll (the engine's)
        self.ax = if p.corner0[k][0] < 0.0 { 0 } else { 1 };
        let (ab1, low) = (&cfg.unlock1, &cfg.low);
        let out16 = ab1.get("out16").unwrap_or(2.0);
        let hang_beats = ab1.get("hang_beats").unwrap_or(1.0);
        self.out_t = out16 * beat / 4.0;
        self.hang_t = hang_beats * beat;
        self.reload_beats = low.get("reload_beats").unwrap_or(0.0);
        // the keys the kit's look reads from self.cfg (look.json `cfg_keys`): the two ability tables (88 KEYS'
        // `reload_beats`; SUSTAIN's `out16` and `hang_beats`), the register, the roll's margin and its range
        self.read = format!(
            "abilities.low=reload_beats:{:?}; abilities.unlock1=out16:{:?},hang_beats:{:?}; register={:?}; \
             weapon.margin={:?}; weapon.reg={:?}",
            self.reload_beats, out16, hang_beats, cfg.register, self.margin, self.reg
        );
        self.rows.clear();
        self.fired.clear();
        self.vols.clear();
        self.runoffs.clear();
        for e in &p.rec.events {
            if e.t > p.t_ko + 1e-6 {
                break;
            }
            // len(e) < 3 or e[2] != k
            if int(e, 2) != Some(k as i64) {
                continue;
            }
            if e.kind == "roll" {
                let (Some(vol), Some(abs8)) = (int(e, 3), int(e, 4)) else {
                    continue;
                };
                if self
                    .rows
                    .insert(
                        abs8,
                        Row {
                            t: e.t,
                            pitches: pitches(e, 6),
                        },
                    )
                    .is_none()
                {
                    self.fired.push(abs8);
                }
                self.vols.insert(vol, (e.t, Kind::Row));
            } else if e.kind == "runoff" {
                let Some(vol) = int(e, 3) else {
                    continue;
                };
                self.vols.insert(vol, (e.t, Kind::Runoff));
                self.runoffs.push((e.t, e.t + self.reload_beats * beat));
            }
        }
        // np.array(sorted(v[0] for v in self.rows.values()))
        self.row_t = self.rows.values().map(|r| r.t).collect();
        self.row_t.sort_by(f64::total_cmp);
    }

    // ------------------------------------------------------------ events
    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "runoff" || e.kind == "slam" {
            Some(1.6)
        } else {
            None
        }
    }

    fn rope(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "runoff" {
            return Some(0.8);
        }
        // super().rope(e), with this look's rope_kinds
        if e.kind == "ability" {
            let kind = text(e, 4);
            return self
                .rope_kinds()
                .iter()
                .find(|(name, _)| *name == kind)
                .map(|&(_, s)| s);
        }
        DefaultLook.rope(p, e)
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        if text(e, 7) == "key"
            && let Some(att) = int(e, 2).and_then(|a| usize::try_from(a).ok()).filter(|&a| a < 2)
        {
            // self.P.COL[e[2]] * 0.5 + 120
            return (p.col[att].map(|c| c * 0.5 + 120.0), 6, 2.2, 0.2);
        }
        // super().spark(e): the piano keeps the base class's `spark_big`
        DefaultLook.spark(p, e)
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        if text(e, 7) == "key" { Some((6.0, 0.22)) } else { None }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        let k = self.k;
        let t = e.t;
        if e.kind == "roll" {
            // the row fires: a blue light at the read bar
            let ps = pitches(e, 6);
            // float(np.mean([self.lane(m) for m in ps]))
            let lanes: Vec<f64> = ps.iter().map(|&m| self.lane(m as f64)).collect();
            let lat = np::mean(&lanes);
            let pos = self.ring_up(BAR, lat, 0.06);
            fx.fl
                .add(t, pos, p.fl_col[k], 1.0, 0.30 + 0.06 * ps.len() as f64, 0.03, 0.14, 0.0);
        } else if e.kind == "hit" && text(e, 7) == "key" {
            let (Some(x), Some(z)) = (num(e, 5), num(e, 6)) else {
                return;
            };
            let pos = up(x, z, SHOT_Y + 0.1);
            fx.bu.add(t, pos, p.col[k], 0.45, None, false);
            fx.pa.emit(
                t,
                pos,
                "keyshards",
                &Emit {
                    n: Some(3),
                    scale: 0.8,
                    ..Emit::default()
                },
            );
            fx.fl.add(t, pos, p.fl_col[k], 1.0, 0.4, 0.03, 0.16, 0.0);
        } else if e.kind == "runoff" {
            // 88 KEYS: the whole roll lights, the paper runs off
            for lat in [-LAT * 0.7, 0.0, LAT * 0.7] {
                fx.fl
                    .add(t, self.ring_up(BAR, lat, 0.1), p.fl_col[k], 2.4, 1.0, 0.03, 0.45, 0.0);
            }
            for j in 0..9 {
                let j = f64::from(j);
                let lat = -LAT + 2.0 * LAT * (j + 0.5) / 9.0;
                fx.pa.emit(
                    t + 0.02 * j,
                    self.ring_up(BAR + DEPTH * 0.5, lat, 0.05),
                    "shreds",
                    &Emit {
                        n: Some(5),
                        col: Some(vec![PAPER, PAPER_DK, [250.0, 244.0, 226.0]]),
                        dir: [0.0, 1.0, 0.0],
                        scale: 1.2,
                        ..Emit::default()
                    },
                );
            }
            fx.bu.add(t, self.ring_up(BAR, 0.0, 0.2), p.col[k], 1.6, None, true);
        } else if e.kind == "sustain" {
            // the pedal goes down: the roll glows
            fx.fl.add(
                t,
                self.ring_up(BAR + 0.2, 0.0, 0.2),
                p.fl_col[k],
                2.0,
                0.5,
                0.03,
                0.4,
                0.0,
            );
        } else if e.kind == "ability" && text(e, 4) == "arc" {
            let st = p.fighter(k, t);
            fx.fl
                .add(t, up(st.p[0], st.p[1], R), p.fl_col[k], 1.6, 0.6, 0.03, 0.3, 0.0);
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        fx.pa.emit(
            p.t_ko,
            pos,
            "keyshards",
            &Emit {
                n: Some(26),
                spread: Some(180.0),
                speed: Some((0.8, 2.4)),
                ..Emit::default()
            },
        );
    }

    // ------------------------------------------------------------ canvas: the roll, the mines' rings, shot lights
    fn canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let k = self.k;
        let t = c.t;
        let rim = CANVAS_RIM;
        // P.COL[k] / 255.0
        let col = p.col[k].map(|v| v / 255.0);
        let near = |out: f64, lat: f64| out > -0.02 && out < SPOOL + 0.08 && lat.abs() < LAT + 0.05;
        let any_near = (0..ic.len()).any(|j| {
            let (out, lat) = self.local(c.sxp[j], c.szp[j]);
            near(out, lat)
        });
        if any_near {
            let bayer = bayer_full();
            let (edge, gone) = self.paper(p, t);
            let tc = p.fx_clock(t);
            let (lo, hi) = self.reg;
            // range(lo - lo % 12 + 12, hi + 1, 12): a faint line under every C
            let guides: Vec<f64> = (0..)
                .map(|n| lo - py::imod_py(lo, 12) + 12 + 12 * n)
                .take_while(|&cn| cn < hi + 1)
                .map(|cn| self.lane(cn as f64))
                .collect();
            let visible: Vec<(f64, Vec<f64>)> = self
                .visible_rows(p, t)
                .into_iter()
                .filter(|&(d, ..)| d >= BAR + edge)
                .map(|(d, ps, ..)| (d, ps.iter().map(|&m| self.lane(m as f64)).collect()))
                .collect();
            // the rows whose fire still lights the bar: (a, their lanes)
            let mut lit: Vec<(f64, Vec<f64>)> = Vec::new();
            if !self.row_t.is_empty() {
                // j = np.searchsorted(self.row_t, tc, 'right') - 1; for jj in range(max(0, j - 1), j + 1)
                let j = self.row_t.partition_point(|&v| v <= tc) as i64 - 1;
                for jj in (j - 1).max(0)..j + 1 {
                    let te = self.row_t[jj as usize];
                    if 0.0 <= tc - te && tc - te < 0.2 {
                        // r = [v for v in self.rows.values() if v[0] == te][0]
                        let r = self
                            .fired
                            .iter()
                            .map(|abs8| &self.rows[abs8])
                            .find(|r| r.t == te)
                            .expect("row_t holds the rows' own times");
                        let a = dmath::exp(-(tc - te) / 0.07);
                        lit.push((a, r.pitches.iter().map(|&m| self.lane(m as f64)).collect()));
                    }
                }
            }
            // 88 KEYS: every lane at once
            let flashes: Vec<f64> = self
                .runoffs
                .iter()
                .filter(|&&(t0, _)| 0.0 <= tc - t0 && tc - t0 < 0.5)
                .map(|&(t0, _)| dmath::exp(-(tc - t0) / 0.18))
                .collect();
            // SUSTAIN: the pedal down: the bar and the paper glow softly
            let sus = if c.in_fight && p.run.has("ROLL") {
                p.run.row("ROLL", c.i, k).and_then(|r| r.get(1).copied()).unwrap_or(0.0)
            } else {
                0.0
            };
            for (j, light) in ic.iter_mut().enumerate() {
                let (o, la) = self.local(c.sxp[j], c.szp[j]);
                if !near(o, la) {
                    continue;
                }
                let bay = bayer.px(c.xs[j], c.ys[j])[0];
                let paper = o >= BAR + edge && o <= BAR + DEPTH && la.abs() <= LAT && !gone;
                let mut mul = [1.0; 3];
                let mut add = [0.0; 3];
                // the paper (a cream decal: albedo x this = cream), faint octave guide lines, a scroll texture
                // (whole 8ths: loop-safe)
                let ph = (o - BAR) / ROWGAP + t / self.t8;
                let tex = 0.96 + 0.04 * (py::fmod_py(ph * 0.5, 1.0) * 2.0 + bay * 0.5).floor();
                if paper {
                    for ch in 0..3 {
                        mul[ch] = PAPER[ch] / rim[ch] * tex * 1.12;
                    }
                    for &g in &guides {
                        if (la - g).abs() < 0.012 {
                            for ch in 0..3 {
                                mul[ch] = PAPER_DK[ch] / rim[ch] * 0.95;
                            }
                        }
                    }
                    // the holes: rows scrolling toward the bar (their notes are what will play)
                    for (d, lanes) in &visible {
                        if (o - d).abs() >= HOLE_L {
                            continue;
                        }
                        for &ln in lanes {
                            if (la - ln).abs() < HOLE_W {
                                for ch in 0..3 {
                                    mul[ch] = HOLE[ch] / rim[ch] * 0.9;
                                }
                                // passing the bar: blue
                                if *d < BAR + 0.06 {
                                    for ch in 0..3 {
                                        add[ch] += col[ch] * 1.4;
                                    }
                                }
                            }
                        }
                    }
                }
                // the brass read bar along the side, lit where holes fire
                if (o - (BAR - 0.014)).abs() < 0.022 && la.abs() <= LAT + 0.04 {
                    for ch in 0..3 {
                        mul[ch] = BRASS[ch] / rim[ch] * 1.15;
                    }
                }
                let mut gl = 0.0;
                for (a, lanes) in &lit {
                    for &ln in lanes {
                        gl += a
                            * np::clip(1.0 - (la - ln).abs() / 0.12, 0.0, 1.0)
                            * np::clip(1.0 - (o - BAR).abs() / 0.14, 0.0, 1.0);
                    }
                }
                for a in &flashes {
                    let on = if la.abs() <= LAT { 1.0 } else { 0.0 };
                    gl += a * 1.6 * on * np::clip(1.0 - (o - BAR).abs() / 0.3, 0.0, 1.0);
                }
                let g = (np::clip(gl, 0.0, 1.6) * 3.0 + bay * 0.9).floor() / 3.0;
                for ch in 0..3 {
                    add[ch] += g * (col[ch] * 1.3 + 0.2);
                }
                // the supply spool: a wooden roller with brass end caps, turning (stripes step with the scroll)
                let spool = (o - SPOOL).abs() < 0.035 && la.abs() <= LAT + 0.02;
                let caps = (o - SPOOL).abs() < 0.04 && (la.abs() - (LAT + 0.05)).abs() < 0.04;
                let stripe = py::fmod_py(ph * 2.0, 1.0) < 0.5;
                if spool {
                    for ch in 0..3 {
                        mul[ch] = if gone {
                            WOOD[ch] * 0.7 / rim[ch]
                        } else if stripe {
                            WOOD[ch] * 1.15 / rim[ch]
                        } else {
                            WOOD[ch] * 0.85 / rim[ch]
                        };
                    }
                    if !gone && (o - SPOOL).abs() < 0.02 {
                        for ch in 0..3 {
                            mul[ch] = PAPER_DK[ch] / rim[ch] * 0.95;
                        }
                    }
                }
                if caps {
                    for ch in 0..3 {
                        mul[ch] = BRASS[ch] / rim[ch];
                    }
                }
                if sus > 0.0 && paper {
                    let sk = py_min(1.0, sus / 0.25);
                    for ch in 0..3 {
                        add[ch] += sk * 0.18 * col[ch];
                    }
                }
                for ch in 0..3 {
                    light[ch] = light[ch] * mul[ch] + add[ch];
                }
            }
        }
        // the mines' rings and the shots' light on the canvas
        if c.in_fight {
            for s in self.shots(p, c.t, Some(c.i)) {
                let pulse = s.hang.map(|hang| 0.6 + 0.4 * dmath::sin(2.0 * dmath::PI * 4.0 * hang));
                for (j, light) in ic.iter_mut().enumerate() {
                    let rd = np::hypot(c.sxp[j] - s.x, c.szp[j] - s.z);
                    match pulse {
                        Some(pulse) => {
                            let ring = np::clip(1.0 - (rd - 0.16).abs() / 0.035, 0.0, 1.0) * pulse;
                            for ch in 0..3 {
                                light[ch] += ring * 0.9 * (col[ch] + 0.2);
                            }
                        }
                        None => {
                            let spot = np::clip(1.0 - rd / 0.3, 0.0, 1.0);
                            let spot = spot * spot;
                            for ch in 0..3 {
                                light[ch] += spot * 0.35 * col[ch];
                            }
                        }
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------ shadows of the shots
    fn shadows_free(&self, p: &PictureCtx, f: &FrameCtx<'_>, spot: V3) -> Vec<Vec<(f64, f64)>> {
        self.shots(p, f.t, if f.in_fight { Some(f.i) } else { None })
            .iter()
            .map(|s| {
                let q = self.key_quad(s.x, s.z, s.ang, s.m, self.shot_y(s));
                proj_point(&q, spot, 0.93)
            })
            .collect()
    }

    // ------------------------------------------------------------ the note-shots (painter items)
    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if !f.in_fight {
            return Vec::new();
        }
        self.shots(p, f.t, Some(f.i))
            .into_iter()
            .map(|s| {
                let depth = f.cam.depth(&[to_world(s.x, s.z, SHOT_Y)])[0];
                let payload: Payload = Box::new(s);
                (depth, payload)
            })
            .collect()
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let Some(s) = payload.downcast_ref::<Shot>() else {
            return;
        };
        let k = self.k;
        let cam = f.cam;
        let mut ang = s.ang;
        if let Some(hang) = s.hang {
            // a hanging key turns slowly
            ang = hang * 3.0 + s.sid as f64;
        }
        let y = self.shot_y(s);
        // a short trail back along its flight (where it was 1 and 2 frames ago)
        if s.hang.is_none() {
            let mut pts = vec![(s.x, s.z)];
            for b in [1, 2] {
                if let Some(q) = self.trail_pos(p, f.t, s.sid, b) {
                    let last = pts[pts.len() - 1];
                    if dmath::py_hypot(q.0 - last.0, q.1 - last.1) < 0.6 {
                        pts.push(q);
                    }
                }
            }
            if pts.len() > 1 {
                let world: Vec<V3> = pts.iter().map(|q| to_world(q.0, q.1, y)).collect();
                let sp = cam.wp(&world);
                let c = ink(clip_col(p.col[k].map(|v| v * 0.7 + 70.0)));
                pil::line(&mut fr.pil, &sp, &c, 2);
            }
        }
        let q = self.key_quad(s.x, s.z, ang, s.m, y);
        let black = s.m.is_some_and(is_black);
        let base = if black { KEY_B } else { KEY_W };
        let mut c = p.shade(f.l, &[[0.0, 1.0, 0.0]], &[mean(&q)], &[base], None, 0.5, 0.0, 5.0)[0];
        let glow = match s.hang {
            None => 0.35,
            Some(hang) => 0.5 + 0.3 * dmath::sin(2.0 * dmath::PI * 4.0 * hang),
        };
        let lift = if black { 10.0 } else { 30.0 };
        for (v, own) in c.iter_mut().zip(p.col[k]) {
            *v = *v * (1.0 - 0.3 * glow) + own * 0.3 * glow + lift * glow;
        }
        fr.poly(cam, &q, clip_col(c), true);
        let mut sc = cam.wp(&q);
        let rim = ink(clip_col(p.col[k].map(|v| v * 0.55 + if black { 120.0 } else { 90.0 })));
        // blue-lit edges (a black key reads)
        sc.push(sc[0]);
        pil::line(&mut fr.pil, &sc, &rim, 1);
    }

    fn glows(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, _drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let k = self.k;
        for s in self.shots(p, f.t, Some(f.i)) {
            let y = self.shot_y(&s);
            let sp = f.cam.wp(&[to_world(s.x, s.z, y)])[0];
            let (r, a) = match s.hang {
                None => (8.0, 0.30),
                Some(hang) => (10.0, 0.42 + 0.2 * dmath::sin(2.0 * dmath::PI * 4.0 * hang)),
            };
            fr.glow(sp.0, sp.1, r, p.col[k].map(|v| v * 0.8 + 50.0), a);
        }
    }

    // ------------------------------------------------------------ shards
    fn draw_shard(
        &self,
        _p: &PictureCtx,
        arr: &mut Img<f32>,
        cam: &Cam,
        kind: &str,
        pos: V3,
        rot: f64,
        _size: f64,
        curl: f64,
    ) {
        let (sx, sz) = (pos[0], pos[2]);
        let d = [dmath::cos(rot), dmath::sin(rot)];
        let nn = [-d[1], d[0]];
        if kind == "key" || kind == "blackkey" {
            let (lk, wk) = if kind == "key" { (0.26, 0.10) } else { (0.18, 0.07) };
            // c0 + d * Lk / 2 * a1 + nn * Wk / 2 * a2
            let quad: Vec<V3> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .iter()
                .map(|&(a1, a2)| {
                    to_world(
                        sx + d[0] * lk / 2.0 * a1 + nn[0] * wk / 2.0 * a2,
                        sz + d[1] * lk / 2.0 * a1 + nn[1] * wk / 2.0 * a2,
                        pos[1],
                    )
                })
                .collect();
            let col = if kind == "key" { KEY_W } else { KEY_B };
            layer_blend(arr, &[screen(cam, &quad)], col, 1.0, None);
            return;
        }
        if kind == "hammer" {
            // a felt hammer on its shank
            let ends = [
                to_world(sx - d[0] * 0.14, sz - d[1] * 0.14, pos[1]),
                to_world(sx + d[0] * 0.06, sz + d[1] * 0.06, pos[1]),
            ];
            let sp = screen(cam, &ends);
            let mut im = to_bytes(arr);
            pil::line(&mut im, &sp, &[200u8, 170, 120], 1);
            let (x, y) = sp[1];
            pil::ellipse(
                &mut im,
                (x - 2.0, y - 2.0, x + 2.0, y + 2.0),
                Some(&[150u8, 40, 46]),
                None,
                1,
            );
            from_bytes(arr, &im);
            return;
        }
        // a curl of punched paper
        let pts: Vec<V3> = (0..6)
            .map(|j| {
                let a = rot + f64::from(j) * 0.6 * (1.0 + curl);
                to_world(sx + 0.12 * dmath::cos(a), sz + 0.12 * dmath::sin(a), pos[1])
            })
            .collect();
        let sp = screen(cam, &pts);
        let mut im = to_bytes(arr);
        pil::line(&mut im, &sp, &ink(PAPER), 2);
        from_bytes(arr, &im);
    }

    // ------------------------------------------------------------ checks
    fn focus_points(&self, _p: &PictureCtx, _t: f64, _st: &BallState) -> Vec<(V3, String)> {
        let mut out = Vec::new();
        for lat in [-LAT, 0.0, LAT] {
            let (sx, sz) = self.ring_pt(BAR, lat);
            out.push((to_world(sx, sz, 0.02), "read bar".to_string()));
        }
        let (sx, sz) = self.ring_pt(SPOOL, 0.0);
        out.push((to_world(sx, sz, 0.05), "spool".to_string()));
        out
    }
}
