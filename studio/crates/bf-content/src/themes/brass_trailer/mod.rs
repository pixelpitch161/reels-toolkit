//! Theme: `brass_trailer` (`projects/ballfight/kit/moods/brass_trailer.py`). Not ported yet: its port (milestone 3, Template T) fills
//! this folder: the card, the options, the voices its bed plays, the voicings and the classic bed (`classic.rs`).

mod classic;

use super::{NOT_PORTED_CARD, Theme};

/// The theme as the registry lists it.
pub static THEME: Theme = Theme {
    name: "brass_trailer",
    ported: false,
    card: &NOT_PORTED_CARD,
    options: &[],
    voices: &[],
    classic: classic::classic,
};
