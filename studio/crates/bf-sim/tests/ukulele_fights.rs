//! UKULELE's quick port (ruling M4-R16): its SURF'S UP and UKE-NADO fight ACOUSTIC (whose rake shares the `dc` counter
//! and `surf_rake_step`) and three other fighters to the end, from both corners, without a panic; K.O.s happen, and
//! its waves and waterspouts run (events and the `X33_SPOUT` path).

use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{Rules, Setup, run};

fn roster() -> Vec<FighterCfg> {
    parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads")
}

#[test]
fn ukulele_fights_four_fighters_to_the_end() {
    let all = roster();
    let of = |n: &str| {
        all.iter()
            .find(|f| f.name == n)
            .unwrap_or_else(|| panic!("{n} in the roster"))
    };
    let uke = of("UKULELE");
    let (mut fights, mut kos, mut surfs, mut spouts, mut paths) = (0, 0, 0, 0, 0);
    for foe in ["ACOUSTIC", "FLUTE", "TRUMPET", "BAGPIPES"] {
        let foe = of(foe);
        for seed in 1..=4u64 {
            for fighters in [[uke, foe], [foe, uke]] {
                let setup = Setup {
                    fighters,
                    rules: Rules::new(132.0, 3.5),
                    seed,
                };
                let rec =
                    run(&setup, true).unwrap_or_else(|e| panic!("{} v {}: {e}", fighters[0].name, fighters[1].name));
                fights += 1;
                kos += usize::from(rec.t_ko.is_some());
                surfs += rec.events.iter().filter(|e| e.kind == "surf").count();
                spouts += rec.events.iter().filter(|e| e.kind == "spout").count();
                paths += usize::from(rec.arrays.contains_key("X33_SPOUT"));
                if foe.name == "ACOUSTIC" {
                    assert!(rec.events.iter().any(|e| e.kind == "rake"), "ACOUSTIC still rakes");
                }
            }
        }
    }
    eprintln!("{fights} fights, {kos} K.O.s, {surfs} waves, {spouts} spouts, {paths} paths");
    assert_eq!(fights, 32);
    assert!(kos >= fights / 2, "{kos} K.O.s in {fights} fights");
    assert!(surfs > fights * 5, "{surfs} waves");
    assert!(spouts > 0 && paths > 0, "{spouts} spouts, {paths} paths");
}
