//! CELLO's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// CELLO's personality.
pub static PERSONALITY: Personality = Personality {
    home: 50, // D3, from ballfight_cello_vs_contrabass/episode.py
    scale: [0, 2, 3, 5, 7, 8, 11],
    register: (57, 70),
    rhythms: &[&[0, 2, 4, 10, 12], &[0, 2, 4, 6, 8, 14], &[0, 2, 4, 6, 8]],
    intervals: &[1, 3, 2],
    ornament: Ornament::Slide,
    roles: &[Role::Bass, Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_cello_vs_contrabass/episode.py: its solo, beats 1 to 4
        &[s(7, 2), s(9, 2), s(11, 6), s(10, 2), s(9, 4)],
        // ballfight_cello_vs_contrabass/episode.py: its line, the first 8 notes
        &[s(7, 2), s(7, 2), s(7, 2), s(8, 2), s(9, 2), s(9, 2), s(8, 2), s(6, 2)],
    ],
};
