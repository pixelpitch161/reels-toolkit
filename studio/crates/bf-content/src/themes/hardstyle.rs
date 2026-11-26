//! Theme: hardstyle kick, a Studio theme (the batch of 2026-11-18; no kit mood behind it). Euphoric and pounding: a distorted kick on every beat, a soaring supersaw lead, an offbeat bass and big risers.
//! Built for F minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "hardstyle",
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
    &["Fm", "Db", "Ab", "Eb", "Fm", "Db", "Ab", "C"],
    &["Fm", "Db", "Ab", "Eb"],
    &["Db", "Eb", "Fm", "Fm"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (145.0, 158.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Hardstyle kick",
    text: "Euphoric and pounding: a distorted kick on every beat, a soaring supersaw lead, an offbeat bass and big risers.",
    key: "F",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/hardstyle/1",
        fighters: ["SYNTH", "DRUMS"],
        bpm: 150.0,
        key: "F",
        scale: "minor",
        intro: ["Fm", "C"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/hardstyle/2",
        fighters: ["SYNTH", "DRUMS"],
        bpm: 150.0,
        key: "F",
        scale: "minor",
        intro: ["Fm", "Eb"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/hardstyle/3",
        fighters: ["SYNTH", "DRUMS"],
        bpm: 150.0,
        key: "F",
        scale: "minor",
        intro: ["Db", "Fm"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("chords", 2), ("pad", 1)],
    fill: &["drums"],
    breakdown: &[("chords", 2), ("bass", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("chords", 3), ("pad", 2)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("chords", 3), ("pad", 1)],
    build: &[("drums", 3), ("bass", 3), ("chords", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("chords", 2), ("pad", 1)],
    &[("drums", 3), ("bass", 3), ("chords", 3), ("riser", 1)],
);

/// The hardstyle's own band (sound upgrade, stage 2): the distorted kick on every beat with the open hat between,
/// the clap on 2 and 4 and a kick roll for the fill, the Reese bass on the off-beats (rolling in pairs at the top),
/// and the supersaw lead's two-bar 3-3-2 figure.
pub static GROOVE: Groove = Groove {
    push_ms: 2.0,
    vel_spread: 0.06,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(KICK), "X...o...x...o..."),
                line(1, Play::Drum(HAT), "..x...x...x...x."),
                line(2, Play::Drum(SNARE), "....X.......X..."),
                only(3, Play::Drum(HAT), ".g.g.g.g.g.g.g.g"),
                only(3, Play::Drum(KICK), "..............x.").maybe(0.5),
            ],
            fill: &[
                line(1, Play::Drum(KICK), "X.x.XxXX"),
                line(1, Play::Drum(SNARE), "....x.xX"),
            ],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(1, Play::Root, "..x.......x....."),
                line(2, Play::Root, "..x...x...x...x."),
                only(3, Play::Root, "...x...x...x...x"),
            ],
            fill: &[],
        },
        Part {
            layer: "chords",
            lines: &[
                upto(1, Play::Chord, "x___....x___...."),
                line(2, Play::Chord, "X__X__X_X__X__X_X__X__X_X_X_X_X_"),
                line(2, Play::Top, "X__X__X_X__X__X_X__X__X_X_X_X_X_"),
                line(2, Play::Upper, "o__o__o_o__o__o_o__o__o_o_o_o_o_"),
                only(3, Play::Top, "...x...........x").maybe(0.5),
            ],
            fill: &[],
        },
    ],
    mix: &[("bass", -4.0), ("kit", -4.0), ("lead", 8.0), ("pad", 6.0)],
    lows: &["bass", "kit"],
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
                KICK => super::dark_breakbeat::crunch(&super::drift_phonk::kick(1.0))
                    .into_iter()
                    .map(|s| s * v)
                    .collect(),
                SNARE => scaled(super::dark_disco::clap(seed_of(v)), 0.8 * v),
                HAT => super::brass_house::hat(0.45 * v, v > 0.6, seed_of(v)),
                CRASH => scaled(synth::crash(1.8), 0.25 * v),
                _ => Vec::new(),
            },
            0.42,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| super::dark_breakbeat::reese_plain(m, d.clamp(0.08, 0.2), v),
            0.35,
            0.0,
            (29, 41),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "lead",
            |_, m, d, v| super::fresh::voices::supersaw(m, d.clamp(0.1, 0.4), 4200.0, v),
            0.1,
            0.2,
            (60, 74),
        ),
        layer(
            "pad",
            LayerKind::Pad,
            "pad",
            |_, m, d, v| super::fresh::voices::supersaw(m, d, 1600.0, v),
            0.05,
            -0.2,
            (53, 65),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("hardstyle", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::grooved("hardstyle", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
