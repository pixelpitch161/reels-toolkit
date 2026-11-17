//! Fixtures for the tests: hand-made recordings and a one-bone rig.

use bf2_sim::{Bone, Joint, Outcome, Recording2, Rig, StepSnap, V3};

pub const T_FIGHT: f64 = 4.5;
pub const DT: f64 = 1.0 / 120.0;

/// One bone, `arm`, of length `len`.
pub fn arm(len: f64) -> Rig {
    Rig::new(vec![Bone {
        name: "arm",
        parent: None,
        length: len,
        radius: 0.1,
    }])
    .unwrap()
}

pub fn snap(pos: [V3; 2], yaw: [f64; 2]) -> StepSnap {
    StepSnap {
        pos,
        vel: [[0.0; 3]; 2],
        facing: [0.0, std::f64::consts::PI],
        hp: [100.0, 100.0],
        mv: [None, None],
        joints: [
            vec![Joint {
                yaw: yaw[0],
                ..Joint::REST
            }],
            vec![Joint {
                yaw: yaw[1],
                ..Joint::REST
            }],
        ],
        hitstop: [0, 0],
        projs: vec![],
    }
}

pub fn recording(steps: Vec<StepSnap>) -> Recording2 {
    Recording2 {
        names: ["A".into(), "B".into()],
        seed: 1,
        dt: DT,
        t_fight: T_FIGHT,
        steps,
        events: vec![],
        outcome: Outcome {
            winner: None,
            t_ko: None,
            hp_end: [100.0, 100.0],
            hits: [0, 0],
            damage: [0.0, 0.0],
            by_decision: true,
        },
    }
}

/// Fighter 0 at the origin swings its arm's yaw from 0 to `sweep` over `n` steps, then holds; fighter 1 stands
/// at (2, 0). `extra` more steps follow.
pub fn swing(n: usize, sweep: f64, extra: usize) -> Recording2 {
    let steps = (0..=n + extra)
        .map(|i| {
            let yaw = sweep * (i.min(n) as f64 / n as f64);
            snap([[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]], [yaw, 0.0])
        })
        .collect();
    recording(steps)
}

pub fn step_to_t(s: u32) -> f64 {
    T_FIGHT + f64::from(s) * DT
}
