//! Theme: boogie shuffle, a Studio theme (the batch of 2026-11-18; no kit mood behind it). A gritty, swaggering shuffle that rolls like a juke joint: boogie-woogie piano, a crunchy guitar, organ swells and a sax.
//! Built for E mixolydian; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "boogie_shuffle",
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
    &["E7", "E7", "A7", "E7", "B7", "A7", "E7", "B7"],
    &["E7", "A7", "E7", "B7"],
    &["E7", "E7", "A7", "B7"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (110.0, 132.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Boogie shuffle",
    text: "A gritty, swaggering shuffle that rolls like a juke joint: boogie-woogie piano, a crunchy guitar, organ swells and a sax.",
    key: "E",
    scale: "mixolydian",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/boogie_shuffle/1",
        fighters: ["PIANO", "ELECTRIC"],
        bpm: 120.0,
        key: "E",
        scale: "mixolydian",
        intro: ["E7", "B7"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/boogie_shuffle/2",
        fighters: ["PIANO", "ELECTRIC"],
        bpm: 120.0,
        key: "E",
        scale: "mixolydian",
        intro: ["E7", "B7"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/boogie_shuffle/3",
        fighters: ["PIANO", "ELECTRIC"],
        bpm: 120.0,
        key: "E",
        scale: "mixolydian",
        intro: ["E7", "B7"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("boogie", 2), ("chords", 2), ("organ", 1)],
    fill: &["drums"],
    breakdown: &[("boogie", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("boogie", 3), ("chords", 3), ("organ", 2)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("boogie", 3), ("organ", 1)],
    build: &[("drums", 3), ("bass", 3), ("boogie", 3), ("chords", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("boogie", 1), ("chords", 2), ("organ", 1)],
    &[("drums", 3), ("bass", 3), ("boogie", 3), ("chords", 3), ("riser", 1)],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "kit",
            |_, m, _, v| match m {
                KICK => scaled(super::stadium_rock::kick(), v),
                SNARE => scaled(super::stadium_rock::snare(), 0.75 * v),
                HAT => scaled(super::noir_jazz::ride(false), 0.4 * v),
                CRASH => scaled(synth::crash(1.2), 0.2 * v),
                _ => Vec::new(),
            },
            0.45,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| scaled(super::stadium_rock::pbass(m, d.clamp(0.12, 0.4)), v),
            0.5,
            0.0,
            (36, 48),
        ),
        layer(
            "boogie",
            LayerKind::Arp,
            "piano",
            |_, m, d, v| super::fresh::voices::piano(m, d.clamp(0.1, 0.25), v),
            0.2,
            -0.2,
            (45, 60),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "guitar",
            |_, m, d, v| scaled(super::stadium_rock::pgtr(m, d.clamp(0.1, 0.3), false, false), v),
            0.22,
            0.25,
            (40, 52),
        ),
        layer(
            "organ",
            LayerKind::Pad,
            "organ",
            |_, m, d, v| super::ska_punk::organ(&[m], d.min(2.0), v),
            0.06,
            0.1,
            (55, 67),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("boogie_shuffle", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("boogie_shuffle", layers, ROOM, TABLE, cues, seed)
}
