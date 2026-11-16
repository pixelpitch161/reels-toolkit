//! CONTRABASSOON's quick proof (a fighter made from scratch, `docs/studio/2026-11-16-contrabassoon.md`): its pipes
//! fight five fighters to the end from both corners, every move shows up, and the low move's box holds the enemy.

use bf_core::event::Value;
use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{Rules, Setup, run};

fn roster() -> Vec<FighterCfg> {
    {
        let mut all = parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads");
        all.extend(parse_roster(include_str!("../../bf-content/data/roster_new.json")).expect("the new roster loads"));
        all
    }
}

fn how(e: &bf_core::event::Event) -> &str {
    match e.fields.get(5) {
        Some(Value::Str(s)) => s,
        _ => "",
    }
}

#[test]
fn contrabassoon_fights_to_the_end() {
    let all = roster();
    let get = |name: &str| all.iter().find(|f| f.name == name).expect("in roster.json");
    let me = get("CONTRABASSOON");
    let (mut fights, mut kos, mut wins) = (0, 0, 0);
    let mut seen = std::collections::BTreeMap::<String, usize>::new();
    for enemy in ["BASSOON", "TROMBONE", "CELLO", "HARP", "TUBA"] {
        let other = get(enemy);
        let mut won = 0;
        for k in 0..2 {
            for seed in 1..=6 {
                let mut fighters = [other, other];
                fighters[k] = me;
                let setup = Setup {
                    fighters,
                    rules: Rules::new(132.0, 3.5),
                    seed,
                };
                let rec = run(&setup, true).expect("runs");
                fights += 1;
                for e in rec.events.iter().filter(|e| e.who() == Some(k)) {
                    let key = if e.kind == "hit" {
                        format!("hit:{}", how(e))
                    } else {
                        e.kind.clone()
                    };
                    *seen.entry(key).or_default() += 1;
                }
                if let Some(w) = rec.winner {
                    kos += 1;
                    if w == k {
                        won += 1;
                    }
                }
            }
        }
        eprintln!("CONTRABASSOON against {enemy}: {won} of 12 won");
        wins += won;
    }
    eprintln!("{wins} won, {kos} K.O.s in {fights} fights; its events: {seen:?}");
    for key in ["pipe", "hit:pipe", "spray", "hot_pipes", "pipe_box"] {
        assert!(seen.get(key).copied().unwrap_or(0) > 0, "never saw {key}: {seen:?}");
    }
    assert!(kos * 4 >= fights * 3, "{kos} K.O.s in {fights} fights");
}

#[test]
fn the_box_holds_the_enemy() {
    let all = roster();
    let me = all.iter().find(|f| f.name == "CONTRABASSOON").expect("CONTRABASSOON");
    let foe = all.iter().find(|f| f.name == "TROMBONE").expect("TROMBONE");
    let mut checked = 0;
    for seed in 1..=24 {
        let setup = Setup {
            fighters: [me, foe],
            rules: Rules::new(132.0, 3.5),
            seed,
        };
        let rec = run(&setup, true).expect("runs");
        let Some(b) = rec.events.iter().find(|e| e.kind == "pipe_box") else {
            continue;
        };
        let f = |i: usize| match &b.fields[i] {
            Value::Float(x) => *x,
            other => panic!("{other:?}"),
        };
        let (cx, cy, h) = (f(2), f(3), f(4));
        let off = rec
            .events
            .iter()
            .find(|e| e.kind == "pipe_box_off")
            .map_or(f64::INFINITY, |e| e.t);
        let Some(bf_core::recording::Array::F64 { data, .. }) = rec.arrays.get("P") else {
            panic!("no P")
        };
        let fps = f64::from(rec.fps);
        for (n, row) in data.chunks(4).enumerate() {
            let t = (n + rec.f0) as f64 / fps;
            if t > b.t + 0.05 && t < off - 0.05 {
                let (x, y) = (row[2], row[3]);
                assert!(
                    (x - cx).abs() < h && (y - cy).abs() < h,
                    "seed {seed}: TROMBONE left the box at {t}"
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "no box in 24 fights");
}
