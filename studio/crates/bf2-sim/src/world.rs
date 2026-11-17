//! The world of one fight.

use std::sync::Arc;

use bf_core::rng::Generator;

use crate::body::Body;
use crate::event::Events;
use crate::fighter::FighterDef;
use crate::proj::Proj;
use crate::rig::{Pose, Segment, solve};
use crate::rules::Rules2;

/// One fight's whole state.
#[derive(Clone, Debug)]
pub struct World {
    pub defs: [Arc<FighterDef>; 2],
    pub bodies: [Body; 2],
    pub poses: [Pose; 2],
    /// One segment per bone, in rig order.
    pub segments: [Vec<Segment>; 2],
    pub projs: Vec<Proj>,
    pub rules: Rules2,
    pub rng: Generator,
    pub events: Events,
    /// Per fighter, per hit of its current move: that window has landed (one window hits once).
    pub hit_mask: [Vec<bool>; 2],
    pub step: u32,
    /// The loser and the step of the K.O.
    pub ko: Option<(usize, u32)>,
    /// The next projectile id.
    pub next_proj: u32,
}

/// Step stage 4: each fighter's pose from its move (rest when it has none), then its segments.
#[allow(dead_code)] // until the step (Task 8)
pub(crate) fn solve_all(world: &mut World) {
    for who in 0..2 {
        let def = &world.defs[who];
        let body = &world.bodies[who];
        match body.mv {
            Some(ms) => ms.pose(&def.moves[ms.def], &def.rig, &mut world.poses[who]),
            None => world.poses[who].reset(&def.rig),
        }
        solve(
            &def.rig,
            &world.poses[who],
            body.centre(),
            body.r,
            body.facing,
            &mut world.segments[who],
        );
    }
}
