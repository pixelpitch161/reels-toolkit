//! CONTRABASSOON's card: what the Match-up corner card and the Music screen say about it (its moves from
//! `docs/studio/2026-11-16-contrabassoon.md`, colour oxblood).

use crate::cards::FighterCard;

/// Its own tags, for the caption drafts.
const TAGS: [&str; 2] = ["#contrabassoon", "#doublereed"];

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "U-bend",
    text: "Five and a half metres of tube has to go somewhere. Every bar it lays a bent piece of its tubing in the \
           ring as a short wall, right in the enemy's path. The enemy gets hurt bouncing off a pipe; the \
           contrabassoon speeds up off them. Every pipe hit that lands makes the next one hurt more.",
    abilities: [
        "Every pipe end sprays water from the water key",
        "Hot pipes: for 8 beats every pipe hits twice as hard",
        "Its whole tube boxes the enemy in for a bar, below 25 HP.",
    ],
    colour: "Oxblood",
    usually_plays: "Bass",
    music_title: "Contrabassoon, the plumbing",
    music_text: "A low grumbling reed under everything, a hollow clank on every pipe, a riff that steps down in \
                 fourths.",
    can_play_bass: true,
    // There is no bassoon emoji: the double reeds' caption uses the flute.
    emoji: "\u{1FA88}",
    tags: &TAGS,
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
