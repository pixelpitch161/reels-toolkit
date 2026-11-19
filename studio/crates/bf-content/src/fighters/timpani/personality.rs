//! TIMPANI's musical personality (a Studio fighter: its motifs are its own, written for it): the orchestra's two
//! tuned drums, low and heavy, built from roots, fifths and octaves, with pedal glissandos into the big notes.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// TIMPANI's personality.
pub static PERSONALITY: Personality = Personality {
    home: 43, // G2
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (38, 57),
    rhythms: &[&[0, 4, 8, 12], &[0, 6, 8, 12, 14], &[0, 2, 4, 8, 11, 12]],
    intervals: &[7, 5, 12, 4],
    ornament: Ornament::Slide,
    roles: &[Role::Bass, Role::Answer],
    motifs: &[
        // the call: low kettle, high kettle (boom-bah), then the low one answers twice
        &[s(0, 4), s(4, 4), s(0, 2), s(0, 2), s(4, 4)],
        // the climb: root, fifth, octave, and a hard landing on the fifth
        &[s(0, 2), s(4, 2), s(7, 4), s(4, 2), s(0, 2), s(4, 4)],
        // the tuning fill: a pedal slide up a step at a time into the root
        &[s(-3, 1), s(-2, 1), s(-1, 2), s(0, 4), s(4, 2), s(0, 6)],
    ],
};
