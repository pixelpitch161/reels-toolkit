//! BASS CLARINET's card: what the Match-up corner card and the Music screen say about it. From
//! `kit/fighters/bass_clarinet.py`'s docstring, its roster entry (ENDPIN ANCHOR every bar; SLAP TONGUE a heavy
//! knockback smack, MULTIPHONICS a ghost twin for 8 beats, LIGHTS OUT a bar in the shadows below 25 HP),
//! `docs/ballfight/RULES.md`'s colour table (slate blue) and its roles in its kit configs (a counter-line an octave
//! over the bari's bass in #40, the growling bass line in #55: it can play the bass).

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Endpin anchor",
    text: "Every bar it drives its floor peg into the canvas and whirls round it like a hammer-thrower. The whirl \
           smashes whatever it sweeps over. Every smash that lands widens the orbit.",
    abilities: [
        "The reed slaps out like a paddle: a heavy knockback smack",
        "A ghost twin whirls on the far side of the peg for 8 beats",
        "The arena dims, it sinks into the shadows for a bar, then erupts, below 25 HP",
    ],
    colour: "Slate blue",
    usually_plays: "Counter-line",
    music_title: "Bass clarinet, the hammer-thrower",
    music_text: "A chalumeau growl, the line on every 8th of a whirl. A smash pops a slap tongue.",
    can_play_bass: true,
    // There is no clarinet emoji: the note, as for the CLARINET.
    emoji: "\u{1F3B5}",
    tags: &["#bandkids", "#woodwinds"],
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The card is there, every text says something, in the interface's style (sentence case, no shouting, no
    /// retired name), with a line for each of its three abilities.
    #[test]
    fn the_card_is_written() {
        let c = card().expect("BASS CLARINET's card");
        let texts = [
            c.weapon,
            c.text,
            c.abilities[0],
            c.abilities[1],
            c.abilities[2],
            c.colour,
            c.usually_plays,
            c.music_title,
            c.music_text,
        ];
        for text in texts {
            assert!(!text.trim().is_empty());
            assert!(
                text.starts_with(|ch: char| ch.is_uppercase()),
                "{text:?} starts in lower case"
            );
            assert_ne!(text, text.to_uppercase(), "{text:?} shouts");
            for retired in bf_core::fightfile::RETIRED {
                assert!(!text.to_uppercase().contains(retired), "{text:?}");
            }
        }
        assert!(c.abilities[2].ends_with("below 25 HP"));
        assert!(c.can_play_bass && !c.emoji.is_empty());
        assert!(c.tags.iter().all(|t| t.starts_with('#') && *t == t.to_lowercase()));
        // the dispatch finds it
        assert_eq!(crate::fighters::card("BASS CLARINET"), Some(c));
    }
}
