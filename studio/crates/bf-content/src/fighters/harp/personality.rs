//! HARP's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// HARP's personality.
pub static PERSONALITY: Personality = Personality {
    home: 38, // D2, from ballfight_organ_vs_harp/timeline.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (46, 86),
    rhythms: &[
        &[0, 1, 2, 3, 4, 5, 6, 8, 10, 11, 12, 13, 14],
        &[0, 1, 2, 3, 4, 5, 6, 8, 12, 14],
        &[0],
    ],
    intervals: &[5, 4, 7],
    ornament: Ornament::Grace,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_organ_vs_harp/timeline.py: its solo, beats 1 to 4
        &[
            s(7, 1),
            s(11, 1),
            s(14, 1),
            s(16, 1),
            s(18, 1),
            s(21, 1),
            s(23, 2),
            s(25, 2),
        ],
        // ballfight_organ_vs_harp/timeline.py: its solo, beats 5 to 8
        &[
            s(5, 1),
            s(9, 1),
            s(12, 1),
            s(14, 1),
            s(16, 1),
            s(19, 1),
            s(21, 2),
            s(28, 4),
        ],
    ],
};
