//! What SOUSAPHONE's look and its part both derive from the recording's events: the drill sets with their blasts
//! and COMPANY FRONT's walls (`Look.setup` in `kit/fighters/sousaphone.py`, lines 90 to 107). The Python part reads
//! them off its look (`M.P.LOOKS[k].SETS`, `.FRONT`); here the part recomputes them from the events (ruling M3-R15).
//!
//! An event is Python's tuple `(t, kind, who, ...)`: `e.f(i)` is `e[i]`.

use bf_core::event::Event;

/// One drill set: a `drill` event (`fight.py` 4856), and the blast that ended it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Set {
    /// `e[0]`: when the set started.
    pub t0: f64,
    /// `(e[3], e[4])`: where it started.
    pub p0: [f64; 2],
    /// `(e[5], e[6])`: where it halts.
    pub p1: [f64; 2],
    /// `e[8]`: the time of its last footfall.
    pub t1: f64,
    /// `e[9]`: its steps.
    pub n: i64,
    /// `e[10]`: 1 when it started under haste.
    pub kind: i64,
    /// The time of its blast: the first blast not before its last footfall, when that comes within a second of
    /// it; `None` for a set that was called off.
    pub t_blast: Option<f64>,
}

impl Set {
    /// `S_['t_blast'] or S_['t1']`: when the set ends (Python's `or`: a blast time of 0.0 counts as none).
    pub fn end(&self) -> f64 {
        match self.t_blast {
            Some(t) if t != 0.0 => t,
            _ => self.t1,
        }
    }
}

/// One bell blast: a `blast` event (`fight.py` 4866).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blast {
    /// `e[0]`: when it fired.
    pub t0: f64,
    /// `e[3]`: the bell's x.
    pub ox: f64,
    /// `e[4]`: the bell's y.
    pub oy: f64,
    /// `e[5]`: the cone's angle.
    pub a: f64,
    /// `e[6]`: its length.
    pub ln: f64,
    /// `e[7]`: its width at the bell.
    pub w0: f64,
    /// `e[8]`: its width at the far end.
    pub w1: f64,
}

/// One COMPANY FRONT: a `company_front` event (`fight.py` 5004) as `(e[0], e[3], e[4], e[6])`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Front {
    /// `e[0]`: when the wall started.
    pub tf: f64,
    /// `e[3]`: the axis it marches along, 0 or 1.
    pub ax: i64,
    /// `e[4]`: the way it marches.
    pub sg: f64,
    /// `e[6]`: when it ends.
    pub t1: f64,
}

/// Fighter `k`'s events of one kind, in order.
fn mine<'a>(events: &'a [Event], kind: &'a str, k: usize) -> impl Iterator<Item = &'a Event> {
    events.iter().filter(move |e| e.kind == kind && e.who() == Some(k))
}

/// `self.BLASTS`: fighter `k`'s blasts.
pub fn blasts(events: &[Event], k: usize) -> Vec<Blast> {
    mine(events, "blast", k)
        .map(|e| Blast {
            t0: e.t,
            ox: e.f(3),
            oy: e.f(4),
            a: e.f(5),
            ln: e.f(6),
            w0: e.f(7),
            w1: e.f(8),
        })
        .collect()
}

/// `self.SETS`: fighter `k`'s drill sets, each with its blast (the first blast at or after its last footfall, less
/// 1e-6; kept only when it comes less than a second after it).
pub fn sets(events: &[Event], k: usize) -> Vec<Set> {
    let blasts = blasts(events, k);
    mine(events, "drill", k)
        .map(|e| {
            let t1 = e.f(8);
            let t_blast = blasts
                .iter()
                .find(|b| b.t0 >= t1 - 1e-6)
                .and_then(|b| (b.t0 < t1 + 1.0).then_some(b.t0));
            Set {
                t0: e.t,
                p0: [e.f(3), e.f(4)],
                p1: [e.f(5), e.f(6)],
                t1,
                n: e.i(9),
                kind: e.i(10),
                t_blast,
            }
        })
        .collect()
}

/// `self.FRONT`: fighter `k`'s COMPANY FRONTs.
pub fn fronts(events: &[Event], k: usize) -> Vec<Front> {
    mine(events, "company_front", k)
        .map(|e| Front {
            tf: e.t,
            ax: e.i(3),
            sg: e.f(4),
            t1: e.f(6),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bf_core::event::Value;

    use super::*;

    fn ev(t: f64, kind: &str, who: i64, data: &[f64]) -> Event {
        let mut fields = vec![Value::Int(who)];
        fields.extend(data.iter().map(|&x| Value::Float(x)));
        Event {
            t,
            kind: kind.to_string(),
            fields,
        }
    }

    fn drill(t: f64, who: i64, t1: f64) -> Event {
        let mut e = ev(t, "drill", who, &[0.0, 0.0, 1.0, 0.0, t, t1]);
        e.fields.extend([Value::Int(8), Value::Int(0)]);
        e
    }

    #[test]
    fn a_set_takes_the_first_blast_after_its_last_footfall_within_a_second() {
        let blast = |t: f64| ev(t, "blast", 1, &[0.0, 0.0, 0.5, 2.0, 0.5, 1.6]);
        let events = vec![
            drill(10.0, 1, 11.8),
            blast(12.0),
            drill(14.0, 1, 15.8),
            // called off: the next blast is the third set's, more than a second later
            drill(18.0, 1, 19.8),
            blast(20.0),
            // the other fighter's are not its own
            drill(21.0, 0, 22.0),
        ];
        let s = sets(&events, 1);
        assert_eq!(s.len(), 3);
        assert_eq!(
            (s[0].t_blast, s[1].t_blast, s[2].t_blast),
            (Some(12.0), None, Some(20.0))
        );
        assert_eq!((s[0].end(), s[1].end()), (12.0, 15.8));
        assert_eq!((s[0].n, s[0].kind, s[0].p1), (8, 0, [1.0, 0.0]));
        assert_eq!(blasts(&events, 1).len(), 2);
        assert!(sets(&events, 0)[0].t_blast.is_none());
    }

    #[test]
    fn a_front_is_its_events_axis_way_and_end() {
        let mut e = ev(30.0, "company_front", 0, &[]);
        e.fields.extend([
            Value::Int(1),
            Value::Float(-1.0),
            Value::Float(30.0),
            Value::Float(33.6),
        ]);
        let f = fronts(&[e], 0);
        assert_eq!(
            f,
            vec![Front {
                tf: 30.0,
                ax: 1,
                sg: -1.0,
                t1: 33.6
            }]
        );
    }
}
