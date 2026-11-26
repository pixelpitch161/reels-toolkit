//! BASSOON's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// BASSOON's personality.
pub static PERSONALITY: Personality = Personality {
    home: 28, // E1, from ballfight_bassclarinet_vs_bassoon/episode.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (38, 76),
    rhythms: &[&[0, 2, 4, 6, 8, 10, 12, 14], &[0, 2, 4, 6, 8, 12]],
    intervals: &[2, 3, 4],
    ornament: Ornament::Grace,
    roles: &[Role::Bass, Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_bassclarinet_vs_bassoon/episode.py: its solo, beats 1 to 4
        &[
            s(21, 2),
            s(23, 2),
            s(25, 2),
            s(24, 2),
            s(23, 2),
            s(22, 2),
            s(21, 2),
            s(25, 1),
        ],
        // ballfight_oboe_vs_bassoon/episode.py: its solo, beats 1 to 4
        &[
            s(14, 2),
            s(16, 2),
            s(18, 2),
            s(17, 2),
            s(16, 2),
            s(15, 2),
            s(14, 2),
            s(18, 1),
        ],
        // ballfight_bassoon_vs_barisax/timeline.py: its solo, beats 1 to 4
        &[
            s(21, 2),
            s(23, 2),
            s(25, 2),
            s(24, 2),
            s(23, 2),
            s(22, 2),
            s(21, 2),
            s(25, 1),
        ],
        // ballfight/examples/bassoon_vs_barisax/episode.py: its solo, beats 1 to 4
        &[
            s(21, 2),
            s(23, 2),
            s(25, 2),
            s(24, 2),
            s(23, 2),
            s(22, 2),
            s(21, 2),
            s(25, 1),
        ],
    ],
};
