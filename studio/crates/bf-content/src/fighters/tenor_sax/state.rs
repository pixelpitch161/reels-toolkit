//! What TENOR SAX's look and its part both derive from the recording (ruling M3-R15): when a strut down WALK THE
//! BAR really ended.
//!
//! A `runway` event carries the strut's planned end. The strut ends earlier when the tenor fires STAGE DIVE in the
//! middle of it (`trigger` clears `B.strut` without a `strut_end` event), and its own `strut_end` falls on the
//! step after the planned time. Both the look (`setup`, `RUNWAY`) and the part (`play`, the honks of a `runway`
//! event) take the earliest of the planned end and the tenor's later `strut_end` and `dive` events.

use bf_core::event::Event;
use bf_core::py::py_min;

/// `min([t1] + [x[0] for x in EV if x[2] == k and x[1] in ('strut_end', 'dive') and x[0] >= from])`: the end of a
/// strut planned to end at `t1`, given fighter `k`'s `strut_end` and `dive` events at or after `from`. The look
/// asks from 1e-6 s before the runway's event, the part from the event's own time.
pub fn runway_end(events: &[Event], k: usize, t1: f64, from: f64) -> f64 {
    events
        .iter()
        .filter(|x| x.who() == Some(k) && (x.kind == "strut_end" || x.kind == "dive") && x.t >= from)
        .fold(t1, |least, x| py_min(least, x.t))
}

#[cfg(test)]
mod tests {
    use bf_core::event::Value;

    use super::*;

    fn ev(t: f64, kind: &str, who: i64) -> Event {
        Event {
            t,
            kind: kind.to_string(),
            fields: vec![Value::Int(who)],
        }
    }

    #[test]
    fn a_strut_ends_at_its_plan_or_at_the_first_end_or_dive_after_it_began() {
        let events = vec![
            ev(9.0, "strut_end", 1),
            ev(11.0, "dive", 0),
            ev(12.5, "dive", 1),
            ev(13.0, "strut_end", 1),
        ];
        // the plan, when nothing of its own comes before it
        assert_eq!(runway_end(&events, 1, 12.0, 10.0), 12.0);
        // its own dive cuts it short; the other fighter's does not
        assert_eq!(runway_end(&events, 1, 14.0, 10.0), 12.5);
        assert_eq!(runway_end(&events, 0, 14.0, 10.0), 11.0);
        // an end before the strut began is another strut's
        assert_eq!(runway_end(&events, 1, 9.5, 9.2), 9.5);
        assert_eq!(runway_end(&events, 1, 9.5, 9.0), 9.0);
    }
}
