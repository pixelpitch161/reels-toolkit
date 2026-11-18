//! Theme: chiptune brawl, a Studio theme (the batch of 2026-11-18; no kit mood behind it). An eight-bit arcade boss fight: square-wave leads, a triangle bass, noise drums and lightning-fast arpeggios.
//! Built for C major; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "chiptune",
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
    &["C", "Am", "F", "G", "C", "Am", "F", "G"],
    &["C", "G", "Am", "F"],
    &["Am", "F", "C", "G"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (140.0, 170.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Chiptune brawl",
    text: "An eight-bit arcade boss fight: square-wave leads, a triangle bass, noise drums and lightning-fast arpeggios.",
    key: "C",
    scale: "major",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 44),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/chiptune/1",
        fighters: ["SYNTH", "PICCOLO"],
        bpm: 156.0,
        key: "C",
        scale: "major",
        intro: ["C", "G"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/chiptune/2",
        fighters: ["SYNTH", "PICCOLO"],
        bpm: 156.0,
        key: "C",
        scale: "major",
        intro: ["C", "F"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/chiptune/3",
        fighters: ["SYNTH", "PICCOLO"],
        bpm: 156.0,
        key: "C",
        scale: "major",
        intro: ["Am", "G"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("chords", 2), ("lead", 2)],
    fill: &["drums"],
    breakdown: &[("lead", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("chords", 3), ("lead", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("lead", 3)],
    build: &[("drums", 3), ("bass", 3), ("chords", 3), ("lead", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("chords", 2), ("lead", 1)],
    &[("drums", 3), ("bass", 3), ("chords", 3), ("lead", 3), ("riser", 1)],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "kit",
            |_, m, _, v| match m {
                KICK => super::fresh::voices::chip_tri(36, 0.08, v),
                SNARE => scaled(super::fresh::voices::chip_noise(0.12, false, seed_of(v)), 0.6 * v),
                HAT => scaled(super::fresh::voices::chip_noise(0.04, true, seed_of(v)), 0.4 * v),
                CRASH => scaled(super::fresh::voices::chip_noise(0.5, false, 9), 0.25 * v),
                _ => Vec::new(),
            },
            0.4,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "tri",
            |_, m, d, v| super::fresh::voices::chip_tri(m, d.clamp(0.08, 0.3), v),
            0.45,
            0.0,
            (36, 48),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "pulse",
            |_, m, d, v| super::fresh::voices::chip(m, d.clamp(0.06, 0.18), 0.5, v),
            0.08,
            -0.2,
            (55, 69),
        ),
        layer(
            "lead",
            LayerKind::Arp,
            "square",
            |_, m, d, v| super::fresh::voices::chip(m, d.clamp(0.06, 0.15), 0.125, v),
            0.08,
            0.25,
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
    fresh::bed("chiptune", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("chiptune", layers, ROOM, TABLE, cues, seed)
}
