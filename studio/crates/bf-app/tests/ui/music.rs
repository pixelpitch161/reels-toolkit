//! The Music screen (interface reference, section 5.4, with the placeholders of "What is live and what waits"): the
//! controls column, the arrangement lanes, the loudness lines and the footer.

use bf_app::app::BfApp;
use bf_app::engine::{self, EngineConfig, content, convert};
use bf_app::fixtures::music;
use bf_app::model::queue::Queue;
use bf_app::model::shortlist::{SearchState, Shortlist};
use bf_app::model::take::CheckLine;
use bf_app::model::{EncodeChoice, Settings, Step, View};
use bf_app::screens::music::{CONTROLS_W, NAMES_W};
use bf_app::seam::{Command, Event};
use bf_app::state::{Action, AppState, ViewModel};
use bf_app::store::Paths;
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
        "Parts",
        "Flute plays the melody.",
        "Trombone plays the bass.",
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
    // the FLUTE cannot play bass, so who plays it is not asked (Task 60), and the big top has no options
    assert!(!has(&harness, "Who plays bass"));
    assert!(!has(&harness, "Theme options"));
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
        (
            "Key",
            Role::ComboBox,
            "The key comes with the chord loop in this version.",
        ),
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

/// The Music fixture with the sample FLUTE's card able to play bass, so both fighters can (only then is "Who plays
/// bass" asked, Task 60).
fn both_play_bass() -> ViewModel {
    let mut vm = music::vm();
    let roster = vm.roster.as_mut().unwrap();
    let flute = roster.fighters.iter_mut().find(|f| f.name == "FLUTE").unwrap();
    flute.card.as_mut().unwrap().can_play_bass = true;
    vm
}

/// With a fighter that cannot play bass the parts are shown as the fight file writes them, read-only.
#[test]
fn the_flute_cannot_take_the_bass() {
    let _gpu = gpu();
    let mut harness = showing(REF, music::vm());
    assert!(!harness.query_all_by_role(Role::RadioButton).any(|n| {
        let label = n.accesskit_node().label().unwrap_or_default();
        label == "Flute" || label == "Trombone"
    }));
    assert!(has(&harness, "Flute plays the melody."));
    harness.get_by_label("Trombone plays the bass.").click();
    harness.run();
    assert!(harness.state().log.is_empty(), "{:?}", harness.state().log);
}

#[test]
fn a_fighter_that_can_take_the_bass_is_pushed() {
    let _gpu = gpu();
    let mut harness = showing(REF, both_play_bass());
    assert!(has(&harness, "Who plays bass"));
    assert!(has(&harness, "The other fighter takes the melody."));
    let trombone = by_role(&harness, "Trombone", Role::RadioButton);
    assert_eq!(
        trombone.accesskit_node().toggled(),
        Some(Toggled::True),
        "the chosen one"
    );
    trombone.click();
    harness.run();
    assert!(harness.state().log.is_empty(), "the chosen one is not asked for again");
    let flute = by_role(&harness, "Flute", Role::RadioButton);
    assert!(!flute.accesskit_node().is_disabled());
    flute.click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SetBass(0)]);
    // the parts follow: the FLUTE takes the bass, the TROMBONE the melody
    assert!(has(&harness, "Flute plays the bass."));
    assert!(has(&harness, "Trombone plays the melody."));
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

/// Ruling M3-R2: a theme whose music lacks a hit one of the pair's parts plays is not offered for that pair, on
/// the Music screen or on Match-up; the same theme is offered to a pair it does not refuse.
#[test]
fn a_theme_the_pair_cannot_play_is_not_offered() {
    let _gpu = gpu();
    let with_themes = |mut vm: ViewModel| {
        let roster = vm.roster.as_mut().unwrap();
        let mut refusing = roster.themes[0].clone();
        refusing.name = "heist".to_string();
        refusing.display = "Heist".to_string();
        refusing.refuses = vec!["TROMBONE".to_string(), "TUBA".to_string()];
        let mut open = roster.themes[0].clone();
        open.name = "mambo".to_string();
        open.display = "Mambo".to_string();
        roster.themes.extend([refusing, open]);
        vm
    };
    for (vm, step) in [
        (with_themes(music::vm()), "Music"),
        (with_themes(bf_app::fixtures::matchup::vm()), "Match-up"),
    ] {
        assert_eq!(vm.draft.fighters[1], "TROMBONE", "{step}");
        let mut harness = showing(REF, vm);
        by_role(&harness, "Theme", Role::ComboBox).click();
        harness.run();
        assert!(
            harness
                .query_all_by_label("Mambo")
                .any(|n| n.accesskit_node().role() == Role::ListBoxOption),
            "{step}: a theme the pair can play is offered"
        );
        assert!(
            !harness
                .query_all_by_label("Heist")
                .any(|n| n.accesskit_node().role() == Role::ListBoxOption),
            "{step}: a theme that refuses the TROMBONE is not offered"
        );
    }
    // without the TROMBONE (or the TUBA) in the pair, the same theme is offered
    let mut vm = with_themes(music::vm());
    vm.draft.fighters[1] = "OBOE".to_string();
    let mut harness = showing(REF, vm);
    by_role(&harness, "Theme", Role::ComboBox).click();
    harness.run();
    by_role(&harness, "Heist", Role::ListBoxOption).click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SetTheme("heist".to_string())]);
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
    let mut harness = showing(REF, both_play_bass());
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
const FOCUSABLE: [(&str, Role); 4] = [
    ("Theme", Role::ComboBox),
    (FIRST_LOOP, Role::Button),
    (SECOND_LOOP, Role::Button),
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

const MARKERS: [&str; 7] = [
    "Title card",
    "Fight",
    "Lead change",
    "Lead change",
    "Desperation",
    "K.O.",
    "Reset",
];

/// The text a node shows: a label's value, any other node's name.
fn text_of(node: &egui_kittest::Node<'_>) -> String {
    let node = node.accesskit_node();
    let text = if node.role() == Role::Label {
        node.value()
    } else {
        node.label()
    };
    text.unwrap_or_default()
}

/// The marker lane's markers on screen (role `Mark`), left to right: name and rect.
fn marker_nodes(harness: &Harness<'_, BfApp>) -> Vec<(String, Rect)> {
    let mut marks: Vec<(String, Rect)> = harness
        .query_all_by_role(Role::Mark)
        .map(|n| (text_of(&n), n.rect()))
        .collect();
    marks.sort_by(|a, b| a.1.min.x.total_cmp(&b.1.min.x));
    marks
}

#[test]
fn markers_show_whole_names_or_none() {
    let _gpu = gpu();
    for size in [REF, MIN] {
        let harness = showing(size, music::vm());
        let marks = marker_nodes(&harness);
        let names: Vec<&str> = marks.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, MARKERS, "every marker carries its whole name at {size:?}");
        let row = marks[0].1;
        let mut drawn = 0;
        for (i, (name, mark)) in marks.iter().enumerate() {
            let next = marks.get(i + 1).map_or(f32::INFINITY, |(_, r)| r.min.x);
            let words: Vec<Rect> = harness
                .query_all_by_role(Role::Label)
                .filter(|n| text_of(n) == *name)
                .map(|n| n.rect())
                .filter(|r| r.center().y >= row.min.y && r.center().y <= row.max.y && r.min.x >= mark.min.x)
                .filter(|r| r.min.x < next)
                .collect();
            assert!(words.len() <= 1, "{name}: {words:?}");
            if let Some(words) = words.first() {
                drawn += 1;
                let whole = harness.ctx.fonts_mut(|f| {
                    f.layout_no_wrap(name.clone(), bf_app::theme::small(), egui::Color32::WHITE)
                        .size()
                        .x
                });
                assert!(
                    words.width() >= whole - 0.5,
                    "{name} at {size:?} is whole: {words:?}, needs {whole}"
                );
                assert!(words.max.x <= next, "{name} stops before the next marker");
            }
        }
        assert!(drawn >= 4, "most names fit at {size:?}: {drawn}");
        let all_text: Vec<String> = harness.query_all_by_role(Role::Label).map(|n| text_of(&n)).collect();
        assert!(
            all_text.iter().all(|t| !t.contains('…')),
            "nothing is cut short in the marker lane"
        );
    }
}

#[test]
fn hovering_a_marker_without_its_words_shows_its_name() {
    let _gpu = gpu();
    let mut harness = showing(REF, music::vm());
    let words = |harness: &Harness<'_, BfApp>| {
        harness
            .query_all_by_role(Role::Label)
            .filter(|n| text_of(n) == "Title card")
            .count()
    };
    assert_eq!(words(&harness), 0, "Title card does not fit before Fight at 1440 x 900");
    let mark = marker_nodes(&harness)[0].1;
    harness.hover_at(mark.center());
    harness.run();
    assert_eq!(words(&harness), 1, "hovering shows its name");
}

#[test]
fn the_key_keeps_its_line_beside_it() {
    let _gpu = gpu();
    for size in [REF, MIN] {
        let harness = showing(size, music::vm());
        let key = by_role(&harness, "Key", Role::ComboBox).rect();
        let line = harness.get_by_label("harmonic minor, from the theme").rect();
        assert!(
            line.min.x >= key.max.x && (line.center().y - key.center().y).abs() <= 1.0,
            "{line:?}"
        );
        let workspace = harness.state().layout.workspace;
        assert!(
            line.max.x <= workspace.min.x + CONTROLS_W,
            "inside the column at {size:?}"
        );
    }
}

#[test]
fn no_text_says_nan() {
    let _gpu = gpu();
    let mut vm = music::vm();
    let m = vm.take.music.as_mut().unwrap();
    m.lanes[0].clips.push((f32::NAN, 20.0));
    m.lanes[1].clips.push((5.0, f32::INFINITY));
    m.markers.push((f32::NAN, "Lost".to_string()));
    m.markers.push((f32::NEG_INFINITY, "Gone".to_string()));
    let harness = showing(REF, vm);
    let mut read = 0;
    for node in harness.root().children_recursive() {
        let text = text_of(&node);
        read += usize::from(!text.is_empty());
        assert!(!text.contains("NaN") && !text.contains("inf"), "{text}");
    }
    assert!(read > 50, "the texts were read: {read}");
    assert!(
        !has(&harness, "Lost") && !has(&harness, "Gone"),
        "markers with no time are left out"
    );
}

#[test]
fn rings_in_the_scrolled_controls_column_at_the_minimum_size() {
    let _gpu = gpu();
    let mut harness = showing(MIN, music::vm());
    harness.get_by_label(TEMPO_LINE).scroll_to_me();
    harness.run();
    let workspace = harness.state().layout.workspace;
    let tempo = harness.get_by_label(TEMPO_LINE).rect();
    assert!(workspace.contains_rect(tempo), "scrolled to the bottom: {tempo:?}");
    for (label, role) in &FOCUSABLE[..3] {
        by_role(&harness, label, *role).focus();
        harness.run();
        assert!(by_role(&harness, label, *role).is_focused(), "{label}");
        let rect = by_role(&harness, label, *role).rect();
        assert!(
            workspace.contains_rect(rect.expand(4.0)),
            "{label} with its ring in view: {rect:?}"
        );
        assert_ring(&mut harness, rect, "ltrb", &format!("{label}, column scrolled"));
    }
}

#[test]
fn the_controls_column_shows_its_scroll_bar_when_it_overflows() {
    let _gpu = gpu();
    let mut harness = showing(MIN, music::vm());
    let workspace = harness.state().layout.workspace;
    // The bar: 4 from the column's right edge, 6 wide; its handle is FAINT without hovering.
    let x = (workspace.min.x + CONTROLS_W - 1.0 - 4.0 - 3.0) as u32;
    let column: Vec<(u32, u32)> = (workspace.min.y as u32 + 2..workspace.max.y as u32 - 2)
        .map(|y| (x, y))
        .collect();
    let px = pixels(&mut harness, &column);
    assert!(
        px.contains(&[0x77, 0x77, 0x77]),
        "the handle is drawn FAINT without hovering"
    );
    let theme = by_role(&harness, "Theme", Role::ComboBox).rect();
    assert!(
        theme.max.x < x as f32 - 4.0,
        "the controls keep clear of the bar: {theme:?}"
    );
}

// ---------------------------------------------------------------------------------------------------------------
// The whole kit (milestone 3, Task 60): real pairs, their themes, options and parts
// ---------------------------------------------------------------------------------------------------------------

/// The app's state with the engine's roster and `pair`'s kit fight file opened, described by the engine, on
/// `step`; with `take`, the real take of the file's own seed (its picture's story and its sound's lanes and checks).
pub fn kit_vm(pair: &str, step: Step, take: bool) -> ViewModel {
    let (_, ff) = bf_content::kit_pair_fights()
        .into_iter()
        .find(|(p, _)| *p == pair)
        .unwrap_or_else(|| panic!("{pair}"));
    let toml = ff.to_toml();
    let draft = convert::draft_of(ff, &toml);
    let settings = Settings {
        output_folder: std::path::PathBuf::from("Videos").join("BALLFIGHT"),
        workers: 7,
        encode: EncodeChoice::Best,
    };
    let (mut state, _) = AppState::new(settings, Queue::new(), Vec::new(), 8);
    state.apply(Event::Roster(content::roster()));
    // the draft and the engine's description of it laid in as the state would hold them (applying them would begin
    // a new generation, which clears the seed)
    let mut vm = state.vm;
    vm.draft = draft.clone();
    let Event::FightInfo {
        pair_rules,
        problem,
        music,
        ..
    } = content::describe(1, &draft)
    else {
        panic!("{pair}: not a description");
    };
    assert_eq!(problem, None, "{pair}");
    vm.pair_rules = pair_rules;
    vm.pair_music = music;
    vm.view = View::Step(step);
    vm.step = step;
    if take {
        let root = std::env::temp_dir().join(format!("bf-app kit take {pair} {}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let config = EngineConfig {
            workers: 4,
            paths: Paths::under(&root),
            clip: None,
            search_limit: None,
        };
        let engine = engine::spawn(std::sync::Arc::new(|| {}), config);
        engine.send(Command::LoadTake { take: 1, draft });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
        while vm.take.music.is_none() {
            assert!(std::time::Instant::now() < deadline, "{pair}: no take in 180 s");
            while let Some(e) = engine.poll() {
                match e {
                    Event::TakePicture { story, .. } => vm.take.story = Some(story),
                    Event::TakeSound { music, .. } => vm.take.music = Some(music),
                    Event::TakeFailed { problem, .. } => panic!("{pair}: {problem}"),
                    _ => {}
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        engine.send(Command::Shutdown);
        drop(engine);
        let _ = std::fs::remove_dir_all(&root);
    }
    // the status line as the state words it for such a draft
    vm.status = match (take, vm.draft.seed) {
        (true, Some(seed)) => format!("Seed {seed} is ready"),
        _ => format!("{}, not balanced yet", bf_app::wording::versus(&vm.draft.fighters)),
    };
    vm
}

/// The Music screen of a kit pair: its theme's chord loops, its theme options read-only and its parts as the kit
/// writes them; "Who plays bass" only where both fighters' cards can play bass.
#[test]
fn kit_pairs_show_their_options_and_parts() {
    let _gpu = gpu();
    for (pair, options, parts, bass) in [
        (
            "electric_vs_acoustic",
            &["Plugged: Electric"][..],
            ["Electric plays the riff.", "Acoustic plays the rhythm."],
            false,
        ),
        (
            "cello_vs_contrabass",
            &[][..],
            ["Cello plays the melody.", "Contrabass plays the bass."],
            false,
        ),
        (
            "bassdrum_vs_tuba",
            &["No bass drums", "Bass drums back after the K.O."][..],
            ["Bass drum plays the drums.", "Tuba plays the bass."],
            false,
        ),
        (
            "horn_vs_trombone_rematch",
            &["The sequel"][..],
            ["Horn plays the melody.", "Trombone plays the riff."],
            false,
        ),
        (
            "trombone_vs_tuba",
            &[][..],
            ["Trombone plays the riff.", "Tuba plays the bass."],
            true,
        ),
    ] {
        let vm = kit_vm(pair, Step::Music, false);
        let theme = vm.theme().expect("an offered theme").clone();
        let (key, scale) = vm.key_scale().expect("a key");
        let ff = convert::fight_file(&vm.draft).expect("a fight file");
        assert_eq!(
            (key.as_str(), scale.as_str()),
            (ff.music.key.as_str(), ff.music.scale.as_str()),
            "{pair}"
        );
        let harness = showing(REF, vm);
        assert_eq!(
            by_role(&harness, "Key", Role::ComboBox).value().as_deref(),
            Some(key.as_str()),
            "{pair}"
        );
        assert!(has(&harness, &format!("{scale}, from the theme")), "{pair}");
        for words in options.iter().chain(&parts) {
            assert!(has(&harness, words), "{pair}: {words}");
        }
        assert_eq!(has(&harness, "Theme options"), !options.is_empty(), "{pair}");
        assert_eq!(has(&harness, "Who plays bass"), bass, "{pair}");
        for chords in &theme.chord_loops {
            let label = chords.join("  ");
            assert!(
                harness
                    .query_all_by_label(&label)
                    .any(|n| n.accesskit_node().role() == Role::Button),
                "{pair}: {label}"
            );
        }
        assert_eq!(
            by_role(&harness, "Theme", Role::ComboBox).value().as_deref(),
            Some(theme.display.as_str()),
            "{pair}"
        );
        assert_eq!(harness.unnamed_widgets(), Vec::<String>::new(), "{pair}");
    }
}

/// Where both can play bass the parts swap: the TUBA's bass goes to the TROMBONE, which then plays it, and the
/// TUBA takes the TROMBONE's riff.
#[test]
fn the_bass_moves_between_two_who_can_play_it() {
    let _gpu = gpu();
    let mut harness = showing(REF, kit_vm("trombone_vs_tuba", Step::Music, false));
    by_role(&harness, "Trombone", Role::RadioButton).click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SetBass(0)]);
    assert!(has(&harness, "Trombone plays the bass."));
    // the TUBA has no riff of its own: it plays the TROMBONE's part as its top line, and is said to
    assert!(has(&harness, "Tuba plays the top line."));
    assert!(has(&harness, "The other fighter takes the top line."));
    assert!(!has(&harness, "Tuba plays the riff."));
}

/// Note (c) of Task 60: a fit whose soundtrack fails a check (the search does not listen to the music) says that
/// its video would stop there and offers the next fight that fits.
#[test]
fn a_take_that_fails_a_check_offers_the_next_fit() {
    let _gpu = gpu();
    let mut vm = music::vm();
    vm.shortlist = Shortlist {
        rows: bf_app::seam::sample::stories(),
        fits_total: 14,
        checked: 18_240,
        state: SearchState::Searching,
        workers: 7,
    };
    let seed = vm.draft.seed.expect("a seed");
    let at = vm
        .shortlist
        .rows
        .iter()
        .position(|r| r.seed == seed)
        .expect("the seed is on the list");
    let next = vm.shortlist.rows[(at + 1) % vm.shortlist.rows.len()].seed;
    let failing = |mut vm: ViewModel| {
        vm.take.music.as_mut().unwrap().checks[0] = CheckLine {
            title: "Loudness".to_string(),
            pass: false,
            note: FAILING.to_string(),
        };
        vm
    };
    let words = format!(
        "Seed {seed} fails the check \"Loudness\", so its video would stop there and nothing would be saved. Try seed \
         {next}, the next fight that fits."
    );
    let passing = showing(REF, vm.clone());
    assert!(!has(&passing, &words) && !has(&passing, &format!("Use seed {next}")));
    drop(passing);
    let mut harness = showing(REF, failing(vm.clone()));
    assert!(has(&harness, &words), "{words}");
    harness.get_by_label(&format!("Use seed {next}")).click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SelectSeed(next)]);
    // with no other fight that fits, it sends the owner to Seeds
    let mut alone = failing(vm);
    alone.shortlist.rows.retain(|r| r.seed == seed);
    let mut harness = showing(REF, alone);
    assert!(has(
        &harness,
        &format!(
            "Seed {seed} fails the check \"Loudness\", so its video would stop there and nothing would be saved. Try another seed."
        )
    ));
    harness.get_by_label("Go to seeds").click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::Go(View::Step(Step::Seeds))]);
}

#[test]
fn music_matches_for_cello_vs_contrabass() {
    let _gpu = gpu();
    let mut harness = showing(REF, kit_vm("cello_vs_contrabass", Step::Music, true));
    harness.snapshot("music_cello_vs_contrabass_1440x900");
}

#[test]
fn music_matches_for_electric_vs_acoustic() {
    let _gpu = gpu();
    let mut harness = showing(REF, kit_vm("electric_vs_acoustic", Step::Music, true));
    harness.snapshot("music_electric_vs_acoustic_1440x900");
}

/// The Music screen with both fighters able to play bass and a take that fails a check: Tab reaches the theme, the
/// loops, both bass segments, the next fit's button and "Go to render" in that order, each with its ring; with no
/// other fit, "Go to seeds" takes the next fit's place.
#[test]
fn tab_walks_the_bass_and_the_next_fit() {
    let _gpu = gpu();
    let mut vm = both_play_bass();
    vm.shortlist = Shortlist {
        rows: bf_app::seam::sample::stories(),
        fits_total: 14,
        checked: 18_240,
        state: SearchState::Searching,
        workers: 7,
    };
    vm.take.music.as_mut().unwrap().checks[0].pass = false;
    let next = vm.next_fit().expect("a next fit");
    let use_next = format!("Use seed {next}");
    let mut alone = vm.clone();
    alone.shortlist.rows.retain(|r| Some(r.seed) == alone.draft.seed);
    for (vm, offer) in [(vm, use_next.as_str()), (alone, "Go to seeds")] {
        let order = [
            ("Theme", Role::ComboBox),
            (FIRST_LOOP, Role::Button),
            (SECOND_LOOP, Role::Button),
            ("Flute", Role::RadioButton),
            ("Trombone", Role::RadioButton),
            (offer, Role::Button),
            ("Go to render", Role::Button),
        ];
        let mut harness = showing(REF, vm);
        by_role(&harness, order[0].0, order[0].1).focus();
        harness.run();
        for pair in order.windows(2) {
            let (from, to) = (pair[0], pair[1]);
            assert!(
                by_role(&harness, from.0, from.1).is_focused(),
                "{} has the keyboard",
                from.0
            );
            harness.key_press(egui::Key::Tab);
            harness.run();
            let node = by_role(&harness, to.0, to.1);
            assert!(node.is_focused(), "Tab from {} goes to {}", from.0, to.0);
            node.scroll_to_me();
            harness.run();
            let rect = by_role(&harness, to.0, to.1).rect();
            assert_ring(&mut harness, rect, "ltrb", to.0);
        }
        assert!(harness.state().log.is_empty(), "walking through changes nothing");
    }
}
