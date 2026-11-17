use std::sync::Arc;

use bf2_direct::{DirectCfg, TimeMap};
use bf2_sim::{Bone, Brain, Choice, Cue, Ev, EvKind, FighterDef, HitDef, MoveDef, Rig, Rules2, Setup2, Tag, run};

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

/// The map with no slow-mo and no K.O.: video time is fight time plus the fight's start.
fn identity(rec: &Recording2) -> TimeMap {
    TimeMap::build(
        &[],
        rec.steps.len() as u32,
        rec.dt,
        rec.t_fight,
        None,
        &DirectCfg::default(),
    )
}

/// The map of `rec` with these events instead of its own (a slow-mo cue, a K.O.).
fn mapped(rec: &Recording2, events: &[Ev], ko: Option<u32>) -> TimeMap {
    TimeMap::build(
        events,
        rec.steps.len() as u32,
        rec.dt,
        rec.t_fight,
        ko,
        &DirectCfg::default(),
    )
}

fn close(a: V3, b: V3) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9)
}

#[test]
fn frames_interpolate_between_steps() {
    let rec = recording();
    let map = identity(&rec);
    let mut checked = 0;
    for f in 0..rec.steps.len() / 4 {
        let fr = frame_at(&rec, &map, f);
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
    let map = identity(&rec);
    let mut held = 0;
    for f in 0..rec.steps.len() / 4 {
        let fr = frame_at(&rec, &map, f);
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
    let map = identity(&rec);
    for f in [0, 30, 134] {
        let fr = frame_at(&rec, &map, f);
        assert!(fr.t_fight < 0.0);
        assert_eq!((fr.step, fr.alpha), (0, 0.0));
        assert_eq!(fr.snap.pos, rec.steps[0].pos);
        assert_eq!(fr.snap.hp, rec.steps[0].hp);
    }
}

#[test]
fn frames_past_the_end_hold_last_step() {
    let rec = recording();
    let map = identity(&rec);
    let last = rec.steps.len() - 1;
    for f in [1829, 5000] {
        let fr = frame_at(&rec, &map, f);
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

#[test]
fn slowmo_frames_advance_slowly() {
    let rec = recording();
    // A 0.25x slow-mo cued at step 1200 (10 fight-s), its move recovering 0.5 fight-s later.
    let events = [
        Ev {
            step: 1200,
            who: 0,
            kind: EvKind::Cue(Cue::SlowMo { scale: 0.25 }),
        },
        Ev {
            step: 1260,
            who: 0,
            kind: EvKind::Recovery { mv: 0 },
        },
    ];
    let map = mapped(&rec, &events, None);
    let plain = identity(&rec);
    let v = map.video_t(10.2);
    let f = (v * FPS).round() as usize;
    let (a, b) = (frame_at(&rec, &map, f), frame_at(&rec, &map, f + 1));
    let gap = b.t_fight - a.t_fight;
    assert!((gap - 0.25 / FPS).abs() < 1e-9, "fight time between frames {gap}");
    assert!((a.t_fight - map.fight_t(f as f64 / FPS)).abs() < 1e-12);
    let (c, d) = (frame_at(&rec, &plain, f), frame_at(&rec, &plain, f + 1));
    assert!((d.t_fight - c.t_fight - 1.0 / FPS).abs() < 1e-9);
    // The sub-frame times go through the map too.
    let sub = frame_at_time(&rec, &map, (f as f64 + 0.5) / FPS);
    assert!((sub.t_fight - a.t_fight - 0.125 / FPS).abs() < 1e-9);
    assert!(sub.step as f64 * rec.dt <= sub.t_fight + 1e-12);
}

#[test]
fn freeze_frames_are_identical_in_fight_time() {
    let rec = recording();
    let ko = 1500;
    let map = mapped(&rec, &[], Some(ko));
    let v = map.ko_video_t().expect("a K.O.");
    let first = (v * FPS).ceil() as usize;
    // 0.35 s of freeze: ten frames show the K.O. step.
    let frames: Vec<_> = (first..first + 10).map(|f| frame_at(&rec, &map, f)).collect();
    for fr in &frames {
        assert_eq!(fr.t_fight, frames[0].t_fight, "at {} s", fr.t_video);
        assert_eq!((fr.step, fr.alpha), (frames[0].step, frames[0].alpha));
        assert_eq!(fr.snap, frames[0].snap);
    }
    assert!((frames[0].t_fight - f64::from(ko) * rec.dt).abs() < 1e-9);
    assert!(frame_at(&rec, &map, first + 12).t_fight > frames[0].t_fight);
}
