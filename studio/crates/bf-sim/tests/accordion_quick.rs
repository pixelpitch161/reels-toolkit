//! ACCORDION's quick port (ruling M4-R16): BELLOWS BITE fights four fighters to the end from both corners, its bites
//! land, FULL BELLOWS clamps, and TUBA's BRASS WALL can block a clamp.

use bf_core::event::Value;
use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{Rules, Setup, run};

fn roster() -> Vec<FighterCfg> {
    parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads")
}

fn how_is(e: &bf_core::event::Event, how: &str) -> bool {
    matches!(e.fields.get(5), Some(Value::Str(s)) if s == how)
}

#[test]
fn accordion_fights_to_the_end() {
    let all = roster();
    let get = |name: &str| all.iter().find(|f| f.name == name).expect("in roster.json");
    let accordion = get("ACCORDION");
    let (mut fights, mut kos, mut clamps, mut blocked) = (0, 0, 0, 0);
    for enemy in ["BAGPIPES", "TUBA", "FLUTE", "TROMBONE"] {
        let other = get(enemy);
        for k in 0..2 {
            for seed in 1..=4 {
                let mut fighters = [other, other];
                fighters[k] = accordion;
                let setup = Setup {
                    fighters,
                    rules: Rules::new(132.0, 3.5),
                    seed,
                };
                let rec = run(&setup, true).expect("runs");
                fights += 1;
                let count = |kind: &str| {
                    rec.events
                        .iter()
                        .filter(|e| e.kind == kind && e.who() == Some(k))
                        .count()
                };
                let bites = rec
                    .events
                    .iter()
                    .filter(|e| e.kind == "hit" && e.who() == Some(k) && how_is(e, "bite"))
                    .count();
                assert!(
                    bites > 0,
                    "ACCORDION against {enemy}, corner {k}, seed {seed}: no bite landed"
                );
                assert!(count("bite") >= count("snap"), "a snap without a bite");
                for e in rec
                    .events
                    .iter()
                    .filter(|e| e.kind == "clamp_snap" && e.who() == Some(k))
                {
                    clamps += 1;
                    if enemy == "TUBA" && e.i(3) == 0 {
                        blocked += 1;
                    }
                }
                if rec.winner.is_some() {
                    kos += 1;
                } else {
                    eprintln!("no K.O.: ACCORDION against {enemy}, corner {k}, seed {seed}");
                }
            }
        }
    }
    eprintln!("{kos} K.O.s in {fights} fights, {clamps} clamps, {blocked} blocked by TUBA");
    // unhandicapped (the seed finder handicaps a pairing): a K.O. in at least half the fights
    assert!(kos * 2 >= fights, "{kos} K.O.s in {fights} fights");
    assert!(clamps > 0, "FULL BELLOWS never clamped");
}

#[test]
fn tubas_brass_wall_blocks_a_clamp() {
    let all = roster();
    let get = |name: &str| all.iter().find(|f| f.name == name).expect("in roster.json");
    let (accordion, tuba) = (get("ACCORDION"), get("TUBA"));
    let mut blocked = None;
    'seeds: for seed in 1..=60 {
        for k in 0..2 {
            let mut fighters = [tuba, tuba];
            fighters[k] = accordion;
            let setup = Setup {
                fighters,
                rules: Rules::new(132.0, 3.5),
                seed,
            };
            let rec = run(&setup, true).expect("runs");
            if rec
                .events
                .iter()
                .any(|e| e.kind == "clamp_snap" && e.who() == Some(k) && e.i(3) == 0)
            {
                blocked = Some((seed, k));
                break 'seeds;
            }
        }
    }
    let (seed, k) = blocked.expect("no clamp met TUBA's shield in 60 seeds");
    eprintln!("a clamp blocked by BRASS WALL: seed {seed}, ACCORDION in corner {k}");
}
