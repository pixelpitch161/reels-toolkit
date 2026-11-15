//! PIANO's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// PIANO's personality.
pub static PERSONALITY: Personality = Personality {
    home: 57, // A3, from ballfight_piano_vs_organ/episode.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (64, 87),
    rhythms: &[
        &[0, 1, 2, 3, 4, 7, 8, 10, 11, 12],
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12],
        &[0, 2, 4, 6, 8, 11, 12, 14],
    ],
    intervals: &[3, 2, 4],
    ornament: Ornament::Grace,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_piano_vs_organ/episode.py: its solo, beats 1 to 4
        &[
            s(7, 1),
            s(9, 1),
            s(11, 1),
            s(14, 1),
            s(16, 3),
            s(15, 1),
            s(14, 2),
            s(11, 1),
        ],
        // ballfight/timeline.py: its solo, beats 1 to 4
        &[
            s(7, 2),
            s(9, 2),
            s(11, 2),
            s(14, 2),
            s(16, 3),
            s(15, 1),
            s(14, 2),
            s(13, 2),
        ],
        // ballfight_piano_vs_organ/episode.py: its line, the first 8 notes
        &[s(11, 4), s(9, 2), s(14, 4), s(11, 2), s(9, 2), s(10, 2)],
    ],
};
