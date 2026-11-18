//! TARGET DUMMY's card (a developer fighter: the app never shows it, but a fight file's tools may ask).

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "None",
    text: "A straw training dummy on a post. It never attacks: it drifts round the ring and takes the hits, so the \
           other fighter's moves can be watched. It can still be knocked out.",
    abilities: ["None", "None", "None"],
    colour: "Straw grey",
    usually_plays: "Melody",
    music_title: "Target dummy, the knock",
    music_text: "A dry wooden knock on every hit it takes.",
    can_play_bass: false,
    emoji: "\u{1F3AF}",
    tags: &["#dev"],
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
