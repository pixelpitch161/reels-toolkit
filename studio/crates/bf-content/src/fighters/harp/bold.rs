//! HARP with the bold effects (`picture.effects = "bold"`; the owner's picks 9a to 9d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **String trap** (9a): a new string fires across thick and bright, is plucked tight and gets a gold peg at
//!   each rope; a waiting string is a clean outlined line with a glint travelling along it; an enemy crossing
//!   one bends it round its front, then it snaps back with a wide twang and notes pop off.
//! - **PLUCK** (9b): seven strings show on the ball and are plucked one after another as seven gold teardrop
//!   darts fan out; a dart that lands pops in a gold ring.
//! - **ANGEL WINGS** (9c): two big outlined wings and a halo fold down round the ball; the wing that takes a
//!   blocked hit flares white and sheds feathers; the wings lose feathers and shrink as the shield runs down.
//! - **47 STRINGS** (9d): fine strings rise right across the canvas and light up in a run just before the
//!   wave; the wave rolls out as three string rings, and the struck enemy flies off with speed lines.

use bf_core::dmath::{self, PI};
use bf_core::event::Event;
use bf_core::roster::AbilityKind;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::scene::{self, R_BALL as R};

use super::{DRAW_IN, FEATHER, FEATHER_DK, GOLD, HarpLook, STR_Y, Str, how, ss};

/// Seconds a crossed string stays bent round the enemy after the hit, before it snaps back.
const BEND_S: f64 = 0.12;
/// Seconds before the crossing the string may already be pushed by the enemy's front.
const PUSH_S: f64 = 0.25;
/// Seconds a blocked hit flares its wing.
const FLARE_S: f64 = 0.25;
/// Seconds the shed feathers fall.
const SHED_S: f64 = 0.7;
/// Seconds before 47 STRINGS its strings rise from the canvas, and seconds after it they are gone.
const RISE_S: f64 = 0.4;
const SINK_S: f64 = 0.5;
/// Seconds a landed dart's ring and a crossing's notes show.
const POP_S: f64 = 0.22;
const NOTES_S: f64 = 0.55;

/// One painter item of the bold look.
enum BItem {
    /// A string.
    Str(Box<BStr>),
    /// A dart: x, z, heading.
    Dart(f64, f64, f64),
    /// ANGEL WINGS at strength `k`.
    Wings(f64),
    /// The halo at strength `k`.
    Halo(f64),
}

/// A string as the bold look draws it.
struct BStr {
    pts: Vec<V3>,
    /// 1, or less while a dropped string goes.
    alpha: f64,
    /// How bright and thick: 1 as it is laid, bent or twanging hard, 0 at rest.
    hot: f64,
    /// Its pegs on the ropes.
    pegs: Vec<V3>,
    /// The glint travelling along a waiting string.
    glint: Option<V3>,
}

/// A little note: a head, a stem and a flag, `s` the head's radius.
fn note(img: &mut Pic, c: (f64, f64), s: f64, col: V3, a: f64) {
    let top = (c.0 + s * 0.8, c.1 - s * 3.2);
    px::ink_line(img, (c.0 + s * 0.8, c.1), top, 1.0, col, a);
    px::ink_line(img, top, (top.0 + s * 1.3, top.1 + s * 1.1), 1.2, col, a);
    px::ink_disc(img, c, s, col, a);
}

/// A teardrop dart at `(x, z)` flying along `ang`: round in front, a point behind.
fn teardrop(x: f64, z: f64, ang: f64, y: f64, k: f64) -> Vec<V3> {
    let (cs, sn) = (dmath::cos(ang), dmath::sin(ang));
    let at = |fx: f64, fz: f64| scene::to_world(x + (fx * cs - fz * sn) * k, z + (fx * sn + fz * cs) * k, y);
    let mut pts = vec![at(-0.42, 0.0)];
    for j in 0..=8 {
        let a = (-125.0 + 250.0 * j as f64 / 8.0).to_radians();
        pts.push(at(0.08 + 0.17 * dmath::cos(a), 0.17 * dmath::sin(a)));
    }
    pts
}

impl HarpLook {
    /// The last time at or before `t + ahead` this side fired an ability of `kind` (the recording's own name
    /// for it: `arc`, `wave`).
    fn fired(&self, p: &PictureCtx, kind: &str, t: f64, ahead: f64) -> Option<f64> {
        p.rec
            .events
            .iter()
            .rev()
            .find(|e| e.kind == "ability" && e.who() == Some(self.k) && e.t <= t + ahead && e.s(4) == kind)
            .map(|e| e.t)
    }

    /// One string at `t` with the enemy at `foe`.
    fn bold_string(&self, p: &PictureCtx, s: &Str, t: f64, foe: [f64; 2]) -> Option<BStr> {
        let (alpha, drawn) = Self::string_state(s, t)?;
        let (a, b) = (s.a, s.b);
        let d = [b[0] - a[0], b[1] - a[1]];
        let len = dmath::hypot(d[0], d[1]) + 1e-9;
        let dir = [d[0] / len, d[1] / len];
        let nrm = [-dir[1], dir[0]];
        let o = 1 - self.k;
        let side_of = |q: [f64; 2]| (q[0] - a[0]) * nrm[0] + (q[1] - a[1]) * nrm[1];
        // a new string is plucked tight
        let since_laid = t - s.t0 - DRAW_IN;
        let mut wave = 0.0;
        let mut hot = (1.0 - (t - s.t0) / 0.5).max(0.0);
        if (0.0..0.5).contains(&since_laid) {
            wave += 0.16 * dmath::exp(-since_laid / 0.13) * dmath::sin(2.0 * PI * 9.0 * since_laid);
        }
        // a crossing: pushed by the enemy's front, held round it, then the snap back and the twang
        let mut bend: Option<(f64, f64)> = None;
        for &th in &s.hits {
            if !(th - PUSH_S..th + BEND_S + 0.8).contains(&t) {
                continue;
            }
            let way = side_of(p.run.p(p.sim_i(th + 0.08), o)) - side_of(p.run.p(p.sim_i(th - 0.08), o));
            let sign = if way < 0.0 { -1.0 } else { 1.0 };
            if t < th + BEND_S {
                let push = side_of(foe) + sign * R * 0.95;
                if push * sign > 0.0 {
                    let along = (foe[0] - a[0]) * dir[0] + (foe[1] - a[1]) * dir[1];
                    bend = Some((along, push.clamp(-1.3, 1.3)));
                    hot = 1.0;
                }
            } else {
                let dt = t - th - BEND_S;
                let ring = dmath::exp(-dt / 0.2);
                wave += sign * 0.32 * ring * dmath::cos(2.0 * PI * 8.0 * dt);
                hot = hot.max(ring);
            }
        }
        let n = 33;
        let pts = (0..n)
            .map(|j| {
                let u = j as f64 / (n - 1) as f64 * drawn;
                let mut disp = wave * dmath::sin(PI * u);
                if let Some((along, push)) = bend {
                    let w = (1.0 - (u * len - along).abs() / 1.5).max(0.0);
                    disp += push * w * w * (3.0 - 2.0 * w);
                }
                scene::to_world(a[0] + d[0] * u + nrm[0] * disp, a[1] + d[1] * u + nrm[1] * disp, STR_Y)
            })
            .collect::<Vec<V3>>();
        let mut pegs = vec![scene::to_world(a[0], a[1], STR_Y)];
        if drawn >= 1.0 {
            pegs.push(scene::to_world(b[0], b[1], STR_Y));
        }
        let waiting = drawn >= 1.0 && alpha >= 1.0 && hot < 0.1;
        let glint = waiting.then(|| {
            let u = (t * 0.6 + s.id as f64 * 0.37).rem_euclid(1.0);
            pts[((u * (n - 1) as f64) as usize).min(n - 1)]
        });
        Some(BStr {
            pts,
            alpha,
            hot,
            pegs,
            glint,
        })
    }

    pub(super) fn bold_items_ball(&self, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        let sk = self.shield_k(st);
        if sk <= 0.0 {
            return Vec::new();
        }
        let dz = f.cam.depth(&[scene::to_world(st.p[0], st.p[1], R)])[0];
        let wings: Payload = Box::new(BItem::Wings(sk));
        let halo: Payload = Box::new(BItem::Halo(sk));
        vec![(dz + 0.05, wings), (dz - 0.05, halo)]
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let mut out: Vec<(f64, Payload)> = Vec::new();
        let tc = p.fx_clock(f.t);
        let fade = if f.t >= p.t_ko {
            1.0 - ss(p.t_hs + 0.5, p.t_hs + 1.1, f.t)
        } else {
            1.0
        };
        if !self.strings.is_empty() && fade > 0.0 && tc >= p.show.grid.t_fight {
            let foe = f.fs[1 - self.k].p;
            let j1 = self.str_t.partition_point(|&v| v <= tc);
            for s in &self.strings[j1.saturating_sub(12)..j1] {
                let Some(mut bs) = self.bold_string(p, s, tc, foe) else {
                    continue;
                };
                bs.alpha *= fade;
                let mut m = [0.0; 3];
                for q in &bs.pts {
                    for i in 0..3 {
                        m[i] += q[i] / bs.pts.len() as f64;
                    }
                }
                out.push((f.cam.depth(&[m])[0], Box::new(BItem::Str(Box::new(bs)))));
            }
        }
        if f.in_fight
            && let Some(rows) = p.run.frame_values("K", f.i)
        {
            for row in rows.as_chunks::<7>().0 {
                if row[0].is_nan() || row[3] as i64 != self.k as i64 {
                    continue;
                }
                let d = f.cam.depth(&[scene::to_world(row[0], row[1], 0.45)])[0];
                out.push((d, Box::new(BItem::Dart(row[0], row[1], row[2]))));
            }
        }
        out
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let col = p.col[self.k];
        let at = f.fs[self.k].p;
        match payload.downcast_ref::<BItem>() {
            Some(BItem::Str(s)) => {
                if s.alpha < 1.0 {
                    // a dropped string goes thin, then is gone
                    if s.alpha > 0.35 {
                        fr.line(f.cam, &s.pts, col.map(|c| c * 0.8), 1);
                    }
                    return;
                }
                if s.hot > 0.05 {
                    let c: V3 = std::array::from_fn(|i| col[i] + (bold::WHITE[i] - col[i]) * s.hot);
                    bold::thick_line(fr, f.cam, &s.pts, c, 3);
                } else {
                    fr.line(f.cam, &s.pts, bold::INK, 3);
                    fr.line(f.cam, &s.pts, col, 1);
                }
                for peg in &s.pegs {
                    bold::blob(fr, f.cam, *peg, 0.075, GOLD);
                }
                if let Some(g) = s.glint {
                    bold::blob(fr, f.cam, g, 0.05, bold::WHITE);
                }
            }
            Some(&BItem::Dart(x, z, ang)) => {
                // a short streak behind it, then the gold drop with a bright core
                let (cs, sn) = (dmath::cos(ang), dmath::sin(ang));
                fr.line(
                    f.cam,
                    &[
                        scene::to_world(x - cs * 0.85, z - sn * 0.85, 0.45),
                        scene::to_world(x - cs * 0.3, z - sn * 0.3, 0.45),
                    ],
                    bold::pale(GOLD),
                    1,
                );
                bold::outlined_poly(fr, f.cam, &teardrop(x, z, ang, 0.45, 1.0), GOLD);
                fr.poly(
                    f.cam,
                    &teardrop(x + cs * 0.04, z + sn * 0.04, ang, 0.455, 0.45),
                    bold::WHITE,
                    true,
                );
            }
            Some(&BItem::Wings(k)) => {
                let (mut arr, depth) = (fr.array(), fr.depth());
                self.bold_wings(p, &mut arr, f.cam, at, f.t, k);
                fr.set_array(&arr, Some(&depth));
            }
            Some(&BItem::Halo(k)) => {
                let (mut arr, depth) = (fr.array(), fr.depth());
                let y = R * 2.4 + 0.05 * dmath::sin(2.0 * PI * f.t / 0.95);
                let pts: Vec<V3> = (0..29)
                    .map(|j| {
                        let a = 2.0 * PI * j as f64 / 28.0;
                        scene::to_world(at[0] + 0.46 * dmath::cos(a), at[1] + 0.46 * dmath::sin(a), y)
                    })
                    .collect();
                let c = if self.flare(p, f.t).is_some() {
                    bold::WHITE
                } else {
                    GOLD
                };
                px::ink_polyline(&mut arr, &f.cam.wp(&pts), 2.2, c, k.clamp(0.0, 1.0));
                fr.set_array(&arr, Some(&depth));
            }
            None => {}
        }
    }

    /// The blocked hit that is flaring a wing at `t`: its age and where it struck (ring frame).
    fn flare(&self, p: &PictureCtx, t: f64) -> Option<(f64, [f64; 2])> {
        let (tb, x, z) = p.bold_last_block(self.k, t)?;
        (t - tb < FLARE_S).then_some((t - tb, [x, z]))
    }

    /// Which wing (-1 screen left, +1 right) a hit at ring point `at` struck, the ball at `c`.
    fn wing_side(cam: &Cam, c: [f64; 2], at: [f64; 2]) -> f64 {
        if bold::at(cam, at[0], at[1], R).0 < bold::at(cam, c[0], c[1], R).0 {
            -1.0
        } else {
            1.0
        }
    }

    /// The two wings: outlined feathers that fold down round the ball as the shield goes up, thin out as it runs
    /// down, and flare white on the side a blocked hit struck.
    fn bold_wings(&self, p: &PictureCtx, arr: &mut Img<f32>, cam: &Cam, c: [f64; 2], t: f64, k: f64) {
        let left = p.bold_shield_left(self.k, t).unwrap_or(1.0);
        let dur = self.shield.as_ref().map_or(1.0, |s| s.1);
        let fold = bold::ease_out(dur * (1.0 - left) / 0.4);
        let flare = self
            .flare(p, t)
            .map(|(age, at)| (1.0 - age / FLARE_S, Self::wing_side(cam, c, at)));
        let flap = 0.12 * dmath::sin(2.0 * PI * t / 0.95);
        let base = scene::to_world(c[0], c[1], R * 1.1);
        let bs = cam.wp(&[base])[0];
        let rp = R * cam.f / cam.depth(&[base])[0];
        // six feathers a wing at first, down to three as the shield runs out
        let n = (3.0 + (3.0 * left).ceil()).clamp(3.0, 6.0) as usize;
        for side in [-1.0, 1.0] {
            let lit = flare.filter(|&(_, s)| s == side).map_or(0.0, |(v, _)| v);
            for j in (0..n).rev() {
                let jf = j as f64;
                // spread high as they open, then folded down round the ball
                let open = (38.0 + 24.0 * jf).to_radians();
                let rest = (-20.0 + 21.0 * jf).to_radians();
                let a = open + (rest - open) * fold + flap * (1.0 + 0.25 * jf);
                let l = rp
                    * 2.6
                    * [1.0, 1.15, 1.1, 0.95, 0.8, 0.65][j]
                    * (0.4 + 0.6 * k)
                    * (0.72 + 0.28 * left)
                    * (1.0 + 0.18 * lit);
                let w = rp * 0.46;
                let d = [side * dmath::cos(a), -dmath::sin(a)];
                let nn = [-d[1], d[0]];
                let o = (bs.0 + side * rp * 0.5, bs.1 - rp * 0.15);
                let pt = |along: f64, across: f64| {
                    (o.0 + d[0] * along + nn[0] * across, o.1 + d[1] * along + nn[1] * across)
                };
                let pts = [
                    pt(0.0, w * 0.22),
                    pt(0.0, -w * 0.22),
                    pt(l * 0.6, -w * 0.5),
                    pt(l * 0.9, -w * 0.38),
                    pt(l, 0.0),
                    pt(l * 0.9, w * 0.38),
                    pt(l * 0.6, w * 0.5),
                ];
                let col = if lit > 0.0 {
                    bold::WHITE
                } else if j % 2 == 0 {
                    FEATHER
                } else {
                    FEATHER_DK
                };
                px::ink_poly(arr, &pts, col, 1.0);
                // the quill
                px::line(
                    arr,
                    pt(l * 0.1, 0.0),
                    pt(l * 0.8, 0.0),
                    1.0,
                    FEATHER_DK,
                    if lit > 0.0 { 0.0 } else { 0.8 },
                );
            }
        }
    }

    /// The light on the canvas: 47 STRINGS's strings rise right across the ring and light up in a run.
    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight || c.t >= p.t_hs {
            return;
        }
        let Some(te) = self.fired(p, "wave", c.t, RISE_S) else {
            return;
        };
        let dt = c.t - te;
        if dt >= SINK_S {
            return;
        }
        let rise = if dt < 0.0 {
            bold::ease_out(1.0 + dt / RISE_S)
        } else {
            1.0 - dt / SINK_S
        };
        // the run of light: from one side of the ring to the other, ending as the wave starts
        let run_x = -3.6 + 7.2 * ((dt + RISE_S * 0.75) / (RISE_S * 0.75)).clamp(0.0, 1.0);
        let tint = bold::light(GOLD);
        for (j, light) in ic.iter_mut().enumerate() {
            let x = c.sxp[j];
            let q = (x + 3.6) / 7.2 * 47.0;
            if q - q.floor() > 0.38 {
                continue;
            }
            let lit = (1.0 - (x - run_x).abs() / 0.9).max(0.0);
            let behind = if x < run_x { 0.45 } else { 0.22 };
            let v = rise * (behind + 0.9 * lit);
            for ch in 0..3 {
                light[ch] += v * tint[ch];
            }
        }
    }

    /// The wave: three string rings, one after another.
    pub(super) fn bold_canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        let tint = bold::light(p.col[w.owner]);
        floor::wave(ic, c, Family::Strings, w, c.t, tint, 1.6);
        let rmax = if w.rmax > 0.0 { w.rmax } else { 1.0 };
        for (n, back) in [0.75, 1.5].into_iter().enumerate() {
            let r = w.r - back;
            if r > 0.15 {
                let fade = (1.0 - r / rmax) * (0.9 - 0.15 * n as f64);
                floor::band(
                    ic,
                    c,
                    Family::Strings,
                    w.cx,
                    w.cz,
                    r,
                    fade,
                    c.t + 1.7 * (n + 1) as f64,
                    tint,
                    1.4,
                );
            }
        }
    }

    /// On the finished picture: the plucked strings on the ball, the darts' pops, the crossings' notes, the shed
    /// feathers and the speed lines of the enemy the wave knocks back.
    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight || f.t >= p.t_hs {
            return;
        }
        let k = self.k;
        let me = drawn.iter().find(|d| d.k == k);
        // PLUCK: seven strings across the ball, plucked in order as the darts leave
        if let Some(d) = me
            && let Some(t0) = self.fired(p, "arc", f.t, 0.0)
        {
            let cfg = &p.cfg[k];
            let ab = [&cfg.unlock1, &cfg.unlock2, &cfg.low]
                .into_iter()
                .find(|a| a.kind == AbilityKind::Arc);
            let n = ab.and_then(|a| a.get("n")).unwrap_or(7.0).clamp(2.0, 12.0) as usize;
            let over = ab.and_then(|a| a.get("beats")).unwrap_or(1.0) * p.show.grid.beat;
            let dt = f.t - t0;
            if dt < over + 0.3 {
                let out = (1.0 - (dt - over) / 0.3).clamp(0.0, 1.0);
                for j in 0..n {
                    let xn = -0.66 + 1.32 * j as f64 / (n - 1) as f64;
                    let half = d.rp * 0.92 * (1.0 - xn * xn).max(0.0).sqrt();
                    let x = d.s.0 + d.rp * xn;
                    let since = dt - over * j as f64 / (n - 1) as f64;
                    if (0.0..0.2).contains(&since) {
                        // plucked: it bows out and rings white
                        let amp = d.rp * 0.3 * dmath::exp(-since / 0.07) * dmath::cos(since * 70.0);
                        let pts: Vec<(f64, f64)> = (0..=6)
                            .map(|i| {
                                let u = i as f64 / 6.0;
                                (x + amp * dmath::sin(PI * u), d.s.1 - half + 2.0 * half * u)
                            })
                            .collect();
                        px::ink_polyline(img, &pts, 1.6, bold::WHITE, out);
                    } else {
                        let col = if since < 0.0 { GOLD } else { bold::pale(GOLD) };
                        px::line(img, (x, d.s.1 - half), (x, d.s.1 + half), 1.0, col, 0.9 * out);
                    }
                }
            }
        }
        for e in p.rec.events.iter().filter(|e| {
            e.kind == "hit" && e.who() == Some(k) && e.t <= f.t && f.t - e.t < NOTES_S && e.fields.len() > 5
        }) {
            let age = f.t - e.t;
            match e.s(7) {
                // a dart that lands pops: a gold ring and four sparks
                "key" if age < POP_S => {
                    let u = age / POP_S;
                    let c = bold::at(f.cam, e.f(5), e.f(6), 0.45);
                    let r = 4.0 + 11.0 * bold::ease_out(u);
                    px::ink_ring(img, c, r, 1.5, GOLD, 1.0 - u);
                    for q in 0..4 {
                        let a = PI / 4.0 + PI / 2.0 * q as f64;
                        let (dx, dy) = (dmath::cos(a), dmath::sin(a));
                        px::line(
                            img,
                            (c.0 + dx * r * 1.2, c.1 + dy * r * 1.2),
                            (c.0 + dx * r * 1.7, c.1 + dy * r * 1.7),
                            1.4,
                            bold::WHITE,
                            1.0 - u,
                        );
                    }
                }
                // a crossing: after the snap three notes pop off the string
                "string" if age >= BEND_S => {
                    let u = (age - BEND_S) / (NOTES_S - BEND_S);
                    let c = bold::at(f.cam, e.f(5), e.f(6), R * 1.3);
                    for (q, lean) in [-0.75, 0.0, 0.75].into_iter().enumerate() {
                        let far = 9.0 + (24.0 + 5.0 * q as f64) * bold::ease_out(u);
                        let at = (c.0 + dmath::sin(lean) * far, c.1 - dmath::cos(lean) * far);
                        let col = if q == 1 { bold::WHITE } else { GOLD };
                        note(img, at, 2.3, col, (1.6 - 1.6 * u).min(1.0));
                    }
                }
                _ => {}
            }
        }
        // the wing that took the hit sheds feathers
        if let Some(d) = me
            && let Some((tb, x, z)) = p.bold_last_block(k, f.t)
            && f.t - tb < SHED_S
        {
            let u = (f.t - tb) / SHED_S;
            let side = Self::wing_side(f.cam, f.fs[k].p, [x, z]);
            let from = (d.s.0 + side * d.rp * 1.5, d.s.1 - d.rp * 0.7);
            for j in 0..5 {
                let jf = j as f64;
                let c = (
                    from.0 + side * (6.0 + 5.0 * jf) * u * 2.4 + 3.0 * dmath::sin(u * 9.0 + jf),
                    from.1 - (16.0 - 5.0 * jf) * u + 52.0 * u * u,
                );
                let a = jf * 1.3 + u * (4.0 + jf);
                let (dx, dy) = (dmath::cos(a) * 5.0, dmath::sin(a) * 5.0);
                let (nx, ny) = (-dy * 0.32, dx * 0.32);
                px::ink_poly(
                    img,
                    &[
                        (c.0 - dx, c.1 - dy),
                        (c.0 + nx, c.1 + ny),
                        (c.0 + dx, c.1 + dy),
                        (c.0 - nx, c.1 - ny),
                    ],
                    FEATHER,
                    (1.5 - 1.5 * u).min(1.0),
                );
            }
        }
        // the enemy the wave knocks back: speed lines behind it
        let i = p.sim_i(f.t.min(p.t_ko));
        if let Some((age, _)) = bold::since(p, k, &["hit:wave"], f.t, 0.3)
            && i >= 3
            && let Some(d) = drawn.iter().find(|d| d.k == 1 - k)
        {
            let (now, was) = (p.run.p(i, 1 - k), p.run.p(i - 3, 1 - k));
            let (dx, dz) = (now[0] - was[0], now[1] - was[1]);
            let n = dmath::hypot(dx, dz);
            if n > 0.15 {
                let a = bold::at(f.cam, now[0], now[1], R);
                let b = bold::at(f.cam, now[0] - dx / n, now[1] - dz / n, R);
                let sn = dmath::hypot(b.0 - a.0, b.1 - a.1).max(1e-9);
                let (ux, uy) = ((b.0 - a.0) / sn, (b.1 - a.1) / sn);
                let fade = 1.0 - age / 0.3;
                for side in [-0.6, 0.0, 0.6_f64] {
                    let o = (d.s.0 - uy * d.rp * side, d.s.1 + ux * d.rp * side);
                    let near = d.rp * (1.25 - 0.3 * side.abs());
                    let far = near + d.rp * (1.1 + 0.6 * (1.0 - side.abs())) * fade;
                    px::ink_line(
                        img,
                        (o.0 + ux * near, o.1 + uy * near),
                        (o.0 + ux * far, o.1 + uy * far),
                        1.6,
                        bold::WHITE,
                        fade,
                    );
                }
            }
        }
    }

    /// A landed dart and a crossing pop as solid hits; the wave is heavy whatever the handicap.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match how(e) {
            "wave" => Some(HitSize::Heavy),
            "key" | "string" => Some(by_damage.max(HitSize::Solid)),
            _ => Some(by_damage),
        }
    }
}
