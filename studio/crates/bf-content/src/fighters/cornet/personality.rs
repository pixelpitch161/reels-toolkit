//! CORNET's musical personality (a from-scratch fighter, 2026-11-16): hot-jazz cornet, warm and lyrical in the middle
//! register, with swung pickups, blue thirds and stop-time breaks. Its motifs are invented for it (no episode has
//! one), in scale degrees from its home key.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// CORNET's personality.
pub static PERSONALITY: Personality = Personality {
    home: 58, // B-flat 3: the cornet's own key
    scale: [0, 2, 4, 5, 7, 9, 10],
    register: (55, 79),
    rhythms: &[
        &[0, 3, 4, 8, 11, 12],
        &[0, 2, 6, 8, 12, 14],
        &[0, 4, 7, 8, 10, 12],
    ],
    intervals: &[2, 3, 1],
    ornament: Ornament::Slide,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // the stop-time call: a pickup into a held fifth, a blue fall back to the root
        &[s(2, 1), s(4, 3), s(4, 4), s(3, 2), s(2, 2), s(0, 4)],
        // the wah answer: two muted repeated notes, a lift and a sigh
        &[s(5, 2), s(5, 2), s(7, 4), s(6, 2), s(4, 2), s(5, 4)],
        // the parade turn: up the chord with a turn at the top
        &[s(0, 2), s(2, 2), s(4, 2), s(7, 3), s(8, 1), s(7, 2), s(6, 2), s(7, 2)],
    ],
};
