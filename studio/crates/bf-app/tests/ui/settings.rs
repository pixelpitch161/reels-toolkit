//! The Settings view (spec section 4; plan M6, Task 13): the output folder, the worker threads, the video encoding,
//! the video encoder, the version, updates, problems, the guide and removal.
//!
//! Every test here runs against the sample upkeep (`seam::upkeep::SampleUpkeep`), which `BfApp::showing` gives the
//! state: nothing here can reach a real install, the network, git or cargo.

use std::path::PathBuf;

use bf_app::app::BfApp;
use bf_app::fixtures::settings;
use bf_app::model::EncodeChoice;
use bf_app::model::queue::JobSpec;
use bf_app::screens::settings::{ENCODING_NOTE, FFMPEG_MISSING, LOOKING_FOR_FFMPEG, NO_GUIDE, reports_line};
use bf_app::seam::Command;
use bf_app::seam::upkeep::{sample_removal_words, sample_removed_words};
use bf_app::state::upkeep::{self as upkeep, CHECKING, NOT_INSTALLED, NOTHING_TO_REMOVE, RemovalView, UP_TO_DATE};
use bf_app::state::{Action, ViewModel};
use bf_app::update::worker::Request;
use bf_app::update::{RENDERING, UpdateState};
use egui::accesskit::{Role, Toggled};
use egui::{Key, Rect, ViewportCommand, ViewportId, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common::{MIN, REF, app_harness, gpu, pixels};

const WORKERS_NOTE: &str = "One core stays free for the preview. This PC has 8 cores.";
const REMOVE: &str = "Remove BALLFIGHT Studio from this PC";

/// The rows' labels, top to bottom.
const ROWS: [&str; 9] = [
    "Output folder",
    "Worker threads",
    "Video encoding",
    "Video encoder",
    "Version",
    "Updates",
    "Problems",
    "Guide",
    "Remove",
];

/// The shell showing `vm` as it is.
fn showing<'a>(size: (f32, f32), vm: ViewModel) -> Harness<'a, BfApp> {
    let mut harness = app_harness(size, move |cc| BfApp::showing(cc, vm));
    harness.run();
    harness
}

fn has(harness: &Harness<'_, BfApp>, text: &str) -> bool {
    harness.query_all_by_label(text).next().is_some()
}

/// The control with `label` and `role`.
fn control<'t>(harness: &'t Harness<'_, BfApp>, label: &'t str, role: Role) -> egui_kittest::Node<'t> {
    harness
        .query_all_by_label(label)
        .find(|n| n.accesskit_node().role() == role)
        .unwrap_or_else(|| panic!("a {role:?} named {label}"))
}

/// The button named `label`.
fn button<'t>(harness: &'t Harness<'_, BfApp>, label: &'t str) -> egui_kittest::Node<'t> {
    control(harness, label, Role::Button)
}

/// Whether the button named `label` is there and can be used.
fn usable(harness: &Harness<'_, BfApp>, label: &str) -> bool {
    !button(harness, label).accesskit_node().is_disabled()
}

/// The worker counts pushed so far.
fn workers(harness: &Harness<'_, BfApp>) -> Vec<usize> {
    harness
        .state()
        .log
        .iter()
        .filter_map(|a| match a {
            Action::SetWorkers(n) => Some(*n),
            _ => None,
        })
        .collect()
}

/// What the upkeep has been sent.
fn upkeep_sent(harness: &Harness<'_, BfApp>) -> Vec<Request> {
    harness.state().state.upkeep_sent()
}

/// A video rendering.
fn rendering(mut vm: ViewModel) -> ViewModel {
    let id = vm.queue.add(JobSpec {
        title: "Tuba vs Bass drum".to_string(),
        colors: [[0x68, 0x5c, 0xec], [0x80, 0x80, 0x8c]],
        theme: "Big top".to_string(),
        seed: 9120,
        fight_toml: String::new(),
        out_root: PathBuf::from("Videos"),
    });
    vm.queue.start(id);
    vm
}

#[test]
fn the_rows() {
    let _gpu = gpu();
    let harness = showing(REF, settings::vm());
    let workspace = harness.state().layout.workspace;
    let labels: Vec<Rect> = ROWS
        .iter()
        .map(|label| control(&harness, label, Role::Label).rect())
        .collect();
    for (i, rect) in labels.iter().enumerate() {
        assert_eq!(rect.min.x, workspace.min.x + 28.0, "row {i}: padded 28 at the side");
        if i > 0 {
            assert!(rect.min.y > labels[i - 1].max.y, "row {i} under row {}", i - 1);
        }
    }
    assert!(
        labels[0].min.y >= workspace.min.y + 28.0,
        "padded 28 above: {:?}",
        labels[0]
    );
    // The output folder: its full path in a read field, and the button that changes it.
    let folder = control(&harness, "Output folder", Role::TextInput);
    let path = settings::vm().settings.output_folder.display().to_string();
    assert_eq!(folder.value().as_deref(), Some(path.as_str()));
    let field = folder.rect();
    assert_eq!(
        field.min.x,
        workspace.min.x + 28.0 + 150.0,
        "the label column is 150 wide"
    );
    let change = button(&harness, "Change folder").rect();
    assert_eq!(change.max.x, workspace.min.x + 28.0 + 560.0, "the column is 560 wide");
    assert!(change.min.x > field.max.x);
    // The worker threads: a slider from 1 to the cores, a value field and what it means.
    let slider = control(&harness, "Worker threads", Role::Slider);
    let node = slider.accesskit_node();
    assert_eq!(node.numeric_value(), Some(7.0));
    assert_eq!(node.min_numeric_value(), Some(1.0));
    assert_eq!(node.max_numeric_value(), Some(8.0));
    assert_eq!(slider.rect().min.x, workspace.min.x + 28.0 + 150.0);
    let value = control(&harness, "Worker threads value", Role::SpinButton);
    assert_eq!(value.value().as_deref(), Some("7"));
    assert!(has(&harness, WORKERS_NOTE), "{WORKERS_NOTE}");
    // The encoding: two segments, Best quality chosen, and what they mean.
    let best = control(&harness, "Best quality", Role::RadioButton);
    let faster = control(&harness, "Faster", Role::RadioButton);
    assert_eq!(best.rect().min.x, workspace.min.x + 28.0 + 150.0);
    assert!(faster.rect().min.x >= best.rect().max.x);
    assert_eq!(best.accesskit_node().toggled(), Some(Toggled::True));
    assert_ne!(faster.accesskit_node().toggled(), Some(Toggled::True));
    assert!(has(&harness, ENCODING_NOTE));
    // The encoder, the version, updates, problems, the guide and removal.
    assert!(has(&harness, "ffmpeg 9.0.2, found on this PC"));
    assert!(has(&harness, settings::APP_LINE));
    assert!(has(&harness, CHECKING));
    assert!(has(&harness, &reports_line(&settings::reports())));
    assert!(usable(&harness, "Open the reports folder"));
    assert!(!usable(&harness, "Open the guide"), "{NO_GUIDE}");
    assert!(usable(&harness, REMOVE));
    // Rows 16 apart: the folder row is 26 tall, so the next label starts 26 + 16 below it.
    let row = |label: Rect| label.center().y - 13.0;
    assert_eq!(row(labels[1]) - row(labels[0]), 26.0 + 16.0);
    assert!(harness.state().log.is_empty());
}

#[test]
fn the_encode_segments_push() {
    let _gpu = gpu();
    let mut harness = showing(REF, settings::vm());
    control(&harness, "Best quality", Role::RadioButton).click();
    harness.run();
    assert!(harness.state().log.is_empty(), "the chosen one pushes nothing");
    control(&harness, "Faster", Role::RadioButton).click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SetEncode(EncodeChoice::Faster)]);
    assert_eq!(harness.state().state.vm.settings.encode, EncodeChoice::Faster);
    harness.run();
    assert_eq!(
        control(&harness, "Faster", Role::RadioButton)
            .accesskit_node()
            .toggled(),
        Some(Toggled::True),
        "the segment follows"
    );
    // It is inverted, its label ON_LIGHT on the TEXT fill (widget rule 2).
    let rgb = |c: egui::Color32| [c.r(), c.g(), c.b()];
    let rect = control(&harness, "Faster", Role::RadioButton).rect();
    let (y, x0, x1) = (rect.center().y as u32, rect.min.x as u32 + 2, rect.max.x as u32 - 2);
    let row: Vec<(u32, u32)> = (x0..x1).map(|x| (x, y)).collect();
    let seen = pixels(&mut harness, &row);
    assert!(seen.contains(&rgb(bf_app::theme::TEXT)), "the fill");
    assert!(seen.contains(&rgb(bf_app::theme::ON_LIGHT)), "the label");
    control(&harness, "Best quality", Role::RadioButton).click();
    harness.run();
    assert_eq!(harness.state().log.last(), Some(&Action::SetEncode(EncodeChoice::Best)));
}

#[test]
fn the_worker_slider_stops_at_the_cores() {
    let _gpu = gpu();
    let mut harness = showing(REF, settings::vm());
    let slider = control(&harness, "Worker threads", Role::Slider).rect();
    let y = slider.center().y;
    harness.drag_at(pos2(slider.min.x + 0.5 * slider.width(), y));
    harness.run();
    harness.hover_at(pos2(slider.max.x - 4.0, y));
    harness.run();
    harness.hover_at(pos2(slider.max.x + 200.0, y));
    harness.run();
    harness.drop_at(pos2(slider.max.x + 200.0, y));
    harness.run();
    let pushed = workers(&harness);
    assert_eq!(pushed.last(), Some(&8), "{pushed:?}");
    assert!(pushed.iter().all(|n| (1..=8).contains(n)), "{pushed:?}");
    assert_eq!(harness.state().state.vm.settings.workers, 8);
    assert_eq!(
        control(&harness, "Worker threads value", Role::SpinButton)
            .value()
            .as_deref(),
        Some("8"),
        "the value field follows"
    );
    // At the end the arrow goes no further.
    let before = workers(&harness).len();
    control(&harness, "Worker threads", Role::Slider).focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(workers(&harness).len(), before, "{:?}", workers(&harness));
    // And it steps down one whole thread at a time.
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(workers(&harness).last(), Some(&7));
    let sent = harness.state_mut().sent();
    assert!(sent.iter().any(|c| matches!(c, Command::SetWorkers(7))), "{sent:?}");
}

#[test]
fn typing_a_worker_count_sets_it_within_the_cores() {
    let _gpu = gpu();
    let mut harness = showing(REF, settings::vm());
    control(&harness, "Worker threads value", Role::SpinButton).focus();
    harness.run();
    control(&harness, "Worker threads value", Role::SpinButton).type_text("3");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(workers(&harness).last(), Some(&3), "{:?}", harness.state().log);
    control(&harness, "Worker threads value", Role::SpinButton).focus();
    harness.run();
    control(&harness, "Worker threads value", Role::SpinButton).type_text("99");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(
        workers(&harness).last(),
        Some(&8),
        "kept to the cores: {:?}",
        harness.state().log
    );
}

#[test]
fn change_folder_pushes() {
    let _gpu = gpu();
    let mut harness = showing(REF, settings::vm());
    button(&harness, "Change folder").click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::ChangeFolder]);
    let sent = harness.state_mut().sent();
    assert!(sent.iter().any(|c| matches!(c, Command::PickFolder { .. })), "{sent:?}");
}

#[test]
fn ffmpeg_missing_says_what_to_do() {
    let _gpu = gpu();
    let mut vm = settings::vm();
    vm.ffmpeg = Some(Err("ffmpeg is not on PATH".to_string()));
    let harness = showing(REF, vm);
    assert!(has(&harness, FFMPEG_MISSING));
    assert!(!has(&harness, "ffmpeg 9.0.2, found on this PC"));
    let workspace = harness.state().layout.workspace;
    let text = harness.get_by_label(FFMPEG_MISSING).rect();
    assert!(
        text.max.x <= workspace.min.x + 28.0 + 560.0 + 0.5,
        "it wraps inside the column: {text:?}"
    );
    drop(harness);
    let mut vm = settings::vm();
    vm.ffmpeg = None;
    let harness = showing(REF, vm);
    assert!(has(&harness, LOOKING_FOR_FFMPEG));
}

#[test]
fn a_long_folder_is_cut_short_and_shown_whole_on_hover() {
    let _gpu = gpu();
    let long: PathBuf = [
        "D:",
        "Projects",
        "Music videos",
        "Instrument battles",
        "Season two",
        "Finished renders",
    ]
    .iter()
    .collect::<PathBuf>()
    .join("BALLFIGHT exports for every platform");
    let text = long.display().to_string();
    let mut vm = settings::vm();
    vm.settings.output_folder = long;
    let mut harness = showing(MIN, vm);
    let field = control(&harness, "Output folder", Role::TextInput);
    assert_eq!(
        field.value().as_deref(),
        Some(text.as_str()),
        "a screen reader gets all of it"
    );
    let rect = field.rect();
    let change = button(&harness, "Change folder").rect();
    assert!(rect.max.x < change.min.x, "it stops before its button");
    assert!(!has(&harness, &text), "no tooltip yet");
    control(&harness, "Output folder", Role::TextInput).hover();
    harness.run();
    assert!(has(&harness, &text), "hovering shows all of it");
}

#[test]
fn the_version_is_the_apps_line() {
    let _gpu = gpu();
    let mut vm = settings::vm();
    vm.roster = None;
    vm.app_line = "0.6.0 (unknown, unknown)".to_string();
    let harness = showing(REF, vm);
    assert!(has(&harness, "0.6.0 (unknown, unknown)"));
}

#[test]
fn a_count_from_a_bigger_pc_is_shown_at_the_end_and_left_alone() {
    let _gpu = gpu();
    let mut vm = settings::vm();
    vm.settings.workers = 15;
    let mut harness = showing(REF, vm);
    harness.run();
    let slider = control(&harness, "Worker threads", Role::Slider);
    assert_eq!(slider.accesskit_node().numeric_value(), Some(8.0));
    assert!(
        harness.state().log.is_empty(),
        "showing it changes nothing: {:?}",
        harness.state().log
    );
    assert_eq!(harness.state().state.vm.settings.workers, 15);
}

#[test]
fn a_one_core_pc_has_one_worker() {
    let _gpu = gpu();
    let mut vm = settings::vm();
    vm.cores = 1;
    vm.settings.workers = 1;
    let harness = showing(REF, vm);
    assert!(has(
        &harness,
        "One core stays free for the preview. This PC has 1 core."
    ));
    let node = control(&harness, "Worker threads", Role::Slider);
    let node = node.accesskit_node();
    assert_eq!(
        (node.min_numeric_value(), node.max_numeric_value()),
        (Some(1.0), Some(1.0))
    );
}

/// The words and buttons each update state shows: (fixture name, words, the buttons that can be used, the request
/// the first of them sends).
fn update_rows() -> Vec<(&'static str, String, Vec<String>, Option<Request>)> {
    let back = "Go back to version 0.0.9".to_string();
    vec![
        ("not installed", NOT_INSTALLED.to_string(), vec![], None),
        ("checking", CHECKING.to_string(), vec![], None),
        (
            "up to date",
            UP_TO_DATE.to_string(),
            vec!["Look again".to_string(), back.clone()],
            Some(Request::Check),
        ),
        (
            "available",
            upkeep::available("0.2.0"),
            vec!["Build version 0.2.0".to_string(), back.clone()],
            Some(Request::Build),
        ),
        (
            "building",
            "Building version 0.2.0: 03:07 so far.".to_string(),
            vec!["Stop building".to_string()],
            Some(Request::StopBuild),
        ),
        (
            "ready",
            upkeep::ready("0.2.0"),
            vec!["Restart now".to_string(), back.clone()],
            Some(Request::Restart),
        ),
        (
            "failed",
            bf_app::update::check_failed("git could not reach the repository"),
            vec!["Try again".to_string(), back],
            Some(Request::Check),
        ),
    ]
}

#[test]
fn each_update_state_reads() {
    let _gpu = gpu();
    let rows = update_rows();
    for ((name, vm), (row_name, text, usable_buttons, first)) in settings::each_update_state().into_iter().zip(rows) {
        assert_eq!(name, row_name);
        let mut harness = showing(REF, vm);
        assert!(has(&harness, &text), "{name}: {text}");
        for label in &usable_buttons {
            assert!(usable(&harness, label), "{name}: {label} can be used");
        }
        if name == "building" {
            assert!(has(&harness, "Building bf-app."), "what the build is doing");
            assert!(
                harness
                    .query_all_by_label("Go back to version 0.0.9")
                    .all(|n| n.accesskit_node().is_disabled()),
                "no going back while building"
            );
        }
        if let (Some(request), Some(label)) = (first, usable_buttons.first()) {
            button(&harness, label).click();
            harness.run();
            assert_eq!(harness.state().log, vec![Action::Upkeep(request)], "{name}");
            assert_eq!(upkeep_sent(&harness), vec![request], "{name}");
        }
        if usable_buttons.iter().any(|b| b.starts_with("Go back")) {
            button(&harness, "Go back to version 0.0.9").click();
            harness.run();
            assert_eq!(upkeep_sent(&harness).last(), Some(&Request::GoBack), "{name}");
        }
    }
}

#[test]
fn a_note_and_the_guide() {
    let _gpu = gpu();
    let mut vm = settings::with_update(UpdateState::UpToDate);
    vm.update.note = Some("This copy has no guide.".to_string());
    let mut harness = showing(REF, vm);
    assert!(has(&harness, "This copy has no guide."), "the note under the row");
    button(&harness, "Open the guide").click();
    harness.run();
    assert_eq!(upkeep_sent(&harness), vec![Request::OpenGuide]);
    button(&harness, "Open the reports folder").click();
    harness.run();
    let sent = harness.state_mut().sent();
    assert!(
        sent.contains(&Command::RevealFolder {
            path: settings::reports()
        }),
        "{sent:?}"
    );
}

#[test]
fn restart_waits_for_a_render() {
    let _gpu = gpu();
    let vm = rendering(settings::with_update(UpdateState::Ready {
        version: "0.2.0".to_string(),
    }));
    let mut harness = showing(REF, vm);
    for label in ["Restart now", "Go back to version 0.0.9", REMOVE] {
        assert!(!usable(&harness, label), "{label} waits");
        button(&harness, label).click();
        harness.run();
    }
    assert!(harness.state().log.is_empty());
    assert!(upkeep_sent(&harness).is_empty());
    assert!(!has(&harness, RENDERING), "no tooltip yet");
    button(&harness, "Restart now").hover();
    harness.run();
    assert!(has(&harness, RENDERING), "hovering says why");
}

#[test]
fn remove_asks_first() {
    let _gpu = gpu();
    let mut harness = showing(REF, settings::with_update(UpdateState::UpToDate));
    // Enter on the first button only asks.
    button(&harness, REMOVE).focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::AskRemove]);
    assert_eq!(
        harness.state().state.vm.removal,
        RemovalView::asking(sample_removal_words())
    );
    for line in sample_removal_words() {
        assert!(has(&harness, &line), "{line}");
    }
    assert!(usable(&harness, "Remove the app"));
    assert!(
        button(&harness, "Keep it").is_focused(),
        "the keyboard starts on Keep it"
    );
    assert!(
        harness.query_all_by_label(REMOVE).next().is_none(),
        "the question in its place"
    );
    assert!(upkeep_sent(&harness).is_empty(), "nothing is sent yet");
    // Enter again: on Keep it, so nothing is removed.
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().log.last(), Some(&Action::KeepApp));
    assert_eq!(harness.state().state.vm.removal, RemovalView::Closed);
    assert!(usable(&harness, REMOVE), "back as it was");
    assert!(upkeep_sent(&harness).is_empty());
    // Asked again and confirmed: the removal is sent once.
    button(&harness, REMOVE).click();
    harness.run();
    harness.run();
    button(&harness, "Remove the app").click();
    harness.run();
    assert_eq!(upkeep_sent(&harness), vec![Request::Remove]);
    // The sample answers without removing anything: the question again, with why.
    assert!(has(&harness, bf_app::seam::upkeep::SAMPLE_KEPT), "the answer is shown");
    assert!(usable(&harness, "Remove the app") && usable(&harness, "Keep it"));
    assert!(!has(&harness, upkeep::REMOVING), "never left on Removing");
    button(&harness, "Remove the app").click();
    harness.run();
    assert_eq!(upkeep_sent(&harness), vec![Request::Remove, Request::Remove]);
    assert!(
        has(&harness, bf_app::seam::upkeep::SAMPLE_KEPT),
        "the same answer again is shown again"
    );
}

#[test]
fn a_copy_that_was_not_installed_cannot_be_removed_or_update() {
    let _gpu = gpu();
    let mut harness = showing(REF, settings::vm_not_installed());
    assert!(has(&harness, NOT_INSTALLED));
    assert!(!usable(&harness, REMOVE));
    button(&harness, REMOVE).click();
    harness.run();
    assert!(harness.state().log.is_empty());
    button(&harness, REMOVE).hover();
    harness.run();
    assert!(has(&harness, NOTHING_TO_REMOVE), "hovering says why");
    assert!(upkeep_sent(&harness).is_empty());
}

#[test]
fn after_a_removal_the_words_show_and_the_app_closes_on_a_click() {
    let _gpu = gpu();
    let mut harness = showing(REF, settings::removed());
    for line in sample_removed_words() {
        assert!(has(&harness, &line), "{line}");
    }
    // Nothing else can be used: no update button, every other row disabled, the toolbar's Update too.
    for gone in ["Look again", "Go back to version 0.0.9", "Remove the app", REMOVE] {
        assert!(harness.query_all_by_label(gone).next().is_none(), "{gone} is not shown");
    }
    for label in ["Change folder", "Open the reports folder", "Open the guide", "Update"] {
        assert!(!usable(&harness, label), "{label} cannot be used");
    }
    assert!(
        control(&harness, "Faster", Role::RadioButton)
            .accesskit_node()
            .is_disabled()
    );
    ring_after_focus(&mut harness, "Close BALLFIGHT Studio", Role::Button, 2, REF);
    button(&harness, "Change folder").click();
    harness.run();
    assert!(harness.state().log.is_empty(), "{:?}", harness.state().log);
    let closes = |harness: &Harness<'_, BfApp>| {
        harness.output().viewport_output[&ViewportId::ROOT]
            .commands
            .iter()
            .any(|c| matches!(c, ViewportCommand::Close))
    };
    assert!(!closes(&harness), "the words are read first");
    button(&harness, "Close BALLFIGHT Studio").click();
    harness.step();
    assert!(closes(&harness), "then the window closes (and saves)");
}

#[test]
fn a_restart_closes_at_once() {
    let _gpu = gpu();
    let mut vm = settings::with_update(UpdateState::Ready {
        version: "0.2.0".to_string(),
    });
    vm.update.close = Some(Vec::new());
    // Stepped, not run: asking to close asks for a repaint, so it never settles.
    let mut harness = app_harness(REF, move |cc| BfApp::showing(cc, vm));
    harness.step();
    let commands = &harness.output().viewport_output[&ViewportId::ROOT].commands;
    assert!(
        commands.iter().any(|c| matches!(c, ViewportCommand::Close)),
        "{commands:?}"
    );
}

/// Whether white (`TEXT_STRONG`) pixels lie on each side of `rect` (top, right, bottom, left), looking along a short
/// line across the middle of the side, from `inside` inside it to 5 outside.
fn ring_sides(harness: &mut Harness<'_, BfApp>, rect: Rect, inside: i32) -> [bool; 4] {
    let (cx, cy) = (rect.center().x as i32, rect.center().y as i32);
    let (x0, x1, y0, y1) = (
        rect.min.x as i32,
        rect.max.x as i32,
        rect.min.y as i32,
        rect.max.y as i32,
    );
    let lines: [Vec<(i32, i32)>; 4] = [
        (y0 - 5..y0 + inside).map(|y| (cx, y)).collect(),
        (x1 - inside..x1 + 5).map(|x| (x, cy)).collect(),
        (y1 - inside..y1 + 5).map(|y| (cx, y)).collect(),
        (x0 - 5..x0 + inside).map(|x| (x, cy)).collect(),
    ];
    lines.map(|line| {
        let at: Vec<(u32, u32)> = line.iter().map(|&(x, y)| (x.max(0) as u32, y.max(0) as u32)).collect();
        pixels(harness, &at).contains(&[255, 255, 255])
    })
}

/// The controls of the plain fixture in reading order, with their roles and how far inside their rect the ring may
/// lie (the slider paints its ring inside its 26 tall row).
const CONTROLS: [(&str, Role, i32); 7] = [
    ("Change folder", Role::Button, 2),
    ("Worker threads", Role::Slider, 5),
    ("Worker threads value", Role::SpinButton, 2),
    ("Best quality", Role::RadioButton, 2),
    ("Faster", Role::RadioButton, 2),
    ("Open the reports folder", Role::Button, 2),
    (REMOVE, Role::Button, 2),
];

/// The buttons of other fixtures that can be used, in reading order after the encoding segments.
fn more_controls() -> Vec<(ViewModel, Vec<&'static str>)> {
    const BACK: &str = "Go back to version 0.0.9";
    const REPORTS: &str = "Open the reports folder";
    const GUIDE: &str = "Open the guide";
    vec![
        (
            settings::with_update(UpdateState::Ready {
                version: "0.2.0".to_string(),
            }),
            vec!["Restart now", BACK, REPORTS, GUIDE, REMOVE],
        ),
        (
            settings::with_update(UpdateState::Available {
                version: "0.2.0".to_string(),
            }),
            vec!["Build version 0.2.0", BACK, REPORTS, GUIDE, REMOVE],
        ),
        (
            settings::with_update(UpdateState::Failed {
                text: bf_app::update::check_failed("no network"),
            }),
            vec!["Try again", BACK, REPORTS, GUIDE, REMOVE],
        ),
        (
            // Going back and removing wait for the build.
            settings::with_update(UpdateState::Building {
                version: "0.2.0".to_string(),
                seconds: 3,
                doing: String::new(),
            }),
            vec!["Stop building", REPORTS, GUIDE],
        ),
        (
            settings::asking_removal(),
            vec!["Look again", BACK, REPORTS, GUIDE, "Remove the app", "Keep it"],
        ),
    ]
}

/// Focuses `name` (scrolling it into view, as Tab does), and checks its ring is whole at its rect then.
fn ring_after_focus(harness: &mut Harness<'_, BfApp>, name: &str, role: Role, inside: i32, size: (f32, f32)) {
    control(harness, name, role).focus();
    harness.run();
    harness.run();
    assert!(control(harness, name, role).is_focused(), "{size:?}: {name}");
    let rect = control(harness, name, role).rect();
    let workspace = harness.state().layout.workspace;
    assert!(
        workspace.contains_rect(rect.expand(4.0)),
        "{size:?}: {name} at {rect:?} is in view"
    );
    assert_eq!(
        ring_sides(harness, rect, inside),
        [true; 4],
        "{size:?}: {name}'s ring is whole"
    );
}

#[test]
fn every_control_shows_its_focus_ring() {
    let _gpu = gpu();
    for size in [REF, MIN] {
        let mut harness = showing(size, settings::vm());
        for (name, role, inside) in CONTROLS {
            let rect = control(&harness, name, role).rect();
            if harness.state().layout.workspace.contains_rect(rect.expand(4.0)) {
                assert_eq!(
                    ring_sides(&mut harness, rect, inside),
                    [false; 4],
                    "{size:?}: {name} before focus"
                );
            }
            ring_after_focus(&mut harness, name, role, inside, size);
            harness.key_press(Key::Escape);
            harness.run();
        }
        for (vm, names) in more_controls() {
            let mut harness = showing(size, vm);
            for name in names {
                ring_after_focus(&mut harness, name, Role::Button, 2, size);
                harness.key_press(Key::Escape);
                harness.run();
            }
        }
    }
}

#[test]
fn tab_walks_every_control_in_reading_order() {
    let _gpu = gpu();
    for size in [REF, MIN] {
        let mut harness = showing(size, settings::vm());
        control(&harness, CONTROLS[0].0, CONTROLS[0].1).focus();
        harness.run();
        for (i, (name, role, _)) in CONTROLS.iter().enumerate() {
            if i > 0 {
                harness.key_press(Key::Tab);
                harness.run();
            }
            assert!(
                control(&harness, name, *role).is_focused(),
                "{size:?}: Tab number {i} reaches {name}"
            );
        }
        assert!(
            harness.state().log.is_empty(),
            "walking through does nothing: {:?}",
            harness.state().log
        );
        // The other states' buttons follow the segments in reading order.
        for (vm, names) in more_controls() {
            let mut harness = showing(size, vm);
            control(&harness, "Faster", Role::RadioButton).focus();
            harness.run();
            for name in names {
                harness.key_press(Key::Tab);
                harness.run();
                assert!(button(&harness, name).is_focused(), "{size:?}: Tab reaches {name}");
            }
            assert!(harness.state().log.is_empty(), "{size:?}: {:?}", harness.state().log);
        }
    }
}

#[test]
fn nothing_is_unnamed() {
    let _gpu = gpu();
    let mut missing = settings::vm();
    missing.ffmpeg = Some(Err("not found".to_string()));
    let mut all = vec![settings::vm(), missing, settings::asking_removal(), settings::removed()];
    all.extend(settings::each_update_state().into_iter().map(|(_, vm)| vm));
    for vm in all {
        let harness = showing(REF, vm);
        assert_eq!(harness.unnamed_widgets(), Vec::<String>::new());
    }
}

#[test]
fn the_question_fits_the_smallest_window() {
    let _gpu = gpu();
    let mut harness = showing(MIN, settings::asking_removal());
    // The column scrolls: its bar shows, and the question's buttons can be reached.
    assert!(
        harness.query_all_by_role(Role::ScrollBar).next().is_some(),
        "the column scrolls"
    );
    ring_after_focus(&mut harness, "Keep it", Role::Button, 2, MIN);
}

#[test]
fn settings_matches() {
    let _gpu = gpu();
    let mut harness = showing(REF, settings::vm());
    harness.snapshot("settings_1440x900");
}
