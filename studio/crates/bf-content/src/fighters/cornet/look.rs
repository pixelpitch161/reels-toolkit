//! CORNET's look (a from-scratch fighter, 2026-11-16; design `docs/superpowers/specs/2026-11-16-cornet-fighter-design.md`).
//!
//! What it draws: PLUNGER MUTE's cord from the ball to the cup (sagging while it flies or reels back, taut while it
//! is stuck), the red rubber cup with its wooden handle, and a "wah" ring of light round the stuck cup on every wah;
//! DERBY HAT's bowler spinning through the air and sat on the muffled enemy; STOP-TIME's freeze bracket round the
//! frozen enemy on each stop and the break's ring. WA-WA is the engine's shield, drawn by the ring as every shield.
//! The ball's model is `data/models/CORNET.json` (made from photos of real cornets).

use bf_core::dmath;
use bf_core::event::Event;
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::BAYER;
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, FrameCtx, Look, Payload};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::Wave;
use bf_picture::scene::{self, R_BALL as R};

use crate::fighters::trumpet::TrumpetLook;

/// The cup's rubber.
const RUBBER: V3 = [178.0, 34.0, 40.0];
/// The cup's rim, lighter.
const RIM: V3 = [226.0, 76.0, 70.0];
/// The wooden handle.
const WOOD: V3 = [176.0, 120.0, 66.0];
/// The cord: waxed cotton.
const CORD: V3 = [236.0, 214.0, 160.0];
/// The bowler's felt.
const FELT: V3 = [36.0, 30.0, 34.0];
/// The bowler's band (the cornet's colour is laid over it).
const BAND: V3 = [120.0, 40.0, 56.0];
/// Seconds a wah ring spreads.
const WAH_S: f64 = 0.35;
/// Seconds a freeze bracket shows.
const STOP_S: f64 = 0.4;

/// One painter item.
enum Item {
    /// The cord and cup: cup x, z, phase code; the ball's x, z.
    Cup(f64, f64, f64, f64, f64),
    /// A hat: x, z, height above the floor, spin angle.
    Hat(f64, f64, f64, f64),
}

/// `BAYER_FULL[y, x]`.
fn bayer(x: usize, y: usize) -> f64 {
    BAYER[y % 4][x % 4]
}

/// A flat regular polygon of `n` corners round (x, z) at height y.
fn ngon(x: f64, z: f64, y: f64, r: f64, n: usize, rot: f64) -> Vec<V3> {
    (0..n)
        .map(|i| {
            let a = rot + i as f64 * std::f64::consts::TAU / n as f64;
            scene::to_world(x + r * dmath::cos(a), z + r * dmath::sin(a), y)
        })
        .collect()
}

/// The time of the latest event of `kind` by `who` at or before `t`, if any.
fn last_event(events: &[Event], kind: &str, who: usize, t: f64) -> Option<f64> {
    events
        .iter()
        .rev()
        .find(|e| e.kind == kind && e.who() == Some(who) && e.t <= t)
        .map(|e| e.t)
}

/// CORNET's look.
#[derive(Clone, Debug, Default)]
pub struct CornetLook {
    /// Its side.
    k: usize,
}

impl CornetLook {
    /// This side's `PLG` row at frame `i`: cup x, z, phase, reach, hat x, z, height.
    fn plg(p: &PictureCtx, i: usize, k: usize) -> Option<[f64; 7]> {
        let rows = p.run.frame_values("PLG", i)?;
        let o = 7 * k;
        let row: [f64; 7] = rows.get(o..o + 7)?.try_into().ok()?;
        Some(row)
    }

    /// The muffles this side put on the enemy that last at `t`: (start, seconds).
    fn muffled(p: &PictureCtx, k: usize, t: f64) -> bool {
        p.rec
            .events
            .iter()
            .filter(|e| e.kind == "muffle" && e.i(3) == k as i64)
            .any(|e| t >= e.t && t < e.t + e.f(4))
    }
}

/// The cord's screen-free world points from the ball's surface to the cup, sagging by `sag`.
fn cord(bx: f64, bz: f64, cx: f64, cz: f64, sag: f64) -> Vec<V3> {
    let (dx, dz) = (cx - bx, cz - bz);
    let n = np::hypot(dx, dz).max(1e-9);
    let (sx, sz) = (bx + dx / n * R * 0.9, bz + dz / n * R * 0.9);
    (0..=8)
        .map(|j| {
            let u = j as f64 / 8.0;
            let y = 0.5 - sag * 4.0 * u * (1.0 - u);
            scene::to_world(sx + (cx - sx) * u, sz + (cz - sz) * u, y.max(0.05))
        })
        .collect()
}

/// The cup: a rubber disc facing the way it flies, its lighter rim and the wooden handle behind it.
fn draw_cup(fr: &mut Frame, cam: &Cam, cx: f64, cz: f64, bx: f64, bz: f64) {
    let (dx, dz) = (cx - bx, cz - bz);
    let n = np::hypot(dx, dz).max(1e-9);
    let (ux, uz) = (dx / n, dz / n);
    let (hx, hz) = (cx - ux * 0.3, cz - uz * 0.3);
    fr.line(
        cam,
        &[scene::to_world(hx, hz, 0.5), scene::to_world(cx, cz, 0.5)],
        WOOD,
        3,
    );
    fr.poly(cam, &ngon(cx, cz, 0.52, 0.19, 10, 0.0), RIM, true);
    fr.poly(
        cam,
        &ngon(cx + ux * 0.02, cz + uz * 0.02, 0.56, 0.14, 10, 0.3),
        RUBBER,
        true,
    );
}

/// The bowler: a felt brim, a crown of two stacked rings and the band in the cornet's colour.
fn draw_hat(fr: &mut Frame, cam: &Cam, x: f64, z: f64, y: f64, spin: f64, col: V3) {
    let band = [0, 1, 2].map(|c| BAND[c] * 0.4 + col[c] * 0.6);
    fr.poly(cam, &ngon(x, z, y, 0.42, 12, spin), FELT, true);
    fr.poly(cam, &ngon(x, z, y + 0.06, 0.27, 10, spin), band, true);
    fr.poly(cam, &ngon(x, z, y + 0.12, 0.25, 10, spin), FELT, true);
    fr.poly(cam, &ngon(x, z, y + 0.24, 0.19, 8, spin), [52.0, 44.0, 50.0], true);
}

impl Look for CornetLook {
    /// Its shatter parts: brass tubing, the three valve caps and chips of the bell (drawn as TRUMPET's).
    fn shards(&self) -> Vec<String> {
        [("tube", 5), ("valve", 3), ("brass", 5)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
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
        curl: f64,
    ) {
        TrumpetLook::default().draw_shard(p, arr, cam, kind, pos, rot, size, curl);
    }

    fn spark_flash(&self) -> &[&str] {
        &["wah", "stoptime", "break"]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    fn setup_as(&mut self, _p: &PictureCtx, k: usize, _cfg: &FighterCfg) {
        self.k = k;
    }

    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let k = self.k;
        let tint = p.col[k].map(|v| v / 255.0 * 1.3 + 0.3);
        // a wah ring round the stuck cup
        if let (Some(row), Some(t_wah)) = (Self::plg(p, c.i, k), last_event(&p.rec.events, "wah", k, c.t)) {
            let u = (c.t - t_wah) / WAH_S;
            if row[2] == 2.0 && (0.0..1.0).contains(&u) {
                let r = 0.3 + 1.1 * u;
                for (j, light) in ic.iter_mut().enumerate() {
                    let d = np::hypot(c.sxp[j] - row[0], c.szp[j] - row[1]);
                    let band = np::clip(1.0 - (d - r).abs() / 0.18, 0.0, 1.0);
                    let band = (band * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
                    let v = band * (1.0 - u) * 0.8;
                    for ch in 0..3 {
                        light[ch] += v * tint[ch];
                    }
                }
            }
        }
        // STOP-TIME: a freeze bracket round the frozen enemy
        if let Some(t_stop) = last_event(&p.rec.events, "stop", k, c.t) {
            let u = (c.t - t_stop) / STOP_S;
            if (0.0..1.0).contains(&u) {
                let e = c.fs[1 - k].p;
                let half = R * 1.5;
                for (j, light) in ic.iter_mut().enumerate() {
                    let (dx, dz) = ((c.sxp[j] - e[0]).abs(), (c.szp[j] - e[1]).abs());
                    let edge = dx.max(dz);
                    let corner = dx.min(dz) > half * 0.45;
                    if corner || (edge - half).abs() > 0.07 {
                        continue;
                    }
                    let v = 0.9 * (1.0 - u);
                    light[0] += v * 0.75;
                    light[1] += v * 0.88;
                    light[2] += v * 1.0;
                }
            }
        }
    }

    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        // STOP-TIME's break: the band comes back in, a bold ring in the cornet's colour
        let fade = 1.0 - w.r / w.rmax;
        let tint = p.col[w.owner].map(|v| v / 255.0 * 1.4 + 0.35);
        for (j, light) in ic.iter_mut().enumerate() {
            let d = np::hypot(c.sxp[j] - w.cx, c.szp[j] - w.cz);
            let band = np::clip(1.0 - (d - w.r).abs() / 0.3, 0.0, 1.0);
            let band = (band * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
            let v = band * fade * 1.1;
            for ch in 0..3 {
                light[ch] += v * tint[ch];
            }
        }
    }

    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if !f.in_fight {
            return Vec::new();
        }
        let k = self.k;
        let mut out: Vec<(f64, Payload)> = Vec::new();
        if let Some(row) = Self::plg(p, f.i, k) {
            if row[2] > 0.0 && !f.fs[k].shattered {
                let b = f.fs[k].p;
                let depth = f.cam.depth(&[scene::to_world(row[0], row[1], 0.5)])[0];
                out.push((depth, Box::new(Item::Cup(row[0], row[1], row[2], b[0], b[1]))));
            }
            if !row[4].is_nan() {
                let y = 0.5 + row[6];
                let depth = f.cam.depth(&[scene::to_world(row[4], row[5], y)])[0];
                out.push((depth, Box::new(Item::Hat(row[4], row[5], y, f.t * 9.0))));
            }
        }
        // the hat sat on the muffled enemy
        let o = 1 - k;
        if Self::muffled(p, k, f.t) && !f.fs[o].shattered {
            let e = f.fs[o].p;
            let y = 2.0 * R * f.fs[o].scale.max(0.5) + 0.02;
            let depth = f.cam.depth(&[scene::to_world(e[0], e[1], y)])[0] - 0.02;
            out.push((depth, Box::new(Item::Hat(e[0], e[1], y, 0.3))));
        }
        out
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        match payload.downcast_ref::<Item>() {
            Some(&Item::Cup(cx, cz, phase, bx, bz)) => {
                let sag = if phase == 2.0 { 0.02 } else { 0.18 };
                fr.line(f.cam, &cord(bx, bz, cx, cz, sag), CORD, 1);
                draw_cup(fr, f.cam, cx, cz, bx, bz);
            }
            Some(&Item::Hat(x, z, y, spin)) => draw_hat(fr, f.cam, x, z, y, spin, p.col[self.k]),
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cord_runs_from_the_ball_to_the_cup_and_sags_only_when_slack() {
        let slack = cord(0.0, 0.0, 2.0, 0.0, 0.18);
        let taut = cord(0.0, 0.0, 2.0, 0.0, 0.02);
        assert_eq!(slack.len(), 9);
        let low = |pts: &[V3]| pts.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
        assert!(low(&slack) < low(&taut));
    }

    #[test]
    fn an_ngon_has_its_corners() {
        assert_eq!(ngon(0.0, 0.0, 0.5, 0.2, 10, 0.0).len(), 10);
    }
}
