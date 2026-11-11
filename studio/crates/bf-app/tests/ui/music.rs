//! The Music screen (interface reference, section 5.4, with the placeholders of "What is live and what waits"): the
//! controls column, the arrangement lanes, the loudness lines and the footer.

use bf_app::app::BfApp;
use bf_app::fixtures::music;
use bf_app::model::take::CheckLine;
use bf_app::model::{Step, View};
use bf_app::screens::music::{CONTROLS_W, NAMES_W};
use bf_app::state::{Action, ViewModel};
use bf_app::theme::{LINE_SOFT, ON_LIGHT, PANEL, TEXT, large_fill};
use bf_app::widgets::ribbon::x_of;
use egui::Rect;
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common::{MIN, REF, app_harness, gpu, pixels};

const FIRST_LOOP: &str = "Em  Am  B7  Em  C  Am  B7  B7";
const SECOND_LOOP: &str = "Em  C  Am  B7  Em  C  Am  B7";
const TAKES_LINE: &str = "New takes arrive with the music system. This version plays the series' own arrangement.";
const TEMPO_LINE: &str = "Tempo is set on Match-up, because it changes the fight.";
const NO_SEED: &str = "Pick a seed first. The music is built for the fight you choose.";
const FAILING: &str = "The opening is 2.1 dB quieter than the ending, and the limit is 1.5 dB.";
const FLUTE: [u8; 3] = [0xde, 0xe8, 0xf6];

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
fn by_role<'t, 'h>(harness: &'t Harness<'h, BfApp>, label: &'t str, role: Role) -> egui_kittest::Node<'t> {
    harness
        .query_all_by_label(label)
        .find(|n| n.accesskit_node().role() == role)
        .unwrap_or_else(|| panic!("a {role:?} named {label}"))
}

/// The rect of the lane name `name` in the arrangement's name column.
fn lane_name(harness: &Harness<'_, BfApp>, name: &str) -> Rect {
    let workspace = harness.state().layout.workspace;
    let column = workspace.min.x + CONTROLS_W..=workspace.min.x + CONTROLS_W + NAMES_W;
    harness
        .query_all_by_label(name)
        .map(|n| n.rect())
        .find(|r| column.contains(&r.min.x) && workspace.contains_rect(*r))
        .unwrap_or_else(|| panic!("the lane name {name}"))
}

#[test]
fn the_words_that_stay() {
    let _gpu = gpu();
    let harness = showing(REF, music::vm());
    for text in [
        "Theme",
        "Key",
        "harmonic minor, from the theme",
        "Chord loop",
        "Who plays bass",
        "The other fighter takes the melody.",
        "This take",
        "Music seed",
        "Re-roll music",
        "Flute, the charmer",
        "High and ornamented. Runs of eighths that fall back to the home note.",
        "Trombone, the slide",
        "Smears up into notes, oom-pah struts, low fanfares.",
        TEMPO_LINE,
        "Arrangement",
        "How this take follows the fight you picked",
        "Fighters",
        "Band",
        "Sounds right?",
        "Go to render",
        "Loudness",
        "Checked as soon as the take is built, so a problem shows here and not in the queue.",
        "The opening is 0.6 dB quieter than the ending, and the limit is 1.5 dB.",
        "Perceived loudness",
        "The title card is 0.4 LU under the fight.",
        "Fighters above the music",
        "Both fighters sit above the band in their own range.",
        "Music that follows the fight, with fills on lead changes and fresh lines for every take, arrives with the \
         music system.",
        FIRST_LOOP,
        SECOND_LOOP,
    ] {
        assert!(has(&harness, text), "{text}");
    }
    for lane in [
        "Flute",
        "Trombone",
        "Drums",
        "Calliope",
        "Organ",
        "Glockenspiel",
        "Band hits",
    ] {
        lane_name(&harness, lane);
    }
    for marker in ["Title card", "Fight", "Lead change", "Desperation", "K.O.", "Reset"] {
        assert!(has(&harness, marker), "the marker {marker}");
    }
    assert_eq!(
        by_role(&harness, "Theme", Role::ComboBox).value().as_deref(),
        Some("Big top")
    );
    assert_eq!(by_role(&harness, "Key", Role::ComboBox).value().as_deref(), Some("E"));
    assert_eq!(
        by_role(&harness, "Music seed", Role::TextInput).value().as_deref(),
        Some("0417")
    );
}

#[test]
fn the_controls_column_is_300_wide() {
    let _gpu = gpu();
    let mut harness = showing(REF, music::vm());
    let workspace = harness.state().layout.workspace;
    let theme = by_role(&harness, "Theme", Role::ComboBox).rect();
    assert_eq!(theme.min.x, workspace.min.x + 12.0, "padded 12 at the side");
    assert_eq!(theme.max.x, workspace.min.x + 288.0, "a full-width dropdown");
    let label = harness
        .query_all_by_label("Theme")
        .find(|n| n.accesskit_node().role() == Role::Label && workspace.contains_rect(n.rect()))
        .expect("the group's label")
        .rect();
    assert_eq!(label.min.y, workspace.min.y + 14.0, "and 14 above");
    let key = by_role(&harness, "Key", Role::ComboBox).rect();
    assert_eq!(key.width(), 70.0);
    let seed = by_role(&harness, "Music seed", Role::TextInput).rect();
    assert_eq!((seed.width(), seed.height()), (62.0, 22.0));
    let first = harness.get_by_label(FIRST_LOOP).rect();
    assert_eq!((first.min.x, first.width(), first.height()), (12.0, 276.0, 28.0));
    let y = 400;
    let rgb = |c: egui::Color32| [c.r(), c.g(), c.b()];
    let px = pixels(&mut harness, &[(298, y), (299, y), (300, y)]);
    assert_eq!(
        px,
        vec![rgb(PANEL), rgb(LINE_SOFT), rgb(PANEL)],
        "a 1 px line on its right"
    );
    let heading = harness.get_by_label("Arrangement").rect();
    assert_eq!(
        heading.min.x,
        CONTROLS_W + 14.0,
        "the arrangement's header is padded 14"
    );
    assert!(heading.max.y <= workspace.min.y + 40.0);
}

#[test]
fn placeholders_are_disabled_and_say_why() {
    let _gpu = gpu();
    let mut harness = showing(REF, music::vm());
    let key = by_role(&harness, "Key", Role::ComboBox);
    assert!(key.accesskit_node().is_disabled());
    let seed = by_role(&harness, "Music seed", Role::TextInput);
    assert!(seed.accesskit_node().is_disabled());
    let reroll = harness.get_by_label("Re-roll music");
    assert!(reroll.accesskit_node().is_disabled());
    assert!(has(&harness, TAKES_LINE), "the stated line");
    assert!(!has(&harness, "What the music listens for"));
    assert!(!has(
        &harness,
        "A new take writes fresh lines and solos. The fight doesn't change."
    ));

    for (label, role, why) in [
        ("Key", Role::ComboBox, "The key follows the theme in this version."),
        ("Music seed", Role::TextInput, "New takes arrive with the music system."),
        ("Re-roll music", Role::Button, "New takes arrive with the music system."),
    ] {
        assert!(!has(&harness, why), "{label}: not before hovering");
        by_role(&harness, label, role).hover();
        harness.run();
        assert!(has(&harness, why), "{label}: hovering says why");
        harness.hover_at(egui::pos2(700.0, 500.0));
        harness.run();
    }
    by_role(&harness, "Key", Role::ComboBox).click();
    harness.run();
    harness.get_by_label("Re-roll music").click();
    harness.run();
    assert!(harness.state().log.is_empty(), "{:?}", harness.state().log);
}

#[test]
fn the_flute_cannot_take_the_bass() {
    let _gpu = gpu();
    let mut harness = showing(REF, music::vm());
    let flute = by_role(&harness, "Flute", Role::RadioButton);
    assert!(flute.accesskit_node().is_disabled());
    flute.click();
    harness.run();
    assert!(harness.state().log.is_empty(), "{:?}", harness.state().log);
    let trombone = by_role(&harness, "Trombone", Role::RadioButton);
    assert_eq!(
        trombone.accesskit_node().toggled(),
        Some(Toggled::True),
        "the chosen one"
    );
    assert!(!trombone.accesskit_node().is_disabled());
    by_role(&harness, "Flute", Role::RadioButton).hover();
    harness.run();
    assert!(has(&harness, "Flute has no bass part in this version"));
    by_role(&harness, "Trombone", Role::RadioButton).click();
    harness.run();
    assert!(harness.state().log.is_empty(), "the chosen one is not asked for again");
}

#[test]
fn a_fighter_that_can_take_the_bass_is_pushed() {
    let _gpu = gpu();
    let mut vm = music::vm();
    vm.draft.bass = None;
    let mut harness = showing(REF, vm);
    by_role(&harness, "Trombone", Role::RadioButton).click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SetBass(1)]);
}

#[test]
fn choosing_a_chord_loop_pushes() {
    let _gpu = gpu();
    let mut harness = showing(REF, music::vm());
    let first = harness.get_by_label(FIRST_LOOP);
    assert_eq!(first.accesskit_node().toggled(), Some(Toggled::True), "the chosen loop");
    first.click();
    harness.run();
    assert!(harness.state().log.is_empty(), "the chosen loop is not asked for again");
    harness.get_by_label(SECOND_LOOP).click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SetChordLoop(1)]);
    assert_eq!(
        harness.get_by_label(SECOND_LOOP).accesskit_node().toggled(),
        Some(Toggled::True),
        "the state took it"
    );
}

#[test]
fn choosing_a_theme_pushes() {
    let _gpu = gpu();
    let mut vm = music::vm();
    let roster = vm.roster.as_mut().unwrap();
    let mut other = roster.themes[0].clone();
    other.name = "dark_disco".to_string();
    other.display = "Dark disco".to_string();
    roster.themes.push(other);
    let mut harness = showing(REF, vm);
    by_role(&harness, "Theme", Role::ComboBox).click();
    harness.run();
    by_role(&harness, "Dark disco", Role::ListBoxOption).click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SetTheme("dark_disco".to_string())]);
}

#[test]
fn loudness_lines_show_pass_and_fail() {
    let _gpu = gpu();
    let mut vm = music::vm();
    let checks = &mut vm.take.music.as_mut().unwrap().checks;
    checks[0] = CheckLine {
        title: "Loudness".to_string(),
        pass: false,
        note: FAILING.to_string(),
    };
    let harness = showing(REF, vm);
    assert!(has(&harness, FAILING), "the failing check's note");
    assert!(
        has(&harness, "The title card is 0.4 LU under the fight."),
        "the passing ones stay"
    );
    assert!(!has(
        &harness,
        "The opening is 0.6 dB quieter than the ending, and the limit is 1.5 dB."
    ));
    let note = harness.get_by_label(FAILING).rect();
    let workspace = harness.state().layout.workspace;
    assert!(workspace.contains_rect(note), "{note:?}");
}

#[test]
fn clips_sit_at_their_times() {
    let _gpu = gpu();
    let mut vm = music::vm();
    vm.take.music.as_mut().unwrap().lanes[0].clips = vec![(3.6, 20.0)];
    let mut harness = showing(REF, vm);
    let workspace = harness.state().layout.workspace;
    let left = workspace.min.x + CONTROLS_W + NAMES_W;
    let width = workspace.max.x - left;
    let y = lane_name(&harness, "Flute").center().y as u32;
    let row: Vec<(u32, u32)> = (left as u32..workspace.max.x as u32).map(|x| (x, y)).collect();
    let px = pixels(&mut harness, &row);
    let fill = large_fill(FLUTE);
    let fill = [fill.r(), fill.g(), fill.b()];
    let start = row[px.iter().position(|p| *p == fill).expect("the clip's fill")].0 as f32;
    let end = row[px.iter().rposition(|p| *p == fill).unwrap()].0 as f32;
    let want = x_of(3.6, left, width);
    assert!((start - want).abs() <= 1.0, "starts at {start}, wants {want}");
    let want = x_of(20.0, left, width);
    assert!((end + 1.0 - want).abs() <= 1.0, "ends at {end}, wants {want}");
}

#[test]
fn while_the_sound_is_built_the_lanes_wait() {
    let _gpu = gpu();
    let mut vm = music::vm();
    vm.take.music = None;
    let harness = showing(REF, vm);
    assert!(has(&harness, "Building the sound for seed 18433"));
    assert!(!has(&harness, "Calliope"), "no lanes yet");
    assert!(has(&harness, "Go to render"));
}

#[test]
fn without_a_seed_it_says_what_to_do() {
    let _gpu = gpu();
    let mut harness = showing(REF, music::no_seed());
    assert!(has(&harness, NO_SEED));
    assert!(!has(&harness, "Calliope"), "no lanes");
    assert!(!has(&harness, "Go to render"), "nothing to take to render yet");
    let text = harness.get_by_label(NO_SEED).rect();
    let workspace = harness.state().layout.workspace;
    assert!(text.min.x >= CONTROLS_W && text.max.x <= workspace.max.x, "{text:?}");
    harness.get_by_label("Go to seeds").click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::Go(View::Step(Step::Seeds))]);
}

#[test]
fn go_to_render_pushes() {
    let _gpu = gpu();
    let mut harness = showing(REF, music::vm());
    harness.get_by_label("Go to render").click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::Go(View::Step(Step::Render))]);
    assert_eq!(harness.state().state.vm.view, View::Step(Step::Render));
}

#[test]
fn inverted_controls_have_dark_labels() {
    let _gpu = gpu();
    let mut harness = showing(REF, music::vm());
    for (label, role) in [("Trombone", Role::RadioButton), ("Go to render", Role::Button)] {
        let rect = by_role(&harness, label, role).rect();
        let inside: Vec<(u32, u32)> = (rect.min.x as u32 + 2..rect.max.x as u32 - 2)
            .flat_map(|x| (rect.min.y as u32 + 2..rect.max.y as u32 - 2).map(move |y| (x, y)))
            .collect();
        let px = pixels(&mut harness, &inside);
        let fill = [TEXT.r(), TEXT.g(), TEXT.b()];
        let glyph = [ON_LIGHT.r(), ON_LIGHT.g(), ON_LIGHT.b()];
        assert!(px.contains(&fill), "{label}: the inverted fill");
        assert!(px.contains(&glyph), "{label}: ON_LIGHT glyphs");
    }
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

/// The controls Tab reaches on the screen with a take, in reading order, with their roles.
const FOCUSABLE: [(&str, Role); 5] = [
    ("Theme", Role::ComboBox),
    (FIRST_LOOP, Role::Button),
    (SECOND_LOOP, Role::Button),
    ("Trombone", Role::RadioButton),
    ("Go to render", Role::Button),
];

#[test]
fn every_control_shows_its_focus_ring() {
    let _gpu = gpu();
    for size in [REF, MIN] {
        let mut harness = showing(size, music::vm());
        for (label, role) in FOCUSABLE {
            let rect = by_role(&harness, label, role).rect();
            assert_ne!(
                pixel(&mut harness, egui::pos2(rect.min.x - 3.0, rect.center().y)),
                [0xff; 3]
            );
            by_role(&harness, label, role).focus();
            harness.run();
            assert!(by_role(&harness, label, role).is_focused(), "{label} has the keyboard");
            let rect = by_role(&harness, label, role).rect();
            // A segment's ring is drawn round the segment; the left one is Flute's, which cannot be chosen.
            assert_ring(&mut harness, rect, "ltrb", &format!("{label} at {size:?}"));
        }
        drop(harness);
        let mut harness = showing(size, music::no_seed());
        harness.get_by_label("Go to seeds").focus();
        harness.run();
        let rect = harness.get_by_label("Go to seeds").rect();
        assert_ring(&mut harness, rect, "ltrb", &format!("Go to seeds at {size:?}"));
    }
}

#[test]
fn tab_walks_the_screen_in_order() {
    let _gpu = gpu();
    let mut harness = showing(REF, music::vm());
    by_role(&harness, FOCUSABLE[0].0, FOCUSABLE[0].1).focus();
    harness.run();
    for pair in FOCUSABLE.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        assert!(
            by_role(&harness, from.0, from.1).is_focused(),
            "{} has the keyboard",
            from.0
        );
        harness.key_press(egui::Key::Tab);
        harness.run();
        assert!(
            by_role(&harness, to.0, to.1).is_focused(),
            "Tab from {} goes to {}",
            from.0,
            to.0
        );
    }
    assert!(harness.state().log.is_empty(), "walking through changes nothing");
}

#[test]
fn at_the_minimum_size() {
    let _gpu = gpu();
    let mut harness = showing(MIN, music::vm());
    let workspace = harness.state().layout.workspace;
    let go = harness.get_by_label("Go to render").rect();
    assert!(
        workspace.contains_rect(go),
        "Go to render at {go:?} lies inside {workspace:?}"
    );
    for label in ["Re-roll music", TEMPO_LINE] {
        harness.get_by_label(label).scroll_to_me();
        harness.run();
        let r = harness.get_by_label(label).rect();
        assert!(
            workspace.contains_rect(r),
            "{label} at {r:?} can be scrolled into {workspace:?}"
        );
    }
    let names = harness.get_by_label("Glockenspiel").rect();
    assert!(names.max.x <= CONTROLS_W + NAMES_W, "lane names stay in their column");
    let heading = harness
        .get_by_label("How this take follows the fight you picked")
        .rect();
    assert!(heading.max.x <= workspace.max.x);
}

#[test]
fn nothing_is_unnamed() {
    let _gpu = gpu();
    let mut building = music::vm();
    building.take.music = None;
    for vm in [music::vm(), music::no_seed(), building] {
        let harness = showing(REF, vm);
        assert_eq!(harness.unnamed_widgets(), Vec::<String>::new());
    }
}

#[test]
fn music_matches() {
    let _gpu = gpu();
    let mut harness = showing(REF, music::vm());
    harness.snapshot("music_1440x900");
}
