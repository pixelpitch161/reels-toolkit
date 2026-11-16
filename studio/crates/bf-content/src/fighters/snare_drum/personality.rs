//! SNARE DRUM's musical personality (a Studio fighter: its motifs are its own, written for it): crisp rudiments on
//! one tuned head, a marching figure with a flammed accent on beat 1, sixteenth diddles and an accented crack on 4,
//! and a break that builds from paradiddles into a roll. Its part plays every note as a snare stroke; the degrees
//! only tune the head a little higher or lower.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// SNARE DRUM's personality.
pub static PERSONALITY: Personality = Personality {
    home: 72, // C5: the head's pitch
    scale: [0, 2, 4, 5, 7, 9, 10],
    register: (67, 79),
    rhythms: &[
        &[0, 2, 3, 4, 6, 8, 10, 11, 12, 14],
        &[0, 1, 2, 4, 6, 7, 8, 12, 14, 15],
        &[0, 4, 6, 8, 10, 12, 13, 14, 15],
    ],
    intervals: &[2, 1],
    ornament: Ornament::Grace,
    roles: &[Role::Answer],
    motifs: &[
        // the marching figure: a flammed accent, diddles, the crack on 4
        &[
            s(2, 2),
            s(0, 1),
            s(0, 1),
            s(0, 2),
            s(0, 1),
            s(0, 1),
            s(0, 2),
            s(0, 2),
            s(4, 4),
        ],
        // the paradiddle, accented: R L R R, L R L L
        &[
            s(2, 1),
            s(0, 1),
            s(0, 1),
            s(0, 1),
            s(2, 1),
            s(0, 1),
            s(0, 1),
            s(0, 1),
            s(0, 2),
            s(0, 2),
            s(4, 4),
        ],
        // the break: diddles tightening into a roll, then one crack
        &[
            s(0, 2),
            s(0, 2),
            s(0, 1),
            s(0, 1),
            s(0, 1),
            s(0, 1),
            s(1, 1),
            s(1, 1),
            s(2, 1),
            s(2, 1),
            s(4, 4),
        ],
    ],
};
