//! The theme dropdown of the Match-up and Music screens: the themes the pair may be set in (ruling M3-R2). Its list
//! shows only group names, "Recommended for this pair" first and then the categories; hovering one opens a submenu of
//! its themes (design `docs/superpowers/specs/2026-11-15-theme-categories-design.md`; the submenus were asked for
//! on Nov 16, 2026). A recommended theme is also listed under its own category.

use egui::Ui;

use crate::model::roster::Roster;
use crate::widgets::buttons::{self, MenuGroup};

/// The name of the group of recommended themes.
pub const RECOMMENDED_HEADING: &str = "Recommended for this pair";

/// The picker's groups for `fighters`: each item's index is the theme's place in `roster.themes`.
pub fn groups(roster: &Roster, fighters: &[String; 2]) -> Vec<MenuGroup> {
    let index = |name: &str| roster.themes.iter().position(|t| t.name == name).unwrap_or(usize::MAX);
    let mut out = Vec::new();
    let recommended = roster.recommended(fighters);
    if !recommended.is_empty() {
        out.push(MenuGroup {
            name: RECOMMENDED_HEADING.to_string(),
            items: recommended
                .iter()
                .map(|t| (index(&t.name), t.display.clone()))
                .collect(),
        });
    }
    for (group, themes) in roster.grouped(fighters) {
        out.push(MenuGroup {
            name: group,
            items: themes.iter().map(|t| (index(&t.name), t.display.clone())).collect(),
        });
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
    let picked = buttons::dropdown_grouped(ui, "Theme", &groups(roster, fighters), value, chosen, width)?;
    let theme = roster.themes.get(picked)?;
    (theme.name != current).then(|| theme.name.clone())
}
