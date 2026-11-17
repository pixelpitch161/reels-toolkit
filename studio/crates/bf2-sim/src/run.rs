//! Running a whole fight.

use crate::event::EvKind;
use crate::fighter::Setup2;
use crate::proj::expire_all;
pub use crate::record::{Outcome, Recording2, StepSnap};
use crate::rules::SimError2;
use crate::world::{World, step};

/// Runs one fight: checks the rules and both fighters, then steps until a K.O. plus one second of steps (so the
/// K.O. flight is recorded), or until the last step at or before `t_max` (step `n` is at video time
/// `t_fight + n·dt`). A fight with no K.O. goes to the higher HP by decision; equal HP is a draw. Projectiles still
/// flying at the end expire then. With `record` false, `steps` stays empty but the events are kept.
pub fn run(setup: &Setup2, record: bool) -> Result<Recording2, SimError2> {
    setup.rules.check()?;
    for (i, f) in setup.fighters.iter().enumerate() {
        f.check()
            .map_err(|e| SimError2::new(format!("fighters[{i}].{}", e.field), e.message))?;
    }
    let mut w = World::new(setup);
    let rules = w.rules.clone();
    let tail = (1.0 / rules.dt).round() as u32;
    let hp_start = [w.bodies[0].hp, w.bodies[1].hp];
    let mut steps = Vec::new();
    if record {
        steps.reserve(((rules.t_max - rules.t_fight) / rules.dt) as usize + 2);
    }
    loop {
        if rules.t_fight + w.step as f64 * rules.dt > rules.t_max {
            break;
        }
        if let Some((_, k)) = w.ko
            && w.step > k + tail
        {
            break;
        }
        step(&mut w);
        if record {
            steps.push(StepSnap::of(&w));
        }
    }
    let last = w.step.saturating_sub(1);
    expire_all(&mut w, last);

    let hp_end = [w.bodies[0].hp, w.bodies[1].hp];
    let mut hits = [0u32; 2];
    for e in &w.events.0 {
        if matches!(e.kind, EvKind::Contact { .. } | EvKind::Block { .. }) {
            hits[e.who] += 1;
        }
    }
    let (winner, t_ko) = match w.ko {
        Some((loser, k)) => (Some(1 - loser), Some(rules.t_fight + k as f64 * rules.dt)),
        None if hp_end[0] > hp_end[1] => (Some(0), None),
        None if hp_end[1] > hp_end[0] => (Some(1), None),
        None => (None, None),
    };
    let outcome = Outcome {
        winner,
        t_ko,
        hp_end,
        hits,
        damage: [hp_start[1] - hp_end[1], hp_start[0] - hp_end[0]],
        by_decision: w.ko.is_none(),
    };
    Ok(Recording2 {
        names: [setup.fighters[0].name.clone(), setup.fighters[1].name.clone()],
        seed: setup.seed,
        dt: rules.dt,
        t_fight: rules.t_fight,
        steps,
        events: std::mem::take(&mut w.events.0),
        outcome,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::brain::Choice;
    use crate::event::EvKind;
    use crate::fighter::{FighterDef, Setup2};
    use crate::moves::{Ease, HitDef, Key};
    use crate::rig::Joint;
    use crate::rules::Rules2;
    use crate::testkit::*;

    /// One bone, one light move with a pulled-back wind-up and a knock.
    fn jabber(damage: f64) -> FighterDef {
        let pull = Joint {
            yaw: 0.0,
            pitch: 0.0,
            ext: 0.4,
        };
        let mut mv = poke(vec![HitDef {
            knock: 3.0,
            hitstop: 4,
            hitstun: 6,
            ..hit(damage)
        }]);
        mv.windup = 6;
        mv.keys = vec![
            Key {
                at: 0.3,
                ease: Ease::Out,
                joints: vec![(0, pull)],
            },
            Key {
                at: 0.45,
                ease: Ease::Snap,
                joints: vec![(0, Joint { ext: 1.3, ..pull })],
            },
            Key {
                at: 1.0,
                ease: Ease::InOut,
                joints: vec![(0, Joint::REST)],
            },
        ];
        let mut f = fighter(vec![mv]);
        f.brain.orbit = 0.3;
        f.brain.choices.push(Choice {
            mv: 0,
            range: (0.0, 1.9),
            weight: 1.0,
            cooldown: 30,
            needs_meter: false,
            air: None,
        });
        f
    }

    fn setup(damage: f64, seed: u64) -> Setup2 {
        let f = Arc::new(jabber(damage));
        Setup2 {
            fighters: [f.clone(), f],
            rules: Rules2::new(4.5),
            seed,
        }
    }

    #[test]
    fn same_seed_same_recording() {
        let a = run(&setup(4.0, 11), true).unwrap();
        let b = run(&setup(4.0, 11), true).unwrap();
        assert!(a == b);
        assert!(count_contacts(&a) > 3);
        let c = run(&setup(4.0, 11), false).unwrap();
        let d = run(&setup(4.0, 11), false).unwrap();
        assert!(c == d);
        assert!(c.outcome == a.outcome);
        assert_eq!(c.events, a.events);
        // and on other threads
        let e = std::thread::spawn(|| run(&setup(4.0, 11), true).unwrap())
            .join()
            .unwrap();
        assert!(e == a);
    }

    fn count_contacts(r: &Recording2) -> usize {
        r.events
            .iter()
            .filter(|e| matches!(e.kind, EvKind::Contact { .. }))
            .count()
    }

    #[test]
    fn different_seeds_differ() {
        let a = run(&setup(4.0, 1), true).unwrap();
        let b = run(&setup(4.0, 2), true).unwrap();
        assert!(a != b);
        assert_ne!(a.events, b.events);
    }

    #[test]
    fn no_ko_ends_at_t_max() {
        let r = run(&setup(0.0, 3), true).unwrap();
        let o = &r.outcome;
        assert!(o.by_decision);
        assert_eq!((o.winner, o.t_ko), (None, None));
        assert_eq!(o.hp_end, [100.0, 100.0]);
        let n = r.steps.len() as f64;
        assert!(r.t_fight + (n - 1.0) * r.dt <= 58.0, "a step past t_max");
        assert!(r.t_fight + n * r.dt > 58.0, "stopped early");
        assert!(r.events.iter().all(|e| ((e.step as f64) * r.dt + r.t_fight) <= 58.0));
        // the higher HP wins a decision
        let mut s = setup(0.0, 3);
        let mut weak = jabber(0.0);
        weak.hp_scale = 0.5;
        s.fighters[1] = Arc::new(weak);
        let r = run(&s, false).unwrap();
        assert_eq!((r.outcome.winner, r.outcome.by_decision), (Some(0), true));
    }

    #[test]
    fn record_off_has_no_steps() {
        let r = run(&setup(4.0, 5), false).unwrap();
        assert!(r.steps.is_empty());
        assert!(!r.events.is_empty());
        assert_eq!((r.names[0].as_str(), r.seed, r.t_fight), ("FIXTURE", 5, 4.5));
    }

    #[test]
    fn steps_cover_the_fight() {
        let r = run(&setup(25.0, 9), true).unwrap();
        let o = &r.outcome;
        let t_ko = o.t_ko.expect("a K.O. at 25 a hit");
        assert!(!o.by_decision);
        let loser = 1 - o.winner.unwrap();
        assert!(o.hp_end[loser] <= 0.0);
        let k = ((t_ko - r.t_fight) / r.dt).round() as usize;
        assert_eq!(r.steps.len(), k + 1 + 120, "the K.O. step plus a second of flight");
        let last = r.events.iter().map(|e| e.step).max().unwrap() as usize;
        assert!(last < r.steps.len());
        assert_eq!(
            o.hits[1 - loser] as usize,
            r.events
                .iter()
                .filter(|e| e.who == 1 - loser && matches!(e.kind, EvKind::Contact { .. } | EvKind::Block { .. }))
                .count()
        );
        assert!((o.damage[1 - loser] - (100.0 - o.hp_end[loser])).abs() < 1e-9);
        let snap = &r.steps[k];
        assert_eq!(snap.hp, o.hp_end, "no damage after the K.O.");
        assert_eq!(snap.joints[0].len(), 1);
        let spawns = r
            .events
            .iter()
            .filter(|e| matches!(e.kind, EvKind::Spawn { .. }))
            .count();
        assert_eq!(
            spawns,
            r.events
                .iter()
                .filter(|e| matches!(e.kind, EvKind::Expire { .. }))
                .count()
        );
    }

    #[test]
    fn bad_setups_are_refused() {
        let mut s = setup(4.0, 1);
        s.rules.dt = 0.0;
        assert_eq!(run(&s, false).unwrap_err().field, "rules.dt");
        let mut s = setup(4.0, 1);
        let mut f = jabber(4.0);
        f.brain.low = Some(9);
        s.fighters[1] = Arc::new(f);
        assert_eq!(run(&s, false).unwrap_err().field, "fighters[1].brain.low");
    }
}
