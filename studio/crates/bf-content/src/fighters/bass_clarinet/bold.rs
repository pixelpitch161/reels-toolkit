//! BASS CLARINET with the bold effects (`picture.effects = "bold"`; the owner's picks 17a to 17d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **ENDPIN ANCHOR** (17a): a landing mark closes in where the peg will go; it plants with a crack (chips, a
//!   quake ring) and the track the ball will sweep shows on the floor; the ball whirls on a thick silver rod with
//!   a smear behind it; on contact a white print of the rod and ball holds where it struck for three frames, and
//!   the enemy flies off along the swing with speed lines.
//! - **SLAP TONGUE** (17b): a big flat cane paddle slides out and winds back with a glint, slaps through a white
//!   swoosh and holds at full reach; the enemy spins off with speed lines and spin marks.
//! - **MULTIPHONICS** (17c): a see-through blue twin: an echo that trails the ball between whirls, and in a whirl
//!   a ball of its own on a blue rod on the far side of the peg, with its own smear; its hit pops blue.
//! - **LIGHTS OUT** (17d): the arena goes almost black (only the enemy stays lit); of the bass clarinet only two
//!   eyes show, over a shadow puddle whose ripples slide under the enemy; it erupts in a crown of spikes and a
//!   quake ring.

use bf_core::dmath;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px::{self, Surface};
use bf_picture::bold::{self};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::bayer_full;
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Pic};
use bf_picture::ring::PictureCtx;
use bf_picture::scene::R_BALL as R;

use super::{BLACK, BassClarinetLook, NICKEL, Whirl, sub, whirl_light};
use crate::fighters::shared::reeds::{CANE, CANE_D};

/// The twin's see-through blue.
const TWIN: V3 = [96.0, 176.0, 255.0];
/// The body of a shadow spike.
const SHADOW: V3 = [30.0, 24.0, 50.0];
/// The eyes in the dark.
const EYE: V3 = [214.0, 236.0, 255.0];
/// Seconds the paddle winds back before the reed opens.
const WIND_S: f64 = 0.3;
/// Radians the paddle is wound back.
const BACK: f64 = 1.9;
/// Seconds the paddle stays out after the slap (held at full reach first).
const SLAP_S: f64 = 0.34;
/// Seconds an enemy sent flying trails speed lines.
const FLY_S: f64 = 0.36;
/// Seconds the print of a smash holds (three frames).
const HOLD_S: f64 = 0.1;
/// Seconds the paddle holds at full reach after the slap.
const PADDLE_HOLD_S: f64 = 0.14;
/// Seconds the spikes of the eruption stand.
const SPIKE_S: f64 = 0.55;
/// How dark the arena goes at most.
const DARK: f64 = 0.86;

/// A point of the circle of radius `r` round `c` at angle `a`.
fn on_circle(c: [f64; 2], r: f64, a: f64) -> [f64; 2] {
    [c[0] + r * dmath::cos(a), c[1] + r * dmath::sin(a)]
}

/// The smear behind a whirling ball at angle `th` of the whirl `w`: a ribbon along its path that thins out
/// backward, in three flat steps.
fn smear<S: Surface + ?Sized>(s: &mut S, cam: &Cam, w: &Whirl, th: f64, col: V3, a: f64) {
    let sgn = if w.om >= 0.0 { 1.0 } else { -1.0 };
    let len = (w.om.abs() * 0.13 * (w.age / 0.15).min(1.0)).min(1.7);
    if len < 0.15 || w.r < 0.2 || a <= 0.0 {
        return;
    }
    // it starts at the ball's own rim
    let d0 = (R * 0.9 / w.r).min(1.2);
    for (band, alpha) in [(2.0, 0.22), (1.0, 0.42), (0.0, 0.66)] {
        let (u0, u1) = (band / 3.0, (band + 1.0) / 3.0);
        let mut outer = Vec::new();
        let mut inner = Vec::new();
        for i in 0..=5 {
            let u = u0 + (u1 - u0) * f64::from(i) / 5.0;
            let ang = th - sgn * (d0 + len * u);
            let hw = R * 0.8 * (1.0 - u) + 0.03;
            let (po, pi) = (on_circle(w.c, w.r + hw, ang), on_circle(w.c, (w.r - hw).max(0.0), ang));
            outer.push(bold::at(cam, po[0], po[1], R));
            inner.push(bold::at(cam, pi[0], pi[1], R));
        }
        inner.reverse();
        outer.extend(inner);
        px::poly(s, &outer, col, alpha * a);
    }
}

/// The endpin: a thick rod from the rim of the ball at `pb` down to the peg at `c`. `tone` is `None` for the
/// silver rod with its black collar, or a colour and strength for a flat rod (the twin's, a held print).
fn draw_rod<S: Surface + ?Sized>(s: &mut S, cam: &Cam, c: [f64; 2], pb: [f64; 2], tone: Option<(V3, f64)>) {
    let d = [c[0] - pb[0], c[1] - pb[1]];
    let l = dmath::hypot(d[0], d[1]).max(1e-6);
    let a = [pb[0] + d[0] / l * R * 0.62, pb[1] + d[1] / l * R * 0.62];
    let (top, tip) = (bold::at(cam, a[0], a[1], R * 0.55), bold::at(cam, c[0], c[1], 0.02));
    let wpx = (0.11 * bold::px_per_unit(cam, pb[0], pb[1], R)).max(4.0);
    if let Some((col, k)) = tone {
        px::line(s, top, tip, wpx + 2.0, bold::deep(col), 0.6 * k);
        px::line(s, top, tip, wpx - 1.0, bold::pale(col), 0.75 * k);
        return;
    }
    px::ink_line(s, top, tip, wpx, NICKEL, 1.0);
    px::line(
        s,
        (top.0, top.1 - wpx * 0.25),
        (tip.0, tip.1 - wpx * 0.25),
        1.2,
        bold::WHITE,
        0.9,
    );
    // the black rubber collar and the bright point where it bites
    let col_at = (top.0 + (tip.0 - top.0) * 0.68, top.1 + (tip.1 - top.1) * 0.68);
    px::ink_line(s, col_at, tip, wpx + 1.5, BLACK, 1.0);
    px::disc(s, tip, 2.2, bold::WHITE, 1.0);
}

/// The twin's ball: a see-through blue disc in a bright rim, at the screen point `c` with radius `rp`. Pixels
/// inside `avoid` (the real ball's disc) are left alone, so an echo sits behind its ball.
fn ghost_ball<S: Surface + ?Sized>(
    s: &mut S,
    c: (f64, f64),
    rp: f64,
    a: f64,
    fill: f64,
    avoid: Option<((f64, f64), f64)>,
) {
    if a <= 0.0 || !rp.is_finite() || rp < 1.0 {
        return;
    }
    let (x0, x1) = ((c.0 - rp - 3.0).floor() as i32, (c.0 + rp + 3.0).ceil() as i32);
    let (y0, y1) = ((c.1 - rp - 3.0).floor() as i32, (c.1 + rp + 3.0).ceil() as i32);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            if let Some((q, rq)) = avoid
                && dmath::hypot(fx - q.0, fy - q.1) < rq
            {
                continue;
            }
            let d = dmath::hypot(fx - c.0, fy - c.1);
            if d > rp + 2.2 {
                continue;
            }
            if d > rp + 1.0 {
                s.mix(x, y, bold::INK, 0.7 * a);
            } else if d > rp - 1.8 {
                s.mix(x, y, bold::pale(TWIN), 0.95 * a);
            } else {
                // a lighter crescent up and to the left, flat
                let lit = dmath::hypot(fx - c.0 + rp * 0.3, fy - c.1 + rp * 0.3) < rp * 0.55;
                s.mix(x, y, if lit { bold::pale(TWIN) } else { TWIN }, fill * a);
            }
        }
    }
}

/// SLAP TONGUE's paddle at a frame.
struct Paddle {
    /// Where it points (radians in the ring frame), wound back or through.
    ang: f64,
    /// Where it is aimed (the slap ends here).
    aim: f64,
    /// Its reach beyond the ball's surface.
    ext: f64,
    /// 0 to 1: the wind-up's glint.
    glint: f64,
    /// Seconds since the slap (negative before it).
    since: f64,
}

/// The corners of the paddle of ball `pb` in the ring frame: a narrow neck that widens into a broad flat blade.
fn paddle_points(pb: [f64; 2], ang: f64, ext: f64) -> Vec<[f64; 2]> {
    let (d, n) = ([dmath::cos(ang), dmath::sin(ang)], [-dmath::sin(ang), dmath::cos(ang)]);
    let at = |along: f64, across: f64| {
        [
            pb[0] + d[0] * along + n[0] * across,
            pb[1] + d[1] * along + n[1] * across,
        ]
    };
    let hw = 0.36;
    vec![
        at(R * 0.75, 0.07),
        at(R + ext * 0.16, 0.08),
        at(R + ext * 0.34, hw),
        at(R + ext * 0.9, hw),
        at(R + ext, hw * 0.6),
        at(R + ext, -hw * 0.6),
        at(R + ext * 0.9, -hw),
        at(R + ext * 0.34, -hw),
        at(R + ext * 0.16, -0.08),
        at(R * 0.75, -0.07),
    ]
}

fn draw_paddle(s: &mut Img<f32>, cam: &Cam, pb: [f64; 2], pd: &Paddle) {
    let scr = |q: [f64; 2]| bold::at(cam, q[0], q[1], R);
    let (d, n) = (
        [dmath::cos(pd.ang), dmath::sin(pd.ang)],
        [-dmath::sin(pd.ang), dmath::cos(pd.ang)],
    );
    let on = |along: f64, across: f64| {
        scr([
            pb[0] + d[0] * along + n[0] * across,
            pb[1] + d[1] * along + n[1] * across,
        ])
    };
    // the slap's swoosh: a white fan over the arc it swung through
    if (0.0..0.14).contains(&pd.since) {
        let u = pd.since / 0.14;
        let mut fan = Vec::new();
        for i in 0..=8 {
            let a = pd.aim - BACK * (1.0 - u * 0.5) * (1.0 - f64::from(i) / 8.0);
            fan.push(scr(on_circle(pb, R + pd.ext + 0.08, a)));
        }
        for i in (0..=8).rev() {
            let a = pd.aim - BACK * (1.0 - u * 0.5) * (1.0 - f64::from(i) / 8.0);
            fan.push(scr(on_circle(pb, R + pd.ext * 0.45, a)));
        }
        px::poly(s, &fan, bold::WHITE, 0.55 * (1.0 - u));
    }
    let pts: Vec<(f64, f64)> = paddle_points(pb, pd.ang, pd.ext).into_iter().map(scr).collect();
    px::ink_poly(s, &pts, CANE, 1.0);
    // the reed's grain and its thin pale tip
    px::line(
        s,
        on(R + pd.ext * 0.36, 0.0),
        on(R + pd.ext * 0.86, 0.0),
        1.5,
        CANE_D,
        1.0,
    );
    px::line(
        s,
        on(R + pd.ext * 0.93, 0.27),
        on(R + pd.ext * 0.93, -0.27),
        2.0,
        bold::WHITE,
        0.9,
    );
    // the black mouthpiece it sits on
    px::ink_line(s, on(R * 0.75, 0.0), on(R + pd.ext * 0.2, 0.0), 6.0, BLACK, 1.0);
    if (0.0..HOLD_S).contains(&pd.since) {
        px::poly(s, &pts, bold::WHITE, 0.75 * (1.0 - pd.since / HOLD_S));
    }
    if pd.glint > 0.0 {
        px::glint(s, on(R + pd.ext * 0.9, 0.0), 5.0 + 9.0 * pd.glint, bold::WHITE, 1.0);
    }
}

impl BassClarinetLook {
    /// SLAP TONGUE's paddle at `t`: from its wind-up, a little before the reed opens, until it is back in.
    fn paddle(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Option<Paddle> {
        let (k, t) = (self.k, f.t);
        let reach = self.slap_reach?;
        let &(t0, t_snap, aim0) = self
            .snip_open
            .iter()
            .find(|s| s.0 - WIND_S <= t && t < s.1 + SLAP_S && s.0 < p.t_ko)?;
        let (me, en) = (f.fs[k].p, f.fs[1 - k].p);
        let aim = if t < t0 {
            // not open yet: it faces the enemy
            dmath::atan2(en[1] - me[1], en[0] - me[0])
        } else {
            match p.run.row("SNIP", p.sim_i(t.min(p.t_ko)), k) {
                Some(row) if !row[1].is_nan() => row[1],
                _ => aim0,
            }
        };
        let since = t - t_snap;
        Some(if t < t0 {
            let u = bold::ease_out((t - (t0 - WIND_S)) / WIND_S);
            Paddle {
                ang: aim - BACK * u,
                aim,
                ext: 0.15 + 0.6 * u,
                glint: 0.0,
                since,
            }
        } else if since < 0.0 {
            let u = (t - t0) / (t_snap - t0).max(1e-6);
            Paddle {
                ang: aim - BACK - 0.3 * u,
                aim,
                ext: 0.75 + 0.1 * u,
                glint: u,
                since,
            }
        } else {
            // through in a frame, held at full reach, then back in
            let back = bold::ease_in((since - PADDLE_HOLD_S) / (SLAP_S - PADDLE_HOLD_S));
            Paddle {
                ang: aim - (BACK + 0.3) * (1.0 - since / 0.03).max(0.0),
                aim,
                ext: (reach + 0.15) * (1.0 - 0.8 * back),
                glint: 0.0,
                since,
            }
        })
    }

    /// The orbit the whirl at `t` eases to (the track on the floor), or the whirl's radius without a peg event.
    fn orbit(&self, w: &Whirl, t: f64) -> f64 {
        self.anch.iter().rev().find(|a| a.t <= t).map_or(w.r, |a| a.orbit)
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let k = self.k;
        let tint = bold::light(p.col[k]);
        // where the peg will go: a small landing mark closes in
        if let Some((u, e)) = bold::until(p, k, &["anchor"], c.t, 0.22) {
            floor::target(ic, c, e.f(3), e.f(4), 0.2, u, tint);
        }
        if let Some(w) = Whirl::of(&c.fs[k]) {
            // the track it will sweep: a faint lane between two bright edges
            let orbit = self.orbit(&w, c.t);
            let show = (w.age / 0.08).min(1.0);
            floor::ring(ic, c, w.c[0], w.c[1], orbit, R, tint, 0.2 * show);
            floor::ring(ic, c, w.c[0], w.c[1], orbit + R, 0.045, tint, 0.9 * show);
            if orbit > R + 0.15 {
                floor::ring(ic, c, w.c[0], w.c[1], orbit - R, 0.045, tint, 0.7 * show);
            }
            // the swept trail behind the arm (and the twin's, in its blue)
            let bayer = bayer_full();
            let blue = bold::light(TWIN);
            for (j, light) in ic.iter_mut().enumerate() {
                let (sx, sz) = (c.sxp[j], c.szp[j]);
                let dither = bayer.px(c.xs[j], c.ys[j])[0] * 0.9;
                let v = whirl_light(sx, sz, w.c, w.r, w.th, w.om, w.age);
                let v = (v * 4.0 + dither).floor() / 4.0;
                for ch in 0..3 {
                    light[ch] += v * 1.25 * tint[ch];
                }
                if w.tw > 0.5 {
                    let v = whirl_light(sx, sz, w.c, w.r, w.th + dmath::PI, w.om, w.age);
                    let v = (v * 4.0 + dither).floor() / 4.0;
                    for ch in 0..3 {
                        light[ch] += v * 0.8 * blue[ch];
                    }
                }
            }
            // the peg goes in: a quake ring races out from it
            if w.age < 0.32 {
                let u = w.age / 0.32;
                floor::band(
                    ic,
                    c,
                    Family::Quake,
                    w.c[0],
                    w.c[1],
                    0.2 + 1.3 * bold::ease_out(u),
                    1.0 - u,
                    c.t,
                    tint,
                    0.9,
                );
                floor::disc(ic, c, w.c[0], w.c[1], 0.3, tint, 1.2 * (1.0 - u));
            }
        }
        // LIGHTS OUT's eruption: a hot centre and a quake ring out to its radius
        for &(t, at) in &self.erupts {
            let age = c.t - t;
            if (0.0..0.5).contains(&age) {
                let radius = self.low_radius.unwrap_or(1.5);
                let u = age / 0.5;
                floor::disc(ic, c, at[0], at[1], 0.7, tint, 1.3 * (1.0 - age / 0.25).max(0.0));
                floor::band(
                    ic,
                    c,
                    Family::Quake,
                    at[0],
                    at[1],
                    radius * bold::ease_out(age / 0.22),
                    1.0 - u,
                    c.t,
                    tint,
                    1.3,
                );
            }
        }
    }

    pub(super) fn bold_canvas_after(&self, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let st = &c.fs[self.k];
        if let Some(w) = Whirl::of(st) {
            floor::shade(ic, c, w.c[0], w.c[1], 0.17, 0.75);
        }
        // the shadow puddle it slides in: wider and darker than the old pool
        if st.sink > 0.02 {
            floor::shade(ic, c, st.p[0], st.p[1], R * 2.1, 0.94 * st.sink);
        }
    }

    pub(super) fn bold_over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        if !f.in_fight {
            return;
        }
        let (k, cam) = (self.k, f.cam);
        let st = &f.fs[k];
        if !st.vis {
            return;
        }
        let col = p.col[k];
        let twin = st.extra.get("twin").copied().unwrap_or(0.0);
        let screen = |q: [f64; 2]| (bold::at(cam, q[0], q[1], R), R * bold::px_per_unit(cam, q[0], q[1], R));
        let whirl = Whirl::of(st);
        if let Some(w) = whirl {
            smear(arr, cam, &w, w.th, bold::pale(bold::pale(col)), 1.0);
            if w.tw > 0.5 {
                // MULTIPHONICS: the twin on the far side of the peg, on its own rod
                let q = on_circle(w.c, w.r, w.th + dmath::PI);
                // solid for a moment as its own hit lands
                let struck = bold::events_in(p, k, &["hit:anchor"], f.t - 0.15, f.t).any(|e| sub(e) == Some(1));
                smear(arr, cam, &w, w.th + dmath::PI, TWIN, 0.8 * twin);
                draw_rod(arr, cam, w.c, q, Some((TWIN, twin)));
                let (s, rp) = screen(q);
                ghost_ball(arr, s, rp, twin, if struck { 0.85 } else { 0.42 }, None);
            }
            draw_rod(arr, cam, w.c, st.p, None);
            // the crack as the peg goes in
            let peg = bold::at(cam, w.c[0], w.c[1], 0.03);
            px::chips(
                arr,
                peg,
                w.age,
                0.3,
                9,
                26.0,
                (w.c[0] * 977.0).abs() as u64 + 3,
                bold::pale(col),
            );
            if w.age < 0.1 {
                px::ink_star(arr, peg, 7.0 + 60.0 * w.age, 3.0, 6, 0.4, bold::WHITE, 1.0);
            }
        }
        if twin > 0.0 && whirl.is_none_or(|w| w.tw <= 0.5) {
            // between whirls the twin is an echo trailing the ball
            let was = p.run.p(p.sim_i(f.t).saturating_sub(5), k);
            let ((s, rp), (s0, rp0)) = (screen(was), screen(st.p));
            ghost_ball(arr, s, rp, twin, 0.42, Some((s0, rp0 + 1.0)));
        }
        // a smash holds: a white print of the rod and the ball where it struck, for three frames
        for e in bold::events_in(p, k, &["hit:anchor"], f.t - HOLD_S, f.t) {
            let Some(w) = self.anc_at(p, e.t) else {
                continue;
            };
            let far = if sub(e) == Some(1) { dmath::PI } else { 0.0 };
            let q = on_circle(w.c, w.r, w.th + far);
            let tone = if far > 0.0 { TWIN } else { bold::WHITE };
            draw_rod(arr, cam, w.c, q, Some((tone, 1.0)));
            let (s, rp) = screen(q);
            px::disc(arr, s, rp, bold::pale(tone), 0.45);
            px::ink_ring(arr, s, rp, 2.5, bold::pale(tone), 1.0);
            // the twin's own hit pops in its blue
            if far > 0.0 {
                let u = (f.t - e.t) / HOLD_S;
                let hit = bold::at(cam, e.f(5), e.f(6), R);
                px::ink_ring(arr, hit, 8.0 + 22.0 * u, 2.5, TWIN, 1.0 - 0.5 * u);
                px::chips(arr, hit, f.t - e.t, 0.3, 7, 30.0, (e.t * 1000.0) as u64, TWIN);
            }
        }
        if let Some(pd) = self.paddle(p, f) {
            draw_paddle(arr, cam, st.p, &pd);
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let (k, o, t, cam) = (self.k, 1 - self.k, f.t, f.cam);
        let col = p.col[k];
        let enemy = drawn.iter().find(|d| d.k == o);
        // LIGHTS OUT: the arena goes almost black; only the enemy stays lit
        let d = self.dim_at(p.t_ko, t) / 0.40;
        if d > 0.0 {
            let bayer = bayer_full();
            let (c, r, soft) = enemy.map_or(((-1e5, -1e5), 0.0, 0.0), |b| (b.s, b.rp * 1.7, b.rp * 1.5));
            px::dim_outside(img, c, r, soft, 1.0 - DARK * d, &|x, y| bayer.px(x, y)[0]);
        }
        if !f.in_fight {
            return;
        }
        let st = &f.fs[k];
        if st.sink > 0.02 && self.erupts.iter().all(|e| !(e.0..e.0 + 0.2).contains(&t)) {
            let a = st.sink;
            // the puddle's rim and its ripples, sliding under the enemy
            let hidden = |s: (f64, f64)| enemy.is_some_and(|b| dmath::hypot(s.0 - b.s.0, s.1 - b.s.1) < b.rp);
            let mut circle = |r: f64, w: f64, tone: V3, alpha: f64| {
                let mut run: Vec<(f64, f64)> = Vec::new();
                for i in 0..=32 {
                    let q = on_circle(st.p, r, dmath::TAU * f64::from(i) / 32.0);
                    let s = bold::at(cam, q[0], q[1], 0.01);
                    if hidden(s) {
                        px::polyline(img, &run, w, tone, alpha);
                        run.clear();
                    } else {
                        run.push(s);
                    }
                }
                px::polyline(img, &run, w, tone, alpha);
            };
            circle(R * 1.25, 2.0, bold::pale(col), a);
            for j in 0..3 {
                let ph = (t * 1.3 + f64::from(j) / 3.0).rem_euclid(1.0);
                circle(R * (1.25 + 1.5 * ph), 1.5, col, a * (1.0 - ph));
            }
            // only the eyes show, turned to the enemy
            if let Some(me) = drawn.iter().find(|b| b.k == k) {
                let rp = R * bold::px_per_unit(cam, st.p[0], st.p[1], R * 0.4);
                let look = enemy.map_or((0.0, 0.0), |b| {
                    let (dx, dy) = (b.s.0 - me.s.0, b.s.1 - me.s.1);
                    let n = dmath::hypot(dx, dy).max(1.0);
                    (dx / n * rp * 0.12, dy / n * rp * 0.08)
                });
                let c = (me.s.0 + look.0, me.s.1 - rp * 0.1 + look.1);
                for side in [-1.0, 1.0] {
                    // a slanted slit: high outside, low toward the nose
                    let eye = [
                        (c.0 + side * rp * 0.62, c.1 - rp * 0.3),
                        (c.0 + side * rp * 0.14, c.1 + rp * 0.02),
                        (c.0 + side * rp * 0.2, c.1 + rp * 0.2),
                        (c.0 + side * rp * 0.58, c.1 + rp * 0.02),
                    ];
                    px::disc(img, (c.0 + side * rp * 0.38, c.1), rp * 0.42, col, 0.3 * a);
                    px::ink_poly(img, &eye, EYE, a);
                }
            }
        }
        // the eruption: a crown of spikes out of the floor
        for &(t0, at) in &self.erupts {
            let age = t - t0;
            if !(0.0..SPIKE_S).contains(&age) || t0 > p.t_ko {
                continue;
            }
            let radius = self.low_radius.unwrap_or(1.5);
            let seed = (t0 * 1000.0) as u64;
            // (base, height, half-width in units, delay): the tall one in the middle, the rest in two rows
            let mut spikes: Vec<([f64; 2], f64, f64, f64)> = vec![(at, 2.3, 0.34, 0.0)];
            for j in 0..12u64 {
                let row = if j % 2 == 0 { 0.45 } else { 0.85 };
                let rho = radius * (row + 0.12 * px::rnd(seed, j));
                let ang = dmath::TAU * (j as f64 + 0.6 * px::rnd(seed, j + 20)) / 12.0;
                let h = (1.75 - 0.7 * row) * (0.75 + 0.5 * px::rnd(seed, j + 40));
                spikes.push((on_circle(at, rho, ang), h, 0.24, 0.07 * row));
            }
            // far ones first
            spikes.sort_by(|a, b| {
                bold::at(cam, a.0[0], a.0[1], 0.0)
                    .1
                    .total_cmp(&bold::at(cam, b.0[0], b.0[1], 0.0).1)
            });
            for (q, h, hw, delay) in spikes {
                let up = bold::ease_out((age - delay) / 0.07) * (1.0 - bold::ease_in((age - 0.32) / (SPIKE_S - 0.32)));
                if up <= 0.02 {
                    continue;
                }
                // it leans a little away from the middle
                let lean = [(q[0] - at[0]) * 0.18, (q[1] - at[1]) * 0.18];
                let base = bold::at(cam, q[0], q[1], 0.0);
                let tip = bold::at(cam, q[0] + lean[0] * up, q[1] + lean[1] * up, h * up);
                let wpx = hw * bold::px_per_unit(cam, q[0], q[1], 0.0);
                let (l, r) = ((base.0 - wpx, base.1), (base.0 + wpx, base.1));
                px::poly(img, &[l, r, tip], SHADOW, 1.0);
                // the lit facet and a pale edge all round
                px::poly(img, &[l, (base.0 - wpx * 0.25, base.1 + 1.0), tip], col, 0.9);
                px::polyline(img, &[l, tip, r, l], 1.6, bold::pale(col), 1.0);
            }
            let s = bold::at(cam, at[0], at[1], 0.1);
            px::chips(img, s, age, 0.45, 14, 70.0, seed, bold::pale(col));
        }
        // sent flying: speed lines behind the enemy, and spin marks after the slap
        if let Some((age, e)) = bold::since(p, k, &["hit:anchor", "hit:snip", "hit:erupt"], t, FLY_S)
            && let Some(b) = enemy
        {
            let i = p.sim_i(t);
            let (now, was) = (p.run.p(i, o), p.run.p(i.saturating_sub(2), o));
            let (s1, s0) = (bold::at(cam, now[0], now[1], R), bold::at(cam, was[0], was[1], R));
            let dir = (s1.0 - s0.0, s1.1 - s0.1);
            let u = age / FLY_S;
            if dmath::hypot(dir.0, dir.1) > 1.5 {
                px::speed_lines(img, b.s, b.rp, dir, 34.0 * (1.0 - 0.5 * u), 6, bold::WHITE, 1.0 - u);
            }
            if bold::how(e) == "snip" {
                for n in 0..2 {
                    let a0 = t * 24.0 + dmath::PI * f64::from(n);
                    px::ink_arc(img, b.s, b.rp * 1.25, 2.0, a0, a0 + 1.5, bold::pale(CANE), 1.0 - u);
                }
            }
        }
    }
}
