//! Theme: `dark_disco` (`projects/ballfight/kit/moods/dark_disco.py`). Not ported yet: its port (milestone 3, Template T) fills
//! this folder: the card, the options, the voices its bed plays, the voicings and the classic bed (`classic.rs`).

mod classic;

use super::{NOT_PORTED_CARD, Theme};

/// The theme as the registry lists it.
pub static THEME: Theme = Theme {
    name: "dark_disco",
    ported: false,
    card: &NOT_PORTED_CARD,
    options: &[],
    voices: &[],
    classic: classic::classic,
};
