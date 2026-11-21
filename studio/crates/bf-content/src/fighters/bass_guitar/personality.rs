//! BASS GUITAR's musical personality (a Studio fighter: its motifs are its own, written for it): a picked rock bass
//! line, driving eighth notes on the root with octave jumps and a walk up to the fifth, and a fill that runs up the
//! neck. It plays the bass under everything, or answers.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// BASS GUITAR's personality.
pub static PERSONALITY: Personality = Personality {
    home: 28, // E1: the open low string
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (28, 52),
    rhythms: &[
        &[0, 2, 4, 6, 8, 10, 12, 14],
        &[0, 2, 3, 4, 6, 8, 10, 11, 12, 14],
        &[0, 3, 6, 8, 10, 12, 14],
    ],
    intervals: &[7, 4, 1],
    ornament: Ornament::Slide,
    roles: &[Role::Bass, Role::Answer],
    motifs: &[
        // the driving root: eighths on the low E, an octave jump and back
        &[s(0, 2), s(0, 2), s(0, 2), s(7, 2), s(0, 2), s(0, 2), s(7, 2), s(0, 2)],
        // root, fifth, octave, and a walk back down
        &[s(0, 2), s(4, 2), s(7, 2), s(4, 2), s(5, 2), s(4, 2), s(2, 2), s(1, 2)],
        // the fill: a run up the neck to the octave, held
        &[s(0, 1), s(2, 1), s(3, 1), s(4, 1), s(5, 2), s(6, 2), s(7, 8)],
    ],
};
