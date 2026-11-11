//! Every kind of thing a fight of FLUTE against TROMBONE can do, done by the Rust engine where Python did it.
//!
//! `studio/reference/fights/flute_vs_trombone/coverage.json` (the `scenarios` export) lists, for every event kind,
//! every hit sub-kind and every one of the six abilities, the first place it happens in the scenario recordings: the
//! recording, the position in its event list and the event itself. Each entry lies within `window` seconds (20) of
//! the start of its fight. This test runs the Rust fight of each entry's recording and finds the entry's event in it:
//! the same kind at the same time with the same fields (whole numbers and texts exactly, floats to 1e-6), at the same
//! position in the list, and inside the span where the Rust fight and the Python recording still agree.

mod support;

use std::collections::BTreeMap;

use bf_core::event::{Event, Value};
use bf_core::recording::Recording;
use bf_sim::parity::prefix_window;
use bf_sim::run;
use serde_json::Value as Json;
use support::{TOL, kind_of};

/// A value of the coverage table as a recording holds it: a number with a fraction or an exponent is a float, any
/// other number a whole number (`recording-format.md` section 2).
fn value(j: &Json) -> Value {
    match j {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => match n.as_i64() {
            Some(i) => Value::Int(i),
            None => Value::Float(n.as_f64().expect("a number")),
        },
        Json::String(s) => Value::Str(s.clone()),
        Json::Array(items) => Value::List(items.iter().map(value).collect()),
        Json::Object(_) => panic!("an event holds no table: {j}"),
    }
}

/// An event of the coverage table: `[t, kind, who, ...]`.
fn event(j: &Json) -> Event {
    let items = j.as_array().expect("an event is a list");
    Event {
        t: items[0].as_f64().expect("an event's time"),
        kind: items[1].as_str().expect("an event's kind").to_string(),
        fields: items[2..].iter().map(value).collect(),
    }
}

/// Whether two events are the same: kind, time, whole numbers and texts exactly, floats to 1e-6.
fn same(a: &Event, b: &Event) -> bool {
    fn close(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Float(x), Value::Float(y)) => (x - y).abs() <= TOL,
            (Value::List(x), Value::List(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| close(p, q)),
            _ => a == b,
        }
    }
    a.kind == b.kind
        && a.t.to_bits() == b.t.to_bits()
        && a.fields.len() == b.fields.len()
        && a.fields.iter().zip(&b.fields).all(|(p, q)| close(p, q))
}

/// The item an event stands for in the table: its kind, `hit/<what hit>`, `ability/<name>`.
fn items_of(e: &Event) -> Vec<String> {
    match e.kind.as_str() {
        "hit" => vec!["hit".to_string(), kind_of(e)],
        "ability" => vec!["ability".to_string(), format!("ability/{}", e.s(3))],
        kind => vec![kind.to_string()],
    }
}

/// Brief, step 1: every entry of `coverage.json` occurs in the Rust run of its scenario at the same time with the
/// same fields.
#[test]
fn every_kind_fires_inside_the_window() {
    let path = support::reference("flute_vs_trombone/coverage.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let doc: Json = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let window = doc["window"].as_f64().expect("the window in seconds");
    let entries = doc["entries"].as_array().expect("the entries");
    let required: Vec<&str> = doc["required"]
        .as_array()
        .expect("the required items")
        .iter()
        .map(|j| j.as_str().expect("an item"))
        .collect();
    assert_eq!(window, 20.0);

    // the Rust fight of each recording an entry names, run once
    let roster = support::roster();
    let mut fights: BTreeMap<String, (Recording, Recording, f64)> = BTreeMap::new();
    let mut missing = Vec::new();
    let mut covered = Vec::new();
    for entry in entries {
        let item = entry["item"].as_str().expect("an item");
        let file = entry["file"].as_str().expect("a file");
        let index = entry["index"].as_u64().expect("an index") as usize;
        let want = event(&entry["event"]);
        let rel = file.strip_prefix("fights/").expect("a file under fights/");
        let (py, rs, agree_until) = fights.entry(file.to_string()).or_insert_with(|| {
            let py = support::python_recording(rel);
            let rs = run(&support::setup_like(&py, &roster), true);
            let agree_until = prefix_window(&rs, &py).agree_until_s;
            (py, rs, agree_until)
        });

        // the table against the recording it describes
        assert_eq!(
            py.events[index], want,
            "{item}: {file} event {index} is not the table's"
        );
        assert!(
            items_of(&want).iter().any(|i| i == item),
            "{item}: the event is {want:?}"
        );
        assert_eq!(
            py.hash(),
            doc["hashes"][rel.rsplit('/').next().unwrap().trim_end_matches(".bfrec")]
                .as_str()
                .expect("the recording's hash"),
            "{file}"
        );
        let after_fight = entry["after_fight"].as_f64().expect("seconds after the start");
        assert!(after_fight <= window, "{item}: {after_fight} s after the start");
        // it is the first of its item in that recording
        let first = py.events.iter().position(|e| items_of(e).iter().any(|i| i == item));
        assert_eq!(first, Some(index), "{item}: the first in {file}");

        // the Rust fight: the same event at the same place, and the first of its item there too
        let rs_first = rs.events.iter().position(|e| items_of(e).iter().any(|i| i == item));
        let found = rs.events.get(index).is_some_and(|e| same(e, &want));
        if !found || rs_first != Some(index) || want.t >= *agree_until {
            let near: Vec<String> = rs
                .events
                .iter()
                .enumerate()
                .filter(|(_, e)| (e.t - want.t).abs() < 0.05)
                .map(|(i, e)| format!("\n     {i}: {e:?}"))
                .collect();
            missing.push(format!(
                "{item}: {file} event {index}\n   Python: {want:?}\n   Rust event {index}: {:?}\n   the first {item} \
                 in the Rust fight is event {rs_first:?}; the two fights agree until {agree_until} s\n   Rust events \
                 within 0.05 s:{}",
                rs.events.get(index),
                near.concat()
            ));
        } else {
            covered.push(item);
        }
    }
    assert!(
        missing.is_empty(),
        "{} of {} entries do not occur in the Rust fight as in Python's:\n\n{}",
        missing.len(),
        entries.len(),
        missing.join("\n\n")
    );
    // every required item has its entry, and the list is the one the format document gives
    for item in &required {
        assert!(covered.contains(item), "no entry covers {item}");
    }
    assert_eq!(covered.len(), 23);
    for item in [
        "ability/FLUTTER TONGUE",
        "ability/SNAKE DANCE",
        "ability/PICCOLO",
        "ability/SAD TROMBONE",
        "ability/POWER SLIDE",
        "ability/PEDAL TONE",
        "hit/blade",
        "hit/fang",
        "hit/shockwave",
        "hit/wave",
        "hit/slam",
        "slow",
        "slam",
        "swat",
        "ko",
    ] {
        assert!(required.contains(&item), "{item} is not required");
    }
    println!(
        "{} entries of {} recordings occur in the Rust fights: {covered:?}",
        covered.len(),
        fights.len()
    );
}
