//! BASS GUITAR's look (a Studio fighter: no kit look to match). The ball is its model (an ultraviolet electric bass
//! built from web photos). Over the ring, from the recorded `PICK` rows: its big picks spin round their loops with a
//! fading trail; a FRET SLIDE lights a row of fret bars along its line (bright behind the ball, dim ahead) and a
//! streak follows the dash; a HEADSTOCK SWING is a long maple neck with dark frets and a headstock with four chrome
//! tuners, sweeping round; a SLINGSHOT ties two humming strings from the corner posts to the ball. A landed move
//! cracks a flash and a burst on the enemy. It shatters into body, neck and string pieces.

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
use bf_picture::scene::{R_BALL as R, to_world};

use super::state::{self, Throw};
use crate::fighters::shared::reeds::{self, Num};

/// The same moves with the bold effects.
#[path = "bold.rs"]
mod bold;

/// The neck's maple.
const MAPLE: V3 = [232.0, 196.0, 132.0];
/// The frets and the headstock's dark face.
const DARK: V3 = [60.0, 40.0, 30.0];
/// The tuners' and strings' chrome.
const CHROME: V3 = [214.0, 220.0, 230.0];
/// A lit fret bar, a pick's bright edge.
const HOT: V3 = [255.0, 246.0, 216.0];
/// The height the picks fly and the neck swings at.
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

/// One recorded row: `(owner, type, a, x0, z0, x1, z1)` (see `bf_sim::kinds::pick`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Row {
    owner: usize,
    ty: i64,
    a: f64,
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
            ty: n(1)?,
            a: f(2)?,
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

/// A line in the ring at height `y`, `w` pixels wide.
#[allow(clippy::too_many_arguments)]
fn line(img: &mut Img<f32>, cam: &Cam, a: (f64, f64), b: (f64, f64), y: f64, w: u32, col: V3, alpha: f64) {
    let sp = wp(cam, &[[a.0, a.1, y], [b.0, b.1, y]]);
    let mk = reeds::mask(&[], std::slice::from_ref(&sp), w);
    reeds::blend(img, &mk, col, Num::Py(alpha));
}

/// A filled polygon in the ring at height `y`.
fn poly(img: &mut Img<f32>, cam: &Cam, pts: &[(f64, f64)], y: f64, col: V3, alpha: f64) {
    let q: Vec<V3> = pts.iter().map(|p| [p.0, p.1, y]).collect();
    let sp = wp(cam, &q);
    let mk = reeds::mask(std::slice::from_ref(&sp), &[], 1);
    reeds::blend(img, &mk, col, Num::Py(alpha));
}

/// A pick's outline at `c`, turned `rot`, `s` across: a rounded triangle, the point leading.
fn pick_shape(c: (f64, f64), rot: f64, s: f64) -> Vec<(f64, f64)> {
    // a classic 351 shape: broad shoulders, a rounded point
    let pts = [
        (0.0, -1.0),
        (0.55, -0.85),
        (0.95, -0.35),
        (0.75, 0.25),
        (0.0, 1.05),
        (-0.75, 0.25),
        (-0.95, -0.35),
        (-0.55, -0.85),
    ];
    let (cs, sn) = (dmath::cos(rot), dmath::sin(rot));
    pts.iter()
        .map(|&(x, y)| (c.0 + (x * cs - y * sn) * s * 0.5, c.1 + (x * sn + y * cs) * s * 0.5))
        .collect()
}

/// BASS GUITAR's look.
#[derive(Default)]
pub struct BassGuitarLook {
    k: usize,
    throws: Vec<Throw>,
    slides: Vec<(f64, bool)>,
    swings: Vec<(f64, bool)>,
    slings: Vec<(f64, bool)>,
}

impl std::fmt::Debug for BassGuitarLook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BassGuitarLook")
            .field("k", &self.k)
            .field("throws", &self.throws.len())
            .finish()
    }
}

impl BassGuitarLook {
    /// Its rows of recorded frame `i`.
    fn rows_at(&self, p: &PictureCtx, i: usize) -> Vec<Row> {
        match p.run.nested("PICK", i) {
            Some(Value::List(rows)) => rows.iter().filter_map(Row::of).filter(|r| r.owner == self.k).collect(),
            _ => Vec::new(),
        }
    }

    /// Its rows showing at `t`, and the recorded frame they are from.
    fn live(&self, p: &PictureCtx, t: f64) -> (Vec<Row>, usize) {
        if !(p.show.grid.t_fight <= t && t < p.t_hs) || !p.run.has("PICK") {
            return (Vec::new(), 0);
        }
        let i = p.sim_i(py_min(t, p.t_ko));
        (self.rows_at(p, i), i)
    }

    /// Where ball `kk` is at `t`, in the world, `y` up.
    fn at(&self, p: &PictureCtx, kk: usize, t: f64, y: f64) -> V3 {
        let q = p.run.p(p.sim_i(py_min(t, p.t_ko)), kk);
        to_world(q[0], q[1], y)
    }

    /// Where ball `kk` is at `t`, in the ring's plane.
    fn ring_at(&self, p: &PictureCtx, kk: usize, t: f64) -> (f64, f64) {
        let q = p.run.p(p.sim_i(py_min(t, p.t_ko)), kk);
        (q[0], q[1])
    }

    fn draw_pick(&self, arr: &mut Img<f32>, cam: &Cam, col: V3, r: &Row, alpha: f64) {
        let c = (r.x0, r.z0);
        let shape = pick_shape(c, r.a, 0.26);
        poly(arr, cam, &shape, FLY_Y, col, 0.95 * alpha);
        if alpha > 0.6 {
            let mut edge = shape.clone();
            edge.push(shape[0]);
            let q: Vec<V3> = edge.iter().map(|p| [p.0, p.1, FLY_Y]).collect();
            let sp = wp(cam, &q);
            let mk = reeds::mask(&[], std::slice::from_ref(&sp), 1);
            reeds::blend(arr, &mk, HOT, Num::Py(0.7 * alpha));
        }
    }

    fn draw_frets(&self, arr: &mut Img<f32>, cam: &Cam, col: V3, r: &Row, me: (f64, f64), t: f64) {
        let (a, b) = ((r.x0, r.z0), (r.x1, r.z1));
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        let len = dmath::py_hypot(dx, dz);
        if len < 1e-6 {
            return;
        }
        let (ux, uz) = (dx / len, dz / len);
        let (nx, nz) = (-uz, ux);
        let done = ((me.0 - a.0) * ux + (me.1 - a.1) * uz).max(0.0);
        // while it holds, the bars light up one after another along the line
        let shown = if r.a < 0.5 {
            len * np::clip(t.fract() * 4.0, 0.3, 1.0)
        } else {
            len
        };
        let mut s = 0.25;
        let mut j = 0;
        while s < shown {
            let half = 0.17 - 0.004 * j as f64;
            let c = (a.0 + ux * s, a.1 + uz * s);
            let lit = s <= done + 0.05;
            let (cl, al) = if lit { (HOT, 0.9) } else { (col, 0.55) };
            line(
                arr,
                cam,
                (c.0 - nx * half, c.1 - nz * half),
                (c.0 + nx * half, c.1 + nz * half),
                0.02,
                1,
                cl,
                al,
            );
            // fret spacing shrinks up the neck, as a real one's does
            s += py_max(0.16, 0.32 - 0.012 * j as f64);
            j += 1;
        }
        if r.a > 0.5 && r.a < 1.5 {
            // the dash's streak
            line(arr, cam, a, me, FLY_Y * 0.6, 4, col, 0.3);
        }
    }

    fn draw_neck(&self, arr: &mut Img<f32>, cam: &Cam, r: &Row) {
        let (a, b) = ((r.x0, r.z0), (r.x1, r.z1));
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        let len = dmath::py_hypot(dx, dz);
        if len < 0.05 {
            return;
        }
        let (ux, uz) = (dx / len, dz / len);
        let (nx, nz) = (-uz, ux);
        let w = 0.06;
        // the fingerboard
        poly(
            arr,
            cam,
            &[
                (a.0 - nx * w, a.1 - nz * w),
                (b.0 - nx * w, b.1 - nz * w),
                (b.0 + nx * w, b.1 + nz * w),
                (a.0 + nx * w, a.1 + nz * w),
            ],
            FLY_Y,
            MAPLE,
            1.0,
        );
        // frets, closer together toward the headstock
        let mut s = 0.18;
        let mut j = 0;
        while s < len - 0.05 {
            let c = (a.0 + ux * s, a.1 + uz * s);
            line(
                arr,
                cam,
                (c.0 - nx * w, c.1 - nz * w),
                (c.0 + nx * w, c.1 + nz * w),
                FLY_Y,
                1,
                DARK,
                0.8,
            );
            s += py_max(0.07, 0.16 - 0.008 * j as f64);
            j += 1;
        }
        // the headstock: a dark paddle with four chrome tuners in a line
        let h0 = (b.0, b.1);
        let h1 = (b.0 + ux * 0.22, b.1 + uz * 0.22);
        poly(
            arr,
            cam,
            &[
                (h0.0 - nx * w, h0.1 - nz * w),
                (h1.0 - nx * w * 1.2, h1.1 - nz * w * 1.2),
                (h1.0 + nx * w * 1.6, h1.1 + nz * w * 1.6),
                (h0.0 + nx * w * 1.3, h0.1 + nz * w * 1.3),
            ],
            FLY_Y,
            DARK,
            1.0,
        );
        for j in 0..4 {
            let s = 0.03 + 0.055 * j as f64;
            let c = (h0.0 + ux * s + nx * w * 1.7, h0.1 + uz * s + nz * w * 1.7);
            line(
                arr,
                cam,
                (c.0 - nx * 0.03, c.1 - nz * 0.03),
                (c.0 + nx * 0.03, c.1 + nz * 0.03),
                FLY_Y,
                2,
                CHROME,
                1.0,
            );
        }
    }
}

impl Look for BassGuitarLook {
    /// Its own panel on the VS card ([`crate::fighters::shared::panels::bass_guitar`]).
    fn title_pattern(&self, base: &mut bf_picture::img::RgbF) {
        crate::fighters::shared::panels::bass_guitar(base);
    }

    fn shards(&self) -> Vec<String> {
        [("body", 6), ("neck", 3), ("string", 3)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["headstock", "slingshot"]
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
        self.slides = state::moves(ev, k, "fret_slide");
        self.swings = state::moves(ev, k, "headstock");
        self.slings = state::moves(ev, k, "slingshot");
    }

    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match (e.kind.as_str(), how(e)) {
            ("hit", "slingshot") => Some(1.0),
            ("hit", "headstock") => Some(0.8),
            ("hit", "fret_slide") => Some(0.5),
            _ => None,
        }
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let col = p.col[owner(e)];
        match how(e) {
            "slingshot" | "headstock" => (HOT.map(|c| c * 0.6 + 100.0), 10, 2.4, 0.24),
            "fret_slide" => (col.map(|c| c * 0.6 + 100.0), 7, 2.0, 0.18),
            _ => (HOT.map(|c| c * 0.5 + 100.0), 4, 1.5, 0.13),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "pick" => Some((4.0, 0.12)),
            "fret_slide" => Some((7.0, 0.2)),
            "headstock" | "slingshot" => Some((10.0, 0.26)),
            _ => None,
        }
    }

    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let col = p.col[self.k];
        for (moves, big) in [(&self.slides, 1.2), (&self.swings, 1.6), (&self.slings, 2.0)] {
            for &(t, landed) in moves.iter() {
                if t > p.t_ko {
                    break;
                }
                if !landed {
                    // the move starts: a soft flare in its colour on the ball
                    fx.fl.add(t, self.at(p, self.k, t, R), col, 1.2, 0.5, 0.03, 0.3, 0.0);
                    continue;
                }
                // it lands: a bright flash, a burst and sparks on the enemy
                let pos = self.at(p, 1 - self.k, t, R);
                fx.fl
                    .add(t, pos, [255.0, 250.0, 240.0], 1.2 * big, 0.9, 0.01, 0.25, 0.0);
                fx.bu.add(t, pos, col.map(|c| c * 0.5 + 120.0), 0.9 * big, None, true);
                let sparks = Emit {
                    n: Some((8.0 * big) as i64),
                    spread: Some(180.0),
                    col: Some(vec![HOT, CHROME, col]),
                    ..Emit::default()
                };
                fx.pa.emit(t, pos, "sparks", &sparks);
            }
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.kind == "hit" && owner(e) == self.k && how(e) == "pick" {
            fx.bu.add(
                e.t,
                up(e.f(5), e.f(6), FLY_Y),
                p.col[self.k].map(|c| c * 0.5 + 120.0),
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
            col: Some(vec![CHROME, MAPLE, p.col[self.k]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
    }

    fn state(
        &self,
        p: &PictureCtx,
        kk: usize,
        st: &mut bf_picture::ring::state::BallState,
        t: f64,
        _phase: bf_picture::look::Phase,
        _gl: f64,
    ) {
        if p.bold {
            self.bold_state(p, kk, st, t);
        }
    }

    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &bf_picture::look::CanvasCtx<'_>) {
        if p.bold {
            self.bold_canvas(p, ic, c);
        }
    }

    fn canvas_overlay(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        if p.bold {
            self.bold_canvas_overlay(p, f, arr);
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
            self.bold_draw_item(p, f, fr, payload);
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

    fn bold_hit_size(
        &self,
        _p: &PictureCtx,
        e: &Event,
        by_damage: bf_picture::bold::HitSize,
    ) -> Option<bf_picture::bold::HitSize> {
        Self::bold_hit(e, by_damage)
    }

    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _d: &mut Img<f32>) {
        if p.bold {
            // the bold look draws its moves in the painter pass and on the canvas (`bold.rs`)
            return;
        }
        if !f.in_fight {
            return;
        }
        let col = p.col[self.k];
        let (rows, i) = self.live(p, f.t);
        if rows.is_empty() {
            return;
        }
        let me = self.ring_at(p, self.k, f.t);
        // the picks' trails: where each was a few recorded frames ago
        for back in 1..=3usize {
            if i < back {
                break;
            }
            for r in self.rows_at(p, i - back).iter().filter(|r| r.ty == 0) {
                let fade = 0.32 - 0.09 * back as f64;
                self.draw_pick(arr, f.cam, col, r, fade);
            }
        }
        for r in &rows {
            match r.ty {
                0 => self.draw_pick(arr, f.cam, col, r, 1.0),
                1 if r.a < 2.5 => self.draw_frets(arr, f.cam, col, r, me, f.t),
                2 => self.draw_neck(arr, f.cam, r),
                3 => {
                    // the slingshot's strings, humming harder as they stretch
                    let hum = 0.03 * r.a * dmath::sin(f.t * 90.0);
                    for (ax, az) in [(r.x0, r.z0), (r.x1, r.z1)] {
                        let mid = ((ax + me.0) / 2.0 + hum, (az + me.1) / 2.0 - hum);
                        line(arr, f.cam, (ax, az), mid, FLY_Y * 0.8, 1, CHROME, 0.9);
                        line(arr, f.cam, mid, me, FLY_Y * 0.8, 1, CHROME, 0.9);
                        line(arr, f.cam, (ax, az), me, FLY_Y * 0.8, 3, col, 0.25 + 0.3 * r.a);
                    }
                }
                _ => {}
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
            "body" => (0.12, 0.08, p.col[self.k]),
            "neck" => (0.2, 0.03, MAPLE),
            _ => (0.2, 0.01, CHROME),
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
            Value::Int(2),
            Value::Float(0.5),
            Value::Float(-1.0),
            Value::Float(0.25),
            Value::Float(0.5),
            Value::Float(0.75),
        ]);
        let r = Row::of(&v).expect("a row");
        assert_eq!((r.owner, r.ty), (1, 2));
        assert_eq!((r.a, r.x0, r.z1), (0.5, -1.0, 0.75));
        assert!(Row::of(&Value::Int(3)).is_none());
    }

    #[test]
    fn a_pick_is_a_closed_shape_round_its_centre() {
        let s = pick_shape((1.0, 2.0), 0.3, 0.26);
        assert_eq!(s.len(), 8);
        let (cx, cz) = s.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0 / 8.0, a.1 + p.1 / 8.0));
        assert!((cx - 1.0).abs() < 0.05 && (cz - 2.0).abs() < 0.05);
    }
}
