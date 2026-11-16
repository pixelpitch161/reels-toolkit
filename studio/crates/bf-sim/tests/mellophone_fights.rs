//! MELLOPHONE (a Studio fighter, no kit reference): its SLAPBACK, HORNS UP, PRESS BOX and STADIUM ECHO fight several
//! fighters to the end from both corners without a panic; waves fly, land on the ropes, echo back and hit, its three
//! moves fire, and the per-frame `SLAP` rows are recorded. A lenient balance check: the mellophone wins some and loses
//! some (the Studio balances a pair by handicap and seed choice).

use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{Rules, Setup, run};

fn roster() -> Vec<FighterCfg> {
    let mut all = parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads");
    all.extend(parse_roster(include_str!("../../bf-content/data/roster_new.json")).expect("the new roster loads"));
    all
}

#[test]
fn mellophone_fights_six_fighters_to_the_end() {
    let all = roster();
    let of = |n: &str| {
        all.iter()
            .find(|f| f.name == n)
            .unwrap_or_else(|| panic!("{n} in the roster"))
    };
    let mar = of("MELLOPHONE");
    let (mut fights, mut kos, mut wins) = (0, 0, 0);
    let mut count = std::collections::BTreeMap::<&str, usize>::new();
    let mut hits = std::collections::BTreeMap::<String, usize>::new();
    let mut rows = 0;
    for foe in ["FLUTE", "PIANO", "EUPHONIUM", "TRUMPET", "HARP", "MARIMBA"] {
        let foe = of(foe);
        let mut foe_wins = 0;
        for seed in 1..=4u64 {
            for (side, fighters) in [(0usize, [mar, foe]), (1, [foe, mar])] {
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
                for e in &rec.events {
                    let mine = e.who() == Some(side);
                    for kind in [
                        "slap",
                        "slap_rope",
                        "slap_end",
                        "horns_up",
                        "press_box",
                        "horns_stun",
                        "stadium_echo",
                    ] {
                        if e.kind == kind && (mine || kind == "horns_stun") {
                            *count.entry(kind).or_default() += 1;
                        }
                    }
                    if e.kind == "hit" && mine {
                        *hits.entry(e.s(7).to_string()).or_default() += 1;
                    }
                }
                rows += usize::from(rec.arrays.contains_key("SLAP"));
            }
        }
        eprintln!("MELLOPHONE v {}: the foe won {foe_wins} of 8", foe.name);
    }
    eprintln!("{fights} fights, {kos} K.O.s, MELLOPHONE won {wins}; {count:?}; hits {hits:?}");
    assert_eq!(fights, 48);
    assert_eq!(rows, fights, "every fight records SLAP");
    assert!(kos >= fights / 2, "{kos} K.O.s in {fights} fights");
    for kind in [
        "slap",
        "slap_rope",
        "slap_end",
        "horns_up",
        "press_box",
        "horns_stun",
        "stadium_echo",
    ] {
        assert!(count.get(kind).copied().unwrap_or(0) > 0, "no {kind}: {count:?}");
    }
    for how in ["slapback", "stab", "stadium"] {
        assert!(hits.get(how).copied().unwrap_or(0) > 0, "no {how} hit: {hits:?}");
    }
    // lenient: it neither always wins nor always loses
    assert!((5..=43).contains(&wins), "MELLOPHONE won {wins} of {fights}");
}
