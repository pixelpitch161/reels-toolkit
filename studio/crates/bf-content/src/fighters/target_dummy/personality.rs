//! TARGET DUMMY's musical personality: MARIMBA's register, plain knocks on the beat (it has no tune of its own).

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// TARGET DUMMY's personality.
pub static PERSONALITY: Personality = Personality {
    home: 67, // G4
    scale: [0, 2, 4, 5, 7, 9, 11],
    register: (60, 84),
    rhythms: &[&[0, 4, 8, 12], &[0, 8]],
    intervals: &[2, 4],
    ornament: Ornament::Grace,
    roles: &[Role::Answer],
    motifs: &[
        // knock, knock
        &[s(0, 4), s(0, 4), s(4, 8)],
        // the wobble after a hit
        &[s(4, 2), s(2, 2), s(0, 4), s(-3, 8)],
    ],
};
