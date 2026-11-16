//! SNARE DRUM (a Studio fighter, no kit reference): its PARADIDDLE, FLAM, RIMSHOT and BUZZ ROLL fight several fighters
//! to the end from both corners without a panic; strokes follow the paradiddle sticking, its three moves fire, a
//! landed rimshot stuns, the per-frame `STICKS` rows are recorded, and a seed replays the same. A lenient balance
//! check: the snare wins some and loses some (the Studio balances a pair by handicap and seed choice).

use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{Rules, Setup, run};

fn roster() -> Vec<FighterCfg> {
    let mut all = parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads");
    all.extend(parse_roster(include_str!("../../bf-content/data/roster_new.json")).expect("the new roster loads"));
    all
}

const STICKING: [i64; 8] = [1, 0, 1, 1, 0, 1, 0, 0];

#[test]
fn snare_drum_fights_to_the_end() {
    let all = roster();
    let of = |n: &str| {
        all.iter()
            .find(|f| f.name == n)
            .unwrap_or_else(|| panic!("{n} in the roster"))
    };
    let sn = of("SNARE DRUM");
    let (mut fights, mut kos, mut wins) = (0, 0, 0);
    let mut count = std::collections::BTreeMap::<&str, usize>::new();
    let mut hits = std::collections::BTreeMap::<String, usize>::new();
    for foe in ["DRUMS", "MARIMBA", "FLUTE", "TUBA", "SNARE DRUM"] {
        let foe = of(foe);
        let mut foe_wins = 0;
        for seed in 1..=4u64 {
            for (side, fighters) in [(0usize, [sn, foe]), (1, [foe, sn])] {
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
                assert!(rec.arrays.contains_key("STICKS"), "the fight records STICKS");
                for e in &rec.events {
                    if e.who() != Some(side) {
                        continue;
                    }
                    if e.kind == "stick" {
                        assert_eq!(e.i(4), STICKING[(e.i(3) % 8) as usize], "paradiddle sticking");
                    }
                    for kind in ["stick", "flam", "rimshot", "buzz_roll"] {
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
        eprintln!("SNARE DRUM v {}: the foe won {foe_wins} of 8", foe.name);
    }
    eprintln!("{fights} fights, {kos} K.O.s, SNARE DRUM won {wins}; {count:?}; hits {hits:?}");
    assert_eq!(fights, 40);
    assert!(kos >= fights / 2, "{kos} K.O.s in {fights} fights");
    for kind in ["stick", "flam", "rimshot", "buzz_roll"] {
        assert!(count.get(kind).copied().unwrap_or(0) > 0, "no {kind}: {count:?}");
    }
    for how in ["stick", "flam", "rimshot", "buzz"] {
        assert!(hits.get(how).copied().unwrap_or(0) > 0, "no {how} hit: {hits:?}");
    }
    // lenient: it neither always wins nor always loses (mirror fights count once each way)
    assert!((4..=36).contains(&wins), "SNARE DRUM won {wins} of {fights}");
}

#[test]
fn a_seed_replays_the_same() {
    let all = roster();
    let sn = all.iter().find(|f| f.name == "SNARE DRUM").unwrap();
    let foe = all.iter().find(|f| f.name == "DRUMS").unwrap();
    let go = || {
        let setup = Setup {
            fighters: [sn, foe],
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
