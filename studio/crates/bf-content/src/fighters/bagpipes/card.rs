//! BAGPIPES's card: what the Match-up corner card and the Music screen say about it.
//!
//! Sources: its reference kit module's docstring (`tools/studio_export/older/fighters/bagpipes.py`, #30 v3), its
//! roster entry (a cycle every bar: 2 beats of swell to 1.4x, +0.015 per landed hit up to 1.65x, a flight of 1.25
//! beats; WAR CRY a ring every 4 bars, HIGHLAND FLING 8 beats of double speed, BAG BURST below 25 HP), the colour
//! table of `docs/ballfight/RULES.md` (tartan green) and #30's README (its drone and chanter).

use crate::cards::FighterCard;

/// Its own tags, for the caption drafts.
const TAGS: [&str; 2] = ["#bagpipes", "#highlandpipes"];

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Bag inflate",
    text: "Every bar the bag swells to about one and a half times its size and weight, slow and heavy, then lets go \
           and rockets round the ring like a released balloon. Every hit that lands makes the next swell bigger and \
           the flight longer.",
    abilities: [
        "A tartan shockwave rolls out across the ring",
        "Two flights every bar for 8 beats",
        "The bag pops in a big blast of air, below 25 HP.",
    ],
    colour: "Tartan green",
    usually_plays: "Melody",
    music_title: "Bagpipes, the drone",
    music_text: "The drones swell as the bag fills, the chanter squeals up on every flight.",
    can_play_bass: false,
    emoji: "\u{1F3F4}\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F}",
    tags: &TAGS,
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
