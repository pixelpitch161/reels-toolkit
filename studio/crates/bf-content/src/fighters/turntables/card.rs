//! TURNTABLES's card: what the Match-up corner card and the Music screen say about it.
//!
//! Sources: the docstring of its reference kit module (`tools/studio_export/older/fighters/turntables.py`, #31 v3),
//! its roster entry (a scratch every 4 beats, 0.06 stronger per landed rope slam; SCRATCH 8 beats at twice the rate,
//! REWIND undoes the last 2 s of damage, BASS DROP below 25 HP), the series colour table (lime) and #31's captions.

use crate::cards::FighterCard;

/// Its own tags, for the caption drafts (#31's captions).
const TAGS: [&str; 2] = ["#turntablism", "#dj"];

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "The ring is the record",
    text: "Every fourth beat the DJ scratches the record the ring sits on: the floor jerks and flings the enemy into \
           the ropes. Every rope slam makes the next scratches stronger.",
    abilities: [
        "Scratches twice as often for 8 beats",
        "Rewinds: undoes the last 2 seconds of damage and zips back along its own path",
        "A giant record drops on the enemy, below 25 HP.",
    ],
    colour: "Lime",
    usually_plays: "Melody",
    music_title: "Turntables, the decks",
    music_text: "Scratches that follow the floor's jerks, organ stabs on every rope slam, a backspin rewind.",
    can_play_bass: false,
    emoji: "\u{1F39A}",
    tags: &TAGS,
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
