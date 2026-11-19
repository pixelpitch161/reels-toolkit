//! CLARINET's card: what the Match-up corner card and the Music screen say about it. From `kit/fighters/clarinet.py`'s
//! docstring, its roster entry (TWELFTH JUMP; SQUEAK stuns for half a second, CLARION doubles the jumps for 8 beats,
//! RHAPSODY is five jumps below 25 HP), `docs/ballfight/RULES.md`'s colour table (lavender) and its role in its kit
//! configs (`line` in all three: the lower line under a melody).

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Twelfth jump",
    text: "It vanishes on the beat and reappears past the enemy. The line between its two spots cuts. Every cut \
           that lands lets it jump further.",
    abilities: [
        "A shrill ring that hits and stuns for half a second",
        "Twice as fast for 8 beats",
        "Five zig-zag jumps through the enemy, below 25 HP",
    ],
    colour: "Lavender",
    usually_plays: "Counter-line",
    music_title: "Clarinet, the leaper",
    music_text: "Low and liquid, one note a jump. A cut that lands adds the note a twelfth above.",
    can_play_bass: false,
    // There is no clarinet emoji: the kit's own clarinet caption uses the note.
    emoji: "\u{1F3B5}",
    tags: &["#bandkids", "#woodwinds"],
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
        let c = card().expect("CLARINET's card");
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
        assert_eq!(crate::fighters::card("CLARINET"), Some(c));
    }
}
