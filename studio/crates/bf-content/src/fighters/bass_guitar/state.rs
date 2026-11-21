//! What BASS GUITAR's look and its part both read from the recording: its throws and its three moves, as rows of its
//! own events (`bf_sim::kinds::pick` writes them).

use bf_core::event::Event;

/// One throw of PICK TOSS (`pick`).
#[derive(Clone, Debug, PartialEq)]
pub struct Throw {
    pub t: f64,
    /// Picks thrown.
    pub n: i64,
    /// The throw's number.
    pub i: i64,
}

/// Side `k`'s throws, in order.
pub fn throws(ev: &[Event], k: usize) -> Vec<Throw> {
    ev.iter()
        .filter(|e| e.kind == "pick" && e.who() == Some(k))
        .map(|e| Throw {
            t: e.t,
            n: e.i(3),
            i: e.i(4),
        })
        .collect()
}

/// `(t, landed)` of side `k`'s events of `kind` (`fret_slide`, `headstock`, `slingshot`): false when it starts, true
/// when it lands.
pub fn moves(ev: &[Event], k: usize, kind: &str) -> Vec<(f64, bool)> {
    ev.iter()
        .filter(|e| e.kind == kind && e.who() == Some(k))
        .map(|e| (e.t, e.i(3) == 1))
        .collect()
}
