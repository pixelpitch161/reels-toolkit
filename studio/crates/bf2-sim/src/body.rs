//! A fighter's ball: where it is, how it moves and what state it is in.

use crate::moves::{Grab, MoveState};
use crate::v3::V3;

/// A grab under way, kept on the holder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Holding {
    pub target: usize,
    /// Steps left before the release.
    pub left: u32,
    pub grab: Grab,
}

/// A fighter's ball. `pos` is the bottom of the ball: `pos.z == 0` means it sits on the canvas, and the
/// sphere's centre is `centre()`, `r` above `pos`.
#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    pub pos: V3,
    pub vel: V3,
    /// Radians in the floor plane, from +x toward +y.
    pub facing: f64,
    pub r: f64,
    pub mass: f64,
    pub hp: f64,
    /// Steps left unable to act.
    pub hitstun: u32,
    /// Steps left frozen (freeze-on-hit).
    pub hitstop: u32,
    pub airborne: bool,
    /// Air hits taken since leaving the canvas.
    pub juggle: u32,
    /// The fighter holding this one.
    pub held_by: Option<usize>,
    pub mv: Option<MoveState>,
    /// Steps left before each move can start again.
    pub cooldowns: Vec<u32>,
    pub meter: f64,
    /// The low-HP move has fired.
    pub low_done: bool,
    /// The grab this fighter is holding.
    pub holding: Option<Holding>,
    /// Each move has started at least once (for `Ability { first }`).
    pub used: Vec<bool>,
}

impl Body {
    /// A ball at rest on the canvas, with `n_moves` moves.
    pub fn new(pos: V3, facing: f64, r: f64, mass: f64, hp: f64, n_moves: usize) -> Body {
        Body {
            pos,
            vel: [0.0; 3],
            facing,
            r,
            mass,
            hp,
            hitstun: 0,
            hitstop: 0,
            airborne: pos[2] > 0.0,
            juggle: 0,
            held_by: None,
            mv: None,
            cooldowns: vec![0; n_moves],
            meter: 0.0,
            low_done: false,
            holding: None,
            used: vec![false; n_moves],
        }
    }

    /// The sphere's centre.
    pub fn centre(&self) -> V3 {
        [self.pos[0], self.pos[1], self.pos[2] + self.r]
    }

    /// The step's countdowns: hitstop, then hitstun (which waits out a freeze) and the move cooldowns.
    pub fn countdown(&mut self) {
        if self.hitstop > 0 {
            self.hitstop -= 1;
        } else if self.hitstun > 0 {
            self.hitstun -= 1;
        }
        for c in &mut self.cooldowns {
            *c = c.saturating_sub(1);
        }
    }
}
