//! The themes: one folder per theme (the kit's moods, `projects/ballfight/kit/moods/<name>.py`), each with its
//! instruments, its voicings and its classic arrangement (the bed that reproduces the kit's), and the registry
//! that lists them.

use bf_sound::traits::Bed;

pub mod big_top;

/// The arrangement every theme has: the one that reproduces the kit's bed.
pub const CLASSIC: &str = "classic";

/// A theme as the registry lists it.
#[derive(Debug)]
pub struct Theme {
    /// Its name in a fight file (`music.theme`): the kit's mood name.
    pub name: &'static str,
    /// Whether the theme has a voicing written for a chord (by its name in a fight file, such as `B7`).
    voices: fn(&str) -> bool,
    /// Its classic arrangement.
    classic: fn() -> Box<dyn Bed>,
}

impl Theme {
    /// Whether the theme has a voicing for `chord`: a fight file may only ask it for chords it has.
    pub fn voices(&self, chord: &str) -> bool {
        (self.voices)(chord)
    }

    /// The theme's bed in the arrangement `arrangement` (`classic` in Milestone 1), or `None` for an
    /// arrangement it does not have.
    pub fn bed(&self, arrangement: &str) -> Option<Box<dyn Bed>> {
        (arrangement == CLASSIC).then(|| (self.classic)())
    }
}

/// Every theme, in the order the app lists them.
static THEMES: [&Theme; 1] = [&big_top::THEME];

/// The names of the themes this app has (`big_top` in Milestone 1).
pub fn theme_names() -> Vec<&'static str> {
    THEMES.iter().map(|t| t.name).collect()
}

/// The theme `name`, or `None` for a name that is no theme.
pub fn theme(name: &str) -> Option<&'static Theme> {
    THEMES.iter().copied().find(|t| t.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_has_the_big_top() {
        assert_eq!(theme_names(), ["big_top"]);
        let t = theme("big_top").expect("the big top");
        assert_eq!(t.name, "big_top");
        for name in ["polka", "", "Big_Top", "big top"] {
            assert!(theme(name).is_none(), "{name:?}");
        }
        // The arrangement that reproduces the kit's bed, and no other yet.
        let bed = t.bed(CLASSIC).expect("the classic arrangement");
        assert_eq!(bed.reverb(), (2.0, 0.55, 0.22));
        assert!(t.bed("layered").is_none() && t.bed("").is_none());
        // It has voicings for its own chords.
        assert!(t.voices("Em") && t.voices("B7") && !t.voices("F#m") && !t.voices("em"));
    }
}
