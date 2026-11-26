//! Ability kind `joke` (VIOLA's VIOLA JOKE): `fight.py` `trigger` 2921 to 2922; 5118 to 5136 (`start_joke`: the stun is
//! the ball's `stun`, and the note it drops is the `rests` kind's `fire_note`).
//!
//! A rimshot and the crowd laughs: the enemy freezes for `stun_beats` beats (#37's stun state: it holds still and starts
//! no attack; not while shielded, rewinding or sunk in shadow), while the biggest whole note (`size` x the current note
//! radius, `dmg`, `knock`) drops from above onto its spot, landing `tele` beats later, then rolls on along the enemy's
//! old heading for a whole note. Events: `stun` (the victim, the fighter, its length), then the note's `note` and
//! `joke` (the fighter, the victim, when it lands, where). Nothing here draws from the generator.

use bf_core::event::Value;

use super::param;
use crate::fight::{Fight, rewinding};
use crate::geom::py_max;
use crate::kinds::rests;
use crate::roster::AbilityCfg;

/// This kind's per-fight state (`Fight::joke`): nothing beyond the ball's `stun` and the rests kind's notes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `trigger`, kind `joke` (2921 to 2922): `start_joke(k, ab)` (5118 to 5136).
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let o = 1 - k;
    let beat = f.rules.beat;
    let ob = &f.balls[o];
    if ob.shield <= 0.0 && !rewinding(f, o) && ob.shadow <= 0.0 {
        let st = param(ab, "stun_beats") * beat;
        f.balls[o].stun = py_max(f.balls[o].stun, st);
        f.ev("stun", o as i64, vec![Value::Int(k as i64), Value::Float(st)]);
    }
    rests::joke_note(f, k, |name| param(ab, name));
}
