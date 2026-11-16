//! SNARE DRUM's look (a Studio fighter: no kit look to match). The ball is its model (a cherry-sparkle snare built
//! from web photos). Over the ring its two hickory sticks jab out from the recorded `STICKS` rows, a pale bead at the
//! tip and a soft streak behind it (brighter on an accent); under FLAM a faint grace stick leads each jab; RIMSHOT
//! arms the ball with a quick chrome glint, and a landed rimshot cracks a bright ring and a burst on the enemy; BUZZ
//! ROLL is a spinning disc of blurred sticks in its colour. It shatters into shell, chrome hoop and stick pieces.

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{py_max, py_min};
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{H, W};
use bf_picture::fx::Emit;
use bf_picture::img::Img;
use bf_picture::look::{FrameCtx, Fx, Look, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::effects::up;
use bf_picture::ring::{self, PictureCtx};
use bf_picture::scene::{R_BALL as R, to_world};

use super::state::{self, Stroke};
use crate::fighters::shared::reeds::{self, Num};

/// The hickory of a stick.
const HICKORY: V3 = [226.0, 196.0, 140.0];
/// A stick's tip bead.
const BEAD: V3 = [250.0, 244.0, 230.0];
/// The hoops' chrome.
const CHROME: V3 = [214.0, 220.0, 230.0];
/// The coated head.
const HEAD: V3 = [238.0, 236.0, 228.0];
/// The height the sticks jab at.
const STICK_Y: f64 = R * 0.75;

/// A hit's kind (`e[7]`), or `""`.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// The event's fighter as a side.
fn owner(e: &Event) -> usize {
    e.who().filter(|&k| k < 2).unwrap_or(0)
}

/// One recorded row: `(owner, side, ext, x0, z0, x1, z1)`; side 2 is a buzz disc (`ext` its radius, `x0, z0` its
/// centre).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Row {
    owner: usize,
    side: i64,
    ext: f64,
    x0: f64,
    z0: f64,
    x1: f64,
    z1: f64,
}

impl Row {
    fn of(v: &Value) -> Option<Row> {
        let Value::List(r) = v else { return None };
        let f = |i: usize| match r.get(i) {
            Some(Value::Float(x)) => Some(*x),
            Some(Value::Int(n)) => Some(*n as f64),
            _ => None,
        };
        let n = |i: usize| match r.get(i) {
            Some(Value::Int(n)) => Some(*n),
            _ => None,
        };
        Some(Row {
            owner: usize::try_from(n(0)?).ok()?,
            side: n(1)?,
            ext: f(2)?,
            x0: f(3)?,
            z0: f(4)?,
            x1: f(5)?,
            z1: f(6)?,
        })
    }
}

/// Ring-frame points to the screen.
fn wp(cam: &Cam, pts: &[V3]) -> Vec<(f64, f64)> {
    let world: Vec<V3> = pts.iter().map(|q| to_world(q[0], q[1], q[2])).collect();
    cam.wp(&world)
}

/// A filled disc of radius `rp` pixels at screen point `c`, with a dark rim.
fn bead(img: &mut Img<f32>, c: (f64, f64), rp: f64, col: V3) {
    let (x0, x1) = ((c.0 - rp - 1.0) as i64, (c.0 + rp + 2.0) as i64);
    let (y0, y1) = ((c.1 - rp - 1.0) as i64, (c.1 + rp + 2.0) as i64);
    for y in y0.max(0)..y1.min(H as i64) {
        for x in x0.max(0)..x1.min(W as i64) {
            let (dx, dy) = (x as f64 + 0.5 - c.0, y as f64 + 0.5 - c.1);
            let dd = (dx * dx + dy * dy) / (rp * rp);
            let px = img.px_mut(x as usize, y as usize);
            if dd <= 1.0 {
                for (v, c) in px.iter_mut().zip(col) {
                    *v = c as f32;
                }
            } else if dd <= 1.6 {
                for v in px.iter_mut() {
                    *v *= 0.5;
                }
            }
        }
    }
}

/// One stick from ring point `a` to `b`: the shaft, then the bead at `b`.
fn draw_stick(img: &mut Img<f32>, cam: &Cam, a: (f64, f64), b: (f64, f64), alpha: f64, glow: Option<V3>) {
    let sp = wp(cam, &[[a.0, a.1, STICK_Y], [b.0, b.1, STICK_Y]]);
    if let Some(g) = glow {
        let wide = reeds::mask(&[], std::slice::from_ref(&sp), 3);
        reeds::blend(img, &wide, g, Num::Py(0.35 * alpha));
    }
    let shaft = reeds::mask(&[], std::slice::from_ref(&sp), 1);
    reeds::blend(img, &shaft, HICKORY, Num::Py(0.95 * alpha));
    let depth = cam.depth(&[to_world(b.0, b.1, STICK_Y)])[0];
    let rp = py_max(1.0, 0.05 * cam.f / py_max(1e-6, depth));
    if alpha > 0.6 {
        bead(img, sp[1], rp, BEAD);
    }
}

/// SNARE DRUM's look.
#[derive(Default)]
pub struct SnareDrumLook {
    k: usize,
    strokes: Vec<Stroke>,
    flams: Vec<(f64, f64)>,
    armed: Vec<f64>,
    rims: Vec<(f64, f64)>,
    buzz: Vec<(f64, f64)>,
}

impl std::fmt::Debug for SnareDrumLook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnareDrumLook")
            .field("k", &self.k)
            .field("strokes", &self.strokes.len())
            .finish()
    }
}

impl SnareDrumLook {
    /// Its rows showing at `t`, from the recorded `STICKS`.
    fn live(&self, p: &PictureCtx, t: f64) -> Vec<Row> {
        if !(p.show.grid.t_fight <= t && t < p.t_hs) || !p.run.has("STICKS") {
            return Vec::new();
        }
        let i = p.sim_i(py_min(t, p.t_ko));
        match p.run.nested("STICKS", i) {
            Some(Value::List(rows)) => rows.iter().filter_map(Row::of).filter(|r| r.owner == self.k).collect(),
            _ => Vec::new(),
        }
    }

    /// The stroke that started the jab showing at `t` on `side`: an accent?
    fn accent_at(&self, t: f64, side: i64) -> bool {
        self.strokes
            .iter()
            .rev()
            .find(|s| s.side == side && s.t <= t)
            .is_some_and(|s| s.accent)
    }

    /// Where ball `kk` is at `t`, in the world, `y` up.
    fn at(&self, p: &PictureCtx, kk: usize, t: f64, y: f64) -> V3 {
        let q = p.run.p(p.sim_i(py_min(t, p.t_ko)), kk);
        to_world(q[0], q[1], y)
    }
}

impl Look for SnareDrumLook {
    fn shards(&self) -> Vec<String> {
        [("shell", 6), ("hoop", 4), ("stick", 2)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["rimshot"]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    fn cfg_summary(&self) -> Option<String> {
        Some(String::new())
    }

    fn setup_as(&mut self, p: &PictureCtx, k: usize, _cfg: &FighterCfg) {
        self.k = k;
        let ev = &p.rec.events;
        self.strokes = state::strokes(ev, k);
        self.flams = state::windows(ev, k, "flam");
        self.armed = state::rim_armed(ev, k);
        self.rims = state::rimshots(ev, k);
        self.buzz = state::windows(ev, k, "buzz_roll");
    }

    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match (e.kind.as_str(), how(e)) {
            ("hit", "rimshot") => Some(0.9),
            ("hit", "stick") if e.i(8) != 0 => Some(0.2),
            ("buzz_roll", _) if e.who() == Some(self.k) => Some(0.4),
            _ => None,
        }
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let col = p.col[owner(e)];
        match how(e) {
            "rimshot" => (CHROME.map(|c| c * 0.6 + 100.0), 10, 2.4, 0.24),
            "buzz" => (col.map(|c| c * 0.6 + 100.0), 2, 1.2, 0.1),
            "flam" => (BEAD.map(|c| c * 0.5 + 100.0), 3, 1.4, 0.12),
            _ => (BEAD.map(|c| c * 0.6 + 100.0), 4, 1.6, 0.14),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "stick" => Some((4.0, 0.12)),
            "rimshot" => Some((9.0, 0.24)),
            _ => None,
        }
    }

    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let col = p.col[self.k];
        // RIMSHOT armed: a quick chrome glint on the ball
        for &t in &self.armed {
            if t > p.t_ko {
                break;
            }
            fx.fl.add(t, self.at(p, self.k, t, R), CHROME, 1.2, 0.6, 0.02, 0.2, 0.0);
        }
        // a landed rimshot: a bright crack ring and a burst of chrome on the enemy
        for &(t, _) in &self.rims {
            if t > p.t_ko {
                break;
            }
            let pos = self.at(p, 1 - self.k, t, R);
            fx.fl.add(t, pos, [255.0, 250.0, 240.0], 2.0, 0.9, 0.01, 0.25, 0.0);
            fx.bu.add(t, pos, CHROME, 1.4, None, true);
            let sparks = Emit {
                n: Some(14),
                spread: Some(180.0),
                col: Some(vec![CHROME, BEAD, col]),
                ..Emit::default()
            };
            fx.pa.emit(t, pos, "sparks", &sparks);
        }
        // BUZZ ROLL: a flare in its colour as the disc spins up
        for &(t, _) in &self.buzz {
            if t > p.t_ko {
                break;
            }
            fx.fl.add(t, self.at(p, self.k, t, R), col, 2.0, 0.7, 0.03, 0.4, 0.0);
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.kind == "hit" && owner(e) == self.k && how(e) == "stick" && e.i(8) != 0 {
            fx.bu.add(
                e.t,
                up(e.f(5), e.f(6), STICK_Y),
                p.col[self.k].map(|c| c * 0.5 + 120.0),
                0.5,
                None,
                true,
            );
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let sparks = Emit {
            n: Some(26),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            col: Some(vec![CHROME, HEAD, p.col[self.k]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
    }

    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _d: &mut Img<f32>) {
        if !f.in_fight {
            return;
        }
        let col = p.col[self.k];
        let flam = state::within(&self.flams, f.t);
        for r in self.live(p, f.t) {
            if r.side == 2 {
                // BUZZ ROLL: six blurred sticks spinning round the ball and a faint rim of the disc
                let spin = f.t * 24.0;
                for j in 0..6 {
                    let a = spin + j as f64 * dmath::PI / 3.0;
                    let tip = (r.x0 + r.ext * dmath::cos(a), r.z0 + r.ext * dmath::sin(a));
                    let mid = (r.x0 + 0.35 * r.ext * dmath::cos(a), r.z0 + 0.35 * r.ext * dmath::sin(a));
                    draw_stick(arr, f.cam, mid, tip, 0.45, Some(col));
                }
                let rim: Vec<V3> = (0..=24)
                    .map(|j| {
                        let a = j as f64 * dmath::PI / 12.0;
                        [r.x0 + r.ext * dmath::cos(a), r.z0 + r.ext * dmath::sin(a), STICK_Y]
                    })
                    .collect();
                let sp = wp(f.cam, &rim);
                let mk = reeds::mask(&[], std::slice::from_ref(&sp), 1);
                reeds::blend(arr, &mk, col, Num::Py(0.5));
                continue;
            }
            if r.ext <= 0.02 {
                continue;
            }
            let (a, b) = ((r.x0, r.z0), (r.x1, r.z1));
            if flam {
                // the grace stick: a faint copy just ahead, a little to the side
                let (dx, dz) = (b.0 - a.0, b.1 - a.1);
                let n = py_max(1e-9, dmath::py_hypot(dx, dz));
                let (ox, oz) = (-dz / n * 0.06, dx / n * 0.06);
                draw_stick(
                    arr,
                    f.cam,
                    (a.0 + ox, a.1 + oz),
                    (b.0 + ox + dx * 0.08, b.1 + oz + dz * 0.08),
                    0.4,
                    None,
                );
            }
            let glow = self.accent_at(f.t, r.side).then_some(col);
            draw_stick(arr, f.cam, a, b, 1.0, glow);
        }
    }

    #[allow(clippy::too_many_arguments)]
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
        let at = |q: [f64; 2]| to_world(q[0], q[1], pos[1]);
        let (ln, wd, col) = match kind {
            "shell" => (0.12, 0.07, p.col[self.k]),
            "hoop" => (0.16, 0.025, CHROME),
            _ => (0.18, 0.02, HICKORY),
        };
        let q4: Vec<V3> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .iter()
            .map(|&(a1, a2)| {
                at([
                    sx + d[0] * ln * a1 * size / 0.1 + nn[0] * wd * a2,
                    sz + d[1] * ln * a1 * size / 0.1 + nn[1] * wd * a2,
                ])
            })
            .collect();
        let sp = ring::screen(cam, &q4);
        let shade_k = 0.75 + 0.25 * dmath::cos(rot * 2.0);
        ring::layer_blend(arr, &[sp], col.map(|c| np::clip(c * shade_k, 0.0, 255.0)), 1.0, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recorded_row_reads() {
        let v = Value::List(vec![
            Value::Int(1),
            Value::Int(0),
            Value::Float(0.5),
            Value::Float(-1.0),
            Value::Float(0.25),
            Value::Float(0.5),
            Value::Float(0.75),
        ]);
        let r = Row::of(&v).expect("a row");
        assert_eq!((r.owner, r.side), (1, 0));
        assert_eq!((r.ext, r.x0, r.z0, r.x1, r.z1), (0.5, -1.0, 0.25, 0.5, 0.75));
        assert!(Row::of(&Value::Int(3)).is_none());
    }
}
