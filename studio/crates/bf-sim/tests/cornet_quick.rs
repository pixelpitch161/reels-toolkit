//! CORNET (a from-scratch fighter, 2026-11-16): PLUNGER MUTE, WA-WA, DERBY HAT and STOP-TIME fight a spread of
//! fighters to the end from both corners. The cups stick and wah, the hats land and muffle (the muffled enemy keeps
//! moving), STOP-TIME dashes and breaks, and a lenient balance check: CORNET neither always wins nor always loses.

use bf_core::event::Event;
use bf_core::recording::Array;
use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{Rules, Setup, run};

fn roster() -> Vec<FighterCfg> {
    let mut all = parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads");
    all.extend(parse_roster(include_str!("../../bf-content/data/roster_new.json")).expect("the new roster loads"));
    all
}

const ENEMIES: [&str; 8] = [
    "TRUMPET",
    "TUBA",
    "FLUTE",
    "BARI SAX",
    "EUPHONIUM",
    "CLARINET",
    "VIOLIN",
    "CORNET",
];

#[test]
fn cornet_fights_to_the_end() {
    let all = roster();
    let get = |name: &str| all.iter().find(|f| f.name == name).expect("in the roster");
    let cornet = get("CORNET");
    let (mut fights, mut kos, mut wins) = (0, 0, 0);
    let (mut sticks, mut wahs, mut hats, mut muffles, mut stops, mut breaks) = (0, 0, 0, 0, 0, 0);
    let mut moved_while_muffled = 0;
    for enemy in ENEMIES {
        let other = get(enemy);
        for k in 0..2 {
            for seed in 1..=6 {
                let mut fighters = [other, other];
                fighters[k] = cornet;
                let setup = Setup {
                    fighters,
                    rules: Rules::new(120.0, 3.5),
                    seed,
                };
                let rec = run(&setup, true).expect("runs");
                fights += 1;
                let mine = |e: &&Event, kind: &str| e.kind == kind && e.who() == Some(k);
                let n = |kind: &str| rec.events.iter().filter(|e| mine(e, kind)).count();
                sticks += n("plunger_stick");
                wahs += n("wah");
                hats += n("derby_land");
                stops += n("stop");
                breaks += n("break");
                assert!(
                    n("plunger") > 0,
                    "CORNET against {enemy}, corner {k}, seed {seed}: never threw"
                );
                if let Some(w) = rec.winner {
                    kos += 1;
                    if w == k {
                        wins += 1;
                    }
                }
                // a muffled enemy keeps moving
                let Some(Array::F64 { data: p, .. }) = rec.arrays.get("P") else {
                    panic!("no P");
                };
                for e in rec.events.iter().filter(|e| e.kind == "muffle" && e.i(3) == k as i64) {
                    muffles += 1;
                    let o = 1 - k;
                    let f0 = ((e.t - setup.rules.t_fight) * 30.0) as usize + 1;
                    let f1 = f0 + 10;
                    if f1 * 4 + 4 > p.len() {
                        continue;
                    }
                    let at = |f: usize| [p[f * 4 + o * 2], p[f * 4 + o * 2 + 1]];
                    let (a, b) = (at(f0), at(f1));
                    if (a[0] - b[0]).hypot(a[1] - b[1]) > 0.05 {
                        moved_while_muffled += 1;
                    }
                }
            }
        }
    }
    println!(
        "{fights} fights, {kos} K.O.s, CORNET won {wins}; sticks {sticks}, wahs {wahs}, hats {hats}, \
         muffles {muffles}, stops {stops}, breaks {breaks}"
    );
    assert!(sticks > fights, "the cup should stick about once a fight or more");
    assert!(wahs > sticks, "a stuck cup wahs");
    assert!(hats > 0 && muffles > 0, "DERBY HAT lands and muffles");
    assert!(stops > 0 && breaks > 0, "STOP-TIME plays");
    assert!(moved_while_muffled * 2 >= muffles, "a muffled enemy keeps moving");
    assert!(kos * 10 >= fights * 7, "most fights end in a K.O.");
    let rate = wins as f64 / kos.max(1) as f64;
    assert!((0.2..=0.8).contains(&rate), "CORNET won {rate:.2} of its K.O.s");
}
