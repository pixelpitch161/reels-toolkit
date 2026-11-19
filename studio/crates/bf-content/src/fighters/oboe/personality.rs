//! OBOE's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// OBOE's personality.
pub static PERSONALITY: Personality = Personality {
    home: 62, // D4, from ballfight_oboe_vs_bassoon/episode.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (72, 89),
    rhythms: &[&[0, 4, 6, 8, 10, 12, 14], &[0, 2, 4, 6, 8]],
    intervals: &[1, 2, 3],
    ornament: Ornament::Grace,
    roles: &[Role::Melody, Role::Answer],
    motifs: &[
        // ballfight_oboe_vs_bassoon/episode.py: its solo, beats 1 to 4
        &[s(11, 4), s(12, 2), s(11, 2), s(9, 2), s(7, 2), s(6, 2), s(8, 2)],
        // ballfight_clarinet_vs_oboe/timeline.py: its solo, beats 1 to 4
        &[s(11, 4), s(12, 2), s(11, 2), s(10, 2), s(9, 2), s(8, 2), s(9, 2)],
        // ballfight/examples/clarinet_vs_oboe/episode.py: its solo, beats 1 to 4
        &[s(11, 4), s(12, 2), s(11, 2), s(10, 2), s(9, 2), s(8, 2), s(9, 2)],
    ],
};
