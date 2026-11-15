//! HORN's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// HORN's personality.
pub static PERSONALITY: Personality = Personality {
    home: 50, // D3, from ballfight_horn_vs_trombone_rematch/episode.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (61, 77),
    rhythms: &[&[0, 2, 4, 8, 10, 12], &[0, 2, 4, 7, 8, 10, 12], &[0]],
    intervals: &[2, 1, 4],
    ornament: Ornament::Slide,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_horn_vs_trombone_rematch/episode.py: its solo, beats 1 to 4
        &[s(7, 2), s(11, 2), s(14, 4), s(13, 2), s(12, 2), s(11, 4)],
        // ballfight_trombone_vs_horn/timeline.py: its solo, beats 1 to 4
        &[s(7, 2), s(11, 2), s(14, 4), s(13, 2), s(12, 2), s(11, 4)],
        // ballfight/examples/trombone_vs_horn/episode.py: its solo, beats 1 to 4
        &[s(7, 2), s(11, 2), s(14, 4), s(13, 2), s(12, 2), s(11, 4)],
    ],
};
