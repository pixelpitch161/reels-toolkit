//! MARIMBA's look (a Studio fighter: no kit look to match). The ball is its model (a concert marimba built from web
//! photos). Over the ring fly its mallets, from the recorded `MALLETS`: a pale birch shaft and a yarn head in its
//! colour that heats toward gold with every wall bounce (a pair of heads under TREMOLO, small heads for FIVE
//! OCTAVES). A bounce flashes a little tick of light where it met the rope; a mallet that lands bursts; DEAD STROKE
//! arms the ball with a dark ring and a pin drops a dry dust ring round the enemy; FIVE OCTAVES opens with a flare.
//! It shatters into rosewood bars, brass tubes and mallet heads.

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

use super::state::{self, Bounce, End, Throw};
use crate::fighters::shared::reeds::{self, Num};

/// The birch of a mallet's shaft.
const BIRCH: V3 = [232.0, 214.0, 178.0];
/// A hot head (many bounces): gold.
const HOT: V3 = [255.0, 214.0, 96.0];
/// The highlight on a head.
const HI: V3 = [255.0, 246.0, 226.0];
/// Rosewood.
const ROSEWOOD: V3 = [150.0, 70.0, 46.0];
/// The resonators' brass.
const BRASS: V3 = [214.0, 172.0, 84.0];
/// Bounce sparks.
const TICK: [V3; 3] = [[255.0, 236.0, 190.0], [255.0, 214.0, 120.0], [236.0, 200.0, 255.0]];
/// The height a mallet flies at.
const FLY_Y: f64 = R * 0.75;

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

/// One recorded mallet: `(owner, id, x, z, vx, vz, bounces, octave)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Row {
    owner: usize,
    x: f64,
    z: f64,
    vx: f64,
    vz: f64,
    b: i64,
    octave: bool,
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
            x: f(2)?,
            z: f(3)?,
            vx: f(4)?,
            vz: f(5)?,
            b: n(6)?,
            octave: n(7)? != 0,
        })
    }
}

/// A head's colour after `b` bounces: its own colour warming to gold (4 bounces and up are fully hot).
fn heat(col: V3, b: i64) -> V3 {
    let k = py_min(1.0, b as f64 / 4.0);
    std::array::from_fn(|c| np::clip(col[c] * (1.0 - k) + HOT[c] * k, 0.0, 255.0))
}

/// Ring-frame points to the screen.
fn wp(cam: &Cam, pts: &[V3]) -> Vec<(f64, f64)> {
    let world: Vec<V3> = pts.iter().map(|q| to_world(q[0], q[1], q[2])).collect();
    cam.wp(&world)
}

/// A filled disc of radius `rp` pixels at screen point `c`, with a highlight and a dark rim.
fn disc(img: &mut Img<f32>, c: (f64, f64), rp: f64, col: V3) {
    let (x0, x1) = ((c.0 - rp - 1.0) as i64, (c.0 + rp + 2.0) as i64);
    let (y0, y1) = ((c.1 - rp - 1.0) as i64, (c.1 + rp + 2.0) as i64);
    for y in y0.max(0)..y1.min(H as i64) {
        for x in x0.max(0)..x1.min(W as i64) {
            let (dx, dy) = (x as f64 + 0.5 - c.0, y as f64 + 0.5 - c.1);
            let dd = (dx * dx + dy * dy) / (rp * rp);
            let px = img.px_mut(x as usize, y as usize);
            if dd <= 1.0 {
                let hi = dx < -0.25 * rp && dy < -0.25 * rp;
                let c = if hi { HI } else { col };
                for (v, c) in px.iter_mut().zip(c) {
                    *v = c as f32;
                }
            } else if dd <= 1.7 {
                for v in px.iter_mut() {
                    *v *= 0.45;
                }
            }
        }
    }
}

/// One mallet in flight at ring point `(x, z)` heading `(vx, vz)`: the shaft trailing behind the head.
fn draw_mallet(img: &mut Img<f32>, cam: &Cam, x: f64, z: f64, dir: (f64, f64), head: V3, small: bool) {
    let n = py_max(1e-9, dmath::py_hypot(dir.0, dir.1));
    let (ux, uz) = (dir.0 / n, dir.1 / n);
    let len = if small { 0.22 } else { 0.38 };
    let tail = [x - ux * len, z - uz * len, FLY_Y + 0.04];
    let sp = wp(cam, &[[x, z, FLY_Y], tail]);
    let shaft = reeds::mask(&[], std::slice::from_ref(&sp), 1);
    reeds::blend(img, &shaft, BIRCH, Num::Py(0.9));
    let depth = cam.depth(&[to_world(x, z, FLY_Y)])[0];
    let rp = py_max(1.3, if small { 0.07 } else { 0.10 } * cam.f / py_max(1e-6, depth));
    disc(img, sp[0], rp, head);
}

/// MARIMBA's look.
#[derive(Default)]
pub struct MarimbaLook {
    k: usize,
    throws: Vec<Throw>,
    bounces: Vec<Bounce>,
    ends: Vec<End>,
    tremolo: Vec<(f64, f64)>,
    dead: Vec<(f64, f64)>,
    pins: Vec<(f64, f64)>,
    five: Option<f64>,
}

impl std::fmt::Debug for MarimbaLook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarimbaLook")
            .field("k", &self.k)
            .field("throws", &self.throws.len())
            .field("five", &self.five)
            .finish()
    }
}

impl MarimbaLook {
    fn in_window(w: &[(f64, f64)], t: f64) -> bool {
        w.iter().any(|&(a, b)| a <= t && t < b)
    }

    /// Its mallets showing at `t`, from the recorded `MALLETS`.
    fn live(&self, p: &PictureCtx, t: f64) -> Vec<Row> {
        if !(p.show.grid.t_fight <= t && t < p.t_hs) || !p.run.has("MALLETS") {
            return Vec::new();
        }
        let i = p.sim_i(py_min(t, p.t_ko));
        match p.run.nested("MALLETS", i) {
            Some(Value::List(rows)) => rows.iter().filter_map(Row::of).filter(|r| r.owner == self.k).collect(),
            _ => Vec::new(),
        }
    }

    /// Where ball `kk` is at `t`, in the world, `y` up.
    fn at(&self, p: &PictureCtx, kk: usize, t: f64, y: f64) -> V3 {
        let q = p.run.p(p.sim_i(py_min(t, p.t_ko)), kk);
        to_world(q[0], q[1], y)
    }
}

impl Look for MarimbaLook {
    /// Its own panel on the VS card ([`crate::fighters::shared::panels::marimba`]).
    fn title_pattern(&self, base: &mut bf_picture::img::RgbF) {
        crate::fighters::shared::panels::marimba(base);
    }

    fn shards(&self) -> Vec<String> {
        [("bar", 8), ("tube", 4), ("head", 2)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["octave"]
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
        self.throws = state::throws(ev, k);
        self.bounces = state::bounces(ev, k);
        self.ends = state::ends(ev, k);
        self.tremolo = state::windows(ev, k, "tremolo");
        self.dead = state::windows(ev, k, "dead_stroke");
        self.pins = state::pins(ev, k);
        self.five = state::five_octaves(ev, k);
    }

    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match (e.kind.as_str(), how(e)) {
            ("hit", "mallet") => Some(0.25),
            ("pinned", _) if e.i(3) == self.k as i64 => Some(0.8),
            ("five_octaves", _) if e.who() == Some(self.k) => Some(0.6),
            _ => None,
        }
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let col = p.col[owner(e)];
        match how(e) {
            "mallet" | "tremolo" => (heat(col, e.i(8)).map(|c| c * 0.6 + 100.0), 6, 2.0, 0.2),
            "octave" => (HOT.map(|c| c * 0.6 + 100.0), 4, 1.6, 0.16),
            _ => (col.map(|c| c * 0.6 + 100.0), 5, 1.8, 0.18),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "mallet" => Some((6.0, 0.18)),
            "tremolo" => Some((5.0, 0.14)),
            _ => None,
        }
    }

    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let col = p.col[self.k];
        // each wall bounce: a tick of light and a few sparks, warmer as the mallet climbs the scale
        for b in &self.bounces {
            if b.t > p.t_ko {
                break;
            }
            let pos = up(b.x, b.z, FLY_Y);
            fx.fl.add(b.t, pos, heat(col, b.b), 0.9, 0.45, 0.02, 0.14, 0.0);
            let sparks = Emit {
                n: Some(3 + b.b.min(4)),
                col: Some(TICK.to_vec()),
                scale: 0.7,
                ..Emit::default()
            };
            fx.pa.emit(b.t, pos, "sparks", &sparks);
        }
        // a mallet that fizzles: a puff of yarn
        for e in &self.ends {
            if e.t > p.t_ko {
                break;
            }
            if !e.hit {
                let puff = Emit {
                    n: Some(2),
                    col: Some(vec![col, BIRCH]),
                    scale: 0.6,
                    ..Emit::default()
                };
                fx.pa.emit(e.t, up(e.x, e.z, FLY_Y), "dust", &puff);
            }
        }
        // DEAD STROKE: the ball arms (a dark flare); a pin: a dry dust ring round the enemy
        for &(t, _) in &self.dead {
            if t > p.t_ko {
                break;
            }
            fx.fl.add(
                t,
                self.at(p, self.k, t, R),
                col.map(|c| c * 0.5),
                1.6,
                0.6,
                0.03,
                0.3,
                0.0,
            );
        }
        for &(t, _) in &self.pins {
            if t > p.t_ko {
                break;
            }
            let pos = self.at(p, 1 - self.k, t, 0.1);
            let dust = Emit {
                n: Some(12),
                spread: Some(180.0),
                col: Some(vec![[200.0, 186.0, 160.0], [150.0, 136.0, 116.0]]),
                ..Emit::default()
            };
            fx.pa.emit(t, pos, "dust", &dust);
            fx.bu
                .add(t, self.at(p, 1 - self.k, t, R), [235.0, 225.0, 210.0], 1.2, None, false);
        }
        // FIVE OCTAVES: a flare and a burst of gold
        if let Some(t) = self.five.filter(|&t| t <= p.t_ko) {
            let pos = self.at(p, self.k, t, R);
            fx.fl.add(t, pos, HOT, 2.4, 0.9, 0.03, 0.5, 0.0);
            let sparks = Emit {
                n: Some(24),
                spread: Some(180.0),
                col: Some(TICK.to_vec()),
                ..Emit::default()
            };
            fx.pa.emit(t, pos, "sparks", &sparks);
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.kind == "hit" && owner(e) == self.k && how(e) == "mallet" {
            fx.bu.add(
                e.t,
                up(e.f(5), e.f(6), R * 0.6),
                heat(p.col[self.k], e.i(8)),
                0.7,
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
            col: Some(vec![ROSEWOOD, BRASS, p.col[self.k]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
    }

    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _d: &mut Img<f32>) {
        if !f.in_fight {
            return;
        }
        let col = p.col[self.k];
        let trem = Self::in_window(&self.tremolo, f.t);
        let armed = Self::in_window(&self.dead, f.t)
            && !self
                .pins
                .iter()
                .any(|&(t, _)| self.dead.iter().any(|&(a, b)| a <= t && t <= f.t && f.t < b));
        for m in self.live(p, f.t) {
            let head = if m.octave { heat(col, 4) } else { heat(col, m.b) };
            let head = if armed && !m.octave {
                head.map(|c| c * 0.55)
            } else {
                head
            };
            draw_mallet(arr, f.cam, m.x, m.z, (m.vx, m.vz), head, m.octave);
            if trem && !m.octave {
                // TREMOLO: its partner buzzes beside it
                let n = py_max(1e-9, dmath::py_hypot(m.vx, m.vz));
                let wob = 0.09 + 0.03 * dmath::sin(f.t * 90.0);
                let (ox, oz) = (-m.vz / n * wob, m.vx / n * wob);
                draw_mallet(arr, f.cam, m.x + ox, m.z + oz, (m.vx, m.vz), head, false);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
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
        let (sx, sz) = (pos[0], pos[2]);
        let d = [dmath::cos(rot), dmath::sin(rot)];
        let nn = [-d[1], d[0]];
        let at = |q: [f64; 2]| to_world(q[0], q[1], pos[1]);
        let (ln, wd, col) = match kind {
            "bar" => (0.20, 0.05, ROSEWOOD),
            "tube" => (0.16, 0.03, BRASS),
            _ => (0.05, 0.05, BIRCH),
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
    fn a_head_warms_to_gold_with_its_bounces() {
        let col = [160.0, 80.0, 160.0];
        assert_eq!(heat(col, 0), col);
        assert_eq!(heat(col, 4), HOT);
        assert_eq!(heat(col, 9), HOT);
        let mid = heat(col, 2);
        assert!(mid[1] > col[1] && mid[1] < HOT[1]);
    }

    #[test]
    fn a_recorded_row_reads() {
        let v = Value::List(vec![
            Value::Int(1),
            Value::Int(7),
            Value::Float(0.5),
            Value::Float(-1.0),
            Value::Float(3.0),
            Value::Float(4.0),
            Value::Int(2),
            Value::Int(0),
        ]);
        let r = Row::of(&v).expect("a row");
        assert_eq!((r.owner, r.b, r.octave), (1, 2, false));
        assert_eq!((r.x, r.z, r.vx, r.vz), (0.5, -1.0, 3.0, 4.0));
        assert!(Row::of(&Value::Int(3)).is_none());
    }
}
