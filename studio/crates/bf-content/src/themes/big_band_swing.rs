//! Theme: big band swing, a Studio theme (the batch of 2026-11-18; no kit mood behind it). Swaggering ballroom roar: a sax section, trumpet stabs and trombone shouts over a walking bass and a swinging kit.
//! Built for F major; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "big_band_swing",
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
    &["F", "D7", "Gm7", "C7", "F", "D7", "Gm7", "C7"],
    &["F", "Bb", "C7", "F"],
    &["Gm7", "C7", "F", "D7"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (128.0, 152.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Big band swing",
    text: "Swaggering ballroom roar: a sax section, trumpet stabs and trombone shouts over a walking bass and a swinging kit.",
    key: "F",
    scale: "major",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/big_band_swing/1",
        fighters: ["TROMBONE", "TENOR SAX"],
        bpm: 140.0,
        key: "F",
        scale: "major",
        intro: ["F", "C7"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/big_band_swing/2",
        fighters: ["TROMBONE", "TENOR SAX"],
        bpm: 140.0,
        key: "F",
        scale: "major",
        intro: ["F", "F"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/big_band_swing/3",
        fighters: ["TROMBONE", "TENOR SAX"],
        bpm: 140.0,
        key: "F",
        scale: "major",
        intro: ["Gm7", "D7"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("saxes", 1), ("chords", 2), ("shouts", 2)],
    fill: &["drums"],
    breakdown: &[("shouts", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("saxes", 2), ("chords", 3), ("shouts", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("saxes", 1), ("shouts", 3)],
    build: &[("drums", 3), ("bass", 3), ("chords", 3), ("shouts", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("saxes", 1), ("chords", 2), ("shouts", 1)],
    &[("drums", 3), ("bass", 3), ("chords", 3), ("shouts", 3), ("riser", 1)],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "kit",
            |_, m, _, v| match m {
                KICK => scaled(super::hot_jazz::kick_felt(), v),
                SNARE => scaled(super::heist::brush(seed_of(v)), 0.7 * v),
                HAT => scaled(super::noir_jazz::ride(v > 0.7), 0.45 * v),
                CRASH => scaled(synth::crash(1.2), 0.15 * v),
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
            |_, m, d, v| super::noir_jazz::upright(m, d.min(0.45), v),
            0.6,
            0.0,
            (36, 48),
        ),
        layer(
            "saxes",
            LayerKind::Pad,
            "saxes",
            |_, m, d, v| scaled(super::balkan_fire::reed(&[m], d.min(2.0), 0.03, 0.15, 1800.0, 0.0), v),
            0.08,
            -0.2,
            (55, 67),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "brass",
            |_, m, d, v| {
                scaled(
                    super::villain_march::brass(&[m], d.clamp(0.12, 0.3), 0.01, 2600.0, 1.0),
                    v,
                )
            },
            0.14,
            0.2,
            (58, 72),
        ),
        layer(
            "shouts",
            LayerKind::Arp,
            "trumpets",
            |_, m, d, v| {
                scaled(
                    super::villain_march::brass(&[m], d.clamp(0.1, 0.2), 0.008, 3600.0, 1.0),
                    v,
                )
            },
            0.1,
            0.3,
            (67, 81),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("big_band_swing", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("big_band_swing", layers, ROOM, TABLE, cues, seed)
}
