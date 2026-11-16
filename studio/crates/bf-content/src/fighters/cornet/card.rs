//! CORNET's card: what the Match-up corner card and the Music screen say about it.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Plunger mute",
    text: "Every two beats a plunger on a cord flies at the enemy and sticks. Each beat it stays on it wahs and drags \
           them closer. Every stick makes the cord longer and the grip last longer.",
    abilities: [
        "A wobbling wah ring blocks hits for 6 beats",
        "A bowler hat that stops the enemy attacking for 4 beats",
        "Stop-time: three frozen beats of dashes, then the band comes back in, below 25 HP",
    ],
    colour: "Rose brass",
    usually_plays: "Melody",
    music_title: "Cornet, the plunger",
    music_text: "Mellow, talking wah-wah notes in a hot-jazz style, band stabs on every stop and a lyrical solo. Give \
                 it a line and it takes the melody.",
    can_play_bass: false,
    emoji: "\u{1F3BA}",
    tags: &["#bandkids", "#brassband", "#jazz"],
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
