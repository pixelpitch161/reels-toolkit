//! What the parity tests of the two weapons share: reading a Python recording, building the Rust fight of the same
//! fighters, rules and seed, and comparing values, events and per-frame arrays with a tolerance.
//!
//! Whole numbers, texts, kinds and times are compared exactly; floats to [`TOL`]. Every comparison returns the
//! largest float difference it saw, so a test can report how close the two engines really are.

// each test file uses its own part of this module
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use bf_core::event::{Event, Value};
use bf_core::recording::{Array, Recording};
use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{Rules, Setup};

/// The parity bar of a fight (the plan's global constraints): positions, hit points and events agree to this.
pub const TOL: f64 = 1e-6;

/// A file or folder under `studio/reference/fights/`.
pub fn reference(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference/fights")
        .join(rel)
}

/// A Python recording under `studio/reference/fights/`.
pub fn python_recording(rel: &str) -> Recording {
    Recording::read(&reference(rel)).unwrap_or_else(|e| panic!("{e}"))
}

/// The exported roster.
pub fn roster() -> Vec<FighterCfg> {
    parse_roster(include_str!("../../../bf-content/data/roster.json")).expect("the roster loads")
}

/// A number of a recording's rules.
pub fn rule(rec: &Recording, key: &str) -> f64 {
    match rec.rules.get(key) {
        Some(Value::Float(x)) => *x,
        Some(Value::Int(n)) => *n as f64,
        other => panic!("rules.{key} is {other:?}"),
    }
}

/// The same fight as a Python recording: its fighters in its order (found in `fighters`), its rules, its seed.
pub fn setup_like<'a>(py: &Recording, fighters: &'a [FighterCfg]) -> Setup<'a> {
    let find = |name: &str| {
        fighters
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("{name} is not among the fighters"))
    };
    let mut rules = Rules::new(rule(py, "bpm"), rule(py, "t_fight"));
    rules.apply(&py.rules).expect("the recording's rules apply");
    Setup {
        fighters: [find(&py.names[0]), find(&py.names[1])],
        rules,
        seed: py.seed,
    }
}

/// The largest float difference between two values of the same layout (lists of the same length, the same whole
/// numbers and texts); panics where the layouts differ or a float differs by more than [`TOL`].
pub fn compare_values(what: &str, a: &Value, b: &Value) -> f64 {
    match (a, b) {
        (Value::List(x), Value::List(y)) => {
            assert_eq!(x.len(), y.len(), "{what}: {a:?} vs Python's {b:?}");
            x.iter()
                .zip(y)
                .map(|(p, q)| compare_values(what, p, q))
                .fold(0.0, f64::max)
        }
        (Value::Float(x), Value::Float(y)) => {
            let d = (x - y).abs();
            assert!(d <= TOL, "{what}: {x} vs Python's {y} ({d:e} apart)");
            d
        }
        _ => {
            assert_eq!(a, b, "{what}");
            0.0
        }
    }
}

/// Compares two events: the kind and the time exactly, the fields as [`compare_values`] does. Returns the largest
/// float difference.
pub fn compare_event(what: &str, a: &Event, b: &Event) -> f64 {
    let what = format!("{what}:\n Rust   {a:?}\n Python {b:?}\n");
    assert_eq!((a.t.to_bits(), &a.kind), (b.t.to_bits(), &b.kind), "{what}");
    compare_values(&what, &Value::List(a.fields.clone()), &Value::List(b.fields.clone()))
}

/// An event's kind as the reports count it: `hit` and `block` with what hit (`hit/blade`, `hit/fang`,
/// `block/blade`, ...), every other kind as it is.
pub fn kind_of(e: &Event) -> String {
    match e.kind.as_str() {
        "hit" => format!("hit/{}", e.s(7)),
        "block" => format!("block/{}", e.s(6)),
        kind => kind.to_string(),
    }
}

/// The events of a list, counted by [`kind_of`].
pub fn count_kinds<'a>(events: impl IntoIterator<Item = &'a Event>) -> BTreeMap<String, usize> {
    let mut kinds = BTreeMap::new();
    for e in events {
        *kinds.entry(kind_of(e)).or_insert(0) += 1;
    }
    kinds
}

/// Compares the first `frames` frames of one per-frame array of two recordings: the same kind and shape, whole
/// numbers exactly, floats to [`TOL`] (two floats of the same bits are equal, so the NaN of an unused `K` slot
/// passes), ragged rows as [`compare_values`] does. Returns the largest float difference.
pub fn compare_array(what: &str, rs: &Array, py: &Array, frames: usize) -> f64 {
    match (rs, py) {
        (
            Array::F64 {
                shape: rs_shape,
                data: rs_data,
            },
            Array::F64 {
                shape: py_shape,
                data: py_data,
            },
        ) => {
            assert_eq!(&rs_shape[1..], &py_shape[1..], "{what}: the shape of a frame");
            assert!(
                rs_shape[0] >= frames && py_shape[0] >= frames,
                "{what}: {frames} frames are needed"
            );
            let per_frame: usize = py_shape[1..].iter().product();
            let mut worst = 0.0f64;
            for i in 0..frames * per_frame {
                let (a, b) = (rs_data[i], py_data[i]);
                if a.to_bits() == b.to_bits() {
                    continue;
                }
                let d = (a - b).abs();
                assert!(
                    d <= TOL,
                    "{what}: differs by {d:e} at frame {} (element {}): {a} vs Python's {b}",
                    i / per_frame,
                    i % per_frame
                );
                worst = worst.max(d);
            }
            worst
        }
        (
            Array::I64 {
                shape: rs_shape,
                data: rs_data,
            },
            Array::I64 {
                shape: py_shape,
                data: py_data,
            },
        ) => {
            assert_eq!(&rs_shape[1..], &py_shape[1..], "{what}: the shape of a frame");
            assert!(
                rs_shape[0] >= frames && py_shape[0] >= frames,
                "{what}: {frames} frames are needed"
            );
            let per_frame: usize = py_shape[1..].iter().product();
            for i in 0..frames * per_frame {
                assert_eq!(
                    rs_data[i],
                    py_data[i],
                    "{what}: frame {} (element {})",
                    i / per_frame,
                    i % per_frame
                );
            }
            0.0
        }
        (Array::Nested(Value::List(rs_rows)), Array::Nested(Value::List(py_rows))) => {
            assert!(
                rs_rows.len() >= frames && py_rows.len() >= frames,
                "{what}: {frames} frames are needed"
            );
            (0..frames)
                .map(|i| compare_values(&format!("{what} at frame {i}"), &rs_rows[i], &py_rows[i]))
                .fold(0.0, f64::max)
        }
        _ => panic!("{what}: not the same kind of array: {rs:?} vs Python's {py:?}"),
    }
}
