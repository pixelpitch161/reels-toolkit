//! Search, statistics and balance over a fight function: the `_by` forms give exactly what the set-up forms give
//! (they are the same bodies; the set-up forms call them), and a failing fight function is said at its seed.

mod support;

use std::sync::atomic::AtomicBool;

use bf_story::{
    BalancePlan, BalanceProgress, Candidate, SearchEnd, SearchOpts, SearchProgress, auto_balance_by, auto_balance_with,
    fight, search, search_by, stats, stats_by,
};

use support::{base, roster, sheet, with_handicap, with_seed};

fn quiet(_: SearchProgress) {}

fn quiet_balance(_: BalanceProgress) {}

fn opts() -> SearchOpts {
    SearchOpts {
        from: 1,
        stop_fits: 20,
        max_seeds: Some(300),
        workers: 4,
    }
}

#[test]
fn search_by_equals_search() {
    let roster = roster();
    let base = base(&roster);
    let sheet = sheet(&base);
    let cancel = AtomicBool::new(false);
    let old = search(&|seed| with_seed(&base, seed), &sheet, &opts(), &quiet, &cancel);
    let new = search_by(
        &|seed| fight(&with_seed(&base, seed), &sheet).map_err(|e| e.to_string()),
        &opts(),
        &quiet,
        &cancel,
    );
    assert!(old.checked > 0 && !old.candidates.is_empty());
    assert_eq!(new, old);
}

#[test]
fn stats_by_equals_stats() {
    let roster = roster();
    let base = base(&roster);
    let sheet = sheet(&base);
    let old = stats(&|seed| with_seed(&base, seed), &sheet, 1..201, 4).unwrap();
    let new = stats_by(
        &|seed| fight(&with_seed(&base, seed), &sheet).map_err(|e| e.to_string()),
        1..201,
        4,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(old.n, 200);
    assert_eq!(new, old);
}

#[test]
fn auto_balance_by_equals_auto_balance() {
    let roster = roster();
    let base = base(&roster);
    let sheet = sheet(&base);
    let plan = BalancePlan {
        eval_seeds: 40,
        steps: 2,
        confirm_seeds: 40,
    };
    let cancel = AtomicBool::new(false);
    let old = auto_balance_with(
        &plan,
        &|h, seed| with_handicap(&base, h, seed),
        &sheet,
        [1.0, 1.0],
        4,
        &quiet_balance,
        &cancel,
    )
    .unwrap();
    let new = auto_balance_by(
        &plan,
        &|h, seed| fight(&with_handicap(&base, h, seed), &sheet).map_err(|e| e.to_string()),
        [1.0, 1.0],
        4,
        &quiet_balance,
        &cancel,
    )
    .unwrap();
    assert_eq!(old.steps.len(), 8);
    assert_eq!(new, old);
}

#[test]
fn search_by_reports_the_failing_seed() {
    let roster = roster();
    let base = base(&roster);
    let sheet = sheet(&base);
    let fight_for = |seed: u64| -> Result<Candidate, String> {
        if seed == 7 {
            Err("the engine said no".to_string())
        } else {
            fight(&with_seed(&base, seed), &sheet).map_err(|e| e.to_string())
        }
    };
    for workers in [1, 4] {
        let r = search_by(
            &fight_for,
            &SearchOpts { workers, ..opts() },
            &quiet,
            &AtomicBool::new(false),
        );
        assert_eq!(r.end, SearchEnd::Failed("seed 7: the engine said no".to_string()));
        assert_eq!(r.checked, 6);
        let mut seeds: Vec<u64> = r.candidates.iter().map(|c| c.seed).collect();
        seeds.sort_unstable();
        assert_eq!(seeds, (1..=6).collect::<Vec<_>>());
    }
}
