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
    let third = chip(&harness, "Clarinet").rect();
    assert_eq!(
        first.min.x,
        area.min.x + 250.0 + 16.0,
        "the roster starts at 250, padded 16"
    );
    assert_eq!(
        third.max.x,
        area.max.x - 250.0 - 16.0,
        "and ends 250 from the right, padded 16"
    );
    assert_eq!(
        third.min.y, first.min.y,
        "three chips to a row: the longest name stays whole (M2-R24)"
    );
    assert_eq!(first.width(), 162.0, "three chips of 162 with 6 between");
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
    assert!(
        text[0].min.y > in_workspace(&harness, "Left corner")[0].max.y,
        "below the columns"
    );
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
    let oboe = chip(&harness, "Oboe").rect();
    let clarinet = chip(&harness, "Clarinet").rect();
    assert!(
        oboe.min.y == first.min.y && clarinet.min.y > first.min.y,
        "two columns at the minimum size"
    );
    assert_eq!(oboe.min.x, first.max.x + 6.0);

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

// ---------------------------------------------------------------------------------------------------------------
// Ruling M2-R24 (whole names), the visible scroll bar, focus rings and Tab order (rule 19)
// ---------------------------------------------------------------------------------------------------------------

/// Every text the last frame painted: its full text, whether it was cut short, and the characters actually drawn.
fn painted_texts(harness: &Harness<'_, BfApp>) -> Vec<(String, bool, String)> {
    fn walk(shape: &egui::Shape, found: &mut Vec<(String, bool, String)>) {
        match shape {
            egui::Shape::Text(text) => {
                let drawn: String = text
                    .galley
                    .rows
                    .iter()
                    .flat_map(|r| r.row.glyphs.iter().map(|g| g.chr))
                    .collect();
                found.push((text.galley.text().to_string(), text.galley.elided, drawn));
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, found)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    for clipped in &harness.output().shapes {
        walk(&clipped.shape, &mut found);
    }
    found
}

/// The chip's accessible name for `display` in the Match-up fixture.
fn chip_name(display: &str) -> String {
    match display {
        "Flute" => "Flute, left corner".to_string(),
        "Trombone" => "Trombone, right corner".to_string(),
        _ => display.to_string(),
    }
}

#[test]
fn no_chip_name_is_cut_short() {
    let _gpu = gpu();
    let roster = sample::roster();
    for size in [REF, MIN] {
        let mut harness = showing(size, matchup::vm());
        harness.run();
        for fighter in &roster.fighters {
            let name = chip_name(&fighter.display);
            chip(&harness, &name).scroll_to_me();
            harness.run();
            let texts = painted_texts(&harness);
            let painted: Vec<_> = texts.iter().filter(|(text, _, _)| *text == fighter.display).collect();
            assert_eq!(painted.len(), 1, "{size:?}: {} is painted once", fighter.display);
            let (_, elided, drawn) = painted[0];
            assert!(!elided, "{size:?}: {} is cut short", fighter.display);
            assert_eq!(drawn, &fighter.display, "{size:?}: the whole name is drawn");
        }
    }
}

#[test]
fn the_roster_shows_its_scroll_bar_when_it_overflows() {
    let _gpu = gpu();
    let mut harness = showing(MIN, matchup::vm());
    harness.run();
    let area = workspace(&harness);
    let top = area.min.y as u32;
    let column = |harness: &mut Harness<'_, BfApp>, x: f32| {
        let at: Vec<(u32, u32)> = (top + 2..top + 60).map(|y| (x as u32, y)).collect();
        pixels(harness, &at)
    };
    let bar = column(&mut harness, area.max.x - 250.0 - 4.0 - 3.0);
    assert!(
        bar.contains(&[0x77, 0x77, 0x77]),
        "the roster's handle is drawn FAINT without hovering: {bar:?}"
    );
    drop(harness);
    // The left card fits at the reference size, so no bar is drawn there.
    let mut harness = showing(REF, matchup::vm());
    harness.run();
    let area = workspace(&harness);
    let card = column(&mut harness, area.min.x + 250.0 - 4.0 - 3.0);
    assert!(
        card.iter().all(|p| *p == [0x28, 0x28, 0x28]),
        "no bar where nothing overflows"
    );
}

/// Whether white (`TEXT_STRONG`) pixels lie on each side of `rect` (top, right, bottom, left), looking along a short
/// line across the middle of the side, from 2 inside it to 5 outside.
fn ring_sides(harness: &mut Harness<'_, BfApp>, rect: Rect) -> [bool; 4] {
    let (cx, cy) = (rect.center().x as i32, rect.center().y as i32);
    let (x0, x1, y0, y1) = (
        rect.min.x as i32,
        rect.max.x as i32,
        rect.min.y as i32,
        rect.max.y as i32,
    );
    let lines: [Vec<(i32, i32)>; 4] = [
        (y0 - 5..y0 + 2).map(|y| (cx, y)).collect(),
        (x1 - 2..x1 + 5).map(|x| (x, cy)).collect(),
        (y1 - 2..y1 + 5).map(|y| (cx, y)).collect(),
        (x0 - 5..x0 + 2).map(|x| (x, cy)).collect(),
    ];
    lines.map(|line| {
        let at: Vec<(u32, u32)> = line.iter().map(|&(x, y)| (x.max(0) as u32, y.max(0) as u32)).collect();
        pixels(harness, &at).contains(&[255, 255, 255])
    })
}

/// The controls a ring test checks: the chips at the roster's edges (the first and last of its first row and of its
/// last row) and every control of the strip.
fn edge_controls(harness: &Harness<'_, BfApp>) -> Vec<(String, Role)> {
    let chips: Vec<(String, Rect)> = sample::roster()
        .fighters
        .iter()
        .map(|f| {
            let name = chip_name(&f.display);
            let r = chip(harness, &name).rect();
            (name, r)
        })
        .collect();
    let top = chips.iter().map(|c| c.1.min.y).fold(f32::INFINITY, f32::min);
    let bottom = chips.iter().map(|c| c.1.min.y).fold(f32::NEG_INFINITY, f32::max);
    let first_row: Vec<_> = chips.iter().filter(|c| c.1.min.y == top).collect();
    let last_row: Vec<_> = chips.iter().filter(|c| c.1.min.y == bottom).collect();
    let mut out = vec![
        (first_row[0].0.clone(), Role::Button),
        (first_row[first_row.len() - 1].0.clone(), Role::Button),
        (last_row[0].0.clone(), Role::Button),
        (last_row[last_row.len() - 1].0.clone(), Role::Button),
    ];
    out.extend(strip_controls());
    out
}

/// The strip's controls in reading order.
fn strip_controls() -> [(String, Role); 5] {
    [
        ("Theme".to_string(), Role::ComboBox),
        ("Tempo".to_string(), Role::Slider),
        ("Tempo in bpm".to_string(), Role::SpinButton),
        ("Swap sides".to_string(), Role::Button),
        ("Balance this match-up".to_string(), Role::Button),
    ]
}

#[test]
fn every_control_shows_its_focus_ring() {
    let _gpu = gpu();
    for size in [REF, MIN] {
        let mut harness = showing(size, matchup::vm());
        harness.run();
        for (name, role) in edge_controls(&harness) {
            control(&harness, &name, role).scroll_to_me();
            harness.run();
            let rect = control(&harness, &name, role).rect();
            assert!(
                workspace(&harness).contains_rect(rect),
                "{size:?}: {name} is on screen at {rect:?}"
            );
            assert_eq!(
                ring_sides(&mut harness, rect),
                [false; 4],
                "{size:?}: {name} has no ring while another control has the focus"
            );
            control(&harness, &name, role).focus();
            harness.run();
            assert!(
                control(&harness, &name, role).is_focused(),
                "{size:?}: {name} takes the focus"
            );
            assert_eq!(
                ring_sides(&mut harness, rect),
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
    let mut order: Vec<(String, Role)> = sample::roster()
        .fighters
        .iter()
        .map(|f| (chip_name(&f.display), Role::Button))
        .collect();
    order.extend(strip_controls());
    for size in [REF, MIN] {
        let mut harness = showing(size, matchup::vm());
        harness.run();
        control(&harness, &order[0].0, order[0].1).focus();
        harness.run();
        for (i, (name, role)) in order.iter().enumerate() {
            if i > 0 {
                harness.key_press(Key::Tab);
                harness.run();
            }
            assert!(
                control(&harness, name, *role).is_focused(),
                "{size:?}: Tab number {i} reaches {name}"
            );
        }
    }
}
