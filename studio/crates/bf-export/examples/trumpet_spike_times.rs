//! SPIKE: when the TRUMPET's candidate standard attacks happen in a fight, in video frames (30 fps), so the
//! in-engine example clips can be cut around them. CPU only.
//!
//! `BF_TRUMPET_STD=1 cargo run --release -p bf-export --example trumpet_spike_times -- <fight.toml> <seed>`

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
                            && matches!(f.kind, EvKind::Contact { mv: m, .. } if m == *mv)
                    })
                    .count();
                contacts += hits;
                let dist = rec.steps.get(e.step as usize).map(|st| {
                    let (a, b) = (st.pos[0], st.pos[1]);
                    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
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
    for (i, st) in rec.steps.iter().enumerate().step_by(120) {
        let (a, b) = (st.pos[0], st.pos[1]);
        println!(
            "step {i} dist {:.2} a {:.1?} b {:.1?}",
            ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt(),
            a,
            b
        );
    }
    println!("{contacts} contacts");
}
