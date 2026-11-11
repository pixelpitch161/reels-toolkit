//! ELECTRIC's card: what the Match-up corner card and the Music screen say about it. Written with its port
//! (Task 23).

use crate::cards::FighterCard;

/// Its card, or `None` until it is written.
pub static CARD: Option<FighterCard> = None;

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
