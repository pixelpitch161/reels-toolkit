//! What makes a theme fit a pair: each fighter's traits and energy, and each theme's category, the traits it likes
//! and its tempo range. The app scores the themes for a pair from these (the theme picker's "Recommended" group and
//! the recommended tempo; design `docs/superpowers/specs/2026-11-15-theme-categories-design.md`).
//!
//! The values are a first draft by ear, meant to be nudged: a fighter or a theme added later only needs its own row.

/// The traits fighters have and themes like.
pub const TRAITS: [&str; 16] = [
    "brassy",
    "reedy",
    "airy",
    "bowed",
    "plucked",
    "keys",
    "electric",
    "percussive",
    "high",
    "low",
    "folk",
    "jazz",
    "classical",
    "rock",
    "club",
    "sinister",
];

/// The theme picker's categories, in the order it lists them.
pub const CATEGORIES: [&str; 5] = [
    "Jazz & Latin",
    "Folk & World",
    "Cinematic & Dark",
    "Rock & Punk",
    "Electronic & Club",
];

/// A fighter's traits (3 to 5 of [`TRAITS`]) and energy: 1 heavy and stately, 3 fast and bright.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FighterFit {
    pub traits: &'static [&'static str],
    pub energy: u8,
}

/// A theme's category (one of [`CATEGORIES`]), the traits it likes with a weight of 1 or 2, and the tempo range in
/// bpm its recommended tempo is placed in (low end for the heaviest pair, high end for the brightest).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeFit {
    pub category: &'static str,
    pub likes: &'static [(&'static str, u8)],
    pub tempo: (f64, f64),
}

const fn f(traits: &'static [&'static str], energy: u8) -> FighterFit {
    FighterFit { traits, energy }
}

const fn t(category: &'static str, likes: &'static [(&'static str, u8)], lo: f64, hi: f64) -> ThemeFit {
    ThemeFit {
        category,
        likes,
        tempo: (lo, hi),
    }
}

const FIGHTERS: [(&str, FighterFit); 34] = [
    ("FLUTE", f(&["airy", "high", "classical", "folk"], 3)),
    ("OBOE", f(&["reedy", "high", "classical", "folk"], 2)),
    ("CLARINET", f(&["reedy", "jazz", "classical", "folk"], 2)),
    ("BASS CLARINET", f(&["reedy", "low", "jazz", "sinister"], 1)),
    ("CONTRABASS CLARINET", f(&["reedy", "low", "sinister"], 1)),
    ("BASSOON", f(&["reedy", "low", "classical", "sinister"], 1)),
    ("ALTO SAX", f(&["reedy", "high", "jazz", "club"], 3)),
    ("SOPRANO SAX", f(&["reedy", "high", "jazz", "folk"], 3)),
    ("TENOR SAX", f(&["reedy", "jazz", "rock", "club"], 2)),
    ("BARI SAX", f(&["reedy", "low", "jazz", "club"], 2)),
    ("BAGPIPES", f(&["reedy", "high", "folk"], 3)),
    ("TRUMPET", f(&["brassy", "high", "jazz", "classical"], 3)),
    ("CORNET", f(&["brassy", "jazz", "folk"], 2)),
    ("HORN", f(&["brassy", "classical", "sinister"], 2)),
    ("TROMBONE", f(&["brassy", "low", "jazz"], 2)),
    ("EUPHONIUM", f(&["brassy", "low", "classical", "folk"], 2)),
    ("MARIMBA", f(&["percussive", "high", "folk", "jazz"], 3)),
    ("TUBA", f(&["brassy", "low", "folk"], 1)),
    ("SOUSAPHONE", f(&["brassy", "low", "jazz", "folk"], 2)),
    ("VIOLIN", f(&["bowed", "high", "classical", "folk"], 3)),
    ("VIOLA", f(&["bowed", "classical", "folk"], 2)),
    ("CELLO", f(&["bowed", "low", "classical", "sinister"], 2)),
    ("CONTRABASS", f(&["bowed", "plucked", "low", "jazz"], 1)),
    ("HARP", f(&["plucked", "high", "classical"], 2)),
    ("ACOUSTIC", f(&["plucked", "folk", "rock"], 2)),
    ("UKULELE", f(&["plucked", "high", "folk"], 3)),
    ("ELECTRIC", f(&["electric", "plucked", "rock"], 3)),
    ("PIANO", f(&["keys", "jazz", "classical"], 2)),
    ("ORGAN", f(&["keys", "classical", "sinister", "rock"], 1)),
    ("ACCORDION", f(&["keys", "reedy", "folk"], 2)),
    ("SYNTH", f(&["electric", "keys", "club"], 3)),
    ("TURNTABLES", f(&["electric", "percussive", "club"], 3)),
    ("DRUMS", f(&["percussive", "rock", "club"], 3)),
    ("BASS DRUM", f(&["percussive", "low", "rock"], 1)),
];

const JAZZ: &str = CATEGORIES[0];
const FOLK: &str = CATEGORIES[1];
const CINEMA: &str = CATEGORIES[2];
const ROCK: &str = CATEGORIES[3];
const CLUB: &str = CATEGORIES[4];

const THEMES: [(&str, ThemeFit); 26] = [
    (
        "hot_jazz",
        t(
            JAZZ,
            &[("jazz", 2), ("brassy", 1), ("reedy", 1), ("high", 1)],
            128.0,
            160.0,
        ),
    ),
    (
        "noir_jazz",
        t(
            JAZZ,
            &[("jazz", 2), ("reedy", 1), ("low", 1), ("sinister", 1), ("keys", 1)],
            110.0,
            140.0,
        ),
    ),
    (
        "bossa_noir",
        t(
            JAZZ,
            &[("plucked", 2), ("jazz", 1), ("airy", 1), ("reedy", 1)],
            110.0,
            138.0,
        ),
    ),
    (
        "mambo",
        t(
            JAZZ,
            &[("brassy", 2), ("jazz", 1), ("percussive", 1), ("keys", 1)],
            120.0,
            150.0,
        ),
    ),
    (
        "dark_tango",
        t(
            JAZZ,
            &[("bowed", 2), ("keys", 1), ("reedy", 1), ("sinister", 1), ("low", 1)],
            108.0,
            128.0,
        ),
    ),
    (
        "second_line",
        t(
            JAZZ,
            &[("brassy", 2), ("jazz", 2), ("percussive", 1), ("low", 1)],
            110.0,
            136.0,
        ),
    ),
    (
        "balkan_fire",
        t(
            FOLK,
            &[("folk", 2), ("brassy", 1), ("reedy", 1), ("keys", 1)],
            128.0,
            160.0,
        ),
    ),
    (
        "sea_shanty",
        t(
            FOLK,
            &[("folk", 2), ("keys", 1), ("bowed", 1), ("low", 1)],
            108.0,
            130.0,
        ),
    ),
    (
        "spaghetti_western",
        t(
            FOLK,
            &[("plucked", 2), ("brassy", 1), ("folk", 1), ("sinister", 1)],
            116.0,
            140.0,
        ),
    ),
    (
        "big_top",
        t(
            FOLK,
            &[("brassy", 1), ("folk", 1), ("sinister", 1), ("keys", 1), ("low", 1)],
            118.0,
            140.0,
        ),
    ),
    (
        "baroque_storm",
        t(
            CINEMA,
            &[("classical", 2), ("bowed", 1), ("keys", 1), ("high", 1)],
            128.0,
            155.0,
        ),
    ),
    (
        "orchestral_drill",
        t(
            CINEMA,
            &[("classical", 2), ("brassy", 1), ("bowed", 1), ("club", 1), ("low", 1)],
            128.0,
            150.0,
        ),
    ),
    (
        "brass_trailer",
        t(
            CINEMA,
            &[("brassy", 2), ("classical", 1), ("percussive", 1), ("low", 1)],
            110.0,
            130.0,
        ),
    ),
    (
        "villain_march",
        t(
            CINEMA,
            &[("sinister", 2), ("brassy", 1), ("low", 1), ("classical", 1)],
            112.0,
            134.0,
        ),
    ),
    (
        "gothic_horror",
        t(
            CINEMA,
            &[("sinister", 2), ("keys", 1), ("bowed", 1), ("classical", 1)],
            112.0,
            134.0,
        ),
    ),
    (
        "heist",
        t(
            CINEMA,
            &[("jazz", 1), ("sinister", 1), ("plucked", 1), ("bowed", 1)],
            108.0,
            128.0,
        ),
    ),
    (
        "dark_drumline",
        t(
            CINEMA,
            &[("percussive", 2), ("brassy", 1), ("sinister", 1), ("low", 1)],
            116.0,
            136.0,
        ),
    ),
    (
        "stadium_rock",
        t(
            ROCK,
            &[("rock", 2), ("electric", 1), ("percussive", 1), ("brassy", 1)],
            116.0,
            136.0,
        ),
    ),
    (
        "prog_duel",
        t(
            ROCK,
            &[("rock", 1), ("keys", 1), ("electric", 1), ("classical", 1)],
            122.0,
            144.0,
        ),
    ),
    (
        "ska_punk",
        t(
            ROCK,
            &[("brassy", 2), ("rock", 1), ("reedy", 1), ("high", 1)],
            140.0,
            166.0,
        ),
    ),
    ("brass_house", t(CLUB, &[("club", 2), ("brassy", 2)], 120.0, 130.0)),
    (
        "dark_disco",
        t(
            CLUB,
            &[("club", 2), ("electric", 1), ("bowed", 1), ("sinister", 1)],
            112.0,
            124.0,
        ),
    ),
    (
        "dark_breakbeat",
        t(
            CLUB,
            &[("percussive", 2), ("club", 1), ("electric", 1), ("sinister", 1)],
            130.0,
            150.0,
        ),
    ),
    (
        "drift_phonk",
        t(
            CLUB,
            &[("sinister", 2), ("club", 1), ("low", 1), ("electric", 1)],
            120.0,
            140.0,
        ),
    ),
    (
        "horror_trap",
        t(CLUB, &[("sinister", 2), ("club", 1), ("low", 1)], 130.0, 150.0),
    ),
    (
        "industrial",
        t(
            CLUB,
            &[("percussive", 2), ("electric", 1), ("sinister", 1), ("rock", 1)],
            118.0,
            134.0,
        ),
    ),
];

/// The fit of the roster fighter `name` ("BASS CLARINET"), or `None` for a name that has none.
pub fn fighter_fit(name: &str) -> Option<&'static FighterFit> {
    FIGHTERS.iter().find(|(n, _)| *n == name).map(|(_, fit)| fit)
}

/// The fit of the theme `name` ("dark_tango"), or `None` for a name that has none.
pub fn theme_fit(name: &str) -> Option<&'static ThemeFit> {
    THEMES.iter().find(|(n, _)| *n == name).map(|(_, fit)| fit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_roster_fighter_has_a_fit() {
        for name in crate::shown_order() {
            let fit = fighter_fit(name).unwrap_or_else(|| panic!("{name} has a fit"));
            assert!((3..=5).contains(&fit.traits.len()), "{name}: 3 to 5 traits");
            assert!(
                fit.traits.iter().all(|t| TRAITS.contains(t)),
                "{name}: traits from the list"
            );
            assert!((1..=3).contains(&fit.energy), "{name}: energy 1 to 3");
        }
    }

    #[test]
    fn every_theme_has_a_fit() {
        for name in crate::theme_names() {
            let fit = theme_fit(name).unwrap_or_else(|| panic!("{name} has a fit"));
            assert!(CATEGORIES.contains(&fit.category), "{name}: a known category");
            assert!(
                fit.likes.iter().all(|(t, w)| TRAITS.contains(t) && (1..=2).contains(w)),
                "{name}: likes from the list, weighted 1 or 2"
            );
            assert!(fit.tempo.0 < fit.tempo.1, "{name}: a tempo range");
            if let Some(card) = crate::theme_card(name) {
                let bpm = |bars: u32| f64::from(bars) * 240.0 / 61.0;
                let (lo, hi) = (bpm(card.bars.0), bpm(card.bars.1));
                assert!(lo <= fit.tempo.0 && fit.tempo.1 <= hi, "{name}: tempo inside its bars");
            }
        }
    }
}
