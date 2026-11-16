//! FLUGELHORN's card: what the Match-up corner card and the Music screen say about it. From its design
//! (`docs/studio/2026-11-16-flugelhorn-design.md`): the WINGBEAT every two beats, FLANKING CALL, FEATHER SHOWER,
//! TAKE WING; sage; the soft melody horn.

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Wingbeat",
    text: "Its name means wing horn. Every two beats two feathered wings sweep out of the ball and clap shut in \
           front of it, blowing the enemy back. Every clap that lands makes the wingspan longer.",
    abilities: [
        "It swoops round to the enemy's side and claps from there",
        "A flurry of feathers drifts across the ring and stings on touch",
        "Its wings spread across the whole ring for one giant clap, then it glides at double speed, below 25 HP",
    ],
    colour: "Sage",
    usually_plays: "Melody",
    music_title: "Flugelhorn, the wing horn",
    music_text: "The softest, darkest horn of the brass: a mellow call rising a sixth on every clap, a fluttering \
                 figure for the feathers and long, floating notes in its solo.",
    can_play_bass: false,
    // No flugelhorn emoji: the trumpet, its nearest relative.
    emoji: "\u{1F3BA}",
    tags: &["#flugelhorn", "#jazz"],
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
