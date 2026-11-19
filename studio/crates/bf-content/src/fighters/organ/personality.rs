//! ORGAN's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// ORGAN's personality.
pub static PERSONALITY: Personality = Personality {
    home: 45, // A2, from ballfight_piano_vs_organ/episode.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (50, 77),
    rhythms: &[
        &[0, 2, 3, 4, 6, 12, 13, 14, 15],
        &[0, 2, 4, 12],
        &[0, 1, 2, 6, 7, 8, 9, 10, 13],
    ],
    intervals: &[2, 1, 3],
    ornament: Ornament::Trill,
    roles: &[Role::Bass, Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_piano_vs_organ/episode.py: its solo, beats 1 to 4
        &[
            s(18, 2),
            s(16, 1),
            s(17, 1),
            s(18, 2),
            s(14, 6),
            s(19, 1),
            s(18, 1),
            s(17, 1),
        ],
        // ballfight_organ_vs_harp/timeline.py: its solo, beats 1 to 4
        &[
            s(11, 1),
            s(10, 1),
            s(11, 4),
            s(10, 1),
            s(9, 1),
            s(8, 1),
            s(7, 1),
            s(6, 3),
        ],
    ],
};
