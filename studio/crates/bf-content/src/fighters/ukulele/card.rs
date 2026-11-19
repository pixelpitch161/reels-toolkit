//! UKULELE's card: what the Match-up corner card and the Music screen say about it. From #33's episode
//! (`projects/ballfight_ukulele_vs_acoustic/`: SURF'S UP, CHUNK, SPEED STRUM, UKE-NADO) and its roster entry (pink).

use crate::cards::FighterCard;

/// Its own tags, for the caption drafts.
const TAGS: [&str; 3] = ["#ukulele", "#uke", "#strumming"];

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Surf's up",
    text: "Every other beat its strum raises a wave and the uke rides it at the enemy, a burst of speed behind a \
           curling wall of water that crashes into it once and pushes it into the ropes. Every wave that lands \
           makes the next one taller.",
    abilities: [
        "A chunky muted strum splashes out as a shockwave.",
        "For 8 beats it strums twice as fast: twice the waves.",
        "A spinning waterspout chases the enemy round the ring, below 25 HP.",
    ],
    colour: "Pink",
    usually_plays: "Chords",
    music_title: "Ukulele, the surf strum",
    music_text: "Every wave is a bright four-string strum of the bar's chord, down on the beat and up on the extra \
                 waves; a wave that lands plays a note climbing the scale over a crash of water.",
    can_play_bass: false,
    emoji: "\u{1F30A}",
    tags: &TAGS,
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
