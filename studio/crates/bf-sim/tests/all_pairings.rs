//! Every offered pairing, from both corners, fights a few seeds to the end: no panic, no number that is not finite in
//! the positions, speeds or hit points, and nobody further out than a throw can carry a fighter (the ropes are at
//! 3.0; TENOR SAX's stage dive reaches 3.9 by design). About 10 seconds in a release build.
//!
//! Written by the code review of 2026-11-23 (findings F1 and F2). On studio-v1.8.1 it failed for TUBA v VIOLIN (every
//! seed: positions became NaN once the VIOLIN was swallowed) and for BASS GUITAR (a FRET SLIDE or SLINGSHOT dash that
//! missed near a rope carried it out of the ring); 1.8.2 fixed both. A standing quick test: every new fighter is in
//! it through the rosters (`docs/studio/HANDOFF.md`, How to add a fighter). `ALL_PAIRINGS_SEEDS=40` runs more seeds
//! per pairing (about 2.5 minutes).

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use bf_core::recording::Array;
use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{Rules, Setup, run};

/// The furthest a fighter's centre may be from the middle of the ring on either axis.
const FURTHEST: f64 = 4.5;

fn roster() -> Vec<FighterCfg> {
    let mut all = parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads");
    all.extend(parse_roster(include_str!("../../bf-content/data/roster_new.json")).expect("the new roster loads"));
    all.extend(parse_roster(include_str!("../../bf-content/data/roster_dev.json")).expect("the dev roster loads"));
    // the retired fighters are never offered
    all.retain(|f| f.name != "GUITAR" && f.name != "SAX");
    all
}

#[test]
fn every_pairing_fights_to_the_end_inside_the_ring() {
    let all = roster();
    let seeds: u64 = std::env::var("ALL_PAIRINGS_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    let bpms = [120.0, 84.0, 174.0];
    let jobs: Vec<(usize, usize)> = (0..all.len())
        .flat_map(|a| (0..all.len()).map(move |b| (a, b)))
        .collect();
    let next = AtomicUsize::new(0);
    let problems = Mutex::new(Vec::<String>::new());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(a, b)) = jobs.get(i) else { break };
                    for seed in 1..=seeds {
                        let bpm = bpms[(seed as usize + a + b) % bpms.len()];
                        let tag = format!("{} v {} seed {seed} at {bpm} bpm", all[a].name, all[b].name);
                        let fought = std::panic::catch_unwind(|| {
                            let setup = Setup {
                                fighters: [&all[a], &all[b]],
                                rules: Rules::new(bpm, 3.5),
                                seed,
                            };
                            run(&setup, true)
                        });
                        let said = match fought {
                            Err(_) => Some(format!("{tag}: the fight panicked")),
                            Ok(Err(e)) => Some(format!("{tag}: {e}")),
                            Ok(Ok(rec)) => first_problem(&rec.arrays).map(|p| format!("{tag}: {p}")),
                        };
                        if let Some(said) = said {
                            problems.lock().unwrap_or_else(|e| e.into_inner()).push(said);
                        }
                    }
                }
            });
        }
    });
    let mut problems = problems.into_inner().unwrap_or_else(|e| e.into_inner());
    problems.sort();
    assert!(
        problems.is_empty(),
        "{} fights went wrong, the first of them:\n{}",
        problems.len(),
        problems.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// The first thing wrong in a recording's per-frame arrays, in words.
fn first_problem(arrays: &std::collections::BTreeMap<String, Array>) -> Option<String> {
    for key in ["P", "V", "HP"] {
        let Some(Array::F64 { data, shape }) = arrays.get(key) else {
            return Some(format!("the recording has no {key} array of floats"));
        };
        let per_frame = (data.len() / shape.first().copied().unwrap_or(1).max(1)).max(1);
        if let Some(j) = data.iter().position(|x| !x.is_finite()) {
            return Some(format!("{key} is not a finite number from frame {}", j / per_frame));
        }
        if key == "P"
            && let Some(j) = data.iter().position(|x| x.abs() > FURTHEST)
        {
            let side = if (j / 2) % 2 == 0 { "left" } else { "right" };
            return Some(format!(
                "the {side} fighter is outside the ring from frame {} (a coordinate of {:.1}, the ropes are at 3.0)",
                j / per_frame,
                data[j]
            ));
        }
    }
    None
}
