//! The Library view (spec section 4, interface reference sections 9 and 11): a grid of covers with the match-up and
//! the date, a footer for the chosen video, and what to do when there are none or the folder is missing.

use std::path::PathBuf;

use bf_app::app::BfApp;
use bf_app::fixtures::{library, settings};
use bf_app::screens::library::{COVER, EMPTY, MISSING_ADVICE, TILE_H, TILE_W};
use bf_app::seam::Command;
use bf_app::state::{Action, ViewModel};
use bf_app::store::library::local_date;
use egui::accesskit::{Role, Toggled};
use egui::{Key, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common::{MIN, REF, app_harness, gpu, pixels};

const NEWER: &str = "Made by a newer version of the app.";

/// The shell showing `vm` as it is.
fn showing<'a>(size: (f32, f32), vm: ViewModel) -> Harness<'a, BfApp> {
    let mut harness = app_harness(size, move |cc| BfApp::showing(cc, vm));
    harness.run();
    harness
}

fn has(harness: &Harness<'_, BfApp>, text: &str) -> bool {
    harness.query_all_by_label(text).next().is_some()
}

/// The six sample videos' tile names, newest first: "Flute vs Trombone, Nov 10, 2026", ...
fn tile_names() -> Vec<String> {
    library::vm()
        .library
        .entries
        .iter()
        .map(|e| format!("{}, {}", e.title, local_date(e.modified)))
        .collect()
}

/// The folder of the sample video at `i`.
fn folder(i: usize) -> PathBuf {
    library::vm().library.entries[i].folder.clone()
}

/// The tiles on screen in reading order (top to bottom, then left to right): their names and rects.
fn tiles(harness: &Harness<'_, BfApp>) -> Vec<(String, Rect)> {
    let mut tiles: Vec<(String, Rect)> = harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| {
            let label = n.accesskit_node().label()?;
            label.contains(" vs ").then(|| (label, n.rect()))
        })
        .collect();
    tiles.sort_by(|a, b| a.1.min.y.total_cmp(&b.1.min.y).then(a.1.min.x.total_cmp(&b.1.min.x)));
    tiles
}

/// The control with `label` and `role`.
fn control<'t>(harness: &'t Harness<'_, BfApp>, label: &'t str, role: Role) -> egui_kittest::Node<'t> {
    harness
        .query_all_by_label(label)
        .find(|n| n.accesskit_node().role() == role)
        .unwrap_or_else(|| panic!("a {role:?} named {label}"))
}

/// The rects of the nodes named `text` inside the footer (the bottom 46 of the workspace).
fn in_footer(harness: &Harness<'_, BfApp>, text: &str) -> Vec<Rect> {
    let workspace = harness.state().layout.workspace;
    harness
        .query_all_by_label(text)
        .map(|n| n.rect())
        .filter(|r| r.min.y >= workspace.max.y - 46.0 && r.max.y <= workspace.max.y)
        .collect()
}

#[test]
fn six_tiles_newest_first() {
    let _gpu = gpu();
    let harness = showing(REF, library::vm());
    let tiles = tiles(&harness);
    let names: Vec<String> = tiles.iter().map(|t| t.0.clone()).collect();
    assert_eq!(names, tile_names());
    assert_eq!(
        names[0],
        format!(
            "Flute vs Trombone, {}",
            local_date(library::vm().library.entries[0].modified)
        )
    );
    assert!(
        names[0].ends_with("Nov 10, 2026"),
        "12:00 UTC reads the same day: {}",
        names[0]
    );
    assert!(names[5].starts_with("Trumpet vs Horn, Nov 5"), "{}", names[5]);
    let workspace = harness.state().layout.workspace;
    let first = tiles[0].1;
    assert_eq!(
        (first.min.x, first.min.y),
        (workspace.min.x + 16.0, workspace.min.y + 40.0 + 16.0),
        "under the 40 tall header, padded 16"
    );
    for (i, (name, rect)) in tiles.iter().enumerate() {
        assert_eq!((rect.width(), rect.height()), (TILE_W, TILE_H), "{name}");
        if i > 0 && rect.min.y == tiles[i - 1].1.min.y {
            assert_eq!(rect.min.x, tiles[i - 1].1.max.x + 16.0, "16 apart: {name}");
        }
    }
    // Four fit across the reference workspace (1030 less the padding and the scroll bar), so two are on row two.
    let per_row = tiles.iter().filter(|t| t.1.min.y == first.min.y).count();
    assert_eq!(per_row, 4);
    assert_eq!(tiles[4].1.min.y, first.max.y + 16.0, "rows are 16 apart");
    assert!(has(&harness, "Library"));
    let count = format!("6 videos in {}", library::vm().settings.output_folder.display());
    assert!(has(&harness, &count), "{count}");
    assert!(has(&harness, "Look again"));
}

#[test]
fn a_tile_is_named_by_title_and_date() {
    let _gpu = gpu();
    let mut harness = showing(REF, library::vm());
    let names = tile_names();
    let tile = control(&harness, &names[1], Role::Button);
    assert_eq!(tile.accesskit_node().toggled(), Some(Toggled::True), "the selected one");
    let tile = tile.rect();
    let other = control(&harness, &names[0], Role::Button);
    assert_eq!(other.accesskit_node().toggled(), Some(Toggled::False));
    let other = other.rect();
    // The tile holds its cover, title and date.
    let cover = control(&harness, "Cover of Flute vs Trombone", Role::Image).rect();
    assert_eq!((cover.width(), cover.height()), COVER);
    assert!(other.contains_rect(cover));
    assert_eq!(cover.center().x, other.center().x, "the cover is centred in its tile");
    let title = harness
        .query_all_by_label("Flute vs Trombone")
        .map(|n| n.rect())
        .find(|r| other.contains_rect(*r))
        .expect("the title in its tile");
    assert!(title.min.y >= cover.max.y, "under the cover");
    let date = harness
        .query_all_by_label(&local_date(library::vm().library.entries[0].modified))
        .map(|n| n.rect())
        .find(|r| other.contains_rect(*r))
        .expect("the date in its tile");
    assert!(date.min.y >= title.max.y, "under the title");
    // The cover is the two fighters' colours on a diagonal: Flute top left, Trombone bottom right.
    let px = pixels(
        &mut harness,
        &[
            (cover.min.x as u32 + 10, cover.min.y as u32 + 10),
            (cover.max.x as u32 - 10, cover.max.y as u32 - 10),
        ],
    );
    assert_eq!(px, vec![[0xde, 0xe8, 0xf6], [0x78, 0xf0, 0xb8]]);
    // The selected tile has the selected fill.
    let at = (tile.min.x as u32 + 4, tile.max.y as u32 - 4);
    assert_eq!(pixels(&mut harness, &[at]), vec![[0x38, 0x38, 0x38]]);
}

#[test]
fn a_date_the_calendar_cannot_show_reads_unknown() {
    let _gpu = gpu();
    let mut vm = library::vm();
    vm.library.entries[0].modified = u64::MAX;
    let harness = showing(REF, vm);
    let tile = control(&harness, "Flute vs Trombone, Date unknown", Role::Button).rect();
    let date = harness.get_by_label("Date unknown").rect();
    assert!(tile.contains_rect(date));
}

#[test]
fn a_cover_not_here_yet_is_a_raised_block() {
    let _gpu = gpu();
    let mut vm = library::vm();
    vm.library.covers.clear();
    let mut harness = showing(REF, vm);
    let cover = control(&harness, "Cover of Flute vs Trombone", Role::Image).rect();
    let px = pixels(&mut harness, &[(cover.center().x as u32, cover.center().y as u32)]);
    assert_eq!(px, vec![[0x33, 0x33, 0x33]], "RAISED until the cover arrives");
}

#[test]
fn clicking_selects_and_the_footer_follows() {
    let _gpu = gpu();
    let mut harness = showing(REF, library::vm());
    assert_eq!(in_footer(&harness, "Tuba vs Bass drum").len(), 1);
    assert_eq!(in_footer(&harness, "seed 9120").len(), 1);
    let names = tile_names();
    control(&harness, &names[2], Role::Button).click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SelectVideo(folder(2))]);
    assert_eq!(harness.state().state.vm.library.selected, Some(folder(2)));
    assert_eq!(in_footer(&harness, "Cello vs Contrabass").len(), 1);
    assert_eq!(in_footer(&harness, "seed 4471").len(), 1);
    let date = local_date(library::vm().library.entries[2].modified);
    assert_eq!(in_footer(&harness, &date).len(), 1, "the footer shows its date");
    assert!(in_footer(&harness, "Tuba vs Bass drum").is_empty());
    assert_eq!(
        control(&harness, &names[2], Role::Button).accesskit_node().toggled(),
        Some(Toggled::True)
    );
    // The chosen one is not asked for again.
    control(&harness, &names[2], Role::Button).click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SelectVideo(folder(2))]);
}

#[test]
fn reopen_and_open_folder_push() {
    let _gpu = gpu();
    let mut harness = showing(REF, library::vm());
    let reopen = control(&harness, "Reopen", Role::Button);
    assert!(!reopen.accesskit_node().is_disabled());
    harness.get_by_label("Open folder").click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::OpenVideoFolder(folder(1))]);
    let sent = harness.state_mut().sent();
    assert!(
        sent.iter()
            .any(|c| matches!(c, Command::RevealFolder { path } if *path == folder(1))),
        "{sent:?}"
    );
    harness.get_by_label("Reopen").click();
    harness.run();
    assert_eq!(
        harness.state().log,
        vec![Action::OpenVideoFolder(folder(1)), Action::Reopen(folder(1))]
    );
    let sent = harness.state_mut().sent();
    assert!(sent.iter().any(|c| matches!(c, Command::OpenFight { .. })), "{sent:?}");
}

#[test]
fn a_video_from_a_newer_version_cannot_be_reopened() {
    let _gpu = gpu();
    let mut vm = library::vm();
    vm.library.selected = Some(folder(3));
    let mut harness = showing(REF, vm);
    let names = tile_names();
    let tile = control(&harness, &names[3], Role::Button).rect();
    let reason = harness
        .query_all_by_label(NEWER)
        .map(|n| n.rect())
        .find(|r| tile.contains_rect(*r))
        .expect("the reason is in its tile");
    assert!(reason.min.y > tile.min.y + 8.0 + COVER.1, "under the cover");
    let reopen = harness.get_by_label("Reopen");
    assert!(reopen.accesskit_node().is_disabled());
    reopen.hover();
    harness.run();
    assert!(
        harness.query_all_by_label(NEWER).count() >= 2,
        "hovering Reopen says why"
    );
    harness.get_by_label("Reopen").click();
    harness.run();
    assert!(harness.state().log.is_empty(), "{:?}", harness.state().log);
    // Its folder can still be opened.
    harness.get_by_label("Open folder").click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::OpenVideoFolder(folder(3))]);
    // The other tiles carry no reason.
    let reasons = harness.query_all_by_label(NEWER).filter(|n| {
        let r = n.rect();
        r.max.y < harness.state().layout.workspace.max.y - 46.0
    });
    assert_eq!(reasons.count(), 1, "only its own tile");
}

#[test]
fn with_nothing_selected_the_footer_waits() {
    let _gpu = gpu();
    let mut vm = library::vm();
    vm.library.selected = None;
    let mut harness = showing(REF, vm);
    for label in ["Reopen", "Open folder"] {
        assert!(harness.get_by_label(label).accesskit_node().is_disabled(), "{label}");
    }
    assert!(has(&harness, bf_app::screens::library::PICK_ONE));
    harness.get_by_label("Reopen").click();
    harness.run();
    assert!(harness.state().log.is_empty());
}

#[test]
fn the_empty_library_says_what_to_do() {
    let _gpu = gpu();
    let harness = showing(REF, library::empty());
    assert!(has(&harness, EMPTY));
    assert!(tiles(&harness).is_empty());
    let count = format!("0 videos in {}", library::vm().settings.output_folder.display());
    assert!(has(&harness, &count), "{count}");
    assert!(has(&harness, "Look again"));
    assert!(!has(&harness, "Reopen") && !has(&harness, "Open folder"));
    let text = harness.get_by_label(EMPTY).rect();
    let workspace = harness.state().layout.workspace;
    assert!(workspace.contains_rect(text), "{text:?}");
    assert!(
        (text.center().x - workspace.center().x).abs() < 2.0,
        "centred: {text:?}"
    );
}

#[test]
fn a_missing_folder_says_what_to_do() {
    let _gpu = gpu();
    let vm = library::missing();
    let problem = library::missing_text(&vm);
    let mut harness = showing(REF, vm);
    assert!(has(&harness, &problem), "{problem}");
    assert!(has(&harness, MISSING_ADVICE));
    assert!(!has(&harness, EMPTY), "it is not an empty folder");
    assert!(tiles(&harness).is_empty());
    let workspace = harness.state().layout.workspace;
    for text in [problem.as_str(), MISSING_ADVICE] {
        assert!(workspace.contains_rect(harness.get_by_label(text).rect()), "{text}");
    }
    // Looking again asks the engine.
    harness.get_by_label("Look again").click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::RefreshLibrary]);
    let sent = harness.state_mut().sent();
    assert!(
        sent.iter().any(|c| matches!(c, Command::ScanLibrary { .. })),
        "{sent:?}"
    );
}

#[test]
fn a_long_title_is_cut_short_and_shown_whole_on_hover() {
    let _gpu = gpu();
    let long = "Bass clarinet vs Contrabass, the rematch nobody asked for";
    let mut vm = library::vm();
    vm.library.entries[0].title = long.to_string();
    let mut harness = showing(MIN, vm);
    let name = format!("{long}, {}", local_date(library::vm().library.entries[0].modified));
    let tile = control(&harness, &name, Role::Button).rect();
    let title = harness.get_by_label(long).rect();
    assert!(
        title.max.x <= tile.max.x - 8.0,
        "cut short inside the tile: {title:?} in {tile:?}"
    );
    assert_eq!(harness.query_all_by_label(long).count(), 1);
    harness.get_by_label(long).hover();
    harness.run();
    assert_eq!(harness.query_all_by_label(long).count(), 2, "hovering shows all of it");
}

#[test]
fn only_the_visible_tiles_are_laid_out() {
    let _gpu = gpu();
    let mut vm = library::vm();
    let sample = vm.library.entries.clone();
    vm.library.entries = (0..400)
        .map(|i| {
            let mut entry = sample[i % sample.len()].clone();
            entry.folder = PathBuf::from("Videos").join(format!("video_{i}"));
            entry.title = format!("Video {i} vs Video {i}");
            entry
        })
        .collect();
    vm.library.selected = None;
    let harness = showing(REF, vm);
    let drawn = tiles(&harness).len();
    assert!((4..=12).contains(&drawn), "{drawn} tiles laid out of 400");
}

#[test]
fn the_grid_shows_its_scroll_bar_when_it_overflows() {
    let _gpu = gpu();
    for size in [REF, MIN] {
        let mut harness = showing(size, library::vm());
        let workspace = harness.state().layout.workspace;
        let x = (workspace.max.x - 4.0 - 3.0) as u32;
        let top = workspace.min.y as u32 + 40;
        let at: Vec<(u32, u32)> = (top + 2..top + 80).map(|y| (x, y)).collect();
        let bar = pixels(&mut harness, &at);
        assert!(
            bar.contains(&[0x77, 0x77, 0x77]),
            "{size:?}: the handle is drawn FAINT without hovering"
        );
    }
    // Nothing overflows when the folder is empty: no bar.
    let mut harness = showing(REF, library::empty());
    let workspace = harness.state().layout.workspace;
    let x = (workspace.max.x - 4.0 - 3.0) as u32;
    let top = workspace.min.y as u32 + 40;
    let at: Vec<(u32, u32)> = (top + 2..top + 80).map(|y| (x, y)).collect();
    assert!(pixels(&mut harness, &at).iter().all(|p| *p == [0x28; 3]), "no bar");
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

/// Every control of the Library in reading order: "Look again", the six tiles, "Open folder", "Reopen".
fn reading_order() -> Vec<String> {
    let mut order = vec!["Look again".to_string()];
    order.extend(tile_names());
    order.extend(["Open folder".to_string(), "Reopen".to_string()]);
    order
}

#[test]
fn every_control_shows_its_focus_ring() {
    let _gpu = gpu();
    for size in [REF, MIN] {
        let mut harness = showing(size, library::vm());
        for name in reading_order() {
            control(&harness, &name, Role::Button).scroll_to_me();
            harness.run();
            let rect = control(&harness, &name, Role::Button).rect();
            let workspace = harness.state().layout.workspace;
            assert!(
                workspace.contains_rect(rect),
                "{size:?}: {name} is on screen at {rect:?}"
            );
            assert_eq!(
                ring_sides(&mut harness, rect),
                [false; 4],
                "{size:?}: {name} before focus"
            );
            control(&harness, &name, Role::Button).focus();
            harness.run();
            assert!(control(&harness, &name, Role::Button).is_focused(), "{size:?}: {name}");
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
    let order = reading_order();
    for size in [REF, MIN] {
        let mut harness = showing(size, library::vm());
        control(&harness, &order[0], Role::Button).focus();
        harness.run();
        for (i, name) in order.iter().enumerate() {
            if i > 0 {
                harness.key_press(Key::Tab);
                harness.run();
            }
            assert!(
                control(&harness, name, Role::Button).is_focused(),
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
fn both_views_at_the_minimum_size() {
    let _gpu = gpu();
    let harness = showing(MIN, library::vm());
    let workspace = harness.state().layout.workspace;
    for label in ["Reopen", "Open folder", "Look again"] {
        let rect = control(&harness, label, Role::Button).rect();
        assert!(
            workspace.contains_rect(rect),
            "{label} at {rect:?} lies inside {workspace:?}"
        );
    }
    let tiles = tiles(&harness);
    assert_eq!(
        tiles.iter().filter(|t| t.1.min.y == tiles[0].1.min.y).count(),
        3,
        "three across"
    );
    drop(harness);
    let harness = showing(MIN, settings::vm());
    let workspace = harness.state().layout.workspace;
    for (label, role) in [
        ("Change folder", Role::Button),
        ("Worker threads", Role::Slider),
        ("Worker threads value", Role::SpinButton),
    ] {
        let rect = control(&harness, label, role).rect();
        assert!(
            workspace.contains_rect(rect),
            "{label} at {rect:?} lies inside {workspace:?}"
        );
    }
}

#[test]
fn nothing_is_unnamed() {
    let _gpu = gpu();
    for vm in [library::vm(), library::empty(), library::missing()] {
        let harness = showing(REF, vm);
        assert_eq!(harness.unnamed_widgets(), Vec::<String>::new());
    }
}

#[test]
fn library_matches() {
    let _gpu = gpu();
    let mut harness = showing(REF, library::vm());
    harness.snapshot("library_1440x900");
}
