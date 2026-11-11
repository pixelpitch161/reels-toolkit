//! Auto-balance: from no handicap it finds one that makes FLUTE against TROMBONE even, near the kit's hand-tuned
//! 2.0 / 0.40, and gives the same answer for any number of workers.

mod support;

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use bf_story::{
    BALANCE_SEED0, BalancePlan, BalanceProgress, BalanceRound, Halt, auto_balance, auto_balance_with, in_band, stats,
};

use support::{base, roster, sheet, with_handicap};

fn quiet(_: BalanceProgress) {}

/// A small plan for the tests that need the method, not the answer: 2 steps a round on 24 fights, 40 to confirm.
const SMALL: BalancePlan = BalancePlan {
    eval_seeds: 24,
    steps: 2,
    confirm_seeds: 40,
};

/// The whole run: 28 evaluations of 400 fights and 3,000 to confirm, 14,200 fights. Release only: about 5 s there
/// on 16 workers, several times that in a debug build.
#[test]
#[cfg_attr(debug_assertions, ignore = "14,200 fights: run with --release")]
fn lands_in_band() {
    let roster = roster();
    let base = base(&roster);
    let sheet = sheet(&base);
    let workers = std::thread::available_parallelism().map_or(1, |n| n.get());
    let seen = Mutex::new(Vec::new());
    let started = std::time::Instant::now();
    let r = auto_balance(
        &|h, seed| with_handicap(&base, h, seed),
        &sheet,
        [1.0, 1.0],
        workers,
        &|p| seen.lock().unwrap().push(p),
        &AtomicBool::new(false),
    )
    .unwrap();
    let secs = started.elapsed().as_secs_f64();
    let ratio = r.handicap[0] / r.handicap[1];
    println!(
        "auto-balance from [1.0, 1.0]: handicap {:?} (ratio {ratio:.2}), confirmed on {} fights: FLUTE wins {:.1} % \
         of {} decided, median K.O. {:.2} s, no K.O. {:.1} %, fit rate {:.2} %; 14,200 fights in {secs:.1} s on \
         {workers} workers ({:.0} fights a second)",
        r.handicap,
        r.confirm.n,
        100.0 * r.confirm.win_share,
        r.confirm.decided,
        r.confirm.ko_median,
        100.0 * r.confirm.no_ko_share,
        100.0 * r.confirm.fit_rate,
        14_200.0 / secs,
    );
    assert!(r.in_band, "{:?}", r.confirm);
    assert!(in_band(&r.confirm));
    assert!((0.40..=0.60).contains(&r.confirm.win_share));
    assert!((49.0..=52.0).contains(&r.confirm.ko_median));
    assert!((2.5..=10.0).contains(&ratio), "ratio {ratio}");
    assert_eq!(r.confirm.n, 3_000);
    // rounded to 2 decimals
    for h in r.handicap {
        assert_eq!(h, bf_core::py::round_nd(h, 2));
    }
    // the confirmation is the statistics of the confirmation seeds under that handicap, nothing else
    let again = stats(
        &|seed| with_handicap(&base, r.handicap, seed),
        &sheet,
        BALANCE_SEED0 + 1_000_000..BALANCE_SEED0 + 1_003_000,
        workers,
    )
    .unwrap();
    assert_eq!(again, r.confirm);
    // A, B, A, B, seven steps each, and the progress counts them and the confirmation
    assert_eq!(r.steps.len(), 28);
    for (i, step) in r.steps.iter().enumerate() {
        assert_eq!(step.round, i / 7);
        let what = if (i / 7) % 2 == 0 {
            BalanceRound::Ratio
        } else {
            BalanceRound::Scale
        };
        assert_eq!(step.what, what);
        assert_eq!(step.stats.n, 400);
    }
    let seen = seen.into_inner().unwrap();
    let want: Vec<BalanceProgress> = (1..=29).map(|done| BalanceProgress { done, total: 29 }).collect();
    assert_eq!(seen, want);
}

#[test]
fn same_result_for_any_worker_count() {
    let roster = roster();
    let base = base(&roster);
    let sheet = sheet(&base);
    let run = |workers| {
        auto_balance_with(
            &SMALL,
            &|h, seed| with_handicap(&base, h, seed),
            &sheet,
            [1.0, 1.0],
            workers,
            &quiet,
            &AtomicBool::new(false),
        )
        .unwrap()
    };
    let one = run(1);
    assert_eq!(one.steps.len(), 8);
    assert_eq!(one.confirm.n, 40);
    assert_eq!(run(7), one);
    assert_eq!(run(16), one);
}

#[test]
fn the_rounds_bisect_the_ratio_then_the_scale() {
    let roster = roster();
    let base = base(&roster);
    let sheet = sheet(&base);
    // every fight is asked for with the handicap of its step, on the evaluation seeds
    let asked = Mutex::new(Vec::new());
    let r = auto_balance_with(
        &SMALL,
        &|h, seed| {
            asked.lock().unwrap().push((h, seed));
            with_handicap(&base, h, seed)
        },
        &sheet,
        [1.0, 1.0],
        3,
        &quiet,
        &AtomicBool::new(false),
    )
    .unwrap();
    let asked = asked.into_inner().unwrap();
    assert_eq!(asked.len(), 8 * 24 + 40);
    for step in &r.steps {
        let mut seeds: Vec<u64> = asked
            .iter()
            .filter(|(h, s)| *h == step.handicap && *s < BALANCE_SEED0 + 1_000_000)
            .map(|(_, s)| *s)
            .collect();
        seeds.sort_unstable();
        seeds.dedup();
        assert_eq!(seeds, (BALANCE_SEED0..BALANCE_SEED0 + 24).collect::<Vec<_>>());
    }
    let mut confirm: Vec<u64> = asked
        .iter()
        .filter(|(_, s)| *s >= BALANCE_SEED0 + 1_000_000)
        .map(|(_, s)| *s)
        .collect();
    confirm.sort_unstable();
    assert_eq!(
        confirm,
        (BALANCE_SEED0 + 1_000_000..BALANCE_SEED0 + 1_000_040).collect::<Vec<_>>()
    );
    assert!(
        asked
            .iter()
            .filter(|(_, s)| *s >= BALANCE_SEED0 + 1_000_000)
            .all(|(h, _)| *h == r.handicap)
    );

    // the answer is rounded to 2 decimals, and the band is read off its confirmation
    for h in r.handicap {
        assert_eq!(h, bf_core::py::round_nd(h, 2));
    }
    assert_eq!(r.in_band, in_band(&r.confirm));

    // round A, step 1: the middle of [1/8, 8] is a ratio of 1 at the starting scale of 1
    let first = r.steps[0].handicap;
    assert!(
        (first[0] - 1.0).abs() < 1e-12 && (first[1] - 1.0).abs() < 1e-12,
        "{first:?}"
    );
    // round A moves the ratio and keeps the scale: the product of the two handicaps stays 1
    for step in &r.steps[..2] {
        assert_eq!(step.what, BalanceRound::Ratio);
        assert!(
            (step.handicap[0] * step.handicap[1] - 1.0).abs() < 1e-12,
            "{:?}",
            step.handicap
        );
    }
    // step 2 goes the way step 1's win share says: more for the left fighter when it won less than half
    let second = r.steps[1].handicap;
    if r.steps[0].stats.win_share > 0.5 {
        assert!(second[0] < second[1], "{second:?}");
    } else {
        assert!(second[0] > second[1], "{second:?}");
    }
    // round B moves the scale and keeps the ratio round A ended on
    let ratio_b: Vec<f64> = r.steps[2..4].iter().map(|s| s.handicap[0] / s.handicap[1]).collect();
    assert!((ratio_b[0] - ratio_b[1]).abs() < 1e-9 * ratio_b[0], "{ratio_b:?}");
    assert_eq!(r.steps[2].what, BalanceRound::Scale);
    // its first step is the middle of [1/4, 4]: a scale of 1
    let b0 = r.steps[2].handicap;
    assert!((b0[0] * b0[1] - 1.0).abs() < 1e-12, "{b0:?}");
    // and its second step goes the way the median K.O. says: more damage when the fights ran long
    let b1 = r.steps[3].handicap;
    if r.steps[2].stats.ko_median > 50.5 {
        assert!(b1[0] * b1[1] > 1.0, "{b1:?}");
    } else {
        assert!(b1[0] * b1[1] < 1.0, "{b1:?}");
    }
}

#[test]
fn balance_can_be_cancelled() {
    let roster = roster();
    let base = base(&roster);
    let sheet = sheet(&base);
    // before it starts
    let fought = AtomicUsize::new(0);
    let r = auto_balance(
        &|h, seed| {
            fought.fetch_add(1, Ordering::SeqCst);
            with_handicap(&base, h, seed)
        },
        &sheet,
        [1.0, 1.0],
        7,
        &quiet,
        &AtomicBool::new(true),
    );
    assert_eq!(r, Err(Halt::Cancelled));
    assert_eq!(fought.load(Ordering::SeqCst), 0);
    // on the way: it stops within a few fights, not at the end of the evaluation
    let cancel = AtomicBool::new(false);
    let fought = AtomicUsize::new(0);
    let r = auto_balance(
        &|h, seed| {
            if fought.fetch_add(1, Ordering::SeqCst) == 30 {
                cancel.store(true, Ordering::SeqCst);
            }
            with_handicap(&base, h, seed)
        },
        &sheet,
        [1.0, 1.0],
        7,
        &quiet,
        &cancel,
    );
    assert_eq!(r, Err(Halt::Cancelled));
    let fought = fought.load(Ordering::SeqCst);
    assert!(fought <= 31 + 7, "{fought} fights were started");
}

#[test]
fn a_handicap_the_engine_refuses_is_said_once_in_words() {
    let roster = roster();
    let base = base(&roster);
    let sheet = sheet(&base);
    // a start that is no handicap
    for start in [[0.0, 1.0], [1.0, -2.0], [f64::NAN, 1.0], [1.0, f64::INFINITY]] {
        let fought = AtomicUsize::new(0);
        let r = auto_balance(
            &|h, seed| {
                fought.fetch_add(1, Ordering::SeqCst);
                with_handicap(&base, h, seed)
            },
            &sheet,
            start,
            2,
            &quiet,
            &AtomicBool::new(false),
        );
        let Err(Halt::Failed(words)) = r else {
            panic!("{start:?}: {r:?}");
        };
        assert!(words.contains("handicap"), "{words}");
        assert_eq!(fought.load(Ordering::SeqCst), 0);
    }
    // rules the engine cannot run: said after the first fight, not after 14,200
    let fought = AtomicUsize::new(0);
    let r = auto_balance(
        &|h, seed| {
            fought.fetch_add(1, Ordering::SeqCst);
            let mut setup = with_handicap(&base, h, seed);
            setup.rules.dt = -1.0;
            setup
        },
        &sheet,
        [1.0, 1.0],
        7,
        &quiet,
        &AtomicBool::new(false),
    );
    let Err(Halt::Failed(words)) = r else {
        panic!("{r:?}");
    };
    assert!(
        words.starts_with(&format!("seed {BALANCE_SEED0}: rules.DT: ")),
        "{words}"
    );
    assert!(fought.load(Ordering::SeqCst) <= 4 * 7);
}
