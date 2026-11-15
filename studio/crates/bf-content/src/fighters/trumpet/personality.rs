//! TRUMPET's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// TRUMPET's personality.
pub static PERSONALITY: Personality = Personality {
    home: 62, // D4, from ballfight_trumpet_vs_clarinet/episode.py
    scale: [0, 2, 3, 5, 7, 8, 11],
    register: (62, 86),
    rhythms: &[
        &[0, 2, 4, 8, 10, 11, 12],
        &[0, 2, 4, 7, 8, 10, 12],
        &[0, 1, 2, 4, 7, 8, 10, 12],
    ],
    intervals: &[3, 2, 4],
    ornament: Ornament::Stab,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_trumpet_vs_clarinet/episode.py: its solo, beats 1 to 4
        &[s(7, 2), s(9, 2), s(11, 4), s(10, 2), s(9, 1), s(8, 1), s(7, 4)],
        // ballfight_trumpet_vs_trombone/episode.py: its solo, beats 1 to 4
        &[s(0, 2), s(2, 2), s(4, 2), s(7, 4), s(5, 1), s(4, 1), s(6, 4)],
        // ballfight/examples/trumpet_vs_sax/episode.py: its solo, beats 1 to 4
        &[
            s(7, 2),
            s(9, 2),
            s(11, 3),
            s(10, 1),
            s(9, 2),
            s(8, 1),
            s(7, 1),
            s(11, 4),
        ],
    ],
};
