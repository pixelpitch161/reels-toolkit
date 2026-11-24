//! BAGPIPES with the bold effects (`picture.effects = "bold"`; the owner's picks 18a to 18d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **BAG INFLATE** (18a): the bag swells in three clear pumps (each overshoots and settles), the tartan
//!   stretched over the glass from the first one and the three drones sticking further out with each; in the
//!   last pump it shakes and a dashed line on the floor shows the aim. It rockets off wobbling like a jelly,
//!   on a broad ribbon with outlined air puffs and speed lines (a flight hit always pops heavy), and as it lands
//!   the last air leaves it in a puff ring.
//! - **WAR CRY** (18b): a deep breath in (the ball swells, air streaks rush into it), then a broad band of
//!   real tartan rolls out behind a ring of air puffs; the enemy it hits flies back with speed lines.
//! - **HIGHLAND FLING** (18c): two little dancing shoes tap under the ball on the eighths (the name banner and
//!   the draining ring are the ring's own).
//! - **BAG BURST** (18d): the bag swells too far, red stretch marks spreading over it; it pops in a white star
//!   and a rush of air streaks, a wide tartan ring with two rings of puffs rolls out, the bag hangs limp for a
//!   moment and scraps of tartan flutter down.

use bf_core::dmath::{self, PI, TAU};
use bf_core::event::Event;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::V3;
use bf_picture::config;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, GlassPx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::scene::{self, R_BALL as R};

use super::{BagpipesLook, FLY, Item, SWELL, TARTAN_DK, TARTAN_GOLD, TARTAN_LT, TARTAN_RED, how, sc, tartan_rgb};

/// A drone pipe's black wood.
const PIPE: V3 = [38.0, 30.0, 34.0];
/// Its ivory ferrule.
const IVORY: V3 = [240.0, 232.0, 214.0];
/// A puff of air.
const AIR: V3 = [236.0, 246.0, 238.0];
/// Seconds of WAR CRY's breath in.
const BREATH_S: f64 = 0.35;
/// Seconds the bag swells too far before BAG BURST.
const OVER_S: f64 = 0.5;
/// Seconds the scraps of tartan flutter down after the burst.
const SCRAPS_S: f64 = 1.7;
/// Seconds an enemy pushed by a ring trails speed lines.
const PUSH_S: f64 = 0.36;

/// How swollen the bag is `u` of the way through its swell (0 to 1, a little over at each pump's top): three
/// pumps, each rising fast, overshooting and settling.
fn pumps(u: f64) -> f64 {
    let x = u.clamp(0.0, 1.0) * 3.0;
    let n = x.floor().min(2.0);
    let v = x - n;
    let over = 0.3 * dmath::sin(PI * (v / 0.5).min(1.0));
    (n + bold::ease_out(v / 0.3) + over) / 3.0
}

impl BagpipesLook {
    /// The time of this side's BAG BURST, if it has one.
    fn burst_t(p: &PictureCtx, k: usize) -> Option<f64> {
        p.rec
            .events
            .iter()
            .find(|e| e.kind == "ability" && e.who() == Some(k) && e.s(3) == "BAG BURST")
            .map(|e| e.t)
    }

    pub(super) fn bold_state(&self, p: &PictureCtx, st: &mut BallState, t: f64) {
        let k = self.k;
        let tc = t.min(p.t_ko);
        let i = p.sim_i(tc);
        let base = self.rad(i, k) / R;
        st.scale = base;
        let mut tartan = 0.0;
        match self.cycle_at(tc) {
            None => {
                st.extra.insert("phase".into(), 0.0);
            }
            Some((c, ph)) => {
                st.extra.insert("phase".into(), ph);
                let span = (c.s - 1.0).max(1e-6);
                if ph == SWELL {
                    let u = ((tc - c.t0) / (c.t_rel - c.t0).max(1e-6)).clamp(0.0, 1.0);
                    let level = pumps(u);
                    st.scale = 1.0 + span * level;
                    st.extra.insert("infl".into(), level);
                    st.extra.insert("bold_drones".into(), level);
                    tartan = level.max(0.25);
                    // it shakes through the last pump
                    if u > 0.62 {
                        let w = 0.05 * ((u - 0.62) / 0.38) * dmath::sin(tc * TAU * 23.0);
                        st.p = [st.p[0] + w, st.p[1] - 0.6 * w];
                    }
                    st.glow = 0.15 + 0.25 * u;
                } else {
                    let infl = ((base - 1.0) / span).clamp(0.0, 1.0);
                    st.extra.insert("infl".into(), infl);
                    st.extra.insert("bold_drones".into(), infl);
                    tartan = infl;
                    // a released balloon: it wobbles like a jelly as the air leaves it
                    let age = tc - c.t_rel;
                    let wob = 0.13 * (0.35 + 0.65 * infl) * dmath::sin(age * TAU * 8.0);
                    st.sx = Some(1.0 + wob);
                    st.sy = Some(1.0 - wob);
                    st.glow = 0.55 * dmath::exp(-age / 0.25) + 0.15;
                }
            }
        }
        // WAR CRY: a deep breath in
        if let Some((u, _)) = bold::until(p, k, &["ability:shockwave"], tc, BREATH_S) {
            st.scale *= 1.0 + 0.2 * bold::ease_out(u);
            tartan = tartan.max(u);
        }
        // BAG BURST: it swells too far, pops and hangs limp for a moment
        if let Some(tb) = Self::burst_t(p, k) {
            let u = 1.0 - (tb - tc) / OVER_S;
            if (0.0..1.0).contains(&u) {
                st.scale *= 1.0 + 0.55 * bold::ease_in(u);
                tartan = tartan.max(0.4 + 0.6 * u);
                st.extra.insert("bold_over".into(), u);
                let w = 0.045 * u * dmath::sin(tc * TAU * 27.0);
                st.p = [st.p[0] + w, st.p[1] - 0.6 * w];
            }
            let after = tc - tb;
            if (0.0..0.45).contains(&after) {
                let limp = 1.0 - after / 0.45;
                st.scale *= 1.0 - 0.2 * limp;
                st.sx = Some(st.sx.unwrap_or(1.0) * (1.0 + 0.16 * limp));
                st.sy = Some(st.sy.unwrap_or(1.0) * (1.0 - 0.22 * limp));
            }
        }
        st.extra.insert("bold_tartan".into(), tartan);
    }

    /// The bag's tartan over the glass: on from the first pump, its sett stretching as the bag swells.
    pub(super) fn bold_ball_glass(&self, st: &BallState, px: &GlassPx, inner: &mut V3) {
        let tartan = st.extra.get("bold_tartan").copied().unwrap_or(0.0);
        if tartan <= 0.02 {
            return;
        }
        let over = st.extra.get("bold_over").copied().unwrap_or(0.0);
        let tart = sc(
            tartan_rgb(px.dx, px.dy, 0.85 + 0.75 * tartan + 0.5 * over),
            0.62 + 0.42 * px.band,
        );
        let w_t = (0.5 + 0.55 * tartan).min(0.95) * (tartan / 0.15).min(1.0) * (0.75 + 0.25 * px.nz);
        for ch in 0..3 {
            inner[ch] = inner[ch] * (1.0 - w_t) + tart[ch] * w_t;
        }
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let k = self.k;
        let tint = bold::light(p.col[k]);
        // the aim: a dashed line from the bag the way it will let go
        if let Some((u, e)) = bold::until(p, k, &["release"], c.t, 0.5) {
            let v = p.run.row("V", p.sim_i(e.t) + 1, k).map_or([0.0, 0.0], |r| [r[0], r[1]]);
            let n = dmath::hypot(v[0], v[1]);
            if n > 1e-6 {
                let a = c.fs[k].p;
                let b = [a[0] + v[0] / n * 3.4, a[1] + v[1] / n * 3.4];
                floor::dashes(ic, c, a, b, 0.11, 0.36, c.t * 5.0, tint, 0.8 + 0.8 * u);
            }
        }
        // it lands: the last of the air leaves it
        if let Some((age, e)) = bold::since(p, k, &["land"], c.t, 0.3) {
            let u = age / 0.3;
            floor::band(
                ic,
                c,
                Family::Air,
                e.f(3),
                e.f(4),
                R * (1.0 + 1.2 * bold::ease_out(u)),
                1.0 - u,
                c.t,
                tint,
                0.8,
            );
        }
    }

    pub(super) fn bold_canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        // a ring of air puffs leads the tartan; BAG BURST's gust has a second one behind it
        let tint = bold::light(p.col[w.owner]);
        let burst = w.kind != "shockwave";
        floor::wave(ic, c, Family::Air, w, c.t, tint, if burst { 1.5 } else { 1.1 });
        if burst && w.r > 0.9 {
            let fade = 1.0 - w.r / w.rmax.max(1e-6);
            floor::band(ic, c, Family::Air, w.cx, w.cz, w.r - 0.9, fade, c.t + 0.4, tint, 1.0);
        }
    }

    /// WAR CRY's and BAG BURST's bands of real tartan (the bag's own sett, laid on the canvas), lit by the spot
    /// like the canvas.
    pub(super) fn bold_canvas_paint(&self, p: &PictureCtx, col: &mut [V3], ic: &[V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let bayer = config::bayer_full();
        for w in p.run.waves(c.i).iter().filter(|w| w.owner == self.k) {
            let burst = w.kind != "shockwave";
            let width = if burst { 0.85 } else { 0.72 };
            let fade = ((1.0 - w.r / w.rmax.max(1e-6)) * if burst { 4.0 } else { 3.2 }).min(1.0);
            for (j, out) in col.iter_mut().enumerate() {
                let (dx, dz) = (c.sxp[j] - w.cx, c.szp[j] - w.cz);
                let u = (dmath::hypot(dx, dz) - (w.r - width)) / width;
                if !(0.0..=1.0).contains(&u) {
                    continue;
                }
                // a crisp front, a tail that breaks up
                let edge = (u * 3.5).min((1.0 - u) * 9.0).min(1.0);
                let b = bayer.px(c.xs[j], c.ys[j])[0];
                let dw = ((edge * fade * 4.0 + b * 0.9).floor() / 4.0).clamp(0.0, 1.0);
                if dw <= 0.0 {
                    continue;
                }
                let paint = tartan_rgb(dx, dz, 0.62);
                let lit = (ic[j].iter().sum::<f64>() / 3.0).clamp(0.55, 1.6) * 0.86;
                for ch in 0..3 {
                    out[ch] = out[ch] * (1.0 - dw) + paint[ch] * lit * dw;
                }
            }
        }
    }

    /// The flight's trail: a broad pale ribbon and outlined puffs of air that shrink as they are left behind.
    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let Some(Item::Trail) = payload.downcast_ref::<Item>() else {
            return;
        };
        let k = self.k;
        let tc = f.t.min(p.t_ko);
        let j1 = p.sim_i(tc) as i64;
        let first = match self.cycle_at(tc) {
            Some((c, _)) => p.sim_i(c.t_rel) as i64,
            None => j1 - 12,
        };
        let j0 = first.max(j1 - 16).max(0);
        if j1 - j0 < 1 {
            return;
        }
        let ribbon: Vec<V3> = (j0..=j1)
            .map(|j| {
                let q = p.run.p(j as usize, k);
                scene::to_world(q[0], q[1], R * 0.9)
            })
            .collect();
        bold::thick_line(fr, f.cam, &ribbon, bold::pale(p.col[k]), 7);
        for j in (j0..j1).filter(|j| (j1 - j) % 2 == 1) {
            let age = (j1 - j) as f64 / 16.0;
            let q = p.run.p(j as usize, k);
            let side = (px::rnd(j as u64, 7) - 0.5) * 0.3;
            let w = scene::to_world(q[0] + side, q[1] - side, R * 0.8 + 0.3 * age);
            bold::matte_blob(fr, f.cam, w, 0.1 + 0.2 * (1.0 - age), AIR);
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let (k, o, t, cam) = (self.k, 1 - self.k, f.t, f.cam);
        let st = &f.fs[k];
        let me = drawn.iter().find(|d| d.k == k).filter(|_| st.vis && !st.shattered);
        let enemy = drawn.iter().find(|d| d.k == o);
        // an enemy a ring pushed back trails speed lines
        if let (Some((age, _)), Some(b)) = (bold::since(p, k, &["hit:shockwave", "hit:wave"], t, PUSH_S), enemy) {
            let i = p.sim_i(t);
            let (now, was) = (p.run.p(i, o), p.run.p(i.saturating_sub(2), o));
            let (s1, s0) = (bold::at(cam, now[0], now[1], R), bold::at(cam, was[0], was[1], R));
            let u = age / PUSH_S;
            if dmath::hypot(s1.0 - s0.0, s1.1 - s0.1) > 1.5 {
                px::speed_lines(
                    img,
                    b.s,
                    b.rp,
                    (s1.0 - s0.0, s1.1 - s0.1),
                    34.0 * (1.0 - 0.5 * u),
                    6,
                    bold::WHITE,
                    1.0 - u,
                );
            }
        }
        let burst = Self::burst_t(p, k);
        if let Some(me) = me {
            let (s, rp) = (me.s, me.rp);
            let phase = st.extra.get("phase").copied().unwrap_or(0.0);
            // the flight's speed lines
            if phase == FLY {
                let i = p.sim_i(t);
                let (now, was) = (p.run.p(i, k), p.run.p(i.saturating_sub(2), k));
                let (s1, s0) = (bold::at(cam, now[0], now[1], R), bold::at(cam, was[0], was[1], R));
                let infl = st.extra.get("infl").copied().unwrap_or(0.0);
                if dmath::hypot(s1.0 - s0.0, s1.1 - s0.1) > 1.5 {
                    px::speed_lines(
                        img,
                        s,
                        rp,
                        (s1.0 - s0.0, s1.1 - s0.1),
                        20.0 + 26.0 * infl,
                        7,
                        bold::WHITE,
                        0.9,
                    );
                }
            }
            // the three drones stick out of the top of the bag, further with every pump
            let out = st.extra.get("bold_drones").copied().unwrap_or(0.0).clamp(0.0, 1.4);
            if phase != 0.0 && out > 0.03 {
                for (ang, long) in [(-2.2, 0.8), (-1.5, 1.25), (-0.85, 0.9)] {
                    let (dx, dy) = (dmath::cos(ang), dmath::sin(ang));
                    let len = rp * 0.75 * long * out.min(1.25);
                    let at = |d: f64| (s.0 + dx * (rp * 0.9 + d), s.1 + dy * (rp * 0.9 + d));
                    px::ink_line(img, at(0.0), at(len), 5.0, PIPE, 1.0);
                    px::ink_line(img, at(len * 0.72), at(len), 5.0, IVORY, 1.0);
                    px::ink_disc(img, at(len), 3.6, IVORY, 1.0);
                }
            }
            // WAR CRY's breath in: streaks of air rush into the bag
            if let Some((u, _)) = bold::until(p, k, &["ability:shockwave"], t, BREATH_S) {
                for j in 0..10u64 {
                    let ang = TAU * (j as f64 + px::rnd(j, 3)) / 10.0;
                    let far = rp * (1.25 + 2.3 * (1.0 - bold::ease_in(u)) * (0.7 + 0.5 * px::rnd(j, 5)));
                    let at = |d: f64| (s.0 + dmath::cos(ang) * d, s.1 + dmath::sin(ang) * d * 0.8);
                    px::ink_line(img, at(far), at(far + rp * 0.55), 2.0, AIR, (u * 3.0).min(1.0));
                }
            }
            // HIGHLAND FLING: two little dancing shoes tap under the ball, turn about on the eighths
            if st.haste > 0.0 {
                let beat = p.show.grid.beat;
                let step = (t / (beat * 0.5)).floor() as i64;
                let up = 1.0 - ((t / (beat * 0.5)).rem_euclid(1.0) / 0.5).min(1.0);
                for (n, side) in [(0, -1.0), (1, 1.0)] {
                    let lifted = step.rem_euclid(2) == n;
                    let hop = if lifted { 9.0 * (0.4 + 0.6 * up) } else { 0.0 };
                    let x = s.0 + side * rp * 0.5;
                    let y = s.1 + rp * 0.86 + 13.0 - hop;
                    // the leg in its white sock with a red garter, then the black shoe pointing outward
                    px::ink_line(img, (x, y - 14.0 + hop), (x, y - 3.0), 5.0, bold::WHITE, 1.0);
                    px::line(img, (x - 2.5, y - 9.0), (x + 2.5, y - 9.0), 2.5, TARTAN_RED, 1.0);
                    let shoe = [
                        (x - side * 4.5, y - 4.0),
                        (x + side * 10.0, y - 3.0),
                        (x + side * 13.0, y + 2.5),
                        (x - side * 4.5, y + 3.0),
                    ];
                    px::ink_poly(img, &shoe, PIPE, 1.0);
                    px::line(
                        img,
                        (x + side * 3.0, y - 3.0),
                        (x + side * 7.0, y - 2.5),
                        1.5,
                        TARTAN_GOLD,
                        1.0,
                    );
                    if !lifted && up > 0.4 {
                        // the tap
                        px::ink_line(
                            img,
                            (x + side * 16.0, y + 1.0),
                            (x + side * 22.0, y - 2.0),
                            1.6,
                            bold::WHITE,
                            up,
                        );
                        px::ink_line(
                            img,
                            (x + side * 15.0, y + 5.0),
                            (x + side * 21.0, y + 7.0),
                            1.6,
                            bold::WHITE,
                            up,
                        );
                    }
                }
            }
            if let Some(tb) = burst {
                // red stretch marks spread over the bag as it swells too far
                let u = 1.0 - (tb - t) / OVER_S;
                if (0.0..1.0).contains(&u) {
                    for j in 0..9u64 {
                        if (j as f64) > 9.0 * u + 1.0 {
                            break;
                        }
                        let ang = TAU * px::rnd(j, 11);
                        let d = rp * (0.25 + 0.55 * px::rnd(j, 12));
                        let c = (s.0 + dmath::cos(ang) * d, s.1 + dmath::sin(ang) * d);
                        // each mark lies across the bag's skin, with two short ticks: a seam giving way
                        let (tx, ty) = (-dmath::sin(ang), dmath::cos(ang));
                        let half = rp * (0.14 + 0.16 * u);
                        let (a, b) = ((c.0 - tx * half, c.1 - ty * half), (c.0 + tx * half, c.1 + ty * half));
                        px::ink_line(img, a, b, 2.0, [255.0, 60.0, 50.0], 1.0);
                        for q in [0.35, 0.65] {
                            let m = (a.0 + (b.0 - a.0) * q, a.1 + (b.1 - a.1) * q);
                            let tick = rp * 0.09;
                            px::line(
                                img,
                                (m.0 - dmath::cos(ang) * tick, m.1 - dmath::sin(ang) * tick),
                                (m.0 + dmath::cos(ang) * tick, m.1 + dmath::sin(ang) * tick),
                                1.6,
                                [255.0, 60.0, 50.0],
                                1.0,
                            );
                        }
                    }
                }
            }
        }
        // BAG BURST: the pop, the rush of air and the scraps of tartan
        if let Some(tb) = burst {
            let age = t - tb;
            if (0.0..SCRAPS_S).contains(&age) {
                let q = p.run.p(p.sim_i(tb), k);
                let c = bold::at(cam, q[0], q[1], R);
                let rp = R * bold::px_per_unit(cam, q[0], q[1], R);
                let seed = (tb * 1000.0) as u64;
                if age < 0.32 {
                    let u = age / 0.32;
                    // the rush of air: streaks racing out all round
                    for j in 0..18u64 {
                        let ang = TAU * (j as f64 + 0.7 * px::rnd(seed, j)) / 18.0;
                        let d0 = rp * (1.2 + 5.0 * bold::ease_out(u) * (0.7 + 0.6 * px::rnd(seed, j + 30)));
                        let at = |d: f64| (c.0 + dmath::cos(ang) * d, c.1 + dmath::sin(ang) * d * 0.75);
                        px::ink_line(img, at(d0), at(d0 + rp * (1.1 - 0.6 * u)), 2.5, AIR, 1.0 - u);
                    }
                }
                if age < 0.12 {
                    px::ink_star(img, c, rp * (1.3 + 6.0 * age), rp * 0.6, 10, 0.3, bold::WHITE, 1.0);
                }
                // scraps thrown out, then fluttering down
                for j in 0..16u64 {
                    let ang = TAU * px::rnd(seed, j + 60);
                    let far = rp * (1.6 + 3.2 * px::rnd(seed, j + 80));
                    let out = bold::ease_out(age / 0.35);
                    let sway = 9.0 * dmath::sin(age * (4.0 + 3.0 * px::rnd(seed, j + 100)) + j as f64);
                    let fall = 26.0 * (age - 0.25).max(0.0) * (0.7 + 0.6 * px::rnd(seed, j + 120));
                    let x = c.0 + dmath::cos(ang) * far * out + sway * (age / 0.4).min(1.0);
                    let y = c.1 + dmath::sin(ang) * far * out * 0.7 - rp * 0.8 * out + fall;
                    // a flat scrap seen turning: its width breathes
                    let turn = age * (7.0 + 5.0 * px::rnd(seed, j + 140)) + j as f64;
                    let (hw, hh) = (5.5 * dmath::cos(turn).abs() + 1.2, 4.5);
                    let tilt = 0.5 * dmath::sin(turn * 0.7);
                    let quad = [
                        (x - hw, y - hh + tilt * hw),
                        (x + hw, y - hh - tilt * hw),
                        (x + hw, y + hh - tilt * hw),
                        (x - hw, y + hh + tilt * hw),
                    ];
                    let tone = [TARTAN_LT, TARTAN_DK, TARTAN_RED, TARTAN_LT, TARTAN_GOLD][(j % 5) as usize];
                    let a = ((SCRAPS_S - age) / 0.4).min(1.0);
                    px::ink_poly(img, &quad, tone, a);
                    // the over-check
                    px::line(
                        img,
                        (x - hw, y + tilt * hw),
                        (x + hw, y - tilt * hw),
                        1.2,
                        TARTAN_RED,
                        a,
                    );
                }
            }
        }
    }

    /// A flight hit always pops heavy; a body slam at least solid.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match how(e) {
            "flight" => Some(HitSize::Heavy),
            "bodyslam" => Some(by_damage.max(HitSize::Solid)),
            _ => Some(by_damage),
        }
    }
}
