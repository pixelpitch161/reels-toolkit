//! Projectiles: launched by a move's spawn, they fly, home, bounce on the ropes, hit, and always expire with an
//! event.

pub use crate::moves::ProjDef;
use crate::v3::V3;

/// A live (or just expired) projectile.
#[derive(Clone, Debug, PartialEq)]
pub struct Proj {
    /// (fighter, move, spawn) of its `ProjDef`.
    pub def: (usize, usize, usize),
    pub owner: usize,
    /// The centre.
    pub pos: V3,
    pub vel: V3,
    pub age: u32,
    pub bounces: u32,
    pub alive: bool,
    /// Unique within the fight, in spawn order.
    pub id: u32,
}
