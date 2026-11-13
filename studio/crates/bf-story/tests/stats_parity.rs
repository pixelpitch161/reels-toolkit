//! The statistics proof: the Rust engine's fights are, taken together, the kit's fights.
//!
//! `studio/reference/fights/flute_vs_trombone/stats.json` (written by `python -m tools.studio_export stats`) holds
//! one row per seed 1 to 4,000 of the kit's own fights of the example episode, and the pooled figures as
//! `kit/tuning.py` computes them.
//!
//! Two different things are tested:
//!
//! - **The reduction** (`Stats::of`): the kit's 4,000 rows pooled in Rust give the kit's pooled figures bit for bit
//!   (numpy's median and its linear percentile included). No fight is run.
//! - **The fights** (release only): the statistics of 40,000 Rust fights, seeds 1 to 40,000, against the kit's
//!   4,000. Seeds are Studio-owned: the Rust engine agrees with the kit for the first 25 s or more of a fight and
//!   then drifts in the last bits, so one seed's winner or K.O. time may differ from the kit's. Nothing here
//!   compares seed by seed; only the distributions are compared, with bars several times the sampling noise of the
//!   kit's 4,000 fights (win share 0.8 points, median K.O. 0.12 s, mean lead changes 0.015, fit rate 0.14
//!   points, one standard error each). A single fight that goes another way (a knife-edge clash that only one
//!   engine sees) moves the win share and the fit rate by 1 / 40,000 and leaves the median where it is, so the
//!   comparison cannot break on it.

mod support;

use std::fs;

use bf_story::{BALANCE_SEED0, KO_BINS, KO_HIST_FROM, Outcome, Stats, ko_hist, stats};
use serde_json::Value;

use support::{base, roster, sheet, with_seed};

fn stats_json() -> Value {
    let path = bf_testdata::reference("fights/flute_vs_trombone/stats.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap()
}

/// The kit's pooled figures as a `Stats`.
fn python_pooled(doc: &Value) -> Stats {
    let p = &doc["pooled"];
    let f = |k: &str| p[k].as_f64().unwrap_or_else(|| panic!("pooled.{k}"));
    Stats {
        n: p["n"].as_u64().unwrap() as usize,
        decided: p["decided"].as_u64().unwrap() as usize,
        win_share: f("win_share"),
        ko_median: f("ko_median"),
        ko_p10: f("ko_p10"),
        ko_p90: f("ko_p90"),
        no_ko_share: f("no_ko_share"),
        lead_changes_mean: f("lead_changes_mean"),
        fit_rate: f("fit_rate"),
        // the kit prints no histogram: the Rust count of the kit's own rows (only the bins are compared, below)
        ko_hist: ko_hist(&python_rows(doc)),
    }
}

/// The kit's rows as outcomes.
fn python_rows(doc: &Value) -> Vec<Outcome> {
    doc["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| Outcome {
            winner: r["winner"].as_u64().map(|w| w as usize),
            t_ko: r["t_ko"].as_f64(),
            lead_changes: r["lead_changes"].as_u64().unwrap() as u32,
            fits: r["fits"].as_bool().unwrap(),
        })
        .collect()
}

fn bits(s: &Stats) -> [u64; 7] {
    [
        s.win_share,
        s.ko_median,
        s.ko_p10,
        s.ko_p90,
        s.no_ko_share,
        s.lead_changes_mean,
        s.fit_rate,
    ]
    .map(f64::to_bits)
}

#[test]
fn the_kits_rows_pool_to_the_kits_figures() {
    let doc = stats_json();
    let rows = python_rows(&doc);
    assert_eq!(rows.len(), 4_000);
    let want = python_pooled(&doc);
    let got = Stats::of(&rows).unwrap();
    assert_eq!((got.n, got.decided), (want.n, want.decided));
    assert_eq!(bits(&got), bits(&want), "\n got {got:?}\nwant {want:?}");
    // the bins of the kit's rows, counted here without the helper: a decided fight with 30 <= t < 58 is in bin
    // floor(t) - 30
    let mut bins = [0u32; KO_BINS];
    for r in rows.iter().filter(|r| r.winner.is_some()) {
        if let Some(t) = r.t_ko.filter(|t| (30.0..58.0).contains(t)) {
            bins[(t as usize) - 30] += 1;
        }
    }
    assert_eq!(got.ko_hist, bins);
    assert_eq!(got.ko_hist, want.ko_hist);
    assert!(bins.iter().sum::<u32>() <= got.decided as u32);
    // an odd count takes the other branch of numpy's median, and other weights in its percentile: the kit's first
    // 1,001 and 37 rows, pooled by numpy (`tools.studio_export.stats.pooled(rows[:n], names)` on this PC)
    for (n, want) in [(1_001, PY_1001), (37, PY_37)] {
        let got = Stats::of(&rows[..n]).unwrap();
        assert_eq!((got.n, got.decided), (n, want.0), "{n} rows");
        assert_eq!(bits(&got), want.1.map(f64::to_bits), "{n} rows: {got:?}");
    }
    assert_eq!(Stats::of(&[]), None, "no fights, no figures");
}

/// `(decided, [win_share, ko_median, ko_p10, ko_p90, no_ko_share, lead_changes_mean, fit_rate])` of the kit's first
/// 1,001 rows.
const PY_1001: (usize, [f64; 7]) = (
    926,
    [
        0.4870410367170626,
        50.25490196078431,
        42.95490196078431,
        57.03823529411764,
        0.07492507492507493,
        0.5674325674325674,
        0.007992007992007992,
    ],
);
/// The same of its first 37 rows.
const PY_37: (usize, [f64; 7]) = (
    32,
    [
        0.625,
        49.813235294117646,
        44.85490196078431,
        99.0,
        0.13513513513513514,
        0.6216216216216216,
        0.0,
    ],
);

/// The brief's bars, and three checks of its own that are looser: the 10th percentile, the share of fights with no
/// K.O., and that fits are there at all (a fit rate of zero would pass a bar of 1.5 points).
fn assert_the_kits(rs: &Stats, py: &Stats) {
    let point = 0.01;
    let checks = [
        ("win share", rs.win_share, py.win_share, 3.0 * point),
        ("median K.O.", rs.ko_median, py.ko_median, 0.7),
        ("mean lead changes", rs.lead_changes_mean, py.lead_changes_mean, 0.25),
        ("fit rate", rs.fit_rate, py.fit_rate, 1.5 * point),
        ("10th percentile of the K.O. times", rs.ko_p10, py.ko_p10, 1.0),
        (
            "share of fights with no K.O.",
            rs.no_ko_share,
            py.no_ko_share,
            2.0 * point,
        ),
    ];
    for (what, rust, kit, bar) in checks {
        println!(
            "  {what}: Rust {rust:.5}, kit {kit:.5}, apart {:.5} (bar {bar})",
            (rust - kit).abs()
        );
        assert!(
            (rust - kit).abs() <= bar,
            "{what}: Rust {rust}, the kit {kit}, more than {bar} apart"
        );
    }
    assert!(rs.fit_rate > 0.002, "fit rate {}: the fits are missing", rs.fit_rate);
}

fn workers() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get())
}

/// A regression guard that every test run can afford, NOT an independent sample: 4,000 Rust fights of the same
/// seeds 1 to 4,000 the kit's figures are of. The two engines agree on most of these fights (3,687 end with the
/// same winner), so the figures are close because the fights are largely the same fights; that makes this a
/// sensitive alarm for a change that moves the engine, and no proof about its statistics. The proof is the
/// 40,000-fight test below. The seeds are fixed, so the figures are too: this cannot pass one day and fail the next.
#[test]
fn rust_statistics_of_4000_fights_are_the_kits() {
    let doc = stats_json();
    let py = python_pooled(&doc);
    let roster = roster();
    let base = base(&roster);
    let rs = stats(&|seed| with_seed(&base, seed), &sheet(&base), 1..4_001, workers()).unwrap();
    assert_eq!(py.n, 4_000);
    println!("  kit  (seeds 1 to 4,000): {py:?}");
    println!("  Rust (seeds 1 to 4,000): {rs:?}");
    assert_eq!(rs.n, 4_000);
    assert_the_kits(&rs, &py);
}

/// The proof at full size: 40,000 Rust fights. About 7 s on the reference PC's 16 threads; a small build machine
/// takes a minute in a debug build, so it runs in release only (`cargo test --release -p bf-story`).
#[test]
#[cfg_attr(debug_assertions, ignore = "40,000 fights: run with --release")]
fn rust_statistics_are_the_kits() {
    let doc = stats_json();
    let py = python_pooled(&doc);
    let roster = roster();
    let base = base(&roster);
    let sheet = sheet(&base);
    let workers = workers();
    let started = std::time::Instant::now();
    let rs = stats(&|seed| with_seed(&base, seed), &sheet, 1..40_001, workers).unwrap();
    let secs = started.elapsed().as_secs_f64();
    println!(
        "{} Rust fights in {secs:.1} s on {workers} workers: {:.0} fights a second",
        rs.n,
        rs.n as f64 / secs
    );
    println!("  kit  (seeds 1 to 4,000):  {py:?}");
    println!("  Rust (seeds 1 to 40,000): {rs:?}");
    assert_eq!((rs.n, py.n), (40_000, 4_000));
    assert_the_kits(&rs, &py);
    // The brief's bars for the fit rate (1.5 points on a rate of 0.75 %) and the lead changes (0.25 on 0.57) would
    // let a fault through, so both are also held to about 4 standard errors of the kit's 4,000 fights (fit rate
    // 0.14 points, mean lead changes 0.015; the Rust figures' own errors are a third of that).
    for (what, rust, kit, bar) in [
        ("fit rate", rs.fit_rate, py.fit_rate, 0.005),
        ("mean lead changes", rs.lead_changes_mean, py.lead_changes_mean, 0.06),
    ] {
        assert!(
            (rust - kit).abs() <= bar,
            "{what}: Rust {rust}, the kit {kit}, more than {bar} apart (the tight bar)"
        );
    }

    // one worker: the speed of one core, and the same figures as many workers give
    let started = std::time::Instant::now();
    let one = stats(&|seed| with_seed(&base, seed), &sheet, 1..2_001, 1).unwrap();
    let secs = started.elapsed().as_secs_f64();
    println!(
        "2000 Rust fights in {secs:.2} s on 1 worker: {:.0} fights a second",
        2_000.0 / secs
    );
    assert_eq!(
        one,
        stats(&|seed| with_seed(&base, seed), &sheet, 1..2_001, workers.max(7)).unwrap()
    );

    // for the record only (seeds are Studio-owned, nothing is asserted seed by seed): how many of the kit's 4,000
    // seeds end with the same winner here
    let rows = python_rows(&doc);
    let same = (1..=4_000u64)
        .filter(|&seed| {
            let c = bf_story::fight(&with_seed(&base, seed), &sheet).unwrap();
            c.story.winner == rows[seed as usize - 1].winner
        })
        .count();
    println!("{same} of the kit's 4,000 seeds end with the same winner (or the same lack of one) in Rust");
}

#[test]
fn statistics_do_not_depend_on_the_worker_count() {
    let roster = roster();
    let base = base(&roster);
    let sheet = sheet(&base);
    let run = |workers| stats(&|seed| with_seed(&base, seed), &sheet, 1..61, workers).unwrap();
    let one = run(1);
    assert_eq!(one.n, 60);
    assert_eq!(run(7), one);
    // and they are the pooled outcomes of those fights, one by one
    let rows: Vec<Outcome> = (1..61)
        .map(|seed| Outcome::of(&bf_story::fight(&with_seed(&base, seed), &sheet).unwrap()))
        .collect();
    assert_eq!(Stats::of(&rows).unwrap(), one);
}

#[test]
fn statistics_of_a_setup_the_engine_refuses_are_an_error_in_words() {
    let roster = roster();
    let base = base(&roster);
    let sheet = sheet(&base);
    let r = stats(
        &|seed| {
            let mut setup = with_seed(&base, seed);
            setup.rules.dt = 0.0;
            setup
        },
        &sheet,
        1..1_000_000,
        7,
    );
    let Err(bf_story::Halt::Failed(words)) = r else {
        panic!("{r:?}");
    };
    assert!(words.starts_with("seed 1: rules.DT: "), "{words}");
    let none = stats(&|seed| with_seed(&base, seed), &sheet, 5..5, 2);
    assert!(matches!(none, Err(bf_story::Halt::Failed(_))), "{none:?}");
}

/// The K.O. histogram of a balance step's 400 seeds: at most one count per decided fight, every bin a whole second
/// from 30 s, and most K.O.s inside the 28 seconds it covers.
#[test]
fn the_histogram_counts_ko_seconds_of_400_balance_seeds() {
    let roster = roster();
    let base = base(&roster);
    let seeds = BALANCE_SEED0..BALANCE_SEED0 + 400;
    let s = stats(&|seed| with_seed(&base, seed), &sheet(&base), seeds, workers()).unwrap();
    println!(
        "  K.O.s a second from {KO_HIST_FROM} s: {:?} ({} decided)",
        s.ko_hist, s.decided
    );
    let counted = s.ko_hist.iter().sum::<u32>();
    assert!(counted <= s.decided as u32, "{counted} counted, {} decided", s.decided);
    assert!(
        counted * 10 >= s.decided as u32 * 9,
        "{counted} of {} decided counted",
        s.decided
    );
}
