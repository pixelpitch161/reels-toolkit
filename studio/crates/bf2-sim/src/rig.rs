//! Weapon rigs: named bones in parent-first order, posed by joint angles, solved into capsules (forward
//! kinematics). The renderer draws the same capsules, so the picture shows exactly the pose that hit.

use bf_core::dmath;

use crate::rules::SimError2;
use crate::v3::{V3, add, scale};

/// One bone of a weapon rig. Its hitbox (and hurtbox) is a capsule of `radius` around the bone.
#[derive(Clone, Debug, PartialEq)]
pub struct Bone {
    pub name: &'static str,
    /// An earlier bone, or `None` for a root, which starts on the ball's surface.
    pub parent: Option<usize>,
    pub length: f64,
    /// The hitbox capsule's radius.
    pub radius: f64,
}

/// A weapon rig: bones in parent-first order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rig {
    pub bones: Vec<Bone>,
}

impl Rig {
    /// Checks the parent-first order: every parent comes before its child.
    pub fn new(bones: Vec<Bone>) -> Result<Rig, SimError2> {
        let rig = Rig { bones };
        rig.check()?;
        Ok(rig)
    }

    /// The parent-first order and finite, non-negative sizes.
    pub fn check(&self) -> Result<(), SimError2> {
        for (i, b) in self.bones.iter().enumerate() {
            if let Some(p) = b.parent
                && p >= i
            {
                return Err(SimError2::new(
                    format!("rig.bones[{i}].parent"),
                    format!("bone {} must come after its parent ({p})", b.name),
                ));
            }
            if !(b.length.is_finite() && b.length >= 0.0 && b.radius.is_finite() && b.radius >= 0.0) {
                return Err(SimError2::new(
                    format!("rig.bones[{i}]"),
                    format!("bone {} needs a finite length and radius, not below 0", b.name),
                ));
            }
        }
        Ok(())
    }

    /// The index of the bone called `name`.
    pub fn bone(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|b| b.name == name)
    }
}

/// One bone's pose. Angles are radians relative to the parent bone (a root's are relative to the ball's facing).
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Joint {
    /// Turn in the floor plane.
    pub yaw: f64,
    /// Tilt toward +z.
    pub pitch: f64,
    /// Length multiplier: 1.0 at rest (through `Pose::rest`).
    pub ext: f64,
}

impl Joint {
    /// The rest joint: no turn, no tilt, full length.
    pub const REST: Joint = Joint {
        yaw: 0.0,
        pitch: 0.0,
        ext: 1.0,
    };

    /// Each part moved the share `t` of the way to `b`.
    pub fn lerp(self, b: Joint, t: f64) -> Joint {
        Joint {
            yaw: self.yaw + (b.yaw - self.yaw) * t,
            pitch: self.pitch + (b.pitch - self.pitch) * t,
            ext: self.ext + (b.ext - self.ext) * t,
        }
    }
}

/// A rig's pose: one joint per bone, in the rig's order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pose {
    pub joints: Vec<Joint>,
}

impl Pose {
    /// Every joint at rest: yaw and pitch 0, ext 1.
    pub fn rest(rig: &Rig) -> Pose {
        Pose {
            joints: vec![Joint::REST; rig.bones.len()],
        }
    }

    /// Back to rest in place, keeping the buffer.
    pub fn reset(&mut self, rig: &Rig) {
        self.joints.clear();
        self.joints.resize(rig.bones.len(), Joint::REST);
    }
}

/// A solved bone: the capsule from `a` to `b` of `radius`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Segment {
    pub a: V3,
    pub b: V3,
    pub radius: f64,
}

/// Forward kinematics: one segment per bone into `out` (cleared first, buffer kept). A root bone starts on the
/// ball's surface in the facing direction, at `centre + r_ball·(cos facing, sin facing, 0)`. Each child starts at
/// its parent's `b`. A bone's direction is its summed yaw (facing plus every yaw up the chain) in the floor plane,
/// tilted toward +z by its summed pitch.
pub fn solve(rig: &Rig, pose: &Pose, centre: V3, r_ball: f64, facing: f64, out: &mut Vec<Segment>) {
    out.clear();
    let surface = add(centre, [r_ball * dmath::cos(facing), r_ball * dmath::sin(facing), 0.0]);
    for (i, bone) in rig.bones.iter().enumerate() {
        let ext = pose.joints.get(i).map_or(1.0, |j| j.ext);
        let start = bone.parent.map_or(surface, |p| out[p].b);
        // Parents come first and rigs are a few bones long, so walking up the chain beats keeping a buffer.
        let (yaw, pitch) = angles(rig, pose, i, facing);
        let cp = dmath::cos(pitch);
        let dir = [cp * dmath::cos(yaw), cp * dmath::sin(yaw), dmath::sin(pitch)];
        out.push(Segment {
            a: start,
            b: add(start, scale(dir, bone.length * ext)),
            radius: bone.radius,
        });
    }
}

/// The summed yaw (facing included) and pitch of bone `i`, walking up to its root.
fn angles(rig: &Rig, pose: &Pose, mut i: usize, facing: f64) -> (f64, f64) {
    let (mut yaw, mut pitch) = (0.0, 0.0);
    loop {
        let j = pose.joints.get(i).copied().unwrap_or(Joint::REST);
        yaw += j.yaw;
        pitch += j.pitch;
        match rig.bones[i].parent {
            Some(p) => i = p,
            None => return (facing + yaw, pitch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone(name: &'static str, parent: Option<usize>, length: f64) -> Bone {
        Bone {
            name,
            parent,
            length,
            radius: 0.1,
        }
    }

    fn close(a: V3, b: V3) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() <= 1e-12)
    }

    #[test]
    fn one_bone_at_rest_points_along_facing() {
        let rig = Rig::new(vec![bone("arm", None, 1.0)]).unwrap();
        let mut out = Vec::new();
        solve(&rig, &Pose::rest(&rig), [0.0; 3], 0.6, 0.0, &mut out);
        assert_eq!(out.len(), 1);
        assert!(
            close(out[0].a, [0.6, 0.0, 0.0]) && close(out[0].b, [1.6, 0.0, 0.0]),
            "{:?}",
            out[0]
        );
        assert_eq!(out[0].radius, 0.1);
        assert_eq!(rig.bone("arm"), Some(0));
        assert_eq!(rig.bone("leg"), None);
    }

    #[test]
    fn child_adds_parent_yaw() {
        let rig = Rig::new(vec![bone("arm", None, 1.0), bone("fist", Some(0), 1.0)]).unwrap();
        let mut pose = Pose::rest(&rig);
        pose.joints[0].yaw = bf_core::dmath::PI / 2.0;
        let mut out = Vec::new();
        solve(&rig, &pose, [0.0; 3], 0.6, 0.0, &mut out);
        assert!(close(out[1].a, out[0].b));
        assert!(close(out[1].b, [0.6, 2.0, 0.0]), "{:?}", out[1]);
    }

    #[test]
    fn pitch_lifts() {
        let rig = Rig::new(vec![bone("arm", None, 0.7)]).unwrap();
        let mut pose = Pose::rest(&rig);
        pose.joints[0].pitch = bf_core::dmath::PI / 2.0;
        let mut out = Vec::new();
        solve(&rig, &pose, [1.0, 1.0, 2.0], 0.6, 1.0, &mut out);
        assert!((out[0].b[2] - (out[0].a[2] + 0.7)).abs() <= 1e-12);
        assert!((out[0].b[0] - out[0].a[0]).abs() <= 1e-12);
    }

    #[test]
    fn bones_must_be_parent_first() {
        assert!(Rig::new(vec![bone("fist", Some(1), 1.0), bone("arm", None, 1.0)]).is_err());
        assert!(Rig::new(vec![bone("arm", Some(0), 1.0)]).is_err());
        assert!(Rig::new(vec![]).is_ok());
    }
}
