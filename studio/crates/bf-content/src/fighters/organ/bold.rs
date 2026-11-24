//! ORGAN with the bold effects (`picture.effects = "bold"`; the owner's picks 39a to 39d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **PIPE ROWS** (39a): a crack runs along the floor where the row will burst; the pipes are thicker, outlined
//!   silver with a gilt lip and a lit top, each taller than the one before; under a hit enemy one more pipe
//!   shoots up and launches it into the air; then they sink back.
//! - **CATHEDRAL** (39b): eight stained-glass arches rise round the ball (panes behind it, the gilt frames in
//!   front); the pane that takes a hit flashes white; the panes go dark one by one as the shield runs down and
//!   the arches sink as it ends.
//! - **PULL ALL STOPS** (39c): five stop knobs pop out along the top of the ball and jump on every row (the
//!   ring's own speed-up look adds the afterimages and the draining ring).
//! - **TOCCATA** (39d): a target closes in on the darkening spot; the forest bursts up in the bold pipes with
//!   lit tops, chips and a quake ring; they sink back.

use bf_core::dmath;
use bf_picture::bold;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::camera::{Cam, V3};
use bf_picture::lighting::Lights;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Phase, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::BallState;
use bf_picture::scene::{self, R_BALL as R};

use super::{GLASS_B, GOLD, OrganLook, Pipe, TIN_DK, draw_pipe, echo_rings, forest_pipes, seed_of, slams};

/// Seconds the crack runs ahead of a row.
const CRACK_S: f64 = 0.4;
/// Seconds a hit enemy is in the air, and how high it goes.
const LAUNCH_S: f64 = 0.5;
const LAUNCH_H: f64 = 0.95;
/// How much thicker a bold pipe is.
const THICK: f64 = 1.3;
/// The arches of CATHEDRAL: how many, how far from the ball, their shoulders and their points.
const ARCHES: usize = 8;
const ARCH_R: f64 = R * 1.6;
const ARCH_H1: f64 = 1.0;
const ARCH_H2: f64 = 1.75;
/// Seconds a struck pane stays white.
const FLASH_S: f64 = 0.25;
/// A stop knob's ivory.
const IVORY: V3 = [250.0, 240.0, 218.0];
/// The violet pane.
const VIOLET: V3 = [156.0, 84.0, 214.0];

/// One painter item of the bold look.
enum BItem {
    /// A pipe, and how much taller the bold look makes it.
    Pipe(Pipe, f64),
    /// The pipe that launches a hit enemy: x, z, height.
    Launch(f64, f64, f64),
    /// CATHEDRAL's arches behind the ball (`false`) or in front of it (`true`), with the dome's strength.
    Arches(bool, f64),
}

/// How far the ring-frame point `q` is from the segment `a` to `b`.
fn seg_dist(q: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (vx, vz) = (b[0] - a[0], b[1] - a[1]);
    let len2 = vx * vx + vz * vz;
    let u = if len2 > 0.0 {
        (((q[0] - a[0]) * vx + (q[1] - a[1]) * vz) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    dmath::hypot(q[0] - a[0] - u * vx, q[1] - a[1] - u * vz)
}

/// A crack in the canvas along the zigzag `pts`: a dark split with a lit edge in `tint`.
fn crack(ic: &mut [V3], c: &CanvasCtx<'_>, pts: &[[f64; 2]], tint: V3, k: f64) {
    let (Some(&first), Some(&last)) = (pts.first(), pts.last()) else {
        return;
    };
    for (j, light) in ic.iter_mut().enumerate() {
        let q = [c.sxp[j], c.szp[j]];
        if seg_dist(q, first, last) > 0.3 {
            continue;
        }
        let d = pts
            .windows(2)
            .map(|s| seg_dist(q, s[0], s[1]))
            .fold(f64::INFINITY, f64::min);
        if d < 0.035 {
            for ch in light.iter_mut() {
                *ch *= 1.0 - 0.75 * k;
            }
        } else if d < 0.1 {
            for ch in 0..3 {
                light[ch] += tint[ch] * 0.55 * k;
            }
        }
    }
}

/// The direction from a pipe's foot toward the camera in the ring frame, and the tangent across it (both units).
fn facing(cam: &Cam, cx: f64, cz: f64) -> ([f64; 2], [f64; 2]) {
    let foot = scene::to_world(cx, cz, 0.0);
    let ang = dmath::atan2(cam.pos[2] - foot[2], cam.pos[0] - foot[0]);
    let (dx, dz) = (dmath::cos(ang), dmath::sin(ang));
    let root2 = 2.0_f64.sqrt();
    let d = [(dx + dz) / root2, (dz - dx) / root2];
    (d, [-d[1], d[0]])
}

/// A bold pipe: the kit's tin pipe drawn thicker, then dark edges, a highlight, a gilt lip and a lit top.
#[allow(clippy::too_many_arguments)]
fn draw_bold_pipe(fr: &mut Frame, cam: &Cam, l: &Lights, cx: f64, cz: f64, h: f64, glow: f64, col: V3, r: f64) {
    if h < 0.03 {
        return;
    }
    draw_pipe(fr, cam, l, cx, cz, h, glow, col, r);
    let (d, t) = facing(cam, cx, cz);
    let at = |along: f64, out: f64, y: f64| {
        scene::to_world(cx + t[0] * along + d[0] * out, cz + t[1] * along + d[1] * out, y)
    };
    for side in [-1.0, 1.0] {
        fr.line(cam, &[at(side * r, 0.0, 0.0), at(side * r, 0.0, h)], bold::INK, 2);
    }
    if h > 0.4 {
        fr.line(
            cam,
            &[at(-r * 0.45, r * 0.9, h * 0.42), at(-r * 0.45, r * 0.9, h * 0.84)],
            bold::WHITE,
            1,
        );
        let lip = h * 0.9;
        fr.line(cam, &[at(-r, 0.0, lip), at(0.0, r, lip), at(r, 0.0, lip)], GOLD, 2);
    }
    // the top: lit in the owner's colour as the pipe speaks
    let top: Vec<V3> = (0..6)
        .map(|j| {
            let a = f64::from(j) * dmath::PI / 3.0 + dmath::PI / 6.0;
            scene::to_world(cx + r * dmath::cos(a), cz + r * dmath::sin(a), h)
        })
        .collect();
    let lit = bold::pale(col);
    let g = glow.clamp(0.0, 1.0);
    bold::outlined_poly(
        fr,
        cam,
        &top,
        std::array::from_fn(|ch| TIN_DK[ch] * 0.7 * (1.0 - g) + lit[ch] * g),
    );
}

/// The seven corners of arch `j` round the ball at `(cx, cz)`, risen by `rise` (0 to 1): foot, shoulder, haunch,
/// point, haunch, shoulder, foot (the shoulders are the indices 1 and 5).
fn arch(cx: f64, cz: f64, j: usize, rise: f64) -> Vec<V3> {
    let a = dmath::TAU * j as f64 / ARCHES as f64 + dmath::PI / ARCHES as f64;
    let (ca, sa) = (dmath::cos(a), dmath::sin(a));
    let c = [cx + ARCH_R * ca, cz + ARCH_R * sa];
    let half = ARCH_R * 0.38;
    let (h1, h2) = (ARCH_H1 * rise, ARCH_H2 * rise);
    [
        (-1.0, 0.0),
        (-1.0, h1),
        (-0.62, h1 + 0.62 * (h2 - h1)),
        (0.0, h2),
        (0.62, h1 + 0.62 * (h2 - h1)),
        (1.0, h1),
        (1.0, 0.0),
    ]
    .iter()
    .map(|&(s, y)| scene::to_world(c[0] - sa * half * s, c[1] + ca * half * s, y))
    .collect()
}

impl OrganLook {
    /// How far along its row a pipe stands (0 at the organ, 1 at the far end); `None` for a pipe of no row (the
    /// forest, the solo ring).
    fn along(&self, tc: f64, q: &Pipe) -> Option<f64> {
        let j1 = self.line_t.partition_point(|&v| v <= tc);
        self.lines[j1.saturating_sub(8)..j1].iter().rev().find_map(|l| {
            let (dx, dz) = (q.sx - l.p0[0], q.sz - l.p0[1]);
            let x = dx * l.u[0] + dz * l.u[1];
            let off = (dx * l.u[1] - dz * l.u[0]).abs();
            (off < 1e-6 && x > 0.0 && x < l.l).then(|| x / l.l.max(1e-6))
        })
    }

    /// The launch of a hit enemy under way at `t`: the enemy, where it stood, and the seconds since.
    fn launch(&self, p: &PictureCtx, t: f64) -> Option<(usize, [f64; 2], f64)> {
        let tt = t.min(p.t_ko);
        let (age, e) = bold::since(p, self.k, &["hit:pipe"], tt, LAUNCH_S)?;
        let who = usize::try_from(e.i(3)).ok().filter(|&v| v < 2)?;
        Some((who, p.run.p(p.sim_i(e.t), who), age))
    }

    /// The ball thumps on every row; a hit enemy is launched into the air.
    pub(super) fn bold_state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase) {
        if phase != Phase::Fight {
            return;
        }
        let tt = t.min(p.t_ko);
        if kk == self.k {
            let after = self.line_t.partition_point(|&v| v <= tt);
            if let Some(j) = after.checked_sub(1) {
                st.glow = 0.42 * dmath::exp(-(tt - self.line_t[j]) / 0.10);
            }
        } else if let Some((who, _, age)) = self.launch(p, t)
            && who == kk
        {
            st.lift = st.lift.max(LAUNCH_H * dmath::sin(dmath::PI * age / LAUNCH_S));
        }
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let k = self.k;
        let tint = bold::light(p.col[k]);
        // the pipes' glow pools while they speak
        for q in self.pipes_at(p, c.t) {
            if q.glow > 0.15 {
                floor::disc(ic, c, q.sx, q.sz, 0.5, tint, q.glow * 0.3);
            }
        }
        if !c.in_fight {
            return;
        }
        // a crack runs along the floor where the next row will burst
        if let Some((u, e)) = bold::until(p, k, &["line"], c.t, CRACK_S) {
            let (p0, dir, len) = ([e.f(4), e.f(5)], [e.f(6), e.f(7)], e.f(8));
            let reach = len * bold::ease_out(u);
            let n = (reach / 0.22).ceil().max(1.0) as u64;
            let seed = e.i(3).unsigned_abs();
            let pts: Vec<[f64; 2]> = (0..=n)
                .map(|i| {
                    let x = reach * i as f64 / n as f64;
                    let off = if i == 0 { 0.0 } else { (px::rnd(seed, i) - 0.5) * 0.2 };
                    [p0[0] + dir[0] * x - dir[1] * off, p0[1] + dir[1] * x + dir[0] * off]
                })
                .collect();
            crack(ic, c, &pts, tint, 0.5 + 0.5 * u);
        }
        // CATHEDRAL's echo rings: soft puffs of air rolling out
        let st = &c.fs[k];
        if self.shield.is_some()
            && st.shield > 0.0
            && let Some(t0) = self.last_shield(c.t)
        {
            for (rr, a) in echo_rings(c.t, t0, 2.0 * p.show.grid.beat) {
                floor::band(ic, c, Family::Air, st.p[0], st.p[1], rr, a / 0.6, c.t, tint, 1.0);
            }
        }
    }

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
                // TOCCATA: the spot darkens and the target closes in on it
                let u = (c.t - s.t0) / (s.t_slam - s.t0).max(1e-6);
                floor::shade(ic, c, s.cx, s.cz, s.rad * 1.2, 0.55 * u);
                floor::target(ic, c, s.cx, s.cz, s.rad, u, tint);
            } else {
                // the forest bursts up: the floor quakes
                let u = (c.t - s.t_slam) / 0.6;
                floor::band(
                    ic,
                    c,
                    Family::Quake,
                    s.cx,
                    s.cz,
                    s.rad + 1.6 * u,
                    1.0 - u,
                    c.t,
                    tint,
                    1.2,
                );
            }
        }
    }

    pub(super) fn bold_items_ball(&self, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        let sk = self.shield_k(st);
        if sk <= 0.0 {
            return Vec::new();
        }
        let dz = f.cam.depth(&[scene::to_world(st.p[0], st.p[1], R)])[0];
        vec![
            (dz + 0.05, Box::new(BItem::Arches(false, sk)) as Payload),
            (dz - 0.05, Box::new(BItem::Arches(true, sk)) as Payload),
        ]
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let tc = p.fx_clock(f.t);
        let depth_at = |x: f64, z: f64| f.cam.depth(&[scene::to_world(x, z, 0.0)])[0];
        let mut out: Vec<(f64, Payload)> = self
            .pipes_at(p, f.t)
            .into_iter()
            .map(|q| {
                // each pipe of a row taller than the one before
                let taller = self.along(tc, &q).map_or(1.0, |u| 0.72 + 0.62 * u);
                (depth_at(q.sx, q.sz), Box::new(BItem::Pipe(q, taller)) as Payload)
            })
            .collect();
        // one more pipe shoots up under a hit enemy and carries it into the air
        if f.in_fight
            && let Some((_, at, age)) = self.launch(p, f.t)
        {
            let u = age / LAUNCH_S;
            let h = LAUNCH_H
                * if u < 0.5 {
                    dmath::sin(dmath::PI * u)
                } else {
                    1.0 - bold::ease_in((u - 0.62) / 0.38)
                };
            out.push((depth_at(at[0], at[1]) + 0.03, Box::new(BItem::Launch(at[0], at[1], h))));
        }
        out
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let k = self.k;
        match payload.downcast_ref::<BItem>() {
            Some(BItem::Pipe(q, taller)) => draw_bold_pipe(
                fr,
                f.cam,
                f.l,
                q.sx,
                q.sz,
                q.h * taller,
                q.glow,
                p.col[q.owner],
                q.r * THICK,
            ),
            Some(&BItem::Launch(x, z, h)) => draw_bold_pipe(fr, f.cam, f.l, x, z, h, 1.0, p.col[k], 0.27),
            Some(&BItem::Arches(front, sk)) => self.draw_arches(p, f, fr, front, sk),
            None => {}
        }
    }

    /// CATHEDRAL's arches on one side of the ball: stained-glass panes behind it, gilt frames in front.
    fn draw_arches(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, front: bool, sk: f64) {
        let k = self.k;
        let at = f.fs[k].p;
        let col = p.col[k];
        let panes = [col, GLASS_B.map(|v| (v * 1.3).min(255.0)), GOLD, VIOLET];
        let rise = bold::ease_out(sk);
        let left = p.bold_shield_left(k, f.t).unwrap_or(1.0);
        // the pane that took the last hit flashes white, and for a moment its two neighbours with it (the
        // attacker itself often hides the struck one)
        let struck = p.bold_last_block(k, f.t).and_then(|(tb, x, z)| {
            let flash = 1.0 - (f.t - tb) / FLASH_S;
            (flash > 0.0).then(|| {
                let a = dmath::atan2(z - at[1], x - at[0]) - dmath::PI / ARCHES as f64;
                let j = (a / dmath::TAU * ARCHES as f64).round().rem_euclid(ARCHES as f64) as usize;
                (j, flash > 0.5)
            })
        });
        let ball_depth = f.cam.depth(&[scene::to_world(at[0], at[1], 0.0)])[0];
        let mut order: Vec<(f64, usize)> = (0..ARCHES)
            .map(|j| {
                let frame = arch(at[0], at[1], j, rise);
                let foot: V3 = std::array::from_fn(|ch| (frame[0][ch] + frame[6][ch]) / 2.0);
                (f.cam.depth(&[foot])[0], j)
            })
            .filter(|&(d, _)| (d <= ball_depth) == front)
            .collect();
        order.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (_, j) in order {
            let frame = arch(at[0], at[1], j, rise);
            let mut closed = frame.clone();
            closed.push(frame[0]);
            let flash = struck.is_some_and(|(hit, wide)| {
                let off = (j + ARCHES - hit) % ARCHES;
                off == 0 || (wide && (off == 1 || off == ARCHES - 1))
            });
            // the panes go dark one by one as the shield runs down
            let lit = ((j * 3) % ARCHES) as f64 + 0.5 < left * ARCHES as f64;
            if flash {
                bold::outlined_poly(fr, f.cam, &frame, bold::WHITE);
            } else if !front {
                let (low, high) = (panes[j % 4], panes[(j + 1) % 4]);
                let dark = |c: V3| if lit { c } else { bold::deep(c) };
                fr.poly(f.cam, &[frame[0], frame[1], frame[5], frame[6]], dark(low), true);
                fr.poly(f.cam, &frame[1..6], dark(high), true);
            }
            let gilt: V3 = std::array::from_fn(|ch| (GOLD[ch] * 0.85 + col[ch] * 0.15).min(255.0));
            if front {
                bold::thin_line(fr, f.cam, &closed, gilt);
            } else {
                bold::thick_line(fr, f.cam, &closed, gilt, 3);
                // the mullion and the transom
                let foot: V3 = std::array::from_fn(|ch| (frame[0][ch] + frame[6][ch]) / 2.0);
                let mid: V3 = std::array::from_fn(|ch| (frame[1][ch] + frame[5][ch]) / 2.0);
                bold::thin_line(fr, f.cam, &[foot, mid], gilt);
                bold::thin_line(fr, f.cam, &[frame[1], frame[5]], gilt);
            }
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let (k, t) = (self.k, f.t);
        let col = p.col[k];
        // PULL ALL STOPS: stop knobs pop out along the top of the ball and jump on every row
        if f.fs[k].haste > 0.0
            && !f.fs[k].shattered
            && let Some(d) = drawn.iter().find(|d| d.k == k)
        {
            let since_on = bold::since(p, k, &["ability:haste"], t, 60.0).map_or(1.0, |(age, _)| age);
            let jump = bold::since(p, k, &["line"], t, 0.16).map_or(0.0, |(age, _)| 1.0 - age / 0.16);
            for i in 0..5 {
                let ang = -dmath::PI / 2.0 + (f64::from(i) - 2.0) * 0.44;
                let (ux, uy) = (dmath::cos(ang), dmath::sin(ang));
                let u = ((since_on - 0.05 * f64::from(i)) / 0.14).clamp(0.0, 1.0);
                let pop = bold::ease_out(u) + 0.3 * dmath::sin(dmath::PI * u);
                if pop <= 0.0 {
                    continue;
                }
                let out = d.rp + 2.0 + (7.0 + 4.0 * jump * if i % 2 == 0 { 1.0 } else { 0.5 }) * pop;
                let base = (d.s.0 + ux * d.rp * 0.94, d.s.1 + uy * d.rp * 0.94);
                let knob = (d.s.0 + ux * out, d.s.1 + uy * out);
                px::ink_line(img, base, knob, 2.0, bold::deep(GOLD), 1.0);
                px::ink_disc(img, knob, 3.6, IVORY, 1.0);
                px::disc(img, knob, 1.3, col, 1.0);
            }
        }
        // TOCCATA: the forest's tops stay lit under the dimmed ring, and chips fly as it bursts
        let tc = p.fx_clock(t);
        for &(ts, _, cx, cz) in &self.slam_ev {
            if !(ts <= tc && tc < ts + 1.0) {
                continue;
            }
            for (x, z, h, glow, r) in forest_pipes(tc, ts, cx, cz, self.forest_rad, seed_of(ts * 1000.0)) {
                if glow > 0.2 && h > 0.3 {
                    let s = bold::at(f.cam, x, z, h);
                    let rp = r * THICK * bold::px_per_unit(f.cam, x, z, h);
                    px::ink_disc(img, s, (rp * 0.8).max(2.0), bold::pale(col), glow.min(1.0));
                }
            }
            let c = bold::at(f.cam, cx, cz, 0.2);
            px::chips(
                img,
                c,
                tc - ts,
                0.55,
                18,
                64.0,
                (ts * 1000.0) as u64,
                [222.0, 228.0, 242.0],
            );
            px::chips(
                img,
                c,
                tc - ts,
                0.45,
                10,
                44.0,
                (ts * 1000.0) as u64 + 3,
                bold::pale(col),
            );
        }
    }
}
