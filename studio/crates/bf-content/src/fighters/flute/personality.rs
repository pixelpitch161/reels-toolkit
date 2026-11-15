//! FLUTE's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// FLUTE's personality.
pub static PERSONALITY: Personality = Personality {
    home: 62, // D4, from ballfight_flute_vs_clarinet/episode.py
    scale: [0, 2, 3, 5, 7, 8, 11],
    register: (71, 93),
    rhythms: &[
        &[0, 2, 4, 8, 10, 11, 12],
        &[0, 2, 4, 7, 8, 10, 12],
        &[0, 1, 2, 4, 7, 8, 10, 12],
    ],
    intervals: &[1, 2, 3],
    ornament: Ornament::Trill,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_flute_vs_clarinet/episode.py: its solo, beats 1 to 4
        &[s(11, 2), s(14, 2), s(16, 4), s(15, 2), s(14, 1), s(13, 1), s(14, 4)],
        // ballfight_flute_vs_sousaphone/episode.py: its solo, beats 1 to 4
        &[s(14, 2), s(16, 2), s(18, 4), s(17, 2), s(16, 2), s(15, 2), s(13, 2)],
        // ballfight_flute_vs_trombone/timeline.py: its solo, beats 1 to 4
        &[
            s(11, 2),
            s(12, 1),
            s(11, 1),
            s(10, 2),
            s(9, 2),
            s(7, 2),
            s(6, 1),
            s(7, 1),
        ],
        // ballfight_tuba_vs_flute/timeline.py: its solo, beats 1 to 4
        &[
            s(7, 1),
            s(8, 1),
            s(9, 1),
            s(10, 1),
            s(11, 1),
            s(12, 1),
            s(13, 1),
            s(14, 1),
        ],
    ],
};
