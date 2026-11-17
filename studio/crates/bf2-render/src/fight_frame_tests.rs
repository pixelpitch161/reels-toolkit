use std::sync::Arc;

use bf2_sim::{Bone, Brain, Choice, FighterDef, HitDef, MoveDef, Rig, Rules2, Setup2, Tag, run};

use super::*;

/// A ball with one arm and one poke that freezes both fighters for 10 steps on contact.
fn fixture() -> FighterDef {
    FighterDef {
        name: "FIXTURE".into(),
        r: 0.6,
        mass: 1.0,
        hp_scale: 1.0,
        rig: Rig::new(vec![Bone {
            name: "arm",
            parent: None,
            length: 0.6,
            radius: 0.15,
        }])
        .unwrap(),
        moves: vec![MoveDef {
            name: "POKE",
            tags: vec![Tag::Light, Tag::Melee],
            windup: 6,
            active: 5,
            recovery: 10,
            keys: vec![],
            hits: vec![HitDef {
                bones: vec![0],
                window: (0, 5),
                damage: 6.0,
                hitstop: 10,
                knock: 4.0,
                lift: 0.0,
                hitstun: 10,
                strength: 0.5,
                multi: false,
                grab: None,
            }],
            spawns: vec![],
            cues: vec![],
            guard: 1.0,
            armor: false,
            lunge: 0.0,
            hop: 0.0,
        }],
        brain: Brain {
            choices: vec![Choice {
                mv: 0,
                range: (0.0, 2.2),
                weight: 1.0,
                cooldown: 20,
                needs_meter: false,
                air: None,
            }],
            speed: 2.6,
            think: 6,
            low: None,
            orbit: 0.2,
        },
        colour: [200, 200, 200],
    }
}

/// A recorded fight whose steps fall halfway between video frames: 120 t_fight has a fractional part of 0.5.
fn recording() -> Recording2 {
    let f = Arc::new(fixture());
    let mut rules = Rules2::new(4.5 + 0.5 / 120.0);
    rules.t_max = 20.0;
    run(
        &Setup2 {
            fighters: [f.clone(), f],
            rules,
            seed: 11,
        },
        true,
    )
    .expect("the fixture fight runs")
}

fn close(a: V3, b: V3) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9)
}

#[test]
fn frames_interpolate_between_steps() {
    let rec = recording();
    let mut checked = 0;
    for f in 0..rec.steps.len() / 4 {
        let fr = frame_at(&rec, f);
        if fr.t_fight <= 0.0 || fr.step + 1 >= rec.steps.len() || rec.steps[fr.step].hitstop != [0, 0] {
            continue;
        }
        assert!((fr.alpha - 0.5).abs() < 1e-6, "alpha {} at frame {f}", fr.alpha);
        let (a, b) = (&rec.steps[fr.step], &rec.steps[fr.step + 1]);
        for k in 0..2 {
            let mid = lerp3(a.pos[k], b.pos[k], 0.5);
            assert!(close(fr.snap.pos[k], mid), "frame {f} ball {k}");
        }
        checked += 1;
    }
    assert!(checked > 100, "only {checked} frames between steps");
}

#[test]
fn frames_hold_during_hitstop() {
    let rec = recording();
    let mut held = 0;
    for f in 0..rec.steps.len() / 4 {
        let fr = frame_at(&rec, f);
        if fr.t_fight > 0.0 && rec.steps[fr.step].hitstop.iter().any(|&h| h > 0) {
            assert_eq!(fr.alpha, 0.0);
            assert_eq!(fr.snap.pos, rec.steps[fr.step].pos);
            assert_eq!(fr.snap.facing, rec.steps[fr.step].facing);
            held += 1;
        }
    }
    assert!(held > 0, "the fixture fight has no hitstop frames");
}

#[test]
fn frames_before_the_fight_hold_step_0() {
    let rec = recording();
    for f in [0, 30, 134] {
        let fr = frame_at(&rec, f);
        assert!(fr.t_fight < 0.0);
        assert_eq!((fr.step, fr.alpha), (0, 0.0));
        assert_eq!(fr.snap.pos, rec.steps[0].pos);
        assert_eq!(fr.snap.hp, rec.steps[0].hp);
    }
}

#[test]
fn frames_past_the_end_hold_last_step() {
    let rec = recording();
    let last = rec.steps.len() - 1;
    for f in [1829, 5000] {
        let fr = frame_at(&rec, f);
        assert_eq!((fr.step, fr.alpha), (last, 0.0));
        assert_eq!(fr.snap.pos, rec.steps[last].pos);
        assert_eq!(fr.snap.joints, rec.steps[last].joints);
    }
}

#[test]
fn facing_turns_the_short_way() {
    let pi = std::f64::consts::PI;
    assert!((lerp_angle(pi - 0.1, -pi + 0.1, 0.5).abs() - pi).abs() < 1e-12);
}
