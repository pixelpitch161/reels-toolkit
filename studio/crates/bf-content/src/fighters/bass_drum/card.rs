//! BASS DRUM's card: what the Match-up corner card and the Music screen say about it. From the kit module's
//! docstring (`kit/fighters/bass_drum.py`), the roster entry and `docs/ballfight/RULES.md`'s colour table.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Steamroller",
    text: "Every bar it tips onto its side and rolls across the ring like a wheel at the enemy, a dust trail \
           showing the line first. Nothing knocks it off a roll, and the first contact is a huge BOOM. Every BOOM \
           that lands makes it roll faster and grow bigger.",
    abilities: [
        "Two quick booms, lub-dub: two shockwaves back to back",
        "It rolls nonstop for a bar, zig-zagging at the enemy",
        "Five cannon booms land on the enemy's spot, one a beat, below 25 HP",
    ],
    colour: "Gunmetal",
    usually_plays: "The beat",
    music_title: "Bass drum, the steamroller",
    music_text: "It is the band's bass drum: a kick on every beat on the chord's root, a drum roll into every BOOM, \
                 and its five cannons play the opening of an old Russian hymn.",
    can_play_bass: false,
    emoji: "\u{1F941}",
    tags: &["#bassdrum", "#drumline", "#bandkids"],
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
        assert!(!c.can_play_bass, "its part is the beat whatever its role");
    }
}
