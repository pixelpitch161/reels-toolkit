//! The theme dropdown of the Match-up and Music screens: the themes the pair may be set in (ruling M3-R2), the
//! recommended ones first under "Recommended for this pair", then the rest under their categories' headings (each
//! theme once, so its label names one option; the app's font has no star glyph) (design `docs/superpowers/specs/2026-11-15-theme-categories-design.md`).

use egui::Ui;

use crate::model::roster::Roster;
use crate::widgets::buttons::{self, MenuEntry};

/// The heading over the recommended themes.
pub const RECOMMENDED_HEADING: &str = "Recommended for this pair";

/// The picker's lines for `fighters`: each item's index is the theme's place in `roster.themes`.
pub fn entries(roster: &Roster, fighters: &[String; 2]) -> Vec<MenuEntry> {
    let index = |name: &str| roster.themes.iter().position(|t| t.name == name).unwrap_or(usize::MAX);
    let mut out = Vec::new();
    let recommended = roster.recommended(fighters);
    if !recommended.is_empty() {
        out.push(MenuEntry::Heading(RECOMMENDED_HEADING.to_string()));
        out.extend(recommended.iter().map(|t| MenuEntry::Item {
            index: index(&t.name),
            label: t.display.clone(),
        }));
    }
    for (group, themes) in roster.grouped(fighters) {
        let rest: Vec<_> = themes
            .into_iter()
            .filter(|t| !recommended.iter().any(|r| r.name == t.name))
            .collect();
        if rest.is_empty() {
            continue;
        }
        out.push(MenuEntry::Heading(group));
        out.extend(rest.iter().map(|t| MenuEntry::Item {
            index: index(&t.name),
            label: t.display.clone(),
        }));
    }
    out
}

/// The theme dropdown, `width` wide, for `fighters` set in `current`. Returns the name of a different theme picked
/// this frame.
pub fn theme_picker(ui: &mut Ui, roster: &Roster, fighters: &[String; 2], current: &str, width: f32) -> Option<String> {
    let offered = roster.themes_for(fighters);
    let shown = offered.iter().find(|t| t.name == current);
    let value = shown.map_or("", |t| t.display.as_str());
    let chosen = shown
        .and_then(|t| roster.themes.iter().position(|x| x.name == t.name))
        .unwrap_or(usize::MAX);
    let picked = buttons::dropdown_grouped(ui, "Theme", &entries(roster, fighters), value, chosen, width)?;
    let theme = roster.themes.get(picked)?;
    (theme.name != current).then(|| theme.name.clone())
}
