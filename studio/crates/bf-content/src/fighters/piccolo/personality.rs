//! PICCOLO's musical personality (a Studio fighter: its motifs are its own, written for it): bright and high in a
//! major key, fife-march dotted figures, trills on a held note and octave leaps that hop over the band.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// PICCOLO's personality.
pub static PERSONALITY: Personality = Personality {
    home: 86, // D6
    scale: [0, 2, 4, 5, 7, 9, 11],
    register: (74, 108),
    rhythms: &[
        &[0, 3, 4, 6, 8, 11, 12, 14],
        &[0, 1, 2, 3, 4, 8, 12],
        &[0, 2, 4, 6, 7, 8, 10, 12, 14],
    ],
    intervals: &[2, 7, 4, 1],
    ornament: Ornament::Trill,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // the fife call: dotted march steps up to the fifth and a bright fall home
        &[s(0, 3), s(1, 1), s(2, 3), s(4, 1), s(7, 4), s(4, 2), s(2, 2)],
        // the trill: a held tone shaken with its neighbour, then an octave leap
        &[
            s(4, 1),
            s(5, 1),
            s(4, 1),
            s(5, 1),
            s(4, 1),
            s(5, 1),
            s(4, 2),
            s(11, 4),
            s(7, 4),
        ],
        // the ricochet: a run that zig-zags down in thirds and lands on the home note
        &[
            s(9, 2),
            s(7, 1),
            s(8, 1),
            s(6, 1),
            s(7, 1),
            s(5, 1),
            s(4, 1),
            s(0, 4),
            s(7, 4),
        ],
    ],
};
