//! HARP's card: what the Match-up corner card and the Music screen say about it.
//!
//! Sources: #35's README and `audio.py` (its strings, PLUCK, ANGEL WINGS, 47 STRINGS, its arpeggios) and its roster
//! entry (a string every 2 beats, 1 kept at the start, +0.5 per landed crossing up to 7).

use crate::cards::FighterCard;

/// Its own tags, for the caption drafts.
const TAGS: [&str; 2] = ["#harp", "#harpist"];

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "String trap",
    text: "Every two beats the harp stretches a glowing string rope to rope across the ring, between itself and its \
           enemy. Whoever crosses an enemy string takes a hit and stumbles; the harp passes freely. Every crossing \
           that lands lets it keep more strings up at once.",
    abilities: [
        "A fan of seven plucked darts",
        "Angel wings and a halo shield it for 6 beats",
        "All 47 strings snap out in one huge wave, below 25 HP.",
    ],
    colour: "Pearl gold",
    usually_plays: "Melody",
    music_title: "Harp, the arpeggios",
    music_text: "Every new string is a rising arpeggio, every crossing rings the string high.",
    can_play_bass: false,
    // The harp emoji (U+1FA89) is too new for most fonts: the musical notes.
    emoji: "\u{1F3B6}",
    tags: &TAGS,
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
