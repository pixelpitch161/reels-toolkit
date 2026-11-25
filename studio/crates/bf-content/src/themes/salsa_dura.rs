//! Theme: salsa dura, a Studio theme (the batch of 2026-11-18; no kit mood behind it). Hot, tight and relentless: a piano montuno locked with a trombone section over timbales, congas, cowbell and a tumbao bass.
//! Built for A minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
//! layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of the Composed one, with
//! no story cues.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, SHAKER, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled};
use super::fresh::{self, ROOM};
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "salsa_dura",
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
    &["Am", "Dm", "E7", "Am", "Am", "Dm", "E7", "E7"],
    &["Am", "Dm", "G", "C"],
    &["Dm", "E7", "Am", "Am"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (156.0, 172.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Salsa dura",
    text: "Hot, tight and relentless: a piano montuno locked with a trombone section over timbales, congas, cowbell and a tumbao bass.",
    key: "A",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 44),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/salsa_dura/1",
        fighters: ["TROMBONE", "PIANO"],
        bpm: 166.0,
        key: "A",
        scale: "minor",
        intro: ["Am", "E7"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/salsa_dura/2",
        fighters: ["TROMBONE", "PIANO"],
        bpm: 166.0,
        key: "A",
        scale: "minor",
        intro: ["Am", "C"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/salsa_dura/3",
        fighters: ["TROMBONE", "PIANO"],
        bpm: 166.0,
        key: "A",
        scale: "minor",
        intro: ["Dm", "Am"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("congas", 1), ("bass", 2), ("montuno", 2), ("chords", 2)],
    fill: &["drums", "congas"],
    breakdown: &[("montuno", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("congas", 3), ("bass", 3), ("montuno", 3), ("chords", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("congas", 2), ("bass", 2), ("montuno", 3)],
    build: &[
        ("drums", 3),
        ("congas", 3),
        ("bass", 3),
        ("montuno", 3),
        ("chords", 3),
        ("riser", 1),
    ],
};

/// The salsa dura's own band (sound upgrade, stage 2), in 2-3 son clave: the bongo bell on the beats (the mambo
/// bell at the top), the bombo on the and of 2, the clave on the timbale's muted side and an abanico for the fill,
/// the conga tumbao (heel and toe, the slap on 2, the tones on 4 and the and of 4), the tumbao bass (the fifth on the
/// and of 2 and the root on 4, held over the bar line), a two-bar piano montuno and the trombones' mambo figure.
pub static GROOVE: Groove = Groove {
    push_ms: 4.0,
    vel_spread: 0.09,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                upto(2, Play::Drum(HAT), "X.g.x.g.X.g.x.g."),
                only(3, Play::Drum(HAT), "X.x.X.xxX.x.X.xx"),
                line(1, Play::Drum(KICK), "......x........."),
                only(3, Play::Drum(KICK), "............o...").maybe(0.6),
                line(1, Play::Drum(SNARE), "....g...g.......g.....g.....g..."),
            ],
            fill: &[
                line(1, Play::Drum(SNARE), "xxx.xxxX"),
                line(1, Play::Drum(KICK), "X......."),
            ],
        },
        Part {
            layer: "congas",
            lines: &[
                line(1, Play::Drum(SHAKER), "g.g...g.g.g....."),
                line(1, Play::Drum(CLAP), "....X.......x.x."),
                line(2, Play::Drum(SHAKER), "...g...g...g...g").maybe(0.7),
                only(3, Play::Drum(CLAP), "..x.............").maybe(0.5),
            ],
            fill: &[line(1, Play::Drum(CLAP), "x.xxx.xX")],
        },
        Part {
            layer: "bass",
            lines: &[
                line(1, Play::Fifth, "......x_____...."),
                line(1, Play::Root, "............x___"),
                only(3, Play::Octave, "..........g.....").maybe(0.5),
            ],
            fill: &[],
        },
        Part {
            layer: "montuno",
            lines: &[
                line(1, Play::Tone(0), "x...x.x...x.x...x.x...x...x.x..."),
                line(1, Play::Tone(2), "..x.....x.....x...x.....x.....x."),
                line(2, Play::Tone(1), "......x.......x.....x.......x..."),
                only(3, Play::Tone(3), "....x.......x.......x.......x...").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "chords",
            lines: &[
                upto(1, Play::Chord, "x.....x........."),
                line(2, Play::Chord, "X_..x_....x.x...........x.x.X___"),
                only(3, Play::Upper, "..o.......o.....").maybe(0.6),
            ],
            fill: &[],
        },
    ],
    mix: &[("bass", -2.5), ("bones", 4.5), ("perc", 5.0)],
    lows: &["bass"],
    ..STRAIGHT
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("congas", 1), ("bass", 2), ("montuno", 1), ("chords", 2)],
    &[
        ("drums", 3),
        ("congas", 3),
        ("bass", 3),
        ("montuno", 3),
        ("chords", 3),
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
                KICK => scaled(super::mambo::bombo(), v),
                SNARE => scaled(super::mambo::timbale(true, v < 0.6), 0.7 * v),
                HAT => scaled(super::mambo::bell(v > 0.5), 0.5 * v),
                CRASH => scaled(synth::crash(1.0), 0.15 * v),
                _ => Vec::new(),
            },
            0.5,
            0.0,
            (0, 0),
        ),
        layer(
            "congas",
            LayerKind::Perc,
            "perc",
            |_, m, _, v| match m {
                CLAP => scaled(super::mambo::conga(super::mambo::Conga::Slap, false), v),
                SHAKER => scaled(super::mambo::conga(super::mambo::Conga::Slap, true), 0.6 * v),
                _ => Vec::new(),
            },
            0.45,
            -0.2,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| super::noir_jazz::upright(m, d.min(0.45), v),
            0.6,
            0.0,
            (33, 45),
        ),
        layer(
            "montuno",
            LayerKind::Arp,
            "piano",
            |_, m, d, v| super::fresh::voices::piano(m, d.clamp(0.08, 0.2), v),
            0.22,
            -0.15,
            (60, 76),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "bones",
            |_, m, d, v| {
                scaled(
                    super::villain_march::brass(&[m], d.clamp(0.12, 0.3), 0.01, 2000.0, 1.2),
                    v,
                )
            },
            0.16,
            0.2,
            (52, 67),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("salsa_dura", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::grooved("salsa_dura", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
