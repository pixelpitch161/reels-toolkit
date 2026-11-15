//! VIOLA's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// VIOLA's personality.
pub static PERSONALITY: Personality = Personality {
    home: 48, // C3, from ballfight_viola_vs_violin/timeline.py
    scale: [0, 2, 3, 5, 7, 8, 11],
    register: (48, 72),
    rhythms: &[&[0, 4, 6, 8, 14], &[0, 4, 6, 8, 12, 14], &[0, 4, 8]],
    intervals: &[1, 2, 5],
    ornament: Ornament::Slide,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_viola_vs_violin/timeline.py: its solo, beats 1 to 4
        &[s(11, 4), s(12, 2), s(11, 2), s(14, 6), s(12, 2)],
        // ballfight_viola_vs_violin/timeline.py: its solo, beats 5 to 8
        &[s(12, 4), s(11, 2), s(10, 2), s(9, 4), s(8, 2), s(6, 2)],
    ],
};
