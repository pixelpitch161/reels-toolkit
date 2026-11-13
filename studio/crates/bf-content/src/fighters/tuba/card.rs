//! TUBA's card: what the Match-up corner card and the Music screen say about it. From the kit module's docstring
//! (`kit/fighters/tuba.py`), the roster entry and `docs/ballfight/RULES.md`'s colour table.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Oom-pah swallow",
    text: "Every four beats the bell breathes in (oom) and sucks the enemy toward it; if the enemy touches the bell \
           it is swallowed whole and rides inside. Two beats after the oom comes the pah: the tuba spits it out like \
           a cannonball, and it hurts more if it hits the ropes. Every gulp lets the bell reach further.",
    abilities: [
        "An oom-pah every 2 beats instead of 4, for 8 beats",
        "A brass dome that blocks every hit for 6 beats",
        "A sub note builds for 2 beats under where the enemy is heading, then the floor slams there, below 25 HP",
    ],
    colour: "Indigo",
    usually_plays: "Bass",
    music_title: "Tuba, the oom-pah",
    music_text: "Its ooms are the bass line: the root held on every oom, the fifth as a short blast on every pah, and \
                 the octave when it spits the enemy out.",
    can_play_bass: true,
    // There is no tuba emoji: the trumpet, as for the other brass.
    emoji: "\u{1F3BA}",
    tags: &["#tuba", "#brassband", "#bandkids"],
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
        assert!(c.can_play_bass, "its ooms are the bass line");
    }
}
