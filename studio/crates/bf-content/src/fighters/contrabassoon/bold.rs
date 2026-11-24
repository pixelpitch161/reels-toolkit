//! CONTRABASSOON with the bold effects (`picture.effects = "bold"`; the owner's picks 37a to 37d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **U-BEND** (37a): thick maroon pipes with brass joints at both bends and both ends; a pipe slides out with a
//!   clank spark at each joint as it gets there, and slides back in when it is dropped; a pipe the enemy bounces
//!   off flashes white, shakes and rings (rings in the air, a puff ring on the canvas).
//! - **WATER KEY** (37b): every pipe end and the ball spray outlined droplets out to exactly where the spray
//!   reaches, over a wet patch of that size, and the bends squirt inward; an enemy caught gets a rippling puddle
//!   under it and three small ticks.
//! - **HOT PIPES** (37c): the pipes turn red-orange with a yellow core, the joints glow, the canvas glows under
//!   them and wisps of steam rise; a bounce off a hot pipe pops a second, orange star beside the ring's own.
//! - **FIVE AND A HALF METRES** (37d): one long silver tube snakes out of the ball, wriggling, and round the
//!   enemy; it closes with a clank, brass joints at the corners, and lets go of the ball; every wall the enemy
//!   pinballs off flashes.

use bf_core::dmath::{self, TAU};
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, WHITE};
use bf_picture::camera::{Cam, V3};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::scene;

use super::{ContrabassoonLook, PIPE_Y, PipeL, SILVER, WATER, ss};

/// The pipes' maroon.
const MAROON: V3 = [160.0, 40.0, 52.0];
/// The joints' brass.
const BRASS: V3 = [228.0, 180.0, 76.0];
/// A hot pipe.
const HOT_RED: V3 = [255.0, 92.0, 30.0];
/// A hot pipe's core, and a glowing joint.
const HOT_CORE: V3 = [255.0, 226.0, 130.0];
/// A droplet.
const DROP: V3 = [96.0, 184.0, 255.0];
/// The tube's drawn radius (world units).
const TUBE_R: f64 = 0.17;
/// Seconds a new pipe takes to slide out, and a dropped one to slide back in.
const LAY_S: f64 = 0.24;
const RETRACT_S: f64 = 0.25;
/// Seconds the low move's tube takes to snake round the enemy, and then to let go of the ball.
const SNAKE_S: f64 = 0.34;
const LET_GO_S: f64 = 0.2;
/// Seconds a joint's clank spark shows.
const CLANK_S: f64 = 0.14;
/// Seconds a struck pipe rings.
const RING_S: f64 = 0.35;
/// Seconds a puddle lies under a sprayed enemy (about two beats).
const PUDDLE_S: f64 = 0.85;

/// A pipe as it is at one moment: the part of it that is out, and the brass fittings that are (each a short
/// sleeve along the tube: round a bend or over an end).
struct Live {
    path: Vec<[f64; 2]>,
    joints: Vec<Vec<[f64; 2]>>,
}

/// A painter item: the tube's world points, its fittings, the flash of a bounce, the heat and whether it is the
/// low move's box.
struct Tube {
    pts: Vec<V3>,
    joints: Vec<Vec<V3>>,
    flash: f64,
    heat: f64,
    boxed: bool,
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    dmath::hypot(b[0] - a[0], b[1] - a[1])
}

fn length(pts: &[[f64; 2]]) -> f64 {
    pts.windows(2).map(|w| dist(w[0], w[1])).sum()
}

/// The part of a polyline from `a` to `b` along its length.
fn slice(pts: &[[f64; 2]], a: f64, b: f64) -> Vec<[f64; 2]> {
    let mut out = Vec::new();
    let mut s = 0.0;
    for w in pts.windows(2) {
        let l = dist(w[0], w[1]);
        let (s0, s1) = (s, s + l);
        s = s1;
        if l <= 1e-12 || s1 <= a || s0 >= b {
            continue;
        }
        let at = |d: f64| {
            let u = ((d - s0) / l).clamp(0.0, 1.0);
            [w[0][0] + (w[1][0] - w[0][0]) * u, w[0][1] + (w[1][1] - w[0][1]) * u]
        };
        if out.is_empty() {
            out.push(at(a));
        }
        out.push(at(b));
    }
    out
}

/// `a` mixed toward `b` by `k`.
fn mix(a: V3, b: V3, k: f64) -> V3 {
    let k = k.clamp(0.0, 1.0);
    std::array::from_fn(|ch| a[ch] + (b[ch] - a[ch]) * k)
}

fn on_floor(cam: &Cam, q: [f64; 2], y: f64) -> (f64, f64) {
    bold::at(cam, q[0], q[1], y)
}

impl ContrabassoonLook {
    /// Where the low move's tube leaves the ball: the ball when the box was called.
    fn feed(&self, p: &PictureCtx, pp: &PipeL) -> [f64; 2] {
        p.run.p(p.sim_i(pp.t0), self.k)
    }

    /// When each of a pipe's points is reached as it slides out (the box's: its four corners and the closing one).
    fn joint_times(&self, p: &PictureCtx, pp: &PipeL) -> Vec<(f64, [f64; 2])> {
        let (lead, dur) = if pp.boxed {
            (dist(self.feed(p, pp), pp.pts[0]), SNAKE_S)
        } else {
            (0.0, LAY_S)
        };
        let total = (lead + length(&pp.pts)).max(1e-9);
        let mut s = lead;
        let mut out = vec![(pp.t0 + dur * s / total, pp.pts[0])];
        for w in pp.pts.windows(2) {
            s += dist(w[0], w[1]);
            out.push((pp.t0 + dur * s / total, w[1]));
        }
        out
    }

    /// The pipe at the effects clock `tc` (`keep`: 1, or less as everything is put away after the K.O.).
    fn live(&self, p: &PictureCtx, pp: &PipeL, tc: f64, keep: f64) -> Option<Live> {
        if tc < pp.t0 {
            return None;
        }
        let keep = match pp.t_off {
            Some(off) if tc >= off => keep.min(1.0 - (tc - off) / RETRACT_S),
            _ => keep,
        };
        if keep <= 0.0 {
            return None;
        }
        let path = if pp.boxed {
            let mut all = vec![self.feed(p, pp)];
            all.extend_from_slice(&pp.pts);
            let lead = dist(all[0], all[1]);
            let total = length(&all);
            let head = total * ((tc - pp.t0) / SNAKE_S).min(1.0);
            let tail = lead * ((tc - pp.t0 - SNAKE_S) / LET_GO_S).clamp(0.0, 1.0);
            let head = tail + (head - tail) * keep;
            let mut path = slice(&all, tail, head);
            // the head wriggles while it is still on its way
            if head < total - 1e-9 {
                path = wriggle(&path, tc);
            }
            path
        } else {
            let total = length(&pp.pts);
            slice(&pp.pts, 0.0, total * ((tc - pp.t0) / LAY_S).min(1.0).min(keep))
        };
        if path.len() < 2 {
            return None;
        }
        // the fittings the tube has reached: a sleeve a little way along each side of a bend, or over an end
        let n = if pp.boxed { 4 } else { pp.pts.len() };
        let toward = |a: [f64; 2], b: [f64; 2]| {
            let l = dist(a, b).max(1e-9);
            [a[0] + (b[0] - a[0]) / l * 0.11, a[1] + (b[1] - a[1]) / l * 0.11]
        };
        let joints = self
            .joint_times(p, pp)
            .into_iter()
            .take(n)
            .enumerate()
            .filter(|&(_, (tj, _))| tc >= tj && keep >= 1.0)
            .map(|(i, (_, q))| {
                let before = if pp.boxed { Some((i + 3) % 4) } else { i.checked_sub(1) };
                let after = (pp.boxed || i + 1 < n).then_some(i + 1);
                let mut sleeve = Vec::new();
                sleeve.extend(before.map(|j| toward(q, pp.pts[j])));
                sleeve.push(q);
                sleeve.extend(after.map(|j| toward(q, pp.pts[j])));
                sleeve
            })
            .collect();
        Some(Live { path, joints })
    }

    /// How much of everything is still out: 1 in the fight, running down to 0 after the K.O.
    fn keep(p: &PictureCtx, t: f64) -> f64 {
        if t >= p.t_ko {
            1.0 - ss(p.t_hs + 0.5, p.t_hs + 1.1, t)
        } else {
            1.0
        }
    }

    /// The flash of the pipe's last bounce at `tc` (1 fresh, 0 gone) and a little shake to draw it with.
    fn struck(pp: &PipeL, tc: f64) -> (f64, f64) {
        pp.hits
            .iter()
            .rev()
            .map(|&th| tc - th)
            .find(|age| (0.0..RING_S).contains(age))
            .map_or((0.0, 0.0), |age| {
                let fade = dmath::exp(-age / 0.1);
                (fade, 0.04 * fade * dmath::sin(age * 80.0))
            })
    }

    /// The spray's reach.
    fn reach(&self, p: &PictureCtx) -> f64 {
        p.cfg[self.k].raw["abilities"]["unlock1"]["reach"]
            .as_f64()
            .unwrap_or(0.9)
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let mut out: Vec<(f64, Payload)> = Vec::new();
        let tc = p.fx_clock(f.t);
        let keep = Self::keep(p, f.t);
        if keep <= 0.0 || tc < p.show.grid.t_fight {
            return out;
        }
        let heat = self.heat(tc);
        for pp in &self.pipes {
            let Some(live) = self.live(p, pp, tc, keep) else {
                continue;
            };
            let (flash, shake) = Self::struck(pp, tc);
            let up = |q: &[f64; 2]| scene::to_world(q[0] + shake, q[1] - shake, PIPE_Y);
            let pts: Vec<V3> = live.path.iter().map(up).collect();
            let d = f.cam.depth(&pts).iter().sum::<f64>() / pts.len() as f64;
            let tube = Tube {
                joints: live.joints.iter().map(|j| j.iter().map(up).collect()).collect(),
                pts,
                flash,
                heat: if pp.boxed { 0.0 } else { heat },
                boxed: pp.boxed,
            };
            out.push((d, Box::new(tube)));
        }
        out
    }

    pub(super) fn bold_draw_item(f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let Some(tube) = payload.downcast_ref::<Tube>() else {
            return;
        };
        let cam = f.cam;
        let d = cam.depth(&tube.pts).iter().sum::<f64>() / tube.pts.len() as f64;
        let w = ((2.0 * TUBE_R * cam.f / d.max(1e-6)).round() as u32).clamp(6, 16);
        let base = if tube.boxed { SILVER } else { MAROON };
        let col = mix(mix(base, HOT_RED, tube.heat * 0.9), WHITE, tube.flash * 0.85);
        bold::thick_line(fr, cam, &tube.pts, col, w);
        if tube.heat > 0.05 {
            fr.line(cam, &tube.pts, mix(col, HOT_CORE, tube.heat), (w / 3).max(1));
        }
        let brass = mix(mix(BRASS, HOT_CORE, tube.heat * 0.45), WHITE, tube.flash * 0.6);
        for sleeve in &tube.joints {
            // a bend's elbow is rounded off by a knob under its sleeve; an end gets a round cap
            let knob = if sleeve.len() > 2 { sleeve[1] } else { sleeve[0] };
            bold::matte_blob(fr, cam, knob, TUBE_R * 1.12, brass);
            bold::thick_line(fr, cam, sleeve, brass, w + 3);
        }
    }

    /// The canvas: a struck pipe's puff ring, the hot pipes' glow, WATER KEY's wet patches and the puddle.
    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let tc = p.fx_clock(c.t);
        if !c.in_fight {
            return;
        }
        let k = self.k;
        for e in bold::events_in(p, k, &["hit:pipe", "hit:pipe_box"], tc - RING_S, tc) {
            let u = (tc - e.t) / RING_S;
            floor::band(
                ic,
                c,
                Family::Air,
                e.f(5),
                e.f(6),
                0.5 + 1.1 * u,
                1.0 - u,
                tc,
                bold::light(BRASS),
                1.0,
            );
        }
        let heat = self.heat(tc);
        if heat > 0.0 {
            let pulse = 0.85 + 0.15 * dmath::sin(tc * 9.0);
            for pp in self.pipes.iter().filter(|pp| !pp.boxed) {
                let Some(live) = self.live(p, pp, tc, 1.0) else {
                    continue;
                };
                for w in live.path.windows(2) {
                    floor::lane(ic, c, w[0], w[1], 0.45, [1.3, 0.42, 0.08], 0.85 * heat * pulse);
                }
            }
        }
        let water = [0.3, 0.85, 1.7];
        let beat = p.show.grid.beat;
        let reach = self.reach(p);
        let me = c.fs[k].p;
        for s in &self.sprays {
            let age = (tc - s.t0) / beat;
            if !(0.0..1.0).contains(&age) {
                continue;
            }
            let r = reach * bold::ease_out(age * 1.6);
            for q in s.from.iter().copied().chain(std::iter::once(me)) {
                floor::disc(ic, c, q[0], q[1], r, water, 0.3 * (1.0 - age));
                floor::ring(ic, c, q[0], q[1], r, 0.05, water, 0.7 * (1.0 - age));
            }
        }
        for e in bold::events_in(p, k, &["hit:spray"], tc - PUDDLE_S, tc) {
            let age = tc - e.t;
            let fade = (1.0 - age / PUDDLE_S).clamp(0.0, 1.0);
            let (x, z) = (e.f(5), e.f(6));
            let r = 0.85 * bold::ease_out(age / 0.2);
            floor::disc(ic, c, x, z, r, water, 0.75 * fade);
            floor::ring(ic, c, x, z, r, 0.06, water, fade);
            let ripple = (age * 2.6).rem_euclid(1.0);
            let k_ripple = 1.2 * fade * (1.0 - ripple);
            floor::ring(ic, c, x, z, 0.2 + 0.6 * ripple, 0.05, water, k_ripple);
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let tc = p.fx_clock(f.t);
        if !f.in_fight {
            return;
        }
        let (k, cam) = (self.k, f.cam);
        let heat = self.heat(tc);
        for (n, pp) in self.pipes.iter().enumerate() {
            let on = pp.t_off.is_none_or(|off| tc < off);
            // a clank spark at each joint as the tube gets there; the box's closing clank is the biggest
            let joints = self.joint_times(p, pp);
            for (j, &(tj, q)) in joints.iter().enumerate() {
                let age = tc - tj;
                if !(0.0..CLANK_S).contains(&age) || !on {
                    continue;
                }
                let u = age / CLANK_S;
                let s = on_floor(cam, q, PIPE_Y + 0.1);
                let closing = pp.boxed && j + 1 == joints.len();
                let big = if closing { 1.8 } else { 1.0 };
                px::ink_star(img, s, (5.0 + 7.0 * u) * big, 1.6 * big, 4, 0.5, WHITE, 1.0 - u * u);
                px::chips(
                    img,
                    s,
                    age,
                    CLANK_S,
                    4,
                    12.0 * big,
                    (tj * 1000.0) as u64 + j as u64,
                    BRASS,
                );
                if closing {
                    px::ink_ring(img, s, 6.0 + 16.0 * u, 1.6, BRASS, 1.0 - u);
                }
            }
            if !on {
                continue;
            }
            // HOT PIPES: wisps of steam off the pipe
            if heat > 0.2 && !pp.boxed && tc > pp.t0 + LAY_S {
                for j in 0..5u64 {
                    let at = slice(&pp.pts, 0.0, length(&pp.pts) * (0.1 + 0.2 * j as f64));
                    let Some(&q) = at.last() else { continue };
                    let ph = (tc * 1.5 + px::rnd(n as u64, j)).rem_euclid(1.0);
                    let s = on_floor(cam, q, PIPE_Y + TUBE_R);
                    for m in 0..3 {
                        let h = 5.0 + 20.0 * ph + 5.0 * f64::from(m);
                        let at = (s.0 + 2.5 * dmath::sin(h * 0.4 + j as f64), s.1 - h);
                        let r = 3.6 - 0.8 * f64::from(m);
                        px::disc(img, at, r, [255.0, 240.0, 228.0], 0.85 * heat * (1.0 - ph));
                    }
                }
            }
        }
        // a struck pipe rings; a hot one pops a second star; the box's wall flashes
        for e in bold::events_in(p, k, &["hit:pipe", "hit:pipe_box"], tc - RING_S, tc) {
            let age = tc - e.t;
            let u = age / RING_S;
            let (x, z) = (e.f(5), e.f(6));
            let s = bold::at(cam, x, z, PIPE_Y + 0.2);
            for j in 0..2 {
                px::ring(
                    img,
                    s,
                    9.0 + 7.0 * f64::from(j) + 14.0 * u,
                    1.3,
                    bold::pale(BRASS),
                    (1.0 - u) * 0.9,
                );
            }
            if self.heat(e.t) > 0.0 && bold::how(e) == "pipe" && (0.05..0.3).contains(&age) {
                let v = (age - 0.05) / 0.25;
                px::ink_star(
                    img,
                    (s.0 + 11.0, s.1 - 9.0),
                    8.0 + 6.0 * v,
                    3.5,
                    5,
                    0.3,
                    HOT_RED,
                    1.0 - v * v,
                );
                px::star(
                    img,
                    (s.0 + 11.0, s.1 - 9.0),
                    4.0 + 3.0 * v,
                    1.8,
                    5,
                    0.3,
                    HOT_CORE,
                    1.0 - v,
                );
            }
            if bold::how(e) == "pipe_box"
                && age < 0.16
                && let Some(pp) = self.pipes.iter().find(|pp| pp.boxed && pp.id == e.i(8))
            {
                // the wall it bounced off: the side of the square it is nearest
                let (lo, hi) = (pp.pts[0], pp.pts[2]);
                let (cx, cz) = ((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0);
                let wall = if (x - cx).abs() > (z - cz).abs() {
                    let wx = if x > cx { hi[0] } else { lo[0] };
                    [[wx, lo[1]], [wx, hi[1]]]
                } else {
                    let wz = if z > cz { hi[1] } else { lo[1] };
                    [[lo[0], wz], [hi[0], wz]]
                };
                let a = 1.0 - age / 0.16;
                px::ink_line(
                    img,
                    on_floor(cam, wall[0], PIPE_Y),
                    on_floor(cam, wall[1], PIPE_Y),
                    4.0,
                    WHITE,
                    a,
                );
            }
        }
        // WATER KEY: droplets out of every pipe end and the ball, and a squirt inward from every bend
        let beat = p.show.grid.beat;
        let reach = self.reach(p);
        let me = f.fs[k].p;
        for (n, s) in self.sprays.iter().enumerate() {
            let age = (tc - s.t0) / beat;
            if !(0.0..1.0).contains(&age) {
                continue;
            }
            let a = ((1.0 - age) / 0.25).clamp(0.0, 1.0);
            for (m, q) in s.from.iter().copied().chain(std::iter::once(me)).enumerate() {
                spray(img, cam, q, None, reach, age, a, (n * 31 + m) as u64);
            }
            for (m, pp) in self.pipes.iter().enumerate() {
                let out_then = !pp.boxed && pp.t0 <= s.t0 && pp.t_off.is_none_or(|off| off > s.t0);
                if !out_then || pp.pts.len() < 4 {
                    continue;
                }
                let mid = [
                    pp.pts.iter().map(|q| q[0]).sum::<f64>() / pp.pts.len() as f64,
                    pp.pts.iter().map(|q| q[1]).sum::<f64>() / pp.pts.len() as f64,
                ];
                for bend in [pp.pts[1], pp.pts[2]] {
                    spray(img, cam, bend, Some(mid), 0.55, age, a, (n * 57 + m + 7) as u64);
                }
            }
        }
        // the enemy caught in the spray: three small ticks
        for e in bold::events_in(p, k, &["hit:spray"], tc - 0.4, tc) {
            let Some(d) = drawn.iter().find(|d| Some(d.k) == usize::try_from(e.i(3)).ok()) else {
                continue;
            };
            for j in 0..3u64 {
                let age = tc - e.t - 0.09 * j as f64;
                if !(0.0..0.14).contains(&age) {
                    continue;
                }
                let ang = TAU * px::rnd((e.t * 1000.0) as u64, j);
                let at = (
                    d.s.0 + d.rp * 0.9 * dmath::cos(ang),
                    d.s.1 + d.rp * 0.9 * dmath::sin(ang),
                );
                px::ink_star(img, at, 4.0 + 3.0 * age / 0.14, 1.6, 4, 0.78, bold::pale(WATER), 1.0);
            }
        }
    }
}

/// The path with its head wriggling like a snake's (the last 1.1 units of it, more toward the head).
fn wriggle(path: &[[f64; 2]], tc: f64) -> Vec<[f64; 2]> {
    let total = length(path);
    let n = ((total / 0.12).ceil() as usize).clamp(2, 80);
    (0..=n)
        .filter_map(|j| {
            let s = total * j as f64 / n as f64;
            let ahead = slice(path, 0.0, (s + 0.05).min(total));
            let (&b, &a) = (ahead.last()?, slice(path, 0.0, s.max(1e-6)).last()?);
            let l = dist(a, b);
            let near_head = (1.0 - (total - s) / 1.1).max(0.0);
            if l < 1e-9 || near_head <= 0.0 || j == n {
                return Some(a);
            }
            let amp = 0.1 * near_head * dmath::sin(s * 8.0 - tc * 40.0);
            Some([a[0] - (b[1] - a[1]) / l * amp, a[1] + (b[0] - a[0]) / l * amp])
        })
        .collect()
}

/// Droplets out of `(x, z)`: all round to `reach`, or a narrow squirt `toward` a point. `age` is 0 to 1 of the
/// spray, `a` its strength.
#[allow(clippy::too_many_arguments)]
fn spray(img: &mut Pic, cam: &Cam, from: [f64; 2], toward: Option<[f64; 2]>, reach: f64, age: f64, a: f64, seed: u64) {
    let n = if toward.is_some() { 4u64 } else { 10 };
    let aim = toward.map(|q| dmath::atan2(q[1] - from[1], q[0] - from[0]));
    let out = bold::ease_out(age * 1.6);
    for j in 0..n {
        let ang = match aim {
            Some(th) => th + (px::rnd(seed, j) - 0.5) * 0.9,
            None => TAU * (j as f64 + 0.6 * px::rnd(seed, j)) / n as f64,
        };
        let far = reach * (0.55 + 0.45 * px::rnd(seed, j + 40));
        let at = |k: f64| {
            let y = PIPE_Y + 0.1 + 0.55 * 4.0 * (age * k) * (1.0 - age * k);
            bold::at(
                cam,
                from[0] + far * out * k * dmath::cos(ang),
                from[1] + far * out * k * dmath::sin(ang),
                y.max(0.03),
            )
        };
        let (tail, head) = (at(0.8), at(1.0));
        px::ink_line(img, tail, head, 1.6, DROP, a);
        px::ink_disc(img, head, 2.4, DROP, a);
        px::disc(img, (head.0 - 0.8, head.1 - 0.8), 0.9, WHITE, a);
    }
}
