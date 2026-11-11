//! The seed search and the shortlist filters.
//!
//! The search must give the same shortlist for any number of workers, stop when told, say in words why it could not
//! go on (a setup the engine refuses, a fight that broke off) and never count such a seed as a fight that did not
//! fit. The filters are tested on candidates built by hand.

mod support;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use bf_sim::Setup;
use bf_story::{
    Candidate, Filters, SearchEnd, SearchOpts, SearchProgress, SearchResult, Story, Verdict, fight, rank, search,
};

use support::{base, roster, sheet, with_seed};

fn quiet(_: SearchProgress) {}

fn run(base: &Setup, opts: &SearchOpts) -> SearchResult {
    search(
        &|seed| with_seed(base, seed),
        &sheet(base),
        opts,
        &quiet,
        &AtomicBool::new(false),
    )
}

fn seeds(r: &SearchResult) -> Vec<u64> {
    r.candidates.iter().map(|c| c.seed).collect()
}

#[test]
fn same_shortlist_for_any_worker_count() {
    let roster = roster();
    let base = base(&roster);
    let opts = |workers| SearchOpts {
        from: 1,
        stop_fits: 5,
        max_seeds: None,
        workers,
    };
    let one = run(&base, &opts(1));
    assert_eq!(one.end, SearchEnd::Full);
    let fits: Vec<&Candidate> = one.candidates.iter().filter(|c| c.verdict.fits).collect();
    assert_eq!(fits.len(), 5, "the search stops on the fifth fit");
    // every seed from 1 to the last one checked is there once, and the last one is the fifth fit
    assert_eq!(one.candidates.len() as u64, one.checked);
    let mut all = seeds(&one);
    all.sort_unstable();
    assert_eq!(all, (1..=one.checked).collect::<Vec<_>>());
    assert_eq!(fits.iter().map(|c| c.seed).max(), Some(one.checked));
    // fits first, by score downward, then by seed upward; the same for the rest
    let fits_n = fits.len();
    for part in [&one.candidates[..fits_n], &one.candidates[fits_n..]] {
        assert!(part.iter().all(|c| c.verdict.fits == part[0].verdict.fits));
        for w in part.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            assert!(
                a.verdict.score > b.verdict.score || (a.verdict.score == b.verdict.score && a.seed < b.seed),
                "seeds {} and {} are out of order",
                a.seed,
                b.seed
            );
        }
    }
    for workers in [7, 8] {
        let many = run(&base, &opts(workers));
        assert_eq!(
            seeds(&many),
            seeds(&one),
            "{workers} workers: other seeds or another order"
        );
        assert_eq!(many, one, "{workers} workers: another result");
    }
}

#[test]
fn a_candidate_is_the_fight_of_its_seed() {
    let roster = roster();
    let base = base(&roster);
    let sheet = sheet(&base);
    let r = run(
        &base,
        &SearchOpts {
            from: 5,
            stop_fits: 200,
            max_seeds: Some(3),
            workers: 2,
        },
    );
    assert_eq!(r.end, SearchEnd::OutOfSeeds);
    assert_eq!(r.checked, 3);
    let mut got = seeds(&r);
    got.sort_unstable();
    assert_eq!(got, [5, 6, 7]);
    for c in &r.candidates {
        assert_eq!(c.story.seed, c.seed);
        assert_eq!(*c, fight(&with_seed(&base, c.seed), &sheet).unwrap());
    }
    // the seed handed out is the seed fought, whatever seed the closure left in the set-up
    let fixed = search(
        &|_| with_seed(&base, 999),
        &sheet,
        &SearchOpts {
            from: 5,
            stop_fits: 200,
            max_seeds: Some(3),
            workers: 2,
        },
        &quiet,
        &AtomicBool::new(false),
    );
    assert_eq!(fixed, r);
}

#[test]
fn a_search_in_chunks_is_the_search_in_one_go() {
    let roster = roster();
    let base = base(&roster);
    let chunk = |from, n| {
        run(
            &base,
            &SearchOpts {
                from,
                stop_fits: 200,
                max_seeds: Some(n),
                workers: 4,
            },
        )
    };
    let whole = chunk(1, 48);
    let (a, b) = (chunk(1, 16), chunk(17, 32));
    assert_eq!((a.checked, b.checked), (16, 32));
    let mut merged = a.candidates;
    merged.extend(b.candidates);
    rank(&mut merged);
    assert_eq!(merged, whole.candidates);
}

#[test]
fn progress_counts_every_seed_in_order() {
    let roster = roster();
    let base = base(&roster);
    let seen = std::sync::Mutex::new(Vec::new());
    let r = search(
        &|seed| with_seed(&base, seed),
        &sheet(&base),
        &SearchOpts {
            from: 10,
            stop_fits: 200,
            max_seeds: Some(40),
            workers: 7,
        },
        &|p| seen.lock().unwrap().push(p),
        &AtomicBool::new(false),
    );
    let seen = seen.into_inner().unwrap();
    assert_eq!(seen.len(), 40);
    for (i, p) in seen.iter().enumerate() {
        assert_eq!(p.checked, i as u64 + 1);
    }
    let fits = r.candidates.iter().filter(|c| c.verdict.fits).count();
    assert_eq!(seen.last().unwrap().fits, fits);
}

#[test]
fn search_can_be_cancelled() {
    let roster = roster();
    let base = base(&roster);
    let endless = SearchOpts {
        from: 1,
        stop_fits: usize::MAX,
        max_seeds: None,
        workers: 7,
    };
    // told to stop before it starts: nothing is fought
    let fought = AtomicU64::new(0);
    let r = search(
        &|seed| {
            fought.fetch_add(1, Ordering::SeqCst);
            with_seed(&base, seed)
        },
        &sheet(&base),
        &endless,
        &quiet,
        &AtomicBool::new(true),
    );
    assert_eq!((r.end, r.checked, r.candidates.len()), (SearchEnd::Stopped, 0, 0));
    assert_eq!(fought.load(Ordering::SeqCst), 0);

    // told to stop on the way: it stops within a few fights and what it has is whole
    let cancel = AtomicBool::new(false);
    let fought = AtomicU64::new(0);
    let r = search(
        &|seed| {
            fought.fetch_add(1, Ordering::SeqCst);
            with_seed(&base, seed)
        },
        &sheet(&base),
        &endless,
        &|p| {
            if p.checked >= 20 {
                cancel.store(true, Ordering::SeqCst);
            }
        },
        &cancel,
    );
    assert_eq!(r.end, SearchEnd::Stopped);
    assert!(r.checked >= 20, "checked {}", r.checked);
    // each worker finishes at most the fight it is in, and one more it had taken before the flag was set
    let fought = fought.load(Ordering::SeqCst);
    assert!(
        fought <= 20 + 2 * 7,
        "{fought} fights were started after 20 were checked"
    );
    assert!(r.checked <= fought);
    let mut got = seeds(&r);
    got.sort_unstable();
    assert_eq!(
        got,
        (1..=r.checked).collect::<Vec<_>>(),
        "the seeds checked are 1 to `checked`"
    );
    let sheet = sheet(&base);
    for c in &r.candidates {
        assert_eq!(*c, fight(&with_seed(&base, c.seed), &sheet).unwrap());
    }
}

#[test]
fn a_setup_the_engine_refuses_is_said_once_in_words() {
    let roster = roster();
    let base = base(&roster);
    for workers in [1, 7] {
        let asked = AtomicU64::new(0);
        let r = search(
            &|seed| {
                asked.fetch_add(1, Ordering::SeqCst);
                let mut setup = with_seed(&base, seed);
                setup.rules.dt = 0.0;
                setup
            },
            &sheet(&base),
            &SearchOpts {
                from: 1,
                stop_fits: 5,
                max_seeds: None,
                workers,
            },
            &quiet,
            &AtomicBool::new(false),
        );
        let SearchEnd::Failed(words) = &r.end else {
            panic!("{workers} workers: the search ended {:?}", r.end);
        };
        assert!(words.starts_with("seed 1: rules.DT: "), "{words}");
        assert!(!words.contains('\n'), "{words}");
        // not one fight that "did not fit", and no walk through the seeds
        assert_eq!((r.checked, r.candidates.len()), (0, 0));
        assert!(asked.load(Ordering::SeqCst) <= 2 * workers as u64);
    }
}

#[test]
fn an_error_ends_the_search_at_its_seed_for_any_worker_count() {
    let roster = roster();
    let base = base(&roster);
    let run = |workers| {
        search(
            &|seed| {
                let mut setup = with_seed(&base, seed);
                if seed == 9 {
                    setup.rules.t_max = f64::NAN;
                }
                setup
            },
            &sheet(&base),
            &SearchOpts {
                from: 1,
                stop_fits: usize::MAX,
                max_seeds: Some(30),
                workers,
            },
            &quiet,
            &AtomicBool::new(false),
        )
    };
    let one = run(1);
    let SearchEnd::Failed(words) = &one.end else {
        panic!("the search ended {:?}", one.end);
    };
    assert!(words.starts_with("seed 9: rules."), "{words}");
    assert_eq!(one.checked, 8);
    let mut got = seeds(&one);
    got.sort_unstable();
    assert_eq!(got, (1..=8).collect::<Vec<_>>());
    assert_eq!(run(7), one);
}

#[test]
fn a_fight_that_panics_ends_the_search_in_words() {
    let roster = roster();
    let base = base(&roster);
    let run = |workers| {
        search(
            &|seed| {
                assert!(seed != 4, "the set-up of seed four is broken");
                with_seed(&base, seed)
            },
            &sheet(&base),
            &SearchOpts {
                from: 1,
                stop_fits: usize::MAX,
                max_seeds: None,
                workers,
            },
            &quiet,
            &AtomicBool::new(false),
        )
    };
    let one = run(1);
    let SearchEnd::Failed(words) = &one.end else {
        panic!("the search ended {:?}", one.end);
    };
    assert!(words.starts_with("seed 4: "), "{words}");
    assert!(words.contains("the set-up of seed four is broken"), "{words}");
    assert_eq!(one.checked, 3);
    assert_eq!(run(7), one);
}

#[test]
fn no_fits_asked_for_is_no_search() {
    let roster = roster();
    let base = base(&roster);
    let r = run(
        &base,
        &SearchOpts {
            from: 1,
            stop_fits: 0,
            max_seeds: None,
            workers: 3,
        },
    );
    assert_eq!((r.end, r.checked, r.candidates.len()), (SearchEnd::Full, 0, 0));
    let r = run(
        &base,
        &SearchOpts {
            from: u64::MAX - 1,
            stop_fits: usize::MAX,
            max_seeds: None,
            workers: 3,
        },
    );
    // the seeds end at the largest whole number a seed can be
    assert_eq!((r.end, r.checked), (SearchEnd::OutOfSeeds, 2));
}

// ---- the filters, on candidates built by hand ----

/// A fit: the right fighter wins at 50 s on 20 HP after two lead changes.
fn candidate(seed: u64) -> Candidate {
    Candidate {
        seed,
        story: Story {
            seed,
            winner: Some(1),
            t_ko: Some(50.0),
            lead_changes: 2,
            lc_t: vec![20.0, 41.0],
            winner_hp: 20.0,
            firsts: vec![
                (10.0, 0, "FLUTTER TONGUE".to_string()),
                (14.0, 1, "SAD TROMBONE".to_string()),
            ],
            caps: vec![
                (10.0, 0, "FLUTTER TONGUE".to_string()),
                (14.0, 1, "SAD TROMBONE".to_string()),
            ],
            max_gap: 2.0,
            lows: vec![],
            // the right fighter is hit first and trails by 10, then leads; the K.O. leaves it 20 ahead
            lead: vec![(5.0, 10.0), (20.0, -10.0), (41.0, -15.0), (50.0, -20.0)],
        },
        verdict: Verdict {
            fits: true,
            score: 1.0,
            misses: vec![],
        },
    }
}

fn all() -> Filters {
    Filters {
        show_misfits: true,
        ..Filters::default()
    }
}

#[test]
fn no_filter_keeps_the_fits() {
    let mut c = candidate(1);
    assert!(Filters::default().keep(&c));
    c.verdict.fits = false;
    c.verdict.misses = vec!["no K.O.".to_string()];
    assert!(!Filters::default().keep(&c), "a misfit is hidden until asked for");
    assert!(all().keep(&c));
}

#[test]
fn the_winner_filter() {
    let c = candidate(1);
    let f = |w| Filters {
        winner: Some(w),
        ..all()
    };
    assert!(f(1).keep(&c));
    assert!(!f(0).keep(&c));
    let mut none = candidate(2);
    none.story.winner = None;
    none.story.t_ko = None;
    assert!(
        !f(0).keep(&none) && !f(1).keep(&none),
        "nobody won a fight with no K.O."
    );
}

#[test]
fn the_ko_time_filter() {
    let c = candidate(1);
    let f = |lo, hi| Filters {
        ko: Some((lo, hi)),
        ..all()
    };
    assert!(f(49.0, 51.0).keep(&c));
    assert!(f(50.0, 50.0).keep(&c), "both ends are inside");
    assert!(!f(50.5, 53.0).keep(&c));
    assert!(!f(40.0, 49.99).keep(&c));
    let mut none = candidate(2);
    none.story.winner = None;
    none.story.t_ko = None;
    assert!(!f(0.0, 100.0).keep(&none), "a fight with no K.O. has no K.O. time");
}

#[test]
fn the_lead_changes_filter() {
    let c = candidate(1);
    let f = |n| Filters {
        min_lead_changes: Some(n),
        ..all()
    };
    assert!(f(0).keep(&c) && f(2).keep(&c));
    assert!(!f(3).keep(&c));
}

#[test]
fn the_close_finish_filter() {
    let f = Filters {
        close_finish: true,
        ..all()
    };
    let mut c = candidate(1);
    assert!(!f.keep(&c), "20 HP is not close");
    c.story.winner_hp = 12.0;
    assert!(f.keep(&c), "12 HP is");
    c.story.winner_hp = 12.01;
    assert!(!f.keep(&c));
    c.story.winner = None;
    c.story.t_ko = None;
    c.story.winner_hp = 5.0;
    assert!(!f.keep(&c), "no finish without a K.O.");
}

#[test]
fn the_ability_filter() {
    let c = candidate(1);
    let f = |name: &str| Filters {
        ability: Some(name.to_string()),
        ..all()
    };
    assert!(f("SAD TROMBONE").keep(&c));
    assert!(!f("PICCOLO").keep(&c), "an ability nobody used");
    assert!(!f("SAD").keep(&c), "the whole name");
}

#[test]
fn the_comeback_filter() {
    let f = Filters {
        comeback: true,
        ..all()
    };
    // the hand-built fit: the winner (right) trailed by 10 at most and was never first below 25
    assert!(!f.keep(&candidate(1)));

    // trailing by 20 or more after 15 s
    let mut c = candidate(2);
    c.story.lead = vec![(16.0, 20.0), (30.0, -5.0), (50.0, -20.0)];
    assert!(f.keep(&c), "exactly 20 behind at 16 s");
    c.story.lead = vec![(16.0, 19.99), (30.0, -5.0), (50.0, -20.0)];
    assert!(!f.keep(&c), "19.99 behind is not 20");
    // ... the same seen from the other corner
    c.story.winner = Some(0);
    c.story.lead = vec![(16.0, -20.0), (30.0, 5.0), (50.0, 20.0)];
    assert!(f.keep(&c));
    c.story.lead = vec![(16.0, 20.0), (30.0, 5.0), (50.0, 20.0)];
    assert!(!f.keep(&c), "a winner 20 ahead did not come back");

    // "after 15 s": a deficit that was made up by 15 s does not count, one still standing at 15 s does
    let mut c = candidate(3);
    c.story.lead = vec![(6.0, 30.0), (15.0, 10.0), (50.0, -20.0)];
    assert!(!f.keep(&c), "30 behind until the hit at 15.0 s exactly");
    c.story.lead = vec![(6.0, 30.0), (15.01, 10.0), (50.0, -20.0)];
    assert!(f.keep(&c), "still 30 behind just after 15 s");

    // first below 25 HP: the winner (right) is brought to 24 while the left fighter still has 30
    let mut c = candidate(4);
    // HP after each hit: (30, 100), (30, 24), (0, 24): never 20 behind after 15 s, but first below 25
    c.story.lead = vec![(5.0, -70.0), (9.0, 6.0), (50.0, -24.0)];
    assert!(f.keep(&c));
    // the loser was first below 25: no comeback
    c.story.lead = vec![(5.0, -76.0), (9.0, 0.0), (50.0, -24.0)];
    assert!(
        !f.keep(&c),
        "HP (24, 100), (24, 24), (0, 24): the loser was below 25 first"
    );
    // exactly 25 is not below 25
    c.story.lead = vec![(5.0, -70.0), (9.0, 5.0), (12.0, -5.0), (50.0, -25.0)];
    assert!(!f.keep(&c), "HP (30, 100), (30, 25), (20, 25), (0, 25)");

    // no winner, no comeback
    let mut c = candidate(5);
    c.story.winner = None;
    c.story.t_ko = None;
    c.story.lead = vec![(16.0, 40.0), (30.0, -40.0)];
    assert!(!f.keep(&c));
}

#[test]
fn filters_add_up() {
    let c = candidate(1);
    let f = Filters {
        winner: Some(1),
        ko: Some((49.0, 53.0)),
        min_lead_changes: Some(2),
        close_finish: false,
        comeback: false,
        ability: Some("FLUTTER TONGUE".to_string()),
        show_misfits: false,
    };
    assert!(f.keep(&c));
    assert!(
        !Filters {
            min_lead_changes: Some(3),
            ..f.clone()
        }
        .keep(&c)
    );
    assert!(!Filters { winner: Some(0), ..f }.keep(&c));
}
