//! What SNARE DRUM's look and its part both read from the recording: its strokes, flams, rimshots and buzz rolls, as
//! rows of its own events (`bf_sim::kinds::sticks` writes them).

use bf_core::event::Event;

/// One stroke of the paradiddle (`stick`).
#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub t: f64,
    /// The stroke's number.
    pub n: i64,
    /// 0 = left stick, 1 = right.
    pub side: i64,
    /// The first stroke of a group of four.
    pub accent: bool,
}

/// Side `k`'s strokes, in order.
pub fn strokes(ev: &[Event], k: usize) -> Vec<Stroke> {
    ev.iter()
        .filter(|e| e.kind == "stick" && e.who() == Some(k))
        .map(|e| Stroke {
            t: e.t,
            n: e.i(3),
            side: e.i(4),
            accent: e.i(5) != 0,
        })
        .collect()
}

/// `(t, until)` of side `k`'s events of `kind` (`flam`, `buzz_roll`).
pub fn windows(ev: &[Event], k: usize, kind: &str) -> Vec<(f64, f64)> {
    ev.iter()
        .filter(|e| e.kind == kind && e.who() == Some(k))
        .map(|e| (e.t, e.f(3)))
        .collect()
}

/// When side `k` armed RIMSHOT.
pub fn rim_armed(ev: &[Event], k: usize) -> Vec<f64> {
    ev.iter()
        .filter(|e| e.kind == "rimshot" && e.who() == Some(k) && e.i(3) == 0)
        .map(|e| e.t)
        .collect()
}

/// `(t, stun seconds)` of every rimshot side `k` landed.
pub fn rimshots(ev: &[Event], k: usize) -> Vec<(f64, f64)> {
    ev.iter()
        .filter(|e| e.kind == "rimshot" && e.who() == Some(k) && e.i(3) == 1)
        .map(|e| (e.t, e.f(4)))
        .collect()
}

/// `t` is inside one of the windows.
pub fn within(w: &[(f64, f64)], t: f64) -> bool {
    w.iter().any(|&(a, b)| a <= t && t < b)
}
