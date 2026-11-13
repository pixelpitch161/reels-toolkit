//! EUPHONIUM's card: what the Match-up corner card and the Music screen say about it. From
//! `kit/fighters/euphonium.py`'s docstring, its roster entry (BELL MORTAR every 2 beats; MARCHING ORDERS a salvo every
//! beat for 8 beats, IDENTITY CRISIS the enemy's last ability, NOT A TUBA a barrage below 25 HP), the series colour
//! table (amber) and its role in its kit config (the warm tenor melody of #38).

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Bell mortar",
    text: "Every two beats the upright bell lobs brass shells that arc high and land where the enemy is heading. A \
           shadow grows where each will fall. Every salvo that lands adds half a shell to the next.",
    abilities: [
        "Marching orders: a salvo every beat for 8 beats",
        "It copies the enemy's last move, and for a bar its ball shows the enemy's instrument in amber",
        "A rain of shells across the whole ring, below 25 HP",
    ],
    colour: "Amber",
    usually_plays: "Melody",
    music_title: "Euphonium, the artillery",
    music_text: "The warm tenor melody: every salvo plays its line's note on the beat, a landing a fat chord tone.",
    can_play_bass: false,
    // There is no euphonium emoji: the trumpet, its brass cousin.
    emoji: "\u{1F3BA}",
    tags: &["#bandkids", "#brass", "#euphonium"],
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The card is there, every text says something, in the interface's style (sentence case, no shouting, no
    /// retired name), with a line for each of its three abilities.
    #[test]
    fn the_card_is_written() {
        let c = card().expect("EUPHONIUM's card");
        let texts = [
            c.weapon,
            c.text,
            c.abilities[0],
            c.abilities[1],
            c.abilities[2],
            c.colour,
            c.usually_plays,
            c.music_title,
            c.music_text,
        ];
        for text in texts {
            assert!(!text.trim().is_empty());
            assert!(
                text.starts_with(|ch: char| ch.is_uppercase()),
                "{text:?} starts in lower case"
            );
            assert_ne!(text, text.to_uppercase(), "{text:?} shouts");
            for retired in bf_core::fightfile::RETIRED {
                assert!(!text.to_uppercase().contains(retired), "{text:?}");
            }
        }
        assert!(c.abilities[2].ends_with("below 25 HP"));
        assert!(!c.can_play_bass && !c.emoji.is_empty());
        assert!(c.tags.iter().all(|t| t.starts_with('#') && *t == t.to_lowercase()));
        // the dispatch finds it
        assert_eq!(crate::fighters::card("EUPHONIUM"), Some(c));
    }
}
