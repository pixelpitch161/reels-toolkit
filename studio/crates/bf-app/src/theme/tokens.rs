//! The design tokens: the greys, the sizes, the corners and the type styles every screen and widget draws with.
//! Values are the interface reference's (`docs/studio/interface-reference.md`, section 2).

use egui::{Color32, FontFamily, FontId};

use super::fonts;

/// Window background, status bar, text and value field wells.
pub const GROUND: Color32 = Color32::from_rgb(0x1e, 0x1e, 0x1e);
/// Toolbar, bout bar, workspaces, side panels, timeline. The base fighter fills are mixed over.
pub const PANEL: Color32 = Color32::from_rgb(0x28, 0x28, 0x28);
/// The column behind the 9:16 preview.
pub const STAGE: Color32 = Color32::from_rgb(0x2b, 0x2b, 0x2b);
/// Dropdowns, box buttons, menus.
pub const RAISED: Color32 = Color32::from_rgb(0x33, 0x33, 0x33);
/// Fill of a selected row or chip, always with a 1.5 px `TEXT` ring.
pub const SELECTED: Color32 = Color32::from_rgb(0x38, 0x38, 0x38);
/// Hover fill of flat buttons and of menu items that are not highlighted.
pub const HOVER: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3a);
/// Dividers between panels and sections.
pub const LINE_SOFT: Color32 = Color32::from_rgb(0x38, 0x38, 0x38);
/// Control outlines, field borders, slider tracks, the ribbon's centre line.
pub const LINE: Color32 = Color32::from_rgb(0x44, 0x44, 0x44);
/// Disabled text, the outline and knob of a switch that is off. Never for text that must be read.
pub const FAINT: Color32 = Color32::from_rgb(0x77, 0x77, 0x77);
/// Labels, hints, counts, lane names, the status bar.
pub const MUTED: Color32 = Color32::from_rgb(0xa0, 0xa0, 0xa0);
/// Values, names, body text. Also the fill of on and primary buttons.
pub const TEXT: Color32 = Color32::from_rgb(0xe2, 0xe2, 0xe2);
/// Playhead, K.O. bar, slider knob, text of a selected chip, the failed-job headline.
pub const TEXT_STRONG: Color32 = Color32::from_rgb(0xff, 0xff, 0xff);
/// Text on a `TEXT` fill.
pub const ON_LIGHT: Color32 = Color32::from_rgb(0x1e, 0x1e, 0x1e);
/// The hairline under each queue job and each arrangement lane.
pub const HAIRLINE: Color32 = Color32::from_rgb(0x30, 0x30, 0x30);
/// The group header lanes "Fighters" and "Band".
pub const GROUP: Color32 = Color32::from_rgb(0x2e, 0x2e, 0x2e);
/// Histogram bars outside the K.O. window.
pub const BAR_DIM: Color32 = Color32::from_rgb(0x66, 0x66, 0x66);
/// Band clips on the Music screen (under a 1 px `MUTED` top edge).
pub const BAND_CLIP: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3a);

/// Buttons, fields, setting rows.
pub const CONTROL_H: f32 = 26.0;
/// Value fields and dropdowns inside a row.
pub const VALUE_H: f32 = 22.0;
/// A roster chip.
pub const ROW_H: f32 = 30.0;
/// The toolbar.
pub const TOOLBAR_H: f32 = 40.0;
/// The bout bar.
pub const BOUT_H: f32 = 48.0;
/// The stage column beside the workspace.
pub const STAGE_W: f32 = 410.0;
/// The story timeline, transport row included.
pub const TIMELINE_H: f32 = 112.0;
/// The transport row at the top of the story timeline.
pub const TRANSPORT_H: f32 = 30.0;
/// The status bar.
pub const STATUS_H: f32 = 24.0;
/// The label column of a setting row.
pub const LABEL_W: f32 = 108.0;
/// The value field column.
pub const VALUE_W: f32 = 62.0;
/// The lane name column of the story timeline.
pub const LANE_NAME_W: f32 = 120.0;

/// Buttons, fields, dropdowns, menus, switches.
pub const RADIUS: u8 = 2;
/// Roster chips and shortlist rows only, so they read as objects.
pub const RADIUS_ROW: u8 = 8;

/// An opaque colour from its three channels.
pub fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// `over` mixed `share` (0 to 1) over `base`, per channel `round(base + share * (over - base))`. Opaque.
pub fn mix(over: Color32, base: Color32, share: f32) -> Color32 {
    let channel = |o: u8, b: u8| {
        let (o, b) = (f32::from(o), f32::from(b));
        (b + share * (o - b)).round().clamp(0.0, 255.0) as u8
    };
    Color32::from_rgb(
        channel(over.r(), base.r()),
        channel(over.g(), base.g()),
        channel(over.b(), base.b()),
    )
}

/// The large fill of a fighter's colour: 22 % of it over `PANEL`. It is drawn with a 1 px edge in the full colour.
pub fn large_fill(fighter: [u8; 3]) -> Color32 {
    mix(rgb(fighter), PANEL, 0.22)
}

/// Buttons, labels, rows, menus: Plex Sans 13. The default.
pub fn body() -> FontId {
    FontId::new(13.0, FontFamily::Proportional)
}

/// Hints, counts, lane names, column headers, the status bar: Plex Sans 12.
pub fn small() -> FontId {
    FontId::new(12.0, FontFamily::Proportional)
}

/// Section headings, the app name, job titles: Plex Sans SemiBold 13.
pub fn section() -> FontId {
    FontId::new(13.0, fonts::semibold())
}

/// The text of a selected chip or row: Plex Sans Medium 13.
pub fn strong() -> FontId {
    FontId::new(13.0, fonts::medium())
}

/// The big numbers on Balance, window titles: Plex Sans SemiBold 18.
pub fn title() -> FontId {
    FontId::new(18.0, fonts::semibold())
}

/// Numbers in value fields, seeds, scores, K.O. times, chord loops: Plex Mono 12.
pub fn value() -> FontId {
    FontId::new(12.0, FontFamily::Monospace)
}

/// The transport's time: Plex Mono 14.
pub fn timecode() -> FontId {
    FontId::new(14.0, FontFamily::Monospace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_fills_match_the_mock_ups() {
        assert_eq!(large_fill([0xde, 0xe8, 0xf6]), Color32::from_rgb(0x50, 0x52, 0x55));
        assert_eq!(large_fill([0x78, 0xf0, 0xb8]), Color32::from_rgb(0x3a, 0x54, 0x48));
    }

    #[test]
    fn mix_ends_on_its_base_and_its_colour() {
        assert_eq!(mix(TEXT, PANEL, 0.0), PANEL);
        assert_eq!(mix(TEXT, PANEL, 1.0), TEXT);
    }
}
