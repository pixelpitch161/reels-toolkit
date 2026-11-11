//! The theme: the design system's egui notes hold once `theme::install` has run, and plain text draws in `TEXT`.

use bf_app::theme::{self, PANEL};
use egui::{Color32, CornerRadius, Stroke, TextStyle, vec2};

use crate::common;

#[test]
fn the_design_systems_egui_notes_hold() {
    let _gpu = common::gpu();
    let mut checked = false;
    let mut harness = common::widget_harness((200.0, 60.0), |ui| {
        let style = ui.style();
        let visuals = &style.visuals;
        assert_eq!(visuals.panel_fill, PANEL);
        assert_eq!(visuals.window_fill, PANEL);
        assert_eq!(visuals.extreme_bg_color, theme::GROUND);
        assert_eq!(visuals.faint_bg_color, theme::RAISED);
        assert_eq!(visuals.window_stroke, Stroke::new(1.0, theme::LINE));
        assert_eq!(visuals.widgets.inactive.corner_radius, CornerRadius::same(2));
        assert_eq!(visuals.widgets.inactive.weak_bg_fill, Color32::TRANSPARENT);
        assert_eq!(visuals.widgets.hovered.weak_bg_fill, theme::HOVER);
        assert_eq!(visuals.widgets.active.weak_bg_fill, theme::TEXT);
        assert_eq!(visuals.widgets.active.fg_stroke.color, theme::ON_LIGHT);
        assert_eq!(visuals.selection.bg_fill, theme::TEXT);
        let spacing = &style.spacing;
        assert_eq!(spacing.item_spacing, vec2(8.0, 6.0));
        assert_eq!(spacing.button_padding, vec2(10.0, 5.0));
        assert_eq!(spacing.interact_size.y, 26.0);
        let styles = &style.text_styles;
        assert_eq!(styles[&TextStyle::Body], theme::body());
        assert_eq!(styles[&TextStyle::Button], theme::body());
        assert_eq!(styles[&TextStyle::Small], theme::small());
        assert_eq!(styles[&TextStyle::Monospace], theme::value());
        assert_eq!(styles[&TextStyle::Heading], theme::title());
        checked = true;
    });
    harness.run();
    drop(harness);
    assert!(checked, "the checks ran");
}

#[test]
fn plain_text_is_the_text_colour() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((120.0, 40.0), |ui| {
        ui.painter().rect_filled(ui.ctx().content_rect(), 0.0, PANEL);
        ui.label("Flute");
    });
    harness.run();
    let image = harness.render().expect("the harness renders");
    let brightest = image
        .pixels()
        .map(|pixel| [pixel.0[0], pixel.0[1], pixel.0[2]])
        .max_by_key(|&[r, g, b]| u32::from(r) + u32::from(g) + u32::from(b))
        .expect("the picture has pixels");
    assert_eq!(brightest, [0xe2, 0xe2, 0xe2]);
}
