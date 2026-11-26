//! HORN's card: what the Match-up corner card and the Music screen say about it.
//!
//! Sources: the docstring of `kit/fighters/horn.py`, its roster entry (a call on every beat, up to 4 echo ghosts,
//! one more per landed blast; STOPPED HORN 6 beats, HORN RIP 8 beats, FINAL FLICK below 25 HP), the colour table of
//! `docs/ballfight/RULES.md` (copper) and its kit config `ballfight_trombone_vs_horn` (role `melody`).

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Echo call",
    text: "On every beat a round blast rings out of the ball. Echo ghosts trail along its path and repeat each call a \
           beat later. Every blast that lands adds a ghost, up to four.",
    abilities: [
        "A copper hand plugs the bell and blocks every hit for 6 beats",
        "Twice as fast for 8 beats: the echoes come back on the eighths",
        "A giant glove flicks the enemy into the ropes, below 25 HP",
    ],
    colour: "Copper",
    usually_plays: "Melody",
    music_title: "Horn, the echo call",
    music_text: "A round, mellow call on every beat, each echo quieter and darker, a hunting call in fifths.",
    can_play_bass: false,
    // No horn of its own among the emoji: the postal horn is the nearest.
    emoji: "\u{1F4EF}",
    tags: &["#bandkids", "#marchingband"],
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
        let c = card().expect("HORN's card");
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
        assert_eq!(crate::fighters::card("HORN"), Some(c));
    }
}
