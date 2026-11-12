//! OBOE's look: the port of `Look` in `projects/ballfight/kit/fighters/oboe.py`, hook by hook.
//!
//! What it draws: its reed pointing out of the ball at the aim; the A gathering at the reed's tip while the aim
//! locks, then the beam with the A's sine running along it and a lit lane under it; the detuned enemy's soured glass,
//! seasick tilt, sour halo on the canvas and doubled, warped outline; DOUBLE REED's inked blades; ILL WIND's
//! swirling olive ring, streaks and smoke ring; ENDLESS NOTE's held beam; cane chips; woodwind shards when it loses.
//!
//! Its panel on the VS card is the card's own (`_oboe_reeds`, `bf_picture::title::patterns::oboe`).
//!
//! Every roster number comes from the `cfg` that `setup_as` is given (`self.cfg` in Python): the beam's `hold`,
//! DOUBLE REED's `half` and `reach`, ENDLESS NOTE's `len` and `width`.

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::roster::{FighterCfg, WeaponCfg};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{H, W, bayer_full, ease};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DefaultLook, DrawnBall, FrameCtx, Fx, Look, Payload, Phase, Pic, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::scene::{self, R_BALL as R};

use crate::fighters::shared::reeds::{self, CANE, CANE_D, INK, KO_CHIPS, Num, Px, SOUR, THREAD, WHITE};

/// `BEAM_Y = R * 1.0`: the beam's and the reed's height (the ball's centre).
const BEAM_Y: f64 = R * 1.0;

/// `np.array([math.cos(a), math.sin(a)])`.
fn unit(a: f64) -> [f64; 2] {
    [dmath::cos(a), dmath::sin(a)]
}

/// `e[7]` of a hit (what hit), or `""` for an event without one.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// `np.linspace(0, b, n)`: `i * step + 0`, the last value `b` itself.
fn linspace0(b: f64, n: usize) -> Vec<f64> {
    let step = (b - 0.0) / (n - 1) as f64;
    (0..n)
        .map(|i| if i == n - 1 { b } else { i as f64 * step + 0.0 })
        .collect()
}

/// A Python slice `lo:hi` of an axis `n` long, `lo` not negative: the indices it holds (a negative `hi` counts
/// from the end, as Python's does).
fn py_slice(lo: i64, hi: i64, n: usize) -> std::ops::Range<usize> {
    let n = n as i64;
    let hi = if hi < 0 { (hi + n).max(0) } else { hi.min(n) };
    let lo = lo.min(n);
    if lo >= hi {
        return 0..0;
    }
    lo as usize..hi as usize
}

// ---------------------------------------------------------------- TUNING A
/// `reed_polys(cam, p, aim, y, R, opening, length, half_w)`: the oboe's reed pointing out of the ball at `aim`: two
/// cane blades hinged at the rim; `opening` (radians) spreads them (DOUBLE REED's scissors). `[(screen poly,
/// shade)]`.
fn reed_polys(
    cam: &Cam,
    p: [f64; 2],
    aim: f64,
    y: f64,
    opening: f64,
    length: f64,
    half_w: f64,
) -> [(Vec<(f64, f64)>, f64); 2] {
    let ua = unit(aim);
    [1.0, -1.0].map(|sgn: f64| {
        let a = aim + sgn * opening;
        let d = unit(a);
        let nrm = [-d[1], d[0]];
        // b0 = base_ + [cos(aim), sin(aim)] * (R * 0.92) + nrm * 0.03 * sgn
        let b0 = [
            p[0] + ua[0] * (R * 0.92) + nrm[0] * 0.03 * sgn,
            p[1] + ua[1] * (R * 0.92) + nrm[1] * 0.03 * sgn,
        ];
        let tip = [b0[0] + d[0] * length, b0[1] + d[1] * length];
        // q = [b0 + nrm * 0.03, tip + nrm * half_w, tip - nrm * half_w * 0.3, b0 - nrm * 0.03]
        let q = [
            [b0[0] + nrm[0] * 0.03, b0[1] + nrm[1] * 0.03],
            [tip[0] + nrm[0] * half_w, tip[1] + nrm[1] * half_w],
            [tip[0] - nrm[0] * half_w * 0.3, tip[1] - nrm[1] * half_w * 0.3],
            [b0[0] - nrm[0] * 0.03, b0[1] - nrm[1] * 0.03],
        ];
        let world: Vec<V3> = q.iter().map(|c| scene::to_world(c[0], c[1], y + 0.04 * sgn)).collect();
        (cam.wp(&world), if sgn > 0.0 { 1.0 } else { 0.82 })
    })
}

/// `draw_reed(img, cam, p, aim, y, R, opening, length, glow, col)`: the reed's two blades (inked and edged in the
/// oboe's colour when they are out for DOUBLE REED), the green thread where it meets the ball, and the A gathering at
/// the tip (`glow`).
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_reed(
    img: &mut Img<f32>,
    cam: &Cam,
    p: [f64; 2],
    aim: f64,
    y: f64,
    opening: f64,
    length: f64,
    glow: f64,
    col: V3,
) {
    // DOUBLE REED: the blades out, inked and edged in olive
    let big = opening > 0.02 || length > 0.4;
    let half_w = 0.07 + 0.07 * py_min(1.0, opening * 2.0);
    for (poly, sh) in reed_polys(cam, p, aim, y, opening, length, half_w) {
        let mut closed = poly.clone();
        closed.push(poly[0]);
        let closed = [closed];
        if big {
            reeds::blend(img, &reeds::mask(&[], &closed, 3), INK, Num::Py(0.85));
        }
        let mk = reeds::mask(&[poly], &[], 1);
        reeds::blend(img, &mk, CANE.map(|c| c * sh), Num::Py(1.0));
        let edge = reeds::mask(&[], &closed, 1);
        let edge_col = if big {
            col.map(|c| c * 0.8)
        } else {
            CANE_D.map(|c| c * 0.55)
        };
        reeds::blend(img, &edge, edge_col, Num::Py(0.9));
    }
    // the green thread where it meets the ball
    let ua = unit(aim);
    let q = [p[0] + ua[0] * (R * 0.98), p[1] + ua[1] * (R * 0.98)];
    let s = cam.wp(&[scene::to_world(q[0], q[1], y)])[0];
    let (xi, yi) = (py::round_half_even(s.0) as i64, py::round_half_even(s.1) as i64);
    // img[max(0, yi - 1):yi + 2, max(0, xi - 1):xi + 2] = THREAD
    for yy in py_slice((yi - 1).max(0), yi + 2, H) {
        for xx in py_slice((xi - 1).max(0), xi + 2, W) {
            for (v, c) in img.px_mut(xx, yy).iter_mut().zip(THREAD) {
                *v = c as f32;
            }
        }
    }
    if glow > 0.02 {
        // the A gathering at the tip before it sounds
        let tip = [p[0] + ua[0] * (R + length), p[1] + ua[1] * (R + length)];
        let s2 = cam.wp(&[scene::to_world(tip[0], tip[1], y)])[0];
        let k = 3.0 + 4.0 * glow;
        // np.exp(-(rr / (3 + 4 * glow)) ** 2).astype(np.float32), rr = np.hypot(xx - s2[0], yy - s2[1])
        let data = (0..W * H)
            .map(|i| {
                let rr = np::hypot((i % W) as f64 - s2.0, (i / W) as f64 - s2.1);
                let q = rr / k;
                dmath::exp(-(q * q)) as f32
            })
            .collect();
        let mk: Img<f32> = Img::from_vec(W, H, 1, data);
        reeds::add(img, &mk, col.map(|c| c * 0.6 + 80.0), 0.55 * glow);
    }
}

/// `draw_beam(img, cam, o, aim, length, width, k, t, col, y, held)`: the tuning A: a narrow beam with the A's sine
/// running along it (scrolling outward) and a warm white core.
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_beam(
    img: &mut Img<f32>,
    cam: &Cam,
    o: [f64; 2],
    aim: f64,
    length: f64,
    width: f64,
    k: f64,
    t: f64,
    col: V3,
    y: f64,
    held: bool,
) {
    if k <= 0.01 {
        return;
    }
    let d = unit(aim);
    let nrm = [-d[1], d[0]];
    let length = py_min(length, reeds::ring_exit(o, d, None));
    let s = linspace0(length, 48);
    // P = column_stack([o[0] + d[0] * s, o[1] + d[1] * s, full(y)])
    let path: Vec<V3> = s.iter().map(|&v| [o[0] + d[0] * v, o[1] + d[1] * v, y]).collect();
    let glow = reeds::ribbon(cam, &path, &[0.11 + 0.04 * f64::from(u8::from(held))]);
    reeds::blend(
        img,
        &reeds::mask(&[glow], &[], 1),
        col.map(|c| c * 0.7 + 70.0),
        Num::Py(0.30 * k),
    );
    let amp = width * 0.75 + 0.02;
    // wave = o + d * s + nrm * (amp * np.sin(2 * np.pi * (s / 0.42 - t * 7.0)))
    let wave: Vec<V3> = s
        .iter()
        .map(|&v| {
            let w = amp * dmath::sin(2.0 * dmath::PI * (v / 0.42 - t * 7.0));
            scene::to_world(o[0] + d[0] * v + nrm[0] * w, o[1] + d[1] * v + nrm[1] * w, y)
        })
        .collect();
    let sp = cam.wp(&wave);
    reeds::blend(
        img,
        &reeds::mask(&[], &[sp], 2),
        col.map(|c| np::clip(c * 0.6 + 110.0, 0.0, 255.0)),
        Num::Py(0.85 * k),
    );
    let world: Vec<V3> = path.iter().map(|q| scene::to_world(q[0], q[1], q[2])).collect();
    let core = cam.wp(&world);
    reeds::blend(img, &reeds::mask(&[], &[core], 1), WHITE, Num::Py(0.95 * k));
}

/// `wobble_rim(img, x, y, rp, depth, t, col, seed)`: OUT OF TUNE: a doubled, warped outline round the ball (a sour
/// ghost a few pixels off, its rim waving), stronger with the detune's depth.
#[allow(clippy::too_many_arguments)] // the Python signature
fn wobble_rim<T: Px>(img: &mut Img<T>, x: f64, y: f64, rp: f64, depth: f64, t: f64, col: V3, seed: f64) {
    if depth <= 0.02 {
        return;
    }
    // int(): toward zero
    let pad = (rp + 8.0) as i64;
    let (xi, yi) = (x as i64, y as i64);
    let (x0, x1) = ((xi - pad).max(0), (xi + pad + 1).min(W as i64));
    let (y0, y1) = ((yi - pad).max(0), (yi + pad + 1).min(H as i64));
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    // c = np.asarray(col) * 0.55 + np.array([120, 130, 40]) * 0.45
    let sour = [120.0, 130.0, 40.0];
    let c: V3 = std::array::from_fn(|ch| col[ch] * 0.55 + sour[ch] * 0.45);
    for (j, (ox, oy, a)) in [(2.2, -1.2, 0.85), (-1.8, 1.4, 0.55)].into_iter().enumerate() {
        let j = j as f64;
        let sh = depth * (1.0 + 0.6 * dmath::sin(t * 9.0 + j * 2.0 + seed));
        let (cx, cy) = (x + ox * sh, y + oy * sh);
        let aa = a * py_min(1.0, depth * 1.4);
        for py_ in y0..y1 {
            for px_ in x0..x1 {
                let (dx, dy) = (px_ as f64 - cx, py_ as f64 - cy);
                let ang = dmath::atan2(dy, dx);
                let rr = np::hypot(dx, dy);
                let wav = rp * (1.0 + 0.10 * depth * dmath::sin(5.0 * ang + t * 13.0 + j * 1.7));
                if (rr - wav).abs() < 0.9 {
                    // sub[ring] = sub[ring] * (1 - aa) + c * aa: the picture's dtype times a Python float, plus a
                    // float64 colour
                    for (v, cc) in img.px_mut(px_ as usize, py_ as usize).iter_mut().zip(c) {
                        *v = T::set(v.times(1.0 - aa) + cc * aa);
                    }
                }
            }
        }
    }
}

/// One live beam of `live_beams`: `(o, aim, len, width, alpha, held)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct LiveBeam {
    o: [f64; 2],
    aim: f64,
    len: f64,
    width: f64,
    k: f64,
    held: bool,
}

/// One `tune` event of its own: `(t, o, aim, len, width)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Tune {
    t: f64,
    o: [f64; 2],
    aim: f64,
    len: f64,
    width: f64,
}

// ---------------------------------------------------------------- the look
/// OBOE's look: `Look` of `kit/fighters/oboe.py`.
#[derive(Clone, Debug, Default)]
pub struct OboeLook {
    /// Its side.
    k: usize,
    /// `TUNES`: its beams.
    tunes: Vec<Tune>,
    /// `LOCKS`: `(t, aim)` of its aim locks.
    locks: Vec<(f64, f64)>,
    /// `ENDLESS`: `(t0, t1)` of its held beams.
    endless: Vec<(f64, f64)>,
    /// `SNIP_OPEN`: `(t0, t_snap, aim)` of its DOUBLE REEDs.
    snip_open: Vec<(f64, f64, f64)>,
    /// `self.cfg['weapon']['hold']`: seconds a beam lives (`None` for a weapon without one).
    hold: Option<f64>,
    /// `self.cfg['abilities']['unlock1']['half']`: DOUBLE REED's half angle in degrees.
    snip_half: Option<f64>,
    /// `self.cfg['abilities']['unlock1']['reach']`: DOUBLE REED's reach.
    snip_reach: Option<f64>,
    /// `self.cfg['abilities']['unlock2']['len']`: ENDLESS NOTE's length.
    endless_len: Option<f64>,
    /// `self.cfg['abilities']['unlock2']['width']`: ENDLESS NOTE's width.
    endless_width: Option<f64>,
}

impl OboeLook {
    /// `detune_at(kk, t)`: depth x fade of ball `kk`'s OUT OF TUNE status at `t` (from the recorded `DET`).
    fn detune_at(&self, p: &PictureCtx, kk: usize, t: f64) -> f64 {
        if !(p.show.grid.t_fight <= t && t < p.t_ko) {
            return 0.0;
        }
        let Some(row) = p.run.row("DET", p.sim_i(t), kk) else {
            return 0.0;
        };
        let (left, dep) = (row[0], row[1]);
        dep * py_min(1.0, left / 0.25)
    }

    /// `lock_k(t)`: how far the A has gathered at the reed's tip: from the aim lock to the shot, 0 to 1.
    fn lock_k(&self, t: f64) -> f64 {
        for &(t0, _aim) in &self.locks {
            if t0 <= t {
                let nxt = self.tunes.iter().find(|q| q.t >= t0);
                if let Some(nxt) = nxt.filter(|q| t < q.t) {
                    return (t - t0) / py_max(1e-6, nxt.t - t0);
                }
            }
        }
        0.0
    }

    /// `snip_state(t)`: `(aim, opening rad, length, snapping flash)` of DOUBLE REED's blades at `t`, or `None`.
    fn snip_state(&self, p: &PictureCtx, t: f64) -> Option<(f64, f64, f64, f64)> {
        let k = self.k;
        for &(t0, t_snap, aim) in &self.snip_open {
            if t0 <= t && t < t_snap + 0.25 && t0 < p.t_ko {
                // ab = self.cfg['abilities']['unlock1'] (Python raises for an ability without the two numbers)
                let (half, reach) = (self.snip_half?, self.snip_reach?);
                let i = p.sim_i(py_min(t, p.t_ko));
                // a = float(row[1]) if row is not None and not np.isnan(row[1]) else aim
                let a = match p.run.row("SNIP", i, k) {
                    Some(row) if !row[1].is_nan() => row[1],
                    _ => aim,
                };
                if t < t_snap {
                    let u = (t - t0) / (t_snap - t0);
                    return Some((
                        a,
                        half * (dmath::PI / 180.0) * 0.75 * ease(u),
                        0.34 + (reach - 0.34) * ease(u),
                        0.0,
                    ));
                }
                let u = (t - t_snap) / 0.25;
                return Some((a, 0.0, reach * (1.0 - u) + 0.34 * u, 1.0 - u));
            }
        }
        None
    }

    /// `live_beams(F)`: its A beams showing at `t` (a shot lives `hold` seconds plus a 0.12 s fade; ENDLESS NOTE's
    /// beam rides the oboe's recorded aim).
    fn live_beams(&self, p: &PictureCtx, t: f64) -> Vec<LiveBeam> {
        let mut out = Vec::new();
        if !(p.show.grid.t_fight <= t && t < p.t_hs) {
            return out;
        }
        let k = self.k;
        let tc = py_min(t, p.t_ko);
        for q in &self.tunes {
            // hold = self.cfg['weapon']['hold'] (Python raises for a weapon without one)
            let Some(hold) = self.hold else { break };
            if q.t <= tc && tc < q.t + hold + 0.12 {
                let kk = if tc < q.t + hold {
                    1.0
                } else {
                    1.0 - (tc - q.t - hold) / 0.12
                };
                out.push(LiveBeam {
                    o: q.o,
                    aim: q.aim,
                    len: q.len,
                    width: q.width,
                    k: kk,
                    held: false,
                });
            }
        }
        for &(t0, t1) in &self.endless {
            if t0 <= tc && tc < t1 + 0.12 {
                let i = p.sim_i(tc);
                let (Some(len), Some(width)) = (self.endless_len, self.endless_width) else {
                    break;
                };
                let a = p.run.f("TH", i, k).unwrap_or(0.0);
                let at = p.run.p(i, k);
                let u = unit(a);
                let o = [at[0] + u[0] * (R + 0.04), at[1] + u[1] * (R + 0.04)];
                let fade = if tc < t1 { 1.0 } else { 1.0 - (tc - t1) / 0.12 };
                let kk = py_min(1.0, (tc - t0) / 0.08) * fade;
                out.push(LiveBeam {
                    o,
                    aim: a,
                    len,
                    width,
                    k: kk,
                    held: true,
                });
            }
        }
        out
    }
}

/// `st.get(key, 0.0)` of a look's own picture state.
fn extra(st: &BallState, key: &str) -> f64 {
    st.extra.get(key).copied().unwrap_or(0.0)
}

impl Look for OboeLook {
    /// `['wood'] * 6 + ['key'] * 8 + ['reed'] * 3`.
    fn shards(&self) -> Vec<String> {
        [("wood", 6), ("key", 8), ("reed", 3)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["wave", "snip", "stun"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["wave", "snip"]
    }

    fn quiet_hits(&self) -> &[&str] {
        &["sting"]
    }

    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[("wave", 1.0), ("shockwave", 0.5), ("stun", 0.5)]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// What `setup_as` took from `cfg`, by the keys `oboe.py` reads from `self.cfg`.
    fn cfg_summary(&self) -> Option<String> {
        Some(format!(
            "abilities.unlock1.half={:?}; abilities.unlock1.reach={:?}; abilities.unlock2.len={:?}; \
             abilities.unlock2.width={:?}; weapon.hold={:?}",
            self.snip_half, self.snip_reach, self.endless_len, self.endless_width, self.hold
        ))
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`: the roster numbers come from `cfg`, the events from side
    /// `k`. For its own side `cfg` is `p.cfg[k]`, and this is `setup`.
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.hold = match &cfg.weapon {
            WeaponCfg::Tune(w) => Some(w.hold),
            _ => None,
        };
        self.snip_half = cfg.unlock1.get("half");
        self.snip_reach = cfg.unlock1.get("reach");
        self.endless_len = cfg.unlock2.get("len");
        self.endless_width = cfg.unlock2.get("width");
        let ev = &p.rec.events;
        let mine = |e: &&Event, kind: &str| e.kind == kind && e.who() == Some(k);
        // (e[0], e[2], np.array([e[3], e[4]]), e[5], e[6], e[7])
        self.tunes = ev
            .iter()
            .filter(|e| mine(e, "tune"))
            .map(|e| Tune {
                t: e.t,
                o: [e.f(3), e.f(4)],
                aim: e.f(5),
                len: e.f(6),
                width: e.f(7),
            })
            .collect();
        self.locks = ev
            .iter()
            .filter(|e| mine(e, "tune_lock"))
            .map(|e| (e.t, e.f(3)))
            .collect();
        // (t0, min([e[3]] + ends + [P.T_KO]), k), ends = the times of its `endless_end` events not before t0
        self.endless = ev
            .iter()
            .filter(|e| mine(e, "endless"))
            .map(|e| {
                let mut t1 = e.f(3);
                for q in ev.iter().filter(|q| mine(q, "endless_end") && q.t >= e.t) {
                    t1 = py_min(t1, q.t);
                }
                (e.t, py_min(t1, p.t_ko))
            })
            .collect();
        self.snip_open = ev
            .iter()
            .filter(|e| mine(e, "snip_open"))
            .map(|e| (e.t, e.f(3), e.f(4)))
            .collect();
    }

    /// `sprite_rolls(kk, roll)`: the seasick wobble's steps (the enemy's sprite at 4-degree rolls).
    fn sprite_rolls(&self, kk: usize, roll: f64) -> Vec<f64> {
        if kk == 1 - self.k {
            (-3..=3).map(|j| roll + 4.0 * f64::from(j)).collect()
        } else {
            Vec::new()
        }
    }

    /// OUT OF TUNE: the glass sours, the instrument rolls seasick (4-degree steps).
    fn ball(&self, _p: &PictureCtx, kk: usize, st: &BallState, t: f64, glass: [f64; 3], roll: f64) -> ([f64; 3], f64) {
        let sour = py_min(1.0, extra(st, "det") * 1.2);
        // glass = glass * (1 - 0.35 * sour) + np.array([186, 196, 120]) * 0.35 * sour
        let tint = [186.0, 196.0, 120.0];
        let glass = std::array::from_fn(|ch| glass[ch] * (1.0 - 0.35 * sour) + tint[ch] * 0.35 * sour);
        let roll = if sour > 0.05 {
            // roll + 4 * int(round(3.0 * sour * math.sin(t * 9.0 + kk)))
            roll + 4.0 * py::round_half_even(3.0 * sour * dmath::sin(t * 9.0 + kk as f64))
        } else {
            roll
        };
        (glass, roll)
    }

    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, gl: f64) {
        if phase == Phase::Fight {
            // its A knocks the enemy OUT OF TUNE
            st.extra.insert("det".to_string(), self.detune_at(p, kk, t));
            if kk == self.k {
                let lock = if t < p.t_ko { self.lock_k(t) } else { 0.0 };
                st.extra.insert("lock".to_string(), lock);
                st.glow = py_max(st.glow, 0.35 * lock);
            }
        } else if phase == Phase::Tail && kk == self.k {
            reeds::tail_theta(p, kk, st, gl);
        }
    }

    // ------------------------------------------------------------ events
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        match how(e) {
            "tune" => Some(0.5),
            "snip" => Some(1.3),
            "wave" => Some(2.0),
            _ => DefaultLook.shake(p, e),
        }
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let col = p.col[e.who().filter(|&k| k < 2).unwrap_or(0)];
        let what = how(e);
        if what == "sting" {
            // the stings: a few sour flecks, no more
            return (col.map(|c| c * 0.5 + 110.0), 3, 1.2, 0.16);
        }
        if self.spark_big().contains(&what) {
            return (col.map(|c| c * 0.6 + 100.0), 16, 3.2, 0.3);
        }
        (col.map(|c| c * 0.6 + 100.0), 6, 2.0, 0.2)
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "wave" | "snip" => Some((14.0, 0.35)),
            "tune" | "stun" | "cut" => Some((8.0, 0.2)),
            _ => None,
        }
    }

    fn fringe(&self, _p: &PictureCtx, e: &Event) -> bool {
        matches!(how(e), "wave" | "snip")
    }

    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let k = self.k;
        // TUNING A: olive light along the beam
        for q in &self.tunes {
            if q.t > p.t_ko {
                break;
            }
            let u = unit(q.aim);
            for f in [0.15, 0.5] {
                let at = [q.o[0] + u[0] * q.len * f, q.o[1] + u[1] * q.len * f];
                fx.fl.add(q.t, up(at[0], at[1], R), p.col[k], 1.4, 0.5, 0.03, 0.25, 0.0);
            }
        }
        // ENDLESS NOTE: a held olive glow, lit again every beat
        for &(t0, t1) in &self.endless {
            let mut tb = t0;
            while tb < t1 {
                let i = p.sim_i(tb);
                let (at, a) = (p.run.p(i, k), p.run.f("TH", i, k).unwrap_or(0.0));
                let u = unit(a);
                let q = [at[0] + u[0] * 1.6, at[1] + u[1] * 1.6];
                fx.fl.add(tb, up(q[0], q[1], R), p.col[k], 1.8, 0.55, 0.03, 0.35, 0.0);
                tb += p.show.grid.beat;
            }
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        let t = e.t;
        let Some(att) = e.who().filter(|&k| k < 2) else { return };
        // seed=int(t_ * 10)
        let seed = np::f64_to_i64(t * 10.0).max(0) as u64;
        if e.kind == "hit" {
            let (x, z) = (e.f(5), e.f(6));
            let pos = up(x, z, R);
            match how(e) {
                "tune" => {
                    // the A lands: a sour burst
                    fx.bu.add(t, pos, p.col[att], 0.9, None, true);
                    fx.fl.add(t, pos, p.col[att], 1.6, 0.7, 0.03, 0.3, 0.0);
                    let emit = Emit {
                        n: Some(8),
                        col: Some(SOUR.to_vec()),
                        ..Emit::default()
                    };
                    fx.pa.emit(t, pos, "motes", &emit);
                }
                "endless" => {
                    let emit = Emit {
                        n: Some(4),
                        col: Some(SOUR.to_vec()),
                        ..Emit::default()
                    };
                    fx.pa.emit(t, pos, "motes", &emit);
                }
                "snip" => {
                    // DOUBLE REED snaps: cane splinters
                    fx.bu.add(t, pos, [232.0, 210.0, 152.0], 1.3, None, true);
                    fx.fl.add(t, pos, p.col[att], 1.8, 0.9, 0.03, 0.3, 0.0);
                    let emit = Emit {
                        n: Some(12),
                        col: Some(vec![[232.0, 210.0, 152.0], [196.0, 168.0, 110.0]]),
                        ..Emit::default()
                    };
                    fx.pa.emit(t, pos, "chips", &emit);
                    fx.mk.add(
                        t,
                        Mark {
                            size: 0.4,
                            angle: 0.6,
                            col: [110.0, 96.0, 60.0],
                            life: 3.0,
                            n: 2,
                            seed,
                            strength: 0.6,
                            ..Mark::new(MarkKind::Claw, x, z)
                        },
                    );
                }
                "wave" => {
                    // ILL WIND lands
                    fx.bu.add(t, pos, p.col[att], 1.5, None, true);
                    fx.fl.add(t, pos, p.col[att], 2.2, 1.0, 0.03, 0.4, 0.0);
                    let emit = Emit {
                        n: Some(16),
                        spread: Some(180.0),
                        col: Some(SOUR.to_vec()),
                        ..Emit::default()
                    };
                    fx.pa.emit(t, pos, "motes", &emit);
                }
                _ => {}
            }
        } else if e.kind == "ability" && matches!(e.fields.get(2), Some(Value::Str(s)) if s == "wave") {
            // ILL WIND: the gust leaves the oboe
            let at = p.run.p(p.sim_i(t), att);
            let emit = Emit {
                n: Some(24),
                spread: Some(180.0),
                col: Some(SOUR.to_vec()),
                speed: Some((2.0, 3.5)),
                ..Emit::default()
            };
            fx.pa.emit(t, up(at[0], at[1], R * 0.5), "streaks", &emit);
            fx.mk.add(
                t,
                Mark {
                    size: 1.4,
                    life: 3.0,
                    seed,
                    strength: 0.5,
                    ..Mark::new(MarkKind::SmokeRing, at[0], at[1])
                },
            );
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let emit = Emit {
            n: Some(26),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            col: Some(KO_CHIPS.to_vec()),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "chips", &emit);
    }

    // ------------------------------------------------------------ canvas, reed, beams
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        // the beams' lit lanes
        let tint = p.col[self.k].map(|v| v / 255.0 * 0.9 + 0.25);
        for b in self.live_beams(p, c.t) {
            let lane = reeds::beam_lane(c.sxp, c.szp, b.o, b.aim, b.len, b.width, c.ys, c.xs);
            for (light, &l) in ic.iter_mut().zip(&lane) {
                let v = l * 0.42 * b.k;
                for ch in 0..3 {
                    light[ch] += v * tint[ch];
                }
            }
        }
        // the sour halo of a detuned ball
        for kk in 0..2 {
            let st = &c.fs[kk];
            let det = extra(st, "det");
            if c.in_fight && det > 0.02 {
                let sour = [0.62, 0.72, 0.18];
                for (j, light) in ic.iter_mut().enumerate() {
                    let (dx, dz) = (c.sxp[j] - st.p[0], c.szp[j] - st.p[1]);
                    let rd = np::hypot(dx, dz);
                    let ang = dmath::atan2(dz, dx);
                    let wob = R * 1.35 + 0.08 * dmath::sin(5.0 * ang - 12.0 * c.t);
                    let band = np::clip(1.0 - (rd - wob).abs() / 0.05, 0.0, 1.0);
                    let v = band * 0.5 * det;
                    for ch in 0..3 {
                        light[ch] += v * sour[ch];
                    }
                }
            }
        }
    }

    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind == "stun" || w.kind == "pulse" {
            return;
        }
        // ILL WIND: a swirling sour ring
        let k_ = (1.0 - w.r / w.rmax) * 1.3;
        let tint = p.col[w.owner].map(|v| v / 255.0 * 1.3 + 0.2);
        let bayer = bayer_full();
        for (j, light) in ic.iter_mut().enumerate() {
            let (dx, dz) = (c.sxp[j] - w.cx, c.szp[j] - w.cz);
            let rd = np::hypot(dx, dz);
            let band = np::clip(1.0 - (rd - w.r).abs() / 0.32, 0.0, 1.0);
            let band = (band * 3.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 3.0;
            let swirl = 0.5 + 0.5 * dmath::sin(dmath::atan2(dz, dx) * 6.0 + rd * 3.0 - c.t * 8.0);
            let v = band * k_ * (0.6 + 0.5 * swirl);
            for ch in 0..3 {
                light[ch] += v * tint[ch];
            }
        }
    }

    fn items_ball(&self, p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        if st.shattered {
            return Vec::new();
        }
        let u = unit(st.th);
        let tip = [st.p[0] + (R + 0.2) * u[0], st.p[1] + (R + 0.2) * u[1]];
        let mut dz = f.cam.depth(&[scene::to_world(tip[0], tip[1], R)])[0] + 0.01;
        if f.in_fight && self.snip_state(p, f.t).is_some() {
            // DOUBLE REED's blades: over both balls
            dz = -1e9;
        }
        let reed: Payload = Box::new("reed");
        vec![(dz, reed)]
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, _payload: &Payload) {
        let k = self.k;
        let st = &f.fs[k];
        let (mut arr, depth) = (fr.array(), fr.depth());
        let sn = if f.in_fight { self.snip_state(p, f.t) } else { None };
        if let Some((a, opening, ln, flash)) = sn {
            draw_reed(&mut arr, f.cam, st.p, a, BEAM_Y, opening, ln, flash, p.col[k]);
        } else {
            let mut g = if f.in_fight && f.t < p.t_ko {
                extra(st, "lock")
            } else {
                0.0
            };
            if k == p.win && f.t >= p.t_hs {
                // the champion's A on every solo note
                g = p.solo_env(f.t);
            }
            draw_reed(&mut arr, f.cam, st.p, st.th, BEAM_Y, 0.0, 0.34, g, p.col[k]);
        }
        fr.set_array(&arr, Some(&depth));
    }

    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        for b in self.live_beams(p, f.t) {
            draw_beam(
                arr,
                f.cam,
                b.o,
                b.aim,
                b.len,
                b.width,
                b.k,
                f.t,
                p.col[self.k],
                BEAM_Y,
                b.held,
            );
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
        _curl: f64,
    ) {
        reeds::draw_woodwind_shard(arr, cam, kind, pos, rot, size);
    }

    fn shimmer(&self, _p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, w: &Wave) {
        let wave = w.kind == "wave";
        let c = scene::to_world(w.cx, w.cz, 0.05);
        img.shimmer_world(
            cam,
            c,
            w.r,
            if wave { 0.4 } else { 0.25 },
            if wave { 3.0 } else { 2.0 },
            t,
            1.0 - w.r / w.rmax,
            w.owner as f64,
        );
    }

    fn after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        // OUT OF TUNE outlines, in the oboe's colour
        let col = p.col[self.k];
        for d in drawn {
            let det = extra(&f.fs[d.k], "det");
            match img {
                Pic::F32(a) => wobble_rim(a, d.s.0, d.s.1, d.rp, det, f.t, col, d.k as f64),
                Pic::F64(a) => wobble_rim(a, d.s.0, d.s.1, d.rp, det, f.t, col, d.k as f64),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Python slice with a negative end counts from the end of the axis; one past the end stops at it.
    #[test]
    fn a_python_slice_keeps_pythons_ends() {
        assert_eq!(py_slice(0, 2, 640), 0..2);
        assert_eq!(py_slice(638, 641, 640), 638..640);
        assert_eq!(py_slice(0, -3, 640), 0..637);
        assert_eq!(py_slice(700, 703, 640), 0..0);
        assert_eq!(py_slice(0, 0, 640), 0..0);
    }

    /// The A gathers from the aim lock to the next shot, and not before a lock or after its shot.
    #[test]
    fn the_lock_runs_from_the_aim_lock_to_the_shot() {
        let tune = |t: f64| Tune {
            t,
            o: [0.0, 0.0],
            aim: 0.0,
            len: 6.0,
            width: 0.12,
        };
        let look = OboeLook {
            tunes: vec![tune(2.0), tune(4.0)],
            locks: vec![(1.5, 0.3), (3.5, 0.4)],
            ..OboeLook::default()
        };
        assert_eq!(look.lock_k(1.0), 0.0);
        assert_eq!(look.lock_k(1.75), 0.5);
        assert_eq!(look.lock_k(2.5), 0.0);
        assert_eq!(look.lock_k(3.75), 0.5);
        assert_eq!(look.lock_k(4.0), 0.0);
    }

    /// The seasick steps are the enemy's only: seven rolls 4 degrees apart round its own.
    #[test]
    fn the_enemys_sprite_rolls_in_four_degree_steps() {
        let look = OboeLook::default();
        assert_eq!(look.sprite_rolls(1, 10.0), [-2.0, 2.0, 6.0, 10.0, 14.0, 18.0, 22.0]);
        assert!(look.sprite_rolls(0, 10.0).is_empty());
    }
}
