//! CONTRABASS CLARINET's look, drawn for the Studio (design `docs/studio/2026-11-16-contrabass-clarinet-design.md`;
//! no episode behind it). The silver paperclip body is its model (`data/models/CONTRABASS CLARINET.json`); this look
//! draws what happens round it:
//!
//! - FOGHORN: each blast as a soft wedge of sound from the bell, with three wavefronts rolling out along it;
//! - FOG BANK: while the shield holds, banks of pale fog drift round the ball;
//! - MOTORBOAT: while the haste holds, a rattling wake of puffs where the ball has just been;
//! - SHIP'S HORN: the ability beam as a fog-lamp shaft across the ring;
//! - its shatter parts (silver tube, gold keys, the bell) and a panel of fog bands and rings on the VS card.

use bf_core::dmath;
use bf_core::event::Event;
use bf_core::py::py_min;
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::img::{Img, RgbF};
use bf_picture::look::{FrameCtx, Look};
use bf_picture::raster::Frame;
use bf_picture::ring::{self, PictureCtx};
use bf_picture::scene::{R_BALL as R, to_world};

use super::state::{Horn, horns};

/// The silver of the body.
const SILVER: V3 = [214.0, 216.0, 226.0];
/// The gold of the key pads.
const GOLD: V3 = [226.0, 204.0, 148.0];
/// Sea fog.
const FOG: V3 = [196.0, 212.0, 214.0];
/// Seconds a blast takes to fade after it ends.
const FADE: f64 = 0.25;
/// Seconds a SHIP'S HORN shaft fades.
const BEAM_FADE: f64 = 0.35;

/// A SHIP'S HORN blast, from its `beam` event.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Beam {
    t: f64,
    o: [f64; 2],
    a: f64,
    len: f64,
    wd: f64,
}

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

/// A wedge from radius `r0` to `r1` round `c`, `half` radians each side of `aim`, at height `y`.
fn wedge(c: [f64; 2], aim: f64, half: f64, r0: f64, r1: f64, y: f64) -> Vec<V3> {
    let n = 10;
    let at = |r: f64, a: f64| world([c[0] + r * dmath::cos(a), c[1] + r * dmath::sin(a)], y);
    let mut pts: Vec<V3> = (0..=n)
        .map(|i| at(r1, aim - half + 2.0 * half * i as f64 / n as f64))
        .collect();
    pts.extend(
        (0..=n)
            .rev()
            .map(|i| at(r0, aim - half + 2.0 * half * i as f64 / n as f64)),
    );
    pts
}

/// `c` brightened toward white by `w` (0..1).
fn toward_white(c: V3, w: f64) -> V3 {
    c.map(|v| v + (255.0 - v) * w)
}

/// CONTRABASS CLARINET's look.
#[derive(Clone, Debug, Default)]
pub struct ContraLook {
    /// Its side.
    k: usize,
    /// Its blasts.
    horns: Vec<Horn>,
    /// Its SHIP'S HORN blasts.
    beams: Vec<Beam>,
    /// How long a SHIP'S HORN shaft holds (`abilities.low.hold`).
    hold: f64,
}

impl ContraLook {
    /// The blast live (or fading) at `t`, with its fade 0..1.
    fn horn_at(&self, t: f64) -> Option<(Horn, f64)> {
        self.horns
            .iter()
            .rev()
            .find(|h| h.t0 <= t && t < h.t0 + h.dur + FADE)
            .map(|h| {
                let fade = if t < h.t0 + h.dur {
                    1.0
                } else {
                    1.0 - (t - h.t0 - h.dur) / FADE
                };
                (*h, fade)
            })
    }

    /// The blast as a wedge of sound with wavefronts rolling out.
    fn draw_horn(&self, arr: &mut Img<f32>, cam: &Cam, c: [f64; 2], h: Horn, fade: f64, t: f64, col: V3) {
        let y = R * 0.7;
        let (r0, r1) = (R * 0.9, 2.0 * R + h.reach);
        // the swell: it opens over the first 80 ms
        let open = py_min(1.0, (t - h.t0) / 0.08);
        let r1 = r0 + (r1 - r0) * open;
        let body = cam.wp(&wedge(c, h.aim, h.half, r0, r1, y));
        ring::layer_blend(arr, &[body], toward_white(col, 0.35), 0.20 * fade, None);
        let core = cam.wp(&wedge(c, h.aim, h.half * 0.45, r0, r1, y));
        ring::layer_blend(arr, &[core], toward_white(col, 0.6), 0.16 * fade, None);
        // three wavefronts, bowed, rolling out at 5 units a second
        let span = (r1 - r0).max(1e-3);
        for j in 0..3 {
            let r = r0 + ((t - h.t0) * 5.0 + span * j as f64 / 3.0) % span;
            let band = cam.wp(&wedge(c, h.aim, h.half * 0.95, r, r + 0.07, y));
            ring::layer_blend(
                arr,
                &[band],
                toward_white(col, 0.75),
                0.42 * fade * (1.0 - (r - r0) / span),
                None,
            );
        }
    }

    /// FOG BANK: banks of fog drifting round the ball.
    fn draw_fog(arr: &mut Img<f32>, cam: &Cam, c: [f64; 2], k_: f64, t: f64) {
        for j in 0..7 {
            let a = 0.9 * j as f64 + 0.35 * t;
            let d = 0.25 + 0.12 * ((j * 3 % 5) as f64);
            let q = [c[0] + d * dmath::cos(a), c[1] + d * dmath::sin(a)];
            let r = 0.32 + 0.06 * ((j % 3) as f64) + 0.03 * dmath::sin(1.3 * t + j as f64);
            let blob = cam.wp(&circle(q, r, R * (0.6 + 0.15 * (j % 2) as f64), 14));
            ring::layer_blend(arr, &[blob], FOG, 0.16 * k_, None);
        }
        let veil = cam.wp(&circle(c, R * 1.25, R, 18));
        ring::layer_blend(arr, &[veil], FOG, 0.30 * k_, None);
    }

    /// MOTORBOAT: puffs where the ball was a moment ago, popping on the 16ths.
    fn draw_wake(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        let col = p.col[self.k];
        for j in 1..=5usize {
            let Some(i) = f.i.checked_sub(j * 5) else { break };
            let Some(pos) = p.run.frame_values("P", i) else { break };
            let Some(q) = pos.get(2 * self.k..2 * self.k + 2) else {
                break;
            };
            let pulse = 0.75 + 0.25 * dmath::sin(f.t * 40.0 + j as f64);
            let puff = f
                .cam
                .wp(&circle([q[0], q[1]], (0.10 + 0.05 * j as f64) * pulse, R * 0.3, 10));
            ring::layer_blend(
                arr,
                &[puff],
                toward_white(col, 0.5),
                0.30 * (1.0 - j as f64 / 6.0),
                None,
            );
        }
    }

    /// SHIP'S HORN: a fog-lamp shaft from the bell across the ring.
    fn draw_beam(arr: &mut Img<f32>, cam: &Cam, b: Beam, k_: f64, ext: f64, col: V3) {
        let d = [dmath::cos(b.a), dmath::sin(b.a)];
        let nrm = [-d[1], d[0]];
        let y = R * 0.8;
        for (wk, c, a) in [
            (1.6, toward_white(col, 0.4), 0.30),
            (1.0, toward_white(col, 0.7), 0.45),
            (0.35, [255.0, 252.0, 236.0], 0.7),
        ] {
            let w = b.wd * wk / 2.0;
            let s1 = (b.len + R) * ext;
            let q = [
                [b.o[0] + nrm[0] * w * 0.4, b.o[1] + nrm[1] * w * 0.4],
                [b.o[0] + d[0] * s1 + nrm[0] * w, b.o[1] + d[1] * s1 + nrm[1] * w],
                [b.o[0] + d[0] * s1 - nrm[0] * w, b.o[1] + d[1] * s1 - nrm[1] * w],
                [b.o[0] - nrm[0] * w * 0.4, b.o[1] - nrm[1] * w * 0.4],
            ];
            let pts: Vec<V3> = q.iter().map(|&v| world(v, y)).collect();
            ring::layer_blend(arr, &[cam.wp(&pts)], c, a * k_, None);
        }
    }
}

impl Look for ContraLook {
    fn shards(&self) -> Vec<String> {
        [("tube", 6), ("key", 6), ("bell", 1)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &["wall"]
    }

    fn spark_flash(&self) -> &[&str] {
        &["wall", "beam"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["wall", "beam"]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    fn cfg_summary(&self) -> Option<String> {
        Some(format!("abilities.low.hold={}", self.hold))
    }

    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.horns = horns(&p.rec.events, k);
        self.hold = cfg.low.get("hold").unwrap_or(0.3);
        let num = |e: &Event, i: usize| -> Option<f64> { (e.fields.len() > i - 2).then(|| e.f(i)) };
        self.beams = p
            .rec
            .events
            .iter()
            .filter(|e| e.kind == "beam" && e.who() == Some(k))
            .filter_map(|e| {
                Some(Beam {
                    t: e.t,
                    o: [num(e, 3)?, num(e, 4)?],
                    a: num(e, 5)?,
                    len: num(e, 6)?,
                    wd: num(e, 7)?,
                })
            })
            .collect();
    }

    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        let (cam, t, k) = (f.cam, f.t, self.k);
        let st = &f.fs[k];
        if !st.vis || st.shattered {
            return;
        }
        let col = p.col[k];
        if f.in_fight && st.shield > 0.0 {
            Self::draw_fog(arr, cam, st.p, py_min(1.0, st.shield / 0.3), t);
        }
        if f.in_fight && st.haste > 0.0 {
            self.draw_wake(p, f, arr);
        }
        if let Some((h, fade)) = self.horn_at(t).filter(|_| t <= p.t_ko) {
            self.draw_horn(arr, cam, st.p, h, fade, t, col);
        }
        for b in &self.beams {
            let u = t - b.t;
            if u < 0.0 || u > self.hold + BEAM_FADE {
                continue;
            }
            let ext = py_min(1.0, u / 0.06);
            let k_ = if u <= self.hold {
                1.0
            } else {
                1.0 - (u - self.hold) / BEAM_FADE
            };
            Self::draw_beam(arr, cam, *b, k_, ext, col);
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
        let d = [dmath::cos(rot), dmath::sin(rot)];
        let nn = [-d[1], d[0]];
        let (ln, wd, col) = match kind {
            "tube" => (0.24, 0.05, SILVER),
            "key" => (0.05, 0.05, GOLD),
            _ => (0.12, 0.12, toward_white(SILVER, 0.2)),
        };
        let (ln, wd) = (ln * size.max(0.3), wd * size.max(0.3));
        let q: Vec<V3> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .iter()
            .map(|&(a1, a2): &(f64, f64)| {
                // the bell flares: wider at its mouth
                let w = if kind == "bell" && a1 > 0.0 { wd * 1.8 } else { wd };
                to_world(
                    sx + d[0] * ln * a1 + nn[0] * w * a2,
                    sz + d[1] * ln * a1 + nn[1] * w * a2,
                    pos[1],
                )
            })
            .collect();
        ring::layer_blend(arr, &[ring::screen(cam, &q)], col, 1.0, None);
    }

    /// Bands of sea fog across the panel and the rings of a foghorn round its corner.
    fn title_pattern(&self, base: &mut RgbF) {
        let (w, h) = (base.w, base.h);
        let cx = if self.k == 0 { w as f64 * 0.12 } else { w as f64 * 0.88 };
        let cy = if self.k == 0 { h as f64 * 0.12 } else { h as f64 * 0.88 };
        for y in 0..h {
            for x in 0..w {
                let (fx, fy) = (x as f64, y as f64);
                let band = 0.5 + 0.5 * dmath::sin(fy / 23.0 + 0.6 * dmath::sin(fx / 61.0));
                let r = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
                let ringv = if (r / 34.0).fract() < 0.18 { 0.10 } else { 0.0 };
                let px = base.px_mut(x, y);
                for v in px.iter_mut() {
                    *v = *v * (0.92 + 0.08 * band) + 255.0 * ringv * (1.0 - r / 1400.0).max(0.0);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_shatter_parts_and_wedge() {
        let look = ContraLook::default();
        assert_eq!(look.shards().len(), 13);
        let w = wedge([0.0, 0.0], 0.0, 0.4, 0.2, 1.0, 0.1);
        assert_eq!(w.len(), 22);
        assert_eq!(circle([0.0, 0.0], 1.0, 0.0, 8).len(), 8);
    }
}
