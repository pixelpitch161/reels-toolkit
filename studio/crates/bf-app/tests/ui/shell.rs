//! The shell: the frame every screen sits in (toolbar, bout bar, notice, stage, timeline, status bar), the keys, and
//! how a frame runs: events in `logic`, actions out of `ui`, effects carried out once.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bf_app::app::{self, BfApp, Boot, EngineKind};
use bf_app::fixtures::base;
use bf_app::model::queue::{JobSpec, Queue};
use bf_app::model::take::{SoundState, TransportAction};
use bf_app::model::{Notice, Settings, Step, View};
use bf_app::preview::audio::NoAudio;
use bf_app::screens::shell;
use bf_app::seam::{Command, EngineHandle, EngineSide, Event, channel, sample};
use bf_app::state::{Action, ViewModel};
use bf_app::store::settings::{load_settings, save_settings};
use bf_app::store::{Paths, Writer};
use bf_app::theme;
use bf_app::widgets::{buttons, fields};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common::{MIN, REF, app_harness, gpu, widget_harness};

/// A fresh folder under the system temp folder, removed when dropped: the app's files in tests live here, never in
/// the real `%APPDATA%` or `%LOCALAPPDATA%`.
struct Temp(PathBuf);

impl Temp {
    fn new(test: &str) -> Temp {
        let root = std::env::temp_dir().join(format!("bf-app shell {test} {}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Temp(root)
    }

    fn paths(&self) -> Paths {
        Paths::under(&self.0)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// What the app starts with on a hand-held seam: no writer, no sound device, 2 workers on 8 cores.
fn boot(engine: EngineHandle) -> Boot {
    Boot {
        engine,
        writer: None,
        audio: Box::new(NoAudio::new("No sound device in tests.")),
        settings: Settings {
            output_folder: PathBuf::from("Videos").join("BALLFIGHT"),
            workers: 2,
        },
        queue: Queue::new(),
        notices: Vec::new(),
        cores: 8,
    }
}

/// The real app on a seam whose engine side the test holds, with `writer`.
fn live<'a>(writer: Option<Writer>) -> (Harness<'a, BfApp>, EngineSide) {
    let (handle, side) = channel(Arc::new(|| {}));
    let harness = app_harness(REF, move |cc| BfApp::new(cc, Boot { writer, ..boot(handle) }));
    (harness, side)
}

/// The shell showing `vm` as it is.
fn showing<'a>(size: (f32, f32), vm: ViewModel) -> Harness<'a, BfApp> {
    app_harness(size, move |cc| BfApp::showing(cc, vm))
}

/// Every command that has arrived on the engine side since the last call.
fn received(side: &EngineSide) -> Vec<Command> {
    side.commands.try_iter().collect()
}

fn corners(r: egui::Rect) -> [[f32; 2]; 2] {
    [[r.min.x, r.min.y], [r.max.x, r.max.y]]
}

/// The labels of the toolbar, left to right.
const TOOLBAR: [&str; 10] = [
    "BALLFIGHT Studio",
    "1 Match-up",
    "2 Balance",
    "3 Seeds",
    "4 Music",
    "5 Render",
    "Library",
    "Queue",
    "Update",
    "Settings",
];

fn job(title: &str) -> JobSpec {
    JobSpec {
        title: title.to_string(),
        colors: [[0x68, 0x5c, 0xec], [0x80, 0x80, 0x8c]],
        theme: "Big top".to_string(),
        seed: 9120,
        fight_toml: String::new(),
        out_root: PathBuf::from("Videos").join("BALLFIGHT"),
    }
}

#[test]
fn the_harness_runs_logic() {
    let _gpu = gpu();
    let (mut harness, side) = live(None);
    harness.run();
    assert!(
        received(&side).contains(&Command::LoadRoster),
        "the start-up effects were carried out in logic"
    );
}

#[test]
fn showing_mode_keeps_the_fixtures_transport() {
    let _gpu = gpu();
    let mut vm = base::vm();
    vm.transport.frame = 942;
    let mut harness = showing(REF, vm);
    harness.run();
    assert_eq!(harness.state().state.vm.transport.frame, 942);
}

#[test]
fn shutdown_flushes() {
    let _gpu = gpu();
    let temp = Temp::new("shutdown");
    let paths = temp.paths();
    let (mut harness, side) = live(Some(Writer::start(paths.clone())));
    harness.run();
    // The worker count is a setting: its action returns a `SaveSettings` effect.
    harness.state_mut().act(Action::SetWorkers(3));
    harness.state_mut().shutdown();
    let (saved, notice) = load_settings(&paths.settings(), 8);
    assert_eq!(notice, None);
    assert_eq!(saved.workers, 3, "the settings file holds the new value");
    assert!(received(&side).contains(&Command::Shutdown));
}

#[test]
fn the_frame_has_the_reference_measurements() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::vm());
    harness.run();
    let l = harness.state().layout;
    assert_eq!(corners(l.toolbar), [[0.0, 0.0], [1440.0, 40.0]]);
    assert_eq!(corners(l.bout), [[0.0, 40.0], [1440.0, 88.0]]);
    assert_eq!(corners(l.workspace), [[0.0, 88.0], [1030.0, 764.0]]);
    assert_eq!(corners(l.stage), [[1030.0, 88.0], [1440.0, 764.0]]);
    assert_eq!(corners(l.timeline), [[0.0, 764.0], [1440.0, 876.0]]);
    assert_eq!(corners(l.status), [[0.0, 876.0], [1440.0, 900.0]]);
}

#[test]
fn the_frame_at_the_minimum_size() {
    let _gpu = gpu();
    let mut harness = showing(MIN, base::vm());
    harness.run();
    let l = harness.state().layout;
    assert_eq!(corners(l.workspace), [[0.0, 88.0], [870.0, 664.0]]);
    assert_eq!(l.stage.width(), 410.0);
    let window = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(MIN.0, MIN.1));
    for label in TOOLBAR {
        let r = harness.get_by_label(label).rect();
        assert!(window.contains_rect(r), "{label} at {r:?} lies inside the window");
        assert!(l.toolbar.contains_rect(r), "{label} at {r:?} lies inside the toolbar");
    }
}

#[test]
fn the_toolbar_names() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::vm());
    harness.run();
    for label in TOOLBAR {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    let marked = |harness: &Harness<'_, BfApp>, label: &str| {
        harness.get_by_label(label).accesskit_node().toggled() == Some(Toggled::True)
    };
    assert!(marked(&harness, "1 Match-up"), "the current step is the one marked");
    assert!(!marked(&harness, "2 Balance"));
}

#[test]
fn clicking_a_step_goes_there() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::vm());
    harness.run();
    harness.get_by_label("3 Seeds").click();
    harness.run();
    assert_eq!(harness.state().state.vm.view, View::Step(Step::Seeds));
    assert_eq!(harness.state().log, vec![Action::Go(View::Step(Step::Seeds))]);
    // In showing mode the commands are kept, not sent, and each is given once.
    let sent = harness.state_mut().sent();
    assert!(
        sent.iter().any(|c| matches!(c, Command::StartSearch { .. })),
        "{sent:?}"
    );
    assert!(harness.state_mut().sent().is_empty());
    for (label, view) in [
        ("Library", View::Library),
        ("Queue", View::Step(Step::Render)),
        ("Settings", View::Settings),
    ] {
        harness.get_by_label(label).click();
        harness.run();
        assert_eq!(harness.state().state.vm.view, view, "{label}");
    }
}

#[test]
fn keys_1_to_5_switch_steps() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::vm());
    harness.run();
    for (key, step) in [
        (egui::Key::Num2, Step::Balance),
        (egui::Key::Num3, Step::Seeds),
        (egui::Key::Num4, Step::Music),
        (egui::Key::Num5, Step::Render),
        (egui::Key::Num1, Step::Matchup),
    ] {
        harness.key_press(key);
        harness.run();
        assert_eq!(harness.state().state.vm.view, View::Step(step), "{key:?}");
    }
    // Ctrl and 3 is not a step key.
    harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::Num3);
    harness.run();
    assert_eq!(harness.state().state.vm.view, View::Step(Step::Matchup));
}

#[test]
fn step_keys_are_left_to_a_text_field_that_has_the_keyboard() {
    let _gpu = gpu();
    let mut text = String::new();
    let mut out = Vec::new();
    let mut harness = widget_harness((300.0, 80.0), |ui| {
        shell::keys(ui.ctx(), &mut out);
        fields::text_line(ui, "Caption", &mut text, "Write a caption", 200.0);
    });
    harness.run();
    harness.key_press(egui::Key::Num3);
    harness.run();
    harness.get_by_label("Caption").click();
    harness.run();
    // A real key press brings the key and its text.
    harness.key_press(egui::Key::Num2);
    harness.event(egui::Event::Text("2".to_string()));
    harness.run();
    drop(harness);
    assert_eq!(
        out,
        vec![Action::Go(View::Step(Step::Seeds))],
        "3 went to the shell, 2 to the field"
    );
    assert_eq!(text, "2");
}

#[test]
fn step_keys_still_work_when_tab_has_moved_to_a_button() {
    let _gpu = gpu();
    let mut out = Vec::new();
    let mut harness = widget_harness((300.0, 80.0), |ui| {
        shell::keys(ui.ctx(), &mut out);
        buttons::flat(ui, "Swap sides");
    });
    harness.run();
    harness.key_press(egui::Key::Tab);
    harness.run();
    assert!(
        harness.ctx.memory(|m| m.focused()).is_some(),
        "the button has the keyboard"
    );
    harness.key_press(egui::Key::Num4);
    harness.run();
    drop(harness);
    assert_eq!(out, vec![Action::Go(View::Step(Step::Music))]);
}

#[test]
fn update_is_disabled_and_says_why() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::vm());
    harness.run();
    assert!(harness.get_by_label("Update").accesskit_node().is_disabled());
    let why = "Updates from inside the app arrive in a later version.";
    assert!(harness.query_by_label(why).is_none());
    harness.get_by_label("Update").hover();
    harness.run();
    assert!(harness.query_by_label(why).is_some(), "hovering says why");
    harness.get_by_label("Update").click();
    harness.run();
    assert!(harness.state().log.is_empty(), "it does nothing");
}

#[test]
fn the_queue_count_shows_only_above_zero() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::vm());
    harness.run();
    assert!(harness.query_by_label("Queue").is_some());
    assert!(harness.query_by_label("Queue 0").is_none());
    drop(harness);

    let mut vm = base::vm();
    let rendering = vm.queue.add(job("Tuba vs Bass drum"));
    vm.queue.add(job("Alto sax vs Tenor sax"));
    vm.queue.start(rendering);
    assert_eq!(vm.queue.counts(), (1, 1));
    let mut harness = showing(REF, vm);
    harness.run();
    assert!(
        harness.query_by_label("Queue 2").is_some(),
        "one rendering and one waiting"
    );
    assert!(harness.query_by_label("Queue").is_none());
    harness.get_by_label("Queue 2").click();
    harness.run();
    assert_eq!(harness.state().state.vm.view, View::Step(Step::Render));
}

#[test]
fn the_bout_bar_reads_the_draft() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::vm());
    harness.run();
    // "Theme" and "Tempo" also stand in the Match-up workspace: look for them in the bout bar only.
    let bout = harness.state().layout.bout;
    for text in ["Theme", "Tempo"] {
        assert!(
            harness.query_all_by_label(text).any(|n| bout.contains_rect(n.rect())),
            "{text}"
        );
    }
    for text in ["Big top", "133.77 bpm", "Seed", "not picked yet", "VS"] {
        assert!(harness.query_by_label(text).is_some(), "{text}");
    }
    assert!(harness.query_by_label("Handicap").is_none(), "not on Match-up");
    let images: Vec<String> = harness
        .query_all_by_role(Role::Image)
        .filter_map(|n| n.accesskit_node().label())
        .collect();
    assert!(
        images.contains(&"Flute".to_string()) && images.contains(&"Trombone".to_string()),
        "{images:?}"
    );
    drop(harness);

    for step in [Step::Seeds, Step::Music, Step::Render] {
        let mut vm = base::vm();
        vm.view = View::Step(step);
        vm.step = step;
        vm.draft.handicap = [2.0, 0.4];
        vm.draft.seed = Some(18433);
        let mut harness = showing(REF, vm);
        harness.run();
        assert!(harness.query_by_label("Handicap").is_some(), "{step:?}");
        assert!(harness.query_by_label("×2.00 / ×0.40").is_some(), "{step:?}");
        assert!(harness.query_by_label("18433").is_some(), "{step:?}");
        assert!(harness.query_by_label("not picked yet").is_none(), "{step:?}");
    }
}

#[test]
fn the_status_bar_counts() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::vm());
    harness.run();
    assert!(harness.query_by_label("30 fighters, 1 theme").is_some());
    assert!(harness.query_by_label("Flute vs Trombone, not balanced yet").is_some());
    drop(harness);

    // Only fighters that can be picked are counted.
    let mut vm = base::vm();
    let roster = vm.roster.as_mut().expect("the base has the roster");
    roster.fighters[0].available = false;
    roster.themes.push(roster.themes[0].clone());
    let mut harness = showing(REF, vm);
    harness.run();
    assert!(harness.query_by_label("29 fighters, 2 themes").is_some());
}

#[test]
fn a_notice_can_be_dismissed() {
    let _gpu = gpu();
    let mut vm = base::vm();
    for text in [
        "The settings file could not be read.",
        "1 video is waiting in the queue.",
    ] {
        vm.notices.push(Notice { text: text.to_string() });
    }
    let mut harness = showing(REF, vm);
    harness.run();
    // The newest is shown, above the workspace, which gives up its 34 points.
    assert!(harness.query_by_label("1 video is waiting in the queue.").is_some());
    assert!(harness.query_by_label("The settings file could not be read.").is_none());
    assert_eq!(
        corners(harness.state().layout.workspace),
        [[0.0, 122.0], [1030.0, 764.0]]
    );
    harness.get_by_label("Dismiss").click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::Dismiss(1)]);
    assert!(harness.query_by_label("The settings file could not be read.").is_some());
    harness.get_by_label("Dismiss").click();
    harness.run();
    assert!(harness.state().state.vm.notices.is_empty());
    assert!(harness.query_by_label("Dismiss").is_none());
    assert_eq!(
        corners(harness.state().layout.workspace),
        [[0.0, 88.0], [1030.0, 764.0]]
    );
}

#[test]
fn events_are_taken_in_logic() {
    let _gpu = gpu();
    let (mut harness, side) = live(None);
    side.emit(Event::Roster(sample::roster()));
    harness.run();
    let vm = &harness.state().state.vm;
    assert!(vm.roster.is_some(), "the roster is in the view model");
    assert_eq!(vm.draft.fighters, ["FLUTE".to_string(), "TROMBONE".to_string()]);
    assert!(
        received(&side)
            .iter()
            .any(|c| matches!(c, Command::DescribeFight { .. })),
        "its effects were carried out"
    );
    assert!(harness.query_by_label("30 fighters, 1 theme").is_some());
}

#[test]
fn writer_problems_become_notices() {
    let _gpu = gpu();
    let temp = Temp::new("problems");
    // A file where the settings folder should be: nothing can be saved there.
    std::fs::write(temp.0.join("config"), b"not a folder").unwrap();
    let (mut harness, _side) = live(Some(Writer::start(temp.paths())));
    harness.run();
    harness.state_mut().act(Action::SetWorkers(3));
    // The writer fails on its own thread; each frame's `logic` reads its problems. Bounded by a step count.
    for _ in 0..2_000 {
        if !harness.state().state.vm.notices.is_empty() {
            break;
        }
        harness.step();
        std::thread::sleep(Duration::from_millis(5));
    }
    harness.run();
    let notices = &harness.state().state.vm.notices;
    assert_eq!(notices.len(), 1, "{notices:?}");
    assert!(harness.query_by_label("Dismiss").is_some(), "the notice is on screen");
}

#[test]
fn the_log_is_capped() {
    let _gpu = gpu();
    let (mut harness, _side) = live(None);
    harness.run();
    for i in 0..app::LOG_CAP + 1 {
        let action = if i % 2 == 0 {
            Action::Go(View::Library)
        } else {
            Action::Go(View::Settings)
        };
        harness.state_mut().act(action);
    }
    let log = &harness.state().log;
    assert_eq!(log.len(), app::LOG_KEEP + 1, "the oldest were dropped");
    assert_eq!(log.last(), Some(&Action::Go(View::Library)), "the newest is kept");
}

#[test]
fn without_a_folder_the_app_says_so() {
    let _gpu = gpu();
    let mut harness = app_harness(REF, |cc| {
        app::start(
            cc,
            None,
            || Box::new(NoAudio::new("No sound device in tests.")),
            EngineKind::Sample,
        )
    });
    harness.run();
    let notices = &harness.state().state.vm.notices;
    assert_eq!(
        notices,
        &vec![Notice {
            text: app::NO_FOLDER.to_string()
        }]
    );
    assert!(harness.query_by_label(app::NO_FOLDER).is_some());
    harness.state_mut().shutdown();
}

#[test]
fn long_lines_are_cut_short_at_the_minimum_size() {
    let _gpu = gpu();
    let sentence = "This line is far too long to fit, so it is cut short with an ellipsis rather than drawn over \
                    its neighbours.";
    let long = [sentence; 4].join(" ");
    let mut vm = base::vm();
    vm.notices.push(Notice { text: long.clone() });
    vm.status = long.clone();
    let mut harness = showing(MIN, vm);
    harness.run();
    let l = harness.state().layout;
    let lines: Vec<egui::Rect> = harness.query_all_by_label(&long).map(|n| n.rect()).collect();
    assert_eq!(lines.len(), 2, "the notice and the status line");
    let dismiss = harness.get_by_label("Dismiss").rect();
    let counts = harness.get_by_label("30 fighters, 1 theme").rect();
    for line in lines {
        if line.center().y < l.workspace.min.y {
            assert!(
                line.max.x <= dismiss.min.x,
                "the notice {line:?} stops before Dismiss {dismiss:?}"
            );
        } else {
            assert!(
                l.status.contains_rect(line),
                "the status {line:?} is inside the status bar"
            );
            assert!(
                line.max.x <= counts.min.x,
                "the status {line:?} stops before the counts {counts:?}"
            );
        }
    }
}

#[test]
fn a_silent_pc_says_so_in_the_status_bar() {
    let _gpu = gpu();
    let (mut harness, side) = live(None);
    side.emit(Event::Roster(sample::roster()));
    harness.run();
    harness.state_mut().act(Action::SelectSeed(18433));
    let take = received(&side)
        .iter()
        .find_map(|c| match c {
            Command::LoadTake { take, .. } => Some(*take),
            _ => None,
        })
        .expect("the take was asked for");
    side.emit(Event::TakePicture {
        take,
        story: sample::story(18433),
        frames: sample::frames([[0xde, 0xe8, 0xf6], [0x78, 0xf0, 0xb8]]),
    });
    side.emit(Event::TakeSound {
        take,
        audio: sample::audio(),
        music: sample::music(),
    });
    harness.step();
    assert!(
        harness.state().state.vm.transport.enabled,
        "logic took the take to the player"
    );
    // With no device its sound cannot start when it plays, and the preview goes on silently. Waits for the
    // condition, bounded: the player's frames are drawn on its own threads.
    harness.state_mut().act(Action::Transport(TransportAction::PlayPause));
    let silent = "No sound device was found, so the preview is silent.";
    let deadline = Instant::now() + Duration::from_secs(20);
    while harness.state().state.vm.status != silent && Instant::now() < deadline {
        harness.step();
        std::thread::sleep(Duration::from_millis(5));
    }
    let vm = &harness.state().state.vm;
    assert_eq!(vm.status, silent);
    assert_eq!(vm.transport.sound, SoundState::NoDevice, "the player's view is copied");
    assert!(
        received(&side).contains(&Command::PreviewBusy(true)),
        "playing tells the engine the preview is busy"
    );
}

#[test]
fn nothing_is_unnamed() {
    let _gpu = gpu();
    let mut vm = base::vm();
    vm.notices.push(Notice {
        text: "1 video is waiting in the queue.".to_string(),
    });
    let mut harness = showing(REF, vm);
    harness.run();
    assert_eq!(harness.unnamed_widgets(), Vec::<String>::new());
}

#[test]
fn starting_installs_the_theme_and_loads_what_was_saved() {
    let _gpu = gpu();
    let temp = Temp::new("start");
    let paths = temp.paths();
    std::fs::create_dir_all(&paths.config).unwrap();
    let saved = Settings {
        output_folder: temp.0.join("My videos"),
        workers: 3,
    };
    save_settings(&paths.settings(), &saved).unwrap();
    std::fs::write(paths.queue(), b"{ this is not a queue").unwrap();
    let start_paths = paths.clone();
    let mut harness = app_harness(REF, move |cc| {
        app::start(
            cc,
            Some(start_paths),
            || Box::new(NoAudio::new("No sound device in tests.")),
            EngineKind::Sample,
        )
    });
    // The live engine answers on its own thread, and each answer asks for another frame: `run` could run out of
    // steps while answers trickle in on a busy PC. Step until the start-up answers are in, bounded, instead.
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        harness.step();
        let vm = &harness.state().state.vm;
        if (vm.roster.is_some() && vm.ffmpeg.is_some()) || Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let vm = &harness.state().state.vm;
    assert!(
        vm.roster.is_some() && vm.ffmpeg.is_some(),
        "the engine answered at start"
    );
    let style = harness.ctx.global_style();
    assert_eq!(style.visuals.panel_fill, theme::PANEL, "the theme is installed");
    assert_eq!(style.spacing.interact_size.y, theme::CONTROL_H);
    let vm = &harness.state().state.vm;
    assert_eq!(vm.settings, saved, "the settings were loaded through the paths");
    assert_eq!(
        vm.notices.len(),
        1,
        "the queue file's notice is shown: {:?}",
        vm.notices
    );
    harness.state_mut().shutdown();
    drop(harness);
    let names: Vec<String> = std::fs::read_dir(&paths.config)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        names.iter().any(|n| n.starts_with("queue.corrupt")),
        "the unreadable queue was moved aside, not deleted: {names:?}"
    );
}

#[test]
fn the_shell_matches() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::vm());
    harness.run();
    harness.snapshot("shell_1440x900");
}
