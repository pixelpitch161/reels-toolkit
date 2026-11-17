//! Gravity, the canvas, the ropes and ball against ball.

use bf_core::dmath;
use bf_core::rng::Generator;

use crate::body::Body;
use crate::event::{EvKind, Events, Surface};
use crate::rules::Rules2;
use crate::v3::{add, dot, len, norm, scale, sub};

/// A landing faster than this (units/s, downward) bounces instead.
const LAND_SPEED: f64 = 2.0;
/// A rope bounce slower than this makes no event.
const ROPE_EVENT_SPEED: f64 = 1.0;
/// Ball against ball restitution.
const RESTITUTION: f64 = 0.6;

/// One step of motion for one ball. Nothing happens while it is frozen (`hitstop`) or held. Otherwise: gravity
/// when airborne, `pos += vel·dt`, the canvas (a fast landing bounces, a slow one lands), friction on the canvas
/// when not in hitstun, then the ropes on x and then y (clamp, reflect, a seeded turn of up to `rope_give_deg`).
/// Clamping each axis after the move is enough at 120 Hz: at speed 60 a step is 0.5, far less than the ring.
pub fn integrate(b: &mut Body, rules: &Rules2, rng: &mut Generator, ev: &mut Events, who: usize, step: u32) {
    if b.hitstop > 0 || b.held_by.is_some() {
        return;
    }
    if !b.airborne && (b.pos[2] > 0.0 || b.vel[2] > 0.0) {
        b.airborne = true;
    }
    if b.airborne {
        b.vel[2] += rules.gravity * rules.dt;
    }
    b.pos = add(b.pos, scale(b.vel, rules.dt));
    if b.pos[2] < 0.0 {
        b.pos[2] = 0.0;
        if b.vel[2] < -LAND_SPEED {
            b.vel[2] = -b.vel[2] * rules.floor_bounce;
            ev.push(step, who, EvKind::Bounce { what: Surface::Floor });
        } else {
            b.vel[2] = 0.0;
            if b.airborne {
                ev.push(step, who, EvKind::Land);
            }
            b.airborne = false;
            b.juggle = 0;
        }
    }
    if !b.airborne && b.hitstun == 0 {
        let k = (1.0 - rules.friction * rules.dt).max(0.0);
        b.vel[0] *= k;
        b.vel[1] *= k;
    }
    let lim = rules.a - b.r;
    for axis in 0..2 {
        if b.pos[axis].abs() <= lim {
            continue;
        }
        let side = if b.pos[axis] > 0.0 { 1.0 } else { -1.0 };
        b.pos[axis] = side * lim;
        if b.vel[axis] * side <= 0.0 {
            continue;
        }
        b.vel[axis] = -b.vel[axis] * rules.rope_bounce;
        let give = rules.rope_give_deg * dmath::PI / 180.0;
        let th = rng.uniform(-give, give);
        let (c, s) = (dmath::cos(th), dmath::sin(th));
        let (vx, vy) = (b.vel[0], b.vel[1]);
        b.vel[0] = c * vx - s * vy;
        b.vel[1] = s * vx + c * vy;
        if dmath::hypot(b.vel[0], b.vel[1]) > ROPE_EVENT_SPEED {
            ev.push(step, who, EvKind::Bounce { what: Surface::Rope });
        }
    }
}

/// Keeps a ball's position inside the ropes and on or above the canvas (after a push or a carry), without
/// touching its velocity.
pub fn confine(b: &mut Body, rules: &Rules2) {
    let lim = rules.a - b.r;
    b.pos[0] = b.pos[0].clamp(-lim, lim);
    b.pos[1] = b.pos[1].clamp(-lim, lim);
    b.pos[2] = b.pos[2].max(0.0);
}

/// Sphere against sphere in 3D: overlapping balls are pushed apart by inverse mass, and approaching ones trade
/// an impulse with restitution 0.6, so momentum is kept. Skipped when either is held or frozen.
pub fn collide(a: &mut Body, b: &mut Body) {
    if a.held_by.is_some() || b.held_by.is_some() || a.hitstop > 0 || b.hitstop > 0 {
        return;
    }
    let d = sub(b.centre(), a.centre());
    let dist = len(d);
    let min = a.r + b.r;
    if dist >= min {
        return;
    }
    let n = if dist > 0.0 { norm(d) } else { [1.0, 0.0, 0.0] };
    let (wa, wb) = (1.0 / a.mass, 1.0 / b.mass);
    let w = wa + wb;
    let pen = min - dist;
    a.pos = sub(a.pos, scale(n, pen * wa / w));
    b.pos = add(b.pos, scale(n, pen * wb / w));
    let closing = dot(sub(b.vel, a.vel), n);
    if closing < 0.0 {
        let j = -(1.0 + RESTITUTION) * closing / w;
        a.vel = sub(a.vel, scale(n, j * wa));
        b.vel = add(b.vel, scale(n, j * wb));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EvKind, Surface};
    use crate::v3::V3;

    fn ball(pos: V3, vel: V3) -> Body {
        let mut b = Body::new(pos, 0.0, 0.6, 1.0, 100.0, 0);
        b.vel = vel;
        b
    }

    #[test]
    fn launched_ball_lands() {
        let rules = Rules2::new(4.5);
        let mut rng = Generator::new(1);
        let mut ev = Events::default();
        let mut b = ball([0.0; 3], [0.0, 0.0, 8.0]);
        let mut landed_at = None;
        for step in 0..180 {
            integrate(&mut b, &rules, &mut rng, &mut ev, 0, step);
            assert!(b.pos[2] >= 0.0);
            if step > 2 && !b.airborne && b.pos[2] == 0.0 {
                landed_at = Some(step);
                break;
            }
        }
        assert!(landed_at.is_some(), "still in the air after 1.5 s: {:?}", b.pos);
        assert!(
            ev.0.iter()
                .any(|e| matches!(e.kind, EvKind::Land | EvKind::Bounce { what: Surface::Floor }))
        );
        assert!(matches!(ev.0.last().unwrap().kind, EvKind::Land));
        assert_eq!(b.vel[2], 0.0);
    }

    #[test]
    fn fast_ball_never_leaves_the_ring() {
        let rules = Rules2::new(4.5);
        let lim = rules.a - rules.r;
        for k in 0..8 {
            for down in [0.0, -60.0] {
                let th = k as f64 * bf_core::dmath::PI / 4.0 + 0.1;
                let mut b = ball([0.0, 0.0, 1.0], [60.0 * dmath::cos(th), 60.0 * dmath::sin(th), down]);
                let (mut rng, mut ev) = (Generator::new(k), Events::default());
                for step in 0..600 {
                    integrate(&mut b, &rules, &mut rng, &mut ev, 0, step);
                    assert!(b.pos[0].abs() <= lim && b.pos[1].abs() <= lim, "{k} {step} {:?}", b.pos);
                    assert!(b.pos[2] >= 0.0, "{k} {step} {:?}", b.pos);
                }
                assert!(
                    ev.0.iter()
                        .any(|e| matches!(e.kind, EvKind::Bounce { what: Surface::Rope }))
                );
            }
        }
    }

    #[test]
    fn hitstop_freezes() {
        let rules = Rules2::new(4.5);
        let (mut rng, mut ev) = (Generator::new(1), Events::default());
        let mut b = ball([0.5, 0.5, 1.0], [3.0, 1.0, 2.0]);
        b.hitstop = 5;
        for step in 0..5 {
            integrate(&mut b, &rules, &mut rng, &mut ev, 0, step);
            assert_eq!((b.pos, b.vel), ([0.5, 0.5, 1.0], [3.0, 1.0, 2.0]));
        }
        b.held_by = Some(1);
        b.hitstop = 0;
        integrate(&mut b, &rules, &mut rng, &mut ev, 0, 6);
        assert_eq!(b.pos, [0.5, 0.5, 1.0]);
        b.held_by = None;
        integrate(&mut b, &rules, &mut rng, &mut ev, 0, 7);
        assert_ne!(b.pos, [0.5, 0.5, 1.0]);
    }

    #[test]
    fn rope_give_is_seeded() {
        let rules = Rules2::new(4.5);
        let bounce = |seed| {
            let (mut rng, mut ev) = (Generator::new(seed), Events::default());
            let mut b = ball([2.38, 0.0, 0.0], [6.0, 0.0, 0.0]);
            integrate(&mut b, &rules, &mut rng, &mut ev, 0, 0);
            assert!(matches!(ev.0[0].kind, EvKind::Bounce { what: Surface::Rope }));
            assert!(b.vel[0] < 0.0);
            b.vel
        };
        assert_eq!(bounce(3), bounce(3));
        assert_ne!(bounce(3), bounce(4));
    }

    #[test]
    fn collide_separates() {
        let mut a = ball([0.0, 0.0, 0.0], [2.0, 0.5, 0.0]);
        let mut b = ball([0.7, 0.2, 0.3], [-1.0, 0.0, 0.0]);
        a.mass = 1.0;
        b.mass = 2.5;
        let p0 = add(scale(a.vel, a.mass), scale(b.vel, b.mass));
        collide(&mut a, &mut b);
        assert!(len(sub(b.centre(), a.centre())) >= a.r + b.r - 1e-9);
        let p1 = add(scale(a.vel, a.mass), scale(b.vel, b.mass));
        assert!((0..3).all(|i| (p0[i] - p1[i]).abs() <= 1e-12), "{p0:?} {p1:?}");
        // now separating, so a second call changes nothing
        let (va, vb) = (a.vel, b.vel);
        collide(&mut a, &mut b);
        assert_eq!((a.vel, b.vel), (va, vb));
        // a held ball is skipped
        let mut c = ball([0.1, 0.0, 0.0], [0.0; 3]);
        c.held_by = Some(0);
        let mut d = ball([0.0; 3], [0.0; 3]);
        collide(&mut c, &mut d);
        assert_eq!(c.pos, [0.1, 0.0, 0.0]);
    }
}
