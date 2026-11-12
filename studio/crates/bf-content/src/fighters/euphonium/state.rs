//! What EUPHONIUM's look and its part both derive from the recording (the part recomputes its look's state from the
//! run, ruling M3-R15): the look's `SALVOS` and `LANDS`, which the kit's part reads as `M.P.LOOKS[k].SALVOS` and
//! `.LANDS` (`euphonium.py` 538 to 553); here both halves call [`salvos`] and [`lands`].

use bf_core::event::Event;

/// One of its salvos: a row of the look's `SALVOS`, `(e[0], e[2], e[3], e[4])` of a `mortar` event of its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Salvo {
    /// When it was fired.
    pub t: f64,
    /// How many shells.
    pub n: i64,
    /// Its number.
    pub salvo: i64,
}

/// One of its shells landing: a row of the look's `LANDS`, `(e[0], e[2], e[3], e[4], e[5], e[6], e[7], e[8])` of a
/// `shell` event of its own.
#[derive(Clone, Debug, PartialEq)]
pub struct Land {
    /// When it landed.
    pub t: f64,
    /// Where it landed.
    pub x: f64,
    /// Where it landed.
    pub z: f64,
    /// 1 if it hit, else 0.
    pub hit: i64,
    /// Its salvo's number.
    pub salvo: i64,
    /// Its index in the salvo.
    pub j: i64,
    /// `mortar` or `barrage`.
    pub kind: String,
}

/// `[(e[0], e[2], e[3], e[4]) for e in EV if e[1] == 'mortar' and e[2] == k]`: side `k`'s salvos, in the run's order.
pub fn salvos(events: &[Event], k: usize) -> Vec<Salvo> {
    events
        .iter()
        .filter(|e| e.kind == "mortar" && e.who() == Some(k))
        .map(|e| Salvo {
            t: e.t,
            n: e.i(3),
            salvo: e.i(4),
        })
        .collect()
}

/// `[(e[0], e[2], ..., e[8]) for e in EV if e[1] == 'shell' and e[2] == k]`: side `k`'s landings, in the run's order.
pub fn lands(events: &[Event], k: usize) -> Vec<Land> {
    events
        .iter()
        .filter(|e| e.kind == "shell" && e.who() == Some(k))
        .map(|e| Land {
            t: e.t,
            x: e.f(3),
            z: e.f(4),
            hit: e.i(5),
            salvo: e.i(6),
            j: e.i(7),
            kind: e.s(8).to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bf_core::event::Value::{self, Float as F, Int as I, Str as S};

    use super::*;

    #[test]
    fn its_salvos_and_landings_are_its_own_events_in_order() {
        let ev = |t: f64, kind: &str, fields: Vec<Value>| Event {
            t,
            kind: kind.into(),
            fields,
        };
        let events = [
            ev(3.0, "mortar", vec![I(0), I(2), I(0), F(0.5), F(-0.5)]),
            ev(3.5, "mortar", vec![I(1), I(1), I(1), F(0.0), F(0.0)]),
            ev(
                4.0,
                "shell",
                vec![I(0), F(0.5), F(-0.25), I(1), I(0), I(0), S("mortar".into())],
            ),
            ev(
                4.1,
                "shell",
                vec![I(1), F(0.1), F(0.2), I(0), I(1), I(0), S("mortar".into())],
            ),
            ev(6.0, "mortar", vec![I(0), I(3), I(2), F(0.5), F(-0.5)]),
        ];
        assert_eq!(
            salvos(&events, 0),
            [Salvo { t: 3.0, n: 2, salvo: 0 }, Salvo { t: 6.0, n: 3, salvo: 2 }]
        );
        let l = lands(&events, 0);
        assert_eq!(l.len(), 1);
        assert_eq!(
            l[0],
            Land {
                t: 4.0,
                x: 0.5,
                z: -0.25,
                hit: 1,
                salvo: 0,
                j: 0,
                kind: "mortar".into()
            }
        );
        assert_eq!(lands(&events, 1)[0].hit, 0);
    }
}
