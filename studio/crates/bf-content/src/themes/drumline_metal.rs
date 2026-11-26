//! Theme: drumline metal, a Studio theme (the batch of 2026-11-16; no kit mood behind it): a marching snare, quad toms and a pitched bass drum, palm-muted drop-D chugs, a driven bass and a low brass pad.
//! Built for D minor; any chord works (the layers voice the chord's tones in their ranges). Both
//! arrangements are layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of
//! the Composed one, with no story cues.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{ArrangementTable, CRASH, Cue, HAT, KICK, Layer, LayerKind, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled};
use super::dark_drumline::{bassdrum, msnare, tenor};
use super::fresh::{self, ROOM};
use super::prog_duel::bass_note;
use super::stadium_rock::pgtr;
use super::villain_march::horn_pad;
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "drumline_metal",
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
    &["Dm", "Dm", "Bb", "A", "Dm", "Dm", "Bb", "C"],
    &["Dm", "Bb", "C", "A"],
    &["Dm", "Gm", "Bb", "A"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (132.0, 160.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Drumline metal",
    text: "Heavy and military: a marching drumline cadence locked to drop-D guitar chugs, double kicks and low brass.",
    key: "D",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/drumline_metal/1",
        fighters: ["TIMPANI", "BASS DRUM"],
        bpm: 144.0,
        key: "D",
        scale: "minor",
        intro: ["Dm", "C"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/drumline_metal/2",
        fighters: ["TIMPANI", "BASS DRUM"],
        bpm: 144.0,
        key: "D",
        scale: "minor",
        intro: ["Dm", "A"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/drumline_metal/3",
        fighters: ["TIMPANI", "BASS DRUM"],
        bpm: 144.0,
        key: "D",
        scale: "minor",
        intro: ["Dm", "A"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 3), ("quads", 1), ("bass", 3), ("chug", 3), ("chords", 1)],
    fill: &["drums", "quads"],
    breakdown: &[("drums", 1), ("quads", 3), ("chords", 1)],
    final_exchange: &[("drums", 3), ("quads", 3), ("bass", 3), ("chug", 3), ("chords", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("quads", 2), ("bass", 2), ("chords", 2)],
    build: &[
        ("drums", 3),
        ("quads", 3),
        ("bass", 3),
        ("chug", 3),
        ("chords", 2),
        ("riser", 1),
    ],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 3), ("bass", 3), ("chug", 3), ("chords", 1)],
    &[
        ("drums", 3),
        ("quads", 3),
        ("bass", 3),
        ("chug", 3),
        ("chords", 2),
        ("riser", 1),
    ],
);

/// The drumline metal's own band (sound upgrade, stage 2): a marching snare cadence (accents on the backbeat,
/// taps and grace notes between) over the pitched bass drum, double kicks at full tilt, the quads sweeping round
/// the drums, the drop-D chug galloping on the root with the fifth on its accents, and the bass locked to it in
/// short notes.
pub static GROOVE: Groove = Groove {
    push_ms: 3.0,
    vel_spread: 0.08,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                upto(1, Play::Drum(KICK), "X.......X......."),
                upto(1, Play::Drum(SNARE), "....X..g....X.g."),
                only(2, Play::Drum(KICK), "X.....x.X.....x."),
                line(2, Play::Drum(SNARE), "o.g.X.g.o.goX.go"),
                line(2, Play::Drum(HAT), "...g.......g...."),
                only(3, Play::Drum(KICK), "XxxxXxxxXxxxXxxx"),
                only(3, Play::Drum(SNARE), ".g...g...g...g..").maybe(0.6),
            ],
            fill: &[
                line(1, Play::Drum(SNARE), "xgxgxxxX"),
                line(1, Play::Drum(KICK), "X...X..X"),
            ],
        },
        Part {
            layer: "quads",
            lines: &[
                line(1, Play::Tone(0), "x.......x......."),
                line(1, Play::Tone(2), "....x.x.....x.x."),
                line(2, Play::Tone(1), "..x.......x....."),
                line(2, Play::Tone(3), "...x.......x...x"),
                only(3, Play::Tone(1), ".x...x...x...x..").maybe(0.7),
                only(3, Play::Tone(3), "...............x...x.x.x.x.xxx.x"),
            ],
            fill: &[
                line(1, Play::Tone(3), "xx......"),
                line(1, Play::Tone(2), "..xx...."),
                line(1, Play::Tone(1), "....xx.."),
                line(1, Play::Tone(0), "......xX"),
            ],
        },
        Part {
            layer: "chug",
            lines: &[
                upto(1, Play::Root, "X.......X.x....."),
                only(2, Play::Root, "X.xxX.xxX.xxX.x."),
                line(3, Play::Root, "X.xxX.xxX.xxX.xxX.xxX.xxX.X.X.X."),
                line(2, Play::Fifth, "X.......X......."),
            ],
            fill: &[],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(1, Play::Root, "X_______X_______"),
                only(2, Play::Root, "X...x...X...x.x."),
                line(3, Play::Root, "X.xxX.x.X.xxX.x."),
                only(3, Play::Octave, "..............g.").maybe(0.5),
            ],
            fill: &[],
        },
    ],
    mix: &[("bass", -2.0), ("guitar", 4.0), ("brass", 2.0), ("line", 1.5)],
    lows: &["bass"],
    ..STRAIGHT
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "line",
            |_, m, _, v| match m {
                KICK => scaled(bassdrum(38), v),
                SNARE => scaled(msnare(if v > 0.6 { 'A' } else { 't' }), v),
                HAT => scaled(msnare('g'), 0.8 * v),
                CRASH => scaled(synth::crash(1.6), 0.3 * v),
                _ => Vec::new(),
            },
            0.42,
            0.0,
            (0, 0),
        ),
        layer(
            "quads",
            LayerKind::Arp,
            "line",
            |_, m, _, v| scaled(tenor(m.rem_euclid(4) as usize), v),
            0.2,
            0.15,
            (50, 62),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| bass_note(m, d.clamp(0.1, 0.4), v, 0.8),
            0.3,
            0.0,
            (26, 38),
        ),
        layer(
            "chug",
            LayerKind::Bass,
            "guitar",
            |_, m, d, v| scaled(pgtr(m, d.clamp(0.08, 0.3), true, false), v),
            0.6,
            -0.25,
            (38, 50),
        ),
        layer(
            "chords",
            LayerKind::Pad,
            "brass",
            |_, m, d, v| scaled(horn_pad(&[m], d.min(3.0)), v),
            0.08,
            0.2,
            (45, 60),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("drumline_metal", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::grooved("drumline_metal", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
