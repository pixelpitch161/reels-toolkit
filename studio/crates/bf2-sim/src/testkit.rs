//! Fixtures for the unit tests: small fighters and hand-built worlds.

use std::sync::Arc;

use bf_core::rng::Generator;

use crate::body::Body;
use crate::brain::Brain;
use crate::event::{EvKind, Events};
use crate::fighter::FighterDef;
use crate::moves::{HitDef, MoveDef, MoveState, Tag};
use crate::rig::{Bone, Pose, Rig};
use crate::rules::Rules2;
use crate::v3::V3;
use crate::world::{World, solve_all};

/// A hit on bone 0, live for the whole of a 5-step active phase.
pub fn hit(damage: f64) -> HitDef {
    HitDef {
        bones: vec![0],
        window: (0, 5),
        damage,
        hitstop: 0,
        knock: 0.0,
        lift: 0.0,
        hitstun: 0,
        strength: 0.5,
        multi: false,
        grab: None,
    }
}

/// A move with no wind-up, 5 active steps and 5 of recovery.
pub fn poke(hits: Vec<HitDef>) -> MoveDef {
    MoveDef {
        name: "POKE",
        tags: vec![Tag::Light, Tag::Melee],
        windup: 0,
        active: 5,
        recovery: 5,
        keys: vec![],
        hits,
        spawns: vec![],
        cues: vec![],
        guard: 1.0,
        armor: false,
        lunge: 0.0,
        hop: 0.0,
    }
}

pub fn brain() -> Brain {
    Brain {
        choices: vec![],
        speed: 2.6,
        think: 6,
        low: None,
        orbit: 0.0,
    }
}

/// A ball with one bone, `arm` (length 0.6, radius 0.15), and these moves.
pub fn fighter(moves: Vec<MoveDef>) -> FighterDef {
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
        moves,
        brain: brain(),
        colour: [1, 2, 3],
    }
}

/// Two fighters on the canvas at `x0` and `x1` on the x axis, facing each other.
pub fn world(defs: [FighterDef; 2], x0: f64, x1: f64) -> World {
    let rules = Rules2::new(4.5);
    let place = |d: &FighterDef, x: f64, facing: f64| {
        Body::new([x, 0.0, 0.0], facing, d.r, d.mass, rules.hp * d.hp_scale, d.moves.len())
    };
    let bodies = [place(&defs[0], x0, 0.0), place(&defs[1], x1, bf_core::dmath::PI)];
    let masks = [mask(&defs[0]), mask(&defs[1])];
    let poses = [Pose::rest(&defs[0].rig), Pose::rest(&defs[1].rig)];
    let [d0, d1] = defs;
    let mut w = World {
        defs: [Arc::new(d0), Arc::new(d1)],
        bodies,
        poses,
        segments: [Vec::new(), Vec::new()],
        projs: Vec::new(),
        rules,
        rng: Generator::new(7),
        events: Events::default(),
        hit_mask: masks,
        step: 0,
        ko: None,
        next_proj: 0,
    };
    solve_all(&mut w);
    w
}

fn mask(d: &FighterDef) -> Vec<bool> {
    vec![false; d.moves.iter().map(|m| m.hits.len()).max().unwrap_or(0)]
}

/// Puts fighter `who` into move `mv` at `step`, and solves the poses.
pub fn set_move(w: &mut World, who: usize, mv: usize, step: u32) {
    w.bodies[who].mv = Some(MoveState { def: mv, step });
    solve_all(w);
}

/// The events that match.
pub fn count(w: &World, f: impl Fn(&EvKind) -> bool) -> usize {
    w.events.0.iter().filter(|e| f(&e.kind)).count()
}

pub fn close(a: V3, b: V3, tol: f64) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() <= tol)
}
