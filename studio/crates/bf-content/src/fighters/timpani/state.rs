//! What TIMPANI's look and its part both read from the recording: its hops, its kettles' booms and its moves, as rows
//! of its own events (`bf_sim::kinds::kettles` writes them).

use bf_core::event::{Event, Value};

/// One hop (`kettle_hop`).
#[derive(Clone, Debug, PartialEq)]
pub struct Hop {
    pub t: f64,
    pub from: i64,
    pub to: i64,
    /// When it lands.
    pub t1: f64,
}

/// One kettle booming (`kettle_boom`).
#[derive(Clone, Debug, PartialEq)]
pub struct Boom {
    pub t: f64,
    /// The kettle: 0 low, 1 high, 2 and 3 FULL SET's.
    pub kettle: i64,
    pub x: f64,
    pub z: f64,
    pub reach: f64,
}

/// Side `k`'s hops, in order.
pub fn hops(ev: &[Event], k: usize) -> Vec<Hop> {
    ev.iter()
        .filter(|e| e.kind == "kettle_hop" && e.who() == Some(k))
        .map(|e| Hop {
            t: e.t,
            from: e.i(3),
            to: e.i(4),
            t1: e.f(5),
        })
        .collect()
}

/// Side `k`'s booms, in order.
pub fn booms(ev: &[Event], k: usize) -> Vec<Boom> {
    ev.iter()
        .filter(|e| e.kind == "kettle_boom" && e.who() == Some(k))
        .map(|e| Boom {
            t: e.t,
            kettle: e.i(3),
            x: e.f(4),
            z: e.f(5),
            reach: e.f(6),
        })
        .collect()
}

/// `(t, until)` of side `k`'s PEDAL SLIDEs.
pub fn slides(ev: &[Event], k: usize) -> Vec<(f64, f64)> {
    ev.iter()
        .filter(|e| e.kind == "pedal_slide" && e.who() == Some(k))
        .map(|e| (e.t, e.f(3)))
        .collect()
}

/// `(t0, t1)` of side `k`'s hastes (CROSS-STICKING).
pub fn hastes(ev: &[Event], k: usize, beat: f64) -> Vec<(f64, f64)> {
    ev.iter()
        .filter(|e| {
            e.kind == "ability" && e.who() == Some(k) && e.fields.iter().any(|v| *v == Value::Str("haste".into()))
        })
        .map(|e| (e.t, e.t + 4.0 * beat))
        .collect()
}

/// When side `k`'s FULL SET came in.
pub fn full_set(ev: &[Event], k: usize) -> Option<f64> {
    ev.iter()
        .find(|e| e.kind == "full_set" && e.who() == Some(k))
        .map(|e| e.t)
}

/// The time of every hit of kind `how` by side `k`, with where it landed.
pub fn hits(ev: &[Event], k: usize, how: &str) -> Vec<(f64, f64, f64)> {
    ev.iter()
        .filter(|e| e.kind == "hit" && e.who() == Some(k) && matches!(e.fields.get(5), Some(Value::Str(s)) if s == how))
        .map(|e| (e.t, e.f(5), e.f(6)))
        .collect()
}
