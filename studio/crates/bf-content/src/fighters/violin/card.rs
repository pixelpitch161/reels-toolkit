//! VIOLIN's card: what the Match-up corner card and the Music screen say about it.
//!
//! Sources: its roster entry (the bow spun like a rapier, spinning faster with every landed hit; PIZZICATO five
//! darts, TREMOLO a frantic double-speed bow, SCREECH a shockwave below 25 HP), the colour table (violet) and its
//! episodes (#28 `ballfight_drums_vs_violin`, #42 `ballfight_viola_vs_violin`: the violin plays the melody).

use crate::cards::FighterCard;

/// Its own tags, for the caption drafts.
const TAGS: [&str; 2] = ["#violin", "#strings"];

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Spinning bow",
    text: "The bow whirls round the violin like a rapier, slashing anything it sweeps past. Every landed slash spins \
           it faster, up to a blur.",
    abilities: [
        "Five pizzicato darts fan out at the enemy.",
        "For a few seconds the bow saws at double speed.",
        "A beginner's screech rings out as a shockwave, below 25 HP.",
    ],
    colour: "Violet",
    usually_plays: "Melody",
    music_title: "Violin, the spinning bow",
    music_text: "Every quarter turn of the bow plays the next note of the line, pizzicato for the darts and a squeal \
                 for the screech.",
    can_play_bass: false,
    emoji: "\u{1F3BB}",
    tags: &TAGS,
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
