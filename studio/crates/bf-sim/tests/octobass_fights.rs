//! OCTOBASS (a Studio fighter, no kit reference): its LEVER PUNCH, FOOT PEDAL, SEVEN LEVERS and BELOW HEARING fight
//! several fighters to the end from both corners without a panic; its three moves fire and land, the per-frame
//! `LEVERS` rows are recorded, and a seed replays the same. A lenient balance check: the octobass wins some and loses
//! some (the Studio balances a pair by handicap and seed choice).

use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{Rules, Setup, run};

fn roster() -> Vec<FighterCfg> {
    let mut all = parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads");
    all.extend(parse_roster(include_str!("../../bf-content/data/roster_new.json")).expect("the new roster loads"));
    all
}

#[test]
fn octobass_fights_to_the_end() {
    let all = roster();
    let of = |n: &str| {
        all.iter()
            .find(|f| f.name == n)
            .unwrap_or_else(|| panic!("{n} in the roster"))
    };
    let ob = of("OCTOBASS");
    let (mut fights, mut kos, mut wins) = (0, 0, 0);
    let mut count = std::collections::BTreeMap::<&str, usize>::new();
    let mut hits = std::collections::BTreeMap::<String, usize>::new();
    for foe in ["CONTRABASS", "SNARE DRUM", "FLUTE", "TUBA", "OCTOBASS"] {
        let foe = of(foe);
        let mut foe_wins = 0;
        for seed in 1..=4u64 {
            for (side, fighters) in [(0usize, [ob, foe]), (1, [foe, ob])] {
                let setup = Setup {
                    fighters,
                    rules: Rules::new(120.0, 3.5),
                    seed,
                };
                let rec =
                    run(&setup, true).unwrap_or_else(|e| panic!("{} v {}: {e}", fighters[0].name, fighters[1].name));
                fights += 1;
                kos += usize::from(rec.t_ko.is_some());
                if rec.winner == Some(side) {
                    wins += 1;
                } else if rec.winner.is_some() {
                    foe_wins += 1;
                }
                assert!(rec.arrays.contains_key("LEVERS"), "the fight records LEVERS");
                for e in &rec.events {
                    if e.who() != Some(side) {
                        continue;
                    }
                    for kind in ["lever", "pedal_snap", "seven_levers", "below_start", "below_hit"] {
                        if e.kind == kind {
                            *count.entry(kind).or_default() += 1;
                        }
                    }
                    if e.kind == "hit" {
                        *hits.entry(e.s(7).to_string()).or_default() += 1;
                    }
                }
            }
        }
        eprintln!("OCTOBASS v {}: the foe won {foe_wins} of 8", foe.name);
    }
    eprintln!("{fights} fights, {kos} K.O.s, OCTOBASS won {wins}; {count:?}; hits {hits:?}");
    assert_eq!(fights, 40);
    assert!(kos >= fights / 2, "{kos} K.O.s in {fights} fights");
    for kind in ["lever", "pedal_snap", "seven_levers", "below_start", "below_hit"] {
        assert!(count.get(kind).copied().unwrap_or(0) > 0, "no {kind}: {count:?}");
    }
    for how in ["lever", "pedal", "seven", "below"] {
        assert!(hits.get(how).copied().unwrap_or(0) > 0, "no {how} hit: {hits:?}");
    }
    // lenient: it neither always wins nor always loses (mirror fights count once each way)
    assert!((4..=36).contains(&wins), "OCTOBASS won {wins} of {fights}");
}

#[test]
fn a_seed_replays_the_same() {
    let all = roster();
    let ob = all.iter().find(|f| f.name == "OCTOBASS").unwrap();
    let foe = all.iter().find(|f| f.name == "CONTRABASS").unwrap();
    let go = || {
        let setup = Setup {
            fighters: [ob, foe],
            rules: Rules::new(120.0, 3.5),
            seed: 7,
        };
        run(&setup, true).unwrap()
    };
    let (a, b) = (go(), go());
    assert_eq!(a.winner, b.winner);
    assert_eq!(a.events.len(), b.events.len());
    assert_eq!(format!("{:?}", a.events), format!("{:?}", b.events));
}
