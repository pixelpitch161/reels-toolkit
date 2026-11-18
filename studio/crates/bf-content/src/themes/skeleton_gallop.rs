//! Theme: skeleton gallop, a Studio theme (the batch of 2026-11-18; no kit mood behind it). Rattling bones and a creepy graveyard whirl: a xylophone, plucked strings, a bass clarinet, a tolling bell and a tambourine.
//! Built for G harmonic minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
//! layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of the Composed one, with
//! no story cues.

use bf_sound::music::layers::{ArrangementTable, CRASH, Cue, HAT, KICK, Layer, LayerKind, SNARE};
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled, seed_of};
use super::fresh::{self, ROOM};
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "skeleton_gallop",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| classic(),
    composed: Some(|_, c, s| composed(c, s)),
};

/// The chord loops, the first one the preset's.
pub static LOOPS: [&[&str]; 3] = [
    &["Gm", "Gm", "D7", "D7", "Gm", "Cm", "D7", "Gm"],
    &["Gm", "Cm", "D7", "Gm"],
    &["Gm", "Eb", "Cm", "D7"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (120.0, 144.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Skeleton gallop",
    text: "Rattling bones and a creepy graveyard whirl: a xylophone, plucked strings, a bass clarinet, a tolling bell and a tambourine.",
    key: "G",
    scale: "harmonic minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/skeleton_gallop/1",
        fighters: ["MARIMBA", "BASS CLARINET"],
        bpm: 132.0,
        key: "G",
        scale: "harmonic minor",
        intro: ["Gm", "Gm"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/skeleton_gallop/2",
        fighters: ["MARIMBA", "BASS CLARINET"],
        bpm: 132.0,
        key: "G",
        scale: "harmonic minor",
        intro: ["Gm", "Gm"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/skeleton_gallop/3",
        fighters: ["MARIMBA", "BASS CLARINET"],
        bpm: 132.0,
        key: "G",
        scale: "harmonic minor",
        intro: ["Gm", "D7"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("chords", 2), ("xylophone", 2)],
    fill: &["drums"],
    breakdown: &[("xylophone", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("chords", 3), ("xylophone", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("xylophone", 3)],
    build: &[("drums", 3), ("bass", 3), ("chords", 3), ("xylophone", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("chords", 2), ("xylophone", 1)],
    &[("drums", 3), ("bass", 3), ("chords", 3), ("xylophone", 3), ("riser", 1)],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "bones",
            |_, m, _, v| match m {
                KICK => scaled(super::hot_jazz::kick_felt(), v),
                SNARE => scaled(super::hot_jazz::woodblock(v > 0.6), 0.7 * v),
                HAT => scaled(super::big_top::tambourine(v, seed_of(v)), 0.35 * v),
                CRASH => scaled(super::gothic_horror::bell(55, 1.5, 1.0), 0.15 * v),
                _ => Vec::new(),
            },
            0.45,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "reed",
            |_, m, d, v| scaled(super::dark_tango::reed(m, d.clamp(0.12, 0.4), 0.01, 0.5), v),
            0.3,
            0.0,
            (34, 46),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "pizz",
            |_, m, _, v| scaled(super::heist::pizz(&[m]), v),
            0.22,
            -0.25,
            (55, 69),
        ),
        layer(
            "xylophone",
            LayerKind::Arp,
            "xylo",
            |_, m, _, v| scaled(super::dark_drumline::marimba(m), v),
            0.22,
            0.3,
            (72, 86),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("skeleton_gallop", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("skeleton_gallop", layers, ROOM, TABLE, cues, seed)
}
