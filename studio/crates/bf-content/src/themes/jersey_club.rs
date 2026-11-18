//! Theme: jersey club bounce, a Studio theme (the batch of 2026-11-18; no kit mood behind it). Bouncy and chopped: triplet kicks that make everything jump, chopped synth hits, a sub bass and claps.
//! Built for C minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "jersey_club",
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
    &["Cm", "Cm", "Ab", "Bb", "Cm", "Cm", "Ab", "G"],
    &["Cm", "Ab", "Eb", "Bb"],
    &["Cm", "Bb", "Ab", "G"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (136.0, 146.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Jersey club bounce",
    text: "Bouncy and chopped: triplet kicks that make everything jump, chopped synth hits, a sub bass and claps.",
    key: "C",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/jersey_club/1",
        fighters: ["TURNTABLES", "SNARE DRUM"],
        bpm: 140.0,
        key: "C",
        scale: "minor",
        intro: ["Cm", "G"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/jersey_club/2",
        fighters: ["TURNTABLES", "SNARE DRUM"],
        bpm: 140.0,
        key: "C",
        scale: "minor",
        intro: ["Cm", "Bb"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/jersey_club/3",
        fighters: ["TURNTABLES", "SNARE DRUM"],
        bpm: 140.0,
        key: "C",
        scale: "minor",
        intro: ["Cm", "G"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bounce", 1), ("bass", 2), ("chords", 2)],
    fill: &["drums", "bounce"],
    breakdown: &[("chords", 2), ("bass", 1)],
    final_exchange: &[("drums", 3), ("bounce", 3), ("bass", 3), ("chords", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bounce", 2), ("bass", 2), ("chords", 3)],
    build: &[("drums", 3), ("bounce", 3), ("bass", 3), ("chords", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bounce", 1), ("bass", 2), ("chords", 2)],
    &[("drums", 3), ("bounce", 3), ("bass", 3), ("chords", 3), ("riser", 1)],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "kit",
            |_, m, _, v| match m {
                KICK => super::brass_house::kick(v),
                SNARE => scaled(super::horror_trap::clap(seed_of(v)), 0.7 * v),
                HAT => scaled(super::horror_trap::hat(false, seed_of(v)), 0.4 * v),
                CRASH => scaled(synth::crash(1.4), 0.2 * v),
                _ => Vec::new(),
            },
            0.45,
            0.0,
            (0, 0),
        ),
        layer(
            "bounce",
            LayerKind::Perc,
            "kit",
            |_, m, _, v| match m {
                CLAP => super::brass_house::kick(0.6 * v),
                SHAKER => scaled(super::orchestral_drill::rim(v, seed_of(v)), 0.5),
                _ => Vec::new(),
            },
            0.35,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "sub",
            |_, m, d, v| {
                super::horror_trap::b808(m, d.clamp(0.15, 0.6), None, 0.0)
                    .into_iter()
                    .map(|s| s * v)
                    .collect()
            },
            0.4,
            0.0,
            (29, 41),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "chops",
            |_, m, d, v| scaled(super::brass_house::stab(&[m], d.clamp(0.06, 0.15), 3000.0), v),
            0.16,
            0.2,
            (60, 74),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("jersey_club", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("jersey_club", layers, ROOM, TABLE, cues, seed)
}
