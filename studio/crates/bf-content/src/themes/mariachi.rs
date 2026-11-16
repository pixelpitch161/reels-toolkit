//! Theme: mariachi showdown, a Studio theme (the batch of 2026-11-16; no kit mood behind it): a vihuela strumming the chord, a guitarrón on the root, a violin section, bright trumpet stabs and light percussion.
//! Built for G major; any chord works (the layers voice the chord's tones in their ranges). Both
//! arrangements are layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of
//! the Composed one, with no story cues.

use bf_sound::music::layers::{ArrangementTable, CRASH, Cue, HAT, KICK, Layer, LayerKind, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled, seed_of};
use super::baroque_storm::strng;
use super::bossa_noir::{kick_felt, pluck, shaker};
use super::fresh::{self, ROOM};
use super::mambo::clave;
use super::villain_march::brass;
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "mariachi",
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
    &["G", "G", "C", "D7", "G", "G", "D7", "D7"],
    &["G", "Em", "C", "D7"],
    &["G", "C", "D7", "G"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (130.0, 160.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Mariachi showdown",
    text: "Sunlit and proud, with a dueling edge: a strummed vihuela, a guitarrón bass, violin swells and bright trumpet stabs.",
    key: "G",
    scale: "major",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for (no kit config: a Studio theme).
pub static PRESETS: [Preset; 1] = [Preset {
    config: "studio/mariachi",
    fighters: ["TRUMPET", "MELLOPHONE"],
    bpm: 144.0,
    key: "G",
    scale: "major",
    intro: ["G", "D7"],
    chord_loop: LOOPS[0],
    tail: &[],
}];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 1), ("bass", 2), ("vihuela", 2), ("violins", 1), ("chords", 1)],
    fill: &["drums", "vihuela"],
    breakdown: &[("vihuela", 1), ("violins", 2)],
    final_exchange: &[("drums", 2), ("bass", 3), ("vihuela", 3), ("violins", 2), ("chords", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 1), ("bass", 2), ("vihuela", 2), ("violins", 3)],
    build: &[
        ("drums", 3),
        ("bass", 3),
        ("vihuela", 3),
        ("violins", 3),
        ("chords", 2),
        ("riser", 1),
    ],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 1), ("bass", 2), ("vihuela", 2), ("chords", 1)],
    &[
        ("drums", 3),
        ("bass", 3),
        ("vihuela", 3),
        ("violins", 3),
        ("chords", 2),
        ("riser", 1),
    ],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "perc",
            |_, m, _, v| match m {
                KICK => scaled(kick_felt(), v),
                SNARE => scaled(clave(), 0.5 * v),
                HAT => scaled(shaker(seed_of(v), v > 0.5), 0.6 * v),
                CRASH => scaled(synth::crash(1.0), 0.15 * v),
                _ => Vec::new(),
            },
            0.6,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "guitarron",
            |_, m, d, v| scaled(pluck(m, d.clamp(0.2, 0.6), 1, 0.4, 0.995), v),
            0.7,
            0.0,
            (40, 52),
        ),
        layer(
            "vihuela",
            LayerKind::Comp,
            "vihuela",
            |_, m, d, v| scaled(pluck(m, d.clamp(0.1, 0.25), i64::from(m % 3), 0.8, 0.992), v),
            0.22,
            -0.3,
            (59, 72),
        ),
        layer(
            "violins",
            LayerKind::Pad,
            "violins",
            |_, m, d, v| scaled(strng(m, d, 1800.0, 0.004, 0.08), v),
            0.15,
            0.3,
            (67, 79),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "trumpets",
            |_, m, d, v| scaled(brass(&[m], d.clamp(0.12, 0.3), 0.01, 3000.0, 1.0), v),
            0.12,
            0.15,
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
    fresh::bed("mariachi", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("mariachi", layers, ROOM, TABLE, cues, seed)
}
