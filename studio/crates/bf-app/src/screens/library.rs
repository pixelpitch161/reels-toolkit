//! The Library (spec section 4; interface reference, sections 9 and 11): the videos saved under the output folder, as
//! a grid of covers with the match-up and the date, and a footer for the chosen one.
//!
//! Layout. A header 40 tall (padded 16 at the sides, a 1 px `LINE_SOFT` line under it): "Library", how many videos
//! the folder holds, and "Look again" at the right. Then the grid, padded 16, of tiles 196 wide and 16 apart, as many
//! across as fit beside the scroll bar; it scrolls when it does not fit and then shows a solid bar without being
//! hovered. A footer 46 tall (a 1 px `LINE_SOFT` line above it) names the chosen video and holds "Open folder" and
//! "Reopen". With no videos the grid's place holds one centred line that says what to do, and there is no footer;
//! when the output folder is missing or cannot be read (Review Focus 1) it holds the store's words and what to do,
//! and the header offers "Change folder" beside "Look again" (ruling M2-R26). The grid has no edge fade.
//!
//! A tile is an object row: the cover at 180 x 320 (a `RAISED` block until it arrives), the title, the date and, for
//! a video that cannot be reopened, the reason, two lines at most. Only the tiles in view are laid out.
//!
//! The view reads the view model and pushes actions: a tile's click `SelectVideo`, "Look again" `RefreshLibrary`,
//! "Open folder" `OpenVideoFolder` and "Reopen" `Reopen` (disabled, saying why, for a video with a problem). It keeps
//! nothing of its own beyond egui's widget memory, and nothing here deletes anything.

use std::sync::{Arc, LazyLock};

use egui::scroll_area::ScrollBarVisibility;
use egui::style::{ScrollFadeStyle, ScrollStyle};
use egui::text::{LayoutJob, TextWrapping};
use egui::{Align, Color32, FontId, Label, Layout, Rect, RichText, ScrollArea, Ui, UiBuilder, vec2};

use crate::model::library::{FolderState, LibraryEntry};
use crate::model::take::Frame;
use crate::state::{Action, ViewModel};
use crate::store::library::local_date;
use crate::theme::{FAINT, GROUND, LINE_SOFT, MUTED, TEXT, body, section, small};
use crate::widgets::buttons;
use crate::widgets::chips::{object_row, picture};
use crate::wording;

/// A tile's width and height: 8 above the cover, the cover, 8, the title (18), the date (16), two lines for a
/// problem (32) and 8 below.
pub const TILE_W: f32 = 196.0;
pub const TILE_H: f32 = 410.0;
/// A cover's size in a tile.
pub const COVER: (f32, f32) = (180.0, 320.0);
/// What an output folder with no videos says.
pub const EMPTY: &str = "No videos yet. Render one and it appears here.";
/// What to do when the output folder is missing. Rendering does not make it (Render holds "Add to queue" back and
/// offers "Create folder" there), so the advice names the place that does.
pub const MISSING_ADVICE: &str = "Choose another folder, or go to Render to create it.";
/// What to do when the output folder is there but cannot be read: only another folder helps.
pub const UNREADABLE_ADVICE: &str = "Choose another folder.";
/// The footer with no video chosen.
pub const PICK_ONE: &str = "Pick a video to reopen it or open its folder.";
/// Why the footer's buttons wait with no video chosen.
const PICK_FIRST: &str = "Pick a video first.";
/// While the folder is being looked through and nothing has been found yet.
const LOOKING: &str = "Looking for videos.";
/// Why a video without a fight file (and no problem of its own) cannot be reopened.
const NO_FIGHT_FILE: &str = "This video has no fight file, so it cannot be reopened.";

/// The header's and the footer's heights, and their side padding.
const HEADER_H: f32 = 40.0;
const FOOTER_H: f32 = 46.0;
const SIDE_PAD: f32 = 16.0;
/// The gap after "Library" in the header and between the footer's words; the gap between the footer's buttons.
const WORD_GAP: f32 = 12.0;
const ITEM_GAP: f32 = 8.0;
/// The grid's padding and the gap between tiles (across and down).
const GRID_PAD: f32 = 16.0;
const GAP: f32 = 16.0;
/// The space inside a tile around its cover and text.
const TILE_PAD: f32 = 8.0;
/// The most lines a tile's problem takes.
const PROBLEM_ROWS: usize = 2;
/// The scroll bar: 6 wide, 4 from the content and 4 from the edge. The grid always keeps its room, so the number of
/// tiles across never depends on whether the bar shows.
const BAR_W: f32 = 6.0;
const BAR_MARGIN: f32 = 4.0;
const BAR_ROOM: f32 = BAR_W + 2.0 * BAR_MARGIN;
/// A button's side padding, as the buttons widget pads its label.
const BUTTON_PAD_X: f32 = 10.0;

/// A cover that has not arrived: a picture with no pixels, which `chips::picture` draws as a `RAISED` block.
static NO_COVER: LazyLock<Frame> = LazyLock::new(|| Frame {
    w: 0,
    h: 0,
    rgb: Arc::from(Vec::new()),
});

/// How many tiles fit across a grid `width` wide (its padding and the scroll bar's room taken off): at least 1.
pub fn tiles_across(width: f32) -> usize {
    let n = ((width - 2.0 * GRID_PAD - BAR_ROOM + GAP) / (TILE_W + GAP)).floor();
    if n.is_finite() && n >= 1.0 { n as usize } else { 1 }
}

/// Draws it into `ui` from `vm`, pushing what the owner does to `out`.
pub fn show(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let lib = &vm.library;
    let area = ui.max_rect();
    let header = Rect::from_min_size(area.min, vec2(area.width(), HEADER_H));
    let with_footer = lib.problem.is_none() && !lib.entries.is_empty();
    let bottom = if with_footer { area.max.y - FOOTER_H } else { area.max.y };
    let list = Rect::from_x_y_ranges(area.x_range(), header.max.y..=bottom.max(header.max.y));
    let painter = ui.painter().clone();
    painter.rect_filled(
        Rect::from_x_y_ranges(area.x_range(), header.max.y - 1.0..=header.max.y),
        0.0,
        LINE_SOFT,
    );
    header_row(ui, vm, header.shrink2(vec2(SIDE_PAD, 0.0)), out);
    if let Some(problem) = &lib.problem {
        let advice = if vm.folder == FolderState::Missing {
            MISSING_ADVICE
        } else {
            UNREADABLE_ADVICE
        };
        message(ui, list, &[(problem, TEXT), (advice, MUTED)]);
    } else if lib.entries.is_empty() {
        let text = if lib.scanning { LOOKING } else { EMPTY };
        message(ui, list, &[(text, MUTED)]);
    } else {
        grid(ui, vm, list, out);
    }
    if with_footer {
        let footer = Rect::from_x_y_ranges(area.x_range(), bottom..=area.max.y);
        painter.rect_filled(
            Rect::from_x_y_ranges(area.x_range(), footer.min.y..=footer.min.y + 1.0),
            0.0,
            LINE_SOFT,
        );
        footer_row(ui, vm, footer.shrink2(vec2(SIDE_PAD, 0.0)), out);
    }
}

/// Text that is read, not clicked or selected, cut short with an ellipsis when it does not fit (hovering it then
/// shows all of it).
fn plain(ui: &mut Ui, text: RichText) -> egui::Response {
    ui.add(Label::new(text).selectable(false).truncate())
}

/// The width `text` takes on one line in `font`, rounded up.
fn text_w(ui: &Ui, text: &str, font: FontId) -> f32 {
    ui.painter().layout_no_wrap(text.to_owned(), font, TEXT).size().x.ceil()
}

/// The width a button with `label` takes.
fn button_w(ui: &Ui, label: &str) -> f32 {
    let text = ui.painter().layout_no_wrap(label.to_owned(), body(), TEXT).size().x;
    (text + 2.0 * BUTTON_PAD_X).ceil()
}

/// The header: "Library" (`section`), "{n} videos in {folder}" (`small` `MUTED`, cut short when it does not fit),
/// then at the right the box button "Change folder" while the folder is missing or unreadable, and "Look again".
fn header_row(ui: &mut Ui, vm: &ViewModel, inside: Rect, out: &mut Vec<Action>) {
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt("library header")
            .max_rect(inside)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let ui = &mut child;
    ui.spacing_mut().item_spacing.x = 0.0;
    plain(ui, RichText::new("Library").font(section()).color(TEXT));
    ui.add_space(WORD_GAP);
    let count = format!(
        "{} in {}",
        wording::plural(vm.library.entries.len() as u64, "video", "videos"),
        vm.settings.output_folder.display()
    );
    let look = "Look again";
    // With the folder missing or unreadable (Review Focus 1, ruling M2-R26) the way forward is offered here too.
    let change = vm.library.problem.is_some().then_some("Change folder");
    let buttons = button_w(ui, look) + change.map_or(0.0, |c| button_w(ui, c) + ITEM_GAP);
    let room = (ui.available_width() - buttons - WORD_GAP).max(0.0);
    ui.allocate_ui_with_layout(
        vec2(room, inside.height()),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_min_width(room);
            plain(ui, RichText::new(count).font(small()).color(MUTED));
        },
    );
    ui.add_space(WORD_GAP);
    if let Some(change) = change {
        if buttons::boxed(ui, change).clicked() {
            out.push(Action::ChangeFolder);
        }
        ui.add_space(ITEM_GAP);
    }
    if buttons::boxed(ui, look).clicked() {
        out.push(Action::RefreshLibrary);
    }
}

/// Lines of text centred in `rect`, each wrapped to its width less the side padding, in its colour.
fn message(ui: &mut Ui, rect: Rect, lines: &[(&str, Color32)]) {
    let wrap = (rect.width() - 2.0 * SIDE_PAD).max(0.0);
    let gap = ui.spacing().item_spacing.y;
    let height: f32 = lines
        .iter()
        .map(|(text, colour)| {
            let mut job = LayoutJob::simple((*text).to_owned(), body(), *colour, wrap);
            job.halign = Align::Center;
            ui.painter().layout_job(job).size().y
        })
        .sum::<f32>()
        + gap * lines.len().saturating_sub(1) as f32;
    let at = Rect::from_center_size(rect.center(), vec2(wrap, height));
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt("library message")
            .max_rect(at)
            .layout(Layout::top_down(Align::Center)),
    );
    for (text, colour) in lines {
        child.add(
            Label::new(RichText::new(*text).font(body()).color(*colour))
                .wrap()
                .selectable(false)
                .halign(Align::Center),
        );
    }
}

/// The grid of tiles in a scroll area over `rect`. Only the rows in view are laid out.
fn grid(ui: &mut Ui, vm: &ViewModel, rect: Rect, out: &mut Vec<Action>) {
    let entries = &vm.library.entries;
    let across = tiles_across(rect.width());
    let rows = entries.len().div_ceil(across);
    let step = TILE_H + GAP;
    let total = 2.0 * GRID_PAD + rows as f32 * step - GAP;
    let mut child = ui.new_child(UiBuilder::new().id_salt("library grid").max_rect(rect));
    let style = child.style_mut();
    style.spacing.scroll = ScrollStyle {
        bar_width: BAR_W,
        bar_inner_margin: BAR_MARGIN,
        bar_outer_margin: BAR_MARGIN,
        // No fade at the edges: the covers are drawn at full strength up to the grid's edge.
        fade: ScrollFadeStyle {
            strength: 0.0,
            size: 0.0,
        },
        ..ScrollStyle::solid()
    };
    style.visuals.extreme_bg_color = GROUND;
    let widgets = &mut style.visuals.widgets;
    widgets.inactive.bg_fill = FAINT;
    widgets.hovered.bg_fill = MUTED;
    widgets.active.bg_fill = TEXT;
    ScrollArea::vertical()
        .id_salt("library tiles")
        .auto_shrink(false)
        .scroll_bar_visibility(ScrollBarVisibility::VisibleWhenNeeded)
        .show_viewport(&mut child, |ui, viewport| {
            ui.set_height(total);
            let origin = ui.max_rect().min;
            // Only the rows that reach into the view: row `r` starts at `GRID_PAD + r * step`.
            let first = ((viewport.min.y - GRID_PAD - TILE_H) / step).ceil().max(0.0) as usize;
            let last = ((viewport.max.y - GRID_PAD) / step).floor().max(0.0) as usize;
            for row in first..=last.min(rows.saturating_sub(1)) {
                let top = GRID_PAD + row as f32 * step;
                for col in 0..across {
                    let Some(entry) = entries.get(row * across + col) else {
                        break;
                    };
                    let left = GRID_PAD + col as f32 * (TILE_W + GAP);
                    let at = Rect::from_min_size(origin + vec2(left, top), vec2(TILE_W, TILE_H));
                    tile(ui, vm, entry, at, out);
                }
            }
        });
}

/// One tile at `rect`: an object row named "{title}, {date}" holding the cover, the title, the date and the problem.
fn tile(ui: &mut Ui, vm: &ViewModel, entry: &LibraryEntry, rect: Rect, out: &mut Vec<Action>) {
    let lib = &vm.library;
    let selected = lib.selected.as_ref() == Some(&entry.folder);
    let date = local_date(entry.modified);
    let name = format!("{}, {date}", entry.title);
    let mut cell = ui.new_child(
        UiBuilder::new()
            .id_salt(("tile", &entry.folder))
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
    );
    let (response, ()) = object_row(&mut cell, &name, selected, TILE_H, |ui| {
        let inside = rect.shrink(TILE_PAD);
        let mut column = ui.new_child(
            UiBuilder::new()
                .id_salt("tile content")
                .max_rect(inside)
                .layout(Layout::top_down(Align::Min)),
        );
        let ui = &mut column;
        ui.spacing_mut().item_spacing.y = 0.0;
        let cover = lib.covers.get(&entry.folder).unwrap_or(&NO_COVER);
        let key = format!("cover {}", entry.folder.display());
        let cover_name = format!("Cover of {}", entry.title);
        picture(ui, &key, cover, vec2(COVER.0, COVER.1), &cover_name);
        ui.add_space(TILE_PAD);
        plain(ui, RichText::new(&entry.title).font(body()));
        plain(ui, RichText::new(&date).font(small()).color(MUTED));
        if let Some(problem) = &entry.problem {
            let mut job = LayoutJob::simple(problem.clone(), small(), MUTED, inside.width());
            job.wrap = TextWrapping {
                max_width: inside.width(),
                max_rows: PROBLEM_ROWS,
                break_anywhere: false,
                overflow_character: Some('…'),
            };
            ui.add(Label::new(job).wrap().selectable(false));
        }
    });
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
    if response.clicked() && !selected {
        out.push(Action::SelectVideo(entry.folder.clone()));
    }
}

/// Why `entry` cannot be reopened, in words; `None` when it can.
fn why_not_reopen(entry: &LibraryEntry) -> Option<&str> {
    match (&entry.problem, &entry.fight_path) {
        (Some(problem), _) => Some(problem),
        (None, None) => Some(NO_FIGHT_FILE),
        (None, Some(_)) => None,
    }
}

/// The footer: the chosen video's title, date and "seed {n}" (or what to do when none is chosen), then the box
/// button "Open folder" and the primary button "Reopen", in reading order.
fn footer_row(ui: &mut Ui, vm: &ViewModel, inside: Rect, out: &mut Vec<Action>) {
    let lib = &vm.library;
    let chosen = lib
        .selected
        .as_ref()
        .and_then(|folder| lib.entries.iter().find(|e| &e.folder == folder));
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt("library footer")
            .max_rect(inside)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let ui = &mut child;
    ui.spacing_mut().item_spacing.x = ITEM_GAP;
    let (open, reopen) = ("Open folder", "Reopen");
    let buttons = button_w(ui, open) + button_w(ui, reopen) + 2.0 * ITEM_GAP;
    let room = (ui.available_width() - buttons).max(0.0);
    ui.allocate_ui_with_layout(
        vec2(room, inside.height()),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_min_width(room);
            ui.spacing_mut().item_spacing.x = WORD_GAP;
            let Some(entry) = chosen else {
                plain(ui, RichText::new(PICK_ONE).font(body()).color(MUTED));
                return;
            };
            let date = local_date(entry.modified);
            let seed = entry.seed.map(|s| format!("seed {s}"));
            // The title gives way first, so the date and the seed always show whole.
            let mut rest = text_w(ui, &date, body()) + WORD_GAP;
            if let Some(seed) = &seed {
                rest += text_w(ui, seed, body()) + WORD_GAP;
            }
            let title_w = (ui.available_width() - rest).max(0.0);
            ui.allocate_ui_with_layout(
                vec2(title_w, inside.height()),
                Layout::left_to_right(Align::Center),
                |ui| plain(ui, RichText::new(&entry.title).font(body()).color(TEXT)),
            );
            plain(ui, RichText::new(date).font(body()).color(MUTED));
            if let Some(seed) = seed {
                plain(ui, RichText::new(seed).font(body()).color(MUTED));
            }
        },
    );
    match chosen {
        Some(entry) => {
            if buttons::boxed(ui, open).clicked() {
                out.push(Action::OpenVideoFolder(entry.folder.clone()));
            }
            match why_not_reopen(entry) {
                Some(why) => {
                    buttons::disabled(ui, reopen, why);
                }
                None => {
                    if buttons::primary(ui, reopen).clicked() {
                        out.push(Action::Reopen(entry.folder.clone()));
                    }
                }
            }
        }
        None => {
            buttons::disabled(ui, open, PICK_FIRST);
            buttons::disabled(ui, reopen, PICK_FIRST);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_across_the_two_windows() {
        // The reference workspace is 1030 wide and the smallest 870.
        assert_eq!(tiles_across(1030.0), 4);
        assert_eq!(tiles_across(870.0), 3);
        assert_eq!(tiles_across(100.0), 1, "at least one");
        assert_eq!(tiles_across(f32::NAN), 1);
    }

    #[test]
    fn a_tile_has_room_for_its_parts() {
        assert_eq!(TILE_H, 8.0 + COVER.1 + 8.0 + 18.0 + 16.0 + 32.0 + 8.0);
        assert_eq!(TILE_W, COVER.0 + 2.0 * TILE_PAD);
    }

    #[test]
    fn a_video_with_a_problem_or_no_fight_file_cannot_be_reopened() {
        let (mut entry, _) = crate::seam::sample::library().swap_remove(0);
        assert_eq!(why_not_reopen(&entry), None);
        entry.fight_path = None;
        assert_eq!(why_not_reopen(&entry), Some(NO_FIGHT_FILE));
        entry.problem = Some("Made by a newer version of the app.".to_string());
        assert_eq!(why_not_reopen(&entry), Some("Made by a newer version of the app."));
    }
}
