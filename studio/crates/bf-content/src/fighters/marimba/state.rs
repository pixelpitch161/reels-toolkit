//! What MARIMBA's look and its part both read from the recording: its throws, bounces, ends, hits and moves, as rows
//! of its own events (`bf_sim::kinds::mallets` writes them).

use bf_core::event::Event;

/// One mallet thrown (`mallet`).
#[derive(Clone, Debug, PartialEq)]
pub struct Throw {
    pub t: f64,
    pub id: i64,
    pub x: f64,
    pub z: f64,
    /// FIVE OCTAVES' (else MALLET RUN's).
    pub octave: bool,
}

/// One wall bounce (`mallet_bounce`).
#[derive(Clone, Debug, PartialEq)]
pub struct Bounce {
    pub t: f64,
    pub id: i64,
    /// Bounces so far, 1 for the first.
    pub b: i64,
    pub x: f64,
    pub z: f64,
}

/// A mallet gone (`mallet_end`): it landed on the enemy or fizzled.
#[derive(Clone, Debug, PartialEq)]
pub struct End {
    pub t: f64,
    pub id: i64,
    pub hit: bool,
    pub b: i64,
    pub x: f64,
    pub z: f64,
}

/// Side `k`'s throws, in order.
pub fn throws(ev: &[Event], k: usize) -> Vec<Throw> {
    ev.iter()
        .filter(|e| e.kind == "mallet" && e.who() == Some(k))
        .map(|e| Throw {
            t: e.t,
            id: e.i(3),
            x: e.f(4),
            z: e.f(5),
            octave: e.s(8) == "octave",
        })
        .collect()
}

/// Side `k`'s bounces, in order.
pub fn bounces(ev: &[Event], k: usize) -> Vec<Bounce> {
    ev.iter()
        .filter(|e| e.kind == "mallet_bounce" && e.who() == Some(k))
        .map(|e| Bounce {
            t: e.t,
            id: e.i(3),
            b: e.i(4),
            x: e.f(5),
            z: e.f(6),
        })
        .collect()
}

/// Side `k`'s ends, in order.
pub fn ends(ev: &[Event], k: usize) -> Vec<End> {
    ev.iter()
        .filter(|e| e.kind == "mallet_end" && e.who() == Some(k))
        .map(|e| End {
            t: e.t,
            id: e.i(3),
            hit: e.s(4) == "hit",
            b: e.i(5),
            x: e.f(6),
            z: e.f(7),
        })
        .collect()
}

/// `(t, until)` of side `k`'s events of `kind` (`tremolo`, `dead_stroke`).
pub fn windows(ev: &[Event], k: usize, kind: &str) -> Vec<(f64, f64)> {
    ev.iter()
        .filter(|e| e.kind == kind && e.who() == Some(k))
        .map(|e| (e.t, e.f(3)))
        .collect()
}

/// `(t, x, z)` of every pin by side `k` (`pinned`: the pinned fighter is the enemy), where the enemy stood is read
/// by the caller.
pub fn pins(ev: &[Event], k: usize) -> Vec<(f64, f64)> {
    ev.iter()
        .filter(|e| e.kind == "pinned" && e.i(3) == k as i64)
        .map(|e| (e.t, e.f(4)))
        .collect()
}

/// When side `k`'s FIVE OCTAVES fired.
pub fn five_octaves(ev: &[Event], k: usize) -> Option<f64> {
    ev.iter()
        .find(|e| e.kind == "five_octaves" && e.who() == Some(k))
        .map(|e| e.t)
}
