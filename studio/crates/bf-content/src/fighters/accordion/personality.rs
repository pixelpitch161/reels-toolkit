//! ACCORDION's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// ACCORDION's personality.
pub static PERSONALITY: Personality = Personality {
    home: 57, // A3, from ballfight_bagpipes_vs_accordion/timeline.py
    scale: [0, 2, 3, 5, 7, 9, 10],
    register: (68, 84),
    rhythms: &[
        &[0, 2, 4, 6, 8, 9, 10, 12],
        &[0, 1, 2, 3, 4, 6, 8, 10, 11, 12, 14],
        &[0, 2, 4, 6, 8],
    ],
    intervals: &[2, 1, 3],
    ornament: Ornament::Trill,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_bagpipes_vs_accordion/timeline.py: its solo, beats 1 to 4
        &[
            s(7, 2),
            s(9, 2),
            s(11, 2),
            s(14, 2),
            s(13, 1),
            s(14, 1),
            s(15, 2),
            s(16, 4),
        ],
        // ballfight_bagpipes_vs_accordion/timeline.py: its solo, beats 5 to 8
        &[
            s(15, 1),
            s(14, 1),
            s(13, 1),
            s(12, 1),
            s(11, 2),
            s(10, 2),
            s(9, 2),
            s(10, 1),
        ],
    ],
};
