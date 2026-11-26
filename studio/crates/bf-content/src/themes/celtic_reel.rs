//! Theme: celtic reel, a Studio theme (the batch of 2026-11-16; no kit mood behind it): a bodhrán, foot stomps, a bouzouki chopping the chord, a hammered dulcimer running its tones and a droning reed.
//! Built for D mixolydian; any chord works (the layers voice the chord's tones in their ranges). Both
//! arrangements are layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of
//! the Composed one, with no story cues.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
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

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/celtic_reel/1",
        fighters: ["FLUTE", "BAGPIPES"],
        bpm: 124.0,
        key: "D",
        scale: "mixolydian",
        intro: ["D", "D"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/celtic_reel/2",
        fighters: ["FLUTE", "BAGPIPES"],
        bpm: 124.0,
        key: "D",
        scale: "mixolydian",
        intro: ["D", "D"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/celtic_reel/3",
        fighters: ["FLUTE", "BAGPIPES"],
        bpm: 124.0,
        key: "D",
        scale: "mixolydian",
        intro: ["D", "A"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

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

/// The celtic reel's own band (sound upgrade, stage 2): driving eighths, the bodhrán's open bass on 1 and 3 under
/// a running tipper with a triplet roll at the end of the phrase, feet stomping the beat, the bass on root and
/// fifth, the bouzouki chopping eighths with the lift on 1 and 3, and the dulcimer running the chord up and down
/// in eighths, a two-bar figure, with grace notes when the session takes off.
pub static GROOVE: Groove = Groove {
    push_ms: 5.0,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(KICK), "X.......X......."),
                upto(1, Play::Drum(SNARE), "..x...x...x...x."),
                line(2, Play::Drum(SNARE), "o.x.X.x.o.x.X.x.o.x.X.x.o.x.X.x."),
                line(2, Play::Drum(HAT), "...g.......g...........g...ggg.."),
                only(3, Play::Drum(HAT), ".g...g.g.g...g.g.g...g...g......"),
                only(3, Play::Drum(KICK), "......g...........g.......g.....").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "stomp",
            lines: &[
                line(1, Play::Drum(CLAP), "X.......X......."),
                line(2, Play::Drum(CLAP), "....x.......x..."),
                only(3, Play::Drum(CLAP), "..............o.").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(2, Play::Root, "x.......x......."),
                only(2, Play::Fifth, "....x.......x..."),
                only(3, Play::Root, "x.x.....x.x....."),
                only(3, Play::Fifth, "....x.x.....x.x."),
            ],
            fill: &[],
        },
        Part {
            layer: "chords",
            lines: &[
                upto(1, Play::Chord, "x...x...x...x..."),
                line(2, Play::Chord, "X...x...X...x..."),
                line(2, Play::Upper, "..o...o...o...o."),
                only(3, Play::Upper, "...g.......g...g").maybe(0.7),
            ],
            fill: &[],
        },
        Part {
            layer: "dulcimer",
            lines: &[
                upto(1, Play::Tone(0), "x.......x......."),
                upto(1, Play::Tone(2), "....x.......x..."),
                upto(1, Play::Top, "..............x."),
                line(2, Play::Tone(0), "X...............X..............."),
                line(2, Play::Tone(1), "..x...........x.....x.........x."),
                line(2, Play::Tone(2), "....x.......x.....x.....x......."),
                line(2, Play::Tone(3), "......X...x...........X.....x..."),
                line(2, Play::Tone(4), "........x.................x....."),
                only(3, Play::Top, ".g.......g.......g.......g......").maybe(0.8),
            ],
            fill: &[],
        },
    ],
    mix: &[
        ("bass", -2.0),
        ("bodhran", 0.0),
        ("bouzouki", 3.0),
        ("drone", 1.5),
        ("dulcimer", 1.0),
    ],
    lows: &["bass", "bodhran"],
    ..STRAIGHT
};

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
    fresh::grooved("celtic_reel", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
