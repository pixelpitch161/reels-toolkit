//! Moves: phases in steps (wind-up, active, recovery), a pose track of eased keys, hits on bones, projectile
//! spawns and the camera and time cues the director reads.

use crate::rig::{Joint, Pose, Rig};

/// A move's phases, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The telegraph: no hitboxes yet.
    Windup,
    /// Hitboxes live.
    Active,
    Recovery,
}

impl Phase {
    /// 0, 1 or 2, for the recording.
    pub fn index(self) -> u8 {
        match self {
            Phase::Windup => 0,
            Phase::Active => 1,
            Phase::Recovery => 2,
        }
    }
}

/// How a pose track moves into a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ease {
    Linear,
    /// t².
    In,
    /// 1 − (1 − t)².
    Out,
    /// Smoothstep.
    InOut,
    /// Holds, then jumps in the last 15 %.
    Snap,
}

/// The eased share of the way at `t`, which is clamped to 0 to 1.
pub fn ease(e: Ease, t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    match e {
        Ease::Linear => t,
        Ease::In => t * t,
        Ease::Out => 1.0 - (1.0 - t) * (1.0 - t),
        Ease::InOut => t * t * (3.0 - 2.0 * t),
        Ease::Snap => {
            if t <= 0.85 {
                0.0
            } else {
                ((t - 0.85) / 0.15).min(1.0)
            }
        }
    }
}

/// A pose key. Joints not named keep the previous key's value, starting from rest.
#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    /// 0 to 1 of the whole move.
    pub at: f64,
    /// The easing into this key.
    pub ease: Ease,
    pub joints: Vec<(usize, Joint)>,
}

/// What kind of move it is, for behaviour, captions and the director.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    Light,
    Heavy,
    Special,
    Melee,
    Ranged,
    Grab,
    Low,
    /// Its Active start hastes the fighter for [`HASTE_STEPS`]: move cooldowns run twice as fast and it moves 1.3x
    /// as fast.
    Haste,
    /// The brain picks it only while hasted, and while hasted picks only such moves.
    Hasted,
}

/// How long a [`Tag::Haste`] move's haste lasts: 4 s.
pub const HASTE_STEPS: u32 = 480;

/// A grab: the target is held at the tip of `bone`, then slammed down or thrown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grab {
    /// Steps held.
    pub hold: u32,
    /// The holder's bone whose tip holds the target.
    pub bone: usize,
    /// Downward speed on release; 0 means a throw.
    pub slam: f64,
    /// Horizontal speed of a throw, along the holder's facing.
    pub throw: f64,
}

/// A hit: which bones carry it, when it is live and what it does.
#[derive(Clone, Debug, PartialEq)]
pub struct HitDef {
    pub bones: Vec<usize>,
    /// Steps from the Active start, inclusive to exclusive.
    pub window: (u32, u32),
    pub damage: f64,
    /// Freeze-on-hit, in steps, for both fighters.
    pub hitstop: u32,
    /// Speed away from the attacker.
    pub knock: f64,
    /// Upward speed.
    pub lift: f64,
    pub hitstun: u32,
    /// 0 to 1, for effects and the camera.
    pub strength: f64,
    /// Hits every step of the window instead of once.
    pub multi: bool,
    pub grab: Option<Grab>,
}

/// A projectile kind (Task 6 runs them). It lives here so a `Spawn` can hold one.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjDef {
    pub name: &'static str,
    /// Speed along the attacker's facing.
    pub speed: f64,
    /// Upward speed at launch.
    pub up: f64,
    /// Whether gravity pulls it.
    pub gravity: bool,
    pub radius: f64,
    /// Steps before it expires.
    pub life: u32,
    /// What it does on contact (its `bones` are empty).
    pub hit: HitDef,
    /// Rope bounces before it expires.
    pub bounces: u32,
    /// Radians/s toward the target; 0 for none.
    pub homing: f64,
    /// Placed instead of launched: it appears at rest on the canvas at (angle off the attacker's facing, distance)
    /// from the target's centre. `None` launches it from the tip of the last bone.
    pub place: Option<(f64, f64)>,
    /// 0: it expires on its first hit. Otherwise it stays and can hit again this many steps later (a post the
    /// target bounces off).
    pub rehit: u32,
    /// Radians off the attacker's facing that a launched projectile sets off along (0: straight ahead), so a
    /// volley can fan out before its homing curves it back in.
    pub aim: f64,
    /// With homing: it also levels off to the target's centre height (a rocket thrown from the air comes down to
    /// meet the target instead of flying over it).
    pub level: bool,
}

/// A projectile launched during the Active phase.
#[derive(Clone, Debug, PartialEq)]
pub struct Spawn {
    /// Step from the Active start.
    pub at: u32,
    pub proj: ProjDef,
}

/// A camera or time cue. The sim passes cues through as events; the director (E4) reads them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cue {
    PunchIn,
    Tilt(f64),
    SlowMo { scale: f64 },
    WhipBack,
    Flash,
    Shake(f64),
    Custom(&'static str),
}

/// A cue at a step of a phase.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CueAt {
    pub phase: Phase,
    pub step: u32,
    pub cue: Cue,
}

/// A move as data.
#[derive(Clone, Debug, PartialEq)]
pub struct MoveDef {
    pub name: &'static str,
    pub tags: Vec<Tag>,
    /// Steps per phase.
    pub windup: u32,
    pub active: u32,
    pub recovery: u32,
    pub keys: Vec<Key>,
    pub hits: Vec<HitDef>,
    pub spawns: Vec<Spawn>,
    pub cues: Vec<CueAt>,
    /// Damage taken multiplier while in this move; 1.0 for none, below 1 blocks.
    pub guard: f64,
    /// Not interrupted by hits.
    pub armor: bool,
    /// Forward speed added at the Active start.
    pub lunge: f64,
    /// Upward speed at the Active start.
    pub hop: f64,
}

impl MoveDef {
    /// Steps in all three phases.
    pub fn total(&self) -> u32 {
        self.windup + self.active + self.recovery
    }

    pub fn has(&self, tag: Tag) -> bool {
        self.tags.contains(&tag)
    }
}

/// A move under way: which of the fighter's moves, and the step since it started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveState {
    pub def: usize,
    pub step: u32,
}

impl MoveState {
    /// The phase and the step within it. Past the end it stays in Recovery.
    pub fn phase(&self, d: &MoveDef) -> (Phase, u32) {
        if self.step < d.windup {
            (Phase::Windup, self.step)
        } else if self.step < d.windup + d.active {
            (Phase::Active, self.step - d.windup)
        } else {
            (Phase::Recovery, self.step - d.windup - d.active)
        }
    }

    pub fn done(&self, d: &MoveDef) -> bool {
        self.step >= d.total()
    }

    /// The pose at `t = step / total`: the keys at or before `t` applied in order from rest, then the next key's
    /// joints eased in from there. `out` keeps its buffer.
    pub fn pose(&self, d: &MoveDef, rig: &Rig, out: &mut Pose) {
        out.reset(rig);
        let total = d.total();
        let t = if total == 0 {
            1.0
        } else {
            self.step as f64 / total as f64
        };
        let mut prev_at = 0.0;
        for key in &d.keys {
            if key.at <= t {
                for &(bone, j) in &key.joints {
                    if let Some(slot) = out.joints.get_mut(bone) {
                        *slot = j;
                    }
                }
                prev_at = key.at;
            } else {
                let e = ease(key.ease, (t - prev_at) / (key.at - prev_at));
                for &(bone, j) in &key.joints {
                    if let Some(slot) = out.joints.get_mut(bone) {
                        *slot = slot.lerp(j, e);
                    }
                }
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::{Bone, Joint, Pose, Rig};

    fn rig() -> Rig {
        Rig::new(vec![
            Bone {
                name: "arm",
                parent: None,
                length: 0.5,
                radius: 0.1,
            },
            Bone {
                name: "fist",
                parent: Some(0),
                length: 0.3,
                radius: 0.1,
            },
        ])
        .unwrap()
    }

    fn joint(yaw: f64) -> Joint {
        Joint {
            yaw,
            pitch: 0.0,
            ext: 1.0,
        }
    }

    fn mv(windup: u32, active: u32, recovery: u32, keys: Vec<Key>) -> MoveDef {
        MoveDef {
            name: "TEST",
            tags: vec![Tag::Light, Tag::Melee],
            windup,
            active,
            recovery,
            keys,
            hits: vec![],
            spawns: vec![],
            cues: vec![],
            guard: 1.0,
            armor: false,
            lunge: 0.0,
            hop: 0.0,
        }
    }

    fn pose_at(d: &MoveDef, step: u32) -> Pose {
        let rig = rig();
        let mut out = Pose::rest(&rig);
        MoveState { def: 0, step }.pose(d, &rig, &mut out);
        out
    }

    #[test]
    fn phases_run_in_order() {
        let d = mv(3, 2, 4, vec![]);
        let at = |step| MoveState { def: 0, step }.phase(&d);
        assert_eq!(
            [at(0), at(1), at(2)],
            [(Phase::Windup, 0), (Phase::Windup, 1), (Phase::Windup, 2)]
        );
        assert_eq!([at(3), at(4)], [(Phase::Active, 0), (Phase::Active, 1)]);
        assert_eq!([at(5), at(8)], [(Phase::Recovery, 0), (Phase::Recovery, 3)]);
        assert!(!MoveState { def: 0, step: 8 }.done(&d));
        assert!(MoveState { def: 0, step: 9 }.done(&d));
        assert_eq!(d.total(), 9);
    }

    #[test]
    fn pose_hits_keys_exactly() {
        let keys = vec![
            Key {
                at: 0.2,
                ease: Ease::InOut,
                joints: vec![(0, joint(-0.9))],
            },
            Key {
                at: 0.5,
                ease: Ease::Snap,
                joints: vec![(0, joint(0.7)), (1, joint(0.3))],
            },
        ];
        let d = mv(4, 2, 4, keys);
        assert_eq!(pose_at(&d, 2).joints[0], joint(-0.9));
        assert_eq!(pose_at(&d, 5).joints[0], joint(0.7));
        assert_eq!(pose_at(&d, 5).joints[1], joint(0.3));
    }

    #[test]
    fn pose_eases_between_keys() {
        let key = |ease| {
            vec![
                Key {
                    at: 0.0,
                    ease: Ease::Linear,
                    joints: vec![],
                },
                Key {
                    at: 1.0,
                    ease,
                    joints: vec![(0, joint(1.0))],
                },
            ]
        };
        assert_eq!(pose_at(&mv(4, 2, 4, key(Ease::InOut)), 5).joints[0].yaw, 0.5);
        assert_eq!(pose_at(&mv(4, 2, 4, key(Ease::Linear)), 5).joints[0].yaw, 0.5);
        assert!(pose_at(&mv(4, 2, 4, key(Ease::In)), 5).joints[0].yaw < 0.5);
        assert!(pose_at(&mv(4, 2, 4, key(Ease::Out)), 5).joints[0].yaw > 0.5);
        assert_eq!(pose_at(&mv(4, 2, 4, key(Ease::Snap)), 5).joints[0].yaw, 0.0);
    }

    #[test]
    fn unnamed_joints_hold() {
        let keys = vec![
            Key {
                at: 0.2,
                ease: Ease::Linear,
                joints: vec![(0, joint(0.4)), (1, joint(-0.6))],
            },
            Key {
                at: 0.6,
                ease: Ease::Linear,
                joints: vec![(0, joint(1.2))],
            },
        ];
        let d = mv(4, 2, 4, keys);
        // Before the first key, from rest; the fist is never named again, so it holds -0.6.
        assert!((pose_at(&d, 1).joints[1].yaw + 0.3).abs() < 1e-12);
        for step in 2..10 {
            assert_eq!(pose_at(&d, step).joints[1], joint(-0.6), "step {step}");
        }
        assert!((pose_at(&d, 4).joints[0].yaw - 0.8).abs() < 1e-12);
        assert_eq!(pose_at(&d, 9).joints[0], joint(1.2));
    }

    #[test]
    fn ease_clamps() {
        for e in [Ease::Linear, Ease::In, Ease::Out, Ease::InOut, Ease::Snap] {
            assert_eq!(ease(e, -1.0), 0.0);
            assert_eq!(ease(e, 0.0), 0.0);
            assert_eq!(ease(e, 1.0), 1.0);
            assert_eq!(ease(e, 7.0), 1.0);
        }
        assert_eq!(ease(Ease::Snap, 0.85), 0.0);
        assert!((ease(Ease::Snap, 0.925) - 0.5).abs() < 1e-12);
        assert_eq!(ease(Ease::In, 0.5), 0.25);
        assert_eq!(ease(Ease::Out, 0.5), 0.75);
    }
}
