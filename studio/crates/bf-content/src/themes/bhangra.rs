//! Theme: bhangra blast, a Studio theme (the batch of 2026-11-18; no kit mood behind it). A joyful, bouncing, hands-in-the-air party: a booming dhol, one-string tumbi plucks, clacking tongs, a bass and synth stabs.
//! Built for G mixolydian; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
//! layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of the Composed one, with
//! no story cues.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, SHAKER, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled, seed_of};
use super::fresh::{self, ROOM};
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "bhangra",
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
    &["G", "G", "F", "G", "G", "G", "F", "C"],
    &["G", "F", "C", "G"],
    &["G", "F", "G", "F"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (120.0, 144.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Bhangra blast",
    text: "A joyful, bouncing, hands-in-the-air party: a booming dhol, one-string tumbi plucks, clacking tongs, a bass and synth stabs.",
    key: "G",
    scale: "mixolydian",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/bhangra/1",
        fighters: ["DRUMS", "UKULELE"],
        bpm: 132.0,
        key: "G",
        scale: "mixolydian",
        intro: ["G", "C"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/bhangra/2",
        fighters: ["DRUMS", "UKULELE"],
        bpm: 132.0,
        key: "G",
        scale: "mixolydian",
        intro: ["G", "G"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/bhangra/3",
        fighters: ["DRUMS", "UKULELE"],
        bpm: 132.0,
        key: "G",
        scale: "mixolydian",
        intro: ["G", "F"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("chimta", 1), ("bass", 2), ("chords", 2), ("tumbi", 2)],
    fill: &["drums", "chimta"],
    breakdown: &[("tumbi", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("chimta", 3), ("bass", 3), ("chords", 3), ("tumbi", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("chimta", 2), ("bass", 2), ("tumbi", 3)],
    build: &[
        ("drums", 3),
        ("chimta", 3),
        ("bass", 3),
        ("chords", 3),
        ("tumbi", 3),
        ("riser", 1),
    ],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("chimta", 1), ("bass", 2), ("chords", 2), ("tumbi", 1)],
    &[
        ("drums", 3),
        ("chimta", 3),
        ("bass", 3),
        ("chords", 3),
        ("tumbi", 3),
        ("riser", 1),
    ],
);

/// The bhangra's own band (sound upgrade, stage 2): the dhol's chaal, the big head (dagga) on 1, the and of 2 and
/// 3 and the small head (tilli) bouncing the swung off-beats, the chimta clacking eighths with claps on 2 and 4 at
/// full tilt, the bass locked to the dagga, the synth stabs on the off-beats and the tumbi's one-string riff, a
/// two-bar call and answer on the root and the top.
pub static GROOVE: Groove = Groove {
    swing: 0.6,
    push_ms: 4.0,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                upto(1, Play::Drum(KICK), "X.....x.X......."),
                line(2, Play::Drum(KICK), "X.....x.X.....x."),
                only(3, Play::Drum(KICK), "...g.......g....").maybe(0.6),
                upto(1, Play::Drum(SNARE), "....x.......x..."),
                line(2, Play::Drum(SNARE), "..o.X.o...o.X.o."),
                only(3, Play::Drum(SNARE), ".......g.......g").maybe(0.7),
                only(3, Play::Drum(HAT), "x.x.x.x.x.x.x.x."),
            ],
            fill: &[],
        },
        Part {
            layer: "chimta",
            lines: &[
                upto(1, Play::Drum(SHAKER), "..x...x...x...x."),
                line(2, Play::Drum(SHAKER), "o.x.o.x.o.x.o.x."),
                only(3, Play::Drum(SHAKER), ".g.......g......").maybe(0.6),
                only(3, Play::Drum(CLAP), "....X.......X..."),
            ],
            fill: &[],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(1, Play::Root, "x_____..x_______"),
                line(2, Play::Root, "x_....x.x_......"),
                line(2, Play::Fifth, "..............x."),
                only(3, Play::Octave, "...g.......g....").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "chords",
            lines: &[
                upto(1, Play::Chord, "....x.......x..."),
                line(2, Play::Chord, "..x...x...x..x.."),
                only(3, Play::Chord, ".......x.......x").maybe(0.5),
            ],
            fill: &[],
        },
        Part {
            layer: "tumbi",
            lines: &[
                upto(1, Play::Root, "x.......x......."),
                upto(1, Play::Top, "....x.......x..."),
                line(2, Play::Root, "X..x..x.X..x..x.X..x..x.X.x.x.x."),
                line(2, Play::Top, "..x..x....x..x....x..x...x.x.x.x"),
                only(3, Play::Fifth, ".x.....x.x.....x.x.....x........"),
            ],
            fill: &[],
        },
    ],
    mix: &[
        ("bass", -2.0),
        ("dhol", -0.5),
        ("perc", 2.5),
        ("synth", 4.5),
        ("tumbi", 5.0),
    ],
    lows: &["bass", "dhol"],
    ..STRAIGHT
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "dhol",
            |_, m, _, v| match m {
                KICK => scaled(super::brass_trailer::taiko(true, seed_of(v)), 0.8 * v),
                SNARE => scaled(super::brass_trailer::taiko(false, seed_of(v)), 0.7 * v),
                HAT => scaled(super::balkan_fire::sticks(v), 0.6),
                CRASH => scaled(synth::crash(1.2), 0.15 * v),
                _ => Vec::new(),
            },
            0.45,
            0.0,
            (0, 0),
        ),
        layer(
            "chimta",
            LayerKind::Perc,
            "perc",
            |_, m, _, v| match m {
                CLAP => scaled(super::dark_disco::clap(seed_of(v)), 0.6 * v),
                SHAKER => scaled(super::big_top::tambourine(v, seed_of(v)), 0.6 * v),
                _ => Vec::new(),
            },
            0.4,
            0.2,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| {
                super::dark_disco::bass(m, d.clamp(0.12, 0.4))
                    .into_iter()
                    .map(|s| s * v)
                    .collect()
            },
            0.45,
            0.0,
            (31, 43),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "synth",
            |_, m, d, v| scaled(super::brass_house::stab(&[m], d.clamp(0.08, 0.2), 2600.0), v),
            0.16,
            -0.2,
            (55, 69),
        ),
        layer(
            "tumbi",
            LayerKind::Arp,
            "tumbi",
            |_, m, d, v| super::fresh::voices::banjo(m, d.clamp(0.08, 0.2), v),
            0.2,
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
    fresh::bed("bhangra", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::grooved("bhangra", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
