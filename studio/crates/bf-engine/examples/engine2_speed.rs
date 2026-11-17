//! Engine 2's fight speed (milestone E1): TEST DUMMY against TEST DUMMY on seeds 1 to 2000, unrecorded, on one
//! worker and then on all workers but one (as `ballfight bench` counts them, so the two compare on one machine).
//! The two passes must give the same outcomes. Prints fights per second for both, the K.O. share, the median and
//! 10th to 90th percentile K.O. time in video seconds, the share that K.O. between 40 and 55 s, and the mean hits
//! per fight.
//!
//! Run: `cargo run --release -p bf-engine --example engine2_speed [seeds] [workers]`.

use std::time::Instant;

use bf_core::Engine;
use bf_core::fightfile::FightFile;
use bf_engine::{Fought, fight};
use rayon::prelude::*;

const EXAMPLE: &str = include_str!("../../../fights/flute_vs_trombone.toml");

fn outcome(ff: &FightFile, seed: u64) -> bf2_sim::Outcome {
    match fight(ff, seed, false).expect("the dummy fights") {
        Fought::Two(r) => r.outcome,
        Fought::One(_) => unreachable!("an Engine 2 file"),
    }
}

/// The bits of an outcome, for an exact comparison.
fn bits(o: &bf2_sim::Outcome) -> String {
    format!(
        "{:?} {:?} {:?} {:?} {:?} {}",
        o.winner,
        o.t_ko.map(f64::to_bits),
        o.hp_end.map(f64::to_bits),
        o.hits,
        o.damage.map(f64::to_bits),
        o.by_decision
    )
}

fn pct(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

fn main() {
    let mut args = std::env::args().skip(1);
    let seeds: u64 = args.next().map_or(2000, |s| s.parse().expect("seeds is a number"));
    let workers: usize = args.next().map_or_else(
        || std::thread::available_parallelism().map_or(1, |n| n.get().saturating_sub(1).max(1)),
        |s| s.parse().expect("workers is a number"),
    );
    let ff = FightFile {
        engine: Engine::Two,
        fighters: vec!["TEST DUMMY".into(), "TEST DUMMY".into()],
        ..FightFile::parse(EXAMPLE).expect("the example parses")
    };
    // Warm up (builds the dummy once).
    let _ = outcome(&ff, 0);

    let t = Instant::now();
    let one: Vec<_> = (1..=seeds).map(|s| outcome(&ff, s)).collect();
    let one_rate = seeds as f64 / t.elapsed().as_secs_f64();

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .expect("a thread pool");
    let t = Instant::now();
    let all: Vec<_> = pool.install(|| (1..=seeds).into_par_iter().map(|s| outcome(&ff, s)).collect());
    let all_rate = seeds as f64 / t.elapsed().as_secs_f64();

    assert!(
        one.iter().zip(&all).all(|(a, b)| bits(a) == bits(b)),
        "the 1-worker and {workers}-worker passes differ"
    );

    let mut kos: Vec<f64> = one.iter().filter_map(|o| o.t_ko).collect();
    kos.sort_by(f64::total_cmp);
    let n = one.len() as f64;
    let band = kos.iter().filter(|&&t| (40.0..=55.0).contains(&t)).count();
    let hits: u64 = one.iter().map(|o| u64::from(o.hits[0] + o.hits[1])).sum();
    println!("engine2_speed: TEST DUMMY vs TEST DUMMY, seeds 1 to {seeds}, record off");
    println!("fights/s, 1 worker:  {one_rate:.0}");
    println!("fights/s, {workers} workers: {all_rate:.0}");
    println!("outcomes equal on 1 and {workers} workers: yes");
    println!("K.O. share: {:.1} %", 100.0 * kos.len() as f64 / n);
    println!(
        "t_ko (video s): median {:.2}, p10 {:.2}, p90 {:.2}",
        pct(&kos, 0.5),
        pct(&kos, 0.1),
        pct(&kos, 0.9)
    );
    println!("K.O. between 40 and 55 s: {:.1} %", 100.0 * band as f64 / n);
    println!("mean hits per fight: {:.1}", hits as f64 / n);
}
