//! DRUMS's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// DRUMS's personality.
pub static PERSONALITY: Personality = Personality {
    home: 38, // D2, from ballfight_drums_vs_violin/timeline.py
    scale: [0, 2, 3, 5, 7, 8, 11],
    register: (38, 57),
    rhythms: &[
        &[0, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        &[0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        &[0, 2, 4, 5, 6, 8, 9, 10, 11, 12],
    ],
    intervals: &[12, 3, 4],
    ornament: Ornament::Stab,
    roles: &[Role::Answer],
    motifs: &[
        // ballfight_drums_vs_violin/timeline.py: its solo, beats 1 to 4
        &[s(11, 2), s(7, 1), s(7, 1), s(0, 2), s(5, 1), s(5, 1), s(4, 1), s(4, 1)],
        // ballfight_drums_vs_violin/timeline.py: its solo, beats 5 to 8
        &[s(11, 2), s(0, 1), s(7, 1), s(0, 1), s(5, 1), s(4, 1), s(2, 1), s(7, 1)],
    ],
};
