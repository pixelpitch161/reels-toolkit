//! Fixtures for the unit tests: TEST DUMMY fights and hand-built recordings.

use bf2_sim::{Cue, Ev, EvKind, Outcome, Recording2, Rules2, Setup2, StepSnap, V3};

pub const DT: f64 = 1.0 / 120.0;

/// FIGHT! on bar 2 at 132 bpm (FLUTE vs TROMBONE's tempo, as E1 measured the dummy).
pub fn t_fight() -> f64 {
    bf_core::grid::Grid::new(132.0).t_fight
}

/// A TEST DUMMY mirror match set up as E1's measurement (`bf-engine/examples/engine2_speed.rs`): the
/// `fights/flute_vs_trombone.toml` tempo and handicap, so most seeds end in a K.O.
pub fn dummy(seed: u64, record: bool) -> Recording2 {
    let d = bf_content::fighters2::def("TEST DUMMY").expect("the dummy");
    let mut rules = Rules2::new(t_fight());
    rules.dmg_side = [2.0, 0.4];
    let setup = Setup2 {
        fighters: [d.clone(), d],
        rules,
        seed,
    };
    bf2_sim::run(&setup, record).expect("the dummy fights")
}

pub fn ev(step: u32, who: usize, kind: EvKind) -> Ev {
    Ev { step, who, kind }
}

pub fn slowmo(step: u32, who: usize, scale: f64) -> Ev {
    ev(step, who, EvKind::Cue(Cue::SlowMo { scale }))
}

pub fn recovery(step: u32, who: usize) -> Ev {
    ev(step, who, EvKind::Recovery { mv: 0 })
}

/// A step with both balls at these places, at rest.
pub fn snap(pos: [V3; 2]) -> StepSnap {
    StepSnap {
        pos,
        vel: [[0.0; 3]; 2],
        facing: [0.0, bf_core::dmath::PI],
        hp: [100.0, 100.0],
        mv: [None, None],
        joints: [Vec::new(), Vec::new()],
        hitstop: [0, 0],
        projs: Vec::new(),
    }
}

/// A recording of `n` steps whose ball positions come from `pos(step)`, with these events.
pub fn synthetic(n: u32, pos: impl Fn(u32) -> [V3; 2], mut events: Vec<Ev>, ko: Option<(usize, u32)>) -> Recording2 {
    events.sort_by_key(|e| e.step);
    let tf = t_fight();
    Recording2 {
        names: ["A".into(), "B".into()],
        seed: 3,
        dt: DT,
        t_fight: tf,
        steps: (0..n).map(|i| snap(pos(i))).collect(),
        events,
        outcome: Outcome {
            winner: ko.map(|(loser, _)| 1 - loser),
            t_ko: ko.map(|(_, k)| tf + k as f64 * DT),
            hp_end: [100.0, 100.0],
            hits: [0, 0],
            damage: [0.0, 0.0],
            by_decision: ko.is_none(),
        },
    }
}

/// Two balls standing 3 apart on the x axis.
pub fn standing(_: u32) -> [V3; 2] {
    [[-1.5, 0.0, 0.0], [1.5, 0.0, 0.0]]
}
