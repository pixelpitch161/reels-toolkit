//! The Balance screen (interface reference, section 5.2): the win split, the damage handicap, the four numbers, when
//! the K.O. lands, and the way on to the search.

use bf_app::app::BfApp;
use bf_app::fixtures::{balance, base};
use bf_app::model::{Step, View};
use bf_app::screens::balance::{HISTOGRAM, WIN_SPLIT, one_in};
use bf_app::seam::sample;
use bf_app::state::{Action, ViewModel};
use bf_app::theme::large_fill;
use egui::{Event, Modifiers, PointerButton, Pos2, Rect, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common::{MIN, REF, app_harness, gpu, pixels};

const NOTE: &str =
    "Damage is the only thing you can change here. Reach, timing and abilities stay exactly as each fighter was built.";
const FOOTER: &str = "Changing the handicap gives a new set of fights, so the search starts over.";
const NOT_MEASURED: &str = "Not measured yet. Auto-balance finds the handicap for you.";
const RESET: &str = "Reset both to ×1.00";

/// The shell showing `vm` as it is.
fn showing<'a>(size: (f32, f32), vm: ViewModel) -> Harness<'a, BfApp> {
    app_harness(size, move |cc| BfApp::showing(cc, vm))
}

/// Whether a node with exactly `label` is on screen (there may be several).
fn has(harness: &Harness<'_, BfApp>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// The text a node shows: a label keeps it in its value, any other node in its name.
fn text_of(node: &egui_kittest::Node<'_>) -> String {
    let node = node.accesskit_node();
    let text = if node.role() == egui::accesskit::Role::Label {
        node.value()
    } else {
        node.label()
    };
    text.unwrap_or_default()
}

fn is_disabled(harness: &Harness<'_, BfApp>, label: &str) -> bool {
    harness.get_by_label(label).accesskit_node().is_disabled()
}

/// The handicap actions logged so far, as (side, value, released).
fn handicaps(harness: &Harness<'_, BfApp>) -> Vec<(usize, f64, bool)> {
    harness
        .state()
        .log
        .iter()
        .filter_map(|a| match a {
            Action::SetHandicap { side, value, released } => Some((*side, *value, *released)),
            _ => None,
        })
        .collect()
}

fn press(harness: &Harness<'_, BfApp>, at: Pos2) {
    harness.event(Event::PointerMoved(at));
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
}

fn release(harness: &Harness<'_, BfApp>, at: Pos2) {
    harness.event(Event::PointerMoved(at));
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
}

/// The base on Balance with no numbers and nothing running.
fn unmeasured() -> ViewModel {
    let mut vm = base::vm();
    vm.view = View::Step(Step::Balance);
    vm.step = Step::Balance;
    vm
}

/// The pixel at `share` of `rect`'s width, at its middle height.
fn at_share(rect: Rect, share: f32) -> (u32, u32) {
    ((rect.min.x + share * rect.width()) as u32, rect.center().y as u32)
}

#[test]
fn every_word_of_the_mock_up() {
    let _gpu = gpu();
    let mut harness = showing(REF, balance::vm());
    harness.run();
    for label in [
        "Flute wins 47%",
        "Trombone wins 53%",
        "3,000 fights, on seeds the search never uses",
        "40%",
        "60%",
        "In range",
        "Damage handicap",
        RESET,
        "Auto-balance",
        "Flute damage",
        "Trombone damage",
        NOTE,
        "Typical K.O.",
        "50.6 s",
        "Most land between 44.2 and 56.9 s",
        "Lead changes",
        "2.6",
        "On average per fight",
        "No K.O. by 58 s",
        "4%",
        "These never reach the shortlist",
        "Fit the beat sheet",
        "8%",
        "About 1 fight in 12",
        "When the K.O. lands",
        "Lighter bars are inside the beat sheet's window, 49 to 53 s",
        "30 s",
        "37 s",
        "44 s",
        "51 s",
        "58 s",
        FOOTER,
        "Search seeds",
    ] {
        assert!(has(&harness, label), "{label:?} is on screen");
    }
    for value in ["×2.00", "×0.40"] {
        assert!(
            harness.query_all_by_value(value).next().is_some(),
            "{value:?} is on screen"
        );
    }
    assert!(!has(&harness, NOT_MEASURED));
    assert!(!has(&harness, "Stop balancing"));
}

#[test]
fn one_in_reads_the_fit_rate() {
    assert_eq!(one_in(0.083), "About 1 fight in 12");
    assert_eq!(one_in(0.5), "About 1 fight in 2");
    assert_eq!(one_in(0.0), "None in this sample");
    assert_eq!(one_in(1.0), "About 1 fight in 1");
    assert_eq!(one_in(0.0002), "About 1 fight in 5,000");
    for nothing in [f64::NAN, f64::INFINITY, -0.1] {
        assert_eq!(one_in(nothing), "–", "{nothing} is no value");
    }
}

#[test]
fn the_column_is_780_wide_and_centred() {
    let _gpu = gpu();
    for size in [REF, MIN] {
        let mut harness = showing(size, balance::vm());
        harness.run();
        let workspace = harness.state().layout.workspace;
        let left = (workspace.center().x - 390.0).round();
        let bar = harness.get_by_label(WIN_SPLIT).rect();
        assert_eq!(
            (bar.min.x, bar.width()),
            (left, 780.0),
            "the bar spans the column at {size:?}"
        );
        let flute = harness.get_by_label("Flute wins 47%").rect();
        assert_eq!(flute.min.x, left, "the left share starts the column");
        let trombone = harness.get_by_label("Trombone wins 53%").rect();
        assert!((trombone.max.x - (left + 780.0)).abs() <= 1.0, "{trombone:?}");
        let search = harness.get_by_label("Search seeds").rect();
        assert!((search.max.x - (left + 780.0)).abs() <= 1.0, "{search:?}");
        let histogram = harness.get_by_label(HISTOGRAM).rect();
        assert_eq!((histogram.min.x, histogram.width()), (left, 780.0));
        let top = harness.get_by_label("Flute wins 47%").rect().min.y;
        assert!(
            (top - (workspace.min.y + 28.0)).abs() <= 1.0,
            "28 above the column: {top}"
        );
    }
}

#[test]
fn the_split_bar_follows_the_share() {
    let _gpu = gpu();
    let mut harness = showing(REF, balance::vm());
    harness.run();
    let colors = harness.state().state.vm.colors();
    let bar = harness.get_by_label(WIN_SPLIT).rect();
    let at = [
        at_share(bar, 0.46),
        at_share(bar, 0.48),
        at_share(bar, 0.40),
        at_share(bar, 0.60),
        (at_share(bar, 0.40).0, bar.min.y as u32 - 3),
        (at_share(bar, 0.60).0, bar.max.y as u32 + 2),
    ];
    let seen = pixels(&mut harness, &at);
    let fill = |c: [u8; 3]| {
        let c = large_fill(c);
        [c.r(), c.g(), c.b()]
    };
    assert_eq!(seen[0], fill(colors[0]), "Flute's share is Flute's large fill");
    assert_eq!(seen[1], fill(colors[1]), "the rest is Trombone's");
    assert_eq!(seen[2], [255; 3], "the 40 % mark");
    assert_eq!(seen[3], [255; 3], "the 60 % mark");
    assert_eq!(seen[4], [255; 3], "the marks reach above the bar");
    assert_eq!(seen[5], [255; 3], "and below it");
}

#[test]
fn the_split_bar_has_its_fighters_top_edges() {
    let _gpu = gpu();
    let mut harness = showing(REF, balance::vm());
    harness.run();
    let colors = harness.state().state.vm.colors();
    let bar = harness.get_by_label(WIN_SPLIT).rect();
    let top = bar.min.y as u32;
    let seen = pixels(
        &mut harness,
        &[(at_share(bar, 0.2).0, top), (at_share(bar, 0.8).0, top + 1)],
    );
    assert_eq!(seen, colors.to_vec(), "a 2 px top edge in each fighter's full colour");
}

#[test]
fn out_of_range_names_the_side() {
    let _gpu = gpu();
    for (share, words) in [(0.71, "Flute wins too often"), (0.2, "Trombone wins too often")] {
        let mut vm = balance::vm();
        vm.balance.stats.as_mut().expect("numbers").win_share = share;
        let mut harness = showing(REF, vm);
        harness.run();
        assert!(has(&harness, words), "{words}");
        assert!(!has(&harness, "In range"));
    }
}

#[test]
fn the_window_bars_are_lighter() {
    let _gpu = gpu();
    let mut harness = showing(REF, balance::vm());
    harness.run();
    let histogram = harness.get_by_label(HISTOGRAM).rect();
    // 28 bars 2 apart across the histogram; the bar for second s is number s - 30.
    let bar_w = (histogram.width() - 27.0 * 2.0) / 28.0;
    let centre = |second: f32| (histogram.min.x + (second - 30.0) * (bar_w + 2.0) + bar_w / 2.0) as u32;
    let low = histogram.max.y as u32 - 3;
    let seen = pixels(&mut harness, &[(centre(50.0), low), (centre(40.0), low)]);
    assert_eq!(seen[0], [0xe2; 3], "second 50 is inside the window");
    assert_eq!(seen[1], [0x66; 3], "second 40 is outside it");
}

#[test]
fn the_tallest_bar_fills_the_histogram() {
    let _gpu = gpu();
    let mut harness = showing(REF, balance::vm());
    harness.run();
    let histogram = harness.get_by_label(HISTOGRAM).rect();
    let bar_w = (histogram.width() - 27.0 * 2.0) / 28.0;
    // The count 100 is second 48's; second 30's count is 2, so its bar is 2 % of the height.
    let x48 = (histogram.min.x + 18.0 * (bar_w + 2.0) + bar_w / 2.0) as u32;
    let x30 = (histogram.min.x + bar_w / 2.0) as u32;
    let near_top = histogram.min.y as u32 + 1;
    let seen = pixels(&mut harness, &[(x48, near_top), (x30, near_top)]);
    assert_eq!(seen[0], [0x66; 3], "the largest count reaches the top");
    assert_eq!(seen[1], [0x28; 3], "a small count does not");
}

#[test]
fn dragging_a_slider_sets_the_handicap_and_releases() {
    let _gpu = gpu();
    let mut harness = showing(REF, balance::vm());
    harness.run();
    let slider = harness.get_by_label("Flute damage").rect();
    let y = slider.center().y;
    harness.drag_at(pos2(slider.min.x + 0.6 * slider.width(), y));
    harness.run();
    harness.hover_at(pos2(slider.min.x + 0.4 * slider.width(), y));
    harness.run();
    harness.hover_at(pos2(slider.min.x + 0.25 * slider.width(), y));
    harness.run();
    harness.drop_at(pos2(slider.min.x + 0.25 * slider.width(), y));
    harness.run();
    let moves = handicaps(&harness);
    assert!(moves.len() >= 2, "{moves:?}");
    assert!(moves.iter().all(|m| m.0 == 0), "only Flute's handicap moves: {moves:?}");
    assert!(!moves[0].2, "a drag in progress is not released: {moves:?}");
    let last = *moves.last().unwrap();
    assert!(last.2, "letting go releases: {moves:?}");
    assert_eq!(moves.iter().filter(|m| m.2).count(), 1, "released once: {moves:?}");
    assert!((0.6..0.9).contains(&last.1), "a quarter of 0.05 to 3.00: {}", last.1);
    let handicap = harness.state().state.vm.draft.handicap;
    assert_eq!(handicap, [last.1, 0.4]);
    assert!(
        harness.query_all_by_value(&format!("×{:.2}", last.1)).next().is_some(),
        "the value field follows"
    );
}

#[test]
fn a_handicap_above_three_pins_the_knob_but_the_field_takes_it() {
    let _gpu = gpu();
    let mut vm = balance::vm();
    vm.draft.handicap = [7.5, 0.4];
    let mut harness = showing(REF, vm);
    harness.run();
    let node = harness.get_by_label("Flute damage");
    assert_eq!(
        node.accesskit_node().numeric_value(),
        Some(3.0),
        "the knob stays at the end"
    );
    assert!(harness.query_all_by_value("×7.50").next().is_some());
    assert!(handicaps(&harness).is_empty(), "showing it changes nothing");
    assert_eq!(harness.state().state.vm.draft.handicap, [7.5, 0.4]);
}

#[test]
fn typing_a_handicap_sets_it_and_releases() {
    let _gpu = gpu();
    let mut harness = showing(REF, balance::vm());
    harness.run();
    let field = harness.get_by_label("Trombone damage value");
    field.focus();
    harness.run();
    harness.get_by_label("Trombone damage value").type_text("1.25");
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    let moves = handicaps(&harness);
    assert_eq!(
        moves.last(),
        Some(&(1, 1.25, true)),
        "Enter releases the typed number: {moves:?}"
    );
    assert_eq!(moves.iter().filter(|m| m.2).count(), 1, "released once: {moves:?}");
    assert!(moves.iter().all(|m| m.0 == 1), "{moves:?}");
    assert_eq!(harness.state().state.vm.draft.handicap, [2.0, 1.25]);
}

#[test]
fn passing_through_a_field_changes_nothing() {
    let _gpu = gpu();
    let mut harness = showing(REF, balance::vm());
    harness.run();
    harness.get_by_label("Flute damage value").focus();
    harness.run();
    harness.get_by_label("Search seeds").focus();
    harness.run();
    assert!(handicaps(&harness).is_empty(), "{:?}", harness.state().log);
}

#[test]
fn the_arrow_keys_move_a_slider_a_step_and_release() {
    let _gpu = gpu();
    let mut harness = showing(REF, balance::vm());
    harness.run();
    harness.get_by_label("Trombone damage").focus();
    harness.run();
    harness.key_press(egui::Key::ArrowRight);
    harness.run();
    let moves = handicaps(&harness);
    assert_eq!(moves.last(), Some(&(1, 0.41, true)), "{moves:?}");
    assert_eq!(harness.state().state.vm.draft.handicap, [2.0, 0.41]);
}

#[test]
fn the_buttons_push_their_actions() {
    let _gpu = gpu();
    for (label, action) in [
        (RESET, Action::ResetHandicap),
        ("Auto-balance", Action::AutoBalance),
        ("Search seeds", Action::Go(View::Step(Step::Seeds))),
    ] {
        let mut harness = showing(REF, balance::vm());
        harness.run();
        harness.get_by_label(label).click();
        harness.run();
        assert_eq!(harness.state().log, vec![action], "{label}");
    }
    let mut harness = showing(REF, balance::running());
    harness.run();
    harness.get_by_label("Stop balancing").click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::StopBalance]);
}

#[test]
fn while_balancing_the_controls_wait() {
    let _gpu = gpu();
    let mut harness = showing(REF, balance::running());
    harness.run();
    assert!(has(&harness, "Trying handicaps, step 9 of 29"));
    assert!(has(&harness, "Stop balancing"));
    assert!(!has(&harness, "Auto-balance"));
    assert!(harness.query_by_label(WIN_SPLIT).is_none(), "the bar is replaced");
    assert!(has(&harness, "Balance progress"));
    for label in ["Flute damage", "Trombone damage", RESET] {
        assert!(is_disabled(&harness, label), "{label} is disabled");
    }
    let slider = harness.get_by_label("Flute damage").rect();
    press(&harness, slider.center());
    harness.run();
    release(&harness, slider.center());
    harness.run();
    harness.get_by_label(RESET).click();
    harness.run();
    assert!(harness.state().log.is_empty(), "nothing moves while balancing");
}

#[test]
fn not_measured_yet_says_what_to_do() {
    let _gpu = gpu();
    let mut harness = showing(REF, unmeasured());
    harness.run();
    assert!(has(&harness, NOT_MEASURED));
    assert_eq!(harness.query_all_by_label("–").count(), 4, "the four numbers read –");
    for label in [
        "In range",
        "50.6 s",
        "About 1 fight in 12",
        "3,000 fights, on seeds the search never uses",
    ] {
        assert!(!has(&harness, label), "{label} is not shown");
    }
    assert!(has(&harness, "Auto-balance"));
    assert!(!is_disabled(&harness, "Flute damage"));
}

#[test]
fn numbers_that_are_not_numbers_read_as_none() {
    let _gpu = gpu();
    let mut vm = balance::vm();
    let stats = vm.balance.stats.as_mut().expect("numbers");
    stats.ko_median = f64::NAN;
    stats.lead_changes_mean = f64::INFINITY;
    stats.no_ko_share = -0.0001;
    stats.fit_rate = f64::NAN;
    stats.ko_p10 = f64::NAN;
    let mut harness = showing(REF, vm);
    harness.run();
    assert_eq!(
        harness.query_all_by_label("–").count(),
        6,
        "four numbers, the K.O. range and the fit note"
    );
    for node in harness.query_all_by_role(egui::accesskit::Role::Label) {
        let label = text_of(&node);
        assert!(!label.contains("NaN") && !label.contains("inf"), "{label}");
    }
}

#[test]
fn a_k_o_range_the_wrong_way_round_reads_as_none() {
    let _gpu = gpu();
    let mut vm = balance::vm();
    let stats = vm.balance.stats.as_mut().expect("numbers");
    stats.ko_p10 = 57.0;
    stats.ko_p90 = 44.0;
    let mut harness = showing(REF, vm);
    harness.run();
    assert_eq!(harness.query_all_by_label("–").count(), 1, "the range note");
    assert!(!harness.query_all_by_label_contains("Most land").any(|_| true));
}

#[test]
fn the_two_shares_add_up_to_100() {
    let _gpu = gpu();
    for share in [0.475, 0.4949, 0.505, 0.125, 0.995, 0.0, 1.0] {
        let mut vm = balance::vm();
        vm.balance.stats.as_mut().expect("numbers").win_share = share;
        let mut harness = showing(REF, vm);
        harness.run();
        let percent = |prefix: &str| -> u32 {
            let node = harness
                .query_all_by_label_contains(prefix)
                .find(|n| text_of(n).ends_with('%'))
                .unwrap_or_else(|| panic!("{prefix} at {share}"));
            let label = text_of(&node);
            label[prefix.len()..]
                .trim_end_matches('%')
                .parse()
                .expect("a whole percentage")
        };
        let (left, right) = (percent("Flute wins "), percent("Trombone wins "));
        assert_eq!(left + right, 100, "{share}: {left} + {right}");
    }
}

#[test]
fn a_share_on_a_bound_is_in_range() {
    let _gpu = gpu();
    for (share, words) in [
        (0.40, "In range"),
        (0.60, "In range"),
        (0.3999, "Trombone wins too often"),
        (0.6001, "Flute wins too often"),
    ] {
        let mut vm = balance::vm();
        vm.balance.stats.as_mut().expect("numbers").win_share = share;
        let mut harness = showing(REF, vm);
        harness.run();
        assert!(has(&harness, words), "{share}: {words}");
        let verdicts = ["In range", "Flute wins too often", "Trombone wins too often"];
        assert_eq!(
            verdicts.iter().filter(|v| has(&harness, v)).count(),
            1,
            "{share}: one verdict"
        );
    }
}

#[test]
fn a_rare_k_o_second_still_shows() {
    let _gpu = gpu();
    let mut vm = balance::vm();
    let mut hist = vec![0; 28];
    hist[0] = 1;
    hist[18] = 1_000;
    vm.balance.stats.as_mut().expect("numbers").ko_hist = hist;
    let mut harness = showing(REF, vm);
    harness.run();
    let histogram = harness.get_by_label(HISTOGRAM).rect();
    let bar_w = (histogram.width() - 27.0 * 2.0) / 28.0;
    let x = |i: f32| (histogram.min.x + i * (bar_w + 2.0) + bar_w / 2.0) as u32;
    let bottom = histogram.max.y as u32 - 1;
    let seen = pixels(&mut harness, &[(x(0.0), bottom), (x(1.0), bottom)]);
    assert_eq!(seen[0], [0x66; 3], "a count of 1 in 1,000 is still 1 px tall");
    assert_eq!(seen[1], [0x28; 3], "a count of 0 draws nothing");
}

#[test]
fn a_fight_problem_holds_auto_balance() {
    let _gpu = gpu();
    let problem = "These two cannot fight yet.";
    let mut vm = balance::vm();
    vm.fight_problem = Some(problem.to_string());
    let mut harness = showing(REF, vm);
    harness.run();
    assert!(is_disabled(&harness, "Auto-balance"));
    harness.get_by_label("Auto-balance").hover();
    harness.run();
    assert!(has(&harness, problem), "hovering says why");
    harness.get_by_label("Auto-balance").click();
    harness.run();
    assert!(harness.state().log.is_empty(), "{:?}", harness.state().log);
}

/// Whether a `TEXT_STRONG` pixel lies 1 to 5 px outside `rect` on the side `side` ("above", "below", "left",
/// "right"), along a line through `along` (a share of that side's length).
fn ring_beside(harness: &mut Harness<'_, BfApp>, rect: Rect, side: &str, along: f32) -> bool {
    let at: Vec<(u32, u32)> = (1..=5)
        .map(|d| {
            let d = d as f32;
            let x = rect.min.x + along * rect.width();
            let y = rect.min.y + along * rect.height();
            match side {
                "above" => (x, rect.min.y - d),
                "below" => (x, rect.max.y - 1.0 + d),
                "left" => (rect.min.x - d, y),
                _ => (rect.max.x - 1.0 + d, y),
            }
        })
        .map(|(x, y)| (x as u32, y as u32))
        .collect();
    pixels(harness, &at).contains(&[255; 3])
}

/// Whether a `TEXT_STRONG` pixel lies on the slider's row edge (its ring is drawn inside the 26 tall row), on the
/// side `side`, at `along` of its width.
fn ring_on_row_edge(harness: &mut Harness<'_, BfApp>, row: Rect, side: &str, along: f32) -> bool {
    let x = (row.min.x + along * row.width()) as u32;
    let ys: Vec<(u32, u32)> = (0..4)
        .map(|d| match side {
            "above" => (x, row.min.y as u32 + d),
            _ => (x, row.max.y as u32 - 1 - d),
        })
        .collect();
    pixels(harness, &ys).contains(&[255; 3])
}

#[test]
fn every_control_shows_its_focus_ring() {
    let _gpu = gpu();
    for size in [REF, MIN] {
        let mut harness = showing(size, balance::vm());
        harness.run();
        // Buttons and the value fields: the ring is 2 px outside them. With nothing focused there is none.
        let controls = [
            RESET,
            "Auto-balance",
            "Search seeds",
            "Flute damage value",
            "Trombone damage value",
        ];
        let sides = ["above", "below", "left", "right"];
        for label in controls {
            let rect = harness.get_by_label(label).rect();
            for side in sides {
                assert!(
                    !ring_beside(&mut harness, rect, side, 0.3),
                    "{label} has no ring before focus"
                );
            }
        }
        for label in controls {
            let rect = harness.get_by_label(label).rect();
            harness.get_by_label(label).focus();
            harness.run();
            for side in sides {
                assert!(
                    ring_beside(&mut harness, rect, side, 0.3),
                    "{label}: ring {side} at {size:?}"
                );
            }
        }
        // The sliders: the ring is inside their 26 tall rows, which touch; the lower row must not cover the upper
        // row's ring, nor the upper the lower's.
        for label in ["Flute damage", "Trombone damage"] {
            let row = harness.get_by_label(label).rect();
            harness.get_by_label(label).focus();
            harness.run();
            for side in ["above", "below"] {
                assert!(
                    ring_on_row_edge(&mut harness, row, side, 0.1),
                    "{label}: ring {side} at {size:?}"
                );
            }
        }
    }
}

#[test]
fn tab_walks_the_screen_in_order() {
    let _gpu = gpu();
    let mut harness = showing(REF, balance::vm());
    harness.run();
    let order = [
        RESET,
        "Auto-balance",
        "Flute damage",
        "Flute damage value",
        "Trombone damage",
        "Trombone damage value",
        "Search seeds",
    ];
    harness.get_by_label(order[0]).focus();
    harness.run();
    for pair in order.windows(2) {
        assert!(
            harness.get_by_label(pair[0]).is_focused(),
            "{} has the keyboard",
            pair[0]
        );
        harness.key_press(egui::Key::Tab);
        harness.run();
        assert!(
            harness.get_by_label(pair[1]).is_focused(),
            "Tab from {} goes to {}",
            pair[0],
            pair[1]
        );
    }
    assert!(handicaps(&harness).is_empty(), "walking through changes nothing");
}

#[test]
fn at_the_minimum_size_while_balancing() {
    let _gpu = gpu();
    let mut harness = showing(MIN, balance::running());
    harness.run();
    let workspace = harness.state().layout.workspace;
    for label in [
        "Stop balancing",
        RESET,
        "Flute damage",
        "Trombone damage",
        "Search seeds",
        "Balance progress",
    ] {
        let rect = harness.get_by_label(label).rect();
        assert!(workspace.contains_rect(rect), "{label} {rect:?} inside {workspace:?}");
    }
    assert!(has(&harness, "Trying handicaps, step 9 of 29"));
}

#[test]
fn at_the_minimum_size() {
    let _gpu = gpu();
    for vm in [balance::vm(), unmeasured()] {
        let mut harness = showing(MIN, vm);
        harness.run();
        let workspace = harness.state().layout.workspace;
        for label in ["Search seeds", "Auto-balance", RESET, "Flute damage", "Trombone damage"] {
            let rect = harness.get_by_label(label).rect();
            assert!(workspace.contains_rect(rect), "{label} {rect:?} inside {workspace:?}");
        }
        let histogram = harness.get_by_label(HISTOGRAM).rect();
        assert!(
            histogram.height() >= 60.0 && histogram.height() < 120.0,
            "{histogram:?}"
        );
        let search = harness.get_by_label("Search seeds").rect();
        assert!(
            search.max.y <= workspace.max.y - 28.0 + 0.5,
            "28 below the column: {search:?} in {workspace:?}"
        );
    }
}

#[test]
fn the_histogram_is_120_tall_at_the_reference_size() {
    let _gpu = gpu();
    let mut harness = showing(REF, balance::vm());
    harness.run();
    assert_eq!(harness.get_by_label(HISTOGRAM).rect().height(), 120.0);
}

#[test]
fn nothing_is_unnamed() {
    let _gpu = gpu();
    for vm in [balance::vm(), balance::running(), unmeasured()] {
        let mut harness = showing(REF, vm);
        harness.run();
        assert_eq!(harness.unnamed_widgets(), Vec::<String>::new());
    }
}

#[test]
fn balance_matches() {
    let _gpu = gpu();
    let mut harness = showing(REF, balance::vm());
    let vm = &harness.state().state.vm;
    let masks = [0, 1].map(|side| vm.fighter(side).expect("a fighter").mask.clone());
    let card = sample::title_card(vm.colors(), [&masks[0], &masks[1]]);
    harness.state_mut().show_frame(card);
    harness.run();
    harness.snapshot("balance_1440x900");
}

/// A kit pair's Balance screen names the series' handicap for the match-up (ruling M3-R6) and, once the handicap
/// has moved from it, offers the way back.
#[test]
fn a_kit_pair_names_its_series_handicap() {
    let _gpu = gpu();
    let mut vm = crate::music::kit_vm("bassdrum_vs_tuba", bf_app::model::Step::Balance, false);
    let words = bf_app::screens::balance::series_words([0.86, 1.0]);
    assert_eq!(words, "The series' handicap for this match-up: ×0.86 and ×1.00.");
    let harness = showing_vm(vm.clone());
    assert!(harness.query_all_by_label(&words).next().is_some(), "{words}");
    assert!(
        harness
            .query_all_by_label(bf_app::screens::balance::USE_SERIES)
            .next()
            .is_none(),
        "already on it"
    );
    drop(harness);
    vm.draft.handicap = [1.2, 1.0];
    let mut harness = showing_vm(vm);
    harness.get_by_label(bf_app::screens::balance::USE_SERIES).click();
    harness.run();
    assert_eq!(harness.state().log, vec![bf_app::state::Action::UseSeriesHandicap]);
    assert_eq!(harness.unnamed_widgets(), Vec::<String>::new());
}

fn showing_vm<'a>(vm: bf_app::state::ViewModel) -> egui_kittest::Harness<'a, bf_app::app::BfApp> {
    let mut harness = crate::common::app_harness(crate::common::REF, move |cc| bf_app::app::BfApp::showing(cc, vm));
    harness.run();
    harness
}
