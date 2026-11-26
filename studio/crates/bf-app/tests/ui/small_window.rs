//! Every screen at the smallest window, 1280 x 800 (Review Focus 5): every control named, none outside the window
//! unless it sits in a scroll area, every primary button in the workspace, and a picture of each.

use bf_app::app::BfApp;
use bf_app::fixtures::{balance, library, matchup, music, render, seeds, settings};
use bf_app::seam::sample;
use bf_app::state::ViewModel;
use egui::Rect;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common::{MIN, app_harness, gpu};

/// The seven screens: the picture's name, the fixture, the screen's primary buttons (Settings has none), and
/// whether the stage shows the VS title card.
fn screens() -> Vec<(&'static str, ViewModel, &'static [&'static str], bool)> {
    vec![
        ("matchup", matchup::vm(), &["Balance this match-up"], true),
        ("balance", balance::vm(), &["Search seeds"], true),
        ("seeds", seeds::vm(), &["Use seed 18433"], false),
        ("music", music::vm(), &["Go to render"], false),
        ("render", render::vm(), &["Add to queue"], false),
        ("library", library::vm(), &["Reopen"], false),
        ("settings", settings::vm(), &[], false),
    ]
}

/// The shell showing `vm` at the smallest window, settled, with the title card on the stage when `card`.
fn showing<'a>(vm: ViewModel, card: bool) -> Harness<'a, BfApp> {
    let mut harness = app_harness(MIN, move |cc| BfApp::showing(cc, vm));
    if card {
        let vm = &harness.state().state.vm;
        let masks = [0, 1].map(|side| vm.fighter(side).expect("a fighter").mask.clone());
        let card = sample::title_card(vm.colors(), [&masks[0], &masks[1]]);
        harness.state_mut().show_frame(card);
    }
    harness.run();
    harness
}

/// Roles that are controls: what the owner clicks, types into or moves.
fn is_control(role: Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::CheckBox
            | Role::Switch
            | Role::ComboBox
            | Role::TextInput
            | Role::MultilineTextInput
            | Role::SpinButton
            | Role::Slider
            | Role::RadioButton
            | Role::Tab
            | Role::MenuItem
            | Role::Link
    )
}

/// Whether `rect` lies in a scroll area whose bar is shown: at or below the top of a shown scroll bar, and ending at
/// or before its right edge. (egui gives a scroll area no node of its own, only its bar, shown while the content
/// overflows: widget rule 21.)
fn in_scroll_area(rect: Rect, bars: &[Rect]) -> bool {
    bars.iter()
        .any(|bar| rect.min.y >= bar.min.y - 8.0 && rect.max.x <= bar.max.x + 8.0 && rect.min.x < bar.min.x)
}

#[test]
fn every_screen_fits_the_smallest_window() {
    let _gpu = gpu();
    let window = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(MIN.0, MIN.1));
    for (name, vm, primaries, card) in screens() {
        let harness = showing(vm, card);
        assert_eq!(harness.unnamed_widgets(), Vec::<String>::new(), "{name}");
        let bars: Vec<Rect> = harness.query_all_by_role(Role::ScrollBar).map(|n| n.rect()).collect();
        let mut outside = Vec::new();
        for node in harness.query_all_by(|n| is_control(n.role())) {
            let rect = node.rect();
            if !window.contains_rect(rect) && !in_scroll_area(rect, &bars) {
                let label = node.accesskit_node().label().unwrap_or_default();
                outside.push(format!("{label:?} at {rect:?}"));
            }
        }
        assert!(outside.is_empty(), "{name}: outside the window: {outside:?}");
        let workspace = harness.state().layout.workspace;
        for primary in primaries {
            let button = harness
                .query_all_by_label(primary)
                .find(|n| n.accesskit_node().role() == Role::Button)
                .unwrap_or_else(|| panic!("{name}: the primary button {primary:?}"));
            assert!(
                workspace.contains_rect(button.rect()),
                "{name}: {primary:?} at {:?} is not inside the workspace {workspace:?}",
                button.rect()
            );
        }
    }
}

/// The picture `<screen>_1280x800` of the screen named `screen` in `screens()`. One harness per test, so a bless
/// rewrites each picture on its own.
fn matches(screen: &str) {
    let _gpu = gpu();
    let (name, vm, _, card) = screens()
        .into_iter()
        .find(|s| s.0 == screen)
        .expect("a screen of the seven");
    let mut harness = showing(vm, card);
    harness.snapshot(format!("{name}_1280x800"));
}

#[test]
fn matchup_matches_at_1280x800() {
    matches("matchup");
}

#[test]
fn balance_matches_at_1280x800() {
    matches("balance");
}

#[test]
fn seeds_matches_at_1280x800() {
    matches("seeds");
}

#[test]
fn music_matches_at_1280x800() {
    matches("music");
}

#[test]
fn render_matches_at_1280x800() {
    matches("render");
}

#[test]
fn library_matches_at_1280x800() {
    matches("library");
}

#[test]
fn settings_matches_at_1280x800() {
    matches("settings");
}
