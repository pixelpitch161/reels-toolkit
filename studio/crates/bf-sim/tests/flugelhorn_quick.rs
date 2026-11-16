//! FLUGELHORN, a fighter made in the Studio (design `docs/studio/2026-11-16-flugelhorn-design.md`): its WINGBEAT
//! fights four different fighters to the end from both corners, its claps land, and its FLANKING CALL, FEATHER
//! SHOWER and TAKE WING fire. A lenient balance check: the seed finder handicaps pairs.

use bf_core::event::Value;
use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{Rules, Setup, run};

fn roster() -> Vec<FighterCfg> {
    let mut all = parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads");
    all.extend(parse_roster(include_str!("../../bf-content/data/roster_new.json")).expect("the studio roster"));
    all
}

fn how(e: &bf_core::event::Event) -> Option<&str> {
    match e.fields.get(5) {
        Some(Value::Str(s)) => Some(s),
        _ => None,
    }
}

#[test]
fn the_wingbeat_fights_to_the_end() {
    let all = roster();
    let get = |name: &str| all.iter().find(|f| f.name == name).expect("in the roster");
    let flugel = get("FLUGELHORN");
    let (mut fights, mut kos, mut wins, mut t_ko) = (0usize, 0u32, 0, 0.0);
    let mut hits = std::collections::BTreeMap::<String, usize>::new();
    let mut abilities = std::collections::BTreeSet::new();
    for enemy in ["TRUMPET", "TUBA", "VIOLIN", "DRUMS"] {
        let other = get(enemy);
        for k in 0..2 {
            for seed in 1..=4 {
                let mut fighters = [other, other];
                fighters[k] = flugel;
                let setup = Setup {
                    fighters,
                    rules: Rules::new(120.0, 3.5),
                    seed,
                };
                let rec = run(&setup, true).expect("runs");
                fights += 1;
                assert!(
                    rec.events.iter().any(|e| e.kind == "clap" && e.who() == Some(k)),
                    "against {enemy}, corner {k}, seed {seed}: the wings never clapped"
                );
                for e in rec.events.iter().filter(|e| e.who() == Some(k)) {
                    if e.kind == "hit"
                        && let Some(h) = how(e)
                    {
                        *hits.entry(h.to_string()).or_default() += 1;
                    }
                    if e.kind == "ability"
                        && let Some(Value::Str(s)) = e.fields.get(2)
                    {
                        abilities.insert(s.clone());
                    }
                }
                if rec.winner.is_some() {
                    kos += 1;
                }
                if rec.winner == Some(k) {
                    wins += 1;
                }
                t_ko += rec.t_ko.unwrap_or(0.0);
            }
        }
    }
    eprintln!(
        "{fights} fights, {kos} K.O.s (it won {wins}, mean K.O. at {:.1} s), hits {hits:?}, abilities {abilities:?}",
        t_ko / f64::from(kos.max(1))
    );
    assert!(hits.get("wingbeat").copied().unwrap_or(0) > fights, "claps landed: {hits:?}");
    assert!(kos as usize * 4 >= fights * 3, "{kos} K.O.s in {fights} fights");
    for name in ["flank", "arc", "take_wing"] {
        assert!(abilities.contains(name), "{name} never fired: {abilities:?}");
    }
    assert!((3..=29).contains(&wins), "FLUGELHORN won {wins} of {fights}");
}
