//! OBOE's card: what the Match-up corner card and the Music screen say about it. The texts come from the kit
//! module's docstring (`kit/fighters/oboe.py`), the roster's numbers, `docs/ballfight/RULES.md`'s colour table and
//! the fighter's role in its two kit episodes.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Tuning A",
    text: "Every bar it sounds its A as a narrow beam. A beam that lands knocks the enemy out of tune for 2 bars: the \
           enemy hits weaker, and every eighth note of the oboe stings it. Each beam that lands puts the enemy \
           further out of tune.",
    abilities: [
        "Two blades snap shut on an enemy close by, half a beat after they open",
        "The beam is held for 4 beats and follows the enemy",
        "A sour gust that puts the enemy fully out of tune, below 25 HP",
    ],
    colour: "Olive",
    usually_plays: "Melody",
    music_title: "Oboe, the nasal reed",
    music_text: "High and nasal. Its tuning A with every beam, running eighths as it stings.",
    can_play_bass: false,
    // The series uses the flute: there is no oboe emoji.
    emoji: "\u{1FA88}",
    tags: &["#bandkids", "#marchingband"],
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}

#[cfg(test)]
mod tests {
    use bf_core::fightfile::RETIRED;

    use super::*;

    /// Every text is there, in the interface's sentence case, free of the retired names; the low ability says when
    /// it fires.
    #[test]
    fn the_card_has_every_text() {
        let c = card().expect("OBOE has a card");
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
        texts.extend(c.tags);
        for t in &texts {
            assert!(!t.is_empty());
            for name in RETIRED {
                assert!(!t.to_uppercase().contains(&name.to_uppercase()), "{t}");
            }
        }
        for t in [c.weapon, c.text, c.music_title, c.music_text]
            .iter()
            .chain(&c.abilities)
        {
            assert!(t.starts_with(|ch: char| ch.is_uppercase()), "{t}");
        }
        assert!(c.abilities[2].ends_with("below 25 HP"));
        assert!(!c.can_play_bass);
        assert!(c.tags.iter().all(|t| t.starts_with('#')));
    }
}
