//! The recording of a fight: every step's state, the events and the outcome.

use crate::event::Ev;
use crate::rig::Joint;
use crate::v3::V3;
use crate::world::World;

/// The state after one step: enough to draw it (positions, facings, poses, projectiles) and to read it.
#[derive(Clone, Debug)]
pub struct StepSnap {
    pub pos: [V3; 2],
    pub vel: [V3; 2],
    pub facing: [f64; 2],
    pub hp: [f64; 2],
    /// (move, phase 0 to 2, step in the phase).
    pub mv: [Option<(u16, u8, u16)>; 2],
    pub joints: [Vec<Joint>; 2],
    pub hitstop: [u16; 2],
    /// (id, centre) of each live projectile.
    pub projs: Vec<(u32, V3)>,
}

impl StepSnap {
    pub fn of(w: &World) -> StepSnap {
        let mv = |who: usize| {
            w.bodies[who].mv.map(|ms| {
                let (phase, k) = ms.phase(&w.defs[who].moves[ms.def]);
                (ms.def as u16, phase.index(), k.min(u16::MAX as u32) as u16)
            })
        };
        let b = &w.bodies;
        StepSnap {
            pos: [b[0].pos, b[1].pos],
            vel: [b[0].vel, b[1].vel],
            facing: [b[0].facing, b[1].facing],
            hp: [b[0].hp, b[1].hp],
            mv: [mv(0), mv(1)],
            joints: [w.poses[0].joints.clone(), w.poses[1].joints.clone()],
            hitstop: [
                b[0].hitstop.min(u16::MAX as u32) as u16,
                b[1].hitstop.min(u16::MAX as u32) as u16,
            ],
            projs: w.projs.iter().filter(|p| p.alive).map(|p| (p.id, p.pos)).collect(),
        }
    }
}

/// How a fight ended.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub winner: Option<usize>,
    /// Video seconds of the K.O.: `t_fight + step·dt`.
    pub t_ko: Option<f64>,
    pub hp_end: [f64; 2],
    /// Hits landed (Contact or Block) by each fighter.
    pub hits: [u32; 2],
    /// Damage dealt by each fighter.
    pub damage: [f64; 2],
    /// No K.O.: decided on HP at `t_max` (or a draw).
    pub by_decision: bool,
}

/// A whole fight. Equality is bit-exact on every float.
#[derive(Clone, Debug)]
pub struct Recording2 {
    pub names: [String; 2],
    pub seed: u64,
    pub dt: f64,
    pub t_fight: f64,
    /// Every step's state; empty when the fight ran without recording.
    pub steps: Vec<StepSnap>,
    pub events: Vec<Ev>,
    pub outcome: Outcome,
}

fn f(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
}

fn fs(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| f(*x, *y))
}

fn v(a: &[V3], b: &[V3]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| fs(x, y))
}

fn joints(a: &[Joint], b: &[Joint]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| f(x.yaw, y.yaw) && f(x.pitch, y.pitch) && f(x.ext, y.ext))
}

impl PartialEq for StepSnap {
    fn eq(&self, o: &Self) -> bool {
        v(&self.pos, &o.pos)
            && v(&self.vel, &o.vel)
            && fs(&self.facing, &o.facing)
            && fs(&self.hp, &o.hp)
            && self.mv == o.mv
            && joints(&self.joints[0], &o.joints[0])
            && joints(&self.joints[1], &o.joints[1])
            && self.hitstop == o.hitstop
            && self.projs.len() == o.projs.len()
            && self
                .projs
                .iter()
                .zip(&o.projs)
                .all(|(a, b)| a.0 == b.0 && fs(&a.1, &b.1))
    }
}

impl PartialEq for Outcome {
    fn eq(&self, o: &Self) -> bool {
        let t = match (self.t_ko, o.t_ko) {
            (Some(a), Some(b)) => f(a, b),
            (None, None) => true,
            _ => false,
        };
        self.winner == o.winner
            && t
            && fs(&self.hp_end, &o.hp_end)
            && self.hits == o.hits
            && fs(&self.damage, &o.damage)
            && self.by_decision == o.by_decision
    }
}

/// Events hold floats deep in their kinds; Rust prints an `f64` with `{:?}` as the shortest text that reads back
/// to the same bits (and `-0.0` as such), so comparing the debug text is bit-exact for every non-NaN float.
fn events(a: &[Ev], b: &[Ev]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.step == y.step && x.who == y.who && format!("{:?}", x.kind) == format!("{:?}", y.kind))
}

impl PartialEq for Recording2 {
    fn eq(&self, o: &Self) -> bool {
        self.names == o.names
            && self.seed == o.seed
            && f(self.dt, o.dt)
            && f(self.t_fight, o.t_fight)
            && self.outcome == o.outcome
            && self.steps == o.steps
            && events(&self.events, &o.events)
    }
}
