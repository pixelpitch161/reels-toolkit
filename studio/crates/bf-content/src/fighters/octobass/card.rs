//! OCTOBASS's card: what the Match-up corner card and the Music screen say about it.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Lever punch",
    text: "The octobass is so tall that fingers can't reach its strings, so big levers press them for the player. \
           Here the levers punch: felt-padded arms shoot straight at the enemy and stop dead on contact with a thud. \
           It starts with one arm and grows to three, one per string, as its punches land.",
    abilities: [
        "Foot pedal: a pedal plate under the enemy snaps up, throws it in the air and stuns it",
        "Seven levers: all seven levers punch one after another in a fan, the last one shoving the enemy away",
        "Below hearing: the music drops out while one slow ring rolls out, then a huge boom, below 25 HP",
    ],
    colour: "Deep umber",
    usually_plays: "Bass",
    music_title: "Octobass, the lowest strings",
    music_text: "It plays the bottom of the band, an octave under the bass line, long bowed notes and heavy plucks. \
                 Each punch is a wooden clunk, the pedal a metal snap, and below hearing is a moment of near silence \
                 before the boom.",
    can_play_bass: true,
    emoji: "\u{1F3BB}",
    tags: &["#octobass", "#strings", "#lowestnote"],
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
        let c = card().expect("OCTOBASS's card");
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
        assert_eq!(crate::fighters::card("OCTOBASS"), Some(c));
    }
}
