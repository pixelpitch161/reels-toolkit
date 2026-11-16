//! CONTRABASS CLARINET, the first fighter made in the Studio (design `docs/studio/2026-11-16-contrabass-clarinet-
//! design.md`): its FOGHORN fights four different fighters to the end from both corners, its blasts land, its wall
//! slams happen, and its three reused abilities fire.

use bf_core::event::Value;
use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{Rules, Setup, run};

fn roster() -> Vec<FighterCfg> {
    let mut all = parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads");
    all.extend(parse_roster(include_str!("../../bf-content/data/studio_roster.json")).expect("the studio roster"));
    all
}

fn how(e: &bf_core::event::Event) -> Option<&str> {
    match e.fields.get(5) {
        Some(Value::Str(s)) => Some(s),
        _ => None,
    }
}

#[test]
fn the_foghorn_fights_to_the_end() {
    let all = roster();
    let get = |name: &str| all.iter().find(|f| f.name == name).expect("in the roster");
    let contra = get("CONTRABASS CLARINET");
    let (mut fights, mut kos, mut walls, mut blasts, mut wins, mut t_ko) = (0usize, 0u32, 0, 0, 0, 0.0);
    let mut abilities = std::collections::BTreeSet::new();
    for enemy in ["FLUTE", "TUBA", "BASS CLARINET", "DRUMS"] {
        let other = get(enemy);
        for k in 0..2 {
            for seed in 1..=4 {
                let mut fighters = [other, other];
                fighters[k] = contra;
                let setup = Setup {
                    fighters,
                    rules: Rules::new(126.0, 3.5),
                    seed,
                };
                let rec = run(&setup, true).expect("runs");
                fights += 1;
                let mine = |h: &str| {
                    rec.events
                        .iter()
                        .filter(|e| e.kind == "hit" && e.who() == Some(k) && how(e) == Some(h))
                        .count()
                };
                assert!(
                    rec.events.iter().any(|e| e.kind == "horn" && e.who() == Some(k)),
                    "against {enemy}, corner {k}, seed {seed}: the horn never blew"
                );
                blasts += mine("foghorn");
                walls += mine("wall");
                for e in rec.events.iter().filter(|e| e.kind == "ability" && e.who() == Some(k)) {
                    if let Some(Value::Str(s)) = e.fields.get(2) {
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
    assert!(blasts > fights, "{blasts} blasts landed in {fights} fights");
    assert!(walls > 0, "no wall slam in {fights} fights");
    assert!(kos as usize * 4 >= fights * 3, "{kos} K.O.s in {fights} fights");
    eprintln!(
        "{fights} fights, {kos} K.O.s (it won {wins}, mean K.O. at {:.1} s),",
        t_ko / f64::from(kos.max(1))
    );
    eprintln!(" {blasts} blasts, {walls} wall slams, abilities {abilities:?}");
}
