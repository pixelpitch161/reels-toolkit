//! The Match-up screen (interface reference, 5.1): the two corner cards, the roster, the bottom strip with the theme,
//! the tempo and the pair rules, and what clicking does.

use bf_app::app::BfApp;
use bf_app::fixtures::{base, matchup};
use bf_app::model::draft::Draft;
use bf_app::model::{Step, View};
use bf_app::screens::matchup::columns_for;
use bf_app::seam::sample;
use bf_app::state::{Action, ViewModel};
use egui::accesskit::{Role, Toggled};
use egui::{Key, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common::{MIN, REF, app_harness, gpu, pixels};

const NOT_BUILT: &str = "Not built yet. It arrives in an update.";
const RULE: &str = "A live lance knocks a risen cobra flat.";
const PROBLEM: &str = "These two cannot fight yet: the ring has no room for both weapons.";
const SECTIONS: [&str; 5] = ["Woodwinds", "Brass", "Strings", "Keys and electronics", "Percussion"];

/// The shell showing `vm` as it is.
fn showing<'a>(size: (f32, f32), vm: ViewModel) -> Harness<'a, BfApp> {
    app_harness(size, move |cc| BfApp::showing(cc, vm))
}

fn workspace(harness: &Harness<'_, BfApp>) -> Rect {
    harness.state().layout.workspace
}

/// The rectangles of every node labelled `text` (a text's label is its value) or, for a control, holding it as its
/// value, inside the workspace.
fn in_workspace(harness: &Harness<'_, BfApp>, text: &str) -> Vec<Rect> {
    let area = workspace(harness);
    let controls = harness
        .query_all_by_value(text)
        .filter(|n| n.accesskit_node().role() != Role::Label);
    harness
        .query_all_by_label(text)
        .chain(controls)
        .map(|n| n.rect())
        .filter(|r| area.contains_rect(*r))
        .collect()
}

/// The one chip named `name` (a button; it may be scrolled out of sight).
fn chip<'t>(harness: &'t Harness<'_, BfApp>, name: &'t str) -> egui_kittest::Node<'t> {
    harness
        .query_all_by_label(name)
        .find(|n| n.accesskit_node().role() == Role::Button)
        .unwrap_or_else(|| panic!("a chip named {name}"))
}

/// The one control named `name` with `role`.
fn control<'t>(harness: &'t Harness<'_, BfApp>, name: &'t str, role: Role) -> egui_kittest::Node<'t> {
    harness
        .query_all_by_label(name)
        .find(|n| n.accesskit_node().role() == role)
        .unwrap_or_else(|| panic!("a {role:?} named {name}"))
}

/// The tempo of `bars` bars, as the draft works it out.
fn tempo(bars: u32) -> f64 {
    Draft {
        bars,
        ..base::vm().draft
    }
    .tempo_bpm()
}

#[test]
fn every_word_of_the_mock_up() {
    let _gpu = gpu();
    let mut harness = showing(REF, matchup::vm());
    harness.run();
    let words = [
        "Left corner",
        "Right corner",
        "Snake charmer",
        "Cobras rise from dropped baskets and strike on the beat. Every hit that lands lets them reach further.",
        "Slide lance",
        "The slide shoots out at the enemy on the beat, then pulls back. Every hit that lands makes it longer. It can \
         parry.",
        "Abilities",
        "LV2",
        "LV3",
        "LOW",
        "Flutter tongue",
        "Twice as fast for 8 beats",
        "Snake dance",
        "Lasts 4 beats",
        "Piccolo",
        "A wave across the ring, below 25 HP",
        "Sad trombone",
        "A ring that hits and slows",
        "Power slide",
        "Pedal tone",
        "A heavy slam, below 25 HP",
        "Range",
        "C5 to A6",
        "E2 to F4",
        "Usually plays",
        "Melody",
        "Bass",
        "Colour",
        "Silver",
        "Mint",
        "Roster",
        "30 fighters",
        "Click for the left corner, right-click for the right",
        "Woodwinds",
        "Brass",
        "Strings",
        "Keys and electronics",
        "Percussion",
        "Theme",
        "Big top",
        "A sinister circus in a minor key. Leaves the bass and the top line to the fighters.",
        "Tempo",
        "Swap sides",
        "Balance this match-up",
        "34 bars fit in 61 seconds",
        "133.77 bpm",
        "Pair rules",
        RULE,
    ];
    for word in words {
        assert!(!in_workspace(&harness, word).is_empty(), "{word:?} is in the workspace");
    }
    for tag in ["LV2", "LV3", "LOW"] {
        assert_eq!(in_workspace(&harness, tag).len(), 2, "{tag} in both corners");
    }
    assert_eq!(in_workspace(&harness, "Twice as fast for 8 beats").len(), 2);
}

#[test]
fn the_roster_has_30_chips_in_5_sections() {
    let _gpu = gpu();
    let mut harness = showing(REF, matchup::vm());
    harness.run();
    let roster = sample::roster();
    assert_eq!(roster.fighters.len(), 30);
    let heads: Vec<Rect> = SECTIONS
        .iter()
        .map(|s| {
            let found = in_workspace(&harness, s);
            assert_eq!(found.len(), 1, "{s} once");
            found[0]
        })
        .collect();
    for pair in heads.windows(2) {
        assert!(pair[0].max.y < pair[1].min.y, "the sections run top to bottom");
    }
    let chips = harness
        .query_all_by_role(Role::Button)
        .filter(|n| {
            let name = n.accesskit_node().label().unwrap_or_default();
            roster
                .fighters
                .iter()
                .any(|f| name == f.display || name.starts_with(&format!("{}, ", f.display)))
        })
        .count();
    assert_eq!(chips, 30, "one chip per fighter");
    let order = [
        bf_app::model::roster::Section::Woodwinds,
        bf_app::model::roster::Section::Brass,
        bf_app::model::roster::Section::Strings,
        bf_app::model::roster::Section::Keys,
        bf_app::model::roster::Section::Percussion,
    ];
    for f in &roster.fighters {
        let name = match f.name.as_str() {
            "FLUTE" => "Flute, left corner".to_string(),
            "TROMBONE" => "Trombone, right corner".to_string(),
            _ => f.display.clone(),
        };
        let r = chip(&harness, &name).rect();
        let i = order.iter().position(|s| *s == f.section).unwrap();
        assert!(r.min.y > heads[i].max.y, "{name} is under {}", SECTIONS[i]);
        if let Some(next) = heads.get(i + 1) {
            assert!(r.max.y < next.min.y, "{name} is above {}", SECTIONS[i + 1]);
        }
    }
}

#[test]
fn the_grid_has_two_to_four_columns() {
    assert_eq!(columns_for(498.0), 4);
    assert_eq!(columns_for(338.0), 2);
    assert_eq!(columns_for(2000.0), 4);
    assert_eq!(columns_for(372.0), 3, "(372 + 6) / 126 is exactly 3");
    assert_eq!(columns_for(371.9), 2);
    assert_eq!(columns_for(0.0), 2);
    assert_eq!(columns_for(f32::NAN), 2);
}

#[test]
fn the_corner_cards_are_250_wide() {
    let _gpu = gpu();
    let mut harness = showing(REF, matchup::vm());
    harness.run();
    let area = workspace(&harness);
    assert_eq!(area.width(), 1030.0);
    let left = in_workspace(&harness, "Left corner")[0];
    let right = in_workspace(&harness, "Right corner")[0];
    assert_eq!(left.min.x, area.min.x + 12.0, "the left card's padding");
    assert_eq!(
        right.min.x,
        area.max.x - 250.0 + 12.0,
        "the right card starts 250 from the right"
    );
    let first = chip(&harness, "Flute, left corner").rect();
    let fourth = chip(&harness, "Bass clarinet").rect();
    assert_eq!(
        first.min.x,
        area.min.x + 250.0 + 16.0,
        "the roster starts at 250, padded 16"
    );
    assert_eq!(
        fourth.max.x,
        area.max.x - 250.0 - 16.0,
        "and ends 250 from the right, padded 16"
    );
    assert_eq!(first.width(), 120.0, "four chips of 120 with 6 between");
    assert_eq!(first.height(), 30.0);
    for text in ["Snake charmer", "C5 to A6", "Silver"] {
        let r = in_workspace(&harness, text)[0];
        assert!(
            r.max.x <= area.min.x + 250.0 - 12.0,
            "{text} stays inside the left card"
        );
    }
    for text in ["Slide lance", "E2 to F4", "Mint"] {
        let r = in_workspace(&harness, text)[0];
        assert!(
            r.min.x >= area.max.x - 250.0 + 12.0,
            "{text} stays inside the right card"
        );
    }
}

#[test]
fn a_click_and_a_right_click_pick_corners() {
    let _gpu = gpu();
    let mut harness = showing(REF, matchup::vm());
    harness.run();
    chip(&harness, "Oboe").click();
    harness.run();
    assert_eq!(
        harness.state().log,
        [Action::PickFighter {
            side: 0,
            name: "OBOE".to_string()
        }]
    );
    assert_eq!(harness.state().state.vm.draft.fighters[0], "OBOE");
    chip(&harness, "Oboe, left corner").click_secondary();
    harness.run();
    assert_eq!(
        harness.state().log[1],
        Action::PickFighter {
            side: 1,
            name: "OBOE".to_string()
        }
    );
    assert_eq!(
        harness.state().state.vm.draft.fighters,
        ["TROMBONE".to_string(), "OBOE".to_string()],
        "picking the fighter in the other corner swaps"
    );
}

#[test]
fn the_chosen_two_are_marked() {
    let _gpu = gpu();
    let mut harness = showing(REF, matchup::vm());
    harness.run();
    for name in ["Flute, left corner", "Trombone, right corner"] {
        assert_eq!(
            chip(&harness, name).accesskit_node().toggled(),
            Some(Toggled::True),
            "{name}"
        );
    }
    assert_ne!(chip(&harness, "Oboe").accesskit_node().toggled(), Some(Toggled::True));
    assert_eq!(in_workspace(&harness, "Left").len(), 1, "the chip says Left");
    assert_eq!(in_workspace(&harness, "Right").len(), 1, "the chip says Right");
}

#[test]
fn a_fighter_not_built_cannot_be_picked() {
    let _gpu = gpu();
    let mut vm = matchup::vm();
    let roster = vm.roster.as_mut().unwrap();
    roster.fighters.iter_mut().find(|f| f.name == "OBOE").unwrap().available = false;
    let mut harness = showing(REF, vm);
    harness.run();
    assert!(chip(&harness, "Oboe").accesskit_node().is_disabled());
    chip(&harness, "Oboe").click();
    harness.run();
    chip(&harness, "Oboe").click_secondary();
    harness.run();
    assert_eq!(harness.state().log, [], "nothing is picked");
    chip(&harness, "Oboe").hover();
    harness.run();
    assert!(harness.query_by_label(NOT_BUILT).is_some(), "hovering says why");
    assert!(
        !in_workspace(&harness, "29 of 30 ready").is_empty(),
        "the count says how many are ready"
    );
}

#[test]
fn swap_and_balance_buttons() {
    let _gpu = gpu();
    let mut harness = showing(REF, matchup::vm());
    harness.run();
    harness.get_by_label("Swap sides").click();
    harness.run();
    assert_eq!(harness.state().log, [Action::SwapSides]);
    assert_eq!(
        harness.state().state.vm.draft.fighters,
        ["TROMBONE".to_string(), "FLUTE".to_string()]
    );
    assert!(harness.query_by_label("Trombone, left corner").is_some());
    harness.get_by_label("Balance this match-up").click();
    harness.run();
    assert_eq!(harness.state().log[1], Action::Go(View::Step(Step::Balance)));
}

#[test]
fn the_tempo_slider_moves_in_bars() {
    let _gpu = gpu();
    let mut harness = showing(REF, matchup::vm());
    harness.run();
    let slider = control(&harness, "Tempo", Role::Slider);
    assert_eq!(slider.accesskit_node().numeric_value(), Some(34.0));
    assert_eq!(
        slider.accesskit_node().min_numeric_value(),
        Some(24.0),
        "the theme's range"
    );
    assert_eq!(slider.accesskit_node().max_numeric_value(), Some(42.0));
    slider.focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(harness.state().log, [Action::SetBars(35)], "one whole bar");
    assert_eq!(harness.state().state.vm.draft.bars, 35);
    assert!(!in_workspace(&harness, "35 bars fit in 61 seconds").is_empty());
    let shown = format!("{:.2} bpm", tempo(35));
    assert!(!in_workspace(&harness, &shown).is_empty(), "the field shows {shown}");

    // Typing a tempo picks the nearest bar count: 125 bpm is nearest 32 bars (125.90).
    let field = control(&harness, "Tempo in bpm", Role::SpinButton);
    field.focus();
    harness.run();
    control(&harness, "Tempo in bpm", Role::SpinButton).type_text("125");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().log.last(), Some(&Action::SetBars(32)));
    assert_eq!(harness.state().state.vm.draft.bars, 32);
    assert!(!in_workspace(&harness, &format!("{:.2} bpm", tempo(32))).is_empty());
}

#[test]
fn pair_rules_are_read_only() {
    let _gpu = gpu();
    let mut harness = showing(REF, matchup::vm());
    harness.run();
    let found: Vec<_> = harness.query_all_by_label(RULE).collect();
    assert_eq!(found.len(), 1, "the rule is on screen once");
    let node = found[0].accesskit_node();
    assert_eq!(node.role(), Role::Label, "and is text, not a control");
    assert!(!egui::accessibility::is_input(node.role()));
    drop(found);
    drop(harness);

    let mut harness = showing(REF, base::vm());
    harness.run();
    assert!(!in_workspace(&harness, "Pair rules").is_empty());
    assert!(!in_workspace(&harness, "None for this pair").is_empty());
}

#[test]
fn a_fight_problem_blocks_balancing() {
    let _gpu = gpu();
    let mut vm = matchup::vm();
    vm.fight_problem = Some(PROBLEM.to_string());
    let mut harness = showing(REF, vm);
    harness.run();
    let text = in_workspace(&harness, PROBLEM);
    assert_eq!(text.len(), 1, "the problem stands in the workspace");
    let swap = harness.get_by_label("Swap sides").rect();
    assert!(text[0].max.y <= swap.min.y, "above the bottom strip");
    assert!(text[0].min.y > in_workspace(&harness, "Percussion")[0].max.y);
    let balance = harness.get_by_label("Balance this match-up");
    assert!(balance.accesskit_node().is_disabled());
    balance.click();
    harness.run();
    assert_eq!(harness.state().log, [], "balancing is blocked");
    // Its words are TEXT_STRONG: white glyph pixels inside its rectangle.
    let r = text[0];
    let mut at = Vec::new();
    for y in (r.min.y as u32)..(r.max.y as u32) {
        for x in (r.min.x as u32)..(r.min.x as u32 + 200) {
            at.push((x, y));
        }
    }
    let found = pixels(&mut harness, &at);
    assert!(found.contains(&[255, 255, 255]), "the problem is drawn in TEXT_STRONG");
}

#[test]
fn at_the_minimum_size() {
    let _gpu = gpu();
    let mut harness = showing(MIN, matchup::vm());
    harness.run();
    let area = workspace(&harness);
    assert_eq!(area.width(), 870.0);
    for label in ["Swap sides", "Balance this match-up"] {
        let r = harness.get_by_label(label).rect();
        assert!(
            area.contains_rect(r),
            "{label} at {r:?} lies inside the workspace {area:?}"
        );
    }
    let swap = harness.get_by_label("Swap sides").rect();
    let balance = harness.get_by_label("Balance this match-up").rect();
    assert!(swap.max.x <= balance.min.x, "the buttons do not overlap");
    let field = control(&harness, "Tempo in bpm", Role::SpinButton).rect();
    let note = in_workspace(&harness, "34 bars fit in 61 seconds")[0];
    assert!(
        field.max.x <= note.min.x && note.max.x <= swap.min.x,
        "the tempo row does not overlap"
    );
    let first = chip(&harness, "Flute, left corner").rect();
    assert_eq!(first.width(), 166.0, "two columns at the minimum size");

    let bottom = chip(&harness, "Bass drum").rect();
    assert!(bottom.min.y > swap.min.y, "Bass drum starts below the fold: {bottom:?}");
    chip(&harness, "Bass drum").scroll_to_me();
    harness.run();
    let r = chip(&harness, "Bass drum").rect();
    let strip_top = swap.min.y - 12.0 - 26.0 - 8.0 - 1.0;
    assert!(
        r.min.y >= area.min.y && r.max.y <= strip_top,
        "Bass drum is brought on screen: {r:?}"
    );
    chip(&harness, "Bass drum").click();
    harness.run();
    assert_eq!(
        harness.state().log,
        [Action::PickFighter {
            side: 0,
            name: "BASS DRUM".to_string()
        }]
    );
}

#[test]
fn nothing_is_unnamed() {
    let _gpu = gpu();
    let mut problem = matchup::vm();
    problem.fight_problem = Some(PROBLEM.to_string());
    problem.roster.as_mut().unwrap().fighters[3].available = false;
    for (size, vm) in [(REF, matchup::vm()), (MIN, base::vm()), (REF, problem)] {
        let mut harness = showing(size, vm);
        harness.run();
        assert_eq!(harness.unnamed_widgets(), Vec::<String>::new());
    }
}

#[test]
fn match_up_matches() {
    let _gpu = gpu();
    let mut harness = showing(REF, matchup::vm());
    let vm = &harness.state().state.vm;
    let masks = [0, 1].map(|side| vm.fighter(side).expect("a fighter").mask.clone());
    let card = sample::title_card(vm.colors(), [&masks[0], &masks[1]]);
    harness.state_mut().show_frame(card);
    harness.run();
    harness.snapshot("matchup_1440x900");
}

#[test]
fn the_keyboard_works_the_screen() {
    let _gpu = gpu();
    let mut harness = showing(REF, matchup::vm());
    harness.run();
    let area = workspace(&harness);
    let balance = harness.get_by_label("Balance this match-up").rect();
    assert_eq!(
        balance.max.x,
        area.max.x - 16.0,
        "the main action ends at the strip's padding"
    );
    chip(&harness, "Oboe").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(
        harness.state().log,
        [Action::PickFighter {
            side: 0,
            name: "OBOE".to_string()
        }],
        "Enter on a chip picks the left corner"
    );
    // Tab goes on from the tempo to the buttons, in the order they are read.
    control(&harness, "Tempo", Role::Slider).focus();
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    assert!(control(&harness, "Tempo in bpm", Role::SpinButton).is_focused());
    harness.key_press(Key::Tab);
    harness.run();
    assert!(harness.get_by_label("Swap sides").is_focused());
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!(harness.state().log.last(), Some(&Action::SwapSides));
    harness.key_press(Key::Tab);
    harness.run();
    assert!(harness.get_by_label("Balance this match-up").is_focused());
    // Its focus ring, 2 px wide and 2 px outside it, is whole at the strip's padding.
    let r = harness.get_by_label("Balance this match-up").rect();
    let ring = [
        ((r.max.x + 3.0) as u32, r.center().y as u32),
        (r.center().x as u32, (r.min.y - 3.0) as u32),
        (r.center().x as u32, (r.max.y + 2.0) as u32),
    ];
    assert_eq!(
        pixels(&mut harness, &ring),
        vec![[255, 255, 255]; 3],
        "the ring is not cut off"
    );
}
