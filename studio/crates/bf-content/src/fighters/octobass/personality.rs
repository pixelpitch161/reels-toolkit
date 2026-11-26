//! OCTOBASS's musical personality (a Studio fighter: its motifs are its own, written for it): a slow giant's walk in
//! the lowest register, and a climb up its three strings one lever at a time.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// OCTOBASS's personality.
pub static PERSONALITY: Personality = Personality {
    home: 24, // C1: its low string
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (22, 46),
    rhythms: &[
        // the giant's walk: one heavy note a beat
        &[0, 4, 8, 12],
        // a held note and a lever's clunk
        &[0, 8, 14],
        // three strings, three levers
        &[0, 3, 6, 8],
    ],
    intervals: &[7, 5, 12],
    ornament: Ornament::Slide,
    roles: &[Role::Bass],
    motifs: &[
        // the giant's walk: root, fifth below, root, a step up
        &[s(7, 4), s(4, 4), s(7, 4), s(8, 4)],
        // up the three strings, one lever at a time
        &[s(0, 2), s(1, 2), s(2, 2), s(4, 2), s(5, 2), s(6, 2), s(7, 4)],
        // below hearing: one very long low note, then the boom an octave up
        &[s(0, 12), s(7, 4)],
    ],
};
