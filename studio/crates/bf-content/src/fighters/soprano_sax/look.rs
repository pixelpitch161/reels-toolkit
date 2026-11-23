//! SOPRANO SAX's look (a from-scratch fighter, 2026-11-16; design
//! `docs/superpowers/specs/2026-11-16-soprano-sax-fighter-design.md`).
//!
//! What it draws: HORNET SWARM's hornets (yellow and black, wings flickering) circling the ball, the chasers in the air
//! with a faint buzz trail, KICK THE HIVE's chargers the same, QUEEN's big hornet with a gold crown band; BUZZ OFF's
//! tight, flickering ring of light round the ball while it lasts. Shards are ALTO SAX's (brass, keys and pearls). The
//! ball's model is `data/models/SOPRANO SAX.json` (made from photos of real straight sopranos) once it exists.

use bf_core::dmath;
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::BAYER;
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, FrameCtx, Look, Payload};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::scene::{self, R_BALL as R};

use crate::fighters::alto_sax::AltoSaxLook;

/// A hornet's yellow.
const YELLOW: V3 = [250.0, 204.0, 40.0];
/// A hornet's black stripes and head.
const BLACK: V3 = [28.0, 22.0, 20.0];
/// Its wings, pale and see-through looking.
const WING: V3 = [210.0, 226.0, 236.0];
/// The queen's crown band.
const GOLD: V3 = [255.0, 236.0, 150.0];
/// The record's floats per ball (`HRN`).
const W: usize = 33;
/// The hornets in the air the record keeps per ball.
const SLOTS: usize = 10;

/// One painter item: a hornet at x, z, height, heading (radians on the floor), size, whether it is the queen, and an
/// optional trail start (x, z).
struct Bee {
    x: f64,
    z: f64,
    y: f64,
    head: f64,
    size: f64,
    queen: bool,
    trail: Option<(f64, f64)>,
}

/// `BAYER_FULL[y, x]`.
fn bayer(x: usize, y: usize) -> f64 {
    BAYER[y % 4][x % 4]
}

/// A flat ellipse of `n` corners round (x, z) at height y: `a` along the heading, `b` across.
fn ellipse(x: f64, z: f64, y: f64, a: f64, b: f64, head: f64, n: usize) -> Vec<V3> {
    let (c, s) = (dmath::cos(head), dmath::sin(head));
    (0..n)
        .map(|i| {
            let t = i as f64 * 2.0 * dmath::PI / n as f64;
            let (u, v) = (a * dmath::cos(t), b * dmath::sin(t));
            scene::to_world(x + u * c - v * s, z + u * s + v * c, y)
        })
        .collect()
}

/// Where orbiting hornet `i` of `n` is at time `t` round a ball at (x, z): the engine's own circle.
fn orbit(x: f64, z: f64, i: usize, n: usize, t: f64) -> (f64, f64, f64) {
    let a = t * 4.0 + 2.0 * dmath::PI * i as f64 / n.max(1) as f64;
    let d = R + 0.22;
    (x + d * dmath::cos(a), z + d * dmath::sin(a), a + dmath::PI / 2.0)
}

/// One hornet: two flickering wings, a yellow body with a black stripe and a black head; the queen bigger, with a
/// gold band.
fn draw_bee(fr: &mut Frame, cam: &Cam, b: &Bee, t: f64) {
    let s = b.size;
    let (c, sn) = (dmath::cos(b.head), dmath::sin(b.head));
    // wings: up and down at a fast flicker
    let flap = if (t * 30.0 + b.x * 7.0).floor() as i64 % 2 == 0 {
        1.0
    } else {
        0.55
    };
    for side in [-1.0, 1.0] {
        let (wx, wz) = (b.x - sn * side * s * 0.9 * flap, b.z + c * side * s * 0.9 * flap);
        fr.poly(
            cam,
            &ellipse(wx, wz, b.y + 0.04, s * 0.55, s * 0.3, b.head + side * 0.5, 8),
            WING,
            true,
        );
    }
    fr.poly(cam, &ellipse(b.x, b.z, b.y, s * 1.1, s * 0.6, b.head, 10), YELLOW, true);
    // the stripe across the middle and the head in front
    fr.poly(
        cam,
        &ellipse(
            b.x - c * s * 0.15,
            b.z - sn * s * 0.15,
            b.y + 0.01,
            s * 0.22,
            s * 0.58,
            b.head,
            8,
        ),
        BLACK,
        true,
    );
    fr.poly(
        cam,
        &ellipse(
            b.x + c * s * 0.95,
            b.z + sn * s * 0.95,
            b.y + 0.01,
            s * 0.38,
            s * 0.38,
            b.head,
            8,
        ),
        BLACK,
        true,
    );
    if b.queen {
        fr.poly(
            cam,
            &ellipse(
                b.x + c * s * 0.45,
                b.z + sn * s * 0.45,
                b.y + 0.02,
                s * 0.18,
                s * 0.6,
                b.head,
                8,
            ),
            GOLD,
            true,
        );
    }
}

/// SOPRANO SAX's look.
#[derive(Clone, Debug, Default)]
pub struct SopranoSaxLook {
    /// Its side.
    k: usize,
}

impl SopranoSaxLook {
    /// This side's `HRN` row at frame `i`.
    fn hrn(p: &PictureCtx, i: usize, k: usize) -> Option<[f64; W]> {
        let rows = p.run.frame_values("HRN", i)?;
        rows.get(W * k..W * k + W)?.try_into().ok()
    }
}

impl Look for SopranoSaxLook {
    /// Its own panel on the VS card ([`crate::fighters::shared::panels::soprano_sax`]).
    fn title_pattern(&self, base: &mut bf_picture::img::RgbF) {
        crate::fighters::shared::panels::soprano_sax(base);
    }

    /// Its shatter parts: brass, keys and pearls (drawn as ALTO SAX's).
    fn shards(&self) -> Vec<String> {
        [("brass", 6), ("key", 4), ("pearl", 3)]
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
        AltoSaxLook::default().draw_shard(p, arr, cam, kind, pos, rot, size, curl);
    }

    fn spark_flash(&self) -> &[&str] {
        &["queen", "hive"]
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
        // BUZZ OFF: a tight, flickering ring of yellow light hugging the ball
        let Some(row) = Self::hrn(p, c.i, k) else {
            return;
        };
        if row[2].is_nan() || row[2] <= 0.0 || c.fs[k].shattered {
            return;
        }
        let b = c.fs[k].p;
        let fade = np::clip(row[2] / 0.3, 0.0, 1.0);
        let r = R + 0.18 + 0.04 * dmath::sin(c.t * 40.0);
        for (j, light) in ic.iter_mut().enumerate() {
            let d = np::hypot(c.sxp[j] - b[0], c.szp[j] - b[1]);
            let band = np::clip(1.0 - (d - r).abs() / 0.12, 0.0, 1.0);
            let band = (band * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
            let v = band * fade * 0.9;
            light[0] += v * 1.2;
            light[1] += v * 0.95;
            light[2] += v * 0.2;
        }
    }

    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if !f.in_fight {
            return Vec::new();
        }
        let k = self.k;
        let Some(row) = Self::hrn(p, f.i, k) else {
            return Vec::new();
        };
        let prev = Self::hrn(p, f.i.saturating_sub(3), k);
        let mut bees = Vec::new();
        // the orbit (hidden once the ball is shattered)
        let n = row[1].max(0.0) as usize;
        if !f.fs[k].shattered {
            let b = f.fs[k].p;
            let tight = row[2] > 0.0;
            for i in 0..n.min(16) {
                let (mut x, mut z, head) = orbit(b[0], b[1], i, n, f.t);
                if tight {
                    // BUZZ OFF pulls them in close
                    x = b[0] + (x - b[0]) * 0.8;
                    z = b[1] + (z - b[1]) * 0.8;
                }
                let y = 0.55 + 0.08 * dmath::sin(f.t * 9.0 + i as f64);
                bees.push(Bee {
                    x,
                    z,
                    y,
                    head,
                    size: 0.09,
                    queen: false,
                    trail: None,
                });
            }
        }
        // the hornets in the air
        for s in 0..SLOTS {
            let (x, z, code) = (row[3 + 3 * s], row[4 + 3 * s], row[5 + 3 * s]);
            if x.is_nan() {
                continue;
            }
            let what = code % 10.0;
            let queen = what == 3.0;
            let trail = prev.and_then(|pr| {
                let (px, pz) = (pr[3 + 3 * s], pr[4 + 3 * s]);
                (!px.is_nan() && np::hypot(px - x, pz - z) < 1.5).then_some((px, pz))
            });
            let head = match trail {
                Some((px, pz)) if np::hypot(x - px, z - pz) > 1e-6 => dmath::atan2(z - pz, x - px),
                _ => f.t * 6.0,
            };
            let size = if queen { 0.2 } else { 0.1 };
            bees.push(Bee {
                x,
                z,
                y: if queen { 0.75 } else { 0.6 },
                head,
                size,
                queen,
                trail,
            });
        }
        bees.into_iter()
            .map(|b| {
                let depth = f.cam.depth(&[scene::to_world(b.x, b.z, b.y)])[0];
                (depth, Box::new(b) as Payload)
            })
            .collect()
    }

    fn draw_item(&self, _p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        if let Some(b) = payload.downcast_ref::<Bee>() {
            if let Some((px, pz)) = b.trail {
                // a faint dotted buzz trail
                for j in 1..4 {
                    let u = j as f64 / 4.0;
                    let (tx, tz) = (px + (b.x - px) * u, pz + (b.z - pz) * u);
                    let pts = ellipse(tx, tz, b.y, 0.025, 0.025, 0.0, 4);
                    fr.poly(f.cam, &pts, [120.0, 110.0, 70.0], true);
                }
            }
            draw_bee(fr, f.cam, b, f.t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_orbit_is_the_engines_circle() {
        let (x, z, _) = orbit(0.0, 0.0, 0, 4, 0.0);
        assert!((np::hypot(x, z) - (R + 0.22)).abs() < 1e-9);
    }

    #[test]
    fn an_ellipse_has_its_corners() {
        assert_eq!(ellipse(0.0, 0.0, 0.5, 0.2, 0.1, 0.3, 10).len(), 10);
    }
}
