//! BASS DRUM's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// BASS DRUM's personality.
pub static PERSONALITY: Personality = Personality {
    home: 36, // C2, from ballfight_bassdrum_vs_tuba/episode.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (36, 36),
    rhythms: &[&[0, 4, 6, 12], &[0, 2, 3, 4, 14], &[2, 4, 12, 14]],
    intervals: &[2],
    ornament: Ornament::None,
    roles: &[Role::Bass, Role::Answer],
    motifs: &[
        // ballfight_bassdrum_vs_tuba/episode.py: its solo, beats 1 to 4
        &[s(0, 4), s(0, 2), s(0, 6), s(0, 4)],
        // ballfight_bassdrum_vs_tuba/episode.py: its solo, beats 5 to 8
        &[s(0, 2), s(0, 1), s(0, 1), s(0, 10), s(0, 4)],
    ],
};
