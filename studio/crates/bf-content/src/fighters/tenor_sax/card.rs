//! TENOR SAX's card: what the Match-up corner card and the Music screen say about it. From the kit module's
//! docstring (`kit/fighters/tenor_sax.py`), the roster entry and `docs/ballfight/RULES.md`'s colour table.

use crate::cards::FighterCard;

/// Its card. (No text names the instrument by its second word alone: the fight file retires that word, and the test
/// below reads every text word by word.)
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Crowd pleaser",
    text: "Every bar it strikes a pose and honks, its hype meter fills and the crowd hurls tomatoes, roses and foam \
           fingers at the enemy from the stands. They only reach so far in, so the middle of the ring is safe. Every \
           volley that lands brings more throws, thrown harder and further.",
    abilities: [
        "Struts down a spotlit runway across the ring for 4 beats, honking on every beat",
        "The crowd hushes for 2 beats, then throws twice as hard",
        "Leaps into the crowd, surfs along the stands and is fired back like a cannonball, below 25 HP",
    ],
    colour: "Burnt orange",
    usually_plays: "Melody",
    music_title: "Tenor, the crowd pleaser",
    music_text: "A husky, honking low line: a big honk on every pose, a note of its riff on every throw, a scream \
                 into the crowd's roar on the stage dive.",
    can_play_bass: false,
    emoji: "\u{1F3B7}",
    tags: &["#tenorsax", "#jazz", "#bandkids"],
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
