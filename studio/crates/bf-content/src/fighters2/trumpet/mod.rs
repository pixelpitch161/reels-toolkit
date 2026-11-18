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
pub const LANCE: usize = 3;
pub const HORN: usize = 4;
pub const HUB: usize = 5;
pub const CAP1: usize = 6;
pub const CAP2: usize = 7;
pub const CAP3: usize = 8;
/// ROYAL HERALDS: the three herald trumpets (root bones, 120 degrees apart).
pub const H1: usize = 9;
pub const H2: usize = 10;
pub const H3: usize = 11;
/// CAVALRY CHARGE's front (never drawn): the bone that hits. Last, so projectiles launch from the ball's front.
pub const RAM: usize = 12;
pub const BONES: usize = 13;

/// A bone's rest length: tiny, so the drawn parts sit inside the casing until a move stretches them.
pub const STUB: f64 = 0.02;

pub const VALVE_COMBO: usize = 0;
pub const LEAD_PIPE_LANCE: usize = 1;
pub const VALVE_CAP_YOYOS: usize = 2;
pub const HERALD_LINE: usize = 3;
pub const CAVALRY_CHARGE: usize = 4;
pub const TICKER_TAPE: usize = 5;
pub const ROYAL_HERALDS: usize = 6;

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
    1.0 + (c + 1.0) * cube(x - 1.0) + c * (x - 1.0) * (x - 1.0)
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
                if u < 0.0 { 0.0 } else { amt * exp(-u / 4.0) }
            };
            -0.18 * (1.0 - smooth(act / 4.0)) + kick(1.0, 0.06) + kick(7.0, 0.06) + kick(13.0, 0.14)
        };
        let jab = 1.5;
        let chord = 1.7;
        let p1 = stroke(act - 1.0, jab, 2.0, 2.0, 5.0).max(stroke(act - 13.0, chord, 2.0, 3.0, 14.0));
        let p2 = stroke(act - 7.0, jab, 2.0, 2.0, 5.0).max(stroke(act - 13.0, chord, 2.0, 3.0, 17.0));
        let p3 = stroke(act - 13.0, chord, 2.0, 3.0, 20.0);
        // the jabs fan a little; the chord spreads wider so the three read as three
        let spread = if act >= 12.0 { 0.3 } else { 0.18 };
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
            0.12 * sin(s / wf * std::f64::consts::PI).max(0.0)
        } else if act < 2.0 {
            2.1 * (1.0 - cube(1.0 - act / 2.0))
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
            let wobble = sin(s * 1.3) * 0.08 * x;
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
    }
}

/// The herald poles: six, round the foe 1.15 m out, standing 3.5 s.
pub const POLES: usize = 6;
pub const POLE_LIFE: u32 = 420;

/// HERALD LINE, 30/8/24 (round 2, idea 2). Wind-up: the bell tips up and the call sounds. Active: six banner poles
/// spring up out of the canvas in a ring round the foe (placed projectiles that stay 3.5 s); every time the foe
/// touches one it is nicked and bounced back inside (a hit at most every 30 steps per pole).
fn herald_line() -> MoveDef {
    let (w, a, r) = (30u32, 8u32, 24u32);
    let total = w + a + r;
    let wf = f64::from(w);
    let keys = sampled(total, |s| {
        let up = smooth(s / (wf * 0.7)) * (1.0 - smooth((s - wf - 10.0) / 14.0));
        let mut p = [Joint::REST; BONES];
        p[HORN] = j(-std::f64::consts::FRAC_PI_2 * up, 0.0, 1.0);
        p
    });
    let spawns = (0..POLES)
        .map(|i| Spawn {
            at: i as u32,
            proj: ProjDef {
                place: Some((std::f64::consts::TAU * i as f64 / POLES as f64 + 0.3, 1.15)),
                radius: 0.14,
                life: POLE_LIFE,
                rehit: 30,
                ..proj(
                    "HERALD POLE",
                    HitDef {
                        hitstop: 2,
                        knock: 4.0,
                        hitstun: 6,
                        strength: 0.15,
                        ..hit(vec![], (0, 1), 1.0)
                    },
                )
            },
        })
        .collect();
    MoveDef {
        keys,
        spawns,
        cues: vec![cue(Phase::Windup, 4, Cue::PunchIn)],
        ..base("HERALD LINE", vec![Tag::Special, Tag::Ranged], (w, a, r))
    }
}

/// CAVALRY CHARGE, 36/36/24 (idea 3). Wind-up: the bugle call; the ball rocks back on its heels. Active: it rockets
/// forward (a big lunge) and bowls the foe over (a launch). Recovery: it skids to a stop.
fn cavalry_charge() -> MoveDef {
    let (w, a, r) = (36u32, 36u32, 24u32);
    let total = w + a + r;
    let wf = f64::from(w);
    let keys = sampled(total, |s| {
        let act = s - wf;
        let rock = if act < 0.0 {
            -0.25 * sin(s / wf * std::f64::consts::PI).max(0.0)
        } else {
            0.0
        };
        let mut p = [Joint::REST; BONES];
        p[HORN] = j(0.0, rock, 1.0);
        p[RAM] = j(0.0, 0.0, if (0.0..=36.0).contains(&act) { reach(0.25) } else { 1.0 });
        p
    });
    MoveDef {
        keys,
        hits: vec![HitDef {
            hitstop: 8,
            knock: 6.0,
            lift: 8.0,
            hitstun: 30,
            strength: 0.55,
            ..hit(vec![RAM], (0, 36), 6.0)
        }],
        cues: vec![cue(Phase::Windup, 0, Cue::PunchIn)],
        lunge: 10.0,
        ..base("CAVALRY CHARGE", vec![Tag::Special, Tag::Melee], (w, a, r))
    }
}

/// The confetti flakes: ten, launched up over the foe.
pub const FLAKES: usize = 16;

/// TICKER TAPE, 22/14/20 (idea 6). Wind-up: the bell tips up. Active: ten brass flakes shoot up and flutter down
/// over the foe (light homing), each one that lands on it a tiny hit.
fn ticker_tape() -> MoveDef {
    let (w, a, r) = (22u32, 14u32, 20u32);
    let total = w + a + r;
    let wf = f64::from(w);
    let keys = sampled(total, |s| {
        let up = smooth(s / (wf * 0.8)) * (1.0 - smooth((s - wf - 14.0) / 12.0));
        let mut p = [Joint::REST; BONES];
        p[HORN] = j(-1.1 * up, 0.0, 1.0);
        p
    });
    let spawns = (0..FLAKES)
        .map(|i| {
            let f = i as f64 / (FLAKES - 1) as f64;
            Spawn {
                at: (i as u32 * 13) / FLAKES as u32,
                proj: ProjDef {
                    speed: 1.2 + 1.0 * ((i * 7 % FLAKES) as f64 / FLAKES as f64),
                    up: 7.0 + 1.0 * f,
                    gravity: true,
                    radius: 0.12,
                    life: 200,
                    homing: 2.5,
                    ..proj(
                        "CONFETTI",
                        HitDef {
                            hitstop: 0,
                            knock: 0.2,
                            hitstun: 2,
                            strength: 0.03,
                            ..hit(vec![], (0, 1), 0.35)
                        },
                    )
                },
            }
        })
        .collect();
    MoveDef {
        keys,
        spawns,
        ..base("TICKER TAPE", vec![Tag::Special, Tag::Ranged], (w, a, r))
    }
}

/// ROYAL HERALDS, 32/8/28 (idea 7). Wind-up: three long herald trumpets fold out of the ball in a Y, turning slowly.
/// Active: all three blow at once (each a hit along its length and a cone of air past its bell). Recovery: they
/// fold back in like a fan.
fn royal_heralds() -> MoveDef {
    let (w, a, r) = (32u32, 8u32, 28u32);
    let total = w + a + r;
    let wf = f64::from(w);
    let third = std::f64::consts::TAU / 3.0;
    let keys = sampled(total, |s| {
        let act = s - wf;
        let open = out_back(s / (wf * 0.8)) * (1.0 - smooth((act - 12.0) / 14.0));
        let turn = 0.5 * smooth(s / (wf + 8.0)) - 0.25;
        let len = 1.5 * open;
        let mut p = [Joint::REST; BONES];
        for (i, b) in [H1, H2, H3].into_iter().enumerate() {
            p[b] = j(turn + third * i as f64, 0.0, reach(len.max(STUB)));
        }
        p
    });
    MoveDef {
        keys,
        hits: vec![HitDef {
            hitstop: 6,
            knock: 5.0,
            hitstun: 18,
            strength: 0.4,
            ..hit(vec![H1, H2, H3], (1, 8), 4.0)
        }],
        cues: vec![cue(Phase::Windup, 6, Cue::PunchIn), cue(Phase::Active, 1, Cue::Flash)],
        ..base("ROYAL HERALDS", vec![Tag::Special, Tag::Melee], (w, a, r))
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
        bone("herald1", None, 0.12),
        bone("herald2", None, 0.12),
        bone("herald3", None, 0.12),
        bone("ram", None, 0.4),
    ])
    .expect("the trumpet's rig is parent-first");
    // Round 1: BF_TRUMPET_STD picks one standard attack (default: VALVE COMBO+, Aedan's pick). Round 2:
    // BF_TRUMPET_SPECIAL (2, 3, 6 or 7) adds that special, thrown whenever it is off cooldown.
    let std_pick = std::env::var("BF_TRUMPET_STD").unwrap_or_else(|_| "1".into());
    let std_all = [
        ("1", choice(VALVE_COMBO, (1.1, 2.6), 2.0, 110)),
        ("2", choice(LEAD_PIPE_LANCE, (1.6, 3.4), 1.5, 150)),
        ("5", choice(VALVE_CAP_YOYOS, (1.1, 2.1), 1.5, 150)),
    ];
    let special = std::env::var("BF_TRUMPET_SPECIAL").ok();
    let sp_all = [
        ("2", choice(HERALD_LINE, (1.2, 4.0), 50.0, 720)),
        ("3", choice(CAVALRY_CHARGE, (2.0, 6.0), 50.0, 150)),
        ("6", choice(TICKER_TAPE, (1.4, 3.2), 50.0, 480)),
        ("7", choice(ROYAL_HERALDS, (1.1, 2.0), 50.0, 480)),
    ];
    let choices = std_all
        .into_iter()
        .filter(|(n, _)| *n == std_pick)
        .chain(sp_all.into_iter().filter(|(n, _)| special.as_deref() == Some(*n)))
        .map(|(_, c)| c)
        .collect();
    FighterDef {
        name: "TRUMPET".into(),
        r: 0.6,
        mass: 1.0,
        hp_scale: 4.0,
        rig,
        moves: vec![
            valve_combo(),
            lead_pipe_lance(),
            valve_cap_yoyos(),
            herald_line(),
            cavalry_charge(),
            ticker_tape(),
            royal_heralds(),
        ],
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
