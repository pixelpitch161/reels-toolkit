//! CLARINET against the Python kit: every gate of Template F (studio/docs/porting-a-fighter.md), and Task 10's own
//! test of SQUEAK's stun in the scenario fights.

use bf_core::event::Value;
use bf_core::recording::{Array, Recording};
use bf_harness::fight::{doc, setup_of, traced};
use bf_harness::refs::{cached_recording, committed_recording, fighter_folder, index, json, list, num, report, text};
use bf_harness::sound::{compare, fighter_soundtrack};

bf_harness::fighter_suite!("CLARINET");

/// The frames of `P`: fighter `k`'s spot in frame `i`.
fn spot(rec: &Recording, i: usize, k: usize) -> [f64; 2] {
    match rec.arrays.get("P") {
        Some(Array::F64 { data, .. }) => [data[i * 4 + k * 2], data[i * 4 + k * 2 + 1]],
        other => panic!("P is {other:?}"),
    }
}

/// The stuns of a fight: `(time, victim, seconds)` of every `stun` event.
fn stuns(rec: &Recording) -> Vec<(f64, usize, f64)> {
    let int = |v: &Value| match v {
        Value::Int(n) => *n as usize,
        other => panic!("{other:?}"),
    };
    let float = |v: &Value| match v {
        Value::Float(x) => *x,
        other => panic!("{other:?}"),
    };
    rec.events
        .iter()
        .filter(|e| e.kind == "stun")
        .map(|e| (e.t, int(&e.fields[0]), float(&e.fields[2])))
        .collect()
}

/// Task 10's extra test, on the scenario fights where the unlocks come after a few hits (`early_levels`,
/// `early_low`: SQUEAK lands inside the first seconds): while a FLUTE is stunned it drops no basket and starts no
/// strike, and between two recorded frames it only moves when a clash pushed the two balls apart. The Rust fight is
/// run from the scenario's seed and rules; the kit's recording of the same fight must show the same stuns (the
/// `prefix` gate compares the fights themselves).
#[test]
#[ignore = "reads the pinned cache: python -m tools.studio_export cache fighters/<slug>"]
fn a_stunned_flute_starts_no_strike_and_does_not_move() {
    let doc = doc(NAME);
    let (mut fights, mut windows, mut still, mut pushed) = (0, 0, 0, 0);
    for s in list(&doc["scenarios"]) {
        let scenario = text(&s["name"]);
        if !scenario.starts_with("early_l") {
            continue;
        }
        for (seed, file) in list(&s["seeds"]).iter().zip(list(&s["cache"])) {
            let seed = seed.as_u64().expect("a seed");
            let what = format!("{NAME} {scenario} seed {seed}");
            let setup = setup_of(&s["fighters"], &s["rules"], seed);
            let dt = setup.rules.dt;
            let t_fight = setup.rules.t_fight;
            let rs = traced(&bf_harness::sim!(), &setup, true, &what);
            let py = cached_recording(text(file));
            let flute = rs.names.iter().position(|n| n == "FLUTE").expect("FLUTE fights");
            fights += 1;
            // the kit's fight has the same stuns over the prefix floor (10 s of fighting)
            let early = |r: &Recording| -> Vec<(f64, usize, f64)> {
                stuns(r).into_iter().filter(|x| x.0 < t_fight + 10.0).collect()
            };
            assert_eq!(early(&rs), early(&py), "{what}: the stuns of the first 10 s");
            let end = rs.t_ko.unwrap_or(f64::INFINITY);
            for (t0, victim, dur) in stuns(&rs) {
                if victim != flute {
                    continue;
                }
                windows += 1;
                assert_eq!(dur, 0.5, "{what}: SQUEAK's stun");
                // the stun runs down by a step a step from the step after the hit: stunned until t0 + dur
                let t1 = (t0 + dur).min(end);
                for e in &rs.events {
                    let starts = (e.kind == "strike" || e.kind == "basket") && e.fields[0] == Value::Int(flute as i64);
                    assert!(
                        !(starts && e.t > t0 + dt / 2.0 && e.t < t1 - dt),
                        "{what}: a stunned FLUTE (from {t0} for {dur} s) started something: {e:?}"
                    );
                }
                // frame i is the state at t_fight + i * 4 * dt
                let frame_t = |i: usize| t_fight + i as f64 * 4.0 * dt;
                for i in 1..rs.frames() {
                    let (a, b) = (frame_t(i - 1), frame_t(i));
                    if a < t0 + dt || b > t1 - dt {
                        continue;
                    }
                    let clash = rs
                        .events
                        .iter()
                        .any(|e| e.kind == "clash" && e.t > a - dt && e.t < b + dt);
                    if clash {
                        pushed += 1;
                    } else {
                        assert_eq!(
                            spot(&rs, i, flute),
                            spot(&rs, i - 1, flute),
                            "{what}: the stunned FLUTE moved between frames {} and {i} with no clash",
                            i - 1
                        );
                        still += 1;
                    }
                }
            }
        }
    }
    assert!(
        fights >= 6 && windows >= 6 && still >= 40,
        "{fights} fights, {windows} stuns of the FLUTE, {still} frame steps held still: too few to prove anything"
    );
    println!(
        "{NAME}: {windows} stuns of the FLUTE in {fights} scenario fights: no basket, no strike, {still} frame steps \
         still ({pushed} with a clash left out)"
    );
}

/// The suite's `sound` gate without its minimum of one first-time note: the same comparisons (`sound::compare`: the
/// 16-bit master, the buses, the stems, every 100 ms block of every bus, the note log, the noise stream's end) of
/// the same soundtrack (the `left` fight, CLARINET's real part, the kit's base `Sound` for FLUTE).
///
/// The suite's gate requires `sound.json`'s `left.notes.count` to be above 0, and CLARINET's is 0 by the kit's own
/// design: the note log lists the first-time notes of `kit.music.note` (the Pixel Pitch voices through
/// `pp_bridge.voice`), and `clarinet.py`'s `Sound` never calls it. It overrides `voice` and makes every note with
/// `clarinet60_lowpoly.reed` through a cache of its own (`clar`). So that gate cannot pass for this fighter as
/// written (reported to the controller), and this test holds the part to the kit with another minimum: the kit's
/// clarinet bus sounds in at least 150 of its 100 ms blocks.
#[test]
fn sound_through_its_own_reed() {
    let folder = fighter_folder(NAME);
    let doc = json(&folder, "sound.json");
    let fd = doc_of_fight();
    let rec = committed_recording(text(&fd["left"]["file"]), Some(text(&fd["left"]["hash"])));
    let left = &doc["left"];
    assert_eq!(
        index(&left["notes"]["count"]),
        0,
        "the kit's CLARINET part now plays Pixel Pitch notes: the suite's `sound` gate covers it"
    );
    let loud = list(&left["rms"]["clarinet"]).iter().filter(|b| num(b) > -60.0).count();
    assert!(
        loud >= 150,
        "{folder}: the kit's clarinet bus sounds in {loud} blocks: too few to compare"
    );
    let s = fighter_soundtrack(NAME, &rec, false, None);
    let mut failures = Vec::new();
    let n = compare("left", &s, left, &mut failures);
    assert!(
        doc["roles"].as_object().expect("roles").is_empty(),
        "a role variant to compare too"
    );
    report(&folder, "sound", "sound", "soundtrack facts", failures, n);
}

/// CLARINET's `fight.json`.
fn doc_of_fight() -> serde_json::Value {
    doc(NAME)
}
