//! Projectiles: launched by a move's spawn, they fly, home, bounce on the ropes, hit, and always expire with an
//! event.

use bf_core::dmath;

use crate::event::EvKind;
use crate::hits::apply_hit;
pub use crate::moves::ProjDef;
use crate::v3::{V3, add, len, norm, scale, sub};
use crate::world::World;

/// A live (or just expired) projectile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Proj {
    /// (fighter, move, spawn) of its `ProjDef`.
    pub def: (usize, usize, usize),
    pub owner: usize,
    /// The centre.
    pub pos: V3,
    pub vel: V3,
    pub age: u32,
    pub bounces: u32,
    pub alive: bool,
    /// Unique within the fight, in spawn order.
    pub id: u32,
}

/// `a - b` wrapped into -π to π, for angles that are each within -π to π.
pub(crate) fn angle_diff(a: f64, b: f64) -> f64 {
    let mut d = a - b;
    if d > dmath::PI {
        d -= dmath::TAU;
    } else if d < -dmath::PI {
        d += dmath::TAU;
    }
    d
}

/// Launches spawn `spawn` of fighter `who`'s move `mv`: from the tip of the last bone (or the ball's surface when
/// the rig is empty), along the fighter's facing at the projectile's speed plus its upward speed. Emits Spawn.
pub(crate) fn spawn(world: &mut World, who: usize, mv: usize, spawn: usize, step: u32) {
    let def = &world.defs[who].moves[mv].spawns[spawn].proj;
    let b = &world.bodies[who];
    let (c, s) = (dmath::cos(b.facing), dmath::sin(b.facing));
    let pos = match world.segments[who].last() {
        Some(seg) => seg.b,
        None => add(b.centre(), [c * b.r, s * b.r, 0.0]),
    };
    let vel = [c * def.speed, s * def.speed, def.up];
    let id = world.next_proj;
    world.next_proj += 1;
    world.projs.push(Proj {
        def: (who, mv, spawn),
        owner: who,
        pos,
        vel,
        age: 0,
        bounces: 0,
        alive: true,
        id,
    });
    world.events.push(step, who, EvKind::Spawn { proj: id as usize });
}

fn expire(world: &mut World, i: usize, step: u32) {
    let p = &mut world.projs[i];
    if p.alive {
        p.alive = false;
        world.events.push(step, p.owner, EvKind::Expire { proj: p.id as usize });
    }
}

/// Step stage 9. Each live projectile, in spawn order: ages, homes (turning its floor heading toward the target's
/// centre by at most `homing·dt`), falls if it has gravity, moves, skips off the canvas, then hits the opponent's
/// ball (ProjHit, the hit through `apply_hit`, then it expires), bounces on the ropes (expiring once past its
/// bounces) and expires at the end of its life. Every projectile emits Expire exactly once. Expired ones are
/// dropped at the end, keeping the buffer.
pub fn step_projs(world: &mut World, step: u32) {
    let defs = world.defs.clone();
    for i in 0..world.projs.len() {
        if !world.projs[i].alive {
            continue;
        }
        let mut p = world.projs[i];
        let def = &defs[p.def.0].moves[p.def.1].spawns[p.def.2].proj;
        let target = 1 - p.owner;
        let tc = world.bodies[target].centre();
        let rules = &world.rules;
        p.age += 1;
        if def.homing > 0.0 {
            let want = dmath::atan2(tc[1] - p.pos[1], tc[0] - p.pos[0]);
            let now = dmath::atan2(p.vel[1], p.vel[0]);
            let limit = def.homing * rules.dt;
            let turn = angle_diff(want, now).clamp(-limit, limit);
            let (c, s) = (dmath::cos(turn), dmath::sin(turn));
            let (vx, vy) = (p.vel[0], p.vel[1]);
            p.vel[0] = c * vx - s * vy;
            p.vel[1] = s * vx + c * vy;
        }
        if def.gravity {
            p.vel[2] += rules.gravity * rules.dt;
        }
        p.pos = add(p.pos, scale(p.vel, rules.dt));
        if p.pos[2] < def.radius {
            p.pos[2] = def.radius;
            if p.vel[2] < 0.0 {
                p.vel[2] = -p.vel[2] * rules.floor_bounce;
            }
        }
        let mut gone = false;
        let tb = &world.bodies[target];
        if world.ko.is_none()
            && tb.hp > 0.0
            && world.bodies[p.owner].hp > 0.0
            && tb.held_by.is_none()
            && len(sub(tc, p.pos)) <= def.radius + tb.r
        {
            let point = add(tc, scale(norm(sub(p.pos, tc)), tb.r));
            let normal = norm(sub(tc, point));
            world.projs[i] = p;
            world.events.push(
                step,
                p.owner,
                EvKind::ProjHit {
                    proj: p.id as usize,
                    target,
                    point,
                },
            );
            apply_hit(world, p.owner, target, &def.hit, point, normal, &[], p.def.1, step);
            gone = true;
        } else {
            let lim = world.rules.a - def.radius;
            for axis in 0..2 {
                if p.pos[axis].abs() > lim {
                    let side = if p.pos[axis] > 0.0 { 1.0 } else { -1.0 };
                    p.pos[axis] = side * lim;
                    if p.vel[axis] * side > 0.0 {
                        p.vel[axis] = -p.vel[axis];
                        p.bounces += 1;
                        gone |= p.bounces > def.bounces;
                    }
                }
            }
            gone |= p.age >= def.life;
            world.projs[i] = p;
        }
        if gone {
            expire(world, i, step);
        }
    }
    world.projs.retain(|p| p.alive);
}

/// Expires every live projectile (the end of the fight), so each one still has its exit event.
pub(crate) fn expire_all(world: &mut World, step: u32) {
    for i in 0..world.projs.len() {
        expire(world, i, step);
    }
    world.projs.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EvKind;
    use crate::moves::{MoveDef, Spawn};
    use crate::testkit::*;
    use crate::world::World;
    use bf_core::rng::Generator;

    fn dart(life: u32) -> ProjDef {
        ProjDef {
            name: "DART",
            speed: 6.0,
            up: 0.0,
            gravity: false,
            radius: 0.1,
            life,
            hit: crate::moves::HitDef {
                bones: vec![],
                ..hit(4.0)
            },
            bounces: 0,
            homing: 0.0,
        }
    }

    /// Fighter 0 at the origin facing +x with a move that throws `p`; fighter 1 at `(x1, y1)`.
    fn thrower(p: ProjDef, x1: f64, y1: f64) -> World {
        let mv = MoveDef {
            spawns: vec![Spawn { at: 0, proj: p }],
            ..poke(vec![])
        };
        let mut w = world([fighter(vec![mv]), fighter(vec![poke(vec![])])], 0.0, x1);
        w.bodies[1].pos[1] = y1;
        set_move(&mut w, 0, 0, 0);
        spawn(&mut w, 0, 0, 0, 0);
        w
    }

    fn run(w: &mut World, steps: u32) {
        for s in 1..=steps {
            step_projs(w, s);
            w.bodies.iter_mut().for_each(|b| b.countdown());
        }
    }

    #[test]
    fn projectile_expires_with_one_event() {
        let mut w = thrower(dart(10), -2.0, 0.0);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Spawn { proj: 0 })), 1);
        let p = &w.projs[0];
        assert!(close(p.pos, w.segments[0][0].b, 1e-12));
        assert_eq!(p.vel, [6.0, 0.0, 0.0]);
        run(&mut w, 9);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Expire { .. })), 0);
        run(&mut w, 30);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Expire { proj: 0 })), 1);
        assert!(w.projs.is_empty());
        assert_eq!(w.bodies[1].hp, 100.0);
    }

    #[test]
    fn hit_projectile_expires_and_damages() {
        let mut w = thrower(dart(120), 2.0, 0.0);
        run(&mut w, 120);
        assert_eq!(
            count(&w, |k| matches!(k, EvKind::ProjHit { proj: 0, target: 1, .. })),
            1
        );
        assert_eq!(count(&w, |k| matches!(k, EvKind::Contact { target: 1, .. })), 1);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Expire { proj: 0 })), 1);
        assert_eq!(w.bodies[1].hp, 96.0);
        let hit_at = w
            .events
            .0
            .iter()
            .position(|e| matches!(e.kind, EvKind::ProjHit { .. }))
            .unwrap();
        assert!(matches!(w.events.0.last().unwrap().kind, EvKind::Expire { .. }));
        assert!(hit_at < w.events.0.len() - 1);
    }

    #[test]
    fn a_knocked_out_owner_deals_no_damage() {
        let mut w = thrower(dart(120), 2.0, 0.0);
        w.bodies[0].hp = -1.0;
        run(&mut w, 120);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Contact { .. })), 0);
        assert_eq!(w.bodies[1].hp, 100.0);
    }

    #[test]
    fn projectile_bounces_then_expires() {
        let mut w = thrower(
            ProjDef {
                speed: 20.0,
                bounces: 1,
                ..dart(600)
            },
            0.0,
            2.0,
        );
        let mut turned = false;
        for s in 1..200 {
            step_projs(&mut w, s);
            if let Some(p) = w.projs.first() {
                turned |= p.vel[0] < 0.0;
                assert!(p.pos[0].abs() <= w.rules.a - 0.1);
            }
        }
        assert!(turned, "it bounced once");
        assert_eq!(count(&w, |k| matches!(k, EvKind::Expire { proj: 0 })), 1);
        assert!(w.projs.is_empty(), "expired on the second rope, long before its life");
    }

    #[test]
    fn homing_turns_toward_target() {
        let y_after = |homing| {
            let mut w = thrower(
                ProjDef {
                    homing,
                    speed: 3.0,
                    ..dart(600)
                },
                0.0,
                2.2,
            );
            run(&mut w, 30);
            let p = &w.projs[0];
            assert!(
                (crate::v3::len([p.vel[0], p.vel[1], 0.0]) - 3.0).abs() < 1e-9,
                "homing keeps the speed"
            );
            p.pos[1]
        };
        assert_eq!(y_after(0.0), 0.0);
        assert!(y_after(3.0) > 0.1);
    }

    #[test]
    fn every_spawn_has_an_expire() {
        let mut g = Generator::new(2026);
        for case in 0..200 {
            let p = ProjDef {
                speed: g.uniform(0.0, 15.0),
                up: g.uniform(0.0, 6.0),
                gravity: g.random() < 0.5,
                radius: g.uniform(0.05, 0.4),
                life: 1 + (g.random() * 300.0) as u32,
                bounces: (g.random() * 4.0) as u32,
                homing: if g.random() < 0.5 { 0.0 } else { g.uniform(0.0, 4.0) },
                ..dart(1)
            };
            let mut w = thrower(p, g.uniform(-2.4, 2.4), g.uniform(-2.4, 2.4));
            w.bodies[0].facing = g.uniform(-3.0, 3.0);
            for _ in 0..(g.random() * 3.0) as u32 {
                set_move(&mut w, 0, 0, 0);
                spawn(&mut w, 0, 0, 0, 0);
            }
            run(&mut w, 310);
            let spawns = count(&w, |k| matches!(k, EvKind::Spawn { .. }));
            assert_eq!(spawns, count(&w, |k| matches!(k, EvKind::Expire { .. })), "case {case}");
            assert!(w.projs.is_empty(), "case {case}");
        }
    }
}
