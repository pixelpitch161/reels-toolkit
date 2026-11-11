//! The value field and the text fields (interface reference, section 3).
//!
//! - **Value field**: a `GROUND` well, 1 px `LINE` border, 22 tall, mono text right-aligned with its unit ("×2.00",
//!   "49.0 s"). Drag sideways, use the up and down arrows, or type to change it. It is an `egui::DragValue` named
//!   for the accessibility tree by the caller's `name`.
//! - **Text field**: a `GROUND` well, 1 px `LINE` border, 26 tall (the caption box is as many lines as asked). A
//!   placeholder is `FAINT`. Text lines and boxes are `egui::TextEdit`s; a read field is a well painted here, whose
//!   text is cut short with an ellipsis when it does not fit (hovering it then shows all of it).
//!
//! Each keeps its `LINE` border whether it has the keyboard or not, and paints the theme's focus ring (2 px
//! `TEXT_STRONG`, 2 px outside) when it has it. The ring is painted on a layer of its own just above the field's,
//! clipped 4 px wider than the field's own clip rectangle, so a field that reaches a panel's edge still shows all of
//! its ring. Inside a disabled parent each field draws its text `FAINT` itself (not egui's half-alpha fade) and takes
//! no input.
//!
//! The value field never produces a number that is not finite or lies outside its range, whatever is typed or
//! however the range is given: the range is made safe as the slider's is (ends turned the right way round, an end
//! that is not finite replaced by the other end); typed text is read as a number with or without the unit, and text
//! that is not a finite number ("abc", "nan", "inf", "1e999", nothing) leaves the value as it was; a number outside
//! the range is moved to the nearer end. A caller's value that is not a number is shown as the range's low end and
//! is left alone until the person changes it or leaves the field.

use egui::accesskit::Role;
use egui::{
    Align, CornerRadius, DragValue, Frame, Layout, Margin, Painter, Rect, Response, Sense, Stroke, StrokeKind,
    TextEdit, TextStyle, Ui, UiBuilder, WidgetInfo, vec2,
};
use egui::{LayerId, text::LayoutJob, text::TextWrapping};

use super::buttons::focus_ring;
use super::slider::{clamp, half_span, tidy_range};
use crate::theme::{CONTROL_H, FAINT, GROUND, LINE, RADIUS, TEXT, VALUE_H, body};

/// Side padding of the text inside a well.
const PAD_X: f32 = 6.0;
/// The space the focus ring takes outside a control: a 2 px gap, then 2 px of ring.
const RING_ROOM: f32 = 4.0;
/// The most decimals a value field shows.
const MAX_DECIMALS: usize = 15;
/// The narrowest field.
const MIN_W: f32 = 24.0;

/// How a value field writes its number: `prefix`, the number with `decimals` decimals, `suffix` ("×2.00", "49.0 s").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unit<'a> {
    /// Written before the number, as in "×2.00".
    pub prefix: &'a str,
    /// Written after it, as in "49.0 s" (the space is part of the suffix).
    pub suffix: &'a str,
    /// How many decimals are shown (at most 15).
    pub decimals: usize,
}

impl Unit<'_> {
    /// `value` as the field shows it. Zero is never written with a minus sign.
    fn write(&self, value: f64) -> String {
        let decimals = self.decimals.min(MAX_DECIMALS);
        let number = format!("{value:.decimals$}");
        let zero = number.trim_start_matches('-').chars().all(|c| c == '0' || c == '.');
        let number = if zero { number.trim_start_matches('-') } else { &number };
        format!("{}{number}{}", self.prefix, self.suffix)
    }

    /// The number in `text`, typed with or without the unit, if it is a finite number. A single comma is read as the
    /// decimal point when there is no point ("1,5" is 1.5); any other comma makes the text unreadable.
    fn read(&self, text: &str) -> Option<f64> {
        let mut text = text.trim();
        let (prefix, suffix) = (self.prefix.trim(), self.suffix.trim());
        if !prefix.is_empty() {
            text = text.strip_prefix(prefix).unwrap_or(text);
        }
        if !suffix.is_empty() {
            text = text.strip_suffix(suffix).unwrap_or(text);
        }
        let cleaned: String = text
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| if c == '\u{2212}' { '-' } else { c })
            .collect();
        let commas = cleaned.matches(',').count();
        let cleaned = if commas == 1 && !cleaned.contains('.') {
            cleaned.replace(',', ".")
        } else {
            cleaned
        };
        cleaned.parse::<f64>().ok().filter(|value| value.is_finite())
    }
}

/// A width that can be used: finite and at least `MIN_W`.
fn tidy_width(width: f32) -> f32 {
    if width.is_finite() { width.max(MIN_W) } else { MIN_W }
}

/// Makes `ui` paint at full strength when it is disabled: a disabled control is drawn `FAINT` by the widget, not
/// faded by egui.
fn unfade(ui: &mut Ui) {
    if !ui.is_enabled() {
        ui.set_opacity(1.0);
    }
}

/// The text colour of a field: `TEXT`, or `FAINT` when disabled.
fn text_colour(ui: &Ui) -> egui::Color32 {
    if ui.is_enabled() { TEXT } else { FAINT }
}

/// The 1 px `LINE` border of a well.
fn border(painter: &Painter, rect: Rect) {
    painter.rect_stroke(rect, RADIUS, Stroke::new(1.0, LINE), StrokeKind::Inside);
}

/// The frame of a text field's well: `GROUND`, 1 px `LINE`, corner 2.
fn well_frame() -> Frame {
    Frame {
        fill: GROUND,
        stroke: Stroke::new(1.0, LINE),
        corner_radius: CornerRadius::same(RADIUS),
        inner_margin: Margin::symmetric(PAD_X as i8 - 1, 3),
        ..Frame::default()
    }
}

/// Paints the focus ring around `rect` on a layer of its own just above `ui`'s, clipped 4 px wider than `ui`'s clip
/// rectangle, so a panel's clip rectangle does not cut it off.
///
/// The ring's layer shares its parent's order (it is drawn above everything else of that order). That is right in
/// panels; revisit it if a field ever sits in a movable window (the ring could draw over another window) or in a
/// scroll area (the ring of a field scrolled half out of view shows 4 px past the area's edge).
pub(crate) fn ring_above(ui: &Ui, rect: Rect) {
    let parent = ui.layer_id();
    let layer = LayerId::new(parent.order, parent.id.with("bf-app focus ring"));
    let ctx = ui.ctx();
    ctx.set_sublayer(parent, layer);
    let transform = ctx.layer_transform_to_global(parent).unwrap_or_default();
    ctx.set_transform_layer(layer, transform);
    let painter = Painter::new(ctx.clone(), layer, ui.clip_rect().expand(RING_ROOM));
    focus_ring(&painter, rect);
}

/// A value field named `name` for `value` in `range`, written with `unit`, `width` wide and 22 tall (see the
/// module's notes for what it does with numbers it cannot use). Dragging sideways moves it a hundredth of the range
/// per point; the up and down arrows do the same once it has the keyboard; typing a number and pressing Enter sets it.
pub fn value_field(
    ui: &mut Ui,
    name: &str,
    value: &mut f64,
    range: (f64, f64),
    unit: Unit<'_>,
    width: f32,
) -> Response {
    let range = tidy_range(range);
    let (id, rect) = ui.allocate_space(vec2(tidy_width(width), VALUE_H));
    let mut field = ui.new_child(
        UiBuilder::new()
            .id_salt(id)
            .max_rect(rect)
            .layout(Layout::left_to_right(Align::Center).with_main_align(Align::Max)),
    );
    // A number too long for the well is cut off at its edge, never drawn over its neighbours.
    field.set_clip_rect(rect.intersect(field.clip_rect()));
    unfade(&mut field);
    let colour = text_colour(&field);
    let style = field.style_mut();
    style.spacing.interact_size.y = VALUE_H;
    style.spacing.button_padding = vec2(PAD_X, 0.0);
    style.drag_value_text_style = TextStyle::Monospace;
    style.explanation_tooltips = false;
    style.visuals.override_text_color = Some(colour);
    style.visuals.text_edit_bg_color = Some(GROUND);
    let widgets = &mut style.visuals.widgets;
    for state in [
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
        &mut widgets.noninteractive,
    ] {
        state.bg_fill = GROUND;
        state.weak_bg_fill = GROUND;
        state.bg_stroke = Stroke::NONE;
        state.fg_stroke.color = colour;
        state.expansion = 0.0;
    }
    field.painter().rect_filled(rect, RADIUS, GROUND);

    let before = clamp(*value, range);
    let mut shown = before;
    let decimals = unit.decimals.min(MAX_DECIMALS);
    let response = field
        .add(
            DragValue::new(&mut shown)
                .range(range.0..=range.1)
                .speed(2.0 * half_span(range.0, range.1) / 100.0)
                .fixed_decimals(decimals)
                .custom_formatter(|v, _| unit.write(v))
                .custom_parser(|text| unit.read(text))
                .min_size(vec2(rect.width(), VALUE_H)),
        )
        .accessible_name(name);
    // The drag value clamps typed and dragged numbers, but a step from the arrow keys can pass an end for a frame
    // and a typed one could be rounded past it: keep every number inside the range here.
    // A caller's value that is not a number or lies outside the range is replaced by the one shown once the person
    // finishes with the field (leaves it or ends a drag), even when what they did left the shown number as it was.
    let shown = clamp(shown, range);
    let finished = response.lost_focus() || response.drag_stopped();
    if shown != before || (finished && *value != shown) {
        *value = shown;
    }

    border(field.painter(), rect);
    if response.has_focus() {
        ring_above(&field, rect);
    }
    response
}

/// A read-only field named `name` showing `text`, `width` wide and 26 tall: a `GROUND` well with a 1 px `LINE`
/// border that cannot be edited. Text that does not fit is cut short with an ellipsis, and hovering the field then
/// shows all of it; a screen reader always gets all of it.
pub fn read_field(ui: &mut Ui, name: &str, text: &str, width: f32) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(vec2(tidy_width(width), CONTROL_H), Sense::hover());
    let enabled = ui.is_enabled();
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(Role::TextInput, enabled, name);
        info.current_text_value = Some(text.to_owned());
        info
    });
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_read_only());
    if ui.is_rect_visible(rect) {
        let mut painter = ui.painter().clone();
        if !enabled {
            painter.set_opacity(1.0);
        }
        painter.rect_filled(rect, RADIUS, GROUND);
        border(&painter, rect);
        let colour = text_colour(ui);
        let mut job = LayoutJob::simple_singleline(text.to_owned(), body(), colour);
        job.wrap = TextWrapping::truncate_at_width(rect.width() - 2.0 * PAD_X);
        let galley = painter.layout_job(job);
        let top = (rect.center().y - galley.size().y / 2.0).round();
        let elided = galley.elided;
        painter
            .with_clip_rect(rect.shrink(1.0))
            .galley(egui::pos2(rect.min.x + PAD_X, top), galley, colour);
        if elided {
            response = response.on_hover_text(text);
        }
    }
    response
}

/// Shows `edit` in a well, named `name`, with the field look: `FAINT` hint, `TEXT` text (`FAINT` when disabled), the
/// focus ring.
fn text_field(ui: &mut Ui, name: &str, edit: TextEdit<'_>) -> Response {
    ui.scope(|ui| {
        unfade(ui);
        let colour = text_colour(ui);
        ui.visuals_mut().weak_text_color = Some(FAINT);
        ui.style_mut().spacing.interact_size.y = CONTROL_H;
        let response = ui
            .add(edit.frame(well_frame()).font(body()).text_color(colour))
            .accessible_name(name);
        if response.has_focus() {
            ring_above(ui, response.rect);
        }
        response
    })
    .inner
}

/// A one-line text field named `name` for `text`, `width` wide and 26 tall, showing `hint` in `FAINT` while it is
/// empty. Text longer than the field scrolls inside it.
pub fn text_line(ui: &mut Ui, name: &str, text: &mut String, hint: &str, width: f32) -> Response {
    let width = tidy_width(width);
    let edit = TextEdit::singleline(text)
        .hint_text(hint.to_owned())
        .desired_width(width)
        .min_size(vec2(width, CONTROL_H));
    text_field(ui, name, edit)
}

/// A text box named `name` for `text`, `rows` lines tall (at least one), filling the width it is given. Lines that
/// are too long wrap.
pub fn text_box(ui: &mut Ui, name: &str, text: &mut String, rows: usize) -> Response {
    let edit = TextEdit::multiline(text)
        .desired_rows(rows.max(1))
        .desired_width(f32::INFINITY);
    text_field(ui, name, edit)
}
