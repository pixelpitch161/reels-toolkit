//! EUPHONIUM against the Python kit: every gate of Template F (studio/docs/porting-a-fighter.md), and the copy sweep
//! of IDENTITY CRISIS (ruling M3-R3, the plan's Task 26).

bf_harness::fighter_suite!("EUPHONIUM");

use std::collections::{BTreeMap, BTreeSet};

use bf_core::recording::Recording;
use bf_harness::fight::{doc, same_event, setup_like, traced};
use bf_harness::refs::{cached_recording, list, recording_of, text};

/// The kinds IDENTITY CRISIS copies (`fight.py` 2802 `COPYABLE`; the engine's `abilities::copy::COPYABLE`).
const COPYABLE: [&str; 11] = [
    "shockwave",
    "wave",
    "shield",
    "haste",
    "arc",
    "slam",
    "snip",
    "twin",
    "lights_out",
    "barrage",
    "stun",
];

/// The copy table: for every other ported fighter, the copyable kinds it has (its roster entry's abilities whose
/// kind is in `COPYABLE`) and, for BASS CLARINET, its signature (`anchor`, the whirl). The copy sweep's scenarios must
/// show the euphonium copying each, in fights that agree with the kit's.
const COPY_TABLE: [(&str, &[&str]); 19] = [
    ("FLUTE", &["haste", "wave"]),
    ("TROMBONE", &["haste", "shockwave", "slam"]),
    ("PIANO", &["arc"]),
    ("TRUMPET", &["haste", "shockwave"]),
    ("ORGAN", &["haste", "shield", "slam"]),
    ("CLARINET", &["haste", "stun"]),
    ("OBOE", &["snip", "wave"]),
    ("SOUSAPHONE", &["haste"]),
    ("TENOR SAX", &[]),
    ("ALTO SAX", &[]),
    ("BASS CLARINET", &["anchor", "lights_out", "snip", "twin"]),
    ("TUBA", &["haste", "shield", "slam"]),
    ("CONTRABASS", &["haste"]),
    ("BASSOON", &["haste"]),
    ("HORN", &["haste", "shield"]),
    ("ACOUSTIC", &["haste"]),
    ("ELECTRIC", &[]),
    ("CELLO", &[]),
    ("BASS DRUM", &[]),
];

/// The `copy` events of side `k` in a recording: (time, name, kind).
fn copies(rec: &Recording, k: usize) -> Vec<(f64, String, String)> {
    rec.events
        .iter()
        .filter(|e| e.kind == "copy" && e.who() == Some(k))
        .map(|e| (e.t, e.s(3).to_string(), e.s(4).to_string()))
        .collect()
}

/// The table agrees with the roster: every fighter's copyable kinds are exactly the table's (but BASS CLARINET's
/// signature), and the table names every other fighter of the kit roster.
#[test]
fn the_copy_table_is_the_rosters() {
    for (name, kinds) in COPY_TABLE {
        let f = bf_harness::fight::fighter(name);
        let mut have: BTreeSet<&str> = [&f.unlock1, &f.unlock2, &f.low]
            .iter()
            .map(|ab| ab.kind.name())
            .filter(|k| COPYABLE.contains(k))
            .collect();
        if f.weapon.kind() == "anchor" {
            have.insert("anchor");
        }
        let want: BTreeSet<&str> = kinds.iter().copied().collect();
        assert_eq!(have, want, "{name}");
    }
    assert_eq!(COPY_TABLE.len(), 19);
    assert!(COPY_TABLE.iter().all(|(n, _)| *n != NAME));
}

/// The copy sweep: every scenario fight against another fighter (the `copy_*` groups, the template's FLUTE fights
/// and `vs_trombone`, and the left and right fights), run by the engine, gives the kit's `copy` events; the kinds the
/// euphonium copies against each fighter cover the copy table; against FLUTE the copies are haste and wave (and
/// FLUTE's signature, the striker). Ignored: reads the pinned cache. The fights themselves are proven by `prefix`,
/// their drawing by `scenarios_draw`.
#[test]
#[ignore = "reads the pinned cache: python -m tools.studio_export cache fighters/euphonium"]
fn copy_sweep() {
    let sim = bf_harness::sim!();
    let d = doc(NAME);
    let mut fights: Vec<(String, Recording)> = Vec::new();
    let left = &d["left"];
    fights.push(("left".into(), recording_of(left, Some(text(&left["hash"])))));
    fights.push(("right".into(), cached_recording(text(&d["right"]["cache"]))));
    for s in list(&d["scenarios"]) {
        for file in list(&s["cache"]) {
            fights.push((text(file).to_string(), cached_recording(text(file))));
        }
    }
    let mut seen: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut n_copies = 0;
    for (what, py) in &fights {
        let k = py.names.iter().position(|n| n == NAME).expect("the euphonium fights");
        let enemy = py.names[1 - k].clone();
        let setup = setup_like(py);
        let rs = traced(&sim, &setup, false, &format!("{NAME} {what}"));
        let (a, b): (Vec<_>, Vec<_>) = (
            rs.events.iter().filter(|e| e.kind == "copy").collect(),
            py.events.iter().filter(|e| e.kind == "copy").collect(),
        );
        assert_eq!(
            a.len(),
            b.len(),
            "{what}: the engine's copy events {a:?}, the kit's {b:?}"
        );
        for (x, y) in a.iter().zip(&b) {
            assert!(same_event(x, y), "{what}: the engine's {x:?}, the kit's {y:?}");
        }
        for (_, _, kind) in copies(py, k) {
            n_copies += 1;
            seen.entry(enemy.clone()).or_default().insert(kind);
        }
    }
    assert_eq!(
        (fights.len(), n_copies),
        (52, 134),
        "the sweep's fights and the kit's copy events in them"
    );
    let mut rows = Vec::new();
    for (name, kinds) in COPY_TABLE {
        let got = seen.get(name).cloned().unwrap_or_default();
        for kind in kinds {
            assert!(
                got.contains(*kind),
                "{name}: no fight of the sweep copies {kind} (it copies {got:?})"
            );
        }
        assert!(!got.is_empty(), "{name}: no fight of the sweep has a copy at all");
        rows.push(format!(
            "{name}: {}",
            got.iter().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    let flute: BTreeSet<&str> = seen["FLUTE"]
        .iter()
        .map(String::as_str)
        .filter(|k| COPYABLE.contains(k))
        .collect();
    assert_eq!(
        flute,
        BTreeSet::from(["haste", "wave"]),
        "against FLUTE the copies are haste and wave"
    );
    println!(
        "{NAME}: {} fights, {n_copies} copies agree with the kit's; copied kinds by opponent:\n  {}",
        fights.len(),
        rows.join("\n  ")
    );
}
