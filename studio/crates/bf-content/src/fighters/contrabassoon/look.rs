//! CONTRABASSOON's look (made from scratch, Nov 16, 2026): the folded tube in the ball is its model; in the ring it
//! draws its pipes from the run's events.
//!
//! What it draws: every U-BEND pipe as a dark wooden tube with a silver bend at its bottom, sliding out over
//! `DRAW_IN` when laid, flashing when the enemy bounces off it and fading when it is dropped; the pipes glow hot
//! orange while HOT PIPES lasts; WATER KEY's spray as blue droplets fanning out of every pipe end and the ball; the
//! low move's box as a silver square of tube round the enemy. The ball thumps on every pipe it lays.

use bf_core::dmath::{self, PI};
use bf_core::event::Event;
use bf_core::py::py_min;
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_picture::look::{FrameCtx, Look, Payload, Phase, Spark};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::BallState;
use bf_picture::scene;

use crate::fighters::shared::older::{Poly, blend_into, mask_of_lines, mask_of_polys};
use crate::fighters::shared::reeds::Num;

/// The same moves with the bold effects.
#[path = "bold.rs"]
mod bold;

/// The tube's centre height above the floor.
const PIPE_Y: f64 = 0.14;
/// The tube's drawn radius (world units).
const PIPE_R: f64 = 0.11;
/// Seconds a new pipe takes to slide out.
const DRAW_IN: f64 = 0.16;
/// Seconds a dropped pipe takes to fade.
const FADE_OUT: f64 = 0.35;
/// The dark maple of the tube.
const WOOD: V3 = [108.0, 40.0, 30.0];
/// Its outline.
const WOOD_DK: V3 = [44.0, 14.0, 12.0];
/// The silver of the bends and the box.
const SILVER: V3 = [214.0, 218.0, 228.0];
/// HOT PIPES' glow.
const HOT: V3 = [255.0, 132.0, 40.0];
/// WATER KEY's droplets.
const WATER: V3 = [140.0, 206.0, 255.0];

/// A pipe as the look rebuilds it from the run.
#[derive(Clone, Debug, PartialEq)]
struct PipeL {
    id: i64,
    t0: f64,
    pts: Vec<[f64; 2]>,
    t_off: Option<f64>,
    hits: Vec<f64>,
    boxed: bool,
}

/// A spray: when, and where from (the ball is added when drawn).
#[derive(Clone, Debug, PartialEq)]
struct SprayL {
    t0: f64,
    from: Vec<[f64; 2]>,
}

/// What the look draws in the painter's list.
enum Item {
    /// A pipe: its world points, alpha, flash, heat and whether it is the box.
    Pipe(Vec<V3>, f64, f64, f64, bool),
    /// A spray's droplets at `(x, z)`, its age 0..1.
    Spray(f64, f64, f64),
}

/// `smoothstep(a, b, x)`.
fn ss(a: f64, b: f64, x: f64) -> f64 {
    let u = ((x - a) / (b - a)).clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// `e[7]` of a hit.
fn how(e: &Event) -> &str {
    if e.fields.len() > 5 { e.s(7) } else { "" }
}

/// The first `frac` of a polyline's length.
fn part_of(pts: &[[f64; 2]], frac: f64) -> Vec<[f64; 2]> {
    let lens: Vec<f64> = pts
        .windows(2)
        .map(|w| dmath::hypot(w[1][0] - w[0][0], w[1][1] - w[0][1]))
        .collect();
    let mut left = lens.iter().sum::<f64>() * frac.clamp(0.0, 1.0);
    let mut out = vec![pts[0]];
    for (w, &l) in pts.windows(2).zip(&lens) {
        if left >= l {
            out.push(w[1]);
            left -= l;
        } else {
            let u = if l > 1e-12 { left / l } else { 0.0 };
            out.push([w[0][0] + (w[1][0] - w[0][0]) * u, w[0][1] + (w[1][1] - w[0][1]) * u]);
            break;
        }
    }
    out
}

/// CONTRABASSOON's look.
#[derive(Clone, Debug, Default)]
pub struct ContrabassoonLook {
    k: usize,
    pipes: Vec<PipeL>,
    sprays: Vec<SprayL>,
    /// When each pipe was laid, in order (the ball's thump).
    laid: Vec<f64>,
    /// HOT PIPES: `(start, seconds)`.
    hot: Vec<(f64, f64)>,
}

impl ContrabassoonLook {
    /// A pipe's `(alpha, drawn fraction)` at `t`, or `None` when it is not on screen.
    fn pipe_state(p: &PipeL, t: f64) -> Option<(f64, f64)> {
        if t < p.t0 {
            return None;
        }
        let drawn = py_min(1.0, (t - p.t0) / DRAW_IN);
        let mut a = 1.0;
        if let Some(off) = p.t_off
            && t >= off
        {
            a = 1.0 - (t - off) / FADE_OUT;
            if a <= 0.0 {
                return None;
            }
        }
        Some((a, drawn))
    }

    /// How hot the pipes are at `t` (0..1).
    fn heat(&self, t: f64) -> f64 {
        self.hot
            .iter()
            .filter(|(t0, d)| t >= *t0 && t < t0 + d)
            .map(|(t0, d)| py_min(ss(*t0, t0 + 0.25, t), 1.0 - ss(t0 + d - 0.4, t0 + d, t)))
            .fold(0.0, f64::max)
    }
}

/// Draws one pipe: an outline, the wooden body, a highlight; the U's bottom (or the whole box) in silver.
fn draw_pipe(arr: &mut Img<f32>, cam: &Cam, pts: &[V3], alpha: f64, flash: f64, heat: f64, boxed: bool) {
    if pts.len() < 2 {
        return;
    }
    let d = cam.depth(pts).iter().sum::<f64>() / pts.len() as f64;
    let w = ((2.0 * PIPE_R * cam.f / d.max(1e-6)).round() as u32).max(3);
    let all: Poly = cam.wp(pts);
    let bend: Vec<Poly> = if boxed {
        vec![all.clone()]
    } else if pts.len() >= 3 {
        vec![cam.wp(&pts[1..3])]
    } else {
        Vec::new()
    };
    blend_into(
        arr,
        &mask_of_lines(std::slice::from_ref(&all), Some(w + 2)),
        WOOD_DK,
        Num::Py(alpha),
    );
    if !boxed {
        let body = WOOD.map(|c| (c + 120.0 * flash).clamp(0.0, 255.0));
        blend_into(
            arr,
            &mask_of_lines(std::slice::from_ref(&all), Some(w)),
            body,
            Num::Py(alpha),
        );
    }
    if !bend.is_empty() {
        let col = SILVER.map(|c| (c + 40.0 * flash).clamp(0.0, 255.0));
        blend_into(arr, &mask_of_lines(&bend, Some(w)), col, Num::Py(alpha));
    }
    blend_into(
        arr,
        &mask_of_lines(std::slice::from_ref(&all), Some(1)),
        [255.0, 236.0, 220.0],
        Num::Py(alpha * 0.45),
    );
    if heat > 0.0 {
        blend_into(
            arr,
            &mask_of_lines(std::slice::from_ref(&all), Some(w + 4)),
            HOT,
            Num::Py(alpha * 0.45 * heat),
        );
        blend_into(
            arr,
            &mask_of_lines(std::slice::from_ref(&all), Some(w.saturating_sub(2).max(1))),
            HOT,
            Num::Py(alpha * 0.7 * heat),
        );
    }
}

/// Draws a spray's droplets: twelve drops flying out of `(x, z)` and falling, `age` 0..1 of its beat.
fn draw_spray(arr: &mut Img<f32>, cam: &Cam, x: f64, z: f64, age: f64) {
    let r = 0.9 * ss(0.0, 1.0, age);
    let a = 0.9 * (1.0 - age);
    let polys: Vec<Poly> = (0..12)
        .map(|j| {
            let ang = 2.0 * PI * j as f64 / 12.0 + 0.3 * (j % 3) as f64;
            let rr = r * (0.7 + 0.3 * ((j * 7) % 5) as f64 / 4.0);
            let (cx, cz) = (x + rr * dmath::cos(ang), z + rr * dmath::sin(ang));
            let y = PIPE_Y + 0.5 * age * (1.0 - age) * 2.0;
            let q = cam.wp(&[scene::to_world(cx, cz, y)])[0];
            let s = 2.0;
            vec![(q.0 - s, q.1), (q.0, q.1 - s), (q.0 + s, q.1), (q.0, q.1 + s)]
        })
        .collect();
    blend_into(arr, &mask_of_polys(&polys), WATER, Num::Py(a));
}

impl Look for ContrabassoonLook {
    /// Its own panel on the VS card ([`crate::fighters::shared::panels::contrabassoon`]).
    fn title_pattern(&self, base: &mut bf_picture::img::RgbF) {
        crate::fighters::shared::panels::contrabassoon(base);
    }

    /// Its shatter parts: maple sticks, silver keys and bends, the reed.
    fn shards(&self) -> Vec<String> {
        [("wood", 7), ("key", 6), ("reed", 2)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["pipe_box"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["pipe_box"]
    }

    fn block_spark(&self) -> Spark {
        ([230.0, 232.0, 240.0], 10, 2.4, 0.3)
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.k = k;
        let ev = &p.rec.events;
        let mut pipes: Vec<PipeL> = Vec::new();
        for e in ev.iter().filter(|e| e.who() == Some(k)) {
            match e.kind.as_str() {
                "pipe" => pipes.push(PipeL {
                    id: e.i(3),
                    t0: e.t,
                    pts: (0..4).map(|j| [e.f(4 + 2 * j), e.f(5 + 2 * j)]).collect(),
                    t_off: None,
                    hits: Vec::new(),
                    boxed: false,
                }),
                "pipe_box" => {
                    let (cx, cz, h) = (e.f(4), e.f(5), e.f(6));
                    pipes.push(PipeL {
                        id: e.i(3),
                        t0: e.t,
                        pts: vec![
                            [cx - h, cz - h],
                            [cx + h, cz - h],
                            [cx + h, cz + h],
                            [cx - h, cz + h],
                            [cx - h, cz - h],
                        ],
                        t_off: None,
                        hits: Vec::new(),
                        boxed: true,
                    });
                }
                "pipe_off" | "pipe_box_off" => {
                    let id = e.i(3);
                    if let Some(pp) = pipes.iter_mut().find(|pp| pp.id == id) {
                        pp.t_off = Some(e.t);
                    }
                }
                "hit" if matches!(how(e), "pipe" | "pipe_box") => {
                    let id = e.i(8);
                    if let Some(pp) = pipes.iter_mut().find(|pp| pp.id == id) {
                        pp.hits.push(e.t);
                    }
                }
                "spray" => {
                    let n = (e.fields.len().saturating_sub(2)) / 2;
                    self.sprays.push(SprayL {
                        t0: e.t,
                        from: (0..n).map(|j| [e.f(4 + 2 * j), e.f(5 + 2 * j)]).collect(),
                    });
                }
                "hot_pipes" => self.hot.push((e.t, e.f(3))),
                _ => {}
            }
        }
        self.laid = pipes.iter().filter(|pp| !pp.boxed).map(|pp| pp.t0).collect();
        self.pipes = pipes;
    }

    /// The ball thumps on every pipe it lays.
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if kk != self.k || phase != Phase::Fight || self.laid.is_empty() {
            return;
        }
        let tt = py_min(t, p.t_ko);
        let after = self.laid.partition_point(|&v| v <= tt);
        if let Some(j) = after.checked_sub(1) {
            st.glow = 0.36 * dmath::exp(-(tt - self.laid[j]) / 0.10);
        }
    }

    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if p.bold {
            return self.bold_items_free(p, f);
        }
        let mut out: Vec<(f64, Payload)> = Vec::new();
        let tc = p.fx_clock(f.t);
        let fade = if f.t >= p.t_ko {
            1.0 - ss(p.t_hs + 0.5, p.t_hs + 1.1, f.t)
        } else {
            1.0
        };
        if fade <= 0.0 || tc < p.show.grid.t_fight {
            return out;
        }
        let heat = self.heat(tc);
        for pp in &self.pipes {
            let Some((a, drawn)) = Self::pipe_state(pp, tc) else {
                continue;
            };
            let flash = pp
                .hits
                .iter()
                .filter(|&&th| (0.0..0.4).contains(&(tc - th)))
                .fold(0.0, |m: f64, &th| m.max(dmath::exp(-(tc - th) / 0.12)));
            let pts: Vec<V3> = part_of(&pp.pts, drawn)
                .iter()
                .map(|q| scene::to_world(q[0], q[1], PIPE_Y))
                .collect();
            let d = f.cam.depth(&pts).iter().sum::<f64>() / pts.len() as f64;
            let h = if pp.boxed { 0.0 } else { heat };
            out.push((d, Box::new(Item::Pipe(pts, a * fade, flash, h, pp.boxed))));
        }
        let beat = p.show.grid.beat;
        let me = f.fs[self.k].p;
        for s in &self.sprays {
            let age = (tc - s.t0) / beat;
            if !(0.0..1.0).contains(&age) {
                continue;
            }
            for q in s.from.iter().copied().chain(std::iter::once([me[0], me[1]])) {
                let d = f.cam.depth(&[scene::to_world(q[0], q[1], PIPE_Y)])[0];
                out.push((d - 0.01, Box::new(Item::Spray(q[0], q[1], age))));
            }
        }
        out
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        if p.bold {
            return Self::bold_draw_item(f, fr, payload);
        }
        let Some(item) = payload.downcast_ref::<Item>() else {
            return;
        };
        let (mut arr, depth) = (fr.array(), fr.depth());
        match item {
            Item::Pipe(pts, a, fl, h, b) => draw_pipe(&mut arr, f.cam, pts, *a, *fl, *h, *b),
            Item::Spray(x, z, age) => draw_spray(&mut arr, f.cam, *x, *z, *age),
        }
        fr.set_array(&arr, Some(&depth));
    }

    // ------------------------------------------------------------ the bold effects (nothing without `p.bold`)
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &bf_picture::look::CanvasCtx<'_>) {
        if p.bold {
            self.bold_canvas(p, ic, c);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pipe_slides_out_and_fades() {
        let pp = PipeL {
            id: 0,
            t0: 1.0,
            pts: vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            t_off: Some(3.0),
            hits: vec![],
            boxed: false,
        };
        assert_eq!(ContrabassoonLook::pipe_state(&pp, 0.5), None);
        assert_eq!(ContrabassoonLook::pipe_state(&pp, 1.5), Some((1.0, 1.0)));
        assert!(ContrabassoonLook::pipe_state(&pp, 3.4).is_none());
        let half = part_of(&pp.pts, 0.5);
        assert_eq!(half.last(), Some(&[0.5, 0.0]));
    }
}
