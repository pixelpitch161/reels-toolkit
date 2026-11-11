//! The basic controls (interface reference, section 3): flat, toggle, box, primary and disabled buttons, the
//! segmented control, the switch, the dropdown and the progress bar.
//!
//! Each is painted here rather than left to egui's own widget look, because the theme cannot give it: the theme's
//! `override_text_color` would draw every label in `TEXT` (invisible on the inverted fill), egui's focus outline is
//! invisible in these colours, and egui fades a disabled control instead of drawing it `FAINT`. So every control
//! sets its label colour for each state (`ON_LIGHT` on the inverted fill), paints its own focus ring (2 px
//! `TEXT_STRONG`, 2 px outside) and names itself for the accessibility tree.

use egui::accesskit::Role;
use egui::{
    Color32, ComboBox, CornerRadius, IdSalt, Painter, Rect, Response, RichText, Sense, Stroke, StrokeKind, Style, Ui,
    UiBuilder, WidgetInfo, vec2,
};

use super::icons::{self, Icon};
use crate::theme::{CONTROL_H, FAINT, HOVER, LINE, MUTED, ON_LIGHT, RADIUS, RAISED, TEXT, TEXT_STRONG, VALUE_H, body};

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

/// What a button looks like in its present state.
struct Look {
    fill: Color32,
    border: Option<Color32>,
    label: Color32,
}

/// The keyboard focus ring: 2 px `TEXT_STRONG`, 2 px outside `rect`.
fn focus_ring(painter: &Painter, rect: Rect) {
    painter.rect_stroke(
        rect.expand(2.0),
        f32::from(RADIUS) + 2.0,
        Stroke::new(2.0, TEXT_STRONG),
        StrokeKind::Outside,
    );
}

/// `label` laid out in the body font and `colour`, its top-left corner placed so it sits `left` points into `rect`
/// (or centred, when `left` is `None`) and in the middle of its height, on whole pixels.
fn paint_label(painter: &Painter, rect: Rect, label: &str, colour: Color32, left: Option<f32>) {
    let galley = painter.layout_no_wrap(label.to_owned(), body(), colour);
    let x = match left {
        Some(left) => rect.min.x + left,
        None => rect.center().x - galley.size().x / 2.0,
    };
    let y = rect.center().y - galley.size().y / 2.0;
    painter.galley(egui::pos2(x.round(), y.round()), galley, colour);
}

/// The width of `label` in the body font.
fn label_width(ui: &Ui, label: &str) -> f32 {
    ui.painter().layout_no_wrap(label.to_owned(), body(), TEXT).size().x
}

/// A 26 tall button with `label`, padded 10 at each side, looking as `look` says for the state it is in. `enabled`
/// false makes it sense only hovering; `selected` makes it a toggle for the accessibility tree.
fn button(
    ui: &mut Ui,
    label: &str,
    enabled: bool,
    selected: Option<bool>,
    look: impl FnOnce(&Response) -> Look,
) -> Response {
    let size = vec2((label_width(ui, label) + 2.0 * PAD_X).ceil(), CONTROL_H);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    response.widget_info(|| match selected {
        Some(on) => WidgetInfo::selected(Role::Button, enabled, on, label),
        None => WidgetInfo::labeled(Role::Button, enabled, label),
    });
    if ui.is_rect_visible(rect) {
        let look = look(&response);
        let painter = ui.painter();
        painter.rect_filled(rect, RADIUS, look.fill);
        if let Some(border) = look.border {
            painter.rect_stroke(rect, RADIUS, Stroke::new(1.0, border), StrokeKind::Inside);
        }
        paint_label(painter, rect, label, look.label, Some(PAD_X));
        if response.has_focus() {
            focus_ring(painter, rect);
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
    button(ui, label, false, None, |_| Look {
        fill: Color32::TRANSPARENT,
        border: None,
        label: FAINT,
    })
    .on_hover_text(why)
}

/// A segmented control named `name`, `width` wide: one 24 tall segment per option, sharing a 1 px `LINE` border
/// with no gaps and square inner corners; the `chosen` one is inverted. Each option is a label and, when it cannot
/// be chosen, the reason, which hovering it shows; such a segment is `FAINT` and does nothing. Returns the index of
/// the segment clicked this frame.
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
    ui.scope_builder(group, |ui| {
        let (_, outer) = ui.allocate_space(vec2(width, SEGMENT_H));
        let count = options.len().max(1);
        let edge = |i: usize| (outer.min.x + width * i as f32 / count as f32).round();
        let mut clicked = None;
        let mut focused = None;
        let painter = ui.painter().clone();
        painter.rect_stroke(outer, RADIUS, Stroke::new(1.0, LINE), StrokeKind::Inside);
        for (i, &(label, reason)) in options.iter().enumerate() {
            let cell = Rect::from_x_y_ranges(edge(i)..=edge(i + 1), outer.y_range());
            let enabled = reason.is_none();
            let sense = if enabled { Sense::click() } else { Sense::hover() };
            let mut response = ui.interact(cell, ui.make_persistent_id((name, i)), sense);
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
                painter.rect_filled(inside, corners, TEXT);
                ON_LIGHT
            } else if !enabled {
                FAINT
            } else {
                if response.hovered() {
                    painter.rect_filled(inside, corners, HOVER);
                }
                TEXT
            };
            paint_label(&painter, inside, label, label_colour, None);
            if let Some(reason) = reason {
                response = response.on_hover_text(reason);
            }
            if response.has_focus() {
                focused = Some(cell);
            }
            if enabled && response.clicked() {
                clicked = Some(i);
            }
        }
        if let Some(cell) = focused {
            focus_ring(&painter, cell);
        }
        clicked
    })
    .inner
}

/// A switch with `label` after it: a square 30 x 16 outline (1 px, corner 2) with a 10 x 10 knob 2 from the edge;
/// `TEXT` with the knob right when `on`, `FAINT` with the knob left when off. The row is 26 tall; clicking anywhere
/// on it reports a click.
pub fn switch(ui: &mut Ui, label: &str, on: bool) -> Response {
    let size = vec2((SWITCH_W + SWITCH_GAP + label_width(ui, label)).ceil(), CONTROL_H);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Switch, true, on, label));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let top = (rect.center().y - SWITCH_H / 2.0).round();
        let outline = Rect::from_min_size(egui::pos2(rect.min.x, top), vec2(SWITCH_W, SWITCH_H));
        let colour = if on { TEXT } else { FAINT };
        painter.rect_stroke(outline, RADIUS, Stroke::new(1.0, colour), StrokeKind::Inside);
        let knob_x = if on { SWITCH_W - KNOB_INSET - KNOB } else { KNOB_INSET };
        let knob = Rect::from_min_size(outline.min + vec2(knob_x, KNOB_INSET), vec2(KNOB, KNOB));
        painter.rect_filled(knob, 0.0, colour);
        let text = Rect::from_x_y_ranges(outline.max.x + SWITCH_GAP..=rect.max.x, rect.y_range());
        paint_label(painter, text, label, TEXT, Some(0.0));
        if response.has_focus() {
            focus_ring(painter, outline);
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
/// left and a 10 x 10 `MUTED` chevron on the right. While its list is open it is inverted. Its list is a menu of 24
/// tall items; the hovered item is inverted, or the chosen one while none is hovered. Returns the index picked this
/// frame.
///
/// It is an `egui::ComboBox` named through its response, so the accessibility tree has a combo box called `name`
/// whose value is the chosen option, and each item is a list option named by its label.
pub fn dropdown(ui: &mut Ui, name: &str, options: &[String], chosen: usize, width: f32) -> Option<usize> {
    let value = options.get(chosen).map_or("", String::as_str);
    let mut picked = None;
    let response = ui
        .scope(|ui| {
            let open = ComboBox::is_open(ui.ctx(), ui.make_persistent_id(IdSalt::new(name)));
            let style = ui.style_mut();
            style.spacing.button_padding = vec2(MENU_PAD_X, 0.0);
            style.spacing.interact_size.y = VALUE_H;
            style.spacing.icon_width = 10.0;
            let widgets = &mut style.visuals.widgets;
            widgets.inactive.weak_bg_fill = RAISED;
            widgets.hovered.weak_bg_fill = HOVER;
            widgets.active = widgets.hovered;
            let label = if open { ON_LIGHT } else { TEXT };
            ComboBox::from_id_salt(name)
                .width(width)
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
        focus_ring(painter, response.rect);
    }
    picked
}

/// The items of a dropdown's list, 24 tall and as wide as the list, padded 8 at each side. The hovered one is
/// inverted, else the chosen one. Returns the index clicked.
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
    let lit = responses.iter().position(Response::hovered).unwrap_or(chosen);
    let painter = ui.painter();
    for (i, (option, response)) in options.iter().zip(&responses).enumerate() {
        let label = if i == lit {
            painter.rect_filled(response.rect, RADIUS, TEXT);
            ON_LIGHT
        } else {
            TEXT
        };
        paint_label(painter, response.rect, option, label, Some(MENU_PAD_X));
    }
    responses.iter().position(Response::clicked)
}

/// A progress bar named `name`, `width` wide and 4 tall: a `LINE` track with a `TEXT` fill over the first `share`
/// (0 to 1) of it.
pub fn progress(ui: &mut Ui, name: &str, share: f32, width: f32) -> Response {
    let share = share.clamp(0.0, 1.0);
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
