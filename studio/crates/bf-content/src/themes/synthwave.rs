//! Theme: synthwave night drive, a Studio theme (the batch of 2026-11-18; no kit mood behind it). A neon eighties night drive: shimmering analog arps, a pulsing bass, a gated snare, a chorused guitar and a sax solo feel.
//! Built for A minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
//! layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of the Composed one, with
//! no story cues.

use bf_sound::music::layers::{ArrangementTable, CRASH, Cue, HAT, KICK, Layer, LayerKind, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled, seed_of};
use super::fresh::{self, ROOM};
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "synthwave",
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
    &["Am", "F", "C", "G", "Am", "F", "C", "E"],
    &["Am", "F", "C", "G"],
    &["F", "G", "Am", "Am"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (108.0, 128.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Synthwave night drive",
    text: "A neon eighties night drive: shimmering analog arps, a pulsing bass, a gated snare, a chorused guitar and a sax solo feel.",
    key: "A",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/synthwave/1",
        fighters: ["SYNTH", "ALTO SAX"],
        bpm: 118.0,
        key: "A",
        scale: "minor",
        intro: ["Am", "E"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/synthwave/2",
        fighters: ["SYNTH", "ALTO SAX"],
        bpm: 118.0,
        key: "A",
        scale: "minor",
        intro: ["Am", "G"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/synthwave/3",
        fighters: ["SYNTH", "ALTO SAX"],
        bpm: 118.0,
        key: "A",
        scale: "minor",
        intro: ["F", "Am"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("arp", 2), ("chords", 1), ("guitar", 2)],
    fill: &["drums"],
    breakdown: &[("arp", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("arp", 3), ("chords", 2), ("guitar", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("arp", 3), ("guitar", 3)],
    build: &[("drums", 3), ("bass", 3), ("arp", 3), ("guitar", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("arp", 1), ("chords", 2), ("guitar", 1)],
    &[("drums", 3), ("bass", 3), ("arp", 3), ("guitar", 3), ("riser", 1)],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "kit",
            |_, m, _, v| match m {
                KICK => scaled(super::prog_duel::kick(), v),
                SNARE => scaled(super::prog_duel::snare(seed_of(v)), 0.8 * v),
                HAT => super::brass_house::hat(0.4 * v, false, seed_of(v)),
                CRASH => scaled(synth::crash(1.8), 0.2 * v),
                _ => Vec::new(),
            },
            0.42,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| super::fresh::voices::supersaw(m, d.clamp(0.08, 0.25), 900.0, v),
            0.4,
            0.0,
            (33, 45),
        ),
        layer(
            "arp",
            LayerKind::Arp,
            "arp",
            |_, m, d, v| super::fresh::voices::chip(m, d.clamp(0.08, 0.2), 0.25, v),
            0.08,
            0.25,
            (64, 79),
        ),
        layer(
            "chords",
            LayerKind::Pad,
            "pad",
            |_, m, d, v| scaled(super::dark_breakbeat::cold_pad(&[m], d, 2200.0), v),
            0.14,
            -0.1,
            (53, 68),
        ),
        layer(
            "guitar",
            LayerKind::Comp,
            "guitar",
            |_, m, d, v| {
                scaled(
                    super::bossa_noir::pluck(m, d.clamp(0.1, 0.3), i64::from(m % 4), 0.8, 0.993),
                    v,
                )
            },
            0.12,
            -0.3,
            (57, 72),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("synthwave", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("synthwave", layers, ROOM, TABLE, cues, seed)
}
