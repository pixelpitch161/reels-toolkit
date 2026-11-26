//! What BASS DRUM's look and its part both derive from the recording's events: its rolls, BOOMs, heartbeats,
//! cannons and DRUM ROLLs (`Look.setup` in `kit/fighters/bass_drum.py`, lines 109 to 135). The Python part reads the
//! rolls, the DRUM ROLLs and the cannons off its look (`M.P.LOOKS[k].SEGS`, `.DROLLS`, `.CANNONS`, lines 500 and 554);
//! here the part recomputes them from the events (ruling M3-R15).
//!
//! An event is Python's tuple `(t, kind, who, ...)`: `e.f(i)` is `e[i]`.

use bf_core::event::Event;
use bf_core::py::py_min;

/// One roll: a `roll` event (`fight.py` 5339) and the `roll_end` or `roll_off` that ended it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seg {
    /// `e[0]`: when it tipped (the event's time).
    pub t_tip: f64,
    /// `e[8]`: when it started rolling.
    pub t0: f64,
    /// `e[9]`: when it was to reach its line's end.
    pub t1: f64,
    /// When it ended: the next `roll_end` or `roll_off` of the fighter, no later than `t1`.
    pub t_end: f64,
    /// `e[10]`: 0 the weapon's roll, 1 a DRUM ROLL leg.
    pub kind: i64,
    /// `(e[3], e[4])`: where the line starts.
    pub p0: [f64; 2],
    /// `(e[5], e[6])`: where it ends.
    pub p1: [f64; 2],
}

/// Fighter `k`'s events of one kind, in order.
fn mine<'a>(events: &'a [Event], kind: &'a str, k: usize) -> impl Iterator<Item = &'a Event> {
    events.iter().filter(move |e| e.kind == kind && e.who() == Some(k))
}

/// `self.SEGS`: every roll of fighter `k`, in order.
pub fn segs(events: &[Event], k: usize) -> Vec<Seg> {
    let mut out = Vec::new();
    for (i, e) in events.iter().enumerate() {
        if e.kind != "roll" || e.who() != Some(k) {
            continue;
        }
        let t1 = e.f(9);
        let t_end = events[i + 1..]
            .iter()
            .find(|f| f.who() == Some(k) && matches!(f.kind.as_str(), "roll_end" | "roll_off"))
            .map_or(t1, |f| f.t);
        out.push(Seg {
            t_tip: e.t,
            t0: e.f(8),
            t1,
            t_end: py_min(t_end, t1),
            kind: e.i(10),
            p0: [e.f(3), e.f(4)],
            p1: [e.f(5), e.f(6)],
        });
    }
    out
}

/// `self.BOOMS`: `(t, x, y, kind)` of fighter `k`'s BOOMs (`roll_boom`).
pub fn booms(events: &[Event], k: usize) -> Vec<(f64, f64, f64, i64)> {
    mine(events, "roll_boom", k)
        .map(|e| (e.t, e.f(3), e.f(4), e.i(5)))
        .collect()
}

/// `self.BEATS`: `(t, i, x, y)` of fighter `k`'s HEARTBEAT thumps (`heartbeat`).
pub fn beats(events: &[Event], k: usize) -> Vec<(f64, i64, f64, f64)> {
    mine(events, "heartbeat", k)
        .map(|e| (e.t, e.i(3), e.f(4), e.f(5)))
        .collect()
}

/// `self.CANNONS`: `(t, i, x, y, t_land)` of fighter `k`'s cannons (`cannon`).
pub fn cannons(events: &[Event], k: usize) -> Vec<(f64, i64, f64, f64, f64)> {
    mine(events, "cannon", k)
        .map(|e| (e.t, e.i(3), e.f(4), e.f(5), e.f(6)))
        .collect()
}

/// `self.DROLLS`: `(start, end)` of fighter `k`'s DRUM ROLLs: the end is the next `drum_roll_end` of the fighter, or
/// the end the `drum_roll` event planned.
pub fn drolls(events: &[Event], k: usize) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for (i, e) in events.iter().enumerate() {
        if e.kind != "drum_roll" || e.who() != Some(k) {
            continue;
        }
        let t_end = events[i + 1..]
            .iter()
            .find(|f| f.kind == "drum_roll_end" && f.who() == Some(k))
            .map_or(e.f(3), |f| f.t);
        out.push((e.t, t_end));
    }
    out
}

#[cfg(test)]
mod tests {
    use bf_core::event::Value;

    use super::*;

    fn ev(t: f64, kind: &str, who: i64, rest: &[Value]) -> Event {
        let mut fields = vec![Value::Int(who)];
        fields.extend_from_slice(rest);
        Event {
            t,
            kind: kind.to_string(),
            fields,
        }
    }

    #[test]
    fn a_roll_ends_at_its_own_end_or_off_and_never_after_its_plan() {
        let f = Value::Float;
        let roll = |t: f64| {
            let rest = [
                f(0.0),
                f(0.0),
                f(1.0),
                f(0.0),
                f(t),
                f(t + 0.2),
                f(t + 1.0),
                Value::Int(0),
            ];
            rest.to_vec()
        };
        let events = vec![
            ev(1.0, "roll", 0, &roll(1.0)),
            ev(1.5, "roll_end", 1, &[f(0.0), f(0.0)]),
            ev(1.6, "roll_off", 0, &[Value::Str("held".into())]),
            ev(3.0, "roll", 0, &roll(3.0)),
            ev(4.5, "roll_end", 0, &[f(0.0), f(0.0)]),
            ev(5.0, "drum_roll", 0, &[f(7.0)]),
            ev(6.0, "drum_roll_end", 0, &[]),
            ev(8.0, "drum_roll", 0, &[f(10.0)]),
        ];
        let s = segs(&events, 0);
        assert_eq!(s.len(), 2);
        // the other fighter's roll_end is not this roll's end
        assert_eq!((s[0].t_tip, s[0].t0, s[0].t1, s[0].t_end), (1.0, 1.2, 2.0, 1.6));
        // an end after the plan is the plan
        assert_eq!(s[1].t_end, 4.0);
        assert_eq!(drolls(&events, 0), [(5.0, 6.0), (8.0, 10.0)]);
        assert!(segs(&events, 1).is_empty());
    }
}
