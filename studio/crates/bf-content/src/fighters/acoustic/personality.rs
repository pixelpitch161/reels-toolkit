//! ACOUSTIC's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// ACOUSTIC's personality.
pub static PERSONALITY: Personality = Personality {
    home: 40, // E2, from ballfight_electric_vs_acoustic/episode.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (40, 76),
    rhythms: &[
        &[0, 2, 4, 6, 8, 10, 12],
        &[0, 2, 4, 6, 8, 10, 12, 14],
        &[0, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 14],
    ],
    intervals: &[3, 4, 1],
    ornament: Ornament::Slide,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_electric_vs_acoustic/episode.py: its solo, beats 1 to 4
        &[s(0, 2), s(9, 2), s(11, 2), s(14, 2), s(7, 2), s(11, 2), s(14, 4)],
        // ballfight_ukulele_vs_acoustic/timeline.py: its solo, beats 1 to 4
        &[
            s(11, 2),
            s(11, 1),
            s(12, 1),
            s(11, 1),
            s(10, 1),
            s(9, 2),
            s(13, 1),
            s(13, 1),
        ],
        // ballfight/examples/ukulele_vs_acoustic/episode.py: its solo, beats 1 to 4
        &[
            s(11, 2),
            s(11, 1),
            s(12, 1),
            s(11, 1),
            s(10, 1),
            s(9, 2),
            s(13, 1),
            s(13, 1),
        ],
    ],
};
