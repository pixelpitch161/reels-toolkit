//! BARI SAX's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// BARI SAX's personality.
pub static PERSONALITY: Personality = Personality {
    home: 31, // G1, from ballfight_barisax_vs_bassclarinet/timeline.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (36, 55),
    rhythms: &[&[0, 4, 6, 8, 10, 12, 14], &[0, 4, 5, 6, 7, 8], &[0, 4, 6, 8, 12, 14]],
    intervals: &[3, 2, 7],
    ornament: Ornament::Stab,
    roles: &[Role::Bass, Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_barisax_vs_bassclarinet/timeline.py: its solo, beats 1 to 4
        &[s(3, 4), s(7, 2), s(10, 2), s(12, 2), s(10, 2), s(9, 2), s(7, 1)],
        // ballfight_bassoon_vs_barisax/timeline.py: its solo, beats 1 to 4
        &[s(7, 4), s(14, 2), s(13, 2), s(11, 4), s(10, 2), s(9, 2)],
        // ballfight/examples/barisax_vs_bassclarinet/episode.py: its solo, beats 1 to 4
        &[s(3, 4), s(7, 2), s(10, 2), s(12, 2), s(10, 2), s(9, 2), s(7, 1)],
        // ballfight/examples/bassoon_vs_barisax/episode.py: its solo, beats 1 to 4
        &[s(7, 4), s(14, 2), s(13, 2), s(11, 4), s(10, 2), s(9, 2)],
    ],
};
