//! The fonts built into the exe: the pinned files, which file backs which family, and a glyph for every character
//! the app shows.

use bf_app::theme::fonts;

#[test]
fn the_font_files_are_the_pinned_ones() {
    let lengths: Vec<usize> = fonts::FILES.iter().map(|(_, bytes)| bytes.len()).collect();
    assert_eq!(lengths, vec![200_500, 202_460, 202_632, 173_052]);
}

/// What a font file says it is: "family style weight", read from its own tables. The family and style are the
/// typographic ones (name ids 16 and 17, Windows platform), else the legacy ones (1 and 2); the weight is the OS/2
/// weight class.
fn identity(font: &[u8]) -> String {
    let u16_at = |at: usize| u16::from_be_bytes([font[at], font[at + 1]]) as usize;
    let u32_at = |at: usize| u32::from_be_bytes([font[at], font[at + 1], font[at + 2], font[at + 3]]) as usize;
    let table = |tag: &[u8]| {
        (0..u16_at(4))
            .map(|i| 12 + 16 * i)
            .find(|&entry| &font[entry..entry + 4] == tag)
            .map(|entry| u32_at(entry + 8))
            .expect("the font has the table")
    };
    let names = table(b"name");
    let strings = names + u16_at(names + 4);
    let name = |id: usize| {
        (0..u16_at(names + 2))
            .map(|i| names + 6 + 12 * i)
            .find(|&record| u16_at(record) == 3 && u16_at(record + 6) == id)
            .map(|record| {
                let start = strings + u16_at(record + 10);
                let units: Vec<u16> = (0..u16_at(record + 8) / 2)
                    .map(|k| u16_at(start + 2 * k) as u16)
                    .collect();
                String::from_utf16(&units).expect("names are UTF-16")
            })
    };
    let family = name(16).or_else(|| name(1)).expect("the font has a family name");
    let style = name(17).or_else(|| name(2)).expect("the font has a style name");
    let weight = u16_at(table(b"OS/2") + 4);
    format!("{family} {style} {weight}")
}

#[test]
fn each_family_is_backed_by_its_own_font() {
    let definitions = fonts::definitions();
    let chain = |family: egui::FontFamily| -> Vec<String> {
        definitions.families[&family]
            .iter()
            .map(|name| identity((*definitions.font_data[name].font).as_ref()))
            .collect()
    };
    assert_eq!(chain(egui::FontFamily::Proportional), ["IBM Plex Sans Regular 400"]);
    assert_eq!(
        chain(egui::FontFamily::Monospace),
        ["IBM Plex Mono Regular 400", "IBM Plex Sans Regular 400"]
    );
    assert_eq!(chain(fonts::medium()), ["IBM Plex Sans Medium 500"]);
    assert_eq!(chain(fonts::semibold()), ["IBM Plex Sans SemiBold 600"]);
    assert_eq!(definitions.families.len(), 4);
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
    // which is the assertion. Emoji are not checked: no built-in font draws them (no official monochrome Noto Emoji
    // has the harp, U+1FA89, yet), so the app leaves them to the PC's own fonts.
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
