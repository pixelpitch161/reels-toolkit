//! The world of one fight, and the step.
//!
//! The step order is fixed:
//!
//! 1. hitstop and hitstun countdowns (and move cooldowns);
//! 2. `think` for fighter 0, then 1;
//! 3. advance moves: phase events, cues, lunge and hop at the Active start, projectile spawns;
//! 4. poses and segments (`solve`);
//! 5. grabs count down and held balls follow their holder's grab bone tip;
//! 6. `integrate` 0, then 1;
//! 7. `collide`, then both balls kept inside the ring;
//! 8. `resolve` (clashes, then hits);
//! 9. `step_projs`;
//! 10. the K.O. check;
//! 11. `step += 1`.

use std::sync::Arc;

use bf_core::dmath;
use bf_core::rng::Generator;

use crate::body::Body;
use crate::brain::think;
use crate::event::{EvKind, Events};
use crate::fighter::{FighterDef, Setup2};
use crate::hits::{hold, resolve};
use crate::moves::Phase;
use crate::physics::{collide, confine, integrate};
use crate::proj::{Proj, spawn};
use crate::rig::{Pose, Segment, solve};
use crate::rules::Rules2;

/// The corners the fighters start in, as in today's engine.
const START: [[f64; 2]; 2] = [[-1.6, -1.6], [1.6, 1.6]];
/// Each fighter's opening heading is within this many degrees of the line to the centre.
const LAUNCH_SPREAD_DEG: f64 = 38.0;

/// One fight's whole state.
#[derive(Clone, Debug)]
pub struct World {
    pub defs: [Arc<FighterDef>; 2],
    pub bodies: [Body; 2],
    pub poses: [Pose; 2],
    /// One segment per bone, in rig order.
    pub segments: [Vec<Segment>; 2],
    pub projs: Vec<Proj>,
    pub rules: Rules2,
    pub rng: Generator,
    pub events: Events,
    /// Per fighter, per hit of its current move: that window has landed (one window hits once).
    pub hit_mask: [Vec<bool>; 2],
    pub step: u32,
    /// The loser and the step of the K.O.
    pub ko: Option<(usize, u32)>,
    /// The next projectile id.
    pub next_proj: u32,
}

impl World {
    /// The opening position: fighter 0 at (−1.6, −1.6) and 1 at (1.6, 1.6), on the canvas, facing each other,
    /// each heading off at its brain's speed toward the centre turned by a seeded angle within ±38° (0 first).
    pub fn new(setup: &Setup2) -> World {
        let rules = setup.rules.clone();
        let mut rng = Generator::new(setup.seed);
        let defs = setup.fighters.clone();
        let body = |who: usize, rng: &mut Generator| {
            let d = &defs[who];
            let [x, y] = START[who];
            let [ox, oy] = START[1 - who];
            let facing = dmath::atan2(oy - y, ox - x);
            let mut b = Body::new([x, y, 0.0], facing, d.r, d.mass, rules.hp * d.hp_scale, d.moves.len());
            let heading = dmath::atan2(-y, -x) + rng.uniform(-1.0, 1.0) * LAUNCH_SPREAD_DEG * dmath::PI / 180.0;
            b.vel = [
                dmath::cos(heading) * d.brain.speed,
                dmath::sin(heading) * d.brain.speed,
                0.0,
            ];
            b
        };
        let b0 = body(0, &mut rng);
        let b1 = body(1, &mut rng);
        let mask = |d: &FighterDef| vec![false; d.moves.iter().map(|m| m.hits.len()).max().unwrap_or(0)];
        let mut w = World {
            poses: [Pose::rest(&defs[0].rig), Pose::rest(&defs[1].rig)],
            segments: [
                Vec::with_capacity(defs[0].rig.bones.len()),
                Vec::with_capacity(defs[1].rig.bones.len()),
            ],
            hit_mask: [mask(&defs[0]), mask(&defs[1])],
            bodies: [b0, b1],
            defs,
            projs: Vec::new(),
            rules,
            rng,
            events: Events::default(),
            step: 0,
            ko: None,
            next_proj: 0,
        };
        solve_all(&mut w);
        w
    }
}

/// One step of the fight, in the fixed order in the module's documentation.
pub fn step(world: &mut World) {
    let s = world.step;
    for b in &mut world.bodies {
        b.countdown();
    }
    let had = [world.bodies[0].mv.is_some(), world.bodies[1].mv.is_some()];
    think(world, 0, s);
    think(world, 1, s);
    for (who, had) in had.into_iter().enumerate() {
        advance(world, who, !had, s);
    }
    solve_all(world);
    hold(world, s);
    for who in 0..2 {
        integrate(
            &mut world.bodies[who],
            &world.rules,
            &mut world.rng,
            &mut world.events,
            who,
            s,
        );
    }
    let [a, b] = &mut world.bodies;
    collide(a, b);
    confine(a, &world.rules);
    confine(b, &world.rules);
    resolve(world, s);
    crate::proj::step_projs(world, s);
    if world.ko.is_none()
        && let Some(loser) = (0..2).find(|&i| world.bodies[i].hp <= 0.0)
    {
        world.ko = Some((loser, s));
        world.bodies[loser].mv = None;
    }
    world.step += 1;
}

/// Step stage 3 for one fighter: its move goes one step on (unless it started this step, or it is frozen) and
/// ends when done. Entering the Active or Recovery phase emits its event; the Active start adds the lunge (along
/// the facing) and the hop; cues fire at their phase step, and spawns at their Active step.
fn advance(world: &mut World, who: usize, fresh: bool, step: u32) {
    let Some(mut ms) = world.bodies[who].mv else { return };
    if world.bodies[who].hitstop > 0 {
        return;
    }
    let def = world.defs[who].clone();
    let d = &def.moves[ms.def];
    if !fresh {
        ms.step += 1;
    }
    if ms.done(d) {
        world.bodies[who].mv = None;
        return;
    }
    world.bodies[who].mv = Some(ms);
    let (phase, k) = ms.phase(d);
    if k == 0 {
        match phase {
            Phase::Windup => {}
            Phase::Active => {
                world.events.push(step, who, EvKind::Active { mv: ms.def });
                let b = &mut world.bodies[who];
                b.vel[0] += dmath::cos(b.facing) * d.lunge;
                b.vel[1] += dmath::sin(b.facing) * d.lunge;
                if d.hop > 0.0 {
                    b.vel[2] += d.hop;
                    b.airborne = true;
                }
            }
            Phase::Recovery => world.events.push(step, who, EvKind::Recovery { mv: ms.def }),
        }
    }
    for c in d.cues.iter().filter(|c| c.phase == phase && c.step == k) {
        world.events.push(step, who, EvKind::Cue(c.cue));
    }
    if phase == Phase::Active {
        for (i, _) in d.spawns.iter().enumerate().filter(|(_, sp)| sp.at == k) {
            spawn(world, who, ms.def, i, step);
        }
    }
}

/// Step stage 4: each fighter's pose from its move (rest when it has none), then its segments.
pub(crate) fn solve_all(world: &mut World) {
    for who in 0..2 {
        let def = &world.defs[who];
        let body = &world.bodies[who];
        match body.mv {
            Some(ms) => ms.pose(&def.moves[ms.def], &def.rig, &mut world.poses[who]),
            None => world.poses[who].reset(&def.rig),
        }
        solve(
            &def.rig,
            &world.poses[who],
            body.centre(),
            body.r,
            body.facing,
            &mut world.segments[who],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::Choice;
    use crate::event::EvKind;
    use crate::fighter::Setup2;
    use crate::moves::{Cue, CueAt, MoveDef, Phase, ProjDef, Spawn};
    use crate::testkit::*;

    #[test]
    fn step_order_is_fixed() {
        let dart = ProjDef {
            name: "DART",
            speed: 1.0,
            up: 0.0,
            gravity: false,
            radius: 0.1,
            life: 600,
            hit: hit(1.0),
            bounces: 0,
            homing: 0.0,
        };
        let mv = MoveDef {
            cues: vec![CueAt {
                phase: Phase::Active,
                step: 0,
                cue: Cue::Flash,
            }],
            spawns: vec![Spawn { at: 0, proj: dart }],
            ..poke(vec![hit(50.0)])
        };
        let mut f0 = fighter(vec![mv]);
        f0.brain.low = Some(0);
        let mut w = world([f0, fighter(vec![poke(vec![])])], 0.0, 1.5);
        w.bodies[0].hp = 20.0;
        w.bodies[1].hp = 10.0;
        w.bodies[1].pos[2] = 0.001;
        w.bodies[1].airborne = true;
        step(&mut w);
        let kinds: Vec<_> = w
            .events
            .0
            .iter()
            .map(|e| (e.who, std::mem::discriminant(&e.kind)))
            .collect();
        let want = [
            (0, EvKind::LowHp),
            (0, EvKind::MoveStart { mv: 0 }),
            (0, EvKind::Windup { mv: 0 }),
            (0, EvKind::Ability { mv: 0, first: true }),
            (0, EvKind::Active { mv: 0 }),
            (0, EvKind::Cue(Cue::Flash)),
            (0, EvKind::Spawn { proj: 0 }),
            (1, EvKind::Land),
            (
                0,
                EvKind::Contact {
                    mv: 0,
                    target: 1,
                    point: [0.0; 3],
                    normal: [0.0; 3],
                    strength: 0.0,
                    damage: 0.0,
                    bones: vec![],
                },
            ),
            (0, EvKind::Ko { target: 1 }),
        ];
        let want: Vec<_> = want.iter().map(|(who, k)| (*who, std::mem::discriminant(k))).collect();
        assert_eq!(kinds, want, "{:#?}", w.events.0);
        assert!(w.events.0.iter().all(|e| e.step == 0));
        assert_eq!(w.ko, Some((1, 0)));
        assert_eq!(w.step, 1);
    }

    #[test]
    fn a_ko_stops_moves() {
        let jab = Choice {
            mv: 0,
            range: (0.0, 1.9),
            weight: 1.0,
            cooldown: 0,
            needs_meter: false,
            air: None,
        };
        let mut f = fighter(vec![poke(vec![crate::moves::HitDef {
            knock: 1.0,
            ..hit(30.0)
        }])]);
        f.brain.choices.push(jab);
        let setup = Setup2 {
            fighters: [Arc::new(f.clone()), Arc::new(f)],
            rules: crate::rules::Rules2::new(4.5),
            seed: 3,
        };
        let mut w = World::new(&setup);
        assert_eq!(w.bodies[0].pos, [-1.6, -1.6, 0.0]);
        assert_eq!(w.bodies[1].pos, [1.6, 1.6, 0.0]);
        assert!((w.bodies[0].facing - bf_core::dmath::PI / 4.0).abs() < 1e-12);
        assert!((crate::v3::len(w.bodies[0].vel) - 2.6).abs() < 1e-12);
        while w.ko.is_none() {
            step(&mut w);
            assert!(w.step < 120 * 60, "no K.O. in a minute");
        }
        let (_, at) = w.ko.unwrap();
        for _ in 0..600 {
            step(&mut w);
        }
        assert!(count(&w, |k| matches!(k, EvKind::MoveStart { .. })) > 0);
        assert!(
            !w.events
                .0
                .iter()
                .any(|e| e.step > at && matches!(e.kind, EvKind::MoveStart { .. }))
        );
        assert_eq!(count(&w, |k| matches!(k, EvKind::Ko { .. })), 1);
    }
}
