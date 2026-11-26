//! What FLUGELHORN's look and its part both derive from the recording: its claps (`clap`), its FLANKING CALL swoops
//! (`flank`) and its TAKE WINGs (`take_wing`).

use bf_core::event::{Event, Value};

/// One WINGBEAT clap: when, its aim, reach and half-angle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clap {
    /// When the wings shut.
    pub t0: f64,
    /// Its aim, radians in the ring's x-y plane.
    pub aim: f64,
    /// Its reach beyond the balls' surfaces.
    pub reach: f64,
    /// Its half-angle, radians.
    pub half: f64,
}

/// One FLANKING CALL swoop: when it starts and when it ends at the latest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flank {
    /// When it starts.
    pub t0: f64,
    /// When it ends if it never gets there.
    pub t_end: f64,
}

/// One TAKE WING: when the wings spread, when the giant clap lands and its radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Take {
    /// When the wings spread.
    pub t0: f64,
    /// When the clap lands.
    pub t_clap: f64,
    /// Its radius.
    pub radius: f64,
}

fn f(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Float(x)) => Some(*x),
        Some(Value::Int(i)) => Some(*i as f64),
        _ => None,
    }
}

fn mine<'a>(events: &'a [Event], k: usize, kind: &'a str) -> impl Iterator<Item = &'a Event> {
    events.iter().filter(move |e| e.kind == kind && e.who() == Some(k))
}

/// The claps of fighter `k`, in time order.
pub fn claps(events: &[Event], k: usize) -> Vec<Clap> {
    mine(events, k, "clap")
        .filter_map(|e| {
            Some(Clap {
                t0: e.t,
                aim: f(e.fields.get(1))?,
                reach: f(e.fields.get(2))?,
                half: f(e.fields.get(3))?,
            })
        })
        .collect()
}

/// The FLANKING CALL swoops of fighter `k`, each ending at its own end or at its first `flank` hit.
pub fn flanks(events: &[Event], k: usize) -> Vec<Flank> {
    mine(events, k, "flank")
        .filter_map(|e| {
            let t_end = f(e.fields.get(2))?;
            let hit = events
                .iter()
                .filter(|h| h.t >= e.t && h.t <= t_end && h.kind == "hit" && h.who() == Some(k))
                .find(|h| matches!(h.fields.get(5), Some(Value::Str(s)) if s == "flank"))
                .map(|h| h.t);
            Some(Flank {
                t0: e.t,
                t_end: hit.unwrap_or(t_end),
            })
        })
        .collect()
}

/// The TAKE WINGs of fighter `k`.
pub fn takes(events: &[Event], k: usize) -> Vec<Take> {
    mine(events, k, "take_wing")
        .filter_map(|e| {
            Some(Take {
                t0: e.t,
                t_clap: f(e.fields.get(1))?,
                radius: f(e.fields.get(2))?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_events_read_back() {
        let ev = |t: f64, kind: &str, fields: Vec<Value>| Event {
            t,
            kind: kind.to_string(),
            fields,
        };
        let events = vec![
            ev(
                1.0,
                "clap",
                vec![Value::Int(1), Value::Float(0.5), Value::Float(0.9), Value::Float(1.0)],
            ),
            ev(2.0, "flank", vec![Value::Int(1), Value::Float(1.0), Value::Float(3.5)]),
            ev(
                2.5,
                "hit",
                vec![
                    Value::Int(1),
                    Value::Int(0),
                    Value::Float(3.0),
                    Value::Float(0.0),
                    Value::Float(0.0),
                    Value::Str("flank".into()),
                    Value::Int(0),
                ],
            ),
            ev(
                4.0,
                "take_wing",
                vec![Value::Int(1), Value::Float(4.5), Value::Float(3.2)],
            ),
        ];
        assert_eq!(claps(&events, 1)[0].reach, 0.9);
        assert!(claps(&events, 0).is_empty());
        assert_eq!(flanks(&events, 1)[0].t_end, 2.5);
        assert_eq!(takes(&events, 1)[0].t_clap, 4.5);
    }
}
