//! OCTOBASS's look (a Studio fighter: no kit look to match). The ball is its model (a deep-umber octobass built from
//! web photos). Over the ring its lever arms punch out from the recorded `LEVERS` rows: a dark wood rod edged in
//! brass, a knuckle that folds while the arm is short, a dark red felt fist, and a thud burst where a fist stops on
//! the enemy. FOOT PEDAL is a brass plate on the floor under the enemy that rises as it is about to snap, and a
//! landed snap pops the enemy into the air; SEVEN LEVERS is a fan of seven arms; BELOW HEARING darkens the ring while
//! one wide slow ring grows from the bass, then a flash and a big burst when it lands. It breaks into wood, string
//! and lever pieces.

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{py_max, py_min};
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{H, W};
use bf_picture::fx::Emit;
use bf_picture::img::Img;
use bf_picture::look::{FrameCtx, Fx, Look, Phase, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::state::BallState;
use bf_picture::ring::{self, PictureCtx};
use bf_picture::scene::{R_BALL as R, to_world};

use super::state;
use crate::fighters::shared::reeds::{self, Num};

/// The same moves with the bold effects.
#[path = "bold.rs"]
mod bold;

/// The arms' dark wood.
const WOOD: V3 = [70.0, 44.0, 24.0];
/// The brass edge of an arm and the pedal plate.
const BRASS: V3 = [227.0, 192.0, 106.0];
/// The felt of a fist.
const FELT: V3 = [124.0, 43.0, 34.0];
/// The wood's light varnish (the ring of BELOW HEARING).
const VARNISH: V3 = [217.0, 160.0, 102.0];
/// A bright flash.
const HOT: V3 = [255.0, 241.0, 201.0];
/// The height the arms punch at.
const ARM_Y: f64 = R * 0.7;
/// How long a pedal-popped enemy is in the air.
const AIR_S: f64 = 0.55;

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

/// One recorded row: `(owner, kind, a, x0, z0, b, c, d)`. Kind 0 / 1 is an arm (`a` how far out, `x0, z0` its
/// butt, `b, c` its tip, `d` 1 when held on the enemy); 2 a pedal (`a` the time left 0..1, `x0, z0` where, `b` its
/// radius); 3 a BELOW HEARING ring (`a` its age, `x0, z0` its centre, `b` its radius).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Row {
    owner: usize,
    kind: i64,
    a: f64,
    x0: f64,
    z0: f64,
    b: f64,
    c: f64,
    d: f64,
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
            kind: n(1)?,
            a: f(2)?,
            x0: f(3)?,
            z0: f(4)?,
            b: f(5)?,
            c: f(6)?,
            d: f(7)?,
        })
    }
}

/// Ring-frame points to the screen.
fn wp(cam: &Cam, pts: &[V3]) -> Vec<(f64, f64)> {
    let world: Vec<V3> = pts.iter().map(|q| to_world(q[0], q[1], q[2])).collect();
    cam.wp(&world)
}

/// A filled disc of radius `rp` pixels at screen point `c`, with a dark rim.
fn disc(img: &mut Img<f32>, c: (f64, f64), rp: f64, col: V3) {
    let (x0, x1) = ((c.0 - rp - 2.0) as i64, (c.0 + rp + 3.0) as i64);
    let (y0, y1) = ((c.1 - rp - 2.0) as i64, (c.1 + rp + 3.0) as i64);
    for y in y0.max(0)..y1.min(H as i64) {
        for x in x0.max(0)..x1.min(W as i64) {
            let (dx, dy) = (x as f64 + 0.5 - c.0, y as f64 + 0.5 - c.1);
            let dd = (dx * dx + dy * dy) / (rp * rp);
            let px = img.px_mut(x as usize, y as usize);
            if dd <= 0.35 {
                for (v, c) in px.iter_mut().zip(col) {
                    *v = (c * 1.25).min(255.0) as f32;
                }
            } else if dd <= 1.0 {
                for (v, c) in px.iter_mut().zip(col) {
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

/// A line through ring points at height `y`, `width` pixels, blended in `col`.
fn line(img: &mut Img<f32>, cam: &Cam, pts: &[(f64, f64)], y: f64, width: u32, col: V3, alpha: f64) {
    let q: Vec<V3> = pts.iter().map(|&(x, z)| [x, z, y]).collect();
    let sp = wp(cam, &q);
    let mk = reeds::mask(&[], std::slice::from_ref(&sp), width);
    reeds::blend(img, &mk, col, Num::Py(alpha));
}

/// One lever arm from its butt `a` to its fist `b`, `ext` out (the knuckle folds while it is short).
fn draw_arm(img: &mut Img<f32>, cam: &Cam, a: (f64, f64), b: (f64, f64), ext: f64, flip: f64) {
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let n = py_max(1e-9, dmath::py_hypot(dx, dz));
    let fold = (1.0 - ext.clamp(0.0, 1.0)) * 0.16 * flip;
    let knee = ((a.0 + b.0) / 2.0 - dz / n * fold, (a.1 + b.1) / 2.0 + dx / n * fold);
    let pts = [a, knee, b];
    line(img, cam, &pts, ARM_Y, 5, WOOD, 1.0);
    line(img, cam, &pts, ARM_Y, 2, BRASS, 0.95);
    let depth = |q: (f64, f64)| cam.depth(&[to_world(q.0, q.1, ARM_Y)])[0];
    let kp = wp(cam, &[[knee.0, knee.1, ARM_Y]])[0];
    disc(img, kp, py_max(1.0, 0.05 * cam.f / py_max(1e-6, depth(knee))), WOOD);
    let bp = wp(cam, &[[b.0, b.1, ARM_Y]])[0];
    disc(img, bp, py_max(2.0, 0.1 * cam.f / py_max(1e-6, depth(b))), FELT);
}

/// OCTOBASS's look.
#[derive(Default)]
pub struct OctobassLook {
    k: usize,
    /// When its pedals snapped and landed.
    pops: Vec<f64>,
    /// Its BELOW HEARING windows.
    quiet: Vec<(f64, f64)>,
    /// `(t, x, z)` of its pedals' snaps.
    pedals: Vec<(f64, f64, f64)>,
    /// When its rings landed.
    booms: Vec<f64>,
    /// What the bold look works out from the recording (empty without the bold effects).
    bold: bold::Bold,
}

impl std::fmt::Debug for OctobassLook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OctobassLook")
            .field("k", &self.k)
            .field("pops", &self.pops.len())
            .finish()
    }
}

impl OctobassLook {
    /// Its rows showing at `t`, from the recorded `LEVERS`.
    fn live(&self, p: &PictureCtx, t: f64) -> Vec<Row> {
        if !(p.show.grid.t_fight <= t && t < p.t_hs) || !p.run.has("LEVERS") {
            return Vec::new();
        }
        let i = p.sim_i(py_min(t, p.t_ko));
        match p.run.nested("LEVERS", i) {
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

impl Look for OctobassLook {
    /// Its own panel on the VS card ([`crate::fighters::shared::panels::octobass`]).
    fn title_pattern(&self, base: &mut bf_picture::img::RgbF) {
        crate::fighters::shared::panels::octobass(base);
    }

    fn shards(&self) -> Vec<String> {
        [("wood", 6), ("lever", 3), ("string", 2)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["below", "pedal"]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    fn cfg_summary(&self) -> Option<String> {
        Some(String::new())
    }

    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        let ev = &p.rec.events;
        self.pops = ev
            .iter()
            .filter(|e| e.kind == "pedal_snap" && e.who() == Some(k) && e.i(3) == 1)
            .map(|e| e.t)
            .collect();
        self.quiet = state::silences(ev, k, p.t_ko);
        self.pedals = state::pedals(ev, k);
        self.booms = ev
            .iter()
            .filter(|e| e.kind == "below_hit" && e.who() == Some(k) && e.i(3) == 1)
            .map(|e| e.t)
            .collect();
        if p.bold {
            self.bold = Self::bold_setup(p, k, cfg);
        }
    }

    /// A pedal-popped enemy flies up and drops back.
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, _phase: Phase, _gl: f64) {
        if p.bold {
            return self.bold_state(p, kk, st, t);
        }
        if kk == self.k {
            return;
        }
        for &t0 in &self.pops {
            let u = (t - t0) / AIR_S;
            if (0.0..1.0).contains(&u) {
                st.lift = py_max(st.lift, R * 1.3 * dmath::sin(dmath::PI * u));
            }
        }
    }

    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match (e.kind.as_str(), how(e)) {
            ("hit", "below") => Some(1.4),
            ("hit", "pedal") => Some(0.5),
            ("hit", "seven") => Some(0.25),
            ("hit", "lever") => Some(0.15),
            _ => None,
        }
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let col = p.col[owner(e)];
        match how(e) {
            "below" => (HOT, 14, 2.8, 0.3),
            "pedal" => (BRASS.map(|c| c * 0.6 + 100.0), 8, 2.0, 0.2),
            "seven" => (FELT.map(|c| c * 0.5 + 120.0), 3, 1.4, 0.12),
            _ => (col.map(|c| c * 0.5 + 120.0), 4, 1.6, 0.14),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "lever" => Some((4.0, 0.12)),
            "pedal" => Some((7.0, 0.2)),
            "below" => Some((12.0, 0.3)),
            _ => None,
        }
    }

    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let col = p.col[self.k];
        // a pedal snapping up: a brass burst on the floor where it was
        for &(t, x, z) in &self.pedals {
            if t > p.t_ko {
                break;
            }
            let pos = to_world(x, z, R * 0.2);
            fx.bu.add(t, pos, BRASS, 1.0, None, true);
        }
        // a landed ring: a big light flash and a burst of wood and light on the enemy
        for &t in &self.booms {
            if t > p.t_ko {
                break;
            }
            let pos = self.at(p, 1 - self.k, t, R);
            fx.fl.add(t, pos, HOT, 3.2, 1.0, 0.01, 0.45, 0.0);
            fx.bu.add(t, pos, VARNISH, 2.0, None, true);
            let sparks = Emit {
                n: Some(22),
                spread: Some(180.0),
                col: Some(vec![HOT, VARNISH, col]),
                ..Emit::default()
            };
            fx.pa.emit(t, pos, "sparks", &sparks);
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        // a fist stopping on the enemy: a small thud burst at the contact
        if e.kind == "hit" && owner(e) == self.k && matches!(how(e), "lever" | "seven") {
            fx.bu.add(
                e.t,
                to_world(e.f(5), e.f(6), ARM_Y),
                p.col[self.k].map(|c| c * 0.4 + 140.0),
                0.45,
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
            col: Some(vec![VARNISH, BRASS, p.col[self.k]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
    }

    fn ball(&self, p: &PictureCtx, kk: usize, _st: &BallState, t: f64, glass: [f64; 3], roll: f64) -> ([f64; 3], f64) {
        if p.bold {
            return (glass, self.bold_roll(kk, t, roll));
        }
        (glass, roll)
    }

    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &bf_picture::look::CanvasCtx<'_>) {
        if p.bold {
            self.bold_canvas(p, ic, c);
        }
    }

    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, bf_picture::look::Payload)> {
        if p.bold {
            return self.bold_items_free(p, f);
        }
        Vec::new()
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &bf_picture::look::Payload) {
        if p.bold {
            self.bold_draw_item(f, fr, payload);
        }
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

    fn bold_status(&self, _p: &PictureCtx, t: f64) -> Vec<bf_picture::bold::Status> {
        self.bold_statuses(t)
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
            return self.bold_over_ring(f, arr);
        }
        if !f.in_fight {
            return;
        }
        let rows = self.live(p, f.t);
        // BELOW HEARING: the whole ring sinks into the dark while the ring grows
        if state::within(&self.quiet, f.t) {
            let since = self
                .quiet
                .iter()
                .find(|&&(a, b)| a <= f.t && f.t < b)
                .map_or(0.0, |&(a, _)| f.t - a);
            let k = (1.0 - 0.5 * (since / 0.3).clamp(0.0, 1.0)) as f32;
            for v in arr.data.iter_mut() {
                *v *= k;
            }
        }
        for r in &rows {
            match r.kind {
                2 => {
                    // the pedal plate: a brass square on the floor, rising as it is about to snap
                    let rise = 1.0 - r.a;
                    let s = r.b * 0.9;
                    let y = R * (0.05 + 0.35 * rise * rise);
                    let pts = [
                        (r.x0 - s, r.z0 - s * 0.5),
                        (r.x0 + s, r.z0 - s * 0.5),
                        (r.x0 + s, r.z0 + s * 0.5),
                        (r.x0 - s, r.z0 + s * 0.5),
                        (r.x0 - s, r.z0 - s * 0.5),
                    ];
                    let q: Vec<V3> = pts.iter().map(|&(x, z)| [x, z, y]).collect();
                    let sp = wp(f.cam, &q);
                    let fill = reeds::mask(std::slice::from_ref(&sp), &[], 1);
                    reeds::blend(arr, &fill, BRASS, Num::Py(0.35 + 0.5 * rise));
                    line(arr, f.cam, &pts, y, 1, WOOD, 0.9);
                }
                3 => {
                    // the slow ring: wide, varnish-coloured, a hot thin line on it
                    let pts: Vec<(f64, f64)> = (0..=48)
                        .map(|j| {
                            let a = j as f64 * dmath::PI / 24.0;
                            (r.x0 + r.b * dmath::cos(a), r.z0 + r.b * dmath::sin(a))
                        })
                        .collect();
                    line(arr, f.cam, &pts, R * 0.1, 6, VARNISH, 0.7);
                    line(arr, f.cam, &pts, R * 0.1, 1, HOT, 0.9);
                }
                _ => {
                    if r.a <= 0.02 {
                        continue;
                    }
                    let flip = if dmath::sin(r.x0 * 7.0 + r.z0 * 3.0) > 0.0 {
                        1.0
                    } else {
                        -1.0
                    };
                    draw_arm(arr, f.cam, (r.x0, r.z0), (r.b, r.c), r.a, flip);
                }
            }
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
            "wood" => (0.14, 0.08, p.col[self.k]),
            "lever" => (0.16, 0.03, BRASS),
            _ => (0.22, 0.012, [236.0, 226.0, 204.0]),
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
            Value::Float(1.0),
        ]);
        let r = Row::of(&v).expect("a row");
        assert_eq!((r.owner, r.kind), (1, 0));
        assert_eq!((r.a, r.x0, r.z0, r.b, r.c, r.d), (0.5, -1.0, 0.25, 0.5, 0.75, 1.0));
        assert!(Row::of(&Value::Int(3)).is_none());
    }
}
