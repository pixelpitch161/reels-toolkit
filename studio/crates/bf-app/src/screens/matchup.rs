//! The Match-up step's workspace (interface reference, 5.1): the left corner card, the roster and the right corner
//! card side by side, and under them the bottom strip with the theme, the tempo and the pair rules.
//!
//! Layout. The three columns are 250, the rest, and 250 wide, with 1 px `LINE_SOFT` lines drawn on the roster's
//! edges. Each column scrolls by itself when its content does not fit (the roster does at the minimum window, two
//! chips wide). The bottom strip (a 1 px `LINE_SOFT` line above it, padded 12 x 16, rows 26 tall and 8 apart) holds
//! three rows: "Theme", "Tempo" with "Swap sides" and "Balance this match-up" at the right, and "Pair rules". When the
//! pair cannot fight, the reason stands between the columns and the strip in `TEXT_STRONG`, and "Balance this
//! match-up" is disabled with it.
//!
//! The screen reads the view model and pushes actions: a chip's click picks the left corner and its right-click the
//! right (`PickFighter`; picking the fighter in the other corner swaps, which the state does), the dropdown picks a
//! theme (`SetTheme`), the slider and the value field set the tempo in whole bars (`SetBars`), and the two buttons
//! push `SwapSides` and `Go(Balance)`. It keeps nothing of its own beyond egui's widget memory.

use egui::{Align, Frame, Label, Layout, Margin, Rect, RichText, ScrollArea, Sense, Stroke, Ui, UiBuilder, pos2, vec2};

use crate::model::draft::Draft;
use crate::model::roster::{Roster, RosterEntry, Section};
use crate::model::{Step, View};
use crate::state::{Action, ViewModel};
use crate::theme::{CONTROL_H, LABEL_W, LINE_SOFT, MUTED, RADIUS, TEXT, TEXT_STRONG, body, rgb, section, small, value};
use crate::widgets::buttons;
use crate::widgets::chips::{portrait, roster_chip};
use crate::widgets::fields::{Unit, value_field};
use crate::widgets::pixel::{fit_scale, pixel_text};
use crate::widgets::slider::bar_slider;
use crate::wording;

/// A corner card's width.
pub const CARD_W: f32 = 250.0;
/// A corner card's padding: 12 at the sides, 14 at the top and bottom; and the gap between its blocks.
const CARD_PAD: (i8, i8) = (12, 14);
const CARD_GAP: f32 = 14.0;
/// The portrait's size and the gap after it.
const PORTRAIT: f32 = 52.0;
const PORTRAIT_GAP: f32 = 12.0;
/// The largest pixel scale of a corner card's name (28 tall), and the one it drops to when that does not fit.
const NAME_SCALE: u32 = 4;
/// The width of an ability's tag column and of a fact's label column.
const TAG_W: f32 = 30.0;
const FACT_W: f32 = 92.0;
/// The height of one line of body text in a card's rows.
const LINE_H: f32 = 18.0;
/// The colour swatch.
const SWATCH: f32 = 12.0;
/// The gap between a weapon and its paragraph, and between rows inside a card's block.
const SMALL_GAP: f32 = 4.0;
const ROW_GAP: f32 = 6.0;

/// The roster's padding: 16 at the sides, 14 at the top and bottom; the gap between its blocks.
const ROSTER_PAD: (i8, i8) = (16, 14);
const ROSTER_GAP: f32 = 10.0;
/// The gap under a section's name, between chips side by side and between rows of chips.
const SECTION_GAP: f32 = 4.0;
const CHIP_GAP_X: f32 = 6.0;
const CHIP_GAP_Y: f32 = 3.0;
/// The narrowest chip plus the gap after it: a column takes this much.
const CHIP_STEP: f32 = 126.0;
/// The gap between the roster's count and its hint when both fit on one line.
const HINT_GAP: f32 = 16.0;
/// What the roster's header says about the mouse.
const HINT: &str = "Click for the left corner, right-click for the right";

/// The bottom strip: its padding, the gap between its rows, the dropdown, the slider at most, the value field.
const STRIP_PAD: (f32, f32) = (16.0, 12.0);
const STRIP_GAP: f32 = 8.0;
const STRIP_ROWS: f32 = 3.0;
const THEME_W: f32 = 220.0;
const SLIDER_MAX_W: f32 = 220.0;
const SLIDER_MIN_W: f32 = 40.0;
const FIELD_W: f32 = 86.0;
/// The space a row's items keep between them.
const ITEM_GAP: f32 = 8.0;
/// A button's side padding (as the buttons widget pads its label).
const BUTTON_PAD_X: f32 = 10.0;
/// The two buttons of the tempo row.
const SWAP: &str = "Swap sides";
const BALANCE: &str = "Balance this match-up";
/// The padding around the fight problem's text.
const PROBLEM_PAD: (f32, f32) = (16.0, 8.0);

/// How the value field writes a tempo: "133.77 bpm".
const BPM: Unit<'static> = Unit {
    prefix: "",
    suffix: " bpm",
    decimals: 2,
};
/// The value field's name.
pub const TEMPO_FIELD: &str = "Tempo in bpm";

/// How many columns of chips fit in `width`: `(width + 6) / 126` rounded down, kept in 2 to 4 (2 for a width that
/// is not a number).
pub fn columns_for(width: f32) -> usize {
    let columns = ((width + CHIP_GAP_X) / CHIP_STEP).floor();
    if columns.is_nan() {
        return 2;
    }
    columns.clamp(2.0, 4.0) as usize
}

/// A section's name as the roster shows it.
fn section_name(section: Section) -> &'static str {
    match section {
        Section::Woodwinds => "Woodwinds",
        Section::Brass => "Brass",
        Section::Strings => "Strings",
        Section::Keys => "Keys and electronics",
        Section::Percussion => "Percussion",
    }
}

/// The sections in the order they are shown.
const SECTIONS: [Section; 5] = [
    Section::Woodwinds,
    Section::Brass,
    Section::Strings,
    Section::Keys,
    Section::Percussion,
];

/// Draws it into `ui` from `vm`, pushing what the owner does to `out`. Nothing is drawn until the roster arrives.
pub fn show(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let Some(roster) = &vm.roster else {
        return;
    };
    let area = ui.max_rect();
    let line = Stroke::new(1.0, LINE_SOFT);

    let strip_h = 1.0 + 2.0 * STRIP_PAD.1 + STRIP_ROWS * CONTROL_H + (STRIP_ROWS - 1.0) * STRIP_GAP;
    let strip = Rect::from_min_max(pos2(area.min.x, (area.max.y - strip_h).max(area.min.y)), area.max);
    let problem_h = vm.fight_problem.as_ref().map_or(0.0, |problem| {
        let wrap = (area.width() - 2.0 * PROBLEM_PAD.0).max(0.0);
        let galley = ui.painter().layout(problem.clone(), body(), TEXT_STRONG, wrap);
        1.0 + 2.0 * PROBLEM_PAD.1 + galley.size().y
    });
    let problem_rect = Rect::from_min_max(
        pos2(area.min.x, (strip.min.y - problem_h).max(area.min.y)),
        pos2(area.max.x, strip.min.y),
    );
    let columns = Rect::from_min_max(area.min, pos2(area.max.x, problem_rect.min.y));
    let card_w = CARD_W.min(columns.width() / 3.0);
    let left = Rect::from_min_max(columns.min, pos2(columns.min.x + card_w, columns.max.y));
    let right = Rect::from_min_max(pos2(columns.max.x - card_w, columns.min.y), columns.max);
    let middle = Rect::from_min_max(pos2(left.max.x, columns.min.y), pos2(right.min.x, columns.max.y));

    region(ui, left, left, |ui| corner_card(ui, vm, 0));
    region(ui, middle, middle, |ui| roster_column(ui, vm, roster, out));
    region(ui, right, right, |ui| corner_card(ui, vm, 1));
    if let Some(problem) = &vm.fight_problem {
        let inner = problem_rect.shrink2(vec2(PROBLEM_PAD.0, PROBLEM_PAD.1));
        let inner = Rect::from_min_max(pos2(inner.min.x, inner.min.y + 1.0), inner.max);
        region(ui, inner, problem_rect, |ui| {
            let text = RichText::new(problem).font(body()).color(TEXT_STRONG);
            ui.add(Label::new(text).selectable(false).wrap());
        });
    }
    let inner = Rect::from_min_max(
        pos2(strip.min.x + STRIP_PAD.0, strip.min.y + 1.0 + STRIP_PAD.1),
        pos2(strip.max.x - STRIP_PAD.0, strip.max.y - STRIP_PAD.1),
    );
    region(ui, inner, strip, |ui| bottom_strip(ui, vm, roster, out));

    let painter = ui.painter();
    painter.vline(middle.min.x + 0.5, columns.y_range(), line);
    painter.vline(middle.max.x - 0.5, columns.y_range(), line);
    if vm.fight_problem.is_some() {
        painter.hline(area.x_range(), problem_rect.min.y + 0.5, line);
    }
    painter.hline(area.x_range(), strip.min.y + 0.5, line);
}

/// Runs `add` in a child of `ui` that fills `rect`, top to bottom, clipped to `clip` (the whole region the content
/// sits in, padding included, so a focus ring 4 px outside a control at the padding's edge is not cut off).
fn region(ui: &mut Ui, rect: Rect, clip: Rect, add: impl FnOnce(&mut Ui)) {
    ui.scope_builder(
        UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::Min)),
        |ui| {
            ui.set_clip_rect(clip.intersect(ui.clip_rect()));
            add(ui);
        },
    );
}

/// One line of text that is read, not clicked or selected, cut short with an ellipsis when it does not fit.
fn line(ui: &mut Ui, text: RichText) {
    ui.add(Label::new(text).selectable(false).truncate());
}

/// One line of text in a column `width` wide and `height` tall, cut short when it does not fit.
fn cell(ui: &mut Ui, width: f32, height: f32, text: RichText) {
    ui.allocate_ui_with_layout(vec2(width, height), Layout::left_to_right(Align::Center), |ui| {
        ui.set_min_size(vec2(width, height));
        line(ui, text);
    });
}

/// A 1 px `LINE_SOFT` line across `ui`.
fn divider(ui: &mut Ui) {
    let (_, rect) = ui.allocate_space(vec2(ui.available_width(), 1.0));
    ui.painter()
        .hline(rect.x_range(), rect.center().y, Stroke::new(1.0, LINE_SOFT));
}

/// A corner card: "Left corner" or "Right corner", the portrait and the name in pixel letters, the weapon, the three
/// abilities, and the range, what the fighter usually plays and its colour. It scrolls when it does not fit.
fn corner_card(ui: &mut Ui, vm: &ViewModel, side: usize) {
    ScrollArea::vertical()
        .id_salt(("matchup corner", side))
        .auto_shrink(false)
        .show(ui, |ui| {
            Frame::new()
                .inner_margin(Margin::symmetric(CARD_PAD.0, CARD_PAD.1))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing = vec2(ITEM_GAP, 0.0);
                    let corner = if side == 0 { "Left corner" } else { "Right corner" };
                    line(ui, RichText::new(corner).font(small()).color(MUTED));
                    let Some(fighter) = vm.fighter(side) else {
                        return;
                    };
                    ui.add_space(CARD_GAP);
                    name_row(ui, fighter);
                    let Some(card) = &fighter.card else {
                        return;
                    };
                    ui.add_space(CARD_GAP);
                    line(ui, RichText::new(&card.weapon).font(section()).color(TEXT));
                    ui.add_space(SMALL_GAP);
                    let paragraph = RichText::new(&card.text).font(body()).color(MUTED);
                    ui.add(Label::new(paragraph).selectable(false).wrap());
                    ui.add_space(CARD_GAP);
                    divider(ui);
                    ui.add_space(CARD_GAP);
                    line(ui, RichText::new("Abilities").font(section()).color(TEXT));
                    for ability in &card.abilities {
                        ui.add_space(ROW_GAP);
                        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                            cell(
                                ui,
                                TAG_W,
                                LINE_H,
                                RichText::new(&ability.tag).font(value()).color(MUTED),
                            );
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                line(ui, RichText::new(&ability.name).font(body()).color(TEXT));
                                line(ui, RichText::new(&ability.text).font(small()).color(MUTED));
                            });
                        });
                    }
                    ui.add_space(CARD_GAP);
                    divider(ui);
                    ui.add_space(CARD_GAP);
                    fact(ui, "Range", |ui| {
                        line(ui, RichText::new(&card.range).font(body()).color(TEXT))
                    });
                    ui.add_space(ROW_GAP);
                    fact(ui, "Usually plays", |ui| {
                        line(ui, RichText::new(&card.usually_plays).font(body()).color(TEXT));
                    });
                    ui.add_space(ROW_GAP);
                    fact(ui, "Colour", |ui| {
                        let (rect, _) = ui.allocate_exact_size(vec2(SWATCH, SWATCH), Sense::hover());
                        ui.painter().rect_filled(rect, RADIUS, rgb(fighter.color));
                        line(ui, RichText::new(&card.colour_name).font(body()).color(TEXT));
                    });
                });
        });
}

/// The portrait (a 52 disc, or the sprite) and the name in pixel letters, 28 tall when it fits beside it, else the
/// largest whole scale that does.
fn name_row(ui: &mut Ui, fighter: &RosterEntry) {
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), PORTRAIT),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = PORTRAIT_GAP;
            let sprite = fighter.card.as_ref().and_then(|c| c.portrait.as_ref());
            let name = format!("Portrait of {}", fighter.display);
            portrait(ui, fighter.color, sprite, PORTRAIT, &name);
            let scale = fit_scale(fighter.mask.w, NAME_SCALE, ui.available_width());
            pixel_text(ui, &fighter.mask, scale, TEXT, &fighter.display);
        },
    );
}

/// A fact row: `label` in `MUTED`, 92 wide, then what `add` lays out.
fn fact(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), LINE_H),
        Layout::left_to_right(Align::Center),
        |ui| {
            cell(ui, FACT_W, LINE_H, RichText::new(label).font(body()).color(MUTED));
            add(ui);
        },
    );
}

/// The roster: its header, then one block per section with a grid of chips. It scrolls when it does not fit.
fn roster_column(ui: &mut Ui, vm: &ViewModel, roster: &Roster, out: &mut Vec<Action>) {
    ScrollArea::vertical()
        .id_salt("matchup roster")
        .auto_shrink(false)
        .show(ui, |ui| {
            Frame::new()
                .inner_margin(Margin::symmetric(ROSTER_PAD.0, ROSTER_PAD.1))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing = vec2(ITEM_GAP, 0.0);
                    roster_header(ui, roster);
                    for section in SECTIONS {
                        let fighters: Vec<&RosterEntry> =
                            roster.fighters.iter().filter(|f| f.section == section).collect();
                        if fighters.is_empty() {
                            continue;
                        }
                        ui.add_space(ROSTER_GAP);
                        line(ui, RichText::new(section_name(section)).font(small()).color(MUTED));
                        ui.add_space(SECTION_GAP);
                        chip_grid(ui, vm, &fighters, out);
                    }
                });
        });
}

/// "Roster", the count ("30 fighters", or "29 of 30 ready" when some are not built yet) and the hint, which moves to
/// a line of its own when the three do not fit on one.
fn roster_header(ui: &mut Ui, roster: &Roster) {
    let total = roster.fighters.len() as u64;
    let ready = roster.fighters.iter().filter(|f| f.available).count() as u64;
    let count = if ready == total {
        wording::plural(total, "fighter", "fighters")
    } else {
        format!("{} of {} ready", wording::thousands(ready), wording::thousands(total))
    };
    let width = |text: &str, font| ui.painter().layout_no_wrap(text.to_owned(), font, TEXT).size().x;
    let needed = width("Roster", section()) + ITEM_GAP + width(&count, small()) + HINT_GAP + width(HINT, small());
    let one_line = needed <= ui.available_width();
    ui.horizontal(|ui| {
        line(ui, RichText::new("Roster").font(section()).color(TEXT));
        line(ui, RichText::new(&count).font(small()).color(MUTED));
        if one_line {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                line(ui, RichText::new(HINT).font(small()).color(MUTED));
            });
        }
    });
    if !one_line {
        ui.add_space(SECTION_GAP);
        line(ui, RichText::new(HINT).font(small()).color(MUTED));
    }
}

/// The chips of one section, as many columns as fit (2 to 4), 6 apart side by side and 3 between rows.
fn chip_grid(ui: &mut Ui, vm: &ViewModel, fighters: &[&RosterEntry], out: &mut Vec<Action>) {
    let width = ui.available_width();
    let columns = columns_for(width);
    let chip_w = ((width - CHIP_GAP_X * (columns as f32 - 1.0)) / columns as f32).floor();
    for (i, row) in fighters.chunks(columns).enumerate() {
        if i > 0 {
            ui.add_space(CHIP_GAP_Y);
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = CHIP_GAP_X;
            for fighter in row {
                let side = vm.draft.fighters.iter().position(|name| name == &fighter.name);
                let got = roster_chip(ui, &fighter.display, fighter.color, side, fighter.available, chip_w);
                for (picked, corner) in [(got.left, 0), (got.right, 1)] {
                    if picked {
                        out.push(Action::PickFighter {
                            side: corner,
                            name: fighter.name.clone(),
                        });
                    }
                }
            }
        });
    }
}

/// The tempo of `bars` bars, by the draft's own rule.
fn tempo_of(draft: &Draft, bars: u32) -> f64 {
    Draft { bars, ..draft.clone() }.tempo_bpm()
}

/// The bar count in `range` whose tempo is nearest `bpm` (the lower one on a tie).
fn nearest_bars(draft: &Draft, range: (u32, u32), bpm: f64) -> u32 {
    (range.0..=range.1)
        .min_by(|&a, &b| {
            let (da, db) = ((tempo_of(draft, a) - bpm).abs(), (tempo_of(draft, b) - bpm).abs());
            da.total_cmp(&db)
        })
        .unwrap_or(draft.bars)
}

/// A row of the bottom strip, 26 tall and as wide as `ui`, laid out left to right.
fn strip_row(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), CONTROL_H),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_min_size(vec2(ui.available_width(), CONTROL_H));
            add(ui);
        },
    );
}

/// The bottom strip's three rows: the theme, the tempo with the two buttons, the pair rules.
fn bottom_strip(ui: &mut Ui, vm: &ViewModel, roster: &Roster, out: &mut Vec<Action>) {
    ui.spacing_mut().item_spacing = vec2(ITEM_GAP, STRIP_GAP);
    let draft = &vm.draft;
    let theme = vm.theme();

    strip_row(ui, |ui| {
        cell(ui, LABEL_W, CONTROL_H, RichText::new("Theme").font(body()).color(MUTED));
        let options: Vec<String> = roster.themes.iter().map(|t| t.display.clone()).collect();
        let chosen = roster
            .themes
            .iter()
            .position(|t| t.name == draft.theme)
            .unwrap_or(usize::MAX);
        if let Some(i) = buttons::dropdown(ui, "Theme", &options, chosen, THEME_W)
            && let Some(picked) = roster.themes.get(i)
            && picked.name != draft.theme
        {
            out.push(Action::SetTheme(picked.name.clone()));
        }
        if let Some(theme) = theme {
            line(ui, RichText::new(&theme.text).font(small()).color(MUTED));
        }
    });

    strip_row(ui, |ui| {
        // Laid out left to right, so the keyboard reaches the tempo before the buttons; the buttons' room is kept
        // at the right end first.
        let button_w = |label: &str| (text_width(ui, label, body()) + 2.0 * BUTTON_PAD_X).ceil();
        let buttons_w = button_w(SWAP) + ITEM_GAP + button_w(BALANCE);
        let range = theme.map_or((draft.bars, draft.bars), |t| t.bars);
        tempo_controls(ui, draft, range, buttons_w, out);
        ui.add_space((ui.available_width() - buttons_w).max(0.0));
        if buttons::boxed(ui, SWAP).clicked() {
            out.push(Action::SwapSides);
        }
        match &vm.fight_problem {
            Some(problem) => {
                buttons::disabled(ui, BALANCE, problem);
            }
            None => {
                if buttons::primary(ui, BALANCE).clicked() {
                    out.push(Action::Go(View::Step(Step::Balance)));
                }
            }
        }
    });

    strip_row(ui, |ui| {
        cell(
            ui,
            LABEL_W,
            CONTROL_H,
            RichText::new("Pair rules").font(body()).color(MUTED),
        );
        if vm.pair_rules.is_empty() {
            line(ui, RichText::new("None for this pair").font(body()).color(MUTED));
        }
        for rule in &vm.pair_rules {
            line(ui, RichText::new(rule).font(body()).color(TEXT));
        }
    });
}

/// The width of `text` in `font`.
fn text_width(ui: &Ui, text: &str, font: egui::FontId) -> f32 {
    ui.painter().layout_no_wrap(text.to_owned(), font, TEXT).size().x
}

/// "Tempo", the slider over the theme's `bars` range in whole bars, the value field in bpm and the note "{bars} bars
/// fit in 61 seconds", leaving `keep` (and the gap before it) free at the row's right end. The slider is as wide as
/// the row allows, at most 220; the note is cut short when even the narrowest slider leaves it too little room.
fn tempo_controls(ui: &mut Ui, draft: &Draft, range: (u32, u32), keep: f32, out: &mut Vec<Action>) {
    let range = (range.0.min(range.1), range.0.max(range.1));
    let note = format!("{} bars fit in 61 seconds", draft.bars);
    let note_w = text_width(ui, &note, small()).ceil();
    let room = ui.available_width() - keep - ITEM_GAP;
    let fixed = LABEL_W + ITEM_GAP + ITEM_GAP + FIELD_W + ITEM_GAP;
    let slider_w = (room - fixed - note_w - ITEM_GAP)
        .clamp(SLIDER_MIN_W, SLIDER_MAX_W)
        .floor();
    let note_cell = (room - fixed - slider_w - ITEM_GAP).max(0.0);

    cell(ui, LABEL_W, CONTROL_H, RichText::new("Tempo").font(body()).color(MUTED));
    let mut bars = f64::from(draft.bars);
    let moved = bar_slider(
        ui,
        "Tempo",
        &mut bars,
        (f64::from(range.0), f64::from(range.1)),
        1.0,
        slider_w,
    );
    let mut wanted = None;
    if moved.changed {
        wanted = Some(bars.round() as u32);
    }
    let shown = draft.tempo_bpm();
    let mut bpm = shown;
    let ends = (tempo_of(draft, range.0), tempo_of(draft, range.1));
    value_field(ui, TEMPO_FIELD, &mut bpm, ends, BPM, FIELD_W);
    if bpm != shown {
        wanted = Some(nearest_bars(draft, range, bpm));
    }
    if let Some(bars) = wanted
        && bars != draft.bars
    {
        out.push(Action::SetBars(bars));
    }
    cell(ui, note_cell, CONTROL_H, RichText::new(note).font(small()).color(MUTED));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::base;

    #[test]
    fn columns_for_follows_its_rule() {
        assert_eq!(columns_for(498.0), 4);
        assert_eq!(columns_for(338.0), 2);
        assert_eq!(columns_for(2000.0), 4);
        assert_eq!(columns_for(372.0), 3, "(372 + 6) / 126 is exactly 3");
        assert_eq!(columns_for(371.9), 2);
        assert_eq!(columns_for(-50.0), 2);
        assert_eq!(columns_for(f32::NAN), 2);
        assert_eq!(columns_for(f32::INFINITY), 4);
    }

    #[test]
    fn a_typed_tempo_picks_the_nearest_bar_count() {
        let draft = base::vm().draft;
        assert_eq!(nearest_bars(&draft, (24, 42), 125.0), 32);
        assert_eq!(nearest_bars(&draft, (24, 42), draft.tempo_bpm()), 34);
        assert_eq!(nearest_bars(&draft, (24, 42), 300.0), 42);
        assert_eq!(nearest_bars(&draft, (24, 42), 1.0), 24);
        assert_eq!(nearest_bars(&draft, (34, 34), 1.0), 34);
    }
}
