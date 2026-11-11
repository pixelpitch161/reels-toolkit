//! The fonts built into the exe: the pinned files, and a glyph for every character the app shows.

use bf_app::theme::fonts;

#[test]
fn the_font_files_are_the_pinned_ones() {
    let lengths: Vec<usize> = fonts::FILES.iter().map(|(_, bytes)| bytes.len()).collect();
    assert_eq!(lengths, vec![200_500, 202_460, 202_632, 173_052, 1_982_596]);
}

/// Every printable ASCII character plus the typography the wording uses: times, ellipsis, apostrophe, en dash.
fn text_characters() -> String {
    let mut text: String = (0x20u8..=0x7e).map(char::from).collect();
    text.push_str("\u{00d7}\u{2026}\u{2019}\u{2013}");
    text
}

#[test]
fn every_character_the_app_shows_has_a_glyph() {
    // Nothing is drawn with wgpu here, so no `common::gpu()`. The harness panics on any character that no font has,
    // which is the assertion. Emoji are not checked: no built-in font draws them (the pinned Noto Emoji has no harp,
    // U+1FA89), so the app leaves them to the PC's own fonts.
    let text = text_characters();
    let families = [
        egui::FontFamily::Proportional,
        egui::FontFamily::Monospace,
        fonts::medium(),
        fonts::semibold(),
    ];
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::Vec2::new(1440.0, 400.0))
        .with_context_setup(|ctx| ctx.set_fonts(fonts::definitions()))
        .build_ui(|ui| {
            for family in &families {
                ui.label(egui::RichText::new(&text).family(family.clone()));
            }
        });
    harness.run();
}
