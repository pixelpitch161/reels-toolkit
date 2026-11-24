//! BASS DRUM with the bold effects (`picture.effects = "bold"`; the owner's picks 35a to 35d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **STEAMROLLER** (35a): a dotted dust lane runs down the line before the drum tips; rolling, the drum is a
//!   spoked wheel (hoop, six spokes, hub) that leaves a tyre track of two ruts with herringbone bars; the BOOM
//!   stamps a white drum skin on the canvas under the enemy, a quake ring rolls out of it and the skin's
//!   vibration shows as arcs either side.
//! - **HEARTBEAT** (35b): a little heart monitor over the drum draws its trace, a small spike for the lub and a
//!   tall one for the dub; the lub's ring is pale and dies half way out, the dub's is red, thick and doubled.
//! - **DRUM ROLL** (35c): the wheel sets off in a puff with speed lines behind it; the leg it is on is dashed ahead
//!   of it, the next leg shows faintly before it turns, and every leg leaves its track, so the zig-zag route can
//!   be read off the canvas; the first hit and every hit after a pause is a heavy one.
//! - **1812 OVERTURE** (35d): a cannon barrel on a wooden carriage rises out of the drum and aims at the spot; the
//!   spot gets a closing landing mark and a row of pips that count the shots down; each shot is a muzzle flash, a
//!   puff of smoke, a ball lobbed over and a blast, and the last shot's blast is half as big again.

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
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::scene::{self, R_BALL as R};

use super::{BassDrumLook, CHROME, DUSTC, HEADC, HOOPC, slams};

/// The dub's red, and the heart monitor's trace.
const RED: V3 = [255.0, 64.0, 76.0];
/// The cannon's iron.
const IRON: V3 = [72.0, 74.0, 88.0];
/// The carriage's wood.
const WOOD: V3 = [150.0, 96.0, 52.0];
/// A cannon's fire.
const FIRE: V3 = [255.0, 176.0, 84.0];
/// Seconds a track stays on the canvas after its roll (about two beats).
const TRACK_S: f64 = 0.8;
/// Seconds a BOOM's drum skin shows.
const SKIN_S: f64 = 0.45;
/// Seconds before a roll tips that its lane starts to show.
const LANE_S: f64 = 0.25;
/// Seconds the heart monitor is up before the lub and after the dub.
const ECG_PRE: f64 = 0.3;
const ECG_POST: f64 = 0.45;
/// Seconds a cannon's smoke hangs at the muzzle.
const SMOKE_S: f64 = 0.6;
/// Seconds a cannon's blast shows.
const BLAST_S: f64 = 0.5;

/// One cannon of [`BassDrumLook::cannons`]: `(t, i, x, y, t_land)`.
type Cannon = (f64, i64, f64, f64, f64);

/// The cannon on the drum: the ball's x, z, the aim's unit vector and the recoil (0 to 1).
struct Gun(f64, f64, f64, f64, f64);

/// One wheel of the cannon's carriage, on the canvas at x, z.
struct CarriageWheel(f64, f64);

/// The wheel's track from `a` along the unit vector `d`, `done` units long: two ruts with herringbone bars, at
/// strength `k`.
fn track(ic: &mut [V3], c: &CanvasCtx<'_>, a: [f64; 2], d: [f64; 2], done: f64, k: f64, tint: V3) {
    if done <= 0.0 || k <= 0.0 {
        return;
    }
    for (j, light) in ic.iter_mut().enumerate() {
        let (rx, rz) = (c.sxp[j] - a[0], c.szp[j] - a[1]);
        let along = rx * d[0] + rz * d[1];
        if along <= 0.0 || along >= done {
            continue;
        }
        let off = (-rx * d[1] + rz * d[0]).abs();
        if off >= R * 0.5 {
            continue;
        }
        let rut = off > R * 0.2;
        let bar = ((along + off * 0.9) / 0.18).rem_euclid(1.0) < 0.5;
        let dark = match (rut, bar) {
            (true, true) => 0.55,
            (true, false) => 0.2,
            _ => 0.1,
        };
        for v in light.iter_mut() {
            *v *= 1.0 - dark * k;
        }
        if rut && !bar {
            for ch in 0..3 {
                light[ch] += 0.4 * k * tint[ch];
            }
        }
    }
}

/// The muzzle of the cannon on a drum at `(bx, bz)` aimed along `(ux, uz)`: its x, z and height.
fn muzzle(bx: f64, bz: f64, ux: f64, uz: f64, recoil: f64) -> V3 {
    let d = R * (1.75 - 0.45 * recoil);
    [bx + ux * d, bz + uz * d, R * 2.1]
}

impl BassDrumLook {
    /// The 1812 OVERTURE volley under way at `t` (or just over): its cannons and which of them was fired last.
    fn volley(&self, p: &PictureCtx, t: f64) -> Option<(&[Cannon], usize)> {
        let fired = self.cannons.partition_point(|c| c.0 <= t);
        let last = fired.checked_sub(1)?;
        let mut start = last;
        while start > 0 && self.cannons[start].1 > 0 {
            start -= 1;
        }
        let mut end = last;
        while end + 1 < self.cannons.len() && self.cannons[end + 1].1 > self.cannons[end].1 {
            end += 1;
        }
        (t < self.cannons[end].4 + 0.3 && t < p.t_ko).then_some((&self.cannons[start..=end], last - start))
    }

    /// How many shots a volley has.
    fn shots(&self, p: &PictureCtx) -> i64 {
        p.cfg[self.k].raw["abilities"]["low"]["n"].as_f64().unwrap_or(5.0) as i64
    }

    /// The cannon's aim at `t`: the ball, the unit vector to the spot and the recoil of the last shot.
    fn aim(&self, p: &PictureCtx, t: f64, b: [f64; 2]) -> Option<Gun> {
        let (shots, j) = self.volley(p, t)?;
        let (t0, _, x, z, _) = shots[j];
        let (dx, dz) = (x - b[0], z - b[1]);
        let n = dmath::hypot(dx, dz);
        let (ux, uz) = if n > 1e-6 { (dx / n, dz / n) } else { (1.0, 0.0) };
        Some(Gun(b[0], b[1], ux, uz, (1.0 - (t - t0) / 0.16).max(0.0)))
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let t = c.t;
        if !(self.t_fight <= t && t < p.t_ko + 0.6) {
            return;
        }
        let tc = self.ta(p, t);
        let dust = bold::light(DUSTC);
        // a DRUM ROLL leg is under way: the next leg may show before it turns
        let on_leg = self
            .segs
            .iter()
            .any(|s| s.kind == 1 && s.t0 <= tc && tc < s.t_end + 0.05);
        for s in &self.segs {
            if !(s.t_tip - 0.4 <= tc && tc < s.t_end + TRACK_S) {
                continue;
            }
            let a = s.p0;
            let d = [s.p1[0] - a[0], s.p1[1] - a[1]];
            let len = dmath::hypot(d[0], d[1]) + 1e-9;
            let d = [d[0] / len, d[1] / len];
            let u = ((tc.min(s.t_end) - s.t0) / (s.t1 - s.t0).max(1e-9)).clamp(0.0, 1.0);
            // the lane: dotted dust down the line, before the drum tips and ahead of the wheel as it rolls
            let lane_k = if tc >= s.t_end {
                0.0
            } else if tc >= s.t_tip {
                0.55 + 0.45 * bold::ease_out((tc - s.t_tip) / (s.t0 - s.t_tip).max(0.05))
            } else if s.kind == 0 {
                0.55 * bold::ease_out(1.0 - (s.t_tip - tc) / LANE_S)
            } else if on_leg {
                0.4
            } else {
                0.0
            };
            if lane_k > 0.0 {
                let from = (len * u + R * 0.7).min(len);
                let head = [a[0] + d[0] * from, a[1] + d[1] * from];
                floor::dashes(ic, c, head, s.p1, 0.1, 0.3, tc * 4.0, dust, lane_k);
                floor::ring(ic, c, s.p1[0], s.p1[1], 0.2, 0.06, dust, lane_k);
            }
            // the track behind the wheel
            if tc >= s.t0 {
                let out = ((tc - s.t_end) / TRACK_S).max(0.0);
                track(ic, c, a, d, len * u, 1.0 - out, dust);
            }
        }
        // a BOOM: the drum skin stamped on the canvas, and the quake rolling out of it
        let skin = bold::light(HEADC);
        for &(t0, x, z, kind) in &self.booms {
            let age = t - t0;
            if !(0.0..SKIN_S).contains(&age) || t0 > p.t_ko {
                continue;
            }
            let big = if kind == 0 { 1.0 } else { 0.62 };
            let u = age / SKIN_S;
            let r = R * (0.9 + 1.5 * bold::ease_out(age / 0.12)) * big;
            floor::disc(ic, c, x, z, r, skin, 0.75 * (1.0 - u) * (1.0 - u));
            floor::ring(ic, c, x, z, r, 0.07, skin, 1.1 * (1.0 - u));
            floor::band(
                ic,
                c,
                Family::Quake,
                x,
                z,
                r + R * 3.2 * u * big,
                1.0 - u,
                t,
                skin,
                1.2 * big,
            );
        }
        // DRUM ROLL: the wheel sets off in a puff
        if let Some((age, e)) = bold::since(p, self.k, &["drum_roll"], t, 0.3) {
            let at = p.run.p(p.sim_i(e.t), self.k);
            let u = age / 0.3;
            floor::band(
                ic,
                c,
                Family::Air,
                at[0],
                at[1],
                R * (0.8 + 1.6 * u),
                1.0 - u,
                t,
                dust,
                1.3,
            );
        }
        // 1812 OVERTURE: the shadow of the ball in the air
        if let Some((shots, _)) = self.volley(p, t) {
            let b = c.fs[self.k].p;
            for &(t0, _, x, z, t_land) in shots.iter().filter(|s| s.0 <= t && t < s.4) {
                let Some(Gun(bx, bz, ux, uz, _)) = self.aim(p, t0, b) else {
                    continue;
                };
                let m = muzzle(bx, bz, ux, uz, 0.0);
                let u = (t - t0) / (t_land - t0).max(1e-6);
                floor::shade(ic, c, m[0] + (x - m[0]) * u, m[1] + (z - m[1]) * u, 0.2, 0.5);
            }
        }
    }

    pub(super) fn bold_canvas_wave(&self, _p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind != "shockwave" {
            return;
        }
        // which thump of HEARTBEAT this ring is: the one that started it
        let dub = self
            .beats
            .iter()
            .min_by(|a, b| (a.0 - w.t0).abs().total_cmp(&(b.0 - w.t0).abs()))
            .is_none_or(|b| b.1 > 0);
        let rmax = if w.rmax > 0.0 { w.rmax } else { 1.0 };
        if dub {
            // the dub: big, red, thick and doubled
            let red = [1.9, 0.2, 0.3];
            let fade = 1.0 - w.r / rmax;
            floor::wave(ic, c, Family::Quake, w, c.t, red, 1.6);
            floor::ring(ic, c, w.cx, w.cz, w.r, 0.12, red, 1.2 * fade);
            floor::ring(ic, c, w.cx, w.cz, (w.r - 0.42).max(0.05), 0.07, red, 0.9 * fade);
        } else {
            // the lub: small and pale, gone half way out
            let fade = 1.0 - w.r / (rmax * 0.55);
            floor::band(
                ic,
                c,
                Family::Quake,
                w.cx,
                w.cz,
                w.r,
                fade,
                c.t,
                bold::light(HEADC),
                0.8,
            );
        }
    }

    pub(super) fn bold_canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let fire = bold::light(FIRE);
        let last_i = self.shots(p) - 1;
        // 1812 OVERTURE: the landing mark closes in, then the blast (the last one half as big again)
        for s in slams(p, c.i) {
            if s.owner != self.k as i64 {
                continue;
            }
            if c.t < s.t_slam {
                let u = ((c.t - s.t0) / (s.t_slam - s.t0).max(1e-9)).clamp(0.0, 1.0);
                floor::shade(ic, c, s.cx, s.cz, s.rad, 0.35 * u);
                floor::target(ic, c, s.cx, s.cz, s.rad, u, fire);
            } else {
                let u = (c.t - s.t_slam) / BLAST_S;
                if u >= 1.0 {
                    continue;
                }
                let last = self
                    .cannons
                    .iter()
                    .any(|cn| (cn.4 - s.t_slam).abs() < 0.05 && cn.1 >= last_i);
                let big = if last { 1.5 } else { 1.0 };
                floor::disc(
                    ic,
                    c,
                    s.cx,
                    s.cz,
                    s.rad * 0.9 * big,
                    fire,
                    1.3 * (1.0 - u * 4.0).max(0.0),
                );
                floor::band(
                    ic,
                    c,
                    Family::Quake,
                    s.cx,
                    s.cz,
                    s.rad * (0.6 + 0.9 * u) * big,
                    1.0 - u,
                    c.t,
                    fire,
                    1.3 * big,
                );
                if last {
                    floor::band(
                        ic,
                        c,
                        Family::Quake,
                        s.cx,
                        s.cz,
                        s.rad * (0.3 + 0.7 * u) * big,
                        1.0 - u,
                        c.t + 3.0,
                        fire,
                        1.0,
                    );
                }
            }
        }
    }

    /// The rolling drum as a spoked wheel: a dark hoop, six chrome spokes turning with the roll and a hub, drawn
    /// over the sprite inside the ball.
    #[allow(clippy::too_many_arguments)] // the hook's own arguments
    pub(super) fn bold_wheel(
        &self,
        p: &PictureCtx,
        kk: usize,
        st: &BallState,
        t: f64,
        arr: &mut Img<f32>,
        s: (f64, f64),
        rp: f64,
    ) {
        if kk != self.k || st.shattered || !st.extra.contains_key("drum_roll") {
            return;
        }
        let Some(rw) = self.row(p, t) else { return };
        let ca = if rw[2].is_nan() { 1.0 } else { dmath::cos(rw[2]) };
        let spin = if ca >= 0.0 { rw[3] } else { -rw[3] };
        let hoop = rp * 0.74;
        px::ring(arr, s, hoop, 3.4, bold::INK, 1.0);
        for j in 0..6 {
            let a = spin + f64::from(j) * dmath::TAU / 6.0;
            let tip = (s.0 + hoop * dmath::cos(a), s.1 + hoop * dmath::sin(a));
            px::ink_line(arr, s, tip, 1.8, CHROME, 1.0);
        }
        px::ring(arr, s, hoop, 2.0, HOOPC.map(|v| v + 40.0), 1.0);
        // the lugs: one between each pair of spokes
        for j in 0..6 {
            let a = spin + (f64::from(j) + 0.5) * dmath::TAU / 6.0;
            px::disc(
                arr,
                (s.0 + hoop * dmath::cos(a), s.1 + hoop * dmath::sin(a)),
                1.6,
                CHROME,
                1.0,
            );
        }
        px::ink_disc(arr, s, rp * 0.2, HOOPC.map(|v| v + 40.0), 1.0);
        px::disc(arr, s, rp * 0.08, CHROME, 1.0);
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if !f.in_fight || f.fs[self.k].shattered {
            return Vec::new();
        }
        let b = f.fs[self.k].p;
        let Some(gun) = self.aim(p, f.t, b) else {
            return Vec::new();
        };
        let depth_at = |x: f64, z: f64, y: f64| f.cam.depth(&[scene::to_world(x, z, y)])[0];
        let mut out: Vec<(f64, Payload)> = Vec::new();
        // the carriage: a wooden wheel either side of the drum
        for side in [-1.0, 1.0] {
            let (x, z) = (b[0] - gun.3 * side * R * 1.1, b[1] + gun.2 * side * R * 1.1);
            out.push((depth_at(x, z, 0.2), Box::new(CarriageWheel(x, z))));
        }
        out.push((depth_at(b[0], b[1], R) - 0.3, Box::new(gun)));
        out
    }

    pub(super) fn bold_draw_item(&self, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        if let Some(&CarriageWheel(x, z)) = payload.downcast_ref::<CarriageWheel>() {
            bold::matte_blob(fr, f.cam, scene::to_world(x, z, 0.22), 0.22, WOOD);
            bold::matte_blob(fr, f.cam, scene::to_world(x, z, 0.22), 0.08, IRON);
            return;
        }
        let Some(&Gun(bx, bz, ux, uz, recoil)) = payload.downcast_ref::<Gun>() else {
            return;
        };
        // the barrel, kicked back by each shot: a fat breech, the tube and the mouth's ring
        let m = muzzle(bx, bz, ux, uz, recoil);
        let back = R * (-0.15 - 0.45 * recoil);
        let breech = scene::to_world(bx + ux * back, bz + uz * back, R * 1.25);
        let mouth = scene::to_world(m[0], m[1], m[2]);
        let mid: V3 = std::array::from_fn(|ch| breech[ch] + (mouth[ch] - breech[ch]) * 0.45);
        bold::thick_line(fr, f.cam, &[breech, mouth], IRON, 11);
        bold::thick_line(fr, f.cam, &[breech, mid], IRON, 15);
        bold::matte_blob(fr, f.cam, mouth, 0.19, IRON);
        bold::matte_blob(fr, f.cam, mouth, 0.1, bold::INK);
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let k = self.k;
        let t = f.t;
        if !(self.t_fight <= t && t < p.t_ko + 0.3) {
            return;
        }
        let me = drawn.iter().find(|d| d.k == k);
        // a BOOM: the skin's vibration, arcs either side of where it landed
        for &(t0, x, z, kind) in &self.booms {
            let age = t - t0;
            if !(0.0..0.28).contains(&age) || t0 > p.t_ko {
                continue;
            }
            let u = age / 0.28;
            let s = bold::at(f.cam, x, z, R);
            let r0 = bold::px_per_unit(f.cam, x, z, R) * R * if kind == 0 { 1.5 } else { 1.1 };
            for j in 0..(if kind == 0 { 3 } else { 2 }) {
                let r = r0 + 6.0 * f64::from(j) + 12.0 * bold::ease_out(u);
                let a = 1.0 - u;
                px::ink_arc(img, s, r, 1.6, -0.55, 0.55, bold::WHITE, a);
                px::ink_arc(img, s, r, 1.6, dmath::PI - 0.55, dmath::PI + 0.55, bold::WHITE, a);
            }
        }
        // DRUM ROLL: speed lines behind the wheel
        if let (Some(d), Some(rw)) = (me, self.row(p, t))
            && rw[6] == 1.0
            && !rw[2].is_nan()
        {
            let b = f.fs[k].p;
            let ahead = bold::at(f.cam, b[0] + dmath::cos(rw[2]), b[1] + dmath::sin(rw[2]), R);
            let here = bold::at(f.cam, b[0], b[1], R);
            px::speed_lines(
                img,
                d.s,
                d.rp,
                (ahead.0 - here.0, ahead.1 - here.1),
                16.0,
                5,
                bold::pale(DUSTC),
                0.9,
            );
        }
        // HEARTBEAT: the heart monitor over the drum
        if let Some(d) = me {
            for &(tb, i, _, _) in &self.beats {
                if i != 0 || tb > p.t_ko {
                    continue;
                }
                let gap = self
                    .beats
                    .iter()
                    .find(|b| b.1 == 1 && b.0 > tb && b.0 < tb + 0.6)
                    .map_or(0.21, |b| b.0 - tb);
                let end = tb + gap + ECG_POST;
                if t < tb - ECG_PRE || t >= end {
                    continue;
                }
                let a = ((end - t) / 0.15).clamp(0.0, 1.0);
                let c = (d.s.0.clamp(40.0, 320.0), d.s.1 - d.rp - 22.0);
                Self::heart_monitor(img, c, t - tb, gap, a);
            }
        }
        // 1812 OVERTURE
        let Some((shots, j)) = self.volley(p, t) else { return };
        let b = f.fs[k].p;
        let n = self.shots(p).max(1);
        let unit = bold::px_per_unit(f.cam, b[0], b[1], R);
        for &(t0, i, x, z, t_land) in shots.iter().filter(|s| s.0 <= t) {
            let last = i >= n - 1;
            let Some(Gun(bx, bz, ux, uz, _)) = self.aim(p, t0, b) else {
                continue;
            };
            let m = muzzle(bx, bz, ux, uz, 0.0);
            let ms = bold::at(f.cam, m[0] + ux * 0.2, m[1] + uz * 0.2, m[2] + 0.1);
            let age = t - t0;
            // the muzzle flash, then the smoke
            if age < 0.12 {
                let u = age / 0.12;
                px::ink_star(img, ms, 9.0 + 9.0 * u, 4.0, 8, t0 * 7.0, FIRE, 1.0);
                px::star(img, ms, 5.0 + 4.0 * u, 2.0, 8, t0 * 7.0 + 0.4, bold::WHITE, 1.0);
            }
            if age < SMOKE_S {
                let u = age / SMOKE_S;
                for q in 0..5u64 {
                    let (rx, ry) = (px::rnd((t0 * 1000.0) as u64, q), px::rnd((t0 * 1000.0) as u64, q + 9));
                    let at = (ms.0 + (rx - 0.5) * 16.0 * u, ms.1 - (6.0 + 16.0 * ry) * u);
                    let grey = if q % 2 == 0 {
                        [176.0, 172.0, 178.0]
                    } else {
                        [124.0, 120.0, 130.0]
                    };
                    px::ink_disc(img, at, 2.5 + 3.5 * u * (0.6 + rx), grey, (1.0 - u).min(0.9));
                }
            }
            // the ball in the air
            if t < t_land {
                let u = age / (t_land - t0).max(1e-6);
                let at = |u: f64| {
                    let y = m[2] + (0.15 - m[2]) * u + 2.2 * 4.0 * u * (1.0 - u);
                    bold::at(f.cam, m[0] + (x - m[0]) * u, m[1] + (z - m[1]) * u, y)
                };
                for q in 1..4 {
                    let back = u - 0.05 * f64::from(q);
                    if back > 0.0 {
                        px::disc(img, at(back), 2.4 - 0.5 * f64::from(q), FIRE, 0.8 - 0.2 * f64::from(q));
                    }
                }
                let r = (unit * 0.17).max(3.0);
                let s = at(u);
                px::ink_disc(img, s, r, IRON, 1.0);
                px::disc(img, (s.0 - r * 0.35, s.1 - r * 0.35), r * 0.3, bold::pale(IRON), 1.0);
            } else if t - t_land < 0.22 {
                // the blast: a star on the spot, bigger for the last shot
                let u = (t - t_land) / 0.22;
                let s = bold::at(f.cam, x, z, 0.3);
                let big = if last { 1.6 } else { 1.0 };
                px::ink_star(
                    img,
                    s,
                    (14.0 + 12.0 * u) * big,
                    6.0 * big,
                    10,
                    t0 * 5.0,
                    FIRE,
                    1.0 - u * u,
                );
                px::star(
                    img,
                    s,
                    (8.0 + 6.0 * u) * big,
                    3.0 * big,
                    10,
                    t0 * 5.0 + 0.3,
                    bold::WHITE,
                    1.0 - u,
                );
                px::chips(
                    img,
                    s,
                    t - t_land,
                    0.22,
                    if last { 12 } else { 7 },
                    26.0 * big,
                    (t0 * 1000.0) as u64,
                    IRON,
                );
            }
        }
        // the pips over the spot: one goes out as each shot lands
        let (_, _, x, z, _) = shots[j];
        let landed = shots.iter().filter(|s| s.4 <= t).count() as i64;
        let top = bold::at(f.cam, x, z, 2.0 * R + 0.75);
        for q in 0..n {
            let at = (top.0 + (q as f64 - (n - 1) as f64 / 2.0) * 9.0, top.1);
            if q < n - landed {
                px::ink_disc(img, at, 3.0, FIRE, 1.0);
                px::disc(img, (at.0 - 1.0, at.1 - 1.0), 1.0, bold::WHITE, 1.0);
            } else {
                px::ink_disc(img, at, 2.2, [70.0, 66.0, 82.0], 1.0);
            }
        }
    }

    /// The heart monitor: a dark screen centred on `c` with the trace drawn up to `now` seconds after the lub (the
    /// dub comes `gap` later), at strength `a`.
    fn heart_monitor(img: &mut Pic, c: (f64, f64), now: f64, gap: f64, a: f64) {
        let (half_w, half_h) = (34.0, 13.0);
        let frame = [
            (c.0 - half_w, c.1 - half_h),
            (c.0 + half_w, c.1 - half_h),
            (c.0 + half_w, c.1 + half_h),
            (c.0 - half_w, c.1 + half_h),
        ];
        px::poly(img, &frame, bold::INK, 0.78 * a);
        let mut edge = frame.to_vec();
        edge.push(frame[0]);
        px::polyline(img, &edge, 1.0, bold::deep(RED), a);
        // the trace: seconds after the lub, pixels above the base line
        let trace = [
            (-ECG_PRE, 0.0),
            (-0.045, 0.0),
            (-0.01, 7.0),
            (0.03, -3.0),
            (0.06, 0.0),
            (gap - 0.045, 0.0),
            (gap - 0.01, 16.0),
            (gap + 0.03, -6.0),
            (gap + 0.06, 0.0),
            (gap + ECG_POST, 0.0),
        ];
        let span = ECG_PRE + gap + ECG_POST;
        let on_screen = |(tau, h): (f64, f64)| {
            (
                c.0 - half_w + 4.0 + (tau + ECG_PRE) / span * (2.0 * half_w - 8.0),
                c.1 + 6.0 - h,
            )
        };
        let mut pts: Vec<(f64, f64)> = Vec::new();
        for pair in trace.windows(2) {
            let (p0, p1) = (pair[0], pair[1]);
            if pts.is_empty() {
                pts.push(on_screen(p0));
            }
            if now >= p1.0 {
                pts.push(on_screen(p1));
            } else {
                let u = ((now - p0.0) / (p1.0 - p0.0).max(1e-6)).clamp(0.0, 1.0);
                pts.push(on_screen((p0.0 + (p1.0 - p0.0) * u, p0.1 + (p1.1 - p0.1) * u)));
                break;
            }
        }
        px::polyline(img, &pts, 1.6, RED, a);
        if let Some(&pen) = pts.last() {
            px::disc(img, pen, 2.0, bold::WHITE, a);
        }
    }

    /// A STEAMROLLER BOOM is always a heavy hit; DRUM ROLL's first hit, and each one after a pause, flattens the
    /// enemy too; the last cannon is the biggest.
    pub(super) fn bold_hit(&self, p: &PictureCtx, e: &Event, by_damage: HitSize) -> Option<HitSize> {
        Some(match bold::how(e) {
            "roll" => HitSize::Heavy,
            "drumroll" if bold::since(p, self.k, &["hit:drumroll"], e.t - 1e-6, 0.3).is_none() => HitSize::Heavy,
            "cannon" => {
                let last_i = self.shots(p) - 1;
                if self.cannons.iter().any(|c| (c.4 - e.t).abs() < 0.05 && c.1 >= last_i) {
                    HitSize::Heavy
                } else {
                    by_damage.max(HitSize::Solid)
                }
            }
            _ => by_damage,
        })
    }
}
