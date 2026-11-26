//! Theme: chiptune brawl, a Studio theme (the batch of 2026-11-18; no kit mood behind it). An eight-bit arcade boss fight: square-wave leads, a triangle bass, noise drums and lightning-fast arpeggios.
//! Built for C major; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
//! layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of the Composed one, with
//! no story cues.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{ArrangementTable, CRASH, Cue, HAT, KICK, Layer, LayerKind, SNARE};
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled, seed_of};
use super::fresh::{self, ROOM};
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "chiptune",
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
    &["C", "Am", "F", "G", "C", "Am", "F", "G"],
    &["C", "G", "Am", "F"],
    &["Am", "F", "C", "G"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (140.0, 170.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Chiptune brawl",
    text: "An eight-bit arcade boss fight: square-wave leads, a triangle bass, noise drums and lightning-fast arpeggios.",
    key: "C",
    scale: "major",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 44),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/chiptune/1",
        fighters: ["SYNTH", "PICCOLO"],
        bpm: 156.0,
        key: "C",
        scale: "major",
        intro: ["C", "G"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/chiptune/2",
        fighters: ["SYNTH", "PICCOLO"],
        bpm: 156.0,
        key: "C",
        scale: "major",
        intro: ["C", "F"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/chiptune/3",
        fighters: ["SYNTH", "PICCOLO"],
        bpm: 156.0,
        key: "C",
        scale: "major",
        intro: ["Am", "G"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("chords", 2), ("lead", 2)],
    fill: &["drums"],
    breakdown: &[("lead", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("chords", 3), ("lead", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("lead", 3)],
    build: &[("drums", 3), ("bass", 3), ("chords", 3), ("lead", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("chords", 2), ("lead", 1)],
    &[("drums", 3), ("bass", 3), ("chords", 3), ("lead", 3), ("riser", 1)],
);

/// The chiptune's own band (sound upgrade, stage 2): noise drums with the kick on 1, the and of 2 and 3 and the
/// snare on 2 and 4, the triangle bass jumping root to octave in eighths, the pulse chords in a 3-3-2 and the
/// square lead's arpeggio at full speed, up one bar and down the next when everything is in.
pub static GROOVE: Groove = Groove {
    push_ms: 2.0,
    vel_spread: 0.06,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(KICK), "X.....x.X......."),
                line(1, Play::Drum(SNARE), "....X.......X..."),
                line(2, Play::Drum(HAT), "x.o.x.o.x.o.x.o."),
                only(3, Play::Drum(HAT), ".g.g.g.g.g.g.g.g"),
                only(3, Play::Drum(KICK), "..........x.....").maybe(0.5),
            ],
            fill: &[
                line(1, Play::Drum(KICK), "X...x..."),
                line(1, Play::Drum(SNARE), "x.x.xxxX"),
            ],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(1, Play::Root, "x.......x......."),
                line(2, Play::Root, "x...x...x...x..."),
                line(2, Play::Octave, "..x...x...x...x."),
                only(3, Play::Fifth, ".......x.......x"),
            ],
            fill: &[],
        },
        Part {
            layer: "chords",
            lines: &[
                upto(1, Play::Chord, "x.......x......."),
                line(2, Play::Chord, "x..x..x...x..x.."),
                only(3, Play::Upper, ".......x.......x").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "lead",
            lines: &[
                upto(1, Play::Tone(0), "x.......x......."),
                upto(1, Play::Tone(1), "..x.......x....."),
                upto(1, Play::Tone(2), "....x.......x..."),
                upto(1, Play::Tone(3), "......x.......x."),
                only(2, Play::Tone(0), "x...x...x...x..."),
                only(2, Play::Tone(1), ".x...x...x...x.."),
                only(2, Play::Tone(2), "..x...x...x...x."),
                only(2, Play::Tone(3), "...x...x...x...x"),
                only(3, Play::Tone(0), "X...x...x...x......x...x...x...x"),
                only(3, Play::Tone(1), ".x...x...x...x....x...x...x...x."),
                only(3, Play::Tone(2), "..x...x...x...x..x...x...x...x.."),
                only(3, Play::Tone(3), "...x...x...x...xX...x...x...x..."),
            ],
            fill: &[],
        },
    ],
    mix: &[("kit", -1.0), ("pulse", 3.0), ("square", 7.5), ("tri", -1.5)],
    lows: &["tri"],
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
                KICK => super::fresh::voices::chip_tri(36, 0.08, v),
                SNARE => scaled(super::fresh::voices::chip_noise(0.12, false, seed_of(v)), 0.6 * v),
                HAT => scaled(super::fresh::voices::chip_noise(0.04, true, seed_of(v)), 0.4 * v),
                CRASH => scaled(super::fresh::voices::chip_noise(0.5, false, 9), 0.25 * v),
                _ => Vec::new(),
            },
            0.4,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "tri",
            |_, m, d, v| super::fresh::voices::chip_tri(m, d.clamp(0.08, 0.3), v),
            0.45,
            0.0,
            (36, 48),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "pulse",
            |_, m, d, v| super::fresh::voices::chip(m, d.clamp(0.06, 0.18), 0.5, v),
            0.08,
            -0.2,
            (55, 69),
        ),
        layer(
            "lead",
            LayerKind::Arp,
            "square",
            |_, m, d, v| super::fresh::voices::chip(m, d.clamp(0.06, 0.15), 0.125, v),
            0.08,
            0.25,
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
    fresh::bed("chiptune", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::grooved("chiptune", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
