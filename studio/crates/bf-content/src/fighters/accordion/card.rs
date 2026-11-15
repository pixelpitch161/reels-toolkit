//! ACCORDION's card: what the Match-up corner card and the Music screen say about it.
//!
//! Sources: #30's roster entry (a bite every 2 beats, open 2 16ths, reach 0.9 growing by 0.07 per landed bite up to
//! 2.0; POLKA 8 beats of double speed every 5 bars, SQUEEZEBOX 6 beats of shield every 6 bars, FULL BELLOWS below 25
//! HP), its colour (magenta) and #30's audio notes (oom on the opening, a chord stab on the snap).

use crate::cards::FighterCard;

/// Its own tags, for the caption drafts.
const TAGS: [&str; 2] = ["#accordion", "#squeezebox"];

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Bellows bite",
    text: "Every two beats the two halves fly apart with the pleats stretched between them like jaws, aimed at \
           where the enemy is going, then snap shut. Every main bite that lands makes the jaws reach further.",
    abilities: [
        "Bites every beat for 8 beats",
        "The bellows fold shut round it as a shield",
        "The bellows stretch across the whole ring and clamp shut, below 25 HP.",
    ],
    colour: "Magenta",
    usually_plays: "Chords",
    music_title: "Accordion, oom-pah",
    music_text: "The bass button on every opening, a wheezy reed chord on every snap.",
    can_play_bass: false,
    emoji: "\u{1FA97}",
    tags: &TAGS,
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
