//! TRUMPET on Engine 2. SPIKE (throwaway, Aedan's round-1 picks for the standard attack): three candidate standard
//! attacks, so each can be rendered in-engine and compared before the real move set is designed.
//!
//! - VALVE COMBO: the Engine 1 attack grown: the three pearl buttons sink (wind-up), the pistons fire 1, 2, 3 on
//!   the eighth notes, then all three slam out together as one chord (the heavy hit).
//! - LEAD PIPE LANCE: the horn turns mouthpiece-first, and the lead pipe telescopes out in three brass sections
//!   behind a lunge.
//! - VALVE CAP YO-YOS: the three valve caps unscrew and whirl round the ball on springy cords, one hit each.
//!
//! `BF_TRUMPET_STD` = `1`, `2` or `5` (the concept numbers) makes the brain throw only that attack; unset, it picks
//! among all three.
//!
//! The rig: three pistons (`P1..P3`) and the `LANCE`, each a root bone 0.02 long at rest (inside the valve casing)
//! that the moves stretch out with `ext`; `HORN`, a root bone that is never drawn or hit, whose joint turns the
//! model inside the glass (yaw) and tilts it (pitch); and `HUB`, a root bone the yo-yo moves turn back into the
//! ball's centre, with the three cap cords (`CAP1..CAP3`) hanging from it, so the caps orbit the ball's centre.

pub mod look;

use bf2_sim::{Bone, Brain, Choice, Cue, CueAt, Ease, FighterDef, HitDef, Joint, Key, MoveDef, Phase, Rig, Tag};

pub const P1: usize = 0;
pub const P2: usize = 1;
pub const P3: usize = 2;
pub const LANCE: usize = 3;
pub const HORN: usize = 4;
pub const HUB: usize = 5;
pub const CAP1: usize = 6;
pub const CAP2: usize = 7;
pub const CAP3: usize = 8;
pub const BONES: usize = 9;

/// A bone's rest length: tiny, so the drawn parts sit inside the casing until a move stretches them.
pub const STUB: f64 = 0.02;

pub const VALVE_COMBO: usize = 0;
pub const LEAD_PIPE_LANCE: usize = 1;
pub const VALVE_CAP_YOYOS: usize = 2;

/// The yellow of the current engine's TRUMPET (roster `color`).
pub const COLOUR: [u8; 3] = [255, 214, 60];

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

fn out_back(x: f64) -> f64 {
    // An ease-out with a little overshoot (1.0 at x = 1, peaks about 1.08).
    let x = x.clamp(0.0, 1.0);
    let c = 1.6;
    1.0 + (c + 1.0) * (x - 1.0).powi(3) + c * (x - 1.0).powi(2)
}

/// One piston's stroke: out in `t_out` steps (fast ease-out), hold `t_hold`, back in `t_back` with a little spring
/// bounce; `u` is steps since it fired. Gives metres out of the glass.
fn stroke(u: f64, len: f64, t_out: f64, t_hold: f64, t_back: f64) -> f64 {
    if u < 0.0 {
        0.0
    } else if u < t_out {
        len * (1.0 - (1.0 - u / t_out).powi(3))
    } else if u < t_out + t_hold {
        len
    } else if u < t_out + t_hold + t_back {
        let x = (u - t_out - t_hold) / t_back;
        // home, with a small rebound out and back at the end
        let home = 1.0 - smooth(x);
        let bounce = (x * std::f64::consts::PI * 2.0).sin().max(0.0) * 0.06 * (1.0 - x);
        len * (home + bounce)
    } else {
        0.0
    }
}

/// VALVE COMBO, 18/18/22. Wind-up: the pearl buttons sink and the horn rocks back a touch (anticipation). Active:
/// piston 1 at step 1, piston 2 at step 7 (light jabs, 1.3 m), then all three at step 13 as one chord, spread a
/// little wider and reaching 1.45 m (the heavy). Recovery: they spring home one by one and the horn settles.
fn valve_combo() -> MoveDef {
    let (w, a, r) = (18u32, 18u32, 22u32);
    let total = w + a + r;
    let wf = f64::from(w);
    let keys = sampled(total, |s| {
        let act = s - wf; // steps into Active
        // the horn rocks back through the wind-up, kicks forward on each shot
        let rock = if act < 0.0 {
            -0.18 * smooth(s / wf)
        } else {
            let kick = |at: f64, amt: f64| {
                let u = act - at;
                if u < 0.0 { 0.0 } else { amt * (-u / 4.0).exp() }
            };
            -0.18 * (1.0 - smooth(act / 4.0)) + kick(1.0, 0.06) + kick(7.0, 0.06) + kick(13.0, 0.14)
        };
        let jab = 1.5;
        let chord = 1.7;
        let p1 = stroke(act - 1.0, jab, 2.0, 2.0, 5.0).max(stroke(act - 13.0, chord, 2.0, 3.0, 14.0));
        let p2 = stroke(act - 7.0, jab, 2.0, 2.0, 5.0).max(stroke(act - 13.0, chord, 2.0, 3.0, 17.0));
        let p3 = stroke(act - 13.0, chord, 2.0, 3.0, 20.0);
        // the jabs fan a little; the chord spreads wider so the three read as three
        let spread = if act >= 12.0 { 0.16 } else { 0.1 };
        let mut p = [Joint::REST; BONES];
        p[P1] = j(spread, 0.0, reach(p1));
        p[P2] = j(0.0, 0.03, reach(p2));
        p[P3] = j(-spread, 0.0, reach(p3));
        p[HORN] = j(0.0, rock, 1.0);
        p
    });
    let light = |bone, window| HitDef {
        hitstop: 2,
        knock: 1.6,
        hitstun: 8,
        strength: 0.18,
        ..hit(vec![bone], window, 1.6)
    };
    MoveDef {
        keys,
        hits: vec![
            light(P1, (1, 4)),
            light(P2, (7, 10)),
            HitDef {
                hitstop: 6,
                knock: 5.0,
                hitstun: 18,
                strength: 0.5,
                ..hit(vec![P1, P2, P3], (13, 17), 4.0)
            },
        ],
        cues: vec![CueAt {
            phase: Phase::Active,
            step: 10,
            cue: Cue::PunchIn,
        }],
        lunge: 0.0,
        ..base("VALVE COMBO", vec![Tag::Light, Tag::Melee], (w, a, r))
    }
}

/// LEAD PIPE LANCE, 22/5/24. Wind-up: the horn turns a quarter so its mouthpiece faces the foe and draws back
/// (the ball rolls back a little with it). Active: the lunge, and the lead pipe shoots out to 2.1 m in three
/// telescoping sections (the look draws them from one bone). Recovery: the sections click home and the horn turns
/// back.
fn lead_pipe_lance() -> MoveDef {
    let (w, a, r) = (22u32, 5u32, 24u32);
    let total = w + a + r;
    let wf = f64::from(w);
    let keys = sampled(total, |s| {
        let act = s - wf;
        let turn = if act < 0.0 {
            out_back(s / (wf * 0.7))
        } else if act < 5.0 + 6.0 {
            1.0
        } else {
            1.0 - smooth((act - 11.0) / 12.0)
        };
        let len = if act < 0.0 {
            // a short draw back: a touch out, then home, just before the thrust
            0.12 * (s / wf * std::f64::consts::PI).sin().max(0.0)
        } else if act < 2.0 {
            2.1 * (1.0 - (1.0 - act / 2.0).powi(3))
        } else if act < 7.0 {
            2.1
        } else {
            // three clicks home: each section snaps in over 4 steps, 2 apart
            let x = act - 7.0;
            let click = |at: f64| 0.7 * smooth((x - at) / 3.0);
            (2.1 - click(0.0) - click(5.0) - click(10.0)).max(0.0)
        };
        let mut p = [Joint::REST; BONES];
        p[LANCE] = j(0.0, 0.0, reach(len));
        // HORN: yaw is the turn (a quarter turn = mouthpiece first), pitch a small dip on the thrust
        p[HORN] = j(
            turn * std::f64::consts::FRAC_PI_2,
            if (0.0..6.0).contains(&act) { 0.06 } else { 0.0 },
            1.0,
        );
        p
    });
    MoveDef {
        keys,
        hits: vec![HitDef {
            hitstop: 6,
            knock: 6.0,
            hitstun: 20,
            strength: 0.45,
            ..hit(vec![LANCE], (0, 5), 5.0)
        }],
        cues: vec![CueAt {
            phase: Phase::Windup,
            step: 6,
            cue: Cue::PunchIn,
        }],
        lunge: 3.2,
        ..base("LEAD PIPE LANCE", vec![Tag::Heavy, Tag::Melee], (w, a, r))
    }
}

/// VALVE CAP YO-YOS, 16/26/20. Wind-up: the three caps pop up off the valves and wobble on their cords, close to
/// the glass. Active: they swing out to 1.35 m and whirl one full turn round the ball, 120 degrees apart, each
/// able to hit once. Recovery: they reel in fast with a twist and screw home.
fn valve_cap_yoyos() -> MoveDef {
    let (w, a, r) = (16u32, 26u32, 20u32);
    let total = w + a + r;
    let wf = f64::from(w);
    let af = f64::from(a);
    let rf = f64::from(r);
    let third = std::f64::consts::TAU / 3.0;
    let keys = sampled(total, |s| {
        let act = s - wf;
        // cord length from the ball's centre (the glass is at 0.6)
        let (len, spin, lift) = if act < 0.0 {
            let x = s / wf;
            let wobble = (s * 1.3).sin() * 0.08 * x;
            (0.6 + 0.25 * out_back(x), -0.35 * smooth(x) + wobble, 0.9 * smooth(x))
        } else if act <= af {
            let x = act / af;
            (
                0.85 + 0.5 * smooth(act / 4.0),
                std::f64::consts::TAU * smooth(x) * 1.0 - 0.35 * (1.0 - smooth(act / 4.0)),
                0.9 * (1.0 - smooth(act / 5.0)) + 0.12,
            )
        } else {
            let x = (act - af) / rf;
            (
                1.35 - 0.75 * smooth(x / 0.6),
                std::f64::consts::TAU + 0.8 * smooth(x),
                0.12 + 0.8 * smooth(x),
            )
        };
        let mut p = [Joint::REST; BONES];
        // the hub turns back into the ball's centre (0.6 behind the surface point)
        let hub_on = if s <= 1.0 || s >= f64::from(total) { 0.0 } else { 1.0 };
        p[HUB] = j(
            std::f64::consts::PI * hub_on,
            0.0,
            if hub_on > 0.0 { reach(0.6) } else { 1.0 },
        );
        for (i, b) in [CAP1, CAP2, CAP3].into_iter().enumerate() {
            // relative to the hub's yaw (pi): the caps' own angle round the ball is spin + i * third
            let ang = spin + f64::from(i as u32) * third;
            let l = if hub_on > 0.0 { len } else { STUB };
            p[b] = j(
                ang - std::f64::consts::PI * hub_on,
                lift.min(1.2) * (0.35 + 0.15 * f64::from(i as u32 % 2)),
                reach(l),
            );
        }
        p
    });
    let cap = |bone| HitDef {
        hitstop: 2,
        knock: 2.2,
        hitstun: 10,
        strength: 0.18,
        ..hit(vec![bone], (2, 26), 1.8)
    };
    MoveDef {
        keys,
        hits: vec![cap(CAP1), cap(CAP2), cap(CAP3)],
        ..base("VALVE CAP YO-YOS", vec![Tag::Light, Tag::Melee], (w, a, r))
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
    let bone = |name, parent, radius| Bone {
        name,
        parent,
        length: STUB,
        radius,
    };
    let rig = Rig::new(vec![
        bone("p1", None, 0.08),
        bone("p2", None, 0.08),
        bone("p3", None, 0.08),
        bone("lance", None, 0.08),
        bone("horn", None, 0.001),
        bone("hub", None, 0.001),
        bone("cap1", Some(HUB), 0.1),
        bone("cap2", Some(HUB), 0.1),
        bone("cap3", Some(HUB), 0.1),
    ])
    .expect("the trumpet's rig is parent-first");
    let only = std::env::var("BF_TRUMPET_STD").ok();
    let all = [
        ("1", choice(VALVE_COMBO, (1.7, 2.6), 2.0, 110)),
        ("2", choice(LEAD_PIPE_LANCE, (1.6, 3.4), 1.5, 150)),
        ("5", choice(VALVE_CAP_YOYOS, (1.3, 2.1), 1.5, 150)),
    ];
    let choices = all
        .into_iter()
        .filter(|(n, _)| only.as_deref().is_none_or(|o| o == *n))
        .map(|(_, c)| c)
        .collect();
    FighterDef {
        name: "TRUMPET".into(),
        r: 0.6,
        mass: 1.0,
        hp_scale: 4.0,
        rig,
        moves: vec![valve_combo(), lead_pipe_lance(), valve_cap_yoyos()],
        brain: Brain {
            choices,
            speed: 2.6,
            think: 6,
            low: None,
            orbit: 0.4,
        },
        colour: COLOUR,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn trumpet_checks() {
        assert_eq!(super::def().check(), Ok(()));
    }
}
