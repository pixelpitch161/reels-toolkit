//! What BASS CLARINET's look and its part both derive from the recording (the part recomputes its look's state from
//! the run, ruling M3-R15): the look's `ANCH`, its own pegs. The kit's part reads `M.P.LOOKS[k].ANCH`
//! (`bass_clarinet.py` 537 to 538); here both halves call [`anch`].

use bf_core::event::Event;

/// One of its pegs: a row of the look's `ANCH`, `(e[0], e[2], np.array([e[3], e[4]]), e[5], e[6])` of an `anchor`
/// event of its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anch {
    /// When the peg went in.
    pub t: f64,
    /// The peg.
    pub c: [f64; 2],
    /// The orbit the whirl eases to.
    pub orbit: f64,
    /// When the peg is pulled (the whirl's planned end).
    pub te: f64,
}

/// `[... for e in EV if e[1] == 'anchor' and e[2] == k]`: side `k`'s pegs, in the run's order.
pub fn anch(events: &[Event], k: usize) -> Vec<Anch> {
    events
        .iter()
        .filter(|e| e.kind == "anchor" && e.who() == Some(k))
        .map(|e| Anch {
            t: e.t,
            c: [e.f(3), e.f(4)],
            orbit: e.f(5),
            te: e.f(6),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bf_core::event::Value::{Float as F, Int as I};

    use super::*;

    #[test]
    fn the_pegs_are_its_own_anchor_events_in_order() {
        let ev = |t: f64, kind: &str, who: i64| Event {
            t,
            kind: kind.into(),
            fields: vec![I(who), F(0.5), F(-0.25), F(0.95), F(t + 0.9)],
        };
        let events = [
            ev(3.0, "anchor", 0),
            ev(4.0, "pull", 0),
            ev(5.0, "anchor", 1),
            ev(7.0, "anchor", 0),
        ];
        let got = anch(&events, 0);
        assert_eq!(got.len(), 2);
        assert_eq!(
            got[0],
            Anch {
                t: 3.0,
                c: [0.5, -0.25],
                orbit: 0.95,
                te: 3.9
            }
        );
        assert_eq!(got[1].t, 7.0);
        assert_eq!(anch(&events, 1).len(), 1);
    }
}
