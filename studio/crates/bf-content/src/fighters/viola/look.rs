//! VIOLA's look: the `Look` half of its reference kit module (`tools/studio_export/older/fighters/viola.py`, moved
//! from `projects/ballfight_viola_vs_violin/` (#42)), hook by hook, with #42's `styles.py` draw functions.
//!
//! What it draws: the whole notes (the hollow oval of the notation standing up as a wheel, ink with a teal-lit rim,
//! rolling and bobbing; their shadows from the spot; a soft glow; a flash and motes as one appears; dust, ink chips and
//! a crack where one crushes, VIOLA JOKE's also a scorch); the resting viola's pulsing pool on the canvas; the giant
//! quarter rest over the resting ball with the count above it, and the eighth notes rising off it during MELODY AT
//! LAST and over the champion's solo (opt-in `after_post`); MELODY AT LAST's halo (the ball's glow); VIOLA JOKE's
//! target under the falling note, its landing ring and shimmer, the frozen enemy greyed (the glass through `ball`, the
//! opt-ins `ball_tint` and `ball_lit`), the crowd's laugh (`crowd_laugh`), its flashbulbs on the big note only
//! (`flash_hit`), a flattened dart's spark (`spark_events`); the instrument with #42's wider crop (`render_padded`,
//! half-side `ceil(1.2 r)`) in the ball and on the VS card; shards of string, wood in its varnish, the bridge, the
//! scroll.
//!
//! Numbers: every `math.*` and numpy float ufunc goes through `bf_core::dmath`; `np.linalg.norm` and `@` of a
//! 3-vector are the fused products of the reference PC (ruling M3-R30); `np.cross` is plain.

use std::any::Any;
use std::sync::{Arc, OnceLock};

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{H, SAFE, W, ease, ss};
use bf_picture::fx::{Emit, Mark, MarkKind, shimmer_world};
use bf_picture::img::{Img, Rgb8};
use bf_picture::lighting::{Lights, shade};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Fx, Look, Payload, Phase, Pic, Spark, Tint};
use bf_picture::raster::Frame;
use bf_picture::ring::state::BallState;
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::shadows::proj_point;
use bf_picture::sprite::{Mesh, render_sprite_padded};
use bf_picture::ui::{DrawLog, label};
use bf_picture::{font, np, pil, title};

/// The same moves with the bold effects.
#[path = "bold.rs"]
mod bold;

// ================================================================ #42 styles.py: the whole note
/// `INK`: the notehead's ink (a deep blue-black, so the teal rim reads).
const INK: V3 = [44.0, 44.0, 66.0];
/// `NOTE_A, NOTE_B`: the outer oval (x the note's radius).
const NOTE_A: f64 = 1.0;
const NOTE_B: f64 = 0.68;
/// `HOLE_A, HOLE_B`: the hole.
const HOLE_A: f64 = 0.36;
const HOLE_B: f64 = 0.56;
/// `THICK`: the slab's thickness (x radius).
const THICK: f64 = 0.22;
/// `DRAW`: drawn half-width x the hit radius.
const DRAW: f64 = 1.3;
/// `K_PTS`.
const K_PTS: usize = 20;
/// `NOTE_MEAN = (ST.NOTE_A + ST.NOTE_B) / 2` (render.py 160).
const NOTE_MEAN: f64 = (NOTE_A + NOTE_B) / 2.0;
/// `WOOD_COL`: the viola's varnish (render.py 464).
const WOOD_COL: V3 = [120.0, 50.0, 28.0];
/// `INKY`: ink flecks off a crushing note (render.py 954).
const INKY: [V3; 3] = [[40.0, 40.0, 60.0], [70.0, 76.0, 96.0], [24.0, 22.0, 34.0]];
/// `LAUGH`: seconds the crowd laughs after the rimshot.
const LAUGH: f64 = 1.9;
/// `REST_GLYPH`: the quarter rest.
const REST_GLYPH: [&str; 16] = [
    "..#....", "..##...", "...##..", "....##.", "...###.", "..###..", ".###...", ".##....", "..##...", "...##..",
    "...###.", "..#.##.", ".#.....", ".#.....", "..##...", "....#..",
];
/// `EIGHTH_GLYPH`: a flagged eighth.
const EIGHTH_GLYPH: [&str; 7] = ["..#..", "..##.", "..#.#", "..#..", "..#..", "###..", "###.."];

/// `HOLE_TILT = math.radians(-32)`.
fn hole_tilt() -> f64 {
    -32.0 * (dmath::PI / 180.0)
}

/// `a @ b` of two 3-vectors, as numpy's dot is on the reference PC (fused, ruling M3-R30).
fn dot3(a: V3, b: V3) -> f64 {
    dmath::fma(a[2], b[2], dmath::fma(a[1], b[1], a[0] * b[0]))
}

/// `np.linalg.norm(x)` of one 3-vector: the square root of the fused `x @ x`.
fn norm3(x: V3) -> f64 {
    dot3(x, x).sqrt()
}

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn mul(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

/// `_oval(a, b, tilt, phi)`: `K_PTS` points of an ellipse turned by `tilt - phi`.
fn oval(a: f64, b: f64, tilt: f64, phi: f64) -> Vec<[f64; 2]> {
    // th = np.linspace(0, 2 * math.pi, K_PTS + 1)[:-1]: i * step + 0, step = 2 pi / K_PTS
    let step = (2.0 * dmath::PI - 0.0) / K_PTS as f64;
    let (c, s) = (dmath::cos(tilt - phi), dmath::sin(tilt - phi));
    (0..K_PTS)
        .map(|i| {
            let th = i as f64 * step + 0.0;
            let (x, y) = (a * dmath::cos(th), b * dmath::sin(th));
            [x * c - y * s, x * s + y * c]
        })
        .collect()
}

/// `support(phi)`: the outer oval's half-height turned by `phi` (x radius).
fn support(phi: f64) -> f64 {
    (dmath::pow(NOTE_A * dmath::sin(phi), 2.0) + dmath::pow(NOTE_B * dmath::cos(phi), 2.0)).sqrt()
}

/// `note_axes(cam, lean=0.25)`: the notehead's screen-right `a`, its up axis `b` and its face normal `n`.
fn note_axes(cam: &Cam) -> (V3, V3, V3) {
    let lean = 0.25;
    let mut a = cam.m[0];
    a[1] = 0.0;
    let na = norm3(a);
    a = [a[0] / na, a[1] / na, a[2] / na];
    // b = (1 - lean) * cam.M[1] + lean * np.array([0.0, 1.0, 0.0])
    let m1 = cam.m[1];
    let up = [0.0, 1.0, 0.0];
    let mut b: V3 = std::array::from_fn(|c| (1.0 - lean) * m1[c] + lean * up[c]);
    let ba = dot3(b, a);
    b = sub(b, mul(a, ba));
    let nb = norm3(b);
    b = [b[0] / nb, b[1] / nb, b[2] / nb];
    // np.cross(a, b)
    let mut n = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    if dot3(n, sub(cam.pos, cam.target)) < 0.0 {
        n = [-n[0], -n[1], -n[2]];
    }
    (a, b, n)
}

/// `note_centre(c, r, phi, lift, b)`: the world centre of a note at ring point `c`.
fn note_centre(c: [f64; 2], r: f64, phi: f64, lift: f64, b: V3) -> V3 {
    let hc = b[1] * r * support(phi) + lift;
    let base = add(scene::to_world(c[0], c[1], 0.0), [0.0, hc, 0.0]);
    // - np.array([b[0], 0.0, b[2]]) * r * support(phi) * 0.0
    let sup = support(phi);
    let off: V3 = [b[0], 0.0, b[2]].map(|v| v * r * sup * 0.0);
    sub(base, off)
}

/// `Wp(pq, w) = C + a * pq[0] + b * pq[1] + nrm * w`.
fn wp(cc: V3, a: V3, b: V3, nrm: V3, pq: [f64; 2], w: f64) -> V3 {
    std::array::from_fn(|i| cc[i] + a[i] * pq[0] + b[i] * pq[1] + nrm[i] * w)
}

/// The kind of one face of a note: `'face'`, `'rim'` or `'hole'`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)] // the kit's own names
enum Face {
    Face,
    Rim,
    Hole,
}

/// `note_quads(cam, c, r, phi, lift, scale)`: the note's quads `(Q, normal, kind)`.
fn note_quads(cam: &Cam, c: [f64; 2], r: f64, phi: f64, lift: f64, scale: f64) -> Vec<([V3; 4], V3, Face)> {
    let r = r * scale * DRAW;
    let (a, b, nrm) = note_axes(cam);
    let cc = note_centre(c, r, phi, lift, b);
    let o: Vec<[f64; 2]> = oval(NOTE_A, NOTE_B, 0.0, phi)
        .iter()
        .map(|p| [p[0] * r, p[1] * r])
        .collect();
    let ih: Vec<[f64; 2]> = oval(HOLE_A, HOLE_B, hole_tilt(), phi)
        .iter()
        .map(|p| [p[0] * r, p[1] * r])
        .collect();
    let th = THICK * r / 2.0;
    let w = |pq: [f64; 2], d: f64| wp(cc, a, b, nrm, pq, d);
    let mut out = Vec::with_capacity(K_PTS * 4);
    for j in 0..K_PTS {
        let j1 = (j + 1) % K_PTS;
        for sgn in [1.0, -1.0] {
            let d = sgn * th;
            out.push((
                [w(o[j], d), w(o[j1], d), w(ih[j1], d), w(ih[j], d)],
                mul(nrm, sgn),
                Face::Face,
            ));
        }
        let mo = [(o[j][0] + o[j1][0]) / 2.0, (o[j][1] + o[j1][1]) / 2.0];
        let rad: V3 = std::array::from_fn(|i| a[i] * mo[0] + b[i] * mo[1]);
        let nr = norm3(rad) + 1e-9;
        let rad = rad.map(|v| v / nr);
        out.push(([w(o[j], th), w(o[j1], th), w(o[j1], -th), w(o[j], -th)], rad, Face::Rim));
        let mi = [(ih[j][0] + ih[j1][0]) / 2.0, (ih[j][1] + ih[j1][1]) / 2.0];
        let radi: V3 = std::array::from_fn(|i| -(a[i] * mi[0] + b[i] * mi[1]));
        let ni = norm3(radi) + 1e-9;
        let radi = radi.map(|v| v / ni);
        out.push((
            [w(ih[j], th), w(ih[j1], th), w(ih[j1], -th), w(ih[j], -th)],
            radi,
            Face::Hole,
        ));
    }
    out
}

/// `np.argsort(x)` (numpy's default sort) for distinct keys: the ascending order. numpy promises no order for equal
/// keys; a note's faces and its edge's points do not tie.
fn argsort(x: &[f64]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..x.len()).collect();
    idx.sort_by(|&i, &j| x[i].total_cmp(&x[j]));
    idx
}

/// `Q.mean(0)` of a quad: the rows added in order, then divided by 4.
fn mean4(q: &[V3; 4]) -> V3 {
    std::array::from_fn(|c| (((q[0][c] + q[1][c]) + q[2][c]) + q[3][c]) / 4.0)
}

/// `tuple(int(v) for v in x)`: a Pillow ink (truncated toward zero).
fn ink(x: V3) -> [u8; 3] {
    x.map(|v| v as i64 as u8)
}

/// `draw_note(fr, cam, L, shade, c, r, phi, col, lift, scale, glow)`: paints a whole note (far quads first), then a
/// lit edge along the top of its outer oval.
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_note(
    fr: &mut Frame,
    cam: &Cam,
    l: &Lights,
    c: [f64; 2],
    r: f64,
    phi: f64,
    col: V3,
    lift: f64,
    scale: f64,
    glow: f64,
) {
    let quads = note_quads(cam, c, r, phi, lift, scale);
    let ctr: Vec<V3> = quads.iter().map(|(q, _, _)| mean4(q)).collect();
    let d = cam.depth(&ctr);
    let neg: Vec<f64> = d.iter().map(|v| -v).collect();
    let order = argsort(&neg);
    let nrm: Vec<V3> = quads.iter().map(|(_, n, _)| *n).collect();
    let base: Vec<V3> = quads
        .iter()
        .map(|(_, _, k)| match k {
            Face::Face => INK,
            Face::Rim => std::array::from_fn(|i| INK[i] * 0.6 + col[i] * 0.45),
            Face::Hole => INK.map(|v| v * 0.7),
        })
        .collect();
    let cols = shade(l, &nrm, &ctr, &base, None, 0.7, 0.0, 5.0);
    for j in order {
        let (q, _, kind) = &quads[j];
        let mut cc = cols[j];
        match kind {
            Face::Rim => cc = std::array::from_fn(|i| cc[i] + col[i] * (0.35 + 0.45 * glow) + 20.0),
            Face::Face => cc = std::array::from_fn(|i| cc[i] + col[i] * 0.08 * (1.0 + 2.0 * glow)),
            Face::Hole => {}
        }
        fr.poly(cam, q, cc.map(|v| np::clip(v, 0.0, 255.0)), true);
    }
    // a lit edge along the top of the outer oval (the spot catches it)
    let (a, b, n) = note_axes(cam);
    let rr = r * scale * DRAW;
    let cc = note_centre(c, rr, phi, lift, b);
    let o: Vec<[f64; 2]> = oval(NOTE_A, NOTE_B, 0.0, phi)
        .iter()
        .map(|p| [p[0] * rr, p[1] * rr])
        .collect();
    let w = n.map(|v| v * THICK * rr / 2.0);
    let top: Vec<V3> = o
        .iter()
        .filter(|pq| pq[1] > 0.25 * rr)
        .map(|pq| std::array::from_fn(|i| cc[i] + a[i] * pq[0] + b[i] * pq[1] + w[i]))
        .collect();
    if top.len() > 1 {
        let sp = cam.wp(&top);
        let xs: Vec<f64> = sp.iter().map(|s| s.0).collect();
        let pts: Vec<(f64, f64)> = argsort(&xs).into_iter().map(|j| sp[j]).collect();
        let edge = col.map(|v| np::clip(v * 0.6 + 110.0, 0.0, 255.0));
        pil::line(&mut fr.pil, &pts, &ink(edge), 1);
    }
}

/// `note_shadow(cam, c, r, phi, lift)`: the note's outer oval as a world polygon (every second point).
fn note_shadow(cam: &Cam, c: [f64; 2], r: f64, phi: f64, lift: f64) -> Vec<V3> {
    let (a, b, _) = note_axes(cam);
    let r = r * DRAW;
    let cc = note_centre(c, r, phi, lift, b);
    oval(NOTE_A, NOTE_B, 0.0, phi)
        .iter()
        .step_by(2)
        .map(|p| {
            let pq = [p[0] * r, p[1] * r];
            std::array::from_fn(|i| cc[i] + a[i] * pq[0] + b[i] * pq[1])
        })
        .collect()
}

/// `glyph(rows, scale)`: a boolean glyph (1 / 0), each pixel repeated `scale` times both ways.
fn glyph(rows: &[&str], scale: usize) -> Img<u8> {
    let (h, w) = (rows.len() * scale, rows[0].len() * scale);
    let mut g: Img<u8> = Img::new(w, h, 1);
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch == '#' {
                for dy in 0..scale {
                    for dx in 0..scale {
                        g.px_mut(x * scale + dx, y * scale + dy)[0] = 1;
                    }
                }
            }
        }
    }
    g
}

/// `stamp_glyph(img, g, cx, cy, col, alpha, outline, shadow)`: a glyph centred at `(cx, cy)` with a 1 px outline
/// (and a drop shadow), on the `uint8` frame. Each layer goes through `float32` as numpy does: the frame's sample times
/// `1 - a` in `float32`, plus the colour times `a` in `float64`, stored back to `float32`, then truncated to `uint8`.
#[allow(clippy::too_many_arguments)] // the Python signature
fn stamp_glyph(img: &mut Rgb8, g: &Img<u8>, cx: f64, cy: f64, col: V3, alpha: f64, outline: V3, shadow: bool) {
    let o = font::dilate(g, 1);
    let (gh, gw) = (o.h as i64, o.w as i64);
    let x0 = py::round_half_even(cx - gw as f64 / 2.0) as i64;
    let y0 = py::round_half_even(cy - gh as f64 / 2.0) as i64;
    let off = (o.h - g.h) / 2;
    let mut inner: Img<u8> = Img::new(o.w, o.h, 1);
    for y in 0..g.h {
        for x in 0..g.w {
            inner.px_mut(x + off, y + off)[0] = g.px(x, y)[0];
        }
    }
    let mut layers: Vec<(&Img<u8>, V3, i64, i64, f64)> = Vec::new();
    if shadow {
        layers.push((&o, [0.0, 0.0, 0.0], 1, 2, 0.45));
    }
    layers.push((&o, outline, 0, 0, 1.0));
    layers.push((&inner, col, 0, 0, 1.0));
    for (m, c, ox, oy, a0) in layers {
        let (xa, ya) = ((x0 + ox).max(0), (y0 + oy).max(0));
        let (xb, yb) = ((x0 + ox + gw).min(W as i64), (y0 + oy + gh).min(H as i64));
        if xb <= xa || yb <= ya {
            continue;
        }
        let a = alpha * a0;
        let keep = (1.0 - a) as f32;
        for y in ya..yb {
            for x in xa..xb {
                if m.px((x - x0 - ox) as usize, (y - y0 - oy) as usize)[0] == 0 {
                    continue;
                }
                let px = img.px_mut(x as usize, y as usize);
                for ch in 0..3 {
                    let v = f32::from(px[ch]) * keep;
                    let s = (f64::from(v) + c[ch] * a) as f32;
                    px[ch] = s as u8;
                }
            }
        }
    }
}

/// `_safe_x(x, w, y, h)`: a small label kept inside the safe box and out of the button column.
fn safe_x(x: i64, w: i64, y: i64, h: i64) -> i64 {
    let x1 = if y + h > 290 { 300 } else { SAFE.2 as i64 };
    (x.max(SAFE.0 as i64 + 1)).min(x1 - w - 1)
}

/// One whole note on screen: `notes_at`'s tuple.
#[derive(Clone, Copy, Debug, PartialEq)]
struct NoteView {
    c: [f64; 2],
    phi: f64,
    r: f64,
    lift: f64,
    sc: f64,
    glow: f64,
    owner: usize,
}

/// A note's track, from the record (`_notes_build`): its frames and positions, its turn as it rolls.
#[derive(Clone, Debug, PartialEq)]
struct Track {
    id: i64,
    owner: usize,
    r: f64,
    t0: f64,
    t1: f64,
    tl: f64,
    hit: Option<f64>,
    f0: usize,
    f1: usize,
    p: Vec<[f64; 2]>,
    phi: Vec<f64>,
}

fn num(v: &Value) -> f64 {
    match v {
        Value::Float(x) => *x,
        Value::Int(n) => *n as f64,
        other => panic!("NOTES / REST: {other:?} is not a number"),
    }
}

/// VIOLA's look.
#[derive(Default)]
pub struct ViolaLook {
    k: usize,
    enemy: usize,
    col: V3,
    dark: V3,
    pp: String,
    view: (f64, f64),
    low_size: f64,
    /// `(t, victim, dur)` of its stuns.
    stuns: Vec<(f64, usize, f64)>,
    /// `(t, victim, t_land, x, z)` of its jokes.
    jokes: Vec<(f64, usize, f64, f64, f64)>,
    /// `(t, until)` of its MELODY AT LASTs.
    melodies: Vec<(f64, f64)>,
    /// The times of its weapon's own notes.
    note_ev: Vec<f64>,
    tracks: OnceLock<Vec<Track>>,
}

fn model() -> Arc<Mesh> {
    static MODEL: OnceLock<Arc<Mesh>> = OnceLock::new();
    MODEL
        .get_or_init(|| {
            let json = super::MESH.expect("the VIOLA's model is compiled in");
            Arc::new(Mesh::from_json(json).expect("the VIOLA's model reads"))
        })
        .clone()
}

impl ViolaLook {
    /// `_notes_build`: its own notes' tracks (built on first use: it reads the camera).
    fn tracks(&self, p: &PictureCtx) -> &[Track] {
        self.tracks.get_or_init(|| {
            let mut out: Vec<Track> = Vec::new();
            for i in 0..p.run.nsim {
                let Some(Value::List(rows)) = p.run.nested("NOTES", i) else {
                    continue;
                };
                for row in rows {
                    let Value::List(v) = row else { continue };
                    if num(&v[0]) as usize != self.k {
                        continue;
                    }
                    let id = num(&v[7]) as i64;
                    let idx = match out.iter().position(|t| t.id == id) {
                        Some(j) => j,
                        None => {
                            out.push(Track {
                                id,
                                owner: self.k,
                                r: num(&v[3]),
                                t0: num(&v[4]),
                                t1: num(&v[5]),
                                tl: num(&v[6]),
                                hit: None,
                                f0: i,
                                f1: i,
                                p: Vec::new(),
                                phi: Vec::new(),
                            });
                            out.len() - 1
                        }
                    };
                    let t = &mut out[idx];
                    t.f1 = i;
                    t.p.push([num(&v[1]), num(&v[2])]);
                    if num(&v[9]) != 0.0 && t.hit.is_none() {
                        t.hit = Some(p.show.grid.t_fight + i as f64 / f64::from(p.show.grid.fps));
                    }
                }
            }
            // (the turn's sign reads the camera's screen-right; the aims of the `note` events only set the starting
            // heading, which the turn does not read)
            let right = p.camera(0.0).m[0];
            for t in &mut out {
                let mut phi = vec![0.0; t.p.len()];
                for j in 1..t.p.len() {
                    let step = [t.p[j][0] - t.p[j - 1][0], t.p[j][1] - t.p[j - 1][1]];
                    let ds = dmath::np_hypot(step[0], step[1]);
                    let sw = sub(scene::to_world(step[0], step[1], 0.0), scene::to_world(0.0, 0.0, 0.0));
                    let sgn = if dot3(sw, right) >= 0.0 { 1.0 } else { -1.0 };
                    phi[j] = phi[j - 1] + sgn * ds / (t.r * NOTE_MEAN);
                }
                t.phi = phi;
            }
            out
        })
    }

    /// `notes_at(t)`: its whole notes on screen at `t`.
    fn notes_at(&self, p: &PictureCtx, t: f64) -> Vec<NoteView> {
        let mut out = Vec::new();
        if t < p.show.grid.t_fight || t >= p.t_hs + 0.6 {
            return out;
        }
        let tt = py_min(t, p.t_ko);
        let fi = p.sim_i(tt);
        let fade = 1.0 - ss(p.t_hs, p.t_hs + 0.6, t);
        for d in self.tracks(p) {
            if !(d.f0 <= fi && fi <= d.f1) {
                continue;
            }
            let j = fi - d.f0;
            let (c, mut phi) = (d.p[j], d.phi[j]);
            let (mut lift, mut glow) = (0.0, 0.0);
            if d.tl > d.t0 + 1e-6 && tt < d.tl {
                // dropped from above (VIOLA JOKE)
                let s = (tt - d.t0) / (d.tl - d.t0);
                lift = 4.6 * (1.0 - s * s);
                phi = 0.4 * dmath::sin(6.0 * s);
            }
            let mut sc = if d.tl <= d.t0 + 1e-6 {
                ease(py_min(1.0, (tt - d.t0) / 0.10))
            } else {
                1.0
            };
            sc *= 1.0 - ss(d.t1 - 0.14, d.t1, tt);
            if let Some(h) = d.hit
                && 0.0 <= tt - h
                && tt - h < 0.25
            {
                glow = 1.0 - (tt - h) / 0.25;
            }
            if tt - d.t0 < 0.15 {
                glow = py_max(glow, 1.0 - (tt - d.t0) / 0.15);
            }
            sc *= fade;
            if sc > 0.03 {
                out.push(NoteView {
                    c,
                    phi,
                    r: d.r,
                    lift,
                    sc,
                    glow,
                    owner: d.owner,
                });
            }
        }
        out
    }

    /// `notes_at(F.t)`, once a frame.
    fn notes<'a>(&self, p: &PictureCtx, f: &'a FrameCtx<'_>) -> &'a [NoteView] {
        f.memo[self.k]
            .get_or_init(|| Box::new(self.notes_at(p, f.t)) as Box<dyn Any + Send + Sync>)
            .downcast_ref::<Vec<NoteView>>()
            .expect("its notes")
    }

    /// `rest_state(t)`: (resting, count, melody) from the record; nothing before the fight or after the K.O.
    fn rest_state(&self, p: &PictureCtx, t: f64) -> (bool, i64, bool) {
        if !(p.show.grid.t_fight <= t && t < p.t_ko) {
            return (false, 0, false);
        }
        let Some(Value::List(balls)) = p.run.nested("REST", p.sim_i(t)) else {
            return (false, 0, false);
        };
        let Value::List(r) = &balls[self.k] else {
            panic!("REST: a row per ball")
        };
        (num(&r[0]) as i64 != 0, num(&r[1]) as i64, num(&r[2]) as i64 != 0)
    }

    fn stun_k(&self, p: &PictureCtx, kk: usize, t: f64) -> f64 {
        let tt = py_min(t, p.t_ko);
        for &(t0, v, d) in &self.stuns {
            if v == kk && t0 <= tt && tt < t0 + d + 0.15 {
                return py_min(1.0, (tt - t0) / 0.06) * (1.0 - ss(t0 + d, t0 + d + 0.15, tt));
            }
        }
        0.0
    }

    /// VIOLA JOKE's freeze on ball `kk` at `t` (the stun state, in the fight only).
    fn frz(&self, p: &PictureCtx, kk: usize, t: f64) -> f64 {
        if kk != self.enemy || !(p.show.grid.t_fight <= t && t < p.t_hs) {
            return 0.0;
        }
        self.stun_k(p, kk, t)
    }

    fn laugh_k(&self, t: f64) -> f64 {
        let mut best = 0.0;
        for &(t0, ..) in &self.jokes {
            if t0 <= t && t < t0 + LAUGH + 0.4 {
                best = py_max(
                    best,
                    py_min(1.0, (t - t0) / 0.12) * (1.0 - ss(t0 + LAUGH, t0 + LAUGH + 0.4, t)),
                );
            }
        }
        best
    }

    fn melody_k(&self, p: &PictureCtx, t: f64) -> f64 {
        let tt = py_min(t, p.t_ko);
        for &(t0, until) in &self.melodies {
            if t0 <= tt && tt < until {
                return py_min(1.0, (tt - t0) / 0.2) * (1.0 - ss(until - 0.25, until, tt));
            }
        }
        0.0
    }

    fn caption_up(&self, p: &PictureCtx, t: f64) -> bool {
        let s = &p.show;
        s.caps_ability[0]
            .iter()
            .chain(&s.caps_ability[1])
            .chain(&s.cap_ko)
            .chain(&s.cap_wins)
            .chain(&s.cap_comment)
            .any(|c| c.t0 - 0.05 <= t && t < c.t1 + 0.1)
    }

    /// `draw_rests(img, t, drawn)`: the giant quarter rest and the count, the rising eighths.
    fn draw_rests(&self, p: &PictureCtx, img: &mut Rgb8, log: &mut DrawLog, t: f64, drawn: &[DrawnBall]) {
        let k = self.k;
        let Some(db) = drawn.iter().find(|d| d.k == k) else {
            return;
        };
        let g = &p.show.grid;
        let (resting, count, _) = self.rest_state(p, t);
        let ((sx, sy), rp) = (db.s, db.rp);
        let rest_g = glyph(&REST_GLYPH, 2);
        if resting && count > 0 {
            // the giant quarter rest over the ball (it settles in on each counted beat, sways a little)
            let t_beat = g.t_fight + ((t - g.t_fight) / g.beat + 1e-6).floor() * g.beat;
            let u = py_min(1.0, (t - t_beat) / 0.08);
            let gy = sy - rp - rest_g.h as f64 / 2.0 - 3.0 - 3.0 * (1.0 - u);
            let col = self.col.map(|v| np::clip(v * 0.55 + 110.0, 0.0, 255.0));
            let outline = ink(self.dark.map(|v| v * 0.45)).map(f64::from);
            let gx = sx + 1.5 * dmath::sin(2.0 * dmath::PI * t / g.bar);
            stamp_glyph(img, &rest_g, gx, gy, col, 0.4 + 0.6 * u, outline, true);
            // the small count above it: "1", "1 2", "1 2 3" (the newest bright)
            let adv = font::ADV as i64;
            let txt_w = count * (adv + 2) - 2;
            let mut y = py::round_half_even(gy - rest_g.h as f64 / 2.0 - 11.0) as i64;
            let mut x = safe_x(py::round_half_even(sx - txt_w as f64 / 2.0) as i64, txt_w, y, 7);
            let band = SAFE.1 as i64 + 34;
            if y < band && self.caption_up(p, t) {
                // under a caption: beside the rest glyph instead
                y = py::round_half_even(gy - 3.0) as i64;
                x = py::round_half_even(sx + rest_g.w as f64 / 2.0 + 4.0) as i64;
                let edge = if y + 7 > 290 { 300 } else { SAFE.2 as i64 };
                if x + txt_w > edge - 1 {
                    x = py::round_half_even(sx - rest_g.w as f64 / 2.0 - 4.0 - txt_w as f64) as i64;
                }
            }
            if SAFE.1 as i64 <= y && y <= SAFE.3 as i64 - 8 && !(y < band && self.caption_up(p, t)) {
                for j in 0..count {
                    let c = if j == count - 1 {
                        [255, 250, 236]
                    } else {
                        ink(self.col.map(|v| np::clip(v * 0.5 + 90.0, 0.0, 255.0)))
                    };
                    let dg = (j + 1).to_string();
                    label(
                        img,
                        log,
                        &dg,
                        (x + j * (adv + 2)) as i32,
                        y as i32,
                        c,
                        1,
                        Some([10, 20, 26]),
                    );
                }
            }
        }
        let mk = self.melody_k(p, t);
        let mut rising: Vec<f64> = self
            .note_ev
            .iter()
            .copied()
            .filter(|&te| {
                let a = py_min(t, p.t_ko) - te;
                (0.0..0.9).contains(&a) && mk > 0.0
            })
            .collect();
        if t >= p.t_hs && k == p.win {
            // the champion's solo: eighths on each note
            rising = p
                .solo_t
                .iter()
                .copied()
                .filter(|&s| 0.0 <= t - s && t - s < 0.9)
                .collect();
        }
        let eighth = glyph(&EIGHTH_GLYPH, 1);
        for (j, &te) in rising.iter().enumerate() {
            let age = (if t < p.t_hs { py_min(t, p.t_ko) } else { t }) - te;
            let side = if np::f64_to_i64(te * 10.0).rem_euclid(2) != 0 {
                1.0
            } else {
                -1.0
            };
            let x = sx + (8.0 + 3.0 * j as f64) * side + 4.0 * dmath::sin(age * 7.0);
            let y = sy - rp * 0.4 - 26.0 * age;
            let col = ink(self.col.map(|v| np::clip(v * 0.5 + 125.0, 0.0, 255.0))).map(f64::from);
            stamp_glyph(
                img,
                &eighth,
                x,
                y,
                col,
                1.0 - ss(0.5, 0.9, age),
                [16.0, 12.0, 26.0],
                false,
            );
        }
    }
}

/// What a note's painter item carries.
struct NoteItem(NoteView);

impl Look for ViolaLook {
    fn shards(&self) -> Vec<String> {
        // ['string'] * 4 + ['wood'] * 7 + ['bridge'] * 2 + ['scroll'] (after the 24 glass shards)
        let mut s = vec!["string".to_string(); 4];
        s.extend(vec!["wood".to_string(); 7]);
        s.extend(vec!["bridge".to_string(); 2]);
        s.push("scroll".to_string());
        s
    }

    fn spark_flash(&self) -> &[&str] {
        &["blade", "note"]
    }

    fn flash_hits(&self) -> &[&str] {
        &[]
    }

    fn spark_events(&self) -> &[&str] {
        &["crushed"]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.enemy = 1 - k;
        self.col = p.col[k];
        self.dark = p.dark[k];
        self.pp = cfg.pp.clone();
        // self.cfg['view']: (yaw centre, yaw swing, roll)
        let view = |i: usize| cfg.raw.get("view").and_then(|v| v.get(i)).and_then(|x| x.as_f64());
        self.view = (view(0).expect("its view"), view(2).expect("its view"));
        self.low_size = cfg.low.get("size").unwrap_or(0.0);
        let ev = &p.rec.events;
        self.stuns = ev
            .iter()
            .filter(|e| e.kind == "stun" && e.i(3) == k as i64)
            .map(|e| (e.t, e.i(2) as usize, e.f(4)))
            .collect();
        self.jokes = ev
            .iter()
            .filter(|e| e.kind == "joke" && e.i(2) == k as i64)
            .map(|e| (e.t, e.i(3) as usize, e.f(4), e.f(5), e.f(6)))
            .collect();
        self.melodies = ev
            .iter()
            .filter(|e| e.kind == "melody" && e.i(2) == k as i64)
            .map(|e| (e.t, e.f(3)))
            .collect();
        self.note_ev = ev
            .iter()
            .filter(|e| e.kind == "note" && e.i(8) == 0 && e.i(2) == k as i64)
            .map(|e| e.t)
            .collect();
        self.tracks = OnceLock::new();
    }

    fn cfg_summary(&self) -> Option<String> {
        Some(format!(
            "abilities.low.size={:?}; pp={}; view.0={:?}; view.2={:?}",
            self.low_size, self.pp, self.view.0, self.view.1
        ))
    }

    fn title_closeup(&self, r: u32) -> Option<Img<u8>> {
        Some(padded(r, self.view.0, self.view.1))
    }

    fn sprite(&self, r: u32, yaw: f64, roll: f64) -> Option<Img<u8>> {
        Some(padded(r, yaw, roll))
    }

    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if phase == Phase::Fight && kk == self.k {
            // MELODY AT LAST: a halo, pulsing on the beat
            st.glow = 0.45 * self.melody_k(p, t) * (0.6 + 0.4 * p.show.grid.beat_pulse(t, 0.2, 0.12));
        }
    }

    fn ball(&self, p: &PictureCtx, kk: usize, _st: &BallState, t: f64, glass: [f64; 3], roll: f64) -> ([f64; 3], f64) {
        let frz = self.frz(p, kk, t);
        if frz > 0.0 {
            let grey = [168.0, 176.0, 196.0];
            return (
                std::array::from_fn(|c| glass[c] * (1.0 - 0.3 * frz) + grey[c] * 0.3 * frz),
                roll,
            );
        }
        (glass, roll)
    }

    fn ball_tint(&self, p: &PictureCtx, kk: usize, _st: &BallState, t: f64, col: V3) -> V3 {
        let frz = self.frz(p, kk, t);
        if frz > 0.0 {
            let grey = [150.0, 150.0, 168.0];
            return std::array::from_fn(|c| col[c] * (1.0 - frz) + grey[c] * frz);
        }
        col
    }

    fn ball_lit<'a>(&'a self, p: &'a PictureCtx, kk: usize, _st: &BallState, t: f64) -> Option<Tint<'a>> {
        let frz = self.frz(p, kk, t);
        if frz <= 0.0 {
            return None;
        }
        Some(Box::new(move |lit: &mut Img<f32>| {
            let keep = (1.0 - 0.7 * frz) as f32;
            let add = [20.0, 24.0, 40.0];
            for px in lit.data.as_chunks_mut::<3>().0 {
                // g_ = lit.mean(-1, keepdims=True), in float32
                let g = ((px[0] + px[1]) + px[2]) / 3.0f32;
                let g85 = g * 0.85f32;
                for c in 0..3 {
                    let a = px[c] * keep;
                    px[c] = (f64::from(a) + (f64::from(g85) + add[c]) * 0.7 * frz) as f32;
                }
            }
        }))
    }

    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind != "hit" {
            return None;
        }
        let how = e.s(7);
        match how {
            "blade" | "wave" | "note" => {
                let big = how == "note" && e.i(8) == 2;
                Some(if big {
                    2.5
                } else {
                    match how {
                        "blade" => 0.4,
                        "wave" => 2.0,
                        _ => 1.3,
                    }
                })
            }
            _ => None,
        }
    }

    fn flash_hit(&self, e: &Event) -> Option<bool> {
        Some(matches!(e.s(7), "wave" | "note") && e.i(8) == 2)
    }

    fn crowd_laugh(&self, _p: &PictureCtx, t: f64) -> Option<f64> {
        Some(self.laugh_k(t))
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let how = e.s(7);
        let (n, sp, life) = if matches!(how, "wave" | "note") {
            (16, 3.2, 0.3)
        } else if how == "blade" {
            (8, 2.4, 0.22)
        } else {
            (5, 1.8, 0.18)
        };
        (p.col[e.i(2) as usize].map(|c| c * 0.6 + 100.0), n, sp, life)
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match e.s(7) {
            "wave" | "note" => Some((14.0, 0.35)),
            "blade" => Some((8.0, 0.22)),
            _ => None,
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        let t = e.t;
        let up = scene::to_world;
        if e.kind == "hit" && e.s(7) == "note" {
            // a whole note crushes (VIOLA JOKE's: the big one)
            let (att, x, z) = (e.i(2) as usize, e.f(5), e.f(6));
            let pos = up(x, z, R);
            let big = e.i(8) == 2;
            fx.bu.add(t, pos, p.col[att], if big { 2.0 } else { 1.1 }, None, true);
            let (rad, peak, decay) = if big { (2.6, 1.0, 0.45) } else { (1.6, 0.7, 0.28) };
            fx.fl.add(t, up(x, z, 0.4), p.col[att], rad, peak, 0.03, decay, 0.0);
            let dust = Emit {
                n: Some(if big { 26 } else { 10 }),
                spread: Some(180.0),
                speed: Some((0.6, 1.8)),
                scale: if big { 1.3 } else { 1.0 },
                ..Emit::default()
            };
            fx.pa.emit(t, up(x, z, 0.1), "dust", &dust);
            let chips = Emit {
                n: Some(if big { 8 } else { 4 }),
                col: Some(INKY.to_vec()),
                ..Emit::default()
            };
            fx.pa.emit(t, pos, "chips", &chips);
            fx.mk.add(
                t,
                Mark {
                    size: if big { 1.4 } else { 0.55 },
                    n: if big { 8 } else { 4 },
                    life: if big { 6.0 } else { 3.0 },
                    seed: seed_of(t * 100.0),
                    ..Mark::new(MarkKind::Crack, x, z)
                },
            );
            if big {
                fx.mk.add(
                    t,
                    Mark {
                        size: 1.2,
                        col: [40.0, 44.0, 64.0],
                        life: 6.0,
                        seed: seed_of(t * 10.0) + 1,
                        strength: 0.5,
                        ..Mark::new(MarkKind::Scorch, x, z)
                    },
                );
            }
        } else if e.kind == "note" {
            // a note appears: a flash at its spot, ink motes (a dropped one shows on landing)
            let (x, z) = (e.f(4), e.f(5));
            if e.f(9) <= t + 1e-6 {
                let k = e.i(2) as usize;
                fx.fl.add(t, up(x, z, 0.5), p.col[k], 1.3, 0.45, 0.03, 0.25, 0.0);
                let motes = Emit {
                    n: Some(5),
                    col: Some(vec![p.col[k].map(|c| c * 0.5 + 120.0)]),
                    ..Emit::default()
                };
                fx.pa.emit(t, up(x, z, 0.3), "motes", &motes);
            }
        } else if e.kind == "joke" {
            // the target glows while the note falls
            let (k, tl, x, z) = (e.i(2) as usize, e.f(4), e.f(5), e.f(6));
            fx.fl.add(
                t + 0.05,
                up(x, z, 2.0),
                p.col[k],
                2.2,
                0.6,
                0.03,
                py_max(0.3, tl - t),
                0.0,
            );
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let emit = Emit {
            n: Some(24),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            col: Some(vec![WOOD_COL, WOOD_COL.map(|c| c * 1.3)]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "chips", &emit);
    }

    fn shadows_free(&self, p: &PictureCtx, f: &FrameCtx<'_>, spot: V3) -> Vec<Vec<(f64, f64)>> {
        self.notes(p, f)
            .iter()
            .map(|n| proj_point(&note_shadow(f.cam, n.c, n.r * n.sc, n.phi, n.lift), spot, 0.93))
            .collect()
    }

    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if p.bold {
            return self.bold_canvas(p, ic, c);
        }
        // the resting viola's pool (it pulses on each count)
        let g = &p.show.grid;
        let t = c.t;
        let st = &c.fs[self.k];
        let (resting, _, _) = self.rest_state(p, t);
        let mk = self.melody_k(p, t);
        if !(c.in_fight && (resting || mk > 0.0)) {
            return;
        }
        let ph = py::fmod_py(t - g.t_fight, g.beat) / g.beat;
        let ring_r = R * (1.15 + 0.7 * ph);
        let tone: V3 = self.col.map(|v| v / 255.0 + 0.1);
        let kb = if resting { 0.6 } else { 0.4 };
        for (j, l) in ic.iter_mut().enumerate() {
            let rd = dmath::np_hypot(c.sxp[j] - st.p[0], c.szp[j] - st.p[1]);
            let band = np::clip(1.0 - (rd - ring_r).abs() / 0.06, 0.0, 1.0) * (1.0 - ph) * kb;
            let pool = np::clip(1.0 - rd / (R * 1.5), 0.0, 1.0) * 0.10 * (1.0 + mk);
            let v = band * 0.8 + pool;
            for ch in 0..3 {
                l[ch] += v * tone[ch];
            }
        }
    }

    fn canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if p.bold {
            return self.bold_canvas_after(p, ic, c);
        }
        let t = c.t;
        for &(t0, _, tl, x, z) in &self.jokes {
            if t0 <= t && t < tl && t < p.t_hs {
                // VIOLA JOKE: the target under the falling note
                let u = (t - t0) / (tl - t0);
                let rad = self.low_size * p.run.f("BL", p.sim_i(t0), self.k).expect("BL");
                let blink = if dmath::sin(t * 40.0) > 0.0 { 1.0 } else { 0.0 };
                let add: V3 = self.col.map(|v| (v / 255.0) * (0.4 + 0.5 * blink));
                for (j, l) in ic.iter_mut().enumerate() {
                    let rd = dmath::np_hypot(c.sxp[j] - x, c.szp[j] - z);
                    if rd < rad * (0.4 + 0.6 * u) {
                        *l = l.map(|v| v * (1.0 - 0.5 * u));
                    }
                    if (rd - rad).abs() < 0.05 {
                        for ch in 0..3 {
                            l[ch] += add[ch];
                        }
                    }
                }
            } else if 0.0 <= t - tl && t - tl < 0.6 && t < p.t_hs {
                let u = (t - tl) / 0.6;
                let tone = [0.6, 0.95, 0.9];
                for (j, l) in ic.iter_mut().enumerate() {
                    let rd = dmath::np_hypot(c.sxp[j] - x, c.szp[j] - z);
                    let band = np::clip(1.0 - (rd - (1.2 + 1.8 * u)).abs() / 0.2, 0.0, 1.0) * (1.0 - u);
                    for ch in 0..3 {
                        l[ch] += band * tone[ch];
                    }
                }
            }
        }
    }

    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        self.notes(p, f)
            .iter()
            .map(|n| {
                let d = f
                    .cam
                    .depth(&[scene::to_world(n.c[0], n.c[1], 0.5 * n.r * n.sc + n.lift)])[0];
                (d, Box::new(NoteItem(*n)) as Payload)
            })
            .collect()
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        if p.bold {
            return self.bold_draw_item(p, f, fr, payload);
        }
        let Some(NoteItem(n)) = payload.downcast_ref::<NoteItem>() else {
            return;
        };
        let glow = py_max(n.glow, 0.5 * self.melody_k(p, f.t));
        draw_note(fr, f.cam, f.l, n.c, n.r, n.phi, p.col[n.owner], n.lift, n.sc, glow);
    }

    #[allow(clippy::too_many_arguments)] // the Python signature
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
        let d = [dmath::cos(rot), dmath::sin(rot)];
        let nn = [-d[1], d[0]];
        let at = |q: [f64; 2]| scene::to_world(q[0], q[1], pos[1]);
        match kind {
            "string" => {
                // a string curling free (#28)
                let pts: Vec<V3> = (0..7)
                    .map(|j| {
                        let (jj, s) = (j as f64 - 3.0, dmath::sin(j as f64));
                        at([
                            sx + d[0] * jj * 0.16 + nn[0] * curl * 0.10 * s,
                            sz + d[1] * jj * 0.16 + nn[1] * curl * 0.10 * s,
                        ])
                    })
                    .collect();
                line_on_bytes(arr, &screen(cam, &pts), [230, 226, 214], 1);
            }
            "bridge" => {
                // the maple bridge, pale, with its two feet
                let q4: Vec<V3> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                    .iter()
                    .map(|&(a1, a2)| {
                        at([
                            sx + d[0] * 0.12 * a1 + nn[0] * 0.05 * a2,
                            sz + d[1] * 0.12 * a1 + nn[1] * 0.05 * a2,
                        ])
                    })
                    .collect();
                layer_blend(arr, &[screen(cam, &q4)], [228.0, 198.0, 148.0], 1.0, None);
            }
            "scroll" => {
                // the scroll: a small varnished spiral
                let pts: Vec<V3> = (0..8)
                    .map(|j| {
                        let rr = 0.13 - 0.016 * j as f64;
                        let a = rot + j as f64 * 0.9;
                        at([sx + rr * dmath::cos(a), sz + rr * dmath::sin(a)])
                    })
                    .collect();
                line_on_bytes(arr, &screen(cam, &pts), ink(WOOD_COL.map(|v| v * 1.2)), 2);
            }
            "wood" => {
                // tumbling flat chips in its varnish
                let tri = [
                    at([sx + d[0] * size, sz + d[1] * size]),
                    at([
                        sx - d[0] * size * 0.7 + nn[0] * size * 0.8,
                        sz - d[1] * size * 0.7 + nn[1] * size * 0.8,
                    ]),
                    at([
                        sx - d[0] * size * 0.4 - nn[0] * size * 0.7,
                        sz - d[1] * size * 0.4 - nn[1] * size * 0.7,
                    ]),
                ];
                let k = 0.75 + 0.25 * dmath::cos(rot * 2.0);
                let col = WOOD_COL.map(|v| np::clip(v * k * 1.2, 0.0, 255.0));
                layer_blend(arr, &[screen(cam, &tri)], col, 1.0, None);
            }
            _ => {}
        }
    }

    fn shimmer_after(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64) {
        for &(_, _, tl, x, z) in &self.jokes {
            if 0.0 <= t - tl && t - tl < 0.6 && t < p.t_hs {
                // the big note's landing
                let u = (t - tl) / 0.6;
                let c = scene::to_world(x, z, 0.05);
                match img {
                    Pic::F32(i) => shimmer_world(i, cam, c, 1.2 + 1.8 * u, 0.35, 3.0, t, 1.0 - u, 0.0),
                    Pic::F64(i) => shimmer_world(i, cam, c, 1.2 + 1.8 * u, 0.35, 3.0, t, 1.0 - u, 0.0),
                }
            }
        }
    }

    fn glows(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, _drawn: &[DrawnBall]) {
        for n in self.notes(p, f) {
            // a soft glow round each rolling note
            let s = f.cam.wp(&[scene::to_world(n.c[0], n.c[1], 0.5 * n.r * n.sc + n.lift)])[0];
            let col = p.col[n.owner].map(|v| v * 0.7 + 60.0);
            fr.glow(s.0, s.1, 10.0 + 14.0 * n.r * n.sc, col, 0.10 + 0.25 * n.glow);
        }
    }

    fn fringe(&self, _p: &PictureCtx, e: &Event) -> bool {
        let how = e.s(7);
        how == "wave" || (how == "note" && e.i(8) == 2)
    }

    fn after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if p.bold {
            self.bold_after_grade(p, img, f, drawn);
        }
    }

    fn bold_status(&self, p: &PictureCtx, t: f64) -> Vec<bf_picture::bold::Status> {
        self.bold_statuses(p, t)
    }

    fn bold_hit_size(
        &self,
        _p: &PictureCtx,
        e: &Event,
        by_damage: bf_picture::bold::HitSize,
    ) -> Option<bf_picture::bold::HitSize> {
        Self::bold_hit(e, by_damage)
    }

    fn after_post(&self, p: &PictureCtx, img: &mut Img<u8>, log: &mut DrawLog, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if p.bold {
            return self.bold_after_post(p, img, log, f, drawn);
        }
        if title::door_offset(&p.show.grid, f.t) != 0.0 {
            self.draw_rests(p, img, log, f.t, drawn);
        }
    }
}

/// `int(x)` of a float made a seed (`seed=int(t_ * 100)`).
fn seed_of(x: f64) -> u64 {
    np::f64_to_i64(x).max(0) as u64
}

/// `render_padded(name, r, yaw, roll)` for the viola's model: `m = int(math.ceil(r * 1.2)) + 3`, a sprite of side
/// `2 m + 1` (the shared renderer's half-side `ceil(1.2 r)`).
fn padded(r: u32, yaw: f64, roll: f64) -> Img<u8> {
    let half = (f64::from(r) * 1.2).ceil();
    render_sprite_padded(&model(), r, yaw, roll, half as u32)
}

/// `Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))`, a Pillow line, and back to `float32`: the whole frame
/// goes through bytes, as the Python does.
fn line_on_bytes(arr: &mut Img<f32>, pts: &[(f64, f64)], fill: [u8; 3], width: u32) {
    let mut bytes: Img<u8> = Img::new(arr.w, arr.h, 3);
    for (b, &v) in bytes.data.iter_mut().zip(&arr.data) {
        *b = np::clip(f64::from(v), 0.0, 255.0) as u8;
    }
    pil::line(&mut bytes, pts, &fill, width);
    for (v, &b) in arr.data.iter_mut().zip(&bytes.data) {
        *v = f32::from(b);
    }
}

/// The VIOLA's view in the kit's roster `(yaw centre, roll)` (its `view` is `(34, 16, 24)`), for a look that was never
/// set up (the VS card's close-up is asked of one); `setup_as` replaces it with the numbers it is given.
const VIEW: (f64, f64) = (34.0, 24.0);

/// VIOLA's look, ready for the ring.
pub fn look() -> Box<dyn Look> {
    Box::new(ViolaLook {
        view: VIEW,
        ..ViolaLook::default()
    })
}
