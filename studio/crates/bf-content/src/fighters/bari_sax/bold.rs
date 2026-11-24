//! BARI SAX with the bold effects (`picture.effects = "bold"`; the owner's picks 40a to 40d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **WRECKING BALL** (40a): the crane is a thick chain with a big hook block and hook; from the moment the
//!   cable drops a dotted arc in the air and a dashed lane on the canvas show the coming swing; the ball is
//!   stretched as it is hoisted back, with a glint at the top; it swings with solid afterimages, holds where it
//!   touched for three frames, and the enemy flies off with speed lines.
//! - **GROWL** (40b): a toothed ring closes in on the shaking ball as the growl builds; the shock wave is a
//!   jagged orange ring on the canvas with a thick toothed front; the pushed enemy trails speed lines.
//! - **SKANK** (40c): a ticker of the bar's eight eighth notes over the ring's far corner, the off-beats big; a
//!   cursor runs along it and every smash stamps a star on the pip it lands on.
//! - **CABLE SNAP** (40d): a target closes in on the darkening spot; the chain frays with sparks where it will
//!   part; it snaps with a white burst and whips away, the ball falls with speed lines; a dark crater, a quake
//!   ring and chips.

use bf_core::dmath;
use bf_picture::bold;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Phase, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::scene::{self, R_BALL as R};

use super::state::Src;
use super::{BariSaxLook, Crane, CraneItem, PicSrc, STEEL, STEEL_HI, VERT, add3, cable, sub3, unit2, unit3, w_at};

/// GROWL's orange, as paint and as canvas light.
const ORANGE: V3 = [255.0, 150.0, 40.0];
const ORANGE_LIGHT: V3 = [1.7, 0.95, 0.3];
/// Seconds a growl builds before it rings out.
const BUILD_S: f64 = 0.4;
/// Seconds an enemy sent flying trails speed lines.
const FLY_S: f64 = 0.45;
/// Seconds the crater of CABLE SNAP stays dark, and its quake ring runs.
const CRATER_S: f64 = 0.8;
const QUAKE_S: f64 = 0.55;
/// Seconds the break's white burst shows.
const BREAK_S: f64 = 0.2;
/// Seconds GROWL's ring lingers at its full size.
const LINGER_S: f64 = 0.3;

/// A chain along the screen polyline `pts`: an inked steel line with a dark link mark every few pixels.
fn chain<S: px::Surface + ?Sized>(s: &mut S, pts: &[(f64, f64)], w: f64, col: V3, a: f64) {
    if a <= 0.02 {
        return;
    }
    px::ink_polyline(s, pts, w, col, a);
    let mut run: f64 = 0.0;
    for seg in pts.windows(2) {
        let (dx, dy) = (seg[1].0 - seg[0].0, seg[1].1 - seg[0].1);
        let len = dmath::hypot(dx, dy);
        if len <= 0.0 {
            continue;
        }
        let mut at = (5.0 - run).rem_euclid(5.0);
        while at < len {
            let q = (seg[0].0 + dx * at / len, seg[0].1 + dy * at / len);
            px::disc(s, q, 0.9, bold::INK, a);
            at += 5.0;
        }
        run = (run + len).rem_euclid(5.0);
    }
}

/// The crane in the bold look: the rig line, the hook block with a stripe in the fighter's colour, the chain to
/// the ball and its hook, the snapped pieces.
fn draw_bold_crane(arr: &mut Img<f32>, cam: &Cam, cs: &Crane, col: V3) {
    let blk = cs.block;
    let rig = cam.wp(&cable(cs.top, add3(blk, [0.0, 0.2, 0.0]), 6, 0.0, 0.0, 0.0));
    chain(arr, &rig, 2.0, STEEL.map(|v| v * 0.8), 1.0);
    if let Some(end) = cs.end {
        let bot = sub3(blk, [0.0, 0.24, 0.0]);
        let slack = if cs.attached { 0.0 } else { 0.02 };
        chain(arr, &cam.wp(&cable(bot, end, 8, slack, 0.0, 0.0)), 3.2, STEEL, 1.0);
        // the hook, opening away from the chain
        let s0 = cam.wp(&[end])[0];
        let s1 = cam.wp(&[add3(end, unit3(sub3(blk, end)).map(|v| v * 0.2))])[0];
        let d = unit2((s1.0 - s0.0, s1.1 - s0.1));
        let n = (-d.1, d.0);
        let r = 4.6;
        let mut pts = vec![(s0.0 + d.0 * r * 0.8, s0.1 + d.1 * r * 0.8)];
        for j in 0..9 {
            let ai = dmath::PI * (0.2 + 1.4 * f64::from(j) / 8.0);
            let (c, s) = (dmath::cos(ai), dmath::sin(ai));
            pts.push((
                s0.0 + (c * n.0 - s * d.0) * r * 0.6 - d.0 * r * 0.3,
                s0.1 + (c * n.1 - s * d.1) * r * 0.6 - d.1 * r * 0.3,
            ));
        }
        px::ink_polyline(arr, &pts, 2.4, STEEL_HI, 1.0);
    }
    // the hook block: a steel box, a stripe, a light top edge
    let size = 0.3;
    let corner = |dx: f64, dy: f64| cam.wp(&[add3(blk, [dx * size, dy * size, 0.0])])[0];
    let boxed = [
        corner(-1.0, -1.2),
        corner(1.0, -1.2),
        corner(0.8, 1.0),
        corner(-0.8, 1.0),
    ];
    px::ink_poly(arr, &boxed, STEEL, 1.0);
    px::line(arr, corner(-0.86, -0.2), corner(0.86, -0.2), 2.4, col, 1.0);
    px::line(arr, corner(-0.7, 0.85), corner(0.7, 0.85), 1.0, STEEL_HI, 0.9);
    for (q, a) in &cs.pieces {
        chain(arr, &cam.wp(q), 3.2, STEEL, *a);
    }
}

impl BariSaxLook {
    /// The time the ball's and the crane's pose are read at: held where a smash touched for three frames (S1).
    fn pose_time(&self, p: &PictureCtx, t: f64) -> f64 {
        bold::hold_time(p, self.k, &["wreck"], t.min(p.t_ko - 1e-4))
    }

    /// The crane at `t`, with the swing held on contact (after the hit-stop the kit's own clock runs on).
    fn bold_crane(&self, p: &PictureCtx, t: f64) -> Option<Crane> {
        self.crane_state(p, if t >= p.t_hs { t } else { self.pose_time(p, t) })
    }

    /// The SKANK on at `t`: when it began and when it ends.
    fn skank_on(&self, p: &PictureCtx, t: f64) -> Option<(f64, f64)> {
        let (_, e) = bold::since(p, self.k, &["skank"], t.min(p.t_ko), 60.0)?;
        (t < e.f(3)).then(|| (e.t, e.f(3)))
    }

    /// The ball's height on the cable, held on contact; stretched as it is hoisted back; shaking as a growl
    /// builds.
    pub(super) fn bold_state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase) {
        if kk != self.k {
            return;
        }
        let lift = |st: &mut BallState, h: f64| {
            st.lift = h;
            if h != 0.0 {
                st.paint_h = Some(R);
            }
            if h > 0.02 {
                st.air_bias = 0.6;
            }
        };
        match phase {
            Phase::Fight => {
                let tp = self.pose_time(p, t);
                if let Some((q, h)) = self.air_pose(p, tp) {
                    st.p = q;
                    lift(st, h);
                }
                if let Some(sw) = self.cycle_at(tp, 0.0).and_then(|c| c.sw)
                    && sw.tl0 <= tp
                    && tp < sw.ts0
                {
                    // hoisted back: pulled long by the hook
                    let u = (tp - sw.tl0) / (sw.ts0 - sw.tl0).max(1e-6);
                    st.sx = Some(1.0 - 0.1 * u);
                    st.sy = Some(1.0 + 0.14 * u);
                }
                if let Some((u, _)) = bold::until(p, self.k, &["ability:shockwave"], t, BUILD_S) {
                    let shake = dmath::sin(t * 90.0) * 0.05 * u;
                    st.sx = Some(1.0 + 0.06 * u + shake);
                    st.sy = Some(1.0 - 0.06 * u - shake);
                    st.glow = st.glow.max(0.4 * u);
                }
            }
            Phase::Tail if kk == p.win => lift(
                st,
                self.h_ko(p) * (1.0 - bf_picture::config::ss(p.t_hs, p.t_hs + 0.4, t)),
            ),
            _ => {}
        }
    }

    /// The swing's afterimages under the ball: five solid ghosts trailing back along the arc.
    pub(super) fn bold_ball_under(&self, p: &PictureCtx, arr: &mut Img<f32>, cam: &Cam, t: f64) {
        let tp = self.pose_time(p, t);
        let Some(c) = self.cycle_at(tp, 0.0) else {
            return;
        };
        let Some(sw) = c.sw else {
            return;
        };
        if !(sw.tl0 <= tp && tp < sw.ts1 + 0.12) {
            return;
        }
        let col = p.col[self.k];
        let uu = ((tp - sw.ts0) / (sw.ts1 - sw.ts0).max(1e-6)).clamp(0.0, 1.0);
        let spd = if tp >= sw.ts0 { dmath::sin(dmath::PI * uu) } else { 0.6 };
        for j in (1..=5).rev() {
            let Some((q, h, _)) = Self::wreck_pose(Some(c), tp - f64::from(j) * 0.035) else {
                continue;
            };
            let s = cam.wp(&[w_at(q, R + h)])[0];
            let rp = R * bold::px_per_unit(cam, q[0], q[1], R + h);
            let a = (1.0 - f64::from(j) / 6.0) * (0.25 + 0.75 * spd);
            px::disc(arr, s, rp, bold::pale(col), 0.6 * a);
            px::ink_ring(arr, s, rp, 1.4, col, a);
        }
    }

    /// GROWL's shock wave: a jagged orange ring.
    pub(super) fn bold_canvas_wave(&self, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.owner != self.k {
            return;
        }
        if w.rmax <= 0.0 {
            return;
        }
        // (the ring is out in a quarter of a second: it stays strong to the end, and lingers there a moment in
        // `bold_canvas_after`)
        let fade = ((1.0 - w.r / w.rmax) * 3.0).clamp(0.35, 1.0);
        floor::band(ic, c, Family::Noise, w.cx, w.cz, w.r, fade, c.t, ORANGE_LIGHT, 1.3);
        if w.r > 0.3 {
            floor::band(
                ic,
                c,
                Family::Noise,
                w.cx,
                w.cz,
                w.r - 0.28,
                fade * 0.6,
                c.t + 1.0,
                ORANGE_LIGHT,
                0.8,
            );
        }
    }

    /// The GROWL whose ring has just reached its full size: where it was, how big, and how far it has faded
    /// (1 to 0 over [`LINGER_S`]).
    fn growl_linger(&self, p: &PictureCtx, t: f64) -> Option<([f64; 2], f64, f64)> {
        let k = self.k;
        let (age, e) = bold::since(p, k, &["ability:shockwave"], t.min(p.t_ko), 1.0)?;
        let abilities = &p.cfg[k].raw["abilities"];
        let ab = ["unlock1", "unlock2", "low"]
            .iter()
            .map(|slot| &abilities[*slot])
            .find(|a| a["kind"].as_str() == Some("shockwave"))?;
        let (speed, radius) = (ab["speed"].as_f64()?, ab["radius"].as_f64()?);
        // (from a frame before the run's own ring is gone, so the two join without a gap)
        let over = age - radius / speed.max(1e-6) + 0.04;
        (0.0..LINGER_S)
            .contains(&over)
            .then(|| (p.run.p(p.sim_i(e.t), k), radius, (1.15 - over / LINGER_S).min(1.0)))
    }

    pub(super) fn bold_canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let t = c.t;
        if t >= p.t_hs {
            return;
        }
        let tcn = t.min(p.t_ko);
        let tint = bold::light(p.col[self.k]);
        if c.in_fight
            && let Some((at, r, fade)) = self.growl_linger(p, t)
        {
            floor::band(ic, c, Family::Noise, at[0], at[1], r, fade, t, ORANGE_LIGHT, 1.1);
        }
        // the coming swing: a dashed lane under its arc, from the cable's drop to the end of the swing
        if c.in_fight
            && let Some(cy) = self.cycle_at(tcn, 0.0)
            && let Some(sw) = cy.sw
            && tcn < sw.ts1.min(cy.cut)
        {
            let reach = sw.l * dmath::sin(sw.th);
            let far = [sw.bot[0] + sw.u[0] * reach, sw.bot[1] + sw.u[1] * reach];
            let u = ((tcn - cy.t0) / (sw.ts0 - cy.t0).max(1e-6)).clamp(0.0, 1.0);
            floor::dashes(ic, c, sw.ps, far, 0.13, 0.36, t * 4.0, tint, 0.3 + 0.5 * u);
        }
        for q in &self.w.snaps {
            if q.t0 > p.t_ko {
                continue;
            }
            if q.t0 <= tcn && tcn < q.t1 {
                // CABLE SNAP: the spot darkens under the hoisted ball and the target closes in on it
                let u = (tcn - q.t0) / (q.t1 - q.t0).max(1e-6);
                floor::shade(ic, c, q.spot[0], q.spot[1], R * (0.6 + 0.8 * u), 0.3 + 0.5 * u);
                floor::target(ic, c, q.spot[0], q.spot[1], R + q.rad, u, tint);
            } else if q.t1 <= tcn && t < q.t1 + CRATER_S && q.t1 <= p.t_ko {
                // the crash: a dark crater with a lit rim, and the canvas quakes
                let age = t - q.t1;
                let fade = 1.0 - bold::ease_in(age / CRATER_S);
                floor::shade(ic, c, q.spot[0], q.spot[1], R + q.rad, 0.7 * fade);
                floor::ring(ic, c, q.spot[0], q.spot[1], R + q.rad, 0.07, tint, 0.8 * fade);
                let u = age / QUAKE_S;
                if u < 1.0 {
                    floor::band(
                        ic,
                        c,
                        Family::Quake,
                        q.spot[0],
                        q.spot[1],
                        R + q.rad + 2.4 * bold::ease_out(u),
                        1.0 - u,
                        t,
                        tint,
                        1.3,
                    );
                }
            }
        }
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        if payload.downcast_ref::<CraneItem>().is_none() {
            return;
        }
        if let Some(cs) = self.bold_crane(p, f.t) {
            let (mut arr, depth) = (fr.array(), fr.depth());
            draw_bold_crane(&mut arr, f.cam, &cs, p.col[self.k]);
            fr.set_array(&arr, Some(&depth));
        }
    }

    pub(super) fn bold_over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        let k = self.k;
        let col = p.col[k];
        if f.in_fight && self.wrecker {
            // the coming swing: a dotted arc in the air, the dots ahead of the ball
            let tp = self.pose_time(p, f.t);
            if let Some(cy) = self.cycle_at(tp, 0.0)
                && let Some(sw) = cy.sw
                && tp < sw.ts1.min(cy.cut)
            {
                for i in 0..=14 {
                    let uu = f64::from(i) / 14.0;
                    if sw.ts0 + uu * (sw.ts1 - sw.ts0) <= tp + 0.03 {
                        continue;
                    }
                    let phi = -sw.th * dmath::cos(dmath::PI * uu);
                    let s = sw.l * dmath::sin(phi);
                    let q = [sw.bot[0] + sw.u[0] * s, sw.bot[1] + sw.u[1] * s];
                    let at = f.cam.wp(&[w_at(q, R + VERT * sw.l * (1.0 - dmath::cos(phi)))])[0];
                    px::ink_disc(arr, at, 1.7, bold::pale(col), 0.9);
                }
            }
        }
        if self.wrecker
            && !self.crane_first(p, f.t)
            && let Some(cs) = self.bold_crane(p, f.t)
        {
            draw_bold_crane(arr, f.cam, &cs, col);
        }
        if f.in_fight {
            // GROWL's front: a thick toothed ring just above the canvas
            let mut fronts: Vec<([f64; 2], f64, f64)> = p
                .run
                .waves(f.i)
                .iter()
                .filter(|w| w.owner == k && w.rmax > 0.0 && w.r < w.rmax)
                .map(|w| ([w.cx, w.cz], w.r, 1.0))
                .collect();
            fronts.extend(self.growl_linger(p, f.t));
            for (at, r0, fade) in fronts {
                let teeth = 26;
                let world: Vec<V3> = (0..=teeth * 2)
                    .map(|j| {
                        let ai = dmath::TAU * f64::from(j) / f64::from(teeth * 2);
                        let r = r0 + if j % 2 == 1 { 0.16 } else { -0.06 };
                        scene::to_world(at[0] + r * dmath::cos(ai), at[1] + r * dmath::sin(ai), 0.3)
                    })
                    .collect();
                px::ink_polyline(arr, &f.cam.wp(&world), 2.2, ORANGE, fade);
            }
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let (k, t) = (self.k, f.t);
        let o = 1 - k;
        let g = &p.show.grid;
        let col = p.col[k];
        let me = drawn.iter().find(|d| d.k == k).filter(|_| !f.fs[k].shattered);
        if let Some(d) = me {
            // hoisted back: a glint at the top of the pull-back, the swing is coming
            let tp = self.pose_time(p, t);
            if let Some(sw) = self.cycle_at(tp, 0.0).and_then(|c| c.sw)
                && sw.tl0 - 0.05 <= tp
                && tp < sw.ts0 + 0.07
            {
                let u = ((tp - sw.tl0 + 0.05) / (sw.ts0 - sw.tl0 + 0.12)).clamp(0.0, 1.0);
                px::glint(
                    img,
                    (d.s.0 - d.rp * 0.7, d.s.1 - d.rp * 0.75),
                    5.0 + 6.0 * dmath::sin(dmath::PI * u),
                    bold::WHITE,
                    1.0,
                );
            }
            // a growl builds: a toothed ring closes in on the ball
            if let Some((u, _)) = bold::until(p, k, &["ability:shockwave"], t, BUILD_S) {
                let r = d.rp * (2.3 - 1.0 * bold::ease_in(u));
                let turn = t * 9.0;
                let pts: Vec<(f64, f64)> = (0..=28)
                    .map(|j| {
                        let ai = dmath::TAU * f64::from(j) / 28.0 + turn;
                        let rr = r * if j % 2 == 1 { 1.14 } else { 0.94 };
                        (d.s.0 + rr * dmath::cos(ai), d.s.1 + rr * dmath::sin(ai) * 0.9)
                    })
                    .collect();
                px::ink_polyline(img, &pts, 1.4 + 1.4 * u, ORANGE, 0.4 + 0.6 * u);
            }
            // CABLE SNAP: the chain frays with sparks where it will part, then parts with a white burst; the
            // ball falls with speed lines
            for q in self.w.snaps.iter().filter(|q| q.t0 <= t && t < q.t1 && q.t0 <= p.t_ko) {
                let fray = (d.s.0, d.s.1 - d.rp - 13.0);
                if t < q.ts {
                    let u = (t - q.t0) / (q.ts - q.t0).max(1e-6);
                    let strands = 2 + (u * 4.0) as u64;
                    for i in 0..strands {
                        let side = if i % 2 == 0 { 1.0 } else { -1.0 };
                        let len = 4.0 + 6.0 * u + 2.0 * px::rnd(40, i);
                        let up = -3.0 + 2.5 * i as f64;
                        px::ink_line(img, fray, (fray.0 + side * len, fray.1 + up), 1.0, STEEL_HI, 1.0);
                    }
                    // a fresh spray of sparks every tenth of a second, faster as it goes
                    let period = 0.14 - 0.07 * u;
                    let n = ((t - q.t0) / period).floor();
                    let age = t - q.t0 - n * period;
                    px::chips(
                        img,
                        fray,
                        age,
                        period + 0.06,
                        5 + (u * 6.0) as u64,
                        16.0 + 16.0 * u,
                        n as u64 + 400,
                        bold::GOLD,
                    );
                    px::glint(img, fray, 4.0 + 5.0 * u * (1.0 - age / period), bold::WHITE, 0.95);
                } else {
                    let age = t - q.ts;
                    if age < BREAK_S {
                        let u = age / BREAK_S;
                        let i_s = p.sim_i(q.ts);
                        let (ps, hs) = (p.run.p(i_s, k), PicSrc(p).row("SNAP", i_s, k).map_or(0.0, |r| r[1]));
                        let at = bold::at(f.cam, ps[0], ps[1], 2.0 * R + hs + 0.55);
                        px::ink_star(
                            img,
                            at,
                            7.0 + 12.0 * bold::ease_out(u),
                            4.0,
                            7,
                            0.3,
                            bold::WHITE,
                            1.0 - u,
                        );
                        px::chips(img, at, age, BREAK_S + 0.15, 10, 30.0, (q.ts * 1000.0) as u64, STEEL_HI);
                    }
                    for i in 0..5 {
                        let dx = (f64::from(i) - 2.0) / 2.0 * d.rp * 0.8;
                        let len = (18.0 + 30.0 * (age / (q.t1 - q.ts).max(1e-6)).min(1.0))
                            * (1.0 - 0.3 * (f64::from(i) - 2.0).abs() / 2.0);
                        let top = d.s.1 - d.rp * (1.0 - (dx / d.rp) * (dx / d.rp)).max(0.0).sqrt() - 3.0;
                        px::line(img, (d.s.0 + dx, top), (d.s.0 + dx, top - len), 1.4, bold::WHITE, 0.85);
                    }
                }
            }
        }
        // the crash: chips fly from the crater
        for q in self
            .w
            .snaps
            .iter()
            .filter(|q| q.t1 <= t && t < q.t1 + 0.6 && q.t1 <= p.t_ko)
        {
            let at = bold::at(f.cam, q.spot[0], q.spot[1], 0.15);
            px::chips(
                img,
                at,
                t - q.t1,
                0.6,
                18,
                70.0,
                (q.t1 * 1000.0) as u64,
                bold::pale(col),
            );
            px::chips(
                img,
                at,
                t - q.t1,
                0.5,
                12,
                46.0,
                (q.t1 * 1000.0) as u64 + 5,
                [110.0, 84.0, 70.0],
            );
        }
        // an enemy smashed or growled away flies off with speed lines
        if let Some((age, e)) = bold::since(p, k, &["hit:wreck", "hit:shockwave", "hit:crash"], t, FLY_S)
            && e.i(3) == o as i64
            && let Some(d) = drawn.iter().find(|d| d.k == o)
        {
            let (a, b) = (p.run.p(p.sim_i(e.t), o), p.run.p(p.sim_i(e.t + 0.1), o));
            let (s0, s1) = (bold::at(f.cam, a[0], a[1], R), bold::at(f.cam, b[0], b[1], R));
            let u = age / FLY_S;
            // (outlined streaks: the smashing ball is right behind it, thin lines would be lost on it)
            let (dx, dy) = unit2((s1.0 - s0.0, s1.1 - s0.1));
            if dx != 0.0 || dy != 0.0 {
                for i in 0..5 {
                    let v = (f64::from(i) - 2.0) / 2.0;
                    let side = v * d.rp * 0.8;
                    let back = d.rp * (1.0 - v * v * 0.64).sqrt() + 3.0;
                    let from = (d.s.0 - dx * back - dy * side, d.s.1 - dy * back + dx * side);
                    let len = (44.0 - 14.0 * v.abs()) * (1.0 - 0.4 * u);
                    px::ink_line(
                        img,
                        from,
                        (from.0 - dx * len, from.1 - dy * len),
                        1.6,
                        bold::WHITE,
                        (1.0 - u * u).clamp(0.0, 1.0),
                    );
                }
            }
        }
        // SKANK: the bar's eight eighth notes over the ring's far corner, the off-beats big; the cursor runs
        // along them and every smash stamps the pip it lands on
        if let Some((t0, until)) = self.skank_on(p, t) {
            let fade = ((t - t0) / 0.15).min((until - t) / 0.2).clamp(0.0, 1.0);
            let post = bold::at(f.cam, -scene::POST, -scene::POST, scene::POST_H);
            let mid = (post.0, post.1 + 40.0);
            let eighth = g.beat / 2.0;
            let pos = (t - g.t_fight) / eighth;
            let (now, part) = ((pos.floor() as i64).rem_euclid(8), pos - pos.floor());
            let bar0 = g.t_fight + (pos.floor() - now as f64) * eighth;
            let pip = |j: i64| (mid.0 + (j as f64 - 3.5) * 15.0, mid.1);
            px::line(img, pip(0), pip(7), 17.0, bold::INK, 0.6 * fade);
            let stamps: Vec<(i64, f64)> = bold::events_in(p, k, &["smash"], bar0.max(t0), t)
                .map(|e| ((((e.t - g.t_fight) / eighth).round() as i64).rem_euclid(8), t - e.t))
                .collect();
            for j in 0..8 {
                let c = pip(j);
                let off = j % 2 == 1;
                let pop = if j == now { 1.0 - part } else { 0.0 };
                let r = if off { 5.0 } else { 2.8 } + 2.4 * pop;
                let fill = if j == now {
                    bold::WHITE
                } else if off {
                    bold::deep(col)
                } else {
                    [120.0, 120.0, 140.0]
                };
                px::ink_disc(img, c, r, fill, fade);
                if let Some(&(_, age)) = stamps.iter().find(|s| s.0 == j) {
                    let burst = 1.0 - (age / 0.3).min(1.0);
                    px::ink_star(img, c, 7.5 + 6.0 * burst, 3.4 + 1.5 * burst, 5, -0.3, col, fade);
                    px::disc(img, c, 1.8, bold::WHITE, fade);
                }
            }
        }
    }
}
