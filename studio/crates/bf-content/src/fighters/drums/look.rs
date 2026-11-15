//! DRUMS's look: #28's `render.py` layers for the drums and the `styles.py` draw code they call (`pulse_state`,
//! `draw_drumhead`, `cymbal_polys`, `draw_cymbal`, `beater_height`, `draw_beater`), close to the episode by eye
//! (ruling M4-R16), not to the bit.
//!
//! What it draws: the drum's reach as a faint dashed ring on the canvas; each pulse as a drumhead blasting out (the
//! cream skin band inside a hoop of its colour, chrome lugs on a main pulse) that lights the canvas under it; CRASH
//! CYMBAL (a golden ride round the ball, lathe grooves and a sweeping glint) while the shield is up; DOUBLE KICK's
//! beater (a felt head on a chrome shaft) tapping an 8th early and then stomping the spot; the ball thumping on
//! every pulse. The winner's drum solo rings of the episode's tail are left out (the kit's tail is its own).

use bf_core::dmath::{self, PI};
use bf_core::event::Value;
use bf_core::py::{py_max, py_min};
use bf_core::roster::{AbilityKind, FighterCfg, WeaponCfg};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::BAYER;
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, FrameCtx, Look, Payload, Phase, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::scene::{self, R_BALL as R};

use crate::fighters::shared::older::{Poly, annulus_polys, blend_into, mask_of_all, mask_of_lines};
use crate::fighters::shared::reeds::Num;

/// `SKIN`: the drumhead's cream.
const SKIN: V3 = [246.0, 238.0, 216.0];
/// `CHROME`: the lugs.
const CHROME: V3 = [214.0, 220.0, 236.0];
/// `BRONZE`.
const BRONZE: V3 = [226.0, 176.0, 70.0];
/// `BRONZE_DK`.
const BRONZE_DK: V3 = [150.0, 104.0, 34.0];
/// `FELT`: the beater's head.
const FELT: V3 = [240.0, 234.0, 220.0];
/// `SHAFT`: the beater's chrome shaft.
const SHAFT: V3 = [200.0, 206.0, 220.0];
/// The drumhead's height on the canvas (`draw_drumhead`'s `y`).
const RING_Y: f64 = 0.34;

/// A pulse as the look rebuilds it from the run (#28 `PULSE_EV`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pulse {
    t0: f64,
    main: bool,
    reach: f64,
    cx: f64,
    cz: f64,
}

/// What the look draws in the painter's list.
enum Item {
    /// CRASH CYMBAL's back (0) or front (1) half at strength `k`.
    Cymbal(usize, f64),
}

/// `BAYER_FULL[y, x]`.
fn bayer(x: usize, y: usize) -> f64 {
    BAYER[y % 4][x % 4]
}

/// `pulse_state(t, t0, reach, speed)`: `(radius from the ball's centre, alpha)` of a pulse, `None` before it or
/// once it has faded (0.22 s after it reached its full size).
fn pulse_state(t: f64, t0: f64, reach: f64, speed: f64) -> Option<(f64, f64)> {
    let u = t - t0;
    if u < 0.0 {
        return None;
    }
    let r = R + py_min(reach, speed * u);
    let t_full = reach / speed;
    let a = if u <= t_full {
        1.0
    } else {
        py_max(0.0, 1.0 - (u - t_full) / 0.22)
    };
    (a > 0.0).then_some((r, a))
}

/// The screen quads of a flat ring, as polygons for the masks.
fn ring_polys(cam: &Cam, cx: f64, cz: f64, r_in: f64, r_out: f64, y: f64) -> Vec<Poly> {
    annulus_polys(cam, cx, cz, r_in, r_out, y, 36, 0.0, 2.0 * PI)
        .into_iter()
        .map(|q| q.to_vec())
        .collect()
}

/// `draw_drumhead(arr, cam, cx, cz, r, a, col, main)`: the hoop (the fighter's colour) outside, the cream skin band
/// inside it (dithered while faint), six chrome lugs on a main pulse's hoop.
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_drumhead(arr: &mut Img<f32>, cam: &Cam, cx: f64, cz: f64, r: f64, a: f64, col: V3, main: bool) {
    let (w_hoop, w_skin) = (0.07, if main { 0.12 } else { 0.07 });
    let k = a * if main { 1.0 } else { 0.65 };
    let mut m_skin = mask_of_all(&ring_polys(
        cam,
        cx,
        cz,
        py_max(R * 0.9, r - w_hoop - w_skin),
        r - w_hoop,
        RING_Y,
    ));
    if k < 0.6 {
        for y in 0..m_skin.h {
            for x in 0..m_skin.w {
                let dz = (bayer(x, y) * 0.9 + 0.55 * k * 2.0).floor() / 2.0;
                m_skin.data[y * m_skin.w + x] *= dz as f32;
            }
        }
    }
    blend_into(arr, &m_skin, SKIN, Num::Py(0.62 * k));
    let m_hoop = mask_of_all(&ring_polys(cam, cx, cz, r - w_hoop, r, RING_Y));
    let hoop = col.map(|c| np::clip(c * 1.1 + 30.0, 0.0, 255.0));
    blend_into(arr, &m_hoop, hoop, Num::Py(0.92 * k));
    if main && k > 0.3 {
        for j in 0..6 {
            let ang = f64::from(j) * PI / 3.0 + 0.3;
            let s = cam.wp(&[scene::to_world(
                cx + r * dmath::cos(ang),
                cz + r * dmath::sin(ang),
                RING_Y + 0.02,
            )])[0];
            let (xi, yi) = (s.0.round(), s.1.round());
            if xi >= 0.0 && yi >= 0.0 && (xi as usize) < arr.w && (yi as usize) < arr.h {
                let at = arr.at(xi as usize, yi as usize);
                for (px, c) in arr.data[at..at + 3].iter_mut().zip(CHROME) {
                    *px = (f64::from(*px) * (1.0 - k) + c * k) as f32;
                }
            }
        }
    }
}

/// `cymbal_polys(cam, cx, cz, y, half)`: the cymbal's ring (0.92 R to 1.62 R) split into the half behind the ball's
/// centre (`half` 0) and the half in front of it (`half` 1), each quad with its middle angle.
fn cymbal_polys(cam: &Cam, cx: f64, cz: f64, y: f64, half: usize) -> Vec<(Poly, f64)> {
    let n = 40;
    let (r_in, r_out) = (R * 0.92, R * 1.62);
    let a: Vec<f64> = (0..=n).map(|j| 2.0 * PI * f64::from(j) / f64::from(n)).collect();
    let ring = |r: f64| -> Vec<V3> {
        a.iter()
            .map(|&u| scene::to_world(cx + r * dmath::cos(u), cz + r * dmath::sin(u), y))
            .collect()
    };
    let (oi, ii) = (ring(r_out), ring(r_in));
    let (so, si) = (cam.wp(&oi), cam.wp(&ii));
    let dz = cam.depth(&oi);
    let c = cam.depth(&[scene::to_world(cx, cz, y)])[0];
    (0..n as usize)
        .filter(|&j| ((dz[j] + dz[j + 1]) / 2.0 < c) == (half == 1))
        .map(|j| (vec![so[j], so[j + 1], si[j + 1], si[j]], (a[j] + a[j + 1]) / 2.0))
        .collect()
}

/// `draw_cymbal(arr, cam, cx, cz, t, k, half)`: bronze quads lit by a glint sweeping round as it spins, then two
/// darker lathe grooves.
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_cymbal(arr: &mut Img<f32>, cam: &Cam, cx: f64, cz: f64, t: f64, k: f64, half: usize) {
    let y = R * 1.02;
    let polys = cymbal_polys(cam, cx, cz, y, half);
    if polys.is_empty() {
        return;
    }
    let spin = t * 9.0;
    let glint = [255.0, 250.0, 230.0];
    for (pts, a) in polys {
        let g = 0.5 + 0.5 * dmath::cos(a - spin);
        let g2 = dmath::pow(py_max(0.0, dmath::cos(a - spin)), 18.0);
        let col: V3 = std::array::from_fn(|ch| {
            np::clip(
                BRONZE_DK[ch] + (BRONZE[ch] - BRONZE_DK[ch]) * g + glint[ch] * 0.35 * g2,
                0.0,
                255.0,
            )
        });
        let m = mask_of_all(&[pts]);
        blend_into(arr, &m, col, Num::Py(0.88 * k));
    }
    let c = cam.depth(&[scene::to_world(cx, cz, y)])[0];
    for rr in [1.15, 1.4] {
        let pts: Vec<V3> = (0..=40)
            .map(|j| {
                let u = 2.0 * PI * f64::from(j) / 40.0;
                scene::to_world(cx + R * rr * dmath::cos(u), cz + R * rr * dmath::sin(u), y + 0.005)
            })
            .collect();
        let s = cam.wp(&pts);
        let dz = cam.depth(&pts);
        let segs: Vec<Poly> = (0..40)
            .filter(|&j| ((dz[j] + dz[j + 1]) / 2.0 < c) == (half == 1))
            .map(|j| vec![s[j], s[j + 1]])
            .collect();
        if segs.is_empty() {
            continue;
        }
        let mk = mask_of_lines(&segs, Some(1));
        blend_into(arr, &mk, BRONZE_DK.map(|v| v * 0.8), Num::Py(0.6 * k));
    }
}

/// `beater_height(t, t0, t_slam, beat)`: the beater's head height: it hovers high, taps an 8th before the hit (the
/// "double"), stomps, stays down briefly and lifts away; `None` outside that.
fn beater_height(t: f64, t0: f64, t_slam: f64, beat: f64) -> Option<f64> {
    let tap = t_slam - beat / 2.0;
    if t < t0 {
        return None;
    }
    Some(if t < tap - 0.10 {
        3.2 - 0.4 * py_min(1.0, (t - t0) / 0.3)
    } else if t < tap {
        let u = (t - (tap - 0.10)) / 0.10;
        2.8 - 1.9 * u * u
    } else if t < tap + 0.08 {
        0.9 + 1.4 * ((t - tap) / 0.08)
    } else if t < t_slam - 0.08 {
        2.3 + 0.6 * py_min(1.0, (t - tap - 0.08) / 0.1)
    } else if t < t_slam {
        let u = (t - (t_slam - 0.08)) / 0.08;
        2.9 - 2.5 * u * u
    } else if t < t_slam + 0.25 {
        0.40
    } else if t < t_slam + 0.6 {
        let u = (t - t_slam - 0.25) / 0.35;
        0.40 + 3.0 * u * u
    } else {
        return None;
    })
}

/// `draw_beater(arr, cam, cx, cz, rad, h, alpha)`: the chrome shaft up to the pedal's pivot (off to the back), the
/// felt head (a squat disc seen from above) and its worn red spot.
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_beater(arr: &mut Img<f32>, cam: &Cam, cx: f64, cz: f64, rad: f64, h: f64, alpha: f64) {
    let head_r = rad * 0.55;
    let top = scene::to_world(cx, cz, h + 0.25);
    let piv = scene::to_world(cx - 0.9, cz + 0.9, h + 3.4);
    let s = cam.wp(&[top, piv]);
    let mk = mask_of_lines(&[vec![s[0], s[1]]], Some(3));
    blend_into(arr, &mk, SHAFT, Num::Py(alpha));
    let disc = |rr: f64, yy: f64| -> Poly {
        let pts: Vec<V3> = (0..24)
            .map(|j| {
                let u = 2.0 * PI * f64::from(j) / 24.0;
                scene::to_world(cx + rr * dmath::cos(u), cz + rr * dmath::sin(u), yy)
            })
            .collect();
        cam.wp(&pts)
    };
    for (yy, col, rr) in [(h, FELT.map(|v| v * 0.72), head_r), (h + 0.24, FELT, head_r * 0.96)] {
        let m = mask_of_all(&[disc(rr, yy)]);
        blend_into(arr, &m, col, Num::Py(alpha));
    }
    let m = mask_of_all(&[disc(head_r * 0.45, h + 0.245)]);
    blend_into(arr, &m, [200.0, 60.0, 52.0], Num::Py(0.5 * alpha));
}

/// A number of a recorded row.
fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Float(x) => Some(*x),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

/// One row of `RUN['SLAMS'][i]`: `(owner, cx, cz, t0, t_slam, radius)`.
type SlamRow = (i64, f64, f64, f64, f64, f64);

/// `RUN['SLAMS'][i]`: each slam in the air or just landed.
fn slams(p: &PictureCtx, i: usize) -> Vec<SlamRow> {
    let Some(Value::List(rows)) = p.run.nested("SLAMS", i) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            let Value::List(v) = row else { return None };
            let Value::Int(owner) = *v.first()? else { return None };
            Some((
                owner,
                num(v.get(1)?)?,
                num(v.get(2)?)?,
                num(v.get(3)?)?,
                num(v.get(4)?)?,
                num(v.get(5)?)?,
            ))
        })
        .collect()
}

/// DRUMS's look.
#[derive(Clone, Debug, Default)]
pub struct DrumsLook {
    k: usize,
    pulses: Vec<Pulse>,
    /// `PULSE_T`: when each pulse fired (the pulses in that order).
    pulse_t: Vec<f64>,
    /// The weapon's ring speed.
    speed: f64,
    /// CRASH CYMBAL's seconds.
    shield: Option<f64>,
}

impl DrumsLook {
    /// `pulse_list`: the pulses on screen at `t`, frozen with the hit-stop and gone 0.3 s after it.
    fn pulse_list(&self, p: &PictureCtx, t: f64) -> Vec<Pulse> {
        if self.pulses.is_empty() || !(p.show.grid.t_fight <= t && t < p.t_hs + 0.3) {
            return Vec::new();
        }
        let t_rings = py_min(t, p.t_ko);
        let j0 = self.pulse_t.partition_point(|&v| v < t_rings - 0.6);
        let j1 = self.pulse_t.partition_point(|&v| v <= t_rings);
        self.pulses[j0..j1.max(j0)].to_vec()
    }

    /// CRASH CYMBAL's strength, fading in over 0.12 s and out over the last 0.2 s.
    fn shield_k(&self, st: &BallState) -> f64 {
        let Some(dur) = self.shield else { return 0.0 };
        if st.shield <= 0.0 {
            return 0.0;
        }
        py_min(py_min(1.0, st.shield / 0.2), (dur - st.shield) / 0.12 + 0.2)
    }
}

impl Look for DrumsLook {
    fn spark_big(&self) -> &[&str] {
        &["pulse", "slam"]
    }

    fn flash_hits(&self) -> &[&str] {
        &["pulse", "slam"]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["slam"]
    }

    /// A hit on CRASH CYMBAL: a bronze spray.
    fn block_burst(&self) -> [f64; 3] {
        [255.0, 214.0, 120.0]
    }

    fn block_spark(&self) -> Spark {
        ([255.0, 220.0, 140.0], 14, 2.8, 0.3)
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.speed = match &cfg.weapon {
            WeaponCfg::Pulse(w) => w.speed,
            _ => 9.0,
        };
        self.pulses = p
            .rec
            .events
            .iter()
            .filter(|e| e.kind == "pulse" && e.who() == Some(k))
            .map(|e| {
                let at = p.fighter(k, e.t).p;
                Pulse {
                    t0: e.t,
                    main: e.i(3) != 0,
                    reach: e.f(4),
                    cx: at[0],
                    cz: at[1],
                }
            })
            .collect();
        self.pulse_t = self.pulses.iter().map(|q| q.t0).collect();
        let sh = [&cfg.unlock1, &cfg.unlock2, &cfg.low]
            .into_iter()
            .find(|a| a.kind == AbilityKind::Shield);
        self.shield = sh.map(|a| a.get("dur_beats").unwrap_or(6.0) * p.show.grid.beat);
    }

    /// The drum ball thumps on every pulse.
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if kk != self.k || phase != Phase::Fight || self.pulse_t.is_empty() {
            return;
        }
        let tt = py_min(t, p.t_ko);
        let after = self.pulse_t.partition_point(|&v| v <= tt);
        if let Some(j) = after.checked_sub(1) {
            st.glow = 0.45 * dmath::exp(-(tt - self.pulse_t[j]) / 0.09);
        }
    }

    /// The drum's reach (a faint dashed ring), then each pulse lighting the canvas under its ring.
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let col = p.col[self.k];
        let st = &c.fs[self.k];
        if c.in_fight && st.vis {
            let tint = col.map(|v| v / 255.0 + 0.2);
            for (j, light) in ic.iter_mut().enumerate() {
                let (dx, dz) = (c.sxp[j] - st.p[0], c.szp[j] - st.p[1]);
                let ring = np::clip(1.0 - (np::hypot(dx, dz) - (R + st.blen)).abs() / 0.05, 0.0, 1.0);
                if ring <= 0.0 {
                    continue;
                }
                let dash = ((dmath::atan2(dz, dx) + PI) * 9.0).floor().rem_euclid(2.0) == 0.0;
                if dash {
                    for ch in 0..3 {
                        light[ch] += ring * 0.20 * tint[ch];
                    }
                }
            }
        }
        let t_rings = py_min(c.t, p.t_ko);
        let tint = col.map(|v| v / 255.0 * 0.9 + 0.25);
        for q in self.pulse_list(p, c.t) {
            let Some((r, a)) = pulse_state(t_rings, q.t0, q.reach, self.speed) else {
                continue;
            };
            let k_ = a * if q.main { 0.55 } else { 0.3 };
            for (j, light) in ic.iter_mut().enumerate() {
                let rd = np::hypot(c.sxp[j] - q.cx, c.szp[j] - q.cz);
                if rd >= r + 0.05 {
                    continue;
                }
                let band = np::clip(1.0 - (rd - r).abs() / 0.22, 0.0, 1.0);
                let band = (band * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
                for ch in 0..3 {
                    light[ch] += band * k_ * tint[ch];
                }
            }
        }
    }

    /// Its rings are the drumheads (drawn in [`Look::canvas`] and [`Look::canvas_overlay`]), not wave bands.
    fn canvas_wave(&self, _p: &PictureCtx, _ic: &mut [V3], _c: &CanvasCtx<'_>, _w: &Wave) {}

    /// The drumhead rings, on the lit canvas.
    fn canvas_overlay(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        let col = p.col[self.k];
        for q in self.pulse_list(p, f.t) {
            let tt = if q.t0 < p.t_ko { py_min(f.t, p.t_ko) } else { f.t };
            if let Some((r, a)) = pulse_state(tt, q.t0, q.reach, self.speed) {
                draw_drumhead(arr, f.cam, q.cx, q.cz, r, a, col, q.main);
            }
        }
    }

    fn items_ball(&self, _p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        let sk = self.shield_k(st);
        if sk <= 0.0 {
            return Vec::new();
        }
        let dz = f.cam.depth(&[scene::to_world(st.p[0], st.p[1], R)])[0];
        let back: Payload = Box::new(Item::Cymbal(0, sk));
        let front: Payload = Box::new(Item::Cymbal(1, sk));
        vec![(dz + 0.05, back), (dz - 0.05, front)]
    }

    fn draw_item(&self, _p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let at = f.fs[self.k].p;
        if let Some(Item::Cymbal(half, k)) = payload.downcast_ref::<Item>() {
            let (mut arr, depth) = (fr.array(), fr.depth());
            draw_cymbal(&mut arr, f.cam, at[0], at[1], f.t, *k, *half);
            fr.set_array(&arr, Some(&depth));
        }
    }

    /// DOUBLE KICK: the beater, a tap an 8th early, then the stomp.
    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, _fr: &mut Frame, arr: &mut Img<f32>, _depth: &mut Img<f32>) {
        if !f.in_fight {
            return;
        }
        let beat = p.show.grid.beat;
        for (owner, cx, cz, t0, t_slam, rad) in slams(p, f.i) {
            if owner != self.k as i64 {
                continue;
            }
            let t = f.t;
            if let Some(h) = beater_height(t, t0, t_slam, beat) {
                let fade = if t < t_slam + 0.4 {
                    1.0
                } else {
                    py_max(0.0, 1.0 - (t - t_slam - 0.4) / 0.2)
                };
                let a = py_min(1.0, (t - t0) / 0.2) * fade;
                draw_beater(arr, f.cam, cx, cz, rad, h, a);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pulse_blasts_out_holds_and_fades() {
        assert_eq!(pulse_state(0.9, 1.0, 0.9, 9.0), None);
        let (r, a) = pulse_state(1.05, 1.0, 0.9, 9.0).expect("out");
        assert!((r - (R + 0.45)).abs() < 1e-9 && a == 1.0);
        let (r, a) = pulse_state(1.2, 1.0, 0.9, 9.0).expect("fading");
        assert!((r - (R + 0.9)).abs() < 1e-9 && a < 1.0);
        assert_eq!(pulse_state(1.4, 1.0, 0.9, 9.0), None);
    }

    #[test]
    fn the_beater_taps_then_stomps() {
        let (t0, ts, beat) = (0.0, 1.0, 0.42);
        assert_eq!(beater_height(-0.1, t0, ts, beat), None);
        let tap = ts - beat / 2.0;
        assert!((beater_height(tap, t0, ts, beat).expect("tap") - 0.9).abs() < 1e-9);
        assert_eq!(beater_height(ts + 0.1, t0, ts, beat), Some(0.40));
        assert_eq!(beater_height(ts + 0.7, t0, ts, beat), None);
    }
}
