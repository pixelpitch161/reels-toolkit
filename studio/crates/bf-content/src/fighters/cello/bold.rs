//! CELLO with the bold effects (`picture.effects = "bold"`; the owner's picks 22a to 22d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs. Everything of its own is drawn after the grade, crisp and outlined, over both balls (the arms go round
//! the enemy), with the warnings as light on the canvas.
//!
//! - **The embrace** (22a): on the rush two thick arms (the bow and the neck with its scroll) open wide ahead
//!   of the ball, which trails speed lines; on the grab the bow hooks right round the enemy, which is swung
//!   with afterimages and a wide trail while dashes on the floor point at the rope it will be thrown to; on
//!   the throw the bow flings out straight, the enemy flies with speed lines and the rope slam pops as a heavy
//!   hit.
//! - **VIBRATO** (22b): the held enemy shakes harder inside trembling wavy arcs and is marked as pinned with
//!   the hold's time draining; every 16th ticks as a tap; the arcs fly apart as it is let go.
//! - **PRELUDE** (22c): each 16th a crescent leaves the ball toward the enemy, at a height and size that rise
//!   and fall with the tune; the long ones are gold and reach it, the short ones are pale and fade out on the
//!   way. The rings themselves are faint string rings on the canvas.
//! - **SWAN SONG** (22d): big white wings, feathers dropping all along the glide, a landing mark closing in on
//!   the spot of the dive, and a burst of feathers where it lands.

use bf_core::dmath::{self, PI, TAU};
use bf_core::event::Event;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize, Status, StatusKind};
use bf_picture::camera::{Cam, V3};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Pic};
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::Wave;
use bf_picture::scene::{self, R_BALL as R};

use super::{BODY, CelloLook, EBONY, HAIR, WING, WING_D, arm_wrap, how, seg_pts, unit, wing_poly};

/// Seconds the bow stays flung out after a throw, and the thrown enemy trails speed lines.
const FLING_S: f64 = 0.28;
/// Seconds the wavy arcs take to fly apart when VIBRATO's hold ends.
const LET_GO_S: f64 = 0.22;
/// Seconds a PRELUDE crescent lingers where it ends.
const LINGER_S: f64 = 0.2;
/// Seconds a dropped feather falls, and the feathers of the dive fly.
const FEATHER_S: f64 = 0.8;
const BURST_S: f64 = 0.7;

/// The bow's stick, light enough to read on the dark canvas.
const BOW: V3 = [168.0, 104.0, 60.0];
/// A PRELUDE crescent that reaches.
const AMBER: V3 = [255.0, 200.0, 110.0];

/// A quadratic curve through three ring-frame points `(x, z, height)`, as 9 points.
fn curve(a: V3, b: V3, c: V3) -> Vec<V3> {
    (0..=8)
        .map(|i| {
            let s = f64::from(i) / 8.0;
            std::array::from_fn(|ch| (1.0 - s) * (1.0 - s) * a[ch] + 2.0 * s * (1.0 - s) * b[ch] + s * s * c[ch])
        })
        .collect()
}

/// The bow along ring-frame points: a thick outlined stick, the ivory hair beside it, the frog at its start
/// and an ivory tip.
fn draw_bow(img: &mut Pic, cam: &Cam, pts: &[V3], a: f64) {
    let sp = seg_pts(cam, pts);
    px::ink_polyline(img, &sp, 4.2, BOW, a);
    let hair: Vec<V3> = pts.iter().map(|q| [q[0], q[1], q[2] - 0.09]).collect();
    px::polyline(img, &seg_pts(cam, &hair), 1.8, HAIR, a);
    px::ink_disc(img, sp[0], 3.2, EBONY, a);
    px::ink_disc(img, sp[sp.len() - 1], 2.4, HAIR, a);
}

/// The other arm: the neck in the cello's varnish with its dark scroll at the end.
fn draw_neck(img: &mut Pic, cam: &Cam, pts: &[V3], a: f64) {
    let sp = seg_pts(cam, pts);
    px::ink_polyline(img, &sp, 4.4, BODY, a);
    px::ink_disc(img, sp[sp.len() - 1], 3.6, EBONY, a);
}

/// A feather on the screen at `c`, turned by `rot`: a white leaf with a quill line.
fn feather(img: &mut Pic, c: (f64, f64), rot: f64, len: f64, a: f64) {
    let (ux, uy) = (dmath::cos(rot), dmath::sin(rot));
    let at = |s: f64, w: f64| (c.0 + ux * s * len - uy * w * len, c.1 + uy * s * len + ux * w * len);
    px::ink_poly(
        img,
        &[at(-1.0, 0.0), at(-0.2, 0.42), at(1.0, 0.0), at(-0.2, -0.42)],
        WING,
        a,
    );
    px::line(img, at(-0.8, 0.0), at(0.8, 0.0), 1.0, WING_D, a);
}

/// Where the line from `c` along the angle `a` meets the ropes.
fn rope_end(c: [f64; 2], a: f64) -> [f64; 2] {
    let d = unit(a);
    let mut m = f64::INFINITY;
    for ax in 0..2 {
        if d[ax].abs() > 1e-9 {
            let edge = if d[ax] > 0.0 { scene::A } else { -scene::A };
            m = m.min((edge - c[ax]) / d[ax]);
        }
    }
    let m = if m.is_finite() { m.max(0.0) } else { 0.0 };
    [c[0] + d[0] * m, c[1] + d[1] * m]
}

impl CelloLook {
    /// PRELUDE's ring speed, from whichever of its moves is the prelude.
    fn prelude_speed(p: &PictureCtx, k: usize) -> f64 {
        ["unlock1", "unlock2", "low"]
            .iter()
            .map(|slot| &p.cfg[k].raw["abilities"][*slot])
            .find(|ab| ab["kind"] == "prelude")
            .and_then(|ab| ab["speed"].as_f64())
            .filter(|&s| s > 0.0)
            .unwrap_or(6.0)
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !self.show(p, c.t) {
            return;
        }
        let k = self.k;
        let ta = self.ta(p, c.t);
        let tint = bold::light(p.col[k]);
        // the hold: dashes run from the cello to the rope the enemy will be thrown at
        if let Some((_, e)) = bold::since(p, k, &["hold"], ta, 2.0)
            && ta < e.f(5)
        {
            let u = (ta - e.t) / (e.f(5) - e.t).max(1e-6);
            if u > 0.35 {
                let me = c.fs[k].p;
                floor::dashes(
                    ic,
                    c,
                    me,
                    rope_end(me, e.f(4)),
                    0.2,
                    0.45,
                    c.t * 5.0,
                    tint,
                    0.2 + 0.5 * u,
                );
            }
        }
        // the throw: a lane of light the way the enemy flies
        if let Some((age, e)) = bold::since(p, k, &["throw"], ta, FLING_S) {
            let from = [e.f(5), e.f(6)];
            floor::lane(
                ic,
                c,
                from,
                rope_end(from, e.f(3)),
                0.3,
                tint,
                0.6 * (1.0 - age / FLING_S),
            );
        }
        // SWAN SONG: the landing mark closes in on the spot through the glide and the dive
        let tc = p.fx_clock(c.t);
        let radius = p.cfg[k].raw["abilities"]["low"]["radius"].as_f64().unwrap_or(1.4);
        for e in bold::events_in(p, k, &["swan"], tc - 8.0, tc) {
            let (tg, tl) = (e.f(4), e.f(6));
            if tg <= tc && tc < tl {
                let u = (tc - tg) / (tl - tg).max(1e-6);
                floor::target(ic, c, e.f(11), e.f(12), radius, u, bold::light(WING));
            }
        }
    }

    pub(super) fn bold_canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind != "prelude" || w.owner != self.k {
            return;
        }
        // the ring itself, faint: the crescent on it is the note
        floor::wave(ic, c, Family::Strings, w, c.t, bold::light(p.col[self.k]), 0.45);
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let (cam, k, t) = (f.cam, self.k, f.t);
        if !self.show(p, t) {
            return;
        }
        let o = 1 - k;
        let col = p.col[k];
        let (st, en_st) = (&f.fs[k], &f.fs[o]);
        let (me, en) = (st.p, en_st.p);
        let ta = self.ta(p, t);
        let d_me = drawn.iter().find(|d| d.k == k);
        let d_en = drawn.iter().find(|d| d.k == o);
        let g = self.grab_at(p, t);
        let sw = self.swan_at(p, t);
        if let (Some((state, a, t0, t1, reach)), None, true) = (g, sw, st.vis) {
            if state == 1 {
                // the rush: both arms open wide ahead of it, speed lines behind
                let u = bold::ease_out((ta - t0) / 0.09);
                let l = 0.15 + (0.6 + reach) * u;
                let h = R * 1.1;
                for side in [-1.0, 1.0] {
                    let at = |da: f64, r: f64| {
                        let d = unit(a + side * da);
                        [me[0] + d[0] * r, me[1] + d[1] * r, h]
                    };
                    let pts = curve(at(1.2, R * 0.8), at(1.0, R + l * 0.65), at(0.5, R + l));
                    if side > 0.0 {
                        draw_bow(img, cam, &pts, 1.0);
                    } else {
                        draw_neck(img, cam, &pts, 1.0);
                    }
                }
                if let Some(d) = d_me {
                    let ahead = unit(a);
                    let s1 = bold::at(cam, me[0] + ahead[0], me[1] + ahead[1], R);
                    px::speed_lines(
                        img,
                        d.s,
                        d.rp,
                        (s1.0 - d.s.0, s1.1 - d.s.1),
                        18.0,
                        6,
                        bold::pale(col),
                        0.85,
                    );
                }
            } else if state == 2 {
                let u = (ta - t0) / (t1 - t0).max(1e-6);
                // the swing: afterimages of the enemy and a wide trail where it has been
                if let Some(d) = d_en {
                    let m3 = (u * 3.0).min(1.0);
                    let trail: Vec<(f64, f64)> = (0..=6)
                        .map(|i| {
                            let q = p.fighter(o, (ta - 0.15 + 0.025 * f64::from(i)).max(t0));
                            bold::at(cam, q.p[0], q.p[1], R + q.lift)
                        })
                        .collect();
                    px::polyline(img, &trail, d.rp * 0.9, bold::pale(col), 0.3 * m3);
                    for j in 1..=3u32 {
                        let past = p.fighter(o, (ta - 0.05 * f64::from(j)).max(t0));
                        let s1 = bold::at(cam, past.p[0], past.p[1], R + past.lift);
                        if dmath::hypot(s1.0 - d.s.0, s1.1 - d.s.1) > d.rp * 0.3 {
                            px::ring(img, s1, d.rp, 2.0, bold::pale(col), 0.75 / f64::from(j));
                        }
                    }
                }
                // the embrace: the bow hooks round the enemy's far side, the neck holds its near side
                let h = R * 1.05 + en_st.lift * 0.6;
                draw_bow(img, cam, &arm_wrap(me, en, h), 1.0);
                let phi = dmath::atan2(en[1] - me[1], en[0] - me[0]);
                let (d0, d1) = (unit(phi - 0.9), unit(phi + PI - 0.75));
                draw_neck(
                    img,
                    cam,
                    &curve(
                        [me[0] + d0[0] * R * 0.8, me[1] + d0[1] * R * 0.8, h],
                        [me[0] + d0[0] * R * 1.5, me[1] + d0[1] * R * 1.5, h],
                        [en[0] + d1[0] * R * 1.05, en[1] + d1[1] * R * 1.05, h],
                    ),
                    1.0,
                );
                // VIBRATO: trembling wavy arcs round the held enemy
                if self.vib_on(ta)
                    && let Some(d) = d_en
                {
                    Self::wavy_arcs(img, d.s, d.rp, t, 0.0, col);
                }
            }
        }
        // VIBRATO's hold ends: the arcs fly apart
        if let Some((age, e)) = bold::since(p, k, &["throw", "release"], ta, LET_GO_S)
            && self.vib_on(e.t)
            && bold::since(p, k, &["hold"], e.t, 1.5).is_some()
            && let Some(d) = d_en
        {
            Self::wavy_arcs(img, d.s, d.rp, t, age / LET_GO_S, col);
        }
        // the throw: the bow flung out straight, the enemy flying with speed lines
        if let Some((age, e)) = bold::since(p, k, &["throw"], ta, FLING_S)
            && sw.is_none()
            && g.is_none_or(|g| g.0 != 2)
        {
            let fade = 1.0 - age / FLING_S;
            let d = unit(e.f(3));
            let l = R * 0.9 + 1.0 * bold::ease_out(age / 0.07);
            let h = R * 1.05;
            let pts: Vec<V3> = (0..=4)
                .map(|i| {
                    let r = R * 0.75 + (l - R * 0.75) * f64::from(i) / 4.0;
                    [me[0] + d[0] * r, me[1] + d[1] * r, h]
                })
                .collect();
            draw_bow(img, cam, &pts, (fade * 2.0).min(1.0));
            if let Some(den) = d_en {
                let (a, b) = (p.run.p(p.sim_i(ta - 0.07), o), p.run.p(p.sim_i(ta), o));
                let (sa, sb) = (bold::at(cam, a[0], a[1], R), bold::at(cam, b[0], b[1], R));
                px::speed_lines(
                    img,
                    den.s,
                    den.rp,
                    (sb.0 - sa.0, sb.1 - sa.1),
                    8.0 + 20.0 * fade,
                    6,
                    bold::WHITE,
                    0.9 * fade,
                );
            }
        }
        // a grab that slipped off: a quick scrape of the hair
        for &(t0, how, x, y) in &self.grabs {
            if how == 2 && (0.0..0.18).contains(&(ta - t0)) {
                let kk = 1.0 - (ta - t0) / 0.18;
                let d = [x - me[0], y - me[1]];
                let nd = dmath::hypot(d[0], d[1]) + 1e-9;
                let nn = [-d[1] / nd, d[0] / nd];
                px::ink_line(
                    img,
                    bold::at(cam, x - nn[0] * 0.35, y - nn[1] * 0.35, R),
                    bold::at(cam, x + nn[0] * 0.35, y + nn[1] * 0.35, R),
                    2.0,
                    HAIR,
                    kk,
                );
            }
        }
        self.crescents(p, img, f, ta);
        self.swan(p, img, f, d_me);
    }

    /// VIBRATO's wavy arcs round the screen point `c`: four trembling pieces of a ring and four fainter ones
    /// outside them; `gone` from 0 to 1 flies them apart.
    fn wavy_arcs(img: &mut Pic, c: (f64, f64), rp: f64, t: f64, gone: f64, col: V3) {
        let a = 1.0 - gone;
        for (ring, w, alpha) in [(1.38, 1.8, a), (1.72, 1.2, 0.6 * a)] {
            for q in 0..4 {
                let mid = f64::from(q) * PI / 2.0 + PI / 4.0;
                let pts: Vec<(f64, f64)> = (0..=10)
                    .map(|i| {
                        let th = mid + (f64::from(i) / 10.0 - 0.5) * 1.1;
                        let r = rp * (ring + 0.9 * gone) + 2.6 * dmath::sin(th * 9.0 + t * 55.0 + ring * 4.0);
                        (c.0 + r * dmath::cos(th), c.1 + r * 0.9 * dmath::sin(th))
                    })
                    .collect();
                if ring < 1.5 {
                    px::ink_polyline(img, &pts, w, bold::pale(col), alpha);
                } else {
                    px::polyline(img, &pts, w, bold::WHITE, alpha);
                }
            }
        }
    }

    /// PRELUDE: one crescent a note-wave, on its ring on the side of the enemy. The reach of the wave is its
    /// pitch: a long one flies higher and bigger, in gold; a short one low and pale, fading where it ends.
    fn crescents(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, ta: f64) {
        if !f.in_fight {
            return;
        }
        let cam = f.cam;
        let en = f.fs[1 - self.k].p;
        let speed = Self::prelude_speed(p, self.k);
        let j0 = self.notes.partition_point(|n| n.t < ta - 1.0);
        let j1 = self.notes.partition_point(|n| n.t <= ta);
        for nt in &self.notes[j0..j1] {
            let tau = ta - nt.t;
            let fly = (nt.rmax - R).max(0.0) / speed;
            if tau > fly + LINGER_S {
                continue;
            }
            let r = R + speed * tau.min(fly);
            let rank = ((nt.rmax - R - 0.55) / 1.15).clamp(0.0, 1.0);
            let alpha = if tau <= fly { 1.0 } else { 1.0 - (tau - fly) / LINGER_S };
            let aim = dmath::atan2(en[1] - nt.y, en[0] - nt.x);
            // small: about as long as the ball is wide, whatever its ring's size; bowed toward the enemy
            // with its horns trailing
            let half = 0.34 + 0.2 * rank;
            let bulge = 0.3 + 0.1 * rank;
            let y = 0.15 + 0.6 * rank * dmath::sin(PI * (tau / (fly + LINGER_S)).min(1.0)).max(0.3);
            let (u, n) = (unit(aim), unit(aim + PI / 2.0));
            let at = |s: f64, bow: f64| {
                let (fwd, lat) = (r - bulge + bow * (1.0 - s * s), half * s);
                bold::at(cam, nt.x + u[0] * fwd + n[0] * lat, nt.y + u[1] * fwd + n[1] * lat, y)
            };
            let steps = 8;
            let side = |i: i32| f64::from(i) / f64::from(steps) * 2.0 - 1.0;
            let mut pts: Vec<(f64, f64)> = (0..=steps).map(|i| at(side(i), bulge)).collect();
            pts.extend((1..steps).rev().map(|i| at(side(i), bulge * 0.35)));
            let col = if rank > 0.4 { AMBER } else { bold::pale(p.col[self.k]) };
            px::ink_poly(img, &pts, col, alpha);
            if rank > 0.4 {
                px::line(
                    img,
                    at(-0.5, bulge * 0.8),
                    at(0.5, bulge * 0.8),
                    1.0,
                    bold::WHITE,
                    alpha,
                );
            }
        }
    }

    /// SWAN SONG: the wings on the lifted ball, the feathers it drops and the burst of feathers of its dive.
    fn swan(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, d_me: Option<&DrawnBall>) {
        let (cam, k, t) = (f.cam, self.k, f.t);
        let tc = p.fx_clock(t);
        // feathers dropped along the flight: each falls from where the ball was, swaying
        for sw in &self.swans {
            if !(sw.t0..sw.tl + FEATHER_S).contains(&tc) {
                continue;
            }
            let n = ((sw.td - sw.t0) / 0.085).ceil().max(0.0) as u64;
            for i in 0..n {
                let ts = sw.t0 + 0.2 + i as f64 * 0.085;
                let age = tc - ts;
                if ts >= sw.td || !(0.0..FEATHER_S).contains(&age) {
                    continue;
                }
                let from = p.run.p(p.sim_i(ts), k);
                let h0 = self.swan_at(p, ts).map_or(sw.h, |s| s.1) + R * 0.6;
                let u = age / FEATHER_S;
                let sway = 0.16 * dmath::sin(age * 7.0 + i as f64 * 1.7);
                let c = bold::at(cam, from[0] + sway, from[1] - sway * 0.5, h0 * (1.0 - u) + 0.08);
                feather(
                    img,
                    c,
                    0.9 * dmath::sin(age * 6.0 + i as f64),
                    5.5,
                    ((1.0 - u) / 0.3).min(1.0),
                );
            }
        }
        // the wings
        let st = &f.fs[k];
        if let (Some((swd, _, ph, u)), true, Some(d)) = (self.swan_at(p, t), st.vis, d_me) {
            let rate = if ph == 0 { 2.2 } else { 1.3 };
            let mut flap = 0.5 + 0.5 * dmath::sin(TAU * (t - swd.t0) * rate);
            let fold = if ph < 2 { 0.0 } else { (u * 1.6).min(1.0) };
            if ph == 0 {
                flap = 0.3 + 0.7 * flap;
            }
            let (cx, cy) = d.s;
            if ph == 2 {
                // the dive: streaks above it
                for dx in [-0.55, 0.0, 0.55] {
                    let x = cx + dx * d.rp;
                    px::line(
                        img,
                        (x, cy - d.rp * 1.1),
                        (x, cy - d.rp * (2.2 + 1.2 * u)),
                        2.0,
                        WING,
                        0.8 * (1.0 - 0.5 * u),
                    );
                }
            }
            for side in [-1.0, 1.0] {
                // a big outer wing and a shorter row of feathers under it
                for (span, drop, tone) in [(2.7, 0.0, WING), (1.7, 0.3, WING_D)] {
                    let wp = wing_poly(cx, cy + d.rp * drop, side, d.rp * span, flap, fold);
                    px::ink_poly(img, &wp, tone, 1.0);
                    if drop == 0.0 {
                        for &(x, y) in &wp[1..4] {
                            px::line(img, (cx + side * 3.0, cy - 1.0), (x, y), 1.0, WING_D, 0.9);
                        }
                    }
                }
            }
        }
        // the dive lands: a burst of feathers
        for &(t0, x, y, _hit) in &self.lands {
            let age = tc - t0;
            if !(0.0..BURST_S).contains(&age) {
                continue;
            }
            let u = age / BURST_S;
            let seed = (t0 * 1000.0) as u64;
            for i in 0..18u64 {
                let ang = TAU * (i as f64 + px::rnd(seed, i)) / 18.0;
                let far = (0.5 + 1.7 * px::rnd(seed, i + 40)) * bold::ease_out(u);
                let up = (0.5 + 0.9 * px::rnd(seed, i + 80)) * dmath::sin(PI * u);
                let c = bold::at(cam, x + far * dmath::cos(ang), y + far * dmath::sin(ang), 0.1 + up);
                feather(img, c, ang * 3.0 + age * 9.0, 6.0, ((1.0 - u) / 0.35).min(1.0));
            }
        }
    }

    /// The enemy held through VIBRATO is pinned, with the hold's time draining.
    pub(super) fn bold_statuses(&self, p: &PictureCtx, t: f64) -> Vec<Status> {
        let ta = self.ta(p, t);
        match self.grab_at(p, t) {
            Some((2, _, t0, t1, _)) if self.vib_on(ta) => vec![Status {
                who: 1 - self.k,
                kind: StatusKind::Pin,
                left: Some((1.0 - (ta - t0) / (t1 - t0).max(1e-6)).clamp(0.0, 1.0)),
            }],
            _ => Vec::new(),
        }
    }

    /// The ticks of VIBRATO and PRELUDE and the grab are taps, the throw a solid hit and the rope slam a heavy
    /// one, whatever their damage.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match how(e) {
            "vibrato" | "prelude" | "grab" => Some(HitSize::Tap),
            "throw" => Some(by_damage.max(HitSize::Solid)),
            "ropeslam" => Some(HitSize::Heavy),
            _ => Some(by_damage),
        }
    }
}
