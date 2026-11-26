//! VIOLIN's look, as #42 drew it with #28's draw functions (`projects/ballfight_drums_vs_violin/styles.py`
//! `bow_geom` 29, `draw_bow` 47, `bow_trail_polys` 61; the darts through [`crate::fighters::shared::older`]; #42
//! `render.py`'s bow item 1255 and SCREECH's ring 1188). A quick port (ruling M4-R16): close to the episode by eye,
//! not to the byte.
//!
//! What it draws: the bow spun like a rapier (dark pernambuco stick with a highlight line, the pale hair ribbon on
//! the leading side, the ebony frog with its pearl eye, the hatchet tip), its motion fan behind it (TREMOLO turns the
//! fan violet), PIZZICATO's darts in flight, and SCREECH's jagged, shivering ring on the canvas.

use bf_core::dmath;
use bf_core::py::py_min;
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::BAYER;
use bf_picture::lighting::{Lights, shade};
use bf_picture::look::{CanvasCtx, FrameCtx, Look, Payload};
use bf_picture::raster::Frame;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{PictureCtx, layer_blend};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::{np, pil};

use crate::fighters::shared::older::{Dart, draw_dart};

/// The same moves with the bold effects.
#[path = "bold.rs"]
mod bold;

/// `STICK`: pernambuco.
const STICK: V3 = [92.0, 44.0, 30.0];
/// `STICK_HI`.
const STICK_HI: V3 = [150.0, 86.0, 52.0];
/// `HAIR`.
const HAIR: V3 = [236.0, 230.0, 212.0];
/// `FROG`.
const FROG: V3 = [26.0, 22.0, 30.0];
/// `PEARL`.
const PEARL: [u8; 3] = [226, 236, 255];
/// `TIP`.
const TIP: V3 = [240.0, 236.0, 226.0];
/// `STICK_W`.
const STICK_W: f64 = 0.055;
/// `HAIR_W`.
const HAIR_W: f64 = 0.045;
/// `HAIR_GAP`.
const HAIR_GAP: f64 = 0.085;

/// One painter item of the look.
enum Item {
    /// The bow (and its fan).
    Bow,
    /// A dart in flight: `(x, z, ang)`.
    Dart(f64, f64, f64),
}

/// `BAYER_FULL[y, x]`.
fn bayer(x: usize, y: usize) -> f64 {
    BAYER[y % 4][x % 4]
}

/// The mean of four world points.
fn mean4(q: &[V3; 4]) -> V3 {
    let mut c = [0.0; 3];
    for p in q {
        for i in 0..3 {
            c[i] += p[i];
        }
    }
    c.map(|v| v / 4.0)
}

/// One quad of the bow: its world corners, colour and whether it is the stick.
type BowQuad = ([V3; 4], V3, bool);

/// `bow_geom(p, th, blen, sd, y=R)`: the bow's world quads from the ball's surface to the tip (hair, stick, frog,
/// tip in that order) and the frog's pearl eye.
fn bow_geom(p: [f64; 2], th: f64, blen: f64, sd: f64) -> (Vec<BowQuad>, V3) {
    let y = R;
    let d = [dmath::cos(th), dmath::sin(th)];
    let nrm = [-d[1] * sd, d[0] * sd];
    let (r0, r1) = (R * 0.86, R + blen);
    let wp = |r: f64, w: f64| [p[0] + d[0] * r + nrm[0] * w, p[1] + d[1] * r + nrm[1] * w];
    let stick = [
        wp(r0, -STICK_W / 2.0),
        wp(r1, -STICK_W / 2.0 * 0.7),
        wp(r1, STICK_W / 2.0 * 0.7),
        wp(r0, STICK_W / 2.0),
    ];
    let hair = [
        wp(r0 + 0.16, HAIR_GAP - HAIR_W / 2.0),
        wp(r1 - 0.06, HAIR_GAP * 0.55 - HAIR_W / 2.0),
        wp(r1 - 0.06, HAIR_GAP * 0.55 + HAIR_W / 2.0),
        wp(r0 + 0.16, HAIR_GAP + HAIR_W / 2.0),
    ];
    let frog = [
        wp(r0 + 0.02, -0.03),
        wp(r0 + 0.22, -0.03),
        wp(r0 + 0.20, HAIR_GAP + 0.04),
        wp(r0 + 0.04, HAIR_GAP + 0.04),
    ];
    let tip = [
        wp(r1 - 0.13, -0.04),
        wp(r1 + 0.02, -0.04),
        wp(r1 + 0.02, HAIR_GAP * 0.55 + 0.03),
        wp(r1 - 0.08, HAIR_GAP * 0.55 + 0.03),
    ];
    let world = |q: [[f64; 2]; 4], lift: f64| q.map(|v| scene::to_world(v[0], v[1], y + lift));
    let out = vec![
        (world(hair, 0.0), HAIR, false),
        (world(stick, 0.012), STICK, true),
        (world(frog, 0.0), FROG, false),
        (world(tip, 0.0), TIP, false),
    ];
    let e = wp(r0 + 0.12, HAIR_GAP * 0.5);
    (out, scene::to_world(e[0], e[1], y + 0.02))
}

/// `draw_bow(fr, cam, L, shade, st)`: the quads lit from above, a highlight line along the (round) stick, the pearl.
fn draw_bow(fr: &mut Frame, cam: &Cam, l: &Lights, st: &BallState) {
    let (quads, pearl) = bow_geom(st.p, st.th, st.blen, st.sd);
    let up = [0.0, 1.0, 0.0];
    for (q, col, is_stick) in &quads {
        let c = shade(l, &[up], &[mean4(q)], &[*col], None, 0.35, 0.0, 5.0)[0];
        fr.poly(cam, q, c, true);
        if *is_stick {
            let s = cam.wp(q);
            let a = ((s[0].0 + s[3].0) / 2.0, (s[0].1 + s[3].1) / 2.0);
            let b = ((s[1].0 + s[2].0) / 2.0, (s[1].1 + s[2].1) / 2.0);
            let hi = shade(l, &[up], &[mean4(q)], &[STICK_HI], None, 0.6, 0.0, 5.0)[0];
            let ink = hi.map(|v| np::clip(v, 0.0, 255.0) as u8);
            pil::line(&mut fr.pil, &[a, b], &ink, 1);
        }
    }
    let sp = cam.wp(&[pearl])[0];
    pil::line(&mut fr.pil, &[sp, sp], &PEARL, 1);
}

/// `bow_trail_polys(cam, st, dth, nseg=6)`: the swept wedge behind the spinning bow as screen polygons, each with
/// its place `j / nseg` (0 at the bow).
fn bow_trail_polys(cam: &Cam, st: &BallState, dth: f64) -> Vec<(Vec<(f64, f64)>, f64)> {
    let nseg = 6;
    let (r0, r1) = (R + st.blen * 0.35, R + st.blen);
    (0..nseg)
        .map(|j| {
            let a0 = st.th - dth * j as f64 / nseg as f64;
            let a1 = st.th - dth * (j + 1) as f64 / nseg as f64;
            let pts: Vec<V3> = [(a0, r0), (a0, r1), (a1, r1), (a1, r0)]
                .iter()
                .map(|&(a, r)| scene::to_world(st.p[0] + r * dmath::cos(a), st.p[1] + r * dmath::sin(a), R))
                .collect();
            (cam.wp(&pts), j as f64 / nseg as f64)
        })
        .collect()
}

/// VIOLIN's look.
#[derive(Clone, Debug, Default)]
pub struct ViolinLook {
    /// Its side.
    k: usize,
    /// TREMOLO's grid multiplier (`CFG[k]['abilities']['unlock2']['mult']`), 1.0 without one.
    mult: f64,
    /// What the bold look works out from the recording (empty without the bold effects).
    bold: bold::Bold,
}

impl Look for ViolinLook {
    fn spark_flash(&self) -> &[&str] {
        &["blade", "wave"]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// TREMOLO's multiplier: what `setup_as` took from `cfg`.
    fn cfg_summary(&self) -> Option<String> {
        Some(format!("abilities.unlock2.mult={}", self.mult))
    }

    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.mult = cfg.unlock2.get("mult").unwrap_or(1.0);
        if p.bold {
            self.bold = Self::bold_setup(p, k, cfg);
        }
    }

    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, _phase: bf_picture::look::Phase, _gl: f64) {
        if p.bold {
            self.bold_state(kk, st, t);
        }
    }

    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
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

    fn bold_hit_size(
        &self,
        _p: &PictureCtx,
        e: &bf_core::event::Event,
        by_damage: bf_picture::bold::HitSize,
    ) -> Option<bf_picture::bold::HitSize> {
        Self::bold_hit(e, by_damage)
    }

    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if p.bold {
            return self.bold_canvas_wave(p, ic, c, w);
        }
        // SCREECH: a jagged, shivering ring (#28)
        let k_ = (1.0 - w.r / w.rmax) * 1.3;
        let tint = p.col[w.owner].map(|v| v / 255.0 * 1.4 + 0.35);
        let t = c.t;
        for (j, light) in ic.iter_mut().enumerate() {
            let (dx, dz) = (c.sxp[j] - w.cx, c.szp[j] - w.cz);
            let ang = dmath::atan2(dz, dx);
            let rd = np::hypot(dx, dz)
                + 0.10 * (2.0 * (7.0 * ang / dmath::PI + 6.0 * t).rem_euclid(1.0) - 1.0)
                + 0.04 * dmath::sin(23.0 * ang - 50.0 * t);
            let band = np::clip(1.0 - (rd - w.r).abs() / 0.32, 0.0, 1.0);
            let band = (band * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
            let v = band * k_ * 0.9;
            for ch in 0..3 {
                light[ch] += v * tint[ch];
            }
        }
    }

    fn items_ball(&self, p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        if p.bold {
            return self.bold_items_ball(f, st);
        }
        if st.shattered {
            return Vec::new();
        }
        let reach = R + st.blen * 0.55;
        let tip = [st.p[0] + reach * dmath::cos(st.th), st.p[1] + reach * dmath::sin(st.th)];
        let depth = f.cam.depth(&[scene::to_world(tip[0], tip[1], R)])[0] + 0.01;
        let bow: Payload = Box::new(Item::Bow);
        vec![(depth, bow)]
    }

    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if p.bold {
            return self.bold_items_free(p, f);
        }
        if !f.in_fight {
            return Vec::new();
        }
        let Some(rows) = p.run.frame_values("K", f.i) else {
            return Vec::new();
        };
        rows.as_chunks::<7>()
            .0
            .iter()
            .filter(|row| !row[0].is_nan() && np::f64_to_i64(row[3]) == self.k as i64)
            .map(|row| {
                let depth = f.cam.depth(&[scene::to_world(row[0], row[1], 0.45)])[0];
                let dart: Payload = Box::new(Item::Dart(row[0], row[1], row[2]));
                (depth, dart)
            })
            .collect()
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        if p.bold {
            return self.bold_draw_item(p, f, fr, payload);
        }
        let k = self.k;
        match payload.downcast_ref::<Item>() {
            Some(Item::Bow) => {
                // #28's bow, its motion fan and TREMOLO's blur
                let st = &f.fs[k];
                let mult = if st.haste > 0.0 { self.mult } else { 1.0 };
                let dth = py_min(2.4, st.om * mult * 0.055) * st.sd;
                let (mut arr, depth) = (fr.array(), fr.depth());
                for (pts, u) in bow_trail_polys(f.cam, st, dth) {
                    if st.haste > 0.0 {
                        let col = p.col[k].map(|c| c * 0.55 + 115.0 * (1.0 - 0.3 * u));
                        layer_blend(&mut arr, &[pts], col, 0.40 * (1.0 - u), None);
                    } else {
                        let a = py_min(0.30, 0.04 + 0.02 * st.om) * (1.0 - u);
                        layer_blend(&mut arr, &[pts], [236.0, 232.0, 222.0], a, None);
                    }
                }
                fr.set_array(&arr, Some(&depth));
                draw_bow(fr, f.cam, f.l, st);
            }
            Some(&Item::Dart(x, z, ang)) => draw_dart(fr, f.cam, f.l, x, z, ang, p.col[k], Dart::Violin),
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bow_has_its_four_parts() {
        let (quads, _) = bow_geom([0.0, 0.0], 0.0, 1.8, 1.0);
        assert_eq!(quads.len(), 4);
        assert_eq!(quads.iter().filter(|q| q.2).count(), 1);
    }
}
