//! TURNTABLES's look: the `Look` class of its reference kit module (`tools/studio_export/older/fighters/
//! turntables.py`), #31 v3's layers (`render.py`) and draw code (`styles.py`), hook by hook.
//!
//! What it draws: the canvas IS the record (slate vinyl, grooves, track gaps, sheen, dust specks, strobe dots and
//! its label in its colour), turning with the floor angle (a slow spin and every scratch's jerk) through the opt-in
//! `canvas_albedo`, with the vinyl's gloss under the spot (`canvas_first`); the decks thump on every jerk; the
//! instrument tipped toward the camera (its `pitch`) and wiggling while it scratches; the tonearm from its pivot on
//! the apron down onto the DJ's ball, with its shadow; REWIND's ghost rings along the zip and the tape look round
//! the zipping ball with its rewind glyph (the opt-in `after_post`); BASS DROP's darkening target, the giant record
//! hanging, dropping and lying under the balls, its shimmer, crack and scorch; every rope slam's burst, light,
//! shards and crack, every jerk's floor light and streaks; the champion's groove rings of light; its shatter (vinyl,
//! label, tonearm, plinth chips); the VS card's record grooves and its tipped close-up.
//!
//! A solo of the fight file holds plain notes: each is a stab (a big groove ring, no scratch of the record). #31's
//! scratch routine (solo entries that are scratches) comes with ruling M4-R5's solo entry.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use bf_core::dmath::{self, PI};
use bf_core::event::{Event, Value};
use bf_core::py::{fmod_py, py_max, py_min, round_half_even};
use bf_core::rng::Generator as Gen;
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{self, H, SAFE, W};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::{Img, RgbF};
use bf_picture::lighting::{Lights, shade};
use bf_picture::look::{BallSprite, CanvasCtx, DefaultLook, DrawnBall, FrameCtx, Fx, Look, Payload, Phase, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::state::BallState;
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::shadows::proj_point;
use bf_picture::sprite::{Mesh, render_sprite_pitched};
use bf_picture::title::layout::Layout;
use bf_picture::title::patterns::turntables as pattern;
use bf_picture::ui::{DrawLog, Kind as NoteKind};
use bf_picture::{np, pil};

// ---------------------------------------------------------------- #31 styles.py
/// `VINYL`: a record's black.
const VINYL: V3 = [30.0, 28.0, 38.0];
/// `GROOVE`.
const GROOVE: V3 = [74.0, 70.0, 88.0];
/// `REC_R`: the record's radius (inside the ropes at 3.0).
const REC_R: f64 = 2.95;
/// `LABEL_R`.
const LABEL_R: f64 = 0.98;
/// `VINYL2`: slate vinyl.
const VINYL2: V3 = [66.0, 64.0, 82.0];
/// `STROBE`.
const STROBE: V3 = [196.0, 200.0, 214.0];
/// `ARM_Y`: the tonearm's height.
const ARM_Y: f64 = 0.42;
/// `PIVOT`: on the apron, by the DJ's (left) corner.
const PIVOT: [f64; 2] = [-3.42, 1.55];
/// `REST_TIP`: parked along the apron.
const REST_TIP: [f64; 2] = [-3.38, -0.55];
/// `ANG_PER_SEC`: record audio seconds to floor angle (33 1/3 rpm: 1.8 s a turn).
const ANG_PER_SEC: f64 = 2.0 * PI / 1.8;
/// `SCR_AMP`: the floor angle (rad) of a scratch at strength 1.3.
const SCR_AMP: f64 = 0.40;
/// The painter depths of its free items: REWIND's ghosts first (deeper than the SYNTH's cables at 1e9), then the
/// lying record and the tonearm (between the cables and the posts): #31's order.
const GHOST_Z: f64 = 2e9;
const UNDER_Z: f64 = 5e8;

/// `circle_world(cx, cz, r, y, n, a0)`.
fn circle_world(cx: f64, cz: f64, r: f64, y: f64, n: usize, a0: f64) -> Vec<V3> {
    // a = a0 + np.linspace(0, 2 * math.pi, n + 1)[:-1]
    let step = 2.0 * PI / n as f64;
    (0..n)
        .map(|i| {
            let a = a0 + i as f64 * step;
            scene::to_world(cx + r * dmath::cos(a), cz + r * dmath::sin(a), y)
        })
        .collect()
}

/// `record_albedo(alb, sx, sz, phi, label_col)` for one canvas pixel: its albedo on the record floor (ring frame
/// `sx`, `sz`; floor angle `phi`), and whether it is on the record.
fn record_albedo(alb: V3, sx: f64, sz: f64, phi: f64, lc: V3) -> (V3, bool) {
    let r = np::hypot(sx, sz);
    let th = dmath::atan2(sz, sx);
    let thf = fmod_py(th - phi, 2.0 * PI);
    let rec = r < REC_R;
    if !rec {
        return (alb, false);
    }
    if r < LABEL_R {
        // the label: its colour, a printed band, dark arcs of "text", a centre ring + the spindle
        let mut l = lc.map(|c| c * 0.92);
        if (r - 0.62).abs() < 0.05 {
            l = lc.map(|c| c * 0.55);
        }
        let text = (r - 0.78).abs() < 0.06 && thf > 0.4 && thf < 2.3 && fmod_py((thf * 14.0).floor(), 3.0) != 0.0;
        if text {
            l = [26.0, 30.0, 20.0];
        }
        if (r - 0.42).abs() < 0.04 && thf > 3.6 && thf < 5.0 {
            l = [26.0, 30.0, 20.0];
        }
        if (r - 0.2).abs() < 0.025 {
            l = lc.map(|c| c * 0.5);
        }
        if r < 0.07 {
            l = [210.0, 214.0, 226.0];
        }
        return (l, true);
    }
    // vinyl: fine grooves, track gaps, the sheen facing the spot
    let g = 1.0 + 0.07 * dmath::sin(r * 2.0 * PI / 0.085);
    let mut v = VINYL2.map(|c| c * g);
    let mut gap = 0.0f64;
    for rg in [1.42, 1.9, 2.35] {
        gap = np::maximum(gap, np::clip(1.0 - (r - rg).abs() / 0.03, 0.0, 1.0));
    }
    v = v.map(|c| c + gap * 22.0);
    let sheen = dmath::pow(dmath::cos(th - 0.6).abs(), 10.0) * np::clip((r - LABEL_R) / 0.3, 0.0, 1.0);
    v = std::array::from_fn(|ch| v[ch] + sheen * [70.0, 74.0, 92.0][ch]);
    // dust specks fixed on the vinyl
    let sp = fmod_py((r * 23.0).floor() * 7.31 + (thf * 9.0 * (r + 0.5)).floor() * 3.17, 1.0) > 0.985;
    if sp {
        v = v.map(|c| c + 38.0);
    }
    // the rim: strobe dots every 8 deg (they turn with the floor)
    let rim = r > REC_R - 0.2 && r < REC_R - 0.07;
    if rim {
        v = v.map(|c| c * 0.7);
        if fmod_py(thf, 8.0 * (PI / 180.0)) < 3.5 * (PI / 180.0) {
            v = STROBE;
        }
    }
    if r >= REC_R - 0.05 {
        v = [30.0, 28.0, 38.0];
    }
    (v, true)
}

/// `drop_height(t, t0, t_slam)`: the record hangs high while the target blinks, then drops onto the spot; it lies
/// there a moment.
fn drop_height(t: f64, t0: f64, t_slam: f64) -> Option<f64> {
    if t < t0 {
        None
    } else if t < t_slam - 0.32 {
        Some(6.0 - 0.6 * py_min(1.0, (t - t0) / 0.4))
    } else if t < t_slam {
        let u = (t - (t_slam - 0.32)) / 0.32;
        Some(5.4 * (1.0 - u * u) + 0.06)
    } else if t < t_slam + 0.55 {
        Some(0.06)
    } else {
        None
    }
}

/// `drop_polys(cam, cx, cz, rad, h, col, spin)`: the dropping record from above, back to front.
fn drop_polys(cam: &Cam, cx: f64, cz: f64, rad: f64, h: f64, col: V3, spin: f64) -> Vec<(Vec<(f64, f64)>, V3)> {
    let r = rad * 1.1;
    let rings = [
        (r, VINYL),
        (r * 0.84, GROOVE.map(|c| c * 0.85)),
        (r * 0.80, VINYL),
        (r * 0.62, GROOVE.map(|c| c * 0.75)),
        (r * 0.58, VINYL),
        (r * 0.34, col.map(|c| c * 0.9)),
        (r * 0.06, [200.0, 200.0, 210.0]),
    ];
    rings
        .iter()
        .map(|&(rr, c)| (cam.wp(&circle_world(cx, cz, rr, h, 28, spin)), c))
        .collect()
}

/// `tonearm(pivot, tip, h_tip)`: the tonearm's world pieces: `base`, `base2`, `weight`, `arm`, `shell`.
fn tonearm(tip: [f64; 2], h_tip: f64) -> Vec<(&'static str, Vec<V3>)> {
    let (p, q) = (PIVOT, tip);
    let d = [q[0] - p[0], q[1] - p[1]];
    let l = np::hypot(d[0], d[1]) + 1e-9;
    let u = [d[0] / l, d[1] / l];
    let n = [-u[1], u[0]];
    let at = |a: [f64; 2], k: f64, m: [f64; 2], y: f64| scene::to_world(a[0] + m[0] * k, a[1] + m[1] * k, y);
    let w0 = [p[0] - u[0] * 0.55, p[1] - u[1] * 0.55];
    let w1 = [p[0] - u[0] * 0.18, p[1] - u[1] * 0.18];
    let a1 = [q[0] - u[0] * 0.34, q[1] - u[1] * 0.34];
    let hw = 0.05;
    let s0 = [q[0] - u[0] * 0.42, q[1] - u[1] * 0.42];
    let s1 = [q[0] + u[0] * 0.04, q[1] + u[1] * 0.04];
    let m = [n[0] * 0.13, n[1] * 0.13];
    let m7 = [m[0] * 0.7, m[1] * 0.7];
    vec![
        ("base", circle_world(p[0], p[1], 0.32, 0.12, 14, 0.0)),
        ("base2", circle_world(p[0], p[1], 0.16, ARM_Y + 0.02, 10, 0.0)),
        (
            "weight",
            vec![
                at(w0, -0.11, n, ARM_Y),
                at(w1, -0.11, n, ARM_Y),
                at(w1, 0.11, n, ARM_Y),
                at(w0, 0.11, n, ARM_Y),
            ],
        ),
        (
            "arm",
            vec![
                at(p, -hw, n, ARM_Y),
                at(a1, -hw, n, h_tip),
                at(a1, hw, n, h_tip),
                at(p, hw, n, ARM_Y),
            ],
        ),
        (
            "shell",
            vec![
                at(s0, -1.0, m, h_tip),
                at(s1, -1.0, m7, h_tip - 0.06),
                at(s1, 1.0, m7, h_tip - 0.06),
                at(s0, 1.0, m, h_tip),
            ],
        ),
    ]
}

/// `vhs(img, cx, cy, k, t, seed)`: a band round screen point `(cx, cy)`: rows jitter sideways, red and blue split
/// apart, every other row darker, a bright noise bar rolls up through it; `k` its strength.
fn vhs(img: &mut Img<u8>, _cx: f64, cy: f64, k: f64, t: f64, seed: u64) {
    if k <= 0.01 {
        return;
    }
    let mut rng = Gen::new(((t * 30.0) as i64 * 7 + seed as i64).max(0) as u64);
    let y0 = py_max(0.0, (cy - 42.0).trunc()) as i64;
    let y1 = py_min(H as f64, (cy + 42.0).trunc()) as i64;
    if y1 <= y0 {
        return;
    }
    let (y0, y1) = (y0 as usize, y1 as usize);
    let rows = y1 - y0;
    let band: Vec<f32> = img.data[y0 * W * 3..y1 * W * 3].iter().map(|&b| f32::from(b)).collect();
    let mut out = band.clone();
    let row = |v: &[f32], r: usize| -> Vec<f32> { v[r * W * 3..(r + 1) * W * 3].to_vec() };
    for r in 0..rows {
        let yy = (y0 + r) as f64;
        let u = 1.0 - (yy - cy).abs() / 42.0;
        let jit = rng.integers(7) as f64 - 3.0;
        let sh = round_half_even(k * u * (jit + 2.0 * dmath::sin(0.5 * yy + 40.0 * t))) as i64;
        if sh != 0 {
            let src = row(&band, r);
            for x in 0..W {
                let from = (x as i64 - sh).rem_euclid(W as i64) as usize;
                out[(r * W + x) * 3..(r * W + x) * 3 + 3].copy_from_slice(&src[from * 3..from * 3 + 3]);
            }
        }
    }
    let split = (round_half_even(2.0 * k) as i64).max(1);
    let prev = out.clone();
    for r in 0..rows {
        for x in 0..W {
            let red = (x as i64 - split).rem_euclid(W as i64) as usize;
            let blue = (x as i64 + split).rem_euclid(W as i64) as usize;
            out[(r * W + x) * 3] = prev[(r * W + red) * 3];
            out[(r * W + x) * 3 + 2] = prev[(r * W + blue) * 3 + 2];
        }
    }
    let dark = (1.0 - 0.18 * k) as f32;
    for r in 0..rows {
        if (r + y0) % 2 == 0 {
            for v in &mut out[r * W * 3..(r + 1) * W * 3] {
                *v *= dark;
            }
        }
    }
    let bar = ((1.0 - fmod_py(t * 1.7, 1.0)) * rows as f64) as i64;
    let (keep, add) = ((1.0 - 0.5 * k) as f32, (230.0 * 0.5 * k) as f32);
    for r in (bar - 1).max(0)..(bar + 2).min(rows as i64) {
        let r = r as usize;
        for x in 0..W {
            let on = rng.uniform(0.0, 1.0) > 0.55;
            if on {
                for v in &mut out[(r * W + x) * 3..(r * W + x) * 3 + 3] {
                    *v = *v * keep + add;
                }
            }
        }
    }
    for r in 0..rows {
        let wv = np::clip(1.0 - ((r + y0) as f64 - cy).abs() / 42.0, 0.0, 1.0) * k;
        for i in r * W * 3..(r + 1) * W * 3 {
            let v = f64::from(band[i]) * (1.0 - wv) + f64::from(out[i]) * wv;
            img.data[y0 * W * 3 + i] = np::clip(v, 0.0, 255.0) as u8;
        }
    }
}

/// `rewind_glyph(img, x, y, col, k)`: two little filled triangles.
fn rewind_glyph(img: &mut Img<u8>, x: f64, y: f64, col: V3, k: f64) {
    if k <= 0.05 {
        return;
    }
    let c = col.map(|v| np::clip(v * 0.6 + 100.0, 0.0, 255.0) as u8);
    for ox in [0.0, 7.0] {
        pil::polygon(
            img,
            &[(x + ox, y), (x + ox + 6.0, y - 4.0), (x + ox + 6.0, y + 4.0)],
            &[10, 8, 18],
        );
        pil::polygon(
            img,
            &[(x + ox + 1.0, y), (x + ox + 6.0, y - 3.0), (x + ox + 6.0, y + 3.0)],
            &c,
        );
    }
}

// ---------------------------------------------------------------- #31 render.py helpers
/// `Q.mean(0)`: the rows added in order, then divided by their number.
fn mean(rows: &[V3]) -> V3 {
    let mut s = [0.0; 3];
    for r in rows {
        s = [s[0] + r[0], s[1] + r[1], s[2] + r[2]];
    }
    let n = rows.len() as f64;
    [s[0] / n, s[1] / n, s[2] / n]
}

/// `line_blend(arr, pts, col, alpha, width)`: alpha-blends a screen polyline onto the `float32` picture.
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

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Float(x) => Some(*x),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

/// One row of `RUN['SLAMS'][i]`.
#[derive(Clone, Copy, Debug)]
struct Slam {
    owner: i64,
    cx: f64,
    cz: f64,
    t0: f64,
    t_slam: f64,
    rad: f64,
}

/// `RUN['SLAMS'][i]`.
fn slams(p: &PictureCtx, i: usize) -> Vec<Slam> {
    let Some(Value::List(rows)) = p.run.nested("SLAMS", i) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            let Value::List(v) = row else { return None };
            let Value::Int(owner) = *v.first()? else { return None };
            Some(Slam {
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

/// The TURNTABLES's model, compiled in.
fn mesh() -> &'static Mesh {
    static MESH: OnceLock<Mesh> = OnceLock::new();
    MESH.get_or_init(|| {
        let json = super::MESH.expect("the TURNTABLES's model is compiled in");
        Mesh::from_json(json).expect("the TURNTABLES's model reads")
    })
}

/// Its sprites by `(r, yaw step, roll bits)`.
type Sprites = BTreeMap<(u32, i64, u64), Arc<Img<u8>>>;

/// A painter item of this look.
enum Item {
    Ghosts,
    Drop,
    Arm,
}

// ---------------------------------------------------------------- the look
/// TURNTABLES's look.
#[derive(Debug)]
pub struct TurntablesLook {
    /// Its side.
    k: usize,
    /// `pitch`: the model tipped toward the camera.
    pitch: f64,
    /// `pp`: its model's name.
    pp: String,
    /// `view`: (yaw centre, yaw swing, roll).
    view: (f64, f64, f64),
    /// `abilities.low.radius`: BASS DROP's radius (its shimmer).
    slam_rad: f64,
    /// Its colour.
    col: V3,
    /// Its scratches: `(t0, length, strength, direction)`.
    scr: Vec<(f64, f64, f64, f64)>,
    /// When its solo notes start (it won).
    solo_t: Vec<f64>,
    /// REWIND windows: `(t0, t1)`.
    rw_win: Vec<(f64, f64)>,
    /// When the floor jerked.
    jerk_t: Vec<f64>,
    /// Its BASS DROPs: `(t, cx, cz)`.
    slam_ev: Vec<(f64, f64, f64)>,
    /// The record turns once every 5 bars.
    spin_p: f64,
    /// The arm swings over and drops on FIGHT!.
    t_drop: (f64, f64),
    /// Its sprites: `(r, yaw step, roll bits)`.
    sprites: Mutex<Sprites>,
}

/// A look not set up yet (the card's close-up and the sprite cache ask it before any fight: Python's `Look.__init__`
/// reads the roster at once) draws the TURNTABLES's own: its roster's pitch 45, view (20, 22, 0) and lime.
impl Default for TurntablesLook {
    fn default() -> TurntablesLook {
        TurntablesLook {
            k: 0,
            pitch: 45.0,
            pp: "TURNTABLES".to_string(),
            view: (20.0, 22.0, 0.0),
            slam_rad: 1.35,
            col: pattern::LIME,
            scr: Vec::new(),
            solo_t: Vec::new(),
            rw_win: Vec::new(),
            jerk_t: Vec::new(),
            slam_ev: Vec::new(),
            spin_p: 1.0,
            t_drop: (0.0, 0.0),
            sprites: Mutex::new(BTreeMap::new()),
        }
    }
}

impl TurntablesLook {
    /// `scratch_q(t)`: the record's position under the DJ's hand.
    fn scratch_q(&self, p: &PictureCtx, t: f64) -> f64 {
        let tc = p.fx_clock(t);
        let after = self.scr.partition_point(|s| s.0 <= tc);
        let Some(j) = after.checked_sub(1) else {
            return 0.0;
        };
        if tc >= p.t_ko + 0.01 && t >= p.t_hs {
            return 0.0;
        }
        let (t0, d, stg, sgn) = self.scr[j];
        let u = (tc - t0) / d;
        if !(0.0..1.0).contains(&u) {
            return 0.0;
        }
        let a = SCR_AMP * stg / 1.3 / ANG_PER_SEC;
        sgn * a * (1.0 - dmath::cos(2.0 * PI * u)) / 2.0
    }

    /// `floor_angle(t)` (a solo of plain notes scratches nothing: `solo_q` is 0).
    fn floor_angle(&self, p: &PictureCtx, t: f64) -> f64 {
        2.0 * PI * t / self.spin_p + (self.scratch_q(p, t) + 0.0) * ANG_PER_SEC
    }

    /// `rewinding(t)`: the tape look's strength and its window.
    fn rewinding(&self, t: f64) -> (f64, Option<(f64, f64)>) {
        for &(t0, t1) in &self.rw_win {
            if t0 - 0.06 <= t && t < t1 + 0.3 {
                let k = config::ss(t0 - 0.06, t0 + 0.04, t) * (1.0 - config::ss(t1, t1 + 0.3, t));
                return (k, Some((t0, t1)));
            }
        }
        (0.0, None)
    }

    /// `arm_state(t)`: the tonearm's tip (ring frame), its height, and whether it is on the ball.
    fn arm_state(&self, p: &PictureCtx, t: f64) -> ([f64; 2], f64, bool) {
        let k = self.k;
        let on_ball = |tt: f64| {
            let q = p.fighter(k, tt).p;
            let d = [q[0] - PIVOT[0], q[1] - PIVOT[1]];
            let n = np::hypot(d[0], d[1]) + 1e-9;
            [q[0] - d[0] / n * R * 0.92, q[1] - d[1] / n * R * 0.92]
        };
        let (t0, t1) = self.t_drop;
        if t < t0 {
            return (REST_TIP, 0.85, false);
        }
        if t < t1 {
            let u = config::ease((t - t0) / (t1 - t0));
            let b = on_ball(t);
            let tip = [
                REST_TIP[0] + (b[0] - REST_TIP[0]) * u,
                REST_TIP[1] + (b[1] - REST_TIP[1]) * u,
            ];
            return (tip, 0.85 + (ARM_Y - 0.85) * (u * u), u > 0.95);
        }
        let lift_t = if k == p.win { p.show.grid.t_glide } else { p.t_hs };
        if t < lift_t {
            return (on_ball(t), ARM_Y, true);
        }
        let u = config::ease((t - lift_t) / 0.9);
        let tip0 = if k == p.win {
            on_ball(lift_t)
        } else {
            on_ball(py_min(t, lift_t))
        };
        let tip = [
            tip0[0] + (REST_TIP[0] - tip0[0]) * u,
            tip0[1] + (REST_TIP[1] - tip0[1]) * u,
        ];
        let h = ARM_Y + (0.85 - ARM_Y) * dmath::sin(PI * 0.5 * py_min(1.0, u * 1.6));
        (tip, h, false)
    }

    fn draw_tonearm(&self, fr: &mut Frame, cam: &Cam, l: &Lights, arm: ([f64; 2], f64, bool)) {
        let (tip, h, _) = arm;
        let pieces = tonearm(tip, h);
        let ink = [16u8, 14, 24];
        for (kind, q) in &pieces {
            if *kind == "arm" {
                let s = cam.wp(q);
                pil::polygon_outline(&mut fr.pil, &s, None, &ink);
                let a = ((s[0].0 + s[3].0) / 2.0, (s[0].1 + s[3].1) / 2.0);
                let b = ((s[1].0 + s[2].0) / 2.0, (s[1].1 + s[2].1) / 2.0);
                pil::line(&mut fr.pil, &[a, b], &ink, 4);
            }
        }
        for (kind, q) in &pieces {
            let (base, rimk) = match *kind {
                "base" => ([40.0, 38.0, 50.0], 0.4),
                "base2" => ([200.0, 204.0, 216.0], 0.6),
                "weight" => ([150.0, 154.0, 168.0], 0.5),
                "arm" => ([214.0, 218.0, 230.0], 0.7),
                _ => ([30.0, 30.0, 38.0], 0.5),
            };
            let c = shade(l, &[[0.0, 1.0, 0.0]], &[mean(q)], &[base], None, rimk, 0.0, 5.0)[0];
            fr.poly(cam, q, c, true);
            if *kind == "shell" {
                let s = cam.wp(q);
                let c = self.col.map(|v| v as u8);
                pil::line(&mut fr.pil, &[s[0], s[1]], &c, 1);
            }
        }
    }

    #[allow(clippy::too_many_arguments)] // render.py's draw_drop(arr, cam, t, i, in_fight, falling)
    fn draw_drop(
        &self,
        p: &PictureCtx,
        arr: &mut Img<f32>,
        cam: &Cam,
        t: f64,
        i: usize,
        in_fight: bool,
        falling: bool,
    ) {
        if !in_fight {
            return;
        }
        for s in slams(p, i) {
            if s.owner != self.k as i64 {
                continue;
            }
            let Some(h) = drop_height(t, s.t0, s.t_slam) else {
                continue;
            };
            if (h > 0.3) != falling {
                continue;
            }
            let fade = if t < s.t_slam + 0.35 {
                1.0
            } else {
                py_max(0.0, 1.0 - (t - s.t_slam - 0.35) / 0.2)
            };
            let a = py_min(1.0, (t - s.t0) / 0.2) * fade;
            for (poly, c) in drop_polys(cam, s.cx, s.cz, s.rad, h, p.col[self.k], t * 3.0) {
                layer_blend(arr, &[poly], c.map(|v| np::clip(v, 0.0, 255.0)), a, None);
            }
        }
    }

    /// Its sprite, tipped toward the camera, radius `r` px, yaw `yaw` (a 6 degree step).
    fn pitched(&self, r: u32, yaw: i64, roll: f64) -> Arc<Img<u8>> {
        let mut cache = self.sprites.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        Arc::clone(
            cache
                .entry((r, yaw, roll.to_bits()))
                .or_insert_with(|| Arc::new(render_sprite_pitched(mesh(), r, yaw as f64, roll, self.pitch))),
        )
    }
}

impl Look for TurntablesLook {
    fn shards(&self) -> Vec<String> {
        let mut out = vec!["vinyl".to_string(); 8];
        out.extend(std::iter::repeat_n("label".to_string(), 3));
        out.extend(std::iter::repeat_n("arm".to_string(), 2));
        out.extend(std::iter::repeat_n("plinth".to_string(), 3));
        out
    }

    fn spark_big(&self) -> &[&str] {
        &["slam", "wave", "shockwave", "cannon", "scratch"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["slam", "wave", "shockwave", "scratch"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["slam", "wave", "shockwave", "scratch"]
    }

    fn title_pattern_with(&self, base: &mut RgbF, _card: &Layout) {
        pattern::pattern_in(base, self.col);
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// What the kit look reads from `self.cfg` by name: BASS DROP's radius, the pitch, the model's name and the
    /// roll (`view`).
    fn cfg_summary(&self) -> Option<String> {
        Some(format!(
            "abilities.low.radius={:?}; pitch={:?}; pp={}; view.2={:?}",
            self.slam_rad, self.pitch, self.pp, self.view.2
        ))
    }

    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        let beat = p.show.grid.beat;
        self.k = k;
        self.col = p.col[k];
        self.pp = cfg.pp.clone();
        // self.cfg.get('pitch', 0) and self.cfg['view'] (the roster's own numbers: the entry as it is read)
        self.pitch = cfg
            .raw
            .get("pitch")
            .map_or(0.0, |v| v.as_f64().expect("a roster number"));
        let view = cfg
            .raw
            .get("view")
            .and_then(|v| v.as_array())
            .expect("a roster entry has a view");
        let at = |i: usize| view[i].as_f64().expect("a roster number");
        self.view = (at(0), at(1), at(2));
        self.slam_rad = cfg.low.get("radius").unwrap_or(1.35);
        self.spin_p = 5.0 * p.show.grid.bar;
        self.t_drop = (p.show.grid.t_fight - 0.42, p.show.grid.t_fight);
        let ev = &p.rec.events;
        self.scr = ev
            .iter()
            .filter(|e| e.kind == "scratch" && e.who() == Some(k))
            .map(|e| (e.t, e.f(3), e.f(4), e.f(5)))
            .collect();
        self.solo_t = if p.win == k {
            p.solos[k].iter().map(|&(bt, _, _)| p.t_solo + bt * beat).collect()
        } else {
            Vec::new()
        };
        self.rw_win = ev
            .iter()
            .filter(|e| e.kind == "heal" && e.who() == Some(k))
            .map(|e| {
                let t1 = ev
                    .iter()
                    .find(|f| f.kind == "rewound" && f.who() == Some(k) && f.t >= e.t)
                    .map_or(e.t + beat, |f| f.t);
                (e.t, t1)
            })
            .collect();
        self.jerk_t = ev
            .iter()
            .filter(|e| e.kind == "jerk" && e.who() == Some(k))
            .map(|e| e.t)
            .collect();
        self.slam_ev = ev
            .iter()
            .filter(|e| e.kind == "slam" && e.who() == Some(k))
            .map(|e| (e.t, e.f(3), e.f(4)))
            .collect();
    }

    fn title_closeup(&self, r: u32) -> Option<Img<u8>> {
        let (yc, _, roll) = self.view;
        Some(render_sprite_pitched(mesh(), r, yc, roll, self.pitch))
    }

    fn sprite(&self, r: u32, yaw: f64, roll: f64) -> Option<Img<u8>> {
        Some(render_sprite_pitched(mesh(), r, yaw, roll, self.pitch))
    }

    fn ball_sprite<'a>(
        &'a self,
        p: &'a PictureCtx,
        kk: usize,
        st: &BallState,
        t: f64,
        rp: f64,
        yaw: f64,
    ) -> Option<BallSprite<'a>> {
        if kk != self.k {
            return None;
        }
        let mut yaw = yaw;
        if st.haste > 0.0 || (self.k == p.win && p.solo_env(t) > 0.2 && t > p.t_solo) {
            yaw += 12.0 * dmath::sin(2.0 * PI * t / (p.show.grid.beat / 2.0));
        }
        let r = np::clip(round_half_even(rp * self.sprite_k()), 12.0, 30.0) as u32;
        let yk = round_half_even(yaw / 6.0) as i64 * 6;
        Some(BallSprite {
            sprite: self.pitched(r, yk, self.view.2),
            tint: None,
        })
    }

    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        // the decks thump on every jerk of the floor
        if kk != self.k || phase != Phase::Fight || self.jerk_t.is_empty() {
            return;
        }
        let tt = py_min(t, p.t_ko);
        let after = self.jerk_t.partition_point(|&v| v <= tt);
        if let Some(j) = after.checked_sub(1) {
            st.glow = 0.5 * dmath::exp(-(tt - self.jerk_t[j]) / 0.1);
        }
    }

    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        match how(e) {
            "scratch" => Some(1.4),
            "slam" => Some(2.5),
            _ => DefaultLook.shake(p, e),
        }
    }

    fn rope(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "jerk" {
            return Some(0.35);
        }
        DefaultLook.rope(p, e)
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        matches!(how(e), "slam" | "scratch").then_some((12.0, 0.35))
    }

    fn fringe_size(&self, _p: &PictureCtx, e: &Event) -> (f64, f64) {
        if matches!(how(e), "slam" | "wave" | "shockwave") {
            (26.0, 2.5)
        } else {
            (16.0, 1.4)
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        let t0 = e.t;
        match e.kind.as_str() {
            "hit" => {
                let Some(att) = e.who() else { return };
                let (x, z) = (e.f(5), e.f(6));
                let pos = scene::to_world(x, z, R);
                match how(e) {
                    "scratch" => {
                        // a rope slam
                        fx.bu.add(t0, pos, p.col[att], 1.2, None, true);
                        fx.fl.add(t0, pos, p.col[att], 1.8, 0.8, 0.03, 0.3, 0.0);
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
                    "slam" => fx.bu.add(t0, pos, p.col[att], 1.6, None, true),
                    _ => {}
                }
            }
            "jerk" => {
                // the whole floor lights up on every jerk
                let Some(k) = e.who() else { return };
                fx.fl
                    .add(t0, scene::to_world(0.0, 0.0, 1.0), p.col[k], 2.4, 0.32, 0.03, 0.22, 0.0);
                let (sgn, x, z) = (e.f(3), e.f(4), e.f(5));
                let rn = dmath::py_hypot(x, z) + 1e-6;
                let tang = scene::to_world(-z / rn * sgn, x / rn * sgn, 0.0);
                let emit = Emit {
                    n: Some(3),
                    dir: tang,
                    col: Some(vec![p.col[k].map(|c| c * 0.6 + 100.0)]),
                    ..Emit::default()
                };
                fx.pa.emit(t0, scene::to_world(x, z, 0.3), "streaks", &emit);
            }
            "slam" => {
                let Some(owner) = e.who() else { return };
                let (cx, cz) = (e.f(3), e.f(4));
                let c = scene::to_world(cx, cz, 0.1);
                fx.fl
                    .add(t0, scene::to_world(cx, cz, 1.2), p.col[owner], 2.6, 1.2, 0.03, 0.5, 0.0);
                let emit = Emit {
                    n: Some(24),
                    spread: Some(180.0),
                    speed: Some((1.2, 3.6)),
                    scale: 1.3,
                    ..Emit::default()
                };
                fx.pa.emit(t0, c, "shards", &emit);
                let seed = seed_of(t0 * 10.0);
                let crack = Mark {
                    size: 1.4,
                    n: 8,
                    life: 7.0,
                    seed,
                    ..Mark::new(MarkKind::Crack, cx, cz)
                };
                fx.mk.add(t0, crack);
                let scorch = Mark {
                    size: 1.1,
                    col: [60.0, 70.0, 30.0],
                    life: 7.0,
                    seed: seed + 1,
                    strength: 0.6,
                    ..Mark::new(MarkKind::Scorch, cx, cz)
                };
                fx.mk.add(t0, scorch);
            }
            _ => {}
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let emit = Emit {
            n: Some(24),
            spread: Some(180.0),
            speed: Some((0.8, 2.6)),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "shards", &emit);
    }

    // ------------------------------------------------------------ the canvas: THE RECORD
    fn canvas_albedo(&self, p: &PictureCtx, alb: &mut [V3], c: &CanvasCtx<'_>) {
        let phi = self.floor_angle(p, c.t);
        for (j, a) in alb.iter_mut().enumerate() {
            *a = record_albedo(*a, c.sxp[j], c.szp[j], phi, self.col).0;
        }
    }

    /// The vinyl's gloss under the spot: the record's pixels, `r < REC_R`, as `canvas_albedo` found them.
    fn canvas_first(&self, _p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        for (j, light) in ic.iter_mut().enumerate() {
            if np::hypot(c.sxp[j], c.szp[j]) < REC_R {
                for v in light.iter_mut() {
                    *v *= 1.12;
                }
            }
        }
    }

    /// The champion's solo: groove rings of light (a stab's: 1.9 out).
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let t = c.t;
        // (t >= T_SOLO - 0.01, NaN-free times)
        if p.win != self.k || t < p.t_solo - 0.01 {
            return;
        }
        let bayer = config::bayer_full();
        let st = &c.fs[self.k];
        let tint = self.col.map(|v| v / 255.0 * 0.9 + 0.25);
        for &t0 in &self.solo_t {
            if !(t0 <= t && t < t0 + 0.6) {
                continue;
            }
            let reach = 1.9;
            let u = t - t0;
            let r = R + py_min(reach, 6.0 * u);
            let a = if u < reach / 6.0 {
                1.0
            } else {
                py_max(0.0, 1.0 - (u - reach / 6.0) / 0.25)
            };
            for (j, light) in ic.iter_mut().enumerate() {
                let rd = np::hypot(c.sxp[j] - st.p[0], c.szp[j] - st.p[1]);
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

    /// BASS DROP's target darkening with a flashing rim, then the stomp ring.
    fn canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        for s in slams(p, c.i) {
            if s.owner != self.k as i64 {
                continue;
            }
            if c.t < s.t_slam {
                let u = (c.t - s.t0) / (s.t_slam - s.t0);
                let blink = 0.4 + 0.5 * if dmath::sin(c.t * 40.0) > 0.0 { 1.0 } else { 0.0 };
                let ring = p.col[self.k].map(|v| v / 255.0 * blink);
                for (j, light) in ic.iter_mut().enumerate() {
                    let rd = np::hypot(c.sxp[j] - s.cx, c.szp[j] - s.cz);
                    if rd < s.rad * 1.1 {
                        for v in light.iter_mut() {
                            *v *= 1.0 - 0.6 * u;
                        }
                    }
                    if (rd - s.rad).abs() < 0.05 {
                        for ch in 0..3 {
                            light[ch] += ring[ch];
                        }
                    }
                }
            } else {
                let u = (c.t - s.t_slam) / 0.6;
                let white = [0.9, 0.9, 1.0];
                for (j, light) in ic.iter_mut().enumerate() {
                    let rd = np::hypot(c.sxp[j] - s.cx, c.szp[j] - s.cz);
                    let band = np::clip(1.0 - (rd - (s.rad + 1.6 * u)).abs() / 0.2, 0.0, 1.0) * (1.0 - u);
                    for ch in 0..3 {
                        light[ch] += band * white[ch];
                    }
                }
            }
        }
    }

    fn shadows_free(&self, p: &PictureCtx, f: &FrameCtx<'_>, spot: V3) -> Vec<Vec<(f64, f64)>> {
        let (tip, h, _) = self.arm_state(p, f.t);
        tonearm(tip, h)
            .iter()
            .filter(|(kind, _)| matches!(*kind, "arm" | "shell" | "weight"))
            .map(|(_, q)| proj_point(q, spot, 0.93))
            .collect()
    }

    // ------------------------------------------------------------ the painter
    fn items_free(&self, _p: &PictureCtx, _f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let ghosts: Payload = Box::new(Item::Ghosts);
        let drop: Payload = Box::new(Item::Drop);
        let arm: Payload = Box::new(Item::Arm);
        vec![(GHOST_Z, ghosts), (UNDER_Z, drop), (UNDER_Z, arm)]
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let (k, t, cam) = (self.k, f.t, f.cam);
        match payload.downcast_ref::<Item>() {
            Some(Item::Ghosts) => {
                let (rk, win) = self.rewinding(t);
                let Some((w0, _)) = win else { return };
                if rk <= 0.0 || !f.fs[k].vis {
                    return;
                }
                let (mut arr, depth) = (fr.array(), fr.depth());
                for g in 1..6 {
                    let gf = f64::from(g);
                    let tg = py_max(w0, t - 0.035 * gf);
                    if tg >= t - 1e-6 {
                        continue;
                    }
                    let pg = p.run.p(p.sim_i(tg), k);
                    let s = cam.wp(&circle_world(pg[0], pg[1], R * (1.0 - 0.06 * gf), R, 28, 0.0));
                    let mut pts = s.clone();
                    pts.push(s[0]);
                    let col = p.col[k].map(|c| c * 0.7 + 70.0);
                    line_blend(&mut arr, &pts, col, 0.55 * rk * (1.0 - gf / 6.5), 2);
                }
                fr.set_array(&arr, Some(&depth));
            }
            Some(Item::Drop) => {
                let (mut arr, depth) = (fr.array(), fr.depth());
                self.draw_drop(p, &mut arr, cam, t, f.i, f.in_fight, false);
                fr.set_array(&arr, Some(&depth));
            }
            Some(Item::Arm) => self.draw_tonearm(fr, cam, f.l, self.arm_state(p, t)),
            None => {}
        }
    }

    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        self.draw_drop(p, arr, f.cam, f.t, f.i, f.in_fight, true);
    }

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
        if kind == "arm" {
            let (lk, wk) = (0.46, 0.04);
            let q4: Vec<[f64; 2]> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .iter()
                .map(|&(a1, a2)| {
                    [
                        sx + d[0] * lk / 2.0 * a1 + nn[0] * wk / 2.0 * a2,
                        sz + d[1] * lk / 2.0 * a1 + nn[1] * wk / 2.0 * a2,
                    ]
                })
                .collect();
            let k = 0.8 + 0.2 * dmath::cos(rot * 2.0);
            layer_blend(arr, &[on_screen(&q4)], [206.0, 210.0, 222.0].map(|c| c * k), 1.0, None);
            return;
        }
        let col = match kind {
            "vinyl" => VINYL.map(|c| c * 1.3),
            "label" => p.col[self.k].map(|c| c * 0.95),
            "plinth" => [58.0, 56.0, 66.0],
            _ => return,
        };
        let s = size * if kind == "vinyl" { 1.5 } else { 1.0 };
        let tri = [
            [sx + d[0] * s, sz + d[1] * s],
            [
                sx - d[0] * s * 0.7 + nn[0] * s * 0.8,
                sz - d[1] * s * 0.7 + nn[1] * s * 0.8,
            ],
            [
                sx - d[0] * s * 0.4 - nn[0] * s * 0.7,
                sz - d[1] * s * 0.4 - nn[1] * s * 0.7,
            ],
        ];
        let shade_k = 0.75 + 0.25 * dmath::cos(rot * 2.0);
        let sp = on_screen(&tri);
        layer_blend(
            arr,
            std::slice::from_ref(&sp),
            col.map(|c| np::clip(c * shade_k, 0.0, 255.0)),
            1.0,
            None,
        );
        if kind == "vinyl" {
            let g = (
                (sp[0].0 * 0.5 + sp[1].0 * 0.25 + sp[2].0 * 0.25) as i64,
                (sp[0].1 * 0.5 + sp[1].1 * 0.25 + sp[2].1 * 0.25) as i64,
            );
            if (0..W as i64).contains(&g.0) && (0..H as i64).contains(&g.1) {
                let i = (g.1 as usize * W + g.0 as usize) * 3;
                for (v, c) in arr.data[i..i + 3].iter_mut().zip(GROOVE) {
                    *v = (c * 1.4) as f32;
                }
            }
        }
    }

    // ------------------------------------------------------------ lens, post
    fn shimmer_after(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64) {
        let tc = p.fx_clock(t);
        for &(ts, cx, cz) in &self.slam_ev {
            let u = (tc - ts) / 0.6;
            if (0.0..1.0).contains(&u) {
                let c = scene::to_world(cx, cz, 0.05);
                img.shimmer_world(cam, c, self.slam_rad + 1.6 * u, 0.35, 3.0, t, 1.0 - u, 0.0);
            }
        }
    }

    /// REWIND: the tape look round the zipping ball and the rewind glyph.
    fn after_post(&self, _p: &PictureCtx, img: &mut Img<u8>, log: &mut DrawLog, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let Some(d) = drawn.iter().find(|d| d.k == self.k) else {
            return;
        };
        let (rk, _) = self.rewinding(f.t);
        if rk <= 0.0 {
            return;
        }
        vhs(img, d.s.0, d.s.1, rk, f.t, self.k as u64);
        let gx = round_half_even(d.s.0 - 6.0) as i32;
        let gy = round_half_even(d.s.1 - d.rp - 9.0) as i32;
        if SAFE.0 <= gx && gx + 13 <= SAFE.2 && SAFE.1 <= gy - 4 && gy + 4 <= SAFE.3 {
            rewind_glyph(img, f64::from(gx), f64::from(gy), self.col, rk);
            log.note(gx, gy - 4, gx + 13, gy + 4, NoteKind::Label, "rewind glyph");
        }
    }
}

