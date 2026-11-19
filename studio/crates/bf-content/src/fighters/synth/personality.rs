//! SYNTH's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// SYNTH's personality.
pub static PERSONALITY: Personality = Personality {
    home: 65, // F4, from ballfight_turntables_vs_synth/timeline.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (72, 85),
    rhythms: &[
        &[0, 2, 3, 4, 6, 8, 9, 10, 14],
        &[0, 2, 3, 4, 8, 10, 12, 13, 14],
        &[0, 2, 3, 4, 7, 8, 10, 12],
    ],
    intervals: &[2, 3, 1],
    ornament: Ornament::Slide,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_turntables_vs_synth/timeline.py: its solo, beats 1 to 4
        &[s(4, 2), s(6, 1), s(7, 1), s(9, 2), s(8, 2), s(7, 1), s(6, 1), s(7, 4)],
        // ballfight_turntables_vs_synth/timeline.py: its solo, beats 5 to 8
        &[
            s(5, 2),
            s(7, 1),
            s(9, 1),
            s(12, 4),
            s(11, 2),
            s(9, 2),
            s(10, 1),
            s(9, 1),
        ],
    ],
};
