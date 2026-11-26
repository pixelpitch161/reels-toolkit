//! FLUGELHORN's look, drawn for the Studio (design `docs/studio/2026-11-16-flugelhorn-design.md`; no episode behind
//! it). The brass horn is its model (`data/models/FLUGELHORN.json`, from photos); this look draws what happens round it:
//!
//! - WINGBEAT: before every clap two feathered wings open out of the ball's sides, then sweep shut in front of it on
//!   the beat, with a puff of air where they meet;
//! - FLANKING CALL: while it swoops, a streak of loose feathers where the ball has just been;
//! - FEATHER SHOWER: its shots drawn as drifting feathers;
//! - TAKE WING: the wings spread across the ring while the clap gathers, then a ring of air where it lands;
//! - its shatter parts (tube, valves, the bell) and a panel of feathers on the VS card.

use bf_core::dmath;
use bf_core::py::{py_max, py_min};
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::img::{Img, RgbF};
use bf_picture::look::{FrameCtx, Look};
use bf_picture::raster::Frame;
use bf_picture::ring::{self, PictureCtx};
use bf_picture::scene::{R_BALL as R, to_world};

use super::state::{Clap, Flank, Take, claps, flanks, takes};

/// The same moves with the bold effects.
#[path = "bold.rs"]
mod bold;

/// The brass of the body.
const BRASS: V3 = [228.0, 186.0, 84.0];
/// Feather white, warm.
const FEATHER: V3 = [246.0, 242.0, 232.0];
/// Seconds the wings take to open before a clap.
const OPEN: f64 = 0.22;
/// Seconds the wings take to fade after it.
const FADE: f64 = 0.18;

/// A point of the ring's floor plane at height `y`, in the world.
fn world(q: [f64; 2], y: f64) -> V3 {
    to_world(q[0], q[1], y)
}

/// A circle of `n` points round `c` at height `y`.
fn circle(c: [f64; 2], r: f64, y: f64, n: usize) -> Vec<V3> {
    (0..n)
        .map(|i| {
            let a = 2.0 * dmath::PI * i as f64 / n as f64;
            world([c[0] + r * dmath::cos(a), c[1] + r * dmath::sin(a)], y)
        })
        .collect()
}

/// One feather: a long leaf from `root` along angle `a`, `len` long and `wd` wide at its widest, at height `y`.
fn feather(root: [f64; 2], a: f64, len: f64, wd: f64, y: f64) -> Vec<V3> {
    let d = [dmath::cos(a), dmath::sin(a)];
    let n = [-d[1], d[0]];
    let at = |s: f64, w: f64| world([root[0] + d[0] * s + n[0] * w, root[1] + d[1] * s + n[1] * w], y);
    vec![
        at(0.0, 0.0),
        at(len * 0.3, wd * 0.5),
        at(len * 0.75, wd * 0.42),
        at(len, 0.0),
        at(len * 0.75, -wd * 0.42),
        at(len * 0.3, -wd * 0.5),
    ]
}

/// `c` brightened toward white by `w` (0..1).
fn toward_white(c: V3, w: f64) -> V3 {
    c.map(|v| v + (255.0 - v) * w)
}

/// A wing of five feathers from the ball's side `s` (+1 left, -1 right of `aim`), opened `phi` radians off the aim,
/// `span` long.
#[allow(clippy::too_many_arguments)]
fn draw_wing(arr: &mut Img<f32>, cam: &Cam, c: [f64; 2], aim: f64, s: f64, phi: f64, span: f64, a: f64, col: V3) {
    let y = R * 0.75;
    let root_a = aim + s * dmath::PI * 0.5;
    let root = [c[0] + R * 0.8 * dmath::cos(root_a), c[1] + R * 0.8 * dmath::sin(root_a)];
    for j in 0..5 {
        let fa = aim + s * (phi + 0.16 * j as f64);
        let len = span * (1.0 - 0.13 * j as f64);
        let pts = cam.wp(&feather(root, fa, len, 0.16 + 0.02 * j as f64, y));
        let shade = toward_white(col, 0.55 + 0.08 * j as f64);
        ring::layer_blend(arr, &[pts], shade, a * (0.85 - 0.08 * j as f64), None);
    }
    // the wing's leading edge in the fighter's own colour
    let edge = cam.wp(&feather(root, aim + s * phi, span * 1.02, 0.07, y + 0.01));
    ring::layer_blend(arr, &[edge], col, a * 0.9, None);
}

/// FLUGELHORN's look.
#[derive(Clone, Debug, Default)]
pub struct FlugelLook {
    /// The roster numbers its bold moves read, from `setup_as`'s `cfg`.
    nums: bold::Nums,
    /// Its side.
    k: usize,
    /// Its claps.
    claps: Vec<Clap>,
    /// Its FLANKING CALL swoops.
    flanks: Vec<Flank>,
    /// Its TAKE WINGs.
    takes: Vec<Take>,
}

impl FlugelLook {
    /// The clap whose wings show at `t`: opening before it, fading after it; with its phase -1..0 (opening) or
    /// 0..1 (fading).
    fn clap_at(&self, t: f64) -> Option<(Clap, f64)> {
        self.claps
            .iter()
            .find(|c| c.t0 - OPEN <= t && t < c.t0 + FADE)
            .map(|c| {
                let u = if t < c.t0 { (t - c.t0) / OPEN } else { (t - c.t0) / FADE };
                (*c, u)
            })
    }

    /// The wings of a clap: wide open at phase -1, shut at 0, fading after.
    fn draw_clap(arr: &mut Img<f32>, cam: &Cam, c: [f64; 2], cl: Clap, u: f64, col: V3) {
        let span = R + cl.reach + 0.25;
        let open = py_max(cl.half, 1.2);
        let (phi, a) = if u < 0.0 {
            // opening out, then sweeping in: the widest at -0.45
            let w = if u < -0.45 { (u + 1.0) / 0.55 } else { -u / 0.45 };
            (
                cl.half * 0.25 + (open + 0.5 - cl.half * 0.25) * w.clamp(0.0, 1.0),
                py_min(1.0, (u + 1.0) * 3.0),
            )
        } else {
            (cl.half * 0.25, 1.0 - u)
        };
        for s in [1.0, -1.0] {
            draw_wing(arr, cam, c, cl.aim, s, phi, span, 0.8 * a, col);
        }
        if u >= 0.0 {
            // the puff where the wings meet
            let d = [dmath::cos(cl.aim), dmath::sin(cl.aim)];
            let q = [c[0] + d[0] * (R + cl.reach * 0.7), c[1] + d[1] * (R + cl.reach * 0.7)];
            let puff = cam.wp(&circle(q, 0.12 + 0.35 * u, R * 0.7, 14));
            ring::layer_blend(arr, &[puff], FEATHER, 0.45 * (1.0 - u), None);
        }
    }

    /// FLANKING CALL: loose feathers where the ball was a moment ago.
    fn draw_streak(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        let col = p.col[self.k];
        for j in 1..=6usize {
            let Some(i) = f.i.checked_sub(j * 3) else { break };
            let (Some(a), Some(b)) = (p.run.frame_values("P", i), p.run.frame_values("P", i + 2)) else {
                break;
            };
            let (Some(q), Some(q2)) = (a.get(2 * self.k..2 * self.k + 2), b.get(2 * self.k..2 * self.k + 2)) else {
                break;
            };
            let back = dmath::atan2(q[1] - q2[1], q[0] - q2[0]);
            let wob = 0.5 * dmath::sin(f.t * 9.0 + j as f64 * 1.7);
            let pts = f.cam.wp(&feather([q[0], q[1]], back + wob, 0.34, 0.12, R * 0.5));
            ring::layer_blend(arr, &[pts], toward_white(col, 0.6), 0.55 * (1.0 - j as f64 / 7.0), None);
        }
    }

    /// FEATHER SHOWER: its shots as feathers, turning as they drift.
    fn draw_feathers(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        let Some(rows) = p.run.frame_values("K", f.i) else {
            return;
        };
        let col = p.col[self.k];
        for row in rows.as_chunks::<7>().0 {
            if row[0].is_nan() || row[3].round() as i64 != self.k as i64 {
                continue;
            }
            let a = f.t * 3.0 + row[0] * 2.0;
            let pts = f.cam.wp(&feather([row[0], row[1]], a, 0.36, 0.14, 0.45));
            ring::layer_blend(arr, &[pts], toward_white(col, 0.7), 0.95, None);
            let quill = f.cam.wp(&feather([row[0], row[1]], a, 0.38, 0.03, 0.46));
            ring::layer_blend(arr, &[quill], col, 0.9, None);
        }
    }

    /// TAKE WING: wings spread across the ring as the clap gathers, then the ring of air.
    fn draw_take(arr: &mut Img<f32>, cam: &Cam, c: [f64; 2], aim: f64, tk: Take, t: f64, col: V3) {
        if t < tk.t_clap {
            let u = ((t - tk.t0) / (tk.t_clap - tk.t0).max(1e-3)).clamp(0.0, 1.0);
            let span = R + tk.radius * (0.3 + 0.7 * u);
            for s in [1.0, -1.0] {
                // spread right out, folding forward as the clap comes
                draw_wing(arr, cam, c, aim, s, 1.9 - 0.9 * u * u, span, 0.85, col);
                draw_wing(
                    arr,
                    cam,
                    c,
                    aim + dmath::PI,
                    -s,
                    1.9 - 0.9 * u * u,
                    span * 0.7,
                    0.5,
                    col,
                );
            }
        } else {
            let u = (t - tk.t_clap) / 0.4;
            if u < 1.0 {
                let r = tk.radius * (0.5 + 0.5 * u);
                let disc = cam.wp(&circle(c, r, R * 0.5, 40));
                ring::layer_blend(arr, &[disc], toward_white(col, 0.6), 0.30 * (1.0 - u), None);
                for j in 0..12 {
                    let a = 2.0 * dmath::PI * j as f64 / 12.0 + u;
                    let q = [c[0] + r * dmath::cos(a), c[1] + r * dmath::sin(a)];
                    let pts = cam.wp(&feather(q, a + 1.2, 0.4, 0.14, R * 0.6));
                    ring::layer_blend(arr, &[pts], FEATHER, 0.8 * (1.0 - u), None);
                }
            }
        }
    }
}

impl Look for FlugelLook {
    fn shards(&self) -> Vec<String> {
        [("tube", 6), ("valve", 3), ("feather", 6), ("bell", 1)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["take_wing", "flank"]
    }

    fn spark_flash(&self) -> &[&str] {
        &["take_wing"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["take_wing"]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.nums = bold::Nums::of(cfg);
        self.claps = claps(&p.rec.events, k);
        self.flanks = flanks(&p.rec.events, k);
        self.takes = takes(&p.rec.events, k);
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

    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        if p.bold {
            // the bold wings and feathers are painter items and crisp overlays (`bold.rs`)
            return;
        }
        let (cam, t, k) = (f.cam, f.t, self.k);
        if f.in_fight {
            self.draw_feathers(p, f, arr);
        }
        let st = &f.fs[k];
        if !st.vis || st.shattered || t > p.t_ko {
            return;
        }
        let col = p.col[k];
        if f.in_fight && self.flanks.iter().any(|fl| fl.t0 <= t && t < fl.t_end) {
            self.draw_streak(p, f, arr);
        }
        if let Some(tk) = self.takes.iter().find(|tk| tk.t0 <= t && t < tk.t_clap + 0.4) {
            Self::draw_take(arr, cam, st.p, st.th, *tk, t, col);
        } else if let Some((cl, u)) = self.clap_at(t) {
            // the wings follow the aim while they open
            let cl = if u < 0.0 { Clap { aim: st.th, ..cl } } else { cl };
            Self::draw_clap(arr, cam, st.p, cl, u, col);
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
        let (sx, sz) = (pos[0], pos[2]);
        if kind == "feather" {
            let pts = ring::screen(cam, &feather([sx, sz], rot, 0.3 * size.max(0.3), 0.1, pos[1]));
            ring::layer_blend(arr, &[pts], FEATHER, 1.0, None);
            return;
        }
        let d = [dmath::cos(rot), dmath::sin(rot)];
        let nn = [-d[1], d[0]];
        let (ln, wd) = match kind {
            "tube" => (0.24, 0.045),
            "valve" => (0.09, 0.05),
            _ => (0.14, 0.12),
        };
        let (ln, wd) = (ln * size.max(0.3), wd * size.max(0.3));
        let q: Vec<V3> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .iter()
            .map(|&(a1, a2): &(f64, f64)| {
                // the bell flares wide at its mouth
                let w = if kind == "bell" && a1 > 0.0 { wd * 2.2 } else { wd };
                to_world(
                    sx + d[0] * ln * a1 + nn[0] * w * a2,
                    sz + d[1] * ln * a1 + nn[1] * w * a2,
                    pos[1],
                )
            })
            .collect();
        ring::layer_blend(arr, &[ring::screen(cam, &q)], BRASS, 1.0, None);
    }

    /// Soft feathers drifting down the panel, leaning toward its own corner.
    fn title_pattern(&self, base: &mut RgbF) {
        let (w, h) = (base.w, base.h);
        let lean = if self.k == 0 { 0.5 } else { -0.5 };
        for y in 0..h {
            for x in 0..w {
                let (fx, fy) = (x as f64, y as f64);
                // a lattice of feathers, each a soft slanted streak with barbs
                let (cx, cy) = ((fx + lean * fy) / 70.0, fy / 110.0);
                let (gx, gy) = (cx - cx.floor() - 0.5, cy - cy.floor() - 0.5);
                let along = gy;
                let across = gx + 0.15 * dmath::sin(along * 6.0);
                let shape = (1.0 - (along * 2.0).abs()).max(0.0);
                let vane = if across.abs() < 0.14 * shape { 0.12 } else { 0.0 };
                let barb = if vane > 0.0 && ((along * 40.0).fract() < 0.25) {
                    0.05
                } else {
                    0.0
                };
                let px = base.px_mut(x, y);
                for v in px.iter_mut() {
                    *v = *v * (1.0 - vane * 0.4) + 255.0 * (vane + barb);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_shatter_parts_and_feather() {
        let look = FlugelLook::default();
        assert_eq!(look.shards().len(), 16);
        assert_eq!(feather([0.0, 0.0], 0.0, 1.0, 0.2, 0.1).len(), 6);
        assert_eq!(circle([0.0, 0.0], 1.0, 0.0, 8).len(), 8);
    }
}
