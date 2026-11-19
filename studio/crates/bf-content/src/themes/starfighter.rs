//! Theme: starfighter, a Studio theme (the batch of 2026-11-18; no kit mood behind it). A dogfight in deep space: synth arpeggios and a string ostinato race under a heroic horn line and big toms.
//! Built for D minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "starfighter",
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
    &["Dm", "Bb", "C", "Dm", "Dm", "Bb", "Gm", "A"],
    &["Dm", "Bb", "C", "A"],
    &["Dm", "F", "C", "Bb"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (124.0, 148.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Starfighter",
    text: "A dogfight in deep space: synth arpeggios and a string ostinato race under a heroic horn line and big toms.",
    key: "D",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/starfighter/1",
        fighters: ["SYNTH", "HORN"],
        bpm: 136.0,
        key: "D",
        scale: "minor",
        intro: ["Dm", "A"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/starfighter/2",
        fighters: ["SYNTH", "HORN"],
        bpm: 136.0,
        key: "D",
        scale: "minor",
        intro: ["Dm", "A"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/starfighter/3",
        fighters: ["SYNTH", "HORN"],
        bpm: 136.0,
        key: "D",
        scale: "minor",
        intro: ["Dm", "Bb"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("strings", 2), ("chords", 2), ("horns", 1)],
    fill: &["drums"],
    breakdown: &[("strings", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("strings", 3), ("chords", 3), ("horns", 2)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("strings", 3), ("horns", 1)],
    build: &[("drums", 3), ("bass", 3), ("strings", 3), ("chords", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("strings", 1), ("chords", 2), ("horns", 1)],
    &[("drums", 3), ("bass", 3), ("strings", 3), ("chords", 3), ("riser", 1)],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "drums",
            |_, m, _, v| match m {
                KICK => scaled(super::orchestral_drill::tom(70.0, 0.25, 1.0), v),
                SNARE => scaled(super::baroque_storm::punch(), 0.7 * v),
                HAT => super::orchestral_drill::hat(0.4 * v, false),
                CRASH => scaled(synth::crash(2.0), 0.25 * v),
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
            |_, m, d, v| super::dark_breakbeat::reese_plain(m, d.clamp(0.12, 0.8), v),
            0.4,
            0.0,
            (29, 41),
        ),
        layer(
            "strings",
            LayerKind::Arp,
            "strings",
            |_, m, d, v| scaled(super::baroque_storm::strng(m, d.min(0.18), 2600.0, 0.003, 0.01), v),
            0.16,
            -0.25,
            (57, 72),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "arp",
            |_, m, d, v| super::fresh::voices::supersaw(m, d.clamp(0.08, 0.2), 3200.0, v),
            0.12,
            0.25,
            (62, 76),
        ),
        layer(
            "horns",
            LayerKind::Pad,
            "horns",
            |_, m, d, v| scaled(super::villain_march::horn_pad(&[m], d.min(3.0)), v),
            0.07,
            0.0,
            (53, 65),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("starfighter", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("starfighter", layers, ROOM, TABLE, cues, seed)
}
