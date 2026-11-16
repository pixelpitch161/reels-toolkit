//! What PICCOLO's look and its part both read from the recording: its shots, bounces, pierces and moves, as rows of
//! its own events (`bf_sim::kinds::needle` writes them).

use bf_core::event::Event;

/// One needle fired (`needle`).
#[derive(Clone, Debug, PartialEq)]
pub struct Shot {
    pub t: f64,
    pub id: i64,
    pub x: f64,
    pub z: f64,
    /// A GLASS SHATTER shard (else PIERCING NOTE's needle).
    pub shard: bool,
}

/// One wall bounce (`needle_bounce`).
#[derive(Clone, Debug, PartialEq)]
pub struct Bounce {
    pub t: f64,
    pub id: i64,
    /// Bounces so far, 1 for the first.
    pub b: i64,
    pub x: f64,
    pub z: f64,
}

/// A needle passing through the enemy (`needle_hit`).
#[derive(Clone, Debug, PartialEq)]
pub struct Pierce {
    pub t: f64,
    pub id: i64,
    pub x: f64,
    pub z: f64,
}

/// Side `k`'s shots, in order.
pub fn shots(ev: &[Event], k: usize) -> Vec<Shot> {
    ev.iter()
        .filter(|e| e.kind == "needle" && e.who() == Some(k))
        .map(|e| Shot {
            t: e.t,
            id: e.i(3),
            x: e.f(4),
            z: e.f(5),
            shard: e.s(8) == "shard",
        })
        .collect()
}

/// Side `k`'s bounces, in order.
pub fn bounces(ev: &[Event], k: usize) -> Vec<Bounce> {
    ev.iter()
        .filter(|e| e.kind == "needle_bounce" && e.who() == Some(k))
        .map(|e| Bounce {
            t: e.t,
            id: e.i(3),
            b: e.i(4),
            x: e.f(5),
            z: e.f(6),
        })
        .collect()
}

/// Side `k`'s needles passing through the enemy, in order.
pub fn pierces(ev: &[Event], k: usize) -> Vec<Pierce> {
    ev.iter()
        .filter(|e| e.kind == "needle_hit" && e.who() == Some(k))
        .map(|e| Pierce {
            t: e.t,
            id: e.i(3),
            x: e.f(4),
            z: e.f(5),
        })
        .collect()
}

/// `(t, until)` of side `k`'s OVER THE BAND.
pub fn over_band(ev: &[Event], k: usize) -> Vec<(f64, f64)> {
    ev.iter()
        .filter(|e| e.kind == "over_band" && e.who() == Some(k))
        .map(|e| (e.t, e.f(3)))
        .collect()
}

/// `(t, x, z)` of every shield side `k`'s needles cut through (`pierce`).
pub fn shield_cuts(ev: &[Event], k: usize) -> Vec<(f64, f64, f64)> {
    ev.iter()
        .filter(|e| e.kind == "pierce" && e.who() == Some(k))
        .map(|e| (e.t, e.f(3), e.f(4)))
        .collect()
}

/// When side `k`'s GLASS SHATTER fired.
pub fn shatter(ev: &[Event], k: usize) -> Option<f64> {
    ev.iter()
        .find(|e| e.kind == "shatter" && e.who() == Some(k))
        .map(|e| e.t)
}

/// When side `k`'s TRILL (its `haste` ability) fired.
pub fn trills(ev: &[Event], k: usize) -> Vec<f64> {
    ev.iter()
        .filter(|e| e.kind == "ability" && e.who() == Some(k) && e.s(4) == "haste")
        .map(|e| e.t)
        .collect()
}
