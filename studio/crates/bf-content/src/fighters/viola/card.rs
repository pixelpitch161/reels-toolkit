//! VIOLA's card: what the Match-up corner card and the Music screen say about it. From its reference kit module's
//! docstring (`tools/studio_export/older/fighters/viola.py`, #42), the roster entry and `docs/ballfight/RULES.md`'s
//! colour table.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Counting rests",
    text: "The viola rests for three beats, counting them out loud, and takes only half damage while it waits. On \
           beat four a giant whole note appears and rolls at the enemy like a heavy wheel for a whole bar, crushing \
           it once and flattening its darts on the way. Every note that lands makes the next one bigger.",
    abilities: [
        "A double stop: two whole notes roll out at once",
        "For two bars the rests shrink to one beat: a note every other beat, and the viola finally gets the tune",
        "A rimshot, the crowd laughs, the enemy freezes for a beat while the biggest whole note drops on it, below \
         25 HP",
    ],
    colour: "Teal",
    usually_plays: "Inner voice",
    music_title: "Viola, the punchline",
    music_text: "It plays a fifth under the enemy's tune: a soft muted pluck on every counted rest and one long \
                 bowed note for every whole note, with a thud and a scratch when one crushes. Its joke gets a \
                 ba-dum-tss and a laughing crowd.",
    can_play_bass: false,
    emoji: "\u{1F3BB}",
    tags: &["#viola", "#strings", "#orchestra"],
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
        assert!(!c.can_play_bass, "it plays the inner voice, not the bass");
    }
}
