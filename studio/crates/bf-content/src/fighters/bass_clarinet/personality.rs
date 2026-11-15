//! BASS CLARINET's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// BASS CLARINET's personality.
pub static PERSONALITY: Personality = Personality {
    home: 28, // E1, from ballfight_bassclarinet_vs_bassoon/episode.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (35, 62),
    rhythms: &[
        &[0, 2, 4, 6, 8, 11, 12, 14],
        &[0, 2, 4, 6, 7, 8],
        &[0, 2, 4, 6, 8, 10, 12],
    ],
    intervals: &[3, 4, 2],
    ornament: Ornament::Slide,
    roles: &[Role::Bass, Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_bassclarinet_vs_bassoon/episode.py: its solo, beats 1 to 4
        &[
            s(7, 2),
            s(9, 2),
            s(11, 2),
            s(13, 2),
            s(14, 3),
            s(16, 1),
            s(14, 2),
            s(11, 2),
        ],
        // ballfight_barisax_vs_bassclarinet/timeline.py: its solo, beats 1 to 4
        &[
            s(10, 2),
            s(12, 2),
            s(14, 2),
            s(16, 2),
            s(17, 3),
            s(16, 1),
            s(14, 2),
            s(12, 2),
        ],
        // ballfight_bassclarinet_vs_euphonium/timeline.py: its solo, beats 1 to 4
        &[
            s(10, 2),
            s(14, 2),
            s(17, 2),
            s(19, 2),
            s(18, 3),
            s(17, 1),
            s(16, 2),
            s(14, 2),
        ],
        // ballfight/examples/barisax_vs_bassclarinet/episode.py: its solo, beats 1 to 4
        &[
            s(10, 2),
            s(12, 2),
            s(14, 2),
            s(16, 2),
            s(17, 3),
            s(16, 1),
            s(14, 2),
            s(12, 2),
        ],
    ],
};
