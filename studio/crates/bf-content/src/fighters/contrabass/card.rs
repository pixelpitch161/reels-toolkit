//! CONTRABASS's card: what the Match-up corner card and the Music screen say about it. From the kit module's
//! docstring (`kit/fighters/contrabass.py`), the roster entry, its kit config (`ballfight_cello_vs_contrabass`:
//! the bass) and `docs/ballfight/RULES.md`'s colour table.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Walking bass",
    text: "It never glides: it takes one heavy step toward the enemy on every beat. While it steps nothing knocks it \
           back or holds it, and every footfall sends a small quake across the floor. Every quake that lands makes \
           the next one reach further.",
    abilities: [
        "A sharp slap that knocks the enemy away and breaks a grab; it waits until the enemy is close",
        "Twice as fast for 8 beats: it walks on the eighths",
        "A stampede for 6 beats: it charges the enemy, tramples it and shakes the floor, below 25 HP",
    ],
    colour: "Steel blue",
    usually_plays: "Bass",
    music_title: "Contrabass, the walker",
    music_text: "Its footfalls are the walking bass line, plucked on the beat, with slaps and a bowed elephant tune.",
    can_play_bass: true,
    // There is no double bass emoji: the violin, as for the other strings.
    emoji: "\u{1F3BB}",
    tags: &["#doublebass", "#walkingbass", "#jazz"],
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
        assert!(c.can_play_bass, "its footfalls are the walking bass line");
    }
}
