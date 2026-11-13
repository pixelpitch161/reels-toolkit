//! Theme: `villain_march`, one of the older episodes' beds (milestone 4), ported in its Task 24 against its
//! reference mood (`tools/studio_export/older/moods/villain_march.py`, the bed moved from
//! `projects/ballfight_tuba_vs_flute/audio.py`, #32 v3). Not ported yet: its port fills this folder: the card, the
//! options, the voices its bed plays, the voicings and the classic bed (`classic.rs`). Until then the registry
//! lists it but does not offer it, and its bed is refused in words.

mod classic;

use super::{NOT_PORTED_CARD, Theme, no_own_voice};

/// The theme as the registry lists it.
pub static THEME: Theme = Theme {
    name: "villain_march",
    ported: false,
    card: &NOT_PORTED_CARD,
    options: &[],
    voices: &[],
    own_voice: no_own_voice,
    classic: classic::classic,
};
