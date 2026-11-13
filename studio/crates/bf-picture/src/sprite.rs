//! Meshes and the sprite renderer: the low-poly instrument inside each glass ball.
//!
//! A port of `sprites.render_sprite` (`projects/which_instrument_wins/sprites.py`) with the rasteriser it calls,
//! `render_geo`, `tri` and `outline` of `series/instrument-history/piano_lowpoly.py`: flat shading in seven bands
//! with stepped highlights, a depth buffer, a group buffer and an ink outline, drawn into the kit's 270 x 480
//! canvas and cut out as an RGBA square of side `2 r + 7`. The numpy code works on whole arrays; here the same
//! arithmetic runs pixel by pixel, in the same order and with the same types (`float64` depth, `int8` groups,
//! `uint8` colour), so the same pixels come out.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use bf_core::{dmath, py};
use serde::Deserialize;

use crate::img::Img;

/// The kit's canvas (`piano_lowpoly.RW`, `RH`). The sprite is drawn in its top-left corner; triangles are clipped
/// to it and the outline pass wraps around its edges (`np.roll`).
const RW: usize = 270;
const RH: usize = 480;
/// `sprites.ELEV`: the degrees the camera looks down on the model.
const ELEV_DEG: f64 = 16.0;
/// The camera's distance from the model's centre (`dist` in `sprites._Cam`).
const DIST: f64 = 9.0;
/// `piano_lowpoly.LIGHT`: (0.45, 1.0, 0.6) divided by its length, which is exactly 1.25.
const LIGHT: [f64; 3] = [0.45 / 1.25, 1.0 / 1.25, 0.6 / 1.25];
/// `piano_lowpoly.INK`, the outline colour.
const INK: [f64; 3] = [14.0, 8.0, 24.0];
/// The group ids `sprites._geo` gives the faces: the ground group keeps 1, everything else is the instrument.
const GID_GROUND: i8 = 1;
const GID_INSTRUMENT: i8 = 2;

/// One face of a model, as `pp_bridge.model_faces` gives it and the model JSON stores it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Face {
    /// The vertices (x, y, z; y up), drawn as a fan from the first.
    #[serde(rename = "P")]
    pub p: Vec<[f64; 3]>,
    /// The unit normal.
    pub n: [f64; 3],
    /// The colour, 0..255; the renderer truncates each channel to a whole number.
    pub rgb: [f64; 3],
    /// Drawn from both sides.
    pub two: bool,
    /// The colour is used as it is, unshaded.
    pub emit: bool,
    /// The group id: 1 marks the ground group, anything else is the instrument.
    pub gid: i32,
    /// The specular strength (0: none).
    pub spec: f64,
}

/// A model: its faces in the order the renderer draws them (a later face wins only where it is strictly nearer).
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Mesh {
    pub faces: Vec<Face>,
}

impl Mesh {
    /// Reads a model file (`bf-content/data/models/<NAME>.json`, written by the export tool's `models`): the
    /// `faces` list, every float to the same bits Python wrote. The file's other fields are notes and are skipped.
    pub fn from_json(text: &str) -> Result<Mesh, serde_json::Error> {
        serde_json::from_str(text)
    }
}

type V3 = [f64; 3];

/// `a @ b` for two 3-vectors, and one element of a numpy matrix product (`P @ R.T`, `Rz @ Ry`: a row of the
/// left matrix against a column of the right).
///
/// numpy hands these products to OpenBLAS, whose kernels on the reference PC are fused: the first product is
/// rounded, the other two are added with one rounding each, `fma(a2, b2, fma(a1, b1, a0 * b0))`. Measured there on
/// random values, every element equal and no other order or the plain sum equal: `P @ R.T` for 2 to 64 rows,
/// `A @ B` for 3 x 3 matrices, `a @ b` and `np.linalg.norm(a)` for 3-vectors. The plain sum differs in the last
/// bit about once in ten, and that reaches pixels: two faces of a model that lie in one plane and overlap have
/// depths a last bit apart or equal, and the depth test `iz > zb` picks the colour (ORGAN, radius 16, yaw 0, row
/// 26, column 21). So the sprite's camera maths is fused exactly where numpy's is, through [`dmath::fma`]
/// (software, the same bits on every PC).
fn dot3(a: V3, b: V3) -> f64 {
    dmath::fma(a[2], b[2], dmath::fma(a[1], b[1], a[0] * b[0]))
}

/// One element of a numpy matrix times a vector (`R @ n`, `Q @ fwd`): the row `a` against the vector `b`.
///
/// OpenBLAS's matrix-vector kernel on the reference PC is fused in another order than [`dot3`]: the second
/// product is the rounded one, `fma(a2, b2, fma(a0, b0, a1 * b1))` (measured as for [`dot3`], for 2 to 64 rows).
fn dot3_mv(a: V3, b: V3) -> f64 {
    dmath::fma(a[2], b[2], dmath::fma(a[0], b[0], a[1] * b[1]))
}

/// `np.linalg.norm(x)` of one vector: the square root of the fused `x @ x`.
fn norm(x: V3) -> f64 {
    dot3(x, x).sqrt()
}

/// `np.cross(a, b)`.
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// `x / s`, element by element.
fn div3(x: V3, s: f64) -> V3 {
    [x[0] / s, x[1] / s, x[2] / s]
}

/// `math.radians`: CPython multiplies by the constant `pi / 180`.
fn radians(deg: f64) -> f64 {
    deg * (dmath::PI / 180.0)
}

/// `math.sin(math.radians(deg))` and `math.cos(math.radians(deg))` as the kit's camera gets them: `bf_core::dmath`
/// is Python's to the bit (ruling M3-R38; the table of 35 whole-degree values ruling M3-R31 kept here for the old
/// sine and cosine is now only the test's).
fn sin_cos_deg(deg: f64) -> (f64, f64) {
    let a = radians(deg);
    (dmath::sin(a), dmath::cos(a))
}

/// `sprites._Cam`: a narrow perspective looking at the origin from `DIST` away, `ELEV_DEG` above the horizon,
/// with the model turned by yaw (about y) and then roll (about z).
struct Cam {
    /// `Rz @ Ry`.
    r: [V3; 3],
    eye: V3,
    fwd: V3,
    right: V3,
    up: V3,
    /// The focal length in pixels: `radius_px * dist`.
    f: f64,
    cx: f64,
    cy: f64,
}

impl Cam {
    fn new(yaw_deg: f64, radius_px: u32, cx: f64, cy: f64, roll_deg: f64) -> Cam {
        let (s, c) = sin_cos_deg(yaw_deg);
        let ry = [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]];
        let (sb, cb) = sin_cos_deg(roll_deg);
        let rz = [[cb, -sb, 0.0], [sb, cb, 0.0], [0.0, 0.0, 1.0]];
        // Rz @ Ry: each element has one non-zero product at most, so it is that product whatever the order.
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, out) in row.iter_mut().enumerate() {
                *out = dot3(rz[i], [ry[0][j], ry[1][j], ry[2][j]]);
            }
        }
        let e = radians(ELEV_DEG);
        let eye = [0.0, dmath::sin(e) * DIST, dmath::cos(e) * DIST];
        let fwd = [-eye[0] / DIST, -eye[1] / DIST, -eye[2] / DIST];
        let right = cross(fwd, [0.0, 1.0, 0.0]);
        let right = div3(right, norm(right));
        let up = cross(right, fwd);
        Cam {
            r,
            eye,
            fwd,
            right,
            up,
            f: f64::from(radius_px) * DIST,
            cx,
            cy,
        }
    }
}

/// The three buffers `render_sprite` allocates: colour (`uint8`), depth as 1 / distance (`float64`, 0 = empty) and
/// group (`int8`, 0 = empty), each `RW` x `RH`.
struct Canvas {
    img: Vec<[u8; 3]>,
    zb: Vec<f64>,
    gb: Vec<i8>,
}

impl Canvas {
    fn new() -> Canvas {
        Canvas {
            img: vec![[0; 3]; RW * RH],
            zb: vec![0.0; RW * RH],
            gb: vec![0; RW * RH],
        }
    }
}

/// `piano_lowpoly.tri`: one triangle, with screen corners `s` and camera distances `d`, into the canvas. A pixel
/// is covered when its centre is inside (each barycentric weight at least -1e-6) and drawn when the triangle is
/// strictly nearer there than what the depth buffer holds.
fn tri(cv: &mut Canvas, s: [[f64; 2]; 3], d: V3, col: [u8; 3], gid: i8) {
    let min_x = s[0][0].min(s[1][0]).min(s[2][0]);
    let max_x = s[0][0].max(s[1][0]).max(s[2][0]);
    let min_y = s[0][1].min(s[1][1]).min(s[2][1]);
    let max_y = s[0][1].max(s[1][1]).max(s[2][1]);
    let x0 = (min_x.floor() as i64).max(0);
    let x1 = (max_x.ceil() as i64).min(RW as i64 - 1);
    let y0 = (min_y.floor() as i64).max(0);
    let y1 = (max_y.ceil() as i64).min(RH as i64 - 1);
    if x0 > x1 || y0 > y1 {
        return;
    }
    let [[ax, ay], [bx, by], [cx, cy]] = s;
    let area = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
    if area.abs() < 1e-9 {
        return;
    }
    for py in y0..=y1 {
        let y = py as f64 + 0.5;
        let row = py as usize * RW;
        for px in x0..=x1 {
            let x = px as f64 + 0.5;
            let w0 = ((cx - bx) * (y - by) - (cy - by) * (x - bx)) / area;
            let w1 = ((ax - cx) * (y - cy) - (ay - cy) * (x - cx)) / area;
            let w2 = 1.0 - w0 - w1;
            if !(w0 >= -1e-6 && w1 >= -1e-6 && w2 >= -1e-6) {
                continue;
            }
            let iz = w0 / d[0] + w1 / d[1] + w2 / d[2];
            let i = row + px as usize;
            if iz > cv.zb[i] {
                cv.zb[i] = iz;
                cv.img[i] = col;
                cv.gb[i] = gid;
            }
        }
    }
}

/// `piano_lowpoly.render_geo` for the faces of a mesh (a sprite's model has no lines), at scale 1.
fn render_geo(mesh: &Mesh, cam: &Cam, cv: &mut Canvas) {
    let mut w: Vec<V3> = Vec::new();
    let mut s: Vec<[f64; 2]> = Vec::new();
    let mut d: Vec<f64> = Vec::new();
    for face in &mesh.faces {
        let k = face.p.len();
        if k < 3 {
            continue; // nothing to fan out (the model files hold no such face)
        }
        // W = P @ R.T and nw = R @ n
        w.clear();
        w.extend(
            face.p
                .iter()
                .map(|&p| [dot3(p, cam.r[0]), dot3(p, cam.r[1]), dot3(p, cam.r[2])]),
        );
        let mut nw = [
            dot3_mv(cam.r[0], face.n),
            dot3_mv(cam.r[1], face.n),
            dot3_mv(cam.r[2], face.n),
        ];
        // v: from the face's centre (the mean of its corners, summed in order) to the eye
        let mut sum = w[0];
        for p in &w[1..] {
            sum = [sum[0] + p[0], sum[1] + p[1], sum[2] + p[2]];
        }
        let mean = div3(sum, k as f64);
        let v = [cam.eye[0] - mean[0], cam.eye[1] - mean[1], cam.eye[2] - mean[2]];
        let v = div3(v, norm(v));
        let dn = dot3(nw, v);
        if dn <= 0.0 {
            if !face.two {
                continue;
            }
            nw = [-nw[0], -nw[1], -nw[2]];
        }
        // S, D = cam.proj(W)
        s.clear();
        d.clear();
        for p in &w {
            let q = [p[0] - cam.eye[0], p[1] - cam.eye[1], p[2] - cam.eye[2]];
            let dist = dot3_mv(q, cam.fwd);
            s.push([
                cam.cx + cam.f * dot3_mv(q, cam.right) / dist,
                cam.cy - cam.f * dot3_mv(q, cam.up) / dist,
            ]);
            d.push(dist);
        }
        if d.iter().any(|&x| x <= 0.05) {
            continue;
        }
        // the colour: `tuple(int(c) for c in col)`, then shaded in bands unless the face emits
        let col = face.rgb.map(f64::trunc);
        let cc = if face.emit {
            col
        } else {
            let lit = dot3(nw, LIGHT);
            let i = 0.36 + 0.64 * if lit > 0.0 { lit } else { 0.0 };
            let i = (i * 7.0).floor() / 7.0 + 0.05;
            let mut cc = col.map(|c| c * i);
            if face.spec != 0.0 {
                let h = [LIGHT[0] + v[0], LIGHT[1] + v[1], LIGHT[2] + v[2]];
                let h = div3(h, norm(h));
                let nh = dot3(nw, h);
                let sp = dmath::pow(if nh > 0.0 { nh } else { 0.0 }, 40.0);
                if sp > 0.3 {
                    let add = face.spec * if sp > 0.7 { 110.0 } else { 50.0 };
                    cc = cc.map(|c| c + add);
                }
            }
            cc
        };
        // np.clip(cc, 0, 255).astype(np.uint8)
        let cc = cc.map(|c| c.clamp(0.0, 255.0) as u8);
        let gid = if face.gid != 1 { GID_INSTRUMENT } else { GID_GROUND };
        for j in 1..k - 1 {
            tri(cv, [s[0], s[j], s[j + 1]], [d[0], d[j], d[j + 1]], cc, gid);
        }
    }
}

/// `piano_lowpoly.outline`: inks the instrument's pixels that touch the background or the ground group, or that
/// stand well in front of a neighbour (the neighbour's depth below 0.9 of theirs), and the ground group's pixels
/// that touch the background. Neighbours are the four around a pixel, wrapping at the canvas edges as `np.roll`
/// does. An inked pixel becomes 0.3 of its colour plus 0.7 of `INK`.
fn outline(cv: &mut Canvas) {
    let mut ink = vec![false; RW * RH];
    for y in 0..RH {
        for x in 0..RW {
            let i = y * RW + x;
            let g = cv.gb[i];
            if g != GID_INSTRUMENT && g != GID_GROUND {
                continue;
            }
            let neighbours = [
                (y + RH - 1) % RH * RW + x,
                (y + 1) % RH * RW + x,
                y * RW + (x + RW - 1) % RW,
                y * RW + (x + 1) % RW,
            ];
            ink[i] = neighbours.iter().any(|&j| {
                if g == GID_INSTRUMENT {
                    cv.gb[j] <= 1 || cv.zb[j] < cv.zb[i] * 0.9
                } else {
                    cv.gb[j] == 0
                }
            });
        }
    }
    for (px, _) in cv.img.iter_mut().zip(&ink).filter(|&(_, &on)| on) {
        for (c, k) in px.iter_mut().zip(INK) {
            *c = (f64::from(*c) * 0.3 + k * 0.7) as u8;
        }
    }
}

/// The sprite of `mesh` at `radius_px`, turned by `yaw_deg` about the vertical and then `roll_deg` in the picture
/// plane: RGBA, side `2 * radius_px + 7`, the model's centre on the middle pixel. A pixel belongs to the sprite
/// where its alpha is 255; elsewhere all four channels are 0. The port of `sprites.render_sprite`.
///
/// Panics if the sprite would not fit the kit's 270 x 480 canvas (a radius above 131), where the Python cannot
/// make a square sprite either.
pub fn render_sprite(mesh: &Mesh, radius_px: u32, yaw_deg: f64, roll_deg: f64) -> Img<u8> {
    render_sprite_padded(mesh, radius_px, yaw_deg, roll_deg, radius_px)
}

/// The largest radius (and half crop) [`render_sprite`] can draw: `(270 - 7) / 2 = 131`.
pub const MAX_RADIUS: u32 = (RW as u32 - 7) / 2;

/// The model drawn as [`render_sprite`] draws it at `radius_px`, but cropped round a larger square: the centre at
/// `half + 3.5` and the side `2 * half + 7`. This is `_render(geo, r, yaw, roll, pad)` of
/// `kit/fighters/contrabass.py` (`padded_sprite`) and `bass_drum.py` with `half = int(round(r * pad))`: their
/// instruments reach past the ball's crop. `half == radius_px` is [`render_sprite`].
///
/// Panics if the crop would not fit the kit's 270 x 480 canvas (`half` above [`MAX_RADIUS`]).
pub fn render_sprite_padded(mesh: &Mesh, radius_px: u32, yaw_deg: f64, roll_deg: f64, half: u32) -> Img<u8> {
    assert!(
        half <= MAX_RADIUS,
        "a sprite of radius {half} does not fit the {RW} x {RH} canvas"
    );
    let c = f64::from(half + 3) + 0.5;
    let cam = Cam::new(yaw_deg, radius_px, c, c, roll_deg);
    draw_crop(mesh, &cam, half)
}

/// The model tipped toward the camera by `pitch_deg` about its x axis, as #31's `styles.render_sprite(name,
/// radius_px, yaw_deg, roll_deg, pitch_deg)` draws it (SYNTH's keys, TURNTABLES' platter; plan m4 Task 10): the
/// kit's camera with its turn replaced by `Rz @ Rx @ Ry` (roll, pitch, yaw), side `2 * radius_px + 7`. At pitch 0 it
/// is [`render_sprite`].
///
/// Panics as [`render_sprite`] does.
pub fn render_sprite_pitched(mesh: &Mesh, radius_px: u32, yaw_deg: f64, roll_deg: f64, pitch_deg: f64) -> Img<u8> {
    assert!(
        radius_px <= MAX_RADIUS,
        "a sprite of radius {radius_px} does not fit the {RW} x {RH} canvas"
    );
    let c = f64::from(radius_px + 3) + 0.5;
    let mut cam = Cam::new(yaw_deg, radius_px, c, c, roll_deg);
    cam.r = pitched_turn(yaw_deg, roll_deg, pitch_deg);
    draw_crop(mesh, &cam, radius_px)
}

/// `Rz @ Rx @ Ry` of #31's `styles.render_sprite` (`a, b, y = radians(pitch), radians(roll), radians(yaw)`), each
/// product as numpy's 3 x 3 matmul computes it on the reference PC: every element `fma(a2, b2, fma(a1, b1, a0 *
/// b0))`, measured on 3,040 angle sets (every element of `Rz @ Rx @ Ry` to the bit; the plain sum matched 1,173),
/// the fused products of rulings M3-R30 / R39.
fn pitched_turn(yaw_deg: f64, roll_deg: f64, pitch_deg: f64) -> [V3; 3] {
    let (sa, ca) = sin_cos_deg(pitch_deg);
    let (sb, cb) = sin_cos_deg(roll_deg);
    let (sy, cy) = sin_cos_deg(yaw_deg);
    let rx = [[1.0, 0.0, 0.0], [0.0, ca, -sa], [0.0, sa, ca]];
    let ry = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
    let rz = [[cb, -sb, 0.0], [sb, cb, 0.0], [0.0, 0.0, 1.0]];
    matmul3(matmul3(rz, rx), ry)
}

/// numpy's `A @ B` of two 3 x 3 `float64` matrices on the reference PC (its BLAS): `fma(a[i][2], b[2][j],
/// fma(a[i][1], b[1][j], a[i][0] * b[0][j]))`.
fn matmul3(a: [V3; 3], b: [V3; 3]) -> [V3; 3] {
    let mut out = [[0.0; 3]; 3];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, o) in row.iter_mut().enumerate() {
            *o = dmath::fma(a[i][2], b[2][j], dmath::fma(a[i][1], b[1][j], a[i][0] * b[0][j]));
        }
    }
    out
}

/// The model drawn with `cam` and cropped round the square of half side `half` (`img[:n, :n]`, `n = 2 half + 7`).
fn draw_crop(mesh: &Mesh, cam: &Cam, half: u32) -> Img<u8> {
    let mut cv = Canvas::new();
    render_geo(mesh, cam, &mut cv);
    outline(&mut cv);
    let n = 2 * half as usize + 7;
    let mut data = Vec::with_capacity(n * n * 4);
    for y in 0..n {
        for x in 0..n {
            let i = y * RW + x;
            data.extend_from_slice(&cv.img[i]);
            data.push(if cv.gb[i] > 0 { 255 } else { 0 });
        }
    }
    Img { w: n, h: n, c: 4, data }
}

/// What a cached sprite is known by: the fighter, the radius in pixels, the yaw as a whole number of 6-degree
/// steps and the roll.
///
/// `kit.picture.sprite` keys its cache by the yaw wrapped into 0..359 and by `int(roll)` but draws with the yaw
/// and the roll as they are, so there two different pictures can share a key and the first one asked for stays.
/// Here the key holds what the sprite is drawn from, the yaw step unwrapped (yaw -6 is step -1, not 59) and the
/// roll to the bit, so a key means one picture whoever asks first.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpriteKey {
    pub fighter: usize,
    pub radius: u32,
    pub yaw_step: i32,
    roll_bits: u64,
}

impl fmt::Debug for SpriteKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpriteKey")
            .field("fighter", &self.fighter)
            .field("radius", &self.radius)
            .field("yaw_step", &self.yaw_step)
            .field("roll_deg", &self.roll_deg())
            .finish()
    }
}

impl SpriteKey {
    /// The smallest and largest radius a ball's sprite is drawn at (`kit.picture.sprite` clips to these).
    pub const RADIUS_MIN: u32 = 12;
    pub const RADIUS_MAX: u32 = 30;
    /// The degrees in one yaw step.
    pub const YAW_STEP_DEG: i32 = 6;

    pub fn new(fighter: usize, radius: u32, yaw_step: i32, roll_deg: f64) -> SpriteKey {
        SpriteKey {
            fighter,
            radius,
            yaw_step,
            roll_bits: roll_deg.to_bits(),
        }
    }

    /// The key `kit.picture.sprite(k, radius_px, yaw_deg, roll)` draws from: the radius rounded (half to even) and
    /// clipped to 12..30, the yaw rounded (half to even) to a multiple of 6 degrees.
    pub fn for_ball(fighter: usize, radius_px: f64, yaw_deg: f64, roll_deg: f64) -> SpriteKey {
        let radius = py::round_half_even(radius_px).clamp(f64::from(Self::RADIUS_MIN), f64::from(Self::RADIUS_MAX));
        let yaw_step = py::round_half_even(yaw_deg / f64::from(Self::YAW_STEP_DEG));
        SpriteKey::new(fighter, radius as u32, yaw_step as i32, roll_deg)
    }

    /// The yaw the sprite is drawn at: `float(yk)`.
    pub fn yaw_deg(&self) -> f64 {
        f64::from(self.yaw_step) * f64::from(Self::YAW_STEP_DEG)
    }

    /// The roll the sprite is drawn at.
    pub fn roll_deg(&self) -> f64 {
        f64::from_bits(self.roll_bits)
    }
}

/// One key's place in the cache: empty until the first thread that asked has drawn the sprite.
type Slot = Arc<OnceLock<Arc<Img<u8>>>>;

/// The sprites drawn so far, by key. Shared by the render threads: a sprite is drawn once, by the first thread that
/// asks for it, while the others wait for that key only; and since a sprite is a pure function of its key, every
/// thread gets the same pixels whichever came first.
#[derive(Default)]
pub struct SpriteCache {
    sprites: Mutex<BTreeMap<SpriteKey, Slot>>,
}

impl SpriteCache {
    pub fn new() -> SpriteCache {
        SpriteCache::default()
    }

    /// The sprite for `key`, drawn with `draw` if no thread has asked for it yet. `draw` must depend on nothing
    /// but the key (the fighter's own mesh or its look's sprite function, at the key's radius, yaw and roll).
    pub fn get_or_draw(&self, key: SpriteKey, draw: impl FnOnce() -> Img<u8>) -> Arc<Img<u8>> {
        let slot = {
            let mut sprites = self.sprites.lock().unwrap_or_else(PoisonError::into_inner);
            Arc::clone(sprites.entry(key).or_default())
        };
        Arc::clone(slot.get_or_init(|| Arc::new(draw())))
    }

    /// The sprite of a fighter drawn from its mesh: `render_sprite(mesh, key.radius, key.yaw_deg(),
    /// key.roll_deg())`. `mesh` must be the mesh of `key.fighter`.
    pub fn mesh_sprite(&self, key: SpriteKey, mesh: &Mesh) -> Arc<Img<u8>> {
        self.get_or_draw(key, || render_sprite(mesh, key.radius, key.yaw_deg(), key.roll_deg()))
    }

    /// How many sprites have been asked for.
    pub fn len(&self) -> usize {
        self.sprites.lock().unwrap_or_else(PoisonError::into_inner).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A right-angled triangle near the canvas corner: the right angle at (1, 1), the long edge on x + y = 10.
    const S: [[f64; 2]; 3] = [[1.0, 1.0], [9.0, 1.0], [1.0, 9.0]];
    /// A colour and what the outline makes of it: `(np.array(px, np.uint8) * 0.3 + np.array(INK) * 0.7)
    /// .astype(np.uint8)` in Python.
    const PAINT: [u8; 3] = [200, 100, 50];
    const INKED: [u8; 3] = [69, 35, 31];

    fn put(cv: &mut Canvas, x: usize, y: usize, gid: i8, depth: f64) {
        let at = y * RW + x;
        (cv.img[at], cv.gb[at], cv.zb[at]) = (PAINT, gid, depth);
    }

    #[test]
    fn light_is_the_kits() {
        // piano_lowpoly.LIGHT on the reference PC
        assert_eq!(
            LIGHT.map(f64::to_bits),
            [0x3fd7_0a3d_70a3_d70a, 0x3fe9_9999_9999_999a, 0x3fde_b851_eb85_1eb8]
        );
    }

    #[test]
    fn products_are_fused_as_numpys_are() {
        // (a, b, v) and, from numpy on the reference PC: (A @ R.T)[0, 0] and a @ b (a row of A against a row of
        // R), (A @ v)[0], np.linalg.norm(a). In each case the plain sum and the two fused orders all differ.
        let cases: [[[u64; 3]; 4]; 3] = [
            [
                [0x3fe4_ba25_6841_e09a, 0x3fd5_c587_d5c1_74b0, 0xbfa1_5f39_da71_52a0],
                [0xbfd9_8a2b_605c_73d0, 0x3fe8_7a2a_4298_b058, 0x3fda_4274_5715_e6e0],
                [0xbfdc_dacf_2936_909c, 0x3fed_c10f_ca8b_3ab0, 0xbfea_0841_ebac_74b0],
                [0xbf88_f7f0_b48f_fce5, 0x3faa_8feb_ed22_28be, 0x3fe7_6fe0_192f_02f7],
            ],
            [
                [0x3fe8_8472_242b_1964, 0x3fe9_389e_3580_8fc0, 0x3fe1_2c2d_833f_04ae],
                [0x3fe6_b025_6f2d_dac0, 0xbfe6_9ad7_7fd1_be0c, 0x3fcd_43cd_9842_c068],
                [0xbfe8_afa7_5805_0d12, 0xbfb3_d0e9_912d_cd40, 0xbfdf_7d14_5f13_c978],
                [0x3fbb_f13c_432c_342c, 0xbfed_50a8_af1b_73ca, 0x3ff3_9230_f9ae_4d66],
            ],
            [
                [0xbfd3_7530_93c0_093c, 0x3fe5_eb64_1fe3_23c4, 0x3fed_372d_6436_99f0],
                [0x3fe7_8676_17e3_ab1e, 0x3fee_ec05_4d8e_ce02, 0xbfe1_e9db_2aca_d98c],
                [0x3fe0_4307_3659_d794, 0xbfe7_724d_d3f0_b02c, 0x3fea_6919_ff13_4d2a],
                [0xbfb2_9bb9_82d1_d301, 0x3fb8_dd06_5028_242b, 0x3ff2_e619_bb35_bbb6],
            ],
        ];
        for [a, b, v, want] in cases {
            let (a, b, v) = (a.map(f64::from_bits), b.map(f64::from_bits), v.map(f64::from_bits));
            assert_eq!(
                [dot3(a, b), dot3_mv(a, v), norm(a)].map(f64::to_bits),
                want,
                "{a:?} {b:?} {v:?}"
            );
        }
        // ORGAN at radius 16, yaw 0: the first corner of the tin face under canvas pixel (26, 21), `Q @ cam.up`
        // and its screen y. The plain sum is three steps away (0xbfdc_2da7_0f18_8fa8) and moved the depth there.
        let cam = Cam::new(0.0, 16, 19.5, 19.5, 0.0);
        let q = [0x3fbe_6b39_94a8_2ad4, 0xc007_16f3_a71c_1ad6, 0xc020_efa5_98e9_d28a].map(f64::from_bits);
        let (qu, d) = (dot3_mv(q, cam.up), dot3_mv(q, cam.fwd));
        assert_eq!(qu.to_bits(), 0xbfdc_2da7_0f18_8fab);
        assert_eq!((cam.cy - cam.f * qu / d).to_bits(), 0x403a_986a_bfb0_c9e7);
    }

    /// (degrees, cosine?, bits): Python's `math.sin` / `math.cos` of `math.radians(d)` on the reference PC at the
    /// 35 whole degrees in -360..720 where the old sine and cosine (musl's) were one step away. A step there
    /// turned every corner of the model and reached a pixel (ORGAN at radius 22, yaw 342).
    const PYTHON_TRIG: [(i32, bool, u64); 35] = [
        (-351, false, 0x3fc4_060b_67a8_537d),
        (-342, false, 0x3fd3_c6ef_372f_e954),
        (-284, true, 0x3fce_f74b_f2e4_b912),
        (-239, true, 0xbfe0_7b31_20fd_df15),
        (-224, false, 0x3fe6_3aa4_30e0_7310),
        (-220, true, 0xbfe8_836f_a2cf_5039),
        (-207, false, 0x3fdd_0e2e_2b44_ddfe),
        (-137, false, 0xbfe5_d2ee_398c_9c2b),
        (-113, true, 0xbfd9_01bd_2298_ffac),
        (-63, true, 0x3fdd_0e2e_2b44_de01),
        (-53, true, 0x3fe3_4211_9455_beb7),
        (-45, false, 0xbfe6_a09e_667f_3bcd),
        (-34, true, 0x3fea_8779_cda8_eea5),
        (34, true, 0x3fea_8779_cda8_eea5),
        (45, false, 0x3fe6_a09e_667f_3bcd),
        (53, true, 0x3fe3_4211_9455_beb7),
        (63, true, 0x3fdd_0e2e_2b44_de01),
        (113, true, 0xbfd9_01bd_2298_ffac),
        (137, false, 0x3fe5_d2ee_398c_9c2b),
        (207, false, 0xbfdd_0e2e_2b44_ddfe),
        (220, true, 0xbfe8_836f_a2cf_5039),
        (224, false, 0xbfe6_3aa4_30e0_7310),
        (239, true, 0xbfe0_7b31_20fd_df15),
        (284, true, 0x3fce_f74b_f2e4_b912),
        (342, false, 0xbfd3_c6ef_372f_e954),
        (351, false, 0xbfc4_060b_67a8_537d),
        (387, false, 0x3fdd_0e2e_2b44_ddfc),
        (400, false, 0x3fe4_91b7_523c_161c),
        (407, false, 0x3fe7_673f_e0c8_6983),
        (459, true, 0xbfc4_060b_67a8_5389),
        (502, false, 0x3fe3_b37f_b1bd_c939),
        (551, false, 0xbfc8_6c6d_dd76_6253),
        (587, true, 0xbfe5_d2ee_398c_9c2a),
        (679, false, 0xbfe4_fe6f_8138_4fd6),
        (688, false, 0xbfe0_f519_3eac_dd26),
    ];

    #[test]
    fn the_cameras_sines_are_pythons() {
        for &(d, cosine, bits) in &PYTHON_TRIG {
            let (s, c) = sin_cos_deg(f64::from(d));
            assert_eq!((if cosine { c } else { s }).to_bits(), bits, "{d} {cosine}");
        }
        let (s, c) = sin_cos_deg(342.0);
        assert_eq!(
            (s.to_bits(), c.to_bits()),
            (0xbfd3_c6ef_372f_e954, 0x3fee_6f0e_1344_54ff)
        );
    }

    #[test]
    fn camera_is_the_kits() {
        // sprites._Cam(204.0, 30, 33.5, 33.5, 30.0) on the reference PC
        let cam = Cam::new(204.0, 30, 33.5, 33.5, 30.0);
        assert_eq!(
            cam.eye.map(f64::to_bits),
            [0, 0x4003_d88c_38d8_dfee, 0x4021_4d7e_6fe5_2578]
        );
        assert_eq!(
            cam.fwd.map(f64::to_bits),
            [0x8000_0000_0000_0000, 0xbfd1_a40a_dd32_8e29, 0xbfee_c2a7_e35e_7b80]
        );
        assert_eq!(cam.right, [1.0, 0.0, 0.0]);
        assert_eq!(
            cam.up,
            [
                0.0,
                f64::from_bits(0x3fee_c2a7_e35e_7b80),
                f64::from_bits(0xbfd1_a40a_dd32_8e29)
            ]
        );
        assert_eq!(
            cam.r.map(|row| row.map(f64::to_bits)),
            [
                [0xbfe9_5121_4cb6_e112, 0xbfdf_ffff_ffff_ffff, 0xbfd6_8b2b_8830_69bb],
                [0xbfdd_3bc3_aeff_7f94, 0x3feb_b67a_e858_4cab, 0xbfca_07f9_2106_1ad0],
                [0x3fda_07f9_2106_1ad1, 0, 0xbfed_3bc3_aeff_7f95],
            ]
        );
        assert_eq!((cam.f, cam.cx, cam.cy), (270.0, 33.5, 33.5));
    }

    #[test]
    fn a_later_triangle_wins_only_where_strictly_nearer() {
        let mut cv = Canvas::new();
        tri(&mut cv, S, [5.0, 5.0, 5.0], [10, 20, 30], GID_INSTRUMENT);
        let at = 2 * RW + 2;
        assert_eq!((cv.img[at], cv.gb[at]), ([10, 20, 30], 2));
        let depth = cv.zb[at];
        assert!((depth - 0.2).abs() < 1e-15);
        // the same depth: the first one stays
        tri(&mut cv, S, [5.0, 5.0, 5.0], [99, 99, 99], GID_GROUND);
        assert_eq!((cv.img[at], cv.gb[at], cv.zb[at]), ([10, 20, 30], 2, depth));
        // farther: stays; nearer: replaced
        tri(&mut cv, S, [6.0, 6.0, 6.0], [99, 99, 99], GID_GROUND);
        assert_eq!((cv.img[at], cv.gb[at], cv.zb[at]), ([10, 20, 30], 2, depth));
        tri(&mut cv, S, [4.0, 4.0, 4.0], [1, 2, 3], GID_GROUND);
        assert_eq!((cv.img[at], cv.gb[at]), ([1, 2, 3], 1));
        assert!(cv.zb[at] > depth);
    }

    #[test]
    fn a_triangle_covers_the_pixels_whose_centres_are_inside() {
        let mut cv = Canvas::new();
        tri(&mut cv, S, [5.0, 5.0, 5.0], [255, 255, 255], GID_INSTRUMENT);
        let covered = |x: usize, y: usize| cv.gb[y * RW + x] != 0;
        // the centres (4.5, 5.5) and (8.5, 1.5) lie on the long edge itself and count; (5.5, 5.5) is outside
        assert!(covered(1, 1) && covered(8, 1) && covered(1, 8) && covered(4, 5) && covered(5, 4));
        assert!(!covered(0, 1) && !covered(1, 0) && !covered(5, 5) && !covered(9, 1) && !covered(1, 9));
        assert_eq!(cv.gb.iter().filter(|&&g| g != 0).count(), 36);
        // a sliver below the area threshold draws nothing
        let mut cv = Canvas::new();
        let sliver = [[1.0, 1.0], [9.0, 1.0], [5.0, 1.0 + 1e-11]];
        tri(&mut cv, sliver, [5.0; 3], [255; 3], GID_INSTRUMENT);
        assert!(cv.gb.iter().all(|&g| g == 0));
        // larger than the canvas: clipped to it
        let mut cv = Canvas::new();
        let huge = [[-50.0, -50.0], [900.0, -40.0], [-40.0, 1500.0]];
        tri(&mut cv, huge, [5.0; 3], [7; 3], GID_INSTRUMENT);
        assert_eq!((cv.gb[0], cv.gb[RW * RH - 1]), (2, 2));
        // wholly off the canvas: nothing
        tri(
            &mut cv,
            [[-50.0, -50.0], [-10.0, -40.0], [-40.0, -10.0]],
            [1.0; 3],
            [9; 3],
            GID_INSTRUMENT,
        );
        tri(
            &mut cv,
            [[300.0, 500.0], [400.0, 500.0], [300.0, 600.0]],
            [1.0; 3],
            [9; 3],
            GID_INSTRUMENT,
        );
        assert!(cv.img.iter().all(|&p| p == [7; 3]));
    }

    #[test]
    fn outline_mixes_ink_as_numpy_does() {
        for (col, want) in [
            (PAINT, INKED),
            ([255, 255, 255], [86, 82, 93]),
            ([0, 0, 0], [9, 5, 16]),
            ([1, 2, 3], [10, 6, 17]),
        ] {
            let mut cv = Canvas::new();
            put(&mut cv, 5, 5, GID_INSTRUMENT, 0.1);
            cv.img[5 * RW + 5] = col;
            outline(&mut cv);
            assert_eq!(cv.img[5 * RW + 5], want);
        }
    }

    /// A 3 x 3 block of instrument pixels at depth value 0.1, with the middle at `middle` and the pixel under it
    /// at `below`; returns whether the middle pixel was inked.
    fn middle_inked(middle: f64, below: f64) -> bool {
        let mut cv = Canvas::new();
        for y in 4..7 {
            for x in 4..7 {
                put(&mut cv, x, y, GID_INSTRUMENT, 0.1);
            }
        }
        cv.zb[5 * RW + 5] = middle;
        cv.zb[6 * RW + 5] = below;
        outline(&mut cv);
        // the eight around the middle touch the background and are always inked
        assert_eq!(
            cv.img.iter().filter(|&&p| p == INKED).count() - usize::from(cv.img[5 * RW + 5] == INKED),
            8
        );
        cv.img[5 * RW + 5] == INKED
    }

    #[test]
    fn outline_inks_a_step_in_depth() {
        // inside a flat surface: no ink
        assert!(!middle_inked(0.1, 0.1));
        // a little nearer than its neighbours (they are at 0.1, not below 0.9 * 0.11): still none
        assert!(!middle_inked(0.11, 0.1));
        // a neighbour well behind (its depth value below 0.9 of the pixel's): ink
        assert!(middle_inked(0.11, 0.09));
        assert!(middle_inked(0.2, 0.1));
        // the comparison is strict: a neighbour at 0.9 of the pixel's value exactly does not ink it
        let edge: f64 = 0.11 * 0.9;
        assert!(!middle_inked(0.11, edge));
        assert!(middle_inked(0.11, f64::from_bits(edge.to_bits() - 1)));
    }

    #[test]
    fn outline_knows_the_ground_group() {
        // a row: background, ground, ground, instrument, instrument, instrument, background; the rows above and
        // below repeat it, so only the row's own neighbours to the left and right decide
        let mut cv = Canvas::new();
        let groups = [
            0,
            GID_GROUND,
            GID_GROUND,
            GID_INSTRUMENT,
            GID_INSTRUMENT,
            GID_INSTRUMENT,
            0,
        ];
        for y in 8..11 {
            for (x, &g) in groups.iter().enumerate() {
                if g != 0 {
                    put(&mut cv, x, y, g, 0.1);
                }
            }
        }
        outline(&mut cv);
        let got: Vec<bool> = (0..7).map(|x| cv.img[9 * RW + x] == INKED).collect();
        // ground is inked next to the background only; the instrument next to the ground and the background
        assert_eq!(got, [false, true, false, true, false, true, false]);
    }

    #[test]
    fn outline_wraps_at_the_canvas_edges() {
        // np.roll wraps: with the whole canvas one flat instrument surface, no pixel has an empty neighbour
        let mut cv = Canvas::new();
        cv.img.fill(PAINT);
        cv.gb.fill(GID_INSTRUMENT);
        cv.zb.fill(0.1);
        outline(&mut cv);
        assert!(cv.img.iter().all(|&p| p == PAINT));
        // and one empty pixel in the last column inks its neighbour in the first column of the same row
        let mut cv = Canvas::new();
        cv.img.fill(PAINT);
        cv.gb.fill(GID_INSTRUMENT);
        cv.zb.fill(0.1);
        cv.gb[3 * RW + RW - 1] = 0;
        outline(&mut cv);
        assert_eq!(cv.img[3 * RW], INKED);
        assert_eq!(cv.img.iter().filter(|&&p| p == INKED).count(), 4);
    }

    #[test]
    fn an_empty_mesh_is_an_empty_sprite() {
        let sprite = render_sprite(&Mesh { faces: Vec::new() }, 12, 0.0, 0.0);
        assert_eq!((sprite.w, sprite.h, sprite.c), (31, 31, 4));
        assert_eq!(sprite.data, vec![0; 31 * 31 * 4]);
        let largest = render_sprite(&Mesh { faces: Vec::new() }, 131, 0.0, 0.0);
        assert_eq!((largest.w, largest.h), (269, 269));
    }

    #[test]
    #[should_panic(expected = "does not fit")]
    fn a_sprite_wider_than_the_canvas_is_refused() {
        render_sprite(&Mesh { faces: Vec::new() }, 132, 0.0, 0.0);
    }

    /// The middle pixel of the sprite of one unit square in the z = 0 plane, seen at yaw 0 and roll 0.
    fn middle_of_square(n: V3, rgb: V3, emit: bool, two: bool, spec: f64) -> [u8; 4] {
        let mesh = Mesh {
            faces: vec![Face {
                p: vec![[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.5, 0.5, 0.0], [-0.5, 0.5, 0.0]],
                n,
                rgb,
                two,
                emit,
                gid: 2,
                spec,
            }],
        };
        let s = render_sprite(&mesh, 12, 0.0, 0.0);
        let at = (15 * s.w + 15) * 4;
        [s.data[at], s.data[at + 1], s.data[at + 2], s.data[at + 3]]
    }

    #[test]
    fn faces_are_shaded_in_bands() {
        // Facing the camera, n . LIGHT = 0.48: 0.36 + 0.64 * 0.48 = 0.6672; floor(7 * 0.6672) / 7 + 0.05 = 0.6214...
        // and int(200.9) * 0.6214 = 124.3, 100 * 0.6214 = 62.1, 50 * 0.6214 = 31.07.
        let front = [0.0, 0.0, 1.0];
        let back = [0.0, 0.0, -1.0];
        let rgb = [200.9, 100.0, 50.0];
        assert_eq!(middle_of_square(front, rgb, false, false, 0.0), [124, 62, 31, 255]);
        // emitting: the colour as it is, truncated to whole numbers
        assert_eq!(middle_of_square(front, rgb, true, false, 0.0), [200, 100, 50, 255]);
        // facing away: culled, unless two-sided, when it is lit as if it faced the camera
        assert_eq!(middle_of_square(back, rgb, false, false, 0.0), [0, 0, 0, 0]);
        assert_eq!(middle_of_square(back, rgb, false, true, 0.0), [124, 62, 31, 255]);
    }
}
