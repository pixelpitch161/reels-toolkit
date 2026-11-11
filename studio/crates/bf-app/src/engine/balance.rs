//! Balancing: `AutoBalance`, `Measure` and `StopBalance`.
//!
//! The coordinator (`super`) routes these commands here. Both kinds of work run on a thread of their own:
//!
//! - `AutoBalance` runs `bf_story::auto_balance` from the draft's handicap, sends `BalanceProgress` after each of
//!   its measurements (28 steps and the confirmation: 29), then `Balanced` with the handicap found and the figures
//!   of its confirmation;
//! - `Measure` fights the draft's handicap as it stands on the confirmation seeds (3,000 from
//!   `BALANCE_SEED0 + CONFIRM_OFFSET`), the same fights a balance run is confirmed on, and sends `Measured`.
//!
//! Only one of them runs at a time: a newer `AutoBalance` or `Measure`, or `StopBalance`, sets the running one's
//! cancel flag. The engine tests it before every fight, so a stopped run ends within a fight or two (well under a
//! second), and a stopped run sends nothing more: the flag is checked right before each event. Nothing restarts a
//! stopped run: only a new command starts one.
//!
//! The worker count is read once, when the work starts (`Ctx::workers`); the figures do not depend on it.
//!
//! A run that cannot be made (the draft is no fight, a handicap that is not a number above 0, a fight that broke
//! off) sends `Event::BalanceFailed` for its generation with a plain sentence; so does a panic on the worker. The
//! interface then shows the run as ended (ruling M2-R29).

use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use bf_sim::Setup;
use bf_story::balance::CONFIRM_OFFSET;
use bf_story::{BALANCE_SEED0, BalancePlan, BalanceProgress, BeatSheet, Halt, Stats};

use super::{Ctx, convert};
use crate::model::Gen;
use crate::model::balance::BalanceStats;
use crate::model::draft::Draft;
use crate::seam::{Command, Event};

/// What this handler keeps between commands: the cancel flag of the run going now, if any.
#[derive(Default)]
pub struct State {
    running: Option<Arc<AtomicBool>>,
}

/// The two kinds of work.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Work {
    Balance,
    Measure,
}

/// Takes one command and returns at once; long work goes to its own thread (`Ctx::spawn`).
pub fn handle(state: &mut State, ctx: &Ctx, command: Command) {
    let (work, r#gen, draft) = match command {
        Command::AutoBalance { r#gen, draft } => (Work::Balance, r#gen, draft),
        Command::Measure { r#gen, draft } => (Work::Measure, r#gen, draft),
        Command::StopBalance => {
            stop(state);
            return;
        }
        _ => return,
    };
    // A newer request stops the older one, whether or not the newer one starts.
    stop(state);
    let cancel = ctx.cancel_flag();
    let flag = cancel.clone();
    let (name, what) = match work {
        Work::Balance => ("engine: balance", "balancing the fight"),
        Work::Measure => ("engine: measure", "measuring the handicap"),
    };
    let started = ctx.spawn(
        name,
        what,
        move |ctx| match work {
            Work::Balance => balance(&ctx, r#gen, &draft, &flag),
            Work::Measure => measure(&ctx, r#gen, &draft, &flag),
        },
        move |text| Event::BalanceFailed { r#gen, text },
    );
    if started {
        state.running = Some(cancel);
    }
}

/// Stops the run going now, if any.
fn stop(state: &mut State) {
    if let Some(flag) = state.running.take() {
        flag.store(true, Ordering::SeqCst);
    }
}

/// The confirmation seeds: 3,000 from `BALANCE_SEED0 + CONFIRM_OFFSET`, which no balance step and no search uses.
fn confirm_seeds() -> Range<u64> {
    let from = BALANCE_SEED0 + CONFIRM_OFFSET;
    from..from + BalancePlan::default().confirm_seeds
}

/// The figures of a set of fights as the Balance screen shows them, with the seconds the run took.
///
/// `Stats::win_share` is already the left fighter's share of decided fights. The histogram's 28 bins are copied as
/// they are (K.O.s before 30 s or from 58 s on are in no bin). A time that is not a number above 0 counts as 0.
pub fn stats_view(s: &Stats, seconds: f64) -> BalanceStats {
    BalanceStats {
        fights: u32::try_from(s.n).unwrap_or(u32::MAX),
        win_share: s.win_share,
        ko_median: s.ko_median,
        ko_p10: s.ko_p10,
        ko_p90: s.ko_p90,
        no_ko_share: s.no_ko_share,
        lead_changes_mean: s.lead_changes_mean,
        fit_rate: s.fit_rate,
        ko_hist: s.ko_hist.to_vec(),
        seconds: if seconds.is_finite() { seconds.max(0.0) } else { 0.0 },
    }
}

/// Sends `e` unless the run was stopped.
fn send(ctx: &Ctx, cancel: &AtomicBool, e: Event) {
    if !cancel.load(Ordering::SeqCst) {
        ctx.emit(e);
    }
}

/// The fight of the draft at seed 0 and its beat sheet, as `bf-cli`'s `balance` builds them; or why there is none,
/// as a sentence.
fn fight_of(draft: &Draft, what: &str) -> Result<(Setup<'static>, BeatSheet), String> {
    if !draft.handicap.iter().all(|x| x.is_finite() && *x > 0.0) {
        return Err(format!(
            "This handicap cannot be {what}: each fighter's handicap must be a number above 0."
        ));
    }
    let ff = convert::fight_file(draft)?;
    let base = bf_sim::setup_from(&ff, bf_content::roster(), 0)
        .map_err(|e| convert::plain(&format!("this fight cannot be {what}: {e}")))?;
    let sheet = BeatSheet::series(base.fighters);
    Ok((base, sheet))
}

/// The fight of `seed` under handicap `h`: only the damage handicap and the seed change.
fn under(base: &Setup<'static>, h: [f64; 2], seed: u64) -> Setup<'static> {
    let mut s = Setup { seed, ..base.clone() };
    s.rules.dmg_side = Some(h);
    s
}

/// Why a run that was not stopped gave no figures, as a sentence.
fn failed(what: &str, why: &str) -> String {
    convert::plain(&format!("the {what} could not finish: {why}"))
}

fn balance(ctx: &Ctx, r#gen: Gen, draft: &Draft, cancel: &AtomicBool) {
    let t0 = Instant::now();
    let (base, sheet) = match fight_of(draft, "balanced") {
        Ok(f) => f,
        Err(text) => return send(ctx, cancel, Event::BalanceFailed { r#gen, text }),
    };
    let setup_for = |h: [f64; 2], seed: u64| under(&base, h, seed);
    let progress = |p: BalanceProgress| {
        send(
            ctx,
            cancel,
            Event::BalanceProgress {
                r#gen,
                done: u32::try_from(p.done).unwrap_or(u32::MAX),
                total: u32::try_from(p.total).unwrap_or(u32::MAX),
            },
        );
    };
    match bf_story::auto_balance(&setup_for, &sheet, draft.handicap, ctx.workers(), &progress, cancel) {
        Ok(found) => send(
            ctx,
            cancel,
            Event::Balanced {
                r#gen,
                handicap: found.handicap,
                stats: stats_view(&found.confirm, t0.elapsed().as_secs_f64()),
            },
        ),
        Err(Halt::Cancelled) => {}
        Err(Halt::Failed(why)) => send(
            ctx,
            cancel,
            Event::BalanceFailed {
                r#gen,
                text: failed("balance", &why),
            },
        ),
    }
}

fn measure(ctx: &Ctx, r#gen: Gen, draft: &Draft, cancel: &AtomicBool) {
    let t0 = Instant::now();
    let (base, sheet) = match fight_of(draft, "measured") {
        Ok(f) => f,
        Err(text) => return send(ctx, cancel, Event::BalanceFailed { r#gen, text }),
    };
    let h = draft.handicap;
    let setup_for = |seed: u64| under(&base, h, seed);
    match bf_story::stats_until(&setup_for, &sheet, confirm_seeds(), ctx.workers(), cancel) {
        Ok(s) => send(
            ctx,
            cancel,
            Event::Measured {
                r#gen,
                stats: stats_view(&s, t0.elapsed().as_secs_f64()),
            },
        ),
        Err(Halt::Cancelled) => {}
        Err(Halt::Failed(why)) => send(
            ctx,
            cancel,
            Event::BalanceFailed {
                r#gen,
                text: failed("measurement", &why),
            },
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::thread;
    use std::time::Duration;

    use super::*;
    use crate::engine::tests::starting_draft;
    use crate::engine::{test_engine, wait_for};
    use crate::seam::EngineHandle;

    /// Every event that arrives within `seconds`, polling every 10 ms (for "nothing more comes").
    fn events_for(handle: &EngineHandle, seconds: f64) -> Vec<Event> {
        let until = Instant::now() + Duration::from_secs_f64(seconds);
        let mut seen = Vec::new();
        while Instant::now() < until {
            while let Some(e) = handle.poll() {
                seen.push(e);
            }
            thread::sleep(Duration::from_millis(10));
        }
        seen
    }

    fn is_balance_event(e: &Event) -> bool {
        matches!(
            e,
            Event::BalanceProgress { .. } | Event::Balanced { .. } | Event::Measured { .. }
        )
    }

    fn draft_with(handicap: [f64; 2]) -> Draft {
        Draft {
            handicap,
            ..starting_draft()
        }
    }

    #[test]
    fn stats_view_carries_every_number() {
        let mut ko_hist = [0u32; bf_story::KO_BINS];
        for (i, b) in ko_hist.iter_mut().enumerate() {
            *b = 3 * i as u32 + 1;
        }
        let s = Stats {
            n: 3_000,
            decided: 2_950,
            win_share: 0.47,
            ko_median: 50.6,
            ko_p10: 44.2,
            ko_p90: 56.9,
            no_ko_share: 0.0167,
            lead_changes_mean: 2.6,
            fit_rate: 0.08,
            ko_hist,
        };
        let v = stats_view(&s, 1.4);
        assert_eq!(
            v,
            BalanceStats {
                fights: 3_000,
                win_share: 0.47,
                ko_median: 50.6,
                ko_p10: 44.2,
                ko_p90: 56.9,
                no_ko_share: 0.0167,
                lead_changes_mean: 2.6,
                fit_rate: 0.08,
                ko_hist: ko_hist.to_vec(),
                seconds: 1.4,
            }
        );
        assert_eq!(v.ko_hist.len(), 28);
        // no time on screen is negative or not a number
        assert_eq!(stats_view(&s, f64::NAN).seconds, 0.0);
        assert_eq!(stats_view(&s, -1.0).seconds, 0.0);
    }

    #[test]
    fn the_confirmation_seeds_are_the_balances() {
        let seeds = confirm_seeds();
        assert_eq!(seeds.start, (1 << 40) + 1_000_000);
        assert_eq!(seeds.end - seeds.start, 3_000);
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn the_episodes_handicap_measures_in_range() {
        let (engine, root) = test_engine();
        engine.send(Command::Measure {
            r#gen: 3,
            draft: draft_with([2.0, 0.4]),
        });
        let Event::Measured { r#gen, stats } = wait_for(&engine, 120.0, |e| {
            assert!(
                !matches!(e, Event::Problem { .. } | Event::BalanceFailed { .. }),
                "{e:?}"
            );
            matches!(e, Event::Measured { .. })
        }) else {
            unreachable!()
        };
        eprintln!("measure: {:.2} s, {stats:?}", stats.seconds);
        assert_eq!(r#gen, 3);
        assert_eq!(stats.fights, 3_000);
        assert!(stats.in_range(), "{stats:?}");
        assert_eq!(stats.ko_hist.len(), 28);
        assert!(stats.seconds > 0.0);
        engine.send(Command::Shutdown);
        std::fs::remove_dir(&root).expect("the engine left its temp folder empty");
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn auto_balance_reports_and_lands() {
        let (engine, root) = test_engine();
        engine.send(Command::AutoBalance {
            r#gen: 5,
            draft: draft_with([1.0, 1.0]),
        });
        let progress = RefCell::new(Vec::new());
        let Event::Balanced { r#gen, handicap, stats } = wait_for(&engine, 300.0, |e| {
            assert!(
                !matches!(e, Event::Problem { .. } | Event::BalanceFailed { .. }),
                "{e:?}"
            );
            if let Event::BalanceProgress { r#gen, done, total } = e {
                assert_eq!(*r#gen, 5);
                progress.borrow_mut().push((*done, *total));
            }
            matches!(e, Event::Balanced { .. })
        }) else {
            unreachable!()
        };
        eprintln!("auto-balance: {:.2} s, {handicap:?}, {stats:?}", stats.seconds);
        let progress = progress.into_inner();
        assert!(!progress.is_empty());
        assert!(progress.iter().all(|&(_, total)| total == 29), "{progress:?}");
        assert!(progress.windows(2).all(|w| w[0].0 < w[1].0), "{progress:?}");
        assert_eq!(progress.last(), Some(&(29, 29)));
        assert_eq!(r#gen, 5);
        assert_eq!(stats.fights, 3_000);
        assert!(stats.in_range(), "{stats:?}");
        let ratio = handicap[0] / handicap[1];
        assert!((2.5..=10.0).contains(&ratio), "{handicap:?}");
        engine.send(Command::Shutdown);
        std::fs::remove_dir(&root).expect("the engine left its temp folder empty");
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn a_newer_request_silences_the_older() {
        let (engine, root) = test_engine();
        engine.send(Command::AutoBalance {
            r#gen: 1,
            draft: draft_with([1.0, 1.0]),
        });
        engine.send(Command::Measure {
            r#gen: 2,
            draft: draft_with([2.0, 0.4]),
        });
        wait_for(&engine, 120.0, |e| {
            assert!(
                !matches!(e, Event::Problem { .. } | Event::BalanceFailed { .. }),
                "{e:?}"
            );
            assert!(!matches!(e, Event::Balanced { .. }), "the older run answered");
            matches!(e, Event::Measured { r#gen: 2, .. })
        });
        for e in events_for(&engine, 3.0) {
            assert!(!is_balance_event(&e), "a late answer: {e:?}");
        }
        engine.send(Command::Shutdown);
        std::fs::remove_dir(&root).expect("the engine left its temp folder empty");
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine::"
    )]
    fn stop_silences_it() {
        let (engine, root) = test_engine();
        engine.send(Command::AutoBalance {
            r#gen: 1,
            draft: draft_with([1.0, 1.0]),
        });
        wait_for(&engine, 120.0, |e| matches!(e, Event::BalanceProgress { .. }));
        engine.send(Command::StopBalance);
        // what was already on its way may still arrive; after that, nothing more (a step takes well under 4 s)
        let _ = events_for(&engine, 1.0);
        for e in events_for(&engine, 4.0) {
            assert!(!is_balance_event(&e), "the stopped run answered: {e:?}");
        }
        // the handler takes the next request as usual
        engine.send(Command::Measure {
            r#gen: 2,
            draft: draft_with([2.0, 0.4]),
        });
        wait_for(&engine, 120.0, |e| matches!(e, Event::Measured { r#gen: 2, .. }));
        engine.send(Command::Shutdown);
        std::fs::remove_dir(&root).expect("the engine left its temp folder empty");
    }

    #[test]
    fn a_handicap_that_cannot_fight_ends_the_run_in_words() {
        let (engine, root) = test_engine();
        engine.send(Command::Measure {
            r#gen: 1,
            draft: draft_with([0.0, 1.0]),
        });
        let failed = |e: &Event| matches!(e, Event::BalanceFailed { .. } | Event::Problem { .. });
        let Event::BalanceFailed { r#gen, text } = wait_for(&engine, 10.0, failed) else {
            panic!("a failed run is answered BalanceFailed, not Problem");
        };
        assert_eq!(r#gen, 1);
        assert_eq!(
            text,
            "This handicap cannot be measured: each fighter's handicap must be a number above 0."
        );
        engine.send(Command::AutoBalance {
            r#gen: 2,
            draft: draft_with([1.0, f64::INFINITY]),
        });
        let Event::BalanceFailed { r#gen, text } = wait_for(&engine, 10.0, failed) else {
            panic!("a failed run is answered BalanceFailed, not Problem");
        };
        assert_eq!(r#gen, 2);
        assert_eq!(
            text,
            "This handicap cannot be balanced: each fighter's handicap must be a number above 0."
        );
        // a draft that is no fight at all ends the run the same way
        let mut lost = draft_with([1.0, 1.0]);
        lost.fighters[1] = "PIANO".to_string();
        engine.send(Command::AutoBalance { r#gen: 3, draft: lost });
        let Event::BalanceFailed { r#gen, text } = wait_for(&engine, 10.0, failed) else {
            panic!("a failed run is answered BalanceFailed, not Problem");
        };
        assert_eq!(
            (r#gen, text.as_str()),
            (3, "No music is written for Flute vs Piano yet.")
        );
        engine.send(Command::Shutdown);
        std::fs::remove_dir(&root).expect("the engine left its temp folder empty");
    }
}
