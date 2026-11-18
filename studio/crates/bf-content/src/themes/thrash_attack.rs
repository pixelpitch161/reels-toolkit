//! Theme: thrash attack, a Studio theme (the batch of 2026-11-18; no kit mood behind it). Fast and furious: palm-muted chugging guitars, a driven bass and double-kick drums under crashing cymbals.
//! Built for E minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "thrash_attack",
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
    &["Em", "Em", "C", "D", "Em", "Em", "Bb", "B"],
    &["Em", "C", "D", "B"],
    &["Em", "G", "F", "Em"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (150.0, 172.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Thrash attack",
    text: "Fast and furious: palm-muted chugging guitars, a driven bass and double-kick drums under crashing cymbals.",
    key: "E",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 44),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/thrash_attack/1",
        fighters: ["ELECTRIC", "DRUMS"],
        bpm: 164.0,
        key: "E",
        scale: "minor",
        intro: ["Em", "B"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/thrash_attack/2",
        fighters: ["ELECTRIC", "DRUMS"],
        bpm: 164.0,
        key: "E",
        scale: "minor",
        intro: ["Em", "B"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/thrash_attack/3",
        fighters: ["ELECTRIC", "DRUMS"],
        bpm: 164.0,
        key: "E",
        scale: "minor",
        intro: ["Em", "Em"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("double", 1), ("bass", 2), ("chug", 2), ("chords", 2)],
    fill: &["drums", "double"],
    breakdown: &[("chug", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("double", 3), ("bass", 3), ("chug", 3), ("chords", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("double", 2), ("bass", 2), ("chug", 3)],
    build: &[
        ("drums", 3),
        ("double", 3),
        ("bass", 3),
        ("chug", 3),
        ("chords", 3),
        ("riser", 1),
    ],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("double", 1), ("bass", 2), ("chug", 1), ("chords", 2)],
    &[
        ("drums", 3),
        ("double", 3),
        ("bass", 3),
        ("chug", 3),
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
                KICK => scaled(super::prog_duel::kick(), v),
                SNARE => scaled(super::prog_duel::snare(seed_of(v)), 0.8 * v),
                HAT => scaled(super::prog_duel::hat(false), 0.4 * v),
                CRASH => scaled(synth::crash(1.4), 0.3 * v),
                _ => Vec::new(),
            },
            0.45,
            0.0,
            (0, 0),
        ),
        layer(
            "double",
            LayerKind::Perc,
            "kit",
            |_, m, _, v| match m {
                CLAP => scaled(super::prog_duel::kick(), 0.6 * v),
                SHAKER => scaled(super::prog_duel::kick(), 0.4 * v),
                _ => Vec::new(),
            },
            0.3,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| super::prog_duel::bass_note(m, d.clamp(0.08, 0.3), v, 0.9),
            0.28,
            0.0,
            (28, 40),
        ),
        layer(
            "chug",
            LayerKind::Bass,
            "guitar",
            |_, m, d, v| scaled(super::stadium_rock::pgtr(m, d.clamp(0.06, 0.2), true, false), v),
            0.6,
            -0.25,
            (40, 52),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "guitar2",
            |_, m, d, v| scaled(super::stadium_rock::pgtr(m, d.clamp(0.1, 0.4), false, true), v),
            0.18,
            0.25,
            (40, 52),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("thrash_attack", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("thrash_attack", layers, ROOM, TABLE, cues, seed)
}
