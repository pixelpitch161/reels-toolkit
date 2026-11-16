//! Theme: surf rock, a Studio theme (the batch of 2026-11-16; no kit mood behind it): a spring-reverb twang guitar running the chord's tones, a fuzzed rhythm guitar, a round bass, floor toms and an organ.
//! Built for E minor; any chord works (the layers voice the chord's tones in their ranges). Both
//! arrangements are layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of
//! the Composed one, with no story cues.

use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, SHAKER, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled};
use super::fresh::{self, ROOM};
use super::ska_punk::organ;
use super::spaghetti_western::twang;
use super::stadium_rock::{kick, pbass, pgtr, snare, tom};
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "surf_rock",
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
    &["Em", "Em", "C", "D", "Em", "Em", "B7", "B7"],
    &["Em", "G", "D", "B7"],
    &["Em", "C", "Am", "B7"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (150.0, 168.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Surf rock",
    text: "Fast, sunny and a little dangerous: a reverb-soaked twang guitar racing a pounding tom beat, with organ stabs on the off-beats.",
    key: "E",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 43),
};

/// The pair the theme was made for (no kit config: a Studio theme).
pub static PRESETS: [Preset; 1] = [Preset {
    config: "studio/surf_rock",
    fighters: ["ELECTRIC", "UKULELE"],
    bpm: 160.0,
    key: "E",
    scale: "minor",
    intro: ["Em", "B7"],
    chord_loop: LOOPS[0],
    tail: &[],
}];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[
        ("drums", 2),
        ("toms", 1),
        ("bass", 2),
        ("rhythm", 2),
        ("twang", 2),
        ("chords", 1),
    ],
    fill: &["drums", "toms"],
    breakdown: &[("toms", 2), ("bass", 1), ("twang", 1)],
    final_exchange: &[
        ("drums", 3),
        ("toms", 2),
        ("bass", 3),
        ("rhythm", 3),
        ("twang", 3),
        ("chords", 2),
    ],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("twang", 3), ("chords", 1)],
    build: &[
        ("drums", 3),
        ("toms", 3),
        ("bass", 3),
        ("rhythm", 3),
        ("twang", 3),
        ("riser", 1),
    ],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("rhythm", 2), ("twang", 2), ("chords", 1)],
    &[
        ("drums", 3),
        ("toms", 3),
        ("bass", 3),
        ("rhythm", 3),
        ("twang", 3),
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
                KICK => scaled(kick(), v),
                SNARE => scaled(snare(), 0.8 * v),
                HAT => scaled(synth::hat(0.05, false), 0.5 * v),
                CRASH => scaled(synth::crash(1.4), 0.3 * v),
                _ => Vec::new(),
            },
            0.45,
            0.0,
            (0, 0),
        ),
        layer(
            "toms",
            LayerKind::Perc,
            "kit",
            |_, m, _, v| match m {
                CLAP => scaled(tom(45), v),
                SHAKER => scaled(tom(50), 0.5 * v),
                _ => Vec::new(),
            },
            0.22,
            -0.15,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| scaled(pbass(m, d.clamp(0.12, 0.5)), v),
            0.5,
            0.0,
            (36, 47),
        ),
        layer(
            "rhythm",
            LayerKind::Bass,
            "guitar",
            |_, m, d, v| scaled(pgtr(m, d.clamp(0.1, 0.4), false, false), v),
            0.3,
            -0.3,
            (40, 52),
        ),
        layer(
            "twang",
            LayerKind::Arp,
            "twang",
            |_, m, d, v| twang(m, d.clamp(0.15, 0.6), v),
            0.16,
            0.3,
            (59, 76),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "organ",
            |_, m, d, v| organ(&[m], d.clamp(0.08, 0.4), v),
            0.1,
            0.25,
            (55, 69),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("surf_rock", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("surf_rock", layers, ROOM, TABLE, cues, seed)
}
