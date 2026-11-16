//! MARIMBA's musical personality (a Studio fighter: its motifs are its own, written for it): bright, rolling
//! mallet figures in a major key, quick sixteenth runs that climb (the mallet run) and rocking broken chords.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// MARIMBA's personality.
pub static PERSONALITY: Personality = Personality {
    home: 67, // G4
    scale: [0, 2, 4, 5, 7, 9, 11],
    register: (60, 91),
    rhythms: &[
        &[0, 2, 4, 6, 8, 10, 12, 14],
        &[0, 3, 6, 8, 10, 12],
        &[0, 1, 2, 4, 8, 9, 10, 12],
    ],
    intervals: &[2, 4, 3, 7],
    ornament: Ornament::Grace,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // the mallet run: up the scale a wall at a time, then the landing
        &[s(0, 1), s(1, 1), s(2, 1), s(3, 1), s(4, 2), s(2, 2), s(7, 4), s(4, 4)],
        // the rocking figure of a four-mallet grip: broken thirds and sixths
        &[s(0, 2), s(4, 2), s(2, 2), s(7, 2), s(4, 2), s(9, 2), s(7, 4)],
        // the roll: a held tone over a falling answer
        &[s(9, 4), s(8, 2), s(7, 2), s(5, 2), s(4, 2), s(2, 4)],
    ],
};
