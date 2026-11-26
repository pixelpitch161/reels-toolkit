//! TIMPANI's card: what the Match-up corner card and the Music screen say about it.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Two kettles",
    text: "The timpani sets a big low kettle and a small high kettle in the ring and hops between them every other \
           beat. Each landing booms that kettle: the low one sends a wide, slow ring, the high one a quick, small one. \
           An enemy caught by both kettles within a beat takes the interval hit on top. The enemy can shove the \
           kettles around.",
    abilities: [
        "Pedal slide: both kettles glide at the enemy and squeeze it",
        "Cross-sticking: for 4 beats it hops every 8th note",
        "Full set: two more kettles drop into the corners, below 25 HP",
    ],
    colour: "Silver",
    usually_plays: "Roots and fifths",
    music_title: "Timpani, the two kettles",
    music_text: "The low kettle plays the chord's root and the high kettle its fifth, so every hop is the bass line; \
                 the interval hit sounds both at once and the pedal slide is a glissando.",
    can_play_bass: true,
    emoji: "\u{1F941}",
    tags: &["#timpani", "#percussion", "#orchestra"],
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
        let c = card().expect("TIMPANI's card");
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
                assert!(
                    !text
                        .to_uppercase()
                        .split(|ch: char| !ch.is_alphanumeric())
                        .any(|w| w == retired),
                    "{text:?}"
                );
            }
        }
        assert!(c.abilities[2].ends_with("below 25 HP"));
        assert!(c.tags.iter().all(|t| t.starts_with('#') && *t == t.to_lowercase()));
        assert_eq!(crate::fighters::card("TIMPANI"), Some(c));
    }
}
