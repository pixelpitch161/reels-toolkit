//! MELLOPHONE's card: what the Match-up corner card and the Music screen say about it.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Slapback",
    text: "Every two beats the bell blows a wave of sound at the enemy. A wave that misses lands on the ropes and \
           echoes back half a beat later, aimed again and louder. Every hit adds another echo.",
    abilities: [
        "Horns up: three quick stabs straight at the enemy, the last one stuns",
        "Press box: for 8 beats every echo comes back as two",
        "Stadium echo: one giant wave that laps the ring for two bars, below 25 HP",
    ],
    colour: "Band royal",
    usually_plays: "Melody",
    music_title: "Mellophone, the stadium echo",
    music_text: "A bright drum-corps horn: every note comes back off the far wall a beat late, louder, so its lines \
                 answer themselves. Give it a line and it takes the melody.",
    can_play_bass: false,
    emoji: "\u{1F4EF}",
    tags: &["#marchingband", "#drumcorps", "#bandkids"],
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
