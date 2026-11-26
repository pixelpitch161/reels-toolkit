//! Theme: heroic fanfare, a Studio theme (the batch of 2026-11-16; no kit mood behind it): a string ostinato, timpani on the root, a marching snare and bass drum, low strings, a horn pad and bright brass calls.
//! Built for Bb major; any chord works (the layers voice the chord's tones in their ranges). Both
//! arrangements are layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of
//! the Composed one, with no story cues.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{ArrangementTable, CRASH, Cue, HAT, KICK, Layer, LayerKind, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled};
use super::baroque_storm::{strng, timp};
use super::brass_trailer::strings_low;
use super::fresh::{self, ROOM};
use super::villain_march::{bass_drum_boom, brass, horn_pad, march_snare_stroke};
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "heroic_fanfare",
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
    &["Bb", "Bb", "F", "Gm", "Eb", "Bb", "F", "F"],
    &["Bb", "Eb", "F", "Bb"],
    &["Bb", "Gm", "Eb", "F"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (100.0, 128.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Heroic fanfare",
    text: "The bright side of the cinema: a rising-hero main title with racing strings, timpani, a marching snare and big horn calls.",
    key: "Bb",
    scale: "major",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/heroic_fanfare/1",
        fighters: ["TRUMPET", "HORN"],
        bpm: 116.0,
        key: "Bb",
        scale: "major",
        intro: ["Bb", "F"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/heroic_fanfare/2",
        fighters: ["TRUMPET", "HORN"],
        bpm: 116.0,
        key: "Bb",
        scale: "major",
        intro: ["Bb", "Bb"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/heroic_fanfare/3",
        fighters: ["TRUMPET", "HORN"],
        bpm: 116.0,
        key: "Bb",
        scale: "major",
        intro: ["Bb", "F"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[
        ("drums", 2),
        ("timpani", 1),
        ("bass", 2),
        ("strings", 3),
        ("horns", 1),
        ("chords", 1),
    ],
    fill: &["drums", "timpani"],
    breakdown: &[("strings", 2), ("horns", 2), ("bass", 1)],
    final_exchange: &[
        ("drums", 3),
        ("timpani", 2),
        ("bass", 3),
        ("strings", 3),
        ("horns", 3),
        ("chords", 3),
    ],
    ability: &["chords", "timpani"],
    victory: &[("drums", 1), ("bass", 2), ("strings", 2), ("horns", 3), ("chords", 1)],
    build: &[
        ("drums", 3),
        ("timpani", 3),
        ("bass", 3),
        ("strings", 3),
        ("horns", 3),
        ("chords", 2),
        ("riser", 1),
    ],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("strings", 2), ("horns", 1), ("chords", 1)],
    &[
        ("drums", 3),
        ("timpani", 3),
        ("bass", 3),
        ("strings", 3),
        ("horns", 3),
        ("chords", 2),
        ("riser", 1),
    ],
);

/// The heroic fanfare's own band (sound upgrade, stage 2): a march on the snare (soft taps on the eighths, the
/// backbeat and its drags) over the bass drum on 1 and 3, the timpani on the root and the fifth with a roll into
/// a lead change, the low strings marching in eighths, the strings racing in sixteenths up a two-bar figure, and
/// the brass calling on the three-three-two.
pub static GROOVE: Groove = Groove {
    push_ms: 4.0,
    vel_spread: 0.08,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(KICK), "X.......x......."),
                upto(1, Play::Drum(SNARE), "....X.......X..."),
                line(2, Play::Drum(SNARE), "....X..g....X.gg"),
                line(2, Play::Drum(HAT), "x.o...o.x.o....."),
                only(3, Play::Drum(SNARE), "...g.......g....").maybe(0.7),
                only(3, Play::Drum(KICK), "..........x....."),
            ],
            fill: &[
                line(1, Play::Drum(KICK), "X......."),
                line(1, Play::Drum(SNARE), "gxgxxxxX"),
            ],
        },
        Part {
            layer: "timpani",
            lines: &[
                upto(1, Play::Root, "X_______........"),
                line(2, Play::Root, "X..........x...."),
                line(2, Play::Fifth, "........X......."),
                only(3, Play::Root, "....x.....x.x.xx"),
            ],
            fill: &[line(1, Play::Fifth, "x.x.x..."), line(1, Play::Root, ".x.x.xxX")],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(1, Play::Root, "X_______x_______"),
                line(2, Play::Root, "X.x.x...X.x.x..."),
                only(2, Play::Root, "......x.......x."),
                only(3, Play::Octave, "......x.......x."),
                only(3, Play::Fifth, ".......g.......g").maybe(0.5),
            ],
            fill: &[],
        },
        Part {
            layer: "strings",
            lines: &[
                only(2, Play::Tone(0), "x.......x......................."),
                only(2, Play::Tone(1), "..o...o...o...o.x.......x......."),
                only(2, Play::Tone(2), "....x.......x.....o...o...o...o."),
                only(2, Play::Tone(3), "....................x.......x..."),
                only(3, Play::Tone(0), "X...x...x...x..................."),
                only(3, Play::Tone(1), ".o.o.o.o.o.o.o.oX...x...x...x..."),
                only(3, Play::Tone(2), "..x...x...x...x..o.o.o.o.o.o.o.o"),
                only(3, Play::Tone(3), "..................x...x...x...x."),
            ],
            fill: &[],
        },
        Part {
            layer: "chords",
            lines: &[
                upto(1, Play::Chord, "X_______........"),
                line(2, Play::Chord, "X..x..x.X......."),
                only(3, Play::Chord, "...........x.x.x").maybe(0.7),
            ],
            fill: &[],
        },
    ],
    mix: &[("brass", 1.0), ("strings", 2.0), ("timp", -1.0)],
    lows: &["timp"],
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
                KICK => scaled(bass_drum_boom(), 0.7 * v),
                SNARE => scaled(march_snare_stroke(), v),
                HAT => scaled(march_snare_stroke(), 0.2 * v),
                CRASH => scaled(synth::crash(2.0), 0.3 * v),
                _ => Vec::new(),
            },
            0.36,
            0.0,
            (0, 0),
        ),
        layer(
            "timpani",
            LayerKind::Bass,
            "timp",
            |_, m, _, v| scaled(timp(m), v),
            0.3,
            0.0,
            (41, 53),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "low",
            |_, m, d, v| strings_low(m, d.min(1.5), v),
            0.4,
            0.0,
            (34, 46),
        ),
        layer(
            "strings",
            LayerKind::Arp,
            "strings",
            |_, m, d, v| scaled(strng(m, d.min(0.2), 2600.0, 0.004, 0.02), v),
            0.18,
            0.25,
            (62, 77),
        ),
        layer(
            "horns",
            LayerKind::Pad,
            "horns",
            |_, m, d, v| scaled(horn_pad(&[m], d.min(3.0)), v),
            0.06,
            -0.15,
            (53, 65),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "brass",
            |_, m, d, v| scaled(brass(&[m], d.clamp(0.15, 0.4), 0.015, 2200.0, 0.8), v),
            0.12,
            0.1,
            (58, 72),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("heroic_fanfare", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::grooved("heroic_fanfare", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
