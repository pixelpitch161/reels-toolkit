//! Theme: kaiju stomp, a Studio theme (the batch of 2026-11-18; no kit mood behind it). A colossal, lumbering city-wrecker: huge low brass blasts, timpani, low strings, piano clusters and sliding horns.
//! Built for C minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "kaiju_stomp",
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
    &["Cm", "Cm", "Ab", "G", "Cm", "Cm", "Db", "G"],
    &["Cm", "Ab", "Db", "G"],
    &["Cm", "Gm", "Ab", "G"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (96.0, 112.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Kaiju stomp",
    text: "A colossal, lumbering city-wrecker: huge low brass blasts, timpani, low strings, piano clusters and sliding horns.",
    key: "C",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/kaiju_stomp/1",
        fighters: ["TUBA", "CONTRABASSOON"],
        bpm: 104.0,
        key: "C",
        scale: "minor",
        intro: ["Cm", "G"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/kaiju_stomp/2",
        fighters: ["TUBA", "CONTRABASSOON"],
        bpm: 104.0,
        key: "C",
        scale: "minor",
        intro: ["Cm", "G"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/kaiju_stomp/3",
        fighters: ["TUBA", "CONTRABASSOON"],
        bpm: 104.0,
        key: "C",
        scale: "minor",
        intro: ["Cm", "G"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("timpani", 2), ("bass", 2), ("chords", 2), ("horns", 1)],
    fill: &["drums"],
    breakdown: &[("timpani", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("timpani", 3), ("bass", 3), ("chords", 3), ("horns", 2)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("timpani", 3), ("bass", 2), ("horns", 1)],
    build: &[("drums", 3), ("timpani", 3), ("bass", 3), ("chords", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("timpani", 1), ("bass", 2), ("chords", 2), ("horns", 1)],
    &[("drums", 3), ("timpani", 3), ("bass", 3), ("chords", 3), ("riser", 1)],
);

/// The kaiju stomp's own band (sound upgrade, stage 2): a half-time stomp, the bass drum's footfalls on 1 and 3
/// with the punch landing on the second, ticks between them and the drums staggering at the top; the timpani on
/// the root with the fifth before the next step, the braams held a bar or a half, and low piano clusters on the
/// footfalls with the chord's upper notes trailing.
pub static GROOVE: Groove = Groove {
    push_ms: 6.0,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(KICK), "X.......X......."),
                line(2, Play::Drum(SNARE), "........X......."),
                line(2, Play::Drum(HAT), "..o...o...o...o."),
                only(3, Play::Drum(KICK), "......x.....x.x."),
                only(3, Play::Drum(HAT), "o...o...o...o..."),
            ],
            fill: &[
                line(1, Play::Drum(KICK), "X.x.X.x."),
                line(1, Play::Drum(SNARE), "...x.xxX"),
            ],
        },
        Part {
            layer: "timpani",
            lines: &[
                line(1, Play::Root, "X.......X......."),
                line(2, Play::Fifth, "............x..."),
                only(3, Play::Fifth, "......x........."),
                only(3, Play::Root, "..............x."),
            ],
            fill: &[],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(1, Play::Root, "X_______________"),
                only(2, Play::Root, "X_____..X_____.."),
                only(3, Play::Root, "X_____..X_..x_.."),
            ],
            fill: &[],
        },
        Part {
            layer: "chords",
            lines: &[
                upto(1, Play::Chord, "X_______........"),
                line(2, Play::Chord, "X.......X......."),
                line(2, Play::Upper, "..............x."),
                only(3, Play::Upper, "......x.....x..."),
            ],
            fill: &[],
        },
    ],
    mix: &[
        ("drums", 3.0),
        ("horns", -2.5),
        ("low", -2.0),
        ("piano", -3.0),
        ("timp", -2.0),
    ],
    ..STRAIGHT
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "drums",
            |_, m, _, v| match m {
                KICK => scaled(super::villain_march::bass_drum_boom(), v),
                SNARE => scaled(super::baroque_storm::punch(), 0.8 * v),
                HAT => scaled(super::brass_trailer::tick(seed_of(v)), 0.3 * v),
                CRASH => scaled(synth::crash(2.4), 0.3 * v),
                _ => Vec::new(),
            },
            0.45,
            0.0,
            (0, 0),
        ),
        layer(
            "timpani",
            LayerKind::Bass,
            "timp",
            |_, m, _, v| super::villain_march::timpani(m, v),
            0.3,
            0.0,
            (36, 48),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "low",
            |_, m, d, v| super::brass_trailer::braam(m, d.clamp(0.3, 1.5), v, 0.3),
            0.3,
            0.0,
            (28, 40),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "piano",
            |_, m, d, v| super::fresh::voices::piano(m, d.clamp(0.2, 0.6), v),
            0.2,
            -0.2,
            (36, 52),
        ),
        layer(
            "horns",
            LayerKind::Pad,
            "horns",
            |_, m, d, v| scaled(super::villain_march::horn_pad(&[m], d.min(3.0)), v),
            0.08,
            0.2,
            (48, 60),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("kaiju_stomp", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::grooved("kaiju_stomp", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
