//! The Seeds screen (interface reference, section 5.3): the filters, the shortlist, its footer and the watch box.

use bf_app::app::BfApp;
use bf_app::fixtures::seeds;
use bf_app::model::shortlist::{FilterSpec, Finish, SearchState};
use bf_app::model::{Step, View};
use bf_app::screens::seeds::{parse_seed, seed_columns};
use bf_app::state::{Action, ViewModel};
use bf_app::theme::{LINE_SOFT, PANEL, SELECTED};
use egui::accesskit::{Role, Toggled};
use egui::{Key, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common::{MIN, REF, app_harness, gpu, pixels};

const FOOTER: &str = "212 fights fit the beat sheet, from 18,240 checked. Still searching.";
const SHOW_OUTSIDE: &str = "Show fights outside the beat sheet";
const EMPTY_SEARCHING: &str = "No fights fit the beat sheet yet. The search is still running.";
const EMPTY_STOPPED: &str = "No fights fit the beat sheet in 18,240 checked. Keep searching, or use the switch on the \
                             left to show fights outside the beat sheet.";
const MISS: &str = "K.O. at 47.20 s (needs 49 to 53 s)";

/// The shell showing `vm` as it is.
fn showing<'a>(size: (f32, f32), vm: ViewModel) -> Harness<'a, BfApp> {
    let mut harness = app_harness(size, move |cc| BfApp::showing(cc, vm));
    harness.run();
    harness
}

/// The shortlist's rows on screen, top to bottom: their accessible names and rects.
fn rows(harness: &Harness<'_, BfApp>) -> Vec<(String, Rect)> {
    let mut rows: Vec<(String, Rect)> = harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| {
            let label = n.accesskit_node().label()?;
            label.starts_with("Seed ").then(|| (label, n.rect()))
        })
        .collect();
    rows.sort_by(|a, b| a.1.min.y.total_cmp(&b.1.min.y));
    rows
}

/// The control with `label` and `role`.
fn by_role<'t, 'h>(harness: &'t Harness<'h, BfApp>, label: &'t str, role: Role) -> egui_kittest::Node<'t> {
    harness
        .query_all_by_label(label)
        .find(|n| n.accesskit_node().role() == role)
        .unwrap_or_else(|| panic!("a {role:?} named {label}"))
}

/// The filters the screen pushed last.
fn last_filters(harness: &Harness<'_, BfApp>) -> FilterSpec {
    harness
        .state()
        .log
        .iter()
        .rev()
        .find_map(|a| match a {
            Action::SetFilters(f) => Some(f.clone()),
            _ => None,
        })
        .expect("SetFilters was pushed")
}

fn has(harness: &Harness<'_, BfApp>, text: &str) -> bool {
    harness.query_all_by_label(text).next().is_some()
}

#[test]
fn every_word_of_the_mock_up() {
    let _gpu = gpu();
    let harness = showing(REF, seeds::vm());
    for text in [
        "Filters",
        "Winner",
        "Any",
        "Flute",
        "Trombone",
        "K.O. lands between",
        "and",
        "Lead changes",
        "At least",
        "Finish",
        "Close",
        "Comeback",
        "This ability lands",
        SHOW_OUTSIDE,
        "Watch any seed",
        "Watch",
        "Seed",
        "Who leads",
        "Flute above",
        "Trombone below",
        "K.O.",
        "Changes",
        "Story",
        "16 HP left",
        "49.1 s",
        "7.84",
        FOOTER,
        "Stop search",
        "Use seed 18433",
    ] {
        assert!(has(&harness, text), "{text}");
    }
    assert_eq!(harness.get_by_label("Earliest K.O.").value().as_deref(), Some("49.0 s"));
    assert_eq!(harness.get_by_label("Latest K.O.").value().as_deref(), Some("53.0 s"));
    assert_eq!(
        harness.get_by_label("Lead changes, at least").value().as_deref(),
        Some("2")
    );
    assert_eq!(
        by_role(&harness, "This ability lands", Role::ComboBox)
            .value()
            .as_deref(),
        Some("Any ability")
    );
    let field = harness.get_by(|node| node.placeholder() == Some("Seed number"));
    assert_eq!(field.accesskit_node().label().as_deref(), Some("Seed to watch"));
}

#[test]
fn the_columns_share_the_row() {
    let c = seed_columns(792.0);
    assert_eq!((c.ribbon, c.winner, c.hp_left), (294.0, 150.0, true));
    let c = seed_columns(632.0);
    assert_eq!((c.ribbon, c.winner, c.hp_left), (188.0, 96.0, false));
    assert_eq!(seed_columns(1200.0).ribbon, 400.0);
    assert_eq!(seed_columns(300.0).ribbon, 120.0, "never under 120");
    assert_eq!(seed_columns(f32::NAN).ribbon, 120.0);
}

#[test]
fn typed_seeds() {
    assert_eq!(parse_seed("20418"), Some(20418));
    assert_eq!(parse_seed(" 20418 "), Some(20418));
    assert_eq!(parse_seed("9223372036854775807"), Some(9_223_372_036_854_775_807));
    for text in [
        "",
        "abc",
        "-3",
        "1.5",
        "99999999999999999999",
        "9223372036854775808",
        "+3",
        "1 2",
    ] {
        assert_eq!(parse_seed(text), None, "{text:?}");
    }
}

#[test]
fn the_filters_column_is_230_wide() {
    let _gpu = gpu();
    let mut harness = showing(REF, seeds::vm());
    let filters = harness.get_by_label("Filters").rect();
    assert_eq!(filters.min.x, 12.0, "padded 12 at the side");
    assert_eq!(filters.min.y, 88.0 + 14.0, "and 14 above");
    let any = harness
        .query_all_by_label("Any")
        .map(|n| n.rect())
        .min_by(|a, b| a.min.y.total_cmp(&b.min.y))
        .unwrap();
    let trombone = by_role(&harness, "Trombone", Role::RadioButton).rect();
    assert_eq!(
        (any.min.x, trombone.max.x),
        (12.0, 218.0),
        "the winner's segments fill the column"
    );
    let y = 400;
    let px = pixels(&mut harness, &[(228, y), (229, y), (230, y)]);
    let rgb = |c: egui::Color32| [c.r(), c.g(), c.b()];
    assert_eq!(
        px,
        vec![rgb(PANEL), rgb(LINE_SOFT), rgb(PANEL)],
        "a 1 px line on its right"
    );
    // The header starts after the list's 4 and the row's 10: rank 30, a gap of 12, then the seed.
    let workspace = harness.state().layout.workspace;
    let seed = harness
        .query_all_by_label("Seed")
        .map(|n| n.rect())
        .find(|r| workspace.contains_rect(*r))
        .expect("the header's Seed (the bout bar has one too)");
    assert_eq!(seed.min.x, 230.0 + 14.0 + 30.0 + 12.0);
    assert!(seed.min.y >= workspace.min.y && seed.max.y <= workspace.min.y + 30.0);
}

#[test]
fn fourteen_rows_ranked() {
    let _gpu = gpu();
    let mut harness = showing(REF, seeds::vm());
    let rows = rows(&harness);
    assert_eq!(rows.len(), 14);
    assert_eq!(rows[0].0, "Seed 18433, Trombone wins at 49.1 s");
    assert_eq!(rows[1].0, "Seed 5627, Flute wins at 50.3 s");
    assert_eq!(rows[11].0, "Seed 20418, Trombone wins at 52.2 s");
    for (i, (_, rect)) in rows.iter().enumerate() {
        assert_eq!(rect.height(), 38.0, "rows are 38 tall");
        assert_eq!(rect.width(), 792.0, "the list is padded 4 at each side");
        assert_eq!(rect.min.x, 234.0);
        if i > 0 {
            assert_eq!(rect.min.y, rows[i - 1].1.max.y, "one under another");
        }
    }
    let first = harness.get_by_label(&rows[0].0);
    assert_eq!(first.accesskit_node().toggled(), Some(Toggled::True), "the chosen row");
    let second = harness.get_by_label(&rows[1].0);
    assert_eq!(second.accesskit_node().toggled(), Some(Toggled::False));
    for rank in ["1", "2", "14"] {
        assert!(has(&harness, rank), "rank {rank}");
    }
    let (r0, r1) = (rows[0].1, rows[1].1);
    let rgb = |c: egui::Color32| [c.r(), c.g(), c.b()];
    let px = pixels(
        &mut harness,
        &[
            (r0.min.x as u32 + 5, r0.center().y as u32),
            (r1.min.x as u32 + 5, r1.center().y as u32),
        ],
    );
    assert_eq!(px, vec![rgb(SELECTED), rgb(PANEL)], "the selected look");
    assert!(has(&harness, "Use seed 18433"));
}

#[test]
fn clicking_a_row_selects_its_seed() {
    let _gpu = gpu();
    let mut harness = showing(REF, seeds::vm());
    harness.get_by_label("Seed 5627, Flute wins at 50.3 s").click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SelectSeed(5627)]);
    harness.get_by_label("Seed 5627, Flute wins at 50.3 s").click();
    harness.run();
    assert_eq!(
        harness.state().log,
        vec![Action::SelectSeed(5627)],
        "the chosen row is not asked for again"
    );
}

#[test]
fn each_filter_pushes_the_whole_spec() {
    let _gpu = gpu();
    let mut harness = showing(REF, seeds::vm());
    let start = FilterSpec::default();
    by_role(&harness, "Trombone", Role::RadioButton).click();
    harness.run();
    assert_eq!(
        last_filters(&harness),
        FilterSpec {
            winner: Some(1),
            ..start.clone()
        }
    );
    by_role(&harness, "Close", Role::RadioButton).click();
    harness.run();
    let spec = FilterSpec {
        winner: Some(1),
        finish: Finish::Close,
        ..start.clone()
    };
    assert_eq!(last_filters(&harness), spec);
    by_role(&harness, SHOW_OUTSIDE, Role::Switch).click();
    harness.run();
    let spec = FilterSpec {
        show_outside: true,
        ..spec
    };
    assert_eq!(last_filters(&harness), spec);
    by_role(&harness, "This ability lands", Role::ComboBox).click();
    harness.run();
    by_role(&harness, "Snake dance", Role::ListBoxOption).click();
    harness.run();
    let spec = FilterSpec {
        ability: Some("SNAKE DANCE".to_string()),
        ..spec
    };
    assert_eq!(last_filters(&harness), spec, "the roster's own name is stored");
    by_role(&harness, "Any", Role::RadioButton).click();
    harness.run();
    assert_eq!(last_filters(&harness).winner, None, "the first Any is the winner's");
    let pushes = harness
        .state()
        .log
        .iter()
        .filter(|a| matches!(a, Action::SetFilters(_)))
        .count();
    assert_eq!(pushes, 5, "one push per change");
}

#[test]
fn typing_a_filter_value_pushes_it() {
    let _gpu = gpu();
    let mut harness = showing(REF, seeds::vm());
    harness.get_by_label("Lead changes, at least").focus();
    harness.run();
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(last_filters(&harness).min_lead_changes, 3);
    harness.get_by_label("Earliest K.O.").focus();
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    let ko = last_filters(&harness).ko;
    assert!(ko.0 < 49.0 && ko.1 == 53.0, "{ko:?}");
}

#[test]
fn the_footer_by_state() {
    let _gpu = gpu();
    let harness = showing(REF, seeds::vm());
    assert!(has(&harness, FOOTER));
    assert!(has(&harness, "Stop search"));
    harness.get_by_label("Stop search").click();
    let mut harness = harness;
    harness.run();
    assert_eq!(harness.state().log, vec![Action::StopSearch]);
    drop(harness);

    let mut vm = seeds::vm();
    vm.shortlist.state = SearchState::Stopped;
    let mut harness = showing(REF, vm);
    assert!(has(
        &harness,
        "212 fights fit the beat sheet, from 18,240 checked. Search stopped."
    ));
    assert!(!has(&harness, "Stop search"));
    harness.get_by_label("Keep searching").click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SearchOn]);
    drop(harness);

    let mut vm = seeds::vm();
    vm.shortlist.state = SearchState::Full;
    vm.shortlist.fits_total = 200;
    let harness = showing(REF, vm);
    assert!(has(
        &harness,
        "200 fights fit the beat sheet, from 18,240 checked. The shortlist is full."
    ));
    assert!(!has(&harness, "Stop search") && !has(&harness, "Keep searching"));
    drop(harness);

    // Idle lasts only until the search is asked for, and reads as searching.
    let mut vm = seeds::vm();
    vm.shortlist.state = SearchState::Idle;
    let harness = showing(REF, vm);
    assert!(has(&harness, FOOTER));
    assert!(has(&harness, "Stop search"));
}

#[test]
fn empty_states_say_what_to_do() {
    let _gpu = gpu();
    let harness = showing(REF, seeds::empty(SearchState::Searching));
    assert!(has(&harness, EMPTY_SEARCHING));
    assert!(!has(&harness, EMPTY_STOPPED));
    assert!(rows(&harness).is_empty());
    drop(harness);
    let harness = showing(REF, seeds::empty(SearchState::Stopped));
    assert!(has(&harness, EMPTY_STOPPED));
    assert!(!has(&harness, EMPTY_SEARCHING));
    let text = harness.get_by_label(EMPTY_STOPPED).rect();
    let workspace = harness.state().layout.workspace;
    assert!(
        text.min.x >= 230.0 && text.max.x <= workspace.max.x,
        "inside the list: {text:?}"
    );
}

#[test]
fn a_row_outside_the_beat_sheet_is_marked() {
    let _gpu = gpu();
    let mut vm = seeds::vm();
    vm.filters.show_outside = true;
    let outside = &mut vm.shortlist.rows[1];
    outside.fits = false;
    outside.misses = vec![MISS.to_string(), "1 lead change (needs 2)".to_string()];
    let harness = showing(REF, vm);
    assert!(has(&harness, "–"), "its rank");
    assert!(has(&harness, MISS), "its first miss");
    assert!(!has(&harness, "1 lead change (needs 2)"), "only the first");
    assert!(!has(&harness, "7.75"), "in place of its score");
    // The ranks count only the fights that fit.
    assert!(has(&harness, "13") && !has(&harness, "14"));
    let miss = harness.get_by_label(MISS).rect();
    let row = harness.get_by_label("Seed 5627, Flute wins at 50.3 s").rect();
    assert!(row.contains_rect(miss), "inside its row");
    // Ruling M2-R23: it spans the K.O., changes and story columns, which it replaces for this row.
    let ko_column = row.min.x + 10.0 + 30.0 + 12.0 + 62.0 + 12.0 + 294.0 + 12.0 + 150.0 + 12.0;
    assert_eq!(miss.min.x, ko_column, "it starts where the K.O. would");
    assert!(!has(&harness, "50.3 s"), "the K.O. column gives way to it");
}

#[test]
fn a_long_miss_is_cut_short_and_shown_whole_on_hover() {
    let _gpu = gpu();
    let long = "winner HP 46.60 (needs 30 or less), and the K.O. lands at 47.20 s (needs 49 to 53 s)";
    let mut vm = seeds::vm();
    vm.filters.show_outside = true;
    vm.shortlist.rows[1].fits = false;
    vm.shortlist.rows[1].misses = vec![long.to_string()];
    let mut harness = showing(MIN, vm);
    let miss = harness.get_by_label(long).rect();
    let row = harness.get_by_label("Seed 5627, Flute wins at 50.3 s").rect();
    // "K.O. at …" alone is about 50 wide; the three columns give it about 150 at the minimum window.
    assert!(miss.width() > 120.0, "more than a few words show: {miss:?}");
    assert!(miss.max.x <= row.max.x - 10.0, "cut short inside the row's padding");
    assert_eq!(harness.query_all_by_label(long).count(), 1);
    harness.get_by_label(long).hover();
    harness.run();
    assert_eq!(harness.query_all_by_label(long).count(), 2, "hovering shows all of it");

    // The fixture's own miss, at the reference size: more of it shows than "K.O. at".
    drop(harness);
    let mut vm = seeds::vm();
    vm.filters.show_outside = true;
    vm.shortlist.rows[1].fits = false;
    vm.shortlist.rows[1].misses = vec![MISS.to_string()];
    let mut harness = showing(REF, vm);
    assert!(harness.get_by_label(MISS).rect().width() > 120.0);
    harness.get_by_label(MISS).hover();
    harness.run();
    assert_eq!(harness.query_all_by_label(MISS).count(), 2, "hovering shows all of it");
}

/// The colour of the pixel at `at`.
fn pixel(harness: &mut Harness<'_, BfApp>, at: egui::Pos2) -> [u8; 3] {
    pixels(harness, &[(at.x as u32, at.y as u32)])[0]
}

/// The focus ring (2 px `TEXT_STRONG`, 2 px outside) shows on each side of `rect` named in `sides`
/// ('l', 'r', 't', 'b'), read at the middle of that side.
fn assert_ring(harness: &mut Harness<'_, BfApp>, rect: Rect, sides: &str, what: &str) {
    let white = [0xff, 0xff, 0xff];
    for side in sides.chars() {
        let at = match side {
            'l' => egui::pos2(rect.min.x - 3.0, rect.center().y),
            'r' => egui::pos2(rect.max.x + 2.0, rect.center().y),
            't' => egui::pos2(rect.center().x, rect.min.y - 3.0),
            _ => egui::pos2(rect.center().x, rect.max.y + 2.0),
        };
        assert_eq!(pixel(harness, at), white, "{what}: the ring on side {side} at {at:?}");
    }
}

#[test]
fn a_focused_row_shows_its_ring_at_the_top_and_the_bottom_of_the_list() {
    let _gpu = gpu();
    // The top row has the keyboard while the row under it is the chosen one (which fills itself).
    let mut vm = seeds::vm();
    vm.draft.seed = Some(5627);
    let mut harness = showing(REF, vm);
    let top = "Seed 18433, Trombone wins at 49.1 s";
    harness.get_by_label(top).focus();
    harness.run();
    assert!(harness.get_by_label(top).accesskit_node().is_focused());
    let rect = harness.get_by_label(top).rect();
    assert_ring(&mut harness, rect, "ltb", "the top row");
    drop(harness);

    // The last of 60 rows, chosen, so the list is scrolled to its end.
    let mut vm = seeds::vm();
    let sample = vm.shortlist.rows.clone();
    vm.shortlist.rows = (0..60)
        .map(|i| {
            let mut story = sample[i % sample.len()].clone();
            story.seed = 1000 + i as u64;
            story
        })
        .collect();
    vm.draft.seed = Some(1059);
    let mut harness = showing(REF, vm);
    let (last, _) = rows(&harness)
        .into_iter()
        .find(|(n, _)| n.starts_with("Seed 1059,"))
        .expect("the chosen row is scrolled into view");
    harness.get_by_label(&last).focus();
    harness.run();
    let rect = harness.get_by_label(&last).rect();
    let workspace = harness.state().layout.workspace;
    assert!(rect.max.y <= workspace.max.y - 46.0, "above the footer: {rect:?}");
    assert_ring(&mut harness, rect, "ltb", "the bottom row");
}

#[test]
fn the_filters_edge_controls_and_the_watch_box_show_their_ring() {
    let _gpu = gpu();
    let mut harness = showing(REF, seeds::vm());
    // The topmost segment named `label`: "Any" is the winner's, not the finish's.
    fn radio<'t>(harness: &'t Harness<'_, BfApp>, label: &'t str) -> egui_kittest::Node<'t> {
        harness
            .query_all_by_label(label)
            .filter(|n| n.accesskit_node().role() == Role::RadioButton)
            .min_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y))
            .unwrap_or_else(|| panic!("a segment {label}"))
    }
    for (label, sides) in [("Any", "ltb"), ("Trombone", "rtb"), ("Comeback", "rtb")] {
        radio(&harness, label).focus();
        harness.run();
        let rect = radio(&harness, label).rect();
        assert_ring(&mut harness, rect, sides, label);
    }
    harness.get_by_label("Seed to watch").focus();
    harness.run();
    let rect = harness.get_by_label("Seed to watch").rect();
    assert_ring(&mut harness, rect, "ltb", "the watch box");
    harness.get_by_label("Seed to watch").type_text("20418");
    harness.run();
    harness.get_by_label("Watch").focus();
    harness.run();
    let rect = harness.get_by_label("Watch").rect();
    assert_ring(&mut harness, rect, "rtb", "Watch");
}

#[test]
fn a_clicked_row_takes_the_arrows() {
    let _gpu = gpu();
    let mut harness = showing(REF, seeds::vm());
    // A real click: the pointer moves there, presses and releases.
    harness.get_by_label("Seed 5627, Flute wins at 50.3 s").click();
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(
        harness.state().log,
        vec![Action::SelectSeed(5627), Action::SelectSeed(8175)]
    );
}

#[test]
fn only_the_switch_carries_its_name() {
    let _gpu = gpu();
    let mut harness = showing(REF, seeds::vm());
    let named = harness
        .query_all_by_label(SHOW_OUTSIDE)
        .filter(|n| !n.accesskit_node().is_hidden())
        .count();
    assert_eq!(named, 1, "the words beside it are hidden from screen readers");
    // The words still turn it over when clicked.
    let switch = by_role(&harness, SHOW_OUTSIDE, Role::Switch).rect();
    let words = egui::pos2(switch.max.x + 40.0, switch.min.y + 13.0);
    harness.hover_at(words);
    harness.drag_at(words);
    harness.drop_at(words);
    harness.run();
    assert!(last_filters(&harness).show_outside);
}

#[test]
fn use_seed_names_the_seed_and_pushes() {
    let _gpu = gpu();
    let mut harness = showing(REF, seeds::vm());
    let button = harness.get_by_label("Use seed 18433");
    assert!(!button.accesskit_node().is_disabled());
    button.click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::UseSeed]);
    assert_eq!(harness.state().state.vm.view, View::Step(Step::Music));
}

#[test]
fn use_seed_waits_for_a_fight() {
    let _gpu = gpu();
    let mut harness = showing(REF, seeds::empty(SearchState::Searching));
    let button = harness.get_by_label("Use seed");
    assert!(button.accesskit_node().is_disabled());
    assert!(!has(&harness, "Pick a fight first"));
    button.hover();
    harness.run();
    assert!(has(&harness, "Pick a fight first"), "hovering says why");
    harness.get_by_label("Use seed").click();
    harness.run();
    assert!(harness.state().log.is_empty());
    drop(harness);

    let problem = "Seed 13 has no K.O. before 58 s, so it cannot be made into a video. Pick another seed.";
    let mut vm = seeds::vm();
    vm.draft.seed = Some(13);
    vm.take.story = None;
    vm.take.problem = Some(problem.to_string());
    let mut harness = showing(REF, vm);
    let button = harness.get_by_label("Use seed 13");
    assert!(button.accesskit_node().is_disabled());
    button.hover();
    harness.run();
    assert!(
        harness.query_all_by_label(problem).count() >= 2,
        "the status bar and the hover say it"
    );
}

#[test]
fn watching_a_typed_seed() {
    let _gpu = gpu();
    let mut harness = showing(REF, seeds::vm());
    let watch = harness.get_by_label("Watch");
    assert!(watch.accesskit_node().is_disabled(), "nothing typed yet");
    watch.hover();
    harness.run();
    assert!(has(&harness, "Type a whole number"));
    harness.get_by_label("Seed to watch").focus();
    harness.run();
    harness.get_by_label("Seed to watch").type_text("1.5");
    harness.run();
    assert!(
        harness.get_by_label("Watch").accesskit_node().is_disabled(),
        "not a whole number"
    );
    for _ in 0..3 {
        harness.key_press(Key::Backspace);
    }
    harness.get_by_label("Seed to watch").type_text("20418");
    harness.run();
    let watch = harness.get_by_label("Watch");
    assert!(!watch.accesskit_node().is_disabled());
    watch.click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SelectSeed(20418)]);
}

#[test]
fn up_and_down_move_the_selection() {
    let _gpu = gpu();
    let mut vm = seeds::vm();
    let sample = vm.shortlist.rows.clone();
    vm.shortlist.rows = (0..60)
        .map(|i| {
            let mut story = sample[i % sample.len()].clone();
            story.seed = 1000 + i as u64;
            story
        })
        .collect();
    vm.draft.seed = Some(1000);
    let mut harness = showing(MIN, vm);
    let name = |harness: &Harness<'_, BfApp>, seed: u64| {
        rows(harness)
            .into_iter()
            .find(|(n, _)| n.starts_with(&format!("Seed {seed},")))
    };
    let (first, _) = name(&harness, 1000).expect("the first row");
    harness.get_by_label(&first).focus();
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SelectSeed(1001)]);
    for _ in 0..20 {
        harness.key_press(Key::ArrowDown);
        harness.run();
    }
    assert_eq!(harness.state().log.last(), Some(&Action::SelectSeed(1021)));
    let workspace = harness.state().layout.workspace;
    let (label, rect) = name(&harness, 1021).expect("row 22 was scrolled into view");
    assert!(
        rect.min.y >= workspace.min.y + 30.0 && rect.max.y <= workspace.max.y - 46.0,
        "between the header and the footer: {rect:?}"
    );
    let row = harness.get_by_label(&label);
    assert!(row.accesskit_node().is_focused(), "the keyboard went with it");
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(harness.state().log.last(), Some(&Action::SelectSeed(1020)));
    assert!(
        harness.state().log.iter().all(|a| matches!(a, Action::SelectSeed(_))),
        "the transport did not take the keys: {:?}",
        harness.state().log
    );
}

#[test]
fn only_the_visible_rows_are_drawn() {
    let _gpu = gpu();
    let mut vm = seeds::vm();
    let sample = vm.shortlist.rows.clone();
    vm.shortlist.rows = (0..400)
        .map(|i| {
            let mut story = sample[i % sample.len()].clone();
            story.seed = 1000 + i as u64;
            story
        })
        .collect();
    let harness = showing(REF, vm);
    let drawn = rows(&harness).len();
    assert!((15..=20).contains(&drawn), "{drawn} rows drawn of 400");
}

#[test]
fn at_the_minimum_size() {
    let _gpu = gpu();
    let harness = showing(MIN, seeds::vm());
    let workspace = harness.state().layout.workspace;
    for label in ["Use seed 18433", "Stop search", "Watch", "Seed to watch", SHOW_OUTSIDE] {
        let r = by_role_any(&harness, label);
        assert!(workspace.contains_rect(r), "{label} at {r:?} lies inside {workspace:?}");
    }
    let rows = rows(&harness);
    assert!(!rows.is_empty());
    let row = rows[0].1;
    assert_eq!(row.width(), 632.0);
    assert!(seed_columns(row.width()).ribbon >= 120.0);
    let footer = harness.get_by_label(FOOTER).rect();
    let stop = harness.get_by_label("Stop search").rect();
    assert!(footer.max.x <= stop.min.x, "the footer's words stop before its button");
}

/// The rect of the first control named `label` that is not a plain label.
fn by_role_any(harness: &Harness<'_, BfApp>, label: &str) -> Rect {
    harness
        .query_all_by_label(label)
        .find(|n| n.accesskit_node().role() != Role::Label)
        .unwrap_or_else(|| panic!("a control named {label}"))
        .rect()
}

#[test]
fn nothing_is_unnamed() {
    let _gpu = gpu();
    for vm in [
        seeds::vm(),
        seeds::empty(SearchState::Searching),
        seeds::empty(SearchState::Stopped),
    ] {
        let harness = showing(REF, vm);
        assert_eq!(harness.unnamed_widgets(), Vec::<String>::new());
    }
}

#[test]
fn seeds_matches() {
    let _gpu = gpu();
    let mut harness = showing(REF, seeds::vm());
    harness.snapshot("seeds_1440x900");
}
