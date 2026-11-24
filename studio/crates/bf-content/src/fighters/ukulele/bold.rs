//! UKULELE with the bold effects (`picture.effects = "bold"`; the owner's picks 42a to 42d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **SURF'S UP** (42a): the wave swells nearly twice as tall and as wide as what it hits, its crest inked in thick foam with foam caps bobbing
//!   along it; the uke hops onto a surfboard and rides it; where the wave crashes (into the enemy, or on the
//!   ropes) big droplets fly up and fall, a ring of foam rolls out over the floor and a wet patch is left that
//!   dries within two beats.
//! - **CHUNK** (42b): a muting bar comes down over the ball and presses it flat; the shockwave goes out as a ring
//!   of thick blocks that jumps outward in four steps, then stands still for a moment at its full size and is cut
//!   off at once instead of fading away.
//! - **SPEED STRUM** (42c): strum streaks flick down and up over the ball for as long as the haste lasts (the
//!   ring's own speed-up look does the rest).
//! - **UKE-NADO** (42d): the waterspout stands tall, a swaying funnel hooped with rings from foot to mouth; a ring of foam runs
//!   under its foot; each time it catches the enemy the enemy is lifted off the floor with spin marks whirling
//!   round it and a splash; when the spout lets go it bursts into droplets and the enemy flies off with speed
//!   lines.

use bf_core::dmath;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::ss;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Phase, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::scene::{self, R_BALL as R};

use super::{
    DEEP, FOAM, LITE, MID, OverlayPoly, PROFILE, UkuleleLook, add3, draw_wave, linspace, mean, mul3, overlay_polys,
    world,
};

/// How much taller the bold wave stands than the classic one.
const TALL: f64 = 1.9;
/// Seconds the uke stays on its board after a strum.
const RIDE_S: f64 = 0.75;
/// Seconds of the hop onto the board.
const HOP_S: f64 = 0.22;
/// Seconds a splash's droplets fly.
const SPLASH_S: f64 = 0.6;
/// Seconds the foam ring of a crash rolls out.
const FOAM_S: f64 = 0.5;
/// Seconds the muting bar takes to come down before CHUNK.
const MUTE_IN_S: f64 = 0.22;
/// Seconds the bar stays pressed, and then takes to lift.
const MUTE_HOLD_S: f64 = 0.14;
/// Seconds CHUNK's ring stands at its full size before it is cut off.
const CHUNK_HOLD_S: f64 = 0.12;
/// Seconds the enemy stays lifted after a tick of the waterspout.
const SUCK_S: f64 = 0.4;
/// Seconds of speed lines after the waterspout lets go.
const THROW_S: f64 = 0.45;
/// The surfboard.
const BOARD: V3 = [255.0, 224.0, 122.0];
/// The muting bar.
const BAR: V3 = [64.0, 54.0, 72.0];
/// The hits of a wave, and of the waterspout.
const SURF_HIT: [&str; 1] = ["hit:surf"];
const SPOUT_HIT: [&str; 1] = ["hit:spout"];
/// CHUNK.
const CHUNK: [&str; 1] = ["ability:shockwave"];

/// One painter item of the bold look.
enum BItem {
    /// A wave (an index into `surfs`).
    Wave(usize),
    /// The surfboard under the uke, on the wave of that index.
    Board(usize),
    /// The waterspout.
    Spout,
}

/// The row of world points along a bold wave at corner `idx` of its cross-section (4 is the crest, 8 the foot of
/// its face), from `-span` to `span` of its half width: the same curve `wave_quads` builds its faces on.
#[allow(clippy::too_many_arguments)]
fn wave_row(c: [f64; 2], u: [f64; 2], hw: f64, hh: f64, t: f64, lip: f64, idx: usize, span: f64) -> Vec<V3> {
    let n = [-u[1], u[0]];
    let (px_, py_) = PROFILE[idx];
    linspace(-span, span, 11)
        .into_iter()
        .map(|v| {
            let hj = hh * dmath::pow((1.0 - dmath::pow(v.abs(), 3.0)).max(0.0), 0.6);
            let mut x = px_ * 0.95 - 0.28 * (1.0 - v * v);
            if (4..=6).contains(&idx) {
                x += 0.05 * dmath::sin(3.1 * v + t * 9.0) - lip * 0.35 * py_;
            }
            let q = [c[0] - u[0] * x + n[0] * v * hw, c[1] - u[1] * x + n[1] * v * hw];
            world(q, (py_ * hj).max(0.0))
        })
        .collect()
}

/// How tall the bold waterspout stands, full grown.
const SPOUT_TOP: f64 = 3.9;

/// How far the waterspout leans at the share `v` of its height: nothing at its foot, half a unit at its top,
/// snaking.
fn sway(t: f64, v: f64) -> [f64; 2] {
    let a = 0.5 * v;
    [a * dmath::sin(t * 5.2 + v * 4.0), a * dmath::cos(t * 4.1 + v * 3.1)]
}

/// The bold waterspout's radius at the share `v` of its height: a narrow foot widening to a broad mouth.
fn funnel(v: f64) -> f64 {
    0.24 + 0.8 * v * v.sqrt()
}

/// The bold waterspout: a tall funnel of water, narrow at its foot and wide at its mouth, swaying, hooped with
/// rings from foot to mouth, a knot of foam running round every other ring to show the spin.
fn draw_bold_spout(fr: &mut Frame, cam: &Cam, p: [f64; 2], t: f64, k: f64, rim: V3) {
    if k <= 0.02 {
        return;
    }
    let top = SPOUT_TOP * (0.3 + 0.7 * (k * 1.4).min(1.0));
    let at = |v: f64, an: f64| {
        let (s, r) = (sway(t, v), funnel(v) * (k * 1.5).min(1.0));
        world(
            [p[0] + s[0] + r * dmath::cos(an), p[1] + s[1] + r * dmath::sin(an)],
            v * top,
        )
    };
    let vs = linspace(0.0, 1.0, 12);
    let na = 20;
    let angs = linspace(0.0, dmath::TAU, na + 1);
    let cdep = cam.depth(&[world(p, 0.8)])[0];
    let (mut back, mut front): (Vec<OverlayPoly>, Vec<OverlayPoly>) = (Vec::new(), Vec::new());
    for i in 0..vs.len() - 1 {
        let sw = t * (9.0 + 3.0 * i as f64) + i as f64 * 0.7;
        for a in 0..na {
            let (a0, a1) = (angs[a] + sw, angs[a + 1] + sw);
            let q = [at(vs[i], a0), at(vs[i], a1), at(vs[i + 1], a1), at(vs[i + 1], a0)];
            let stripe = ((a * 2 + i * 3) / 4) % 4;
            let col = add3(mul3([FOAM, LITE, MID, LITE][stripe], 0.85), mul3(rim, 0.15));
            let al = if stripe == 0 { 0.85 } else { 0.6 } * k.min(1.0);
            let s = cam.wp(&q);
            if cam.depth(&[mean(&q)])[0] > cdep {
                back.push((s, mul3(col, 0.8), al * 0.8));
            } else {
                front.push((s, col, al));
            }
        }
    }
    back.extend(front);
    let (mut arr, depth) = (fr.array(), fr.depth());
    overlay_polys(&mut arr, &back);
    fr.set_array(&arr, Some(&depth));
    const HOOPS: usize = 7;
    for i in 0..HOOPS {
        let v = (i as f64 + 0.5) / HOOPS as f64;
        let pts: Vec<V3> = (0..=16).map(|j| at(v, f64::from(j) / 16.0 * dmath::TAU)).collect();
        bold::thick_line(fr, cam, &pts, if i % 2 == 0 { FOAM } else { LITE }, 3);
        if i % 2 == 0 {
            let spin = t * (11.0 + 2.0 * i as f64) + i as f64 * 1.3;
            bold::blob(fr, cam, at(v, spin), 0.1, FOAM);
        }
    }
}

/// A splash on the finished picture: `n` droplets thrown up from the ring-frame point `at`, leaning along `u`,
/// `age` seconds ago; each flies its own arc and is gone when it lands.
#[allow(clippy::too_many_arguments)]
fn splash(img: &mut Pic, cam: &Cam, at: [f64; 2], u: [f64; 2], age: f64, seed: u64, n: u64, reach: f64) {
    if !(0.0..SPLASH_S).contains(&age) {
        return;
    }
    for i in 0..n {
        let a = px::rnd(seed, i * 3) * dmath::TAU;
        let out = reach * (0.5 + 0.9 * px::rnd(seed, i * 3 + 1));
        let up = 2.4 + 2.4 * px::rnd(seed, i * 3 + 2);
        let y = 0.25 + up * age - 6.0 * age * age;
        if y < 0.0 {
            continue;
        }
        let x = at[0] + (dmath::cos(a) * out + u[0] * reach * 0.8) * age;
        let z = at[1] + (dmath::sin(a) * out + u[1] * reach * 0.8) * age;
        let r = (2.2 + 2.2 * px::rnd(seed, i * 3 + 1)) * (1.0 - 0.4 * age / SPLASH_S);
        px::ink_disc(
            img,
            bold::at(cam, x, z, y),
            r,
            if i % 3 == 0 { FOAM } else { LITE },
            1.0,
        );
    }
}

/// A ring of thick blocks on the canvas: `n` blocks `half_w` thick either side of radius `r` round `(cx, cz)`,
/// hard-edged, with gaps between them.
#[allow(clippy::too_many_arguments)]
fn blocks(ic: &mut [V3], c: &CanvasCtx<'_>, cx: f64, cz: f64, r: f64, half_w: f64, n: f64, tint: V3, k: f64) {
    for (j, light) in ic.iter_mut().enumerate() {
        let (dx, dz) = (c.sxp[j] - cx, c.szp[j] - cz);
        let d = dmath::hypot(dx, dz);
        if (d - r).abs() > half_w {
            continue;
        }
        if (dmath::atan2(dz, dx) / dmath::TAU * n).rem_euclid(1.0) < 0.3 {
            continue;
        }
        for ch in 0..3 {
            light[ch] += k * tint[ch];
        }
    }
}

impl UkuleleLook {
    /// How far the enemy is lifted by the waterspout at `t`: up at every tick that catches it, down again
    /// [`SUCK_S`] after the last.
    fn sucked(&self, p: &PictureCtx, t: f64) -> f64 {
        bold::since(p, self.k, &SPOUT_HIT, t, SUCK_S).map_or(0.0, |(age, _)| {
            bold::ease_out(age / 0.08) * (1.0 - bold::ease_in((age - 0.22) / (SUCK_S - 0.22)))
        })
    }

    /// CHUNK's ring `age` seconds after the strum: its radius, jumping outward in four steps, and whether it has
    /// reached its full size (it then stands for [`CHUNK_HOLD_S`]); `None` once it is cut off.
    fn chunk_ring(p: &PictureCtx, k: usize, age: f64) -> Option<(f64, f64)> {
        let ab = &p.cfg[k].unlock1;
        let (speed, rmax) = (ab.get("speed").unwrap_or(7.5), ab.get("radius").unwrap_or(3.0));
        let dur = (rmax - R) / speed.max(1e-6);
        if !(0.0..dur + CHUNK_HOLD_S).contains(&age) {
            return None;
        }
        let step = ((age / dur.max(1e-6)).min(1.0) * 4.0).ceil().max(1.0) / 4.0;
        Some((R + (rmax - R) * step, rmax))
    }

    pub(super) fn bold_state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase) {
        if phase != Phase::Fight {
            return;
        }
        let tc = t.min(p.t_ko);
        if kk != self.k {
            // the enemy in the waterspout: lifted off the floor at every tick
            st.lift = st.lift.max(0.55 * self.sucked(p, tc));
            return;
        }
        for w in &self.surfs {
            if w.t0 <= tc && tc < w.t_end + 0.25 {
                let ride = 1.0 - ss(w.t0 + 0.55, w.t0 + 0.9, tc);
                // riding the taller wave, and the hop onto the board
                let hop = 0.22 * dmath::sin(dmath::PI * ((tc - w.t0) / HOP_S).clamp(0.0, 1.0));
                st.lift = st.lift.max(0.45 * TALL * Self::height(w, tc) * ride + hop);
                st.glow = st.glow.max(0.45 * dmath::exp(-(tc - w.t0) / 0.14));
                st.glow_np = false;
            }
        }
        // CHUNK: the muting bar presses the ball flat
        let press = match (
            bold::until(p, kk, &CHUNK, tc, MUTE_IN_S),
            bold::since(p, kk, &CHUNK, tc, 2.0 * MUTE_HOLD_S),
        ) {
            (_, Some((age, _))) => 1.0 - bold::ease_in((age - MUTE_HOLD_S) / MUTE_HOLD_S),
            (Some((u, _)), None) => bold::ease_in((u - 0.6) / 0.4),
            _ => 0.0,
        };
        if press > 0.0 {
            st.sx = Some(st.sx.unwrap_or(1.0) * (1.0 + 0.14 * press));
            st.sy = Some(st.sy.unwrap_or(1.0) * (1.0 - 0.2 * press));
        }
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let (k, t) = (self.k, c.t);
        let foam = bold::light(FOAM);
        let dry = 2.0 * p.show.grid.beat;
        // a wave that hit: the wet patch it leaves, drying, and a ring of foam rolling out
        for e in bold::events_in(p, k, &SURF_HIT, t - dry, t) {
            let (age, x, z) = (t - e.t, e.f(5), e.f(6));
            let left = 1.0 - age / dry;
            let r = 0.5 + 0.6 * left;
            floor::shade(ic, c, x, z, r, 0.5 * left);
            floor::ring(ic, c, x, z, r, 0.05, bold::light(LITE), 0.6 * left);
            if age < FOAM_S {
                let u = age / FOAM_S;
                floor::band(
                    ic,
                    c,
                    Family::Air,
                    x,
                    z,
                    0.4 + 1.4 * bold::ease_out(u),
                    1.0 - u,
                    t,
                    foam,
                    1.3,
                );
            }
        }
        // a wave that broke on the ropes: its foam
        for w in self.surfs.iter().filter(|w| w.crashed) {
            let age = t - w.t_end;
            if (0.0..FOAM_S).contains(&age) {
                let (u, q) = (age / FOAM_S, self.front(w, w.t_end));
                floor::band(
                    ic,
                    c,
                    Family::Air,
                    q[0],
                    q[1],
                    0.3 + 1.0 * bold::ease_out(u),
                    1.0 - u,
                    t,
                    foam,
                    1.0,
                );
            }
        }
        // CHUNK: the ring stands at its full size for a moment after the wave itself is gone
        if let Some((age, e)) = bold::since(p, k, &CHUNK, t, 2.0)
            && let Some((r, rmax)) = Self::chunk_ring(p, k, age)
            && r >= rmax
            && !p.run.waves(c.i).iter().any(|w| w.owner == k && w.kind == "shockwave")
        {
            let o = p.run.p(p.sim_i(e.t), k);
            blocks(ic, c, o[0], o[1], r - 0.2, 0.2, 12.0, bold::light(p.col[k]), 1.1);
        }
        // UKE-NADO: foam runs round its foot
        if let Some((_, q, s)) = self.spout_at(t.min(p.t_ko)) {
            floor::band(ic, c, Family::Air, q[0], q[1], 0.85, s, t * 2.0, foam, 1.1);
        }
    }

    pub(super) fn bold_canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind != "shockwave" {
            return;
        }
        // CHUNK: the plucked string's ring, thin, and over it the thick blocks that jump outward in steps
        let tint = bold::light(p.col[w.owner]);
        floor::wave(ic, c, Family::Strings, w, c.t, tint, 0.5);
        if let Some((r, _)) = Self::chunk_ring(p, w.owner, c.t - w.t0) {
            blocks(ic, c, w.cx, w.cz, r - 0.2, 0.2, 12.0, tint, 1.1);
        }
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if !Self::show(p, f.t) {
            return Vec::new();
        }
        let ta = Self::ta(p, f.t);
        let mut out: Vec<(f64, Payload)> = Vec::new();
        for (i, w) in self.surfs.iter().enumerate() {
            if w.t0 <= ta && ta < w.t_end + 0.25 {
                let c = self.front(w, ta);
                let at = scene::to_world(c[0] - w.u[0] * 0.3, c[1] - w.u[1] * 0.3, 0.3);
                out.push((f.cam.depth(&[at])[0], Box::new(BItem::Wave(i))));
            }
        }
        // the board, under the ball, for the newest wave it rides
        let me = &f.fs[self.k];
        let riding = self
            .surfs
            .iter()
            .rposition(|w| w.t0 <= ta && ta < w.t_end.min(w.t0 + RIDE_S));
        if let Some(i) = riding
            && me.vis
            && !me.shattered
        {
            let at = scene::to_world(me.p[0], me.p[1], R + me.lift);
            out.push((f.cam.depth(&[at])[0] + 0.02, Box::new(BItem::Board(i))));
        }
        if let Some((_, q, _)) = self.spout_at(ta) {
            out.push((f.cam.depth(&[world(q, 0.6)])[0], Box::new(BItem::Spout)));
        }
        out
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let ta = Self::ta(p, f.t);
        match payload.downcast_ref::<BItem>() {
            Some(&BItem::Wave(i)) => {
                let w = &self.surfs[i];
                let hh = TALL * Self::height(w, if f.t < p.t_hs { ta } else { p.t_ko });
                if hh <= 0.03 {
                    return;
                }
                let c = self.front(w, ta);
                let lip = ss(w.t_end, w.t_end + 0.25, ta);
                // as wide as what it hits
                let hw = w.hw + 0.6 * R;
                draw_wave(fr, f, c, w.u, hw, hh, ta, p.col[w.owner], lip);
                // the foot of its face in dark water, the crest inked in thick foam, foam caps bobbing along it
                bold::thin_line(fr, f.cam, &wave_row(c, w.u, hw, hh, ta, lip, 8, 0.96), DEEP);
                let crest = wave_row(c, w.u, hw, hh, ta, lip, 4, 0.92);
                bold::thick_line(fr, f.cam, &crest, FOAM, 4);
                for (j, q) in crest.iter().enumerate().skip(1).step_by(2).take(5) {
                    let bob = 0.07 + 0.06 * dmath::sin(ta * 14.0 + j as f64 * 1.9);
                    bold::blob(fr, f.cam, [q[0], q[1] + bob, q[2]], 0.08 + 0.05 * lip, FOAM);
                }
            }
            Some(&BItem::Board(i)) => {
                let (w, me) = (&self.surfs[i], &f.fs[self.k]);
                let grow = bold::ease_out((ta - w.t0) / 0.1);
                let n = [-w.u[1], w.u[0]];
                let at = |a: f64, s: f64| {
                    let (a, s) = (a * R * grow, s * R * grow);
                    world(
                        [me.p[0] + w.u[0] * a + n[0] * s, me.p[1] + w.u[1] * a + n[1] * s],
                        me.lift + 0.04,
                    )
                };
                let half = [
                    (2.0, 0.0),
                    (1.6, 0.34),
                    (0.7, 0.54),
                    (-1.0, 0.54),
                    (-1.6, 0.38),
                    (-1.75, 0.0),
                ];
                let outline: Vec<V3> = half
                    .iter()
                    .map(|&(a, s)| at(a, s))
                    .chain(half.iter().rev().skip(1).take(4).map(|&(a, s)| at(a, -s)))
                    .collect();
                bold::outlined_poly(fr, f.cam, &outline, BOARD);
                bold::thick_line(fr, f.cam, &[at(-1.5, 0.0), at(1.75, 0.0)], p.col[self.k], 3);
            }
            Some(&BItem::Spout) => {
                if let Some((_, q, k)) = self.spout_at(ta) {
                    draw_bold_spout(fr, f.cam, q, f.t, k, p.col[self.k]);
                }
            }
            None => {}
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let (k, o, t, cam) = (self.k, 1 - self.k, f.t, f.cam);
        if !f.in_fight || t >= p.t_ko {
            return;
        }
        let me = drawn.iter().find(|d| d.k == k).filter(|_| !f.fs[k].shattered);
        let foe = drawn.iter().find(|d| d.k == o).filter(|_| !f.fs[o].shattered);
        // the crash: droplets fly where a wave hit, and where one broke on the ropes
        for e in bold::events_in(p, k, &SURF_HIT, t - SPLASH_S, t) {
            let u = self
                .surfs
                .iter()
                .rev()
                .find(|w| w.t0 <= e.t)
                .map_or([0.0, 0.0], |w| w.u);
            splash(img, cam, [e.f(5), e.f(6)], u, t - e.t, (e.t * 1000.0) as u64, 16, 1.5);
        }
        for w in self.surfs.iter().filter(|w| w.crashed) {
            let age = t - w.t_end;
            if (0.0..SPLASH_S).contains(&age) {
                let q = self.front(w, w.t_end);
                splash(
                    img,
                    cam,
                    q,
                    [-w.u[0] * 0.5, -w.u[1] * 0.5],
                    age,
                    (w.t0 * 1000.0) as u64,
                    9,
                    1.1,
                );
            }
        }
        if let Some(d) = me {
            // CHUNK: the muting bar comes down over the ball, presses, and lifts off
            let bar = match (
                bold::until(p, k, &CHUNK, t, MUTE_IN_S),
                bold::since(p, k, &CHUNK, t, 2.0 * MUTE_HOLD_S),
            ) {
                (_, Some((age, _))) => {
                    let off = bold::ease_in((age - MUTE_HOLD_S) / MUTE_HOLD_S);
                    Some((1.0 + 0.5 * off, 1.0 - off, age < MUTE_HOLD_S))
                }
                (Some((u, _)), None) => Some((2.6 - 1.6 * bold::ease_in(u), 1.0, false)),
                _ => None,
            };
            if let Some((high, alpha, pressed)) = bar {
                let (cx, cy) = (d.s.0, d.s.1 - d.rp * 0.62 * high);
                let (hw, hh) = (d.rp * 1.45, d.rp * 0.2);
                let rect = [
                    (cx - hw, cy - hh),
                    (cx + hw, cy - hh),
                    (cx + hw, cy + hh),
                    (cx - hw, cy + hh),
                ];
                px::ink_poly(img, &rect, BAR, alpha);
                px::line(
                    img,
                    (cx - hw + 2.0, cy - hh + 1.5),
                    (cx + hw - 2.0, cy - hh + 1.5),
                    1.2,
                    bold::pale(BAR),
                    alpha,
                );
                if pressed {
                    // pressed home: short ticks fly out at both ends
                    for side in [-1.0, 1.0] {
                        for (dy, len) in [(-0.5, 5.0), (0.3, 7.0), (1.1, 5.0)] {
                            let a = (cx + side * (hw + 3.0), cy + dy * hh * 2.0);
                            px::ink_line(img, a, (a.0 + side * len, a.1 + dy * 3.0), 1.4, bold::WHITE, 1.0);
                        }
                    }
                }
            }
            // SPEED STRUM: strum streaks flick down, then up, over the ball
            if p.bold_haste_left(k, t).is_some() {
                let phase = (t - p.show.grid.t_fight) / (p.show.grid.beat / 2.0);
                let (down, u) = (phase.floor().rem_euclid(2.0) == 0.0, phase.rem_euclid(1.0) / 0.75);
                if u < 1.0 {
                    let big =
                        bold::since(p, k, &["ability:haste"], t, 0.4).map_or(1.0, |(age, _)| 1.6 - 0.6 * age / 0.4);
                    let sweep = (if down { u } else { 1.0 - u } - 0.5) * 1.7 * d.rp;
                    for j in 0..4 {
                        let y = d.s.1 + sweep + (f64::from(j) - 1.5) * 4.5;
                        let half = d.rp * (0.95 - 0.12 * f64::from(j)) * big;
                        px::ink_line(
                            img,
                            (d.s.0 - half, y + 7.0),
                            (d.s.0 + half, y - 7.0),
                            2.0,
                            if j % 2 == 0 { bold::WHITE } else { bold::pale(p.col[k]) },
                            1.0,
                        );
                    }
                }
            }
        }
        // UKE-NADO: each tick splashes; the caught enemy whirls
        for e in bold::events_in(p, k, &SPOUT_HIT, t - SPLASH_S, t) {
            splash(
                img,
                cam,
                [e.f(5), e.f(6)],
                [0.0, 0.0],
                t - e.t,
                (e.t * 1000.0) as u64,
                8,
                1.2,
            );
        }
        if let Some(d) = foe {
            let up = self.sucked(p, t);
            if up > 0.0 {
                for i in 0..3 {
                    let a = t * 16.0 + f64::from(i) * dmath::TAU / 3.0;
                    px::ink_arc(img, d.s, d.rp * 1.28, 2.2, a, a + 1.2, LITE, up.min(1.0));
                }
            }
            // let go: the enemy flies off
            if let Some((age, e)) = bold::since(p, k, &["spout_end"], t, THROW_S)
                && bold::since(p, k, &SPOUT_HIT, e.t, SUCK_S).is_some()
            {
                let (a, b) = (p.run.p(p.sim_i(t - 0.06), o), p.run.p(p.sim_i(t), o));
                let (s0, s1) = (bold::at(cam, a[0], a[1], R), bold::at(cam, b[0], b[1], R));
                px::speed_lines(
                    img,
                    d.s,
                    d.rp,
                    (s1.0 - s0.0, s1.1 - s0.1),
                    22.0,
                    7,
                    bold::WHITE,
                    1.0 - age / THROW_S,
                );
            }
        }
        // the spout bursts as it ends
        if let Some((age, e)) = bold::since(p, k, &["spout_end"], t, SPLASH_S) {
            let seed = (e.t * 1000.0) as u64 + 7;
            splash(img, cam, [e.f(3), e.f(4)], [0.0, 0.0], age, seed, 18, 1.8);
        }
    }
}
