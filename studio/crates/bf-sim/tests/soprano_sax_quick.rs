//! SOPRANO SAX (a from-scratch fighter, 2026-11-16): HORNET SWARM, KICK THE HIVE, BUZZ OFF and QUEEN fight a spread of
//! fighters to the end from both corners. The hornets sting and the swarm grows, the abilities play, and a lenient
//! balance check: SOPRANO SAX neither always wins nor always loses.

use bf_core::event::Event;
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
    "ALTO SAX",
    "EUPHONIUM",
    "CLARINET",
    "VIOLIN",
    "SOPRANO SAX",
];

#[test]
fn soprano_sax_fights_to_the_end() {
    let all = roster();
    let get = |name: &str| all.iter().find(|f| f.name == name).expect("in the roster");
    let me = get("SOPRANO SAX");
    let (mut fights, mut kos, mut wins) = (0, 0, 0);
    let (mut launches, mut grows, mut hives, mut buzzes, mut queens) = (0, 0, 0, 0, 0);
    let mut lengths = Vec::new();
    for enemy in ENEMIES {
        let other = get(enemy);
        for k in 0..2 {
            for seed in 1..=6 {
                let mut fighters = [other, other];
                fighters[k] = me;
                let setup = Setup {
                    fighters,
                    rules: Rules::new(120.0, 3.5),
                    seed,
                };
                let rec = run(&setup, true).expect("runs");
                fights += 1;
                let mine = |e: &&Event, kind: &str| e.kind == kind && e.who() == Some(k);
                let n = |kind: &str| rec.events.iter().filter(|e| mine(e, kind)).count();
                launches += n("hornet");
                grows += n("swarm_grow");
                hives += n("hive");
                buzzes += n("buzz");
                queens += n("queen");
                assert!(
                    n("hornet") > 0,
                    "SOPRANO SAX against {enemy}, corner {k}, seed {seed}: never launched"
                );
                if let Some(w) = rec.winner {
                    kos += 1;
                    if w == k {
                        wins += 1;
                    }
                }
                if let Some(e) = rec.events.iter().rev().find(|e| e.kind == "ko") {
                    lengths.push(e.t);
                }
            }
        }
    }
    lengths.sort_by(f64::total_cmp);
    let median = lengths.get(lengths.len() / 2).copied().unwrap_or(f64::NAN);
    println!(
        "{fights} fights, {kos} K.O.s, SOPRANO SAX won {wins}; launches {launches}, grows {grows}, hives {hives}, \
         buzzes {buzzes}, queens {queens}, median K.O. {median:.1} s"
    );
    assert!(grows > fights, "the swarm should grow about once a fight or more");
    assert!(hives > 0 && buzzes > 0 && queens > 0, "the abilities play");
    assert!(kos * 10 >= fights * 7, "most fights end in a K.O.");
    let rate = wins as f64 / kos.max(1) as f64;
    assert!((0.2..=0.8).contains(&rate), "SOPRANO SAX won {rate:.2} of its K.O.s");
}
