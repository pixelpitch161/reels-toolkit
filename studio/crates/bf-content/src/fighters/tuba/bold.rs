//! TUBA with the bold effects (`picture.effects = "bold"`; the owner's picks 29a to 29d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **OOM-PAH SWALLOW** (29a): the bell opens wide as a dark mouth with a thick lip, lanes of dashes and air
//!   streaks race into it and the enemy shrinks as it is drawn in; the gulp is a lump that travels down the bell
//!   and round the ball; the bell then swings round to where it will spit and a dashed gold line with a landing
//!   mark shows the aim; the pah is stacked brass bands from the mouth, and the enemy flies as a cannonball with
//!   a brass ring and speed lines into the rope.
//! - **OOMPAH** (29b): on top of the ring's speed-up look, big "oom" and small "pah" dots hop round the ball in
//!   turn on the half beat.
//! - **BRASS WALL** (29c): eight brass plates slide in and lock round the ball; the plates facing a blocked hit
//!   flash white; they tip over and drop away one by one as the shield runs down.
//! - **SUBSONIC** (29d): a landing mark and rings close in on the spot; the slab of floor sinks, showing its
//!   wall; on the slam it punches up as a raised slab (the enemy on it is thrown up), with lifted tiles racing
//!   out and chips flying.

use bf_core::dmath;
use bf_core::event::Event;
use bf_core::rng::Generator;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::FPS;
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Fx, Payload, Pic};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::BallState;
use bf_picture::scene::{self, A, R_BALL as R};

use super::{
    BRASS, BRASS_HI, Held, HeldPhase, PAH_COL, Pah, STREAK_COL, Sub, TubaLook, bayer, bell_rings, dot, draw_tuba_bell,
    inhale_light, mouth_of, ring_pts, sub, unit,
};

/// Seconds the bell takes to swing round to where it will spit, and back afterwards.
const TURN_S: f64 = 0.2;
/// Seconds before a gulp the enemy starts to shrink, and how small it is as the bell takes it.
const SHRINK_S: f64 = 0.25;
const SHRINK_TO: f64 = 0.78;
/// Seconds the lump takes down the bell, and then round the ball.
const LUMP_S: f64 = 0.16;
/// Seconds the pah's brass bands run.
const PAH_S: f64 = 0.45;
/// How far the bell opens in the bold picture (classic: 1).
const OPEN_K: f64 = 1.5;
/// BRASS WALL: the plates, where they stand and how tall they are.
const PLATES: usize = 8;
const PLATE_R: f64 = R * 1.6;
const PLATE_H: f64 = 1.25;
/// Seconds a plate takes to tip over, and then to go.
const TIP_S: f64 = 0.22;
const GONE_S: f64 = 0.2;
/// Seconds a plate flashes after a block.
const FLASH_S: f64 = 0.22;
/// SUBSONIC: the share of the build-up after which the slab sinks, how deep, how high it punches, and how long
/// the punch takes to settle.
const SINK_FROM: f64 = 0.5;
const SINK_DEPTH: f64 = 0.34;
const PUNCH_H: f64 = 0.6;
const PUNCH_S: f64 = 0.5;
/// The dark of the bell's mouth and of the slab's wall.
const MOUTH_DARK: V3 = [14.0, 8.0, 26.0];

/// One painter item of the bold look.
enum BItem {
    /// The bell (and the lump going down it).
    Bell,
    /// One plate of BRASS WALL: where it stands, the way out from the ball, the direction that runs level across
    /// the screen, its height, how far it has tipped over (0 upright, 1 flat) and its colour.
    Plate([f64; 2], [f64; 2], [f64; 2], f64, f64, V3),
    /// SUBSONIC's slab: its centre, radius, height (below 0: sunk) and how white it flashes.
    Slab(f64, f64, f64, f64, f64),
}

/// From angle `a` the short way round to `b`, `u` of the way.
fn turn_to(a: f64, b: f64, u: f64) -> f64 {
    a + ((b - a + dmath::PI).rem_euclid(dmath::TAU) - dmath::PI) * u
}

/// `a` to `b`, `u` of the way.
fn mix(a: V3, b: V3, u: f64) -> V3 {
    std::array::from_fn(|ch| a[ch] + (b[ch] - a[ch]) * u.clamp(0.0, 1.0))
}

/// The ring-frame direction that runs level across the screen, left to right, at `(x, z)`: what an upright card
/// there must be as wide along to face the camera.
fn screen_right(cam: &Cam, x: f64, z: f64) -> [f64; 2] {
    let o = bold::at(cam, x, z, 0.0);
    let a = bold::at(cam, x + 0.1, z, 0.0);
    let b = bold::at(cam, x, z + 0.1, 0.0);
    let (dx, dz) = (b.1 - o.1, -(a.1 - o.1));
    let n = dmath::hypot(dx, dz).max(1e-9);
    let s = if (a.0 - o.0) * dx + (b.0 - o.0) * dz < 0.0 {
        -1.0
    } else {
        1.0
    };
    [s * dx / n, s * dz / n]
}

/// Where a ball leaving `from` along `ang` meets the ropes.
fn rope_point(from: [f64; 2], ang: f64) -> [f64; 2] {
    let d = unit(ang);
    let lim = A - R;
    let mut s = f64::INFINITY;
    for ax in 0..2 {
        if d[ax].abs() > 1e-9 {
            let wall = if d[ax] > 0.0 { lim } else { -lim };
            s = s.min(((wall - from[ax]) / d[ax]).max(0.0));
        }
    }
    if s.is_finite() {
        [from[0] + d[0] * s, from[1] + d[1] * s]
    } else {
        from
    }
}

/// One plate of BRASS WALL: an upright brass card facing the camera, tipping outward as it drops.
#[allow(clippy::too_many_arguments)]
fn draw_plate(fr: &mut Frame, cam: &Cam, c: [f64; 2], out: [f64; 2], right: [f64; 2], h: f64, tip: f64, col: V3) {
    let hw = 0.3;
    let (sn, cs) = (dmath::sin(tip * dmath::PI / 2.0), dmath::cos(tip * dmath::PI / 2.0));
    let at = |s: f64, v: f64| {
        scene::to_world(
            c[0] + right[0] * hw * s + out[0] * h * v * sn,
            c[1] + right[1] * hw * s + out[1] * h * v * sn,
            0.04 + h * v * cs,
        )
    };
    bold::outlined_poly(
        fr,
        cam,
        &[
            at(-1.0, 0.0),
            at(1.0, 0.0),
            at(1.0, 0.8),
            at(0.55, 1.0),
            at(-0.55, 1.0),
            at(-1.0, 0.8),
        ],
        col,
    );
    // a lighter band and a rivet
    fr.line(cam, &[at(-0.7, 0.74), at(0.7, 0.74)], bold::pale(col), 1);
    fr.line(cam, &[at(-0.7, 0.16), at(0.7, 0.16)], bold::deep(col), 1);
    bold::matte_blob(fr, cam, at(0.0, 0.45), 0.05, bold::pale(col));
}

/// SUBSONIC's slab: a round piece of the floor at height `y`. Sunk, it shows the far wall of its hole; raised, its
/// near wall under a lit top. `white` flashes the top.
#[allow(clippy::too_many_arguments)]
fn draw_slab(fr: &mut Frame, cam: &Cam, cx: f64, cz: f64, rad: f64, y: f64, white: f64, col: V3) {
    let n = 32;
    let circle = |h: f64| -> Vec<V3> {
        (0..n)
            .map(|j| {
                let a = dmath::TAU * f64::from(j) / f64::from(n);
                scene::to_world(cx + rad * dmath::cos(a), cz + rad * dmath::sin(a), h)
            })
            .collect()
    };
    let rim = cam.wp(&circle(0.0));
    let dy = bold::at(cam, cx, cz, y).1 - bold::at(cam, cx, cz, 0.0).1;
    let yc = rim.iter().map(|q| q.1).sum::<f64>() / rim.len() as f64;
    let floor_col: V3 = std::array::from_fn(|ch| scene::CANVAS[ch] * 0.8 + col[ch] * 0.2);
    let wall = mix(MOUTH_DARK, col, 0.25);
    let mut closed = circle(0.0);
    closed.push(closed[0]);
    if dy > 0.0 {
        // sunk: the darker floor down in the hole, and the far wall above it
        fr.screen_poly(&rim, floor_col.map(|v| v * 0.9), None);
        let mut far: Vec<(f64, f64)> = rim.iter().copied().filter(|q| q.1 <= yc).collect();
        far.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut pts = far.clone();
        pts.extend(far.iter().rev().map(|q| (q.0, (q.1 + dy).min(2.0 * yc - q.1))));
        if pts.len() >= 6 {
            fr.screen_poly(&pts, wall, None);
        }
        fr.line(cam, &closed, bold::INK, 1);
    } else {
        // raised: the near wall, then the top
        let h = -dy;
        let top = circle(y);
        let mut near: Vec<(f64, f64)> = rim.iter().copied().filter(|q| q.1 >= yc).collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut pts = near.clone();
        pts.extend(near.iter().rev().map(|q| (q.0, q.1 - h)));
        if pts.len() >= 6 {
            fr.screen_poly(&pts, wall, None);
        }
        fr.line(cam, &closed, bold::INK, 1);
        // the wall's two upright edges, where the slab's outline turns
        let by_x = |a: &usize, b: &usize| rim[*a].0.total_cmp(&rim[*b].0);
        let ends = [(0..rim.len()).min_by(by_x), (0..rim.len()).max_by(by_x)];
        for i in ends.into_iter().flatten() {
            fr.line(cam, &[closed[i], top[i]], bold::INK, 1);
        }
        fr.poly(
            cam,
            &top,
            mix(floor_col.map(|v| (v * 1.35).min(255.0)), bold::WHITE, white),
            false,
        );
        let mut top_closed = top;
        top_closed.push(top_closed[0]);
        fr.line(cam, &top_closed, bold::INK, 1);
    }
}

impl TubaLook {
    /// The ride inside the bell that `t` belongs to, from its gulp until the bell has swung back after the
    /// spit, with the pah that ends it.
    fn ride(&self, t: f64, after: f64) -> Option<(&Held, &Pah)> {
        let h = self.held.iter().find(|h| h.t_gulp <= t && t < h.t_spit + after)?;
        let q = self.pahs.iter().find(|q| q.caught == 1 && q.t == h.t_spit)?;
        Some((h, q))
    }

    /// The bell's aim in the bold picture. It is the recorded aim `th`, but while the enemy rides inside, the
    /// bell swings round to where the pah will spit it (the fight spits toward the open ring, wherever the bell
    /// points), and swings back afterwards.
    fn bold_th(&self, th: f64, t: f64) -> f64 {
        let hold = 0.12;
        match self.ride(t, hold + TURN_S) {
            Some((h, q)) => {
                let u = if t < h.t_spit + hold {
                    bold::ease_out((t - h.t_gulp - 0.1) / TURN_S)
                } else {
                    1.0 - bold::ease_out((t - h.t_spit - hold) / TURN_S)
                };
                turn_to(th, q.angle, u)
            }
            None => th,
        }
    }

    /// The ball, the bold aim and the bell's mouth at `t`, from the recording.
    fn bold_mouth(&self, p: &PictureCtx, t: f64) -> ([f64; 2], f64, [f64; 2]) {
        let i = p.sim_i(t);
        let at = p.run.p(i, self.k);
        let th = self.bold_th(p.run.f("TH", i, self.k).unwrap_or(0.0), t);
        (at, th, mouth_of(at, th))
    }

    /// The SUBSONIC under way at `t` (from its call until `after` seconds past its slam).
    fn sub_at(&self, p: &PictureCtx, t: f64, after: f64) -> Option<&Sub> {
        self.subs
            .iter()
            .find(|s| s.t0 <= t && t < s.t_slam + after && s.t0 < p.t_hs)
    }

    /// The slab's height at `t`: it sinks through the second half of the build-up, punches up on the slam and
    /// settles.
    fn slab_y(s: &Sub, t: f64) -> f64 {
        if t < s.t_slam {
            let u = (t - s.t0) / (s.t_slam - s.t0).max(1e-6);
            -SINK_DEPTH * bold::ease_in((u - SINK_FROM) / (1.0 - SINK_FROM))
        } else {
            let v = t - s.t_slam;
            if v < 0.07 {
                -SINK_DEPTH + (PUNCH_H + SINK_DEPTH) * v / 0.07
            } else {
                PUNCH_H * (1.0 - bold::ease_in((v - 0.07) / (PUNCH_S - 0.07)))
            }
        }
    }

    /// The bold picture's changes to the balls: the bell's swing, the enemy shrinking as it is drawn in, and the
    /// enemy thrown up by SUBSONIC's slab.
    pub(super) fn bold_state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64) {
        let k = self.k;
        if kk == k {
            st.th = self.bold_th(st.th, t);
            return;
        }
        match self.held_phase(kk, t) {
            Some(HeldPhase::In(u)) => st.scale = SHRINK_TO - (SHRINK_TO - 0.25) * u,
            None => {
                if let Some(h) = self.held.iter().find(|h| h.t_gulp - SHRINK_S <= t && t < h.t_gulp) {
                    st.scale *= 1.0 - (1.0 - SHRINK_TO) * bold::ease_in((t - (h.t_gulp - SHRINK_S)) / SHRINK_S);
                }
            }
            _ => {}
        }
        if let Some((age, _)) = bold::since(p, k, &["hit:slam"], t, PUNCH_S) {
            st.lift += 0.75 * dmath::sin(dmath::PI * age / PUNCH_S);
        }
    }

    /// The classic sparks and puffs, from the bell where the bold picture has it (it has swung round to the
    /// spit).
    pub(super) fn bold_fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let k = self.k;
        let origin = up(0.0, 0.0, 0.0);
        for o in &self.ooms {
            if o.t0 > p.t_ko {
                break;
            }
            let (_, _, m) = self.bold_mouth(p, o.t0);
            fx.fl
                .add(o.t0, up(m[0], m[1], 0.8), p.fl_col[k], 1.8, 0.45, 0.03, 0.45, 0.0);
            let mut rng = Generator::new((o.t0 * 100.0) as u64);
            for j in 0..4 {
                let tj = o.t0 + (o.t_in - o.t0) * f64::from(j) / 4.0;
                if tj > p.t_ko {
                    break;
                }
                let (at, th, m) = self.bold_mouth(p, tj);
                let d = unit(th);
                let nrm = [-d[1], d[0]];
                for _ in 0..8 {
                    let out = R + o.reach * rng.uniform(0.5, 1.0);
                    let across = rng.uniform(-1.0, 1.0);
                    let src = [
                        at[0] + d[0] * out + nrm[0] * across * o.reach * 0.6,
                        at[1] + d[1] * out + nrm[1] * across * o.reach * 0.6,
                    ];
                    let to = [m[0] - src[0], m[1] - src[1]];
                    let pos = up(src[0], src[1], R * rng.uniform(0.5, 1.4));
                    let emit = Emit {
                        n: Some(1),
                        dir: sub(up(to[0], to[1], 0.0), origin),
                        col: Some(STREAK_COL.to_vec()),
                        ..Emit::default()
                    };
                    fx.pa.emit(tj, pos, "streaks", &emit);
                }
            }
        }
        for q in &self.pahs {
            let tp = q.t;
            if tp > p.t_ko {
                break;
            }
            let (_, th, m) = self.bold_mouth(p, tp);
            let pos = up(m[0], m[1], R);
            let d3 = sub(up(dmath::cos(th), dmath::sin(th), 0.0), origin);
            if q.caught != 0 {
                fx.bu.add(tp, pos, PAH_COL, 1.4, None, true);
                fx.fl.add(tp, pos, PAH_COL, 2.0, 0.9, 0.03, 0.3, 0.0);
                let puffs = Emit {
                    n: Some(14),
                    dir: d3,
                    ..Emit::default()
                };
                fx.pa.emit(tp, pos, "puffs", &puffs);
                fx.mk.add(
                    tp,
                    Mark {
                        size: 0.7,
                        life: 2.5,
                        seed: np::f64_to_i64(tp * 10.0).max(0) as u64,
                        strength: 0.6,
                        ..Mark::new(MarkKind::SmokeRing, m[0], m[1])
                    },
                );
                // the cannonball's trail, in the enemy's colour
                let vic = 1 - k;
                let trail = p.col[vic].map(|v| (v * 0.5 + 120.0).trunc());
                for j in 0..15 {
                    let tj = tp + f64::from(j) / f64::from(FPS);
                    if tj > p.t_ko {
                        break;
                    }
                    let at = p.run.p(p.sim_i(tj), vic);
                    let emit = Emit {
                        n: Some(1),
                        dir: [-d3[0], -d3[1], -d3[2]],
                        speed: Some((0.3, 0.6)),
                        col: Some(vec![trail]),
                        ..Emit::default()
                    };
                    fx.pa.emit(tj, up(at[0], at[1], R), "streaks", &emit);
                }
            } else {
                let puffs = Emit {
                    n: Some(5),
                    dir: d3,
                    ..Emit::default()
                };
                fx.pa.emit(tp, pos, "puffs", &puffs);
                fx.fl.add(tp, pos, PAH_COL, 1.4, 0.35, 0.03, 0.25, 0.0);
            }
        }
    }

    /// The suction cone, bolder, with lanes of dashes racing into the mouth.
    pub(super) fn bold_canvas_first(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let st = &c.fs[self.k];
        if !(c.in_fight && st.vis && c.t < p.t_ko) {
            return;
        }
        let Some((reach, kk)) = self.inhaling(c.t) else { return };
        let tint = [0.85, 0.85, 1.2];
        for (j, light) in ic.iter_mut().enumerate() {
            let b = bayer(c.xs[j], c.ys[j]);
            let (br, dk) = inhale_light(c.sxp[j], c.szp[j], st.p, st.th, reach, kk, c.t, b);
            let dim = 1.0 - 0.7 * dk;
            let lit = br * 1.9;
            for (v, &w) in light.iter_mut().zip(&tint) {
                *v *= dim;
                *v += lit * w;
            }
        }
        let m = mouth_of(st.p, st.th);
        let far = R * 2.0 + reach;
        for j in -2..=2 {
            let a = st.th + 0.3 * f64::from(j);
            let from = [st.p[0] + dmath::cos(a) * far, st.p[1] + dmath::sin(a) * far];
            floor::dashes(
                ic,
                c,
                from,
                m,
                0.06,
                0.42,
                c.t * 7.0 + f64::from(j) * 0.37,
                bold::light(bold::pale(p.col[self.k])),
                0.9 * kk,
            );
        }
    }

    /// The aim line while the enemy rides inside, the pah's brass bands, and SUBSONIC on the floor.
    pub(super) fn bold_canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let k = self.k;
        let gold = bold::light(bold::GOLD);
        // the aim: a dashed line from the bell to the rope the cannonball will hit, and a landing mark there
        if let Some((h, q)) = self.ride(c.t, 0.0)
            && c.t >= h.t_gulp + 0.12
            && c.t < p.t_ko
        {
            let u = (c.t - h.t_gulp - 0.12) / (h.t_spit - h.t_gulp - 0.12).max(1e-6);
            let at = c.fs[k].p;
            let d = unit(q.angle);
            let end = rope_point(at, q.angle);
            let from = [at[0] + d[0] * R * 1.2, at[1] + d[1] * R * 1.2];
            floor::dashes(ic, c, from, end, 0.09, 0.4, c.t * 6.0, gold, 0.6 + 0.5 * u);
            floor::target(ic, c, end[0], end[1], R * 0.9, u, gold);
        }
        // the pah: stacked brass bands burst from the mouth
        for q in &self.pahs {
            let u = (c.t - q.t) / PAH_S;
            if !(0.0..1.0).contains(&u) || q.t > p.t_ko {
                continue;
            }
            let (_, _, m) = self.bold_mouth(p, q.t);
            let (reach, k_ring) = if q.caught != 0 { (2.3, 1.2) } else { (1.0, 0.6) };
            floor::band(
                ic,
                c,
                Family::Brass,
                m[0],
                m[1],
                0.3 + reach * bold::ease_out(u),
                1.0 - u,
                c.t,
                bold::light(p.col[k]),
                k_ring,
            );
        }
        // SUBSONIC
        let Some(rad) = self.low_radius else { return };
        let Some(s) = self.sub_at(p, c.t, 0.9) else { return };
        let tint = bold::light(p.col[k]);
        if c.t < s.t_slam {
            // the note builds: a landing mark, rings closing in, and the slab going dark as it sinks
            let u = (c.t - s.t0) / (s.t_slam - s.t0).max(1e-6);
            floor::target(ic, c, s.cx, s.cz, rad, u, tint);
            for j in 0..3 {
                let ph = (u * 3.0 + f64::from(j) / 3.0).rem_euclid(1.0);
                floor::ring(
                    ic,
                    c,
                    s.cx,
                    s.cz,
                    rad * (1.0 + 1.1 * (1.0 - ph)),
                    0.07,
                    tint,
                    0.9 * dmath::sin(dmath::PI * ph),
                );
            }
        } else {
            // the slam: lifted tiles race out
            let v = (c.t - s.t_slam) / 0.9;
            for (lag, k_ring) in [(0.0, 1.3), (0.18, 0.8)] {
                let w = v - lag;
                if w > 0.0 {
                    floor::band(
                        ic,
                        c,
                        Family::Quake,
                        s.cx,
                        s.cz,
                        rad + 2.8 * bold::ease_out(w),
                        1.0 - w,
                        c.t,
                        tint,
                        k_ring,
                    );
                }
            }
        }
    }

    /// The bell, and BRASS WALL's plates round the ball.
    pub(super) fn bold_items_ball(&self, p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        let k = self.k;
        let mut out: Vec<(f64, Payload)> = Vec::new();
        if st.shield > 0.0
            && !st.shattered
            && let Some(beats) = self.shield_beats
        {
            let dur = (beats * p.show.grid.beat).max(1e-6);
            let age = dur - st.shield;
            let left = p.bold_shield_left(k, f.t).unwrap_or(st.shield / dur);
            let block = p.bold_last_block(k, f.t).filter(|&(tb, _, _)| f.t - tb < FLASH_S);
            let right = screen_right(f.cam, st.p[0], st.p[1]);
            for j in 0..PLATES {
                let a = dmath::TAU * (j as f64 + 0.5) / PLATES as f64;
                let dir = unit(a);
                // the plates slide in one after another and lock
                let lock = bold::ease_out((age - 0.015 * j as f64) / 0.16);
                // and drop one by one as the shield runs down, skipping round the ring
                let order = (j * 3) % PLATES;
                let mark = 1.0 - (order as f64 + 1.0) / (PLATES as f64 + 0.6);
                let fallen = (mark - left) * dur;
                if fallen > TIP_S + GONE_S {
                    continue;
                }
                let tip = bold::ease_in(fallen / TIP_S);
                let h = PLATE_H * (1.0 - ((fallen - TIP_S) / GONE_S).clamp(0.0, 1.0));
                // the plates facing a blocked hit flash white and give a little
                let flash = block.map_or(0.0, |(tb, x, z)| {
                    let to = dmath::atan2(z - st.p[1], x - st.p[0]);
                    let off = ((a - to + dmath::PI).rem_euclid(dmath::TAU) - dmath::PI).abs();
                    if off < 1.0 { 1.0 - (f.t - tb) / FLASH_S } else { 0.0 }
                });
                let locked = (1.0 - ((age - 0.16) / 0.1).abs()).max(0.0);
                let rr = PLATE_R + 0.9 * (1.0 - lock) - 0.1 * flash;
                let c = [st.p[0] + dir[0] * rr, st.p[1] + dir[1] * rr];
                let col = mix(mix(BRASS, BRASS_HI, locked), bold::WHITE, flash);
                let depth = f.cam.depth(&[scene::to_world(c[0], c[1], 0.3)])[0];
                out.push((depth, Box::new(BItem::Plate(c, dir, right, h, tip, col))));
            }
        }
        if !st.shattered && st.scale > 0.5 {
            let (rings, _) = bell_rings(st.p, st.th, 0.0, 0.0);
            out.push((f.cam.depth(&[rings[4].0])[0], Box::new(BItem::Bell)));
        }
        out
    }

    /// SUBSONIC's slab, under everything that stands in the ring.
    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let (Some(rad), Some(s)) = (self.low_radius, self.sub_at(p, f.t, PUNCH_S)) else {
            return Vec::new();
        };
        if !f.in_fight {
            return Vec::new();
        }
        let y = Self::slab_y(s, f.t);
        if y.abs() < 0.01 {
            return Vec::new();
        }
        let white = (1.0 - (f.t - s.t_slam) / 0.12).clamp(0.0, 1.0);
        let white = if f.t < s.t_slam { 0.0 } else { white };
        vec![(1e6, Box::new(BItem::Slab(s.cx, s.cz, rad, y, white)))]
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let k = self.k;
        let st = &f.fs[k];
        let (cam, t) = (f.cam, f.t);
        match payload.downcast_ref::<BItem>() {
            Some(BItem::Bell) => {
                let (ok, mut fl, gl) = if t < p.t_ko {
                    self.bell_state(t)
                } else {
                    (0.0, 0.0, 0.0)
                };
                if k == p.win && t >= p.t_hs {
                    // the champion's solo: a pah on every solo note
                    fl = 0.0;
                    for &ts in &p.solo_t {
                        if 0.0 <= t - ts {
                            fl = fl.max((1.0 - (t - ts) / 0.25).max(0.0) * 0.7);
                        }
                    }
                }
                let ok = ok * OPEN_K;
                draw_tuba_bell(p, fr, cam, f.l, st.p, st.th, ok, fl, p.col[k], gl);
                // the mouth: dark inside while it inhales, with a thick outlined lip
                let (rings, axis) = bell_rings(st.p, st.th, ok, fl);
                let (mouth_c, rm) = rings[4];
                if dot(axis, sub(cam.pos, mouth_c)) > 0.0 && ok > 0.05 {
                    fr.poly(cam, &ring_pts(mouth_c, axis, rm * 0.9, 20), MOUTH_DARK, true);
                }
                let mut lip = ring_pts(mouth_c, axis, rm, 20);
                lip.push(lip[0]);
                bold::thick_line(fr, cam, &lip, BRASS_HI, 3);
                // the gulp: a lump goes down the bell
                if let Some((h, _)) = self.ride(t, 0.0) {
                    let u = (t - h.t_gulp - 0.08) / LUMP_S;
                    if (0.0..1.0).contains(&u) {
                        let (a, b) = (mouth_c, rings[0].0);
                        let w: V3 = std::array::from_fn(|ch| a[ch] + (b[ch] - a[ch]) * u);
                        bold::blob(fr, cam, w, 0.36 - 0.14 * u, BRASS);
                    }
                }
            }
            Some(&BItem::Plate(c, out, right, h, tip, col)) => draw_plate(fr, cam, c, out, right, h, tip, col),
            Some(&BItem::Slab(cx, cz, rad, y, white)) => draw_slab(fr, cam, cx, cz, rad, y, white, p.col[k]),
            None => {}
        }
    }

    /// The pah's blast in the lens, from the bell where the bold picture has it.
    pub(super) fn bold_shimmer_after(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64) {
        for s in &self.subs {
            if s.t_slam <= t && t < s.t_slam + 1.0 && s.t_slam < p.t_hs {
                let u = t - s.t_slam;
                let c = scene::to_world(s.cx, s.cz, 0.05);
                img.shimmer_world(cam, c, 0.2 + 3.2 * u, 0.4, 3.0, t, 1.0 - u, 0.0);
            }
        }
        for q in &self.pahs {
            if q.caught != 0 && 0.0 <= t - q.t && t - q.t < 0.35 && q.t < p.t_ko {
                let (_, _, m) = self.bold_mouth(p, q.t);
                let c = scene::to_world(m[0], m[1], R);
                let k_ = 1.0 - (t - q.t) / 0.35;
                img.shimmer_world(cam, c, 0.3 + 2.0 * (t - q.t), 0.25, 2.0, t, k_, 0.0);
            }
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight || f.t >= p.t_ko {
            return;
        }
        let k = self.k;
        let o = 1 - k;
        let (cam, t) = (f.cam, f.t);
        let st = &f.fs[k];
        let me = drawn.iter().find(|d| d.k == k);
        let foe = drawn.iter().find(|d| d.k == o);
        let col = p.col[k];
        // the inhale: streaks of air race into the mouth
        if let Some((reach, kk)) = self.inhaling(t)
            && st.vis
        {
            let m = mouth_of(st.p, st.th);
            for j in 0..7u64 {
                let a = st.th + (px::rnd(j, 3) - 0.5) * 1.3;
                let ph = (t * 2.6 + px::rnd(j, 7)).rem_euclid(1.0);
                let d0 = 0.25 + (R + reach) * (1.0 - ph);
                let d1 = d0 + 0.2 + 0.45 * (1.0 - ph);
                let y = R * (0.5 + 0.8 * px::rnd(j, 11));
                let at = |d: f64| bold::at(cam, m[0] + dmath::cos(a) * d, m[1] + dmath::sin(a) * d, y);
                px::ink_line(img, at(d1), at(d0), 1.6, bold::WHITE, kk * (ph * 6.0).min(1.0));
            }
        }
        // the gulp: after the lump has gone down the bell it runs round the ball's rim, both ways, to the far side
        if let (Some((h, _)), Some(d)) = (self.ride(t, 0.0), me) {
            let u = (t - h.t_gulp - 0.08 - LUMP_S) / LUMP_S;
            if (0.0..1.0).contains(&u) {
                let m = mouth_of(st.p, st.th);
                let ms = bold::at(cam, m[0], m[1], R);
                let a0 = dmath::atan2(ms.1 - d.s.1, ms.0 - d.s.0);
                let (go, span) = (dmath::PI * 0.95 * u, 0.55 * (1.0 - 0.5 * u));
                for side in [-1.0, 1.0] {
                    let mid = a0 + side * go;
                    px::ink_arc(
                        img,
                        d.s,
                        d.rp + 3.0,
                        4.0,
                        mid - span / 2.0,
                        mid + span / 2.0,
                        BRASS_HI,
                        1.0,
                    );
                }
            }
        }
        // the cannonball: a brass ring round the spat enemy and speed lines behind it, until it meets the rope
        if let Some(d) = foe
            && let Some((age, q)) = bold::since(p, k, &["pah"], t, 0.9)
            && q.i(3) == 1
            && bold::events_in(p, k, &["hit:cannon"], q.t, t).next().is_none()
        {
            let prev = p.run.p(p.sim_i((t - 2.0 / f64::from(FPS)).max(q.t)), o);
            let now = f.fs[o].p;
            let (s0, s1) = (bold::at(cam, prev[0], prev[1], R), bold::at(cam, now[0], now[1], R));
            let fade = (1.0 - age / 0.9).clamp(0.0, 1.0);
            px::speed_lines(
                img,
                d.s,
                d.rp,
                (s1.0 - s0.0, s1.1 - s0.1),
                26.0,
                7,
                bold::WHITE,
                0.9 * fade,
            );
            px::ink_ring(img, d.s, d.rp + 3.0, 2.4, bold::GOLD, 0.5 + 0.5 * fade);
        }
        // OOMPAH: "oom" and "pah" dots hop round the ball in turn
        if let Some(d) = me
            && st.haste > 0.0
            && st.vis
        {
            let g = &p.show.grid;
            let half = (t - g.t_fight) / g.beat * 2.0;
            for j in 0..6u32 {
                let oom = j % 2 == 0;
                let a = dmath::TAU * f64::from(j) / 6.0 + t * 1.2;
                // the ooms hop on one half beat, the pahs on the other
                let ph = (half + if oom { 0.0 } else { 1.0 }).rem_euclid(2.0);
                let hop = if ph < 1.0 { dmath::sin(dmath::PI * ph) } else { 0.0 };
                let c = (
                    d.s.0 + dmath::cos(a) * (d.rp + 17.0),
                    d.s.1 + dmath::sin(a) * (d.rp + 17.0) * 0.5 + d.rp * 0.35 - 14.0 * hop,
                );
                if oom {
                    px::ink_disc(img, c, 5.5, bold::pale(col), 1.0);
                } else {
                    px::ink_disc(img, c, 3.4, bold::GOLD, 1.0);
                }
            }
        }
        // SUBSONIC's punch: chips fly off the slab's rim and lines shoot up over it
        if let (Some(rad), Some(s)) = (self.low_radius, self.sub_at(p, t, PUNCH_S))
            && t >= s.t_slam
        {
            let v = t - s.t_slam;
            let c = bold::at(cam, s.cx, s.cz, 0.1);
            px::chips(
                img,
                c,
                v,
                PUNCH_S,
                14,
                70.0,
                (s.t_slam * 1000.0) as u64,
                bold::pale(col),
            );
            let right = screen_right(cam, s.cx, s.cz);
            let u = v / 0.3;
            if u < 1.0 {
                for j in -2..=2i32 {
                    let off = f64::from(j) * rad * 0.42;
                    let (x, z) = (s.cx + right[0] * off, s.cz + right[1] * off);
                    let lo = PUNCH_H + 0.2 + 1.1 * bold::ease_out(u);
                    let len = 0.5 * (1.0 - 0.3 * f64::from(j.abs()));
                    px::ink_line(
                        img,
                        bold::at(cam, x, z, lo),
                        bold::at(cam, x, z, lo + len),
                        2.0,
                        bold::WHITE,
                        1.0 - u,
                    );
                }
            }
        }
    }

    /// The cannonball into the rope and SUBSONIC's slam are heavy hits; the gulp and the spit are solid ones
    /// whatever their damage.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match bold::how(e) {
            "cannon" | "slam" => Some(HitSize::Heavy),
            "gulp" | "spit" => Some(by_damage.max(HitSize::Solid)),
            _ => Some(by_damage),
        }
    }
}
