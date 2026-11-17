//! Events v2: the typed list the director, effects, sound, captions and HUD read.

use crate::moves::Cue;
use crate::v3::V3;

/// What a ball bounced off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Floor,
    Rope,
}

/// One event: the step it happened on, whose it is (the attacker, mover or bouncer) and what.
#[derive(Clone, Debug, PartialEq)]
pub struct Ev {
    pub step: u32,
    pub who: usize,
    pub kind: EvKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EvKind {
    MoveStart {
        mv: usize,
    },
    Windup {
        mv: usize,
    },
    Active {
        mv: usize,
    },
    Recovery {
        mv: usize,
    },
    /// A hit landed. `normal` points from the contact into the target; `bones` are the attacker's bones that
    /// touched (empty for a projectile).
    Contact {
        mv: usize,
        target: usize,
        point: V3,
        normal: V3,
        strength: f64,
        damage: f64,
        bones: Vec<usize>,
    },
    Block {
        target: usize,
        point: V3,
    },
    Clash {
        other: usize,
        point: V3,
    },
    Launch {
        target: usize,
        up: f64,
    },
    Land,
    Bounce {
        what: Surface,
    },
    Slam {
        target: usize,
    },
    Grabbed {
        target: usize,
    },
    Released {
        target: usize,
    },
    Spawn {
        proj: usize,
    },
    Expire {
        proj: usize,
    },
    ProjHit {
        proj: usize,
        target: usize,
        point: V3,
    },
    Cue(Cue),
    Ability {
        mv: usize,
        first: bool,
    },
    LowHp,
    Ko {
        target: usize,
    },
}

/// The fight's events, in the order they happened.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Events(pub Vec<Ev>);

impl Events {
    pub fn push(&mut self, step: u32, who: usize, kind: EvKind) {
        self.0.push(Ev { step, who, kind });
    }
}
