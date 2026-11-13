//! Theme: `ska_punk`, one of the older episodes' beds (milestone 4), ported in its Task 28 against its reference
//! mood (`tools/studio_export/older/moods/ska_punk.py`, the bed moved from
//! `projects/ballfight_bassoon_vs_barisax/audio.py`, #39 v4). Not ported yet: its port fills this folder: the card,
//! the options, the voices its bed plays, the voicings and the classic bed (`classic.rs`). Until then the registry
//! lists it but does not offer it, and its bed is refused in words.

mod classic;

use super::{NOT_PORTED_CARD, Theme, no_own_voice};

/// The theme as the registry lists it.
pub static THEME: Theme = Theme {
    name: "ska_punk",
    ported: false,
    card: &NOT_PORTED_CARD,
    options: &[],
    voices: &[],
    own_voice: no_own_voice,
    classic: classic::classic,
};
