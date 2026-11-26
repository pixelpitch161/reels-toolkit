//! Theme: rockabilly rumble, a Studio theme (the batch of 2026-11-18; no kit mood behind it). Greasy echoing twang at full speed: a slapped upright bass, a train-beat snare and a honking sax.
//! Built for A mixolydian; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "rockabilly",
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
    &["A", "A", "A", "A", "D", "D", "A", "E"],
    &["A", "D", "E", "A"],
    &["A", "A", "D", "E"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (148.0, 172.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Rockabilly rumble",
    text: "Greasy echoing twang at full speed: a slapped upright bass, a train-beat snare and a honking sax.",
    key: "A",
    scale: "mixolydian",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 44),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/rockabilly/1",
        fighters: ["CONTRABASS", "ELECTRIC"],
        bpm: 162.0,
        key: "A",
        scale: "mixolydian",
        intro: ["A", "E"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/rockabilly/2",
        fighters: ["CONTRABASS", "ELECTRIC"],
        bpm: 162.0,
        key: "A",
        scale: "mixolydian",
        intro: ["A", "A"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/rockabilly/3",
        fighters: ["CONTRABASS", "ELECTRIC"],
        bpm: 162.0,
        key: "A",
        scale: "mixolydian",
        intro: ["A", "E"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("chords", 2), ("twang", 2), ("sax", 1)],
    fill: &["drums"],
    breakdown: &[("twang", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("chords", 3), ("twang", 3), ("sax", 2)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("twang", 3), ("sax", 1)],
    build: &[("drums", 3), ("bass", 3), ("chords", 3), ("twang", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("chords", 2), ("twang", 1), ("sax", 1)],
    &[("drums", 3), ("bass", 3), ("chords", 3), ("twang", 3), ("riser", 1)],
);

/// The rockabilly's own band (sound upgrade, stage 2), on a shuffle: the brushed train beat (the snare on every
/// eighth, the backbeat accented) over the kick on 1 and 3, the slapped upright on root and fifth with its slap on
/// the off-beats and a walking boogie at the top, the rhythm guitar's off-beat chops and the twang's boogie figure up
/// the chord.
pub static GROOVE: Groove = Groove {
    swing: 0.6,
    push_ms: 5.0,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(KICK), "X.......X......."),
                upto(1, Play::Drum(SNARE), "....X.......X..."),
                line(2, Play::Drum(SNARE), "o.o.X.o.o.o.X.o."),
                line(2, Play::Drum(HAT), "x...x...x...x..."),
                only(3, Play::Drum(SNARE), ".g.g.g.g.g.g.g.g").maybe(0.7),
                only(3, Play::Drum(KICK), "......x.......x.").maybe(0.5),
            ],
            fill: &[
                line(1, Play::Drum(SNARE), "x.x.xxxX"),
                line(1, Play::Drum(KICK), "X......X"),
            ],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(2, Play::Root, "X.......X......."),
                upto(2, Play::Fifth, "....x.......x..."),
                line(2, Play::Root, "..g...g...g...g.").maybe(0.7),
                only(3, Play::Root, "X.......................x......."),
                only(3, Play::Third, "....x...............x..........."),
                only(3, Play::Fifth, "........x.......x...........x..."),
                only(3, Play::Seventh, "............x..................."),
            ],
            fill: &[],
        },
        Part {
            layer: "chords",
            lines: &[
                upto(1, Play::Chord, "....x.......x..."),
                line(2, Play::Chord, "..x...x...x...x."),
                only(3, Play::Chord, "...........g...g").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "twang",
            lines: &[
                line(1, Play::Root, "x.......x......."),
                line(1, Play::Fifth, "....x.......x..."),
                line(2, Play::Third, "..x.......x....."),
                line(2, Play::Seventh, "......x.......x."),
                only(3, Play::Top, "...x.......x....").maybe(0.6),
            ],
            fill: &[],
        },
    ],
    mix: &[("bass", 6.0), ("twang", 3.0), ("sax", 2.0), ("kit", -1.0)],
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
                KICK => scaled(super::stadium_rock::kick(), 0.8 * v),
                SNARE => scaled(super::heist::brush(seed_of(v)), v),
                HAT => scaled(super::heist::brush(seed_of(v) + 1), 0.4 * v),
                CRASH => scaled(synth::crash(1.2), 0.15 * v),
                _ => Vec::new(),
            },
            0.45,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| super::balkan_fire::upright(m, d.min(0.3), v),
            0.6,
            0.0,
            (33, 45),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "rhythm",
            |_, m, d, v| {
                scaled(
                    super::hot_jazz::pluck(m, d.clamp(0.06, 0.15), i64::from(m % 4), 0.8, 0.985),
                    v,
                )
            },
            0.2,
            -0.3,
            (52, 67),
        ),
        layer(
            "twang",
            LayerKind::Arp,
            "twang",
            |_, m, d, v| super::spaghetti_western::twang(m, d.clamp(0.12, 0.4), v),
            0.16,
            0.3,
            (59, 76),
        ),
        layer(
            "sax",
            LayerKind::Pad,
            "sax",
            |_, m, d, v| scaled(super::dark_tango::reed(m, d.min(1.2), 0.02, 0.7), v),
            0.06,
            0.1,
            (55, 67),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("rockabilly", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::grooved("rockabilly", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
