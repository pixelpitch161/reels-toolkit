//! MELLOPHONE's musical personality (a from-scratch fighter, 2026-11-16): a drum-corps mellophone, bright and singing
//! in the horn's middle register, with big held "mello soli" notes, falls off the top and call-and-echo phrases (a
//! figure, then the same figure back an octave or a step higher). Its motifs are invented for it (no episode has one),
//! in scale degrees from its home key.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// MELLOPHONE's personality.
pub static PERSONALITY: Personality = Personality {
    home: 65, // F 4: the mellophone in F
    scale: [0, 2, 4, 5, 7, 9, 11],
    register: (53, 84),
    rhythms: &[&[0, 2, 4, 8, 12], &[0, 6, 8, 10, 12], &[0, 3, 6, 8, 12, 14]],
    intervals: &[2, 4, 3],
    ornament: Ornament::Slide,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // the press-box call: up a fifth, held, and the same call echoed a step higher
        &[s(0, 2), s(4, 6), s(1, 2), s(5, 6)],
        // the mello soli: a climb through the chord to a held top, a fall off it
        &[s(0, 2), s(2, 2), s(4, 2), s(7, 6), s(6, 2), s(4, 2)],
        // the field-show hit: two short punches and a long answer below
        &[s(4, 1), s(4, 3), s(5, 2), s(2, 6), s(0, 4)],
    ],
};
