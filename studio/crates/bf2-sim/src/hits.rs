//! Hit resolution: clashes, then each attacker's live windows against the other ball, then the results (damage,
//! guard, hitstop, knockback and launches, hitstun, grabs, K.O. and meter).

use bf_core::dmath;

use crate::body::Holding;
use crate::event::EvKind;
use crate::moves::{HitDef, MoveDef, MoveState, Phase};
use crate::physics::confine;
use crate::rig::Segment;
use crate::v3::{V3, add, dot, len, lerp, norm, scale, sub};
use crate::world::World;

/// Freeze-on-hit for both fighters after a clash, in steps.
const CLASH_HITSTOP: u32 = 6;
/// Meter is capped here; a special that needs meter spends all of it.
pub const METER_MAX: f64 = 100.0;

/// The point on segment `a`-`b` closest to `p`.
fn closest_on_segment(a: V3, b: V3, p: V3) -> V3 {
    let ab = sub(b, a);
    let l2 = dot(ab, ab);
    if l2 <= 0.0 {
        return a;
    }
    let t = (dot(sub(p, a), ab) / l2).clamp(0.0, 1.0);
    add(a, scale(ab, t))
}

/// The closest points of two segments (Ericson, Real-Time Collision Detection 5.1.9). Parallel segments take
/// the first segment's start.
fn closest_between(s1: &Segment, s2: &Segment) -> (V3, V3) {
    let d1 = sub(s1.b, s1.a);
    let d2 = sub(s2.b, s2.a);
    let r = sub(s1.a, s2.a);
    let (a, e, f) = (dot(d1, d1), dot(d2, d2), dot(d2, r));
    const EPS: f64 = 1e-12;
    let (s, t);
    if a <= EPS && e <= EPS {
        return (s1.a, s2.a);
    }
    if a <= EPS {
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = dot(d1, r);
        if e <= EPS {
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = dot(d1, d2);
            let denom = a * e - b * b;
            let mut s0 = if denom > EPS {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t0 = (b * s0 + f) / e;
            if t0 < 0.0 {
                t0 = 0.0;
                s0 = (-c / a).clamp(0.0, 1.0);
            } else if t0 > 1.0 {
                t0 = 1.0;
                s0 = ((b - c) / a).clamp(0.0, 1.0);
            }
            s = s0;
            t = t0;
        }
    }
    (add(s1.a, scale(d1, s)), add(s2.a, scale(d2, t)))
}

/// A capsule against a sphere: on contact, the point on the sphere's surface nearest the capsule's bone.
pub fn capsule_hits_sphere(s: &Segment, c: V3, r: f64) -> Option<V3> {
    let p = closest_on_segment(s.a, s.b, c);
    let d = sub(p, c);
    let dist = len(d);
    if dist > s.radius + r {
        return None;
    }
    Some(if dist > 0.0 { add(c, scale(d, r / dist)) } else { p })
}

/// The step within the Active phase of the fighter's move, when it has one and is not frozen or held.
fn active_step(world: &World, who: usize) -> Option<(MoveState, u32)> {
    let b = &world.bodies[who];
    let ms = b.mv?;
    if b.hitstop > 0 || b.held_by.is_some() {
        return None;
    }
    match ms.phase(&world.defs[who].moves[ms.def]) {
        (Phase::Active, s) => Some((ms, s)),
        _ => None,
    }
}

fn live(hit: &HitDef, s: u32) -> bool {
    hit.window.0 <= s && s < hit.window.1
}

/// Two active moves whose live segments meet: the midpoint of the closest pair.
fn clash_point(world: &World, m0: (&MoveDef, u32), m1: (&MoveDef, u32)) -> Option<V3> {
    for h0 in m0.0.hits.iter().filter(|h| live(h, m0.1)) {
        for h1 in m1.0.hits.iter().filter(|h| live(h, m1.1)) {
            for &b0 in &h0.bones {
                for &b1 in &h1.bones {
                    let (Some(s0), Some(s1)) = (world.segments[0].get(b0), world.segments[1].get(b1)) else {
                        continue;
                    };
                    let (p, q) = closest_between(s0, s1);
                    if len(sub(q, p)) <= s0.radius + s1.radius {
                        return Some(lerp(p, q, 0.5));
                    }
                }
            }
        }
    }
    None
}

/// Step stage 8, in a fixed order: clashes first (both moves jump to Recovery, both freeze for 6 steps, and
/// nothing else lands this step), then each attacker in index order, each live window of its move against the
/// other ball. A window lands once on its target unless the hit is `multi`. Nothing lands after a K.O.
pub fn resolve(world: &mut World, step: u32) {
    if world.ko.is_some() {
        return;
    }
    let defs = world.defs.clone();
    if let (Some((ms0, s0)), Some((ms1, s1))) = (active_step(world, 0), active_step(world, 1)) {
        let (d0, d1) = (&defs[0].moves[ms0.def], &defs[1].moves[ms1.def]);
        if let Some(point) = clash_point(world, (d0, s0), (d1, s1)) {
            let hs = CLASH_HITSTOP.min(world.rules.max_hitstop);
            for (who, d) in [(0, d0), (1, d1)] {
                let b = &mut world.bodies[who];
                if let Some(ms) = b.mv.as_mut() {
                    ms.step = d.windup + d.active;
                }
                b.hitstop = b.hitstop.max(hs);
            }
            world.events.push(step, 0, EvKind::Clash { other: 1, point });
            world.events.push(step, 0, EvKind::Recovery { mv: ms0.def });
            world.events.push(step, 1, EvKind::Recovery { mv: ms1.def });
            return;
        }
    }
    for (att, def) in defs.iter().enumerate() {
        let target = 1 - att;
        let Some((ms, s)) = active_step(world, att) else {
            continue;
        };
        let mv = &def.moves[ms.def];
        for (h, hit) in mv.hits.iter().enumerate() {
            // A knocked-out fighter lands nothing (armor or a 0 hitstop can leave its move running).
            if world.bodies[att].hp <= 0.0
                || world.bodies[target].hp <= 0.0
                || world.bodies[target].held_by.is_some_and(|by| by != att)
            {
                break;
            }
            if !live(hit, s) || (!hit.multi && world.hit_mask[att].get(h).copied().unwrap_or(false)) {
                continue;
            }
            let c = world.bodies[target].centre();
            let r = world.bodies[target].r;
            let mut first = None;
            for &bone in &hit.bones {
                if let Some(p) = world.segments[att]
                    .get(bone)
                    .and_then(|seg| capsule_hits_sphere(seg, c, r))
                {
                    first.get_or_insert((p, Vec::new())).1.push(bone);
                }
            }
            if let Some((point, bones)) = first {
                if let Some(m) = world.hit_mask[att].get_mut(h) {
                    *m = true;
                }
                let normal = norm(sub(c, point));
                apply_hit(world, att, target, hit, point, normal, &bones, ms.def, step);
            }
        }
    }
}

/// The results of one landed hit, used by melee hits and projectiles alike (projectiles pass no bones and do not
/// freeze their owner). In order: damage (with the target's guard, the handicap and the juggle decay), Contact or
/// Block, hitstop, knockback and launch, hitstun and the cancelled move, a held target dropped if the target was
/// holding it, the grab, K.O. and meter. A block (guard below 1) keeps the target's move, takes no hitstun, lift
/// or grab, and pushes back at `knock × guard`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_hit(
    world: &mut World,
    att: usize,
    target: usize,
    hit: &HitDef,
    point: V3,
    normal: V3,
    bones: &[usize],
    mv: usize,
    step: u32,
) {
    let rules = &world.rules;
    let guard = world.bodies[target]
        .mv
        .map_or(1.0, |ms| world.defs[target].moves[ms.def].guard);
    let blocked = guard < 1.0;
    let decay = dmath::pow(rules.juggle_decay, world.bodies[target].juggle as f64);
    let damage = hit.damage * rules.dmg_scale * rules.dmg_side[att] * guard * decay;
    let hs = hit.hitstop.min(rules.max_hitstop);

    let hp_before = world.bodies[target].hp;
    world.bodies[target].hp -= damage;
    let kind = if blocked {
        EvKind::Block { target, point }
    } else {
        EvKind::Contact {
            mv,
            target,
            point,
            normal,
            strength: hit.strength,
            damage,
            bones: bones.to_vec(),
        }
    };
    world.events.push(step, att, kind);

    if !bones.is_empty() {
        world.bodies[att].hitstop = world.bodies[att].hitstop.max(hs);
    }
    let away = {
        let (a, t) = (world.bodies[att].pos, world.bodies[target].pos);
        let d = norm([t[0] - a[0], t[1] - a[1], 0.0]);
        if d == [0.0; 3] {
            let f = world.bodies[att].facing;
            [dmath::cos(f), dmath::sin(f), 0.0]
        } else {
            d
        }
    };
    let t = &mut world.bodies[target];
    t.hitstop = t.hitstop.max(hs);
    if blocked {
        t.vel[0] = away[0] * hit.knock * guard;
        t.vel[1] = away[1] * hit.knock * guard;
    } else {
        t.vel[0] = away[0] * hit.knock;
        t.vel[1] = away[1] * hit.knock;
        if hit.lift > 0.0 || t.airborne {
            t.juggle += 1;
        }
        if hit.lift > 0.0 {
            t.vel[2] = hit.lift;
            t.airborne = true;
            world.events.push(step, att, EvKind::Launch { target, up: hit.lift });
        }
        let t = &mut world.bodies[target];
        t.hitstun = t.hitstun.max(hit.hitstun);
        let armor = t.mv.is_some_and(|ms| world.defs[target].moves[ms.def].armor);
        if !armor {
            t.mv = None;
        }
        release(world, target, step);
        if let Some(grab) = hit.grab
            && world.bodies[target].held_by.is_none()
            && world.bodies[target].holding.is_none()
            && world.bodies[att].holding.is_none()
            && world.bodies[att].held_by.is_none()
            && world.bodies[target].hp > 0.0
        {
            let t = &mut world.bodies[target];
            t.held_by = Some(att);
            t.vel = [0.0; 3];
            t.mv = None;
            world.bodies[att].holding = Some(Holding {
                target,
                left: grab.hold,
                grab,
            });
            world.events.push(step, att, EvKind::Grabbed { target });
        }
    }

    if hp_before > 0.0 && world.bodies[target].hp <= 0.0 {
        world.events.push(step, att, EvKind::Ko { target });
    }
    let a = &mut world.bodies[att];
    a.meter = (a.meter + damage).min(METER_MAX);
    let t = &mut world.bodies[target];
    t.meter = (t.meter + damage * 0.5).min(METER_MAX);
}

/// Drops whatever `holder` holds: a slam (straight down at `slam`, with Slam) or a throw (along the holder's
/// facing at `throw`), then Released. Does nothing when it holds nothing.
pub(crate) fn release(world: &mut World, holder: usize, step: u32) {
    let Some(h) = world.bodies[holder].holding.take() else {
        return;
    };
    let facing = world.bodies[holder].facing;
    let t = &mut world.bodies[h.target];
    t.held_by = None;
    t.airborne = true;
    if h.grab.slam > 0.0 {
        t.vel = [0.0, 0.0, -h.grab.slam];
        world.events.push(step, holder, EvKind::Slam { target: h.target });
    } else {
        t.vel = [
            dmath::cos(facing) * h.grab.throw,
            dmath::sin(facing) * h.grab.throw,
            0.0,
        ];
    }
    world.events.push(step, holder, EvKind::Released { target: h.target });
}

/// Step stage 5: each grab counts down (not while the holder is frozen) and releases when it runs out or the
/// holder is K.O.; a held ball sits with its centre on the tip of the holder's grab bone (or just in front of the
/// holder if the rig has no such bone), kept inside the ring.
pub(crate) fn hold(world: &mut World, step: u32) {
    for holder in 0..2 {
        let Some(mut h) = world.bodies[holder].holding else {
            continue;
        };
        if world.bodies[holder].hp <= 0.0 || h.left == 0 {
            release(world, holder, step);
            continue;
        }
        if world.bodies[holder].hitstop == 0 {
            h.left -= 1;
            world.bodies[holder].holding = Some(h);
        }
        let tip = match world.segments[holder].get(h.grab.bone) {
            Some(seg) => seg.b,
            None => {
                let b = &world.bodies[holder];
                let reach = b.r + world.bodies[h.target].r;
                add(
                    b.centre(),
                    [dmath::cos(b.facing) * reach, dmath::sin(b.facing) * reach, 0.0],
                )
            }
        };
        let t = &mut world.bodies[h.target];
        t.pos = [tip[0], tip[1], tip[2] - t.r];
        t.vel = [0.0; 3];
        confine(t, &world.rules);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EvKind;
    use crate::moves::Grab;
    use crate::testkit::*;
    use crate::v3::len;

    fn damages(w: &World) -> Vec<f64> {
        w.events
            .0
            .iter()
            .filter_map(|e| match e.kind {
                EvKind::Contact { damage, .. } => Some(damage),
                _ => None,
            })
            .collect()
    }

    /// Fighter 0 pokes fighter 1, who stands 1.5 away (the arm's tip is 0.3 from its centre).
    fn poking(h: HitDef) -> World {
        world([fighter(vec![poke(vec![h])]), fighter(vec![poke(vec![])])], 0.0, 1.5)
    }

    /// Runs fighter 0's active phase, one resolve per step, clearing the freeze in between.
    fn run_active(w: &mut World) {
        for s in 0..5 {
            set_move(w, 0, 0, s);
            resolve(w, s);
            w.bodies[0].hitstop = 0;
            w.bodies[1].hitstop = 0;
        }
    }

    #[test]
    fn capsule_contact_point_is_on_the_sphere() {
        let s = Segment {
            a: [0.0, 0.0, 0.0],
            b: [1.0, 0.0, 0.0],
            radius: 0.2,
        };
        let p = capsule_hits_sphere(&s, [0.5, 0.7, 0.0], 0.6).unwrap();
        assert!((len(sub(p, [0.5, 0.7, 0.0])) - 0.6).abs() < 1e-12);
        assert!(close(p, [0.5, 0.1, 0.0], 1e-12));
        let p = capsule_hits_sphere(&s, [1.5, 0.0, 0.3], 0.6).unwrap();
        assert!((len(sub(p, [1.5, 0.0, 0.3])) - 0.6).abs() < 1e-12);
        assert!(capsule_hits_sphere(&s, [0.5, 0.81, 0.0], 0.6).is_none());
    }

    #[test]
    fn one_window_hits_once() {
        let mut w = poking(hit(5.0));
        run_active(&mut w);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Contact { .. })), 1);
        assert_eq!(w.bodies[1].hp, 95.0);
    }

    #[test]
    fn multi_window_hits_every_step() {
        let mut w = poking(HitDef {
            multi: true,
            ..hit(5.0)
        });
        run_active(&mut w);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Contact { .. })), 5);
        assert_eq!(w.bodies[1].hp, 75.0);
    }

    #[test]
    fn guard_halves_and_blocks() {
        let mut w = world(
            [
                fighter(vec![poke(vec![hit(10.0)])]),
                fighter(vec![MoveDef {
                    guard: 0.5,
                    ..poke(vec![])
                }]),
            ],
            0.0,
            1.5,
        );
        w.bodies[1].mv = Some(MoveState { def: 0, step: 7 });
        run_active(&mut w);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Block { .. })), 1);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Contact { .. })), 0);
        assert_eq!(w.bodies[1].hp, 95.0);
        assert!(w.bodies[1].mv.is_some(), "a block keeps the guarding move");
    }

    #[test]
    fn clash_cancels_both() {
        let mut w = world(
            [fighter(vec![poke(vec![hit(5.0)])]), fighter(vec![poke(vec![hit(5.0)])])],
            0.0,
            2.0,
        );
        w.bodies[1].mv = Some(MoveState { def: 0, step: 1 });
        set_move(&mut w, 0, 0, 1);
        resolve(&mut w, 0);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Clash { .. })), 1);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Contact { .. })), 0);
        for b in &w.bodies {
            assert_eq!(b.mv.unwrap().phase(&w.defs[0].moves[0]), (Phase::Recovery, 0));
            assert_eq!(b.hitstop, 6);
        }
        let point = w.events.0.iter().find_map(|e| match e.kind {
            EvKind::Clash { point, .. } => Some(point),
            _ => None,
        });
        // the arms overlap from x 0.8 to 1.2, and the clash is between them
        let p = point.unwrap();
        assert!(
            p[0] >= 0.8 - 1e-9 && p[0] <= 1.2 + 1e-9 && p[1].abs() < 1e-9 && (p[2] - 0.6).abs() < 1e-9,
            "{p:?}"
        );
    }

    #[test]
    fn launch_sets_airborne_and_juggle_decays() {
        let mut w = poking(HitDef {
            multi: true,
            lift: 6.0,
            ..hit(10.0)
        });
        set_move(&mut w, 0, 0, 0);
        resolve(&mut w, 0);
        assert!(w.bodies[1].airborne);
        assert_eq!(w.bodies[1].juggle, 1);
        assert_eq!(w.bodies[1].vel[2], 6.0);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Launch { up: 6.0, .. })), 1);
        set_move(&mut w, 0, 0, 1);
        resolve(&mut w, 1);
        let d = damages(&w);
        assert_eq!(d.len(), 2);
        assert!((d[1] - d[0] * 0.8).abs() < 1e-12, "{d:?}");
    }

    #[test]
    fn hitstop_always_ends() {
        let mut w = poking(HitDef {
            hitstop: 999,
            multi: true,
            ..hit(0.5)
        });
        let h = HitDef {
            hitstop: 999,
            ..hit(0.5)
        };
        for s in 0..100 {
            apply_hit(&mut w, 0, 1, &h, [1.0, 0.0, 0.6], [1.0, 0.0, 0.0], &[0], 0, s);
            assert!(w.bodies[0].hitstop <= w.rules.max_hitstop && w.bodies[1].hitstop <= w.rules.max_hitstop);
        }
        let mut steps = 0;
        while w.bodies.iter().any(|b| b.hitstop > 0) {
            w.bodies.iter_mut().for_each(|b| b.countdown());
            steps += 1;
            assert!(steps <= w.rules.max_hitstop);
        }
    }

    #[test]
    fn grab_always_releases_even_if_holder_is_hit() {
        let grab = Grab {
            hold: 30,
            bone: 0,
            slam: 14.0,
            throw: 0.0,
        };
        let h = HitDef {
            grab: Some(grab),
            ..hit(8.0)
        };
        // released when the hold runs out
        let mut w = poking(h.clone());
        set_move(&mut w, 0, 0, 0);
        resolve(&mut w, 0);
        assert_eq!(w.bodies[1].held_by, Some(0));
        assert_eq!(count(&w, |k| matches!(k, EvKind::Grabbed { target: 1 })), 1);
        let mut s = 1;
        while w.bodies[1].held_by.is_some() {
            w.bodies.iter_mut().for_each(|b| b.countdown());
            hold(&mut w, s);
            let tip = w.segments[0][0].b;
            if w.bodies[1].held_by.is_some() {
                assert!(close(w.bodies[1].centre(), tip, 1e-12));
            }
            s += 1;
            assert!(s < 100, "never released");
        }
        assert_eq!(count(&w, |k| matches!(k, EvKind::Slam { target: 1 })), 1);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Released { target: 1 })), 1);
        assert_eq!(w.bodies[1].vel, [0.0, 0.0, -14.0]);
        // released at once when the holder is hit
        let mut w = poking(h);
        set_move(&mut w, 0, 0, 0);
        resolve(&mut w, 0);
        assert_eq!(w.bodies[1].held_by, Some(0));
        apply_hit(&mut w, 1, 0, &hit(1.0), [0.6, 0.0, 0.6], [-1.0, 0.0, 0.0], &[0], 0, 1);
        assert_eq!(w.bodies[1].held_by, None);
        assert!(w.bodies[0].holding.is_none());
        assert_eq!(count(&w, |k| matches!(k, EvKind::Released { target: 1 })), 1);
    }

    #[test]
    fn handicap_scales_damage() {
        let mut w = poking(hit(10.0));
        w.rules.dmg_side = [2.0, 0.5];
        apply_hit(&mut w, 0, 1, &hit(10.0), [1.0, 0.0, 0.6], [1.0, 0.0, 0.0], &[0], 0, 0);
        apply_hit(&mut w, 1, 0, &hit(10.0), [0.5, 0.0, 0.6], [-1.0, 0.0, 0.0], &[0], 0, 0);
        assert_eq!(damages(&w), vec![20.0, 5.0]);
        assert_eq!((w.bodies[0].hp, w.bodies[1].hp), (95.0, 80.0));
        // the attacker gains the damage, the target half of it: 0 gets 20 + 2.5, 1 gets 10 + 5
        assert_eq!((w.bodies[0].meter, w.bodies[1].meter), (22.5, 15.0));
    }

    #[test]
    fn ko_fires_once() {
        let mut w = poking(HitDef {
            multi: true,
            ..hit(5.0)
        });
        w.bodies[1].hp = 7.0;
        run_active(&mut w);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Ko { target: 1 })), 1);
        assert!(w.bodies[1].hp <= 0.0);
    }
}
