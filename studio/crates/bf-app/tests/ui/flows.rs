//! Whole flows through the real app, driven by clicks: on the sample engine (`fake::spawn(.., Pace::Instant)`),
//! storage under a temp folder and no sound device, or on a seam the test answers by hand.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bf_app::app::{BfApp, Boot};
use bf_app::fixtures::render;
use bf_app::model::queue::{JobState, Queue};
use bf_app::model::shortlist::{SearchState, Shortlist};
use bf_app::model::{Notice, Settings, Step, View};
use bf_app::preview::audio::NoAudio;
use bf_app::seam::fake::{self, Pace};
use bf_app::seam::{Command, EngineHandle, EngineSide, Event, channel, sample};
use bf_app::store::queue_file::load_queue;
use bf_app::store::settings::{load_settings, save_settings};
use bf_app::store::{Paths, Writer};
use egui::Rect;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common::{REF, app_harness, gpu};

/// How long a flow waits for the engine, at most. Every wait is on a condition; this only bounds it.
const PATIENCE: Duration = Duration::from_secs(30);
/// The cores the tests say the PC has.
const CORES: usize = 8;
const INTERRUPTED: &str =
    "1 video was interrupted when the app closed. It is back in the queue. Resume the queue to render it.";

/// A fresh folder under the system temp folder, removed when dropped: the app's files in tests live here, never in
/// the real `%APPDATA%`, `%LOCALAPPDATA%` or Videos folders.
struct Temp(PathBuf);

impl Temp {
    fn new(test: &str) -> Temp {
        let root = std::env::temp_dir().join(format!("bf-app flows {test} {}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Temp(root)
    }

    fn paths(&self) -> Paths {
        Paths::under(&self.0)
    }

    /// The settings the app starts with: videos go under the temp folder, 2 workers.
    fn settings(&self) -> Settings {
        Settings {
            output_folder: self.0.join("Videos"),
            workers: 2,
        }
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// What the app starts with on `engine`: the temp folder's settings and writer, no sound device, an empty queue.
fn boot(engine: EngineHandle, temp: &Temp) -> Boot {
    Boot {
        engine,
        writer: Some(Writer::start(temp.paths())),
        audio: Box::new(NoAudio::new("No sound device in tests.")),
        settings: temp.settings(),
        queue: Queue::new(),
        notices: Vec::new(),
        cores: CORES,
    }
}

/// The real app on the sample engine, answering at once, made where the window's context exists.
fn sample_app<'a>(temp: &Temp) -> Harness<'a, BfApp> {
    let settings = temp.settings();
    let paths = temp.paths();
    app_harness(REF, move |cc| {
        let ctx = cc.egui_ctx.clone();
        let engine = fake::spawn(Arc::new(move || ctx.request_repaint()), Pace::Instant);
        BfApp::new(
            cc,
            Boot {
                engine,
                writer: Some(Writer::start(paths)),
                audio: Box::new(NoAudio::new("No sound device in tests.")),
                settings,
                queue: Queue::new(),
                notices: Vec::new(),
                cores: CORES,
            },
        )
    })
}

/// The real app on a seam whose engine side the test holds.
fn hand_app<'a>(make: impl FnOnce(EngineHandle) -> Boot + 'a) -> (Harness<'a, BfApp>, EngineSide) {
    let (handle, side) = channel(Arc::new(|| {}));
    let harness = app_harness(REF, move |cc| BfApp::new(cc, make(handle)));
    (harness, side)
}

/// Steps the app until `done` holds, bounded by `PATIENCE`, then fails with `what` if it still does not.
fn wait_for(harness: &mut Harness<'_, BfApp>, what: &str, done: impl Fn(&Harness<'_, BfApp>) -> bool) {
    let deadline = Instant::now() + PATIENCE;
    while !done(harness) && Instant::now() < deadline {
        harness.step();
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(done(harness), "waited for {what}");
}

/// Every text on screen and where it is: names, and a label's text (its value). Nodes with no place (the window's
/// root) are left out.
fn texts(harness: &Harness<'_, BfApp>) -> Vec<(String, Rect)> {
    harness
        .query_all_by(|n| n.bounding_box().is_some())
        .flat_map(|n| {
            let node = n.accesskit_node();
            let rect = n.rect();
            [node.label(), node.value()]
                .into_iter()
                .flatten()
                .map(move |t| (t, rect))
        })
        .collect()
}

/// Whether some text on screen inside `within` starts with `start`.
fn shows_in(harness: &Harness<'_, BfApp>, start: &str, within: Rect) -> bool {
    texts(harness)
        .iter()
        .any(|(t, rect)| t.starts_with(start) && within.contains_rect(*rect))
}

/// Whether some text on screen starts with `start`.
fn shows(harness: &Harness<'_, BfApp>, start: &str) -> bool {
    texts(harness).iter().any(|(t, _)| t.starts_with(start))
}

/// The enabled button named `label`, if it is on screen.
fn button<'t>(harness: &'t Harness<'_, BfApp>, label: &'t str) -> Option<egui_kittest::Node<'t>> {
    harness
        .query_all_by_label(label)
        .find(|n| n.accesskit_node().role() == Role::Button && !n.accesskit_node().is_disabled())
}

/// Waits for the button named `label` to be on screen and enabled, clicks it, and lets the click land.
fn press(harness: &mut Harness<'_, BfApp>, label: &str) {
    wait_for(harness, &format!("the button {label:?}"), |h| {
        button(h, label).is_some()
    });
    button(harness, label).expect("the button").click();
    harness.step();
    harness.step();
}

/// The shortlist's rows on screen, top to bottom: their names.
fn rows(harness: &Harness<'_, BfApp>) -> Vec<String> {
    let mut rows: Vec<(String, Rect)> = harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| {
            let label = n.accesskit_node().label()?;
            (label.starts_with("Seed ") && label.contains(',')).then(|| (label, n.rect()))
        })
        .collect();
    rows.sort_by(|a, b| a.1.min.y.total_cmp(&b.1.min.y));
    rows.into_iter().map(|(label, _)| label).collect()
}

/// The rect of the queue row of the video on `seed`.
fn job_row(harness: &Harness<'_, BfApp>, seed: u64) -> Option<Rect> {
    let name = format!("Flute vs Trombone, seed {seed}");
    harness
        .query_all_by_label(&name)
        .find(|n| n.accesskit_node().role() == Role::Button)
        .map(|n| n.rect())
}

/// Whether the queue row of `seed` shows a line starting with `start`.
fn row_reads(harness: &Harness<'_, BfApp>, seed: u64, start: &str) -> bool {
    job_row(harness, seed).is_some_and(|row| shows_in(harness, start, row))
}

/// Every command that has arrived on the engine side since the last call.
fn received(side: &EngineSide) -> Vec<Command> {
    side.commands.try_iter().collect()
}

/// Waits for the picture of the take of `seed`: until it is in, "Use seed" has no fight to go on with.
fn wait_for_take(harness: &mut Harness<'_, BfApp>, seed: u64) {
    wait_for(harness, &format!("the take of seed {seed}"), |h| {
        h.state().state.vm.take.story.as_ref().is_some_and(|s| s.seed == seed)
    });
}

/// Picks the shortlist row of `seed` on Seeds, uses it, goes on to Render and adds it to the queue.
fn queue_seed(harness: &mut Harness<'_, BfApp>, seed: u64) {
    press(harness, "3 Seeds");
    let start = format!("Seed {seed},");
    wait_for(harness, &format!("the row of seed {seed}"), |h| {
        rows(h).iter().any(|r| r.starts_with(&start))
    });
    let row = rows(harness)
        .into_iter()
        .find(|r| r.starts_with(&start))
        .expect("the row");
    press(harness, &row);
    wait_for_take(harness, seed);
    press(harness, &format!("Use seed {seed}"));
    press(harness, "Go to render");
    press(harness, "Add to queue");
}

#[test]
fn a_whole_video_in_clicks() {
    let _gpu = gpu();
    let temp = Temp::new("whole");
    let mut harness = sample_app(&temp);
    press(&mut harness, "Balance this match-up");
    wait_for(&mut harness, "the balance numbers", |h| {
        h.state().state.vm.balance.stats.is_some() && h.query_all_by_value("×2.00").next().is_some()
    });
    assert!(shows(&harness, "Flute wins"), "the numbers are on screen");
    assert_eq!(harness.state().state.vm.draft.handicap, [2.0, 0.4]);
    press(&mut harness, "Search seeds");
    wait_for(&mut harness, "fourteen rows", |h| rows(h).len() == 14);
    let first = rows(&harness).swap_remove(0);
    assert!(first.starts_with("Seed 18433,"), "{first}");
    press(&mut harness, &first);
    wait_for_take(&mut harness, 18433);
    press(&mut harness, "Use seed 18433");
    assert_eq!(harness.state().state.vm.view, View::Step(Step::Music));
    press(&mut harness, "Go to render");
    assert_eq!(harness.state().state.vm.view, View::Step(Step::Render));
    press(&mut harness, "Add to queue");
    wait_for(&mut harness, "the video done", |h| row_reads(h, 18433, "Done in"));
    let queue_buttons: Vec<String> = harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| n.accesskit_node().label())
        .filter(|l| l.starts_with("Queue"))
        .collect();
    assert_eq!(queue_buttons, ["Queue"], "nothing renders or waits, so no count");
    let slowest = harness.state().slowest_update_ms;
    assert!(slowest < 20.0, "the slowest update took {slowest} ms");
    harness.state_mut().shutdown();
}

#[test]
fn the_bad_job_stops_and_the_next_one_runs() {
    let _gpu = gpu();
    let temp = Temp::new("bad job");
    let mut harness = sample_app(&temp);
    queue_seed(&mut harness, 902);
    queue_seed(&mut harness, 18433);
    wait_for(&mut harness, "both videos finished", |h| {
        row_reads(h, 902, "Stopped at the loudness check") && row_reads(h, 18433, "Done in")
    });
    let states: Vec<bool> = harness
        .state()
        .state
        .vm
        .queue
        .jobs
        .iter()
        .map(|j| matches!(j.state, JobState::Failed { .. }))
        .collect();
    assert_eq!(states, [true, false], "the first failed, the second did not");
    harness.state_mut().shutdown();
}

#[test]
fn a_late_shortlist_is_not_shown() {
    let _gpu = gpu();
    let temp = Temp::new("late");
    let (mut harness, side) = hand_app(|engine| boot(engine, &temp));
    side.emit(Event::Roster(sample::roster()));
    harness.run_ok();
    press(&mut harness, "3 Seeds");
    let searches = |side: &EngineSide| -> Vec<u64> {
        received(side)
            .into_iter()
            .filter_map(|c| match c {
                Command::StartSearch { r#gen, .. } => Some(r#gen),
                _ => None,
            })
            .collect()
    };
    let asked = searches(&side);
    assert_eq!(asked.len(), 1, "one search was asked for");
    let late = asked[0];
    press(&mut harness, "1 Match-up");
    press(&mut harness, "Swap sides");
    assert_ne!(
        harness.state().state.generation(),
        late,
        "swapping began a new generation"
    );
    let list = |state| Shortlist {
        rows: sample::stories(),
        fits_total: 212,
        checked: 18_240,
        state,
        workers: 7,
    };
    side.emit(Event::Shortlist {
        r#gen: late,
        list: list(SearchState::Searching),
    });
    harness.run_ok();
    press(&mut harness, "3 Seeds");
    harness.run_ok();
    assert!(
        rows(&harness).is_empty(),
        "no row of the late list: {:?}",
        rows(&harness)
    );
    assert!(harness.state().state.vm.shortlist.rows.is_empty());
    // The same list for the search now asked for is shown, so the check above can fail.
    let now = searches(&side);
    assert_eq!(now.len(), 1, "opening Seeds again asked for the new fight's search");
    side.emit(Event::Shortlist {
        r#gen: now[0],
        list: list(SearchState::Stopped),
    });
    wait_for(&mut harness, "the current list", |h| rows(h).len() == 14);
    harness.state_mut().shutdown();
}

#[test]
fn a_seed_with_no_ko_says_so() {
    let _gpu = gpu();
    let temp = Temp::new("no ko");
    let mut harness = sample_app(&temp);
    press(&mut harness, "3 Seeds");
    wait_for(&mut harness, "the watch box", |h| {
        h.query_all_by_label("Seed to watch").next().is_some()
    });
    harness.get_by_label("Seed to watch").click();
    harness.step();
    harness.get_by_label("Seed to watch").type_text("13");
    harness.step();
    press(&mut harness, "Watch");
    wait_for(&mut harness, "the take's problem", |h| {
        h.state().state.vm.take.problem.is_some()
    });
    harness.run_ok();
    let problem = harness.state().state.vm.take.problem.clone().expect("a problem");
    assert!(problem.contains("no K.O."), "{problem}");
    let stage = harness.state().layout.stage;
    assert!(shows_in(&harness, &problem, stage), "the stage reads {problem:?}");
    let use_seed = harness
        .query_all_by_label("Use seed 13")
        .find(|n| n.accesskit_node().role() == Role::Button)
        .expect("the Use seed button");
    assert!(
        use_seed.accesskit_node().is_disabled(),
        "a fight with no K.O. cannot be used"
    );
    harness.state_mut().shutdown();
}

#[test]
fn the_queue_survives_a_restart() {
    let _gpu = gpu();
    let temp = Temp::new("restart");
    let paths = temp.paths();
    std::fs::create_dir_all(&paths.config).unwrap();
    save_settings(&paths.settings(), &temp.settings()).unwrap();
    let (mut harness, side) = hand_app(|engine| boot(engine, &temp));
    side.emit(Event::Roster(sample::roster()));
    let colors = [[0xde, 0xe8, 0xf6], [0x78, 0xf0, 0xb8]];
    for seed in [18433, 5627] {
        side.emit(Event::Enqueued {
            spec: render::spec("Flute vs Trombone", colors, "Big top", seed),
        });
    }
    wait_for(&mut harness, "both videos in the queue", |h| {
        h.state().state.vm.queue.jobs.len() == 2
    });
    let runs = received(&side)
        .into_iter()
        .filter(|c| matches!(c, Command::RunJob { .. }))
        .count();
    assert_eq!(runs, 1, "the first video started rendering");
    harness.state_mut().shutdown();
    drop(harness);
    drop(side);

    let (settings, notice) = load_settings(&paths.settings(), CORES);
    assert_eq!((settings.clone(), notice), (temp.settings(), None));
    let (queue, notice) = load_queue(&paths.queue());
    assert_eq!(notice, None);
    let (mut harness, side) = hand_app(|engine| Boot {
        settings,
        queue,
        ..boot(engine, &temp)
    });
    harness.run_ok();
    let vm = &harness.state().state.vm;
    assert_eq!(vm.queue.jobs.len(), 2);
    assert!(
        vm.queue.jobs.iter().all(|j| j.state == JobState::Waiting),
        "both videos wait"
    );
    assert_eq!(
        vm.notices,
        vec![Notice {
            text: INTERRUPTED.to_string()
        }]
    );
    assert!(shows(&harness, INTERRUPTED), "the notice is on screen");
    received(&side);
    press(&mut harness, "5 Render");
    press(&mut harness, "Resume queue");
    assert!(
        received(&side).iter().any(|c| matches!(c, Command::RunJob { .. })),
        "resuming runs the next video"
    );
    harness.state_mut().shutdown();
}

#[test]
fn reopening_from_the_library() {
    let _gpu = gpu();
    let temp = Temp::new("reopen");
    let mut harness = sample_app(&temp);
    press(&mut harness, "Library");
    let tile = |h: &Harness<'_, BfApp>| {
        h.query_all_by_role(Role::Button)
            .filter_map(|n| n.accesskit_node().label())
            .find(|l| l.starts_with("Flute vs Trombone, "))
    };
    wait_for(&mut harness, "the library's videos", |h| tile(h).is_some());
    let name = tile(&harness).expect("the tile");
    press(&mut harness, &name);
    press(&mut harness, "Reopen");
    wait_for(&mut harness, "the fight opened", |h| {
        h.state().state.vm.view == View::Step(Step::Matchup) && h.state().state.vm.draft.seed == Some(18433)
    });
    harness.run_ok();
    let opened = "Opened Flute vs Trombone, seed 18433";
    assert_eq!(harness.state().state.vm.status, opened);
    let status = harness.state().layout.status;
    assert!(shows_in(&harness, opened, status), "the status bar reads {opened:?}");
    harness.state_mut().shutdown();
}

#[test]
fn the_sample_app_says_so_in_the_status_bar() {
    let _gpu = gpu();
    let temp = Temp::new("note");
    let settings = temp.settings();
    let paths = temp.paths();
    let mut harness = app_harness(REF, move |cc| {
        let ctx = cc.egui_ctx.clone();
        let engine = fake::spawn(Arc::new(move || ctx.request_repaint()), Pace::Instant);
        let mut app = BfApp::new(
            cc,
            Boot {
                engine,
                writer: Some(Writer::start(paths)),
                audio: Box::new(NoAudio::new("No sound device in tests.")),
                settings,
                queue: Queue::new(),
                notices: Vec::new(),
                cores: CORES,
            },
        );
        app.status_note = Some("Sample data, not a real fight".to_string());
        app
    });
    wait_for(&mut harness, "the roster", |h| h.state().state.vm.roster.is_some());
    harness.run_ok();
    let status = harness.state().layout.status;
    assert!(shows_in(&harness, "Sample data, not a real fight", status));
    assert!(
        !shows_in(&harness, "30 fighters", status),
        "the note stands where the counts would"
    );
    assert!(
        shows_in(&harness, "Flute vs Trombone, not balanced yet", status),
        "the status line is still there"
    );
    harness.state_mut().shutdown();
}
