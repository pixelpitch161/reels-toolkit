//! The Matchups tab: every fight made, newest first, with an Uploaded tick per row and a footer that deletes the
//! selected row after asking once.

use bf_app::app::BfApp;
use bf_app::fixtures::{base, matchups};
use bf_app::model::View;
use bf_app::screens::matchups::{EMPTY, PICK_ONE, ROW_H};
use bf_app::state::{Action, ViewModel};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common::{MIN, REF, app_harness, gpu, pixels};

const NEW_TICK: &str = "Uploaded, Flute vs Trombone, seed 2210";

fn showing<'a>(size: (f32, f32), vm: ViewModel) -> Harness<'a, BfApp> {
    let mut harness = app_harness(size, move |cc| BfApp::showing(cc, vm));
    harness.run();
    harness
}

fn has(harness: &Harness<'_, BfApp>, text: &str) -> bool {
    harness.query_all_by_label(text).next().is_some()
}

fn control<'t>(harness: &'t Harness<'_, BfApp>, label: &'t str, role: Role) -> egui_kittest::Node<'t> {
    harness
        .query_all_by_label(label)
        .find(|n| n.accesskit_node().role() == role)
        .unwrap_or_else(|| panic!("a {role:?} named {label}"))
}

#[test]
fn the_header_counts_fights_pairings_and_not_uploaded() {
    let _gpu = gpu();
    let harness = showing(REF, matchups::vm());
    assert!(has(&harness, "Matchups"));
    assert!(has(&harness, "28 fights · 26 pairings · 1 not uploaded"));
}

#[test]
fn every_earlier_episode_is_ticked_and_the_new_fight_is_not() {
    let _gpu = gpu();
    let harness = showing(REF, matchups::vm());
    let new = control(&harness, NEW_TICK, Role::CheckBox);
    assert_eq!(new.accesskit_node().toggled(), Some(Toggled::False));
    let old = control(&harness, "Uploaded, Bass drum vs Tuba, seed 132", Role::CheckBox);
    assert_eq!(old.accesskit_node().toggled(), Some(Toggled::True));
}

#[test]
fn the_newest_fight_is_the_first_row() {
    let _gpu = gpu();
    let harness = showing(REF, matchups::vm());
    let mut ticks: Vec<(String, f32)> = harness
        .query_all_by_role(Role::CheckBox)
        .filter_map(|n| Some((n.accesskit_node().label()?, n.rect().min.y)))
        .collect();
    ticks.sort_by(|a, b| a.1.total_cmp(&b.1));
    assert_eq!(ticks[0].0, NEW_TICK);
    // The next is an episode of Oct 31.
    assert!(ticks[1].0.contains("Bass drum vs Tuba") || ticks[1].0.contains("Bass clarinet vs Bassoon"));
    assert!((ticks[1].1 - ticks[0].1 - (ROW_H + 1.0)).abs() < 0.5, "{ticks:?}");
}

#[test]
fn ticking_a_row_pushes_set_uploaded() {
    let _gpu = gpu();
    let mut harness = showing(REF, matchups::vm());
    control(&harness, NEW_TICK, Role::CheckBox).click();
    harness.run();
    assert_eq!(
        harness.state().log,
        vec![Action::SetUploaded {
            id: matchups::NEW_FIGHT_ID,
            uploaded: true
        }]
    );
    assert!(
        harness
            .state()
            .state
            .vm
            .history
            .fight(matchups::NEW_FIGHT_ID)
            .unwrap()
            .uploaded
    );
    assert!(has(&harness, "28 fights · 26 pairings"));
}

#[test]
fn clicking_a_row_selects_it() {
    let _gpu = gpu();
    let mut harness = showing(REF, matchups::vm());
    let name = harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| n.accesskit_node().label())
        .find(|l| l.starts_with("Bass drum vs Tuba, seed 132"))
        .expect("the Bass drum row");
    control(&harness, &name, Role::Button).click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SelectFight(26)]);
    assert!(has(&harness, "Bass drum vs Tuba, seed 132"));
}

#[test]
fn delete_asks_first_then_deletes() {
    let _gpu = gpu();
    let mut harness = showing(REF, matchups::vm());
    assert!(has(&harness, "Flute vs Trombone, seed 2210"));
    harness.get_by_label("Delete").click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::AskDeleteFight]);
    assert!(has(
        &harness,
        "Delete Flute vs Trombone, seed 2210? This cannot be undone."
    ));
    harness.get_by_label("Keep it").click();
    harness.run();
    assert!(!harness.state().state.vm.matchups.asking_delete);
    harness.get_by_label("Delete").click();
    harness.run();
    harness.get_by_label("Delete").click();
    harness.run();
    assert_eq!(
        harness.state().log.last(),
        Some(&Action::DeleteFight(matchups::NEW_FIGHT_ID))
    );
    assert!(harness.state().state.vm.history.fight(matchups::NEW_FIGHT_ID).is_none());
    assert!(has(&harness, PICK_ONE));
    assert!(has(&harness, "27 fights · 26 pairings"));
}

#[test]
fn a_fighter_the_roster_does_not_know_shows_its_name_in_grey() {
    let _gpu = gpu();
    let mut harness = showing(REF, matchups::unknown_fighter());
    let tick = control(&harness, "Uploaded, Kazoo vs Flute, seed 77", Role::CheckBox).rect();
    let label = harness
        .query_all_by_label("Kazoo")
        .map(|n| n.rect())
        .find(|r| (r.center().y - tick.center().y).abs() < 4.0)
        .expect("Kazoo on its row");
    // The swatch sits just left of the name: 12 wide, then an 8 gap.
    let at = ((label.min.x - 8.0 - 6.0) as u32, label.center().y as u32);
    assert_eq!(pixels(&mut harness, &[at]), vec![[0x80, 0x80, 0x80]]);
}

#[test]
fn an_empty_history_says_what_to_do() {
    let _gpu = gpu();
    let harness = showing(REF, matchups::empty());
    assert!(has(&harness, EMPTY));
    assert!(!has(&harness, PICK_ONE));
}

#[test]
fn the_toolbar_opens_matchups() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::vm());
    control(&harness, "Matchups", Role::Button).click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::Go(View::Matchups)]);
    assert_eq!(harness.state().state.vm.view, View::Matchups);
}

#[test]
fn every_control_is_named() {
    let _gpu = gpu();
    for vm in [matchups::vm(), matchups::unknown_fighter(), matchups::empty()] {
        let harness = showing(REF, vm);
        assert_eq!(harness.unnamed_widgets(), Vec::<String>::new());
    }
}

#[test]
fn matchups_matches() {
    let _gpu = gpu();
    let mut harness = showing(REF, matchups::vm());
    harness.snapshot("matchups_1440x900");
}

#[test]
fn matchups_matches_small() {
    let _gpu = gpu();
    let mut harness = showing(MIN, matchups::vm());
    harness.snapshot("matchups_1280x800");
}
