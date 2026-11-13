//! SYNTH's look: the `Look` class of its reference kit module (`tools/studio_export/older/fighters/synth.py`), #31
//! v3's layers (`render.py`) and draw code (`styles.py`), hook by hook.
//!
//! What it draws: the patch cables between the two balls carrying the waveform of the synth's level (sine, square,
//! saw, scrolling, swelling under LFO), unplugging cables sliding home, snapped cables whipping back, plugs in flight
//! trailing their cable, voltage pulses running down the cables (a bright dot and a glow), SAWTOOTH's lashing saw
//! arms (a glow at each tip), the plugged jacks over the balls; the effects of its hits and events; its spark sprays
//! (none for a pulse, a spray where a plug misses), its flashbulbs and hit glows, its chromatic fringes (big hits and
//! snaps, in event order, after the grade), WHITE NOISE's static ring on the canvas and its heat shimmer, the
//! champion's groove rings; its Pixel Pitch sprite tipped toward the camera by its `pitch`; its K.O. spray and
//! shards (keys, black keys, knobs, jacks); its VS-card panel (#31's `_circuit`) and close-up.
//!
//! The cables lie under everything of the painter pass (depth `1e9`) and the plugged jacks over all of it (`-1e9`),
//! #31's order. What the cables item works out (its drawing, the jacks and the glows) is kept for the frame in
//! `FrameCtx.memo`, where the module keeps `_frame`.

use std::any::Any;
use std::collections::BTreeMap;
use std::sync::OnceLock;

use bf_core::dmath::{self, PI};
use bf_core::event::{Event, Value};
use bf_core::py::{fmod_py, py_max, py_min};
use bf_core::rng::Generator as Gen;
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{self, H, W};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::{Img, RgbF};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Fx, Look, Payload, Pic, Spark};
use bf_picture::raster::Frame;
use bf_picture::ring::state::Wave;
use bf_picture::ring::{PictureCtx, layer_blend, owner, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::sprite::{Mesh, render_sprite_pitched};
use bf_picture::title::layout::Layout;
use bf_picture::title::patterns::synth as pattern;
use bf_picture::{np, pil};

// ---------------------------------------------------------------- #31 render.py: its pieces
/// `CYAN_SPARK`.
const CYAN_SPARK: [V3; 3] = [[150.0, 240.0, 255.0], [230.0, 252.0, 255.0], [70.0, 228.0, 255.0]];
/// `CABLES_DEPTH`: the cables lie under everything of the painter pass.
const CABLES_DEPTH: f64 = 1.0e9;
/// `JACKS_DEPTH`: the plugged jacks over everything of it.
const JACKS_DEPTH: f64 = -1.0e9;
/// The cables' dark sheath.
const SHEATH: V3 = [10.0, 26.0, 40.0];
/// The cables' bright core.
const CORE: V3 = [236.0, 252.0, 255.0];

/// A cable's waveform: `WAVEFORM.get(level, 'sine')`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WaveKind {
    Sine,
    Square,
    Saw,
}

/// `WAVEFORM.get(level, 'sine')`.
fn waveform(level: i64) -> WaveKind {
    match level {
        2 => WaveKind::Square,
        3 => WaveKind::Saw,
        _ => WaveKind::Sine,
    }
}

// ---------------------------------------------------------------- #31 styles.py
/// `np.linspace(0, 1, m)`: `k * (1 / (m - 1))`, the last value 1 exactly.
fn linspace01(m: usize) -> Vec<f64> {
    if m < 2 {
        return vec![0.0; m];
    }
    let step = 1.0 / (m - 1) as f64;
    let mut s: Vec<f64> = (0..m).map(|k| k as f64 * step).collect();
    s[m - 1] = 1.0;
    s
}

/// `wave_shape(x, kind)`: one period of the cable's waveform at phase `x`, in -1..1.
fn wave_shape(x: f64, kind: WaveKind) -> f64 {
    let x = fmod_py(x, 1.0);
    match kind {
        WaveKind::Square => {
            if x < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        WaveKind::Saw => 2.0 * x - 1.0,
        WaveKind::Sine => dmath::sin(2.0 * PI * x),
    }
}

/// `cable_points(a, b, kind, phase, amp, wl=0.34, bow, y0=0.42, y1=0.42, n_per=10, wobble, t)`: the world polyline
/// of a cable from `a` to `b` (ring frame) carrying its waveform, bowed sideways by `bow` at the middle.
#[allow(clippy::too_many_arguments)] // the Python signature
fn cable_points(
    a: [f64; 2],
    b: [f64; 2],
    kind: WaveKind,
    phase: f64,
    amp: f64,
    bow: f64,
    wobble: f64,
    t: f64,
) -> Vec<V3> {
    let (wl, y0, y1, n_per) = (0.34, 0.42, 0.42, 10.0);
    let d = [b[0] - a[0], b[1] - a[1]];
    let l = dmath::np_hypot(d[0], d[1]) + 1e-9;
    let u = [d[0] / l, d[1] / l];
    let n = [-u[1], u[0]];
    // m = max(8, int(L / wl * n_per))
    let m = ((l / wl * n_per) as i64).max(8) as usize;
    let a_ = amp * (1.0 + wobble * dmath::sin(2.0 * PI * t * 4.0));
    linspace01(m)
        .into_iter()
        .map(|s| {
            // env = np.sin(np.pi * s) ** 0.6: the wave fades out at the plugs
            let env = dmath::pow(dmath::sin(PI * s), 0.6);
            let lat = a_ * env * wave_shape(s * l / wl - phase, kind) + bow * dmath::sin(PI * s);
            let px = a[0] + d[0] * s + n[0] * lat;
            let pz = a[1] + d[1] * s + n[1] * lat;
            scene::to_world(px, pz, y0 + (y1 - y0) * s)
        })
        .collect()
}

/// `jack_quad(p, ang, y=0.46, L_=0.26, w=0.07)`: a patch-cable plug, barrel and tip, pointing along `ang`.
fn jack_quad(c: [f64; 2], ang: f64) -> ([V3; 4], [V3; 4]) {
    let (y, l, w) = (0.46, 0.26, 0.07);
    let d = [dmath::cos(ang), dmath::sin(ang)];
    let n = [-d[1], d[0]];
    // c + d * L_ * k + n * w * m; a subtraction is the addition of the negated product, bit for bit
    let at = |k: f64, m: f64| -> V3 {
        let q: [f64; 2] = std::array::from_fn(|i| c[i] + d[i] * l * k + n[i] * w * m);
        scene::to_world(q[0], q[1], y)
    };
    (
        [at(-0.5, -1.0), at(0.15, -1.0), at(0.15, 1.0), at(-0.5, 1.0)],
        [at(0.15, -0.45), at(0.55, -0.3), at(0.55, 0.3), at(0.15, 0.45)],
    )
}

/// `saw_arm_points(p, ang, length, teeth=5, amp=0.12, y=0.5, r0=0.62)`: a cable gone saw-wave, lashing out of the
/// ball along `ang`.
fn saw_arm_points(p: [f64; 2], ang: f64, length: f64) -> Vec<V3> {
    let (teeth, amp, y, r0) = (5i64, 0.12, 0.5, 0.62);
    let d = [dmath::cos(ang), dmath::sin(ang)];
    let n = [-d[1], d[0]];
    // p + d * r +- n * amp
    let at = |r: f64, side: f64| -> V3 {
        let q: [f64; 2] = std::array::from_fn(|i| p[i] + d[i] * r + n[i] * (side * amp));
        scene::to_world(q[0], q[1], y)
    };
    let mut out = Vec::with_capacity(3 * teeth as usize);
    for j in 0..teeth {
        let a = r0 + (length - r0) * j as f64 / teeth as f64;
        let b = r0 + (length - r0) * (j + 1) as f64 / teeth as f64;
        out.extend([at(a, -1.0), at(b, 1.0), at(b, -1.0)]);
    }
    out
}

/// `line_blend(arr, pts, col, alpha, width)`: alpha-blends a screen polyline onto the `float32` picture (`alpha` a
/// Python float: the mask times it stays `float32`).
fn line_blend(arr: &mut Img<f32>, pts: &[(f64, f64)], col: V3, alpha: f64, width: u32) {
    if pts.len() < 2 || alpha <= 0.0 {
        return;
    }
    let mut m: Img<u8> = Img::new(W, H, 1);
    pil::line(&mut m, pts, &[255], width);
    let alpha = alpha as f32;
    for (i, &b) in m.data.iter().enumerate() {
        if b == 0 {
            continue;
        }
        let mk = f32::from(b) / 255.0f32 * alpha;
        for (v, &c) in arr.data[i * 3..i * 3 + 3].iter_mut().zip(&col) {
            let a = f64::from(*v);
            *v = (a + (c - a) * f64::from(mk)) as f32;
        }
    }
}

/// `np.clip(col, 0, 255)` of a colour.
fn clip255(c: V3) -> V3 {
    c.map(|v| np::clip(v, 0.0, 255.0))
}

/// `int(x)` of a time that is never negative, as a seed.
fn seed_of(x: f64) -> u64 {
    np::f64_to_i64(x).max(0) as u64
}

/// `e[7]` of a hit, or `""`.
fn how(e: &Event) -> &str {
    match e.fields.get(5) {
        Some(Value::Str(s)) if e.kind == "hit" => s,
        _ => "",
    }
}

/// The indices Python's slice `start:stop` takes on an axis of length `len`, for `start >= 0`.
fn py_slice(start: i64, stop: i64, len: usize) -> std::ops::Range<usize> {
    let len = len as i64;
    let stop = if stop < 0 { (stop + len).max(0) } else { stop.min(len) };
    let start = start.min(len);
    start as usize..stop.max(start) as usize
}

/// The SYNTH's model, compiled in.
fn mesh() -> &'static Mesh {
    static MESH: OnceLock<Mesh> = OnceLock::new();
    MESH.get_or_init(|| {
        let json = super::MESH.expect("the SYNTH's model is compiled in");
        Mesh::from_json(json).expect("the SYNTH's model reads")
    })
}

// ---------------------------------------------------------------- its run state (setup)
/// A plug of `self.PLUGS`.
#[derive(Clone, Debug)]
struct Plug {
    t0: f64,
    o: [f64; 2],
    ang: f64,
    speed: f64,
    /// When it landed or missed (`None`: still flying at the K.O.).
    t1: Option<f64>,
}

/// A cable of `self.CABLES`.
#[derive(Clone, Copy, Debug)]
struct Cable {
    t0: f64,
    t1: f64,
    /// 1: snapped; 2: still in at the K.O.; otherwise unplugged.
    reason: i64,
}

/// A voltage pulse of `self.VOLTS`: `(t0, cable id, n, travel time)`.
#[derive(Clone, Copy, Debug)]
struct Volt {
    t0: f64,
    cid: i64,
    tr: f64,
}

/// A lash of `self.LASHES`.
#[derive(Clone, Copy, Debug)]
struct Lash {
    t0: f64,
    t1: f64,
    aim: f64,
    n: i64,
    len: f64,
    period: f64,
    sweep: f64,
    spread: f64,
    lag: f64,
}

/// `Fight.lash_angle(L, j, t)`: arm `j`'s angle at time `t`.
fn lash_angle(l: &Lash, j: i64, t: f64) -> f64 {
    let ph = fmod_py((t - l.t0) / l.period + j as f64 * l.lag, 1.0);
    l.aim + (j as f64 - (l.n - 1) as f64 / 2.0) * l.spread + l.sweep * (ph - 0.5)
}

/// A cable's state in a frame: `live_cables`' `'in'`, `'retract'` or `'snap'`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CableState {
    In,
    Retract,
    Snap,
}

/// One drawing step of `draw_cables`, in its order.
#[derive(Clone, Debug)]
enum Op {
    /// `line_blend(arr, pts, col, alpha, width)`.
    Line(Vec<(f64, f64)>, V3, f64, u32),
    /// A voltage pulse's bright dot at `(xi, yi)`.
    Dot(usize, usize),
}

/// A frame's glow: `(x, y, r, col, alpha)`.
type Glow = (f64, f64, f64, V3, f64);

/// What `draw_cables` works out for a frame: its drawing steps, the jacks to draw over the painter pass
/// (`(point, angle)`) and the glows (the module's `_frame`).
#[derive(Clone, Debug, Default)]
struct CableFrame {
    ops: Vec<Op>,
    jacks: Vec<([f64; 2], f64)>,
    glows: Vec<Glow>,
}

/// A painter item of this look.
enum Item {
    Cables,
    Jacks,
}

// ---------------------------------------------------------------- the look
/// SYNTH's look.
#[derive(Clone, Debug)]
pub struct SynthLook {
    /// Its side.
    k: usize,
    /// `weapon.volt_speed`.
    volt_speed: f64,
    /// `pitch`: the model tipped toward the camera.
    pitch: f64,
    /// `view`: (yaw centre, yaw swing, roll).
    view: (f64, f64, f64),
    /// `self.CABLES`.
    cables: BTreeMap<i64, Cable>,
    /// `self.PLUGS`, in the dict's order.
    plugs: Vec<(i64, Plug)>,
    /// `self.VOLTS`.
    volts: Vec<Volt>,
    /// `self.VOLT_T`.
    volt_t: Vec<f64>,
    /// `self.LASHES`.
    lashes: Vec<Lash>,
}

/// A look not set up yet (the card's close-up asks before any fight: Python's `Look.__init__` reads the roster at
/// once) draws the SYNTH's own: its roster's pitch 28 and view (10, 20, 0).
impl Default for SynthLook {
    fn default() -> SynthLook {
        SynthLook {
            k: 0,
            volt_speed: 9.0,
            pitch: 28.0,
            view: (10.0, 20.0, 0.0),
            cables: BTreeMap::new(),
            plugs: Vec::new(),
            volts: Vec::new(),
            volt_t: Vec::new(),
            lashes: Vec::new(),
        }
    }
}

impl SynthLook {
    /// `live_cables(t)`: `[(id, state, k)]`, `k` the progress (0 to 1) of a retract or a snap.
    fn live_cables(&self, p: &PictureCtx, t: f64) -> Vec<(i64, CableState, f64)> {
        let tc = p.fx_clock(t);
        let mut out = Vec::new();
        for (&cid, c) in &self.cables {
            let dur = if c.reason == 1 { 0.3 } else { 0.2 };
            if c.t0 <= tc && tc < c.t1 {
                out.push((cid, CableState::In, 0.0));
            } else if c.t1 <= tc && tc < c.t1 + dur && t < p.t_hs + 0.3 {
                let state = if c.reason == 1 {
                    CableState::Snap
                } else {
                    CableState::Retract
                };
                out.push((cid, state, (tc - c.t1) / dur));
            }
        }
        out
    }

    /// `draw_cables(arr, cam, t, fs, glows)` without the drawing: its steps in order, its jacks and its glows.
    fn cable_frame(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> CableFrame {
        let (sy, t, cam, fs) = (self.k, f.t, f.cam, f.fs);
        let mut out = CableFrame::default();
        if !(p.show.grid.t_fight <= t && t < p.t_hs + 0.3) || fs[sy].shattered || !fs[sy].vis {
            return out;
        }
        let col = p.col[sy];
        let (ps, pe) = (fs[sy].p, fs[1 - sy].p);
        let d = [pe[0] - ps[0], pe[1] - ps[1]];
        let dist = dmath::np_hypot(d[0], d[1]) + 1e-9;
        let u = [d[0] / dist, d[1] / dist];
        let kind = waveform(fs[sy].level);
        let wob = if fs[sy].haste > 0.0 { 0.6 } else { 0.0 };
        let tc = p.fx_clock(t);
        let phase = tc * 3.2;
        let cab = self.live_cables(p, t);
        let n_in = cab.iter().filter(|c| c.1 == CableState::In).count();
        let base = clip255(col.map(|c| c * 0.85 + 40.0));
        let a_s = [ps[0] + u[0] * R * 0.95, ps[1] + u[1] * R * 0.95];
        let a_e = [pe[0] - u[0] * R * 0.92, pe[1] - u[1] * R * 0.92];
        let line = |ops: &mut Vec<Op>, s: &[(f64, f64)], c: V3, a: f64, w: u32| ops.push(Op::Line(s.to_vec(), c, a, w));
        let mut j_in = 0usize;
        for &(cid, state, k) in &cab {
            let pts = match state {
                CableState::In => {
                    // bow = (j_in - (n_in - 1) / 2) * 0.24
                    let bow = (j_in as f64 - (n_in as f64 - 1.0) / 2.0) * 0.24;
                    j_in += 1;
                    out.jacks.push((a_e, dmath::atan2(d[1], d[0])));
                    cable_points(a_s, a_e, kind, phase + cid as f64 * 0.37, 0.1, bow, wob, tc)
                }
                CableState::Retract => {
                    let ek = config::ease(k);
                    let e_ = [a_e[0] + (a_s[0] - a_e[0]) * ek, a_e[1] + (a_s[1] - a_e[1]) * ek];
                    cable_points(a_s, e_, kind, phase, 0.05, 0.0, 0.0, 0.0)
                }
                CableState::Snap => {
                    // snapped: both halves whip back
                    let mid = [(a_s[0] + a_e[0]) / 2.0, (a_s[1] + a_e[1]) / 2.0];
                    let ek = config::ease(k);
                    let amp = 0.1 * (1.0 - k);
                    let m1 = [mid[0] + (a_s[0] - mid[0]) * ek, mid[1] + (a_s[1] - mid[1]) * ek];
                    let m2 = [mid[0] + (a_e[0] - mid[0]) * ek, mid[1] + (a_e[1] - mid[1]) * ek];
                    let p1 = cable_points(a_s, m1, kind, phase, amp, 0.0, 0.0, 0.0);
                    let p2 = cable_points(m2, a_e, kind, phase, amp, 0.0, 0.0, 0.0);
                    for pq in [p1, p2] {
                        let s = cam.wp(&pq);
                        line(&mut out.ops, &s, SHEATH, 0.8 * (1.0 - k), 3);
                        line(&mut out.ops, &s, base, 1.0 - k, 1);
                    }
                    continue;
                }
            };
            let s = cam.wp(&pts);
            line(&mut out.ops, &s, SHEATH, 0.9, 4); // the cable's dark sheath
            line(&mut out.ops, &s, base, 1.0, 2); // its lit waveform
            line(&mut out.ops, &s, CORE, 0.55, 1);
        }
        // plugs in flight, trailing their cable from the synth
        for (_, pl) in &self.plugs {
            let t1 = pl.t1.unwrap_or(p.t_ko);
            if pl.t0 <= tc && tc < t1 {
                let v = [dmath::cos(pl.ang), dmath::sin(pl.ang)];
                let q = [
                    pl.o[0] + v[0] * pl.speed * (tc - pl.t0),
                    pl.o[1] + v[1] * pl.speed * (tc - pl.t0),
                ];
                let s = cam.wp(&cable_points(a_s, q, kind, phase, 0.05, 0.0, 0.0, 0.0));
                line(&mut out.ops, &s, SHEATH, 0.8, 3);
                line(&mut out.ops, &s, base, 0.9, 1);
                out.jacks.push((q, pl.ang));
            }
        }
        // voltage pulses: bright dots running down the plugged cables (they reach the enemy as it is hit)
        if !self.volt_t.is_empty() {
            let j0 = self.volt_t.partition_point(|&v| v < tc - 0.6);
            let j1 = self.volt_t.partition_point(|&v| v <= tc);
            let ids_in: Vec<i64> = cab.iter().filter(|c| c.1 == CableState::In).map(|c| c.0).collect();
            let glow_col = col.map(|c| c * 0.6 + 100.0);
            for v in self.volts.get(j0..j1.max(j0)).unwrap_or(&[]) {
                let uu = (tc - v.t0) / py_max(v.tr, 1e-3);
                let Some(jj) = ids_in.iter().position(|&c| c == v.cid) else {
                    continue;
                };
                if !(0.0 <= uu && uu <= 1.0) {
                    continue;
                }
                let bow = (jj as f64 - (ids_in.len() as f64 - 1.0) / 2.0) * 0.24;
                let pts = cable_points(a_s, a_e, kind, phase + v.cid as f64 * 0.37, 0.1, bow, wob, tc);
                let last = pts.len() - 1;
                let q = pts[last.min((uu * last as f64) as usize)];
                let sq = cam.wp(&[q])[0];
                let (xi, yi) = (np::f64_to_i64(sq.0), np::f64_to_i64(sq.1));
                if 1 <= xi && xi < W as i64 - 2 && 1 <= yi && yi < H as i64 - 2 {
                    out.ops.push(Op::Dot(xi as usize, yi as usize));
                }
                out.glows.push((sq.0, sq.1, 6.0, glow_col, 0.55));
            }
        }
        // SAWTOOTH: the cables go saw-wave and lash side to side
        let arm_col = clip255(col.map(|c| c * 0.6 + 110.0));
        let tip_col = col.map(|c| c * 0.7 + 70.0);
        for l in &self.lashes {
            if l.t0 <= tc && tc < l.t1 {
                for j in 0..l.n {
                    let a = lash_angle(l, j, tc);
                    let reach = R + l.len;
                    let s = cam.wp(&saw_arm_points(ps, a, reach));
                    line(&mut out.ops, &s, SHEATH, 0.9, 4);
                    line(&mut out.ops, &s, arm_col, 1.0, 2);
                    line(&mut out.ops, &s, CORE, 0.6, 1);
                    let tip = [ps[0] + reach * dmath::cos(a), ps[1] + reach * dmath::sin(a)];
                    let st = cam.wp(&[scene::to_world(tip[0], tip[1], 0.5)])[0];
                    out.glows.push((st.0, st.1, 7.0, tip_col, 0.5));
                }
            }
        }
        out
    }

    /// The frame's cables, worked out once (`FrameCtx.memo`); worked out afresh if the slot holds another look's.
    fn with_frame<T>(&self, p: &PictureCtx, f: &FrameCtx<'_>, use_it: impl FnOnce(&CableFrame) -> T) -> T {
        let slot = f.memo[self.k].get_or_init(|| Box::new(self.cable_frame(p, f)) as Box<dyn Any + Send + Sync>);
        match slot.downcast_ref::<CableFrame>() {
            Some(cf) => use_it(cf),
            None => use_it(&self.cable_frame(p, f)),
        }
    }
}

impl Look for SynthLook {
    fn shards(&self) -> Vec<String> {
        let mut out = vec!["key".to_string(); 7];
        out.extend(std::iter::repeat_n("bkey".to_string(), 4));
        out.extend(std::iter::repeat_n("knob".to_string(), 4));
        out.extend(std::iter::repeat_n("jack".to_string(), 4));
        out
    }

    fn flash_hits(&self) -> &[&str] {
        &["slam", "wave", "scratch", "lash"]
    }

    /// None: its fringes are drawn in `after_grade`, hits and snaps in event order.
    fn fringe_hits(&self) -> &[&str] {
        &[]
    }

    fn quiet_hits(&self) -> &[&str] {
        &["volt"]
    }

    fn title_pattern_with(&self, base: &mut RgbF, card: &Layout) {
        pattern::pattern(base, card);
    }

    // ------------------------------------------------------------ setup
    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// The run's cables, plugs, pulses and lashes for side `k`, with `cfg`'s numbers (#31 render.py 293-321).
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.volt_speed = cfg
            .raw
            .get("weapon")
            .and_then(|w| w.get("volt_speed"))
            .and_then(|v| v.as_f64())
            // a weapon without one (Python raises) keeps the SYNTH's own, so any fighter's numbers make the look
            .unwrap_or(9.0);
        self.pitch = cfg
            .raw
            .get("pitch")
            .map_or(0.0, |v| v.as_f64().expect("a roster number"));
        let view = |i: usize| {
            cfg.raw
                .get("view")
                .and_then(|v| v.get(i))
                .and_then(|x| x.as_f64())
                .expect("a roster entry has a view")
        };
        self.view = (view(0), view(1), view(2));
        let ev = &p.rec.events;
        let mut cables = BTreeMap::new();
        let mut plugs: Vec<(i64, Plug)> = Vec::new();
        for e in ev.iter().filter(|e| e.who() == Some(k)) {
            match e.kind.as_str() {
                "plug_fire" => {
                    let pl = Plug {
                        t0: e.t,
                        o: [e.f(4), e.f(5)],
                        ang: e.f(6),
                        speed: e.f(7),
                        t1: None,
                    };
                    // plugs[e[3]] = dict(...): a key already there keeps its place
                    let id = e.i(3);
                    match plugs.iter_mut().find(|(i, _)| *i == id) {
                        Some(slot) => slot.1 = pl,
                        None => plugs.push((id, pl)),
                    }
                }
                "plug" | "plug_miss" => {
                    let id = e.i(3);
                    if let Some((_, pl)) = plugs.iter_mut().find(|(i, _)| *i == id) {
                        pl.t1 = Some(e.t);
                        if e.kind == "plug" {
                            let c = Cable {
                                t0: e.t,
                                t1: p.t_ko,
                                reason: 2,
                            };
                            cables.insert(id, c);
                        }
                    }
                }
                "unplug" => {
                    if let Some(c) = cables.get_mut(&e.i(3)) {
                        c.t1 = e.t;
                        c.reason = e.i(4);
                    }
                }
                _ => {}
            }
        }
        let mut volts = Vec::new();
        for e in ev.iter().filter(|e| e.kind == "volt" && e.who() == Some(k)) {
            let i = p.sim_i(e.t);
            let (a, b) = (p.run.p(i, 1 - k), p.run.p(i, k));
            let dist = dmath::np_hypot(a[0] - b[0], a[1] - b[1]);
            volts.push(Volt {
                t0: e.t,
                cid: e.i(3),
                tr: dist / self.volt_speed,
            });
        }
        self.volt_t = volts.iter().map(|v| v.t0).collect();
        self.volts = volts;
        self.cables = cables;
        self.plugs = plugs;
        self.lashes = ev
            .iter()
            .filter(|e| e.kind == "lash" && e.who() == Some(k))
            .map(|e| Lash {
                t0: e.t,
                t1: e.t + e.f(6),
                aim: e.f(3),
                n: e.i(4),
                len: e.f(5),
                period: e.f(7),
                sweep: e.f(8),
                spread: e.f(9),
                lag: e.f(10),
            })
            .collect();
    }

    /// The card's big ball: the model tipped toward the camera.
    fn title_closeup(&self, r: u32) -> Option<Img<u8>> {
        let (yc, _, roll) = self.view;
        Some(render_sprite_pitched(mesh(), r, yc, roll, self.pitch))
    }

    // ------------------------------------------------------------ the ball
    /// The Pixel Pitch synth tipped toward the camera by its `pitch`.
    fn sprite(&self, r: u32, yaw: f64, roll: f64) -> Option<Img<u8>> {
        Some(render_sprite_pitched(mesh(), r, yaw, roll, self.pitch))
    }

    // ------------------------------------------------------------ events -> camera, sparks, glows
    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match e.kind.as_str() {
            "hit" => match how(e) {
                "scratch" => Some(1.4),
                "lash" => Some(0.8),
                "slam" => Some(2.5),
                "wave" => Some(2.0),
                _ => None,
            },
            "snap" => Some(1.0),
            "slam" => Some(1.6),
            _ => None,
        }
    }

    /// No spray for a pulse; lash and plug sprays mid-sized.
    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let col = p.col[owner(e).unwrap_or(0)].map(|c| c * 0.6 + 100.0);
        let (n, sp, life) = match how(e) {
            "volt" => (0, 1.8, 0.18),
            "slam" | "wave" | "scratch" => (16, 3.2, 0.3),
            "lash" | "plug" => (9, 2.6, 0.24),
            _ => (5, 1.8, 0.18),
        };
        (col, n, sp, life)
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        matches!(how(e), "slam" | "wave" | "scratch" | "lash").then_some((12.0, 0.35))
    }

    // ------------------------------------------------------------ effects (#31 _build_fx, its events)
    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        let t0 = e.t;
        let col = &p.col;
        let cyan = || Some(CYAN_SPARK.to_vec());
        match e.kind.as_str() {
            "hit" => {
                let Some(att) = e.who() else { return };
                let (x, z) = (e.f(5), e.f(6));
                let pos = scene::to_world(x, z, R);
                match how(e) {
                    "scratch" => {
                        // a rope slam
                        fx.bu.add(t0, pos, col[att], 1.2, None, true);
                        fx.fl.add(t0, pos, col[att], 1.8, 0.8, 0.03, 0.3, 0.0);
                        let emit = Emit {
                            n: Some(8),
                            ..Emit::default()
                        };
                        fx.pa.emit(t0, pos, "shards", &emit);
                        let mark = Mark {
                            size: 0.45,
                            n: 4,
                            life: 4.0,
                            seed: seed_of(t0 * 100.0),
                            ..Mark::new(MarkKind::Crack, x, z)
                        };
                        fx.mk.add(t0, mark);
                    }
                    "plug" => {
                        fx.bu.add(t0, pos, col[att], 0.5, None, false);
                        let emit = Emit {
                            n: Some(5),
                            col: cyan(),
                            ..Emit::default()
                        };
                        fx.pa.emit(t0, pos, "sparks", &emit);
                    }
                    "volt" => {
                        let emit = Emit {
                            n: Some(1),
                            col: cyan(),
                            scale: 0.7,
                            ..Emit::default()
                        };
                        fx.pa.emit(t0, pos, "sparks", &emit);
                    }
                    "lash" => {
                        fx.bu.add(t0, pos, col[att], 0.8, None, true);
                        fx.fl.add(t0, pos, col[att], 1.4, 0.6, 0.03, 0.24, 0.0);
                        let emit = Emit {
                            n: Some(6),
                            ..Emit::default()
                        };
                        fx.pa.emit(t0, pos, "glitch", &emit);
                        let mark = Mark {
                            size: 0.4,
                            angle: dmath::atan2(z, x) + 1.2,
                            n: 1,
                            col: [150.0, 236.0, 255.0],
                            life: 3.0,
                            seed: seed_of(t0 * 100.0),
                            ..Mark::new(MarkKind::Claw, x, z)
                        };
                        fx.mk.add(t0, mark);
                    }
                    "slam" => fx.bu.add(t0, pos, col[att], 1.6, None, true),
                    "wave" => {
                        fx.bu.add(t0, pos, col[att], 1.1, None, true);
                        let emit = Emit {
                            n: Some(10),
                            ..Emit::default()
                        };
                        fx.pa.emit(t0, pos, "glitch", &emit);
                    }
                    _ => {}
                }
            }
            "snap" => {
                let Some(who) = e.who() else { return };
                let pos = scene::to_world(e.f(4), e.f(5), R);
                let sparks = Emit {
                    n: Some(14),
                    col: cyan(),
                    ..Emit::default()
                };
                fx.pa.emit(t0, pos, "sparks", &sparks);
                let glitch = Emit {
                    n: Some(6),
                    ..Emit::default()
                };
                fx.pa.emit(t0, pos, "glitch", &glitch);
                fx.bu.add(t0, pos, col[who], 0.9, None, true);
                fx.fl.add(t0, pos, col[who], 1.6, 0.8, 0.03, 0.3, 0.0);
            }
            "leash" => {
                let emit = Emit {
                    n: Some(2),
                    col: cyan(),
                    scale: 0.8,
                    ..Emit::default()
                };
                fx.pa.emit(t0, scene::to_world(e.f(3), e.f(4), R), "sparks", &emit);
            }
            "ability" if matches!(e.fields.get(2), Some(Value::Str(s)) if s == "wave") => {
                let Some(who) = e.who() else { return };
                let q = p.run.p(p.sim_i(t0), who);
                let c = col[who].map(|v| v * 0.9);
                fx.fl
                    .add(t0, scene::to_world(q[0], q[1], 1.0), c, 2.6, 0.75, 0.03, 0.5, 0.0);
                let emit = Emit {
                    n: Some(20),
                    spread: Some(180.0),
                    speed: Some((0.8, 2.0)),
                    ..Emit::default()
                };
                fx.pa.emit(t0, scene::to_world(q[0], q[1], R), "glitch", &emit);
            }
            _ => {}
        }
    }

    /// The synth shatters into glitch and cyan sparks.
    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let glitch = Emit {
            n: Some(24),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "glitch", &glitch);
        let sparks = Emit {
            n: Some(20),
            col: Some(CYAN_SPARK.to_vec()),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
    }

    // ------------------------------------------------------------ the canvas (per pixel)
    /// The champion's solo: groove rings of light round it (#31 render.py solo_rings).
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let t = c.t;
        // P.WIN != self.k or not C.t >= P.T_SOLO - 0.01
        if p.win != self.k || !(t >= p.t_solo - 0.01) {
            return;
        }
        let beat = p.show.grid.beat;
        let bayer = config::bayer_full();
        let st = &c.fs[p.win];
        let tint = p.col[p.win].map(|v| v / 255.0 * 0.9 + 0.25);
        for &(bt, _m, d) in &p.solos[p.win] {
            let t0 = p.t_solo + bt * beat;
            if !(t0 <= t && t < t0 + 0.6) {
                continue;
            }
            let reach = if d >= 0.75 { 1.9 } else { 1.1 };
            let u = t - t0;
            let r = R + py_min(reach, 6.0 * u);
            let a = if u < reach / 6.0 {
                1.0
            } else {
                py_max(0.0, 1.0 - (u - reach / 6.0) / 0.25)
            };
            for (j, light) in ic.iter_mut().enumerate() {
                let rd = dmath::np_hypot(c.sxp[j] - st.p[0], c.szp[j] - st.p[1]);
                let inside = if rd < r + 0.05 { 1.0 } else { 0.0 };
                let band = np::clip(1.0 - (rd - r).abs() / 0.16, 0.0, 1.0) * inside;
                let by = bayer.data[c.ys[j] * W + c.xs[j]];
                let band = (band * 3.0 + by * 0.9).floor() / 3.0;
                let v = band * a * 0.5;
                for ch in 0..3 {
                    light[ch] += v * tint[ch];
                }
            }
        }
    }

    /// One of its rings on the canvas; WHITE NOISE's a ring of static (#31 render.py 1207-1221).
    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        let t = c.t;
        let shock = w.kind == "shockwave";
        let wave = w.kind == "wave";
        let wdt = if shock { 0.16 } else { 0.32 };
        let k_ = (1.0 - w.r / w.rmax) * if shock { 1.0 } else { 1.3 };
        let bayer = config::bayer_full();
        let tint = p.col[w.owner].map(|v| v / 255.0 * 1.4 + if wave { 0.35 } else { 0.15 });
        // math.floor(t * 30) * 1.7
        let tf = (t * 30.0).floor() * 1.7;
        for (j, light) in ic.iter_mut().enumerate() {
            let (sx, sz) = (c.sxp[j], c.szp[j]);
            let mut rd = dmath::np_hypot(sx - w.cx, sz - w.cz);
            if wave {
                let ang = dmath::atan2(sz - w.cz, sx - w.cx);
                rd = rd + 0.08 * dmath::sin(31.0 * ang + 70.0 * t) + 0.05 * dmath::sin(13.0 * ang - 40.0 * t);
            }
            let band = np::clip(1.0 - (rd - w.r).abs() / wdt, 0.0, 1.0);
            let by = bayer.data[c.ys[j] * W + c.xs[j]];
            let band = (band * 3.0 + by * 0.9).floor() / 3.0;
            let v = band * k_ * 0.9;
            for ch in 0..3 {
                light[ch] += v * tint[ch];
            }
            if wave {
                let h = fmod_py((sx * 23.0).floor() * 7.3 + (sz * 23.0).floor() * 3.1 + tf, 1.0);
                if h > 0.72 && (rd - w.r).abs() < wdt * 1.6 {
                    for v in light.iter_mut() {
                        *v += 0.9 * k_;
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------ the painter: the cables and the jacks
    fn items_free(&self, _p: &PictureCtx, _f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let cables: Payload = Box::new(Item::Cables);
        let jacks: Payload = Box::new(Item::Jacks);
        vec![(CABLES_DEPTH, cables), (JACKS_DEPTH, jacks)]
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        match payload.downcast_ref::<Item>() {
            Some(Item::Cables) => self.with_frame(p, f, |cf| {
                // arr = F.fr.array(); draw_cables(...); F.fr.set_array(arr): bytes in, the same bytes out where
                // nothing was drawn
                if cf.ops.is_empty() {
                    return;
                }
                let mut arr = fr.array();
                for op in &cf.ops {
                    match op {
                        Op::Line(s, c, a, w) => line_blend(&mut arr, s, *c, *a, *w),
                        Op::Dot(xi, yi) => {
                            for y in yi - 1..yi + 2 {
                                for x in xi - 1..xi + 2 {
                                    let i = (y * W + x) * 3;
                                    for (v, c) in arr.data[i..i + 3].iter_mut().zip(CORE) {
                                        // arr * 0.3 (float32) + core * 0.7 (float64)
                                        *v = (f64::from(*v * 0.3f32) + c * 0.7) as f32;
                                    }
                                }
                            }
                            let i = (yi * W + xi) * 3;
                            arr.data[i..i + 3].fill(255.0);
                        }
                    }
                }
                fr.set_array(&arr, None);
            }),
            Some(Item::Jacks) => self.with_frame(p, f, |cf| {
                let tip_col = clip255(p.col[self.k].map(|c| c * 0.7 + 70.0));
                for &(q, ang) in &cf.jacks {
                    let (barrel, tip) = jack_quad(q, ang);
                    fr.poly(f.cam, &barrel, [40.0, 44.0, 56.0], true);
                    fr.poly(f.cam, &tip, tip_col, true);
                }
            }),
            None => {}
        }
    }

    /// The sparks where a plug misses (#31 sparks(): its `plug_miss` events).
    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        let (t, cam) = (f.t, f.cam);
        let (n, sp, life) = (4, 1.2, 0.16);
        let lit = clip255([180.0, 230.0, 245.0].map(|c: f64| c + 60.0));
        for i in p.events_between(t - 0.32, t, &["plug_miss"]) {
            let e = p.event(i);
            if e.who() != Some(self.k) {
                continue;
            }
            let dt = t - e.t;
            let (x, z) = (e.f(4), e.f(5));
            if dt > life {
                continue;
            }
            let mut rng = Gen::new(i as u64);
            let fade = 1.0 - dt / life;
            let keep = (1.0 - fade) as f32;
            for _ in 0..n {
                let a = rng.uniform(0.0, 2.0 * PI);
                let v = sp * rng.uniform(0.4, 1.0);
                let q = [x + v * dt * dmath::cos(a), z + v * dt * dmath::sin(a)];
                let hgt = R + rng.uniform(-0.2, 0.3) + 2.5 * dt - 6.0 * dt * dt;
                let s = cam.wp(&[scene::to_world(q[0], q[1], py_max(0.02, hgt))])[0];
                let (xi, yi) = (np::f64_to_i64(s.0), np::f64_to_i64(s.1));
                if 0 <= xi && xi < W as i64 - 1 && 0 <= yi && yi < H as i64 - 1 {
                    let j = (yi as usize * W + xi as usize) * 3;
                    for (v, c) in arr.data[j..j + 3].iter_mut().zip(lit) {
                        *v = np::clip(f64::from(*v * keep) + c * fade, 0.0, 255.0) as f32;
                    }
                }
            }
        }
    }

    /// #31 draw_shards: keys, black keys (flat bars), knobs and jacks (dots).
    fn draw_shard(
        &self,
        p: &PictureCtx,
        arr: &mut Img<f32>,
        cam: &Cam,
        kind: &str,
        pos: V3,
        rot: f64,
        _size: f64,
        _curl: f64,
    ) {
        let (sx, sz) = (pos[0], pos[2]);
        let d = [dmath::cos(rot), dmath::sin(rot)];
        let nn = [-d[1], d[0]];
        match kind {
            "key" | "bkey" => {
                let (lk, wk, c) = if kind == "key" {
                    (0.30, 0.09, [240.0, 238.0, 230.0])
                } else {
                    (0.22, 0.06, [34.0, 32.0, 40.0])
                };
                let q4: Vec<V3> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                    .iter()
                    .map(|&(a1, a2)| {
                        let qx = sx + d[0] * lk / 2.0 * a1 + nn[0] * wk / 2.0 * a2;
                        let qz = sz + d[1] * lk / 2.0 * a1 + nn[1] * wk / 2.0 * a2;
                        scene::to_world(qx, qz, pos[1])
                    })
                    .collect();
                let sp = screen(cam, &q4);
                let k = 0.8 + 0.2 * dmath::cos(rot * 2.0);
                layer_blend(arr, &[sp], c.map(|v: f64| v * k), 1.0, None);
            }
            "knob" | "jack" => {
                let sp = screen(cam, &[scene::to_world(sx, sz, pos[1])])[0];
                let (xi, yi) = (np::f64_to_i64(sp.0), np::f64_to_i64(sp.1));
                let col = p.col[self.k];
                let c = if kind == "jack" {
                    col.map(|v| v * 0.7 + 70.0)
                } else {
                    [60.0, 58.0, 70.0]
                };
                // arr[max(0, yi - 1):yi + 1, max(0, xi - 1):xi + 1] = c
                for y in py_slice((yi - 1).max(0), yi + 1, H) {
                    for x in py_slice((xi - 1).max(0), xi + 1, W) {
                        let i = (y * W + x) * 3;
                        for (v, &cv) in arr.data[i..i + 3].iter_mut().zip(&c) {
                            *v = cv as f32;
                        }
                    }
                }
                if kind == "knob" && 0 <= yi && yi < H as i64 && 0 <= xi && xi < W as i64 {
                    let i = (yi as usize * W + xi as usize) * 3;
                    for (v, &cv) in arr.data[i..i + 3].iter_mut().zip(&col) {
                        *v = (cv * 0.8 + 50.0) as f32;
                    }
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ lens, glows, post
    /// WHITE NOISE's front (#31 shimmers()).
    fn shimmer(&self, _p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, w: &Wave) {
        let c = scene::to_world(w.cx, w.cz, 0.05);
        img.shimmer_world(cam, c, w.r, 0.4, 3.0, t, 1.0 - w.r / w.rmax, w.owner as f64);
    }

    /// The voltage pulses' and the lash tips' glows (#31 draw_cables' `glows`).
    fn glows(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, _drawn: &[DrawnBall]) {
        self.with_frame(p, f, |cf| {
            for &(x, y, r, c, a) in &cf.glows {
                fr.glow(x, y, r, c, a);
            }
        });
    }

    /// Its chromatic fringes, its big hits' and a snapped cable's, in event order (#31 post_fx).
    fn after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, _drawn: &[DrawnBall]) {
        let t = f.t;
        for i in p.events_between(t - 0.12, t, &["hit", "snap"]) {
            let e = p.event(i);
            if e.who() != Some(self.k) {
                continue;
            }
            let snap = e.kind == "snap";
            if !snap && !matches!(how(e), "slam" | "wave" | "scratch") {
                continue;
            }
            let (x, z) = if snap { (e.f(4), e.f(5)) } else { (e.f(5), e.f(6)) };
            let s = f.cam.wp(&[scene::to_world(x, z, R)])[0];
            let big = !snap && matches!(how(e), "slam" | "wave");
            let (r, px) = if big { (26.0, 2.5) } else { (16.0, 1.4) };
            img.fringe(s.0, s.1, r, px, 1.0 - (t - e.t) / 0.12);
        }
    }
}
