//! The egui style made from the tokens: the design system's egui notes (interface reference, section 8). Greys
//! only, corners 2, 1 px lines, and the inverted fill (`TEXT` with `ON_LIGHT`) as the only "on" signal.

use egui::style::{Selection, WidgetVisuals, Widgets};
use egui::{Color32, CornerRadius, Margin, Shadow, Stroke, Style, TextStyle, Visuals, vec2};

use super::tokens::*;

/// The menu shadow, the only shadow in the app: offset 0 x 8, blur 20, `#000000aa`.
const MENU_SHADOW: Shadow = Shadow {
    offset: [0, 8],
    blur: 20,
    spread: 0,
    color: Color32::from_black_alpha(0xaa),
};

/// One widget state: `fill` behind it, `fg` for its text and marks, corner 2, no outline, no growth.
fn state(fill: Color32, fg: Color32) -> WidgetVisuals {
    WidgetVisuals {
        bg_fill: fill,
        weak_bg_fill: fill,
        bg_stroke: Stroke::NONE,
        corner_radius: CornerRadius::same(RADIUS),
        fg_stroke: Stroke::new(1.0, fg),
        expansion: 0.0,
    }
}

/// The app's visuals, the same for egui's dark and light themes.
pub fn visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.weak_text_color = Some(MUTED);
    visuals.hyperlink_color = TEXT;
    visuals.warn_fg_color = TEXT_STRONG;
    visuals.error_fg_color = TEXT_STRONG;

    visuals.panel_fill = PANEL;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = GROUND;
    visuals.text_edit_bg_color = Some(GROUND);
    visuals.faint_bg_color = RAISED;
    visuals.code_bg_color = RAISED;

    visuals.window_stroke = Stroke::new(1.0, LINE);
    visuals.window_corner_radius = CornerRadius::same(RADIUS);
    visuals.menu_corner_radius = CornerRadius::same(RADIUS);
    visuals.window_shadow = MENU_SHADOW;
    visuals.popup_shadow = MENU_SHADOW;

    let mut noninteractive = state(PANEL, TEXT);
    noninteractive.bg_stroke = Stroke::new(1.0, LINE_SOFT);
    let inactive = WidgetVisuals {
        weak_bg_fill: Color32::TRANSPARENT,
        ..state(RAISED, TEXT)
    };
    let active = state(TEXT, ON_LIGHT);
    visuals.widgets = Widgets {
        noninteractive,
        inactive,
        hovered: state(HOVER, TEXT),
        active,
        open: active,
    };

    visuals.selection = Selection {
        bg_fill: TEXT,
        stroke: Stroke::new(1.0, ON_LIGHT),
    };
    visuals.text_cursor.stroke = Stroke::new(2.0, TEXT_STRONG);
    visuals.ime_composition.active_underline_stroke.color = TEXT_STRONG;
    visuals.ime_composition.inactive_underline_stroke.color = MUTED;
    visuals
}

/// Puts the app's visuals, spacing and text styles into `style`.
pub fn apply(style: &mut Style) {
    style.visuals = visuals();

    let spacing = &mut style.spacing;
    spacing.item_spacing = vec2(8.0, 6.0);
    spacing.button_padding = vec2(10.0, 5.0);
    spacing.interact_size.y = CONTROL_H;
    spacing.menu_margin = Margin::same(3);

    let styles = &mut style.text_styles;
    styles.insert(TextStyle::Body, body());
    styles.insert(TextStyle::Button, body());
    styles.insert(TextStyle::Small, small());
    styles.insert(TextStyle::Monospace, value());
    styles.insert(TextStyle::Heading, title());
}
