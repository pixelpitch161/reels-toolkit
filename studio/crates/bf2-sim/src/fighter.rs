//! Fighter definitions (content data; the engine owns no fighter) and a fight's setup.

use std::sync::Arc;

use crate::brain::Brain;
use crate::moves::MoveDef;
use crate::rig::Rig;
use crate::rules::{Rules2, SimError2};

/// One fighter as data.
#[derive(Clone, Debug, PartialEq)]
pub struct FighterDef {
    pub name: String,
    /// Ball radius (0.6).
    pub r: f64,
    pub mass: f64,
    /// HP multiplier on `Rules2::hp`.
    pub hp_scale: f64,
    pub rig: Rig,
    pub moves: Vec<MoveDef>,
    pub brain: Brain,
    pub colour: [u8; 3],
}

/// Everything one fight needs.
#[derive(Clone, Debug)]
pub struct Setup2 {
    pub fighters: [Arc<FighterDef>; 2],
    pub rules: Rules2,
    pub seed: u64,
}

fn finite(field: impl Into<String>, v: f64, min: f64) -> Result<(), SimError2> {
    if v.is_finite() && v >= min {
        Ok(())
    } else {
        Err(SimError2::new(
            field,
            format!("must be a finite number, at least {min}, not {v}"),
        ))
    }
}

impl FighterDef {
    /// Bone and move indices in range, hit windows inside the Active phase, keys sorted by `at` within 0 to 1,
    /// and sane sizes. Fields are named from the fighter (`moves[2].hits[0].window`); the message names it.
    pub fn check(&self) -> Result<(), SimError2> {
        self.check_inner()
            .map_err(|e| SimError2::new(e.field, format!("{}: {}", self.name, e.message)))
    }

    fn check_inner(&self) -> Result<(), SimError2> {
        let bones = self.rig.bones.len();
        let n = self.moves.len();
        let bone_ok = |field: String, b: usize| {
            if b < bones {
                Ok(())
            } else {
                Err(SimError2::new(
                    field,
                    format!("bone {b} is not in the rig ({bones} bones)"),
                ))
            }
        };
        let move_ok = |field: &str, m: usize| {
            if m < n {
                Ok(())
            } else {
                Err(SimError2::new(field, format!("move {m} is not one of the {n} moves")))
            }
        };
        finite("r", self.r, f64::MIN_POSITIVE)?;
        finite("mass", self.mass, f64::MIN_POSITIVE)?;
        finite("hp_scale", self.hp_scale, f64::MIN_POSITIVE)?;
        self.rig.check()?;
        for (i, m) in self.moves.iter().enumerate() {
            let at = format!("moves[{i}]");
            finite(format!("{at}.guard"), m.guard, 0.0)?;
            finite(format!("{at}.lunge"), m.lunge, f64::MIN)?;
            finite(format!("{at}.hop"), m.hop, 0.0)?;
            let mut prev = 0.0;
            for (k, key) in m.keys.iter().enumerate() {
                if !(prev..=1.0).contains(&key.at) {
                    return Err(SimError2::new(
                        format!("{at}.keys[{k}].at"),
                        format!("keys must be sorted by at, from 0 to 1 (got {} after {prev})", key.at),
                    ));
                }
                prev = key.at;
                for &(b, _) in &key.joints {
                    bone_ok(format!("{at}.keys[{k}].joints"), b)?;
                }
            }
            let mut hits = m
                .hits
                .iter()
                .map(|h| (h, format!("{at}.hits")))
                .enumerate()
                .collect::<Vec<_>>();
            hits.extend(
                m.spawns
                    .iter()
                    .map(|s| (&s.proj.hit, format!("{at}.spawns")))
                    .enumerate(),
            );
            for (j, (h, list)) in hits {
                let hf = format!("{list}[{j}]");
                if list.ends_with("hits") {
                    for &b in &h.bones {
                        bone_ok(format!("{hf}.bones"), b)?;
                    }
                    if !(h.window.0 < h.window.1 && h.window.1 <= m.active) {
                        return Err(SimError2::new(
                            format!("{hf}.window"),
                            format!(
                                "{:?} must be a non-empty window inside the {} active steps",
                                h.window, m.active
                            ),
                        ));
                    }
                }
                finite(format!("{hf}.damage"), h.damage, 0.0)?;
                finite(format!("{hf}.knock"), h.knock, f64::MIN)?;
                finite(format!("{hf}.lift"), h.lift, 0.0)?;
                finite(format!("{hf}.strength"), h.strength, 0.0)?;
                if let Some(g) = &h.grab {
                    bone_ok(format!("{hf}.grab.bone"), g.bone)?;
                    finite(format!("{hf}.grab.slam"), g.slam, 0.0)?;
                    finite(format!("{hf}.grab.throw"), g.throw, f64::MIN)?;
                }
            }
            for (j, sp) in m.spawns.iter().enumerate() {
                let sf = format!("{at}.spawns[{j}]");
                if sp.at >= m.active {
                    return Err(SimError2::new(
                        format!("{sf}.at"),
                        format!("{} must be inside the {} active steps", sp.at, m.active),
                    ));
                }
                finite(format!("{sf}.proj.radius"), sp.proj.radius, f64::MIN_POSITIVE)?;
                finite(format!("{sf}.proj.speed"), sp.proj.speed, 0.0)?;
                finite(format!("{sf}.proj.up"), sp.proj.up, f64::MIN)?;
                finite(format!("{sf}.proj.homing"), sp.proj.homing, 0.0)?;
                if sp.proj.hit.grab.is_some() {
                    // A grab holds the target on the thrower's bone: from a projectile it would teleport it.
                    return Err(SimError2::new(
                        format!("{sf}.proj.hit.grab"),
                        "a projectile cannot grab (grabs are melee only)",
                    ));
                }
            }
        }
        let b = &self.brain;
        for (i, c) in b.choices.iter().enumerate() {
            move_ok(&format!("brain.choices[{i}].mv"), c.mv)?;
            finite(format!("brain.choices[{i}].weight"), c.weight, 0.0)?;
        }
        if let Some(low) = b.low {
            move_ok("brain.low", low)?;
        }
        if b.think == 0 {
            return Err(SimError2::new("brain.think", "must be at least 1 step"));
        }
        finite("brain.speed", b.speed, 0.0)?;
        if !(0.0..=1.0).contains(&b.orbit) {
            return Err(SimError2::new(
                "brain.orbit",
                format!("must be from 0 to 1, not {}", b.orbit),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::brain::Choice;
    use crate::moves::{Ease, Key};
    use crate::testkit::*;

    #[test]
    fn check_refuses_a_grab_on_a_projectile() {
        let mut p = crate::proj::ProjDef {
            name: "NET",
            speed: 6.0,
            up: 0.0,
            gravity: false,
            radius: 0.1,
            life: 60,
            hit: crate::moves::HitDef {
                bones: vec![],
                ..hit(1.0)
            },
            bounces: 0,
            homing: 0.0,
        };
        p.hit.grab = Some(crate::moves::Grab {
            hold: 10,
            bone: 0,
            slam: 1.0,
            throw: 0.0,
        });
        let mv = crate::moves::MoveDef {
            spawns: vec![crate::moves::Spawn { at: 0, proj: p }],
            ..poke(vec![])
        };
        let e = fighter(vec![mv]).check().unwrap_err();
        assert_eq!(e.field, "moves[0].spawns[0].proj.hit.grab");
    }

    #[test]
    fn check_refuses_out_of_range_bone() {
        assert!(fighter(vec![poke(vec![hit(1.0)])]).check().is_ok());
        let e = fighter(vec![poke(vec![crate::moves::HitDef {
            bones: vec![1],
            ..hit(1.0)
        }])])
        .check()
        .unwrap_err();
        assert_eq!(e.field, "moves[0].hits[0].bones");
        let late = crate::moves::HitDef {
            window: (2, 6),
            ..hit(1.0)
        };
        assert_eq!(
            fighter(vec![poke(vec![late])]).check().unwrap_err().field,
            "moves[0].hits[0].window"
        );
        let keys = vec![
            Key {
                at: 0.5,
                ease: Ease::Linear,
                joints: vec![],
            },
            Key {
                at: 0.2,
                ease: Ease::Linear,
                joints: vec![],
            },
        ];
        let unsorted = crate::moves::MoveDef { keys, ..poke(vec![]) };
        assert_eq!(
            fighter(vec![unsorted]).check().unwrap_err().field,
            "moves[0].keys[1].at"
        );
        let mut f = fighter(vec![poke(vec![])]);
        f.brain.choices.push(Choice {
            mv: 1,
            range: (0.0, 1.0),
            weight: 1.0,
            cooldown: 0,
            needs_meter: false,
            air: None,
        });
        assert_eq!(f.check().unwrap_err().field, "brain.choices[0].mv");
        let mut f = fighter(vec![poke(vec![])]);
        f.brain.low = Some(3);
        assert_eq!(f.check().unwrap_err().field, "brain.low");
    }
}
