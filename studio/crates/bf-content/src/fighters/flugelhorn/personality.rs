//! FLUGELHORN's musical personality (a from-scratch fighter, 2026-11-16): the mellow jazz-ballad horn, legato and
//! warm in the middle register, with rising sixths (the old flank signal, played softly), fluttering grace notes and
//! long, floating held notes. Its motifs are invented for it (no episode has one), in scale degrees from its home.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// FLUGELHORN's personality.
pub static PERSONALITY: Personality = Personality {
    home: 58, // B-flat 3: the flugelhorn's own key
    scale: [0, 2, 4, 5, 7, 9, 11],
    register: (52, 77),
    rhythms: &[&[0, 6, 8, 14], &[0, 4, 10, 12], &[0, 3, 4, 8, 12]],
    intervals: &[9, 4, 2],
    ornament: Ornament::Slide,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // the flank call: a pickup, then up a sixth and held, settling a step down
        &[s(-1, 1), s(0, 3), s(5, 6), s(4, 2), s(4, 4)],
        // the feather flutter: a quick turn round the third, then drifting down
        &[s(2, 1), s(3, 1), s(2, 1), s(1, 1), s(2, 4), s(1, 2), s(0, 2), s(-2, 4)],
        // the glide: up the octave in a long arc and floating back down in tied notes
        &[s(0, 2), s(2, 2), s(4, 2), s(7, 8), s(6, 2), s(5, 2), s(4, 6), s(2, 4)],
    ],
};
