//! What CONTRABASS CLARINET's look and its part both derive from the recording: its blasts, from the `horn` events.

use bf_core::event::{Event, Value};

/// One blast of the FOGHORN: when it started, its aim, reach, half-angle and length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Horn {
    /// When it started.
    pub t0: f64,
    /// Its aim, radians in the ring's x-y plane.
    pub aim: f64,
    /// Its reach beyond the balls' surfaces.
    pub reach: f64,
    /// Its half-angle, radians.
    pub half: f64,
    /// Seconds it is live.
    pub dur: f64,
}

fn f(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Float(x)) => Some(*x),
        Some(Value::Int(i)) => Some(*i as f64),
        _ => None,
    }
}

/// The blasts of fighter `k`, in time order.
pub fn horns(events: &[Event], k: usize) -> Vec<Horn> {
    events
        .iter()
        .filter(|e| e.kind == "horn" && e.who() == Some(k))
        .filter_map(|e| {
            Some(Horn {
                t0: e.t,
                aim: f(e.fields.get(1))?,
                reach: f(e.fields.get(2))?,
                half: f(e.fields.get(3))?,
                dur: f(e.fields.get(4))?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_horn_event_reads_back() {
        let e = Event {
            t: 2.5,
            kind: "horn".to_string(),
            fields: vec![
                Value::Int(1),
                Value::Float(0.5),
                Value::Float(1.6),
                Value::Float(0.4),
                Value::Float(0.55),
            ],
        };
        let h = horns(std::slice::from_ref(&e), 1);
        assert_eq!(h.len(), 1);
        assert_eq!((h[0].t0, h[0].aim, h[0].reach, h[0].dur), (2.5, 0.5, 1.6, 0.55));
        assert!(horns(&[e], 0).is_empty());
    }
}
