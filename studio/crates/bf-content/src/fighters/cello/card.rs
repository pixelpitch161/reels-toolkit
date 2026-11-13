//! CELLO's card: what the Match-up corner card and the Music screen say about it. From the kit module's docstring
//! (`kit/fighters/cello.py`), the roster entry and `docs/ballfight/RULES.md`'s colour table.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "The embrace",
    text: "Every bar the cello rushes in with its bow arm out. If it reaches the enemy it grabs it, swings it round \
           for a beat and throws it toward the nearest rope; hitting the rope right after costs extra. Every \
           landed throw lets it reach further and throw harder.",
    abilities: [
        "An enemy it holds shakes and takes a hit every 16th, for 8 beats",
        "Bach's rocking prelude: a ring of note-waves every 16th for a bar, reaching far and short in turn",
        "It rises, glides across the ring like a swan and dives onto the enemy, below 25 HP",
    ],
    colour: "Rust",
    usually_plays: "Melody",
    music_title: "Cello, the singer",
    music_text: "It sings the tune with long bowed notes, fuller as it leads and thinner as it loses. A grab swells \
                 into a double stop, a throw is a sharp stab, and a rope slam stops the band for a beat.",
    can_play_bass: false,
    emoji: "\u{1F3BB}",
    tags: &["#cello", "#strings", "#orchestra"],
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
        assert!(!c.can_play_bass, "it sings the melody");
    }
}
