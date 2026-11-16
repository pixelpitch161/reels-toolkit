//! CONTRABASSOON's musical personality: its register, rhythms, intervals, ornament, roles and the signature motifs
//! the composer builds its line and victory solo from. It has no episodes, so its motifs are its own (Nov 16, 2026),
//! in scale degrees from its home key.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// CONTRABASSOON's personality.
pub static PERSONALITY: Personality = Personality {
    home: 22, // B flat 0, its lowest note
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (34, 62),
    rhythms: &[&[0, 4, 6, 8, 12], &[0, 3, 6, 8, 10, 12, 14], &[0, 8]],
    intervals: &[5, 7, 12],
    ornament: Ornament::Grace,
    roles: &[Role::Bass, Role::Answer, Role::Melody],
    motifs: &[
        // the pipe-laying riff: down in fourths, a step back up after each
        &[
            s(14, 2),
            s(11, 2),
            s(12, 2),
            s(9, 2),
            s(10, 2),
            s(7, 2),
            s(7, 4),
        ],
        // the pipe echo: two quick notes and a jump, twice, the second higher
        &[
            s(10, 1),
            s(10, 1),
            s(14, 2),
            s(10, 1),
            s(10, 1),
            s(15, 2),
            s(14, 4),
            s(7, 4),
        ],
        // the grumble: round the bottom note and up a fourth
        &[s(7, 2), s(8, 1), s(7, 1), s(9, 2), s(8, 2), s(11, 4), s(7, 4)],
    ],
};
