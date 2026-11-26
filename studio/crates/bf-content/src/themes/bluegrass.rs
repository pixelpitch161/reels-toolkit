//! Theme: bluegrass breakdown, a Studio theme (the batch of 2026-11-18; no kit mood behind it). A rapid, rollicking hoedown: banjo rolls, a fiddle, a flatpicked guitar, a mandolin chop and a slapped upright bass on two and four.
//! Built for G major; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "bluegrass",
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
    &["G", "G", "C", "G", "G", "G", "D", "G"],
    &["G", "C", "D", "G"],
    &["G", "Em", "C", "D"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (138.0, 166.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Bluegrass breakdown",
    text: "A rapid, rollicking hoedown: banjo rolls, a fiddle, a flatpicked guitar, a mandolin chop and a slapped upright bass on two and four.",
    key: "G",
    scale: "major",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 44),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/bluegrass/1",
        fighters: ["ACOUSTIC", "VIOLIN"],
        bpm: 152.0,
        key: "G",
        scale: "major",
        intro: ["G", "G"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/bluegrass/2",
        fighters: ["ACOUSTIC", "VIOLIN"],
        bpm: 152.0,
        key: "G",
        scale: "major",
        intro: ["G", "G"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/bluegrass/3",
        fighters: ["ACOUSTIC", "VIOLIN"],
        bpm: 152.0,
        key: "G",
        scale: "major",
        intro: ["G", "D"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("chords", 2), ("banjo", 2), ("fiddle", 1)],
    fill: &["drums"],
    breakdown: &[("banjo", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("chords", 3), ("banjo", 3), ("fiddle", 2)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("banjo", 3), ("fiddle", 1)],
    build: &[("drums", 3), ("bass", 3), ("chords", 3), ("banjo", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("chords", 2), ("banjo", 1), ("fiddle", 1)],
    &[("drums", 3), ("bass", 3), ("chords", 3), ("banjo", 3), ("riser", 1)],
);

/// The bluegrass's own band (sound upgrade, stage 2): the boom-chick, the upright slapping the root on 1 and the
/// fifth on 3 (walking the chord at full tilt) under the mandolin's chop on 2 and 4, a brushed kit on the same
/// boom-chick with a train beat when it opens up, the banjo's Scruggs forward roll in sixteenths and the fiddle's
/// long bows turning into a shuffle.
pub static GROOVE: Groove = Groove {
    push_ms: 4.0,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(KICK), "x.......x......."),
                line(1, Play::Drum(SNARE), "....x.......x..."),
                line(2, Play::Drum(HAT), "o.x.o.x.o.x.o.x."),
                only(3, Play::Drum(SNARE), "..g...g.g.g...gg").maybe(0.8),
                only(3, Play::Drum(HAT), ".g.g.g.g.g.g.g.g"),
            ],
            fill: &[],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(2, Play::Root, "x..............."),
                upto(2, Play::Fifth, "........x......."),
                only(2, Play::Third, "..............g.").maybe(0.5),
                only(3, Play::Root, "x...............x..............."),
                only(3, Play::Third, "....x.......x..................."),
                only(3, Play::Fifth, "........x...........x.......x..."),
                only(3, Play::Seventh, "........................x......."),
            ],
            fill: &[],
        },
        Part {
            layer: "chords",
            lines: &[
                upto(1, Play::Chord, "....x.......x..."),
                line(2, Play::Chord, "....X.......X..."),
                line(2, Play::Upper, "..............g.").maybe(0.5),
                only(3, Play::Upper, "......g.......g.").maybe(0.7),
            ],
            fill: &[],
        },
        Part {
            layer: "banjo",
            lines: &[
                upto(1, Play::Tone(0), "x...x...x...x..."),
                upto(1, Play::Top, "..x...x...x...x."),
                line(2, Play::Tone(0), "X..x..x.X..x..x."),
                line(2, Play::Tone(1), ".x..x....x..x..."),
                line(2, Play::Top, "..x..x....x..x.."),
                only(3, Play::Top, ".......x.......x"),
            ],
            fill: &[],
        },
        Part {
            layer: "fiddle",
            lines: &[
                upto(1, Play::Top, "x_______x_______"),
                upto(1, Play::Tone(1), "x_______________"),
                line(2, Play::Top, "x__x__x_x__x__x_"),
                line(2, Play::Tone(1), "x__x__x_x__x__x_"),
            ],
            fill: &[],
        },
    ],
    mix: &[
        ("banjo", 6.0),
        ("bass", -0.5),
        ("fiddle", 4.0),
        ("mandolin", 3.5),
        ("perc", 8.0),
    ],
    lows: &["bass"],
    ..STRAIGHT
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "perc",
            |_, m, _, v| match m {
                KICK => scaled(super::bossa_noir::kick_felt(), 0.5 * v),
                SNARE => scaled(super::stadium_rock::stick(), 0.5 * v),
                HAT => scaled(super::bossa_noir::shaker(seed_of(v), v > 0.5), 0.5 * v),
                CRASH => scaled(synth::crash(1.0), 0.08 * v),
                _ => Vec::new(),
            },
            0.4,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| super::balkan_fire::upright(m, d.min(0.35), v),
            0.6,
            0.0,
            (40, 52),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "mandolin",
            |_, m, d, v| {
                scaled(
                    super::hot_jazz::pluck(m, d.clamp(0.05, 0.12), i64::from(m % 5), 0.9, 0.98),
                    v,
                )
            },
            0.2,
            -0.3,
            (60, 74),
        ),
        layer(
            "banjo",
            LayerKind::Arp,
            "banjo",
            |_, m, d, v| super::fresh::voices::banjo(m, d.clamp(0.1, 0.3), v),
            0.2,
            0.25,
            (62, 79),
        ),
        layer(
            "fiddle",
            LayerKind::Pad,
            "fiddle",
            |_, m, d, v| scaled(super::baroque_storm::strng(m, d.min(1.5), 2800.0, 0.006, 0.04), v),
            0.08,
            0.1,
            (62, 76),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("bluegrass", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::grooved("bluegrass", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
