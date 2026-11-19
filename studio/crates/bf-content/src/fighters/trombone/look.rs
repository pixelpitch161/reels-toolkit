//! The TROMBONE's look: the port of `Look` in `projects/ballfight/kit/fighters/trombone.py`, hook by hook.
//!
//! What it draws: the brass slide lance (two tubes, the thicker outer slide, the braces, the crook) with the
//! streak of a thrust going out and its lit lane on the canvas; the crook's flare as the lance strikes; SAD
//! TROMBONE's ring on the canvas and in the air, and the slowed enemy's droopy wobble; PEDAL TONE's target, its
//! giant hovering bell and the stomp; brass sparks and embers; the winner's solo played on the slide (a lower
//! note is a longer slide); brass shards when it loses.
//!
//! The look has no hook for the VS title card: its panel pattern is the card's own (`_slide_tubes`, found by
//! the fighter's name), its close-up is its model and its glass the base class's.
//!
//! How close to Python (`studio/reference/picture_looks/trombone_frames.json`, the test
//! `tests/trombone_look.rs`): every frame compared is Python's, byte for byte. The arithmetic keeps Python's
//! order and dtypes; the sines, cosines, the arctangent and the exponential are `bf_core::dmath`'s where Python
//! has the C runtime's and numpy's, and the dot products are plain sums where numpy's are fused, so a value can
//! differ from Python's in its last bit before it is cut to a pixel or a colour step.

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{BAYER, FPS, ease, ss};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::Img;
use bf_picture::lighting::{Lights, shade};
use bf_picture::look::{CanvasCtx, DefaultLook, DrawnBall, FrameCtx, Fx, Look, Payload, Phase, Pic, Spark};
use bf_picture::raster::Frame;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::shadows::proj_point;
use bf_picture::{np, pil};

/// `BRASS`.
const BRASS: V3 = [232.0, 188.0, 92.0];
/// `BRASS_HI`.
const BRASS_HI: V3 = [255.0, 238.0, 176.0];
/// `BRASS_DK`.
const BRASS_DK: V3 = [150.0, 104.0, 40.0];
/// `TUBE_GAP`: each tube's distance from the slide's middle line.
const TUBE_GAP: f64 = 0.075;
/// `TUBE_W`: the width of an inner tube.
const TUBE_W: f64 = 0.034;
/// `OUTER_W`: the width of the outer slide and of the crook.
const OUTER_W: f64 = 0.05;
/// `BELL_PROFILE`: `(height above the rim, radius / rim radius)`: a flared bell, mouth down.
const BELL_PROFILE: [(f64, f64); 8] = [
    (0.0, 1.0),
    (0.04, 0.93),
    (0.13, 0.74),
    (0.28, 0.54),
    (0.48, 0.40),
    (0.78, 0.30),
    (1.15, 0.24),
    (1.6, 0.21),
];
/// `nseg` of `draw_giant_bell`: the bell's segments round its axis.
const BELL_SEG: usize = 18;

// ---------------------------------------------------------------- small vectors, as numpy computes them
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

/// `np.dot(a, b)` as the plain sum (numpy's is fused: the last bit can differ).
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// `np.cross(a, b)`.
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// `Q.mean(0)`: the rows added in order, then divided by their number.
fn mean(rows: &[V3]) -> V3 {
    let mut s = [0.0; 3];
    for r in rows {
        s = add(s, *r);
    }
    let n = rows.len() as f64;
    [s[0] / n, s[1] / n, s[2] / n]
}

/// `np.array([math.cos(th), math.sin(th)])`.
fn unit(th: f64) -> [f64; 2] {
    [dmath::cos(th), dmath::sin(th)]
}

/// `tuple(int(v) for v in col)` as a Pillow ink.
fn ink(col: V3) -> [u8; 3] {
    [col[0] as u8, col[1] as u8, col[2] as u8]
}

/// A numpy boolean as a factor: `x * (a > b)`.
fn gate(on: bool) -> f64 {
    if on { 1.0 } else { 0.0 }
}

/// `np.linspace(a, b, n)`: `i * step + a`, the last value `b` itself.
fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    let step = (b - a) / (n - 1) as f64;
    (0..n)
        .map(|i| if i == n - 1 { b } else { i as f64 * step + a })
        .collect()
}

/// `BAYER_FULL[y, x]`.
fn bayer(x: usize, y: usize) -> f64 {
    BAYER[y % 4][x % 4]
}

/// `times[np.searchsorted(times, t, 'right') - 1]` when that index is not negative: the last of the ascending
/// `times` that is not after `t` (a time equal to `t` counts).
fn last_not_after(times: &[f64], t: f64) -> Option<f64> {
    let after = times.partition_point(|&v| v <= t);
    after.checked_sub(1).map(|j| times[j])
}

/// `e[7]` of a hit (what hit), or `""` for an event without one.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// A number of the recording's ragged arrays.
fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Float(x) => Some(*x),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

/// `(e[3], e[4])` of a `slam` event: where the bell lands. `None` for a `slam` without the two numbers (the
/// renderer checks the events it reads itself, and a `slam` is this look's own): such an event is left out
/// where Python raises.
fn centre(e: &Event) -> Option<(f64, f64)> {
    Some((num(e.fields.get(1)?)?, num(e.fields.get(2)?)?))
}

// ---------------------------------------------------------------- the slide lance
/// The `kind` of a quad of the slide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Tube,
    Outer,
    Brace,
    Crook,
}

/// One quad of `slide_geom`: `(Q, colour, kind)`.
struct Quad {
    q: [V3; 4],
    col: V3,
    part: Part,
}

/// `slide_geom(p, th, ext, y=R)`: the world quads of the slide from inside the ball to the crook at `R + ext`,
/// and the tip.
fn slide_geom(p: [f64; 2], th: f64, ext: f64) -> (Vec<Quad>, V3) {
    let y = R;
    let d = unit(th);
    let n = [-d[1], d[0]];
    let (r0, r1) = (R * 0.8, R + ext);
    // Wp = lambda r, w: S.to_world(*(p + d * r + n * w), y)
    let wp = |r: f64, w: f64| scene::to_world(p[0] + d[0] * r + n[0] * w, p[1] + d[1] * r + n[1] * w, y);
    let mut out = Vec::with_capacity(14);
    // the outer slide (thicker) rides on the inner tubes
    let r_out = py_max(r0 + 0.08, r1 - py_max(0.35, 0.62 * (r1 - r0)));
    for side in [-1.0, 1.0] {
        let c = side * TUBE_GAP;
        out.push(Quad {
            q: [
                wp(r0, c - TUBE_W / 2.0),
                wp(r1, c - TUBE_W / 2.0),
                wp(r1, c + TUBE_W / 2.0),
                wp(r0, c + TUBE_W / 2.0),
            ],
            col: BRASS,
            part: Part::Tube,
        });
        out.push(Quad {
            q: [
                wp(r_out, c - OUTER_W / 2.0),
                wp(r1 - 0.02, c - OUTER_W / 2.0),
                wp(r1 - 0.02, c + OUTER_W / 2.0),
                wp(r_out, c + OUTER_W / 2.0),
            ],
            col: scale(BRASS, 1.04),
            part: Part::Outer,
        });
    }
    // the braces across the two tubes
    for rb in [r0 + 0.14, r_out + 0.06] {
        if rb < r1 - 0.1 {
            out.push(Quad {
                q: [
                    wp(rb - 0.025, -TUBE_GAP),
                    wp(rb + 0.025, -TUBE_GAP),
                    wp(rb + 0.025, TUBE_GAP),
                    wp(rb - 0.025, TUBE_GAP),
                ],
                col: scale(BRASS_DK, 1.2),
                part: Part::Brace,
            });
        }
    }
    // the crook: a U-bend beyond the tube ends (8 segments)
    let a = linspace(-dmath::PI / 2.0, dmath::PI / 2.0, 9);
    let (wide, narrow) = (TUBE_GAP + OUTER_W / 2.0, TUBE_GAP - OUTER_W / 2.0);
    for j in 0..8 {
        let corner = |aa: f64, rr: f64| wp(r1 + rr * dmath::cos(aa) * 0.9, rr * dmath::sin(aa));
        out.push(Quad {
            q: [
                corner(a[j], wide),
                corner(a[j + 1], wide),
                corner(a[j + 1], narrow),
                corner(a[j], narrow),
            ],
            col: BRASS,
            part: Part::Crook,
        });
    }
    let reach = r1 + TUBE_GAP;
    let tip = scene::to_world(p[0] + d[0] * reach, p[1] + d[1] * reach, y);
    (out, tip)
}

/// `draw_slide(fr, cam, L, shade, st, glow_col, glow)`: the slide's quads, flat-shaded (the outer slide and the
/// crook glowing in `glow_col`), then a glint line along each outer slide.
fn draw_slide(fr: &mut Frame, cam: &Cam, l: &Lights, st: &BallState, glow_col: V3, glow: f64) {
    let (quads, _tip) = slide_geom(st.p, st.th, st.ext);
    let up_n = [0.0, 1.0, 0.0];
    for quad in &quads {
        let mut c = shade(l, &[up_n], &[mean(&quad.q)], &[quad.col], None, 0.45, 0.0, 5.0)[0];
        if glow > 0.0 && matches!(quad.part, Part::Outer | Part::Crook) {
            // c = c * (1 - 0.45 * glow) + np.asarray(glow_col) * 0.45 * glow + 30 * glow
            for ch in 0..3 {
                c[ch] = c[ch] * (1.0 - 0.45 * glow) + glow_col[ch] * 0.45 * glow + 30.0 * glow;
            }
        }
        fr.poly(cam, &quad.q, c, true);
    }
    // a glint line along each outer slide (round tubes)
    for quad in quads.iter().filter(|q| q.part == Part::Outer) {
        let s = cam.wp(&quad.q);
        let a = ((s[0].0 + s[3].0) / 2.0, (s[0].1 + s[3].1) / 2.0);
        let b = ((s[1].0 + s[2].0) / 2.0, (s[1].1 + s[2].1) / 2.0);
        pil::line(&mut fr.pil, &[a, b], &ink(BRASS_HI), 1);
    }
}

/// `slide_trail_polys(cam, st, ext0)`: the streak left by a fast thrust: a soft band from the previous tip
/// position to the current one.
fn slide_trail_polys(cam: &Cam, st: &BallState, ext0: f64) -> Vec<Vec<(f64, f64)>> {
    let d = unit(st.th);
    let n = [-d[1], d[0]];
    let p = st.p;
    let (a, b) = (R + ext0, R + st.ext + TUBE_GAP);
    if b - a < 0.05 {
        return Vec::new();
    }
    let w = TUBE_GAP + OUTER_W;
    // pts = [p + d * a + n * w * 0.6, p + d * b + n * w, p + d * b - n * w, p + d * a - n * w * 0.6]
    let pts = [
        [p[0] + d[0] * a + n[0] * w * 0.6, p[1] + d[1] * a + n[1] * w * 0.6],
        [p[0] + d[0] * b + n[0] * w, p[1] + d[1] * b + n[1] * w],
        [p[0] + d[0] * b - n[0] * w, p[1] + d[1] * b - n[1] * w],
        [p[0] + d[0] * a - n[0] * w * 0.6, p[1] + d[1] * a - n[1] * w * 0.6],
    ];
    let world: Vec<V3> = pts.iter().map(|q| scene::to_world(q[0], q[1], R)).collect();
    vec![cam.wp(&world)]
}

// ---------------------------------------------------------------- PEDAL TONE's giant bell
/// `bell_height(t, t0, t_slam)`: the bell drops in from high above, hovers (bobbing with the held pedal note)
/// during the telegraph, slams down in the last 0.12 s, sits on the canvas for a moment, then flies back up out
/// of the frame. `None` before and after.
fn bell_height(t: f64, t0: f64, t_slam: f64) -> Option<f64> {
    if t < t0 {
        return None;
    }
    if t < t_slam - 0.12 {
        let u = py_min(1.0, (t - t0) / 0.3);
        return Some(3.2 + 5.0 * ((1.0 - u) * (1.0 - u)) + 0.08 * dmath::sin(2.0 * dmath::PI * 3.0 * (t - t0)));
    }
    if t < t_slam {
        let u = (t - (t_slam - 0.12)) / 0.12;
        return Some(3.2 - 3.1 * u * u);
    }
    if t < t_slam + 0.3 {
        return Some(0.10);
    }
    if t < t_slam + 0.75 {
        let u = (t - t_slam - 0.3) / 0.45;
        return Some(0.10 + 8.0 * (u * u));
    }
    None
}

/// `draw_giant_bell(fr, cam, L, shade, cx, cz, rad, h, col)`: mouth down: a flat-shaded low-poly surface of
/// revolution (its quads painter-sorted), a bright lip, and the leadpipe rising out of the frame.
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_giant_bell(fr: &mut Frame, cam: &Cam, l: &Lights, cx: f64, cz: f64, rad: f64, h: f64, col: V3) {
    let rim = rad * 1.05;
    let a = linspace(0.0, 2.0 * dmath::PI, BELL_SEG + 1);
    let rings: Vec<Vec<V3>> = BELL_PROFILE
        .iter()
        .map(|&(dy, rr)| {
            a.iter()
                .map(|&u| scene::to_world(cx + rim * rr * dmath::cos(u), cz + rim * rr * dmath::sin(u), h + dy))
                .collect()
        })
        .collect();
    // dark enough that the spot, the attack lights and the bloom land it in gold
    let base: V3 = std::array::from_fn(|ch| BRASS[ch] * 0.42 + col[ch] * 0.06);
    let mut quads: Vec<(f64, [V3; 4], V3, usize)> = Vec::new();
    for j in 0..rings.len() - 1 {
        let (lo, hi) = (&rings[j], &rings[j + 1]);
        for i in 0..BELL_SEG {
            let q = [lo[i], lo[i + 1], hi[i + 1], hi[i]];
            let mut nrm = cross(sub(q[1], q[0]), sub(q[3], q[0]));
            // nrm = nrm / (np.linalg.norm(nrm) + 1e-9)
            let len = dot(nrm, nrm).sqrt() + 1e-9;
            nrm = [nrm[0] / len, nrm[1] / len, nrm[2] / len];
            if nrm[1] < 0.0 {
                nrm = [-nrm[0], -nrm[1], -nrm[2]];
            }
            let ctr = mean(&q);
            // if np.dot(nrm, cam.pos - Q.mean(0)) <= 0: continue
            if dot(nrm, sub(cam.pos, ctr)) <= 0.0 {
                continue;
            }
            quads.push((cam.depth(&[ctr])[0], q, nrm, j));
        }
    }
    // quads.sort(key=lambda q: -q[0]): a stable sort, far to near
    quads.sort_by(|x, y| (-x.0 + 0.0).total_cmp(&(-y.0 + 0.0)));
    for (_, q, nrm, j) in &quads {
        let shade_k = 0.92 + 0.03 * *j as f64;
        let c = shade(l, &[*nrm], &[mean(q)], &[scale(base, shade_k)], None, 0.6, 0.0, 5.0)[0];
        fr.poly(cam, q, c, true);
    }
    let s = cam.wp(&rings[0]);
    pil::line(&mut fr.pil, &s, &ink(BRASS_HI), 1);
    let last = BELL_PROFILE[BELL_PROFILE.len() - 1];
    let top = scene::to_world(cx, cz, h + last.0);
    let pipe = scene::to_world(cx + 0.6, cz - 0.6, h + 6.0);
    let pw = rim * last.1 * 0.7;
    // side = np.array([pw, 0.0, pw]) * 0.7
    let side = [pw * 0.7, 0.0, pw * 0.7];
    let q = [sub(top, side), add(top, side), add(pipe, side), sub(pipe, side)];
    let c = shade(l, &[[0.3, 0.6, -0.7]], &[mean(&q)], &[base], None, 0.6, 0.0, 5.0)[0];
    fr.poly(cam, &q, c, true);
}

/// One row of `RUN['SLAMS'][i]`: a PEDAL TONE in the air or just landed.
#[derive(Clone, Copy, Debug)]
struct Slam {
    owner: i64,
    cx: f64,
    cz: f64,
    t0: f64,
    t_slam: f64,
    rad: f64,
}

/// `RUN['SLAMS'][i]`: the rows of recording frame `i` (none when the recording has no `SLAMS`, where Python
/// raises; a row that is not `[owner, x, z, start, slam, radius]` is left out).
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

// ---------------------------------------------------------------- the look
/// The TROMBONE's look: `Look` of `kit/fighters/trombone.py`.
#[derive(Clone, Debug, Default)]
pub struct TromboneLook {
    /// Its side.
    k: usize,
    /// `THR_T`: the times of its thrusts.
    thr_t: Vec<f64>,
    /// `solo_n`: its solo as `(beat, midi note, beats)`.
    solo_n: Vec<(f64, i32, f64)>,
    /// `solo_lo`: the solo's lowest note (0 without a solo).
    solo_lo: i32,
    /// `solo_hi`: the solo's highest note (1 without a solo).
    solo_hi: i32,
    /// `self.cfg['abilities']['low']['radius']`: PEDAL TONE's radius, `None` for a low ability without one.
    low_radius: Option<f64>,
}

impl TromboneLook {
    /// `solo_ext(t)`: the winning trombone plays its solo with the slide: a lower note is a longer slide (real
    /// slide positions run from 1st, closed, to 7th, out), gliding to each note's position as it starts; closed
    /// between phrases.
    fn solo_ext(&self, p: &PictureCtx, t: f64) -> f64 {
        let k = self.k;
        let rest = p.rest[k];
        let reach = p.run.f("BL", p.run.last(), k).unwrap_or(0.0);
        let beat = p.show.grid.beat;
        let past: Vec<(f64, i32)> = self
            .solo_n
            .iter()
            .map(|&(b, m, _)| (p.t_solo + b * beat, m))
            .filter(|&(ts, _)| ts <= t)
            .collect();
        let Some(&(t0, m)) = past.last() else { return rest };
        let (lo, hi) = (self.solo_lo, self.solo_hi);
        let span = f64::from((hi - lo).max(1));
        let pos = 1.0 - f64::from(m - lo) / span;
        let tgt = rest + 0.25 + (reach - rest - 0.25) * pos;
        let p0 = if past.len() > 1 {
            let prev = past[past.len() - 2];
            rest + 0.25 + (reach - rest - 0.25) * (1.0 - f64::from(prev.1 - lo) / span)
        } else {
            rest
        };
        let u = ease(py_min(1.0, (t - t0) / 0.07));
        p0 + (tgt - p0) * u
    }

    /// `bell_down(t)`: its bells of the recording frame at `t`.
    fn bell_down(&self, p: &PictureCtx, t: f64) -> Vec<Slam> {
        if !(p.show.grid.t_fight <= t && t < p.t_hs) {
            return Vec::new();
        }
        let mut rows = slams(p, p.sim_i(t));
        rows.retain(|s| s.owner == self.k as i64);
        rows
    }
}

impl Look for TromboneLook {
    fn shards(&self) -> Vec<String> {
        // ['slidebit'] * 4 + ['tube'] * 4 + ['bellbit'] * 6
        [("slidebit", 4), ("tube", 4), ("bellbit", 6)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_flash(&self) -> &[&str] {
        &["blade", "slam"]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`: the one number `trombone.py` reads from `self.cfg`
    /// (PEDAL TONE's radius) comes from `cfg`, the rest from side `k`. For its own side `cfg` is `p.cfg[k]`, and
    /// this is `setup`.
    /// PEDAL TONE's radius: what `setup_as` took from `cfg`.
    fn cfg_summary(&self) -> Option<String> {
        Some(format!("abilities.low.radius={:?}", self.low_radius))
    }

    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.low_radius = cfg.low.get("radius");
        self.thr_t = p
            .rec
            .events
            .iter()
            .filter(|e| e.kind == "thrust" && e.who() == Some(k))
            .map(|e| e.t)
            .collect();
        self.solo_n = p.solos[k].clone();
        // min((m for b, m, d in self.solo_n), default=0), max(..., default=1)
        self.solo_lo = self.solo_n.iter().map(|n| n.1).min().unwrap_or(0);
        self.solo_hi = self.solo_n.iter().map(|n| n.1).max().unwrap_or(1);
    }

    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        let k = self.k;
        if kk != k {
            return;
        }
        if phase == Phase::Fight && !self.thr_t.is_empty() {
            // the trombone ball thumps on every thrust
            let now = py_min(t, p.t_ko);
            // j = np.searchsorted(self.THR_T, min(t, P.T_KO), 'right') - 1; if j >= 0: ...
            if let Some(thrust) = last_not_after(&self.thr_t, now) {
                st.thr_k = dmath::exp(-(now - thrust) / 0.1);
                st.glow = 0.35 * st.thr_k;
            }
        } else if phase == Phase::Tail && k == p.win {
            // the solo on the slide, closing for the glide home
            let g = &p.show.grid;
            let rest = p.rest[k];
            // (a recording of a lance always has EXT; without it the slide is taken to be at rest)
            let e_ko = p.run.f("EXT", p.run.last(), k).unwrap_or(rest);
            let e = if t < p.t_solo {
                e_ko + (rest - e_ko) * ss(p.t_hs, p.t_hs + 0.4, t)
            } else {
                self.solo_ext(p, t)
            };
            st.ext = e + (rest - e) * ss(g.t_reassemble - 0.3, g.t_reassemble, t);
        }
    }

    /// Trapped under PEDAL TONE's bell.
    fn hides_hp(&self, p: &PictureCtx, kk: usize, t: f64) -> bool {
        if !(p.show.grid.t_fight <= t && t < p.t_hs) {
            return false;
        }
        let at = p.fighter(kk, t).p;
        self.bell_down(p, t).iter().any(|s| {
            bell_height(t, s.t0, s.t_slam)
                .is_some_and(|h| h < 1.6 && dmath::hypot(at[0] - s.cx, at[1] - s.cz) < s.rad * 1.05)
        })
    }

    // ------------------------------------------------------------ events
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        match how(e) {
            "blade" => return Some(0.55),
            "slam" => return Some(2.5),
            "shockwave" => return Some(1.2),
            _ => {}
        }
        if e.kind == "swat" {
            return Some(0.4);
        }
        DefaultLook.shake(p, e)
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        if how(e) == "blade" {
            // self.P.COL[e[2]] * 0.6 + 100, 8, 2.4, 0.22
            let col = p.col[e.who().filter(|&k| k < 2).unwrap_or(0)];
            return (col.map(|c| c * 0.6 + 100.0), 8, 2.4, 0.22);
        }
        // (the base class's spray; this look has the base class's spark_big)
        DefaultLook.spark(p, e)
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "slam" | "shockwave" => Some((14.0, 0.35)),
            "blade" => Some((8.0, 0.22)),
            _ => None,
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        let t = e.t;
        let Some(att) = e.who().filter(|&k| k < 2) else { return };
        if e.kind == "hit" {
            let pos = up(e.f(5), e.f(6), R);
            match how(e) {
                "blade" => {
                    // the slide lance lands: brass sparks
                    fx.bu.add(t, pos, p.col[att], 0.8, None, true);
                    fx.fl.add(t, pos, p.fl_col[att], 1.4, 0.6, 0.03, 0.22, 0.0);
                    let th = p.fighter(att, t).th;
                    let d = up(dmath::cos(th), dmath::sin(th), 0.0);
                    let emit = Emit {
                        n: Some(9),
                        dir: [d[0] + 0.0, d[1] + 0.4, d[2] + 0.0],
                        ..Emit::default()
                    };
                    fx.pa.emit(t, pos, "brass", &emit);
                }
                "shockwave" => {
                    // SAD TROMBONE
                    fx.bu.add(t, pos, p.col[att], 1.0, None, true);
                    fx.fl.add(t, pos, p.fl_col[att], 1.6, 0.8, 0.03, 0.3, 0.0);
                    let emit = Emit {
                        n: Some(6),
                        ..Emit::default()
                    };
                    fx.pa.emit(t, pos, "brass", &emit);
                }
                "slam" => fx.bu.add(t, pos, p.col[att], 2.0, None, true),
                _ => {}
            }
        } else if e.kind == "slam" {
            // PEDAL TONE: the bell stomps
            let Some((cx, cz)) = centre(e) else { return };
            // low: under the bell's rim
            fx.fl.add(t, up(cx, cz, 0.15), p.fl_col[att], 2.4, 0.9, 0.03, 0.5, 0.0);
            let dust = Emit {
                n: Some(30),
                spread: Some(180.0),
                speed: Some((0.6, 1.8)),
                scale: 1.3,
                ..Emit::default()
            };
            fx.pa.emit(t, up(cx, cz, 0.06), "dust", &dust);
            let brass = Emit {
                n: Some(14),
                spread: Some(180.0),
                ..Emit::default()
            };
            fx.pa.emit(t, up(cx, cz, 0.2), "brass", &brass);
            // seed=int(t_ * 10)
            let seed = np::f64_to_i64(t * 10.0).max(0) as u64;
            fx.mk.add(
                t,
                Mark {
                    size: 1.5,
                    n: 8,
                    life: 7.0,
                    seed,
                    ..Mark::new(MarkKind::Crack, cx, cz)
                },
            );
            fx.mk.add(
                t,
                Mark {
                    size: 1.2,
                    col: [70.0, 60.0, 30.0],
                    life: 7.0,
                    seed: seed + 1,
                    strength: 0.5,
                    ..Mark::new(MarkKind::Scorch, cx, cz)
                },
            );
        }
    }

    fn fx_after(&self, p: &PictureCtx, fx: &mut Fx) {
        // PEDAL TONE's bell hovers mint-lit
        let ev = &p.rec.events;
        for e in ev {
            let slam_call = e.kind == "ability" && matches!(e.fields.get(2), Some(Value::Str(s)) if s == "slam");
            if slam_call && e.who() == Some(self.k) && e.t < p.t_ko {
                let first = ev
                    .iter()
                    .find(|q| q.kind == "slam" && q.fields.first() == e.fields.first() && q.t >= e.t);
                if let Some((sl, (cx, cz))) = first.and_then(|sl| Some((sl, centre(sl)?))) {
                    fx.fl.add(
                        e.t + 0.15,
                        up(cx, cz, 2.4),
                        p.fl_col[self.k],
                        2.6,
                        0.65,
                        0.03,
                        py_max(0.3, sl.t - e.t),
                        0.0,
                    );
                }
            }
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let emit = Emit {
            n: Some(30),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "brass", &emit);
    }

    // ------------------------------------------------------------ canvas
    fn shadows(&self, _p: &PictureCtx, st: &BallState, spot: V3) -> Vec<Vec<(f64, f64)>> {
        if st.shattered {
            return Vec::new();
        }
        let (quads, _) = slide_geom(st.p, st.th, st.ext);
        quads
            .iter()
            .filter(|q| matches!(q.part, Part::Outer | Part::Crook))
            .map(|q| proj_point(&q.q, spot, 0.93))
            .collect()
    }

    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let k = self.k;
        let col = p.col[k];
        // SAD TROMBONE: the slowed enemy drags a droopy wobble
        let st = &c.fs[k ^ 1];
        if c.in_fight && st.slow > 0.0 {
            let sk = py_min(1.0, st.slow / 0.25);
            let tint = col.map(|v| v / 255.0 + 0.15);
            for (j, light) in ic.iter_mut().enumerate() {
                let (dx, dz) = (c.sxp[j] - st.p[0], c.szp[j] - st.p[1]);
                let rd = np::hypot(dx, dz);
                let ang = dmath::atan2(dz, dx);
                for (jj, rr) in [(0.0, R * 1.25), (1.0, R * 1.6)] {
                    let wob = rr + 0.07 * dmath::sin(5.0 * ang - 11.0 * c.t + jj * 2.0);
                    let band = np::clip(1.0 - (rd - wob).abs() / 0.05, 0.0, 1.0);
                    let v = band * 0.55 * sk * (1.0 - 0.35 * jj);
                    for ch in 0..3 {
                        light[ch] += v * tint[ch];
                    }
                }
            }
        }
        // the lance's line of attack, lit as it thrusts
        let st = &c.fs[k];
        if c.in_fight && st.thr_k > 0.05 {
            let d = unit(st.th);
            let tint = col.map(|v| v / 255.0 * 0.8 + 0.2);
            let reach = R + st.ext + 0.1;
            for (j, light) in ic.iter_mut().enumerate() {
                let (rx, rz) = (c.sxp[j] - st.p[0], c.szp[j] - st.p[1]);
                let along = rx * d[0] + rz * d[1];
                let across = (-rx * d[1] + rz * d[0]).abs();
                let lane = np::clip(1.0 - across / 0.22, 0.0, 1.0) * gate(along > R) * gate(along < reach);
                // lane = np.floor(lane * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
                let lane = (lane * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
                let v = lane * 0.30 * st.thr_k;
                for ch in 0..3 {
                    light[ch] += v * tint[ch];
                }
            }
        }
    }

    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind != "shockwave" {
            return;
        }
        // SAD TROMBONE's ring
        let k_ = (1.0 - w.r / w.rmax) * 1.0;
        let tint = p.col[w.owner].map(|v| v / 255.0 * 1.4 + 0.15);
        for (j, light) in ic.iter_mut().enumerate() {
            let rd = np::hypot(c.sxp[j] - w.cx, c.szp[j] - w.cz);
            let band = np::clip(1.0 - (rd - w.r).abs() / 0.16, 0.0, 1.0);
            let band = (band * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
            let v = band * k_ * 0.9;
            for ch in 0..3 {
                light[ch] += v * tint[ch];
            }
        }
    }

    fn canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        // PEDAL TONE's target and stomp ring
        for s in slams(p, c.i) {
            if s.owner != self.k as i64 {
                continue;
            }
            if c.t < s.t_slam {
                let u = (c.t - s.t0) / (s.t_slam - s.t0);
                let dim = 1.0 - 0.55 * u;
                // (P.COL[owner] / 255.0) * (0.4 + 0.5 * (math.sin(C.t * 40) > 0))
                let blink = 0.4 + 0.5 * gate(dmath::sin(c.t * 40.0) > 0.0);
                let ring = p.col[self.k].map(|v| v / 255.0 * blink);
                for (j, light) in ic.iter_mut().enumerate() {
                    let rd = np::hypot(c.sxp[j] - s.cx, c.szp[j] - s.cz);
                    if rd < s.rad {
                        for v in light.iter_mut() {
                            *v *= dim;
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

    // ------------------------------------------------------------ the lance, the bell, shards
    fn items_ball(&self, _p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        if st.shattered {
            return Vec::new();
        }
        let d = unit(st.th);
        let reach = R + st.ext * 0.55;
        let tip = [st.p[0] + reach * d[0], st.p[1] + reach * d[1]];
        let depth = f.cam.depth(&[scene::to_world(tip[0], tip[1], R)])[0] + 0.01;
        let slide: Payload = Box::new("slide");
        vec![(depth, slide)]
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, _payload: &Payload) {
        let k = self.k;
        let st = &f.fs[k];
        let cam = f.cam;
        let prev = p.fighter(k, py_max(0.0, f.t - 1.0 / f64::from(FPS)));
        // a thrust going out: the streak
        if st.ext - prev.ext > 0.12 && py::wrap_pi(st.th - prev.th).abs() < 0.3 {
            // arr, depth = fr.array(), fr.depth() ... fr.set_array(arr, depth): Python does this on every
            // frame; without a streak it puts back the bytes it took
            let (mut arr, depth) = (fr.array(), fr.depth());
            let alpha = if st.haste > 0.0 { 0.55 } else { 0.38 };
            let streak = p.col[k].map(|c| c * 0.6 + 100.0);
            layer_blend(&mut arr, &slide_trail_polys(cam, st, prev.ext), streak, alpha, None);
            fr.set_array(&arr, Some(&depth));
        }
        let hk = if st.haste > 0.0 { 0.6 } else { 0.0 };
        draw_slide(fr, cam, f.l, st, p.col[k], py_max(hk, 0.8 * st.thr_k));
    }

    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, arr: &mut Img<f32>, depth: &mut Img<f32>) {
        if !f.in_fight {
            return;
        }
        for s in slams(p, f.i) {
            if s.owner != self.k as i64 {
                continue;
            }
            // PEDAL TONE: the giant bell stomps
            let Some(h) = bell_height(f.t, s.t0, s.t_slam) else {
                continue;
            };
            let col = p.col[self.k];
            fr.set_array(&*arr, Some(&*depth));
            draw_giant_bell(fr, f.cam, f.l, s.cx, s.cz, s.rad, h, col);
            // a mint halo under the lip
            let lip = f.cam.wp(&[scene::to_world(s.cx, s.cz, h + 0.15)])[0];
            fr.glow(lip.0, lip.1, 34.0, col.map(|c| c * 0.8 + 40.0), 0.32);
            *arr = fr.array();
            *depth = fr.depth();
        }
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
        match kind {
            "tube" => {
                // a curl of brass tubing
                let pts: Vec<[f64; 2]> = (0..7)
                    .map(|j| {
                        let a = rot + f64::from(j) * 0.5 * (1.0 + curl);
                        [sx + 0.16 * dmath::cos(a), sz + 0.16 * dmath::sin(a)]
                    })
                    .collect();
                let sp = on_screen(&pts);
                // im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8)); the line; back as float32: the
                // whole picture is cut to whole numbers
                let bytes = arr.data.iter().map(|&v| np::clip_u8_f32(v)).collect();
                let mut im: Img<u8> = Img::from_vec(arr.w, arr.h, arr.c, bytes);
                pil::line(&mut im, &sp, &ink(BRASS), 2);
                for (v, &b) in arr.data.iter_mut().zip(&im.data) {
                    *v = f32::from(b);
                }
            }
            "slidebit" => {
                let q4: Vec<[f64; 2]> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                    .iter()
                    .map(|&(a1, a2)| {
                        [
                            sx + d[0] * 0.22 * a1 + nn[0] * 0.025 * a2,
                            sz + d[1] * 0.22 * a1 + nn[1] * 0.025 * a2,
                        ]
                    })
                    .collect();
                layer_blend(arr, &[on_screen(&q4)], BRASS_HI, 1.0, None);
            }
            "bellbit" | "brass" => {
                // a curved chip of the bell, brass chips
                let s = size * if kind == "bellbit" { 1.5 } else { 1.3 };
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
                let col = BRASS.map(|c| np::clip(c * shade_k, 0.0, 255.0));
                layer_blend(arr, &[on_screen(&tri)], col, 1.0, None);
            }
            // (Python has no colour for another kind and raises; the look's `shards` names no other)
            _ => {}
        }
    }

    // ------------------------------------------------------------ lens, glows
    fn shimmer(&self, _p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, w: &Wave) {
        if w.kind == "shockwave" {
            let c = scene::to_world(w.cx, w.cz, 0.05);
            img.shimmer_world(cam, c, w.r, 0.4, 3.0, t, 1.0 - w.r / w.rmax, w.owner as f64);
        }
    }

    fn shimmer_after(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64) {
        for e in &p.rec.events {
            if e.kind == "slam" && e.who() == Some(self.k) && 0.0 <= t - e.t && t - e.t < 0.6 && t < p.t_hs {
                let u = (t - e.t) / 0.6;
                // rad = self.cfg['abilities']['low']['radius'] (Python raises for an ability without one)
                let (Some(rad), Some((cx, cz))) = (self.low_radius, centre(e)) else {
                    continue;
                };
                let c = scene::to_world(cx, cz, 0.05);
                img.shimmer_world(cam, c, rad + 1.6 * u, 0.35, 3.0, t, 1.0 - u, 0.0);
            }
        }
    }

    fn glows(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, _drawn: &[DrawnBall]) {
        // the crook flares as the lance strikes
        let st = &f.fs[self.k];
        if st.thr_k > 0.2 && !st.shattered && st.vis {
            let d = unit(st.th);
            let reach = R + st.ext + 0.07;
            let tp = [st.p[0] + reach * d[0], st.p[1] + reach * d[1]];
            let s = f.cam.wp(&[scene::to_world(tp[0], tp[1], R)])[0];
            fr.glow(s.0, s.1, 7.0, p.col[self.k].map(|c| c * 0.6 + 90.0), 0.35 * st.thr_k);
        }
    }

    // ------------------------------------------------------------ checks
    fn focus_points(&self, p: &PictureCtx, t: f64, st: &BallState) -> Vec<(V3, String)> {
        let mut out = Vec::new();
        if !st.shattered {
            let d = unit(st.th);
            let reach = R + st.ext;
            let tip = [st.p[0] + reach * d[0], st.p[1] + reach * d[1]];
            out.push((scene::to_world(tip[0], tip[1], R), "lance tip".to_string()));
        }
        // the bell once it stomps onto the canvas (high above it, it is scenery)
        for s in self.bell_down(p, t) {
            if let Some(h) = bell_height(t, s.t0, s.t_slam).filter(|&h| h <= 1.0) {
                out.push((scene::to_world(s.cx, s.cz, h), "pedal bell".to_string()));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    /// `bell_height` through its five phases: dropping in from 8.2, hovering at 3.2 (bobbing by 0.08), slamming
    /// down over the last 0.12 s, sitting at 0.1 for 0.3 s, flying off for 0.45 s.
    #[test]
    fn the_bell_drops_hovers_slams_sits_and_flies_off() {
        let (t0, ts) = (10.0, 11.5);
        assert_eq!(bell_height(9.999, t0, ts), None);
        assert!(bell_height(10.0, t0, ts).is_some_and(|h| close(h, 8.2)));
        // hovering: the drop is over after 0.3 s, a whole bob later the bell is at 3.2
        assert!(bell_height(11.0, t0, ts).is_some_and(|h| close(h, 3.2)));
        assert!(bell_height(ts - 0.06, t0, ts).is_some_and(|h| close(h, 3.2 - 3.1 * 0.25)));
        assert_eq!(bell_height(ts, t0, ts), Some(0.10));
        assert_eq!(bell_height(ts + 0.29, t0, ts), Some(0.10));
        assert!(bell_height(ts + 0.525, t0, ts).is_some_and(|h| close(h, 2.1)));
        // it is there to the end of the 0.75 s, high above the frame by then
        assert!(bell_height(ts + 0.74, t0, ts).is_some_and(|h| h > 7.5));
        assert_eq!(bell_height(ts + 0.75, t0, ts), None);
    }

    /// The slide has two tubes, two outer slides, the braces that fit and the crook's eight segments; a short
    /// slide has no room for a brace.
    #[test]
    fn the_slide_has_its_parts() {
        let count = |ext: f64, part: Part| {
            slide_geom([0.3, -0.2], 0.7, ext)
                .0
                .iter()
                .filter(|q| q.part == part)
                .count()
        };
        for ext in [0.3, 0.95, 2.5] {
            assert_eq!(
                (count(ext, Part::Tube), count(ext, Part::Outer), count(ext, Part::Crook)),
                (2, 2, 8)
            );
        }
        assert_eq!((count(0.1, Part::Brace), count(0.95, Part::Brace)), (0, 2));
        // the tip is TUBE_GAP beyond the tubes' end, at the ball's height
        let (_, tip) = slide_geom([0.0, 0.0], 0.0, 1.0);
        assert_eq!(tip, scene::to_world(R + 1.0 + TUBE_GAP, 0.0, R));
    }

    /// A thrust that moved the tip less than 0.05 leaves no streak; a longer one leaves one quad.
    #[test]
    fn a_short_thrust_leaves_no_streak() {
        let cam = Cam::new([0.12, 0.25, 0.0], 27.5, 0.0, 58.0, 36.0, Some(288.0), 0.0, 0.1);
        let st = |ext: f64| BallState {
            p: [0.5, 0.5],
            vis: true,
            shield: 0.0,
            haste: 0.0,
            glow: 0.0,
            glow_np: false,
            scale: 1.0,
            th: 0.3,
            blen: 0.95,
            frets: 1,
            level: 1,
            shattered: false,
            om: 0.0,
            sd: 1.0,
            ext,
            slow: 0.0,
            thr_k: 0.0,
            lift: 0.0,
            sink: 0.0,
            sx: None,
            sy: None,
            air_bias: 0.0,
            paint_h: None,
            extra: std::collections::BTreeMap::new(),
        };
        assert!(slide_trail_polys(&cam, &st(0.3), 0.33).is_empty());
        let streak = slide_trail_polys(&cam, &st(0.9), 0.3);
        assert_eq!((streak.len(), streak[0].len()), (1, 4));
    }

    /// The thrust that thumps the ball is the last one not after the frame's time: a thrust at the very time
    /// counts (numpy's `searchsorted(.., 'right') - 1`), and before the first thrust there is none.
    #[test]
    fn the_last_thrust_counts_from_its_own_time() {
        let thrusts = [1.0, 2.0, 3.0];
        assert_eq!(last_not_after(&thrusts, 0.5), None);
        assert_eq!(last_not_after(&thrusts, 1.0), Some(1.0));
        assert_eq!(last_not_after(&thrusts, 1.999), Some(1.0));
        assert_eq!(last_not_after(&thrusts, 2.0), Some(2.0));
        assert_eq!(last_not_after(&thrusts, 9.0), Some(3.0));
        assert_eq!(last_not_after(&[], 1.0), None);
    }

    #[test]
    fn linspace_ends_on_its_end() {
        let a = linspace(-dmath::PI / 2.0, dmath::PI / 2.0, 9);
        assert_eq!((a.len(), a[0], a[8]), (9, -dmath::PI / 2.0, dmath::PI / 2.0));
        assert_eq!(a[4], 4.0 * (dmath::PI / 8.0) + -dmath::PI / 2.0);
    }
}
