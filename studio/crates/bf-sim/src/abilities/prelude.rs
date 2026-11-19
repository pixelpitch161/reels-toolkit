//! Ability kind `prelude` (CELLO's PRELUDE): `fight.py` `trigger` 2948 to 2950; `prelude_step` 4220 to 4243.
//!
//! For `beats` beats from the trigger, on every 16th a note-wave ring of kind `prelude` leaves the fighter (`speed`,
//! `dmg`, `knock`, clearing enemy projectiles), out to `pattern[j % len]` beyond its surface. Each ring hits once. A
//! copied ability (IDENTITY CRISIS) runs this code for the copier's ball, so nothing here assumes which side has the
//! kind.

use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::{Fight, Wave};
use crate::roster::AbilityCfg;
use crate::roster::abilities::prelude::pattern;

/// `B.prel`: the prelude under way.
#[derive(Clone, Debug, PartialEq)]
pub struct Prel {
    /// When it fired.
    pub t0: f64,
    /// The next ring's number.
    pub j: i64,
    /// How many rings it fires (`int(round(beats * 4))`).
    pub n: i64,
    /// `ab['pattern']`.
    pub pattern: Vec<f64>,
    /// `ab['speed']`, `ab['dmg']`, `ab['knock']`.
    pub speed: f64,
    /// See `speed`.
    pub dmg: f64,
    /// See `speed`.
    pub knock: f64,
    /// `ab['name']`.
    pub name: String,
}

/// This ability's per-fight state (`Fight::prelude`): `B.prel`, indexed by fighter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `B.prel` (`None` when no prelude is under way).
    pub prel: [Option<Prel>; 2],
}

/// `trigger`, kind `prelude`: `trigger` 2948 to 2950. Event `prelude`: when it ends.
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let beats = param(ab, "beats");
    f.prelude.prel[k] = Some(Prel {
        t0: f.t,
        j: 0,
        // int(round(ab['beats'] * 4)): the roster keeps 4 x beats finite and within 2^53
        n: py::round_half_even(beats * 4.0) as i64,
        pattern: pattern(ab).to_vec(),
        speed: param(ab, "speed"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        name: ab.name.clone(),
    });
    let end = f.t + beats * f.rules.beat;
    f.ev("prelude", k as i64, vec![Value::Float(end)]);
}

/// `prelude_step(k)` (4226 to 4243), from `run()` 6177 to 6178: every ring whose 16th has come. Event
/// `prelude_note`: its number, where it starts, its final radius.
pub fn prelude_step(f: &mut Fight, k: usize) {
    let Some(mut pr) = f.prelude.prel[k].take() else {
        return;
    };
    let s16 = f.rules.beat / 4.0;
    let r = f.rules.r;
    while pr.j < pr.n && f.t + 1e-9 >= pr.t0 + pr.j as f64 * s16 {
        let j = pr.j;
        let len = pr.pattern.len() as i64;
        let rmax = r + pr.pattern[py::imod_py(j, len) as usize];
        let c = f.balls[k].p;
        f.waves.push(Wave {
            owner: k,
            c,
            r,
            speed: pr.speed,
            rmax,
            dmg: pr.dmg,
            knock: pr.knock,
            kind: "prelude".to_string(),
            hit: false,
            name: pr.name.clone(),
            t0: f.t,
            slow: None,
            stun: None,
            detune: None,
            grow: false,
            clears: true,
        });
        let data = vec![
            Value::Int(j),
            Value::Float(c[0]),
            Value::Float(c[1]),
            Value::Float(rmax),
        ];
        f.ev("prelude_note", k as i64, data);
        pr.j += 1;
    }
    if pr.j < pr.n {
        f.prelude.prel[k] = Some(pr);
    }
}
