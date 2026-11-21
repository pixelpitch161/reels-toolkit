//! BASS GUITAR's card: what the Match-up corner card and the Music screen say about it.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Pick toss",
    text: "A four-string electric bass played with a pick. Every two beats it throws a big pick that swings out on a \
           curve, clips its enemy and loops back to the ball. Every pick that lands adds to the next throw, up to \
           three picks in a fan.",
    abilities: [
        "Fret slide: fret marks light up the line, then the ball dashes along it and stops dead on the enemy",
        "Headstock swing: the neck grows long and swings round like a club, stopping dead on the enemy",
        "Slingshot: two strings tie to the corner posts and fire the ball across the ring, below 25 HP",
    ],
    colour: "Ultraviolet",
    usually_plays: "Bass",
    music_title: "Bass guitar, picked",
    music_text: "Every pick it throws is a picked root note on the beat, a fret slide is a slide up the neck, a \
                 headstock swing a big low power chord, and the slingshot a string stretched sharp and let go.",
    can_play_bass: true,
    emoji: "\u{1F3B8}",
    tags: &["#bassguitar", "#bassist", "#rockbass"],
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
        let c = card().expect("BASS GUITAR's card");
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
        }
        assert!(c.abilities[2].ends_with("below 25 HP"));
        assert!(c.tags.iter().all(|t| t.starts_with('#') && *t == t.to_lowercase()));
        assert_eq!(crate::fighters::card("BASS GUITAR"), Some(c));
    }
}
