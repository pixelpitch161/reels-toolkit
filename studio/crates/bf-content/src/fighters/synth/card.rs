//! SYNTH's card: what the Match-up corner card and the Music screen say about it.
//!
//! Sources: the docstring of its reference kit module (`tools/studio_export/older/fighters/synth.py`, #31 v3), its
//! roster entry (a plug every second beat while fewer cables are in than its slots, a voltage pulse down every cable
//! on the 8ths, the leash; LFO doubles the pulses for 3 s, SAWTOOTH lashes saw-wave arms for 4 beats, WHITE NOISE a
//! ring of static below 25 HP) and the series colour table (cyan).

use crate::cards::FighterCard;

/// Its own tags, for the caption drafts.
const TAGS: [&str; 2] = ["#synth", "#modularsynth"];

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Patch cables",
    text: "Every second beat a plug flies at the enemy and patches a cable in. Voltage pulses run down every cable \
           on the arpeggio, and the cables leash the enemy close unless it is flung hard enough to snap them.",
    abilities: [
        "Pulses run twice as fast for 3 seconds",
        "The cables go saw-wave and lash side to side, cutting",
        "A ring of static bursts out, below 25 HP.",
    ],
    colour: "Cyan",
    usually_plays: "Arpeggios",
    music_title: "Synth, the patch",
    music_text: "An arpeggio note on every pulse, toned like the cables' waveform, plug clicks, a wobble bass under \
                 LFO and a supersaw sweep for SAWTOOTH.",
    can_play_bass: false,
    emoji: "\u{1F39B}",
    tags: &TAGS,
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
