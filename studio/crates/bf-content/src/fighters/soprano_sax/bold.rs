//! SOPRANO SAX with the bold effects (`picture.effects = "bold"`; the owner's picks 13a to 13d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **HORNET SWARM** (13a): bigger, outlined, striped hornets circle on a ring of light on the floor; the ones
//!   about to leave blink red and rise; a chaser comes down on the enemy along its curved trail, stings (a
//!   solid hit) and flies back.
//! - **KICK THE HIVE** (13b): the ball squashes with a red pulse, the chargers are red and their thick red
//!   trails read as one stream; a red buzz closes round the enemy while they are on it; they stream home red.
//! - **BUZZ OFF** (13c): the swarm locks into a shell, one ring of hornets low and one high, spinning opposite
//!   ways inside a jagged yellow ring (the ring's own shield ring drains with the time); an enemy that touches
//!   it gets a burst of sting spikes.
//! - **QUEEN** (13d): a big crowned queen with two escorts and a shadow on the floor; red brackets mark the
//!   enemy while she hunts and a red landing mark closes in before each sting, for which she rises and dives;
//!   her stings are heavy hits.

use bf_core::dmath;
use bf_core::event::Event;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::{Cam, V3};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Phase, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::BallState;
use bf_picture::scene::{self, R_BALL as R};

use super::{BLACK, GOLD, SLOTS, SopranoSaxLook, W, WING, YELLOW, ellipse};

/// An angry hornet, and everything KICK THE HIVE and the queen's marks are drawn in.
const RED: V3 = [236.0, 52.0, 44.0];
/// How far out the bold swarm circles (the bigger hornets need the room).
const ORBIT: f64 = R + 0.34;
/// Seconds before they launch that the leavers blink and rise.
const LEAVE_S: f64 = 0.4;
/// Seconds the red pulse of KICK THE HIVE rolls.
const KICK_S: f64 = 0.4;
/// Seconds before a sting of hers that the queen rises and dives and the landing mark closes in.
const DIVE_S: f64 = 0.45;
/// Seconds the sting spikes of BUZZ OFF show.
const SPIKES_S: f64 = 0.3;
/// Frames of a hornet's trail.
const TRAIL: usize = 6;

/// What a hornet is doing, for its colours.
#[derive(Clone, Copy, PartialEq)]
enum Mood {
    /// Yellow and black.
    Calm,
    /// Red and black: a KICK THE HIVE charger, or a leaver on a red blink.
    Angry,
}

/// One painter item: a hornet.
struct BBee {
    x: f64,
    z: f64,
    y: f64,
    head: f64,
    size: f64,
    mood: Mood,
    /// Where it has been (ring frame, oldest first, ending where it is).
    trail: Vec<(f64, f64)>,
    /// How wide its trail's body is drawn, in pixels.
    trail_w: u32,
}

/// The queen now: where she is, her height, her heading.
#[derive(Clone, Copy)]
struct Queen {
    x: f64,
    z: f64,
    y: f64,
    head: f64,
}

/// The bold hornet: a trail, two flickering wings, an outlined body with two stripes, a head and a stinger.
fn draw_bee(fr: &mut Frame, cam: &Cam, b: &BBee, t: f64) {
    let s = b.size;
    let body = if b.mood == Mood::Angry { RED } else { YELLOW };
    if b.trail.len() >= 2 {
        let pts: Vec<V3> = b.trail.iter().map(|q| scene::to_world(q.0, q.1, b.y)).collect();
        fr.line(cam, &pts, bold::INK, b.trail_w + 2);
        fr.line(
            cam,
            &pts,
            if b.trail_w > 1 { body } else { bold::pale(body) },
            b.trail_w,
        );
    }
    let (c, sn) = (dmath::cos(b.head), dmath::sin(b.head));
    let flap = if (t * 30.0 + b.x * 7.0).floor() as i64 % 2 == 0 {
        1.0
    } else {
        0.55
    };
    for side in [-1.0, 1.0] {
        let (wx, wz) = (b.x - sn * side * s * 0.95 * flap, b.z + c * side * s * 0.95 * flap);
        bold::outlined_poly(
            fr,
            cam,
            &ellipse(wx, wz, b.y + 0.04, s * 0.6, s * 0.34, b.head + side * 0.5, 8),
            WING,
        );
    }
    // the stinger behind
    let at = |along: f64, across: f64, up: f64| {
        scene::to_world(
            b.x + c * s * along - sn * s * across,
            b.z + sn * s * along + c * s * across,
            b.y + up,
        )
    };
    fr.poly(
        cam,
        &[at(-1.0, 0.22, 0.0), at(-1.75, 0.0, 0.0), at(-1.0, -0.22, 0.0)],
        BLACK,
        true,
    );
    bold::outlined_poly(fr, cam, &ellipse(b.x, b.z, b.y, s * 1.15, s * 0.66, b.head, 10), body);
    for along in [-0.45, 0.12] {
        fr.poly(
            cam,
            &ellipse(
                b.x + c * s * along,
                b.z + sn * s * along,
                b.y + 0.01,
                s * 0.15,
                s * 0.6,
                b.head,
                6,
            ),
            BLACK,
            true,
        );
    }
    fr.poly(
        cam,
        &ellipse(
            b.x + c * s * 0.95,
            b.z + sn * s * 0.95,
            b.y + 0.01,
            s * 0.42,
            s * 0.42,
            b.head,
            8,
        ),
        BLACK,
        true,
    );
}

impl SopranoSaxLook {
    /// The hornets in the air of a row: x, z and the record's code of each filled slot.
    fn flying(row: &[f64; W]) -> impl Iterator<Item = (f64, f64, f64)> + '_ {
        (0..SLOTS)
            .map(|s| (row[3 + 3 * s], row[4 + 3 * s], row[5 + 3 * s]))
            .filter(|b| !b.0.is_nan())
    }

    /// Where the hornet of kind `what` now at `(x, z)` has been over the last [`TRAIL`] frames (oldest first,
    /// ending at `(x, z)`): frame by frame the nearest hornet of its kind, as the record's slots move up when
    /// one lands.
    fn trail(p: &PictureCtx, i: usize, k: usize, what: f64, x: f64, z: f64) -> Vec<(f64, f64)> {
        let mut pts = vec![(x, z)];
        let mut at = (x, z);
        for back in 1..=TRAIL.min(i) {
            let Some(row) = Self::hrn(p, i - back, k) else { break };
            let near = Self::flying(&row)
                .filter(|b| b.2 % 10.0 == what)
                .map(|b| (dmath::hypot(b.0 - at.0, b.1 - at.1), b))
                .min_by(|a, b| a.0.total_cmp(&b.0));
            match near {
                Some((d, b)) if d < 0.6 => {
                    at = (b.0, b.1);
                    pts.push(at);
                }
                _ => break,
            }
        }
        pts.reverse();
        pts
    }

    /// The way a hornet with this trail is heading (its last step), or a slow spin when it has not moved.
    fn heading(trail: &[(f64, f64)], t: f64) -> f64 {
        match trail {
            [.., a, b] if dmath::hypot(b.0 - a.0, b.1 - a.1) > 1e-6 => dmath::atan2(b.1 - a.1, b.0 - a.0),
            _ => t * 6.0,
        }
    }

    /// The queen at recording frame `i`, time `t`: she rises and dives for each of her stings.
    fn queen(&self, p: &PictureCtx, i: usize, t: f64) -> Option<Queen> {
        let row = Self::hrn(p, i, self.k)?;
        let (x, z, _) = Self::flying(&row).find(|b| b.2 % 10.0 == 3.0)?;
        let trail = Self::trail(p, i, self.k, 3.0, x, z);
        let y = match bold::until(p, self.k, &["hit:queen"], t, DIVE_S) {
            Some((u, _)) if u < 0.4 => 0.8 + 0.6 * bold::ease_out(u / 0.4),
            Some((u, _)) => 1.4 - 0.75 * bold::ease_in((u - 0.4) / 0.6),
            None => 0.8,
        };
        Some(Queen {
            x,
            z,
            y,
            head: Self::heading(&trail, t),
        })
    }

    /// The ball squashes as it kicks the hive.
    pub(super) fn bold_state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase) {
        if kk != self.k || phase != Phase::Fight {
            return;
        }
        if let Some((age, _)) = bold::since(p, self.k, &["hive"], t, 0.2) {
            let k = 1.0 - age / 0.2;
            st.sx = Some(1.0 + 0.24 * k);
            st.sy = Some(1.0 - 0.2 * k);
        }
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let k = self.k;
        let Some(row) = Self::hrn(p, c.i, k) else {
            return;
        };
        let (me, them) = (&c.fs[k], &c.fs[1 - k]);
        let yellow = bold::light(YELLOW);
        let red = bold::light(RED);
        if !me.shattered {
            if row[2] > 0.0 {
                // BUZZ OFF: a jagged, buzzing ring hugging the shell
                let fade = (row[2] / 0.3).clamp(0.0, 1.0);
                floor::band(ic, c, Family::Noise, me.p[0], me.p[1], R + 0.42, fade, c.t, yellow, 1.5);
                floor::ring(ic, c, me.p[0], me.p[1], R + 0.12, 0.05, yellow, 0.7 * fade);
            } else if row[1] > 0.0 {
                // the ring the swarm circles on
                floor::ring(ic, c, me.p[0], me.p[1], ORBIT, 0.05, yellow, 0.9);
            }
        }
        // KICK THE HIVE: a red pulse out of the ball
        if let Some((age, _)) = bold::since(p, k, &["hive"], c.t, KICK_S) {
            let u = age / KICK_S;
            floor::band(
                ic,
                c,
                Family::Air,
                me.p[0],
                me.p[1],
                R + 2.4 * bold::ease_out(u),
                1.0 - u,
                c.t,
                red,
                1.8,
            );
            floor::ring(
                ic,
                c,
                me.p[0],
                me.p[1],
                R + 1.2 * bold::ease_out(u),
                0.08,
                red,
                1.4 * (1.0 - u),
            );
        }
        // ... and a red buzz round the enemy while the chargers are on it
        let on_them = Self::flying(&row)
            .filter(|b| b.2 == 2.0 && dmath::hypot(b.0 - them.p[0], b.1 - them.p[1]) < R + 0.7)
            .count();
        if on_them > 0 {
            floor::band(
                ic,
                c,
                Family::Noise,
                them.p[0],
                them.p[1],
                R * 1.4,
                1.0,
                c.t,
                red,
                0.5 + 0.35 * on_them.min(4) as f64,
            );
        }
        // QUEEN: her shadow on the floor, and the landing mark closing in on the enemy before each sting
        if let Some(q) = self.queen(p, c.i, c.t) {
            floor::shade(ic, c, q.x, q.z, 0.38 + 0.12 * q.y, 0.8);
            if let Some((u, _)) = bold::until(p, k, &["hit:queen"], c.t, DIVE_S) {
                floor::target(ic, c, them.p[0], them.p[1], R * 1.15, u, red);
            }
        }
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if !f.in_fight {
            return Vec::new();
        }
        let k = self.k;
        let Some(row) = Self::hrn(p, f.i, k) else {
            return Vec::new();
        };
        let mut bees: Vec<BBee> = Vec::new();
        let n = (row[1].max(0.0) as usize).min(16);
        let me = &f.fs[k];
        let them = f.fs[1 - k].p;
        if !me.shattered && n > 0 {
            let b = me.p;
            let step = dmath::TAU / n as f64;
            if row[2] > 0.0 {
                // BUZZ OFF: a shell of two rings spinning opposite ways, one low and one high
                for (turn, y, d, size) in [(1.0, 0.4, R + 0.3, 0.15), (-1.0, 0.95, R + 0.16, 0.13)] {
                    for i in 0..n.max(4) {
                        let a = turn * f.t * 10.0 + dmath::TAU * i as f64 / n.max(4) as f64;
                        bees.push(BBee {
                            x: b[0] + d * dmath::cos(a),
                            z: b[1] + d * dmath::sin(a),
                            y,
                            head: a + turn * dmath::PI / 2.0,
                            size,
                            mood: Mood::Calm,
                            trail: Vec::new(),
                            trail_w: 1,
                        });
                    }
                }
            } else {
                // the ones about to leave blink red and rise
                let leaving = bold::until(p, k, &["hornet"], f.t, LEAVE_S).map(|(u, e)| (u, e.i(3).max(0) as usize));
                for i in 0..n {
                    let a = f.t * 4.0 + step * i as f64;
                    let (mut y, mut mood, mut size) =
                        (0.55 + 0.08 * dmath::sin(f.t * 9.0 + i as f64), Mood::Calm, 0.15);
                    if let Some((u, count)) = leaving
                        && i < count
                    {
                        y += 0.45 * bold::ease_out(u);
                        size += 0.03 * u;
                        if u > 0.55 || (f.t * 14.0).floor() as i64 % 2 == 0 {
                            mood = Mood::Angry;
                        }
                    }
                    bees.push(BBee {
                        x: b[0] + ORBIT * dmath::cos(a),
                        z: b[1] + ORBIT * dmath::sin(a),
                        y,
                        head: a + dmath::PI / 2.0,
                        size,
                        mood,
                        trail: Vec::new(),
                        trail_w: 1,
                    });
                }
            }
        }
        // the hornets in the air
        let queen = self.queen(p, f.i, f.t);
        for (x, z, code) in Self::flying(&row) {
            let what = code % 10.0;
            if what == 3.0 {
                continue;
            }
            let home = code >= 10.0;
            let trail = Self::trail(p, f.i, k, what, x, z);
            let hive = what == 2.0;
            // a hunter comes down on the enemy from above
            let far = (dmath::hypot(x - them[0], z - them[1]) / 1.6).min(1.0);
            let y = if home { 0.7 } else { 0.6 + 0.4 * far };
            bees.push(BBee {
                x,
                z,
                y,
                head: Self::heading(&trail, f.t),
                size: 0.16,
                mood: if hive { Mood::Angry } else { Mood::Calm },
                trail,
                trail_w: if hive { 3 } else { 1 },
            });
        }
        if let Some(q) = queen {
            let trail = Self::trail(p, f.i, k, 3.0, q.x, q.z);
            bees.push(BBee {
                x: q.x,
                z: q.z,
                y: q.y,
                head: q.head,
                size: 0.3,
                mood: Mood::Calm,
                trail,
                trail_w: 3,
            });
            // her two escorts
            for side in [0.0, dmath::PI] {
                let a = f.t * 7.0 + side;
                bees.push(BBee {
                    x: q.x + 0.5 * dmath::cos(a),
                    z: q.z + 0.5 * dmath::sin(a),
                    y: q.y - 0.1,
                    head: a + dmath::PI / 2.0,
                    size: 0.11,
                    mood: Mood::Calm,
                    trail: Vec::new(),
                    trail_w: 1,
                });
            }
        }
        bees.into_iter()
            .map(|b| {
                let depth = f.cam.depth(&[scene::to_world(b.x, b.z, b.y)])[0];
                (depth, Box::new(b) as Payload)
            })
            .collect()
    }

    pub(super) fn bold_draw_item(&self, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        if let Some(b) = payload.downcast_ref::<BBee>() {
            draw_bee(fr, f.cam, b, f.t);
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let (k, o, t, cam) = (self.k, 1 - self.k, f.t, f.cam);
        let me = drawn.iter().find(|d| d.k == k);
        let them = drawn.iter().find(|d| d.k == o);
        // KICK THE HIVE: the kick's red ring pops off the ball
        if let (Some((age, _)), Some(me)) = (bold::since(p, k, &["hive"], t, 0.25), me) {
            let u = age / 0.25;
            px::ink_ring(img, me.s, me.rp * (1.05 + 0.9 * bold::ease_out(u)), 2.5, RED, 1.0 - u);
        }
        // BUZZ OFF: a burst of sting spikes into the enemy that touched the shell
        if let Some((age, e)) = bold::since(p, k, &["hit:buzz"], t, SPIKES_S) {
            let u = age / SPIKES_S;
            let at = bold::at(cam, e.f(5), e.f(6), R);
            let from = bold::at(cam, f.fs[k].p[0], f.fs[k].p[1], R);
            let out = dmath::atan2(at.1 - from.1, at.0 - from.0);
            let reach = 14.0 + 22.0 * bold::ease_out(u * 2.0);
            for j in -3..=3 {
                let a = out + f64::from(j) * 0.34;
                let (ca, sa) = (dmath::cos(a), dmath::sin(a));
                let len = reach * if j % 2 == 0 { 1.0 } else { 0.7 };
                let base = (at.0 + ca * 3.0, at.1 + sa * 3.0);
                let spike = [
                    (base.0 - sa * 3.0, base.1 + ca * 3.0),
                    (at.0 + ca * (3.0 + len), at.1 + sa * (3.0 + len)),
                    (base.0 + sa * 3.0, base.1 - ca * 3.0),
                ];
                px::ink_poly(img, &spike, if j % 2 == 0 { YELLOW } else { bold::WHITE }, 1.0 - u * u);
            }
        }
        // QUEEN: red brackets hold the enemy while she hunts, and her crown
        if let Some(q) = self.queen(p, f.i, t) {
            if let Some(them) = them
                && f.fs[o].vis
                && !f.fs[o].shattered
            {
                let lock = bold::until(p, k, &["hit:queen"], t, DIVE_S).map_or(0.0, |(u, _)| u);
                let beat = 0.5 + 0.5 * dmath::sin(t * 9.0);
                px::brackets(img, them.s, them.rp * (1.55 - 0.3 * lock + 0.06 * beat), 1.8, RED, 1.0);
            }
            let (c, sn) = (dmath::cos(q.head), dmath::sin(q.head));
            let head = bold::at(cam, q.x + c * 0.2, q.z + sn * 0.2, q.y + 0.1);
            let unit = bold::px_per_unit(cam, q.x, q.z, q.y);
            let (w, h) = (0.2 * unit, 0.22 * unit);
            let base = head.1 - 0.06 * unit;
            let crown = [
                (head.0 - w, base),
                (head.0 - w, base - h),
                (head.0 - w * 0.5, base - h * 0.45),
                (head.0, base - h * 1.15),
                (head.0 + w * 0.5, base - h * 0.45),
                (head.0 + w, base - h),
                (head.0 + w, base),
            ];
            px::ink_poly(img, &crown, GOLD, 1.0);
        }
    }

    /// A chaser's sting and the shell's sting are solid hits, the queen's is heavy, and the chargers of KICK THE
    /// HIVE tap (they come many at a time).
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match e.s(7) {
            "queen" => Some(HitSize::Heavy),
            "sting" | "buzz" => Some(by_damage.max(HitSize::Solid)),
            "hive" => Some(HitSize::Tap),
            _ => Some(by_damage),
        }
    }
}
