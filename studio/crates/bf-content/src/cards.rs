//! What the interface shows about fighters and themes (Task 26): the roster's sections in score order, the corner
//! card and the music text of each ported fighter, each theme's card, the words for each pair rule, and note names.
//!
//! Every text is the interface reference's own (sections 5.1 and 5.4).

/// A section of the band, as the roster groups its fighters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Woodwinds,
    Brass,
    Strings,
    Keys,
    Percussion,
}

/// What the Match-up corner card and the Music screen say about a ported fighter.
#[derive(Debug, PartialEq)]
pub struct FighterCard {
    /// The weapon's name, such as "Snake charmer".
    pub weapon: &'static str,
    /// The paragraph under the weapon's name.
    pub text: &'static str,
    /// The line under each ability's name: unlock 1 (LV2), unlock 2 (LV3), low (LOW).
    pub abilities: [&'static str; 3],
    /// The colour's name, such as "Silver".
    pub colour: &'static str,
    /// What it usually plays, such as "Melody".
    pub usually_plays: &'static str,
    /// The Music screen's title line, such as "Flute, the charmer".
    pub music_title: &'static str,
    /// The Music screen's line under it.
    pub music_text: &'static str,
    /// Whether it has a bass part (the other fighter then takes the melody).
    pub can_play_bass: bool,
    /// Its emoji, for the caption drafts.
    pub emoji: &'static str,
    /// Its own tags, for the caption drafts.
    pub tags: &'static [&'static str],
}

/// What the interface says about a theme.
#[derive(Debug, PartialEq)]
pub struct ThemeCard {
    /// Its name as shown, such as "Big top".
    pub display: &'static str,
    /// The line beside the theme picker.
    pub text: &'static str,
    /// Its key's note name, such as "E".
    pub key: &'static str,
    /// Its scale's name, such as "harmonic minor".
    pub scale: &'static str,
    /// The chord loops it has, eight chords each.
    pub chord_loops: &'static [[&'static str; 8]],
    /// The fewest and the most bars it can fill 61 s with.
    pub bars: (u32, u32),
}

/// Every roster fighter in score order, with its section.
const SCORE: [(&str, Section); 20] = [
    ("FLUTE", Section::Woodwinds),
    ("OBOE", Section::Woodwinds),
    ("CLARINET", Section::Woodwinds),
    ("BASS CLARINET", Section::Woodwinds),
    ("BASSOON", Section::Woodwinds),
    ("ALTO SAX", Section::Woodwinds),
    ("TENOR SAX", Section::Woodwinds),
    ("TRUMPET", Section::Brass),
    ("HORN", Section::Brass),
    ("TROMBONE", Section::Brass),
    ("EUPHONIUM", Section::Brass),
    ("TUBA", Section::Brass),
    ("SOUSAPHONE", Section::Brass),
    ("CELLO", Section::Strings),
    ("CONTRABASS", Section::Strings),
    ("ACOUSTIC", Section::Strings),
    ("ELECTRIC", Section::Strings),
    ("PIANO", Section::Keys),
    ("ORGAN", Section::Keys),
    ("BASS DRUM", Section::Percussion),
];

/// The tags both ported fighters carry.
const BAND_TAGS: [&str; 2] = ["#bandkids", "#marchingband"];

static FLUTE: FighterCard = FighterCard {
    weapon: "Snake charmer",
    text: "Cobras rise from dropped baskets and strike on the beat. Every hit that lands lets them reach further.",
    abilities: [
        "Twice as fast for 8 beats",
        "Lasts 4 beats",
        "A wave across the ring, below 25 HP",
    ],
    colour: "Silver",
    usually_plays: "Melody",
    music_title: "Flute, the charmer",
    music_text: "High and ornamented. Runs of eighths that fall back to the home note.",
    can_play_bass: false,
    emoji: "\u{1FA88}",
    tags: &BAND_TAGS,
};

static TROMBONE: FighterCard = FighterCard {
    weapon: "Slide lance",
    text: "The slide shoots out at the enemy on the beat, then pulls back. Every hit that lands makes it longer. It \
           can parry.",
    abilities: [
        "A ring that hits and slows",
        "Twice as fast for 8 beats",
        "A heavy slam, below 25 HP",
    ],
    colour: "Mint",
    usually_plays: "Bass",
    music_title: "Trombone, the slide",
    music_text: "Smears up into notes, oom-pah struts, low fanfares.",
    can_play_bass: true,
    // The series uses the trumpet: there is no trombone emoji.
    emoji: "\u{1F3BA}",
    tags: &BAND_TAGS,
};

static BIG_TOP: ThemeCard = ThemeCard {
    display: "Big top",
    text: "A sinister circus in a minor key. Leaves the bass and the top line to the fighters.",
    key: "E",
    scale: "harmonic minor",
    chord_loops: &[["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"]],
    bars: (24, 42),
};

/// The section a roster fighter sits in, or `None` for a name that is not in the roster.
pub fn section(name: &str) -> Option<Section> {
    SCORE.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}

/// Every roster fighter once, in score order (woodwinds, brass, strings, keys, percussion).
pub fn shown_order() -> Vec<&'static str> {
    SCORE.iter().map(|(n, _)| *n).collect()
}

/// The card of a ported fighter, or `None` for any other name.
pub fn card(name: &str) -> Option<&'static FighterCard> {
    match name {
        "FLUTE" => Some(&FLUTE),
        "TROMBONE" => Some(&TROMBONE),
        _ => None,
    }
}

/// The card of a theme (by its name in a fight file), or `None` for a name that is no theme.
pub fn theme_card(name: &str) -> Option<&'static ThemeCard> {
    match name {
        "big_top" => Some(&BIG_TOP),
        _ => None,
    }
}

/// What a pair rule does, in words, or `None` for a rule this app has no words for.
pub fn pair_rule_text(key: &str) -> Option<&'static str> {
    match key {
        "swat_cobras" => Some("A live lance knocks a risen cobra flat."),
        _ => None,
    }
}

/// A MIDI note's name with its octave, sharps for the black keys: 72 is "C5", 61 is "C#4".
pub fn note_name(midi: i32) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = i64::from(midi.div_euclid(12)) - 1;
    // rem_euclid of 12 is always 0 to 11
    let pitch = NAMES[midi.rem_euclid(12) as usize];
    format!("{pitch}{octave}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PORTED, roster, theme_names};

    #[test]
    fn every_roster_fighter_has_a_section() {
        let r = roster();
        assert_eq!(r.len(), 20);
        for f in r {
            assert!(section(&f.name).is_some(), "{} has no section", f.name);
        }
        let order = shown_order();
        assert_eq!(order.len(), 20);
        for f in r {
            assert_eq!(order.iter().filter(|n| **n == f.name).count(), 1, "{}", f.name);
        }
        let count = |s: Section| order.iter().filter(|n| section(n) == Some(s)).count();
        assert_eq!(
            [
                count(Section::Woodwinds),
                count(Section::Brass),
                count(Section::Strings),
                count(Section::Keys),
                count(Section::Percussion)
            ],
            [7, 6, 4, 2, 1]
        );
        assert_eq!(&order[..2], &["FLUTE", "OBOE"]);
        assert_eq!(order.last(), Some(&"BASS DRUM"));
        // sections come in order, each in one block
        let sections: Vec<Section> = order.iter().filter_map(|n| section(n)).collect();
        let mut blocks = sections.clone();
        blocks.dedup();
        assert_eq!(
            blocks,
            [
                Section::Woodwinds,
                Section::Brass,
                Section::Strings,
                Section::Keys,
                Section::Percussion
            ]
        );
        assert_eq!(section("KAZOO"), None);
        assert_eq!(section("flute"), None);
        assert_eq!(section(""), None);
    }

    #[test]
    fn ported_fighters_have_cards() {
        for name in PORTED {
            let c = card(name).unwrap_or_else(|| panic!("{name} has no card"));
            assert_eq!(c.tags, ["#bandkids", "#marchingband"], "{name}");
        }
        let flute = card("FLUTE").expect("flute");
        assert_eq!(flute.weapon, "Snake charmer");
        assert_eq!(
            flute.text,
            "Cobras rise from dropped baskets and strike on the beat. Every hit that lands lets them reach further."
        );
        assert_eq!(
            flute.abilities,
            [
                "Twice as fast for 8 beats",
                "Lasts 4 beats",
                "A wave across the ring, below 25 HP"
            ]
        );
        assert_eq!((flute.colour, flute.usually_plays), ("Silver", "Melody"));
        assert_eq!(flute.music_title, "Flute, the charmer");
        assert_eq!(
            flute.music_text,
            "High and ornamented. Runs of eighths that fall back to the home note."
        );
        assert!(!flute.can_play_bass);
        assert_eq!(flute.emoji, "\u{1FA88}");

        let trombone = card("TROMBONE").expect("trombone");
        assert_eq!(trombone.weapon, "Slide lance");
        assert_eq!(
            trombone.text,
            "The slide shoots out at the enemy on the beat, then pulls back. Every hit that lands makes it longer. It \
             can parry."
        );
        assert_eq!(
            trombone.abilities,
            [
                "A ring that hits and slows",
                "Twice as fast for 8 beats",
                "A heavy slam, below 25 HP"
            ]
        );
        assert_eq!((trombone.colour, trombone.usually_plays), ("Mint", "Bass"));
        assert_eq!(trombone.music_title, "Trombone, the slide");
        assert_eq!(
            trombone.music_text,
            "Smears up into notes, oom-pah struts, low fanfares."
        );
        assert!(trombone.can_play_bass);
        assert_eq!(trombone.emoji, "\u{1F3BA}");

        // the ranges the corner cards show come from the roster's registers
        for (name, range) in [("FLUTE", "C5 to A6"), ("TROMBONE", "E2 to F4")] {
            let f = roster().iter().find(|f| f.name == name).expect("in the roster");
            let (lo, hi) = f.register;
            assert_eq!(format!("{} to {}", note_name(lo), note_name(hi)), range);
        }
    }

    #[test]
    fn nobody_else_does() {
        for f in roster() {
            if !PORTED.contains(&f.name.as_str()) {
                assert!(card(&f.name).is_none(), "{} has a card", f.name);
            }
        }
        for name in ["", "flute", "Flute", "KAZOO", "ALTO SAX"] {
            assert!(card(name).is_none(), "{name:?}");
        }
    }

    #[test]
    fn retired_fighters_have_nothing() {
        for name in bf_core::fightfile::RETIRED {
            assert_eq!(section(name), None, "{name}");
            assert!(card(name).is_none(), "{name}");
            assert!(!shown_order().contains(&name), "{name}");
        }
    }

    #[test]
    fn every_theme_has_a_card() {
        for name in theme_names() {
            assert!(theme_card(name).is_some(), "{name} has no card");
        }
        let t = theme_card("big_top").expect("the big top");
        assert_eq!(t.display, "Big top");
        assert_eq!(
            t.text,
            "A sinister circus in a minor key. Leaves the bass and the top line to the fighters."
        );
        assert_eq!((t.key, t.scale), ("E", "harmonic minor"));
        assert_eq!(t.chord_loops, [["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"]]);
        assert_eq!(t.bars, (24, 42));
        for name in ["", "polka", "Big top", "big top"] {
            assert!(theme_card(name).is_none(), "{name:?}");
        }
    }

    #[test]
    fn pair_rules_have_words() {
        assert_eq!(
            pair_rule_text("swat_cobras"),
            Some("A live lance knocks a risen cobra flat.")
        );
        assert_eq!(pair_rule_text(""), None);
        assert_eq!(pair_rule_text("swat cobras"), None);
        // every rule the table switches on has words
        for key in crate::pair_rules("FLUTE", "TROMBONE").keys() {
            assert!(pair_rule_text(key).is_some(), "{key}");
        }
    }

    #[test]
    fn note_names() {
        let names: Vec<String> = [72, 93, 40, 65, 61].into_iter().map(note_name).collect();
        assert_eq!(names, ["C5", "A6", "E2", "F4", "C#4"]);
        assert_eq!(note_name(0), "C-1");
        assert_eq!(note_name(127), "G9");
        assert_eq!(note_name(-1), "B-2");
        // no panic at the ends of the range
        assert_eq!(note_name(i32::MIN), "E-178956972");
        assert_eq!(note_name(i32::MAX), "G178956969");
    }
}
