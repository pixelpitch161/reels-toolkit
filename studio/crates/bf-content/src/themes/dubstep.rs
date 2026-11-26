//! Theme: dubstep wobble, a Studio theme (the batch of 2026-11-18; no kit mood behind it). Heavy half-time drops: a growling wobble bass, a sub, a slamming kit and siren risers.
//! Built for F minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
//! layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of the Composed one, with
//! no story cues.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
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
    name: "dubstep",
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
    &["Fm", "Fm", "Db", "Db", "Ab", "Ab", "Eb", "C"],
    &["Fm", "Db", "Ab", "Eb"],
    &["Fm", "Fm", "Db", "C"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (138.0, 150.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Dubstep wobble",
    text: "Heavy half-time drops: a growling wobble bass, a sub, a slamming kit and siren risers.",
    key: "F",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/dubstep/1",
        fighters: ["SYNTH", "TUBA"],
        bpm: 140.0,
        key: "F",
        scale: "minor",
        intro: ["Fm", "C"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/dubstep/2",
        fighters: ["SYNTH", "TUBA"],
        bpm: 140.0,
        key: "F",
        scale: "minor",
        intro: ["Fm", "Eb"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/dubstep/3",
        fighters: ["SYNTH", "TUBA"],
        bpm: 140.0,
        key: "F",
        scale: "minor",
        intro: ["Fm", "C"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("chords", 1), ("bells", 2)],
    fill: &["drums"],
    breakdown: &[("bells", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("chords", 2), ("bells", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("bells", 3)],
    build: &[("drums", 3), ("bass", 3), ("bells", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("chords", 2), ("bells", 1)],
    &[("drums", 3), ("bass", 3), ("bells", 3), ("riser", 1)],
);

/// The dubstep's own band (sound upgrade, stage 2): the half-time kit (the kick on 1, the snare on 3, a second
/// kick moving over two bars, hats in eighths and then sixteenths), the wobble bass chopping a two-bar figure with
/// octave stabs at the top, and the bells in dotted eighths over it.
pub static GROOVE: Groove = Groove {
    push_ms: 2.0,
    vel_spread: 0.08,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(KICK), "X..............."),
                line(1, Play::Drum(SNARE), "........X......."),
                line(2, Play::Drum(HAT), "x.o.x.o.x.o.x.o."),
                line(2, Play::Drum(KICK), "..........x.......x.......x....."),
                only(3, Play::Drum(HAT), ".g.g.g.g.g.g.g.g"),
                only(3, Play::Drum(SNARE), "...............g").maybe(0.5),
            ],
            fill: &[
                line(1, Play::Drum(KICK), "X......."),
                line(1, Play::Drum(SNARE), "x.x.xxXX"),
            ],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(1, Play::Root, "X_______........"),
                line(2, Play::Root, "X_____....x__...X__.x__...x_x_.."),
                only(3, Play::Octave, "......x_.....x........x_......xx"),
            ],
            fill: &[],
        },
        Part {
            layer: "bells",
            lines: &[
                line(1, Play::Tone(0), "x.....x.....x..."),
                line(2, Play::Tone(2), "...x.....x.....x"),
                only(3, Play::Top, "........x...............x...x..."),
            ],
            fill: &[],
        },
    ],
    lows: &["wobble"],
    ..STRAIGHT
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "kit",
            |_, m, _, v| match m {
                KICK => scaled(super::horror_trap::kick(), v),
                SNARE => scaled(super::dark_breakbeat::snare_hit(), 0.9 * v),
                HAT => scaled(super::horror_trap::hat(false, seed_of(v)), 0.4 * v),
                CRASH => scaled(synth::crash(1.8), 0.2 * v),
                _ => Vec::new(),
            },
            0.45,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "wobble",
            |_, m, d, v| super::fresh::voices::wobble(m, d.clamp(0.15, 0.8), 6.0, v),
            0.35,
            0.0,
            (29, 41),
        ),
        layer(
            "chords",
            LayerKind::Pad,
            "pad",
            |_, m, d, v| scaled(super::dark_breakbeat::cold_pad(&[m], d, 1800.0), v),
            0.12,
            0.0,
            (53, 68),
        ),
        layer(
            "bells",
            LayerKind::Arp,
            "bells",
            |_, m, d, v| scaled(super::horror_trap::tine(m, d.clamp(0.2, 0.6), 0.004), v),
            0.06,
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
    fresh::bed("dubstep", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::grooved("dubstep", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
