//! What CONTRABASS's look derives from the recording's events (`Look.setup` in `kit/fighters/contrabass.py`, lines
//! 95 to 105). The kit's part reads nothing off its look (it walks the events itself), so nothing here is shared with
//! `sound.rs`; it lives here as every port's look state does.
//!
//! An event is Python's tuple `(t, kind, who, ...)`: `e.f(i)` is `e[i]`.

use bf_core::event::Event;

/// One heavy step: a `step` event (`fight.py` 4016).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    /// `e[0]`: when it started.
    pub t0: f64,
    /// `e[7]`: when it lands.
    pub t1: f64,
    /// `(e[3], e[4])`: where it started.
    pub p0: [f64; 2],
    /// `(e[5], e[6])`: where it lands.
    pub p1: [f64; 2],
    /// `e[8]`: 1 under haste (DOUBLE TIME), else 0.
    pub kind: i64,
}

/// One footfall: a `step_land` event (`fight.py` 4039) as `(t, x, y, reach, kind)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Land {
    /// `e[0]`.
    pub t: f64,
    /// `e[3]`.
    pub x: f64,
    /// `e[4]`.
    pub y: f64,
    /// `e[5]`: the quake's reach.
    pub reach: f64,
    /// `e[6]`: 1 under haste.
    pub kind: i64,
}

/// Fighter `k`'s events of one kind, in order.
fn mine<'a>(events: &'a [Event], kind: &'a str, k: usize) -> impl Iterator<Item = &'a Event> {
    events.iter().filter(move |e| e.kind == kind && e.who() == Some(k))
}

/// `self.STEPS`: fighter `k`'s steps.
pub fn steps(events: &[Event], k: usize) -> Vec<Step> {
    mine(events, "step", k)
        .map(|e| Step {
            t0: e.t,
            t1: e.f(7),
            p0: [e.f(3), e.f(4)],
            p1: [e.f(5), e.f(6)],
            kind: e.i(8),
        })
        .collect()
}

/// `self.LAND`: fighter `k`'s footfalls.
pub fn lands(events: &[Event], k: usize) -> Vec<Land> {
    mine(events, "step_land", k)
        .map(|e| Land {
            t: e.t,
            x: e.f(3),
            y: e.f(4),
            reach: e.f(5),
            kind: e.i(6),
        })
        .collect()
}

/// `self.SLAPS`: `(t, broke)` of fighter `k`'s SLAP BASSes.
pub fn slaps(events: &[Event], k: usize) -> Vec<(f64, bool)> {
    mine(events, "slap", k).map(|e| (e.t, e.i(3) != 0)).collect()
}

/// `self.STAMP`: `(t0, t1)` of fighter `k`'s THE ELEPHANTs.
pub fn stamps(events: &[Event], k: usize) -> Vec<(f64, f64)> {
    mine(events, "stampede", k).map(|e| (e.t, e.f(3))).collect()
}

/// `self.STOMPS`: `(t, x, y, rmax)` of fighter `k`'s stomps.
pub fn stomps(events: &[Event], k: usize) -> Vec<(f64, f64, f64, f64)> {
    mine(events, "stampede_stomp", k)
        .map(|e| (e.t, e.f(3), e.f(4), e.f(5)))
        .collect()
}

/// `self.TRAMPLE`: `(t, x, y)` of fighter `k`'s tramples.
pub fn tramples(events: &[Event], k: usize) -> Vec<(f64, f64, f64)> {
    mine(events, "trample", k).map(|e| (e.t, e.f(3), e.f(4))).collect()
}

/// `int(np.searchsorted(a, x, 'right'))`: the number of values of the sorted `a` at or below `x` (a NaN `x` goes
/// past every number, as numpy sorts NaN last).
pub fn searchsorted_right(a: &[f64], x: f64) -> usize {
    if x.is_nan() {
        return a.iter().filter(|v| !v.is_nan()).count();
    }
    a.partition_point(|&v| v <= x)
}

#[cfg(test)]
mod tests {
    use bf_core::event::Value;

    use super::*;

    fn ev(t: f64, kind: &str, who: i64, data: &[Value]) -> Event {
        let mut fields = vec![Value::Int(who)];
        fields.extend(data.iter().cloned());
        Event {
            t,
            kind: kind.to_string(),
            fields,
        }
    }

    #[test]
    fn the_steps_and_footfalls_are_its_own_events_fields() {
        let f = Value::Float;
        let events = vec![
            ev(1.0, "step", 0, &[f(0.1), f(0.2), f(0.3), f(0.4), f(1.25), Value::Int(1)]),
            ev(1.25, "step_land", 0, &[f(0.3), f(0.4), f(0.71), Value::Int(1)]),
            ev(1.5, "step", 1, &[f(0.0), f(0.0), f(0.0), f(0.0), f(1.75), Value::Int(0)]),
            ev(2.0, "slap", 0, &[Value::Int(1)]),
            ev(3.0, "stampede", 0, &[f(5.7)]),
            ev(3.0, "stampede_stomp", 0, &[f(0.5), f(0.6), f(1.5)]),
            ev(3.1, "trample", 0, &[f(0.7), f(0.8)]),
        ];
        let s = steps(&events, 0);
        assert_eq!(s.len(), 1);
        assert_eq!((s[0].t0, s[0].t1, s[0].p0, s[0].p1, s[0].kind), (1.0, 1.25, [0.1, 0.2], [0.3, 0.4], 1));
        let l = lands(&events, 0);
        assert_eq!((l[0].t, l[0].x, l[0].y, l[0].reach, l[0].kind), (1.25, 0.3, 0.4, 0.71, 1));
        assert_eq!(slaps(&events, 0), vec![(2.0, true)]);
        assert_eq!(stamps(&events, 0), vec![(3.0, 5.7)]);
        assert_eq!(stomps(&events, 0), vec![(3.0, 0.5, 0.6, 1.5)]);
        assert_eq!(tramples(&events, 0), vec![(3.1, 0.7, 0.8)]);
        assert_eq!(steps(&events, 1).len(), 1);
    }

    #[test]
    fn searchsorted_right_counts_the_values_at_or_below() {
        let a = [1.0, 2.0, 2.0, 3.0];
        assert_eq!(
            [0.5, 1.0, 2.0, 2.5, 3.0, 9.0].map(|x| searchsorted_right(&a, x)),
            [0, 1, 3, 3, 4, 4]
        );
        assert_eq!(searchsorted_right(&a, f64::NAN), 4);
        assert_eq!(searchsorted_right(&[], 1.0), 0);
    }
}
