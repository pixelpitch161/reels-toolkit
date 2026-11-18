//! Theme: flamenco fury, a Studio theme (the batch of 2026-11-18; no kit mood behind it). Fiery and proud: furious nylon-guitar strums, storms of handclaps, a cajón, stamping feet and a singing violin.
//! Built for A minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "flamenco",
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
    &["Am", "G", "F", "E", "Am", "G", "F", "E"],
    &["Am", "G", "F", "E"],
    &["Dm", "C", "Bb", "A"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (126.0, 156.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Flamenco fury",
    text: "Fiery and proud: furious nylon-guitar strums, storms of handclaps, a cajón, stamping feet and a singing violin.",
    key: "A",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/flamenco/1",
        fighters: ["ACOUSTIC", "VIOLIN"],
        bpm: 140.0,
        key: "A",
        scale: "minor",
        intro: ["Am", "E"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/flamenco/2",
        fighters: ["ACOUSTIC", "VIOLIN"],
        bpm: 140.0,
        key: "A",
        scale: "minor",
        intro: ["Am", "E"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/flamenco/3",
        fighters: ["ACOUSTIC", "VIOLIN"],
        bpm: 140.0,
        key: "A",
        scale: "minor",
        intro: ["Dm", "A"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("palmas", 1), ("bass", 2), ("chords", 2), ("violin", 2)],
    fill: &["drums", "palmas"],
    breakdown: &[("violin", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("palmas", 3), ("bass", 3), ("chords", 3), ("violin", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("palmas", 2), ("bass", 2), ("violin", 3)],
    build: &[
        ("drums", 3),
        ("palmas", 3),
        ("bass", 3),
        ("chords", 3),
        ("violin", 3),
        ("riser", 1),
    ],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("palmas", 1), ("bass", 2), ("chords", 2), ("violin", 1)],
    &[
        ("drums", 3),
        ("palmas", 3),
        ("bass", 3),
        ("chords", 3),
        ("violin", 3),
        ("riser", 1),
    ],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "cajon",
            |_, m, _, v| match m {
                KICK => scaled(super::stadium_rock::cajon(), v),
                SNARE => scaled(super::sea_shanty::gang_clap_hands(), 0.6 * v),
                HAT => scaled(super::stadium_rock::stick(), 0.3 * v),
                CRASH => scaled(synth::crash(1.0), 0.08 * v),
                _ => Vec::new(),
            },
            0.5,
            0.0,
            (0, 0),
        ),
        layer(
            "palmas",
            LayerKind::Perc,
            "palmas",
            |_, m, _, v| match m {
                CLAP => scaled(super::sea_shanty::gang_clap_hands(), v),
                SHAKER => scaled(super::sea_shanty::stomp(v, seed_of(v)), 0.8),
                _ => Vec::new(),
            },
            0.4,
            0.2,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| scaled(super::bossa_noir::pluck(m, d.clamp(0.2, 0.5), 1, 0.4, 0.995), v),
            0.6,
            0.0,
            (40, 52),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "guitar",
            |_, m, d, v| {
                scaled(
                    super::dark_tango::pluck(m, d.clamp(0.06, 0.18), i64::from(m % 4), 0.95, 0.99),
                    v,
                )
            },
            0.28,
            -0.25,
            (52, 67),
        ),
        layer(
            "violin",
            LayerKind::Arp,
            "violin",
            |_, m, d, v| {
                scaled(
                    super::baroque_storm::strng(m, d.clamp(0.15, 0.5), 2600.0, 0.007, 0.03),
                    v,
                )
            },
            0.14,
            0.3,
            (64, 79),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("flamenco", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("flamenco", layers, ROOM, TABLE, cues, seed)
}
