//! The FLUTE's look: the port of the `Look` half of `projects/ballfight/kit/fighters/flute.py`.
//!
//! The pixel-art flute in its cool steel glass, the woven baskets (wicker dust where they land) and the silver
//! cobras (they rise, sway, strike and bite, slither after the enemy in SNAKE DANCE, and are slapped flat by a
//! lance), PICCOLO's shivering ring and the glass cracks it leaves, the silver shards of its shatter, and the VS
//! card's key-cup panel.
//!
//! Every function is the Python one of the same name, with its operation order. Where the Python calls
//! `math.sin` or a numpy function the port calls `bf_core::dmath`; `np.hypot` is the plain root
//! ([`bf_picture::np::hypot`]); `np.dot` and `np.linalg.norm` of one vector are the plain sums (numpy's are
//! fused on the reference PC, so a value here can differ from Python's in its last bit: none of them decides a
//! pixel in the frames of `tests/flute_look.rs`). A cobra's `aim` is an `atan2` (`dmath`'s here, the C
//! runtime's in Python): of the 1,183 state values that test compares, ten aims differ from Python's by 1 or 2
//! units in the last place, and no pixel moves with them. The shards and `ko_fx` are proven from the left
//! corner only (in the fights of that test the FLUTE loses only from the left corner).

use std::any::Any;
use std::collections::BTreeMap;

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::rng::Generator;
use bf_core::roster::WeaponCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{H, W, bayer_full, ss};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::{Img, RgbF};
use bf_picture::lighting::{Lights, shade};
use bf_picture::look::{CanvasCtx, DefaultLook, FrameCtx, Fx, Look, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{AP, R_BALL as R, to_world};
use bf_picture::shadows::proj_point;
use bf_picture::title::layout;
use bf_picture::{np, pil};

const SILVER: V3 = [206.0, 214.0, 228.0];
const SILVER_HI: V3 = [250.0, 252.0, 255.0];
const SILVER_DK: V3 = [120.0, 128.0, 150.0];
const INK: V3 = [26.0, 20.0, 36.0];
const WICKER: V3 = [196.0, 150.0, 92.0];
const WICKER_DK: V3 = [120.0, 84.0, 46.0];
const WICKER_HI: V3 = [232.0, 196.0, 132.0];
const SCALE_DK: V3 = [96.0, 104.0, 128.0];
const HOOD_MARK: V3 = [40.0, 34.0, 62.0];
const BASKET_R: f64 = 0.34;
const BASKET_H: f64 = 0.30;
const SILV: V3 = [226.0, 234.0, 248.0];
const WICKER_DUST: [V3; 3] = [[196.0, 150.0, 92.0], [150.0, 112.0, 64.0], [226.0, 196.0, 140.0]];
/// `glass`: the cool, darker glass (a silver fighter in cream glass vanishes).
const GLASS: V3 = [150.0, 160.0, 196.0];
/// The roll of the FLUTE's view in the kit's roster, for a look that was never set up.
const VIEW_ROLL: f64 = 32.0;

// ---------------------------------------------------------------- small vector helpers (numpy's order)
fn add3(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub3(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn mul3(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn div3(a: V3, k: f64) -> V3 {
    [a[0] / k, a[1] / k, a[2] / k]
}

fn neg3(a: V3) -> V3 {
    [-a[0], -a[1], -a[2]]
}

/// `np.dot(a, b)` as the plain sum.
fn dot3(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// `np.linalg.norm(a)` as the root of the plain sum.
fn norm3(a: V3) -> f64 {
    dot3(a, a).sqrt()
}

/// `np.cross(a, b)`.
fn cross3(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// `Q.mean(0)`: the rows added in order, then divided by their number.
fn mean3(rows: &[V3]) -> V3 {
    let mut s = [0.0; 3];
    for r in rows {
        s = add3(s, *r);
    }
    div3(s, rows.len() as f64)
}

/// `np.clip(c, 0, 255)` of a colour.
fn clip_col(c: V3) -> V3 {
    c.map(|v| np::clip(v, 0.0, 255.0))
}

/// `np.linspace(0, 2 * math.pi, n + 1)[:-1]`: `n` angles round the circle.
fn circle_angles(n: usize) -> Vec<f64> {
    let step = (2.0 * dmath::PI - 0.0) / n as f64;
    (0..n).map(|i| i as f64 * step + 0.0).collect()
}

/// `int(round(x))` of a screen position, when it is a pixel of the frame.
fn pixel(s: (f64, f64)) -> Option<(usize, usize)> {
    let (xi, yi) = (py::round_half_even(s.0), py::round_half_even(s.1));
    // 0 <= xi < W and 0 <= yi < H (a position that is not a number is no pixel; Python would raise)
    (0.0 <= xi && xi < W as f64 && 0.0 <= yi && yi < H as f64).then_some((xi as usize, yi as usize))
}

// ---------------------------------------------------------------- the flute inside its ball
/// `piccolo_rd(rd, ang, t)`: the piccolo's shiver, a fine, fast ripple on the ring's radius.
pub fn piccolo_rd(rd: f64, ang: f64, t: f64) -> f64 {
    rd + 0.045 * dmath::sin(31.0 * ang + 90.0 * t) + 0.02 * dmath::sin(57.0 * ang - 140.0 * t)
}

/// `flute_sprite(r, yaw, roll)`: the RGBA pixel-art flute fitting a radius-`r` sprite, `2 r + 5` pixels square:
/// a silver tube (banded: highlight, body, shadow), the crown, the lip plate with its dark embouchure hole, a
/// row of key cups and an ink outline. `yaw` swings the flute a little (foreshortening), `roll` is its tilt in
/// the ball, both in degrees.
pub fn flute_sprite(r: u32, yaw: f64, roll: f64) -> Img<u8> {
    let n = 2 * r as usize + 5;
    let c = (n as f64 - 1.0) / 2.0;
    let r = f64::from(r);
    let radians = |x: f64| x * (dmath::PI / 180.0);
    let len = 1.95 * r * (0.88 + 0.12 * dmath::cos(radians(yaw) * 1.6));
    // T = max(3.0, round(r * 0.2))
    let th = py_max(3.0, py::round_half_even(r * 0.2));
    let a = radians(-roll);
    let d = [dmath::cos(a), dmath::sin(a)];
    let nr = [-d[1], d[0]];
    let key_c: Vec<(f64, f64)> = (0..7)
        .map(|j| (-len / 2.0 + len * (0.42 + 0.075 * f64::from(j)), -th * 0.25))
        .collect();
    let lip_u = -len / 2.0 + len * 0.13;
    let tone = [SILVER_DK, SILVER, SILVER_HI];
    let bytes = |col: V3| col.map(np::f64_to_u8);
    let (key_col, lip_col, crown_col) = (bytes(mul3(SILVER_HI, 0.96)), bytes(SILVER_HI), bytes(mul3(SILVER, 0.9)));
    let (pad_col, ink) = (bytes(mul3(SILVER_DK, 0.8)), bytes(INK));
    let mut img: Img<u8> = Img::new(n, n, 4);
    let mut shape = vec![false; n * n];
    for y in 0..n {
        for x in 0..n {
            let (px, py_) = (x as f64 - c, y as f64 - c);
            let u = px * d[0] + py_ * d[1];
            let v = px * nr[0] + py_ * nr[1];
            let body = u.abs() <= len / 2.0 && v.abs() <= th / 2.0;
            let crown = u >= -len / 2.0 - 1.2 && u < -len / 2.0 + 1.5 && v.abs() <= th / 2.0 + 0.7;
            let near = |uk: f64, vk: f64, rad: f64| (u - uk) * (u - uk) + (v - vk) * (v - vk) <= rad * rad;
            let keys = key_c.iter().any(|&(uk, vk)| near(uk, vk, th * 0.52));
            let lip_v = v + th * 0.3;
            let oval = |ru: f64, rv: f64| {
                let (a, b) = ((u - lip_u) / ru, lip_v / rv);
                a * a + b * b <= 1.0
            };
            let lip = oval(2.0, 1.4);
            let inside = body || crown || keys || lip;
            // banded silver by v (light from the top of the ball)
            let band = if v < -th * 0.22 {
                2
            } else if v < th * 0.2 {
                1
            } else {
                0
            };
            let mut col = bytes(tone[band]);
            if keys {
                col = key_col;
            }
            if lip {
                col = lip_col;
            }
            if crown {
                col = crown_col;
            }
            // a dark pad ring centre in each key cup
            if keys && key_c.iter().any(|&(uk, vk)| near(uk, vk, th * 0.2)) {
                col = pad_col;
            }
            if oval(0.9, 0.7) {
                col = ink;
            }
            let px = img.px_mut(x, y);
            px[..3].copy_from_slice(&col);
            px[3] = if inside { 255 } else { 0 };
            shape[y * n + x] = inside;
        }
    }
    // ink outline: one pixel around the shape (np.roll: the sprite's edges wrap)
    for y in 0..n {
        for x in 0..n {
            if shape[y * n + x] {
                continue;
            }
            let around = [n - 1, 0, 1]
                .iter()
                .any(|dy| [n - 1, 0, 1].iter().any(|dx| shape[(y + dy) % n * n + (x + dx) % n]));
            if around {
                let px = img.px_mut(x, y);
                px[..3].copy_from_slice(&ink);
                px[3] = 255;
            }
        }
    }
    img
}

/// `flute_keys(base)`: the VS card's panel: rows of key cups along the divider (tone holes), symmetric about it
/// so it reads the same on either side; a steel silver. `base` is the panel's gradient, changed in place.
pub fn flute_keys(base: &mut RgbF) {
    let dd = layout::dd();
    let side = layout::side();
    for y in 0..base.h {
        for x in 0..base.w {
            // u = (TT.XX * TT.DD[0] + TT.YY * TT.DD[1]) / 22.0;  v = TT.SIDE / 30.0
            let u = (x as f64 * dd[0] + y as f64 * dd[1]) / 22.0;
            let v = side.px(x, y)[0] / 30.0;
            let (cu, cv) = (py::fmod_py(u, 1.0) - 0.5, py::fmod_py(v, 1.0) - 0.5);
            let d = np::hypot(cu * 22.0, cv * 30.0);
            let (cup, hole) = (d < 6.5, d < 3.0);
            for (p, tint) in base.px_mut(x, y).iter_mut().zip([0.70, 0.72, 0.80]) {
                // base_ = base_ * np.array([0.70, 0.72, 0.80])
                let mut c = *p * tint;
                // np.where(cup, base_ * 1.07 + 10, base_ * 0.95), then np.where(hole, base_ * 0.62, base_)
                c = if cup { c * 1.07 + 10.0 } else { c * 0.95 };
                *p = if hole { c * 0.62 } else { c };
            }
        }
    }
}

// ---------------------------------------------------------------- SNAKE CHARMER: baskets + cobras
/// `draw_basket(fr, cam, L, shade, cx, cz, k)`: a woven basket: banded wicker sides (alternating weave rows), a
/// lighter rim and the dark opening on top; `k` (0 to 1) scales it in and out (the drop-in pop, the pick-up).
fn draw_basket(fr: &mut Frame, cam: &Cam, l: &Lights, cx: f64, cz: f64, k: f64) {
    if k <= 0.02 {
        return;
    }
    let r = BASKET_R * k;
    let h = BASKET_H * k;
    let n = 16;
    // a = np.linspace(0, 2 * math.pi, n + 1): the last value is the stop itself
    let mut a = circle_angles(n);
    a.push(2.0 * dmath::PI);
    let rows = 3;
    let origin = to_world(0.0, 0.0, 0.0);
    for j in 0..rows {
        let (jf, rf) = (j as f64, rows as f64);
        let (y0, y1) = (h * jf / rf, h * (jf + 1.0) / rf);
        let (r0, r1) = (r * (0.86 + 0.14 * jf / rf), r * (0.86 + 0.14 * (jf + 1.0) / rf));
        for i in 0..n {
            let (a0, a1) = (a[i], a[i + 1]);
            let q = [
                to_world(cx + r0 * dmath::cos(a0), cz + r0 * dmath::sin(a0), y0),
                to_world(cx + r0 * dmath::cos(a1), cz + r0 * dmath::sin(a1), y0),
                to_world(cx + r1 * dmath::cos(a1), cz + r1 * dmath::sin(a1), y1),
                to_world(cx + r1 * dmath::cos(a0), cz + r1 * dmath::sin(a0), y1),
            ];
            let mid = (a0 + a1) / 2.0;
            let nrm = sub3(to_world(dmath::cos(mid), dmath::sin(mid), 0.0), origin);
            let ctr = mean3(&q);
            if dot3(nrm, sub3(cam.pos, ctr)) <= 0.0 {
                continue;
            }
            let base = if (i + j) % 2 == 0 {
                WICKER
            } else {
                mul3(WICKER_DK, 1.15)
            };
            let c = shade(l, &[nrm], &[ctr], &[base], None, 0.3, 0.0, 5.0)[0];
            fr.poly(cam, &q, c, true);
        }
    }
    let ring = |rad: f64, y: f64| -> Vec<V3> {
        a[..n]
            .iter()
            .map(|&u| to_world(cx + rad * dmath::cos(u), cz + rad * dmath::sin(u), y))
            .collect()
    };
    fr.poly(cam, &ring(r, h), mul3(WICKER_HI, 0.9), true);
    fr.poly(cam, &ring(r * 0.78, h + 0.002), [34.0, 24.0, 18.0], true);
}

/// A cobra's strike: `(t0, t_hit, t_end, aim, len)`.
pub type Strike = (f64, f64, f64, f64, f64);

/// What `cobra_pose` returns.
struct Pose {
    /// The body as world points, tail to head.
    pts: Vec<V3>,
    /// The head.
    head: V3,
    /// Its facing, a ring-frame unit vector.
    face: [f64; 2],
    /// How far it has risen, 0 to 1.
    e: f64,
}

/// `cobra_pose(home, cp, t, t_drop, rise_t, aim, strike, dancing_k, beat, seed)`: the cobra's body. It rises
/// out of the basket over `rise_t`, sways (a slow figure-eight, 2 beats) and lunges on a strike; away from the
/// basket (SNAKE DANCE) its body trails on the canvas.
#[allow(clippy::too_many_arguments)] // the Python signature
fn cobra_pose(
    home: [f64; 2],
    cp: [f64; 2],
    t: f64,
    t_drop: f64,
    rise_t: f64,
    aim: f64,
    strike: Option<Strike>,
    dancing_k: f64,
    beat: f64,
    seed: i64,
) -> Pose {
    let u_rise = py_min(1.0, py_max(0.0, (t - t_drop) / py_max(1e-6, rise_t)));
    let e = u_rise * u_rise * (3.0 - 2.0 * u_rise);
    let mut d_aim = [dmath::cos(aim), dmath::sin(aim)];
    let side = [-d_aim[1], d_aim[0]];
    let ph = 2.0 * dmath::PI * (t / (2.0 * beat)) + seed as f64 * 1.7;
    let sway = 0.15 * dmath::sin(ph) * e;
    let bob = 0.05 * dmath::sin(2.0 * ph) * e;
    let away = np::hypot(cp[0] - home[0], cp[1] - home[1]) > 0.05;
    let base = cp;
    let y_base = if away { 0.04 } else { BASKET_H * 0.9 };
    // head_rest = base_ + side * sway + d_aim * 0.16 * e
    let head_rest = [
        base[0] + side[0] * sway + d_aim[0] * 0.16 * e,
        base[1] + side[1] * sway + d_aim[1] * 0.16 * e,
    ];
    let hy = y_base + (0.25 + 0.85 * e) * (1.0 - 0.25 * dancing_k) + bob;
    let mut head = [head_rest[0], head_rest[1], hy];
    if let Some((t0, t_hit, t_end, s_aim, s_len)) = strike {
        let d_s = [dmath::cos(s_aim), dmath::sin(s_aim)];
        let k = if t < t_hit {
            let k = (t - t0) / py_max(1e-6, t_hit - t0);
            1.0 - (1.0 - k) * (1.0 - k)
        } else {
            py_max(0.0, 1.0 - (t - t_hit) / py_max(1e-6, t_end - t_hit))
        };
        let tgt = [base[0] + d_s[0] * s_len, base[1] + d_s[1] * s_len, R * 0.85];
        head = [
            head[0] + (tgt[0] - head[0]) * k,
            head[1] + (tgt[1] - head[1]) * k,
            head[2] + (tgt[2] - head[2]) * k,
        ];
        if k > 0.2 {
            d_aim = d_s;
        }
    }
    // body: from the basket mouth (or the canvas trail) up to the head, an S-curve
    let mut pts = Vec::with_capacity(17);
    if away {
        // slithering: a wavy trail behind on the canvas
        let back = [home[0] - cp[0], home[1] - cp[1]];
        let bl = np::hypot(back[0], back[1]);
        let b_u = if bl > 1e-6 {
            [back[0] / bl, back[1] / bl]
        } else {
            [-d_aim[0], -d_aim[1]]
        };
        let b_n = [-b_u[1], b_u[0]];
        for j in (1..=7).rev() {
            let s = f64::from(j) / 7.0 * py_min(0.9, bl + 0.2);
            let wave = dmath::sin(6.0 * s - 9.0 * t);
            let q = [
                cp[0] + b_u[0] * s + b_n[0] * 0.08 * wave,
                cp[1] + b_u[1] * s + b_n[1] * 0.08 * wave,
            ];
            pts.push(to_world(q[0], q[1], 0.04));
        }
    }
    let n = 9;
    for j in 0..=n {
        let s = f64::from(j) / f64::from(n);
        let wave = dmath::sin(dmath::PI * s * 1.5 + ph);
        let q = [
            base[0] + (head[0] - base[0]) * s + side[0] * 0.07 * wave * e,
            base[1] + (head[1] - base[1]) * s + side[1] * 0.07 * wave * e,
        ];
        let yq = y_base + (head[2] - y_base) * dmath::pow(s, 0.8);
        pts.push(to_world(q[0], q[1], yq));
    }
    Pose {
        pts,
        head: to_world(head[0], head[1], head[2]),
        face: d_aim,
        e,
    }
}

/// `draw_cobra(fr, cam, L, shade, pts, head_w, face, rise_e, col, glow)`: a silver cobra with an ink outline: a
/// thick scaled body (belly segments darker), the flared hood (with the dark spectacle mark) facing its target,
/// a wedge head and two eye pixels (red while it strikes).
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_cobra(
    fr: &mut Frame,
    cam: &Cam,
    l: &Lights,
    pts: &[V3],
    head_w: V3,
    face: [f64; 2],
    rise_e: f64,
    col: V3,
    glow: f64,
) {
    if rise_e <= 0.02 || pts.len() < 2 {
        return;
    }
    let f = cam.f / py_max(1e-3, np::mean(&cam.depth(pts)));
    // w_body = max(3, int(round(0.16 * f * (0.5 + 0.5 * rise_e))))
    let w_body = np::f64_to_i64(py::round_half_even(0.16 * f * (0.5 + 0.5 * rise_e))).max(3);
    // body_col = np.clip(SILVER * 0.86 + np.array([150, 190, 255]) * 0.08 + np.array(col) * 0.06, 0, 255)
    let tint = [150.0, 190.0, 255.0];
    let body_col: V3 = std::array::from_fn(|c| np::clip(SILVER[c] * 0.86 + tint[c] * 0.08 + col[c] * 0.06, 0.0, 255.0));
    let nseg = pts.len() - 1;
    // widths = [max(2, int(round(w_body * (0.55 + 0.45 * j / nseg)))) for j in range(nseg)]
    let widths: Vec<u32> = (0..nseg)
        .map(|j| {
            let w = py::round_half_even(w_body as f64 * (0.55 + 0.45 * j as f64 / nseg as f64));
            np::f64_to_i64(w).clamp(2, i64::from(u32::MAX) - 2) as u32
        })
        .collect();
    // ink outline pass
    for j in 0..nseg {
        fr.line(cam, &pts[j..j + 2], INK, widths[j] + 2);
    }
    for j in 0..nseg {
        let q = mean3(&pts[j..j + 2]);
        let base = if j % 2 == 0 { body_col } else { mul3(SCALE_DK, 1.45) };
        let c = shade(l, &[[0.0, 1.0, 0.0]], &[q], &[base], None, 0.5, 0.0, 5.0)[0];
        let lit = clip_col(c.map(|v| v * (1.0 + 0.4 * glow) + 25.0 * glow));
        fr.line(cam, &pts[j..j + 2], lit, widths[j]);
    }
    // hood: a flat oval behind the head, perpendicular to the facing direction
    let fa = sub3(to_world(face[0], face[1], 0.0), to_world(0.0, 0.0, 0.0));
    let fa = div3(fa, norm3(fa) + 1e-9);
    let up_ = [0.0, 1.0, 0.0];
    let mut to_cam = sub3(cam.pos, head_w);
    to_cam[1] = 0.0;
    let to_cam = div3(to_cam, norm3(to_cam) + 1e-9);
    // the hood turns partly to camera so it always reads
    let hn = sub3(mul3(fa, 0.45), mul3(to_cam, 0.55));
    let hn = div3(hn, norm3(hn) + 1e-9);
    let side = cross3(up_, hn);
    let side = div3(side, norm3(side) + 1e-9);
    let hc = sub3(sub3(head_w, mul3(fa, 0.05)), mul3(up_, 0.09));
    let a = circle_angles(12);
    let (hw, hh) = (0.24 * rise_e, 0.19 * rise_e);
    // hc[None] + ku * np.cos(a)[:, None] * side[None] + kv * np.sin(a)[:, None] * up[None]
    let oval = |centre: V3, ku: f64, kv: f64| -> Vec<V3> {
        a.iter()
            .map(|&u| {
                add3(
                    add3(centre, mul3(side, ku * dmath::cos(u))),
                    mul3(up_, kv * dmath::sin(u)),
                )
            })
            .collect()
    };
    let shifted = |poly: Vec<V3>, d: V3| -> Vec<V3> { poly.into_iter().map(|q| add3(q, d)).collect() };
    let to_eye = sub3(cam.pos, hc);
    let back_seen = dot3(neg3(fa), to_eye) > 0.0;
    // hood_o - hn * 0.003
    let outline: Vec<V3> = oval(hc, hw + 0.035, hh + 0.035)
        .into_iter()
        .map(|q| sub3(q, mul3(hn, 0.003)))
        .collect();
    fr.poly(cam, &outline, INK, true);
    let facing = if dot3(neg3(hn), to_eye) > 0.0 { neg3(hn) } else { hn };
    let c = shade(l, &[facing], &[hc], &[body_col], None, 0.6, 0.0, 5.0)[0];
    fr.poly(
        cam,
        &oval(hc, hw, hh),
        clip_col(c.map(|v| v * (1.0 + 0.4 * glow) + 20.0 * glow)),
        true,
    );
    let lifted = mul3(hn, 0.005);
    if back_seen {
        // the hood's back: the spectacle mark
        for sg in [-1.0, 1.0] {
            // hc[None] + side[None] * 0.07 * sg * rise_e + ...
            let centre = add3(hc, mul3(mul3(mul3(side, 0.07), sg), rise_e));
            fr.poly(
                cam,
                &shifted(oval(centre, 0.04 * rise_e, 0.035 * rise_e), lifted),
                HOOD_MARK,
                true,
            );
        }
    } else {
        // the front: pale belly scales down the hood
        fr.poly(
            cam,
            &shifted(oval(hc, 0.06 * rise_e, 0.13 * rise_e), lifted),
            clip_col(mul3(SILVER_HI, 0.92)),
            true,
        );
    }
    let head = [
        sub3(add3(head_w, mul3(side, 0.07)), mul3(up_, 0.01)),
        add3(head_w, mul3(fa, 0.17)),
        sub3(sub3(head_w, mul3(side, 0.07)), mul3(up_, 0.01)),
        add3(sub3(head_w, mul3(fa, 0.03)), mul3(up_, 0.04)),
    ];
    // head * 1.0 + (head - head.mean(0)) * 0.25
    let centre = mean3(&head);
    let grown: Vec<V3> = head
        .iter()
        .map(|&q| add3(mul3(q, 1.0), mul3(sub3(q, centre), 0.25)))
        .collect();
    fr.poly(cam, &grown, INK, true);
    let c = shade(l, &[up_], &[head_w], &[mul3(SILVER_HI, 0.95)], None, 0.6, 0.0, 5.0)[0];
    fr.poly(cam, &head, c, true);
    for sgn in [-1.0, 1.0] {
        // head_w + fa * 0.05 + side * 0.04 * sgn + up * 0.03
        let eye = add3(
            add3(add3(head_w, mul3(fa, 0.05)), mul3(mul3(side, 0.04), sgn)),
            mul3(up_, 0.03),
        );
        if let Some((xi, yi)) = pixel(cam.wp(&[eye])[0]) {
            let ink: [u8; 3] = if glow > 0.3 { [255, 70, 110] } else { [120, 255, 220] };
            pil::point(&mut fr.pil, &[(xi as f64, yi as f64)], &ink);
        }
    }
}

// ---------------------------------------------------------------- the look
/// A basket of the run: `dict(t_drop, home, t_gone)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Basket {
    /// When it was dropped.
    pub t_drop: f64,
    /// Where it stands, in the ring frame.
    pub home: [f64; 2],
    /// When the flute took it back (99 s: never).
    pub t_gone: f64,
}

/// A basket on screen with its cobra: one entry of `cobras_at`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cobra {
    /// The basket's centre.
    pub home: [f64; 2],
    /// The basket's size, 0 to 1.
    pub bk: f64,
    /// Where the cobra is (its basket, or on the canvas in SNAKE DANCE).
    pub cp: [f64; 2],
    /// The time its rise is counted from.
    pub t_drop: f64,
    /// The strike it is in, if any.
    pub strike: Option<Strike>,
    /// The angle it faces.
    pub aim: f64,
    /// Its basket's id (100 and 101: the champion's two solo baskets).
    pub bid: i64,
}

/// A swat of the run: when, the lance's owner and where.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Swat {
    t: f64,
    att: usize,
    x: f64,
    z: f64,
}

/// A painter item of the look.
enum Item {
    Basket(Cobra),
    Cobra(Cobra),
}

/// The FLUTE's look: `class Look(base.Look)` of `kit/fighters/flute.py`.
#[derive(Clone, Debug)]
pub struct FluteLook {
    /// Its side.
    k: usize,
    /// `cfg['view'][2]`: the roll of its instrument in the ball.
    roll: f64,
    /// `cfg['weapon']['rise']`: the beats a cobra takes to rise.
    rise: f64,
    /// `cfg['abilities']['low']`: PICCOLO's name and the speed of its ring.
    low: (String, f64),
    /// `T.BEAT`.
    beat: f64,
    /// `STRIKES`: basket id -> its strikes.
    strikes: BTreeMap<i64, Vec<Strike>>,
    /// `BASKETS`, in the order they were dropped.
    baskets: Vec<(i64, Basket)>,
    /// `DANCES`: `(start, end)` of each SNAKE DANCE.
    dances: Vec<(f64, f64)>,
    /// `SWAT_DOWN`: how long a swatted cobra is down.
    swat_down: f64,
    /// `SWAT_EV`.
    swat_ev: Vec<Swat>,
    /// `SWATS`: basket id -> the times it was swatted.
    swats: BTreeMap<i64, Vec<f64>>,
}

impl Default for FluteLook {
    fn default() -> FluteLook {
        FluteLook::new()
    }
}

/// A number of a recording's value.
fn value_num(v: &Value) -> Option<f64> {
    match v {
        Value::Float(x) => Some(*x),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

/// `e[i]` as a number, when the event has one there.
fn num(e: &Event, i: usize) -> Option<f64> {
    value_num(e.fields.get(i.checked_sub(2)?)?)
}

/// `e[i]` as a whole number, when the event has one there.
fn int(e: &Event, i: usize) -> Option<i64> {
    match e.fields.get(i.checked_sub(2)?)? {
        Value::Int(n) => Some(*n),
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

/// A fighter's side from a whole number of an event.
fn side_of(v: i64) -> Option<usize> {
    usize::try_from(v).ok().filter(|&k| k < 2)
}

/// `n=...` alone as the options of an `emit`.
fn count(n: i64) -> Emit {
    Emit {
        n: Some(n),
        ..Emit::default()
    }
}

impl FluteLook {
    /// The look before a picture sets it up: its constants and its title-card hooks work, and it has no
    /// baskets.
    pub fn new() -> FluteLook {
        FluteLook {
            k: 0,
            roll: VIEW_ROLL,
            rise: 1.0,
            low: (String::new(), 1.0),
            beat: 0.5,
            strikes: BTreeMap::new(),
            baskets: Vec::new(),
            dances: Vec::new(),
            swat_down: 1.0,
            swat_ev: Vec::new(),
            swats: BTreeMap::new(),
        }
    }

    /// `BASKETS`: `(id, basket)` in the order the baskets were dropped.
    pub fn baskets(&self) -> &[(i64, Basket)] {
        &self.baskets
    }

    /// `STRIKES`: basket id -> `[(t0, t_hit, t_end, aim, len)]`.
    pub fn strikes(&self) -> &BTreeMap<i64, Vec<Strike>> {
        &self.strikes
    }

    /// `DANCES`: `(start, end)` of each SNAKE DANCE.
    pub fn dances(&self) -> &[(f64, f64)] {
        &self.dances
    }

    /// `SWATS`: basket id -> the times a lance swatted its cobra.
    pub fn swats(&self) -> &BTreeMap<i64, Vec<f64>> {
        &self.swats
    }

    /// `SWAT_DOWN`: the two beats a swatted cobra cannot strike.
    pub fn swat_down(&self) -> f64 {
        self.swat_down
    }

    /// `strike_at(bid, t)`: the strike basket `bid`'s cobra is in at `t`.
    pub fn strike_at(&self, bid: i64, t: f64) -> Option<Strike> {
        self.strikes.get(&bid)?.iter().copied().find(|q| q.0 <= t && t < q.2)
    }

    /// `swat_level(bid, t)`: 1 is up; after a swat the cobra is slapped flat in about 0.07 s (0.12 of its
    /// height), lies there for a beat, then curls back up over the second beat.
    pub fn swat_level(&self, bid: i64, t: f64) -> f64 {
        let mut lv = 1.0;
        for &ts in self.swats.get(&bid).map_or(&[][..], Vec::as_slice) {
            let u = t - ts;
            if 0.0 <= u && u < self.swat_down {
                let down = 0.12 + 0.88 * (1.0 - py_min(1.0, u / 0.07));
                let up = ss(self.swat_down * 0.5, self.swat_down, u);
                lv = py_min(
                    lv,
                    if u >= 0.07 {
                        py_min(down, 0.12 + 0.88 * up)
                    } else {
                        down
                    },
                );
            }
        }
        lv
    }

    /// `swat_lean(bid, t)`: `(time since the swat, strength 0 to 1)` of the most recent swat on this cobra,
    /// for the knocked-back lean.
    pub fn swat_lean(&self, bid: i64, t: f64) -> Option<(f64, f64)> {
        self.swats
            .get(&bid)?
            .iter()
            .rev()
            .find(|&&ts| 0.0 <= t - ts && t - ts < self.swat_down)
            .map(|&ts| (t - ts, 1.0 - ss(self.swat_down * 0.4, self.swat_down, t - ts)))
    }

    /// `basket_k(b, t)`: a basket pops in when dropped (a little bounce) and shrinks away when the flute takes
    /// it back.
    pub fn basket_k(b: &Basket, t: f64) -> f64 {
        let u = t - b.t_drop;
        if u < 0.0 {
            return 0.0;
        }
        let k = if u < 0.3 {
            py_min(1.0, u / 0.12) * (1.0 + 0.18 * dmath::sin(py_min(1.0, u / 0.3) * dmath::PI))
        } else {
            1.0
        };
        k * (1.0 - ss(b.t_gone, b.t_gone + 0.2, t))
    }

    /// `dancing_k(t)`: how far into SNAKE DANCE the cobras are, 0 to 1.
    pub fn dancing_k(&self, t: f64) -> f64 {
        // max([ss(a, a + 0.2, t) * (1 - ss(b, b + 0.3, t)) for a, b in self.DANCES] + [0.0])
        let mut best: Option<f64> = None;
        for v in self
            .dances
            .iter()
            .map(|&(a, b)| ss(a, a + 0.2, t) * (1.0 - ss(b, b + 0.3, t)))
            .chain([0.0])
        {
            best = Some(best.map_or(v, |m| py_max(m, v)));
        }
        best.unwrap_or(0.0)
    }

    /// `cobras_at(t, fs)`: every basket on screen with its cobra.
    pub fn cobras_at(&self, p: &PictureCtx, t: f64, fs: &[BallState; 2]) -> Vec<Cobra> {
        let k = self.k;
        let g = &p.show.grid;
        let mut out = Vec::new();
        let enemy = fs[1 - k].p;
        if g.t_fight <= t && t < p.t_hs + 0.6 {
            let tk = py_min(t, p.t_ko);
            let i = p.sim_i(tk);
            // cob = {c[0]: c for c in P.RUN['COB'][i][k]}: id -> where the cobra is
            let mut cob: BTreeMap<i64, [f64; 2]> = BTreeMap::new();
            if let Some(Value::List(sides)) = p.run.nested("COB", i)
                && let Some(Value::List(mine)) = sides.get(k)
            {
                for c in mine {
                    if let Value::List(c) = c
                        && let (Some(Value::Int(id)), Some(x), Some(z)) = (c.first(), c.get(3), c.get(4))
                        && let (Some(x), Some(z)) = (value_num(x), value_num(z))
                    {
                        cob.insert(*id, [x, z]);
                    }
                }
            }
            let fade = 1.0 - ss(p.t_hs, p.t_hs + 0.6, t);
            for (bid, b) in &self.baskets {
                let bk = Self::basket_k(b, tk) * fade;
                if bk <= 0.01 {
                    continue;
                }
                let cp = cob.get(bid).copied().unwrap_or(b.home);
                let mut strike = if t < p.t_ko { self.strike_at(*bid, tk) } else { None };
                // the strike the lance swatted: no lunge after it
                if let (Some(sw), Some(q)) = (self.swat_lean(*bid, tk), strike)
                    && q.0 <= t - sw.0 + 1e-6
                {
                    strike = None;
                }
                let aim = dmath::atan2(enemy[1] - cp[1], enemy[0] - cp[0]);
                out.push(Cobra {
                    home: b.home,
                    bk,
                    cp,
                    t_drop: b.t_drop,
                    strike,
                    aim,
                    bid: *bid,
                });
            }
        }
        if p.win == k && p.t_solo - 0.8 <= t && t < g.t_glide {
            // its victory: two baskets beside it
            let ctr = fs[k].p;
            let k_in = ss(p.t_solo - 0.8, p.t_solo - 0.5, t) * (1.0 - ss(g.t_reassemble + 0.6, g.t_glide, t));
            for (j, off) in [[-0.85, 0.85], [0.85, -0.85]].into_iter().enumerate() {
                let hb = [ctr[0] + off[0], ctr[1] + off[1]];
                // they face out, to the crowd
                let aim = dmath::atan2(ctr[1] - hb[1], ctr[0] - hb[0]) + dmath::PI;
                out.push(Cobra {
                    home: hb,
                    bk: k_in,
                    cp: hb,
                    t_drop: p.t_solo - 0.5 + 0.12 * j as f64,
                    strike: None,
                    aim,
                    bid: 100 + j as i64,
                });
            }
        }
        out
    }

    /// `cobs(F)`: the cobras of the frame, worked out once.
    fn cobs<'a>(&self, p: &PictureCtx, f: &'a FrameCtx<'_>) -> &'a [Cobra] {
        f.memo[self.k]
            .get_or_init(|| Box::new(self.cobras_at(p, f.t, f.fs)) as Box<dyn Any + Send + Sync>)
            .downcast_ref::<Vec<Cobra>>()
            .map_or(&[], Vec::as_slice)
    }
}

impl Look for FluteLook {
    fn glass(&self) -> [f64; 3] {
        GLASS
    }

    /// Near-white: keeps its attack lights from whitening the canvas.
    fn flare_scale(&self) -> f64 {
        0.85
    }

    /// `['tube_s'] * 6 + ['key'] * 7`.
    fn shards(&self) -> Vec<String> {
        let mut kinds = vec!["tube_s".to_string(); 6];
        kinds.extend(vec!["key".to_string(); 7]);
        kinds
    }

    fn title_pattern(&self, base: &mut RgbF) {
        flute_keys(base);
    }

    fn title_closeup(&self, r: u32) -> Option<Img<u8>> {
        Some(flute_sprite(r, 0.0, self.roll))
    }

    fn sprite(&self, r: u32, yaw: f64, roll: f64) -> Option<Img<u8>> {
        Some(flute_sprite(r, yaw, roll))
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        let cfg = &p.cfg[k];
        self.k = k;
        self.roll = p.view[k].2;
        self.beat = p.show.grid.beat;
        self.rise = match &cfg.weapon {
            WeaponCfg::Striker(s) => s.rise,
            _ => 1.0,
        };
        self.low = (cfg.low.name.clone(), cfg.low.get("speed").unwrap_or(1.0));
        let me = |v: Option<i64>| v.and_then(side_of) == Some(k);
        self.strikes.clear();
        self.baskets.clear();
        self.swat_ev.clear();
        self.swats.clear();
        let ev = &p.rec.events;
        // basket id -> [(t0, t_hit, t_end, aim, len)]
        for e in ev.iter().filter(|e| e.kind == "strike" && me(int(e, 2))) {
            if let (Some(bid), Some(aim), Some(len), Some(dur)) = (int(e, 3), num(e, 4), num(e, 5), num(e, 6)) {
                self.strikes
                    .entry(bid)
                    .or_default()
                    .push((e.t, e.t + dur, e.t + dur * 2.6, aim, len));
            }
        }
        // id -> dict(t_drop, home, t_gone)
        for e in ev.iter().filter(|e| e.kind == "basket" && me(int(e, 2))) {
            let (Some(bid), Some(x), Some(z)) = (int(e, 3), num(e, 4), num(e, 5)) else {
                continue;
            };
            let basket = Basket {
                t_drop: e.t,
                home: [x, z],
                t_gone: 99.0,
            };
            match self.baskets.iter_mut().find(|(id, _)| *id == bid) {
                Some(slot) => slot.1 = basket,
                None => self.baskets.push((bid, basket)),
            }
            // the basket it replaced is taken back
            if let Some(old) = int(e, 6).filter(|&old| old >= 0)
                && let Some(slot) = self.baskets.iter_mut().find(|(id, _)| *id == old)
            {
                slot.1.t_gone = e.t;
            }
        }
        let dur_beats = cfg.unlock2.get("dur_beats").unwrap_or(4.0);
        self.dances = ev
            .iter()
            .filter(|e| e.kind == "ability" && text(e, 4) == "dance" && me(int(e, 2)))
            .map(|e| (e.t, e.t + dur_beats * self.beat))
            .collect();
        // the lance swats cobras: ('swat', lance owner, cobra owner, basket id, x, z); the cobra is down for 2
        // beats, curling back up over the second
        self.swat_down = 2.0 * self.beat;
        for e in ev.iter().filter(|e| e.kind == "swat" && me(int(e, 3))) {
            if let (Some(att), Some(bid), Some(x), Some(z)) =
                (int(e, 2).and_then(side_of), int(e, 4), num(e, 5), num(e, 6))
            {
                self.swat_ev.push(Swat { t: e.t, att, x, z });
                self.swats.entry(bid).or_default().push(e.t);
            }
        }
    }

    // ------------------------------------------------------------ events
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "hit" {
            match text(e, 7) {
                "wave" => return Some(2.0),
                "fang" => return Some(0.35),
                _ => {}
            }
        }
        DefaultLook.shake(p, e)
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match text(e, 7) {
            "wave" => Some((14.0, 0.35)),
            "fang" => Some((7.0, 0.2)),
            _ => None,
        }
    }

    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        // a basket lands: a puff of wicker dust
        for (bid, b) in &self.baskets {
            if b.t_drop < p.t_ko {
                fx.pa.emit(
                    b.t_drop + 0.05,
                    up(b.home[0], b.home[1], 0.05),
                    "dust",
                    &Emit {
                        n: Some(5),
                        col: Some(WICKER_DUST.to_vec()),
                        speed: Some((0.3, 0.8)),
                        ..Emit::default()
                    },
                );
                fx.mk.add(
                    b.t_drop + 0.05,
                    Mark {
                        size: 0.35,
                        col: [176.0, 140.0, 96.0],
                        life: 2.0,
                        seed: u64::try_from(*bid).unwrap_or(0),
                        strength: 0.5,
                        ..Mark::new(MarkKind::Dust, b.home[0], b.home[1])
                    },
                );
            }
        }
        // a lance swats one of its cobras flat
        for s in &self.swat_ev {
            if s.t > p.t_ko {
                break;
            }
            let pos = up(s.x, s.z, 0.6);
            fx.bu.add(s.t, pos, p.col[s.att], 0.55, None, false);
            fx.fl.add(s.t, pos, p.fl_col[s.att], 1.2, 0.45, 0.03, 0.2, 0.0);
            fx.pa.emit(s.t, pos, "scales", &count(6));
            fx.pa.emit(s.t, pos, "brass", &count(4));
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        let t = e.t;
        if e.kind == "hit" {
            let (Some(att), Some(x), Some(z)) = (int(e, 2).and_then(side_of), num(e, 5), num(e, 6)) else {
                return;
            };
            let pos = up(x, z, R);
            match text(e, 7) {
                // a cobra bites
                "fang" => {
                    fx.bu.add(t, pos, SILV, 0.55, None, false);
                    fx.fl.add(t, pos, p.fl_col[att], 1.2, 0.4, 0.03, 0.2, 0.0);
                    fx.pa.emit(t, pos, "scales", &count(5));
                    fx.mk.add(
                        t,
                        Mark {
                            size: 0.18,
                            angle: dmath::atan2(z, x),
                            n: 2,
                            col: [60.0, 54.0, 84.0],
                            life: 2.5,
                            // seed=int(t_ * 100)
                            seed: (t * 100.0) as u64,
                            strength: 0.7,
                            ..Mark::new(MarkKind::Claw, x, z)
                        },
                    );
                }
                // PICCOLO lands
                "wave" => {
                    fx.bu.add(t, pos, SILV, 1.4, None, true);
                    fx.pa.emit(t, pos, "glass", &count(12));
                }
                _ => {}
            }
        } else if e.kind == "ability" && text(e, 3) == self.low.0 {
            // PICCOLO: glass cracks
            let Some(k) = int(e, 2).and_then(side_of) else {
                return;
            };
            let at = p.run.p(p.sim_i(t), k);
            fx.fl
                .add(t, up(at[0], at[1], 1.0), mul3(SILV, 0.85), 3.0, 0.8, 0.03, 0.6, 0.0);
            // rng = np.random.default_rng(int(t_ * 10))
            let mut rng = Generator::new((t * 10.0) as u64);
            for j in 0..9 {
                let r = rng.uniform(1.0, 4.0);
                let a = rng.uniform(0.0, 2.0 * dmath::PI);
                let q = [at[0] + r * dmath::cos(a), at[1] + r * dmath::sin(a)];
                // if np.abs(q).max() > S.AP - 0.1: continue
                if np::maximum(q[0].abs(), q[1].abs()) > AP - 0.1 {
                    continue;
                }
                let tq = t + (r - R) / self.low.1;
                fx.mk.add(
                    tq,
                    Mark {
                        size: 0.45,
                        angle: a,
                        n: 6,
                        life: 5.0,
                        // seed=int(tq * 100) + j
                        seed: (tq * 100.0) as u64 + j,
                        ..Mark::new(MarkKind::Glass, q[0], q[1])
                    },
                );
                fx.pa.emit(tq, up(q[0], q[1], 0.05), "glass", &count(4));
            }
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        fx.pa.emit(
            p.t_ko,
            pos,
            "scales",
            &Emit {
                n: Some(30),
                spread: Some(180.0),
                speed: Some((0.8, 2.2)),
                ..Emit::default()
            },
        );
    }

    // ------------------------------------------------------------ canvas
    /// The baskets and the cobras' raised hoods.
    fn shadows_free(&self, p: &PictureCtx, f: &FrameCtx<'_>, spot: V3) -> Vec<Vec<(f64, f64)>> {
        let mut polys = Vec::new();
        let a = circle_angles(10);
        let disc = |c: V3, r: f64| -> Vec<V3> {
            a.iter()
                .map(|&u| [c[0] + r * dmath::cos(u), c[1], c[2] + r * dmath::sin(u)])
                .collect()
        };
        for c in self.cobs(p, f) {
            if c.bk < 0.05 {
                continue;
            }
            let w = to_world(c.home[0], c.home[1], BASKET_H * c.bk);
            polys.push(proj_point(&disc(w, BASKET_R * c.bk), spot, 0.93));
            let e = py_min(1.0, py_max(0.0, (f.t - c.t_drop) / (self.beat * 1.0)))
                * self.swat_level(c.bid, py_min(f.t, p.t_ko));
            if e > 0.1 {
                let hood = to_world(c.cp[0], c.cp[1], 0.2 + 0.6 * e);
                polys.push(proj_point(&disc(hood, 0.1), spot, 0.93));
            }
        }
        polys
    }

    /// PICCOLO: a thin ring with a fine, fast shiver.
    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind != "wave" {
            return;
        }
        let k = (1.0 - w.r / w.rmax) * 1.3;
        // self.P.COL[owner] / 255.0 * 1.4 + 0.35
        let tint = p.col[w.owner].map(|v| v / 255.0 * 1.4 + 0.35);
        let bayer = bayer_full();
        for (j, light) in ic.iter_mut().enumerate() {
            let (dx, dz) = (c.sxp[j] - w.cx, c.szp[j] - w.cz);
            let rd = piccolo_rd(np::hypot(dx, dz), dmath::atan2(dz, dx), c.t);
            let band = np::clip(1.0 - (rd - w.r).abs() / 0.18, 0.0, 1.0);
            let band = (band * 3.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 3.0;
            let amount = band * k * 0.9;
            for (v, tint) in light.iter_mut().zip(tint) {
                *v += amount * tint;
            }
        }
    }

    // ------------------------------------------------------------ baskets + cobras in the painter pass
    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let mut out: Vec<(f64, Payload)> = Vec::new();
        for c in self.cobs(p, f) {
            let dzb = f.cam.depth(&[to_world(c.home[0], c.home[1], 0.12)])[0];
            let basket: Payload = Box::new(Item::Basket(*c));
            out.push((dzb, basket));
            let dzc = py_min(dzb - 0.01, f.cam.depth(&[to_world(c.cp[0], c.cp[1], 0.5)])[0]);
            let cobra: Payload = Box::new(Item::Cobra(*c));
            out.push((dzc, cobra));
        }
        out
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let c = match payload.downcast_ref::<Item>() {
            Some(Item::Basket(c)) => {
                draw_basket(fr, f.cam, f.l, c.home[0], c.home[1], c.bk);
                return;
            }
            Some(Item::Cobra(c)) => c,
            None => return,
        };
        let (kf, t) = (self.k, f.t);
        let tk = py_min(t, p.t_ko);
        let dk = if t < p.t_ko { self.dancing_k(t) } else { 0.0 };
        let rise_t = self.beat * self.rise;
        let mut t_drop = c.t_drop;
        // a basket being taken back: the cobra sinks in
        if c.bk < 0.98 && t > t_drop + rise_t {
            t_drop = t - rise_t * py_max(0.0, c.bk);
        }
        let lv = if c.bid < 100 { self.swat_level(c.bid, tk) } else { 1.0 };
        // swatted flat, curling back up
        if lv < 1.0 && t > t_drop + rise_t {
            t_drop = t - rise_t * lv;
        }
        let sw = if c.bid < 100 { self.swat_lean(c.bid, tk) } else { None };
        let mut aim = c.aim;
        // knocked back: it faces away from the lance
        if let Some((_, strength)) = sw {
            aim += dmath::PI * 0.55 * strength;
        }
        let pose = cobra_pose(c.home, c.cp, t, t_drop, rise_t, aim, c.strike, dk, self.beat, c.bid);
        let hasted = f.in_fight && t < p.t_ko && p.run.f("HA", f.i, kf).unwrap_or(0.0) > 0.0;
        let mut gl = match c.strike {
            Some(q) if t < q.1 + 0.05 => 0.6,
            _ if hasted => 0.35,
            _ => 0.0,
        };
        // the slap: a flash of the lance's colour
        if let Some((since, _)) = sw
            && since < 0.12
        {
            gl = py_max(gl, 0.8 * (1.0 - since / 0.12));
        }
        if t >= p.t_solo - 0.8 {
            gl = py_max(gl, 0.6 * p.solo_env(t));
        }
        draw_cobra(
            fr,
            f.cam,
            f.l,
            &pose.pts,
            pose.head,
            pose.face,
            pose.e * py_min(1.0, c.bk),
            p.col[kf],
            gl,
        );
    }

    fn draw_shard(
        &self,
        _p: &PictureCtx,
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
        if kind == "tube_s" {
            // a straight silver body section
            let (lk, wk) = (0.40, 0.05);
            // np.array([sx_, sz_]) + d * Lk / 2 * a1 + nn * Wk / 2 * a2
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
            let col = clip_col(mul3(SILVER, 0.8 + 0.2 * dmath::cos(rot * 2.0)));
            layer_blend(arr, &[screen(cam, &quad)], col, 1.0, None);
            // np.array([sx_, sz_]) + d * Lk / 2 * a1 - nn * Wk * 0.25
            let ends: Vec<V3> = [-0.8, 0.8]
                .iter()
                .map(|&a1| {
                    to_world(
                        sx + d[0] * lk / 2.0 * a1 - nn[0] * wk * 0.25,
                        sz + d[1] * lk / 2.0 * a1 - nn[1] * wk * 0.25,
                        pos[1],
                    )
                })
                .collect();
            let hi = screen(cam, &ends);
            // im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8)); the highlight; back to float32
            let mut im: Img<u8> = Img::from_vec(
                arr.w,
                arr.h,
                arr.c,
                arr.data.iter().map(|&v| np::clip_u8_f32(v)).collect(),
            );
            pil::line(&mut im, &hi, &SILVER_HI.map(|v| v as u8), 1);
            for (v, &b) in arr.data.iter_mut().zip(&im.data) {
                *v = f32::from(b);
            }
        } else if kind == "key" {
            // a key cup (silver, dark pad)
            let rr = 0.07;
            let cup: Vec<V3> = circle_angles(8)
                .iter()
                .map(|&u| to_world(sx + rr * dmath::cos(u), sz + rr * dmath::sin(u), pos[1]))
                .collect();
            layer_blend(arr, &[screen(cam, &cup)], SILVER_HI, 1.0, None);
            if let Some((xi, yi)) = pixel(screen(cam, &[to_world(sx, sz, pos[1])])[0]) {
                // arr[yi, xi] = SILVER_DK * 0.7
                for (v, c) in arr.px_mut(xi, yi).iter_mut().zip(mul3(SILVER_DK, 0.7)) {
                    *v = c as f32;
                }
            }
        }
    }

    /// The piccolo ring.
    fn shimmer(&self, _p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, w: &Wave) {
        if w.kind == "wave" {
            img.shimmer_world(
                cam,
                to_world(w.cx, w.cz, 0.05),
                w.r,
                0.3,
                2.4,
                t,
                1.0 - w.r / w.rmax,
                w.owner as f64,
            );
        }
    }

    /// Baskets, raised hoods (about 1.3 high) and a strike's head out to its reach.
    fn focus_points(&self, p: &PictureCtx, t: f64, _st: &BallState) -> Vec<(V3, String)> {
        let fs = [p.fighter(0, t), p.fighter(1, t)];
        let mut out = Vec::new();
        for c in self.cobras_at(p, t, &fs) {
            out.push((to_world(c.home[0], c.home[1], 0.0), "basket".to_string()));
            out.push((to_world(c.cp[0], c.cp[1], 1.3), "cobra".to_string()));
            if let Some(q) = c.strike {
                out.push((
                    to_world(c.cp[0] + q.4 * dmath::cos(q.3), c.cp[1] + q.4 * dmath::sin(q.3), R),
                    "strike".to_string(),
                ));
            }
        }
        out
    }
}
