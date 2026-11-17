//! Behaviour: seeded move choice and movement.

use bf_core::dmath;

use crate::event::EvKind;
use crate::hits::METER_MAX;
use crate::moves::{MoveState, Phase, Tag};
use crate::proj::angle_diff;
use crate::v3::{len, sub};
use crate::world::World;

/// The fastest a free fighter turns to face its opponent, rad/s.
const TURN_RATE: f64 = 12.0;
/// Seeded heading jitter when moving, degrees either way.
const JITTER_DEG: f64 = 10.0;

/// A move the brain may pick, and when.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub mv: usize,
    /// Centre distance, from and to.
    pub range: (f64, f64),
    pub weight: f64,
    /// Steps after it starts before it can start again.
    pub cooldown: u32,
    /// Needs a full meter (100), which it spends.
    pub needs_meter: bool,
    /// `Some(true)` only when airborne, `Some(false)` only on the canvas.
    pub air: Option<bool>,
}

/// A behaviour profile.
#[derive(Clone, Debug, PartialEq)]
pub struct Brain {
    pub choices: Vec<Choice>,
    /// Move speed (2.6 like today).
    pub speed: f64,
    /// Steps between decisions (6).
    pub think: u32,
    /// The move fired once when HP drops below `low_hp`.
    pub low: Option<usize>,
    /// 0 to 1: how much it circles instead of approaching.
    pub orbit: f64,
}

/// Starts move `mv`: MoveStart and Windup, a fresh hit mask, and with `ability` an Ability event (first on the
/// move's first use).
pub(crate) fn start_move(world: &mut World, who: usize, mv: usize, step: u32, ability: bool) {
    let b = &mut world.bodies[who];
    b.mv = Some(MoveState { def: mv, step: 0 });
    let first = !b.used.get(mv).copied().unwrap_or(false);
    if let Some(u) = b.used.get_mut(mv) {
        *u = true;
    }
    world.hit_mask[who].fill(false);
    world.events.push(step, who, EvKind::MoveStart { mv });
    world.events.push(step, who, EvKind::Windup { mv });
    if ability {
        world.events.push(step, who, EvKind::Ability { mv, first });
    }
}

/// Step stage 2 for one fighter. Every step it is not frozen, held or in an Active phase, its facing turns toward
/// the opponent at up to 12 rad/s. On its thinking steps (`step % think == who·3 % think`), when free (no move, no
/// hitstun, no hitstop, not held or holding, no K.O.), it fires the low-HP move if due (LowHp, then the start with
/// Ability), or else picks among the choices in range, off cooldown, with meter when needed and matching its air
/// state, by weight with `rng.random()`. With nothing to pick, it moves: toward the opponent mixed with the
/// tangent by `orbit`, turned by a seeded jitter of up to 10°, at `speed` when on the canvas.
pub fn think(world: &mut World, who: usize, step: u32) {
    if world.ko.is_some() {
        return;
    }
    let def = world.defs[who].clone();
    let opp = world.bodies[1 - who].centre();
    let rules = &world.rules;
    let b = &mut world.bodies[who];
    let active = b.mv.is_some_and(|ms| ms.phase(&def.moves[ms.def]).0 == Phase::Active);
    if b.hitstop > 0 || b.held_by.is_some() || active {
        return;
    }
    let to = sub(opp, b.centre());
    if to[0] != 0.0 || to[1] != 0.0 {
        let want = dmath::atan2(to[1], to[0]);
        let limit = TURN_RATE * rules.dt;
        b.facing = angle_diff(b.facing + angle_diff(want, b.facing).clamp(-limit, limit), 0.0);
    }
    let brain = &def.brain;
    let every = brain.think.max(1);
    if b.mv.is_some() || b.hitstun > 0 || b.holding.is_some() || step % every != (who as u32 * 3) % every {
        return;
    }
    if let Some(low) = brain.low
        && !b.low_done
        && b.hp > 0.0
        && b.hp < rules.low_hp * def.hp_scale
    {
        b.low_done = true;
        world.events.push(step, who, EvKind::LowHp);
        start_move(world, who, low, step, true);
        return;
    }
    let dist = len(to);
    let fits = |c: &Choice| {
        c.weight > 0.0
            && c.range.0 <= dist
            && dist <= c.range.1
            && b.cooldowns.get(c.mv).is_some_and(|&cd| cd == 0)
            && (!c.needs_meter || b.meter >= METER_MAX)
            && c.air.is_none_or(|air| air == b.airborne)
    };
    let total: f64 = brain.choices.iter().filter(|c| fits(c)).map(|c| c.weight).sum();
    if total > 0.0 {
        let r = world.rng.random() * total;
        let mut acc = 0.0;
        let mut pick = None;
        for c in brain.choices.iter().filter(|c| fits(c)) {
            acc += c.weight;
            pick = Some(c);
            if r < acc {
                break;
            }
        }
        let c = pick.expect("a choice fits");
        let b = &mut world.bodies[who];
        b.cooldowns[c.mv] = c.cooldown;
        if c.needs_meter {
            b.meter -= METER_MAX;
        }
        let ability = def.moves[c.mv].has(Tag::Special);
        start_move(world, who, c.mv, step, ability);
        return;
    }
    if b.airborne {
        return;
    }
    let flat = dmath::hypot(to[0], to[1]);
    let (dx, dy) = if flat > 0.0 {
        (to[0] / flat, to[1] / flat)
    } else {
        (dmath::cos(b.facing), dmath::sin(b.facing))
    };
    let (hx, hy) = (
        dx * (1.0 - brain.orbit) - dy * brain.orbit,
        dy * (1.0 - brain.orbit) + dx * brain.orbit,
    );
    let jitter = world.rng.uniform(-JITTER_DEG, JITTER_DEG) * dmath::PI / 180.0;
    let heading = dmath::atan2(hy, hx) + jitter;
    let b = &mut world.bodies[who];
    b.vel[0] = dmath::cos(heading) * brain.speed;
    b.vel[1] = dmath::sin(heading) * brain.speed;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EvKind;
    use crate::fighter::FighterDef;
    use crate::moves::{MoveDef, Tag};
    use crate::physics::integrate;
    use crate::testkit::*;
    use crate::v3::{len, sub};
    use crate::world::World;

    fn choice(mv: usize, weight: f64) -> Choice {
        Choice {
            mv,
            range: (0.0, 100.0),
            weight,
            cooldown: 0,
            needs_meter: false,
            air: None,
        }
    }

    fn special() -> MoveDef {
        MoveDef {
            name: "SPECIAL",
            tags: vec![Tag::Special],
            ..poke(vec![])
        }
    }

    fn with_brain(choices: Vec<Choice>, low: Option<usize>) -> FighterDef {
        let mut f = fighter(vec![poke(vec![]), special(), poke(vec![])]);
        f.brain = Brain {
            choices,
            low,
            ..brain()
        };
        f
    }

    /// Countdowns, think, a bare move advance and motion: enough of a step to watch the brain.
    fn tick(w: &mut World) {
        let s = w.step;
        w.bodies.iter_mut().for_each(|b| b.countdown());
        let had = [w.bodies[0].mv.is_some(), w.bodies[1].mv.is_some()];
        think(w, 0, s);
        think(w, 1, s);
        for (who, had) in had.into_iter().enumerate() {
            if let (true, Some(ms)) = (had, w.bodies[who].mv.as_mut()) {
                ms.step += 1;
                if ms.done(&w.defs[who].moves[ms.def]) {
                    w.bodies[who].mv = None;
                }
            }
            integrate(&mut w.bodies[who], &w.rules, &mut w.rng, &mut w.events, who, s);
        }
        w.step += 1;
    }

    fn starts(w: &World, who: usize) -> Vec<(u32, usize)> {
        w.events
            .0
            .iter()
            .filter_map(|e| match e.kind {
                EvKind::MoveStart { mv } if e.who == who => Some((e.step, mv)),
                _ => None,
            })
            .collect()
    }

    fn fight(defs: [FighterDef; 2], seed: u64, steps: u32) -> World {
        let mut w = world(defs, -1.0, 1.0);
        w.rng = bf_core::rng::Generator::new(seed);
        for _ in 0..steps {
            tick(&mut w);
        }
        w
    }

    #[test]
    fn same_seed_same_choices() {
        let defs = || {
            [
                with_brain(vec![choice(0, 1.0), choice(2, 2.0)], None),
                with_brain(vec![choice(0, 1.0)], None),
            ]
        };
        let (a, b, c) = (fight(defs(), 5, 900), fight(defs(), 5, 900), fight(defs(), 6, 900));
        assert!(starts(&a, 0).len() > 20);
        assert_eq!(starts(&a, 0), starts(&b, 0));
        assert_eq!(starts(&a, 1), starts(&b, 1));
        assert_ne!(starts(&a, 0), starts(&c, 0));
        // the two fighters think on different steps
        assert!(starts(&a, 0).iter().all(|&(s, _)| s % 6 == 0));
        assert!(starts(&a, 1).iter().all(|&(s, _)| s % 6 == 3));
    }

    #[test]
    fn cooldown_respected() {
        let defs = [
            with_brain(
                vec![Choice {
                    cooldown: 50,
                    ..choice(0, 1.0)
                }],
                None,
            ),
            with_brain(vec![], None),
        ];
        let w = fight(defs, 1, 900);
        let s = starts(&w, 0);
        assert!(s.len() > 5);
        assert!(s.windows(2).all(|p| p[1].0 - p[0].0 >= 50), "{s:?}");
    }

    #[test]
    fn meter_gates_specials() {
        let defs = || {
            [
                with_brain(
                    vec![Choice {
                        needs_meter: true,
                        ..choice(1, 1.0)
                    }],
                    None,
                ),
                with_brain(vec![], None),
            ]
        };
        let w = fight(defs(), 1, 300);
        assert!(starts(&w, 0).is_empty());
        let mut w = world(defs(), -1.0, 1.0);
        w.bodies[0].meter = 100.0;
        for _ in 0..12 {
            tick(&mut w);
        }
        assert_eq!(starts(&w, 0), vec![(0, 1)]);
        assert_eq!(w.bodies[0].meter, 0.0);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Ability { mv: 1, first: true })), 1);
    }

    #[test]
    fn low_fires_once() {
        let defs = [with_brain(vec![choice(0, 1.0)], Some(2)), with_brain(vec![], None)];
        let mut w = world(defs, -1.0, 1.0);
        w.bodies[0].hp = 20.0;
        for _ in 0..600 {
            tick(&mut w);
        }
        assert_eq!(count(&w, |k| matches!(k, EvKind::LowHp)), 1);
        assert_eq!(count(&w, |k| matches!(k, EvKind::Ability { mv: 2, first: true })), 1);
        assert_eq!(starts(&w, 0)[0], (0, 2), "the low move comes first");
        assert!(starts(&w, 0).iter().skip(1).all(|&(_, mv)| mv == 0));
        let i = w.events.0.iter().position(|e| e.kind == EvKind::LowHp).unwrap();
        assert_eq!(w.events.0[i + 1].kind, EvKind::MoveStart { mv: 2 });
    }

    #[test]
    fn out_of_range_moves_toward_opponent() {
        let far = Choice {
            range: (0.0, 0.5),
            ..choice(0, 1.0)
        };
        let defs = [with_brain(vec![far.clone()], None), with_brain(vec![far], None)];
        let mut w = world(defs, -2.0, 2.0);
        let d0 = len(sub(w.bodies[1].pos, w.bodies[0].pos));
        for _ in 0..60 {
            tick(&mut w);
        }
        let d1 = len(sub(w.bodies[1].pos, w.bodies[0].pos));
        assert!(d1 < d0 - 1.0, "{d0} -> {d1}");
        assert!(starts(&w, 0).is_empty());
        // facing turns toward the opponent
        let b = &w.bodies[0];
        let to = sub(w.bodies[1].pos, b.pos);
        let want = bf_core::dmath::atan2(to[1], to[0]);
        assert!(crate::proj::angle_diff(want, b.facing).abs() < 0.2);
    }
}
