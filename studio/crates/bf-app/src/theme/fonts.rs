//! The fonts built into the exe, so the app looks the same on every PC: IBM Plex Sans (Regular, Medium, SemiBold)
//! and IBM Plex Mono (Regular). Where each file comes from is in `assets/fonts/SOURCES.md`.
//!
//! Noto Emoji is built in too but in no family: the pinned file has no harp (U+1FA89), so emoji are drawn by the
//! PC's own fonts, which eframe's `system_fonts` feature finds.

use std::sync::Arc;

const SANS_REGULAR: &str = "IBMPlexSans-Regular";
const SANS_MEDIUM: &str = "IBMPlexSans-Medium";
const SANS_SEMIBOLD: &str = "IBMPlexSans-SemiBold";
const MONO_REGULAR: &str = "IBMPlexMono-Regular";

/// Each built-in font file's name and bytes, in the order Sans Regular, Medium, SemiBold, Mono Regular, Noto Emoji.
pub const FILES: [(&str, &[u8]); 5] = [
    (
        "IBMPlexSans-Regular.ttf",
        include_bytes!("../../assets/fonts/IBMPlexSans-Regular.ttf"),
    ),
    (
        "IBMPlexSans-Medium.ttf",
        include_bytes!("../../assets/fonts/IBMPlexSans-Medium.ttf"),
    ),
    (
        "IBMPlexSans-SemiBold.ttf",
        include_bytes!("../../assets/fonts/IBMPlexSans-SemiBold.ttf"),
    ),
    (
        "IBMPlexMono-Regular.ttf",
        include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf"),
    ),
    (
        "NotoEmoji-Variable.ttf",
        include_bytes!("../../assets/fonts/NotoEmoji-Variable.ttf"),
    ),
];

/// The font names `definitions` registers, one per entry of `FILES`, in the same order. Noto Emoji is left out.
const NAMES: [&str; 4] = [SANS_REGULAR, SANS_MEDIUM, SANS_SEMIBOLD, MONO_REGULAR];

/// Plex Sans Medium (500), for the text of a selected chip or row.
pub fn medium() -> egui::FontFamily {
    egui::FontFamily::Name("medium".into())
}

/// Plex Sans SemiBold (600), for titles and section headings.
pub fn semibold() -> egui::FontFamily {
    egui::FontFamily::Name("semibold".into())
}

/// The app's fonts and families, and nothing of egui's own. Proportional is Plex Sans Regular; Monospace is Plex
/// Mono Regular, falling back to Plex Sans for what Mono lacks.
pub fn definitions() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::empty();
    for (name, (_, bytes)) in NAMES.iter().zip(FILES) {
        fonts
            .font_data
            .insert((*name).to_owned(), Arc::new(egui::FontData::from_static(bytes)));
    }
    let family = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>();
    fonts
        .families
        .insert(egui::FontFamily::Proportional, family(&[SANS_REGULAR]));
    fonts
        .families
        .insert(egui::FontFamily::Monospace, family(&[MONO_REGULAR, SANS_REGULAR]));
    fonts.families.insert(medium(), family(&[SANS_MEDIUM]));
    fonts.families.insert(semibold(), family(&[SANS_SEMIBOLD]));
    fonts
}
