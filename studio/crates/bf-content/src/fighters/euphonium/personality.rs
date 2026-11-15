//! EUPHONIUM's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// EUPHONIUM's personality.
pub static PERSONALITY: Personality = Personality {
    home: 52, // E3, from ballfight_euphonium_vs_trombone/episode.py
    scale: [0, 2, 3, 5, 7, 8, 11],
    register: (59, 76),
    rhythms: &[&[0, 4, 6, 8, 12, 14], &[0, 4, 6, 8, 10, 12], &[0, 4, 6, 8]],
    intervals: &[2, 3, 5],
    ornament: Ornament::Slide,
    roles: &[Role::Bass, Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_euphonium_vs_trombone/episode.py: its solo, beats 1 to 4
        &[s(7, 4), s(10, 2), s(12, 2), s(11, 4), s(10, 2), s(9, 2)],
        // ballfight_bassclarinet_vs_euphonium/timeline.py: its solo, beats 1 to 4
        &[s(10, 4), s(12, 2), s(14, 2), s(13, 4), s(12, 2), s(10, 2)],
        // ballfight/examples/bassclarinet_vs_euphonium/episode.py: its solo, beats 1 to 4
        &[s(10, 4), s(12, 2), s(14, 2), s(13, 4), s(12, 2), s(10, 2)],
    ],
};
