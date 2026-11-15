//! CLARINET's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// CLARINET's personality.
pub static PERSONALITY: Personality = Personality {
    home: 38, // D2, from ballfight_flute_vs_clarinet/episode.py
    scale: [0, 2, 3, 5, 7, 8, 11],
    register: (48, 86),
    rhythms: &[
        &[0, 2, 4, 7, 8, 10, 12],
        &[0, 2, 3, 4, 6, 8, 10, 12],
        &[0, 1, 2, 3, 4, 6, 8, 9, 10, 12],
    ],
    intervals: &[3, 2, 1],
    ornament: Ornament::Slide,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_flute_vs_clarinet/episode.py: its solo, beats 1 to 4
        &[s(11, 2), s(14, 2), s(16, 3), s(15, 1), s(14, 2), s(12, 2), s(14, 4)],
        // ballfight_trumpet_vs_clarinet/episode.py: its solo, beats 1 to 4
        &[
            s(14, 1),
            s(16, 1),
            s(18, 1),
            s(21, 1),
            s(23, 3),
            s(22, 1),
            s(21, 2),
            s(20, 2),
        ],
        // ballfight_clarinet_vs_oboe/timeline.py: its solo, beats 1 to 4
        &[
            s(7, 2),
            s(9, 2),
            s(11, 2),
            s(14, 2),
            s(21, 4),
            s(20, 1),
            s(19, 1),
            s(18, 1),
        ],
        // ballfight/examples/clarinet_vs_oboe/episode.py: its solo, beats 1 to 4
        &[
            s(7, 2),
            s(9, 2),
            s(11, 2),
            s(14, 2),
            s(21, 4),
            s(20, 1),
            s(19, 1),
            s(18, 1),
        ],
    ],
};
