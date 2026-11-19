//! Theme: funk rock strut, a Studio theme (the batch of 2026-11-18; no kit mood behind it). Strutting and tight: a slap bass, a wah guitar and a punchy kit under short, sharp horn-section stabs.
//! Built for E dorian; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
//! layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of the Composed one, with
//! no story cues.

use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, SHAKER, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled, seed_of};
use super::fresh::{self, ROOM};
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "funk_rock",
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
    &["E7", "E7", "E7", "E7", "A7", "A7", "E7", "B7"],
    &["E7", "A7", "E7", "B7"],
    &["Em7", "A7", "Em7", "A7"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (104.0, 118.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Funk rock strut",
    text: "Strutting and tight: a slap bass, a wah guitar and a punchy kit under short, sharp horn-section stabs.",
    key: "E",
    scale: "dorian",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/funk_rock/1",
        fighters: ["TENOR SAX", "TROMBONE"],
        bpm: 110.0,
        key: "E",
        scale: "dorian",
        intro: ["E7", "B7"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/funk_rock/2",
        fighters: ["TENOR SAX", "TROMBONE"],
        bpm: 110.0,
        key: "E",
        scale: "dorian",
        intro: ["E7", "B7"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/funk_rock/3",
        fighters: ["TENOR SAX", "TROMBONE"],
        bpm: 110.0,
        key: "E",
        scale: "dorian",
        intro: ["Em7", "A7"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("tamb", 1), ("bass", 2), ("wah", 2), ("chords", 2)],
    fill: &["drums", "tamb"],
    breakdown: &[("wah", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("tamb", 3), ("bass", 3), ("wah", 3), ("chords", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("tamb", 2), ("bass", 2), ("wah", 3)],
    build: &[
        ("drums", 3),
        ("tamb", 3),
        ("bass", 3),
        ("wah", 3),
        ("chords", 3),
        ("riser", 1),
    ],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("tamb", 1), ("bass", 2), ("wah", 1), ("chords", 2)],
    &[
        ("drums", 3),
        ("tamb", 3),
        ("bass", 3),
        ("wah", 3),
        ("chords", 3),
        ("riser", 1),
    ],
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
                SNARE => scaled(super::stadium_rock::snare(), 0.8 * v),
                HAT => scaled(synth::hat(0.04, false), 0.5 * v),
                CRASH => scaled(synth::crash(1.2), 0.2 * v),
                _ => Vec::new(),
            },
            0.45,
            0.0,
            (0, 0),
        ),
        layer(
            "tamb",
            LayerKind::Perc,
            "perc",
            |_, m, _, v| match m {
                CLAP => scaled(super::second_line::clap(seed_of(v) as i64), 0.6 * v),
                SHAKER => scaled(super::stadium_rock::tamb(), 0.6 * v),
                _ => Vec::new(),
            },
            0.4,
            0.25,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| super::fresh::voices::slap(m, d.clamp(0.08, 0.3), v),
            0.42,
            0.0,
            (28, 43),
        ),
        layer(
            "wah",
            LayerKind::Arp,
            "guitar",
            |_, m, d, v| scaled(super::stadium_rock::pgtr(m, d.clamp(0.05, 0.12), true, false), v),
            0.2,
            -0.3,
            (52, 64),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "horns",
            |_, m, d, v| {
                scaled(
                    super::villain_march::brass(&[m], d.clamp(0.08, 0.18), 0.006, 3000.0, 1.2),
                    v,
                )
            },
            0.14,
            0.25,
            (58, 72),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("funk_rock", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("funk_rock", layers, ROOM, TABLE, cues, seed)
}
