//! VIOLIN's quick port (ruling M4-R16): its spinning bow fights four fighters to the end, from both corners, and the
//! bow spins up to `spin_max` as it lands.

use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{Rules, Setup, run};

fn roster() -> Vec<FighterCfg> {
    parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads")
}

#[test]
fn violin_fights_to_the_end() {
    let all = roster();
    let get = |name: &str| all.iter().find(|f| f.name == name).expect("in roster.json");
    let violin = get("VIOLIN");
    let (mut fights, mut kos) = (0, 0);
    for enemy in ["TROMBONE", "CELLO", "FLUTE", "PIANO"] {
        let other = get(enemy);
        for k in 0..2 {
            for seed in 1..=4 {
                let mut fighters = [other, other];
                fighters[k] = violin;
                let setup = Setup {
                    fighters,
                    rules: Rules::new(132.0, 3.5),
                    seed,
                };
                let rec = run(&setup, true).expect("runs");
                fights += 1;
                let blade_hits = rec
                    .events
                    .iter()
                    .filter(|e| {
                        e.kind == "hit"
                            && e.who() == Some(k)
                            && matches!(e.fields.get(5), Some(bf_core::event::Value::Str(s)) if s == "blade")
                    })
                    .count();
                assert!(
                    blade_hits > 0,
                    "VIOLIN against {enemy}, corner {k}, seed {seed}: the bow never landed"
                );
                if rec.winner.is_some() {
                    kos += 1;
                }
            }
        }
    }
    assert!(kos * 4 >= fights * 3, "{kos} K.O.s in {fights} fights");
}

#[test]
fn the_bow_spins_up_to_its_cap() {
    let all = roster();
    let violin = all.iter().find(|f| f.name == "VIOLIN").expect("VIOLIN");
    let tuba = all.iter().find(|f| f.name == "TUBA").expect("TUBA");
    let setup = Setup {
        fighters: [violin, tuba],
        rules: Rules::new(132.0, 3.5),
        seed: 3,
    };
    let rec = run(&setup, true).expect("runs");
    let om = match rec.arrays.get("OM") {
        Some(bf_core::recording::Array::F64 { data, .. }) => data.iter().step_by(2).copied().fold(0.0, f64::max),
        other => panic!("no OM: {other:?}"),
    };
    assert!(om > 2.4, "the bow never sped up ({om})");
    assert!(om <= 14.0 + 1e-9, "the bow passed spin_max ({om})");
}

/// Code review of 2026-11-23, F1: a TUBA on the left that gulps the VIOLIN puts it exactly on its own centre before
/// the bow's step, so the bow once divided 0 by 0 and every TUBA (left) v VIOLIN (right) fight went NaN with no
/// K.O. Each fight must now stay finite and end in a K.O.
#[test]
fn a_gulped_violin_keeps_the_fight_finite() {
    let all = roster();
    let violin = all.iter().find(|f| f.name == "VIOLIN").expect("VIOLIN");
    let tuba = all.iter().find(|f| f.name == "TUBA").expect("TUBA");
    for (seed, bpm) in [(1, 174.0), (2, 120.0), (3, 84.0)] {
        let setup = Setup {
            fighters: [tuba, violin],
            rules: Rules::new(bpm, 3.5),
            seed,
        };
        let rec = run(&setup, true).unwrap_or_else(|e| panic!("TUBA v VIOLIN seed {seed} at {bpm} bpm: {e}"));
        let Some(bf_core::recording::Array::F64 { data, .. }) = rec.arrays.get("P") else {
            panic!("no positions")
        };
        assert!(
            data.iter().all(|x| x.is_finite()),
            "seed {seed}: a position is not finite"
        );
        assert!(rec.winner.is_some(), "seed {seed} at {bpm} bpm: no K.O.");
    }
}
