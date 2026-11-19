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

/// A bare context with the theme installed and one pass run, so its fonts exist. Nothing is drawn with wgpu, so no
/// `common::gpu()`.
fn themed_context() -> egui::Context {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    pass(&ctx);
    ctx
}

/// Runs one pass that lays out a label and says whether it uploaded any texture (a font atlas). The texture changes
/// are cleared afterwards: egui panics when they are dropped unhandled.
fn pass(ctx: &egui::Context) -> bool {
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        ui.label("Flute");
    });
    let uploaded = !output.textures_delta.set.is_empty();
    output.textures_delta.clear();
    uploaded
}

/// `style` with its number formatter taken from `like`: egui compares formatters by pointer, and each theme has its
/// own, so two styles that are the same in every value otherwise read as different.
fn with_formatter_of(style: &egui::Style, like: &egui::Style) -> egui::Style {
    let mut style = style.clone();
    style.number_formatter = like.number_formatter.clone();
    style
}

#[test]
fn both_egui_themes_look_the_same() {
    let ctx = themed_context();
    let dark = ctx.style_of(egui::Theme::Dark);
    let light = ctx.style_of(egui::Theme::Light);
    assert_eq!(
        dark.visuals, light.visuals,
        "the PC's light or dark setting changes nothing"
    );
    assert_eq!(*dark, with_formatter_of(&light, &dark));
}

#[test]
fn the_choices_beyond_the_notes_hold() {
    let ctx = themed_context();
    let style = ctx.style_of(egui::Theme::Dark);
    let visuals = &style.visuals;
    assert_eq!(visuals.widgets.open, visuals.widgets.active, "open looks like active");
    assert_eq!(visuals.override_text_color, Some(theme::TEXT));
    assert_eq!(
        visuals.widgets.noninteractive.bg_stroke,
        Stroke::new(1.0, theme::LINE_SOFT)
    );
    assert_eq!(style.spacing.menu_margin, egui::Margin::same(3));
}

#[test]
fn a_second_install_changes_nothing() {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    assert!(pass(&ctx), "building the fonts uploads their atlas");
    let dark = ctx.style_of(egui::Theme::Dark);
    let light = ctx.style_of(egui::Theme::Light);
    let fonts = ctx.fonts(|fonts| fonts.definitions().clone());
    theme::install(&ctx);
    let uploaded = pass(&ctx);
    assert_eq!(ctx.style_of(egui::Theme::Dark), dark);
    assert_eq!(ctx.style_of(egui::Theme::Light), light);
    assert!(
        ctx.fonts(|now| *now.definitions() == fonts),
        "the font definitions are the same"
    );
    assert!(!uploaded, "the fonts were not rebuilt");
}

/// A trap for later work: egui's "strong" text takes the active state's foreground, which is `ON_LIGHT` here (the
/// inverted fill's label colour), so `.strong()` text would be near-black on a dark panel. That is why `.strong()`
/// is banned in this app and emphasis uses `RichText::color(TEXT_STRONG)`.
#[test]
fn strong_text_would_be_on_light() {
    let ctx = themed_context();
    assert_eq!(
        ctx.style_of(egui::Theme::Dark).visuals.strong_text_color(),
        theme::ON_LIGHT
    );
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
