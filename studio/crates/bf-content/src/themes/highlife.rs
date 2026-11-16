//! Theme: highlife, a Studio theme (the batch of 2026-11-16; no kit mood behind it): two interlocking clean guitars, a bell and congas, a walking upright bass and a soft horn pad over a light kit.
//! Built for C major; any chord works (the layers voice the chord's tones in their ranges). Both
//! arrangements are layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of
//! the Composed one, with no story cues.

use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, SHAKER, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled, seed_of};
use super::bossa_noir::{kick_felt, pluck, rim};
use super::fresh::{self, ROOM};
use super::heist::upright;
use super::mambo::{Conga, bell, conga};
use super::villain_march::horn_pad;
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "highlife",
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
    &["C", "C", "F", "G", "C", "Am", "F", "G"],
    &["C", "F", "G", "C"],
    &["C", "Am", "F", "G"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (112.0, 128.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Highlife",
    text: "Bright and bouncing: two interlocking clean guitars, a bell pattern, congas and a warm horn bed from a West African dance band.",
    key: "C",
    scale: "major",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for (no kit config: a Studio theme).
pub static PRESETS: [Preset; 1] = [Preset {
    config: "studio/highlife",
    fighters: ["CORNET", "SOUSAPHONE"],
    bpm: 120.0,
    key: "C",
    scale: "major",
    intro: ["C", "G"],
    chord_loop: LOOPS[0],
    tail: &[],
}];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[
        ("drums", 2),
        ("bells", 1),
        ("bass", 2),
        ("chords", 2),
        ("lead", 2),
        ("horns", 1),
    ],
    fill: &["drums", "bells"],
    breakdown: &[("bells", 2), ("lead", 1), ("bass", 1)],
    final_exchange: &[
        ("drums", 3),
        ("bells", 3),
        ("bass", 3),
        ("chords", 3),
        ("lead", 3),
        ("horns", 2),
    ],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bells", 2), ("bass", 2), ("lead", 3), ("horns", 1)],
    build: &[
        ("drums", 3),
        ("bells", 3),
        ("bass", 3),
        ("chords", 3),
        ("lead", 3),
        ("riser", 1),
    ],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("chords", 2), ("lead", 1), ("horns", 1)],
    &[
        ("drums", 3),
        ("bells", 3),
        ("bass", 3),
        ("chords", 3),
        ("lead", 3),
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
                KICK => scaled(kick_felt(), v),
                SNARE => scaled(rim(seed_of(v)), 0.7 * v),
                HAT => scaled(synth::hat(0.03, false), 0.4 * v),
                CRASH => scaled(synth::crash(1.0), 0.15 * v),
                _ => Vec::new(),
            },
            0.4,
            0.0,
            (0, 0),
        ),
        layer(
            "bells",
            LayerKind::Perc,
            "perc",
            |_, m, _, v| match m {
                CLAP => scaled(conga(Conga::Slap, false), v),
                SHAKER => scaled(bell(v > 0.3), 0.6 * v),
                _ => Vec::new(),
            },
            0.5,
            0.2,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| scaled(upright(m, d.min(0.45)), v),
            0.6,
            0.0,
            (36, 48),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "guitar",
            |_, m, d, v| scaled(pluck(m, d.clamp(0.1, 0.25), i64::from(m % 4), 0.75, 0.993), v),
            0.18,
            -0.3,
            (55, 69),
        ),
        layer(
            "lead",
            LayerKind::Arp,
            "guitar2",
            |_, m, d, v| scaled(pluck(m, d.clamp(0.1, 0.25), i64::from(m % 5), 0.85, 0.993), v),
            0.25,
            0.3,
            (67, 81),
        ),
        layer(
            "horns",
            LayerKind::Pad,
            "horns",
            |_, m, d, v| scaled(horn_pad(&[m], d.min(3.0)), v),
            0.05,
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
    fresh::bed("highlife", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("highlife", layers, ROOM, TABLE, cues, seed)
}
