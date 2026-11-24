//! TROMBONE with the bold effects (`picture.effects = "bold"`; the owner's picks 12a to 12d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **SLIDE LANCE** (12a): the slide is two chunky brass tubes joined by a U-bend. It pulls back a moment
//!   before each thrust while dashes on the floor show the aim, thrusts with a streak, and stops on the enemy
//!   (held where it touched, never drawn through the ball). A projectile it parries is knocked away tumbling,
//!   with a spark where they met.
//! - **SAD TROMBONE** (12b): a droopy wobble round the ball just before; three rings roll out, the later ones
//!   sagging further behind, and turn from mint to blue; while the enemy is slowed (the ring's own blue tint
//!   and down arrows) blue wobbles drag round it and drips fall from it.
//! - **POWER SLIDE** (12c): little slides flicker round the ball as it starts, and the faster slide leaves
//!   afterimages (the ring's own speed-up look does the rest).
//! - **PEDAL TONE** (12d): a landing mark closes in on the spot under the hovering giant bell (its lip drawn
//!   bright), the bell stomps with a burst round its rim, and a quake of lifted tiles rolls out.

use bf_core::dmath;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self};
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::scene::{self, R_BALL as R};

use super::{BRASS, BRASS_DK, BRASS_HI, TromboneLook, bayer, bell_height, draw_giant_bell, slams, unit};

/// The blue the sad rings turn and the slowed enemy drips.
const BLUE: V3 = [96.0, 150.0, 255.0];
/// Each tube's distance from the slide's middle line (wider than the classic slide's).
const GAP: f64 = 0.13;
/// Seconds before a thrust the slide pulls back and the aim shows.
const WIND_S: f64 = 0.13;
/// Seconds before SAD TROMBONE the droopy wobble warns.
const DROOP_S: f64 = 0.3;
/// Seconds the little slides flicker as POWER SLIDE starts.
const STREAKS_S: f64 = 0.7;
/// Seconds a parried projectile tumbles away.
const PARRY_S: f64 = 0.4;
/// Seconds PEDAL TONE's quake rolls.
const QUAKE_S: f64 = 0.7;

/// The slide's lines in the world: the two inner tubes, the outer slide (a U), and the braces across.
struct Slide {
    tubes: [[V3; 2]; 2],
    outer: Vec<V3>,
    braces: Vec<[V3; 2]>,
}

/// The bold slide from the ball at `p` along `th`, `ext` beyond the glass.
fn slide(p: [f64; 2], th: f64, ext: f64) -> Slide {
    let d = unit(th);
    let n = [-d[1], d[0]];
    let (r0, r1) = (R * 0.8, (R + ext).max(R * 0.8 + 0.1));
    let wp = |r: f64, w: f64| scene::to_world(p[0] + d[0] * r + n[0] * w, p[1] + d[1] * r + n[1] * w, R);
    let r_out = (r0 + 0.08).max(r1 - (0.62 * (r1 - r0)).max(0.35));
    let mut outer = vec![wp(r_out, -GAP), wp(r1, -GAP)];
    for j in 1..6 {
        let a = -dmath::PI / 2.0 + dmath::PI * f64::from(j) / 6.0;
        outer.push(wp(r1 + GAP * dmath::cos(a), GAP * dmath::sin(a)));
    }
    outer.push(wp(r1, GAP));
    outer.push(wp(r_out, GAP));
    let mut braces = vec![[wp(r_out + 0.07, -GAP), wp(r_out + 0.07, GAP)]];
    if r0 + 0.12 < r_out - 0.1 {
        braces.push([wp(r0 + 0.12, -GAP), wp(r0 + 0.12, GAP)]);
    }
    Slide {
        tubes: [
            [wp(r0, -GAP), wp(r_out + 0.04, -GAP)],
            [wp(r0, GAP), wp(r_out + 0.04, GAP)],
        ],
        outer,
        braces,
    }
}

/// A steady 0 to 1 from a whole number.
fn scatter(n: i64, salt: f64) -> f64 {
    let v = dmath::sin(n as f64 * 12.9898 + salt * 78.233) * 43_758.545_3;
    v - v.floor()
}

impl TromboneLook {
    /// The seconds from this side's latest `ability` event of kind `kind` (its element 4) to `t`, looking from
    /// `after` seconds before `t` to `before` seconds after it (negative while it is still to come).
    fn ability_near(&self, p: &PictureCtx, kind: &str, t: f64, before: f64, after: f64) -> Option<f64> {
        let i0 = p.ev_t.partition_point(|&te| te < t - after);
        let i1 = p.ev_t.partition_point(|&te| te <= t + before);
        p.rec.events[i0..i1.max(i0)]
            .iter()
            .rev()
            .find(|e| e.kind == "ability" && e.who() == Some(self.k) && e.s(4) == kind)
            .map(|e| t - e.t)
    }

    /// The slide's angle and length as the bold look draws them at `t`: held where it touched for a moment
    /// after a hit (S1), pulled back before a thrust (S3), and no further into the enemy's ball than the closed
    /// slide reaches.
    fn bold_pose(&self, p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> (f64, f64) {
        let k = self.k;
        if !f.in_fight {
            return (st.th, st.ext);
        }
        let t_held = bold::hold_time(p, k, &["blade"], f.t);
        let (th, mut ext) = if t_held < f.t {
            let held = p.fighter(k, t_held);
            (held.th, held.ext)
        } else {
            (st.th, st.ext)
        };
        if ext <= p.rest[k] + 0.03
            && let Some((u, _)) = bold::until(p, k, &["thrust"], f.t, WIND_S)
        {
            ext -= 0.24 * bold::ease_out(u);
        }
        // it stops on the enemy
        let them = &f.fs[1 - k];
        if them.vis && !them.shattered {
            let d = unit(th);
            let rel = [them.p[0] - st.p[0], them.p[1] - st.p[1]];
            let along = rel[0] * d[0] + rel[1] * d[1];
            let across2 = rel[0] * rel[0] + rel[1] * rel[1] - along * along;
            let rr = R * R * them.scale * them.scale;
            if along > 0.0 && across2 < rr {
                let touch = along - (rr - across2).sqrt();
                ext = ext.min((touch - R - GAP - 0.04).max(p.rest[k]));
            }
        }
        (th, ext)
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let k = self.k;
        let tint = bold::light(p.col[k]);
        let blue = bold::light(BLUE);
        // the slowed enemy drags blue wobbles
        let them = &c.fs[k ^ 1];
        if them.slow > 0.0 {
            let sk = (them.slow / 0.25).min(1.0);
            for (n, r) in [R * 1.3, R * 1.75].into_iter().enumerate() {
                floor::band(
                    ic,
                    c,
                    Family::Strings,
                    them.p[0],
                    them.p[1],
                    r,
                    0.5,
                    c.t * 0.3 + n as f64,
                    blue,
                    (1.4 - 0.4 * n as f64) * sk,
                );
            }
        }
        let me = &c.fs[k];
        // SAD TROMBONE is coming: a droopy wobble closes in on the ball
        if let Some(dt) = self.ability_near(p, "shockwave", c.t, DROOP_S, 0.0)
            && dt < 0.0
        {
            let u = 1.0 + dt / DROOP_S;
            let mix: V3 = std::array::from_fn(|ch| tint[ch] + (blue[ch] - tint[ch]) * 0.5);
            for (n, r) in [R * 2.3 - R * 0.9 * u, R * 2.9 - R * 1.1 * u].into_iter().enumerate() {
                floor::band(
                    ic,
                    c,
                    Family::Strings,
                    me.p[0],
                    me.p[1],
                    r,
                    0.9,
                    c.t * 0.25 + n as f64,
                    mix,
                    0.8 + 0.8 * u,
                );
            }
        }
        // the lance's line: dashes show the aim just before a thrust, and the lane is lit as it goes
        if let Some((u, e)) = bold::until(p, k, &["thrust"], c.t, WIND_S) {
            let i = p.sim_i(e.t);
            let (a, b) = (p.run.p(i, k), p.run.p(i, k ^ 1));
            let aim = dmath::atan2(b[1] - a[1], b[0] - a[0]);
            let d = unit(aim);
            let reach = R + e.f(3) + GAP;
            floor::dashes(
                ic,
                c,
                [me.p[0] + d[0] * R, me.p[1] + d[1] * R],
                [me.p[0] + d[0] * reach, me.p[1] + d[1] * reach],
                0.09,
                0.3,
                c.t * 8.0,
                tint,
                0.5 + 0.9 * u,
            );
        }
        if me.thr_k > 0.05 {
            let d = unit(me.th);
            let reach = R + me.ext + GAP;
            floor::lane(
                ic,
                c,
                [me.p[0] + d[0] * R, me.p[1] + d[1] * R],
                [me.p[0] + d[0] * reach, me.p[1] + d[1] * reach],
                0.3,
                tint,
                0.55 * me.thr_k,
            );
        }
    }

    /// SAD TROMBONE: three rings, the later ones sagging further behind, turning blue as they go.
    pub(super) fn bold_canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind != "shockwave" {
            return;
        }
        let u = (w.r / w.rmax).clamp(0.0, 1.0);
        let fade = ((1.0 - u) * 2.2).min(1.0);
        let (mint, blue) = (bold::light(p.col[w.owner]), bold::light(BLUE));
        let turn = bold::ease_out(u * 1.6);
        let tint: V3 = std::array::from_fn(|ch| mint[ch] + (blue[ch] - mint[ch]) * turn);
        for (j, light) in ic.iter_mut().enumerate() {
            let (dx, dz) = (c.sxp[j] - w.cx, c.szp[j] - w.cz);
            let d = dmath::hypot(dx, dz);
            if (d - w.r).abs() > 1.6 {
                continue;
            }
            let ang = dmath::atan2(dz, dx);
            let mut v: f64 = 0.0;
            for (n, gain) in [(0.0, 1.0), (1.0, 0.8), (2.0, 0.6)] {
                // a later ring lags, and droops in slow lobes
                let sag = n * (0.2 + 0.3 * u) + n * 0.1 * u * (1.0 + dmath::sin(ang * 3.0 + n * 1.9 + c.t * 5.0));
                v = v.max((1.0 - (d - (w.r - sag)).abs() / 0.13) * gain);
            }
            if v > 0.0 {
                let step = (v * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
                for ch in 0..3 {
                    light[ch] += step * fade * 2.4 * tint[ch];
                }
            }
        }
    }

    /// PEDAL TONE: the landing mark closing in under the bell, then the quake.
    pub(super) fn bold_canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let tint = bold::light(p.col[self.k]);
        for s in slams(p, c.i) {
            if s.owner != self.k as i64 {
                continue;
            }
            if c.t < s.t_slam {
                let u = ((c.t - s.t0) / (s.t_slam - s.t0).max(1e-6)).clamp(0.0, 1.0);
                floor::shade(ic, c, s.cx, s.cz, s.rad, 0.2 + 0.4 * u);
                floor::target(ic, c, s.cx, s.cz, s.rad, u, tint);
            } else {
                let u = (c.t - s.t_slam) / QUAKE_S;
                if u < 1.0 {
                    let white = [1.2, 1.2, 1.3];
                    floor::band(
                        ic,
                        c,
                        Family::Quake,
                        s.cx,
                        s.cz,
                        s.rad + 2.2 * u,
                        1.0 - u,
                        c.t,
                        white,
                        1.3,
                    );
                    floor::band(
                        ic,
                        c,
                        Family::Quake,
                        s.cx,
                        s.cz,
                        s.rad + 2.2 * u - 0.6,
                        1.0 - u,
                        c.t,
                        tint,
                        0.9,
                    );
                }
            }
        }
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame) {
        let k = self.k;
        let st = &f.fs[k];
        let cam = f.cam;
        let (th, ext) = self.bold_pose(p, f, st);
        let dt = 1.0 / 30.0;
        let prev = p.fighter(k, (f.t - dt).max(0.0));
        // POWER SLIDE: afterimages of where the slide just was
        if f.in_fight && st.haste > 0.0 {
            let pale = bold::pale(p.col[k]);
            for back in [2.0, 1.0] {
                let was = p.fighter(k, (f.t - back * dt).max(0.0));
                if (was.ext - st.ext).abs() > 0.06 || bf_core::py::wrap_pi(was.th - st.th).abs() > 0.08 {
                    fr.line(cam, &slide(st.p, was.th, was.ext).outer, pale, 1);
                }
            }
        }
        // a thrust going out: streaks beside the tubes
        if f.in_fight && st.ext - prev.ext > 0.12 && bf_core::py::wrap_pi(st.th - prev.th).abs() < 0.3 {
            let d = unit(th);
            let n = [-d[1], d[0]];
            let (a, b) = (R + prev.ext, R + ext);
            for side in [-1.0, 1.0] {
                let w = side * (GAP + 0.12);
                let line = [
                    scene::to_world(st.p[0] + d[0] * a + n[0] * w, st.p[1] + d[1] * a + n[1] * w, R),
                    scene::to_world(st.p[0] + d[0] * b + n[0] * w, st.p[1] + d[1] * b + n[1] * w, R),
                ];
                fr.line(cam, &line, bold::pale(p.col[k]), 1);
            }
        }
        let parts = slide(st.p, th, ext);
        let glow = if st.haste > 0.0 { 0.6 } else { 0.0_f64 }.max(0.8 * st.thr_k);
        // the outer slide and its U-bend take the fighter's colour as it strikes
        let hot: V3 = std::array::from_fn(|ch| {
            (BRASS[ch] * (1.0 - 0.45 * glow) + p.col[k][ch] * 0.45 * glow + 30.0 * glow).min(255.0)
        });
        for tube in &parts.tubes {
            bold::thick_line(fr, cam, tube, BRASS_HI.map(|v| v * 0.9), 3);
        }
        for brace in &parts.braces {
            bold::thick_line(fr, cam, brace, BRASS_DK.map(|v| v * 1.2), 3);
        }
        bold::thick_line(fr, cam, &parts.outer, hot, 5);
    }

    /// PEDAL TONE's giant bell as ever, with a bright lip and a burst round its rim as it lands.
    pub(super) fn bold_over_ring(
        &self,
        p: &PictureCtx,
        f: &FrameCtx<'_>,
        fr: &mut Frame,
        arr: &mut Img<f32>,
        depth: &mut Img<f32>,
    ) {
        if !f.in_fight {
            return;
        }
        for s in slams(p, f.i) {
            if s.owner != self.k as i64 {
                continue;
            }
            let Some(h) = bell_height(f.t, s.t0, s.t_slam) else {
                continue;
            };
            let col = p.col[self.k];
            fr.set_array(&*arr, Some(&*depth));
            draw_giant_bell(fr, f.cam, f.l, s.cx, s.cz, s.rad, h, col);
            let lip = f.cam.wp(&[scene::to_world(s.cx, s.cz, h + 0.15)])[0];
            fr.glow(lip.0, lip.1, 34.0, col.map(|c| c * 0.8 + 40.0), 0.32);
            *arr = fr.array();
            *depth = fr.depth();
            // the lip, bright and outlined
            let rim = rim_points(f.cam, s.cx, s.cz, s.rad * 1.05, h);
            px::ink_polyline(arr, &rim, 2.0, BRASS_HI, 1.0);
            // the stomp: a burst round the rim
            let age = f.t - s.t_slam;
            if (0.0..0.16).contains(&age) {
                let u = age / 0.16;
                for j in 0..12 {
                    let a = dmath::TAU * f64::from(j) / 12.0 + 0.2;
                    let (r_in, r_out) = (s.rad * (1.12 + 0.5 * u), s.rad * (1.3 + 0.9 * u));
                    let at = |r: f64| bold::at(f.cam, s.cx + r * dmath::cos(a), s.cz + r * dmath::sin(a), 0.08);
                    px::ink_line(arr, at(r_in), at(r_out), 2.5, bold::WHITE, 1.0 - u);
                }
            }
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let (k, o, t, cam) = (self.k, 1 - self.k, f.t, f.cam);
        // a parry: a spark where the slide met the projectile, which tumbles away
        let i0 = p.ev_t.partition_point(|&te| te < t - PARRY_S);
        let i1 = p.ev_t.partition_point(|&te| te <= t);
        for e in p.rec.events[i0..i1.max(i0)]
            .iter()
            .filter(|e| e.kind == "parry" && e.i(3) == k as i64)
        {
            let age = t - e.t;
            let (x, z) = (e.f(4), e.f(5));
            let u = age / PARRY_S;
            if age < 0.14 {
                let s = bold::at(cam, x, z, R);
                px::ink_star(img, s, 5.0 + 60.0 * age, 2.5, 8, e.t * 7.0, bold::WHITE, 1.0);
            }
            // knocked off sideways from the slide, up and over
            let from = p.run.p(p.sim_i(e.t), k);
            let (dx, dz) = (x - from[0], z - from[1]);
            let len = dmath::hypot(dx, dz).max(1e-6);
            let side = if scatter(e.i(6), 1.0) < 0.5 { -1.0 } else { 1.0 };
            let (vx, vz) = (
                (dx / len * 0.5 - dz / len * side) * 3.0,
                (dz / len * 0.5 + dx / len * side) * 3.0,
            );
            let c = bold::at(cam, x + vx * age, z + vz * age, R + 2.4 * age - 5.0 * age * age);
            let spin = age * 26.0 * side;
            let (hx, hy) = (5.0 * dmath::cos(spin), 5.0 * dmath::sin(spin));
            px::ink_line(
                img,
                (c.0 - hx, c.1 - hy),
                (c.0 + hx, c.1 + hy),
                2.2,
                bold::pale(p.col[o]),
                1.0 - u * u,
            );
            px::ink_disc(img, (c.0 + hx, c.1 + hy), 2.0, p.col[o], 1.0 - u * u);
        }
        // SAD TROMBONE: drips fall from the slowed enemy
        let them = &f.fs[o];
        if them.slow > 0.0
            && them.vis
            && !them.shattered
            && let Some(d) = drawn.iter().find(|d| d.k == o)
        {
            let sk = (them.slow / 0.25).min(1.0);
            for j in 0..4_i64 {
                let phase = t * 1.9 + j as f64 * 0.27;
                let (cycle, fall) = (phase.floor() as i64, phase.fract());
                let x = d.s.0 + d.rp * (-0.75 + 0.5 * j as f64 + 0.2 * (scatter(cycle * 7 + j, 2.0) - 0.5));
                let y = d.s.1 + d.rp * (0.75 + 1.1 * fall * fall);
                let a = sk * (1.0 - fall * fall);
                px::ink_line(img, (x, y - 3.5), (x, y), 1.6, BLUE, a);
                px::ink_disc(img, (x, y), 1.8, bold::pale(BLUE), a);
            }
        }
        // POWER SLIDE starts: little slides flicker round the ball
        if let Some(age) = self.ability_near(p, "haste", t, 0.0, STREAKS_S)
            && let Some(me) = drawn.iter().find(|d| d.k == k)
        {
            let fade = 1.0 - bold::ease_in(age / STREAKS_S);
            let flick = if f.f.is_multiple_of(2) { 1.0 } else { 0.7 };
            let r = me.rp * (1.4 + 0.35 * bold::ease_out(age / 0.15));
            for j in 0..3 {
                let a = dmath::TAU * f64::from(j) / 3.0 + age * 8.0;
                let (ca, sa) = (dmath::cos(a), dmath::sin(a));
                let c = (me.s.0 + r * ca, me.s.1 + r * sa);
                // a little U lying along the circle, its bend leading
                let (tx, ty) = (-sa, ca);
                let (len, gap) = (me.rp * 0.55, me.rp * 0.14);
                let at = |along: f64, out: f64| (c.0 + tx * along + ca * out, c.1 + ty * along + sa * out);
                let pts = [
                    at(-len, gap),
                    at(len * 0.6, gap),
                    at(len, 0.0),
                    at(len * 0.6, -gap),
                    at(-len, -gap),
                ];
                px::ink_polyline(img, &pts, 2.2, BRASS_HI, fade * flick);
            }
        }
    }
}

/// The bell's rim at height `h`, as a closed line on the screen.
fn rim_points(cam: &Cam, cx: f64, cz: f64, r: f64, h: f64) -> Vec<(f64, f64)> {
    let world: Vec<V3> = (0..=18)
        .map(|j| {
            let a = dmath::TAU * f64::from(j) / 18.0;
            scene::to_world(cx + r * dmath::cos(a), cz + r * dmath::sin(a), h)
        })
        .collect();
    cam.wp(&world)
}
