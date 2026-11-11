//! The Settings view (spec section 4): the output folder, the worker threads, the video encoder and the version.

use std::path::PathBuf;

use bf_app::app::BfApp;
use bf_app::fixtures::settings;
use bf_app::screens::settings::{FFMPEG_MISSING, LOOKING_FOR_FFMPEG};
use bf_app::seam::Command;
use bf_app::state::{Action, ViewModel};
use egui::accesskit::Role;
use egui::{Key, Rect, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common::{MIN, REF, app_harness, gpu, pixels};

const WORKERS_NOTE: &str = "One core stays free for the preview. This PC has 8 cores.";
const VERSION: &str = "0.1.0. The Update button starts working in a later version.";

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

#[test]
fn the_four_rows() {
    let _gpu = gpu();
    let harness = showing(REF, settings::vm());
    let workspace = harness.state().layout.workspace;
    let labels: Vec<Rect> = ["Output folder", "Worker threads", "Video encoder", "Version"]
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
    let change = control(&harness, "Change folder", Role::Button).rect();
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
    // The encoder and the version.
    assert!(has(&harness, "ffmpeg 9.0.2, found on this PC"));
    assert!(has(&harness, VERSION));
    // Rows 16 apart: the folder row is 26 tall, so the next label starts 26 + 16 below it.
    let row = |label: Rect| label.center().y - 13.0;
    assert_eq!(row(labels[1]) - row(labels[0]), 26.0 + 16.0);
    assert!(harness.state().log.is_empty());
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
    harness.get_by_label("Change folder").click();
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
    let change = control(&harness, "Change folder", Role::Button).rect();
    assert!(rect.max.x < change.min.x, "it stops before its button");
    assert!(!has(&harness, &text), "no tooltip yet");
    control(&harness, "Output folder", Role::TextInput).hover();
    harness.run();
    assert!(has(&harness, &text), "hovering shows all of it");
}

#[test]
fn without_the_roster_the_version_is_the_apps_own() {
    let _gpu = gpu();
    let mut vm = settings::vm();
    vm.roster = None;
    let harness = showing(REF, vm);
    let version = format!(
        "{}. The Update button starts working in a later version.",
        env!("CARGO_PKG_VERSION")
    );
    assert!(has(&harness, &version), "{version}");
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

/// The view's controls in reading order, with their roles and how far inside their rect the ring may lie (the
/// slider paints its ring inside its 26 tall row).
const CONTROLS: [(&str, Role, i32); 3] = [
    ("Change folder", Role::Button, 2),
    ("Worker threads", Role::Slider, 5),
    ("Worker threads value", Role::SpinButton, 2),
];

#[test]
fn every_control_shows_its_focus_ring() {
    let _gpu = gpu();
    for size in [REF, MIN] {
        let mut harness = showing(size, settings::vm());
        for (name, role, inside) in CONTROLS {
            let rect = control(&harness, name, role).rect();
            assert_eq!(
                ring_sides(&mut harness, rect, inside),
                [false; 4],
                "{size:?}: {name} before focus"
            );
            control(&harness, name, role).focus();
            harness.run();
            assert!(control(&harness, name, role).is_focused(), "{size:?}: {name}");
            assert_eq!(
                ring_sides(&mut harness, rect, inside),
                [true; 4],
                "{size:?}: {name}'s ring is whole"
            );
            harness.key_press(Key::Escape);
            harness.run();
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
    }
}

#[test]
fn nothing_is_unnamed() {
    let _gpu = gpu();
    let mut missing = settings::vm();
    missing.ffmpeg = Some(Err("not found".to_string()));
    for vm in [settings::vm(), missing] {
        let harness = showing(REF, vm);
        assert_eq!(harness.unnamed_widgets(), Vec::<String>::new());
    }
}

#[test]
fn settings_matches() {
    let _gpu = gpu();
    let mut harness = showing(REF, settings::vm());
    harness.snapshot("settings_1440x900");
}
