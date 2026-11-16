//! MARIMBA's card: what the Match-up corner card and the Music screen say about it.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Mallet run",
    text: "Every beat the marimba banks a mallet off the ropes at its enemy. Each wall it bounces off is the next note \
           up the scale and makes it hit harder. Every fourth hit that lands puts another mallet in the air, up to four.",
    abilities: [
        "Tremolo: for 4 beats every mallet buzzes in a pair and lands twice",
        "Dead stroke: the next mallet to land pins the enemy in place",
        "Five octaves: 61 mallets fan out in one rising sweep, below 25 HP",
    ],
    colour: "Jacaranda",
    usually_plays: "Melody",
    music_title: "Marimba, the mallet run",
    music_text: "Every throw plays its line's note, every bounce the next note up the scale, and a hit lands on a \
                 chord tone.",
    can_play_bass: false,
    // There is no marimba emoji: the musical keyboard's nearest cousin, the notes.
    emoji: "\u{1F3B5}",
    tags: &["#marimba", "#percussion", "#mallets"],
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
        let c = card().expect("MARIMBA's card");
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
        assert_eq!(crate::fighters::card("MARIMBA"), Some(c));
    }
}
