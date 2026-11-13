//! What TUBA's look derives from the recording's events: the ooms, the pahs, the times the enemy rides inside the
//! bell and SUBSONIC's slams (`Look.setup` in `kit/fighters/tuba.py`, lines 176 to 192). The Python part reads
//! nothing off its look (it walks the events itself), so only the look uses these.
//!
//! An event is Python's tuple `(t, kind, who, ...)`: `e.f(i)` is `e[i]`.

use bf_core::event::{Event, Value};

/// One "oom": an `oom` event (`fight.py` 1062) as `(t0, t_in, t_pah, reach)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oom {
    /// `e[0]`: when the bell went "oom".
    pub t0: f64,
    /// `e[3]`: when the inhale ends.
    pub t_in: f64,
    /// `e[4]`: when the "pah" is due.
    pub t_pah: f64,
    /// `e[5]`: the suction's reach.
    pub reach: f64,
}

/// One "pah": a `pah` event (`fight.py` 1117 and 1127) as `(t, caught, angle)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pah {
    /// `e[0]`: when it came.
    pub t: f64,
    /// `e[3]`: 1 when the enemy was spat out, 0 when nothing was caught.
    pub caught: i64,
    /// `e[4]`: the spit's direction, or the bell's aim.
    pub angle: f64,
}

/// One ride inside the bell: `(t_gulp, t_spit)` of `self.HELD` (the swallower is the look's own side).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Held {
    /// When the enemy was gulped.
    pub t_gulp: f64,
    /// When it was spat out: the first pah with a catch not before the gulp (less 1e-6), 99.0 when the fight
    /// ended first.
    pub t_spit: f64,
}

/// One SUBSONIC: `(t0, t_slam, cx, cz)` of `self.SUBS` (the radius is the roster's, the owner the look's side).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sub {
    /// When the ability was called.
    pub t0: f64,
    /// When the slam lands.
    pub t_slam: f64,
    /// Where it lands, x.
    pub cx: f64,
    /// Where it lands, z.
    pub cz: f64,
}

/// Fighter `k`'s events of one kind, in order.
fn mine<'a>(events: &'a [Event], kind: &'a str, k: usize) -> impl Iterator<Item = &'a Event> {
    events.iter().filter(move |e| e.kind == kind && e.who() == Some(k))
}

/// `self.OOMS`: fighter `k`'s ooms.
pub fn ooms(events: &[Event], k: usize) -> Vec<Oom> {
    mine(events, "oom", k)
        .map(|e| Oom {
            t0: e.t,
            t_in: e.f(3),
            t_pah: e.f(4),
            reach: e.f(5),
        })
        .collect()
}

/// `self.PAHS`: fighter `k`'s pahs.
pub fn pahs(events: &[Event], k: usize) -> Vec<Pah> {
    mine(events, "pah", k)
        .map(|e| Pah {
            t: e.t,
            caught: e.i(3),
            angle: e.f(4),
        })
        .collect()
}

/// `self.HELD`: for each of fighter `k`'s gulps, the gulp and the pah that spat the enemy out.
pub fn held(events: &[Event], k: usize, pahs: &[Pah]) -> Vec<Held> {
    mine(events, "gulp", k)
        .map(|g| {
            // n = [q for q in self.PAHS if q[0] >= g[0] - 1e-6 and q[2] == 1]
            let spit = pahs.iter().find(|q| q.t >= g.t - 1e-6 && q.caught == 1);
            Held {
                t_gulp: g.t,
                t_spit: spit.map_or(99.0, |q| q.t),
            }
        })
        .collect()
}

/// `self.SUBS`: for each `ability` event of fighter `k` whose kind is `slam`, the first of its `slam` events not
/// before it (an ability with no slam after it, cut off by the K.O., is left out).
pub fn subs(events: &[Event], k: usize) -> Vec<Sub> {
    mine(events, "ability", k)
        .filter(|e| matches!(e.fields.get(2), Some(Value::Str(kind)) if kind == "slam"))
        .filter_map(|e| {
            let sl = mine(events, "slam", k).find(|q| q.t >= e.t)?;
            Some(Sub {
                t0: e.t,
                t_slam: sl.t,
                cx: sl.f(3),
                cz: sl.f(4),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use Value::{Float as F, Int as I, Str as S};

    fn ev(t: f64, kind: &str, fields: Vec<Value>) -> Event {
        Event {
            t,
            kind: kind.into(),
            fields,
        }
    }

    #[test]
    fn a_gulp_is_held_until_the_next_pah_with_a_catch() {
        let events = vec![
            ev(10.0, "oom", vec![I(0), F(10.45), F(10.9), F(1.8)]),
            ev(10.2, "gulp", vec![I(0), F(0.1), F(0.2), F(1.92)]),
            ev(10.9, "pah", vec![I(0), I(1), F(0.5)]),
            ev(12.0, "oom", vec![I(0), F(12.45), F(12.9), F(1.92)]),
            ev(12.9, "pah", vec![I(0), I(0), F(0.7)]),
            ev(14.0, "oom", vec![I(1), F(14.45), F(14.9), F(1.8)]),
            ev(15.0, "gulp", vec![I(0), F(0.3), F(0.4), F(2.04)]),
        ];
        let o = ooms(&events, 0);
        assert_eq!(o.len(), 2);
        assert_eq!((o[1].t0, o[1].t_in, o[1].t_pah, o[1].reach), (12.0, 12.45, 12.9, 1.92));
        assert_eq!(ooms(&events, 1).len(), 1);
        let p = pahs(&events, 0);
        assert_eq!((p[0].caught, p[1].caught, p[1].angle), (1, 0, 0.7));
        let h = held(&events, 0, &p);
        // the second gulp was never spat out (the fight ended): 99.0
        assert_eq!(
            (h[0].t_gulp, h[0].t_spit, h[1].t_gulp, h[1].t_spit),
            (10.2, 10.9, 15.0, 99.0)
        );
    }

    #[test]
    fn a_subsonic_is_its_call_and_its_first_slam_after() {
        let events = vec![
            ev(
                20.0,
                "ability",
                vec![I(0), S("SUBSONIC".into()), S("slam".into()), I(1)],
            ),
            ev(20.5, "ability", vec![I(0), S("OOMPAH".into()), S("haste".into()), I(1)]),
            ev(20.9, "slam", vec![I(0), F(1.0), F(-0.5)]),
            ev(25.0, "slam", vec![I(1), F(0.0), F(0.0)]),
            ev(
                30.0,
                "ability",
                vec![I(0), S("SUBSONIC".into()), S("slam".into()), I(1)],
            ),
        ];
        let s = subs(&events, 0);
        assert_eq!(s.len(), 1, "the second call has no slam after it");
        assert_eq!((s[0].t0, s[0].t_slam, s[0].cx, s[0].cz), (20.0, 20.9, 1.0, -0.5));
    }
}
