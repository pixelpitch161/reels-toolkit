//! The Matchups tab: every fight the app has made (and the episodes made before it), newest first, each with an
//! Uploaded tick, and a footer that deletes the selected row after asking once.
//!
//! Layout. A header 40 tall (padded 16 at the sides, a 1 px `LINE_SOFT` line under it): "Matchups" and
//! "{n} fights · {n} pairings · {n} not uploaded". Then a row of column names (`small` `MUTED`) and the rows, 28 tall
//! with a 1 px `LINE_SOFT` line between, in a scroll area: Made, Left, Right (each with its fighter's 12 px colour
//! swatch, grey for a fighter the roster does not know), Winner, Seed and the Uploaded tick. A footer 46 tall (a 1 px
//! `LINE_SOFT` line above it) names the selected row and holds "Delete", or asks before deleting. With no fights
//! the list's place holds one centred line, and there is no footer.
//!
//! The view reads the view model and pushes actions: a row's click `SelectFight`, its tick `SetUploaded`, the
//! footer's buttons `AskDeleteFight`, `DeleteFight` and `KeepFight`. It keeps nothing of its own.

use egui::scroll_area::ScrollBarVisibility;
use egui::style::{ScrollFadeStyle, ScrollStyle};
use egui::text::LayoutJob;
use egui::{Align, FontId, Label, Layout, Rect, RichText, ScrollArea, Sense, Ui, UiBuilder, vec2};

use crate::model::history::FightRecord;
use crate::state::{Action, ViewModel};
use crate::store::library::local_date;
use crate::theme::{FAINT, GROUND, LINE_SOFT, MUTED, RADIUS, TEXT, body, rgb, section, small};
use crate::widgets::buttons;
use crate::widgets::chips::object_row;
use crate::wording;

/// What an empty history says.
pub const EMPTY: &str = "No fights yet. Render one and it appears here.";
/// The footer with no row selected.
pub const PICK_ONE: &str = "Pick a fight to delete it.";

/// The header's, the column names' and the footer's heights, and the side padding.
const HEADER_H: f32 = 40.0;
const COLUMNS_H: f32 = 28.0;
const FOOTER_H: f32 = 46.0;
const SIDE_PAD: f32 = 16.0;
/// A row's height, and the line under it.
pub const ROW_H: f32 = 28.0;
const LINE_H: f32 = 1.0;
/// An object row's own side padding (`chips::object_row`), so the column names line up with the rows' text.
const ROW_PAD: f32 = 10.0;
/// The columns' widths: Made, Left, Right, Winner, Seed; Uploaded takes the rest.
const COLUMNS: [(&str, f32); 6] = [
    ("Made", 120.0),
    ("Left", 170.0),
    ("Right", 170.0),
    ("Winner", 150.0),
    ("Seed", 80.0),
    ("Uploaded", 80.0),
];
/// A fighter's colour swatch and the gap after it.
const SWATCH: f32 = 12.0;
const SWATCH_GAP: f32 = 8.0;
const WORD_GAP: f32 = 12.0;
const ITEM_GAP: f32 = 8.0;
/// The scroll bar, as the Library's.
const BAR_W: f32 = 6.0;
const BAR_MARGIN: f32 = 4.0;
/// A fighter the roster does not know.
const UNKNOWN: [u8; 3] = [0x80, 0x80, 0x80];
const BUTTON_PAD_X: f32 = 10.0;

/// "28 fights · 26 pairings · 1 not uploaded" ("not uploaded" left out when every fight is uploaded).
pub fn summary_line(vm: &ViewModel) -> String {
    let (fights, pairings, not_uploaded) = vm.history.summary();
    let mut line = format!(
        "{} · {}",
        wording::plural(fights as u64, "fight", "fights"),
        wording::plural(pairings as u64, "pairing", "pairings")
    );
    if not_uploaded > 0 {
        line.push_str(&format!(" · {not_uploaded} not uploaded"));
    }
    line
}

/// "Flute vs Trombone, seed 2210" (without the seed when it is not known).
pub fn fight_words(row: &FightRecord) -> String {
    let versus = wording::versus(&row.fighters);
    match row.seed {
        Some(seed) => format!("{versus}, seed {seed}"),
        None => versus,
    }
}

/// A fighter's colour: the roster's, else the compiled-in roster's, else grey.
fn colour_of(vm: &ViewModel, name: &str) -> [u8; 3] {
    // A developer fighter's swatch only with the developer switch on (ruling E6-R2).
    if !vm.dev && bf_content::dev_only(name) {
        return UNKNOWN;
    }
    vm.roster
        .as_ref()
        .and_then(|r| r.fighters.iter().find(|f| f.name == name).map(|f| f.color))
        .or_else(|| bf_content::roster().iter().find(|f| f.name == name).map(|f| f.color))
        .unwrap_or(UNKNOWN)
}

/// Draws it into `ui` from `vm`, pushing what the owner does to `out`.
pub fn show(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let area = ui.max_rect();
    let header = Rect::from_min_size(area.min, vec2(area.width(), HEADER_H));
    let empty = vm.history.fights.is_empty();
    let bottom = if empty { area.max.y } else { area.max.y - FOOTER_H };
    let painter = ui.painter().clone();
    painter.rect_filled(
        Rect::from_x_y_ranges(area.x_range(), header.max.y - 1.0..=header.max.y),
        0.0,
        LINE_SOFT,
    );
    header_row(ui, vm, header.shrink2(vec2(SIDE_PAD, 0.0)));
    let below = Rect::from_x_y_ranges(area.x_range(), header.max.y..=bottom.max(header.max.y));
    if empty {
        message(ui, below, EMPTY);
        return;
    }
    let names = Rect::from_min_size(below.min, vec2(below.width(), COLUMNS_H));
    column_names(ui, names.shrink2(vec2(SIDE_PAD + ROW_PAD, 0.0)));
    let list = Rect::from_x_y_ranges(below.x_range(), names.max.y..=below.max.y.max(names.max.y));
    rows(ui, vm, list.shrink2(vec2(SIDE_PAD, 0.0)), out);
    let footer = Rect::from_x_y_ranges(area.x_range(), bottom..=area.max.y);
    painter.rect_filled(
        Rect::from_x_y_ranges(area.x_range(), footer.min.y..=footer.min.y + 1.0),
        0.0,
        LINE_SOFT,
    );
    footer_row(ui, vm, footer.shrink2(vec2(SIDE_PAD, 0.0)), out);
}

/// Text that is read, not clicked, cut short with an ellipsis when it does not fit.
fn plain(ui: &mut Ui, text: RichText) -> egui::Response {
    ui.add(Label::new(text).selectable(false).truncate())
}

fn text_w(ui: &Ui, text: &str, font: FontId) -> f32 {
    ui.painter().layout_no_wrap(text.to_owned(), font, TEXT).size().x.ceil()
}

fn button_w(ui: &Ui, label: &str) -> f32 {
    (text_w(ui, label, body()) + 2.0 * BUTTON_PAD_X).ceil()
}

fn header_row(ui: &mut Ui, vm: &ViewModel, inside: Rect) {
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt("matchups header")
            .max_rect(inside)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let ui = &mut child;
    ui.spacing_mut().item_spacing.x = WORD_GAP;
    plain(ui, RichText::new("Matchups").font(section()).color(TEXT));
    plain(ui, RichText::new(summary_line(vm)).font(small()).color(MUTED));
}

/// One centred line in `rect`.
fn message(ui: &mut Ui, rect: Rect, text: &str) {
    let wrap = (rect.width() - 2.0 * SIDE_PAD).max(0.0);
    let mut job = LayoutJob::simple(text.to_owned(), body(), MUTED, wrap);
    job.halign = Align::Center;
    let height = ui.painter().layout_job(job).size().y;
    let at = Rect::from_center_size(rect.center(), vec2(wrap, height));
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt("matchups message")
            .max_rect(at)
            .layout(Layout::top_down(Align::Center)),
    );
    child.add(
        Label::new(RichText::new(text).font(body()).color(MUTED))
            .wrap()
            .selectable(false)
            .halign(Align::Center),
    );
}

/// A cell `width` wide holding `add`, left to right, centred in its height.
fn cell(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) {
    let height = ui.available_height();
    ui.allocate_ui_with_layout(vec2(width, height), Layout::left_to_right(Align::Center), |ui| {
        ui.set_min_width(width);
        ui.set_max_width(width);
        ui.spacing_mut().item_spacing.x = 0.0;
        add(ui);
    });
}

fn column_names(ui: &mut Ui, inside: Rect) {
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt("matchups columns")
            .max_rect(inside)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let ui = &mut child;
    ui.spacing_mut().item_spacing.x = 0.0;
    for (name, width) in COLUMNS {
        cell(ui, width, |ui| {
            plain(ui, RichText::new(name).font(small()).color(MUTED));
        });
    }
}

/// The rows in a scroll area over `rect`; only the rows in view are laid out.
fn rows(ui: &mut Ui, vm: &ViewModel, rect: Rect, out: &mut Vec<Action>) {
    let rows = vm.history.newest_first();
    let mut child = ui.new_child(UiBuilder::new().id_salt("matchups list").max_rect(rect));
    let style = child.style_mut();
    style.spacing.scroll = ScrollStyle {
        bar_width: BAR_W,
        bar_inner_margin: BAR_MARGIN,
        bar_outer_margin: BAR_MARGIN,
        fade: ScrollFadeStyle {
            strength: 0.0,
            size: 0.0,
        },
        ..ScrollStyle::solid()
    };
    style.spacing.item_spacing.y = 0.0;
    style.visuals.extreme_bg_color = GROUND;
    let widgets = &mut style.visuals.widgets;
    widgets.inactive.bg_fill = FAINT;
    widgets.hovered.bg_fill = MUTED;
    widgets.active.bg_fill = TEXT;
    ScrollArea::vertical()
        .id_salt("matchups rows")
        .auto_shrink(false)
        .scroll_bar_visibility(ScrollBarVisibility::VisibleWhenNeeded)
        .show_rows(&mut child, ROW_H + LINE_H, rows.len(), |ui, range| {
            for row in &rows[range] {
                fight_row(ui, vm, row, out);
            }
        });
}

/// A fighter's swatch and shown name.
fn fighter(ui: &mut Ui, vm: &ViewModel, name: &str) {
    let (swatch, _) = ui.allocate_exact_size(vec2(SWATCH, SWATCH), Sense::hover());
    ui.painter().rect_filled(swatch, RADIUS, rgb(colour_of(vm, name)));
    ui.add_space(SWATCH_GAP);
    plain(ui, RichText::new(wording::display_name(name)).font(body()));
}

fn fight_row(ui: &mut Ui, vm: &ViewModel, row: &FightRecord, out: &mut Vec<Action>) {
    let selected = vm.matchups.selected == Some(row.id);
    let words = fight_words(row);
    let date = local_date(row.made);
    let name = format!("{words}, {date}");
    let tick_name = format!("Uploaded, {words}");
    let mut ticked = None;
    let (response, ()) = ui
        .push_id(row.id, |ui| {
            object_row(ui, &name, selected, ROW_H, |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let [made, left, right, winner, seed, _] = COLUMNS.map(|(_, w)| w);
                cell(ui, made, |ui| {
                    plain(ui, RichText::new(&date).font(body()));
                });
                cell(ui, left, |ui| fighter(ui, vm, &row.fighters[0]));
                cell(ui, right, |ui| fighter(ui, vm, &row.fighters[1]));
                cell(ui, winner, |ui| {
                    let text = row
                        .winner
                        .as_deref()
                        .map_or_else(|| "—".to_string(), wording::display_name);
                    plain(ui, RichText::new(text).font(body()));
                });
                cell(ui, seed, |ui| {
                    let text = row.seed.map_or_else(|| "—".to_string(), |s| s.to_string());
                    plain(ui, RichText::new(text).font(body()));
                    if row.engine == bf_core::Engine::Two {
                        ui.add_space(ITEM_GAP);
                        crate::widgets::chips::engine_mark(ui);
                    }
                });
                if buttons::tick_box(ui, &tick_name, row.uploaded).clicked() {
                    ticked = Some(!row.uploaded);
                }
            })
        })
        .inner;
    let line = Rect::from_min_size(response.rect.left_bottom(), vec2(response.rect.width(), LINE_H));
    ui.allocate_rect(line, Sense::hover());
    ui.painter().rect_filled(line, 0.0, LINE_SOFT);
    if let Some(uploaded) = ticked {
        out.push(Action::SetUploaded { id: row.id, uploaded });
    } else if response.clicked() && !selected {
        out.push(Action::SelectFight(row.id));
    }
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
}

/// The footer: what to do, the selected row and "Delete", or the question with "Delete" and "Keep it".
fn footer_row(ui: &mut Ui, vm: &ViewModel, inside: Rect, out: &mut Vec<Action>) {
    let chosen = vm.matchups.selected.and_then(|id| vm.history.fight(id));
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt("matchups footer")
            .max_rect(inside)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let ui = &mut child;
    ui.spacing_mut().item_spacing.x = ITEM_GAP;
    let Some(row) = chosen else {
        plain(ui, RichText::new(PICK_ONE).font(body()).color(MUTED));
        return;
    };
    let asking = vm.matchups.asking_delete;
    let (text, buttons_w) = if asking {
        (
            format!("Delete {}? This cannot be undone.", fight_words(row)),
            button_w(ui, "Delete") + button_w(ui, "Keep it") + 2.0 * ITEM_GAP,
        )
    } else {
        (fight_words(row), button_w(ui, "Delete") + ITEM_GAP)
    };
    let room = (ui.available_width() - buttons_w).max(0.0);
    ui.allocate_ui_with_layout(
        vec2(room, inside.height()),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_min_width(room);
            plain(ui, RichText::new(text).font(body()).color(TEXT));
        },
    );
    if asking {
        if buttons::primary(ui, "Delete").clicked() {
            out.push(Action::DeleteFight(row.id));
        }
        if buttons::boxed(ui, "Keep it").clicked() {
            out.push(Action::KeepFight);
        }
    } else if buttons::boxed(ui, "Delete").clicked() {
        out.push(Action::AskDeleteFight);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_columns_fit_the_smallest_workspace() {
        // The smallest window's workspace is 870 wide.
        let total: f32 = COLUMNS.iter().map(|(_, w)| w).sum();
        assert!(
            total + 2.0 * (SIDE_PAD + ROW_PAD) + BAR_W + 2.0 * BAR_MARGIN <= 870.0,
            "{total}"
        );
    }
}
