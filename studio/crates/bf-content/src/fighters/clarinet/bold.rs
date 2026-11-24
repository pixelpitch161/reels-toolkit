//! CLARINET with the bold effects (`picture.effects = "bold"`; the owner's picks 23a to 23d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **TWELFTH JUMP** (23a): before each jump a dotted line runs from the ball to a landing mark that closes in;
//!   it vanishes in a sparkle pop, a thick white slash hangs through the enemy it cut, and it reappears inside a
//!   ring that snaps shut on the landing spot.
//! - **SQUEAK** (23b): a sharp breath in (streaks and a ring of air pulled into the ball), then a jagged white
//!   ring with twinkling sparkles; the stunned enemy gets the ring's stun stars.
//! - **CLARION** (23c): bright streaks flicker round the ball as it starts; the ring's speed-up look does the
//!   rest.
//! - **RHAPSODY** (23d): the whole zig-zag route shows as marching dotted lines, each jump turns its line solid
//!   and leaves it hanging in the air, and at the end every line flashes white and bursts.

use bf_core::dmath;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, Status, StatusKind};
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Pic};
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::scene::R_BALL as R;

use super::{ClarinetLook, Jump, draw_zip};

/// The squeak's cold white, as canvas light.
const SHRILL: V3 = [1.05, 1.0, 1.15];
/// Seconds before a jump its dotted line and landing mark show (less when the jumps come faster).
const WARN_S: f64 = 0.3;
/// Seconds the vanish and appear pops last.
const POP_S: f64 = 0.24;
/// Seconds the slash hangs through the enemy.
const SLASH_S: f64 = 0.3;
/// Seconds of breath in before SQUEAK.
const BREATH_S: f64 = 0.28;
/// Seconds CLARION's streaks flicker round the ball.
const STREAK_S: f64 = 0.8;
/// Seconds before RHAPSODY starts that its route shows.
const ROUTE_S: f64 = 0.3;
/// Seconds the lines hang after RHAPSODY's last jump, and seconds they take to flash and burst.
const HANG_S: f64 = 0.24;
const BURST_S: f64 = 0.4;

/// A point `u` of the way from `a` to `b`.
fn lerp(a: (f64, f64), b: (f64, f64), u: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u)
}

/// A dotted line from `a` to `b` on the picture: outlined dashes that march toward `b` as `phase` grows.
fn dotted(img: &mut Pic, a: (f64, f64), b: (f64, f64), phase: f64, col: V3, alpha: f64) {
    const STEP: f64 = 9.0;
    let len = dmath::hypot(b.0 - a.0, b.1 - a.1);
    if len.is_nan() || len < 1.0 {
        return;
    }
    let mut s = phase.rem_euclid(1.0) * STEP - STEP;
    while s < len {
        let (s0, s1) = (s.max(0.0), (s + 4.5).min(len));
        if s1 > s0 {
            px::ink_line(img, lerp(a, b, s0 / len), lerp(a, b, s1 / len), 2.0, col, alpha);
        }
        s += STEP;
    }
}

/// A four-point white star that pops: outlined while it is fresh, plain as it fades (a fading outline would
/// leave a grey ghost).
fn pop_star(img: &mut Pic, c: (f64, f64), r: f64, rot: f64, alpha: f64) {
    if alpha > 0.55 {
        px::ink_star(img, c, r, 2.5, 4, rot, bold::WHITE, 1.0);
    } else {
        px::star(img, c, r, 2.5, 4, rot, bold::WHITE, (alpha / 0.55).clamp(0.0, 1.0));
    }
}

/// A slash from `a` to `b`: a thin diamond, `w` pixels wide in the middle, white inside the fighter's colour.
fn slash(img: &mut Pic, a: (f64, f64), b: (f64, f64), w: f64, col: V3) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let n = dmath::hypot(dx, dy);
    if n.is_nan() || n < 1.0 || w < 0.4 {
        return;
    }
    let (nx, ny) = (-dy / n, dx / n);
    let mid = lerp(a, b, 0.5);
    let diamond = |h: f64, pull: f64| {
        [
            lerp(a, mid, pull),
            (mid.0 + nx * h, mid.1 + ny * h),
            lerp(b, mid, pull),
            (mid.0 - nx * h, mid.1 - ny * h),
        ]
    };
    px::ink_poly(img, &diamond(w, 0.0), col, 1.0);
    px::poly(img, &diamond(w * 0.5, 0.12), bold::WHITE, 1.0);
}

impl ClarinetLook {
    /// The jump that comes next after `t` when its warning is showing: how far the warning is (0 to 1) and the
    /// jump. The warning is shorter when the jumps come faster (CLARION).
    fn next_jump(&self, t: f64) -> Option<(f64, &Jump)> {
        let j = self.jumps.partition_point(|x| x.t <= t);
        let next = self.jumps.get(j)?;
        let gap = if j > 0 { next.t - self.jumps[j - 1].t } else { 1.0 };
        let win = (0.7 * gap).min(WARN_S);
        let u = 1.0 - (next.t - t) / win;
        (u >= 0.0).then_some((u, next))
    }

    /// The RHAPSODY chains whose route, lines or burst may show at `t`: each chain's jumps and when its lines
    /// burst (after its last jump, or at the K.O. when that cuts it short).
    fn chains(&self, p: &PictureCtx, t: f64) -> Vec<(f64, Vec<Jump>, f64)> {
        bold::events_in(p, self.k, &["ability:rhapsody"], 0.0, t + ROUTE_S)
            .filter_map(|e| {
                let chain: Vec<Jump> = self
                    .jumps
                    .iter()
                    .filter(|j| j.kind == 1 && j.t >= e.t && j.t < e.t + 3.0 && j.t <= p.t_ko)
                    .copied()
                    .collect();
                let last = chain.last()?.t;
                let t_end = (last + HANG_S).min(p.t_ko.max(last));
                (t < t_end + BURST_S).then_some((e.t, chain, t_end))
            })
            .collect()
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let k = self.k;
        let tint = bold::light(p.col[k]);
        // the lit path of a jump just made
        for z in self.live_zips(p, c.t) {
            floor::lane(ic, c, z.p0, z.p1, 0.07, tint, 0.6 * (1.0 - z.age / 0.34) * z.hot);
        }
        if !c.in_fight || c.t >= p.t_ko {
            return;
        }
        let b = c.fs[k].p;
        // the next jump: a landing mark closes in (its dotted line is drawn over the balls, after the grade)
        if let Some((u, j)) = self.next_jump(c.t)
            && j.kind == 0
        {
            floor::target(ic, c, j.p1[0], j.p1[1], R * 1.15, u, tint);
        }
        // SQUEAK's breath in: a ring of air pulled into the ball
        if let Some((u, _)) = bold::until(p, k, &["ability:stun"], c.t, BREATH_S) {
            let r = R * (3.4 - 2.2 * bold::ease_in(u));
            floor::band(ic, c, Family::Air, b[0], b[1], r, 0.4 + 0.6 * u, c.t, SHRILL, 1.0);
        }
    }

    pub(super) fn bold_canvas_wave(&self, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind == "stun" {
            // SQUEAK's shrill ring: jagged noise
            floor::wave(ic, c, Family::Noise, w, c.t, SHRILL, 1.3);
        }
    }

    pub(super) fn bold_over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        for z in self.live_zips(p, f.t) {
            draw_zip(arr, f.cam, &z, p.col[self.k], R);
        }
    }

    /// One jump's pops: the sparkle pop where it vanished, the ring that snaps shut where it appears and, when
    /// it cut, the slash hanging through the enemy.
    #[allow(clippy::too_many_arguments)]
    fn jump_pops(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, j: &Jump, seed: u64, enemy: [f64; 2]) {
        let age = t - j.t;
        let col = p.col[self.k];
        let lav = bold::pale(col);
        if age < POP_S {
            let u = age / POP_S;
            let out = bold::ease_out(u);
            let s0 = bold::at(cam, j.p0[0], j.p0[1], R);
            pop_star(img, s0, 6.0 + 9.0 * out, 0.4, 1.0 - u);
            for i in 0..7 {
                let ang = (i as f64 + px::rnd(seed, i)) * dmath::TAU / 7.0;
                let far = 8.0 + 22.0 * out * (0.7 + 0.5 * px::rnd(seed, i + 10));
                let q = (s0.0 + dmath::cos(ang) * far, s0.1 + dmath::sin(ang) * far * 0.75);
                px::glint(
                    img,
                    q,
                    2.0 + 3.0 * (1.0 - u),
                    if i % 2 == 0 { lav } else { bold::WHITE },
                    1.0 - u,
                );
            }
            // the landing: a ring snaps shut on the spot and four sparkles fly off it
            let s1 = bold::at(cam, j.p1[0], j.p1[1], R);
            let rp = bold::px_per_unit(cam, j.p1[0], j.p1[1], R) * R;
            px::ink_ring(img, s1, rp * (1.8 - 0.7 * out), 2.5, lav, 1.0 - u);
            for i in 0..4 {
                let ang = 0.8 + i as f64 * dmath::TAU / 4.0;
                let far = rp * (1.2 + 0.9 * out);
                let q = (s1.0 + dmath::cos(ang) * far, s1.1 + dmath::sin(ang) * far);
                px::glint(img, q, 4.0 * (1.0 - u) + 1.0, bold::WHITE, 1.0 - u);
            }
        }
        if j.hit != 0 && age < SLASH_S {
            // through the enemy, along the jump; it stays on the enemy while it hangs
            let d = [j.p1[0] - j.p0[0], j.p1[1] - j.p0[1]];
            let n = dmath::hypot(d[0], d[1]).max(1e-9);
            let (ux, uz) = (d[0] / n, d[1] / n);
            let [cx, cz] = enemy;
            let half = R * 2.1 * bold::ease_out(age / 0.05);
            let thin = 1.0 - bold::ease_in((age - (SLASH_S - 0.14)) / 0.14);
            let a = bold::at(cam, cx - ux * half, cz - uz * half, R);
            let b = bold::at(cam, cx + ux * half, cz + uz * half, R);
            slash(img, a, b, 7.0 * thin, col);
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let k = self.k;
        let lav = bold::pale(p.col[k]);
        let me = drawn.iter().find(|d| d.k == k);
        // RHAPSODY: the route as dotted lines, each jump's line hanging, and all of them bursting at the end
        for (t0, chain, t_end) in self.chains(p, f.t) {
            let shown = bold::ease_out((f.t - (t0 - ROUTE_S)) / 0.15);
            let burst = (f.t - t_end) / BURST_S;
            for (n, j) in chain.iter().enumerate() {
                let a = bold::at(f.cam, j.p0[0], j.p0[1], R);
                let b = bold::at(f.cam, j.p1[0], j.p1[1], R);
                if f.t < j.t {
                    dotted(img, a, b, f.t * 4.0, lav, shown);
                    px::ink_ring(img, b, 4.5, 1.5, lav, shown);
                } else if burst < 0.0 {
                    let fresh = (1.0 - (f.t - j.t) / 0.1).max(0.0);
                    px::ink_line(img, a, b, 3.0 + 2.5 * fresh, p.col[k], 1.0);
                    px::line(img, a, b, 1.0 + 1.5 * fresh, bold::WHITE, 1.0);
                } else {
                    // the flash, then the line breaks up into chips and stars
                    if burst < 0.3 {
                        px::ink_line(img, a, b, 5.5, bold::WHITE, 1.0);
                    }
                    let seed = (j.t * 1000.0) as u64 + n as u64;
                    for (m, u) in [0.2, 0.5, 0.8].into_iter().enumerate() {
                        let q = lerp(a, b, u);
                        px::chips(img, q, f.t - t_end, BURST_S, 4, 18.0, seed + 7 * m as u64, lav);
                        let r = 4.0 + 10.0 * bold::ease_out(burst);
                        pop_star(img, q, r, u * 3.0, 1.0 - burst);
                    }
                }
            }
        }
        // the next jump: a dotted line from the ball to its landing spot, through the enemy
        if let (Some((u, j)), Some(d)) = (self.next_jump(f.t), me)
            && j.kind == 0
            && f.t < p.t_ko
        {
            let to = bold::at(f.cam, j.p1[0], j.p1[1], R);
            dotted(img, d.s, to, f.t * 5.0, lav, 0.6 + 0.4 * u);
        }
        // every jump: the pops and the slash
        let i0 = self.jumps.partition_point(|j| j.t < f.t - SLASH_S);
        for (n, j) in self.jumps.iter().enumerate().skip(i0) {
            if j.t > f.t || j.t > p.t_ko {
                break;
            }
            self.jump_pops(p, img, f.cam, f.t, j, 31 * n as u64 + 5, f.fs[1 - k].p);
        }
        // SQUEAK's breath in: streaks pulled into the ball
        if let (Some((u, _)), Some(d)) = (bold::until(p, k, &["ability:stun"], f.t, BREATH_S), me) {
            let near = d.rp * (2.6 - 1.5 * bold::ease_in(u));
            for i in 0..8 {
                let ang = 0.3 + f64::from(i) * dmath::TAU / 8.0;
                let (cx, sy) = (dmath::cos(ang), dmath::sin(ang) * 0.8);
                let q0 = (d.s.0 + cx * (near + 15.0), d.s.1 + sy * (near + 15.0));
                let q1 = (d.s.0 + cx * near, d.s.1 + sy * near);
                px::ink_arrow(img, q0, q1, 1.5, 5.0, bold::WHITE, 0.5 + 0.5 * u);
            }
        }
        // SQUEAK: the jagged ring, inked, with sparkles twinkling just outside it
        for w in p.run.waves(f.i) {
            if w.kind != "stun" || w.owner != k {
                continue;
            }
            let fade = (2.0 * (1.0 - w.r / w.rmax)).clamp(0.0, 1.0);
            let ring: Vec<(f64, f64)> = (0..=48)
                .map(|i| {
                    let a = f64::from(i) * dmath::TAU / 48.0;
                    let r = w.r + if i % 2 == 0 { 0.14 } else { -0.1 };
                    bold::at(f.cam, w.cx + r * dmath::cos(a), w.cz + r * dmath::sin(a), 0.25)
                })
                .collect();
            px::ink_polyline(img, &ring, 2.0, bold::WHITE, fade);
            for i in 0..10u64 {
                if (f.f as u64 + i).is_multiple_of(2) {
                    continue;
                }
                let a = (i as f64 + px::rnd(f.f as u64, i)) * dmath::TAU / 10.0;
                let r = w.r + 0.3;
                let q = bold::at(f.cam, w.cx + r * dmath::cos(a), w.cz + r * dmath::sin(a), 0.25);
                px::glint(img, q, 4.0, if i % 3 == 0 { lav } else { bold::WHITE }, fade);
            }
        }
        // CLARION: bright streaks flicker round the ball as it starts
        if let (Some((age, _)), Some(d)) = (bold::since(p, k, &["ability:haste"], f.t, STREAK_S), me) {
            let alpha = (2.0 * (1.0 - age / STREAK_S)).min(1.0);
            let frame = f.f as u64;
            for i in 0..7u64 {
                let a0 = px::rnd(frame, i) * dmath::TAU;
                let r = d.rp * (1.2 + 0.7 * px::rnd(frame, i + 20));
                let sweep = 0.35 + 0.5 * px::rnd(frame, i + 40);
                px::ink_arc(
                    img,
                    d.s,
                    r,
                    2.0,
                    a0,
                    a0 + sweep,
                    if i % 2 == 0 { bold::WHITE } else { lav },
                    alpha,
                );
            }
        }
    }

    /// The breath in before SQUEAK: the ball swells (and lets go as the ring leaves it).
    pub(super) fn bold_state(&self, p: &PictureCtx, st: &mut BallState, t: f64) {
        let k = self.k;
        if let Some((u, _)) = bold::until(p, k, &["ability:stun"], t, BREATH_S) {
            st.scale *= 1.0 + 0.22 * bold::ease_in(u);
        } else if let Some((age, _)) = bold::since(p, k, &["ability:stun"], t, 0.1) {
            st.scale *= 1.0 + 0.22 * (1.0 - age / 0.1);
        }
    }

    /// SQUEAK's stun on the enemy, for the ring's stun stars.
    pub(super) fn bold_statuses(&self, p: &PictureCtx, t: f64) -> Vec<Status> {
        self.stuns
            .iter()
            .filter(|&&(t0, _, dur)| t0 <= t && t < t0 + dur && t0 < p.t_ko)
            .map(|&(t0, who, dur)| Status {
                who,
                kind: StatusKind::Stun,
                left: Some(1.0 - (t - t0) / dur.max(1e-6)),
            })
            .collect()
    }
}
