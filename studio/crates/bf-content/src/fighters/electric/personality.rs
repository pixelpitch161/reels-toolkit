//! ELECTRIC's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// ELECTRIC's personality.
pub static PERSONALITY: Personality = Personality {
    home: 52, // E3, from ballfight_electric_vs_acoustic/episode.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (62, 84),
    rhythms: &[
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 13, 14, 15],
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14],
        &[0],
    ],
    intervals: &[3, 2, 4],
    ornament: Ornament::Slide,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_electric_vs_acoustic/episode.py: its solo, beats 1 to 4
        &[
            s(14, 1),
            s(13, 1),
            s(11, 1),
            s(10, 1),
            s(9, 1),
            s(10, 1),
            s(11, 1),
            s(13, 1),
        ],
        // ballfight_electric_vs_acoustic/episode.py: its line, the first 8 notes
        &[s(7, 2), s(7, 2), s(9, 2), s(7, 2), s(10, 2), s(9, 2), s(7, 2), s(6, 2)],
    ],
};
