//! ACOUSTIC's card: what the Match-up corner card and the Music screen say about it. From the kit module's docstring
//! (`kit/fighters/acoustic.py`), the roster entry and `docs/ballfight/RULES.md`'s colour table.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Six-string rake",
    text: "On every beat six glowing strings sweep across the ring in front of the ball, a down-stroke then an \
           up-stroke, like a giant strum. A stroke that catches the enemy hits once and flicks it along the strings, \
           and every on-beat stroke that lands makes the strings a little longer.",
    abilities: [
        "A capo clamps on: it strums twice as fast for 8 beats",
        "Six single strings flick out at the enemy like whips, one after another",
        "All six strings sweep the whole ring in one giant strum, below 25 HP",
    ],
    colour: "Wood tan",
    usually_plays: "Chords",
    music_title: "Acoustic, the big strum",
    music_text: "Every stroke is a strum of the bar's chord in its own direction, down or up; a stroke that lands \
                 adds a bass pop, and as it gets hurt it loses strings and its up-strokes drop out.",
    can_play_bass: false,
    emoji: "\u{1F3B8}",
    tags: &["#acoustic", "#strumming", "#bandkids"],
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_is_written_and_names_no_retired_fighter() {
        let c = card().expect("the card is written");
        let mut texts = vec![
            c.weapon,
            c.text,
            c.colour,
            c.usually_plays,
            c.music_title,
            c.music_text,
            c.emoji,
        ];
        texts.extend(c.abilities);
        texts.extend(c.tags.iter().copied());
        for text in texts {
            assert!(!text.trim().is_empty());
            // a retired name as a word of its own (the names come from the fight file's list)
            for retired in bf_core::fightfile::RETIRED {
                let upper = text.to_uppercase();
                assert!(
                    !upper.split(|c: char| !c.is_alphanumeric()).any(|w| w == retired),
                    "{text}"
                );
            }
        }
        assert!(!c.can_play_bass, "its part has no bass role");
    }
}
