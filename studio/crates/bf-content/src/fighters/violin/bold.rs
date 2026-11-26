//! VIOLIN with the bold effects (`picture.effects = "bold"`; the owner's picks 6a to 6d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **SPINNING BOW** (6a): a thick outlined bow with a broad white hair ribbon; its trail is a stepped fan that
//!   grows with the spin, from a short fan through a half disc to a full disc with two propeller ghosts. Each
//!   hit leaves a white crescent slash and a puff of rosin dust, and each speed-up pops a ring out along the
//!   tip's circle.
//! - **PIZZICATO** (6b): a string across the ball is pulled back with a glint while five lanes fan out on the
//!   floor, then it snaps and hums; the darts are fat outlined teardrops with a streak; the ones that land pop
//!   in a violet ring with droplets.
//! - **TREMOLO** (6c): the bow shivers inside a jagged outline that flips every frame, and its trail turns violet
//!   with a saw-toothed rim (the ring adds the speed-up look and the draining ring).
//! - **SCREECH** (6d): as a warning a little bow drags slowly across a string on the ball with scratch marks and
//!   a jagged ring trembles under it; then noisy jagged rings scream out, three deep; the enemy it hits shakes,
//!   with shake marks either side.

use bf_core::dmath;
use bf_core::event::Event;
use bf_core::roster::FighterCfg;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize, INK, WHITE};
use bf_picture::camera::V3;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{PictureCtx, layer_blend};
use bf_picture::scene::{self, R_BALL as R};

use super::{FROG, HAIR, STICK_HI, TIP, ViolinLook};

/// Rosin dust.
const ROSIN: V3 = [255.0, 238.0, 196.0];
/// Seconds a crescent slash shows, and the rosin dust drifts.
const SLASH_S: f64 = 0.22;
const DUST_S: f64 = 0.4;
/// Seconds a speed-up's ring and a landed dart's pop show.
const POP_S: f64 = 0.25;
/// Seconds the string is pulled back before PIZZICATO, and hums after it.
const PLUCK_IN_S: f64 = 0.3;
const HUM_S: f64 = 0.3;
/// Seconds the bow drags across the string before SCREECH.
const DRAG_S: f64 = 0.5;
/// Seconds the enemy shakes after SCREECH hits it.
const SHAKE_S: f64 = 0.5;

/// One painter item of the bold look.
enum BItem {
    /// The bow and its trail.
    Bow,
    /// A dart in flight: x, z, heading.
    Dart(f64, f64, f64),
}

/// One of the bow's slashes that landed: when, where, and whether it sped the bow up.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Slash {
    t: f64,
    x: f64,
    z: f64,
    faster: bool,
}

/// What the bold look works out once from the recording.
#[derive(Clone, Debug, Default)]
pub(super) struct Bold {
    /// The bow's spin as the fight starts and at its fastest.
    spin: (f64, f64),
    /// PIZZICATO's fan: how many lanes, and its sweep in degrees.
    pizz: (usize, f64),
    slashes: Vec<Slash>,
    /// Landed darts: when and where.
    pops: Vec<(f64, f64, f64)>,
    /// When PIZZICATO fired, when SCREECH fired, and when SCREECH hit the enemy.
    plucks: Vec<f64>,
    screeches: Vec<f64>,
    shakes: Vec<f64>,
}

/// A number from 0 to 1 that is always the same for the same `(a, b)`.
fn hash(a: i64, b: i64) -> f64 {
    let v = dmath::sin(a as f64 * 12.9898 + b as f64 * 78.233) * 43_758.545_3;
    v - v.floor()
}

/// The bold bow's world quads from the ball's surface to the tip: hair, stick, frog, tip. `sd` is the side the
/// hair is on (the leading one).
fn bow_quads(p: [f64; 2], th: f64, blen: f64, sd: f64) -> [([V3; 4], V3); 4] {
    let d = [dmath::cos(th), dmath::sin(th)];
    let nrm = [-d[1] * sd, d[0] * sd];
    let (r0, r1) = (R * 0.86, R + blen);
    let wp = |r: f64, w: f64, lift: f64| {
        scene::to_world(p[0] + d[0] * r + nrm[0] * w, p[1] + d[1] * r + nrm[1] * w, R + lift)
    };
    let quad = |a: (f64, f64), b: (f64, f64), c: (f64, f64), e: (f64, f64), lift: f64| {
        [
            wp(a.0, a.1, lift),
            wp(b.0, b.1, lift),
            wp(c.0, c.1, lift),
            wp(e.0, e.1, lift),
        ]
    };
    [
        (
            quad(
                (r0 + 0.2, 0.10),
                (r1 - 0.08, 0.07),
                (r1 - 0.08, 0.15),
                (r0 + 0.2, 0.19),
                0.0,
            ),
            HAIR,
        ),
        (
            quad((r0, -0.055), (r1, -0.04), (r1, 0.04), (r0, 0.055), 0.012),
            STICK_HI,
        ),
        (
            quad(
                (r0 + 0.02, -0.07),
                (r0 + 0.28, -0.07),
                (r0 + 0.26, 0.23),
                (r0 + 0.05, 0.23),
                0.02,
            ),
            FROG,
        ),
        (
            quad(
                (r1 - 0.18, -0.06),
                (r1 + 0.04, -0.06),
                (r1 + 0.04, 0.17),
                (r1 - 0.11, 0.17),
                0.02,
            ),
            TIP,
        ),
    ]
}

impl ViolinLook {
    /// The slashes, landed darts and moves of side `k`, from its events and its spin.
    pub(super) fn bold_setup(p: &PictureCtx, k: usize, cfg: &FighterCfg) -> Bold {
        let weapon = &cfg.raw["weapon"];
        let spin0 = weapon["spin"].as_f64().unwrap_or(2.4);
        let spin_max = weapon["spin_max"].as_f64().unwrap_or(spin0 + 1.0);
        let mine = |e: &&Event| e.who() == Some(k);
        let hits = |how: &'static str| {
            p.rec
                .events
                .iter()
                .filter(mine)
                .filter(move |e| e.kind == "hit" && e.s(7) == how)
        };
        let abilities = |kind: &'static str| -> Vec<f64> {
            p.rec
                .events
                .iter()
                .filter(mine)
                .filter(|e| e.kind == "ability" && e.s(4) == kind)
                .map(|e| e.t)
                .collect()
        };
        let pizz = &cfg.raw["abilities"]["unlock1"];
        Bold {
            spin: (spin0, spin_max),
            pizz: (
                pizz["n"].as_f64().unwrap_or(5.0).clamp(1.0, 9.0) as usize,
                pizz["sweep"].as_f64().unwrap_or(64.0),
            ),
            slashes: hits("blade")
                .map(|e| {
                    let i = p.sim_i(e.t);
                    let before = p.run.f("OM", i.saturating_sub(1), k).unwrap_or(spin_max);
                    Slash {
                        t: e.t,
                        x: e.f(5),
                        z: e.f(6),
                        faster: before < spin_max - 1e-6,
                    }
                })
                .collect(),
            pops: hits("key").map(|e| (e.t, e.f(5), e.f(6))).collect(),
            plucks: abilities("arc"),
            screeches: abilities("wave"),
            shakes: hits("wave").map(|e| e.t).collect(),
        }
    }

    /// How far the event of `times` that is next after `t` is, within `window`: 0 as the window opens, 1 as it
    /// happens.
    fn until(times: &[f64], t: f64, window: f64) -> Option<f64> {
        times
            .iter()
            .find(|&&te| t < te && te - window <= t)
            .map(|&te| 1.0 - (te - t) / window)
    }

    /// The seconds since the event of `times` most lately at or before `t`, if under `window`.
    fn since(times: &[f64], t: f64, window: f64) -> Option<f64> {
        times
            .iter()
            .rev()
            .find(|&&te| te <= t)
            .map(|&te| t - te)
            .filter(|&age| age < window)
    }

    /// The enemy shakes for a moment after SCREECH hits it.
    pub(super) fn bold_state(&self, kk: usize, st: &mut BallState, t: f64) {
        if kk != 1 - self.k || st.shattered {
            return;
        }
        if let Some(age) = Self::since(&self.bold.shakes, t, SHAKE_S) {
            let k = 1.0 - age / SHAKE_S;
            st.p[0] += 0.07 * k * dmath::sin(t * 75.0);
            st.p[1] += 0.05 * k * dmath::cos(t * 93.0);
        }
    }

    /// A landed dart is a solid hit whatever its damage.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match e.s(7) {
            "key" => Some(by_damage.max(HitSize::Solid)),
            _ => Some(by_damage),
        }
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let k = self.k;
        let (me, foe) = (c.fs[k].p, c.fs[1 - k].p);
        let tint = bold::light(p.col[k]);
        // PIZZICATO: the lanes of the fan to come
        if let Some(u) = Self::until(&self.bold.plucks, c.t, PLUCK_IN_S) {
            let (n, sweep) = self.bold.pizz;
            let sweep = sweep.to_radians();
            let aim = dmath::atan2(foe[1] - me[1], foe[0] - me[0]);
            for j in 0..n {
                let ang = aim + sweep * (j as f64 / (n.max(2) - 1) as f64 - 0.5);
                let (dx, dz) = (dmath::cos(ang), dmath::sin(ang));
                let far = R + 2.4 * bold::ease_out(u);
                floor::lane(
                    ic,
                    c,
                    [me[0] + dx * R, me[1] + dz * R],
                    [me[0] + dx * far, me[1] + dz * far],
                    0.05,
                    tint,
                    0.25 + 0.5 * u,
                );
            }
        }
        // SCREECH: a jagged ring trembles under the ball while the bow drags
        if let Some(u) = Self::until(&self.bold.screeches, c.t, DRAG_S) {
            floor::band(
                ic,
                c,
                Family::Noise,
                me[0],
                me[1],
                R * 1.3 + 0.3 * u,
                0.4 + 0.6 * u,
                c.t,
                tint,
                0.5 + 0.6 * u,
            );
        }
    }

    pub(super) fn bold_canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        // SCREECH: scratchy, jagged rings, three deep
        let tint = bold::light(p.col[w.owner]);
        // (they stay strong across the ring: the ring's own fade runs to far outside the ropes)
        let fade = (1.0 - w.r / 7.5).clamp(0.0, 1.0);
        for j in 0..3 {
            let r = w.r - 0.5 * f64::from(j);
            if r > 0.4 {
                floor::band(
                    ic,
                    c,
                    Family::Noise,
                    w.cx,
                    w.cz,
                    r,
                    fade.sqrt() * (1.0 - 0.25 * f64::from(j)),
                    c.t + 0.37 * f64::from(j),
                    tint,
                    if j == 0 { 2.2 } else { 1.4 },
                );
            }
        }
    }

    pub(super) fn bold_items_ball(&self, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        if st.shattered {
            return Vec::new();
        }
        let reach = R + st.blen * 0.55;
        let tip = [st.p[0] + reach * dmath::cos(st.th), st.p[1] + reach * dmath::sin(st.th)];
        let depth = f.cam.depth(&[scene::to_world(tip[0], tip[1], R)])[0] + 0.01;
        let bow: Payload = Box::new(BItem::Bow);
        vec![(depth, bow)]
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
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
                let dart: Payload = Box::new(BItem::Dart(row[0], row[1], row[2]));
                (depth, dart)
            })
            .collect()
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let k = self.k;
        match payload.downcast_ref::<BItem>() {
            Some(BItem::Bow) => {
                let st = &f.fs[k];
                let tremolo = st.haste > 0.0 && f.in_fight;
                self.bold_trail(p, f, fr, st, tremolo);
                // TREMOLO: the bow shivers inside a jagged outline that flips every frame
                if tremolo {
                    let d = [dmath::cos(st.th), dmath::sin(st.th)];
                    let nrm = [-d[1] * st.sd, d[0] * st.sd];
                    let flip = if f.f.is_multiple_of(2) { 1.0 } else { -1.0 };
                    let n = 9;
                    let side = |w0: f64, out: f64, i: usize| {
                        let r = R * 0.9 + (st.blen + 0.14) * i as f64 / (n - 1) as f64;
                        let zig = if i.is_multiple_of(2) { flip } else { -flip };
                        let w = w0 + out * (0.07 + 0.06 * zig + 0.04 * hash(f.f as i64, i as i64));
                        scene::to_world(
                            st.p[0] + d[0] * r + nrm[0] * w,
                            st.p[1] + d[1] * r + nrm[1] * w,
                            R + 0.03,
                        )
                    };
                    let mut pts: Vec<V3> = (0..n).map(|i| side(-0.1, -1.0, i)).collect();
                    pts.extend((0..n).rev().map(|i| side(0.24, 1.0, i)));
                    pts.push(pts[0]);
                    fr.line(f.cam, &pts, INK, 4);
                    fr.line(f.cam, &pts, bold::pale(p.col[k]), 2);
                }
                for (q, col) in bow_quads(st.p, st.th, st.blen, st.sd) {
                    bold::outlined_poly(fr, f.cam, &q, col);
                }
                // the frog's pearl eye
                let d = [dmath::cos(st.th), dmath::sin(st.th)];
                let nrm = [-d[1] * st.sd, d[0] * st.sd];
                let r = R * 0.86 + 0.15;
                let eye = scene::to_world(
                    st.p[0] + d[0] * r + nrm[0] * 0.08,
                    st.p[1] + d[1] * r + nrm[1] * 0.08,
                    R + 0.04,
                );
                bold::blob(fr, f.cam, eye, 0.04, [226.0, 236.0, 255.0]);
            }
            Some(&BItem::Dart(x, z, ang)) => {
                // a fat teardrop, round end first, with a light spot and a streak behind
                let (fx, fz) = (dmath::cos(ang), dmath::sin(ang));
                let at = |a: f64, b: f64| scene::to_world(x + fx * a - fz * b, z + fz * a + fx * b, 0.45);
                fr.line(f.cam, &[at(-0.95, 0.0), at(-0.45, 0.0)], bold::pale(p.col[k]), 2);
                let drop = [
                    (-0.5, 0.0),
                    (-0.12, 0.14),
                    (0.08, 0.19),
                    (0.22, 0.14),
                    (0.29, 0.0),
                    (0.22, -0.14),
                    (0.08, -0.19),
                    (-0.12, -0.14),
                ]
                .map(|(a, b)| at(a, b));
                bold::outlined_poly(fr, f.cam, &drop, p.col[k]);
                let spot = [(0.0, 0.0), (0.1, 0.08), (0.2, 0.0), (0.1, -0.08)].map(|(a, b)| at(a, b));
                fr.poly(f.cam, &spot, bold::pale(p.col[k]), true);
            }
            None => {}
        }
    }

    /// The bow's trail: a fan in flat steps that grows with the spin, from a short fan through a half disc to a
    /// full disc with two propeller ghosts; violet with a saw-toothed rim under TREMOLO.
    fn bold_trail(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, st: &BallState, tremolo: bool) {
        let (spin0, spin_max) = self.bold.spin;
        let u = ((st.om - spin0) / (spin_max - spin0).max(1e-6)).clamp(0.0, 1.0);
        let mut sweep = 0.45 + (dmath::TAU - 0.45) * u;
        if tremolo {
            sweep = (sweep * 1.6).min(dmath::TAU);
        }
        let full = sweep > dmath::TAU * 0.96;
        let (r0, r1) = (R + st.blen * 0.25, R + st.blen);
        let col = if tremolo {
            p.col[self.k].map(|c| c * 0.6 + 100.0)
        } else {
            [240.0, 236.0, 226.0]
        };
        let at = |a: f64, r: f64| scene::to_world(st.p[0] + r * dmath::cos(a), st.p[1] + r * dmath::sin(a), R);
        let (mut arr, depth) = (fr.array(), fr.depth());
        // three flat steps of strength, the strongest at the bow; each one strip, so no seams show inside it
        let n = ((sweep / 3.0 / 0.16).ceil() as usize).clamp(2, 14);
        for step in 0..3 {
            let ang = |i: usize| st.th - st.sd * sweep * (step as f64 + i as f64 / n as f64) / 3.0;
            let mut strip: Vec<V3> = (0..=n).map(|i| at(ang(i), r0)).collect();
            strip.extend((0..=n).rev().map(|i| {
                let saw = tremolo && i % 2 == 1;
                at(ang(i), if saw { r1 * 0.92 } else { r1 })
            }));
            let k = f64::from(3 - step) / 3.0;
            let a = if full { 0.16 + 0.3 * k } else { 0.46 * k };
            layer_blend(&mut arr, &[f.cam.wp(&strip)], col, a, None);
        }
        if full {
            // propeller ghosts: the bow a third and two thirds of a turn back
            for g in 1..=2 {
                let th = st.th - st.sd * dmath::TAU * f64::from(g) / 3.0;
                let (q, _) = bow_quads(st.p, th, st.blen, st.sd)[1];
                layer_blend(&mut arr, &[f.cam.wp(&q)], col, 0.55, None);
            }
        }
        fr.set_array(&arr, Some(&depth));
        // the tip's path: a crisp arc along the newest part of the fan
        let arc: Vec<V3> = (0..=10)
            .map(|i| at(st.th - st.sd * sweep.min(2.2) * f64::from(i) / 10.0, r1))
            .collect();
        fr.line(f.cam, &arc, if tremolo { bold::pale(p.col[self.k]) } else { WHITE }, 2);
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let (k, t) = (self.k, f.t);
        let o = 1 - k;
        let col = p.col[k];
        let st = &f.fs[k];
        // each slash that landed: a white crescent, rosin dust, and a ring along the tip's circle if it sped up
        for s in self.bold.slashes.iter().filter(|s| s.t <= t && t < s.t + DUST_S) {
            let age = t - s.t;
            let b = p.run.p(p.sim_i(s.t), k);
            let ang0 = dmath::atan2(s.z - b[1], s.x - b[0]);
            let rr = dmath::hypot(s.x - b[0], s.z - b[1]).clamp(R * 1.3, R + st.blen);
            if age < SLASH_S {
                let half = 0.3 + 0.45 * bold::ease_out(age / 0.08);
                let n = 10;
                let edge = |i: usize, out: f64| {
                    let v = i as f64 / n as f64 * 2.0 - 1.0;
                    let a = ang0 + st.sd * half * v;
                    let r = rr + out * 0.2 * (1.0 - v * v);
                    bold::at(f.cam, b[0] + r * dmath::cos(a), b[1] + r * dmath::sin(a), R)
                };
                let mut moon: Vec<(f64, f64)> = (0..=n).map(|i| edge(i, 1.0)).collect();
                moon.extend((0..=n).rev().map(|i| edge(i, -0.25)));
                let a = if age < 0.1 {
                    1.0
                } else {
                    1.0 - (age - 0.1) / (SLASH_S - 0.1)
                };
                px::ink_poly(img, &moon, WHITE, a);
            }
            let u = age / DUST_S;
            let seed = (s.t * 1000.0) as i64;
            for j in 0..7i64 {
                let a = ang0 + st.sd * (1.2 + (hash(seed, j) - 0.5) * 1.6);
                let far = (0.15 + 0.6 * hash(seed, j + 11)) * bold::ease_out(u);
                let at = bold::at(
                    f.cam,
                    s.x + far * dmath::cos(a),
                    s.z + far * dmath::sin(a),
                    R + 0.35 * u,
                );
                px::ink_disc(img, at, 1.9 - 0.9 * u, ROSIN, 1.0 - u * u);
            }
            if s.faster && age < POP_S {
                let v = age / POP_S;
                let r = (R + st.blen) * (1.0 + 0.1 * bold::ease_out(v));
                let ring: Vec<(f64, f64)> = (0..=40)
                    .map(|i| {
                        let a = f64::from(i) / 40.0 * dmath::TAU;
                        bold::at(f.cam, st.p[0] + r * dmath::cos(a), st.p[1] + r * dmath::sin(a), R)
                    })
                    .collect();
                px::polyline(img, &ring, 2.4 - 1.2 * v, bold::pale(col), 0.9 * (1.0 - v));
            }
        }
        // a landed dart pops: a violet ring and droplets
        for &(tp, x, z) in self.bold.pops.iter().filter(|h| h.0 <= t && t < h.0 + POP_S) {
            let u = (t - tp) / POP_S;
            let c = bold::at(f.cam, x, z, 0.45);
            px::ink_ring(
                img,
                c,
                5.0 + 11.0 * bold::ease_out(u),
                2.2 - 1.2 * u,
                bold::pale(col),
                1.0 - u * u,
            );
            for j in 0..6 {
                let a = f64::from(j) * dmath::TAU / 6.0 + 0.5;
                let far = 8.0 + 14.0 * bold::ease_out(u);
                px::ink_disc(
                    img,
                    (c.0 + far * dmath::cos(a), c.1 + far * dmath::sin(a) * 0.8),
                    2.2 - 1.2 * u,
                    col,
                    1.0 - u,
                );
            }
        }
        let (Some(d), Some(e)) = (drawn.iter().find(|d| d.k == k), drawn.iter().find(|d| d.k == o)) else {
            return;
        };
        if st.shattered {
            return;
        }
        // PIZZICATO: a string across the ball is pulled back from the enemy, then snaps and hums
        let pull = Self::until(&self.bold.plucks, t, PLUCK_IN_S);
        let hum = Self::since(&self.bold.plucks, t, HUM_S);
        if pull.is_some() || hum.is_some() {
            let n = dmath::hypot(e.s.0 - d.s.0, e.s.1 - d.s.1).max(1e-9);
            let (ax, ay) = ((e.s.0 - d.s.0) / n, (e.s.1 - d.s.1) / n);
            let ends = [
                (d.s.0 - ay * d.rp * 1.05, d.s.1 + ax * d.rp * 1.05),
                (d.s.0 + ay * d.rp * 1.05, d.s.1 - ax * d.rp * 1.05),
            ];
            let back = match (pull, hum) {
                (Some(u), _) => d.rp * 1.1 * bold::ease_in(u),
                (None, Some(age)) => -d.rp * 0.55 * dmath::cos(age * 70.0) * dmath::exp(-age / 0.09),
                _ => 0.0,
            };
            let mid = (d.s.0 - ax * back, d.s.1 - ay * back);
            px::ink_polyline(img, &[ends[0], mid, ends[1]], 2.0, WHITE, 1.0);
            if let Some(u) = pull {
                px::ink_disc(img, mid, 2.6, bold::pale(col), 1.0);
                if u > 0.6 {
                    px::glint(img, mid, 5.0 + 9.0 * (u - 0.6) / 0.4, WHITE, 1.0);
                }
            } else {
                // the humming string's ghosts
                for side in [-1.0, 1.0] {
                    let g = (mid.0 + ax * back * side * 0.6, mid.1 + ay * back * side * 0.6);
                    px::polyline(img, &[ends[0], g, ends[1]], 1.0, bold::pale(col), 0.6);
                }
            }
        }
        // SCREECH's warning: a little bow drags slowly across a string on the ball, with scratch marks
        if let Some(u) = Self::until(&self.bold.screeches, t, DRAG_S) {
            let jit = if f.f.is_multiple_of(2) { 1.0 } else { -1.0 };
            px::ink_line(
                img,
                (d.s.0 + jit, d.s.1 - d.rp * 1.15),
                (d.s.0 - jit, d.s.1 + d.rp * 1.15),
                1.6,
                WHITE,
                1.0,
            );
            // the bow: stick and hair, sliding along itself
            let (bx, by) = (dmath::cos(-0.28), dmath::sin(-0.28));
            let slide = d.rp * (0.9 - 1.8 * u);
            let c = (d.s.0 + bx * slide, d.s.1 + by * slide + jit * 0.6);
            let half = d.rp * 1.7;
            let (p0, p1) = ((c.0 - bx * half, c.1 - by * half), (c.0 + bx * half, c.1 + by * half));
            px::ink_line(img, (p0.0, p0.1 + 3.4), (p1.0, p1.1 + 3.4), 1.6, HAIR, 1.0);
            px::ink_line(img, p0, p1, 2.6, STICK_HI, 1.0);
            px::ink_disc(img, (p0.0, p0.1 + 1.7), 2.6, FROG, 1.0);
            // scratch marks jump off the string where the bow drags
            for j in 0..3i64 {
                let a = -1.1 + 0.55 * j as f64 + 0.2 * jit;
                let (r_in, r_out) = (d.rp * 1.25, d.rp * (1.55 + 0.35 * u));
                let zig: Vec<(f64, f64)> = (0..5)
                    .map(|i| {
                        let r = r_in + (r_out - r_in) * f64::from(i) / 4.0;
                        let w = a + 0.09 * if i % 2 == 0 { jit } else { -jit };
                        (d.s.0 + r * dmath::cos(w), d.s.1 + r * dmath::sin(w) * 0.9 - d.rp * 0.2)
                    })
                    .collect();
                px::ink_polyline(img, &zig, 1.4, bold::pale(col), 0.5 + 0.5 * u);
            }
        }
        // the enemy shakes after SCREECH hits it: shake marks either side
        if let Some(age) = Self::since(&self.bold.shakes, t, SHAKE_S)
            && !f.fs[o].shattered
        {
            let a = 1.0 - age / SHAKE_S;
            let wob = if f.f.is_multiple_of(2) { 0.0 } else { 3.0 };
            for side in [0.0, dmath::PI] {
                for j in 0..2 {
                    let r = e.rp + 5.0 + 5.0 * f64::from(j) + wob;
                    px::ink_arc(img, e.s, r, 1.6, side - 0.5, side + 0.5, bold::pale(col), a);
                }
            }
        }
    }
}
