//! PICCOLO's look (a Studio fighter: no kit look to match). The ball is its model (a grenadilla piccolo with silver
//! keys, built from web photos). Over the ring fly its needles, from the recorded `NEEDLES`: each a streak of light
//! in its colour along the last stretch of its path, folded at every rope it bounced off, with a white-hot head (a
//! wider glow under OVER THE BAND, a brighter one once it has pierced the enemy). A bounce flashes a spark where it
//! met the rope; a pierce bursts white; a shield it cuts through shatters with a ring; GLASS SHATTER cracks the ropes
//! with a flare, and its shards fly in as pale glass triangles. It shatters into black wood, silver keys and glass.

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{py_max, py_min};
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::fx::Emit;
use bf_picture::img::Img;
use bf_picture::look::{FrameCtx, Fx, Look, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::effects::up;
use bf_picture::ring::{self, PictureCtx};
use bf_picture::scene::{A, R_BALL as R, to_world};

use super::state::{self, Bounce, Pierce};
use crate::fighters::shared::reeds::{self, Num};

/// The same moves with the bold effects.
#[path = "bold.rs"]
mod bold;

/// A needle's white-hot head and core.
const WHITE: V3 = [255.0, 255.0, 246.0];
/// The grenadilla body.
const EBONY: V3 = [34.0, 30.0, 34.0];
/// The silver keys.
const SILVER: V3 = [205.0, 210.0, 220.0];
/// Glass.
const GLASS: V3 = [220.0, 244.0, 255.0];
/// Bounce sparks.
const TICK: [V3; 3] = [[255.0, 255.0, 230.0], [255.0, 244.0, 160.0], [220.0, 244.0, 255.0]];
/// The height a needle flies at.
const FLY_Y: f64 = R * 0.7;
/// The needles' radius (`hit_r`): they bounce this far inside the ropes.
const HIT_R: f64 = 0.06;
/// Seconds of flight a needle's streak shows.
const TRAIL_S: f64 = 0.055;

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

/// One recorded needle: `(owner, id, x, z, vx, vz, bounces, shard, hit)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Row {
    owner: usize,
    x: f64,
    z: f64,
    vx: f64,
    vz: f64,
    b: i64,
    shard: bool,
    hit: bool,
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
            shard: n(7)? != 0,
            hit: n(8)? != 0,
        })
    }
}

/// The needle's path back from `(x, z)` for `len` units against its heading, folded at the ropes (at most `folds`
/// times): the streak's points, the head first.
fn trail(x: f64, z: f64, vx: f64, vz: f64, len: f64, folds: i64) -> Vec<[f64; 2]> {
    let lim = A - HIT_R;
    let n = py_max(1e-9, dmath::py_hypot(vx, vz));
    let (mut p, mut d) = ([x, z], [-vx / n, -vz / n]);
    let mut pts = vec![p];
    let mut left = len;
    let mut folds = folds;
    while left > 1e-6 {
        // how far to the next rope along `d`
        let mut s = left;
        let mut ax = None;
        for (i, &di) in d.iter().enumerate() {
            if di.abs() > 1e-9 {
                let wall = if di > 0.0 { lim } else { -lim };
                let si = (wall - p[i]) / di;
                if si >= 0.0 && si < s {
                    s = si;
                    ax = Some(i);
                }
            }
        }
        p = [p[0] + d[0] * s, p[1] + d[1] * s];
        pts.push(p);
        left -= s;
        match ax {
            Some(i) if folds > 0 => {
                d[i] = -d[i];
                folds -= 1;
            }
            _ => break,
        }
    }
    pts
}

/// Ring-frame points to the screen.
fn wp(cam: &Cam, pts: &[V3]) -> Vec<(f64, f64)> {
    let world: Vec<V3> = pts.iter().map(|q| to_world(q[0], q[1], q[2])).collect();
    cam.wp(&world)
}

/// PICCOLO's look.
#[derive(Default)]
pub struct PiccoloLook {
    k: usize,
    bounces: Vec<Bounce>,
    pierces: Vec<Pierce>,
    over: Vec<(f64, f64)>,
    cuts: Vec<(f64, f64, f64)>,
    trills: Vec<f64>,
    shatter: Option<f64>,
    /// What the bold look works out from the recording (empty without the bold effects).
    bold: bold::Bold,
}

impl std::fmt::Debug for PiccoloLook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PiccoloLook")
            .field("k", &self.k)
            .field("pierces", &self.pierces.len())
            .field("shatter", &self.shatter)
            .finish()
    }
}

impl PiccoloLook {
    fn in_window(w: &[(f64, f64)], t: f64) -> bool {
        w.iter().any(|&(a, b)| a <= t && t < b)
    }

    /// Its needles showing at `t`, from the recorded `NEEDLES`.
    fn live(&self, p: &PictureCtx, t: f64) -> Vec<Row> {
        if !(p.show.grid.t_fight <= t && t < p.t_hs) || !p.run.has("NEEDLES") {
            return Vec::new();
        }
        let i = p.sim_i(py_min(t, p.t_ko));
        match p.run.nested("NEEDLES", i) {
            Some(Value::List(rows)) => rows.iter().filter_map(Row::of).filter(|r| r.owner == self.k).collect(),
            _ => Vec::new(),
        }
    }

    /// Where ball `kk` is at `t`, in the world, `y` up.
    fn at(&self, p: &PictureCtx, kk: usize, t: f64, y: f64) -> V3 {
        let q = p.run.p(p.sim_i(py_min(t, p.t_ko)), kk);
        to_world(q[0], q[1], y)
    }

    /// One needle's streak and head.
    fn draw_needle(&self, img: &mut Img<f32>, cam: &Cam, m: &Row, col: V3, over: bool) {
        let speed = dmath::py_hypot(m.vx, m.vz);
        let (len, folds) = if m.shard { (0.16, 0) } else { (speed * TRAIL_S, m.b) };
        let pts: Vec<V3> = trail(m.x, m.z, m.vx, m.vz, len, folds)
            .iter()
            .map(|q| [q[0], q[1], FLY_Y])
            .collect();
        let sp = wp(cam, &pts);
        if m.shard {
            // a glass shard: a small pale triangle pointing where it flies
            let n = py_max(1e-9, speed);
            let (ux, uz) = (m.vx / n, m.vz / n);
            let tri: Vec<V3> = [(0.10, 0.0), (-0.06, 0.05), (-0.06, -0.05)]
                .iter()
                .map(|&(a, b)| [m.x + ux * a - uz * b, m.z + uz * a + ux * b, FLY_Y])
                .collect();
            let tp = ring::screen(cam, &tri.iter().map(|q| to_world(q[0], q[1], q[2])).collect::<Vec<_>>());
            ring::layer_blend(img, &[tp], GLASS.map(|c| c * 0.6 + col[0] * 0.0 + 60.0), 0.9, None);
            let edge = reeds::mask(&[], std::slice::from_ref(&sp), 1);
            reeds::blend(img, &edge, col, Num::Py(0.7));
            return;
        }
        // the glow, then the streak in its colour, then a white core and head
        let glow = reeds::mask(&[], std::slice::from_ref(&sp), if over { 5 } else { 3 });
        reeds::blend(img, &glow, col, Num::Py(if over { 0.55 } else { 0.35 }));
        let streak = reeds::mask(&[], std::slice::from_ref(&sp), 2);
        reeds::blend(img, &streak, col, Num::Py(0.95));
        let head: Vec<(f64, f64)> = sp.iter().take(2).copied().collect();
        let core = reeds::mask(&[], std::slice::from_ref(&head), 1);
        reeds::blend(img, &core, WHITE, Num::Py(if m.hit { 1.0 } else { 0.85 }));
    }
}

impl Look for PiccoloLook {
    /// Its own panel on the VS card ([`crate::fighters::shared::panels::piccolo`]).
    fn title_pattern(&self, base: &mut bf_picture::img::RgbF) {
        crate::fighters::shared::panels::piccolo(base);
    }

    fn shards(&self) -> Vec<String> {
        [("wood", 6), ("key", 4), ("glass", 4)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["shard"]
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
        self.bounces = state::bounces(ev, k);
        self.pierces = state::pierces(ev, k);
        self.over = state::over_band(ev, k);
        self.cuts = state::shield_cuts(ev, k);
        self.trills = state::trills(ev, k);
        self.shatter = state::shatter(ev, k);
        if p.bold {
            self.bold = Self::bold_setup(ev, k);
        }
    }

    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match (e.kind.as_str(), how(e)) {
            ("hit", "needle") => Some(0.15),
            ("pierce", _) if e.who() == Some(self.k) => Some(0.5),
            ("shatter", _) if e.who() == Some(self.k) => Some(0.9),
            _ => None,
        }
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let col = p.col[owner(e)];
        match how(e) {
            "needle" => (WHITE.map(|c| c * 0.5 + 120.0), 5, 1.6, 0.16),
            "shard" => (GLASS.map(|c| c * 0.6 + 90.0), 4, 1.4, 0.14),
            _ => (col.map(|c| c * 0.6 + 100.0), 5, 1.8, 0.18),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "needle" => Some((5.0, 0.14)),
            "shard" => Some((3.0, 0.1)),
            _ => None,
        }
    }

    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let col = p.col[self.k];
        // each rope bounce: a pin-point of light and a couple of sparks
        for b in &self.bounces {
            if b.t > p.t_ko {
                break;
            }
            let pos = up(b.x, b.z, FLY_Y);
            fx.fl.add(b.t, pos, col, 0.7, 0.4, 0.015, 0.1, 0.0);
            let sparks = Emit {
                n: Some(2 + b.b.min(3)),
                col: Some(TICK.to_vec()),
                scale: 0.6,
                ..Emit::default()
            };
            fx.pa.emit(b.t, pos, "sparks", &sparks);
        }
        // a needle through the enemy: a white burst where it went in
        for h in &self.pierces {
            if h.t > p.t_ko {
                break;
            }
            fx.bu.add(h.t, up(h.x, h.z, FLY_Y), WHITE, 0.6, None, true);
        }
        // TRILL: a quick double flare on the ball
        for &t in &self.trills {
            if t > p.t_ko {
                break;
            }
            let pos = self.at(p, self.k, t, R);
            fx.fl.add(t, pos, col, 1.4, 0.6, 0.02, 0.2, 0.0);
        }
        // OVER THE BAND: a bright ring on the ball as it starts; each shield cut: a glassy ring and sparks
        for &(t, _) in &self.over {
            if t > p.t_ko {
                break;
            }
            fx.bu.add(t, self.at(p, self.k, t, R), col, 1.4, None, false);
        }
        for &(t, x, z) in &self.cuts {
            if t > p.t_ko {
                break;
            }
            let pos = up(x, z, R);
            fx.bu.add(t, pos, GLASS, 1.3, None, false);
            let sparks = Emit {
                n: Some(10),
                spread: Some(180.0),
                col: Some(vec![GLASS, WHITE, col]),
                ..Emit::default()
            };
            fx.pa.emit(t, pos, "sparks", &sparks);
        }
        // GLASS SHATTER: the ball's flare and a spray of glass off the four ropes
        if let Some(t) = self.shatter.filter(|&t| t <= p.t_ko) {
            let pos = self.at(p, self.k, t, R);
            fx.fl.add(t, pos, WHITE, 2.4, 0.9, 0.03, 0.5, 0.0);
            for (x, z) in [(A, 0.0), (-A, 0.0), (0.0, A), (0.0, -A)] {
                let glass = Emit {
                    n: Some(8),
                    spread: Some(180.0),
                    col: Some(vec![GLASS, WHITE]),
                    ..Emit::default()
                };
                fx.pa.emit(t, up(x, z, FLY_Y), "sparks", &glass);
            }
        }
    }

    fn fx_event(&self, _p: &PictureCtx, _fx: &mut Fx, _e: &Event) {}

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let sparks = Emit {
            n: Some(26),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            col: Some(vec![SILVER, GLASS, p.col[self.k]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
    }

    fn after_grade(
        &self,
        p: &PictureCtx,
        img: &mut bf_picture::look::Pic,
        f: &FrameCtx<'_>,
        drawn: &[bf_picture::look::DrawnBall],
    ) {
        if p.bold {
            self.bold_after_grade(p, img, f, drawn);
        }
    }

    fn rope_events(&self, p: &PictureCtx) -> Vec<bf_picture::ring::ropes::RopeEv> {
        if p.bold {
            return self.bold_rope_events();
        }
        Vec::new()
    }

    fn bold_hit_size(
        &self,
        _p: &PictureCtx,
        e: &Event,
        by_damage: bf_picture::bold::HitSize,
    ) -> Option<bf_picture::bold::HitSize> {
        self.bold_hit(e, by_damage)
    }

    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _d: &mut Img<f32>) {
        if p.bold {
            // the needles are drawn crisp after the grade (`bold_after_grade`)
            return;
        }
        if !f.in_fight {
            return;
        }
        let col = p.col[self.k];
        let over = Self::in_window(&self.over, f.t);
        for m in self.live(p, f.t) {
            self.draw_needle(arr, f.cam, &m, col, over);
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
            "wood" => (0.16, 0.035, EBONY),
            "key" => (0.05, 0.03, SILVER),
            _ => (0.07, 0.05, GLASS),
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
    fn a_streak_folds_at_the_rope() {
        let lim = A - HIT_R;
        // heading +x, just bounced off the left rope: its trail runs back to that rope and folds once
        let pts = trail(-lim + 0.2, 0.0, 10.0, 0.0, 0.5, 1);
        assert_eq!(pts.len(), 3);
        assert!((pts[1][0] + lim).abs() < 1e-9);
        assert!((pts[2][0] - (-lim + 0.3)).abs() < 1e-9);
        // no folds left: it stops at the rope
        assert_eq!(trail(-lim + 0.2, 0.0, 10.0, 0.0, 0.5, 0).len(), 2);
        // a straight stretch
        let pts = trail(0.0, 0.0, 0.0, 5.0, 0.4, 3);
        assert_eq!(pts.len(), 2);
        assert!((pts[1][1] + 0.4).abs() < 1e-9);
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
            Value::Int(1),
        ]);
        let r = Row::of(&v).expect("a row");
        assert_eq!((r.owner, r.b, r.shard, r.hit), (1, 2, false, true));
        assert_eq!((r.x, r.z, r.vx, r.vz), (0.5, -1.0, 3.0, 4.0));
        assert!(Row::of(&Value::Int(3)).is_none());
    }
}
