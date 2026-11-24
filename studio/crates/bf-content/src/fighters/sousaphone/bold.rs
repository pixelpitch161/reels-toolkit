//! SOUSAPHONE with the bold effects (`picture.effects = "bold"`; the owner's picks 41a to 41d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **DRILL SET** (41a): the set is chalked on the floor like a field, yard lines across the line of march and a
//!   hash at every step; each footfall leaves a boot print (gold in double time) and little boots step under the
//!   ball; it halts in a quake and a ring of dust puffs; a dashed aim line and the cone's edges show where the bell
//!   points, the bell glints, and the blast goes out with stacked brass bands rolling along the cone.
//! - **HORN FLASH** (41b): a glint on the bell, then the blast is a cone that sweeps all the way round like a
//!   lighthouse, its afterglow trailing behind it; where it catches the enemy a brass ring bursts.
//! - **DOUBLE TIME** (41c): the boots pop in under the ball and mark time at double pace for as long as the haste
//!   lasts (the ring's own speed-up look does the rest).
//! - **COMPANY FRONT** (41d): the chalk line runs across the ring and the sousaphones fall in along it one after
//!   another; they march in step, every step a row of boot prints, chevrons on the floor pointing the way; the
//!   enemy they trample is sent on ahead of them to the rope with speed lines.

use bf_core::dmath;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, GOLD};
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Pic};
use bf_picture::ring::PictureCtx;
use bf_picture::scene::{self, R_BALL as R};

use super::{CHALK, FIBER, GHOSTS, SousaphoneLook, TURF, across, bayer, paste_ghost, unit};
use crate::fighters::sousaphone::state::Set;

/// The dust a halt kicks up.
const DUST: V3 = [226.0, 216.0, 196.0];
/// Seconds a blast shows (the classic cone's life).
const BLAST_S: f64 = 0.45;
/// Seconds the aim line shows before a blast.
const AIM_S: f64 = 0.5;
/// Seconds of dust after a halt.
const PUFF_S: f64 = 0.35;
/// Seconds the boots take to pop in when DOUBLE TIME starts.
const POP_S: f64 = 0.3;

/// Adds `v` of `tint` to a canvas pixel's light.
fn lit(light: &mut V3, tint: V3, v: f64) {
    for ch in 0..3 {
        light[ch] += v * tint[ch];
    }
}

/// Whether a point is on a boot print: `al` along the boot from its middle (toe ahead), `off` across it. A sole
/// and a heel with a gap between them.
fn on_print(al: f64, off: f64) -> bool {
    let (a, b) = ((al - 0.08) / 0.19, off / 0.11);
    let (c, d) = ((al + 0.22) / 0.085, off / 0.09);
    a * a + b * b < 1.0 || c * c + d * d < 1.0
}

/// A blast cone over the ring: from `o` along the angle `a`, `reach` long, `hw0` half as wide at the bell and
/// `hw1` at its far end: the pale cone with dark edges, its white core, and three brass arcs rolling out along it
/// (`roll` moves them: 1 is the cone's length).
#[allow(clippy::too_many_arguments)]
fn draw_cone(
    arr: &mut Img<f32>,
    cam: &Cam,
    o: [f64; 2],
    a: f64,
    reach: f64,
    hw0: f64,
    hw1: f64,
    col: V3,
    kk: f64,
    roll: f64,
) {
    let d = unit(a);
    let nn = [-d[1], d[0]];
    let at = |al: f64, sd: f64| {
        bold::at(
            cam,
            o[0] + d[0] * al + nn[0] * sd,
            o[1] + d[1] * al + nn[1] * sd,
            R * 0.9,
        )
    };
    let quad = [at(0.0, hw0), at(reach, hw1), at(reach, -hw1), at(0.0, -hw0)];
    px::poly(arr, &quad, col.map(|v| (v * 0.55 + 140.0).min(255.0)), 0.45 * kk);
    let core = [
        at(0.0, hw0 * 0.4),
        at(reach * 0.8, hw1 * 0.3),
        at(reach * 0.8, -hw1 * 0.3),
        at(0.0, -hw0 * 0.4),
    ];
    px::poly(arr, &core, bold::WHITE, 0.5 * kk);
    px::line(arr, quad[0], quad[1], 1.6, bold::INK, 0.6 * kk);
    px::line(arr, quad[3], quad[2], 1.6, bold::INK, 0.6 * kk);
    for j in 0..3 {
        let s = (roll * 1.6 + f64::from(j) / 3.0).rem_euclid(1.0) * reach;
        let hw = hw0 + (hw1 - hw0) * s / reach.max(1e-6);
        let ang = dmath::atan2(hw, s.max(0.05));
        let rr = dmath::hypot(s, hw);
        let pts: Vec<(f64, f64)> = (0..9)
            .map(|i| {
                let th = a - ang + 2.0 * ang * f64::from(i) / 8.0;
                bold::at(cam, o[0] + rr * dmath::cos(th), o[1] + rr * dmath::sin(th), R)
            })
            .collect();
        px::ink_polyline(arr, &pts, 2.4, GOLD, kk);
    }
}

/// A little marching boot on the finished picture: the middle of its sole at `c`, `s` times its 9 by 8 pixels,
/// its toe to the right when `dir` is 1 and to the left when -1.
fn boot(img: &mut Pic, c: (f64, f64), s: f64, dir: f64) {
    let pt = |x: f64, y: f64| (c.0 + x * s * dir, c.1 + y * s);
    let shape = [
        pt(-3.5, -8.0),
        pt(0.5, -8.0),
        pt(0.5, -4.0),
        pt(4.0, -3.6),
        pt(5.5, 0.0),
        pt(-3.5, 0.0),
    ];
    px::ink_poly(img, &shape, FIBER, 1.0);
    px::line(img, pt(-3.5, 0.0), pt(5.5, 0.0), 1.6 * s, bold::INK, 1.0);
}

impl SousaphoneLook {
    /// HORN FLASH turning at the angle `now`: how far it has swept (radians) and the way it turns (1 or -1).
    fn sweep(&self, p: &PictureCtx, t: f64, now: f64) -> (f64, f64) {
        match bold::since(p, self.k, &["horn_flash"], t, 4.0) {
            Some((_, e)) => {
                let a0 = e.f(3);
                ((now - a0).abs(), if now >= a0 { 1.0 } else { -1.0 })
            }
            None => (0.0, 1.0),
        }
    }

    /// A drill set chalked on the floor: the dashed line of march, yard lines across it (a long one every other
    /// step, a hash between), and a boot print pressed in at every footfall so far, fading over two beats.
    fn bold_field(&self, ic: &mut [V3], c: &CanvasCtx<'_>, s: &Set, tc: f64) {
        let chalk = CHALK.map(|v| v / 255.0);
        let gold = bold::light(GOLD);
        let a = s.p0;
        let d = [s.p1[0] - a[0], s.p1[1] - a[1]];
        let len = dmath::hypot(d[0], d[1]) + 1e-9;
        let d = [d[0] / len, d[1] / len];
        let end = s.end();
        let fade = ((tc - s.t0) / 0.15).min(1.0) * (1.0 - ((tc - end) / 0.45).max(0.0));
        let n = s.n.max(1) as f64;
        let pitch = len / n;
        let life = 2.0 * self.beat;
        // the footfalls of this set so far: how far along, which side, how fresh
        let prints: Vec<(f64, f64, f64)> = self
            .steps
            .iter()
            .enumerate()
            .filter(|(_, st)| st.0 >= s.t0 - 1e-6 && st.0 <= s.t1 + 1e-6 && st.0 <= tc && tc - st.0 < life)
            .map(|(i, st)| {
                let along = (st.1 - a[0]) * d[0] + (st.2 - a[1]) * d[1];
                let side = if i % 2 == 0 { 0.24 } else { -0.24 };
                (along, side, 1.0 - (tc - st.0) / life)
            })
            .collect();
        for (j, light) in ic.iter_mut().enumerate() {
            let (rx, rz) = (c.sxp[j] - a[0], c.szp[j] - a[1]);
            let along = rx * d[0] + rz * d[1];
            if along < -0.4 || along > len + 0.4 {
                continue;
            }
            let side = -rx * d[1] + rz * d[0];
            let off = side.abs();
            if off > 0.95 {
                continue;
            }
            let mut v: f64 = 0.0;
            if off < 0.05 && along > -0.1 && along < len + 0.1 && (along / 0.22).rem_euclid(1.0) < 0.6 {
                v = 0.8;
            }
            let q = along / pitch;
            let near = q.round();
            if (0.0..=n).contains(&near) && (q - near).abs() * pitch < 0.035 {
                let yard = (near as i64) % 2 == 0;
                if off < if yard { 0.9 } else { 0.22 } {
                    v = v.max(if yard { 1.1 } else { 0.8 });
                }
            }
            if v > 0.0 {
                lit(light, chalk, v * fade);
            }
            for &(al, sd, left) in &prints {
                if on_print(along - al, side - sd) {
                    // pressed dark into the canvas; gold in double time
                    let dark = 1.0 - (0.4 + 0.45 * left) * fade;
                    *light = light.map(|ch| ch * dark);
                    if s.kind == 1 {
                        lit(light, gold, (0.5 + 0.8 * left) * fade);
                    }
                    break;
                }
            }
        }
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let k = self.k;
        let t = c.t;
        let col = p.col[k].map(|v| v / 255.0);
        let chalk = CHALK.map(|v| v / 255.0);
        let gold = bold::light(GOLD);
        let tc = self.ta(p, t);
        let in_fight = self.t_fight <= t && t < p.t_ko;
        if !in_fight {
            return;
        }
        if let Some(s) = self.set_at(tc) {
            self.bold_field(ic, c, s, tc);
        }
        // the halt: a quake under the planted boots
        if let Some((age, e)) = bold::since(p, k, &["drill_halt"], t, 0.3) {
            let u = age / 0.3;
            floor::band(
                ic,
                c,
                Family::Quake,
                e.f(3),
                e.f(4),
                R * (0.9 + 1.4 * u),
                1.0 - u,
                t,
                bold::light(DUST),
                0.7,
            );
        }
        // a trample: the floor jumps where it trod
        if let Some((age, e)) = bold::since(p, k, &["drill_trample"], t, 0.3) {
            let u = age / 0.3;
            floor::band(
                ic,
                c,
                Family::Quake,
                e.f(3),
                e.f(4),
                0.3 + 1.1 * u,
                1.0 - u,
                t,
                bold::light(p.col[k]),
                0.9,
            );
        }
        // the aim: a dashed line down the middle of the coming blast and the cone's two edges
        if let Some((u, e)) = bold::until(p, k, &["blast"], t, AIM_S) {
            let (o, d, ln) = ([e.f(3), e.f(4)], unit(e.f(5)), e.f(6));
            let (hw0, hw1) = (0.5 * e.f(7), 0.5 * e.f(8));
            let nn = [-d[1], d[0]];
            let far = [o[0] + d[0] * ln, o[1] + d[1] * ln];
            floor::dashes(ic, c, o, far, 0.07, 0.3, t * 6.0, gold, 0.4 + 0.6 * u);
            for sg in [-1.0, 1.0] {
                floor::lane(
                    ic,
                    c,
                    [o[0] + nn[0] * sg * hw0, o[1] + nn[1] * sg * hw0],
                    [far[0] + nn[0] * sg * hw1, far[1] + nn[1] * sg * hw1],
                    0.035,
                    col.map(|v| v * 0.8 + 0.4),
                    0.25 + 0.6 * u,
                );
            }
        }
        // the bell blast: a cone of light with stacked brass bands rolling out along it
        for b in &self.blasts {
            let age = tc - b.t0;
            if !(0.0..BLAST_S).contains(&age) {
                continue;
            }
            let kk = 1.0 - age / BLAST_S;
            let d = unit(b.a);
            let far = b.ln * (0.4 + 0.6 * (age / 0.07).min(1.0));
            let tint = col.map(|v| v * 0.8 + 0.35);
            for (j, light) in ic.iter_mut().enumerate() {
                let (rx, rz) = (c.sxp[j] - b.ox, c.szp[j] - b.oy);
                let along = rx * d[0] + rz * d[1];
                if along <= 0.0 || along >= far {
                    continue;
                }
                let off = (-rx * d[1] + rz * d[0]).abs();
                let hw = 0.5 * (b.w0 + (b.w1 - b.w0) * (along / b.ln.max(1e-6)).clamp(0.0, 1.0));
                if off > hw + 0.15 {
                    continue;
                }
                let band = (1.0 - off / (hw + 0.15)).clamp(0.0, 1.0);
                let band = (band * 4.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 4.0;
                lit(light, tint, band * 0.9 * kk);
                if off < hw && (dmath::hypot(rx, rz) * 2.4 - age * 9.0).rem_euclid(1.0) < 0.4 {
                    lit(light, gold, 0.75 * kk);
                }
            }
        }
        // HORN FLASH's catch: a brass ring bursts round the enemy
        if let Some((age, e)) = bold::since(p, k, &["flash_hit"], t, 0.35) {
            let u = age / 0.35;
            floor::band(
                ic,
                c,
                Family::Brass,
                e.f(3),
                e.f(4),
                0.4 + 1.3 * u,
                1.0 - u,
                t,
                bold::light(p.col[k]),
                1.0,
            );
        }
        let Some(rw) = self.row(p, t) else { return };
        if !rw[7].is_nan() {
            // HORN FLASH: the lighthouse cone, brass bands rolling along it and its afterglow behind
            let st = &c.fs[k];
            let d = unit(rw[7]);
            let len = R + rw[8];
            let (swept, dir) = self.sweep(p, t, rw[7]);
            let glow_a = swept.min(1.9);
            let tint = col.map(|v| v * 0.8 + 0.3);
            for (j, light) in ic.iter_mut().enumerate() {
                let (rx, rz) = (c.sxp[j] - st.p[0], c.szp[j] - st.p[1]);
                let rr = dmath::hypot(rx, rz);
                if rr > len + 0.2 || rr < R * 0.8 {
                    continue;
                }
                let along = rx * d[0] + rz * d[1];
                let side = -rx * d[1] + rz * d[0];
                let off = side.abs();
                let hw = self.fl_w * 0.5 + 0.12 + 0.32 * (along / len).clamp(0.0, 1.0);
                if along > R * 0.8 && along < len && off < hw {
                    let band = (1.0 - off / hw).clamp(0.0, 1.0);
                    let band = (band * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
                    lit(light, tint, band * 0.95);
                    if off < hw * 0.85 && (rr * 2.4 - t * 9.0).rem_euclid(1.0) < 0.35 {
                        lit(light, gold, 0.7);
                    }
                    continue;
                }
                // where it has just been
                let behind = -side * dir;
                if behind > 0.0 && rr < len {
                    let da = dmath::atan2(behind, along);
                    if da < glow_a {
                        let g = ((1.0 - da / 1.9) * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
                        lit(light, tint, g * 0.42);
                    }
                }
            }
        }
        if !rw[9].is_nan() {
            // COMPANY FRONT: the chalk line running across the ring, lit behind; a row of boot prints at every
            // step; chevrons ahead of each sousaphone pointing the way
            let (ax, sg, pos) = (rw[10] as i64, rw[11], rw[9]);
            let form = (rw[15] * 8.0).min(1.0);
            let reach = -scene::A + 2.0 * scene::A * (form * 1.25).min(1.0);
            let life = 2.0 * self.beat;
            let rows: Vec<(f64, f64, f64)> = self
                .fsteps
                .iter()
                .enumerate()
                .filter(|(_, st)| st.0 <= tc && tc - st.0 < life)
                .map(|(i, st)| (st.1, if i % 2 == 0 { 0.2 } else { -0.2 }, 1.0 - (tc - st.0) / life))
                .collect();
            let pitch = 2.0 * scene::A / GHOSTS as f64;
            let tint = col.map(|v| v * 0.8 + 0.4);
            for (j, light) in ic.iter_mut().enumerate() {
                let (c_, v) = if ax == 0 {
                    (c.sxp[j], c.szp[j])
                } else {
                    (c.szp[j], c.sxp[j])
                };
                if v > reach {
                    continue;
                }
                let rel = (c_ - pos) * sg;
                if rel.abs() < 0.065 {
                    lit(light, chalk, 0.85);
                }
                if rel < 0.0 && rel > -0.9 {
                    let glow = ((1.0 + rel / 0.9) * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
                    lit(light, col, glow * 0.4);
                }
                let dv = v - (-scene::A + (((v + scene::A) / pitch).floor() + 0.5) * pitch);
                for &(sp, foot, left) in &rows {
                    if on_print((c_ - sp) * sg + 0.35, dv - foot) {
                        let dark = 1.0 - (0.3 + 0.45 * left);
                        *light = light.map(|ch| ch * dark);
                        break;
                    }
                }
                if rel > 0.9 && rel < 1.7 && dv.abs() < 0.3 && (rel - 1.55 + dv.abs() * 1.3).abs() < 0.07 {
                    lit(light, tint, 0.9);
                }
            }
        }
    }

    pub(super) fn bold_over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        let (cam, k) = (f.cam, self.k);
        let t = f.t;
        let col = p.col[k];
        let tc = self.ta(p, t);
        if self.t_fight <= t {
            for b in &self.blasts {
                let age = tc - b.t0;
                if !(0.0..BLAST_S).contains(&age) {
                    continue;
                }
                let reach = b.ln * (0.35 + age / 0.08).min(1.0);
                let hw1 = 0.5 * (b.w0 + (b.w1 - b.w0) * reach / b.ln.max(1e-6));
                draw_cone(
                    arr,
                    cam,
                    [b.ox, b.oy],
                    b.a,
                    reach,
                    0.5 * b.w0,
                    hw1,
                    col,
                    1.0 - age / BLAST_S,
                    age / BLAST_S,
                );
            }
        }
        let mut rw = self.row(p, t);
        let mut fade = 1.0;
        if rw.is_none() && p.t_ko <= t && t < p.t_ko + 0.6 && p.run.has("DRILL") {
            // the wall holds at the K.O., fading
            rw = p.run.row("DRILL", p.sim_i(p.t_ko - 1e-3), k);
            fade = 1.0 - (t - p.t_ko) / 0.6;
        }
        let Some(rw) = rw else { return };
        if !rw[7].is_nan() && fade == 1.0 {
            // HORN FLASH: the sweeping cone
            let b = f.fs[k].p;
            let d = unit(rw[7]);
            let o = [b[0] + d[0] * (R + 0.04), b[1] + d[1] * (R + 0.04)];
            let hw0 = self.fl_w * 0.5;
            draw_cone(arr, cam, o, rw[7], rw[8], hw0, hw0 + 0.3, col, 0.85, t * 2.2);
        }
        if !rw[9].is_nan() {
            // the sousaphones fall in along the line one after another, then step high on every beat
            let (ax, sg, pos) = (rw[10] as i64, rw[11], rw[9]);
            let form = (rw[15] * 8.0).min(1.0);
            let bob = dmath::sin(dmath::PI * ((t.min(p.t_ko) - self.t_fight) / self.beat)).abs();
            let mut items: Vec<(f64, V3, f64)> = (0..GHOSTS)
                .filter_map(|j| {
                    let uj = ((form * 1.25 - (j as f64 + 0.5) / GHOSTS as f64) / 0.2).clamp(0.0, 1.0);
                    if uj <= 0.0 {
                        return None;
                    }
                    let v = across(j);
                    let (x, y) = if ax == 0 {
                        (pos - sg * 0.1, v)
                    } else {
                        (v, pos - sg * 0.1)
                    };
                    let w = scene::to_world(x, y, R + 0.1 * bob + 0.6 * (1.0 - bold::ease_out(uj)));
                    Some((cam.depth(&[w])[0], w, uj))
                })
                .collect();
            items.sort_by(|x, y| y.0.total_cmp(&x.0));
            for (dz, w, uj) in items {
                let c = cam.wp(&[w])[0];
                let rp = R * cam.f / dz;
                let al = 0.8 * uj * ((1.0 - rw[15]) * 8.0).min(1.0) * fade;
                let spr = self.ghost_sprite(rp * 1.9);
                paste_ghost(arr, &spr, col, al, c.0, c.1);
            }
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let (k, o, t, cam) = (self.k, 1 - self.k, f.t, f.cam);
        if !f.in_fight || t >= p.t_ko {
            return;
        }
        let me = drawn.iter().find(|d| d.k == k).filter(|_| !f.fs[k].shattered);
        let foe = drawn.iter().find(|d| d.k == o).filter(|_| !f.fs[o].shattered);
        let rw = self.row(p, t);
        // the halt: a ring of dust puffs rolls out from the boots
        if let Some((age, e)) = bold::since(p, k, &["drill_halt"], t, PUFF_S) {
            let u = age / PUFF_S;
            for i in 0..8 {
                let a = f64::from(i) * dmath::TAU / 8.0 + 0.3;
                let r = R * (0.75 + 1.25 * bold::ease_out(u));
                let c = bold::at(
                    cam,
                    e.f(3) + r * dmath::cos(a),
                    e.f(4) + r * dmath::sin(a),
                    0.05 + 0.25 * u,
                );
                px::ink_disc(img, c, 2.5 + 3.0 * u, DUST, 1.0 - u * u);
            }
        }
        // a trample: turf flies
        if let Some((age, e)) = bold::since(p, k, &["drill_trample"], t, 0.4) {
            let c = bold::at(cam, e.f(3), e.f(4), 0.1);
            px::chips(img, c, age, 0.4, 9, 30.0, (e.t * 1000.0) as u64, TURF[0]);
        }
        if let Some(d) = me {
            // little boots under the ball: stepping on every footfall of a set, marking time in double time
            let hasted = p.bold_haste_left(k, t).is_some();
            let marching = rw.as_ref().is_some_and(|r| r[0] == 1.0);
            let halted = rw.as_ref().is_some_and(|r| r[0] == 2.0);
            if hasted || marching || halted {
                let phase = match self.set_at(t) {
                    Some(s) if marching => (t - s.t0) / ((s.t1 - s.t0) / s.n.max(1) as f64).max(1e-6),
                    _ if hasted && !halted => (t - self.t_fight) / (self.beat / 4.0),
                    _ => 0.0,
                };
                let stepping = marching || (hasted && !halted);
                let (foot, up) = (
                    phase.floor().rem_euclid(2.0),
                    dmath::sin(dmath::PI * phase.rem_euclid(1.0)),
                );
                let facing = {
                    let a = match &rw {
                        Some(r) if !r[2].is_nan() => unit(r[2]),
                        _ => [f.fs[o].p[0] - f.fs[k].p[0], f.fs[o].p[1] - f.fs[k].p[1]],
                    };
                    let b = f.fs[k].p;
                    let dx =
                        bold::at(cam, b[0] + a[0] * 0.2, b[1] + a[1] * 0.2, 0.0).0 - bold::at(cam, b[0], b[1], 0.0).0;
                    if dx < 0.0 { -1.0 } else { 1.0 }
                };
                let pop = bold::since(p, k, &["ability:haste"], t, POP_S)
                    .map_or(0.0, |(age, _)| 1.0 - bold::ease_out(age / POP_S));
                let s = d.rp / 20.0 * (1.0 + 0.9 * pop);
                for (i, side) in [-1.0, 1.0].into_iter().enumerate() {
                    let lift = if stepping && foot == i as f64 { up } else { 0.0 };
                    let c = (d.s.0 + side * d.rp * 0.42, d.s.1 + d.rp * 1.42 - lift * d.rp * 0.3);
                    boot(img, c, s, facing);
                }
            }
            // the bell glints before a blast and before HORN FLASH
            let glint = bold::until(p, k, &["horn_flash"], t, 0.35)
                .map(|(u, _)| (u, 1.0))
                .or_else(|| bold::until(p, k, &["blast"], t, 0.25).map(|(u, _)| (u, 0.7)));
            if let Some((u, size)) = glint {
                let c = (d.s.0 + d.rp * 0.3, d.s.1 - d.rp * 0.55);
                px::glint(img, c, (3.0 + 9.0 * u) * size * d.rp / 20.0, bold::WHITE, 1.0);
            }
        }
        if let (Some(d), Some(m)) = (foe, me) {
            // HORN FLASH catches it: a gold ring bursts off the enemy and it is sent away
            if let Some((age, _)) = bold::since(p, k, &["flash_hit"], t, 0.3) {
                let u = age / 0.3;
                px::ink_ring(img, d.s, d.rp * (1.1 + 0.9 * bold::ease_out(u)), 2.2, GOLD, 1.0 - u);
                px::speed_lines(
                    img,
                    d.s,
                    d.rp,
                    (d.s.0 - m.s.0, d.s.1 - m.s.1),
                    16.0,
                    5,
                    bold::WHITE,
                    1.0 - u,
                );
            }
        }
        // COMPANY FRONT
        let Some(rw) = rw else { return };
        if rw[9].is_nan() {
            return;
        }
        let (ax, sg, pos) = (rw[10] as i64, rw[11], rw[9]);
        let form = (rw[15] * 8.0).min(1.0);
        if form < 1.0 {
            // each sousaphone lands on the line with a pop of chalk
            for j in 0..GHOSTS {
                let uj = (form * 1.25 - (j as f64 + 0.5) / GHOSTS as f64) / 0.2;
                if !(0.6..2.0).contains(&uj) {
                    continue;
                }
                let u = (uj - 0.6) / 1.4;
                let v = across(j);
                let (x, y) = if ax == 0 { (pos, v) } else { (v, pos) };
                let c = bold::at(cam, x, y, 0.05);
                px::ink_star(img, c, 5.0 + 9.0 * bold::ease_out(u), 3.0, 6, 0.4, CHALK, 1.0 - u);
            }
        }
        // the trampled enemy is sent on ahead of the wall
        if let (Some(d), Some((age, _))) = (foe, bold::since(p, k, &["front_hit"], t, 0.6)) {
            let e = f.fs[o].p;
            let ahead = if ax == 0 { [sg, 0.0] } else { [0.0, sg] };
            let (a, b) = (
                bold::at(cam, e[0], e[1], R),
                bold::at(cam, e[0] + ahead[0], e[1] + ahead[1], R),
            );
            px::speed_lines(
                img,
                d.s,
                d.rp,
                (b.0 - a.0, b.1 - a.1),
                22.0,
                7,
                bold::WHITE,
                1.0 - age / 0.6,
            );
        }
    }
}
