//! Theme: cumbia, a Studio theme (the batch of 2026-11-18; no kit mood behind it). A swaying, sunny street party: an accordion lead, a scraping guacharaca, hand drums, a bass on one and three and clarinet answers.
//! Built for D minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "cumbia",
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
    &["Dm", "Dm", "A7", "A7", "Dm", "Gm", "A7", "Dm"],
    &["Dm", "C", "F", "A7"],
    &["Dm", "Gm", "A7", "Dm"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (100.0, 118.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Cumbia",
    text: "A swaying, sunny street party: an accordion lead, a scraping guacharaca, hand drums, a bass on one and three and clarinet answers.",
    key: "D",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/cumbia/1",
        fighters: ["ACCORDION", "CLARINET"],
        bpm: 108.0,
        key: "D",
        scale: "minor",
        intro: ["Dm", "Dm"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/cumbia/2",
        fighters: ["ACCORDION", "CLARINET"],
        bpm: 108.0,
        key: "D",
        scale: "minor",
        intro: ["Dm", "A7"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/cumbia/3",
        fighters: ["ACCORDION", "CLARINET"],
        bpm: 108.0,
        key: "D",
        scale: "minor",
        intro: ["Dm", "Dm"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[
        ("drums", 2),
        ("guacharaca", 1),
        ("bass", 2),
        ("chords", 2),
        ("accordion", 2),
    ],
    fill: &["drums", "guacharaca"],
    breakdown: &[("accordion", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[
        ("drums", 3),
        ("guacharaca", 3),
        ("bass", 3),
        ("chords", 3),
        ("accordion", 3),
    ],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("guacharaca", 2), ("bass", 2), ("accordion", 3)],
    build: &[
        ("drums", 3),
        ("guacharaca", 3),
        ("bass", 3),
        ("chords", 3),
        ("accordion", 3),
        ("riser", 1),
    ],
};

/// The cumbia's own band (sound upgrade, stage 2): the felt kick on 1 and 3 and the llamador's slap on 2 and 4,
/// the shaker on the off-beats, the guacharaca's long-short-short scrape on every beat, the bass on one and three
/// (root and fifth) with a pickup, the guitar chopping the off-beats and the accordion's figure.
pub static GROOVE: Groove = Groove {
    push_ms: 6.0,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(KICK), "x.......x......."),
                line(1, Play::Drum(SNARE), "....x.......x..."),
                line(1, Play::Drum(HAT), "..x...x...x...x."),
                line(2, Play::Drum(SNARE), "..............g.").maybe(0.6),
                only(3, Play::Drum(HAT), "o...o...o...o..."),
            ],
            fill: &[],
        },
        Part {
            layer: "guacharaca",
            lines: &[
                upto(1, Play::Drum(CLAP), "x_x.x_x.x_x.x_x."),
                line(2, Play::Drum(CLAP), "X_oox_ooX_oox_oo"),
                only(3, Play::Drum(SHAKER), "..x...x...x...x."),
            ],
            fill: &[line(1, Play::Drum(CLAP), "xxxxxxxX")],
        },
        Part {
            layer: "bass",
            lines: &[
                line(1, Play::Root, "x_.............."),
                line(1, Play::Fifth, "........x_......"),
                line(2, Play::Third, "..............o.").maybe(0.7),
                only(3, Play::Root, "......g.........").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "chords",
            lines: &[
                upto(1, Play::Chord, "....x.......x..."),
                line(2, Play::Chord, "..x...x...x...x."),
                only(3, Play::Upper, "...g.......g....").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "accordion",
            lines: &[
                line(1, Play::Tone(0), "x.x...x.x...x..."),
                line(1, Play::Tone(1), "....x.......x.x."),
                only(3, Play::Tone(2), "...x.......x...."),
            ],
            fill: &[],
        },
    ],
    mix: &[("bass", -0.5), ("guitar", 5.5), ("perc", 1.0)],
    lows: &["bass"],
    ..STRAIGHT
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[
        ("drums", 2),
        ("guacharaca", 1),
        ("bass", 2),
        ("chords", 2),
        ("accordion", 1),
    ],
    &[
        ("drums", 3),
        ("guacharaca", 3),
        ("bass", 3),
        ("chords", 3),
        ("accordion", 3),
        ("riser", 1),
    ],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "perc",
            |_, m, _, v| match m {
                KICK => scaled(super::bossa_noir::kick_felt(), v),
                SNARE => scaled(super::mambo::conga(super::mambo::Conga::Slap, false), 0.7 * v),
                HAT => scaled(super::bossa_noir::shaker(seed_of(v), v > 0.5), 0.8 * v),
                CRASH => scaled(synth::crash(1.0), 0.1 * v),
                _ => Vec::new(),
            },
            0.5,
            0.0,
            (0, 0),
        ),
        layer(
            "guacharaca",
            LayerKind::Perc,
            "perc",
            |_, m, _, v| match m {
                CLAP => scaled(super::dark_tango::chicharra(seed_of(v)), 0.6 * v),
                SHAKER => scaled(super::bossa_noir::shaker(seed_of(v), true), 0.5 * v),
                _ => Vec::new(),
            },
            0.35,
            0.25,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| super::balkan_fire::upright(m, d.min(0.45), v),
            0.6,
            0.0,
            (38, 50),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "guitar",
            |_, m, d, v| {
                scaled(
                    super::bossa_noir::pluck(m, d.clamp(0.1, 0.25), i64::from(m % 4), 0.7, 0.993),
                    v,
                )
            },
            0.2,
            -0.3,
            (55, 69),
        ),
        layer(
            "accordion",
            LayerKind::Arp,
            "accordion",
            |_, m, d, v| {
                scaled(
                    super::balkan_fire::reed(&[m], d.clamp(0.1, 0.3), 0.01, 0.06, 2600.0, super::balkan_fire::MUSETTE),
                    v,
                )
            },
            0.1,
            0.25,
            (64, 79),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("cumbia", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::grooved("cumbia", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
