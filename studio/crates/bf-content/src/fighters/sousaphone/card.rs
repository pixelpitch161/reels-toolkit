//! SOUSAPHONE's card: what the Match-up corner card and the Music screen say about it. From the kit module's
//! docstring (`kit/fighters/sousaphone.py`), the roster entry and `docs/ballfight/RULES.md`'s colour table.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Drill set",
    text: "It never glides. Every two bars it marches eight steps straight at the enemy, trampling whatever is in \
           its way, then halts and fires a bell blast. Every hit that lands makes the blast longer.",
    abilities: [
        "The horn spins once in 1 beat and the blast sweeps all round",
        "Twice as fast for 8 beats",
        "A wall of sousaphones marches across the ring for 8 beats, below 25 HP",
    ],
    colour: "Marching crimson",
    usually_plays: "Bass",
    music_title: "Sousaphone, the marcher",
    music_text: "Its footsteps are the bass line: marching eighths, and a big blat on every blast.",
    can_play_bass: true,
    // There is no sousaphone emoji: the trumpet, as for the other brass.
    emoji: "\u{1F3BA}",
    tags: &["#sousaphone", "#marchingband", "#bandkids"],
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
        assert!(c.can_play_bass, "its footfalls are the bass line");
    }
}
