//! Theme: `drift_phonk`, one of the older episodes' beds (milestone 4), ported in its Task 19 against its reference
//! mood (`tools/studio_export/older/moods/drift_phonk.py`, the bed moved from `projects/ballfight/audio.py`, #27).
//! Not ported yet: its port fills this folder: the card, the options, the voices its bed plays, the voicings and
//! the classic bed (`classic.rs`). Until then the registry lists it but does not offer it, and its bed is refused
//! in words.

mod classic;

use super::{NOT_PORTED_CARD, Theme, no_own_voice};

/// The theme as the registry lists it.
pub static THEME: Theme = Theme {
    name: "drift_phonk",
    ported: false,
    card: &NOT_PORTED_CARD,
    options: &[],
    voices: &[],
    own_voice: no_own_voice,
    classic: classic::classic,
};
