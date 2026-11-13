//! BAGPIPES against its reference kit module (tools/studio_export/older/fighters/bagpipes.py, proven against #30 by
//! Gate O): every gate of Template F (studio/docs/porting-a-fighter.md, porting-an-older-fighter.md), the contacts
//! of every kit fighter against a swollen bag (plan m4 Table O4, Review Focus 1) and the copy sweep (Template F').

bf_harness::fighter_suite!("BAGPIPES");

use std::collections::BTreeMap;

use bf_core::event::Value;
use bf_core::recording::{Array, Recording};
use bf_harness::fight::{doc, same_event, setup_like, traced};
use bf_harness::refs::{cached_recording, list, text};

/// The scenario fights of the groups whose name starts with `prefix`: (group, cache file, kit recording).
fn scenario_fights(prefix: &str) -> Vec<(String, String, Recording)> {
    let d = doc(NAME);
    let mut out = Vec::new();
    for s in list(&d["scenarios"]) {
        let name = text(&s["name"]).to_string();
        if !name.starts_with(prefix) {
            continue;
        }
        for file in list(&s["cache"]) {
            out.push((name.clone(), text(file).to_string(), cached_recording(text(file))));
        }
    }
    out
}

/// A number of a recording's rules.
fn rule(rec: &Recording, key: &str) -> f64 {
    match rec.rules.get(key) {
        Some(Value::Float(f)) => *f,
        Some(Value::Int(n)) => *n as f64,
        x => panic!("{:?}: rules[{key:?}] is {x:?}", rec.names),
    }
}

/// `RUN['RAD'][i][k]`: ball `k`'s radius at recording frame `i` (the ragged list `RAD`, held to its last frame).
fn rad(rec: &Recording, i: usize, k: usize) -> f64 {
    let Some(Array::Nested(Value::List(frames))) = rec.arrays.get("RAD") else {
        panic!("{:?}: no RAD", rec.names)
    };
    match &frames[i.min(frames.len() - 1)] {
        Value::List(v) => match v[k] {
            Value::Float(f) => f,
            Value::Int(n) => n as f64,
            ref x => panic!("RAD entry {x:?}"),
        },
        x => panic!("RAD row {x:?}"),
    }
}

/// Table O4's link (a per-ball radius and mass, `move` 3165 to 3216 and every kind reading `getattr(O, 'rad', R)`):
/// BAGPIPES against each kit fighter but FLUTE (the template's own), three seeds with it left and three swapped (the
/// `with_<x>` groups). In every one the engine's events are the kit's, and against each kit fighter some hit of the
/// fighter's own lands while the bag is swollen (its radius above the shared one at the hit's frame): every kit
/// kind's contact meets a swollen ball. Ignored: reads the pinned cache; `prefix` proves the fights to the floor,
/// `scenarios_draw` draws them.
#[test]
#[ignore = "reads the pinned cache: python -m tools.studio_export cache fighters/bagpipes"]
fn contacts_against_a_swollen_bag() {
    let sim = bf_harness::sim!();
    let mut swollen: BTreeMap<String, usize> = BTreeMap::new();
    let fights = scenario_fights("with_");
    for (group, file, py) in &fights {
        let k = py.names.iter().position(|n| n == NAME).expect("a BAGPIPES fight");
        let enemy = py.names[1 - k].clone();
        let rs = traced(&sim, &setup_like(py), true, &format!("{NAME} {file}"));
        assert_eq!(rs.events.len(), py.events.len(), "{group} {file}: the number of events");
        for (a, b) in rs.events.iter().zip(&py.events) {
            assert!(same_event(a, b), "{group} {file}: the engine's {a:?}, the kit's {b:?}");
        }
        let (r, t_fight, fps) = (rad(py, 0, 1 - k), rule(py, "t_fight"), f64::from(py.fps));
        let n = py
            .events
            .iter()
            .filter(|e| e.kind == "hit" && e.who() == Some(1 - k) && e.i(3) == k as i64)
            .filter(|e| {
                let i = bf_core::py::round_half_even(((e.t - t_fight) * fps).max(0.0)) as usize;
                rad(py, i, k) > r
            })
            .count();
        *swollen.entry(enemy).or_default() += n;
    }
    assert_eq!(fights.len(), 19 * 6, "19 kit fighters, three seeds a side");
    let none: Vec<&String> = swollen.iter().filter(|(_, n)| **n == 0).map(|(e, _)| e).collect();
    assert!(none.is_empty(), "no hit of theirs on a swollen bag: {none:?} ({swollen:?})");
}

/// The copy sweep (Template F'): EUPHONIUM against BAGPIPES, three seeds each side at the early levels and low turn:
/// the engine gives the kit's `copy` events, and IDENTITY CRISIS copies at least one of BAGPIPES's copyable kinds
/// (shockwave, haste, wave). `scenarios_draw` draws every one of these fights with both real looks (EUPHONIUM's copy
/// through BAGPIPES's look, `setup_as`). Ignored: reads the pinned cache.
#[test]
#[ignore = "reads the pinned cache: python -m tools.studio_export cache fighters/bagpipes"]
fn copy_sweep() {
    let sim = bf_harness::sim!();
    let fights = scenario_fights("copy_sweep");
    assert_eq!(fights.len(), 6, "three seeds a side");
    let mut kinds = Vec::new();
    for (group, file, py) in &fights {
        let rs = traced(&sim, &setup_like(py), false, &format!("{NAME} {file}"));
        let (a, b): (Vec<_>, Vec<_>) = (
            rs.events.iter().filter(|e| e.kind == "copy").collect(),
            py.events.iter().filter(|e| e.kind == "copy").collect(),
        );
        assert_eq!(a.len(), b.len(), "{group} {file}: the engine's copies {a:?}, the kit's {b:?}");
        for (x, y) in a.iter().zip(&b) {
            assert!(same_event(x, y), "{group} {file}: the engine's {x:?}, the kit's {y:?}");
            kinds.push(y.s(4).to_string());
        }
    }
    assert!(
        kinds.iter().any(|k| ["shockwave", "haste", "wave"].contains(&k.as_str())),
        "IDENTITY CRISIS copied none of BAGPIPES's kinds: {kinds:?}"
    );
}
