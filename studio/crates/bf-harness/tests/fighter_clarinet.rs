//! CLARINET against the Python kit: every gate of Template F (studio/docs/porting-a-fighter.md), and Task 10's own
//! test of SQUEAK's stun in the scenario fights.

use bf_core::event::Value;
use bf_core::recording::{Array, Recording};
use bf_harness::fight::{doc, setup_of, traced};
use bf_harness::refs::{cached_recording, list, text};

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
