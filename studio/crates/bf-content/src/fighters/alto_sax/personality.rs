//! ALTO SAX's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// ALTO SAX's personality.
pub static PERSONALITY: Personality = Personality {
    home: 57, // A3, from ballfight_altosax_vs_tenorsax/episode.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (65, 81),
    rhythms: &[&[0, 2, 4, 8, 10, 12, 14], &[0, 2, 4, 6, 8], &[0, 2, 4]],
    intervals: &[2, 3, 1],
    ornament: Ornament::Slide,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_altosax_vs_tenorsax/episode.py: its solo, beats 1 to 4
        &[s(11, 2), s(13, 2), s(14, 4), s(13, 2), s(11, 2), s(10, 2), s(9, 2)],
        // ballfight_altosax_vs_tenorsax/episode.py: its line, the first 8 notes
        &[
            s(11, 2),
            s(10, 2),
            s(9, 2),
            s(10, 2),
            s(11, 2),
            s(13, 2),
            s(11, 2),
            s(9, 2),
        ],
    ],
};
