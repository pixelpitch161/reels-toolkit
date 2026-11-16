//! Theme: liquid drum & bass, a Studio theme (the batch of 2026-11-16; no kit mood behind it): a rolling breakbeat, a round reese sub bass, an airy chord pad, soft bell arpeggios and a shaker.
//! Built for F minor; any chord works (the layers voice the chord's tones in their ranges). Both
//! arrangements are layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of
//! the Composed one, with no story cues.

use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, SHAKER, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled, seed_of};
use super::brass_house::{hat, shaker};
use super::dark_breakbeat::{cold_pad, kick, reese_plain, snare_hit};
use super::fresh::{self, ROOM};
use super::horror_trap::tine;
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "liquid_dnb",
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
    &["Fm", "Fm", "Db", "Db", "Ab", "Ab", "Eb", "Eb"],
    &["Fm", "Db", "Ab", "Eb"],
    &["Fm", "Ab", "Db", "C"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (160.0, 168.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Liquid drum & bass",
    text: "Fast but smooth: rolling breakbeats under airy pads, a round sub bass and soft bells, with room for a breathy lead to float.",
    key: "F",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (40, 43),
};

/// The pair the theme was made for (no kit config: a Studio theme).
pub static PRESETS: [Preset; 1] = [Preset {
    config: "studio/liquid_dnb",
    fighters: ["FLUTE", "ALTO SAX"],
    bpm: 164.0,
    key: "F",
    scale: "minor",
    intro: ["Fm", "Eb"],
    chord_loop: LOOPS[0],
    tail: &[],
}];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 3), ("shaker", 1), ("bass", 2), ("chords", 2), ("bells", 1)],
    fill: &["drums", "shaker"],
    breakdown: &[("chords", 3), ("bells", 2)],
    final_exchange: &[("drums", 3), ("shaker", 2), ("bass", 3), ("chords", 3), ("bells", 3)],
    ability: &["bells", "drums"],
    victory: &[("drums", 2), ("shaker", 2), ("bass", 2), ("chords", 2), ("bells", 2)],
    build: &[
        ("drums", 3),
        ("shaker", 3),
        ("bass", 3),
        ("chords", 3),
        ("bells", 3),
        ("riser", 1),
    ],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 3), ("bass", 2), ("chords", 2)],
    &[
        ("drums", 3),
        ("shaker", 3),
        ("bass", 3),
        ("chords", 3),
        ("bells", 3),
        ("riser", 1),
    ],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "breaks",
            |_, m, _, v| match m {
                KICK => kick(v),
                SNARE => scaled(snare_hit(), 0.8 * v),
                HAT => hat(0.5 * v, false, seed_of(v)),
                CRASH => scaled(synth::crash(1.6), 0.2 * v),
                _ => Vec::new(),
            },
            0.42,
            0.0,
            (0, 0),
        ),
        layer(
            "shaker",
            LayerKind::Perc,
            "perc",
            |_, m, _, v| match m {
                SHAKER => shaker(v, seed_of(v)),
                CLAP => scaled(snare_hit(), 0.3 * v),
                _ => Vec::new(),
            },
            0.4,
            0.25,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "sub",
            |_, m, d, v| reese_plain(m, d.clamp(0.15, 1.2), v),
            0.45,
            0.0,
            (29, 41),
        ),
        layer(
            "chords",
            LayerKind::Pad,
            "pad",
            |_, m, d, v| scaled(cold_pad(&[m], d, 2400.0), v),
            0.15,
            0.0,
            (53, 68),
        ),
        layer(
            "bells",
            LayerKind::Arp,
            "bells",
            |_, m, d, v| scaled(tine(m, d.clamp(0.2, 0.8), 0.003), v),
            0.07,
            0.3,
            (72, 86),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("liquid_dnb", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("liquid_dnb", layers, ROOM, TABLE, cues, seed)
}
