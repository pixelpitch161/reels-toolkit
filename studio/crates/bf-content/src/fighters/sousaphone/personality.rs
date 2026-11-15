//! SOUSAPHONE's musical personality (milestone 5): its register, rhythms, intervals, ornament, roles and the
//! signature motifs the composer builds its line and victory solo from. Every motif is in scale degrees from
//! its home key and is taken from that fighter's own episodes (read, never edited), named beside it.

use crate::personality::{Ornament, Personality, Role, Step};

const fn s(deg: i8, len: u8) -> Step {
    Step { deg, len }
}

/// SOUSAPHONE's personality.
pub static PERSONALITY: Personality = Personality {
    home: 36, // C2, from ballfight_flute_vs_sousaphone/episode.py
    scale: [0, 2, 3, 5, 7, 8, 10],
    register: (36, 50),
    rhythms: &[&[0, 2, 4, 6, 8, 10, 12], &[0, 2, 4, 6, 8]],
    intervals: &[3, 5, 2],
    ornament: Ornament::Stab,
    roles: &[Role::Bass, Role::Answer],
    motifs: &[
        // ballfight_flute_vs_sousaphone/episode.py: its solo, beats 1 to 4
        &[s(0, 2), s(4, 2), s(7, 2), s(4, 2), s(6, 2), s(5, 2), s(4, 4)],
        // ballfight_flute_vs_sousaphone/episode.py: its line, the first 8 notes
        &[s(0, 2), s(4, 2), s(0, 2), s(4, 2), s(0, 2), s(4, 2), s(2, 2), s(3, 2)],
    ],
};
