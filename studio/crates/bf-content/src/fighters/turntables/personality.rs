//! TURNTABLES's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// TURNTABLES's personality.
pub static PERSONALITY: Personality = Personality {
    home: 41, // F2, from ballfight_turntables_vs_synth/timeline.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (41, 53),
    rhythms: &[
        &[0, 4, 6, 8, 9, 10, 12, 13, 14],
        &[0, 2, 4, 6, 7, 8, 12, 14],
        &[0, 2, 3, 4, 6, 7, 8, 10, 12],
    ],
    intervals: &[7, 9, 2],
    ornament: Ornament::Stab,
    roles: &[Role::Answer],
    motifs: &[
        // ballfight_turntables_vs_synth/timeline.py: its solo, beats 1 to 4
        &[s(7, 4), s(2, 2), s(2, 2), s(4, 1), s(4, 1), s(3, 2), s(7, 1), s(7, 1)],
        // ballfight_turntables_vs_synth/timeline.py: its solo, beats 5 to 8
        &[s(7, 2), s(0, 2), s(0, 2), s(4, 1), s(4, 1), s(3, 4), s(7, 2), s(7, 1)],
    ],
};
