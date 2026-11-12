//! PIANO's card: what the Match-up corner card and the Music screen say about it.
//!
//! Sources: the docstring of `kit/fighters/piano.py`, its roster entry (PIANO ROLL; SUSTAIN for 4 beats, hanging
//! for 1; GLISSANDO, 15 keys over 1 beat; 88 KEYS, 40 keys at once, then 4 beats without a row), the colour table of
//! `docs/ballfight/RULES.md` (blue) and its role in its kit config (`ballfight_piano_vs_organ`: the melody).

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Piano roll",
    text: "A paper roll along its side of the ring plays its melody. Every note fires a key across the ring. Every \
           fourth hit adds a note to its rows, up to 4.",
    abilities: [
        "For 4 beats its keys stop, hang for a beat like mines, then fly at the enemy",
        "A fan of 15 keys from the ball, over 1 beat",
        "40 keys at once across the whole ring, below 25 HP",
    ],
    colour: "Blue",
    usually_plays: "Melody",
    music_title: "Piano, the roll",
    music_text: "What you see is what you hear: the notes of its line are the keys it fires.",
    can_play_bass: false,
    emoji: "\u{1F3B9}",
    tags: &["#piano", "#pianist"],
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
