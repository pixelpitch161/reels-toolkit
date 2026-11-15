//! VIOLIN's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// VIOLIN's personality.
pub static PERSONALITY: Personality = Personality {
    home: 62, // D4, from ballfight_drums_vs_violin/timeline.py
    scale: [0, 2, 3, 5, 7, 8, 11],
    register: (67, 86),
    rhythms: &[
        &[0, 1, 2, 3, 4, 6, 7, 8, 11, 12, 14],
        &[0, 1, 2, 3, 4, 6, 8, 9, 10, 11, 12, 14, 15],
        &[0, 2, 4, 5, 6, 7, 8, 10, 12],
    ],
    intervals: &[1, 2, 3],
    ornament: Ornament::Trill,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_drums_vs_violin/timeline.py: its solo, beats 1 to 4
        &[
            s(7, 1),
            s(8, 1),
            s(9, 1),
            s(10, 1),
            s(11, 2),
            s(12, 1),
            s(13, 1),
            s(14, 3),
        ],
        // ballfight_viola_vs_violin/timeline.py: its solo, beats 1 to 4
        &[
            s(7, 1),
            s(8, 1),
            s(9, 1),
            s(10, 1),
            s(11, 2),
            s(12, 1),
            s(13, 1),
            s(14, 3),
        ],
    ],
};
