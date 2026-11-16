//! SOPRANO SAX's musical personality (a from-scratch fighter, 2026-11-16): high, bright and quick, darting chromatic
//! runs and trills like a hornet in flight. Its motifs are invented for it (no episode has one), in scale degrees from
//! its home key.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// SOPRANO SAX's personality.
pub static PERSONALITY: Personality = Personality {
    home: 70, // B-flat 4: the soprano's own key, an octave up
    scale: [0, 2, 4, 5, 7, 9, 10],
    register: (58, 86),
    rhythms: &[
        &[0, 1, 2, 3, 6, 8, 9, 10, 11, 14],
        &[0, 2, 3, 4, 8, 10, 11, 12],
        &[0, 1, 4, 6, 7, 8, 12, 13],
    ],
    intervals: &[1, 2, 3],
    ornament: Ornament::Trill,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // the buzz: a tight back-and-forth round the fifth, then a dart up to the octave
        &[s(4, 1), s(5, 1), s(4, 1), s(5, 1), s(4, 2), s(7, 2), s(6, 1), s(7, 3)],
        // the dive: a quick run down the scale onto the third
        &[s(7, 1), s(6, 1), s(5, 1), s(4, 1), s(3, 1), s(2, 3), s(2, 2), s(0, 2)],
        // the sting: two jabs at the top, a leap down and a rising answer
        &[s(9, 1), s(9, 1), s(4, 2), s(5, 2), s(7, 4)],
    ],
};
