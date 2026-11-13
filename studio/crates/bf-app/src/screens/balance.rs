//! The Balance step's workspace (interface reference, section 5.2): one column 780 wide, centred, 28 above and below,
//! 22 between its five blocks.
//!
//! 1. The win split: who wins how often, a bar split at the left fighter's share with marks at 40 and 60 %, and
//!    under it whether that is in range. While a balance runs the bar is a progress bar; with no numbers and nothing
//!    running it is a line saying what to do.
//! 2. The damage handicap: "Reset both to ×1.00", "Auto-balance" (or "Stop balancing" while one runs) and a slider
//!    and value field for each fighter. The sliders run from 0.05 to 3.00 in steps of 0.01 (a handicap above 3 pins
//!    the knob at the end); the value fields take 0.05 to 12.00. While a balance runs both rows and the reset wait.
//! 3. Four numbers: the typical K.O., lead changes, fights with no K.O. by 58 s, the share that fits the beat sheet.
//! 4. When the K.O. lands: 28 bars for the seconds 30 to 57, the beat sheet's window (49 to 52) lighter. The
//!    histogram is 120 tall and gives up height, down to 60, when the workspace is short.
//! 5. The way on: "Search seeds".
//!
//! It is a pure function of the view model: it keeps nothing of its own beyond egui's widget memory (a value field
//! remembers that it was changed while it had the keyboard, so letting go of it releases the change once).

use egui::accesskit::Role;
use egui::text::{LayoutJob, TextWrapping};
use egui::{
    Align, Color32, FontId, Label, Layout, Painter, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, UiBuilder,
    WidgetInfo, pos2, vec2,
};

use crate::model::balance::BalanceStats;
use crate::model::{Step, View};
use crate::state::{Action, ViewModel};
use crate::theme::{
    BAR_DIM, CONTROL_H, FAINT, LINE, LINE_SOFT, MUTED, RADIUS, TEXT, TEXT_STRONG, VALUE_W, body, large_fill, rgb,
    section, small, title, value,
};
use crate::widgets::buttons;
use crate::widgets::chips::fighter_bar;
use crate::widgets::fields::{self, Unit};
use crate::widgets::slider::bar_slider;
use crate::wording;

/// The win split bar's accessible name.
pub const WIN_SPLIT: &str = "Win split";
/// The histogram's accessible name.
pub const HISTOGRAM: &str = "K.O. times, second by second";
/// The progress bar's accessible name while a balance runs.
pub const PROGRESS: &str = "Balance progress";
/// What a number reads when there is none.
pub const NO_VALUE: &str = "–";
/// In place of the bar with no numbers and nothing running.
pub const NOT_MEASURED: &str = "Not measured yet. Auto-balance finds the handicap for you.";
/// Under the two rows.
pub const NOTE: &str =
    "Damage is the only thing you can change here. Reach, timing and abilities stay exactly as each fighter was built.";
/// Beside "Search seeds".
pub const FOOTER: &str = "Changing the handicap gives a new set of fights, so the search starts over.";
/// What hovering the reset says while a balance runs.
pub const WAIT: &str = "Wait for the balance run to finish, or stop it.";
/// The reset button's label.
const RESET: &str = "Reset both to ×1.00";
/// The button that goes back to a kit pair's own handicap.
pub const USE_SERIES: &str = "Use the series' handicap";

/// The line under the handicap rows for a kit pair: "The series' handicap for this match-up: ×0.86 and ×1.00."
pub fn series_words(series: [f64; 2]) -> String {
    format!(
        "The series' handicap for this match-up: {} and {}.",
        wording::times(series[0]),
        wording::times(series[1])
    )
}

/// The column's width and the padding above and below it.
const COLUMN_W: f32 = 780.0;
const PAD_Y: f32 = 28.0;
/// The gap between blocks.
const BLOCK_GAP: f32 = 22.0;
/// Heights: a line of `section` or `body`, of `small`, and of `title`.
const LINE_H: f32 = 18.0;
const SMALL_H: f32 = 16.0;
const TITLE_H: f32 = 24.0;
/// The win split: the gap under its row and under the bar, the bar, how far the marks reach past it.
const SPLIT_GAP: f32 = 8.0;
const BAR_H: f32 = 30.0;
const MARK_REACH: f32 = 4.0;
/// The bar's top edge in each fighter's full colour.
const EDGE_H: f32 = 2.0;
/// The win range's ends, as `BalanceStats::in_range()` has them, and where the bar marks them.
const WIN_RANGE: (f64, f64) = (0.40, 0.60);
const LOW: f32 = WIN_RANGE.0 as f32;
const HIGH: f32 = WIN_RANGE.1 as f32;
/// The handicap block: the padding under its top line, the gap between its rows, the fighter bar, the label column.
const HANDICAP_PAD: f32 = 16.0;
const ROW_GAP: f32 = 6.0;
const FIGHTER_BAR_H: f32 = 18.0;
const NAME_W: f32 = 120.0;
/// The gap between a label, its slider and its value; the gap between two small controls.
const GAP: f32 = 8.0;
const SMALL_GAP: f32 = 4.0;
/// The sliders' range and step, and the value fields' range.
const SLIDER_RANGE: (f64, f64) = (0.05, 3.0);
const SLIDER_STEP: f64 = 0.01;
const FIELD_RANGE: (f64, f64) = (0.05, 12.0);
/// The four numbers' box: cell padding (x, y), and room for a note of up to two lines (a quarter of 780 is too
/// narrow for "Most land between 44.2 and 56.9 s" on one).
const CELL_PAD: (f32, f32) = (12.0, 10.0);
const NOTE_ROWS: usize = 2;
const BOX_H: f32 = 2.0 * CELL_PAD.1 + SMALL_H + TITLE_H + NOTE_ROWS as f32 * SMALL_H;
/// The histogram: its tallest and shortest, the gap between bars, the first second and the bar count, the window,
/// the gap under the title row and above the labels.
const HIST_MAX: f32 = 120.0;
const HIST_MIN: f32 = 60.0;
const HIST_GAP: f32 = 2.0;
const FIRST_SECOND: u32 = 30;
const BARS: usize = 28;
const WINDOW: (u32, u32) = (49, 52);
const AXIS_GAP: f32 = 4.0;
/// A button's side padding (as `buttons` lays it out).
const BUTTON_PAD: f32 = 10.0;

/// Each block's height, the histogram's excepted.
const SPLIT_BLOCK: f32 = LINE_H + SPLIT_GAP + BAR_H + SPLIT_GAP + SMALL_H;
const HANDICAP_BLOCK: f32 = 1.0 + HANDICAP_PAD + 3.0 * CONTROL_H + 2.0 * ROW_GAP + SMALL_H;
const KO_BLOCK_REST: f32 = LINE_H + SPLIT_GAP + 1.0 + AXIS_GAP + SMALL_H;
/// Everything in the column but the histogram.
const FIXED_H: f32 = 2.0 * PAD_Y + SPLIT_BLOCK + HANDICAP_BLOCK + BOX_H + KO_BLOCK_REST + CONTROL_H + 4.0 * BLOCK_GAP;

/// Draws it into `ui` from `vm`, pushing what the owner does to `out`.
pub fn show(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let area = ui.max_rect();
    let w = COLUMN_W.min(area.width()).max(0.0).floor();
    let left = (area.center().x - w / 2.0).round();
    let hist_h = (area.height() - FIXED_H).clamp(HIST_MIN, HIST_MAX).floor();
    let column = Rect::from_min_max(
        pos2(left, area.min.y + PAD_Y),
        pos2(left + w, (area.max.y - PAD_Y).max(area.min.y + PAD_Y)),
    );
    let mut col = ui.new_child(UiBuilder::new().max_rect(column).layout(Layout::top_down(Align::Min)));
    col.spacing_mut().item_spacing = vec2(GAP, 0.0);
    let col = &mut col;
    let stats = vm.balance.stats.as_ref();

    win_split(&mut block(col, "win split", w, SPLIT_BLOCK), vm, w);
    col.add_space(BLOCK_GAP);
    handicap(&mut block(col, "handicap", w, HANDICAP_BLOCK), vm, w, out);
    col.add_space(BLOCK_GAP);
    numbers(&mut block(col, "numbers", w, BOX_H), stats, w);
    col.add_space(BLOCK_GAP);
    ko_times(&mut block(col, "ko times", w, KO_BLOCK_REST + hist_h), stats, w, hist_h);
    col.add_space(BLOCK_GAP);
    footer(&mut block(col, "footer", w, CONTROL_H), w, out);
}

/// The next `h` of the column, `w` wide, as a top-down `Ui` whose ids come from `salt` alone, so what one block
/// shows (numbers or none, a run or none) never changes the ids of the controls in another, and a drag goes on
/// when the numbers above it change.
fn block(col: &mut Ui, salt: &str, w: f32, h: f32) -> Ui {
    let rect = take(col, w, h);
    let id = col.scope_id().with(("balance", salt));
    let mut block = col.new_child(
        UiBuilder::new()
            .scope_id(id)
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
    );
    block.spacing_mut().item_spacing = vec2(GAP, 0.0);
    block
}

/// "About 1 fight in {n}" with n the fit rate's inverse, rounded; "None in this sample" for 0; the "no value"
/// wording for a rate that is not a finite number of at least 0.
pub fn one_in(fit_rate: f64) -> String {
    if !fit_rate.is_finite() || fit_rate < 0.0 {
        return NO_VALUE.to_string();
    }
    if fit_rate == 0.0 {
        return "None in this sample".to_string();
    }
    let n = (1.0 / fit_rate).round().clamp(1.0, 1e15) as u64;
    format!("About 1 fight in {}", wording::thousands(n))
}

/// Whether `x` can be shown: a finite number of at least 0 (widget rule 9).
fn usable(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}

/// `x` written by `write`, or the "no value" wording.
fn shown(x: f64, write: impl Fn(f64) -> String) -> String {
    if usable(x) { write(x) } else { NO_VALUE.to_string() }
}

/// A share as a percentage, or the "no value" wording for one that is not between 0 and 1.
fn share(x: f64) -> String {
    if usable(x) && x <= 1.0 {
        wording::percent(x)
    } else {
        NO_VALUE.to_string()
    }
}

/// The left fighter's share of wins, when it is a usable share.
fn left_share(stats: &BalanceStats) -> Option<f64> {
    (usable(stats.win_share) && stats.win_share <= 1.0).then_some(stats.win_share)
}

/// A child of `ui` filling `rect`, laid out with `layout`, with no gaps between its items.
fn child(ui: &mut Ui, rect: Rect, layout: Layout) -> Ui {
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect).layout(layout));
    child.spacing_mut().item_spacing = vec2(GAP, 0.0);
    child
}

/// Text that is read, not clicked or selected, cut short with an ellipsis when it does not fit.
fn text(ui: &mut Ui, words: impl Into<String>, font: FontId, colour: Color32) -> Response {
    ui.add(
        Label::new(RichText::new(words).font(font).color(colour))
            .selectable(false)
            .truncate(),
    )
}

/// `words` in `rect` with `align` (left, centre or right), one line, cut short when it does not fit.
fn text_in(ui: &mut Ui, rect: Rect, align: Align, words: impl Into<String>, font: FontId, colour: Color32) {
    let mut cell = child(ui, rect, Layout::top_down(align));
    text(&mut cell, words, font, colour);
}

/// The width of `words` in `font`.
fn width_of(ui: &Ui, words: &str, font: FontId) -> f32 {
    ui.painter().layout_no_wrap(words.to_owned(), font, TEXT).size().x
}

/// The width `buttons` gives a button labelled `label`.
fn button_w(ui: &Ui, label: &str) -> f32 {
    (width_of(ui, label, body()) + 2.0 * BUTTON_PAD).ceil()
}

/// Takes the next `h` of the column, `w` wide.
fn take(ui: &mut Ui, w: f32, h: f32) -> Rect {
    ui.allocate_exact_size(vec2(w, h), Sense::hover()).0
}

/// A rectangle from `x0` to `x1` and `y0` to `y1`, its edges on whole points.
fn span(x0: f32, x1: f32, y0: f32, y1: f32) -> Rect {
    Rect::from_min_max(pos2(x0.round(), y0.round()), pos2(x1.round(), y1.round()))
}

// ---------------------------------------------------------------------------------------------------------------
// 1. The win split
// ---------------------------------------------------------------------------------------------------------------

fn win_split(ui: &mut Ui, vm: &ViewModel, w: f32) {
    let [left_name, right_name] = vm.names();
    let stats = vm.balance.stats.as_ref();
    let shares = stats.and_then(left_share);
    let (left_words, right_words) = match shares {
        // Both from the left share rounded once, so the two always add up to 100.
        Some(s) => {
            let left = (s * 100.0).round();
            (
                format!("{left_name} wins {}", wording::percent(left / 100.0)),
                format!("{right_name} wins {}", wording::percent((100.0 - left) / 100.0)),
            )
        }
        None => (
            format!("{left_name} wins {NO_VALUE}"),
            format!("{right_name} wins {NO_VALUE}"),
        ),
    };

    // The row: the left share, the fights in the middle, the right share.
    let row = take(ui, w, LINE_H);
    let centre = row.center().x;
    text_in(
        ui,
        span(row.min.x, centre, row.min.y, row.max.y),
        Align::Min,
        left_words.clone(),
        section(),
        TEXT,
    );
    text_in(
        ui,
        span(centre, row.max.x, row.min.y, row.max.y),
        Align::Max,
        right_words.clone(),
        section(),
        TEXT,
    );
    if let Some(stats) = stats {
        let room = (centre - row.min.x - width_of(ui, &left_words, section()))
            .min(row.max.x - centre - width_of(ui, &right_words, section()))
            - GAP;
        if room > 0.0 {
            let fights = wording::plural(u64::from(stats.fights), "fight", "fights");
            let words = format!("{fights}, on seeds the search never uses");
            text_in(
                ui,
                span(centre - room, centre + room, row.min.y, row.max.y),
                Align::Center,
                words,
                small(),
                MUTED,
            );
        }
    }
    ui.add_space(SPLIT_GAP);

    let (bar, bar_response) = ui.allocate_exact_size(vec2(w, BAR_H), Sense::hover());
    ui.add_space(SPLIT_GAP);
    let under = take(ui, w, SMALL_H);
    let at = |share: f32| (bar.min.x + share * bar.width()).round();

    if let Some((done, total)) = vm.balance.running {
        let mut cell = child(ui, bar, Layout::top_down(Align::Min));
        cell.add_space(((BAR_H - 4.0) / 2.0).floor());
        let progress = if total > 0 {
            done.min(total) as f32 / total as f32
        } else {
            0.0
        };
        buttons::progress(&mut cell, PROGRESS, progress, w);
        let words = if total > 0 {
            format!("Trying handicaps, step {done} of {total}")
        } else {
            "Trying handicaps".to_string()
        };
        text_in(ui, under, Align::Center, words, small(), TEXT);
        return;
    }
    let (Some(stats), Some(left_share)) = (stats, shares) else {
        let mut cell = child(ui, bar, Layout::top_down(Align::Center));
        cell.add_space(((BAR_H - LINE_H) / 2.0).floor());
        text(&mut cell, NOT_MEASURED, body(), MUTED);
        return;
    };

    // The bar, named for a screen reader (the shares themselves are the row's labels above it).
    bar_response.widget_info(|| WidgetInfo::labeled(Role::Image, true, WIN_SPLIT));
    if ui.is_rect_visible(bar) {
        let painter = ui.painter();
        let colours = vm.colors();
        let split = at(left_share as f32);
        let parts = [
            span(bar.min.x, split, bar.min.y, bar.max.y),
            span(split, bar.max.x, bar.min.y, bar.max.y),
        ];
        for (part, colour) in parts.into_iter().zip(colours) {
            if part.width() > 0.0 {
                painter.rect_filled(part, 0.0, large_fill(colour));
                let edge = Rect::from_min_max(part.min, pos2(part.max.x, part.min.y + EDGE_H));
                painter.rect_filled(edge, 0.0, rgb(colour));
            }
        }
        for mark in [LOW, HIGH] {
            let x = at(mark);
            let line = span(x, x + 1.0, bar.min.y - MARK_REACH, bar.max.y + MARK_REACH);
            painter.rect_filled(line, 0.0, TEXT_STRONG);
        }
    }

    // Under it: the two ends of the range on their marks, and the verdict between them.
    let label_w = (HIGH - LOW) * w / 2.0;
    for (mark, words) in [(LOW, "40%"), (HIGH, "60%")] {
        let x = at(mark);
        text_in(
            ui,
            span(x - label_w / 2.0, x + label_w / 2.0, under.min.y, under.max.y),
            Align::Center,
            words,
            small(),
            MUTED,
        );
    }
    // `in_range()` decides; outside it the left fighter is above 60 % or the right one is.
    let verdict = if stats.in_range() {
        "In range".to_string()
    } else if left_share > WIN_RANGE.1 {
        format!("{left_name} wins too often")
    } else {
        format!("{right_name} wins too often")
    };
    {
        let half = label_w - width_of(ui, "40%", small()) / 2.0 - SMALL_GAP;
        let x = at(0.5);
        text_in(
            ui,
            span(x - half, x + half, under.min.y, under.max.y),
            Align::Center,
            verdict,
            small(),
            TEXT,
        );
    }
}

// ---------------------------------------------------------------------------------------------------------------
// 2. The damage handicap
// ---------------------------------------------------------------------------------------------------------------

fn handicap(ui: &mut Ui, vm: &ViewModel, w: f32, out: &mut Vec<Action>) {
    let running = vm.balance.running.is_some();
    let line = take(ui, w, 1.0);
    ui.painter()
        .hline(line.x_range(), line.min.y + 0.5, Stroke::new(1.0, LINE_SOFT));
    ui.add_space(HANDICAP_PAD);

    // The heading, then the reset and the balance button at the right.
    let row = take(ui, w, CONTROL_H);
    let balance_label = if running { "Stop balancing" } else { "Auto-balance" };
    let buttons_w = button_w(ui, RESET) + SMALL_GAP + button_w(ui, balance_label);
    let heading_rect = span(row.min.x, row.max.x - buttons_w - GAP, row.min.y, row.max.y);
    let mut heading = child(ui, heading_rect, Layout::left_to_right(Align::Center));
    text(&mut heading, "Damage handicap", section(), TEXT);
    let mut actions = child(
        ui,
        span(row.max.x - buttons_w, row.max.x, row.min.y, row.max.y),
        Layout::left_to_right(Align::Center),
    );
    actions.spacing_mut().item_spacing.x = SMALL_GAP;
    if running {
        buttons::disabled(&mut actions, RESET, WAIT);
        if buttons::boxed(&mut actions, "Stop balancing").clicked() {
            out.push(Action::StopBalance);
        }
    } else {
        if buttons::boxed(&mut actions, RESET).clicked() {
            out.push(Action::ResetHandicap);
        }
        match &vm.fight_problem {
            Some(problem) => {
                buttons::disabled(&mut actions, "Auto-balance", problem);
            }
            None => {
                if buttons::primary(&mut actions, "Auto-balance").clicked() {
                    out.push(Action::AutoBalance);
                }
            }
        }
    }

    let names = vm.names();
    let colours = vm.colors();
    ui.add_space(ROW_GAP);
    // The two setting rows touch, 26 tall each.
    for side in 0..2 {
        let row = take(ui, w, CONTROL_H);
        // Each row's ids come from its side alone, whatever the buttons above it are showing.
        let builder = UiBuilder::new()
            .scope_id(ui.scope_id().with(("handicap row", side)))
            .max_rect(row)
            .layout(Layout::left_to_right(Align::Center));
        let mut cells = ui.new_child(if running { builder.disabled() } else { builder });
        cells.spacing_mut().item_spacing = vec2(GAP, 0.0);
        handicap_row(
            &mut cells,
            side,
            &names[side],
            colours[side],
            vm.draft.handicap[side],
            out,
        );
    }
    ui.add_space(ROW_GAP);
    let note = take(ui, w, SMALL_H);
    text_in(ui, note, Align::Min, NOTE, small(), MUTED);
    // a kit pair: its series handicap, and the way back to it
    let series = vm.roster.as_ref().and_then(|r| r.series_handicap(&vm.draft.fighters));
    if let Some(series) = series {
        ui.add_space(ROW_GAP);
        let row = take(ui, w, CONTROL_H);
        let back = vm.draft.handicap != series && !running;
        let button = if back { button_w(ui, USE_SERIES) } else { 0.0 };
        let words = span(row.min.x, row.max.x - button - GAP, row.min.y, row.max.y);
        text_in(ui, words, Align::Min, series_words(series), small(), MUTED);
        if back {
            let mut cell = child(
                ui,
                span(row.max.x - button, row.max.x, row.min.y, row.max.y),
                Layout::left_to_right(Align::Center),
            );
            if buttons::boxed(&mut cell, USE_SERIES).clicked() {
                out.push(Action::UseSeriesHandicap);
            }
        }
    }
}

/// One fighter's row: its bar, "{Name} damage", a slider and a value field, in a `ui` that is disabled while a
/// balance runs.
fn handicap_row(ui: &mut Ui, side: usize, name: &str, colour: [u8; 3], handicap: f64, out: &mut Vec<Action>) {
    fighter_bar(ui, colour, FIGHTER_BAR_H);
    let label = format!("{name} damage");
    let (cell, _) = ui.allocate_exact_size(vec2(NAME_W, CONTROL_H), Sense::hover());
    // The label is painted, not a widget: the slider carries the name, so a screen reader hears it once.
    let mut painter = ui.painter().clone();
    painter.set_opacity(1.0);
    paint_line(&painter, cell, &label, if ui.is_enabled() { MUTED } else { FAINT });

    let slider_w = ui.available_width() - VALUE_W - GAP;
    let mut moved = handicap;
    let slider = bar_slider(ui, &label, &mut moved, SLIDER_RANGE, SLIDER_STEP, slider_w);
    if slider.changed || slider.released {
        out.push(Action::SetHandicap {
            side,
            value: moved,
            released: slider.released,
        });
    }

    let mut typed = handicap;
    let unit = Unit {
        prefix: "×",
        suffix: "",
        decimals: 2,
    };
    let field = fields::value_field(ui, &format!("{label} value"), &mut typed, FIELD_RANGE, unit, VALUE_W);
    // A change while the field is dragged or has the keyboard waits; letting go of it releases it, once.
    let pending = field.id.with("changed while held");
    let held = field.dragged() || field.has_focus();
    let mut was_pending = ui.data(|d| d.get_temp::<bool>(pending)).unwrap_or(false);
    if typed.to_bits() != handicap.to_bits() {
        out.push(Action::SetHandicap {
            side,
            value: typed,
            released: !held,
        });
        was_pending = held;
    } else if was_pending && !held {
        out.push(Action::SetHandicap {
            side,
            value: typed,
            released: true,
        });
        was_pending = false;
    }
    ui.data_mut(|d| d.insert_temp(pending, was_pending));
}

/// `words` on one line at the left of `rect`, in its middle, in body `colour`, ending in an ellipsis when it does
/// not fit.
fn paint_line(painter: &Painter, rect: Rect, words: &str, colour: Color32) {
    let mut job = LayoutJob::simple_singleline(words.to_owned(), body(), colour);
    job.wrap = TextWrapping {
        max_width: rect.width(),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = painter.layout_job(job);
    let at = pos2(rect.min.x, rect.center().y - galley.size().y / 2.0).round();
    painter.with_clip_rect(rect).galley(at, galley, colour);
}

// ---------------------------------------------------------------------------------------------------------------
// 3. The four numbers
// ---------------------------------------------------------------------------------------------------------------

fn numbers(ui: &mut Ui, stats: Option<&BalanceStats>, w: f32) {
    let frame = take(ui, w, BOX_H);
    let cells: [(&str, String, String); 4] = match stats {
        Some(s) => {
            // Both ends written by `wording::seconds`, the unit said once; ends that cannot be shown, or that are the
            // wrong way round, read as no value.
            let range = if usable(s.ko_p10) && usable(s.ko_p90) && s.ko_p10 <= s.ko_p90 {
                let low = wording::seconds(s.ko_p10);
                let low = low.strip_suffix(" s").unwrap_or(&low);
                format!("Most land between {low} and {}", wording::seconds(s.ko_p90))
            } else {
                NO_VALUE.to_string()
            };
            [
                ("Typical K.O.", shown(s.ko_median, wording::seconds), range),
                (
                    "Lead changes",
                    shown(s.lead_changes_mean, |x| format!("{x:.1}")),
                    "On average per fight".to_string(),
                ),
                (
                    "No K.O. by 58 s",
                    share(s.no_ko_share),
                    "These never reach the shortlist".to_string(),
                ),
                ("Fit the beat sheet", share(s.fit_rate), one_in(s.fit_rate)),
            ]
        }
        None => [
            ("Typical K.O.", NO_VALUE.to_string(), String::new()),
            ("Lead changes", NO_VALUE.to_string(), "On average per fight".to_string()),
            (
                "No K.O. by 58 s",
                NO_VALUE.to_string(),
                "These never reach the shortlist".to_string(),
            ),
            ("Fit the beat sheet", NO_VALUE.to_string(), String::new()),
        ],
    };
    let cell_w = w / cells.len() as f32;
    let painter = ui.painter().clone();
    for (i, (heading, number, note)) in cells.into_iter().enumerate() {
        let x0 = frame.min.x + i as f32 * cell_w;
        let cell = span(x0, x0 + cell_w, frame.min.y, frame.max.y);
        if i > 0 {
            painter.vline(cell.min.x + 0.5, frame.y_range(), Stroke::new(1.0, LINE_SOFT));
        }
        let inner = cell.shrink2(vec2(CELL_PAD.0, CELL_PAD.1));
        let mut content = child(ui, inner, Layout::top_down(Align::Min));
        line_of(&mut content, heading, small(), MUTED, SMALL_H);
        line_of(&mut content, number, title(), TEXT, TITLE_H);
        if !note.is_empty() {
            // Up to two lines, the second ending in an ellipsis if even that is too short.
            let mut job = LayoutJob::simple(note, small(), MUTED, inner.width());
            job.wrap.max_rows = NOTE_ROWS;
            job.wrap.overflow_character = Some('…');
            content.add(Label::new(job).selectable(false).wrap());
        }
    }
    painter.rect_stroke(frame, RADIUS, Stroke::new(1.0, LINE_SOFT), StrokeKind::Inside);
}

/// One line of `words` taking exactly `h` of a top-down `ui`.
fn line_of(ui: &mut Ui, words: impl Into<String>, font: FontId, colour: Color32, h: f32) {
    let w = ui.available_width();
    let rect = take(ui, w, h);
    let mut cell = child(ui, rect, Layout::left_to_right(Align::Center));
    text(&mut cell, words, font, colour);
}

// ---------------------------------------------------------------------------------------------------------------
// 4. When the K.O. lands
// ---------------------------------------------------------------------------------------------------------------

fn ko_times(ui: &mut Ui, stats: Option<&BalanceStats>, w: f32, hist_h: f32) {
    let row = take(ui, w, LINE_H);
    let mut heading = child(ui, row, Layout::left_to_right(Align::Center));
    text(&mut heading, "When the K.O. lands", section(), TEXT);
    heading.add_space(2.0);
    text(
        &mut heading,
        "Lighter bars are inside the beat sheet's window, 49 to 53 s",
        small(),
        MUTED,
    );
    ui.add_space(SPLIT_GAP);

    let (rect, response) = ui.allocate_exact_size(vec2(w, hist_h), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Image, true, HISTOGRAM));
    let line = take(ui, w, 1.0);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.hline(line.x_range(), line.min.y + 0.5, Stroke::new(1.0, LINE));
        let counts: Vec<u32> = (0..BARS)
            .map(|i| stats.and_then(|s| s.ko_hist.get(i).copied()).unwrap_or(0))
            .collect();
        let most = counts.iter().copied().max().unwrap_or(0);
        let bar_w = (w - (BARS as f32 - 1.0) * HIST_GAP) / BARS as f32;
        if most > 0 {
            for (i, &count) in counts.iter().enumerate() {
                // A second with any K.O. at all shows at least 1 px.
                if count == 0 {
                    continue;
                }
                let height = (count as f32 / most as f32 * hist_h).round().max(1.0);
                let x0 = rect.min.x + i as f32 * (bar_w + HIST_GAP);
                let second = FIRST_SECOND + i as u32;
                let colour = if (WINDOW.0..=WINDOW.1).contains(&second) {
                    TEXT
                } else {
                    BAR_DIM
                };
                painter.rect_filled(span(x0, x0 + bar_w, rect.max.y - height, rect.max.y), 0.0, colour);
            }
        }
    }
    ui.add_space(AXIS_GAP);

    // Five labels spread evenly under it: the first at its left edge, the last at its right.
    let axis = take(ui, w, SMALL_H);
    let labels = ["30 s", "37 s", "44 s", "51 s", "58 s"];
    let step = w / (labels.len() - 1) as f32;
    for (i, words) in labels.into_iter().enumerate() {
        let x = axis.min.x + i as f32 * step;
        let (rect, align) = match i {
            0 => (span(x, x + step / 2.0, axis.min.y, axis.max.y), Align::Min),
            4 => (span(x - step / 2.0, x, axis.min.y, axis.max.y), Align::Max),
            _ => (
                span(x - step / 2.0, x + step / 2.0, axis.min.y, axis.max.y),
                Align::Center,
            ),
        };
        text_in(ui, rect, align, words, value(), MUTED);
    }
}

// ---------------------------------------------------------------------------------------------------------------
// 5. The way on
// ---------------------------------------------------------------------------------------------------------------

fn footer(ui: &mut Ui, w: f32, out: &mut Vec<Action>) {
    let row = take(ui, w, CONTROL_H);
    let search_w = button_w(ui, "Search seeds");
    let note = span(row.min.x, row.max.x - search_w - GAP, row.min.y, row.max.y);
    let mut cell = child(ui, note, Layout::right_to_left(Align::Center));
    text(&mut cell, FOOTER, small(), MUTED);
    let mut button = child(
        ui,
        span(row.max.x - search_w, row.max.x, row.min.y, row.max.y),
        Layout::left_to_right(Align::Center),
    );
    if buttons::primary(&mut button, "Search seeds").clicked() {
        out.push(Action::Go(View::Step(Step::Seeds)));
    }
}
