//! PICCOLO's card: what the Match-up corner card and the Music screen say about it.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Piercing note",
    text: "Every beat the piccolo fires one razor-thin high note that ricochets off the ropes and pierces straight \
           through its enemy. Every second hit that lands gives its notes one more bounce, up to five.",
    abilities: [
        "Trill: fires twice as fast for 8 beats",
        "Over the band: for 8 beats its notes cut through shields",
        "Glass shatter: the ropes crack and 24 shards fly in, below 25 HP",
    ],
    colour: "Lemon",
    usually_plays: "Melody",
    music_title: "Piccolo, the sharpshooter",
    music_text: "The highest voice in the band: every shot is a grace note and its line's note, every bounce a quick \
                 tick up the scale, and a pierce rings out on a chord tone.",
    can_play_bass: false,
    // There is no piccolo emoji: the flute's.
    emoji: "\u{1FA88}",
    tags: &["#piccolo", "#woodwinds", "#marchingband"],
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_is_written() {
        let c = card().expect("PICCOLO's card");
        for text in [
            c.weapon,
            c.text,
            c.abilities[0],
            c.abilities[1],
            c.abilities[2],
            c.colour,
            c.usually_plays,
        ] {
            assert!(!text.trim().is_empty());
            assert!(text.starts_with(|ch: char| ch.is_uppercase()), "{text:?}");
            assert_ne!(text, text.to_uppercase(), "{text:?} shouts");
            for retired in bf_core::fightfile::RETIRED {
                assert!(!text.to_uppercase().contains(retired), "{text:?}");
            }
        }
        assert!(c.abilities[2].ends_with("below 25 HP"));
        assert!(c.tags.iter().all(|t| t.starts_with('#') && *t == t.to_lowercase()));
        assert_eq!(crate::fighters::card("PICCOLO"), Some(c));
    }
}
