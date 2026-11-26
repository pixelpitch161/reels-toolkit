//! Theme: pep band punk, a Studio theme (the batch of 2026-11-16; no kit mood behind it): buzzsaw power chords, a driven bass, unison horn stabs on the off-beats, claps and a tambourine over a fast kit.
//! Built for A major; any chord works (the layers voice the chord's tones in their ranges). Both
//! arrangements are layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of
//! the Composed one, with no story cues.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, SHAKER, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled, seed_of};
use super::fresh::{self, ROOM};
use super::prog_duel::{bass_note, hat, kick, snare};
use super::second_line::{clap, tambourine};
use super::stadium_rock::pgtr;
use super::villain_march::brass;
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "pep_band_punk",
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
    &["A", "A", "D", "E", "A", "A", "D", "E"],
    &["A", "F#m", "D", "E"],
    &["A", "D", "F#m", "E"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (150.0, 168.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Pep band punk",
    text: "A stadium pep band turned punk band: buzzsaw power chords, shout-along horn hits and a snare that never lets up.",
    key: "A",
    scale: "major",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 43),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/pep_band_punk/1",
        fighters: ["MELLOPHONE", "SNARE DRUM"],
        bpm: 160.0,
        key: "A",
        scale: "major",
        intro: ["A", "E"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/pep_band_punk/2",
        fighters: ["MELLOPHONE", "SNARE DRUM"],
        bpm: 160.0,
        key: "A",
        scale: "major",
        intro: ["A", "E"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/pep_band_punk/3",
        fighters: ["MELLOPHONE", "SNARE DRUM"],
        bpm: 160.0,
        key: "A",
        scale: "major",
        intro: ["A", "E"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 3), ("cheer", 1), ("bass", 3), ("guitar", 3), ("chords", 1)],
    fill: &["drums", "cheer"],
    breakdown: &[("drums", 1), ("cheer", 3), ("bass", 1)],
    final_exchange: &[("drums", 3), ("cheer", 3), ("bass", 3), ("guitar", 3), ("chords", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("cheer", 2), ("bass", 2), ("chords", 2)],
    build: &[
        ("drums", 3),
        ("cheer", 3),
        ("bass", 3),
        ("guitar", 3),
        ("chords", 2),
        ("riser", 1),
    ],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 3), ("bass", 3), ("guitar", 3), ("chords", 1)],
    &[
        ("drums", 3),
        ("cheer", 3),
        ("bass", 3),
        ("guitar", 3),
        ("chords", 2),
        ("riser", 1),
    ],
);

/// The pep band punk's own band (sound upgrade, stage 2): a skate beat (the kick on the beat, the snare on every
/// off-beat eighth, never letting up) with eighth hats, the cheer's claps on the backbeat and a two-bar "let's go"
/// clap over the tambourine, the bass driving eighths on the root, buzzsaw power chords on downstroke eighths and
/// the horns' unison stabs on the off-beats, shouting a two-bar hit at the top.
pub static GROOVE: Groove = Groove {
    push_ms: 4.0,
    vel_spread: 0.09,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                upto(1, Play::Drum(KICK), "X.......X......."),
                upto(1, Play::Drum(SNARE), "....X.......X..."),
                only(2, Play::Drum(KICK), "X...x...X...x..."),
                only(2, Play::Drum(SNARE), "..X...X...X...X."),
                line(2, Play::Drum(HAT), "x.x.x.x.x.x.x.x."),
                only(3, Play::Drum(KICK), "X...X...X...X..."),
                only(3, Play::Drum(SNARE), "..X..gX...X..gX."),
                only(3, Play::Drum(SNARE), "...g.......g....").maybe(0.6),
            ],
            fill: &[
                line(1, Play::Drum(SNARE), "gxgxxxXX"),
                line(1, Play::Drum(KICK), "X...X..."),
            ],
        },
        Part {
            layer: "cheer",
            lines: &[
                upto(1, Play::Drum(SHAKER), "..x...x...x...x."),
                upto(2, Play::Drum(CLAP), "....X.......X..."),
                line(2, Play::Drum(SHAKER), "x.x.x.x.x.x.x.x."),
                only(3, Play::Drum(CLAP), "....X.......X...X...X...X.X.X..."),
                only(3, Play::Drum(SHAKER), ".g.g.g.g.g.g.g.g"),
            ],
            fill: &[line(1, Play::Drum(CLAP), "x.x.xxxX")],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(1, Play::Root, "X_______X_______"),
                only(2, Play::Root, "X...x...X...x.x."),
                line(3, Play::Root, "X.x.x.x.X.x.x..."),
                line(3, Play::Octave, "..............x."),
            ],
            fill: &[],
        },
        Part {
            layer: "guitar",
            lines: &[
                upto(1, Play::Root, "X_______X_______"),
                upto(1, Play::Fifth, "X_______X_______"),
                only(2, Play::Root, "X...x.x.X...x.x."),
                only(2, Play::Fifth, "X...x.x.X...x.x."),
                line(3, Play::Root, "X.x.x.x.X.x.x.x."),
                line(3, Play::Fifth, "X.x.x.x.X.x.x.x."),
            ],
            fill: &[],
        },
        Part {
            layer: "chords",
            lines: &[
                upto(1, Play::Chord, "..x.......x....."),
                only(2, Play::Chord, "..x...x...x...x."),
                only(3, Play::Chord, "..X...X...X...X.X..X..X...X.X.X."),
            ],
            fill: &[],
        },
    ],
    mix: &[("bass", -2.0), ("guitar", 6.0), ("horns", 4.0), ("perc", 2.0)],
    lows: &["bass"],
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
                KICK => scaled(kick(), v),
                SNARE => scaled(snare(seed_of(v)), 0.8 * v),
                HAT => scaled(hat(false), 0.5 * v),
                CRASH => scaled(synth::crash(1.4), 0.3 * v),
                _ => Vec::new(),
            },
            0.45,
            0.0,
            (0, 0),
        ),
        layer(
            "cheer",
            LayerKind::Perc,
            "perc",
            |_, m, _, v| match m {
                CLAP => scaled(clap(seed_of(v) as i64), v),
                SHAKER => scaled(tambourine(seed_of(v) as i64), 0.6 * v),
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
            |_, m, d, v| bass_note(m, d.clamp(0.1, 0.4), v, 0.6),
            0.25,
            0.0,
            (33, 45),
        ),
        layer(
            "guitar",
            LayerKind::Bass,
            "guitar",
            |_, m, d, v| scaled(pgtr(m, d.clamp(0.1, 0.4), false, true), v),
            0.18,
            -0.3,
            (40, 52),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "horns",
            |_, m, d, v| scaled(brass(&[m], d.clamp(0.12, 0.3), 0.01, 2400.0, 1.0), v),
            0.14,
            0.2,
            (57, 72),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("pep_band_punk", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::grooved("pep_band_punk", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
