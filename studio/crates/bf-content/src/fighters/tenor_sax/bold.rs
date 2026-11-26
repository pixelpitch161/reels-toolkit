//! TENOR SAX with the bold effects (`picture.effects = "bold"`; the owner's picks 16a to 16d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **Crowd pleaser** (16a): the crowd bounces with its arms up while a volley is on; the ball stretches into its
//!   pose with a glint, thick honk lines and a note, under a hype meter of five outlined lights; a dashed circle
//!   shows the safe middle; the tomatoes, roses and foam fingers are big outlined things that lob in over landing
//!   marks, and one that hits splats on the enemy's ball and drips.
//! - **WALK THE BAR** (16b): the lane is dashed in before the strut; the runway is a bright lane with lit edges,
//!   big chasing bulbs and a spotlight on the ball; the ball struts with a bounce; every honk is a ring of puffs
//!   on the canvas with an arc travelling out ahead of it and behind it.
//! - **SUBTONE** (16c): the lights go down round the ball, the crowd stops bouncing, breath puffs rise from the
//!   bell and from the stands; on the throw the lights snap back and the doubled volley comes in.
//! - **STAGE DIVE** (16d): the leap and the cannonball trail speed lines; the surf rides a row of raised, waving
//!   hands; the landing spot is marked before the crowd lets go, with dashes along the flight.

use bf_core::dmath::{self, PI, TAU};
use bf_core::event::Event;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::BAYER;
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Phase, Pic};
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::scene::{self, R_BALL as R};

use super::{CATWALK, ITEM_COLS, LEAF, ROSE, Runway, TOMATO, TenorSaxLook, Throw, lerp3, stand_point};

/// Seconds the lane is dashed in before WALK THE BAR.
const LANE_S: f64 = 0.3;
/// Seconds a splat stays on the enemy's ball.
const SPLAT_S: f64 = 0.7;
/// Seconds the lights take to go down in SUBTONE.
const DIM_S: f64 = 0.18;
/// Seconds the landing spot of the cannonball is marked before the crowd lets go.
const MARK_S: f64 = 0.4;
/// The bulbs of the runway.
const BULB: V3 = [1.0, 0.8, 0.45];
/// A breath puff.
const BREATH: V3 = [226.0, 236.0, 250.0];
/// The crowd's hands.
const SKIN: [V3; 4] = [
    [236.0, 188.0, 150.0],
    [176.0, 120.0, 86.0],
    [250.0, 216.0, 174.0],
    [126.0, 88.0, 64.0],
];

/// The front row's shirts.
const SHIRTS: [V3; 5] = [
    [232.0, 108.0, 70.0],
    [92.0, 168.0, 222.0],
    [240.0, 200.0, 84.0],
    [150.0, 208.0, 122.0],
    [218.0, 122.0, 202.0],
];

/// The front row of the crowd along the two far ropes: `up` is how far the people are up (0 down behind the
/// apron, 1 standing), `bounce` how hard they bounce (arms go up above a half).
fn front_row(arr: &mut Img<f32>, cam: &Cam, t: f64, beat: f64, up: f64, bounce: f64) {
    if up <= 0.05 {
        return;
    }
    let out = scene::A + 0.6;
    let sides: [([f64; 2], [f64; 2]); 4] = [
        ([out, -1.0], [0.0, 1.0]),
        ([-out, -1.0], [0.0, 1.0]),
        ([-1.0, out], [1.0, 0.0]),
        ([-1.0, -out], [1.0, 0.0]),
    ];
    // the two sides farthest from the camera
    let mut order: Vec<(f64, usize)> = sides
        .iter()
        .enumerate()
        .map(|(i, (o, d))| {
            let mid = scene::to_world(o[0] + d[0], o[1] + d[1], 0.0);
            (cam.depth(&[mid])[0], i)
        })
        .collect();
    order.sort_by(|a, b| b.0.total_cmp(&a.0));
    for &(_, i) in order.iter().take(2) {
        let (o, d) = sides[i];
        for j in 0..8u64 {
            let along = -1.7 + 5.4 * j as f64 / 7.0;
            let seed = i as u64 * 16 + j;
            let base = bold::at(cam, o[0] + d[0] * along, o[1] + d[1] * along, 0.1);
            let hop = bounce * dmath::sin(PI * t / (beat / 2.0) + TAU * px::rnd(seed, 1)).abs();
            let size = 0.55 + 0.45 * up;
            let y = base.1 - 9.0 * up - 4.5 * hop;
            let shirt = SHIRTS[(seed % 5) as usize];
            let skin = SKIN[(seed % 4) as usize];
            if bounce > 0.5 && j % 2 == i as u64 % 2 {
                for side in [-1.0, 1.0] {
                    px::ink_line(
                        arr,
                        (base.0 + side * 3.0, y),
                        (base.0 + side * 5.0, y - 6.0 - 2.0 * hop),
                        1.6,
                        skin,
                        1.0,
                    );
                }
            }
            px::ink_disc(arr, (base.0, y + 2.0), 4.2 * size, shirt, 1.0);
            px::ink_disc(arr, (base.0, y - 3.6 * size), 2.8 * size, skin, 1.0);
        }
    }
}

/// Where a throw is at `u` of its flight, in the world.
fn throw_at(th: &Throw, u: f64) -> V3 {
    let w0 = stand_point(th.p0[0], th.p0[1]);
    let w1 = scene::to_world(th.p1[0], th.p1[1], 0.12);
    let mut w = lerp3(w0, w1, u);
    w[1] += 12.0 * u * (1.0 - u);
    w
}

/// One thrown thing in the air: a tomato, a rose or a foam finger, big and outlined, with a dotted trail.
fn draw_throw(arr: &mut Img<f32>, cam: &Cam, th: &Throw, t: f64, col: V3) {
    let u = (t - th.t0) / (th.t1 - th.t0).max(1e-6);
    let w = throw_at(th, u);
    let c = cam.wp(&[w])[0];
    let rp = (0.27 * cam.f / cam.depth(&[w])[0]).max(4.0);
    let spin = (t - th.t0) * 9.0 + th.id as f64;
    let body = ITEM_COLS[th.item.clamp(0, 2) as usize][0];
    for n in 1..=3 {
        let ub = u - 0.05 * f64::from(n);
        if ub > 0.0 {
            let q = cam.wp(&[throw_at(th, ub)])[0];
            px::disc(arr, q, 1.6, bold::pale(body), 0.75 - 0.2 * f64::from(n));
        }
    }
    match th.item {
        0 => {
            // a tomato: a red ball with a shine and a green top
            px::ink_disc(arr, c, rp, TOMATO, 1.0);
            px::disc(
                arr,
                (c.0 - rp * 0.35, c.1 - rp * 0.35),
                rp * 0.28,
                bold::pale(TOMATO),
                1.0,
            );
            px::ink_star(arr, (c.0, c.1 - rp * 0.8), rp * 0.6, rp * 0.22, 5, spin, LEAF, 1.0);
        }
        1 => {
            // a rose: a bloom on a green stem with a leaf
            let e = (dmath::cos(spin), dmath::sin(spin));
            let end = (c.0 - e.0 * rp * 2.8, c.1 - e.1 * rp * 2.8);
            px::ink_line(arr, c, end, 2.0, LEAF, 1.0);
            let leaf = (
                c.0 - e.0 * rp * 1.6 - e.1 * rp * 0.5,
                c.1 - e.1 * rp * 1.6 + e.0 * rp * 0.5,
            );
            px::ink_disc(arr, leaf, rp * 0.36, LEAF, 1.0);
            px::ink_disc(arr, c, rp * 0.95, ROSE, 1.0);
            px::disc(arr, c, rp * 0.5, [186.0, 30.0, 78.0], 1.0);
            px::disc(arr, (c.0 - rp * 0.3, c.1 - rp * 0.3), rp * 0.22, bold::pale(ROSE), 1.0);
        }
        _ => {
            // a foam finger: a big hand in its colour, one finger up, a white cuff
            let e = (dmath::cos(spin * 0.5), dmath::sin(spin * 0.5));
            let n = (-e.1, e.0);
            let at = |along: f64, across: f64| {
                (
                    c.0 + (e.0 * along + n.0 * across) * rp,
                    c.1 + (e.1 * along + n.1 * across) * rp,
                )
            };
            px::ink_line(arr, at(0.6, 0.0), at(2.7, 0.0), (rp * 0.75).max(3.0), col, 1.0);
            px::ink_poly(
                arr,
                &[at(1.0, 0.95), at(1.0, -0.95), at(-0.9, -0.95), at(-0.9, 0.95)],
                col,
                1.0,
            );
            px::line(arr, at(-0.75, -0.8), at(-0.75, 0.8), 2.0, bold::WHITE, 1.0);
        }
    }
}

impl TenorSaxLook {
    /// The hush of SUBTONE that is on at `tc`: how long it has been on.
    fn hushed(&self, p: &PictureCtx, tc: f64) -> Option<f64> {
        bold::since(p, self.k, &["hush"], tc, 6.0)
            .filter(|(_, e)| tc < e.f(3))
            .map(|(age, _)| age)
    }

    /// Whether a volley is on at `tc`: from the pose to the last throw's landing.
    fn volley(&self, tc: f64) -> bool {
        self.pose_at(tc).is_some() || self.throws.iter().any(|th| th.t0 - 0.3 <= tc && tc < th.t1 + 0.15)
    }

    /// Whether the crowd is up for something at `tc`: a volley, a strut or a dive.
    fn crowd_on(&self, p: &PictureCtx, tc: f64) -> bool {
        self.volley(tc)
            || self.runway_at(tc).is_some()
            || self.dive_at(tc).is_some()
            || bold::since(p, self.k, &["subtone"], tc, 1.2).is_some()
    }

    /// The strut's bounce and the pose's stretch; the rest of the ball's state is `look.rs`'s.
    pub(super) fn bold_state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase) {
        if kk != self.k || phase != Phase::Fight {
            return;
        }
        let tc = self.ta(p, t);
        if self.dive_at(tc).is_some() {
            return;
        }
        if let Some(rw) = self.runway_at(tc)
            && tc < rw.t1
        {
            // two steps to the beat
            let hop = dmath::sin(PI * (tc - rw.t0) / (p.show.grid.beat / 2.0)).abs();
            st.lift = st.lift.max(0.2 * hop);
            st.sx = Some(1.06 - 0.1 * hop);
            st.sy = Some(0.94 + 0.12 * hop);
        }
        if let Some((u, _)) = self.pose_at(tc) {
            let k = dmath::sin(PI * u.min(1.0));
            st.sx = Some(1.0 - 0.1 * k);
            st.sy = Some(1.0 + 0.14 * k);
        }
    }

    /// The crowd bounces through a volley, a strut and a dive, and holds still in the hush.
    pub(super) fn bold_crowd(&self, p: &PictureCtx, t: f64) -> Option<f64> {
        if !(p.show.grid.t_fight <= t && t < p.t_hs) {
            return None;
        }
        let tc = self.ta(p, t);
        if self.hushed(p, tc).is_some() {
            return None;
        }
        self.crowd_on(p, tc).then_some(0.9)
    }

    /// WALK THE BAR's runway: a bright lane with lit edges, chasing bulbs and a spotlight on the ball.
    fn runway_light(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, rw: &Runway, tc: f64) {
        let fade = ((tc - rw.t0) / 0.12).min(1.0) * ((rw.t1 + 0.25 - tc) / 0.25).clamp(0.0, 1.0);
        let d = [rw.b[0] - rw.a[0], rw.b[1] - rw.a[1]];
        let len = dmath::hypot(d[0], d[1]) + 1e-9;
        let u = [d[0] / len, d[1] / len];
        let ext = 0.7;
        let (a, b) = (
            [rw.a[0] - u[0] * ext, rw.a[1] - u[1] * ext],
            [rw.b[0] + u[0] * ext, rw.b[1] + u[1] * ext],
        );
        floor::lane(ic, c, a, b, 0.46, CATWALK, 0.8 * fade);
        let step = 0.36;
        let ph = ((tc - rw.t0) / (p.show.grid.beat / 4.0)) as i64;
        let count = ((len + 2.0 * ext) / step) as i64 + 1;
        for side in [-1.0, 1.0] {
            let off = [-u[1] * side * 0.52, u[0] * side * 0.52];
            floor::lane(
                ic,
                c,
                [a[0] + off[0], a[1] + off[1]],
                [b[0] + off[0], b[1] + off[1]],
                0.05,
                BULB,
                0.7 * fade,
            );
            for j in 0..count {
                let s = j as f64 * step;
                let on = (j + ph).rem_euclid(3) == 0;
                floor::disc(
                    ic,
                    c,
                    a[0] + u[0] * s + off[0],
                    a[1] + u[1] * s + off[1],
                    if on { 0.14 } else { 0.08 },
                    BULB,
                    if on { 1.6 } else { 0.5 } * fade,
                );
            }
        }
        let st = &c.fs[self.k];
        if tc < rw.t1 {
            floor::disc(ic, c, st.p[0], st.p[1], 0.95, [1.0, 0.95, 0.8], 0.5 * fade);
        }
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let k = self.k;
        let tc = self.ta(p, c.t);
        let tint = bold::light(p.col[k]);
        // WALK THE BAR: the lane is dashed in, then the runway
        if let Some((u, e)) = bold::until(p, k, &["runway"], tc, LANE_S) {
            floor::dashes(
                ic,
                c,
                [e.f(3), e.f(4)],
                [e.f(5), e.f(6)],
                0.42,
                0.45,
                u * 4.0,
                CATWALK,
                0.25 + 0.45 * u,
            );
        }
        if let Some(rw) = self.runway_at(tc) {
            self.runway_light(p, ic, c, rw, tc);
        }
        // the safe middle while a volley is on: nothing lands inside the dashed circle
        let safe = scene::A - c.fs[k].blen;
        if safe > 0.25 && self.volley(tc) {
            for (j, light) in ic.iter_mut().enumerate() {
                let (dx, dz) = (c.sxp[j], c.szp[j]);
                let ring = (1.0 - (dmath::hypot(dx, dz) - safe).abs() / 0.08).clamp(0.0, 1.0);
                if ring > 0.0 && ((dmath::atan2(dz, dx) + PI) * 7.0 + tc * 2.0).floor().rem_euclid(2.0) == 0.0 {
                    for v in light.iter_mut() {
                        *v += ring * 0.95;
                    }
                }
            }
        }
        // each throw's landing mark closes in as it falls
        for th in self.flying(tc) {
            let u = (tc - th.t0) / (th.t1 - th.t0).max(1e-6);
            let item = bold::light(ITEM_COLS[th.item.clamp(0, 2) as usize][0]);
            floor::target(ic, c, th.p1[0], th.p1[1], 0.32, u, item);
        }
        // SUBTONE: the lights snap back up
        if let Some((age, _)) = bold::since(p, k, &["subtone"], tc, 0.2) {
            floor::disc(ic, c, 0.0, 0.0, 4.6, [1.0, 0.95, 0.85], 0.4 * (1.0 - age / 0.2));
        }
        // STAGE DIVE: the spot the cannonball is thrown at, and dashes along its flight
        for d in &self.dives {
            let Some((l0, l1, t_land)) = d.launch else { continue };
            let from = d.t_launch - MARK_S;
            if from <= tc && tc < t_land {
                let u = (tc - from) / (t_land - from).max(1e-6);
                floor::target(ic, c, l1[0], l1[1], 0.75, u, tint);
                floor::dashes(ic, c, l0, l1, 0.1, 0.4, tc * 8.0, tint, 0.4 + 0.5 * u);
            }
        }
    }

    /// Each throw's shadow grows as it falls.
    pub(super) fn bold_canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let tc = self.ta(p, c.t);
        for th in self.flying(tc) {
            let u = (tc - th.t0) / (th.t1 - th.t0).max(1e-6);
            floor::shade(ic, c, th.p1[0], th.p1[1], 0.08 + 0.2 * u, 0.55 * u);
        }
    }

    pub(super) fn bold_canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.owner != self.k {
            return;
        }
        let tint = bold::light(p.col[self.k]);
        match w.kind.as_str() {
            "honk" => floor::wave(ic, c, Family::Air, w, c.t, tint, 1.2),
            "dive" => floor::wave(ic, c, Family::Quake, w, c.t, tint, 1.2),
            _ => {}
        }
    }

    /// The throws in flight, the honks' arcs and the crowd's hands under a stage diver.
    pub(super) fn bold_over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        if !f.in_fight {
            return;
        }
        let (cam, k) = (f.cam, self.k);
        let col = p.col[k];
        let tc = self.ta(p, f.t);
        // the front row: up and bouncing while the crowd has something to cheer, ducking in the hush
        let beat = p.show.grid.beat;
        match self.hushed(p, tc) {
            Some(age) => {
                let up = (age / 0.08).min(1.0) * (1.0 - 0.8 * bold::ease_in((age - 0.12) / 0.4));
                front_row(arr, cam, f.t, beat, up, 0.0);
            }
            None => {
                // eased over four frames, so the row rises and drops
                let up = (0..4).filter(|&n| self.crowd_on(p, tc - f64::from(n) / 30.0)).count() as f64 / 4.0;
                front_row(arr, cam, f.t, beat, up, up);
            }
        }
        for th in self.flying(tc) {
            draw_throw(arr, cam, th, tc, col);
        }
        // a honk: an arc travelling out ahead of the strut and one behind it
        let along = self
            .runway_at(tc)
            .map_or(0.0, |rw| dmath::atan2(rw.b[1] - rw.a[1], rw.b[0] - rw.a[0]));
        for w in p.run.waves(f.i) {
            if w.owner != k || w.kind != "honk" {
                continue;
            }
            let fade = dmath::pow((1.0 - w.r / w.rmax.max(1e-6)).max(0.0), 0.6);
            for (mid, half) in [(along, 0.9), (along + PI, 0.6)] {
                let arc: Vec<(f64, f64)> = (0..=12)
                    .map(|j| {
                        let a = mid - half + 2.0 * half * f64::from(j) / 12.0;
                        bold::at(cam, w.cx + w.r * dmath::cos(a), w.cz + w.r * dmath::sin(a), 0.3)
                    })
                    .collect();
                px::ink_polyline(arr, &arc, 2.6, bold::pale(col), fade);
            }
        }
        let st = &f.fs[k];
        if st.vis && st.extra.get("surf").is_some_and(|&v| v != 0.0) {
            // the crowd's hands holding it up, waving
            let at = scene::to_world(st.p[0], st.p[1], R + st.lift);
            let c = cam.wp(&[at])[0];
            let rp = R * cam.f / cam.depth(&[at])[0];
            for j in 0..8u32 {
                let x = c.0 - rp * 1.3 + f64::from(j) * rp * 0.37;
                let wob = dmath::sin(f.t * 10.0 + f64::from(j) * 1.7) * 2.5;
                let top = (x + wob, c.1 + rp * 0.8 - f64::from(j % 2) * 2.0);
                let skin = SKIN[(j % 4) as usize];
                px::ink_line(arr, (x, c.1 + rp * 1.1 + 10.0), top, 2.6, skin, 1.0);
                px::ink_disc(arr, top, 2.8, skin, 1.0);
            }
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let k = self.k;
        let o = 1 - k;
        let col = p.col[k];
        let tc = self.ta(p, f.t);
        let st = &f.fs[k];
        let me = drawn.iter().find(|d| d.k == k);
        let diving = self.dive_at(tc).is_some();
        // SUBTONE: the lights go down round the ball; breath puffs from the bell and from the stands
        if let Some(age) = self.hushed(p, tc) {
            let down = bold::ease_out(age / DIM_S);
            let (c, rp) = me.map_or(((180.0, 300.0), 20.0), |d| (d.s, d.rp));
            px::dim_outside(img, c, rp * 2.4, rp * 1.6, 1.0 - 0.55 * down, &|x, y| {
                BAYER[y % 4][x % 4]
            });
            for n in 0..3 {
                let ph = (age / 0.6 + f64::from(n) / 3.0).rem_euclid(1.0);
                let q = (c.0 + rp * 1.0 + 9.0 * ph, c.1 - rp * 0.5 - 20.0 * ph);
                px::ink_disc(img, q, 2.5 + 5.0 * ph, BREATH, (1.0 - ph * ph) * down);
            }
            for j in 0..22u32 {
                let a = TAU * f64::from(j) / 22.0 + 0.2;
                let w = stand_point(dmath::cos(a), dmath::sin(a));
                let s = f.cam.wp(&[[w[0], w[1] + 0.5, w[2]]])[0];
                let ph = (age / 0.7 + px::rnd(u64::from(j), 3)).rem_euclid(1.0);
                px::disc(
                    img,
                    (s.0 + 3.0 * ph, s.1 - 14.0 * ph),
                    2.4 + 3.6 * ph,
                    BREATH,
                    0.85 * (1.0 - ph * ph) * down,
                );
            }
        }
        if let Some(d) = me
            && st.vis
            && st.scale > 0.5
            && !diving
        {
            let pose = self.pose_at(tc);
            if let Some((u, _)) = pose
                && u < 0.9
            {
                // the pose: a glint, thick honk lines from the bell and a note
                let left = 1.0 - u;
                px::glint(
                    img,
                    (d.s.0 + d.rp * 0.8, d.s.1 - d.rp * 0.85),
                    5.0 + 8.0 * left,
                    bold::WHITE,
                    left,
                );
                for j in 0..3 {
                    let a = -0.9 + 0.5 * f64::from(j);
                    let (r0, r1) = (d.rp * (1.2 + 0.5 * u), d.rp * (1.8 + 0.9 * u));
                    let (ca, sa) = (dmath::cos(a), dmath::sin(a));
                    px::ink_line(
                        img,
                        (d.s.0 + r0 * ca, d.s.1 + r0 * sa),
                        (d.s.0 + r1 * ca, d.s.1 + r1 * sa),
                        2.4,
                        bold::pale(col),
                        left,
                    );
                }
                px::note(
                    img,
                    (d.s.0 + d.rp * (2.0 + u), d.s.1 - d.rp * (1.3 + 0.6 * u)),
                    bold::pale(col),
                    left,
                );
            }
            // the hype meter: five lights in an arc over the ball
            let n = self.n_max.unwrap_or(0.0) as i64;
            let n_on = (self.hype(p, f.t) + 1e-9) as i64;
            for j in 0..n {
                let a = -PI / 2.0 + (j as f64 - (n - 1) as f64 / 2.0) * 0.36;
                let q = (d.s.0 + d.rp * 1.55 * dmath::cos(a), d.s.1 + d.rp * 1.55 * dmath::sin(a));
                if j < n_on {
                    let flash = pose.map_or(0.0, |(u, _)| 1.0 - u);
                    let lit: V3 = std::array::from_fn(|ch| bold::GOLD[ch] + (bold::WHITE[ch] - bold::GOLD[ch]) * flash);
                    px::ink_disc(img, q, 3.2 + 1.2 * flash, lit, 1.0);
                    px::disc(img, (q.0 - 1.0, q.1 - 1.0), 1.0, bold::WHITE, 1.0);
                } else {
                    px::ink_disc(img, q, 2.6, [74.0, 62.0, 78.0], 1.0);
                }
            }
        }
        // a throw that hit: it splats on the enemy's ball
        if let Some(e) = drawn.iter().find(|d| d.k == o) {
            let foe = &f.fs[o];
            for &(id, (t0, x, z, hit)) in &self.splat {
                let age = tc - t0;
                if !hit || !(0.0..SPLAT_S).contains(&age) {
                    continue;
                }
                let item = self.throws.iter().find(|th| th.id == id).map_or(0, |th| th.item);
                let then = p.run.p(p.sim_i(t0), o);
                let (ox, oz) = (x - then[0], z - then[1]);
                let far = (dmath::hypot(ox, oz) / 0.3).max(1.0);
                let s = bold::at(f.cam, foe.p[0] + ox / far, foe.p[1] + oz / far, R * 1.15 + foe.lift);
                let seed = (t0 * 1000.0) as u64 + id as u64;
                let u = age / SPLAT_S;
                let a = ((1.0 - u) * 3.0).min(1.0);
                match item {
                    0 => {
                        let r = e.rp * 0.75 * bold::ease_out(age / 0.07);
                        px::ink_star(img, s, r, r * 0.55, 9, px::rnd(seed, 0) * TAU, TOMATO, a);
                        px::disc(img, s, r * 0.4, ITEM_COLS[0][1], a);
                        for n in 0..3u64 {
                            let dx = (n as f64 - 1.0) * r * 0.45;
                            let fall = r * 0.5 + 16.0 * u * (0.6 + 0.3 * px::rnd(seed, 5 + n));
                            px::ink_disc(img, (s.0 + dx, s.1 + fall), 1.8, TOMATO.map(|v| v * 0.85), a);
                        }
                    }
                    1 => {
                        px::chips(img, s, age, 0.5, 9, 22.0, seed, ROSE);
                        px::ink_star(
                            img,
                            s,
                            e.rp * 0.4,
                            e.rp * 0.2,
                            6,
                            0.3,
                            bold::pale(ROSE),
                            (1.0 - age / 0.25).max(0.0),
                        );
                    }
                    _ => {
                        px::chips(img, s, age, 0.4, 6, 20.0, seed, col);
                        px::ink_star(
                            img,
                            s,
                            e.rp * (0.45 + 0.3 * u),
                            e.rp * 0.15,
                            4,
                            0.4,
                            bold::WHITE,
                            (1.0 - age / 0.25).max(0.0),
                        );
                    }
                }
            }
        }
        // STAGE DIVE: speed lines behind the leap and the cannonball
        if let (Some(d), Some(dive)) = (me, self.dive_at(tc))
            && (tc < dive.t_surf || tc >= dive.t_launch)
            && dive.launch.is_some_and(|(_, _, t_land)| tc < t_land)
        {
            let was = p.fighter(k, f.t - 1.0 / 30.0);
            let s0 = bold::at(f.cam, was.p[0], was.p[1], R + was.lift);
            let dir = (d.s.0 - s0.0, d.s.1 - s0.1);
            if dmath::hypot(dir.0, dir.1) > 1.5 {
                let cannon = tc >= dive.t_launch;
                px::speed_lines(
                    img,
                    d.s,
                    d.rp,
                    dir,
                    if cannon { 34.0 } else { 22.0 },
                    if cannon { 9 } else { 6 },
                    bold::pale(col),
                    1.0,
                );
            }
        }
    }

    /// The cannonball is a heavy hit; a honk is a tap.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match bold::how(e) {
            "dive" => Some(HitSize::Heavy),
            "honk" => Some(HitSize::Tap),
            _ => Some(by_damage),
        }
    }
}
