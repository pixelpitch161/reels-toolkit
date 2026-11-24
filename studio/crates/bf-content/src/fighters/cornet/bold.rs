//! CORNET with the bold effects (`picture.effects = "bold"`; the owner's picks 1a to 1d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **PLUNGER MUTE** (1a): a big red rubber cup tumbles out on a thick cord; it sticks with a squash (and the
//!   ring's white pop) and the cord snaps tight; before each wah a purple bead runs down the cord to the cup; the
//!   cup tumbles again as it reels back.
//! - **WA-WA** (1b): the shield reads as one: the cup hovers at the bell, opening and closing on the beat inside
//!   wobbling purple rings that flare when a hit is blocked and thin out as the shield runs down.
//! - **DERBY HAT** (1c): a landing mark closes in on the spot while the hat spins over; it lands with a squash;
//!   the muffled enemy is marked silenced (grey, X marks, a draining ring) and the hat pops off at the end.
//! - **STOP-TIME** (1d): the colour drains out of the picture, freeze brackets lock on the enemy, each dash
//!   leaves a pink slash hanging in the air, and on the break the colour floods back, every slash bursts and
//!   the ring goes out in stacked brass bands.

use bf_core::dmath;
use bf_core::event::Event;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize, Status, StatusKind};
use bf_picture::camera::{Cam, V3};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::Wave;
use bf_picture::scene::{self, R_BALL as R};

use super::{CORD, CornetLook, RIM, RUBBER, WAH_S, WOOD, cord, draw_hat, last_event, ngon};

/// The wah's purple, as paint and as canvas light.
const PURPLE: V3 = [190.0, 110.0, 255.0];
/// A STOP-TIME slash.
const PINK: V3 = [255.0, 120.0, 200.0];
/// The freeze brackets.
const ICE: V3 = [200.0, 232.0, 255.0];
/// Seconds the cup is squashed after it sticks, and a hat after it lands.
const SQUASH_S: f64 = 0.12;
/// Seconds a hat takes to pop off when the muffle ends.
const POP_S: f64 = 0.3;
/// Seconds the colour takes to flood back after the break, with the slashes bursting.
const FLOOD_S: f64 = 0.25;

/// One painter item of the bold look.
enum BItem {
    /// The cord and cup: cup x, z, phase code, the ball's x, z.
    Cup(f64, f64, f64, f64, f64),
    /// WA-WA's cup at the bell: x, z, how open (0 shut, 1 open), the ball's x, z.
    Wah(f64, f64, f64, f64, f64),
    /// A hat: x, z, height, spin, squash (1 none, above 1 wider and flatter).
    Hat(f64, f64, f64, f64, f64),
}

/// The big cup: the handle, the rim and the rubber, each outlined; `wide` squashes it flat against what it hit
/// and `rot` tumbles it.
#[allow(clippy::too_many_arguments)]
fn draw_big_cup(fr: &mut Frame, cam: &Cam, cx: f64, cz: f64, bx: f64, bz: f64, wide: f64, rot: f64) {
    let (dx, dz) = (cx - bx, cz - bz);
    let n = dmath::hypot(dx, dz).max(1e-9);
    let (ux, uz) = (dx / n, dz / n);
    let (hx, hz) = (cx - ux * 0.42, cz - uz * 0.42);
    bold::thick_line(
        fr,
        cam,
        &[scene::to_world(hx, hz, 0.5), scene::to_world(cx, cz, 0.5)],
        WOOD,
        3,
    );
    bold::outlined_poly(fr, cam, &ngon(cx, cz, 0.52, 0.30 * wide, 12, rot), RIM);
    bold::outlined_poly(
        fr,
        cam,
        &ngon(cx + ux * 0.02, cz + uz * 0.02, 0.57, 0.21 * wide, 10, rot + 0.3),
        RUBBER,
    );
}

impl CornetLook {
    /// STOP-TIME under way at `t`: when it started and when its break came (infinite before the break).
    fn stop_time(p: &PictureCtx, k: usize, t: f64) -> Option<(f64, f64)> {
        let start = p
            .rec
            .events
            .iter()
            .rev()
            .find(|e| e.kind == "ability" && e.who() == Some(k) && e.t <= t && e.s(4) == "stoptime")?
            .t;
        let brk = p
            .rec
            .events
            .iter()
            .find(|e| e.kind == "break" && e.who() == Some(k) && e.t >= start)
            .map_or(f64::INFINITY, |e| e.t);
        Some((start, brk))
    }

    /// The muffle this side put on the enemy that is on at `t`, or ended less than `after` seconds ago: the
    /// victim, its start and its length.
    fn muffle(p: &PictureCtx, k: usize, t: f64, after: f64) -> Option<(usize, f64, f64)> {
        p.rec
            .events
            .iter()
            .rev()
            .filter(|e| e.kind == "muffle" && e.i(3) == k as i64 && e.t <= t)
            .find(|e| t < e.t + e.f(4) + after)
            .and_then(|e| Some((e.who()?, e.t, e.f(4))))
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let k = self.k;
        let purple = bold::light(PURPLE);
        // a wah: a wobbling purple ring round the stuck cup
        if let (Some(row), Some(t_wah)) = (Self::plg(p, c.i, k), last_event(&p.rec.events, "wah", k, c.t)) {
            let u = (c.t - t_wah) / WAH_S;
            if row[2] == 2.0 && (0.0..1.0).contains(&u) {
                floor::band(
                    ic,
                    c,
                    Family::Strings,
                    row[0],
                    row[1],
                    0.3 + 1.2 * u,
                    1.0 - u,
                    c.t,
                    purple,
                    1.0,
                );
            }
        }
        // WA-WA: two wobbling rings round the cornet that thin out with the shield and flare on a block
        let me = &c.fs[k];
        if me.shield > 0.0 && !me.shattered {
            let left = p.bold_shield_left(k, c.t).unwrap_or(1.0);
            let flare = p
                .bold_last_block(k, c.t)
                .map_or(0.0, |(tb, _, _)| (1.0 - (c.t - tb) / 0.22).max(0.0));
            let beat = p.show.grid.beat_pulse(c.t, 0.2, 0.05);
            for (n, r) in [R * 1.55, R * 2.0].into_iter().enumerate() {
                let k_ring = (0.3 + 0.55 * left) * if n == 0 { 1.0 } else { 0.7 } + flare;
                floor::band(
                    ic,
                    c,
                    Family::Strings,
                    me.p[0],
                    me.p[1],
                    r + 0.05 * beat,
                    0.35 + 0.65 * left,
                    c.t + n as f64,
                    purple,
                    k_ring,
                );
            }
        }
        // DERBY HAT: the landing mark closes in while the hat is in the air
        let radius = p.cfg[k].raw["abilities"]["unlock2"]["radius"].as_f64().unwrap_or(0.9);
        for e in p
            .rec
            .events
            .iter()
            .filter(|e| e.kind == "derby" && e.who() == Some(k) && e.t <= c.t && c.t < e.f(5))
        {
            let u = (c.t - e.t) / (e.f(5) - e.t).max(1e-6);
            floor::target(ic, c, e.f(3), e.f(4), radius, u, bold::light(p.col[k]));
        }
    }

    pub(super) fn bold_canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        // the break: the band comes back in, stacked brass bands in the cornet's colour
        floor::wave(ic, c, Family::Brass, w, c.t, bold::light(p.col[w.owner]), 1.15);
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if !f.in_fight {
            return Vec::new();
        }
        let k = self.k;
        let o = 1 - k;
        let mut out: Vec<(f64, Payload)> = Vec::new();
        let depth_at = |x: f64, z: f64, y: f64| f.cam.depth(&[scene::to_world(x, z, y)])[0];
        let b = f.fs[k].p;
        if let Some(row) = Self::plg(p, f.i, k) {
            if row[2] > 0.0 && !f.fs[k].shattered {
                out.push((
                    depth_at(row[0], row[1], 0.5),
                    Box::new(BItem::Cup(row[0], row[1], row[2], b[0], b[1])),
                ));
            }
            if !row[4].is_nan() {
                let y = 0.5 + row[6];
                out.push((
                    depth_at(row[4], row[5], y),
                    Box::new(BItem::Hat(row[4], row[5], y, f.t * 9.0, 1.0)),
                ));
            }
        }
        // WA-WA: the cup hovers at the bell (toward the enemy), opening and closing on the beat
        if f.fs[k].shield > 0.0 && !f.fs[k].shattered {
            let e = f.fs[o].p;
            let (dx, dz) = (e[0] - b[0], e[1] - b[1]);
            let n = dmath::hypot(dx, dz).max(1e-9);
            let open = p.show.grid.beat_pulse(f.t, 0.25, 0.08).clamp(0.0, 1.0);
            let d = R * 1.15 + 0.22 * open;
            let (x, z) = (b[0] + dx / n * d, b[1] + dz / n * d);
            out.push((depth_at(x, z, 0.5) - 0.02, Box::new(BItem::Wah(x, z, open, b[0], b[1]))));
        }
        // the hat on the muffled enemy: squashed as it lands, popping off when the muffle ends
        if let Some((who, t0, dur)) = Self::muffle(p, k, f.t, POP_S)
            && !f.fs[who].shattered
        {
            let e = f.fs[who].p;
            let sit = 2.0 * R * f.fs[who].scale.max(0.5) + 0.02;
            let since = f.t - t0;
            let (y, spin, squash) = if since >= dur {
                let u = (since - dur) / POP_S;
                (sit + 1.1 * bold::ease_out(u), 0.3 + u * 9.0, 1.0)
            } else {
                let land = (1.0 - since / SQUASH_S).max(0.0);
                (sit - 0.08 * land, 0.3, 1.0 + 0.35 * land)
            };
            out.push((
                depth_at(e[0], e[1], y) - 0.02,
                Box::new(BItem::Hat(e[0], e[1], y, spin, squash)),
            ));
        }
        out
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let k = self.k;
        match payload.downcast_ref::<BItem>() {
            Some(&BItem::Cup(cx, cz, phase, bx, bz)) => {
                let stuck = phase == 2.0;
                let line = cord(bx, bz, cx, cz, if stuck { 0.0 } else { 0.2 });
                bold::thick_line(fr, f.cam, &line, CORD, 3);
                // before each wah a purple bead runs down the taut cord to the cup
                if stuck && let Some((u, _)) = bold::until(p, k, &["wah"], f.t, 0.3) {
                    let at = (u * 8.0).clamp(0.0, 8.0);
                    let (i, fr_u) = (at.floor() as usize, at - at.floor());
                    let (a, b) = (line[i.min(8)], line[(i + 1).min(8)]);
                    let w: V3 = std::array::from_fn(|ch| a[ch] + (b[ch] - a[ch]) * fr_u);
                    bold::blob(fr, f.cam, w, 0.09, PURPLE);
                }
                // squashed flat as it sticks; tumbling while it flies or reels back
                let wide = match bold::since(p, k, &["plunger_stick"], f.t, SQUASH_S) {
                    Some((age, _)) if stuck => 1.0 + 0.4 * (1.0 - age / SQUASH_S),
                    _ => 1.0,
                };
                let rot = if stuck { 0.0 } else { f.t * 14.0 };
                draw_big_cup(fr, f.cam, cx, cz, bx, bz, wide, rot);
            }
            Some(&BItem::Wah(x, z, open, bx, bz)) => {
                draw_big_cup(fr, f.cam, x, z, bx, bz, 0.8 + 0.25 * open, 0.0);
            }
            Some(&BItem::Hat(x, z, y, spin, squash)) => {
                if squash > 1.01 {
                    // a wider brim under the hat as it lands
                    bold::outlined_poly(fr, f.cam, &ngon(x, z, y, 0.42 * squash, 12, spin), [36.0, 30.0, 34.0]);
                }
                draw_hat(fr, f.cam, x, z, y, spin, p.col[k]);
            }
            None => {}
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let k = self.k;
        let o = 1 - k;
        let Some((t0, t_break)) = Self::stop_time(p, k, f.t) else {
            return;
        };
        if f.t >= t_break + FLOOD_S || f.t > p.t_ko {
            return;
        }
        let after = f.t - t_break;
        // the colour drains out, and floods back in on the break
        let drained = if after >= 0.0 {
            1.0 - after / FLOOD_S
        } else {
            bold::ease_out((f.t - t0) / 0.12)
        };
        px::wash(img, 0.85 * drained);
        // freeze brackets lock on the enemy
        if after < 0.0
            && let Some(d) = drawn.iter().find(|d| d.k == o)
        {
            let lock = bold::since(p, k, &["stop"], f.t, 0.4).map_or(0.0, |(age, _)| 1.0 - age / 0.4);
            px::brackets(img, d.s, d.rp * (1.45 + 0.5 * lock), 1.6, ICE, 1.0);
        }
        // every dash so far has left a pink slash hanging where it crossed the enemy
        let stops = p
            .rec
            .events
            .iter()
            .filter(|e: &&Event| e.kind == "stop" && e.who() == Some(k) && e.t >= t0 && e.t <= f.t.min(t_break));
        for e in stops {
            let i = p.sim_i(e.t);
            let (a, b) = (p.run.p(i, k), p.run.p(i, o));
            let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
            let n = dmath::hypot(dx, dz).max(1e-9);
            let (ux, uz) = (dx / n, dz / n);
            let grow = bold::ease_out((f.t - e.t) / 0.08);
            let half = R * 1.5 * grow;
            // a slash a little off the line of the dash, so three of them cross like claw marks
            let tilt = 0.5 * (e.i(3) as f64 - 1.0);
            let (sx, sz) = (
                ux * dmath::cos(tilt) - uz * dmath::sin(tilt),
                ux * dmath::sin(tilt) + uz * dmath::cos(tilt),
            );
            let p0 = bold::at(f.cam, b[0] - sx * half, b[1] - sz * half, R * 1.2);
            let p1 = bold::at(f.cam, b[0] + sx * half, b[1] + sz * half, R * 0.8);
            if after < 0.0 {
                px::ink_line(img, p0, p1, 2.4, PINK, 1.0);
            } else {
                // the break: each slash bursts
                let u = after / FLOOD_S;
                let mid = ((p0.0 + p1.0) / 2.0, (p0.1 + p1.1) / 2.0);
                px::ink_star(img, mid, 5.0 + 12.0 * bold::ease_out(u), 3.0, 6, e.t, PINK, 1.0 - u);
            }
        }
    }

    pub(super) fn bold_statuses(&self, p: &PictureCtx, t: f64) -> Vec<Status> {
        match Self::muffle(p, self.k, t, 0.0) {
            Some((who, t0, dur)) => vec![Status {
                who,
                kind: StatusKind::Silence,
                left: Some(1.0 - (t - t0) / dur.max(1e-6)),
            }],
            None => Vec::new(),
        }
    }

    /// The stick is a solid hit whatever its damage (the cup's white pop); a wah is a tap.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match e.s(7) {
            "plunger" | "stoptime" => Some(by_damage.max(HitSize::Solid)),
            "wah" => Some(HitSize::Tap),
            _ => Some(by_damage),
        }
    }
}
