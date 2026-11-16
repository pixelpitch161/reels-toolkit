//! CONTRABASS CLARINET's musical personality: the first one written for the Studio, so its motifs are invented
//! (design `docs/studio/2026-11-16-contrabass-clarinet-design.md`), not taken from an episode.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// CONTRABASS CLARINET's personality.
pub static PERSONALITY: Personality = Personality {
    home: 26, // D1, its lowest note
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (26, 58),
    rhythms: &[
        // the foghorn: long held notes
        &[0, 8],
        // the putt-putt: the motorboat's offbeat rattle
        &[0, 3, 6, 8, 11, 14],
        // a slow walk
        &[0, 4, 8, 12],
    ],
    intervals: &[7, 5, 12],
    ornament: Ornament::Slide,
    roles: &[Role::Bass, Role::Answer],
    motifs: &[
        // the foghorn call: low D, then the fifth, both long
        &[s(0, 6), s(4, 8), s(0, 2)],
        // the putt-putt: the root rattling, then a climb to the fifth
        &[s(7, 1), s(7, 2), s(7, 1), s(7, 2), s(9, 2), s(11, 4), s(7, 4)],
        // a harbour at night: up the minor triad and an octave, then sinking back to the root
        &[
            s(7, 3),
            s(9, 3),
            s(11, 2),
            s(14, 6),
            s(13, 2),
            s(11, 4),
            s(10, 4),
            s(7, 8),
        ],
    ],
};
