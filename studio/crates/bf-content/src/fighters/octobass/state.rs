//! What OCTOBASS's look and its part both read from the recording: its lever punches, pedals, SEVEN LEVERS and
//! BELOW HEARING, as rows of its own events (`bf_sim::kinds::levers` writes them).

use bf_core::event::Event;

/// The times side `k`'s events of `kind` happened.
pub fn times(ev: &[Event], k: usize, kind: &str) -> Vec<f64> {
    ev.iter()
        .filter(|e| e.kind == kind && e.who() == Some(k))
        .map(|e| e.t)
        .collect()
}

/// `(t, x, z)` of every pedal side `k` set, `t` when it snaps.
pub fn pedals(ev: &[Event], k: usize) -> Vec<(f64, f64, f64)> {
    ev.iter()
        .filter(|e| e.kind == "pedal_set" && e.who() == Some(k))
        .map(|e| (e.f(5), e.f(3), e.f(4)))
        .collect()
}

/// `(start, end)` of every BELOW HEARING of side `k`: from its ring's start to its hit (or `t_end` when the fight
/// ended first).
pub fn silences(ev: &[Event], k: usize, t_end: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut open: Option<f64> = None;
    for e in ev.iter().filter(|e| e.who() == Some(k)) {
        match e.kind.as_str() {
            "below_start" => open = Some(e.t),
            "below_hit" => {
                if let Some(t0) = open.take() {
                    out.push((t0, e.t));
                }
            }
            _ => {}
        }
    }
    if let Some(t0) = open {
        out.push((t0, t_end.max(t0)));
    }
    out
}

/// `t` is inside one of the windows.
pub fn within(w: &[(f64, f64)], t: f64) -> bool {
    w.iter().any(|&(a, b)| a <= t && t < b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unfinished_silence_ends_with_the_fight() {
        assert!(silences(&[], 0, 9.0).is_empty());
        assert!(within(&[(1.0, 2.0)], 1.5) && !within(&[(1.0, 2.0)], 2.0));
    }
}
