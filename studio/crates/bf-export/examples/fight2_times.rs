//! When the left fighter's moves happen in an Engine 2 fight, in video frames (30 fps), with the hits each landed,
//! so review clips can be cut around them. CPU only.
//!
//! `cargo run --release -p bf-export --example fight2_times -- <fight.toml> [seed]`

use bf_content::fighters2;
use bf_core::fightfile::FightFile;
use bf2_sim::{EvKind, Rules2, Setup2};

fn main() {
    let path = std::env::args().nth(1).expect("a fight file");
    let ff = FightFile::parse(&std::fs::read_to_string(path).unwrap()).expect("the file parses");
    let seed: u64 = std::env::args()
        .nth(2)
        .map_or(ff.seed.unwrap_or(1), |s| s.parse().unwrap());
    let defs = [0, 1].map(|k| fighters2::def(&ff.fighters[k]).expect("a fighter"));
    let mut rules = Rules2::new(ff.grid().t_fight);
    rules.dmg_scale = ff.dmg_scale;
    rules.dmg_side = [ff.handicap[0], ff.handicap[1]];
    let rec = bf2_sim::run(
        &Setup2 {
            fighters: defs.clone(),
            rules,
            seed,
        },
        true,
    )
    .expect("it fights");
    let d = bf_export::job2::direct(&ff, &rec);
    let fr = |s: u32| (d.map.step_video_t(s) * 30.0).round() as i64;
    println!("seed {seed}: K.O. {:?}", d.map.ko_video_t());
    let mut contacts = 0;
    for e in &rec.events {
        if e.who != 0 {
            continue;
        }
        if let EvKind::MoveStart { mv } = &e.kind {
            {
                let hits = rec
                    .events
                    .iter()
                    .filter(|f| {
                        f.who == 0
                            && f.step >= e.step
                            && f.step < e.step + defs[0].moves[*mv].total()
                            && (matches!(f.kind, EvKind::Contact { mv: m, .. } if m == *mv)
                                || matches!(f.kind, EvKind::ProjHit { .. }))
                    })
                    .count();
                contacts += hits;
                let dist = rec.steps.get(e.step as usize).map(|st| {
                    let (a, b) = (st.pos[0], st.pos[1]);
                    (a[0] - b[0]).hypot(a[1] - b[1])
                });
                println!(
                    "frame {:5}  {:18} hits {hits}  dist {:.2?}",
                    fr(e.step),
                    defs[0].moves[*mv].name,
                    dist
                );
            }
        }
    }
    // each projectile hit, by the move that threw it
    let mut thrower = std::collections::BTreeMap::new();
    for e in rec.events.iter().filter(|e| e.who == 0) {
        if let EvKind::Spawn { proj } = e.kind
            && let Some((mv, _, _)) = rec.steps.get(e.step as usize).and_then(|st| st.mv[0])
        {
            thrower.insert(proj, defs[0].moves[mv as usize].name);
        }
    }
    let ph: Vec<(i64, &str)> = rec
        .events
        .iter()
        .filter_map(|e| match e.kind {
            EvKind::ProjHit { proj, .. } if e.who == 0 => {
                Some((fr(e.step), thrower.get(&proj).copied().unwrap_or("?")))
            }
            _ => None,
        })
        .collect();
    println!("{contacts} contacts; projectile hits {ph:?}");
    if std::env::var("BF_TRACE_PROJ").is_ok() {
        for (i, st) in rec.steps.iter().enumerate() {
            for &(id, p) in &st.projs {
                if thrower.get(&(id as usize)).is_some_and(|n| *n == "PISTON ROCKETS") && i % 6 == 0 {
                    let b = st.pos[1];
                    println!("step {i} proj {id} at {:.2?}  foe {:.2?}", p, b);
                }
            }
        }
    }
}
