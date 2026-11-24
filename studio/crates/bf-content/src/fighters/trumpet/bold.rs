//! TRUMPET with the bold effects (`picture.effects = "bold"`; the owner's picks 11a to 11d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **VALVE COMBO** (11a): three valve caps stand on the ball and press down one, two, three with the punches;
//!   the pistons are chunky (a brass collar, a thick silver rod, a ringed pearl button) and each stops where it
//!   landed; the third is all gold, bigger, and lands as a heavy hit.
//! - **FANFARE** (11b): a glint on the bell just before, then the ring rolls out as stacked brass bands with
//!   little pennants riding on it; the enemy it hits trails push lines as it is thrown back.
//! - **DOUBLE TONGUE** (11c): twin arrows flicker round the ball as it starts, and every piston of the faster
//!   combo leaves an afterimage (the ring's own speed-up look does the rest).
//! - **HIGH C** (11d): while it charges a lane on the floor shows the aim, rings close in on the ball and the
//!   bell glints; the blast is a thick flat gold beam with a white core and sparkles, which thins to a line as
//!   it fades.

use bf_core::dmath;
use bf_core::event::Event;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::V3;
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::Wave;
use bf_picture::scene::{self, R_BALL as R};

use super::{
    BANNER, BANNER_TRIM, BELL_IN, COLLAR, P2, PEARL, PISTON, TrumpetLook, beam_outline, bell_mouth, clip_len,
    draw_note_rise, fire_centre, int, piston_ext, piston_geom, search_left, search_right, unit, world,
};

/// The third piston's gold, and the beam's body.
const HOT: V3 = [255.0, 200.0, 48.0];
/// The beam's edge, a deeper gold.
const AMBER: V3 = [236.0, 140.0, 24.0];
/// Seconds a DOUBLE TONGUE piston's afterimage stays.
const GHOST_S: f64 = 0.14;
/// Seconds the twin arrows flicker as DOUBLE TONGUE starts.
const ARROWS_S: f64 = 0.7;
/// Seconds the push lines trail the enemy FANFARE hit.
const PUSH_S: f64 = 0.35;
/// Seconds before FANFARE the bell glints.
const GLINT_S: f64 = 0.25;
/// Pennants on FANFARE's ring.
const FLAGS: usize = 8;

/// A piston of the bold look.
#[derive(Clone, Copy, Debug)]
struct BPiston {
    /// Its aim.
    a: f64,
    /// Its place across the aim.
    off: f64,
    /// Its length beyond the glass.
    ext: f64,
    /// The last punch of its combo: the gold one.
    last: bool,
    /// An afterimage (DOUBLE TONGUE), not the piston itself.
    ghost: bool,
}

/// One painter item of the bold look.
enum BItem {
    /// A piston out of the ball.
    Piston(BPiston),
    /// A pennant on FANFARE's ring: its foot, the way it streams, its flutter and its size (0 to 1).
    Flag(P2, f64, f64, f64),
}

/// A flat disc in the ring's plane, at height `y`.
fn ngon(c: P2, y: f64, r: f64, n: usize) -> Vec<V3> {
    (0..n)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / n as f64;
            scene::to_world(c[0] + r * dmath::cos(a), c[1] + r * dmath::sin(a), y)
        })
        .collect()
}

/// A steady 0 to 1 from a whole number (where a sparkle sits).
fn scatter(n: usize, salt: f64) -> f64 {
    let v = dmath::sin(n as f64 * 12.9898 + salt * 78.233) * 43_758.545_3;
    v - v.floor()
}

impl TrumpetLook {
    /// The latest `ability` event of kind `kind` (its element 4) by this side from `after` seconds before `t` to
    /// `before` seconds after it: the seconds from it to `t` (negative while it is still to come).
    fn ability_near(&self, p: &PictureCtx, kind: &str, t: f64, before: f64, after: f64) -> Option<f64> {
        let i0 = p.ev_t.partition_point(|&te| te < t - after);
        let i1 = p.ev_t.partition_point(|&te| te <= t + before);
        p.rec.events[i0..i1.max(i0)]
            .iter()
            .rev()
            .find(|e| e.kind == "ability" && e.who() == Some(self.k) && e.s(4) == kind)
            .map(|e| t - e.t)
    }

    /// The charge of HIGH C open at `t`: its start, aim, length in seconds, the blast's length and width.
    fn bold_charge(&self, p: &PictureCtx, t: f64) -> Option<(f64, f64, f64, f64, f64)> {
        if !(p.show.grid.t_fight <= t && t < p.t_ko) {
            return None;
        }
        let (_, e) = bold::since(p, self.k, &["charge"], t, 4.0)?;
        (t < e.t + e.f(4)).then(|| (e.t, e.f(3), e.f(4), e.f(5), e.f(6)))
    }

    /// The pistons out at `t`, each held where it touched (S1) and never further out than where it landed, and
    /// the afterimages of DOUBLE TONGUE's.
    fn bold_pistons(&self, p: &PictureCtx, t: f64) -> Vec<BPiston> {
        let Some(w) = &self.w else { return Vec::new() };
        if self.punch_t.is_empty() || t < p.show.grid.t_fight {
            return Vec::new();
        }
        let tt = bold::hold_time(p, self.k, &["piston"], t.min(p.t_ko));
        let (j0, j1) = (search_left(&self.punch_t, tt - 0.8), search_right(&self.punch_t, tt));
        let total = w.t_out + w.t_hold + w.t_back;
        let mut out = Vec::new();
        for q in self.punches.get(j0..j1).unwrap_or(&[]) {
            let u = (tt - q.t0) / q.ks;
            // where it landed, if it did
            let i0 = p.ev_t.partition_point(|&te| te < q.t0);
            let i1 = p
                .ev_t
                .partition_point(|&te| te <= q.t0 + (w.t_out + w.t_hold) * q.ks + 0.02);
            let stop = p.rec.events[i0..i1.max(i0)]
                .iter()
                .find(|e| {
                    e.kind == "hit" && e.who() == Some(self.k) && super::how(e) == "piston" && int(e, 8) == Some(q.i)
                })
                .and_then(|e| piston_ext((e.t - q.t0) / q.ks, w, q.reach))
                .unwrap_or(q.reach);
            let last = q.i == w.punches - 1;
            if let Some(ext) = piston_ext(u, w, q.reach) {
                out.push(BPiston {
                    a: q.a,
                    off: w.off(q.i),
                    ext: ext.min(stop),
                    last,
                    ghost: false,
                });
            } else if q.ks < 1.0 && u < total + GHOST_S / q.ks {
                out.push(BPiston {
                    a: q.a,
                    off: w.off(q.i),
                    ext: stop,
                    last,
                    ghost: true,
                });
            }
        }
        // after the hit-stop they pull home, as in the classic look
        if p.t_ko <= t && t >= p.t_hs {
            let k = (1.0 - (t - p.t_hs) / 0.2).max(0.0);
            out.retain(|q| !q.ghost && k > 0.0);
            for q in &mut out {
                q.ext *= k;
            }
        }
        out
    }

    /// How far valve `i` is pressed at `t`: 1 as its piston fires, back to 0 a moment later.
    fn valve_press(&self, i: i64, t: f64) -> f64 {
        let j1 = search_right(&self.punch_t, t);
        self.punches[j1.saturating_sub(6)..j1]
            .iter()
            .rev()
            .find(|q| q.i == i)
            .map_or(0.0, |q| {
                let age = (t - q.t0) / q.ks;
                if age < 0.1 {
                    1.0
                } else {
                    (1.0 - (age - 0.1) / 0.14).max(0.0)
                }
            })
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let tint = bold::light(p.col[self.k]);
        // each blast throws a hard streak of light on the canvas
        for b in self.beams_live(p, c.t) {
            let d = unit(b.a);
            let bm = bell_mouth(fire_centre(b.ox, b.oz, b.a), b.a);
            let lv = (b.ln + R + 0.04 - BELL_IN * R) * b.ext;
            let lo = lv * b.st.2;
            floor::lane(
                ic,
                c,
                [bm[0] + d[0] * lo, bm[1] + d[1] * lo],
                [bm[0] + d[0] * lv, bm[1] + d[1] * lv],
                0.3 + b.wd * 0.7,
                tint,
                0.9 * b.al,
            );
        }
        // HIGH C charging: the lane the blast will take, dashes running down its middle
        if let Some((t0, aim, dur, ln, wd)) = self.bold_charge(p, c.t) {
            let u = ((c.t - t0) / dur.max(1e-3)).clamp(0.0, 1.0);
            let at = c.fs[self.k].p;
            let d = unit(aim);
            let reach = clip_len(at[0], at[1], aim, ln + R);
            let a = [at[0] + d[0] * R, at[1] + d[1] * R];
            let b = [at[0] + d[0] * reach, at[1] + d[1] * reach];
            floor::lane(ic, c, a, b, wd / 2.0, tint, 0.2 + 0.35 * u);
            floor::dashes(ic, c, a, b, 0.14, 0.45, c.t * 6.0, tint, 0.9 + 0.9 * u);
        }
    }

    pub(super) fn bold_canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind == "pulse" {
            return;
        }
        // FANFARE: stacked brass bands
        floor::wave(ic, c, Family::Brass, w, c.t, bold::light(p.col[w.owner]), 2.0);
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let mut out: Vec<(f64, Payload)> = Vec::new();
        let st = &f.fs[self.k];
        if st.vis && !st.shattered {
            for q in self.bold_pistons(p, f.t) {
                let (a0, tip, _) = piston_geom(st.p, q.a, q.off, q.ext);
                let mid = [(a0[0] + tip[0]) / 2.0, (a0[1] + tip[1]) / 2.0];
                out.push((f.cam.depth(&[world(mid, R)])[0], Box::new(BItem::Piston(q))));
            }
        }
        // FANFARE: pennants ride the leading band
        if f.in_fight {
            for w in p.run.waves(f.i) {
                if w.kind != "shockwave" || w.owner != self.k {
                    continue;
                }
                let size = ((1.0 - w.r / w.rmax) * 1.8).min(1.0);
                if size <= 0.08 {
                    continue;
                }
                for j in 0..FLAGS {
                    let a = std::f64::consts::TAU * j as f64 / FLAGS as f64 + 0.3;
                    let d = unit(a);
                    let foot = [w.cx + d[0] * w.r, w.cz + d[1] * w.r];
                    if foot[0].abs() > 3.15 || foot[1].abs() > 3.15 {
                        continue;
                    }
                    let flutter = dmath::sin(f.t * 34.0 + j as f64 * 1.7);
                    out.push((
                        f.cam.depth(&[world(foot, 0.4)])[0],
                        Box::new(BItem::Flag(foot, a, flutter, size)),
                    ));
                }
            }
        }
        out
    }

    pub(super) fn bold_draw_item(&self, _p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        match payload.downcast_ref::<BItem>() {
            Some(BItem::Piston(q)) => {
                let (a0, tip, d) = piston_geom(f.fs[self.k].p, q.a, q.off, q.ext);
                let (head_r, rod_w) = if q.last { (0.27, 7) } else { (0.19, 5) };
                if q.ghost {
                    // an afterimage: the rod's two edges and the button's rim, in pale lines
                    let n = [-d[1] * 0.07, d[0] * 0.07];
                    let pale = bold::pale(if q.last { HOT } else { PISTON });
                    for s in [-1.0, 1.0] {
                        let edge = [
                            world([a0[0] + n[0] * s, a0[1] + n[1] * s], R),
                            world([tip[0] + n[0] * s, tip[1] + n[1] * s], R),
                        ];
                        fr.line(f.cam, &edge, pale, 1);
                    }
                    let mut rim = ngon(tip, R, head_r, 10);
                    rim.push(rim[0]);
                    fr.line(f.cam, &rim, pale, 1);
                    return;
                }
                bold::outlined_poly(fr, f.cam, &ngon(a0, R, 0.15, 10), COLLAR);
                let rod = [world(a0, R), world(tip, R)];
                bold::thick_line(fr, f.cam, &rod, if q.last { HOT } else { PISTON }, rod_w);
                // the button: a rim and its top, a little further out
                let top = [tip[0] + d[0] * 0.02, tip[1] + d[1] * 0.02];
                if q.last {
                    bold::blob(fr, f.cam, world(tip, R), head_r, HOT);
                    bold::blob(fr, f.cam, world(top, R + 0.01), head_r * 0.5, bold::WHITE);
                } else {
                    bold::blob(fr, f.cam, world(tip, R), head_r, COLLAR);
                    bold::blob(fr, f.cam, world(top, R + 0.01), head_r * 0.62, PEARL);
                }
            }
            Some(&BItem::Flag(foot, a, flutter, size)) => {
                let d = unit(a);
                let h = 0.8 * size;
                let top = world(foot, 0.04 + h);
                bold::thick_line(fr, f.cam, &[world(foot, 0.04), top], [240.0, 230.0, 200.0], 3);
                let out = world(
                    [foot[0] + d[0] * 0.5 * size, foot[1] + d[1] * 0.5 * size],
                    0.04 + h - 0.12 * size + 0.07 * flutter * size,
                );
                let low = world(foot, 0.04 + h - 0.3 * size);
                bold::outlined_poly(fr, f.cam, &[top, out, low], BANNER);
                fr.line(f.cam, &[top, out], BANNER_TRIM, 1);
            }
            None => {}
        }
    }

    /// The blasts (flat: a dark edge, amber, gold, a pale middle; the white core is laid on after the grade), and the champion's solo notes as ever.
    pub(super) fn bold_over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        let (cam, t) = (f.cam, f.t);
        for b in self.beams_live(p, t) {
            let pc = fire_centre(b.ox, b.oz, b.a);
            let (ext, _, tail, age) = b.st;
            let lv = b.ln + R + 0.04 - BELL_IN * R;
            let s1 = lv * ext;
            let s0 = (s1 - 0.05) * tail;
            // it thins to a line as it lets go of the bell, and only then goes out
            let thin = 1.0 - 0.9 * bold::ease_out(tail);
            let alpha = (b.al * 2.5).min(1.0);
            let layers = [
                (1.3 * thin + 0.16, bold::INK),
                (1.3 * thin, AMBER),
                (0.95 * thin, HOT),
                (0.42 * thin, [255.0, 240.0, 150.0]),
            ];
            for (wk, col) in layers {
                if let Some(outline) = beam_outline(pc, b.a, s0, s1, (b.wd * wk).max(0.05), R) {
                    px::poly(arr, &cam.wp(&outline), col, alpha);
                }
            }
            // the muzzle star as it leaves the bell
            let k = 1.0 - age / 0.12;
            if k > 0.0 {
                let s = cam.wp(&[world(bell_mouth(pc, b.a), R)])[0];
                px::ink_star(arr, s, 8.0 + 9.0 * k, 3.5, 8, age * 9.0, bold::WHITE, 1.0);
            }
        }
        if self.k == p.win {
            let st = &f.fs[p.win];
            for (u, m) in self.solo_notes(p, t) {
                let org = bell_mouth(st.p, st.th);
                let side = 0.25 * dmath::sin(f64::from(m) * 1.7);
                draw_note_rise(
                    arr,
                    cam,
                    org[0] + side * u,
                    org[1] - side * u,
                    R + 0.4 + 1.3 * u,
                    p.col[p.win],
                    1.0 - u / 1.4,
                );
            }
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let (k, o, t, cam) = (self.k, 1 - self.k, f.t, f.cam);
        let st = &f.fs[k];
        let me = drawn.iter().find(|d| d.k == k);
        let col = p.col[k];
        // HIGH C: sparkles along the blast, lingering as it thins
        for b in self.beams_live(p, t) {
            let d = unit(b.a);
            let n = [-d[1], d[0]];
            let bm = bell_mouth(fire_centre(b.ox, b.oz, b.a), b.a);
            let lv = (b.ln + R + 0.04 - BELL_IN * R) * b.ext;
            // the white-hot core again on the finished picture, so the low-HP spotlight does not grey it
            let thin = 1.0 - 0.9 * bold::ease_out(b.st.2);
            let pc = fire_centre(b.ox, b.oz, b.a);
            if let Some(core) = beam_outline(pc, b.a, (lv - 0.05) * b.st.2, lv, (b.wd * 0.3 * thin).max(0.04), R) {
                px::poly(img, &cam.wp(&core), bold::WHITE, (b.al * 2.5).min(1.0));
            }
            for j in 0..9 {
                let along = lv * (0.08 + 0.9 * scatter(j, 1.0));
                let side = b.wd * 0.75 * (2.0 * scatter(j, 2.0) - 1.0);
                let twinkle = dmath::sin(t * 26.0 + j as f64 * 2.1);
                if twinkle < -0.2 {
                    continue;
                }
                let q = [bm[0] + d[0] * along + n[0] * side, bm[1] + d[1] * along + n[1] * side];
                let s = bold::at(cam, q[0], q[1], R);
                let tone = if j % 2 == 0 { bold::WHITE } else { bold::pale(HOT) };
                px::glint(img, s, 2.5 + 3.5 * (twinkle + 0.2), tone, (b.al * 2.0).min(1.0));
            }
        }
        if !f.in_fight || !st.vis || st.shattered {
            return;
        }
        let Some(me) = me else { return };
        // VALVE COMBO: the three valve caps on the ball, pressed one, two, three
        if let Some(w) = &self.w {
            for i in 0..w.punches.min(5) {
                let press = self.valve_press(i, t.min(p.t_ko));
                let last = i == w.punches - 1;
                let side = i as f64 - (w.punches - 1) as f64 / 2.0;
                let x = me.s.0 + side * me.rp * 0.46;
                let foot = me.s.1 - me.rp * (0.93 - 0.07 * side * side);
                let cap = foot - 2.0 - me.rp * 0.22 * (1.0 - press);
                px::ink_line(img, (x, foot), (x, cap), 1.6, PISTON, 1.0);
                let tone = if press > 0.5 {
                    bold::WHITE
                } else if last {
                    HOT
                } else {
                    PEARL
                };
                px::ink_disc(
                    img,
                    (x, cap),
                    me.rp * 0.15 + 0.6,
                    if last { AMBER } else { COLLAR },
                    1.0,
                );
                px::disc(img, (x, cap), (me.rp * 0.12).max(1.0), tone, 1.0);
            }
        }
        // FANFARE: a glint on the bell just before it sounds
        if let Some(dt) = self.ability_near(p, "shockwave", t, GLINT_S, 0.0)
            && dt < 0.0
        {
            let u = 1.0 + dt / GLINT_S;
            let bm = bell_mouth(st.p, st.th);
            px::glint(img, bold::at(cam, bm[0], bm[1], R), 6.0 + 14.0 * u, bold::WHITE, 1.0);
        }
        // FANFARE lands: push lines trail the enemy as it is thrown back
        if let Some((age, e)) = bold::since(p, k, &["hit:shockwave"], t, PUSH_S)
            && let Some(them) = drawn.iter().find(|d| d.k == o)
        {
            let from = p.run.p(p.sim_i(e.t), k);
            let a = bold::at(cam, from[0], from[1], R);
            let (dx, dy) = (them.s.0 - a.0, them.s.1 - a.1);
            let len = dmath::hypot(dx, dy).max(1e-6);
            let (ux, uy) = (dx / len, dy / len);
            let fade = 1.0 - age / PUSH_S;
            for side in [-0.62, 0.0, 0.62] {
                let lx = them.s.0 - uy * side * them.rp;
                let ly = them.s.1 + ux * side * them.rp;
                let near = them.rp * (1.25 - 0.3 * f64::abs(side));
                let far = near + them.rp * (0.5 + 1.1 * bold::ease_out(age / 0.1)) * (1.0 - 0.5 * f64::abs(side));
                px::ink_line(
                    img,
                    (lx - ux * near, ly - uy * near),
                    (lx - ux * far, ly - uy * far),
                    2.6,
                    bold::WHITE,
                    fade,
                );
            }
        }
        // DOUBLE TONGUE starts: twin arrows flicker round the ball
        if let Some(age) = self.ability_near(p, "haste", t, 0.0, ARROWS_S) {
            let fade = 1.0 - bold::ease_in(age / ARROWS_S);
            let flick = if f.f.is_multiple_of(2) { 1.0 } else { 0.7 };
            let r = me.rp * (1.35 + 0.35 * bold::ease_out(age / 0.15));
            for turn in [0.0, std::f64::consts::PI] {
                let a = turn + age * 9.0;
                // two chevrons, one behind the other, pointing the way they circle
                for back in [0.0, 0.42] {
                    let at = a - back;
                    let (ca, sa) = (dmath::cos(at), dmath::sin(at));
                    let c = (me.s.0 + r * ca, me.s.1 + r * sa);
                    let (tx, ty) = (-sa, ca);
                    let h = me.rp * 0.36;
                    let pts = [
                        (c.0 - tx * h * 0.5 + ca * h, c.1 - ty * h * 0.5 + sa * h),
                        (c.0 + tx * h * 0.7, c.1 + ty * h * 0.7),
                        (c.0 - tx * h * 0.5 - ca * h, c.1 - ty * h * 0.5 - sa * h),
                    ];
                    px::ink_poly(img, &pts, if back == 0.0 { HOT } else { bold::pale(col) }, fade * flick);
                }
            }
        }
        // HIGH C charging: rings close in on the ball and the bell glints
        if let Some((t0, aim, dur, _, _)) = self.bold_charge(p, t) {
            let u = ((t - t0) / dur.max(1e-3)).clamp(0.0, 1.0);
            for n in 0..2 {
                let ph = (u * 3.0 + n as f64 * 0.5).fract();
                px::ink_ring(
                    img,
                    me.s,
                    me.rp * (2.3 - 1.25 * ph),
                    1.5,
                    bold::pale(col),
                    0.35 + 0.65 * ph,
                );
            }
            let bm = bell_mouth(st.p, aim);
            px::glint(img, bold::at(cam, bm[0], bm[1], R), 3.0 + 11.0 * u, bold::WHITE, 1.0);
        }
    }

    /// The third piston of a combo is the heavy one.
    pub(super) fn bold_hit(&self, e: &Event, by_damage: HitSize) -> Option<HitSize> {
        if e.s(7) == "piston" && self.w.is_some_and(|w| int(e, 8) == Some(w.punches - 1)) {
            return Some(HitSize::Heavy);
        }
        Some(by_damage)
    }
}
