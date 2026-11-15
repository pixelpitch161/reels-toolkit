//! UKULELE's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// UKULELE's personality.
pub static PERSONALITY: Personality = Personality {
    home: 69, // A4, from ballfight_ukulele_vs_acoustic/timeline.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (69, 84),
    rhythms: &[
        &[0, 2, 3, 4, 6, 7, 8, 10, 11, 12, 14],
        &[0, 1, 2, 3, 4, 6, 8, 10, 11, 12, 14],
        &[0, 1, 2, 3, 4, 6, 8, 10, 12],
    ],
    intervals: &[3, 5, 4],
    ornament: Ornament::Grace,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_ukulele_vs_acoustic/timeline.py: its solo, beats 1 to 4
        &[s(0, 2), s(4, 1), s(6, 1), s(7, 2), s(6, 1), s(4, 1), s(1, 2), s(3, 1)],
        // ballfight_ukulele_vs_acoustic/timeline.py: its solo, beats 5 to 8
        &[s(0, 1), s(0, 1), s(5, 1), s(4, 1), s(2, 2), s(4, 2), s(1, 2), s(6, 1)],
    ],
};
