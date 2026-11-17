//! The camera director: one smoothed camera over all 1,830 frames, simulated once per fight in order.
//!
//! Per frame: the base shot is the current engine's camera ([`crate::base`]: the whole ring as a diamond from 27.5
//! away, 58 degrees down, drifting gently over the minute); the accepted shots ([`crate::shots`]) bend it (closer,
//! lower, rolled, turned, toward a ball), scaled by `DirectCfg::intensity` so they stay subtle, but never past what
//! still frames both balls; a critically damped spring smooths every channel, reading its goal a lag ahead so it
//! keeps step with the balls; the distance is raised ahead of any frame that would still lose a ball, and a last
//! framing guard catches what is left; a trauma model adds shake, also scaled by the intensity.

use bf_core::dmath;
use bf2_sim::{Cue, EvKind, Recording2};

use crate::base::base_rig;
use crate::cfg::{DirectCfg, FPS, FRAMES};
use crate::project::{CamPose, P3, SAFE, ball_box, ball_centre, inset, inside};
use crate::shots::{Priority, Shot, ShotKind, plan};
use crate::time::TimeMap;

/// The vertical field of view (the current engine's).
pub const FOV: f64 = 36.0;
/// The closest a shot's framing search looks (half of it).
pub const DIST_MIN: f64 = 14.0;
/// The farthest the framing guard pulls back.
const GUARD_MAX: f64 = 60.0;
/// A shot's push-in stops where both balls are this share of the safe zone inside its edges.
const SHOT_MARGIN: f64 = 0.02;
/// The springs' half-lives in video seconds: normally, and during a whip-back.
const HALF_LIFE: f64 = 0.18;
const WHIP_HALF_LIFE: f64 = 0.05;
/// Shake: trauma lost per video second, the offset at full trauma (px of 1080 x 1920), the roll at full trauma
/// (degrees) and the noise's rate.
const TRAUMA_DECAY: f64 = 1.6;
const SHAKE_PX: f64 = 18.0;
const SHAKE_ROLL: f64 = 1.5;
const SHAKE_HZ: f64 = 25.0;

/// The camera of every frame.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraTrack {
    /// One pose a frame, [`FRAMES`] of them.
    pub poses: Vec<CamPose>,
    /// The screen-space shake offset of each frame, in pixels of the 1080 x 1920 picture.
    pub shake: Vec<[f64; 2]>,
    /// The shots the director accepted, for the job's log and the tests.
    pub shots: Vec<Shot>,
    /// Frames where the framing guard had to pull the camera back at the last moment (a snap, so it should be
    /// rare): for the job's log and the tests.
    pub guarded: usize,
}

/// Both balls' sim positions at video time `t`: the recording's steps through the time map, interpolated, holding
/// the first step before the fight and the last after it.
pub fn balls_at(rec: &Recording2, map: &TimeMap, t: f64) -> [P3; 2] {
    let n = rec.steps.len();
    if n == 0 {
        return [[-1.5, 0.0, 0.0], [1.5, 0.0, 0.0]];
    }
    let x = (map.fight_t(t) / rec.dt).clamp(0.0, (n - 1) as f64);
    let i = (x.floor() as usize).min(n - 1);
    let j = (i + 1).min(n - 1);
    let a = x - i as f64;
    let (p, q) = (&rec.steps[i].pos, &rec.steps[j].pos);
    let lerp = |u: P3, v: P3| {
        [
            u[0] + (v[0] - u[0]) * a,
            u[1] + (v[1] - u[1]) * a,
            u[2] + (v[2] - u[2]) * a,
        ]
    };
    [lerp(p[0], q[0]), lerp(p[1], q[1])]
}

/// A camera as the director thinks of it: what it looks at, from how far, how far down, turned how far round,
/// rolled how far (degrees).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rig {
    target: P3,
    dist: f64,
    pitch: f64,
    yaw: f64,
    roll: f64,
}

impl Rig {
    fn pose(&self) -> CamPose {
        let (p, y) = (self.pitch.to_radians(), self.yaw.to_radians());
        let (cp, sp) = (dmath::cos(p), dmath::sin(p));
        let t = self.target;
        CamPose {
            eye: [
                t[0] + self.dist * cp * dmath::sin(y),
                t[1] + self.dist * sp,
                t[2] + self.dist * cp * dmath::cos(y),
            ],
            target: t,
            roll: self.roll.to_radians(),
            fov_deg: FOV,
        }
    }

    fn fits(&self, balls: &[P3; 2], rect: [f64; 4]) -> bool {
        let c = self.pose();
        balls
            .iter()
            .all(|b| ball_box(&c, *b).is_some_and(|bx| inside(bx, rect)))
    }

    /// The smallest distance from `lo` to `hi` at which both balls fit `rect` (`hi` if none does).
    fn fit_dist(mut self, balls: &[P3; 2], rect: [f64; 4], lo: f64, hi: f64) -> f64 {
        self.dist = lo;
        if self.fits(balls, rect) {
            return lo;
        }
        self.dist = hi;
        if !self.fits(balls, rect) {
            return hi;
        }
        let (mut a, mut b) = (lo, hi);
        for _ in 0..28 {
            self.dist = 0.5 * (a + b);
            if self.fits(balls, rect) {
                b = self.dist;
            } else {
                a = self.dist;
            }
        }
        b
    }
}

fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// 0 at `a`, 1 at `b`, smoothly; a step at `a` when `b <= a`.
fn ramp(t: f64, a: f64, b: f64) -> f64 {
    if b <= a {
        if t >= a { 1.0 } else { 0.0 }
    } else {
        smoothstep((t - a) / (b - a))
    }
}

/// What the active shots do to the base shot at one moment.
struct Mods {
    dist_mul: f64,
    pitch: f64,
    yaw: f64,
    roll: f64,
    /// How far the target moves from the midpoint toward each ball, and the priority that set it.
    focus: Option<(Priority, [f64; 2])>,
    /// The ball the target follows up, and how much.
    follow: Option<(usize, f64)>,
    whip: bool,
}

/// `m` scaled toward no change by `k` (0: none, 1: in full).
fn scaled(m: Mods, k: f64) -> Mods {
    Mods {
        dist_mul: 1.0 + (m.dist_mul - 1.0) * k,
        pitch: m.pitch * k,
        yaw: m.yaw * k,
        roll: m.roll * k,
        focus: m.focus.map(|(p, w)| (p, w.map(|x| x * k))),
        follow: m.follow.map(|(who, w)| (who, w * k)),
        whip: m.whip,
    }
}

fn mods(shots: &[Shot], t: f64) -> Mods {
    let mut m = Mods {
        dist_mul: 1.0,
        pitch: 0.0,
        yaw: 0.0,
        roll: 0.0,
        focus: None,
        follow: None,
        whip: false,
    };
    let focus = |m: &mut Mods, p: Priority, w: [f64; 2]| {
        if m.focus.is_none_or(|(q, _)| p >= q) {
            m.focus = Some((p, w));
        }
    };
    let one = |who: usize, w: f64| {
        let mut v = [0.0; 2];
        v[who.min(1)] = w;
        v
    };
    for s in shots {
        if t < s.t0 || t >= s.t1 {
            continue;
        }
        match s.kind {
            ShotKind::PunchIn => {
                // Eased out over the wind-up: it starts moving at once and settles on the strike.
                let x = if s.t_peak > s.t0 {
                    ((t - s.t0) / (s.t_peak - s.t0)).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                let p = 1.0 - (1.0 - x) * (1.0 - x);
                m.dist_mul *= 1.0 - 0.38 * p;
                focus(&mut m, s.priority, one(s.subject, 0.35 * p));
            }
            ShotKind::Tilt(a) => m.roll += a * ramp(t, s.t0, s.t_peak),
            ShotKind::WhipBack => {
                // Snaps 6 % past the base shot on the fast spring, then eases back onto it on the normal one.
                m.whip = true;
                m.dist_mul *= 1.06;
            }
            ShotKind::Launch => {
                let w = ramp(t, s.t0, s.t_peak);
                m.pitch -= 8.0 * w;
                m.follow = Some((s.subject, w));
            }
            ShotKind::Slam => m.pitch -= 14.0 * ramp(t, s.t0, s.t_peak),
            ShotKind::Clash => {
                let w = if t < s.t_peak {
                    ramp(t, s.t0, s.t_peak)
                } else {
                    1.0 - ramp(t, s.t_peak, s.t1)
                };
                m.dist_mul *= 1.0 - 0.1 * w;
            }
            ShotKind::Ko => {
                let loser = s.subject.min(1);
                let orbit = ramp(t, s.t0, s.t_peak);
                m.yaw += 12.0 * ko_side(loser) * orbit;
                let t_wide = (s.t0 + 2.5).max(s.t_peak + 1.2);
                let w = ramp(t, s.t_peak, t_wide);
                m.dist_mul *= 0.7 + (1.12 - 0.7) * w;
                let mut f = [0.0; 2];
                f[loser] = 0.6 * (1.0 - w);
                f[1 - loser] = 0.35 * w;
                focus(&mut m, Priority::Ko, f);
            }
        }
    }
    m.dist_mul = m.dist_mul.clamp(0.55, 1.3);
    m.pitch = m.pitch.max(-16.0);
    m.roll = m.roll.clamp(-12.0, 12.0);
    m
}

/// Which way the K.O. orbit turns: always the same way for the same loser.
fn ko_side(loser: usize) -> f64 {
    if loser == 0 { -1.0 } else { 1.0 }
}

/// A critically damped spring with an exact step (stable at any `dt`).
#[derive(Clone, Copy, Debug)]
struct Spring {
    x: f64,
    v: f64,
}

impl Spring {
    fn step(&mut self, goal: f64, half_life: f64, dt: f64) {
        let y = 2.0 * dmath::ln(2.0) / half_life;
        let j0 = self.x - goal;
        let j1 = self.v + j0 * y;
        let e = dmath::exp(-y * dt);
        self.x = e * (j0 + j1 * dt) + goal;
        self.v = e * (self.v - j1 * y * dt);
    }
}

/// A smooth noise from −1 to 1 at `t` seconds, `SHAKE_HZ` knots a second, the same for the same seed and axis.
fn noise(seed: u64, axis: u64, t: f64) -> f64 {
    let x = t * SHAKE_HZ;
    let i = x.floor();
    let fr = x - i;
    let knot = |k: f64| {
        let mut z =
            seed ^ axis.wrapping_mul(0xA24B_AED4_963E_E407) ^ (k as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    };
    let (a, b) = (knot(i), knot(i + 1.0));
    a + (b - a) * smoothstep(fr)
}

/// The trauma each event adds, at its video time, in order.
fn trauma_hits(rec: &Recording2, map: &TimeMap) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = rec
        .events
        .iter()
        .filter_map(|e| {
            let amount = match e.kind {
                EvKind::Cue(Cue::Shake(x)) => x,
                EvKind::Contact { strength, .. } if strength >= 0.4 => strength * strength,
                _ => return None,
            };
            Some((map.step_video_t(e.step), amount.max(0.0)))
        })
        .collect();
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// The least share of a shot's lean toward its subject that it keeps, however tight the framing.
const FOCUS_MIN: f64 = 0.4;

/// The shot's camera: `orient` moved toward the balls by `focus` and brought to distance `want`, but never closer
/// than frames both balls in `rect`. When there is not room for both the lean and the distance, the lean gives
/// way first, down to [`FOCUS_MIN`] of itself, and then the distance. Also returns the least distance that frames both balls in `rect` from the chosen target.
fn shot_rig(orient: Rig, balls: &[P3; 2], focus: Option<[f64; 2]>, want: f64, rect: [f64; 4]) -> (Rig, f64) {
    let mid = orient.target;
    let at = |s: f64| {
        let mut t = mid;
        if let Some(w) = focus {
            for k in [0, 2] {
                t[k] += s * (w[0] * (balls[0][k] - mid[k]) + w[1] * (balls[1][k] - mid[k]));
            }
        }
        Rig { target: t, ..orient }
    };
    let need = |s: f64| at(s).fit_dist(balls, rect, 0.5 * DIST_MIN, GUARD_MAX);
    let s = if focus.is_none() || need(1.0) <= want {
        1.0
    } else if need(FOCUS_MIN) > want {
        FOCUS_MIN
    } else {
        let (mut a, mut b) = (FOCUS_MIN, 1.0);
        for _ in 0..12 {
            let c = 0.5 * (a + b);
            if need(c) <= want {
                a = c;
            } else {
                b = c;
            }
        }
        a
    };
    let n = need(s);
    (
        Rig {
            dist: want.max(n),
            ..at(s)
        },
        n,
    )
}

/// One frame's wishes, before smoothing.
struct Want {
    balls: [P3; 2],
    rig: Rig,
    /// The least distance that frames both balls (2 % margin) from `rig`'s target.
    need: f64,
    whip: bool,
}

/// Frames a spring of half-life `h` lags behind a goal moving steadily (`2 / ω`, `h / ln 2` seconds): its goal is
/// read this far ahead, so the camera keeps up with the balls instead of trailing them.
fn lead(h: f64) -> usize {
    (h / dmath::ln(2.0) * FPS).round() as usize
}

impl CameraTrack {
    /// The camera of every frame. Two passes: each frame's wished-for shot (the base shot bent by the accepted
    /// shots, never closer than frames both balls), then the springs in order. A spring reads its goal
    /// [`lead`] frames ahead, so it is in step with the balls rather than behind them, and the distance never aims
    /// closer than the closest that frames both balls over the next two leads, so it pulls back before a launch or
    /// a knock-back, not after. The framing guard is the last word and rarely has to speak.
    pub fn build(rec: &Recording2, map: &TimeMap, cfg: &DirectCfg) -> CameraTrack {
        let shots = plan(&rec.events, map, cfg);
        let hits = trauma_hits(rec, map);
        let dt = 1.0 / FPS;
        let shot_rect = inset(SAFE, SHOT_MARGIN);

        let k = cfg.intensity.clamp(0.0, 1.0);
        // The current engine's keys push in for the final exchange from 44 s and settle back after the K.O.: they
        // need the K.O. after 44 s and before 58 s, so an earlier or later one is held inside that.
        let t_ko = map.ko_video_t().unwrap_or(55.0).clamp(44.5, 55.5);

        let wants: Vec<Want> = (0..FRAMES)
            .map(|f| {
                let t = f as f64 * dt;
                let sim = balls_at(rec, map, t);
                let balls = [ball_centre(sim[0]), ball_centre(sim[1])];
                let m = scaled(mods(&shots, t), k);
                let top = sim[0][2].max(sim[1][2]).max(0.0);
                let lift = match m.follow {
                    Some((who, w)) => 0.6 * top + (0.9 * sim[who.min(1)][2].max(0.0) - 0.6 * top) * w,
                    None => 0.6 * top,
                };
                let (target, base, pitch, yaw) = base_rig(t, t_ko);
                let orient = Rig {
                    target: [target[0], target[1] + k * lift, target[2]],
                    dist: base,
                    pitch: pitch + m.pitch,
                    yaw: yaw + m.yaw,
                    roll: m.roll,
                };
                let (rig, need) = shot_rig(orient, &balls, m.focus.map(|(_, w)| w), base * m.dist_mul, shot_rect);
                Want {
                    balls,
                    rig,
                    need,
                    whip: m.whip,
                }
            })
            .collect();

        // The springs, run until the framing guard has nothing left to do: each pass raises the distance's goal
        // ahead of every frame the guard had to fix, so the pull-back is smooth and early rather than a snap.
        let mut raise = vec![0.0; FRAMES];
        let mut out = springs(rec.seed, &wants, &hits, &raise, false, k);
        for _ in 0..6 {
            if out.fixes.is_empty() {
                break;
            }
            for &(g, d) in &out.fixes {
                let l = lead(if wants[g].whip { WHIP_HALF_LIFE } else { HALF_LIFE });
                for r in &mut raise[g.saturating_sub(2 * l)..=g] {
                    *r = f64::max(*r, d);
                }
            }
            out = springs(rec.seed, &wants, &hits, &raise, false, k);
        }
        let out = springs(rec.seed, &wants, &hits, &raise, true, k);
        CameraTrack {
            poses: out.poses,
            shake: out.shake,
            shots,
            guarded: out.fixes.len(),
        }
    }
}

/// One run of the springs.
struct Run {
    poses: Vec<CamPose>,
    shake: Vec<[f64; 2]>,
    /// The frames where a ball left the safe zone, and the distance that would have kept it in (1 % margin).
    fixes: Vec<(usize, f64)>,
}

/// The springs over every frame, in order, the distance's goal raised to at least `raise`. With `guard`, a frame
/// that still loses a ball is pulled back to fit (the springs carry on from there).
fn springs(seed: u64, wants: &[Want], hits: &[(f64, f64)], raise: &[f64], guard: bool, shake_k: f64) -> Run {
    let dt = 1.0 / FPS;
    let fix_rect = inset(SAFE, 0.01);
    let mut run = Run {
        poses: Vec::with_capacity(FRAMES),
        shake: Vec::with_capacity(FRAMES),
        fixes: Vec::new(),
    };
    let goal_of = |r: &Rig| [r.target[0], r.target[1], r.target[2], r.dist, r.pitch, r.yaw, r.roll];
    let mut s = goal_of(&wants[0].rig).map(|x| Spring { x, v: 0.0 });
    s[3].x = s[3].x.max(raise[0]);
    let mut trauma: f64 = 0.0;
    let mut next_hit = 0;
    // The first frame of the next whip-back from each frame on: the springs never read ahead across one, so the
    // punch-in holds until the whip snaps it back instead of drifting out early.
    let mut next_whip = vec![FRAMES; FRAMES + 1];
    for f in (0..FRAMES).rev() {
        let starts = wants[f].whip && (f == 0 || !wants[f - 1].whip);
        next_whip[f] = if starts { f } else { next_whip[f + 1] };
    }
    for (f, w) in wants.iter().enumerate() {
        let t = f as f64 * dt;
        let hl = if w.whip { WHIP_HALF_LIFE } else { HALF_LIFE };
        // A whip is a reaction: it reads no frame ahead.
        let l = if w.whip { 0 } else { lead(hl) };
        let stop = if next_whip[f] > f { next_whip[f] - 1 } else { FRAMES - 1 };
        let ahead = &wants[(f + l).min(stop).min(FRAMES - 1)];
        let mut goal = goal_of(&ahead.rig);
        let floor = wants[f..(f + 2 * l + 1).min(FRAMES)]
            .iter()
            .map(|w| w.need)
            .fold(raise[f], f64::max);
        goal[3] = goal[3].max(floor);
        for (sp, g) in s.iter_mut().zip(goal) {
            sp.step(g, hl, dt);
        }

        // Shake: this frame's hits, then the offset, then the decay toward the next frame.
        while next_hit < hits.len() && hits[next_hit].0 <= t {
            trauma = (trauma + hits[next_hit].1).min(1.0);
            next_hit += 1;
        }
        let k = trauma * trauma * shake_k;
        let off = [k * SHAKE_PX * noise(seed, 1, t), k * SHAKE_PX * noise(seed, 2, t)];
        let roll_shake = k * SHAKE_ROLL * noise(seed, 3, t);
        trauma = (trauma - TRAUMA_DECAY * dt).max(0.0);

        let mut rig = Rig {
            target: [s[0].x, s[1].x, s[2].x],
            dist: s[3].x,
            pitch: s[4].x,
            yaw: s[5].x,
            roll: s[6].x + roll_shake,
        };
        if !rig.fits(&w.balls, SAFE) {
            let d = rig.fit_dist(&w.balls, fix_rect, rig.dist, GUARD_MAX);
            run.fixes.push((f, d));
            if guard {
                // The framing guard: a shot never breaks framing.
                let d = rig.fit_dist(&w.balls, SAFE, rig.dist, GUARD_MAX);
                s[3].x = d;
                s[3].v = s[3].v.max(0.0);
                rig.dist = d;
            }
        }
        run.poses.push(rig.pose());
        run.shake.push(off);
    }
    run
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{FRAME, dot, sub};
    use crate::testkit::*;
    use bf2_sim::Ev;

    fn track(rec: &Recording2) -> (TimeMap, CameraTrack) {
        track_k(rec, DirectCfg::default().intensity)
    }

    /// The track with the shots and shake at `intensity` (1: in full, to check each shot's shape).
    fn track_k(rec: &Recording2, intensity: f64) -> (TimeMap, CameraTrack) {
        let cfg = DirectCfg {
            intensity,
            ..DirectCfg::default()
        };
        let map = TimeMap::from_recording(rec, &cfg);
        let t = CameraTrack::build(rec, &map, &cfg);
        (map, t)
    }

    fn frame(t: f64) -> usize {
        (t * FPS).round() as usize
    }

    fn dist(p: &CamPose) -> f64 {
        let d = sub(p.eye, p.target);
        dot(d, d).sqrt()
    }

    /// The fight's frames: from FIGHT! to the last recorded step.
    fn fight_frames(rec: &Recording2, map: &TimeMap) -> std::ops::Range<usize> {
        let end = map.video_t((rec.steps.len() - 1) as f64 * rec.dt);
        frame(rec.t_fight)..frame(end).min(FRAMES)
    }

    /// (frames with both balls in the safe zone, fight frames); panics if a ball leaves the picture.
    fn framing(rec: &Recording2, map: &TimeMap, tr: &CameraTrack) -> (usize, usize) {
        let mut ok = 0;
        let range = fight_frames(rec, map);
        let n = range.len();
        for f in range {
            let sim = balls_at(rec, map, f as f64 / FPS);
            let c = &tr.poses[f];
            let boxes = sim.map(|p| ball_box(c, ball_centre(p)).expect("in front of the camera"));
            if boxes.iter().all(|b| inside(*b, SAFE)) {
                ok += 1;
            }
            let [dx, dy] = tr.shake[f];
            for b in boxes {
                let moved = [b[0] + dx, b[1] + dy, b[2] + dx, b[3] + dy];
                assert!(inside(moved, FRAME), "seed {} frame {f}: {moved:?}", rec.seed);
            }
        }
        (ok, n)
    }

    #[test]
    fn both_balls_in_safe_zone_98_percent() {
        for seed in 1..=20 {
            let rec = dummy(seed, true);
            let (map, tr) = track(&rec);
            assert_eq!((tr.poses.len(), tr.shake.len()), (FRAMES, FRAMES));
            let (ok, n) = framing(&rec, &map, &tr);
            assert!(ok as f64 >= 0.98 * n as f64, "seed {seed}: {ok} of {n}");
            assert!(tr.guarded <= 3, "seed {seed}: the guard snapped {} frames", tr.guarded);
        }
    }

    /// No lurches: outside the whip-backs (a snap by design), the camera's distance, target and angles change
    /// smoothly from frame to frame in real fights.
    #[test]
    fn camera_moves_smoothly() {
        for seed in [2, 9, 14] {
            let rec = dummy(seed, true);
            let (_, tr) = track(&rec);
            let whips: Vec<(usize, usize)> = tr
                .shots
                .iter()
                .filter(|s| s.kind == ShotKind::WhipBack)
                .map(|s| (frame(s.t0).saturating_sub(1), frame(s.t1) + 12))
                .collect();
            let d: Vec<f64> = tr.poses.iter().map(dist).collect();
            let (mut worst_d, mut worst_t, mut worst_a) = (0.0f64, 0.0f64, 0.0f64);
            for f in 2..FRAMES {
                if whips.iter().any(|&(a, b)| (a..=b).contains(&f)) {
                    continue;
                }
                let acc = |x: &dyn Fn(usize) -> f64| (x(f) - 2.0 * x(f - 1) + x(f - 2)).abs();
                worst_d = worst_d.max(acc(&|g| d[g]));
                for k in 0..3 {
                    worst_t = worst_t.max(acc(&|g| tr.poses[g].target[k]));
                    worst_a = worst_a.max(acc(&|g| tr.poses[g].eye[k] - tr.poses[g].target[k]) - acc(&|g| d[g]));
                }
            }
            assert!(worst_d < 0.6, "seed {seed}: distance lurches {worst_d}");
            assert!(worst_t < 0.08, "seed {seed}: target lurches {worst_t}");
            assert!(worst_a < 0.6, "seed {seed}: angle lurches {worst_a}");
        }
    }

    #[test]
    fn launch_corner_is_framed() {
        // Ball 0 stands in a far corner; ball 1 is launched from the near corner to 4 up and back.
        let pos = |s: u32| {
            let z = if (100..300).contains(&s) {
                let u = (s - 100) as f64 / 200.0;
                16.0 * u * (1.0 - u)
            } else {
                0.0
            };
            [[2.4, 2.4, 0.0], [-2.4, -2.4, z]]
        };
        let ev = vec![
            ev(100, 0, EvKind::Launch { target: 1, up: 9.0 }),
            ev(300, 1, EvKind::Land),
        ];
        let rec = synthetic(600, pos, ev, None);
        let (map, tr) = track_k(&rec, 1.0);
        let (ok, n) = framing(&rec, &map, &tr);
        assert_eq!(ok, n);
        assert!(tr.shots.iter().any(|s| s.kind == ShotKind::Launch));
        // the camera is lower while it follows
        let mid = &tr.poses[frame(map.step_video_t(250))];
        let before = &tr.poses[frame(map.step_video_t(90))];
        let down = |p: &CamPose| {
            let d = sub(p.eye, p.target);
            (d[1] / dot(d, d).sqrt()).asin().to_degrees()
        };
        assert!(down(mid) < down(before) - 4.0, "{} {}", down(mid), down(before));
    }

    /// Two balls 2 apart in melee range, the near one on the left: portrait room to push in.
    fn diagonal(_: u32) -> [P3; 2] {
        [[-0.6, -0.8, 0.0], [0.6, 0.8, 0.0]]
    }

    fn punch(step: u32, who: usize, windup: u32, special: bool) -> Vec<Ev> {
        let mut v = vec![
            ev(step, who, EvKind::MoveStart { mv: 1 }),
            ev(step, who, EvKind::Windup { mv: 1 }),
            ev(step, who, EvKind::Cue(Cue::PunchIn)),
            ev(step + windup, who, EvKind::Active { mv: 1 }),
            ev(step + windup + 6, who, EvKind::Recovery { mv: 1 }),
        ];
        if special {
            v.push(ev(step + windup, who, EvKind::Cue(Cue::SlowMo { scale: 0.5 })));
        }
        v
    }

    #[test]
    fn punch_in_moves_closer_during_windup() {
        let rec = synthetic(1200, diagonal, punch(600, 0, 36, false), None);
        let (map, tr) = track_k(&rec, 1.0);
        // The springs read ahead to stay in step, so the move starts a beat early: measure from well before.
        let start = &tr.poses[frame(map.step_video_t(600) - 0.6)];
        let hit = &tr.poses[frame(map.step_video_t(636)) + 3];
        let (a, b) = (frame(map.step_video_t(600)), frame(map.step_video_t(636)));
        assert!(
            tr.poses[a..=b].windows(2).all(|w| dist(&w[1]) < dist(&w[0])),
            "closing all through the wind-up"
        );
        assert!(dist(hit) < 0.8 * dist(start), "{} {}", dist(hit), dist(start));
        // toward the attacker, on the left
        assert!(hit.target[0] < start.target[0] - 0.1, "{:?}", hit.target);
        // and back out after
        let later = &tr.poses[frame(map.step_video_t(636) + 2.0)];
        assert!((dist(later) - dist(start)).abs() < 0.05 * dist(start));
        // smooth: no frame jumps
        for w in tr.poses.windows(2) {
            assert!((dist(&w[1]) - dist(&w[0])).abs() < 1.5);
        }
    }

    #[test]
    fn big_shots_keep_their_spacing() {
        // Twenty punch-ins in ten seconds, alternating fighters; the eighth is a special.
        let mut ev = Vec::new();
        for i in 0..20 {
            ev.extend(punch(240 + i * 60, (i % 2) as usize, 20, i == 7));
        }
        let rec = synthetic(1800, standing, ev, None);
        let (_, tr) = track(&rec);
        let big: Vec<&Shot> = tr.shots.iter().filter(|s| s.kind.is_big()).collect();
        assert!(big.len() <= 6, "{}", big.len());
        assert!(big.len() >= 3, "{}", big.len());
        for w in big.windows(2) {
            let gap = w[1].t0 - w[0].t0;
            assert!(gap >= 2.5 - 1e-9 || w[1].priority > w[0].priority, "{gap}");
            assert!(w[0].t1 <= w[1].t0 + 1e-9, "no two big shots at once");
        }
        assert!(
            big.iter().any(|s| s.priority == Priority::Special),
            "the special cuts in"
        );
    }

    #[test]
    fn ko_shot_orbits_then_pulls_out() {
        let ev = vec![ev(1200, 0, EvKind::Ko { target: 1 })];
        let rec = synthetic(1320, diagonal, ev, Some((1, 1200)));
        let (map, tr) = track_k(&rec, 1.0);
        let ko = map.ko_video_t().unwrap();
        let t_ko = ko.clamp(44.5, 55.5);
        // The orbit, measured from the base shot's own yaw at that moment.
        let base_yaw = |t: f64| crate::base::base_rig(t, t_ko).3;
        let at = |t: f64| &tr.poses[frame(t)];
        let yaw = |t: f64| {
            let p = at(t);
            let d = sub(p.eye, p.target);
            d[0].atan2(d[2]).to_degrees() - base_yaw(t)
        };
        assert!(yaw(ko - 0.6).abs() < 0.5);
        let slow_end = map.video_t(1200.0 * DT + 0.8);
        let y_mid = yaw(0.5 * (ko + slow_end));
        let y_end = yaw(slow_end + 0.5);
        assert!(y_mid.abs() > 2.0 && y_mid.abs() < y_end.abs(), "{y_mid} {y_end}");
        assert!((y_end.abs() - 12.0).abs() < 1.5, "{y_end}");
        // pushed in on the loser through the freeze, then pulled out wide
        let before = dist(at(ko - 0.6));
        let pushed = dist(at(ko + 0.6));
        let wide = dist(at(ko + 5.0));
        assert!(pushed < 0.85 * before, "{pushed} {before}");
        assert!(wide > before, "{wide} {before}");
        let [winner, loser] = diagonal(0).map(ball_centre);
        let flat = |a: P3, b: P3| (a[0] - b[0]).hypot(a[2] - b[2]);
        let (t0, t1, t2) = (at(ko - 0.6).target, at(ko + 0.6).target, at(ko + 5.0).target);
        assert!(
            flat(t1, loser) < flat(t0, loser) - 0.25,
            "toward the loser: {t0:?} {t1:?}"
        );
        assert!(flat(t2, winner) < flat(t0, winner) - 0.25, "then the winner: {t2:?}");
    }

    /// By default the shots and the shake are subtle offsets around the current engine's camera: a punch-in comes
    /// at most 10 % closer, the K.O. orbit turns at most 3 degrees, the shake stays within 4 px, and through
    /// real fights the camera keeps within a few percent of the base shot's distance almost all the time.
    #[test]
    fn default_moves_are_subtle() {
        let rec = synthetic(1200, diagonal, punch(600, 0, 36, false), None);
        let (map, tr) = track(&rec);
        let start = dist(&tr.poses[frame(map.step_video_t(600) - 0.6)]);
        let hit = dist(&tr.poses[frame(map.step_video_t(636)) + 3]);
        assert!(hit < start && hit > 0.9 * start, "{hit} {start}");

        let ev = vec![ev(1200, 0, EvKind::Ko { target: 1 })];
        let rec = synthetic(1320, diagonal, ev, Some((1, 1200)));
        let (map, tr) = track(&rec);
        let ko = map.ko_video_t().unwrap();
        let t_ko = ko.clamp(44.5, 55.5);
        for f in frame(ko)..FRAMES {
            let p = &tr.poses[f];
            let d = sub(p.eye, p.target);
            let turn = d[0].atan2(d[2]).to_degrees() - crate::base::base_rig(f as f64 / FPS, t_ko).3;
            assert!(turn.abs() <= 3.0, "frame {f}: {turn}");
        }

        for seed in [3, 8] {
            let rec = dummy(seed, true);
            let (map, tr) = track(&rec);
            let t_ko = map.ko_video_t().unwrap_or(55.0).clamp(44.5, 55.5);
            assert!(tr.shake.iter().all(|s| s[0].hypot(s[1]) <= 4.0 * 2f64.sqrt()));
            let near = (0..FRAMES)
                .filter(|&f| {
                    let base = crate::base::base_rig(f as f64 / FPS, t_ko).1;
                    (dist(&tr.poses[f]) / base - 1.0).abs() < 0.08
                })
                .count();
            assert!(near as f64 >= 0.95 * FRAMES as f64, "seed {seed}: {near}");
        }
    }

    #[test]
    fn shake_decays_to_zero() {
        let ev = vec![ev(600, 0, EvKind::Cue(Cue::Shake(1.0)))];
        let rec = synthetic(1200, standing, ev, None);
        let (map, tr) = track(&rec);
        let t = map.step_video_t(600);
        assert!(tr.shake[..frame(t) - 1].iter().all(|s| *s == [0.0, 0.0]));
        let peak = (frame(t)..frame(t) + 4)
            .map(|f| tr.shake[f][0].hypot(tr.shake[f][1]))
            .fold(0.0, f64::max);
        assert!(peak > 2.0 && peak <= 18.0 * 2f64.sqrt(), "{peak}");
        assert!(tr.shake[frame(t + 1.0 / 1.6) + 1..].iter().all(|s| *s == [0.0, 0.0]));
        // and it fades, it does not stop dead
        let late = tr.shake[frame(t + 0.4)];
        assert!(late[0].hypot(late[1]) < 0.3 * peak);
    }

    #[test]
    fn track_is_deterministic() {
        let rec = dummy(5, true);
        let (_, a) = track(&rec);
        let (_, b) = track(&rec);
        let bits = |t: &CameraTrack| {
            t.poses
                .iter()
                .flat_map(|p| p.eye.into_iter().chain(p.target).chain([p.roll, p.fov_deg]))
                .chain(t.shake.iter().flatten().copied())
                .map(f64::to_bits)
                .collect::<Vec<_>>()
        };
        assert_eq!(bits(&a), bits(&b));
        let c = std::thread::spawn(move || track(&rec).1).join().unwrap();
        assert_eq!(bits(&a), bits(&c));
    }
}
