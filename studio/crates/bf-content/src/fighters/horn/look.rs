//! HORN's look: the port of `Look` in `projects/ballfight/kit/fighters/horn.py` and of the module's draw code
//! (#34 v5's `styles.py`), hook by hook.
//!
//! What it draws: the echo ghosts (translucent copper copies of its ball on its own past path, the newest brightest,
//! flashing as they repeat a blast, with the horn inside as a copper silhouette), a fading copper thread along its
//! path back to its oldest ghost, the call and echo rings (light on the canvas, a raised ring and a heat shimmer),
//! the ghosts' copper glow on the canvas and their aura, STOPPED HORN's open copper palm pressed into the bell with a
//! dome of light, FINAL FLICK's lit lane, the glove on its copper spring (with a real shadow) and the flung enemy's
//! afterimages, copper embers, and shards of brass tubing, valves and bell.
//!
//! Its VS card panel is the card's own (`_horn_coils`, `bf_picture::title::patterns::horn`).

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{py_max, py_min, round_half_even};
use bf_core::roster::{AbilityKind, FighterCfg, WeaponCfg};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{self, FPS, H, W, ease, ss};
use bf_picture::fx::Emit;
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DefaultLook, DrawnBall, FrameCtx, Fx, Look, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::state::BallState;
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::shadows::proj_point;
use bf_picture::{np, pil};

/// `BRASS`.
const BRASS: V3 = [232.0, 188.0, 92.0];
/// `BRASS_DK`.
const BRASS_DK: V3 = [150.0, 104.0, 40.0];
/// `COPPER`.
const COPPER: V3 = [214.0, 120.0, 66.0];
/// `COPPER_DK`.
const COPPER_DK: V3 = [128.0, 62.0, 30.0];
/// `GHOST`.
const GHOST: V3 = [255.0, 226.0, 196.0];
/// `INK_`.
const INK: [u8; 3] = [34, 22, 34];
/// `WHITE`.
const WHITE: V3 = [248.0, 246.0, 240.0];

/// `AGE_A`: ghost strength by slot (the newest, one beat back, brightest).
const AGE_A: [f64; 4] = [0.90, 0.70, 0.52, 0.36];
/// `G_SCALE`: the giant glove, about 1.9 ring units cuff to fingertip.
const G_SCALE: f64 = 1.45;

/// `HAND_PALM`: the STOPPED HORN hand's palm (screen space, x right, y down, 1 unit = the ball's radius).
const HAND_PALM: [(f64, f64); 6] = [
    (-0.44, -0.10),
    (-0.40, 0.42),
    (-0.16, 0.62),
    (0.22, 0.60),
    (0.42, 0.36),
    (0.44, -0.12),
];
/// `HAND_FINGERS`: `(cx, base_y, width, length)`.
const HAND_FINGERS: [(f64, f64, f64, f64); 4] = [
    (-0.33, -0.12, 0.17, 0.62),
    (-0.11, -0.12, 0.18, 0.78),
    (0.11, -0.12, 0.18, 0.72),
    (0.32, -0.10, 0.15, 0.52),
];
/// `HAND_THUMB`.
const HAND_THUMB: [(f64, f64); 5] = [
    (-0.42, 0.34),
    (-0.70, 0.06),
    (-0.80, -0.08),
    (-0.68, -0.14),
    (-0.44, 0.08),
];

/// `GLOVE_CUFF` (the glove's local frame: u forward toward the enemy, v left; 1 unit = 1 ring unit).
const GLOVE_CUFF: [(f64, f64); 8] = [
    (-0.66, -0.17),
    (-0.62, -0.24),
    (-0.44, -0.22),
    (-0.40, -0.15),
    (-0.40, 0.15),
    (-0.44, 0.22),
    (-0.62, 0.24),
    (-0.66, 0.17),
];
/// `GLOVE_BACK`.
const GLOVE_BACK: [(f64, f64); 8] = [
    (-0.42, -0.17),
    (-0.33, -0.26),
    (0.04, -0.27),
    (0.13, -0.20),
    (0.13, 0.20),
    (0.04, 0.27),
    (-0.33, 0.26),
    (-0.42, 0.17),
];
/// `GLOVE_KNUCKLES`: `(v, width)`, pinky to index.
const GLOVE_KNUCKLES: [(f64, f64); 4] = [(-0.19, 0.10), (-0.065, 0.12), (0.065, 0.12), (0.19, 0.11)];
/// `FLICKER`: the middle finger.
const FLICKER: usize = 1;
/// The thumb relaxed at the side (pose 1).
const THUMB_OPEN: [(f64, f64); 6] = [
    (-0.20, 0.22),
    (-0.02, 0.34),
    (0.08, 0.40),
    (0.14, 0.36),
    (0.06, 0.26),
    (-0.10, 0.16),
];
/// The thumb curled forward over the middle finger (pose 0).
const THUMB_COCKED: [(f64, f64); 7] = [
    (-0.20, 0.22),
    (0.02, 0.24),
    (0.20, 0.10),
    (0.26, 0.00),
    (0.20, -0.06),
    (0.08, 0.02),
    (-0.10, 0.12),
];

/// `np.linspace(a, b, n)`: `i * step + a`, the last value `b` itself.
fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    let step = (b - a) / (n - 1) as f64;
    (0..n)
        .map(|i| if i == n - 1 { b } else { i as f64 * step + a })
        .collect()
}

/// `np.array([math.cos(th), math.sin(th)])`.
fn unit(th: f64) -> [f64; 2] {
    [dmath::cos(th), dmath::sin(th)]
}

/// `np.linalg.norm(v)` of a 2-vector, as numpy computes it on the reference PC (ruling M3-R39: the second product
/// fused into the first).
fn norm2(v: [f64; 2]) -> f64 {
    dmath::fma(v[1], v[1], v[0] * v[0]).sqrt()
}

/// `x ** y` of two floats (the C runtime's `pow`).
fn pow(x: f64, y: f64) -> f64 {
    dmath::pow(x, y)
}

/// `e[7]` of a hit (what hit), or `""` for an event without one.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// A number of the recording's events and ragged arrays.
fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Float(x) => Some(*x),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

/// `_last(arr_, t)`: the last value at or before `t` of a sorted list, or `None`.
fn last(arr: &[f64], t: f64) -> Option<f64> {
    // j = np.searchsorted(arr_, t, 'right') - 1
    arr.partition_point(|&v| v <= t).checked_sub(1).map(|j| arr[j])
}

/// `_back_ease(u)`: an ease with an overshoot.
fn back_ease(u: f64) -> f64 {
    let c1 = 1.70158;
    // 1 + (c1 + 1) * (u - 1) ** 3 + c1 * (u - 1) ** 2
    1.0 + (c1 + 1.0) * pow(u - 1.0, 3.0) + c1 * pow(u - 1.0, 2.0)
}

/// `tuple(int(v) for v in np.clip(c, 0, 255))`: a colour for PIL (each sample cut toward zero).
fn ink(c: V3) -> [u8; 3] {
    c.map(|v| np::f64_to_u8(np::clip(v, 0.0, 255.0)))
}

/// `tuple(int(v) for v in c)` of a colour already inside 0..255.
fn ints(c: V3) -> [u8; 3] {
    c.map(np::f64_to_u8)
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

/// `arr += (np.clip(col, 0, 255)[None, None] - arr) * (m * a)[..., None]`, `m` an `L` picture over `255.0` in
/// `float32` and `a` a Python float: the mask `float32`, the sum `float64`, stored as `float32`.
fn blend_mask(arr: &mut Img<f32>, m: &Img<u8>, col: V3, a: f64) {
    let col = col.map(|c| np::clip(c, 0.0, 255.0));
    let a32 = a as f32;
    for (i, &b) in m.data.iter().enumerate() {
        if b == 0 {
            continue; // arr + (col - arr) * 0 is arr
        }
        let w = f64::from(f32::from(b) / 255.0f32 * a32);
        for (v, &c) in arr.data[i * 3..i * 3 + 3].iter_mut().zip(&col) {
            let was = f64::from(*v);
            *v = (was + (c - was) * w) as f32;
        }
    }
}

// ---------------------------------------------------------------- v5 HORN v3: ECHO CALL, the STOPPED HORN hand, FINAL FLICK
/// `draw_ghost(arr, s, rp, spr, col, alpha, flash)`: an echo ghost, a translucent copper copy of the horn's ball
/// (screen centre `s`, radius `rp` px) with the horn inside as a copper silhouette; `flash` (0..1) when it repeats
/// its blast.
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_ghost(arr: &mut Img<f32>, s: (f64, f64), rp: f64, spr: Option<&Img<u8>>, col: V3, alpha: f64, flash: f64) {
    if alpha <= 0.01 {
        return;
    }
    let ri = np::f64_to_i64(rp.ceil()) + 2;
    let (cx, cy) = (
        np::f64_to_i64(round_half_even(s.0)),
        np::f64_to_i64(round_half_even(s.1)),
    );
    let (x0, y0) = (cx - ri, cy - ri);
    let (xa, ya) = (x0.max(0), y0.max(0));
    let (xb, yb) = ((W as i64).min(x0 + 2 * ri + 1), (H as i64).min(y0 + 2 * ri + 1));
    if xb <= xa || yb <= ya {
        return;
    }
    // tint = col * 0.75 + GHOST * 0.25
    let tint: V3 = std::array::from_fn(|c| col[c] * 0.75 + GHOST[c] * 0.25);
    let body_col: V3 = tint.map(|c| c * (1.0 + 0.4 * flash));
    let rim_col: V3 = col.map(|c| np::clip(c * 1.15 + 40.0 + 80.0 * flash, 0.0, 255.0));
    let (ka, kr) = ((0.42 + 0.25 * flash) * alpha, (0.85 + 0.15 * flash) * alpha);
    for y in ya..yb {
        for x in xa..xb {
            // yy, xx = np.mgrid[...].astype(np.float32); np.hypot(xx + 0.5 - s[0], yy + 0.5 - s[1]) / rp: the
            // float32 sum widened by the float64 subtraction
            let dx = f64::from(x as f32 + 0.5f32) - s.0;
            let dy = f64::from(y as f32 + 0.5f32) - s.1;
            let d = np::hypot(dx, dy) / rp;
            // body = np.clip(1 - d, 0, 1) ** 0.35 * (d <= 1)
            let body = pow(np::clip(1.0 - d, 0.0, 1.0), 0.35) * if d <= 1.0 { 1.0 } else { 0.0 };
            let rim = np::clip(1.0 - (d - 0.96).abs() / 0.09, 0.0, 1.0);
            let a_body = np::clip(ka * body, 0.0, 1.0);
            let a_rim = np::clip(kr * rim, 0.0, 1.0);
            let px = arr.px_mut(x as usize, y as usize);
            for c in 0..3 {
                let was = f64::from(px[c]);
                px[c] = (was + (body_col[c] - was) * a_body) as f32;
            }
            for c in 0..3 {
                let was = f64::from(px[c]);
                px[c] = (was + (rim_col[c] - was) * a_rim) as f32;
            }
        }
    }
    let Some(spr) = spr else { return };
    // the horn inside, as a copper silhouette
    let sn = spr.h as i64;
    let (sx0, sy0) = (cx - sn / 2, cy - sn / 2);
    let (ax0, ay0) = (sx0.max(0), sy0.max(0));
    let (ax1, ay1) = ((W as i64).min(sx0 + sn), (H as i64).min(sy0 + sn));
    if ax1 <= ax0 || ay1 <= ay0 {
        return;
    }
    let a32 = alpha as f32;
    // (m * 0.8 * alpha): float32
    let w = f64::from(1.0f32 * 0.8f32 * a32);
    for y in ay0..ay1 {
        for x in ax0..ax1 {
            let q = spr.px((x - sx0) as usize, (y - sy0) as usize);
            // m = (sub[..., 3] > 0).astype(np.float32)
            if q[3] == 0 {
                continue;
            }
            // lum = sub[..., :3].astype(np.float32).mean(-1, keepdims=True) / 255.0: a float32 sum and mean
            let lum = (f32::from(q[0]) + f32::from(q[1]) + f32::from(q[2])) / 3.0f32 / 255.0f32;
            // c = np.clip(col * (0.55 + 0.9 * lum) + 30 * flash, 0, 255): the factor float32, the rest float64
            let f = f64::from(0.55f32 + 0.9f32 * lum);
            let px = arr.px_mut(x as usize, y as usize);
            for c in 0..3 {
                let cc = np::clip(col[c] * f + 30.0 * flash, 0.0, 255.0);
                let was = f64::from(px[c]);
                px[c] = (was + (cc - was) * w) as f32;
            }
        }
    }
}

/// `ring_line(arr, cam, cx, cz, r, y, col, alpha, width=1, n=48)`: a blast ring at height `y`, drawn as a projected
/// circle.
#[allow(clippy::too_many_arguments)] // the Python signature
fn ring_line(arr: &mut Img<f32>, cam: &Cam, cx: f64, cz: f64, r: f64, y: f64, col: V3, alpha: f64, width: u32) {
    if alpha <= 0.01 || r <= 0.02 {
        return;
    }
    let pts: Vec<V3> = linspace(0.0, 2.0 * dmath::PI, 49)
        .iter()
        .map(|&a| scene::to_world(cx + r * dmath::cos(a), cz + r * dmath::sin(a), y))
        .collect();
    let sp = cam.wp(&pts);
    let mut im: Img<u8> = Img::new(W, H, 1);
    pil::line(&mut im, &sp, &[255], width);
    blend_mask(arr, &im, col, py_min(1.0, alpha));
}

/// `path_line(arr, cam, pts, col, alpha)`: the horn's past path on the canvas, `(x, z, a)` oldest first, a fading
/// copper thread.
fn path_line(arr: &mut Img<f32>, cam: &Cam, pts: &[(f64, f64, f64)], col: V3, alpha: f64) {
    if pts.len() < 2 {
        return;
    }
    let world: Vec<V3> = pts.iter().map(|&(x, z, _)| scene::to_world(x, z, 0.012)).collect();
    let sp = cam.wp(&world);
    let mut im: Img<u8> = Img::new(W, H, 1);
    for j in 0..pts.len() - 1 {
        // v = int(255 * max(0.0, min(1.0, pts[j + 1][2])))
        let v = np::f64_to_i64(255.0 * py_max(0.0, py_min(1.0, pts[j + 1].2)));
        if v > 3 {
            pil::line(&mut im, &[sp[j], sp[j + 1]], &[v as u8], 2);
        }
    }
    blend_mask(arr, &im, col, alpha);
}

/// `_finger(cx, base_y, w, ln, lean)`: a finger as a rounded column (screen space, up = -y), leaning `lean` x its
/// length sideways at the tip.
fn finger(cx: f64, base_y: f64, w: f64, ln: f64, lean: f64) -> Vec<(f64, f64)> {
    let mut pts = vec![(cx - w / 2.0, base_y)];
    for a in linspace(0.0, dmath::PI, 7) {
        pts.push((
            cx + lean * ln - w / 2.0 * dmath::cos(a),
            base_y - ln - w / 2.0 * dmath::sin(a) + w / 2.0,
        ));
    }
    pts.push((cx + w / 2.0, base_y));
    pts
}

/// `draw_hand(arr, s, rp, col, k, flash, side)`: STOPPED HORN, an open copper palm at screen centre `s` (sized by the
/// ball's radius `rp` px), alpha `k`; `flash` on a block; `side` (+-1) mirrors it (thumb toward the ball).
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_hand(arr: &mut Img<f32>, s: (f64, f64), rp: f64, col: V3, k: f64, flash: f64, side: f64) {
    if k <= 0.01 {
        return;
    }
    let sc = rp * 1.05;
    let base: V3 = std::array::from_fn(|c| COPPER[c] * 0.65 + col[c] * 0.35);
    let tf = |pts: &[(f64, f64)]| -> Vec<(f64, f64)> {
        pts.iter().map(|&(x, y)| (s.0 + side * x * sc, s.1 + y * sc)).collect()
    };
    let mut parts: Vec<(Vec<(f64, f64)>, V3)> = vec![(tf(&HAND_THUMB), base.map(|c| c * 0.86))];
    for (j, &(cx, by, w, ln)) in HAND_FINGERS.iter().enumerate() {
        let lean = (j as f64 - 1.5) * 0.06;
        let shade = if j % 2 == 1 { 1.04 } else { 0.95 };
        parts.push((tf(&finger(cx, by, w, ln, lean)), base.map(|c| c * shade)));
    }
    parts.push((tf(&HAND_PALM), base.map(|c| c * 1.0)));
    let mut im: Img<u8> = Img::new(W, H, 3);
    let mut mk: Img<u8> = Img::new(W, H, 1);
    for (pts, c) in &parts {
        let cc = ink(c.map(|v| v * (1.0 + 0.5 * flash) + 60.0 * flash));
        pil::polygon_outline(&mut im, pts, Some(&cc), &INK);
        pil::polygon_outline(&mut mk, pts, Some(&[255]), &[255]);
    }
    // a metal glint down the palm and the fingers
    let hi = ink(base.map(|c| c * 1.5 + 60.0));
    pil::line(&mut im, &tf(&[(-0.20, 0.42), (-0.24, 0.05)]), &hi, 1);
    for &(cx, by, w, ln) in &HAND_FINGERS[1..3] {
        let glint = tf(&[(cx - w * 0.2, by - 0.1), (cx - w * 0.2, by - ln * 0.8)]);
        pil::line(&mut im, &glint, &hi, 1);
    }
    // rgb = np.asarray(im, np.float32); m = np.asarray(mk, np.float32)[..., None] / 255.0 * k; arr += (rgb - arr) * m:
    // float32 all the way
    let k32 = k as f32;
    for (i, &b) in mk.data.iter().enumerate() {
        let m = f32::from(b) / 255.0f32 * k32;
        for c in 0..3 {
            let v = &mut arr.data[i * 3 + c];
            *v += (f32::from(im.data[i * 3 + c]) - *v) * m;
        }
    }
}

/// What a part of the glove is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Cuff,
    Finger,
    Knuckle,
    Back,
    Thumb,
}

/// `_bump(u0, v, w, ln, n=7)`: a finger seen from above, a rounded strip from `u0` forward by `ln` (width `w`).
fn bump(u0: f64, v: f64, w: f64, ln: f64) -> Vec<(f64, f64)> {
    let mut pts = vec![(u0, v - w / 2.0)];
    for a in linspace(-dmath::PI / 2.0, dmath::PI / 2.0, 7) {
        pts.push((u0 + ln - w / 2.0 + w / 2.0 * dmath::cos(a), v + w / 2.0 * dmath::sin(a)));
    }
    pts.push((u0, v + w / 2.0));
    pts
}

/// A part of the glove in its local frame: its points, its shade and its kind.
type LocalPart = (Vec<(f64, f64)>, f64, Part);

/// `glove_parts(pose)`: `(local points, shade, kind)` back to front. Pose 0 is cocked (the middle finger curled
/// under the thumb), 1 flicked (the middle finger straight out).
fn glove_parts(pose: f64) -> Vec<LocalPart> {
    let mut out = vec![(GLOVE_CUFF.to_vec(), 0.86, Part::Cuff)];
    for (j, &(v, w)) in GLOVE_KNUCKLES.iter().enumerate() {
        if j == FLICKER && pose > 0.5 {
            out.push((bump(0.02, v, w * 0.95, 0.62), 0.97, Part::Finger));
        } else {
            let ln = if j != FLICKER { 0.21 } else { 0.18 };
            out.push((bump(0.02, v, w * 1.05, ln), 0.92, Part::Knuckle));
        }
    }
    out.push((GLOVE_BACK.to_vec(), 1.0, Part::Back));
    if pose > 0.5 {
        out.push((THUMB_OPEN.to_vec(), 0.9, Part::Thumb));
    } else {
        out.push((THUMB_COCKED.to_vec(), 0.9, Part::Thumb));
    }
    out
}

/// The glove of [`HornLook::glove_at`]: `dict(p, aim, d, scale, pose, wob, light)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Glove {
    p: [f64; 2],
    aim: f64,
    d: f64,
    scale: f64,
    pose: f64,
    wob: f64,
    light: f64,
}

/// A glove's world polygons with their shades and kinds, and its stitch lines.
type GloveWorld = (Vec<(Vec<V3>, f64, Part)>, Vec<Vec<V3>>);

/// `glove_world(p, aim, d, scale, pose, wob, y=R)`: the glove's world polygons `(Q, shade, kind)` (origin at the
/// ball `p + aim * d`, scaled, wobbling by `wob` rad) and its three stitch lines.
fn glove_world(g: &Glove) -> GloveWorld {
    let y = R;
    let a = g.aim + g.wob;
    let du = unit(a);
    let dv = [-du[1], du[0]];
    let au = unit(g.aim);
    let o = [g.p[0] + au[0] * g.d, g.p[1] + au[1] * g.d];
    // o + du * u * scale + dv * v * scale
    let at = |u: f64, v: f64| -> V3 {
        let q: [f64; 2] = std::array::from_fn(|c| o[c] + du[c] * u * g.scale + dv[c] * v * g.scale);
        scene::to_world(q[0], q[1], y)
    };
    let parts = glove_parts(g.pose)
        .into_iter()
        .map(|(pts, sh, kind)| (pts.iter().map(|&(u, v)| at(u, v)).collect(), sh, kind))
        .collect();
    // the three stitch lines on the back of a cartoon glove
    let stitches = [-0.10, 0.0, 0.10]
        .iter()
        .map(|&v| [-0.30, -0.04].iter().map(|&u| at(u, v)).collect())
        .collect();
    (parts, stitches)
}

/// `draw_glove(arr, cam, p, aim, d, scale, pose, col, k=1.0, wob, light)`: the glove and its copper spring from the
/// ball's surface to the cuff; copper-lit (the horn's colour on the edges).
fn draw_glove(arr: &mut Img<f32>, cam: &Cam, g: &Glove, col: V3) {
    let k = 1.0f32;
    if g.scale <= 0.02 {
        return;
    }
    let (parts, stitches) = glove_world(g);
    let mut im = to_bytes(arr);
    let u = unit(g.aim);
    let nrm = [-u[1], u[0]];
    let s0 = R * 0.85;
    let s1 = g.d - 0.66 * g.scale;
    if s1 > s0 + 0.02 {
        // the spring: a copper zig-zag coil
        let ncoil = 9;
        let pts: Vec<V3> = (0..2 * ncoil + 1)
            .map(|j| {
                let along = s0 + (s1 - s0) * j as f64 / (2 * ncoil) as f64;
                // nrm * (0.11 * scale * (1 if j % 2 else -1) if 0 < j < 2 * ncoil else 0)
                let off = if 0 < j && j < 2 * ncoil {
                    0.11 * g.scale * if j % 2 == 1 { 1.0 } else { -1.0 }
                } else {
                    0.0
                };
                let q: [f64; 2] = std::array::from_fn(|c| g.p[c] + u[c] * along + nrm[c] * off);
                scene::to_world(q[0], q[1], R)
            })
            .collect();
        let sp = cam.wp(&pts);
        pil::line(&mut im, &sp, &ints(COPPER_DK), 4);
        pil::line(&mut im, &sp, &ink(COPPER.map(|c| c * 1.25 + 30.0)), 2);
    }
    let tint: V3 = std::array::from_fn(|c| WHITE[c] * 0.86 + col[c] * 0.14);
    for (q, sh, _) in &parts {
        let sp = cam.wp(q);
        let c = ink(tint.map(|v| v * sh * g.light));
        pil::polygon_outline(&mut im, &sp, Some(&c), &INK);
    }
    for st in &stitches {
        let sp = cam.wp(st);
        pil::line(&mut im, &sp, &[176, 160, 160], 1);
    }
    // copper light on the edge facing the horn: the back of the hand (the thumb is always the last part)
    let rim = ink(col.map(|c| c * 1.1 + 40.0));
    let back = if parts[parts.len() - 1].2 == Part::Thumb {
        &parts[parts.len() - 2].0
    } else {
        &parts[parts.len() - 1].0
    };
    let sp = cam.wp(back);
    pil::line(&mut im, &sp[..3], &rim, 1);
    // new = np.asarray(im, np.float32); arr += (new - arr) * k: float32
    for (v, &b) in arr.data.iter_mut().zip(&im.data) {
        *v += (f32::from(b) - *v) * k;
    }
}

/// `glove_shadow_polys(p, aim, d, scale, pose, wob)`: the glove's world polygons.
fn glove_shadow_polys(g: &Glove) -> Vec<Vec<V3>> {
    glove_world(g).0.into_iter().map(|(q, _, _)| q).collect()
}

/// The Bayer value of canvas pixel `j`: `BAYER_FULL[C.ys, C.xs]`.
fn bayer_at(c: &CanvasCtx<'_>, j: usize) -> f64 {
    config::bayer_full().px(c.xs[j], c.ys[j])[0]
}

/// `echo_band(Ic, sxp, szp, cx, cz, r, width, k, col, bayer)`: a blast ring's light on the canvas.
#[allow(clippy::too_many_arguments)] // the Python signature
fn echo_band(ic: &mut [V3], c: &CanvasCtx<'_>, cx: f64, cz: f64, r: f64, width: f64, k: f64, col: V3) {
    let tint = col.map(|v| v / 255.0 * 1.3 + 0.12);
    for (j, light) in ic.iter_mut().enumerate() {
        let rd = np::hypot(c.sxp[j] - cx, c.szp[j] - cz);
        let band = np::clip(1.0 - (rd - r).abs() / width, 0.0, 1.0);
        // band = np.floor(band * 3 + bayer * 0.9) / 3
        let band = (band * 3.0 + bayer_at(c, j) * 0.9).floor() / 3.0;
        let v = band * k;
        for ch in 0..3 {
            light[ch] += v * tint[ch];
        }
    }
}

/// `aim_lane(Ic, sxp, szp, p, aim, length, k, col, bayer, w=0.26)`: FINAL FLICK's wind-up, the line the glove will
/// flick along, lit on the canvas.
#[allow(clippy::too_many_arguments)] // the Python signature
fn aim_lane(ic: &mut [V3], c: &CanvasCtx<'_>, p: [f64; 2], aim: f64, length: f64, k: f64, col: V3) {
    let w = 0.26;
    let d = unit(aim);
    let tint = col.map(|v| v / 255.0 * 0.9 + 0.15);
    for (j, light) in ic.iter_mut().enumerate() {
        let (rx, rz) = (c.sxp[j] - p[0], c.szp[j] - p[1]);
        let along = rx * d[0] + rz * d[1];
        let across = (-rx * d[1] + rz * d[0]).abs();
        let gate = if along > R && along < R + length { 1.0 } else { 0.0 };
        let lane = np::clip(1.0 - across / w, 0.0, 1.0) * gate;
        let lane = (lane * 3.0 + bayer_at(c, j) * 0.9).floor() / 3.0;
        let v = lane * k;
        for ch in 0..3 {
            light[ch] += v * tint[ch];
        }
    }
}

/// A live blast ring of `rings_at`: `(owner, x, z, r, j, alpha)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct RingAt {
    owner: usize,
    x: f64,
    z: f64,
    r: f64,
    j: i64,
    a: f64,
}

/// An echo ghost of `ghosts_at`: `(slot, x, z, alpha, flash)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct GhostAt {
    x: f64,
    z: f64,
    a: f64,
    fl: f64,
}

/// What a painter item of this look is.
enum Item {
    /// `('rings',)`.
    Rings,
    /// `('hand',)`.
    Hand,
    /// `('ghost', j, x, z, a, fl)`.
    Ghost(GhostAt),
    /// `('glove', g)`.
    Glove(Glove),
    /// `('after', vic, q, a)`.
    After(usize, [f64; 2], f64),
}

/// HORN's look: `Look` of `kit/fighters/horn.py`.
#[derive(Clone, Debug, Default)]
pub struct HornLook {
    /// Its side.
    k: usize,
    /// `self.col`: its colour.
    col: V3,
    /// `self.echo`: its weapon is an echo.
    echo: bool,
    /// `self.flicker`: its low ability is a flick.
    flicker: bool,
    /// `self.cfg['weapon']['speed']` and `.get('linger', 0.25)` of an echo weapon.
    ring: Option<(f64, f64)>,
    /// `self.cfg['abilities']['unlock1']['dur_beats']`.
    dur_beats1: Option<f64>,
    /// `self.cfg['abilities']['low']['reach']`.
    reach: Option<f64>,
    /// `self.cfg['view']`.
    view: (f64, f64, f64),
    /// `REP_T`: when each ghost slot (0 the call, 1..4 the echoes) sounded a blast.
    rep_t: [Vec<f64>; 5],
    /// `GROW_T`: ghost n, when it was earned.
    grow_t: Vec<(i64, f64)>,
    /// `BLOCK_T`: when a hit was blocked on it.
    block_t: Vec<f64>,
    /// `FLUNG`: `(t, victim)` of FINAL FLICK's landed hits.
    flung: Vec<(f64, usize)>,
}

impl HornLook {
    /// `GROW_T.get(n, T.T_FIGHT)`.
    fn grow_t(&self, n: i64, t_fight: f64) -> f64 {
        // a dict built in event order: the last entry of a key wins
        self.grow_t
            .iter()
            .rev()
            .find(|&&(m, _)| m == n)
            .map_or(t_fight, |&(_, t)| t)
    }

    /// `ghosts_at(t)`: its echo ghosts at `t`. In the fight, the run's ghosts on its own past path (frozen through
    /// the hit-stop); each fades in when it is earned and flashes when it repeats a blast. After the K.O. a
    /// winner's ghosts fade out, a loser's burst into embers.
    fn ghosts_at(&self, p: &PictureCtx, t: f64) -> Vec<GhostAt> {
        let g = &p.show.grid;
        if !p.run.has("ECHO_G") || !self.echo || t < g.t_fight {
            return Vec::new();
        }
        let k = self.k;
        let tc = p.fx_clock(t);
        let i = p.sim_i(py_min(t, p.t_ko));
        let n = np::f64_to_i64(p.run.f("ECHO_N", i, k).unwrap_or(0.0) + 1e-9).max(0) as usize;
        let mut out_k = 1.0;
        if t >= p.t_hs {
            out_k = if k == p.lose {
                0.0
            } else {
                1.0 - ss(p.t_hs, p.t_hs + 0.8, t)
            };
            if out_k <= 0.0 {
                return Vec::new();
            }
        }
        let Some(gg) = p.run.row("ECHO_G", i, k) else {
            return Vec::new();
        };
        (0..n.min(AGE_A.len()))
            .map(|j| {
                let (x, z) = (gg[j * 3], gg[j * 3 + 1]);
                let tg = self.grow_t(j as i64 + 1, g.t_fight);
                let a = AGE_A[j] * ss(tg, tg + 0.35, tc) * out_k;
                let fl = last(&self.rep_t[j + 1], tc).map_or(0.0, |te| dmath::exp(-(tc - te) / 0.14));
                GhostAt { x, z, a, fl }
            })
            .collect()
    }

    /// `rings_at(t)`: its live blast rings at `t` (j = 0 the call, 1..4 the echoes), lingering as they fade.
    fn rings_at(&self, p: &PictureCtx, t: f64) -> Vec<RingAt> {
        let g = &p.show.grid;
        if !p.run.has("ECHO_R") || !(g.t_fight <= t && t < p.t_hs) {
            return Vec::new();
        }
        let i = p.sim_i(py_min(t, p.t_ko));
        let tc = p.fx_clock(t);
        let Some(Value::List(rows)) = p.run.nested("ECHO_R", i) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for row in rows {
            let Value::List(v) = row else { continue };
            let Some(owner) = v.first().and_then(num) else { continue };
            if np::f64_to_i64(owner) != self.k as i64 {
                continue;
            }
            // (the look's own weapon's numbers: Python reads `self.cfg['weapon']` for its own rings)
            let Some((speed, linger)) = self.ring else { continue };
            let field = |n: usize| v.get(n).and_then(num).unwrap_or(f64::NAN);
            let (x, z, rmax, j, t0) = (field(1), field(2), field(4), field(5), field(6));
            let j = np::f64_to_i64(j);
            let r0 = if j == 0 { R } else { R * 0.5 };
            let te = t0 + (rmax - r0) / speed;
            let rr = py_min(rmax, r0 + py_max(0.0, tc - t0) * speed);
            let a = if tc < te {
                1.0
            } else {
                py_max(0.0, 1.0 - (tc - te) / linger)
            };
            out.push(RingAt {
                owner: self.k,
                x,
                z,
                r: rr,
                j,
                a,
            });
        }
        out
    }

    /// `glove_at(t)`: FINAL FLICK's glove at `t`, or `None`. It bursts out of the bell (overshoot), creeps back with
    /// a growing tremble while it is cocked (one beat), snaps out to the enemy's surface in a frame, holds, then
    /// shrinks back into the bell.
    fn glove_at(&self, p: &PictureCtx, t: f64) -> Option<Glove> {
        let g = &p.show.grid;
        if !p.run.has("FLICK") || !self.flicker || !(g.t_fight <= t && t < g.t_glide) {
            return None;
        }
        let i = p.sim_i(py_min(t, p.t_ko));
        let row = p.run.row("FLICK", i, self.k)?;
        let (st, aim, ext, t0, th, hit) = (row[0], row[1], row[2], row[3], row[4], row[5]);
        if st.is_nan() {
            return None;
        }
        let tc = p.fx_clock(t);
        let pp = p.fighter(self.k, t).p;
        let d_cock = R + 0.62 * G_SCALE;
        if tc < th {
            let tau = py_max(0.0, tc - t0);
            let f = back_ease(py_min(1.0, tau / 0.16));
            let uw = py_min(1.0, tau / py_max(1e-3, th - t0));
            let d = R * 0.7 + (d_cock + 0.14 * (1.0 - uw) - R * 0.7) * f;
            let wob = 0.09 * dmath::sin(2.0 * dmath::PI * 11.0 * tau) * pow(uw, 2.0);
            return Some(Glove {
                p: pp,
                aim,
                d,
                scale: G_SCALE * py_max(0.0, f),
                pose: 0.0,
                wob,
                light: 1.0 + 0.25 * uw,
            });
        }
        let tau = tc - th;
        let full = py_max(R + 0.55, R + ext - 0.62 * G_SCALE);
        let (d, sc, pose) = if tau < 0.034 {
            (d_cock + (full - d_cock) * ease(tau / 0.034), G_SCALE, 1.0)
        } else if tau < 0.2 {
            (full, G_SCALE, 1.0)
        } else {
            let u = py_min(1.0, (tau - 0.2) / 0.38);
            if u >= 1.0 {
                return None;
            }
            (full + (R * 0.7 - full) * ease(u), G_SCALE * (1.0 - ease(u)), 1.0)
        };
        Some(Glove {
            p: pp,
            aim,
            d,
            scale: sc,
            pose,
            wob: 0.0,
            light: 1.0 + 0.6 * dmath::exp(-tau / 0.08) * hit,
        })
    }

    /// `hand_k(t, st)`: STOPPED HORN, the copper hand's strength (it fades in and out with the shield) and its flash
    /// on a block.
    fn hand_k(&self, p: &PictureCtx, t: f64, st: &BallState) -> (f64, f64) {
        if st.shield <= 0.0 {
            return (0.0, 0.0);
        }
        let beats = self
            .dur_beats1
            .expect("HORN's look reads abilities.unlock1.dur_beats (Python raises without it)");
        let dur = beats * p.show.grid.beat;
        let a = py_min(py_min(1.0, st.shield / 0.2), (dur - st.shield) / 0.1 + 0.15);
        let tc = p.fx_clock(t);
        let flash = last(&self.block_t, tc).map_or(0.0, |tb| dmath::exp(-(tc - tb) / 0.12));
        (a, flash)
    }
}

/// `n` particles, `spread` and `speed` as `PA.emit` takes them.
fn emit(n: i64, spread: Option<f64>, speed: Option<(f64, f64)>) -> Emit {
    Emit {
        n: Some(n),
        spread,
        speed,
        ..Emit::default()
    }
}

impl Look for HornLook {
    fn shards(&self) -> Vec<String> {
        // ['tube'] * 6 + ['valve'] * 3 + ['bellbit'] * 6
        [("tube", 6), ("valve", 3), ("bellbit", 6)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["slam", "wave", "shockwave", "cannon", "flick", "flick_rope"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["slam", "wave", "shockwave", "flick", "flick_rope"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["slam", "wave", "shockwave", "flick", "flick_rope"]
    }

    /// The lance bangs off the copper hand.
    fn block_burst(&self) -> [f64; 3] {
        [255.0, 220.0, 190.0]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// What `setup_as` took from `cfg`, by the keys `horn.py` reads from `self.cfg`.
    fn cfg_summary(&self) -> Option<String> {
        Some(format!(
            "abilities.low.kind={}; abilities.low.reach={:?}; abilities.unlock1.dur_beats={:?}; view={:?}; \
             weapon.kind={}; weapon.linger={:?}; weapon.speed={:?}",
            if self.flicker { "flick" } else { "other" },
            self.reach,
            self.dur_beats1,
            self.view,
            if self.echo { "echo" } else { "other" },
            self.ring.map(|r| r.1),
            self.ring.map(|r| r.0),
        ))
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`: the weapon's and abilities' numbers and the view come
    /// from `cfg`, the events from side `k`.
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.col = p.col[k];
        self.ring = match &cfg.weapon {
            WeaponCfg::Echo(w) => Some((w.speed, w.linger)),
            _ => None,
        };
        self.echo = self.ring.is_some();
        self.flicker = cfg.low.kind == AbilityKind::Flick;
        self.dur_beats1 = cfg.unlock1.get("dur_beats");
        self.reach = cfg.low.get("reach");
        let view = |i: usize| -> f64 {
            cfg.raw
                .get("view")
                .and_then(|v| v.get(i))
                .and_then(|r| r.as_f64())
                .expect("a roster entry the picture accepted has a view (yaw, swing, roll)")
        };
        self.view = (view(0), view(1), view(2));
        let ev = &p.rec.events;
        let mine = |e: &&Event| e.who() == Some(k);
        // REP_T = {j: np.array([t for (t, jj, ...) in ev if jj == j]) for j in range(5)}
        self.rep_t = Default::default();
        for e in ev.iter().filter(|e| e.kind == "echo").filter(mine) {
            if let Some(j) = e.fields.get(1).and_then(num) {
                let j = np::f64_to_i64(j);
                if (0..5).contains(&j) {
                    self.rep_t[j as usize].push(e.t);
                }
            }
        }
        self.grow_t = ev
            .iter()
            .filter(|e| e.kind == "echo_grow")
            .filter(mine)
            .filter_map(|e| Some((np::f64_to_i64(num(e.fields.get(1)?)?), e.t)))
            .collect();
        self.block_t = ev
            .iter()
            .filter(|e| e.kind == "block")
            .filter(mine)
            .map(|e| e.t)
            .collect();
        // FLUNG = [(e[0], e[3]) for e in EV if e[1] == 'hit' and e[7] == 'flick' and e[2] == k]
        self.flung = ev
            .iter()
            .filter(|e| how(e) == "flick")
            .filter(mine)
            .filter_map(|e| Some((e.t, usize::try_from(np::f64_to_i64(num(e.fields.get(1)?)?)).ok()?)))
            .collect();
    }

    // ------------------------------------------------------------ events
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        match how(e) {
            "echo" => Some(0.35),
            "flick" => Some(2.5),
            "flick_rope" => Some(2.2),
            _ => DefaultLook.shake(p, e),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        let wide = ["flick", "flick_rope", "slam", "wave", "shockwave"];
        if wide.contains(&how(e)) {
            Some((14.0, 0.35))
        } else {
            None
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        let t = e.t;
        let up = scene::to_world;
        match e.kind.as_str() {
            "hit" => {
                let Some(att) = e.who() else { return };
                let (x, z) = (e.f(5), e.f(6));
                let pos = up(x, z, R);
                let call = e.fields.get(6).and_then(num) == Some(0.0);
                match how(e) {
                    "echo" => {
                        // a blast lands: copper embers, a soft scorch ring
                        fx.bu
                            .add(t, pos, p.col[att], if call { 0.5 } else { 0.35 }, None, false);
                        fx.fl.add(t, pos, p.fl_col[att], 1.1, 0.45, 0.03, 0.2, 0.0);
                        fx.pa
                            .emit(t, pos, "embers", &emit(if call { 6 } else { 4 }, None, None));
                    }
                    "flick" => {
                        // FINAL FLICK lands: the big one
                        fx.bu.add(t, pos, p.col[att], 2.2, None, true);
                        fx.fl.add(t, pos, [255.0, 236.0, 214.0], 2.4, 1.1, 0.03, 0.45, 0.0);
                        fx.pa.emit(t, pos, "embers", &emit(22, Some(180.0), Some((0.8, 2.4))));
                        fx.pa.emit(t, pos, "brass", &emit(10, Some(120.0), None));
                    }
                    "flick_rope" => {
                        // ... and the enemy slams into the ropes
                        fx.bu.add(t, pos, p.col[att], 1.8, None, true);
                        fx.fl.add(t, pos, p.fl_col[att], 2.0, 0.9, 0.03, 0.4, 0.0);
                        fx.pa
                            .emit(t, up(x, z, 0.1), "dust", &emit(18, Some(160.0), Some((0.6, 1.6))));
                        fx.pa.emit(t, pos, "brass", &emit(8, Some(140.0), None));
                    }
                    _ => {}
                }
            }
            "echo" => {
                // a blast: the call (j = 0) or a ghost's repeat
                let Some(who) = e.who() else { return };
                let j = e.i(3);
                let pos = up(e.f(5), e.f(6), 0.5);
                let (radius, peak) = if j == 0 {
                    (1.3, 0.30)
                } else {
                    (1.0, 0.16 * pow(0.85, j as f64))
                };
                fx.fl.add(t, pos, p.fl_col[who], radius, peak, 0.03, 0.2, 0.0);
                if j > 0 {
                    let small = Emit {
                        n: Some(2),
                        speed: Some((0.3, 0.9)),
                        life: Some((0.3, 0.6)),
                        ..Emit::default()
                    };
                    fx.pa.emit(t, up(e.f(5), e.f(6), 0.35), "embers", &small);
                }
            }
            "echo_grow" => {
                // a new ghost peels off the horn
                let Some(who) = e.who() else { return };
                let q = p.fighter(who, t).p;
                fx.pa.emit(
                    t,
                    up(q[0], q[1], 0.5),
                    "embers",
                    &emit(8, Some(180.0), Some((0.5, 1.4))),
                );
                fx.fl
                    .add(t, up(q[0], q[1], 0.6), p.fl_col[who], 1.2, 0.35, 0.03, 0.3, 0.0);
            }
            "flick_wind" => {
                // the glove bursts out of the bell
                let Some(who) = e.who() else { return };
                let q = p.fighter(who, t).p;
                let aim = e.f(4);
                let pos = up(q[0] + 0.9 * dmath::cos(aim), q[1] + 0.9 * dmath::sin(aim), R);
                fx.pa.emit(t, pos, "embers", &emit(10, Some(120.0), Some((0.6, 1.6))));
                fx.fl.add(t, pos, [255.0, 236.0, 214.0], 1.6, 0.55, 0.03, 0.35, 0.0);
            }
            "block" => {
                // (the kit added the burst: block_burst)
                fx.pa.emit(t, up(e.f(4), e.f(5), R), "embers", &emit(5, None, None));
            }
            _ => {}
        }
    }

    fn fx_after(&self, p: &PictureCtx, fx: &mut Fx) {
        // its ghosts burst at the end of the hit-stop
        if self.k == p.lose {
            for gh in self.ghosts_at(p, p.t_hs - 1e-3) {
                let burst = emit(10, Some(180.0), Some((0.8, 2.0)));
                fx.pa
                    .emit(p.fx_clock(p.t_hs), scene::to_world(gh.x, gh.z, R), "embers", &burst);
            }
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        fx.pa
            .emit(p.t_ko, pos, "embers", &emit(30, Some(180.0), Some((0.8, 2.2))));
    }

    // ------------------------------------------------------------ the canvas
    fn shadows_free(&self, p: &PictureCtx, f: &FrameCtx<'_>, spot: V3) -> Vec<Vec<(f64, f64)>> {
        // the glove casts a real shadow
        match self.glove_at(p, f.t) {
            Some(g) if g.scale > 0.05 => glove_shadow_polys(&g)
                .iter()
                .map(|q| proj_point(q, spot, 0.93))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let k = self.k;
        // the call / echo rings light the canvas
        for r in self.rings_at(p, c.t) {
            let (width, kk) = if r.j == 0 {
                (0.11, r.a * 0.95)
            } else {
                (0.06, r.a * (0.6 * pow(0.85, (r.j - 1) as f64)))
            };
            echo_band(ic, c, r.x, r.z, r.r, width, kk, p.col[r.owner]);
        }
        // ghosts cast no shadow: a copper glow instead
        let tint = p.col[k].map(|v| v / 255.0 + 0.1);
        for gh in self.ghosts_at(p, c.t) {
            let s = (0.22 + 0.35 * gh.fl) * gh.a;
            for (j, light) in ic.iter_mut().enumerate() {
                let rd = np::hypot(c.sxp[j] - gh.x, c.szp[j] - gh.z);
                let g = np::clip(1.0 - rd / 0.75, 0.0, 1.0);
                let v = g * g * s;
                for ch in 0..3 {
                    light[ch] += v * tint[ch];
                }
            }
        }
        // FINAL FLICK: the lane the glove will flick along
        if let Some(g) = self.glove_at(p, c.t)
            && g.pose < 0.5
        {
            let pk = 0.25 + 0.2 * (0.5 + 0.5 * dmath::sin(c.t * 2.0 * dmath::PI * 8.0));
            let reach = self
                .reach
                .expect("HORN's look reads abilities.low.reach (Python raises without it)");
            aim_lane(ic, c, g.p, g.aim, reach + R, pk * g.scale, p.col[k]);
        }
    }

    /// A fading copper thread along its path, back to its oldest ghost.
    fn canvas_overlay(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        let k = self.k;
        if !(f.in_fight && p.run.has("ECHO_G") && self.echo) {
            return;
        }
        let i = f.i;
        let n = np::f64_to_i64(p.run.f("ECHO_N", i, k).unwrap_or(0.0) + 1e-9);
        if n <= 0 {
            return;
        }
        let Some(gg) = p.run.row("ECHO_G", i, k) else { return };
        let lag = gg[(n as usize - 1) * 3 + 2] + 0.1;
        let m = np::f64_to_i64(lag * f64::from(FPS));
        // for back in range(m_, -1, -1): q = P.RUN['P'][max(0, i - back), k]
        let pts: Vec<(f64, f64, f64)> = (0..=m)
            .rev()
            .map(|back| {
                let q = p.run.p((i as i64 - back).max(0) as usize, k);
                (q[0], q[1], 0.55 * (1.0 - back as f64 / m.max(1) as f64))
            })
            .collect();
        let col = p.col[k].map(|c| np::clip(c * 0.9 + 30.0, 0.0, 255.0));
        path_line(arr, f.cam, &pts, col, 0.6);
    }

    // ------------------------------------------------------------ the painter pass
    fn items_ball(&self, _p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        // STOPPED HORN: just in front of its ball
        if st.shield > 0.0 && !st.shattered {
            let dz = f.cam.depth(&[scene::to_world(st.p[0], st.p[1], R)])[0];
            let hand: Payload = Box::new(Item::Hand);
            return vec![(dz - 0.05, hand)];
        }
        Vec::new()
    }

    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let (cam, t) = (f.cam, f.t);
        let mut items: Vec<(f64, Payload)> = Vec::new();
        // the blast rings, raised: first in the painter pass
        if !self.rings_at(p, t).is_empty() {
            items.push((1e9, Box::new(Item::Rings)));
        }
        for gh in self.ghosts_at(p, t) {
            let dz = cam.depth(&[scene::to_world(gh.x, gh.z, R)])[0];
            items.push((dz + 0.02, Box::new(Item::Ghost(gh))));
        }
        if let Some(g) = self.glove_at(p, t)
            && g.scale > 0.02
        {
            let au = unit(g.aim);
            let c = [g.p[0] + au[0] * g.d, g.p[1] + au[1] * g.d];
            let dz = cam.depth(&[scene::to_world(c[0], c[1], R)])[0];
            items.push((dz - 0.02, Box::new(Item::Glove(g))));
        }
        // the flung enemy's afterimages
        let tcf = p.fx_clock(t);
        for &(tf, vic) in &self.flung {
            if 0.0 <= tcf - tf && tcf - tf < 0.5 && f.fs[vic].vis {
                for (m, a) in [(1.0, 0.34), (2.0, 0.22), (3.0, 0.12)] {
                    let q = p.fighter(vic, py_max(tf, py_min(t, p.t_ko) - 0.035 * m)).p;
                    let dz = cam.depth(&[scene::to_world(q[0], q[1], R)])[0];
                    let after = Item::After(vic, q, a * (1.0 - (tcf - tf) / 0.5));
                    items.push((dz + 0.05 * m, Box::new(after)));
                }
            }
        }
        items
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let (k, cam, t) = (self.k, f.cam, f.t);
        let Some(item) = payload.downcast_ref::<Item>() else {
            return;
        };
        let (mut arr, depth) = (fr.array(), fr.depth());
        match item {
            Item::Rings => {
                // a burst of sound: the call bright, echoes fainter
                for r in self.rings_at(p, t) {
                    let c = p.col[r.owner];
                    if r.j == 0 {
                        let warm = [255.0, 214.0, 150.0];
                        let col: V3 = std::array::from_fn(|i| np::clip(c[i] * 0.7 + warm[i] * 0.45, 0.0, 255.0));
                        ring_line(&mut arr, cam, r.x, r.z, r.r, 0.32, col, 0.9 * r.a, 2);
                    } else {
                        let col = c.map(|v| np::clip(v * 1.05 + 20.0, 0.0, 255.0));
                        let a = 0.75 * pow(0.85, (r.j - 1) as f64) * r.a;
                        ring_line(&mut arr, cam, r.x, r.z, r.r, 0.32, col, a, 1);
                    }
                }
            }
            Item::Hand => {
                // STOPPED HORN: the copper palm plugs the bell
                let st = &f.fs[k];
                let (hk, hf) = self.hand_k(p, t, st);
                // (its ball's screen spot, as draw_ball found it)
                let pw = scene::to_world(st.p[0], st.p[1], R);
                let pc = cam.to_cam(&[pw])[0];
                let s = cam.proj(&[pc])[0];
                let rp = R * cam.f / pc[2] * st.scale;
                let q = f.fs[1 - k].p;
                let so = cam.wp(&[scene::to_world(q[0], q[1], R)])[0];
                let dv = [so.0 - s.0, so.1 - s.1];
                let n = norm2(dv) + 1e-9;
                let dv = [dv[0] / n, dv[1] / n];
                let dome_col = p.col[k].map(|c| np::clip(c * 1.1 + 50.0, 0.0, 255.0));
                for y in 0..H {
                    for x in 0..W {
                        // dome = np.clip(1 - np.abs(np.hypot(xx_ - s_[0], yy_ - s_[1]) - rp * 1.22) / 1.2, 0, 1)
                        let dd = np::hypot(x as f64 - s.0, y as f64 - s.1);
                        let dome = np::clip(1.0 - (dd - rp * 1.22).abs() / 1.2, 0.0, 1.0);
                        let w = dome * 0.55 * hk;
                        let px = arr.px_mut(x, y);
                        for c in 0..3 {
                            let was = f64::from(px[c]);
                            px[c] = (was + (dome_col[c] - was) * w) as f32;
                        }
                    }
                }
                let c = (s.0 + dv[0] * rp * 0.72, s.1 + dv[1] * rp * 0.72 + rp * 0.1);
                let side = if dv[0] >= 0.0 { 1.0 } else { -1.0 };
                draw_hand(&mut arr, c, rp * (1.2 + 0.1 * hf), p.col[k], hk, hf, side);
            }
            Item::Ghost(gh) => {
                let pw = scene::to_world(gh.x, gh.z, R);
                let pc = cam.to_cam(&[pw])[0];
                let s = cam.proj(&[pc])[0];
                let rp = R * cam.f / pc[2];
                let (yc, sw, roll) = self.view;
                let spr = p.sprite(k, rp * 0.80, yc + sw * dmath::sin(1.25 * gh.x + 0.45 * gh.z), roll);
                draw_ghost(&mut arr, s, rp, Some(&spr), p.col[k], gh.a, gh.fl);
            }
            Item::Glove(g) => draw_glove(&mut arr, cam, g, p.col[k]),
            Item::After(vic, q, a) => {
                let pw = scene::to_world(q[0], q[1], R);
                let pc = cam.to_cam(&[pw])[0];
                let s = cam.proj(&[pc])[0];
                draw_ghost(&mut arr, s, R * cam.f / pc[2], None, p.col[*vic], *a, 0.3);
            }
        }
        fr.set_array(&arr, Some(&depth));
    }

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
        let (sx, sz) = (pos[0], pos[2]);
        let d = unit(rot);
        let nn = [-d[1], d[0]];
        let on_screen = |pts: &[[f64; 2]]| -> Vec<(f64, f64)> {
            let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], pos[1])).collect();
            screen(cam, &world)
        };
        if kind == "tube" {
            // a curl of brass tubing
            let pts: Vec<[f64; 2]> = (0..7)
                .map(|j| {
                    let a = rot + f64::from(j) * 0.5 * (1.0 + curl);
                    [sx + 0.16 * dmath::cos(a), sz + 0.16 * dmath::sin(a)]
                })
                .collect();
            let sp = on_screen(&pts);
            let mut im = to_bytes(arr);
            pil::line(&mut im, &sp, &ints(BRASS), 2);
            // np.asarray(im).astype(np.float32)
            for (v, &b) in arr.data.iter_mut().zip(&im.data) {
                *v = f32::from(b);
            }
            return;
        }
        // valve (a rotary valve: a squat disc), bellbit (a curved chip of the copper bell); Python has no colour for
        // another kind and raises (`shards` names no other)
        let col = match kind {
            "valve" => BRASS_DK.map(|c| c * 1.3),
            "bellbit" => COPPER,
            _ => return,
        };
        let s = size * if kind == "bellbit" { 1.5 } else { 0.8 };
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
        let col = col.map(|c| np::clip(c * shade_k, 0.0, 255.0));
        layer_blend(arr, &[on_screen(&tri)], col, 1.0, None);
    }

    // ------------------------------------------------------------ lens, glows
    fn shimmer_after(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64) {
        // the echo rings shimmer while they expand
        for r in self.rings_at(p, t) {
            if r.a >= 1.0 {
                let (width, amp) = if r.j == 0 { (0.24, 2.2) } else { (0.16, 1.4) };
                let k = 0.9 * pow(0.85, r.j as f64);
                let seed = (r.j + 3 * r.owner as i64) as f64;
                img.shimmer_world(cam, scene::to_world(r.x, r.z, 0.05), r.r, width, amp, t, k, seed);
            }
        }
    }

    fn glows(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, _drawn: &[DrawnBall]) {
        // a copper aura on every ghost, brighter as it echoes
        for gh in self.ghosts_at(p, f.t) {
            let s = f.cam.wp(&[scene::to_world(gh.x, gh.z, R)])[0];
            fr.glow(
                s.0,
                s.1,
                12.0,
                self.col.map(|c| c * 0.7 + 60.0),
                (0.10 + 0.28 * gh.fl) * gh.a,
            );
        }
    }

    fn focus_points(&self, p: &PictureCtx, t: f64, _st: &BallState) -> Vec<(V3, String)> {
        let mut out = Vec::new();
        // the echo ghosts (ball-sized)
        for gh in self.ghosts_at(p, t) {
            for dy in [0.0, 2.0 * R] {
                out.push((scene::to_world(gh.x, gh.z, dy), "echo ghost".to_string()));
            }
        }
        // the glove (cuff and fingertip)
        if let Some(g) = self.glove_at(p, t) {
            let u = unit(g.aim);
            for d in [g.d - 0.66 * g.scale, g.d + 0.62 * g.scale] {
                out.push((
                    scene::to_world(g.p[0] + u[0] * d, g.p[1] + u[1] * d, R),
                    "glove".to_string(),
                ));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `_back_ease` starts at 0, overshoots past 1 and settles at 1.
    #[test]
    fn the_back_ease_overshoots() {
        assert!(back_ease(0.0).abs() < 1e-15);
        assert_eq!(back_ease(1.0), 1.0);
        assert!(back_ease(0.8) > 1.0);
    }

    /// The glove: the cocked pose has four knuckles and the curled thumb, the flicked one the middle finger out.
    #[test]
    fn the_glove_flicks_its_middle_finger() {
        let cocked = glove_parts(0.0);
        let flicked = glove_parts(1.0);
        assert_eq!(cocked.len(), 7);
        assert_eq!(flicked.len(), 7);
        assert_eq!(cocked.iter().filter(|p| p.2 == Part::Knuckle).count(), 4);
        assert_eq!(flicked.iter().filter(|p| p.2 == Part::Finger).count(), 1);
        assert_eq!((cocked[6].2, cocked[6].0.len()), (Part::Thumb, 7));
        assert_eq!((flicked[6].2, flicked[6].0.len()), (Part::Thumb, 6));
        assert_eq!((cocked[0].2, cocked[5].2), (Part::Cuff, Part::Back));
        // a finger is a rounded strip: 7 round points between its two base corners
        assert_eq!(bump(0.02, 0.0, 0.1, 0.6).len(), 9);
        assert_eq!(finger(0.0, 0.0, 0.2, 0.7, 0.0).len(), 9);
    }

    /// `_last`: the last value at or before the time.
    #[test]
    fn last_finds_the_value_at_or_before() {
        let v = [1.0, 2.0, 2.0, 3.5];
        assert_eq!(last(&v, 0.5), None);
        assert_eq!(last(&v, 2.0), Some(2.0));
        assert_eq!(last(&v, 9.0), Some(3.5));
        assert_eq!(last(&[], 1.0), None);
    }
}
