//! The painted icons (interface reference, section 3). They are drawn with the painter, never taken from a font: the
//! built-in fonts have no icon glyphs.

use egui::accesskit::Role;
use egui::{Color32, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, WidgetInfo, vec2};

use crate::theme::{CONTROL_H, FAINT, HOVER, RADIUS, TEXT, TEXT_STRONG};

/// The icons the app paints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    /// A triangle pointing right.
    Play,
    /// Two upright bars.
    Pause,
    /// A check: a step that is done.
    Done,
    /// A ring: the step in progress.
    Now,
    /// A dash: a step still to do.
    Todo,
    /// A downward chevron, on a 10 x 10 box: the dropdown's mark.
    Chevron,
}

/// The width of the line icons (done, now, to do).
const LINE_ICON: f32 = 1.6;
/// The width of the chevron's line.
const CHEVRON_LINE: f32 = 1.5;
/// An icon button's size.
const BUTTON_W: f32 = 24.0;

impl Icon {
    /// The side of the square box the icon's coordinates are given on.
    fn side(self) -> f32 {
        match self {
            Icon::Chevron => 10.0,
            _ => 12.0,
        }
    }
}

/// Paints `icon` in `color` on `rect`: a 12 x 12 box, 10 x 10 for the chevron. A box of another size scales the
/// drawing to fit it.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let scale = rect.size() / icon.side();
    let at = |x: f32, y: f32| -> Pos2 { rect.min + vec2(x * scale.x, y * scale.y) };
    let line = |width: f32| Stroke::new(width * scale.min_elem(), color);
    match icon {
        Icon::Play => {
            painter.add(Shape::convex_polygon(
                vec![at(2.0, 1.0), at(11.0, 6.0), at(2.0, 11.0)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Pause => {
            for x in [2.0, 7.0] {
                painter.rect_filled(Rect::from_min_max(at(x, 1.0), at(x + 3.0, 11.0)), 0.0, color);
            }
        }
        Icon::Done => {
            painter.add(Shape::line(
                vec![at(2.0, 6.5), at(5.0, 9.5), at(10.0, 3.0)],
                line(LINE_ICON),
            ));
        }
        Icon::Now => {
            painter.circle_stroke(at(6.0, 6.0), 3.0 * scale.min_elem(), line(LINE_ICON));
        }
        Icon::Todo => {
            painter.line_segment([at(2.0, 6.0), at(10.0, 6.0)], line(LINE_ICON));
        }
        Icon::Chevron => {
            painter.add(Shape::line(
                vec![at(2.0, 3.5), at(5.0, 6.5), at(8.0, 3.5)],
                line(CHEVRON_LINE),
            ));
        }
    }
}

/// A 24 x 26 flat button that shows only `icon`, named `name` for the accessibility tree. Transparent until hovered
/// (`HOVER`); the icon is `TEXT`, or `FAINT` and senseless to clicks when it cannot be used. It paints its own focus
/// ring (2 px `TEXT_STRONG`, 2 px outside).
pub fn icon_button(ui: &mut Ui, icon: Icon, name: &str, enabled: bool) -> Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(BUTTON_W, CONTROL_H), sense);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, name));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if enabled && response.hovered() {
            painter.rect_filled(rect, RADIUS, HOVER);
        }
        let icon_box = Rect::from_center_size(rect.center(), Vec2::splat(icon.side())).round();
        paint(painter, icon_box, icon, if enabled { TEXT } else { FAINT });
        if response.has_focus() {
            painter.rect_stroke(
                rect.expand(2.0),
                f32::from(RADIUS) + 2.0,
                Stroke::new(2.0, TEXT_STRONG),
                StrokeKind::Outside,
            );
        }
    }
    response
}
