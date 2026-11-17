//! TEST DUMMY: Engine 2's test fighter (design section 7). Hidden: it is not in `PORTED`, so the app never offers
//! it. It carries one of every kind of move so the later milestones have drama to draw, direct and sound: a jab, a
//! launching haymaker, a grab and slam, a thrown projectile, a meter special with camera and time cues, and a
//! low-HP hop.
//!
//! Its rig is one arm: `arm` (0.5 long, 0.12 thick) from the ball's surface, then `fist` (0.35, 0.16). Every move's
//! pose track pulls back in the wind-up (anticipation), snaps through in the active steps and overshoots in the
//! recovery (follow-through) before it settles at rest. Key times are shares of the whole move; the phase
//! boundaries are noted beside each move.

use bf2_sim::{
    Bone, Brain, Choice, Cue, CueAt, Ease, FighterDef, Grab, HitDef, Joint, Key, MoveDef, Phase, ProjDef, Rig, Spawn,
    Tag,
};

const ARM: usize = 0;
const FIST: usize = 1;

/// The move indices, in `moves` order.
const JAB: usize = 0;
const HAYMAKER: usize = 1;
const GRAB_SLAM: usize = 2;
const TOSS: usize = 3;
const DUMMY_BUSTER: usize = 4;
const SECOND_WIND: usize = 5;

/// A joint: yaw (turn in the floor plane), pitch (up) and length multiplier.
fn j(yaw: f64, pitch: f64, ext: f64) -> Joint {
    Joint { yaw, pitch, ext }
}

fn key(at: f64, ease: Ease, arm: Joint, fist: Joint) -> Key {
    Key {
        at,
        ease,
        joints: vec![(ARM, arm), (FIST, fist)],
    }
}

/// The last key of every move: both joints eased back to rest.
fn settle() -> Key {
    key(1.0, Ease::InOut, Joint::REST, Joint::REST)
}

fn hit(bones: Vec<usize>, window: (u32, u32), damage: f64) -> HitDef {
    HitDef {
        bones,
        window,
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

fn base(name: &'static str, tags: Vec<Tag>, phases: (u32, u32, u32)) -> MoveDef {
    MoveDef {
        name,
        tags,
        windup: phases.0,
        active: phases.1,
        recovery: phases.2,
        keys: vec![],
        hits: vec![],
        spawns: vec![],
        cues: vec![],
        guard: 1.0,
        armor: false,
        lunge: 0.0,
        hop: 0.0,
    }
}

/// 6/3/8 (active from 0.35 to 0.53): a short cock back, a straight punch, a small overshoot.
fn jab() -> MoveDef {
    MoveDef {
        keys: vec![
            key(0.25, Ease::Out, j(0.25, 0.0, 0.55), j(-0.35, 0.0, 0.8)),
            key(0.40, Ease::In, j(0.0, 0.05, 1.35), j(0.0, 0.0, 1.2)),
            key(0.58, Ease::Out, j(-0.15, 0.0, 1.2), j(0.2, 0.0, 1.05)),
            settle(),
        ],
        hits: vec![HitDef {
            hitstop: 4,
            knock: 3.0,
            hitstun: 10,
            strength: 0.25,
            ..hit(vec![FIST], (0, 3), 3.0)
        }],
        lunge: 1.2,
        ..base("JAB", vec![Tag::Light, Tag::Melee], (6, 3, 8))
    }
}

/// 18/4/16 (active from 0.47 to 0.58): a dip, a long coil back and up, a hook that sweeps across the front, and a
/// follow-through that carries the arm high past the target.
fn haymaker() -> MoveDef {
    MoveDef {
        keys: vec![
            key(0.12, Ease::Out, j(-0.2, -0.1, 0.9), j(0.1, 0.0, 1.0)),
            key(0.42, Ease::InOut, j(-1.3, 0.35, 0.6), j(-0.6, 0.0, 0.9)),
            key(0.50, Ease::In, j(0.15, 0.15, 1.3), j(0.25, 0.0, 1.2)),
            key(0.64, Ease::Out, j(0.9, 0.4, 1.15), j(0.4, 0.1, 1.0)),
            settle(),
        ],
        hits: vec![HitDef {
            hitstop: 10,
            knock: 7.0,
            lift: 6.0,
            hitstun: 30,
            strength: 0.7,
            ..hit(vec![ARM, FIST], (0, 4), 9.0)
        }],
        cues: vec![CueAt {
            phase: Phase::Windup,
            step: 0,
            cue: Cue::PunchIn,
        }],
        lunge: 2.5,
        ..base("HAYMAKER", vec![Tag::Heavy, Tag::Melee], (18, 4, 16))
    }
}

/// 10/3/30 (active from 0.23 to 0.30): the arm opens wide, shoots out and closes on the target, hauls it up
/// overhead through the recovery and drives it down to the canvas just as the 30-step hold runs out (about 0.95).
fn grab_slam() -> MoveDef {
    MoveDef {
        keys: vec![
            key(0.18, Ease::Out, j(0.5, 0.15, 0.7), j(-0.7, 0.0, 0.9)),
            key(0.25, Ease::In, j(0.0, 0.05, 1.3), j(0.0, 0.0, 1.1)),
            key(0.40, Ease::Out, j(0.0, 0.5, 0.85), j(0.0, 0.3, 1.0)),
            key(0.78, Ease::InOut, j(0.0, 1.35, 0.9), j(0.0, 0.5, 1.0)),
            key(0.94, Ease::In, j(0.0, -0.45, 1.15), j(0.0, -0.2, 1.0)),
            settle(),
        ],
        hits: vec![HitDef {
            hitstop: 6,
            hitstun: 30,
            strength: 0.8,
            grab: Some(Grab {
                hold: 30,
                bone: FIST,
                slam: 14.0,
                throw: 0.0,
            }),
            ..hit(vec![FIST], (0, 3), 8.0)
        }],
        lunge: 1.5,
        ..base("GRAB SLAM", vec![Tag::Heavy, Tag::Grab], (10, 3, 30))
    }
}

/// 10/2/10 (active from 0.45 to 0.55): the arm swings back and up, whips forward and lets go, then follows through
/// low.
fn toss() -> MoveDef {
    MoveDef {
        keys: vec![
            key(0.38, Ease::InOut, j(-0.9, 0.45, 0.7), j(-0.5, 0.3, 0.9)),
            key(0.47, Ease::In, j(0.3, 0.1, 1.25), j(0.2, 0.0, 1.1)),
            key(0.68, Ease::Out, j(0.6, -0.25, 1.1), j(0.3, -0.2, 1.0)),
            settle(),
        ],
        spawns: vec![Spawn {
            at: 0,
            proj: ProjDef {
                name: "DUMMY BALL",
                speed: 7.0,
                up: 0.0,
                gravity: false,
                radius: 0.18,
                life: 120,
                hit: HitDef {
                    hitstop: 5,
                    knock: 3.5,
                    hitstun: 12,
                    strength: 0.4,
                    ..hit(vec![], (0, 1), 4.0)
                },
                bounces: 1,
                homing: 0.8,
            },
        }],
        ..base("TOSS", vec![Tag::Ranged], (10, 2, 10))
    }
}

/// 30/6/24 (active from 0.50 to 0.60): the meter special. A charge that coils the arm far back and up, then two
/// rapid straight hits (a multi window) and a rising uppercut that launches, and a big follow-through overhead. It
/// punches in at the wind-up, slows time, flashes and shakes on the first strike, and whips back at the recovery.
fn dummy_buster() -> MoveDef {
    let cue = |phase, step, cue| CueAt { phase, step, cue };
    MoveDef {
        keys: vec![
            key(0.15, Ease::Out, j(-0.4, 0.6, 0.7), j(0.2, 0.0, 1.0)),
            key(0.46, Ease::InOut, j(-1.6, 0.9, 0.5), j(-0.4, 0.8, 0.8)),
            key(0.50, Ease::Snap, j(0.0, 0.0, 1.35), j(0.0, 0.0, 1.2)),
            key(0.53, Ease::Out, j(0.2, 0.05, 1.1), j(-0.1, 0.0, 1.1)),
            key(0.57, Ease::In, j(0.0, 0.9, 1.3), j(0.0, 0.6, 1.2)),
            key(0.70, Ease::Out, j(0.0, 1.45, 1.1), j(0.0, 0.7, 1.0)),
            settle(),
        ],
        hits: vec![
            HitDef {
                hitstop: 5,
                knock: 0.6,
                hitstun: 30,
                strength: 0.8,
                multi: true,
                ..hit(vec![ARM, FIST], (0, 2), 5.0)
            },
            HitDef {
                hitstop: 14,
                knock: 5.0,
                lift: 9.0,
                hitstun: 40,
                strength: 1.0,
                ..hit(vec![ARM, FIST], (4, 6), 5.0)
            },
        ],
        cues: vec![
            cue(Phase::Windup, 0, Cue::PunchIn),
            cue(Phase::Active, 0, Cue::SlowMo { scale: 0.3 }),
            cue(Phase::Active, 0, Cue::Flash),
            cue(Phase::Active, 0, Cue::Shake(1.0)),
            cue(Phase::Recovery, 0, Cue::WhipBack),
        ],
        armor: true,
        lunge: 3.0,
        ..base("DUMMY BUSTER", vec![Tag::Special, Tag::Melee], (30, 6, 24))
    }
}

/// 12/1/12 (the hop at 0.48): the low-HP ability. It tucks the arm in, springs up with the arm flung high, and
/// guards (0.3) the whole way.
fn second_wind() -> MoveDef {
    MoveDef {
        keys: vec![
            key(0.40, Ease::InOut, j(0.0, -0.6, 0.55), j(0.0, -0.4, 0.8)),
            key(0.52, Ease::Out, j(0.0, 1.3, 1.2), j(0.0, 0.4, 1.1)),
            key(0.75, Ease::InOut, j(0.0, 0.6, 1.0), j(0.0, 0.2, 1.0)),
            settle(),
        ],
        guard: 0.3,
        hop: 4.0,
        ..base("SECOND WIND", vec![Tag::Special, Tag::Low], (12, 1, 12))
    }
}

fn choice(mv: usize, range: (f64, f64), weight: f64, cooldown: u32) -> Choice {
    Choice {
        mv,
        range,
        weight,
        cooldown,
        needs_meter: false,
        air: None,
    }
}

/// The TEST DUMMY. Tuned (E1, `docs/studio/engine2/e1-measurements.md`) so most seeds K.O. between 40 and 55 s of
/// video: cooldowns pace the moves, it circles half the time it closes in, and `hp_scale` 4 makes a fight many
/// smaller hits, which keeps the K.O. time in a narrow band.
pub fn def() -> FighterDef {
    let rig = Rig::new(vec![
        Bone {
            name: "arm",
            parent: None,
            length: 0.5,
            radius: 0.12,
        },
        Bone {
            name: "fist",
            parent: Some(ARM),
            length: 0.35,
            radius: 0.16,
        },
    ])
    .expect("the dummy's rig is parent-first");
    let moves = vec![jab(), haymaker(), grab_slam(), toss(), dummy_buster(), second_wind()];
    let brain = Brain {
        choices: vec![
            choice(JAB, (1.1, 2.4), 3.0, 140),
            choice(HAYMAKER, (1.1, 2.5), 1.5, 360),
            Choice {
                air: Some(false),
                ..choice(GRAB_SLAM, (1.1, 1.9), 0.8, 540)
            },
            choice(TOSS, (2.6, 9.0), 1.0, 360),
            Choice {
                needs_meter: true,
                ..choice(DUMMY_BUSTER, (1.1, 2.6), 4.0, 0)
            },
        ],
        speed: 2.6,
        think: 6,
        low: Some(SECOND_WIND),
        orbit: 0.5,
    };
    FighterDef {
        name: "TEST DUMMY".into(),
        r: 0.6,
        mass: 1.0,
        hp_scale: 4.0,
        rig,
        moves,
        brain,
        colour: [200, 200, 200],
    }
}
