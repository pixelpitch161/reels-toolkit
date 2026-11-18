//! TRUMPET on Engine 2: the move set Aedan picked one slot at a time (2026-11-17/18), drawn in the current
//! engine's own style (see `look.rs`).
//!
//! - VALVE COMBO+ (standard): the pearl buttons sink (wind-up), pistons 1 and 2 jab on the eighth notes, then all
//!   three slam out together as one chord, the heavy hit. The pistons stop on the foe's glass instead of passing
//!   through it (Aedan, 2026-11-18), which the look draws.
//! - HERALD LINE (special 1, FANFARE's slot): six banner poles rise in a ring round the foe and stand as a fence for
//!   about 3 s; every bounce off a pole nicks the foe and pushes it back inside.
//! - DOUBLE TONGUE+ (special 2): a short call, then 4 s of haste (`Tag::Haste`): the combo comes round twice as often
//!   and each jab is tongued twice, "ta-ka" (the hasted combo, `Tag::Hasted`).
//! - PISTON ROCKETS (low HP, HIGH C's slot): the pistons rattle loose, then launch one after another as little steam
//!   rockets that fan out and curve in on the foe.
//!
//! The rig: the three pistons (`P1..P3`), each a root bone 0.02 long at rest (inside the valve casing) that the moves
//! stretch out with `ext`; `HORN`, a root bone never drawn or hit, whose joint tilts the sprite (pitch rocks it,
//! yaw turns it); and `LAUNCH`, the last bone (never drawn or hit), at whose tip the rockets set off.

pub mod card;
pub mod look;
pub mod sound;

use bf_core::dmath::{exp, sin};

fn cube(x: f64) -> f64 {
    x * x * x
}

use bf2_sim::{
    Bone, Brain, Choice, Cue, CueAt, Ease, FighterDef, HitDef, Joint, Key, MoveDef, Phase, ProjDef, Rig, Spawn, Tag,
};

pub const P1: usize = 0;
pub const P2: usize = 1;
pub const P3: usize = 2;
pub const HORN: usize = 3;
/// The rockets' launch point (never drawn or hit): last, so projectiles set off from its tip.
pub const LAUNCH: usize = 4;
pub const BONES: usize = 5;

/// A bone's rest length: tiny, so the drawn parts sit inside the casing until a move stretches them.
pub const STUB: f64 = 0.02;

pub const VALVE_COMBO: usize = 0;
/// VALVE COMBO+ while DOUBLE TONGUE+'s haste lasts.
pub const VALVE_COMBO_TK: usize = 1;
pub const HERALD_LINE: usize = 2;
pub const DOUBLE_TONGUE: usize = 3;
pub const PISTON_ROCKETS: usize = 4;

/// The yellow of the current engine's TRUMPET (roster `color`).
pub const COLOUR: [u8; 3] = [255, 214, 60];

/// VALVE COMBO+'s jab and chord reach, metres out of the glass.
pub const JAB: f64 = 1.5;
pub const CHORD: f64 = 1.7;

fn j(yaw: f64, pitch: f64, ext: f64) -> Joint {
    Joint { yaw, pitch, ext }
}

/// `ext` for a part that reaches `m` metres out of the glass.
fn reach(m: f64) -> f64 {
    (m / STUB).max(1.0)
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
        strength: 0.2,
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

/// Dense keys: `pose(step)` sampled at every step of the move (every bone listed in every key, so nothing holds a
/// stale value between keys), linear in between.
fn sampled(total: u32, pose: impl Fn(f64) -> [Joint; BONES]) -> Vec<Key> {
    (1..=total)
        .map(|s| {
            let p = pose(f64::from(s));
            Key {
                at: f64::from(s) / f64::from(total),
                ease: Ease::Linear,
                joints: p.iter().enumerate().map(|(b, &jj)| (b, jj)).collect(),
            }
        })
        .collect()
}

fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// One piston's stroke: out in `t_out` steps (fast ease-out), hold `t_hold`, back in `t_back` with a little spring
/// bounce; `u` is steps since it fired. Gives metres out of the glass.
fn stroke(u: f64, len: f64, t_out: f64, t_hold: f64, t_back: f64) -> f64 {
    if u < 0.0 {
        0.0
    } else if u < t_out {
        len * (1.0 - cube(1.0 - u / t_out))
    } else if u < t_out + t_hold {
        len
    } else if u < t_out + t_hold + t_back {
        let x = (u - t_out - t_hold) / t_back;
        // home, with a small rebound out and back at the end
        let home = 1.0 - smooth(x);
        let bounce = sin(x * std::f64::consts::PI * 2.0).max(0.0) * 0.06 * (1.0 - x);
        len * (home + bounce)
    } else {
        0.0
    }
}

fn cue(phase: Phase, step: u32, cue: Cue) -> CueAt {
    CueAt { phase, step, cue }
}

fn proj(name: &'static str, hit: HitDef) -> ProjDef {
    ProjDef {
        name,
        speed: 0.0,
        up: 0.0,
        gravity: false,
        radius: 0.12,
        life: 120,
        hit,
        bounces: 0,
        homing: 0.0,
        place: None,
        rehit: 0,
        aim: 0.0,
        level: false,
    }
}

/// The horn's rock: back through the wind-up (anticipation), a kick forward on each shot at `shots` (active step,
/// size), settling after.
fn rock(s: f64, wf: f64, shots: &[(f64, f64)]) -> f64 {
    let act = s - wf;
    if act < 0.0 {
        return -0.18 * smooth(s / wf);
    }
    let kick = |at: f64, amt: f64| {
        let u = act - at;
        if u < 0.0 { 0.0 } else { amt * exp(-u / 4.0) }
    };
    -0.18 * (1.0 - smooth(act / 4.0)) + shots.iter().map(|&(at, amt)| kick(at, amt)).sum::<f64>()
}

/// The buttons sinking through a wind-up: each piston pokes a hair out of its casing, one after another, as its
/// button is pressed (the look draws a short rod and the pearl button).
fn press(s: f64, wf: f64, i: usize) -> f64 {
    let x = (s / wf - 0.15 * i as f64).clamp(0.0, 1.0);
    0.12 * smooth(x / 0.5) * (1.0 - smooth((x - 0.8) / 0.2))
}

/// VALVE COMBO+, 18/18/20. Wind-up: the buttons sink one by one and the horn rocks back. Active: piston 1 jabs at
/// step 2, piston 2 at step 8 (light, 1.5 m), then all three slam out together at step 14 as one chord (the heavy,
/// 1.7 m, spread wider so the three read as three). Recovery: they spring home one by one and the horn settles.
fn valve_combo() -> MoveDef {
    let (w, a, r) = (18u32, 18u32, 20u32);
    let total = w + a + r;
    let wf = f64::from(w);
    let keys = sampled(total, |s| {
        let act = s - wf;
        let p1 = stroke(act - 2.0, JAB, 2.0, 2.0, 5.0).max(stroke(act - 14.0, CHORD, 2.0, 3.0, 12.0));
        let p2 = stroke(act - 8.0, JAB, 2.0, 2.0, 5.0).max(stroke(act - 14.0, CHORD, 2.0, 3.0, 15.0));
        let p3 = stroke(act - 14.0, CHORD, 2.0, 3.0, 18.0);
        let spread = if act >= 13.0 { 0.3 } else { 0.18 };
        let mut p = [Joint::REST; BONES];
        let wind = |i| if act < 0.0 { press(s, wf, i) } else { 0.0 };
        p[P1] = j(spread, 0.0, reach(p1.max(wind(0))));
        p[P2] = j(0.0, 0.03, reach(p2.max(wind(1))));
        p[P3] = j(-spread, 0.0, reach(p3.max(wind(2))));
        p[HORN] = j(0.0, rock(s, wf, &[(2.0, 0.06), (8.0, 0.06), (14.0, 0.14)]), 1.0);
        p
    });
    let light = |bone, window| HitDef {
        hitstop: 2,
        knock: 1.6,
        hitstun: 8,
        strength: 0.2,
        ..hit(vec![bone], window, 1.6)
    };
    MoveDef {
        keys,
        hits: vec![
            light(P1, (2, 5)),
            light(P2, (8, 11)),
            HitDef {
                hitstop: 6,
                knock: 5.0,
                hitstun: 18,
                strength: 0.55,
                ..hit(vec![P1, P2, P3], (14, 18), 4.0)
            },
        ],
        cues: vec![cue(Phase::Active, 13, Cue::PunchIn)],
        // the trumpet rocks back as it fires, so the pistons have room to show before they meet the glass
        lunge: -2.5,
        ..base("VALVE COMBO+", vec![Tag::Light, Tag::Melee], (w, a, r))
    }
}

/// VALVE COMBO+ double-tongued (only while hasted), 10/20/12: every stroke is two, "ta-ka", the second out of a half
/// return; the second of each pair is the ghost jab, a tiny extra hit. Then the chord, also tongued twice.
fn valve_combo_tk() -> MoveDef {
    let (w, a, r) = (10u32, 20u32, 12u32);
    let total = w + a + r;
    let wf = f64::from(w);
    // ta (out), half back, ka (out again), home
    let taka = |u: f64, len: f64, back: f64| -> f64 {
        if u < 0.0 {
            0.0
        } else if u < 3.0 {
            stroke(u, len, 1.5, 1.5, 3.0)
        } else {
            let first = len * (1.0 - 0.55 * smooth((u - 3.0) / 1.5));
            first.max(stroke(u - 4.5, len, 1.5, 1.0, back))
        }
    };
    let keys = sampled(total, |s| {
        let act = s - wf;
        let p1 = taka(act - 1.0, JAB, 4.0).max(taka(act - 12.0, CHORD, 9.0));
        let p2 = taka(act - 6.0, JAB, 4.0).max(taka(act - 12.0, CHORD, 10.0));
        let p3 = taka(act - 12.0, CHORD, 11.0);
        let spread = if act >= 11.0 { 0.3 } else { 0.18 };
        let mut p = [Joint::REST; BONES];
        p[P1] = j(spread, 0.0, reach(p1));
        p[P2] = j(0.0, 0.03, reach(p2));
        p[P3] = j(-spread, 0.0, reach(p3));
        p[HORN] = j(
            0.0,
            rock(
                s,
                wf,
                &[
                    (1.0, 0.05),
                    (5.5, 0.04),
                    (6.0, 0.05),
                    (10.5, 0.04),
                    (12.0, 0.1),
                    (16.5, 0.08),
                ],
            ),
            1.0,
        );
        p
    });
    let jab = |bones: Vec<usize>, window, strength: f64, damage| HitDef {
        hitstop: if strength > 0.3 { 5 } else { 1 },
        knock: if strength > 0.3 { 4.0 } else { 1.2 },
        hitstun: if strength > 0.3 { 14 } else { 6 },
        strength,
        ..hit(bones, window, damage)
    };
    MoveDef {
        keys,
        hits: vec![
            jab(vec![P1], (1, 3), 0.18, 1.2),
            jab(vec![P1], (5, 7), 0.08, 0.5),
            jab(vec![P2], (6, 8), 0.18, 1.2),
            jab(vec![P2], (10, 12), 0.08, 0.5),
            jab(vec![P1, P2, P3], (12, 15), 0.45, 2.6),
            jab(vec![P1, P2, P3], (16, 19), 0.08, 0.8),
        ],
        ..base("TA-KA COMBO", vec![Tag::Light, Tag::Melee, Tag::Hasted], (w, a, r))
    }
}

/// HERALD LINE's poles: how many, and how long one stands (steps).
pub const POLES: usize = 6;
pub const POLE_LIFE: u32 = 420;
/// A pole's rise and fold, steps.
pub const POLE_RISE: f64 = 10.0;
pub const POLE_FOLD: f64 = 20.0;
/// How far from the foe's centre the poles stand.
pub const POLE_RING: f64 = 1.15;

/// HERALD LINE, 30/8/24. Wind-up: the horn lifts to the sky and calls. Active: the six poles rise one a step in a
/// ring round the foe. Each stands `POLE_LIFE` steps as a fence: a bounce off one is a light nick (it can nick again
/// 30 steps later) and a hard push back inside.
fn herald_line() -> MoveDef {
    let (w, a, r) = (30u32, 8u32, 24u32);
    let total = w + a + r;
    let wf = f64::from(w);
    let keys = sampled(total, |s| {
        let up = smooth(s / (wf * 0.7)) * (1.0 - smooth((s - wf - 10.0) / 14.0));
        let mut p = [Joint::REST; BONES];
        // the bell to the sky, with a little shake on the call
        let call = if s > wf * 0.7 { 0.03 * sin(s * 2.1) } else { 0.0 };
        p[HORN] = j(0.0, -1.2 * up + call, 1.0);
        p
    });
    let spawns = (0..POLES)
        .map(|i| Spawn {
            at: i as u32,
            proj: ProjDef {
                place: Some((std::f64::consts::TAU * i as f64 / POLES as f64 + 0.3, POLE_RING)),
                radius: 0.14,
                life: POLE_LIFE,
                rehit: 30,
                ..proj(
                    "HERALD POLE",
                    HitDef {
                        hitstop: 2,
                        knock: 4.0,
                        hitstun: 6,
                        strength: 0.12,
                        ..hit(vec![], (0, 1), 1.0)
                    },
                )
            },
        })
        .collect();
    MoveDef {
        keys,
        spawns,
        cues: vec![cue(Phase::Windup, 20, Cue::PunchIn)],
        ..base("HERALD LINE", vec![Tag::Special, Tag::Ranged], (w, a, r))
    }
}

/// DOUBLE TONGUE+, 30/6/12. Wind-up: the horn lifts, and the three buttons flutter (ta-ka-ta-ka). Active start: the
/// haste (`Tag::Haste`, 4 s) with a small gold flash the look draws. No hit of its own.
fn double_tongue() -> MoveDef {
    let (w, a, r) = (30u32, 6u32, 12u32);
    let total = w + a + r;
    let wf = f64::from(w);
    let keys = sampled(total, |s| {
        let lift = smooth(s / (wf * 0.6)) * (1.0 - smooth((s - wf) / 12.0));
        let mut p = [Joint::REST; BONES];
        for (i, b) in [P1, P2, P3].into_iter().enumerate() {
            // the flutter: each button bobs out and in, a third of a beat apart, faster toward the release
            let ph = s * (0.9 + 0.5 * (s / wf).min(1.0)) + i as f64 * 2.1;
            let out = if s < wf {
                0.06 + 0.1 * sin(ph).max(0.0) * smooth(s / 8.0)
            } else {
                0.0
            };
            p[b] = j((1.0 - i as f64) * 0.18, 0.0, reach(out));
        }
        p[HORN] = j(0.0, -0.5 * lift, 1.0);
        p
    });
    MoveDef {
        keys,
        ..base("DOUBLE TONGUE+", vec![Tag::Special, Tag::Haste], (w, a, r))
    }
}

/// PISTON ROCKETS' rockets: when each sets off (active steps) and the angle it fans out at.
pub const ROCKETS: [(u32, f64); 3] = [(0, 1.0), (30, -1.0), (60, 0.0)];
/// A rocket's life, steps.
pub const ROCKET_LIFE: u32 = 240;

/// PISTON ROCKETS (low HP), 84/64/24. Wind-up 0.7 s: the pistons rattle loose, jumping in their casings, harder and
/// harder. Active: they launch 0.25 s apart, fanning out left, right and straight, and their homing curves them back
/// in on the foe. Three medium hits.
fn piston_rockets() -> MoveDef {
    let (w, a, r) = (84u32, 64u32, 24u32);
    let total = w + a + r;
    let wf = f64::from(w);
    let keys = sampled(total, |s| {
        let act = s - wf;
        let mut p = [Joint::REST; BONES];
        for (i, b) in [P1, P2, P3].into_iter().enumerate() {
            // gone once its rocket has set off; rattling before
            let gone = act >= f64::from(ROCKETS[i].0);
            let shake = (s / wf).min(1.0);
            let out = if gone {
                0.0
            } else {
                0.05 + (0.06 + 0.14 * shake) * sin(s * (1.7 + 0.4 * i as f64) + i as f64).abs()
            };
            p[b] = j((1.0 - i as f64) * 0.18, 0.0, reach(out));
        }
        let shiver = 0.04 * (s / wf).min(1.0) * sin(s * 2.7);
        p[HORN] = j(0.0, if act < 0.0 { shiver - 0.1 * smooth(s / wf) } else { 0.0 }, 1.0);
        p[LAUNCH] = j(0.0, 0.0, reach(0.5));
        p
    });
    let spawns = ROCKETS
        .iter()
        .map(|&(at, aim)| Spawn {
            at,
            proj: ProjDef {
                speed: 4.5,
                radius: 0.14,
                life: ROCKET_LIFE,
                homing: 5.0,
                aim,
                level: true,
                ..proj(
                    "PISTON ROCKET",
                    HitDef {
                        hitstop: 4,
                        knock: 3.0,
                        hitstun: 12,
                        strength: 0.3,
                        ..hit(vec![], (0, 1), 3.0)
                    },
                )
            },
        })
        .collect();
    MoveDef {
        keys,
        spawns,
        // the rattle is long: hits don't stop it, so the rockets always fly
        armor: true,
        ..base("PISTON ROCKETS", vec![Tag::Special, Tag::Low, Tag::Ranged], (w, a, r))
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

/// The SPIKE TRUMPET: only the candidate standard attacks (`BF_TRUMPET_STD` picks one).
pub fn def() -> FighterDef {
    let bone = |name, radius| Bone {
        name,
        parent: None,
        length: STUB,
        radius,
    };
    let rig = Rig::new(vec![
        bone("p1", 0.08),
        bone("p2", 0.08),
        bone("p3", 0.08),
        bone("horn", 0.001),
        bone("launch", 0.001),
    ])
    .expect("the trumpet's rig is parent-first");
    FighterDef {
        name: "TRUMPET".into(),
        r: 0.6,
        mass: 1.0,
        hp_scale: 4.0,
        rig,
        moves: vec![
            valve_combo(),
            valve_combo_tk(),
            herald_line(),
            double_tongue(),
            piston_rockets(),
        ],
        brain: Brain {
            choices: vec![
                choice(VALVE_COMBO, (1.1, 2.3), 3.0, 110),
                choice(VALVE_COMBO_TK, (1.1, 2.3), 3.0, 50),
                choice(HERALD_LINE, (1.2, 4.0), 1.2, 900),
                choice(DOUBLE_TONGUE, (1.1, 3.0), 0.8, 1900),
            ],
            speed: 2.6,
            think: 6,
            low: Some(PISTON_ROCKETS),
            orbit: 0.4,
        },
        colour: COLOUR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trumpet_checks() {
        assert_eq!(def().check(), Ok(()));
    }

    #[test]
    fn trumpet_moves_are_tagged_for_the_brain() {
        let d = def();
        assert!(d.moves[VALVE_COMBO_TK].has(Tag::Hasted));
        assert!(d.moves[DOUBLE_TONGUE].has(Tag::Haste));
        assert!(d.moves[PISTON_ROCKETS].has(Tag::Low));
        assert_eq!(d.brain.low, Some(PISTON_ROCKETS));
    }
}
