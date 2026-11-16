//! Theme: celtic reel, a Studio theme (the batch of 2026-11-16; no kit mood behind it): a bodhrán, foot stomps, a bouzouki chopping the chord, a hammered dulcimer running its tones and a droning reed.
//! Built for D mixolydian; any chord works (the layers voice the chord's tones in their ranges). Both
//! arrangements are layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of
//! the Composed one, with no story cues.

use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled, seed_of};
use super::balkan_fire::{MUSETTE, cimbalom, reed, upright};
use super::fresh::{self, ROOM};
use super::hot_jazz::pluck;
use super::sea_shanty::{frame_drum, stomp};
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "celtic_reel",
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
    &["D", "D", "C", "D", "D", "G", "C", "D"],
    &["D", "C", "G", "D"],
    &["D", "G", "D", "A"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (112.0, 132.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Celtic reel",
    text: "A pub session at full tilt: a skipping reel over a bodhrán, a bouzouki chop and a droning reed, light on its feet.",
    key: "D",
    scale: "mixolydian",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for (no kit config: a Studio theme).
pub static PRESETS: [Preset; 1] = [Preset {
    config: "studio/celtic_reel",
    fighters: ["FLUTE", "BAGPIPES"],
    bpm: 124.0,
    key: "D",
    scale: "mixolydian",
    intro: ["D", "D"],
    chord_loop: LOOPS[0],
    tail: &[],
}];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[
        ("drums", 2),
        ("stomp", 1),
        ("bass", 2),
        ("chords", 2),
        ("dulcimer", 2),
        ("drone", 1),
    ],
    fill: &["drums", "stomp"],
    breakdown: &[("stomp", 3), ("drone", 2), ("dulcimer", 1)],
    final_exchange: &[
        ("drums", 3),
        ("stomp", 3),
        ("bass", 3),
        ("chords", 3),
        ("dulcimer", 3),
        ("drone", 2),
    ],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("chords", 2), ("dulcimer", 3)],
    build: &[
        ("drums", 3),
        ("stomp", 3),
        ("bass", 3),
        ("chords", 3),
        ("dulcimer", 3),
        ("riser", 1),
    ],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("chords", 2), ("drone", 1)],
    &[
        ("drums", 3),
        ("stomp", 3),
        ("bass", 3),
        ("chords", 3),
        ("dulcimer", 3),
        ("riser", 1),
    ],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "bodhran",
            |_, m, _, v| match m {
                KICK => scaled(frame_drum(true), v),
                SNARE => scaled(frame_drum(false), 0.8 * v),
                HAT => scaled(frame_drum(false), 0.3 * v),
                CRASH => scaled(synth::crash(1.0), 0.12 * v),
                _ => Vec::new(),
            },
            0.5,
            0.0,
            (0, 0),
        ),
        layer(
            "stomp",
            LayerKind::Perc,
            "bodhran",
            |_, m, _, v| match m {
                CLAP => scaled(stomp(v, seed_of(v)), 1.0),
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
            |_, m, d, v| upright(m, d.min(0.45), v),
            0.55,
            0.0,
            (38, 50),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "bouzouki",
            |_, m, d, v| scaled(pluck(m, d.clamp(0.1, 0.3), i64::from(m % 4), 0.7, 0.993), v),
            0.2,
            -0.25,
            (50, 66),
        ),
        layer(
            "dulcimer",
            LayerKind::Arp,
            "dulcimer",
            |_, m, _, v| cimbalom(m, v),
            0.18,
            0.3,
            (69, 83),
        ),
        layer(
            "drone",
            LayerKind::Pad,
            "drone",
            |_, m, d, v| scaled(reed(&[m], d, 0.3, 0.3, 1200.0, MUSETTE), v),
            0.04,
            0.0,
            (50, 62),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("celtic_reel", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("celtic_reel", layers, ROOM, TABLE, cues, seed)
}
