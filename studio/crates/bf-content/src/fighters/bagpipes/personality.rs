//! BAGPIPES's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// BAGPIPES's personality.
pub static PERSONALITY: Personality = Personality {
    home: 69, // A4, from ballfight_bagpipes_vs_accordion/timeline.py
    scale: [0, 2, 3, 5, 7, 9, 10],
    register: (69, 81),
    rhythms: &[&[0, 4, 6, 8, 11, 12], &[0, 6, 8, 11, 12, 14], &[0, 3, 4, 6, 8]],
    intervals: &[2, 1, 3],
    ornament: Ornament::Grace,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_bagpipes_vs_accordion/timeline.py: its solo, beats 1 to 4
        &[s(0, 4), s(1, 2), s(2, 2), s(3, 3), s(4, 1), s(4, 4)],
        // ballfight_bagpipes_vs_accordion/timeline.py: its solo, beats 5 to 8
        &[s(7, 6), s(6, 2), s(5, 3), s(4, 1), s(3, 2), s(2, 2)],
    ],
};
