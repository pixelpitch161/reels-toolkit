//! The app state's unit tests: they drive `dispatch` and `apply` only.

use std::sync::Arc;

use super::*;
use crate::model::shortlist::Finish;
use crate::model::take::AudioTake;
use crate::seam::sample;

fn settings() -> Settings {
    Settings {
        output_folder: PathBuf::from("Videos").join("BALLFIGHT"),
        workers: 7,
        encode: crate::model::EncodeChoice::Best,
    }
}

fn queue() -> Queue {
    Queue {
        jobs: Vec::new(),
        paused: false,
        next_id: 1,
    }
}

/// The commands among `effects`, in order.
fn sent(effects: &[Effect]) -> Vec<Command> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::Send(c) => Some(c.clone()),
            _ => None,
        })
        .collect()
}

/// The player commands among `effects`, in order.
fn played(effects: &[Effect]) -> Vec<&PlayerCmd> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::Player(p) => Some(p),
            _ => None,
        })
        .collect()
}

fn count(effects: &[Effect], what: fn(&Command) -> bool) -> usize {
    sent(effects).iter().filter(|c| what(c)).count()
}

fn is_measure(c: &Command) -> bool {
    matches!(c, Command::Measure { .. })
}

fn is_auto_balance(c: &Command) -> bool {
    matches!(c, Command::AutoBalance { .. })
}

fn is_start_search(c: &Command) -> bool {
    matches!(c, Command::StartSearch { .. })
}

fn is_describe(c: &Command) -> bool {
    matches!(c, Command::DescribeFight { .. })
}

/// A new app that has its roster: FLUTE vs TROMBONE, generation 1.
fn ready() -> AppState {
    let (mut s, _) = AppState::new(settings(), queue(), Vec::new(), 8);
    s.apply(Event::Roster(sample::roster()));
    s
}

fn go(s: &mut AppState, step: Step) -> Vec<Effect> {
    s.dispatch(Action::Go(View::Step(step)))
}

fn handicap(side: usize, value: f64, released: bool) -> Action {
    Action::SetHandicap { side, value, released }
}

fn pick(side: usize, name: &str) -> Action {
    Action::PickFighter {
        side,
        name: name.to_string(),
    }
}

fn fight_info(generation: Gen, captions: CaptionTexts) -> Event {
    Event::FightInfo {
        r#gen: generation,
        pair_rules: vec!["A live lance knocks a risen cobra flat.".to_string()],
        captions,
        title_card: None,
        problem: None,
        music: None,
    }
}

fn sample_list() -> Shortlist {
    Shortlist {
        rows: sample::stories(),
        fits_total: 212,
        checked: 18_240,
        state: SearchState::Searching,
        workers: 7,
    }
}

fn story_with(seed: u64, t_fight: f64, winner: Option<usize>) -> StoryView {
    StoryView {
        t_fight,
        winner,
        ..sample::story(seed)
    }
}

fn picture(take: TakeId, story: StoryView) -> Event {
    Event::TakePicture {
        take,
        story,
        frames: sample::frames([[0xde, 0xe8, 0xf6], [0x78, 0xf0, 0xb8]]),
    }
}

fn quiet() -> AudioTake {
    AudioTake {
        samples: Arc::from(vec![[0.0f32; 2]; 4]),
    }
}

/// The take number the last `LoadTake` among `effects` asked for.
fn take_of(effects: &[Effect]) -> TakeId {
    sent(effects)
        .iter()
        .rev()
        .find_map(|c| match c {
            Command::LoadTake { take, .. } => Some(*take),
            _ => None,
        })
        .expect("a LoadTake")
}

#[test]
fn starts_by_asking_for_the_roster() {
    let (s, fx) = AppState::new(settings(), queue(), Vec::new(), 8);
    assert_eq!(s.vm.view, View::Step(Step::Matchup));
    assert_eq!(
        sent(&fx),
        vec![
            Command::LoadRoster,
            Command::CheckTools,
            Command::CheckFolder {
                root: settings().output_folder,
                name: String::new(),
            },
        ]
    );
    assert_eq!(fx.len(), 3);
    assert_eq!(s.generation(), 0);
}

#[test]
fn the_roster_sets_the_starting_pair() {
    let (mut s, _) = AppState::new(settings(), queue(), Vec::new(), 8);
    let fx = s.apply(Event::Roster(sample::roster()));
    let d = &s.vm.draft;
    assert_eq!(d.fighters, ["FLUTE".to_string(), "TROMBONE".to_string()]);
    assert_eq!(d.theme, "big_top");
    assert_eq!(d.bars, 34);
    assert_eq!(d.handicap, [1.0, 1.0]);
    assert_eq!(d.seed, None);
    assert_eq!(d.chord_loop, 0);
    assert_eq!(d.music_seed, 1);
    assert_eq!(d.bass, Some(1));
    assert_eq!(s.generation(), 1);
    assert_eq!(count(&fx, is_describe), 1);
    // The starting pair's folder is checked (Task 12).
    assert!(
        matches!(&sent(&fx)[..], [Command::DescribeFight { r#gen: 1, draft }, Command::CheckFolder { name, .. }]
        if draft == &s.vm.draft && name == "flute_vs_trombone")
    );
    assert_eq!(s.vm.status, "Flute vs Trombone, not balanced yet");
}

#[test]
fn picking_the_other_corners_fighter_swaps() {
    let mut s = ready();
    s.dispatch(handicap(0, 2.0, false));
    s.dispatch(handicap(1, 0.4, false));
    s.dispatch(pick(0, "TROMBONE"));
    let d = &s.vm.draft;
    assert_eq!(d.fighters, ["TROMBONE".to_string(), "FLUTE".to_string()]);
    assert_eq!(d.handicap, [0.4, 2.0]);
    assert_eq!(d.bass, Some(0));
}

#[test]
fn a_new_fighter_resets_the_handicap_and_the_base() {
    let mut s = ready();
    s.dispatch(handicap(0, 2.0, false));
    // No event in this task opens a fight file, so the base is set by hand.
    s.vm.draft.base = Some("format = 1".to_string());
    s.dispatch(Action::SetCaption {
        platform: Platform::Shorts,
        text: "Mine".to_string(),
    });
    assert!(s.vm.draft.captions_edited);
    let fx = s.dispatch(pick(1, "PIANO"));
    let d = &s.vm.draft;
    assert_eq!(d.fighters, ["FLUTE".to_string(), "PIANO".to_string()]);
    assert_eq!(d.handicap, [1.0, 1.0]);
    assert_eq!(d.base, None);
    assert!(!d.captions_edited);
    assert_eq!(d.bass, None, "neither FLUTE nor PIANO can play bass");
    assert_eq!(count(&fx, is_describe), 1);
    // Picking the fighter already in that corner changes nothing.
    let generation = s.generation();
    assert!(s.dispatch(pick(1, "PIANO")).is_empty());
    assert_eq!(s.generation(), generation);
}

#[test]
fn every_fight_input_starts_a_new_generation() {
    let actions = [
        pick(0, "OBOE"),
        Action::SwapSides,
        Action::SetBars(30),
        handicap(1, 0.7, false),
    ];
    for action in actions {
        let mut s = ready();
        go(&mut s, Step::Seeds);
        s.apply(Event::Shortlist {
            r#gen: s.generation(),
            list: sample_list(),
        });
        s.dispatch(Action::SelectSeed(18433));
        s.apply(Event::Measured {
            r#gen: s.generation(),
            stats: sample::stats(),
        });
        assert!(s.vm.balance.stats.is_some());
        go(&mut s, Step::Matchup);
        let before = s.generation();
        let fx = s.dispatch(action.clone());
        assert_eq!(s.generation(), before + 1, "{action:?}");
        assert_eq!(s.vm.draft.seed, None, "{action:?}");
        assert!(s.vm.shortlist.rows.is_empty(), "{action:?}");
        assert_eq!(s.vm.shortlist.state, SearchState::Idle, "{action:?}");
        assert_eq!(s.vm.balance.stats, None, "{action:?}");
        assert_eq!(s.vm.take, TakeView::default(), "{action:?}");
        assert_eq!(count(&fx, |c| matches!(c, Command::StopSearch)), 1, "{action:?}");
        assert_eq!(count(&fx, is_start_search), 0, "{action:?}: not on Seeds");
        assert_eq!(played(&fx).len(), 1, "{action:?}");
        let fighters_changed = !matches!(action, Action::SetBars(_) | Action::SetHandicap { .. });
        assert_eq!(count(&fx, is_describe), usize::from(fighters_changed), "{action:?}");
    }
}

#[test]
fn music_settings_keep_the_generation() {
    let mut s = ready();
    let before = s.generation();
    for action in [
        Action::SetTheme("big_top".to_string()),
        Action::SetBass(1),
        Action::SetChordLoop(1),
    ] {
        let fx = s.dispatch(action.clone());
        assert_eq!(s.generation(), before, "{action:?}");
        assert!(sent(&fx).is_empty(), "{action:?}");
    }
    assert_eq!(s.vm.draft.chord_loop, 1);

    // With a seed chosen, a music setting asks for a new take of the same seed.
    s.dispatch(Action::SelectSeed(18433));
    let fx = s.dispatch(Action::SetChordLoop(0));
    assert_eq!(s.generation(), before);
    assert_eq!(s.vm.draft.seed, Some(18433));
    assert!(matches!(
        &sent(&fx)[..],
        [Command::LoadTake { take: 2, draft }] if draft.seed == Some(18433) && draft.chord_loop == 0
    ));
}

#[test]
fn notes_that_are_a_weapon_feed_the_fight() {
    let mut s = ready();
    s.dispatch(pick(1, "PIANO"));
    let before = s.generation();
    s.dispatch(Action::SetChordLoop(1));
    assert_eq!(s.generation(), before + 1);
    assert_eq!(s.vm.draft.chord_loop, 1);
}

#[test]
fn late_results_are_dropped() {
    let mut s = ready();
    go(&mut s, Step::Seeds);
    let first = take_of(&s.dispatch(Action::SelectSeed(20418)));
    let second = take_of(&s.dispatch(Action::SelectSeed(7731)));
    assert_ne!(first, second);
    let old_take = [
        picture(first, sample::story(20418)),
        Event::TakeSound {
            take: first,
            audio: quiet(),
            music: sample::music(),
        },
        Event::TakeFailed {
            take: first,
            problem: "Too late.".to_string(),
        },
    ];
    for event in old_take {
        let before = s.vm.clone();
        let shown = format!("{event:?}");
        let fx = s.apply(event);
        assert_eq!(s.vm, before, "{shown}");
        assert!(fx.is_empty(), "{shown}");
    }

    let old = s.generation();
    s.dispatch(Action::SwapSides);
    assert_ne!(s.generation(), old);
    s.dispatch(Action::AutoBalance);
    let old_fight = [
        fight_info(old, sample::captions()),
        Event::BalanceProgress {
            r#gen: old,
            done: 3,
            total: 29,
        },
        Event::Balanced {
            r#gen: old,
            handicap: [2.0, 0.4],
            stats: sample::stats(),
        },
        Event::Measured {
            r#gen: old,
            stats: sample::stats(),
        },
        Event::Shortlist {
            r#gen: old,
            list: sample_list(),
        },
    ];
    for event in old_fight {
        let before = s.vm.clone();
        let shown = format!("{event:?}");
        let fx = s.apply(event);
        assert_eq!(s.vm, before, "{shown}");
        assert!(fx.is_empty(), "{shown}");
    }
    // After the swap, the second seed's take is late too.
    for event in [
        picture(second, sample::story(7731)),
        Event::TakeSound {
            take: second,
            audio: quiet(),
            music: sample::music(),
        },
        Event::TakeFailed {
            take: second,
            problem: "Too late.".to_string(),
        },
    ] {
        let before = s.vm.clone();
        let shown = format!("{event:?}");
        let fx = s.apply(event);
        assert_eq!(s.vm, before, "{shown}");
        assert!(fx.is_empty(), "{shown}");
    }
}

#[test]
fn releasing_the_handicap_measures() {
    let mut s = ready();
    let fx = s.dispatch(handicap(0, 2.004, false));
    assert!(sent(&fx).is_empty());
    assert_eq!(s.vm.draft.handicap, [2.0, 1.0]);
    let fx = s.dispatch(handicap(0, 2.0, true));
    assert_eq!(sent(&fx).len(), 1);
    assert!(matches!(&sent(&fx)[0], Command::Measure { r#gen, .. } if *r#gen == s.generation()));
    s.dispatch(handicap(1, 99.0, false));
    assert_eq!(s.vm.draft.handicap, [2.0, 12.0]);
    s.dispatch(handicap(1, 0.0, false));
    assert_eq!(s.vm.draft.handicap, [2.0, 0.05]);

    let fx = s.dispatch(Action::ResetHandicap);
    assert_eq!(s.vm.draft.handicap, [1.0, 1.0]);
    assert_eq!(count(&fx, is_measure), 1);
    assert!(matches!(&sent(&fx)[..], [Command::Measure { r#gen, draft }]
        if *r#gen == s.generation() && draft.handicap == [1.0, 1.0]));
}

#[test]
fn opening_balance_balances_once() {
    let mut s = ready();
    let fx = go(&mut s, Step::Balance);
    assert_eq!(s.vm.view, View::Step(Step::Balance));
    assert_eq!(s.vm.step, Step::Balance);
    assert_eq!(count(&fx, is_auto_balance), 1);
    assert_eq!(sent(&fx).len(), 1);
    assert!(s.vm.balance.running.is_some());
    go(&mut s, Step::Matchup);
    assert!(sent(&go(&mut s, Step::Balance)).is_empty());

    // With a handicap of its own the fight is measured, once.
    let mut s = ready();
    s.dispatch(handicap(0, 2.0, false));
    let fx = go(&mut s, Step::Balance);
    assert_eq!(count(&fx, is_measure), 1);
    assert_eq!(sent(&fx).len(), 1);
    go(&mut s, Step::Matchup);
    assert!(sent(&go(&mut s, Step::Balance)).is_empty());

    // With numbers already there, nothing is asked.
    let mut s = ready();
    s.apply(Event::Measured {
        r#gen: s.generation(),
        stats: sample::stats(),
    });
    assert!(sent(&go(&mut s, Step::Balance)).is_empty());
}

#[test]
fn a_balanced_handicap_keeps_its_numbers() {
    let mut s = ready();
    go(&mut s, Step::Balance);
    let old = s.generation();
    s.apply(Event::BalanceProgress {
        r#gen: old,
        done: 9,
        total: 29,
    });
    assert_eq!(s.vm.balance.running, Some((9, 29)));
    let fx = s.apply(Event::Balanced {
        r#gen: old,
        handicap: [2.0, 0.4],
        stats: sample::stats(),
    });
    assert_eq!(s.vm.draft.handicap, [2.0, 0.4]);
    assert_eq!(s.generation(), old + 1);
    assert_eq!(s.vm.balance.stats, Some(sample::stats()));
    assert_eq!(s.vm.balance.running, None);
    assert!(
        sent(&fx).is_empty(),
        "a finished balance is not stopped, and Balance starts no search"
    );
    assert_eq!(s.vm.status, "Balanced on 3,000 fights in 1.4 s");

    // The numbers belong to the new generation: a measurement for the old one is late.
    let before = s.vm.clone();
    s.apply(Event::Measured {
        r#gen: old,
        stats: BalanceStats {
            win_share: 0.9,
            ..sample::stats()
        },
    });
    assert_eq!(s.vm, before);

    // The same handicap keeps the generation.
    let mut s = ready();
    go(&mut s, Step::Balance);
    let generation = s.generation();
    s.apply(Event::Balanced {
        r#gen: generation,
        handicap: [1.0, 1.0],
        stats: sample::stats(),
    });
    assert_eq!(s.generation(), generation);
    assert_eq!(s.vm.balance.stats, Some(sample::stats()));
}

#[test]
fn a_stopped_balance_ignores_what_was_on_its_way() {
    let mut s = ready();
    go(&mut s, Step::Balance);
    let fx = s.dispatch(Action::StopBalance);
    assert!(matches!(&sent(&fx)[..], [Command::StopBalance]));
    assert_eq!(s.vm.balance.running, None);
    let before = s.vm.clone();
    for event in [
        Event::BalanceProgress {
            r#gen: s.generation(),
            done: 10,
            total: 29,
        },
        Event::Balanced {
            r#gen: s.generation(),
            handicap: [2.0, 0.4],
            stats: sample::stats(),
        },
    ] {
        assert!(s.apply(event).is_empty());
        assert_eq!(s.vm, before);
    }
}

#[test]
fn a_failed_balance_ends_on_screen() {
    let text = "The balance could not finish: a fight broke off.";
    // A balance run that fails: nothing runs, the reason is a notice, and opening Balance does not retry.
    let mut s = ready();
    go(&mut s, Step::Balance);
    assert!(s.vm.balance.running.is_some());
    let fx = s.apply(Event::BalanceFailed {
        r#gen: s.generation(),
        text: text.to_string(),
    });
    assert!(fx.is_empty(), "{fx:?}");
    assert_eq!(s.vm.balance.running, None);
    assert_eq!(s.vm.balance.stats, None);
    assert_eq!(s.vm.notices, vec![Notice { text: text.to_string() }]);
    go(&mut s, Step::Matchup);
    assert!(sent(&go(&mut s, Step::Balance)).is_empty(), "no retry loop");
    // The button still starts one by hand.
    assert_eq!(count(&s.dispatch(Action::AutoBalance), is_auto_balance), 1);

    // A measurement that fails ends the same way.
    let mut s = ready();
    s.dispatch(handicap(0, 2.0, false));
    assert_eq!(count(&go(&mut s, Step::Balance), is_measure), 1);
    s.apply(Event::BalanceFailed {
        r#gen: s.generation(),
        text: text.to_string(),
    });
    go(&mut s, Step::Matchup);
    assert!(sent(&go(&mut s, Step::Balance)).is_empty(), "no retry loop");
    assert_eq!(s.vm.notices.len(), 1);

    // A failure of another generation changes nothing.
    let mut s = ready();
    go(&mut s, Step::Balance);
    let before = s.vm.clone();
    assert!(
        s.apply(Event::BalanceFailed {
            r#gen: s.generation() + 1,
            text: text.to_string(),
        })
        .is_empty()
    );
    assert_eq!(s.vm, before);
}

#[test]
fn opening_seeds_starts_one_search() {
    let mut s = ready();
    let fx = go(&mut s, Step::Seeds);
    assert_eq!(count(&fx, is_start_search), 1);
    assert_eq!(sent(&fx).len(), 1);
    assert!(matches!(&sent(&fx)[0], Command::StartSearch { r#gen, filters, .. }
        if *r#gen == s.generation() && *filters == FilterSpec::default()));
    assert_eq!(s.vm.shortlist.state, SearchState::Searching);
    assert_eq!(s.vm.status, "Searching on 7 cores");
    assert!(sent(&go(&mut s, Step::Seeds)).is_empty());
    go(&mut s, Step::Music);
    assert!(sent(&go(&mut s, Step::Seeds)).is_empty());

    // The shortlist arrives; stopping and going on.
    s.apply(Event::Shortlist {
        r#gen: s.generation(),
        list: sample_list(),
    });
    assert_eq!(s.vm.shortlist, sample_list());
    let fx = s.dispatch(Action::StopSearch);
    assert!(matches!(&sent(&fx)[..], [Command::StopSearch]));
    assert_eq!(s.vm.shortlist.state, SearchState::Stopped);
    let fx = s.dispatch(Action::SearchOn);
    assert_eq!(count(&fx, is_start_search), 1);
    assert_eq!(s.vm.shortlist.state, SearchState::Searching);
}

#[test]
fn a_new_fight_on_seeds_restarts_the_search() {
    let mut s = ready();
    s.dispatch(Action::AutoBalance);
    go(&mut s, Step::Seeds);
    let old = s.generation();
    let fx = s.apply(Event::Balanced {
        r#gen: old,
        handicap: [2.0, 0.4],
        stats: sample::stats(),
    });
    assert_eq!(count(&fx, is_start_search), 1);
    assert!(
        sent(&fx)
            .iter()
            .any(|c| matches!(c, Command::StartSearch { r#gen, draft, .. }
        if *r#gen == old + 1 && draft.handicap == [2.0, 0.4]))
    );
    assert_eq!(s.vm.shortlist.state, SearchState::Searching);

    let mut s = ready();
    go(&mut s, Step::Balance);
    let fx = s.apply(Event::Balanced {
        r#gen: s.generation(),
        handicap: [2.0, 0.4],
        stats: sample::stats(),
    });
    assert_eq!(count(&fx, is_start_search), 0);
    assert_eq!(s.vm.shortlist.state, SearchState::Idle);
    // It restarts when Seeds is next shown.
    assert_eq!(count(&go(&mut s, Step::Seeds), is_start_search), 1);
}

#[test]
fn choosing_a_seed_asks_for_its_take() {
    let mut s = ready();
    go(&mut s, Step::Seeds);
    let fx = s.dispatch(Action::SelectSeed(18433));
    assert_eq!(s.vm.draft.seed, Some(18433));
    assert!(s.vm.take.loading);
    // The output folder is checked again for the new seed (Task 12).
    assert!(
        matches!(&sent(&fx)[..], [Command::LoadTake { take: 1, draft }, Command::CheckFolder { .. }]
        if draft.seed == Some(18433))
    );
    let fx = s.dispatch(Action::SelectSeed(20418));
    assert!(
        matches!(&sent(&fx)[..], [Command::LoadTake { take: 2, draft }, Command::CheckFolder { .. }]
        if draft.seed == Some(20418))
    );
}

#[test]
fn the_picture_starts_at_fight() {
    for (step, autoplay) in [(Step::Seeds, true), (Step::Music, false)] {
        let mut s = ready();
        go(&mut s, step);
        let take = take_of(&s.dispatch(Action::SelectSeed(18433)));
        let fx = s.apply(picture(take, story_with(18433, 3.6, Some(1))));
        match &played(&fx)[..] {
            [
                PlayerCmd::Load {
                    take: t,
                    start,
                    autoplay: a,
                    ..
                },
            ] => {
                assert_eq!(*t, take);
                assert_eq!(*start, 108);
                assert_eq!(*a, autoplay, "{step:?}");
            }
            other => panic!("expected one Load, got {other:?}"),
        }
        assert!(!s.vm.take.loading);
        assert_eq!(s.vm.take.story.as_ref().map(|t| t.seed), Some(18433));
        let status = if step == Step::Seeds {
            "Searching on 7 cores"
        } else {
            "Seed 18433 is ready"
        };
        assert_eq!(s.vm.status, status, "the search outranks the seed");

        let fx = s.apply(Event::TakeSound {
            take,
            audio: quiet(),
            music: sample::music(),
        });
        assert!(matches!(&played(&fx)[..], [PlayerCmd::Sound { take: t, .. }] if *t == take));
        assert_eq!(s.vm.take.music, Some(sample::music()));
    }
}

#[test]
fn a_failed_take_says_why() {
    let mut s = ready();
    let take = take_of(&s.dispatch(Action::SelectSeed(613)));
    let fx = s.apply(Event::TakeFailed {
        take,
        problem: "Seed 613 has no K.O. before 58 s.".to_string(),
    });
    assert_eq!(s.vm.take.problem.as_deref(), Some("Seed 613 has no K.O. before 58 s."));
    assert!(!s.vm.take.loading);
    assert_eq!(s.vm.status, "Seed 613 has no K.O. before 58 s.");
    assert!(matches!(&played(&fx)[..], [PlayerCmd::Clear]), "no title card yet");
}

#[test]
fn a_seed_with_no_winner_cannot_be_used() {
    let mut s = ready();
    go(&mut s, Step::Seeds);
    s.dispatch(Action::UseSeed);
    assert_eq!(s.vm.view, View::Step(Step::Seeds), "no take yet");
    let take = take_of(&s.dispatch(Action::SelectSeed(902)));
    s.apply(picture(take, story_with(902, 3.6, None)));
    s.dispatch(Action::UseSeed);
    assert_eq!(s.vm.view, View::Step(Step::Seeds));
    let take = take_of(&s.dispatch(Action::SelectSeed(18433)));
    s.apply(picture(take, story_with(18433, 3.6, Some(1))));
    s.dispatch(Action::UseSeed);
    assert_eq!(s.vm.view, View::Step(Step::Music));
    assert_eq!(s.vm.step, Step::Music);
}

#[test]
fn drafted_captions_arrive_until_edited() {
    let mut s = ready();
    s.apply(fight_info(s.generation(), sample::captions()));
    assert_eq!(s.vm.draft.captions, sample::captions());
    assert_eq!(
        s.vm.pair_rules,
        vec!["A live lance knocks a risen cobra flat.".to_string()]
    );
    s.dispatch(Action::SetCaption {
        platform: Platform::Shorts,
        text: "Mine".to_string(),
    });
    assert!(s.vm.draft.captions_edited);
    s.dispatch(Action::SetBars(30));
    let drafted = CaptionTexts {
        instagram: "Other".to_string(),
        tiktok: "Other".to_string(),
        shorts: "Other".to_string(),
    };
    s.apply(fight_info(s.generation(), drafted.clone()));
    assert_eq!(s.vm.draft.captions.shorts, "Mine");
    assert_eq!(s.vm.draft.captions.instagram, sample::captions().instagram);

    // A new fighter forgets the edit, so the next drafted captions arrive.
    s.dispatch(pick(0, "OBOE"));
    s.apply(fight_info(s.generation(), drafted.clone()));
    assert_eq!(s.vm.draft.captions, drafted);
}

#[test]
fn a_description_is_of_the_fighters() {
    // The fighters were described for generation 1; dragging a handicap starts generation 2, and the
    // description is still of these two fighters, so it is not dropped.
    let mut s = ready();
    s.dispatch(handicap(0, 2.0, false));
    assert_eq!(s.generation(), 2);
    s.apply(fight_info(1, sample::captions()));
    assert_eq!(s.vm.draft.captions, sample::captions());
    assert_eq!(s.vm.pair_rules.len(), 1);
}

#[test]
fn the_caption_line_is_shared() {
    let mut s = ready();
    s.apply(fight_info(s.generation(), sample::captions()));
    s.dispatch(Action::SetCaption {
        platform: Platform::TikTok,
        text: "New line\n#fyp".to_string(),
    });
    let c = &s.vm.draft.captions;
    assert_eq!(c.tiktok, "New line\n#fyp");
    let (_, tags) = sample::captions()
        .instagram
        .split_once('\n')
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .unwrap();
    assert_eq!(c.instagram, format!("New line\n{tags}"));
    assert_eq!(c.shorts, sample::captions().shorts);

    s.dispatch(Action::SetCaption {
        platform: Platform::Instagram,
        text: "Other line\n#ig".to_string(),
    });
    assert_eq!(s.vm.draft.captions.tiktok, "Other line\n#fyp");
}

#[test]
fn filters_are_kept_in_bounds_and_sent() {
    let mut s = ready();
    let spec = FilterSpec {
        ko: (60.0, 20.0),
        finish: Finish::Close,
        ..FilterSpec::default()
    };
    let fx = s.dispatch(Action::SetFilters(spec));
    assert_eq!(s.vm.filters.ko, (30.0, 58.0));
    assert_eq!(s.vm.filters.finish, Finish::Close);
    assert!(matches!(&sent(&fx)[..], [Command::SetFilters { r#gen, filters }]
        if *r#gen == s.generation() && *filters == s.vm.filters));
    let fx = s.dispatch(Action::SetFilters(FilterSpec {
        ko: (52.0, 50.0),
        ..FilterSpec::default()
    }));
    assert_eq!(s.vm.filters.ko, (50.0, 52.0));
    assert_eq!(sent(&fx).len(), 1);
}

#[test]
fn a_fight_problem_holds_every_request() {
    let mut s = ready();
    s.apply(Event::FightInfo {
        r#gen: s.generation(),
        pair_rules: Vec::new(),
        captions: sample::captions(),
        title_card: None,
        problem: Some("These two cannot fight yet.".to_string()),
        music: None,
    });
    assert_eq!(s.vm.fight_problem.as_deref(), Some("These two cannot fight yet."));
    assert_eq!(s.vm.status, "These two cannot fight yet.");
    assert!(sent(&go(&mut s, Step::Balance)).is_empty());
    assert!(sent(&s.dispatch(Action::AutoBalance)).is_empty());
    assert!(sent(&s.dispatch(handicap(0, 2.0, true))).is_empty());
    assert!(sent(&s.dispatch(Action::ResetHandicap)).is_empty());
    assert!(sent(&go(&mut s, Step::Seeds)).is_empty());
    assert!(sent(&s.dispatch(Action::SearchOn)).is_empty());
    assert!(sent(&s.dispatch(Action::SelectSeed(18433))).is_empty());
    assert_eq!(s.vm.shortlist.state, SearchState::Idle);
    assert_eq!(s.vm.balance.running, None);
    assert_eq!(s.vm.status, "These two cannot fight yet.");
}

#[test]
fn the_title_card_shows_while_there_is_no_take() {
    let mut s = ready();
    let card = sample::title_card(s.vm.colors(), [&sample::mask("FLUTE"), &sample::mask("TROMBONE")]);
    let fx = s.apply(Event::FightInfo {
        r#gen: s.generation(),
        pair_rules: Vec::new(),
        captions: sample::captions(),
        title_card: Some(card.clone()),
        problem: None,
        music: None,
    });
    assert!(matches!(&played(&fx)[..], [PlayerCmd::Still(f)] if *f == card));
    let fx = s.dispatch(handicap(0, 2.0, false));
    assert!(matches!(&played(&fx)[..], [PlayerCmd::Still(f)] if *f == card));
    let fx = s.dispatch(Action::SwapSides);
    assert!(
        matches!(&played(&fx)[..], [PlayerCmd::Clear]),
        "the card shows the other order"
    );
}

#[test]
fn transport_goes_to_the_player_only() {
    let mut s = ready();
    let before = s.vm.clone();
    let fx = s.dispatch(Action::Transport(TransportAction::Step(1)));
    assert!(matches!(
        &fx[..],
        [Effect::Player(PlayerCmd::Transport(TransportAction::Step(1)))]
    ));
    assert_eq!(s.vm, before);
}

#[test]
fn the_view_model_names_its_fighters() {
    let s = ready();
    assert_eq!(s.vm.names(), ["Flute".to_string(), "Trombone".to_string()]);
    assert_eq!(s.vm.colors(), [[0xde, 0xe8, 0xf6], [0x78, 0xf0, 0xb8]]);
    assert_eq!(s.vm.theme().map(|t| t.display.as_str()), Some("Big top"));
    assert_eq!(s.vm.fighter(1).map(|f| f.name.as_str()), Some("TROMBONE"));
    assert!(s.vm.fighter(2).is_none());
}

#[test]
fn showing_keeps_the_view_model() {
    let vm = ready().vm;
    let mut s = AppState::showing(vm.clone());
    assert_eq!(s.vm, vm);
    assert_eq!(s.generation(), 1);
    let fx = s.dispatch(Action::SetCaptionTab(Platform::TikTok));
    assert!(fx.is_empty(), "a fixture's fight is not a new one");
    assert_eq!(s.vm.caption_tab, Platform::TikTok);
}

// ---------------------------------------------------------------------------------------------------------------
// Fix round 1
// ---------------------------------------------------------------------------------------------------------------

/// A new app with `roster`.
fn ready_with(roster: Roster) -> AppState {
    let (mut s, _) = AppState::new(settings(), queue(), Vec::new(), 8);
    s.apply(Event::Roster(roster));
    s
}

/// The sample roster with a second theme, "dark_waltz".
fn two_themes() -> Roster {
    let mut roster = sample::roster();
    let second = ThemeEntry {
        name: "dark_waltz".to_string(),
        display: "Dark waltz".to_string(),
        chord_loops: vec![vec!["Am".to_string(), "E7".to_string()]],
        ..roster.themes[0].clone()
    };
    roster.themes.push(second);
    roster
}

/// Gives the generation everything it can hold: a search with rows, numbers, a chosen seed with its picture.
fn with_everything(s: &mut AppState) {
    go(s, Step::Seeds);
    s.apply(Event::Shortlist {
        r#gen: s.generation(),
        list: sample_list(),
    });
    s.apply(Event::Measured {
        r#gen: s.generation(),
        stats: sample::stats(),
    });
    let take = take_of(&s.dispatch(Action::SelectSeed(18433)));
    s.apply(picture(take, sample::story(18433)));
    go(s, Step::Matchup);
}

fn assert_cleared(s: &AppState, what: &str) {
    assert_eq!(s.vm.draft.seed, None, "{what}");
    assert!(s.vm.shortlist.rows.is_empty(), "{what}");
    assert_eq!(s.vm.shortlist.state, SearchState::Idle, "{what}");
    assert_eq!(s.vm.balance, BalanceView::default(), "{what}");
    assert_eq!(s.vm.take, TakeView::default(), "{what}");
}

#[test]
fn the_theme_feeds_a_note_weapon_fight() {
    let mut s = ready_with(two_themes());
    s.dispatch(pick(1, "PIANO"));
    with_everything(&mut s);
    let before = s.generation();
    let fx = s.dispatch(Action::SetTheme("dark_waltz".to_string()));
    assert_eq!(s.vm.draft.theme, "dark_waltz");
    assert_eq!(s.generation(), before + 1);
    assert_cleared(&s, "PIANO and a new theme");
    assert_eq!(count(&fx, |c| matches!(c, Command::StopSearch)), 1);

    // With FLUTE and TROMBONE the theme is music only.
    let mut s = ready_with(two_themes());
    with_everything(&mut s);
    let before = s.generation();
    let fx = s.dispatch(Action::SetTheme("dark_waltz".to_string()));
    assert_eq!(s.generation(), before);
    assert_eq!(s.vm.draft.seed, Some(18433));
    assert!(matches!(&sent(&fx)[..], [Command::LoadTake { draft, .. }] if draft.theme == "dark_waltz"));
}

#[test]
fn an_opened_draft_always_begins_a_new_generation() {
    let mut s = ready();
    go(&mut s, Step::Seeds);
    let before = s.generation();
    let opened = Draft {
        seed: Some(20418),
        ..s.vm.draft.clone()
    };
    let mut fx = Vec::new();
    s.open_draft(opened, &mut fx);
    assert_eq!(
        s.generation(),
        before + 1,
        "the same fighters, bars and handicap still bump"
    );
    assert_eq!(s.vm.draft.seed, Some(20418), "the seed survives");
    assert_eq!(count(&fx, is_describe), 1);
    assert_eq!(count(&fx, |c| matches!(c, Command::StopSearch)), 1);
    let searches: Vec<Command> = sent(&fx).into_iter().filter(is_start_search).collect();
    assert!(matches!(&searches[..], [Command::StartSearch { r#gen, .. }] if *r#gen == before + 1));
    assert_eq!(
        count(&fx, |c| matches!(c, Command::LoadTake { .. })),
        0,
        "the caller asks for the take"
    );

    // Settling afterwards asks for nothing more; the caller's take request carries the seed.
    let mut more = Vec::new();
    s.settle(&mut more);
    assert!(more.is_empty());
    s.ask_take(&mut more);
    assert!(matches!(&sent(&more)[..], [Command::LoadTake { draft, .. }] if draft.seed == Some(20418)));

    // A description for the generation before the opening is late.
    // (The opened seed's folder check, which this direct call skipped, goes out with the next event: Task 12.)
    let before_info = s.vm.clone();
    let fx = s.apply(fight_info(before, sample::captions()));
    assert!(matches!(&sent(&fx)[..], [Command::CheckFolder { .. }]), "{fx:?}");
    assert_eq!(fx.len(), 1);
    assert_eq!(s.vm, before_info);
}

#[test]
fn a_second_roster_begins_cleanly() {
    let mut s = ready();
    let take = take_of(&s.dispatch(Action::SelectSeed(18433)));
    s.apply(picture(take, sample::story(18433)));
    let before = s.generation();
    let fx = s.apply(Event::Roster(sample::roster()));
    assert_eq!(s.generation(), before + 1);
    assert_cleared(&s, "a second roster");
    assert_eq!(count(&fx, is_describe), 1);
    let after = s.vm.clone();
    assert!(
        s.apply(picture(take, sample::story(18433))).is_empty(),
        "the old take is late"
    );
    assert_eq!(s.vm, after);
}

#[test]
fn a_description_of_the_other_order_is_dropped_after_a_swap_and_back() {
    let mut s = ready();
    assert_eq!(s.generation(), 1);
    s.dispatch(Action::SwapSides);
    s.dispatch(Action::SwapSides);
    assert_eq!(s.generation(), 3);
    for old in [1, 2] {
        let before = s.vm.clone();
        assert!(
            s.apply(fight_info(old, sample::captions())).is_empty(),
            "generation {old}"
        );
        assert_eq!(s.vm, before, "generation {old}");
    }
    s.apply(fight_info(3, sample::captions()));
    assert_eq!(s.vm.draft.captions, sample::captions());
}

#[test]
fn a_late_searching_update_keeps_the_stop() {
    let mut s = ready();
    go(&mut s, Step::Seeds);
    s.dispatch(Action::StopSearch);
    s.apply(Event::Shortlist {
        r#gen: s.generation(),
        list: sample_list(),
    });
    assert_eq!(s.vm.shortlist.state, SearchState::Stopped);
    assert_eq!(s.vm.shortlist.rows, sample::stories(), "the rows are taken");
    assert_eq!(s.vm.status, "Flute vs Trombone, not balanced yet");
}

#[test]
fn keep_searching_acts_only_when_no_search_runs() {
    let mut s = ready();
    go(&mut s, Step::Seeds);
    assert!(sent(&s.dispatch(Action::SearchOn)).is_empty(), "already searching");
    s.apply(Event::Shortlist {
        r#gen: s.generation(),
        list: Shortlist {
            state: SearchState::Full,
            ..sample_list()
        },
    });
    assert!(sent(&s.dispatch(Action::SearchOn)).is_empty(), "the shortlist is full");
}

#[test]
fn a_fight_problem_stops_what_runs() {
    let mut s = ready();
    go(&mut s, Step::Seeds);
    s.dispatch(Action::AutoBalance);
    let fx = s.apply(Event::FightInfo {
        r#gen: s.generation(),
        pair_rules: Vec::new(),
        captions: sample::captions(),
        title_card: None,
        problem: Some("These two cannot fight yet.".to_string()),
        music: None,
    });
    assert_eq!(count(&fx, |c| matches!(c, Command::StopSearch)), 1);
    assert_eq!(count(&fx, |c| matches!(c, Command::StopBalance)), 1);
    assert_eq!(s.vm.shortlist.state, SearchState::Stopped);
    assert_eq!(s.vm.balance.running, None);
}

#[test]
fn a_stopped_balance_is_not_restarted_by_opening_balance() {
    let mut s = ready();
    go(&mut s, Step::Balance);
    s.dispatch(Action::StopBalance);
    go(&mut s, Step::Matchup);
    assert!(sent(&go(&mut s, Step::Balance)).is_empty(), "the owner stopped it");
    assert_eq!(count(&s.dispatch(Action::AutoBalance), is_auto_balance), 1);
    s.dispatch(Action::StopBalance);
    let fx = s.dispatch(handicap(0, 1.0, true));
    assert_eq!(count(&fx, is_measure), 1, "a release still measures");

    // A new generation forgets the stop.
    let mut s = ready();
    go(&mut s, Step::Balance);
    s.dispatch(Action::StopBalance);
    go(&mut s, Step::Matchup);
    s.dispatch(Action::SetBars(30));
    assert_eq!(count(&go(&mut s, Step::Balance), is_auto_balance), 1);
}

#[test]
fn the_status_follows_one_order() {
    let mut s = ready();
    // 5. Nothing yet.
    assert_eq!(s.vm.status, "Flute vs Trombone, not balanced yet");
    // 4. Numbers.
    s.apply(Event::Measured {
        r#gen: s.generation(),
        stats: sample::stats(),
    });
    assert_eq!(s.vm.status, "Balanced on 3,000 fights in 1.4 s");
    // 3. A take outranks the numbers.
    go(&mut s, Step::Music);
    let take = take_of(&s.dispatch(Action::SelectSeed(18433)));
    s.apply(picture(take, sample::story(18433)));
    assert_eq!(s.vm.status, "Seed 18433 is ready");
    // 2. A running search outranks the take; stopping it gives the take back.
    go(&mut s, Step::Seeds);
    assert_eq!(s.vm.status, "Searching on 7 cores");
    s.dispatch(Action::StopSearch);
    assert_eq!(s.vm.status, "Seed 18433 is ready");
    // 1. A problem outranks everything: the take's, then the pair's.
    let take = take_of(&s.dispatch(Action::SelectSeed(613)));
    s.apply(Event::TakeFailed {
        take,
        problem: "Seed 613 cannot be made.".to_string(),
    });
    assert_eq!(s.vm.status, "Seed 613 cannot be made.");
    s.apply(Event::FightInfo {
        r#gen: s.generation(),
        pair_rules: Vec::new(),
        captions: sample::captions(),
        title_card: None,
        problem: Some("These two cannot fight yet.".to_string()),
        music: None,
    });
    assert_eq!(s.vm.status, "These two cannot fight yet.");

    // On Seeds, a Balanced that begins a new generation restarts the search, and the status says so.
    let mut s = ready();
    s.dispatch(Action::AutoBalance);
    go(&mut s, Step::Seeds);
    s.apply(Event::Balanced {
        r#gen: s.generation(),
        handicap: [2.0, 0.4],
        stats: sample::stats(),
    });
    assert_eq!(s.vm.status, "Searching on 7 cores");

    // Numbers with no real time leave the time out (widget rule 9).
    let mut s = ready();
    s.apply(Event::Measured {
        r#gen: s.generation(),
        stats: BalanceStats {
            seconds: f64::NAN,
            ..sample::stats()
        },
    });
    assert_eq!(s.vm.status, "Balanced on 3,000 fights");
}

#[test]
fn a_handicap_that_is_not_a_number_changes_nothing() {
    let mut s = ready();
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let before = s.vm.clone();
        let generation = s.generation();
        let fx = s.dispatch(handicap(0, value, true));
        assert!(fx.is_empty(), "{value}");
        assert_eq!(s.vm, before, "{value}");
        assert_eq!(s.generation(), generation, "{value}");
    }
}

#[test]
fn the_picture_start_is_kept_in_the_video() {
    for (t_fight, expected) in [
        (f64::NAN, 0),
        (f64::INFINITY, 0),
        (-2.0, 0),
        (3.65, 110),
        (1_000.0, 1_829),
    ] {
        let mut s = ready();
        let take = take_of(&s.dispatch(Action::SelectSeed(18433)));
        let fx = s.apply(picture(take, story_with(18433, t_fight, Some(1))));
        assert!(
            matches!(&played(&fx)[..], [PlayerCmd::Load { start, .. }] if *start == expected),
            "{t_fight}"
        );
    }
}

#[test]
fn a_new_fighter_stops_a_running_balance() {
    let mut s = ready();
    s.dispatch(Action::AutoBalance);
    let fx = s.dispatch(pick(0, "OBOE"));
    assert_eq!(count(&fx, |c| matches!(c, Command::StopBalance)), 1);
    assert_eq!(s.vm.balance.running, None);
}

#[test]
fn the_bars_stay_inside_the_theme() {
    let mut roster = sample::roster();
    roster.themes[0].bars = (24, 30);
    let mut s = ready_with(roster);
    assert_eq!(s.vm.draft.bars, 30, "34 is outside 24 to 30");
    s.dispatch(Action::SetBars(40));
    assert_eq!(s.vm.draft.bars, 30);
    s.dispatch(Action::SetBars(10));
    assert_eq!(s.vm.draft.bars, 24);
}

/// The transport as the player shows it with a take loaded at frame `frame`, with `sound`.
fn transport(frame: u32, sound: SoundState) -> TransportView {
    TransportView {
        enabled: true,
        frame,
        playing: false,
        looping: true,
        safe_zone: false,
        sound,
    }
}

#[test]
fn the_players_view_is_copied_and_its_lines_join_the_status_order() {
    let mut s = ready();
    let take = take_of(&s.dispatch(Action::SelectSeed(18433)));
    s.apply(picture(take, sample::story(18433)));
    assert_eq!(s.vm.status, "Seed 18433 is ready");

    s.player_view(transport(942, SoundState::Ready), None);
    assert_eq!(s.vm.transport, transport(942, SoundState::Ready), "the view is copied");
    assert_eq!(s.vm.status, "Seed 18433 is ready");

    // 6. A sound device that is missing or lost stands where the take's own line would.
    s.player_view(transport(942, SoundState::NoDevice), None);
    assert_eq!(s.vm.status, "No sound device was found, so the preview is silent.");

    // 2. A frame that could not be drawn outranks it, and the search, and goes when the player says it is drawn.
    s.player_view(transport(1200, SoundState::NoDevice), Some(1200));
    assert_eq!(s.vm.status, "Frame 1200 could not be drawn. Try another seed.");
    go(&mut s, Step::Seeds);
    assert_eq!(
        s.vm.status, "Frame 1200 could not be drawn. Try another seed.",
        "it outranks the search"
    );
    s.dispatch(Action::StopSearch);
    s.player_view(transport(1201, SoundState::NoDevice), None);
    assert_eq!(s.vm.status, "No sound device was found, so the preview is silent.");

    // 4. A running search outranks the silent device.
    s.dispatch(Action::SearchOn);
    assert_eq!(s.vm.status, "Searching on 7 cores");
    s.dispatch(Action::StopSearch);

    // 1. A problem outranks the failed frame.
    s.player_view(transport(1201, SoundState::Ready), Some(1201));
    let take = take_of(&s.dispatch(Action::SelectSeed(613)));
    s.apply(Event::TakeFailed {
        take,
        problem: "Seed 613 cannot be made.".to_string(),
    });
    assert_eq!(s.vm.status, "Seed 613 cannot be made.");
}

#[test]
fn the_same_view_again_changes_nothing() {
    let mut s = ready();
    s.player_view(transport(10, SoundState::None), None);
    let before = s.vm.clone();
    s.player_view(transport(10, SoundState::None), None);
    assert_eq!(s.vm, before);
}

/// A new app with the engine's own roster (milestone 3: the 20 kit fighters and 14 themes).
fn real() -> AppState {
    ready_with(crate::engine::content::roster())
}

/// Ruling M3-R6: a kit pair starts from the kit's frozen handicap, and the fighter whose part is the bass is on
/// bass; the starting pair is one too. A pair with no kit config starts even.
#[test]
fn a_kit_pair_starts_from_its_kit_handicap() {
    let mut s = real();
    assert_eq!(s.vm.draft.fighters, ["FLUTE".to_string(), "TROMBONE".to_string()]);
    assert_eq!(s.vm.draft.handicap, [2.0, 0.4]);
    assert_eq!(s.vm.draft.bass, Some(1));
    s.dispatch(pick(0, "BASS DRUM"));
    assert_eq!(
        s.vm.draft.handicap,
        [1.0, 1.0],
        "BASS DRUM vs TROMBONE has no kit config"
    );
    s.dispatch(pick(1, "TUBA"));
    assert_eq!(s.vm.draft.handicap, [0.86, 1.0]);
    assert_eq!(s.vm.draft.bass, Some(1), "the TUBA plays the bass");
    // the other order is the same config, swapped
    s.dispatch(pick(0, "CELLO"));
    s.dispatch(pick(1, "BASS DRUM"));
    s.dispatch(pick(0, "TUBA"));
    assert_eq!(s.vm.draft.fighters, ["TUBA".to_string(), "BASS DRUM".to_string()]);
    assert_eq!(s.vm.draft.handicap, [1.0, 0.86]);
    assert_eq!(s.vm.draft.bass, Some(0));
}

/// Note (b) of Task 60: when a new pair cannot play the draft's theme (ruling M3-R2), the draft moves to the pair's
/// starter theme and a notice says so; a theme the new pair can play stays.
#[test]
fn a_theme_the_new_pair_cannot_play_moves_to_the_starters() {
    let mut s = real();
    s.dispatch(pick(1, "CLARINET"));
    s.dispatch(Action::SetTheme("heist".to_string()));
    assert_eq!(s.vm.draft.theme, "heist");
    let before = s.vm.notices.len();
    s.dispatch(pick(1, "TROMBONE"));
    assert_eq!(s.vm.draft.theme, "big_top", "FLUTE vs TROMBONE's own theme");
    assert_eq!(s.vm.notices.len(), before + 1);
    assert_eq!(
        s.vm.notices.last().map(|n| n.text.as_str()),
        Some("Trombone cannot play in Heist, so the theme is now Big top.")
    );
    // a pair with no kit config moves to the theme its starter has: ALTO SAX's home is the dark disco, which
    // ORGAN cannot play, so the ORGAN's home theme
    s.dispatch(pick(0, "ALTO SAX"));
    s.dispatch(pick(1, "TENOR SAX"));
    s.dispatch(Action::SetTheme("dark_disco".to_string()));
    s.dispatch(pick(1, "ORGAN"));
    let starter = bf_content::starter("ALTO SAX", "ORGAN").expect("a starter");
    assert_eq!(s.vm.draft.theme, starter.music.theme);
    assert_eq!(
        s.vm.notices.last().map(|n| n.text.as_str()),
        Some("Organ cannot play in Dark disco, so the theme is now Prog duel.")
    );
    // a theme both can play stays, with no notice
    s.dispatch(Action::SetTheme("dark_tango".to_string()));
    let before = s.vm.notices.len();
    s.dispatch(pick(1, "OBOE"));
    assert_eq!(s.vm.draft.theme, "dark_tango");
    assert_eq!(s.vm.notices.len(), before);
}

/// A new theme starts on its own first loop, whatever loop the old theme was on.
#[test]
fn a_new_theme_starts_on_its_first_loop() {
    let mut s = real();
    s.dispatch(Action::SetTheme("brass_trailer".to_string()));
    s.dispatch(Action::SetChordLoop(1));
    assert_eq!(s.vm.draft.chord_loop, 1);
    s.dispatch(Action::SetTheme("dark_tango".to_string()));
    assert_eq!(s.vm.draft.chord_loop, 0);
}

/// HORN against TROMBONE has a kit config for each order: swapping sides starts from the other one (its handicap,
/// its parts and, in its theme, its loop); a pair with one config just swaps.
#[test]
fn swapping_a_pair_with_a_config_for_each_order_starts_from_the_other() {
    let mut s = real();
    s.dispatch(pick(0, "HORN"));
    assert_eq!(s.vm.draft.fighters, ["HORN".to_string(), "TROMBONE".to_string()]);
    let r = crate::engine::content::roster();
    let rematch = r.start_of(&s.vm.draft.fighters.clone()).expect("a start");
    s.dispatch(Action::SetTheme(rematch.theme.clone()));
    s.dispatch(Action::SwapSides);
    let first = r
        .start_of(&["TROMBONE".to_string(), "HORN".to_string()])
        .expect("a start");
    assert_eq!(s.vm.draft.fighters, ["TROMBONE".to_string(), "HORN".to_string()]);
    assert_eq!(s.vm.draft.handicap, first.handicap);
    assert_eq!(s.vm.draft.chord_loop, first.chord_loop);
    assert_ne!(
        rematch.chord_loop, first.chord_loop,
        "the two configs play the two loops"
    );
    // FLUTE vs TROMBONE has one config: a swap keeps each fighter's handicap
    s.dispatch(pick(1, "FLUTE"));
    assert_eq!(s.vm.draft.handicap, [0.4, 2.0]);
    s.dispatch(handicap(0, 1.5, false));
    s.dispatch(Action::SwapSides);
    assert_eq!(s.vm.draft.handicap, [2.0, 1.5]);
}

/// The way back to a kit pair's own handicap (ruling M3-R6), measured at once; a pair with none ignores it.
#[test]
fn the_series_handicap_comes_back() {
    let mut s = real();
    s.dispatch(handicap(0, 1.0, true));
    assert_eq!(s.vm.draft.handicap, [1.0, 0.4]);
    let fx = s.dispatch(Action::UseSeriesHandicap);
    assert_eq!(s.vm.draft.handicap, [2.0, 0.4]);
    assert!(sent(&fx).iter().any(|c| matches!(c, Command::Measure { .. })));
    s.dispatch(pick(1, "OBOE"));
    s.dispatch(handicap(0, 1.3, true));
    s.dispatch(Action::UseSeriesHandicap);
    assert_eq!(s.vm.draft.handicap, [1.3, 1.0]);
}

/// Review Focus 7: for a PIANO pair a theme (so its key) or loop change is a new fight, so the search starts over;
/// for any other pair it is not.
#[test]
fn only_a_piano_pairs_music_restarts_its_search() {
    let mut s = real();
    for (pair, restarts) in [(["PIANO", "ORGAN"], true), (["FLUTE", "TROMBONE"], false)] {
        s.dispatch(pick(0, pair[0]));
        s.dispatch(pick(1, pair[1]));
        s.dispatch(Action::SetTheme("big_top".to_string()));
        go(&mut s, Step::Seeds);
        let generation = s.generation();
        let fx = s.dispatch(Action::SetTheme("brass_trailer".to_string()));
        assert_eq!(s.generation() != generation, restarts, "{pair:?}: another theme");
        assert_eq!(
            count(&fx, is_start_search),
            usize::from(restarts),
            "{pair:?}: another theme"
        );
        let generation = s.generation();
        let fx = s.dispatch(Action::SetChordLoop(1));
        assert_eq!(s.vm.draft.chord_loop, 1);
        assert_eq!(s.generation() != generation, restarts, "{pair:?}: another loop");
        assert_eq!(
            count(&fx, is_start_search),
            usize::from(restarts),
            "{pair:?}: another loop"
        );
    }
}
