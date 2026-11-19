//! CONTRABASS CLARINET's card: what the Match-up corner card and the Music screen say about it. From its design
//! (`docs/studio/2026-11-16-contrabass-clarinet-design.md`): the FOGHORN every bar, FOG BANK, MOTORBOAT, SHIP'S HORN;
//! abyss teal; the lowest voice in the roster, so it plays the bass.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Foghorn",
    text: "Every bar its bell blows a wide, slow blast of sound that shoves the enemy across the ring. The real \
           damage lands when they slam into the wall, and every wall slam makes the blast reach further.",
    abilities: [
        "It sinks into a fog bank and attacks pass straight through it",
        "The low notes rattle like an outboard motor: double speed, a blast every two beats",
        "One enormous ship's horn blast across the whole ring, below 25 HP",
    ],
    colour: "Abyss teal",
    usually_plays: "Bass",
    music_title: "Contrabass clarinet, the foghorn",
    music_text: "The lowest voice in the roster: a two-note foghorn call on every blast, a sub boom on every wall \
                 slam and a putt-putt rattle while it races.",
    can_play_bass: true,
    // There is no clarinet emoji: the note, as for the other clarinets.
    emoji: "\u{1F3B5}",
    tags: &["#contrabassclarinet", "#woodwinds"],
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
