//! TRUMPET's card: what the Match-up corner card and the Music screen say about it (from the kit module's
//! docstring and the roster).

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Valve combo",
    text: "Every two beats three pistons punch out of the ball, one after another, and the third hits harder. Every \
           punch that lands lets them reach further.",
    abilities: [
        "A ring that hits",
        "Twice as fast for 8 beats",
        "Charges for a beat, then a giant beam, below 25 HP",
    ],
    colour: "Yellow",
    usually_plays: "Melody",
    music_title: "Trumpet, the fanfare",
    music_text: "Three bright notes on every combo, rising calls and a screaming high C. Give it a line and it takes \
                 the melody.",
    can_play_bass: false,
    emoji: "\u{1F3BA}",
    tags: &["#bandkids", "#marchingband"],
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
