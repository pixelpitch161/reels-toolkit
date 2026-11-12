//! ELECTRIC's card: what the Match-up corner card and the Music screen say about it. From the kit module's
//! docstring (`kit/fighters/electric.py`), the roster entry and `docs/ballfight/RULES.md`'s colour table.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Lightning riff",
    text: "Every bar it plays a riff, and every note is an instant bolt of lightning: from its headstock when the \
           enemy is close, or from the corner post nearest the enemy, since the posts are lightning rods. The enemy \
           is safe only away from both, and every two riffs that land add a note to the next one.",
    abilities: [
        "Distortion: for 8 beats the bolts fork and reach further",
        "Tapping: for a bar the second hand taps a bolt on every 8th",
        "It smashes itself on the floor: a shockwave and flying splinters, then it snaps back together, below 25 HP",
    ],
    colour: "Neon green",
    usually_plays: "Riffs",
    music_title: "Electric, the lightning riff",
    music_text: "Every bolt is a riff note through a fuzz and a cab; a bolt that lands hits it hard with a \
                 power-chord stab and a zap, a miss is palm-muted, and as it gets hurt its riff loses notes.",
    can_play_bass: false,
    emoji: "\u{26A1}",
    tags: &["#electric", "#riff", "#bandkids"],
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
