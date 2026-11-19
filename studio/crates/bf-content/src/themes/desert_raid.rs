//! Theme: desert raid, a Studio theme (the batch of 2026-11-18; no kit mood behind it). A sandstorm charge across the dunes: oud-style plucks, frame drums, a snake-charmer reed lead and a driving string ostinato.
//! Built for E harmonic minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
//! layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of the Composed one, with
//! no story cues.

use bf_sound::music::layers::{ArrangementTable, CRASH, Cue, HAT, KICK, Layer, LayerKind, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled};
use super::fresh::{self, ROOM};
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "desert_raid",
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
    &["Em", "Em", "F", "Em", "Em", "Am", "B7", "Em"],
    &["Em", "F", "Em", "B7"],
    &["Em", "Am", "F", "B7"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (112.0, 136.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Desert raid",
    text: "A sandstorm charge across the dunes: oud-style plucks, frame drums, a snake-charmer reed lead and a driving string ostinato.",
    key: "E",
    scale: "harmonic minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/desert_raid/1",
        fighters: ["OBOE", "SOPRANO SAX"],
        bpm: 124.0,
        key: "E",
        scale: "harmonic minor",
        intro: ["Em", "Em"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/desert_raid/2",
        fighters: ["OBOE", "SOPRANO SAX"],
        bpm: 124.0,
        key: "E",
        scale: "harmonic minor",
        intro: ["Em", "B7"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/desert_raid/3",
        fighters: ["OBOE", "SOPRANO SAX"],
        bpm: 124.0,
        key: "E",
        scale: "harmonic minor",
        intro: ["Em", "B7"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("strings", 2), ("chords", 2), ("reed", 1)],
    fill: &["drums"],
    breakdown: &[("strings", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("strings", 3), ("chords", 3), ("reed", 2)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("strings", 3), ("reed", 1)],
    build: &[("drums", 3), ("bass", 3), ("strings", 3), ("chords", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("strings", 1), ("chords", 2), ("reed", 1)],
    &[("drums", 3), ("bass", 3), ("strings", 3), ("chords", 3), ("riser", 1)],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "frame",
            |_, m, _, v| match m {
                KICK => scaled(super::sea_shanty::frame_drum(true), v),
                SNARE => scaled(super::sea_shanty::frame_drum(false), 0.8 * v),
                HAT => scaled(super::balkan_fire::sticks(v), 0.4),
                CRASH => scaled(synth::crash(1.6), 0.15 * v),
                _ => Vec::new(),
            },
            0.5,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "low",
            |_, m, d, v| super::brass_trailer::strings_low(m, d.min(1.0), v),
            0.4,
            0.0,
            (33, 45),
        ),
        layer(
            "strings",
            LayerKind::Arp,
            "strings",
            |_, m, d, v| scaled(super::baroque_storm::strng(m, d.min(0.2), 2400.0, 0.004, 0.01), v),
            0.16,
            -0.25,
            (52, 67),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "oud",
            |_, m, d, v| {
                scaled(
                    super::dark_tango::pluck(m, d.clamp(0.1, 0.3), i64::from(m % 4), 0.8, 0.99),
                    v,
                )
            },
            0.22,
            0.25,
            (52, 67),
        ),
        layer(
            "reed",
            LayerKind::Pad,
            "reed",
            |_, m, d, v| scaled(super::dark_tango::reed(m, d.min(1.5), 0.03, 0.6), v),
            0.07,
            0.1,
            (64, 76),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("desert_raid", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("desert_raid", layers, ROOM, TABLE, cues, seed)
}
