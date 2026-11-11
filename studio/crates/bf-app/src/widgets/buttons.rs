//! The basic controls (interface reference, section 3): flat, toggle, box, primary and disabled buttons, the
//! segmented control, the switch, the dropdown and the progress bar.
//!
//! Each is painted here rather than left to egui's own widget look, because the theme cannot give it: the theme's
//! `override_text_color` would draw every label in `TEXT` (invisible on the inverted fill), egui's focus outline is
//! invisible in these colours, and egui fades a disabled control instead of drawing it `FAINT`. So every control
//! sets its label colour for each state (`ON_LIGHT` on the inverted fill), paints its own focus ring (2 px
//! `TEXT_STRONG`, 2 px outside) and names itself for the accessibility tree.
//!
//! What every control here keeps to (widget rule 17):
//! - The focus ring is painted with [`widget_ring`], on a painter whose clip rectangle reaches 4 px past the `Ui`'s,
//!   so a control at a panel's or a scroll area's edge shows all of its ring.
//! - Inside a disabled parent (`ui.disable()`, `add_enabled_ui(false, ..)`) a control takes no input and draws
//!   itself `FAINT` at full strength, on a painter egui's disabled fade does not touch.
//! - A label too long for its control ends in an ellipsis and is never drawn past the control. A button is as wide
//!   as its label, but never wider than the room its `Ui` has left.
//! - A control that keeps state by its id (the focus of a segment, a dropdown's open list) takes a persistent id from
//!   its name and the number of earlier controls of that name in the same `Ui` this pass ([`named_id`]), so it
//!   neither loses that state when controls above it come and go nor clashes with a second control of its name.
//!   The buttons, the switch and the icon button take egui's automatic ids, as egui's own buttons do.

use std::ops::Range;

use egui::accesskit::Role;
use egui::scroll_area::{ScrollAreaOutput, ScrollBarVisibility};
use egui::style::{ScrollFadeStyle, ScrollStyle};
use egui::text::{LayoutJob, TextWrapping};
use egui::{
    Color32, ComboBox, CornerRadius, Id, IdSalt, Painter, Rect, Response, RichText, ScrollArea, Sense, Stroke,
    StrokeKind, Style, Ui, UiBuilder, Visuals, WidgetInfo, pos2, vec2,
};

use super::icons::{self, Icon};
use crate::theme::{
    CONTROL_H, FAINT, GROUND, HOVER, LINE, MUTED, ON_LIGHT, RADIUS, RAISED, TEXT, TEXT_STRONG, VALUE_H, body,
};

/// Side padding of a button's label.
const PAD_X: f32 = 10.0;
/// Side padding of a dropdown's value and of a menu item's label.
const MENU_PAD_X: f32 = 8.0;
/// A segment's height.
const SEGMENT_H: f32 = 24.0;
/// A menu item's height.
const ITEM_H: f32 = 24.0;
/// The switch's outline.
const SWITCH_W: f32 = 30.0;
const SWITCH_H: f32 = 16.0;
/// The switch's knob, and its distance from the outline's outer edge (1 px outline, then 2).
const KNOB: f32 = 10.0;
const KNOB_INSET: f32 = 3.0;
/// The gap between the switch and its label.
const SWITCH_GAP: f32 = 8.0;
/// The progress bar's height.
const PROGRESS_H: f32 = 4.0;
/// The focus ring: its width and its gap from the control; together the room it takes outside the control.
const RING_W: f32 = 2.0;
const RING_GAP: f32 = 2.0;
const RING_ROOM: f32 = RING_W + RING_GAP;
/// A scroll area's bar: 6 wide, 4 from the content and 4 from the area's edge.
const SCROLL_BAR_W: f32 = 6.0;
const SCROLL_BAR_MARGIN: f32 = 4.0;
/// The width a scroll area's bar takes from its content while it shows ([`scrolled`]).
pub const SCROLL_BAR_ROOM: f32 = SCROLL_BAR_W + 2.0 * SCROLL_BAR_MARGIN;

/// What a button looks like in its present state.
struct Look {
    fill: Color32,
    border: Option<Color32>,
    label: Color32,
}

/// The keyboard focus ring with `painter`: 2 px `TEXT_STRONG`, 2 px outside `rect`, corner 2 + 2. It is cut by the
/// painter's clip rectangle; a widget paints its ring with [`widget_ring`], which is not.
pub(crate) fn focus_ring(painter: &Painter, rect: Rect) {
    ring_with(painter, rect, f32::from(RADIUS));
}

/// The ring around `rect` with corners `corner` + 2, on `painter`.
fn ring_with(painter: &Painter, rect: Rect, corner: f32) {
    painter.rect_stroke(
        rect.expand(RING_GAP),
        corner + RING_GAP,
        Stroke::new(RING_W, TEXT_STRONG),
        StrokeKind::Outside,
    );
}

/// The one focus ring every widget paints: 2 px `TEXT_STRONG`, 2 px outside `rect`, its corners `corner` + 2, on
/// `ui`'s layer with a clip rectangle 4 px wider than `ui`'s, so a control at the edge of a panel or a scroll area
/// shows all of its ring. (The text fields paint the same ring on a layer just above, see `fields::ring_above`.)
pub(crate) fn widget_ring(ui: &Ui, rect: Rect, corner: f32) {
    let painter = Painter::new(ui.ctx().clone(), ui.layer_id(), ui.clip_rect().expand(RING_ROOM));
    ring_with(&painter, rect, corner);
}

/// A persistent id for a control of `kind` named `name` in `ui`: made from `ui`'s scope id, the kind, the name and
/// how many controls of that kind and name `ui` has already placed in this pass (0 for the first). The count is kept
/// in egui's temporary data and starts again at 0 each pass, so the id stays the same from frame to frame while
/// controls of other names come and go around it.
pub(crate) fn named_id(ui: &Ui, kind: &str, name: &str) -> Id {
    let base = ui.make_persistent_id((kind, name));
    let pass = ui.ctx().cumulative_pass_nr();
    let count = ui.ctx().data_mut(|data| {
        let seen = data.get_temp_mut_or_insert_with(base.with("seen this pass"), || (pass, 0_usize));
        if seen.0 != pass {
            *seen = (pass, 0);
        }
        seen.1 += 1;
        seen.1 - 1
    });
    base.with(count)
}

/// A painter on `ui`'s layer and clip rectangle that egui's disabled fade does not touch: a disabled control draws
/// itself `FAINT` with it.
pub(crate) fn exact_painter(ui: &Ui) -> Painter {
    Painter::new(ui.ctx().clone(), ui.layer_id(), ui.clip_rect())
}

/// `label` laid out on one line in the body font and `colour`, at most `max_w` wide (ending in an ellipsis when it
/// does not fit), its left edge `left` points into `rect` (or centred, when `left` is `None`) and in the middle of
/// its height, on whole pixels, never painted outside `rect`.
fn paint_label(painter: &Painter, rect: Rect, label: &str, colour: Color32, left: Option<f32>, max_w: f32) {
    let mut job = LayoutJob::simple_singleline(label.to_owned(), body(), colour);
    job.wrap = TextWrapping::truncate_at_width(max_w.max(0.0));
    let galley = painter.layout_job(job);
    let x = match left {
        Some(left) => rect.min.x + left,
        None => rect.center().x - galley.size().x / 2.0,
    };
    let y = rect.center().y - galley.size().y / 2.0;
    painter
        .with_clip_rect(rect.intersect(painter.clip_rect()))
        .galley(pos2(x.round(), y.round()), galley, colour);
}

/// The width of `label` in the body font.
fn label_width(ui: &Ui, label: &str) -> f32 {
    ui.painter().layout_no_wrap(label.to_owned(), body(), TEXT).size().x
}

/// `natural`, or the room `ui` has left when that is less (and a usable number).
fn fit_width(ui: &Ui, natural: f32) -> f32 {
    let room = ui.available_width();
    if room.is_finite() && room >= 0.0 && room < natural {
        room
    } else {
        natural
    }
}

/// The look of a button that cannot be used: no fill, no border, its label `FAINT`.
fn faint_look() -> Look {
    Look {
        fill: Color32::TRANSPARENT,
        border: None,
        label: FAINT,
    }
}

/// A 26 tall button with `label`, padded 10 at each side, looking as `look` says for the state it is in. `enabled`
/// false, or a disabled parent, makes it sense only hovering and draws it `FAINT`; `selected` makes it a toggle for
/// the accessibility tree.
fn button(
    ui: &mut Ui,
    label: &str,
    enabled: bool,
    selected: Option<bool>,
    look: impl FnOnce(&Response) -> Look,
) -> Response {
    let enabled = enabled && ui.is_enabled();
    let width = fit_width(ui, (label_width(ui, label) + 2.0 * PAD_X).ceil());
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(width, CONTROL_H), sense);
    response.widget_info(|| match selected {
        Some(on) => WidgetInfo::selected(Role::Button, enabled, on, label),
        None => WidgetInfo::labeled(Role::Button, enabled, label),
    });
    if ui.is_rect_visible(rect) {
        let look = if enabled { look(&response) } else { faint_look() };
        let painter = exact_painter(ui);
        painter.rect_filled(rect, RADIUS, look.fill);
        if let Some(border) = look.border {
            painter.rect_stroke(rect, RADIUS, Stroke::new(1.0, border), StrokeKind::Inside);
        }
        paint_label(
            &painter,
            rect,
            label,
            look.label,
            Some(PAD_X),
            rect.width() - 2.0 * PAD_X,
        );
        if response.has_focus() {
            widget_ring(ui, rect, f32::from(RADIUS));
        }
    }
    response
}

/// The flat look: transparent until hovered, then `HOVER`; label `TEXT`.
fn flat_look(response: &Response) -> Look {
    Look {
        fill: if response.hovered() {
            HOVER
        } else {
            Color32::TRANSPARENT
        },
        border: None,
        label: TEXT,
    }
}

/// The inverted look: `TEXT` fill, `ON_LIGHT` label.
fn inverted_look() -> Look {
    Look {
        fill: TEXT,
        border: None,
        label: ON_LIGHT,
    }
}

/// A flat button (toolbar, transport): 26 tall, transparent until hovered.
pub fn flat(ui: &mut Ui, label: &str) -> Response {
    button(ui, label, true, None, flat_look)
}

/// A flat button that is inverted while `on` (the current step, Loop).
pub fn toggle(ui: &mut Ui, label: &str, on: bool) -> Response {
    button(ui, label, true, Some(on), |response| {
        if on { inverted_look() } else { flat_look(response) }
    })
}

/// A box button (small actions: "Swap sides", "Remove"): `RAISED` with a 1 px `LINE` border; `HOVER` when hovered.
pub fn boxed(ui: &mut Ui, label: &str) -> Response {
    button(ui, label, true, None, |response| Look {
        fill: if response.hovered() { HOVER } else { RAISED },
        border: Some(LINE),
        label: TEXT,
    })
}

/// The screen's main action: `TEXT` fill, `ON_LIGHT` label.
pub fn primary(ui: &mut Ui, label: &str) -> Response {
    button(ui, label, true, None, |_| inverted_look())
}

/// A button that cannot be used now: it keeps its place, its label is `FAINT`, it has no hover fill, and hovering
/// it says `why`.
pub fn disabled(ui: &mut Ui, label: &str, why: &str) -> Response {
    button(ui, label, false, None, |_| faint_look())
        .on_hover_text(why)
        .on_disabled_hover_text(why)
}

/// A segmented control named `name`, `width` wide: one 24 tall segment per option, sharing a 1 px `LINE` border
/// with no gaps and square inner corners; the `chosen` one is inverted. Each option is a label and, when it cannot
/// be chosen, the reason, which hovering it shows; such a segment is `FAINT` and does nothing. Inside a disabled
/// parent every segment is `FAINT` (the chosen one a `FAINT` fill) and does nothing. Returns the index of the
/// segment clicked this frame.
pub fn segmented(
    ui: &mut Ui,
    name: &str,
    options: &[(&str, Option<&str>)],
    chosen: Option<usize>,
    width: f32,
) -> Option<usize> {
    let group = UiBuilder::new()
        .accessibility_role(Role::RadioGroup)
        .accessibility_label(name);
    // The segments' ids come from the name, kept over time; a second group of the same name in this `Ui` counts on.
    let base = named_id(ui, "segmented", name);
    ui.scope_builder(group, |ui| {
        let parent_enabled = ui.is_enabled();
        let (_, outer) = ui.allocate_space(vec2(width, SEGMENT_H));
        let count = options.len().max(1);
        let edge = |i: usize| (outer.min.x + width * i as f32 / count as f32).round();
        let mut clicked = None;
        let mut focused = None;
        let painter = exact_painter(ui);
        painter.rect_stroke(outer, RADIUS, Stroke::new(1.0, LINE), StrokeKind::Inside);
        for (i, &(label, reason)) in options.iter().enumerate() {
            let cell = Rect::from_x_y_ranges(edge(i)..=edge(i + 1), outer.y_range());
            let enabled = reason.is_none() && parent_enabled;
            let sense = if enabled { Sense::click() } else { Sense::hover() };
            let mut response = ui.interact(cell, base.with(("segment", i)), sense);
            let on = chosen == Some(i);
            response.widget_info(|| WidgetInfo::selected(Role::RadioButton, enabled, on, label));
            if i > 0 {
                painter.rect_filled(
                    Rect::from_x_y_ranges(cell.min.x..=cell.min.x + 1.0, cell.y_range()),
                    0.0,
                    LINE,
                );
            }
            let last = i + 1 == options.len();
            let inside = Rect::from_min_max(
                cell.min + vec2(1.0, 1.0),
                cell.max - vec2(if last { 1.0 } else { 0.0 }, 1.0),
            );
            let corner = |at_end: bool| if at_end { 1 } else { 0 };
            let corners = CornerRadius {
                nw: corner(i == 0),
                sw: corner(i == 0),
                ne: corner(last),
                se: corner(last),
            };
            let label_colour = if on {
                painter.rect_filled(inside, corners, if parent_enabled { TEXT } else { FAINT });
                ON_LIGHT
            } else if !enabled {
                FAINT
            } else {
                if response.hovered() {
                    painter.rect_filled(inside, corners, HOVER);
                }
                TEXT
            };
            paint_label(&painter, inside, label, label_colour, None, inside.width());
            if let Some(reason) = reason {
                response = response.on_hover_text(reason).on_disabled_hover_text(reason);
            }
            if response.has_focus() {
                focused = Some(cell);
            }
            if enabled && response.clicked() {
                clicked = Some(i);
            }
        }
        if let Some(cell) = focused {
            widget_ring(ui, cell, f32::from(RADIUS));
        }
        clicked
    })
    .inner
}

/// A switch with `label` after it: a square 30 x 16 outline (1 px, corner 2) with a 10 x 10 knob 2 from the edge;
/// `TEXT` with the knob right when `on`, `FAINT` with the knob left when off. The row is 26 tall; clicking anywhere
/// on it reports a click. A label too long for the room left ends in an ellipsis; [`wrapped_switch`] wraps it
/// instead. Inside a disabled parent it is `FAINT` throughout and takes no clicks.
pub fn switch(ui: &mut Ui, label: &str, on: bool) -> Response {
    switch_row(ui, label, on, None)
}

/// A switch whose `label` wraps onto as many lines as it needs inside `width` (the switch, its gap and the words):
/// the first line is level with the switch, and the whole row, words included, is the one control named by the
/// label (clicking the words clicks it). Otherwise as [`switch`].
pub fn wrapped_switch(ui: &mut Ui, label: &str, on: bool, width: f32) -> Response {
    let width = if width.is_finite() {
        width.max(SWITCH_W)
    } else {
        SWITCH_W
    };
    switch_row(ui, label, on, Some(width))
}

fn switch_row(ui: &mut Ui, label: &str, on: bool, wrap: Option<f32>) -> Response {
    let enabled = ui.is_enabled();
    let words_x = SWITCH_W + SWITCH_GAP;
    let (size, galley) = match wrap {
        None => (
            vec2(fit_width(ui, (words_x + label_width(ui, label)).ceil()), CONTROL_H),
            None,
        ),
        Some(width) => {
            let colour = if enabled { TEXT } else { FAINT };
            let job = LayoutJob::simple(label.to_owned(), body(), colour, (width - words_x).max(1.0));
            let galley = ui.painter().layout_job(job);
            let first = galley.rows.first().map_or(0.0, |row| row.height());
            let pad = ((CONTROL_H - first) / 2.0).floor().max(0.0);
            let height = (pad + galley.size().y + pad).max(CONTROL_H).ceil();
            (vec2(width, height), Some((galley, pad)))
        }
    };
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    response.widget_info(|| WidgetInfo::selected(Role::Switch, enabled, on, label));
    if ui.is_rect_visible(rect) {
        let painter = exact_painter(ui);
        let top = (rect.min.y + CONTROL_H / 2.0 - SWITCH_H / 2.0).round();
        let outline = Rect::from_min_size(pos2(rect.min.x, top), vec2(SWITCH_W, SWITCH_H));
        let colour = if on && enabled { TEXT } else { FAINT };
        painter.rect_stroke(outline, RADIUS, Stroke::new(1.0, colour), StrokeKind::Inside);
        let knob_x = if on { SWITCH_W - KNOB_INSET - KNOB } else { KNOB_INSET };
        let knob = Rect::from_min_size(outline.min + vec2(knob_x, KNOB_INSET), vec2(KNOB, KNOB));
        painter.rect_filled(knob, 0.0, colour);
        let label_colour = if enabled { TEXT } else { FAINT };
        let text = Rect::from_x_y_ranges(outline.max.x + SWITCH_GAP..=rect.max.x, rect.y_range());
        match galley {
            None => {
                let row = Rect::from_x_y_ranges(text.x_range(), rect.min.y..=rect.min.y + CONTROL_H);
                paint_label(&painter, row, label, label_colour, Some(0.0), row.width());
            }
            Some((galley, pad)) => {
                let at = pos2(text.min.x, rect.min.y + pad).round();
                painter
                    .with_clip_rect(text.intersect(painter.clip_rect()))
                    .galley(at, galley, label_colour);
            }
        }
        if response.has_focus() {
            widget_ring(ui, outline, f32::from(RADIUS));
        }
    }
    response
}

/// Makes a dropdown's list look like the menu: `RAISED`, with the theme's 1 px `LINE` window stroke, corner 2,
/// padding 3 and the menu shadow.
fn menu_list_style(style: &mut Style) {
    style.visuals.window_fill = RAISED;
    style.spacing.item_spacing.y = 0.0;
}

/// A dropdown named `name`, `width` wide and 22 tall: `RAISED` with a 1 px `LINE` border, the chosen option on the
/// left (ending in an ellipsis when it does not fit) and a 10 x 10 `MUTED` chevron on the right. While its list is
/// open it is inverted. Its list is a menu of 24 tall items; the item with keyboard focus is inverted, else the
/// hovered one, else the chosen one. Space or Enter opens it, the arrow keys move through the list, Space or Enter
/// picks, Escape closes. Returns the index picked this frame. Inside a disabled parent it has the look of
/// [`dropdown_disabled`] and picks nothing.
///
/// It is an `egui::ComboBox` named through its response, so the accessibility tree has a combo box called `name`
/// whose value is the chosen option, and each item is a list option named by its label. Its id comes from its name
/// (see [`named_id`]), so its open list and the keyboard stay with it when controls above it come and go, and two
/// dropdowns of one name do not clash.
pub fn dropdown(ui: &mut Ui, name: &str, options: &[String], chosen: usize, width: f32) -> Option<usize> {
    let value = options.get(chosen).map_or("", String::as_str);
    if !ui.is_enabled() {
        faint_dropdown(ui, name, value, width);
        return None;
    }
    let mut picked = None;
    let salt = named_id(ui, "dropdown", name);
    let response = ui
        .scope(|ui| {
            let open = ComboBox::is_open(ui.ctx(), ui.make_persistent_id(IdSalt::new(salt)));
            let style = ui.style_mut();
            style.spacing.button_padding = vec2(MENU_PAD_X, 0.0);
            style.spacing.interact_size.y = VALUE_H;
            style.spacing.icon_width = 10.0;
            let widgets = &mut style.visuals.widgets;
            widgets.inactive.weak_bg_fill = RAISED;
            widgets.hovered.weak_bg_fill = HOVER;
            widgets.active = widgets.hovered;
            let label = if open { ON_LIGHT } else { TEXT };
            ComboBox::from_id_salt(salt)
                .width(width)
                .truncate()
                .selected_text(RichText::new(value).color(label))
                .icon(move |ui, rect, _, open| {
                    let chevron = Rect::from_center_size(rect.center(), vec2(10.0, 10.0)).round();
                    icons::paint(
                        ui.painter(),
                        chevron,
                        Icon::Chevron,
                        if open { ON_LIGHT } else { MUTED },
                    );
                })
                .popup_style(menu_list_style.into())
                .show_ui(ui, |ui| picked = menu_items(ui, options, chosen))
                .response
        })
        .inner
        .accessible_name(name);
    let painter = ui.painter();
    painter.rect_stroke(response.rect, RADIUS, Stroke::new(1.0, LINE), StrokeKind::Inside);
    if response.has_focus() {
        widget_ring(ui, response.rect, f32::from(RADIUS));
    }
    picked
}

/// A dropdown that cannot be used now, named `name` and showing `value`, `width` wide and 22 tall: a 1 px `LINE`
/// border with no fill, the value (ending in an ellipsis when it does not fit) and the chevron `FAINT`. It takes no
/// input, is disabled for the accessibility tree, and hovering it says `why`.
pub fn dropdown_disabled(ui: &mut Ui, name: &str, value: &str, width: f32, why: &str) -> Response {
    faint_dropdown(ui, name, value, width)
        .on_hover_text(why)
        .on_disabled_hover_text(why)
}

fn faint_dropdown(ui: &mut Ui, name: &str, value: &str, width: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, VALUE_H), Sense::hover());
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(Role::ComboBox, false, name);
        info.current_text_value = Some(value.to_owned());
        info
    });
    if ui.is_rect_visible(rect) {
        let painter = exact_painter(ui);
        painter.rect_stroke(rect, RADIUS, Stroke::new(1.0, LINE), StrokeKind::Inside);
        let chevron_x = rect.max.x - MENU_PAD_X - 10.0;
        let text = Rect::from_x_y_ranges(rect.min.x..=chevron_x, rect.y_range());
        paint_label(
            &painter,
            text,
            value,
            FAINT,
            Some(MENU_PAD_X),
            text.width() - 2.0 * MENU_PAD_X,
        );
        let chevron = Rect::from_min_size(pos2(chevron_x, rect.center().y - 5.0), vec2(10.0, 10.0)).round();
        icons::paint(&painter, chevron, Icon::Chevron, FAINT);
    }
    response
}

/// The items of a dropdown's list, 24 tall and as wide as the list, padded 8 at each side. The one with keyboard
/// focus is inverted, else the hovered one, else the chosen one. Returns the index clicked (or pressed with Space or
/// Enter), and closes the list when one is.
fn menu_items(ui: &mut Ui, options: &[String], chosen: usize) -> Option<usize> {
    let width = ui.available_width();
    let responses: Vec<Response> = options
        .iter()
        .enumerate()
        .map(|(i, option)| {
            let (_, response) = ui.allocate_exact_size(vec2(width, ITEM_H), Sense::click());
            response.widget_info(|| WidgetInfo::selected(Role::ListBoxOption, true, i == chosen, option));
            response
        })
        .collect();
    let lit = (responses.iter().position(Response::has_focus))
        .or_else(|| responses.iter().position(Response::hovered))
        .unwrap_or(chosen);
    let painter = ui.painter();
    for (i, (option, response)) in options.iter().zip(&responses).enumerate() {
        let label = if i == lit {
            painter.rect_filled(response.rect, RADIUS, TEXT);
            ON_LIGHT
        } else {
            TEXT
        };
        let max_w = response.rect.width() - 2.0 * MENU_PAD_X;
        paint_label(painter, response.rect, option, label, Some(MENU_PAD_X), max_w);
    }
    let clicked = responses.iter().position(Response::clicked);
    if clicked.is_some() {
        ui.close();
    }
    clicked
}

/// A progress bar named `name`, `width` wide and 4 tall: a `LINE` track with a `TEXT` fill over the first `share`
/// (0 to 1) of it. A share that is not a finite number is drawn and reported as 0.
pub fn progress(ui: &mut Ui, name: &str, share: f32, width: f32) -> Response {
    let share = if share.is_finite() { share.clamp(0.0, 1.0) } else { 0.0 };
    let (rect, response) = ui.allocate_exact_size(vec2(width, PROGRESS_H), Sense::hover());
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(Role::ProgressIndicator, true, name);
        info.value = Some(f64::from(share) * 100.0);
        info
    });
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, LINE);
        let filled = (width * share).round();
        if filled > 0.0 {
            let fill = Rect::from_min_size(rect.min, vec2(filled, PROGRESS_H));
            painter.rect_filled(fill, 0.0, TEXT);
        }
    }
    response
}

/// Shows `area` in `ui` with the app's scroll look (widget rule 21): no fade at a scrolled edge (egui's fade would
/// cover a focus ring there, and the look has no gradients), and a solid bar whenever the content overflows, hovered
/// or not: 6 wide, 4 from the content and 4 from the area's edge, a `GROUND` track with a `FAINT` handle (`MUTED`
/// when hovered, `TEXT` while dragged). The content keeps the visuals and the scroll style `ui` had.
pub fn scrolled<R>(ui: &mut Ui, area: ScrollArea, add: impl FnOnce(&mut Ui) -> R) -> ScrollAreaOutput<R> {
    ui.scope(|ui| {
        let content = scroll_look(ui);
        area.scroll_bar_visibility(ScrollBarVisibility::VisibleWhenNeeded)
            .show(ui, |ui| {
                restore(ui, content);
                add(ui)
            })
    })
    .inner
}

/// [`scrolled`] for `count` rows of `row_h`, laying out only the rows in view (`ScrollArea::show_rows`): `add` gets
/// the range of rows to lay out.
pub fn scrolled_rows<R>(
    ui: &mut Ui,
    area: ScrollArea,
    row_h: f32,
    count: usize,
    add: impl FnOnce(&mut Ui, Range<usize>) -> R,
) -> ScrollAreaOutput<R> {
    ui.scope(|ui| {
        let content = scroll_look(ui);
        area.scroll_bar_visibility(ScrollBarVisibility::VisibleWhenNeeded)
            .show_rows(ui, row_h, count, |ui, rows| {
                restore(ui, content);
                add(ui, rows)
            })
    })
    .inner
}

/// Gives the content of a scroll area back the visuals and the scroll style its `Ui` had.
fn restore(ui: &mut Ui, (visuals, scroll): (Visuals, ScrollStyle)) {
    *ui.visuals_mut() = visuals;
    ui.spacing_mut().scroll = scroll;
}

/// Gives `ui` the scroll look of [`scrolled`]; returns the visuals and the scroll style it had, for the content.
fn scroll_look(ui: &mut Ui) -> (Visuals, ScrollStyle) {
    let content = (ui.visuals().clone(), ui.spacing().scroll);
    let style = ui.style_mut();
    style.spacing.scroll = ScrollStyle {
        bar_width: SCROLL_BAR_W,
        bar_inner_margin: SCROLL_BAR_MARGIN,
        bar_outer_margin: SCROLL_BAR_MARGIN,
        fade: ScrollFadeStyle {
            strength: 0.0,
            ..ScrollFadeStyle::default()
        },
        ..ScrollStyle::solid()
    };
    style.visuals.extreme_bg_color = GROUND;
    let widgets = &mut style.visuals.widgets;
    widgets.inactive.bg_fill = FAINT;
    widgets.hovered.bg_fill = MUTED;
    widgets.active.bg_fill = TEXT;
    content
}
