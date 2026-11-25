//! The genre families of the themes (sound upgrade, 2026-11-25): the grouping the melody styles are offered by
//! (C11: each theme lists the styles that suit it) and the one the bands' own patterns are written by (C4).
//!
//! The picker's five categories (`crate::fit`) are for finding a theme; a family is about how the music is
//! written, so the two do not line up (a mambo and a hot jazz band share a picker category and nothing else).

use bf_sound::music::Style;

/// A genre family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Family {
    Latin,
    /// Jazz and swing.
    Jazz,
    /// Rock and pop.
    Rock,
    /// Orchestral and film.
    Orchestral,
    /// Electronic and hip-hop.
    Electronic,
    /// Folk and world.
    Folk,
    /// Marching bands, and the rest.
    Marching,
}

impl Family {
    /// Every family.
    pub const ALL: [Family; 7] = [
        Family::Latin,
        Family::Jazz,
        Family::Rock,
        Family::Orchestral,
        Family::Electronic,
        Family::Folk,
        Family::Marching,
    ];
}

/// The themes of each family but [`Family::Marching`], which takes the rest.
const THEMES: [(Family, &[&str]); 6] = [
    (
        Family::Latin,
        &[
            "bossa_noir",
            "cumbia",
            "dark_tango",
            "mambo",
            "mariachi",
            "salsa_dura",
            "samba_bateria",
        ],
    ),
    (
        Family::Jazz,
        &[
            "acid_jazz",
            "bebop_chase",
            "big_band_swing",
            "boogie_shuffle",
            "heist",
            "hot_jazz",
            "noir_jazz",
            "second_line",
        ],
    ),
    (
        Family::Rock,
        &[
            "drumline_metal",
            "funk_rock",
            "glam_stomp",
            "pep_band_punk",
            "prog_duel",
            "rockabilly",
            "ska_punk",
            "stadium_rock",
            "surf_rock",
            "thrash_attack",
        ],
    ),
    (
        Family::Orchestral,
        &[
            "baroque_storm",
            "brass_trailer",
            "desert_raid",
            "gothic_horror",
            "heroic_fanfare",
            "kaiju_stomp",
            "orchestral_drill",
            "samurai_duel",
            "skeleton_gallop",
            "spaghetti_western",
            "starfighter",
        ],
    ),
    (
        Family::Electronic,
        &[
            "amapiano",
            "brass_house",
            "chiptune",
            "dark_breakbeat",
            "dark_disco",
            "drift_phonk",
            "dubstep",
            "hardstyle",
            "horror_trap",
            "industrial",
            "jersey_club",
            "liquid_dnb",
            "synthwave",
        ],
    ),
    (
        Family::Folk,
        &[
            "balkan_fire",
            "bhangra",
            "bluegrass",
            "celtic_reel",
            "cossack_dance",
            "flamenco",
            "highlife",
            "klezmer",
            "oompah_polka",
            "sea_shanty",
        ],
    ),
];

/// The family of theme `theme` (a theme the table does not name is with the marching bands: `big_top`,
/// `dark_drumline`, `villain_march`, and any theme added without a line here).
pub fn of(theme: &str) -> Family {
    THEMES
        .iter()
        .find(|(_, names)| names.contains(&theme))
        .map_or(Family::Marching, |(family, _)| *family)
}

/// The melody styles that suit a family, Signature first (the style of a fight file that names none).
pub fn styles(family: Family) -> &'static [Style] {
    use Style::{Anchor, BrokenChords, Climbing, Riff, Runs, Signature};
    match family {
        // a montuno is a riff and a broken chord; a held note under a moving voice is the bolero's
        Family::Latin => &[Signature, Riff, BrokenChords, Anchor, Climbing],
        // lines that run: scales, climbing sequences, arpeggios, and the big band's riff
        Family::Jazz => &[Signature, Runs, Climbing, BrokenChords, Riff],
        // the riff first, the pedal note under it, and the climb and the run of a solo
        Family::Rock => &[Signature, Riff, Anchor, Climbing, Runs],
        // sequences, arpeggios, scales and the pedal point; a riff is not its language
        Family::Orchestral => &[Signature, Climbing, BrokenChords, Runs, Anchor],
        // loops: the riff, the arpeggiator and the one-note hook
        Family::Electronic => &[Signature, Riff, BrokenChords, Anchor],
        // reels run, dances break chords and climb, and the drone is the anchor note
        Family::Folk => &[Signature, Runs, BrokenChords, Climbing, Anchor],
        // fanfares climb and break chords; a cadence riff
        Family::Marching => &[Signature, Climbing, BrokenChords, Riff],
    }
}

/// The melody styles theme `theme` offers ([`styles`] of its family).
pub fn styles_of(theme: &str) -> &'static [Style] {
    styles(of(theme))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes::PORTED_THEMES;

    /// Every theme the table names is a theme, once; with the three marching themes that is all 62.
    #[test]
    fn every_theme_has_one_family() {
        let named: Vec<&str> = THEMES.iter().flat_map(|(_, names)| names.iter().copied()).collect();
        for name in &named {
            assert!(PORTED_THEMES.contains(name), "{name} is not a theme");
            assert_eq!(named.iter().filter(|n| *n == name).count(), 1, "{name} is listed twice");
        }
        let rest: Vec<&str> = PORTED_THEMES.iter().copied().filter(|t| !named.contains(t)).collect();
        assert_eq!(rest, ["big_top", "dark_drumline", "villain_march"]);
        assert_eq!(of("mambo"), Family::Latin);
        assert_eq!(of("hot_jazz"), Family::Jazz);
        assert_eq!(of("big_top"), Family::Marching);
        assert_eq!(of("no such theme"), Family::Marching);
    }

    /// Three to six styles a family, Signature first, none twice.
    #[test]
    fn a_family_offers_three_to_six_styles() {
        for family in Family::ALL {
            let list = styles(family);
            assert!((3..=6).contains(&list.len()), "{family:?}");
            assert_eq!(list[0], Style::Signature, "{family:?}");
            for s in list {
                assert_eq!(
                    list.iter().filter(|t| *t == s).count(),
                    1,
                    "{family:?} lists {s:?} twice"
                );
            }
        }
        assert_eq!(styles_of("mambo"), styles(Family::Latin));
    }
}
