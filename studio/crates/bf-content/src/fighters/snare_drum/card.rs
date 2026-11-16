//! SNARE DRUM's card: what the Match-up corner card and the Music screen say about it.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Paradiddle",
    text: "Two drumsticks ride the snare and jab at its enemy on a paradiddle, right left right right, left right left \
           left, every eighth note. The first stroke of each four is an accent and hits harder. Every stroke that \
           lands makes the sticks reach further.",
    abilities: [
        "Flam: for 8 beats every stroke lands twice, a grace note and the main one",
        "Rimshot: the next stroke to land cracks off the rim and stuns the enemy",
        "Buzz roll: the sticks blur into a buzzing disc that grinds anything near, below 25 HP",
    ],
    colour: "Cherry sparkle",
    usually_plays: "Rhythm",
    music_title: "Snare drum, the paradiddle",
    music_text: "Every stroke it plays is a snare tap on the beat grid, the accents louder, a flam a grace tap a hair \
                 early, a rimshot a dry crack and a buzz roll a tight press roll.",
    can_play_bass: false,
    emoji: "\u{1F941}",
    tags: &["#snaredrum", "#percussion", "#drumline"],
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_is_written() {
        let c = card().expect("SNARE DRUM's card");
        for text in [
            c.weapon,
            c.text,
            c.abilities[0],
            c.abilities[1],
            c.abilities[2],
            c.colour,
            c.usually_plays,
        ] {
            assert!(!text.trim().is_empty());
            assert!(text.starts_with(|ch: char| ch.is_uppercase()), "{text:?}");
            assert_ne!(text, text.to_uppercase(), "{text:?} shouts");
            for retired in bf_core::fightfile::RETIRED {
                assert!(!text.to_uppercase().contains(retired), "{text:?}");
            }
        }
        assert!(c.abilities[2].ends_with("below 25 HP"));
        assert!(c.tags.iter().all(|t| t.starts_with('#') && *t == t.to_lowercase()));
        assert_eq!(crate::fighters::card("SNARE DRUM"), Some(c));
    }
}
