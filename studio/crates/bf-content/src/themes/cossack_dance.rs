//! Theme: cossack dance, a Studio theme (the batch of 2026-11-18; no kit mood behind it). A wild, accelerating dance of high kicks and shouts: a balalaika tremolo, a bayan accordion, a violin, a tambourine and low brass.
//! Built for E minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "cossack_dance",
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
    &["Em", "Em", "Am", "Em", "B7", "B7", "Em", "Em"],
    &["Em", "Am", "B7", "Em"],
    &["Em", "D", "C", "B7"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (124.0, 160.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Cossack dance",
    text: "A wild, accelerating dance of high kicks and shouts: a balalaika tremolo, a bayan accordion, a violin, a tambourine and low brass.",
    key: "E",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/cossack_dance/1",
        fighters: ["ACCORDION", "VIOLIN"],
        bpm: 140.0,
        key: "E",
        scale: "minor",
        intro: ["Em", "Em"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/cossack_dance/2",
        fighters: ["ACCORDION", "VIOLIN"],
        bpm: 140.0,
        key: "E",
        scale: "minor",
        intro: ["Em", "Em"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/cossack_dance/3",
        fighters: ["ACCORDION", "VIOLIN"],
        bpm: 140.0,
        key: "E",
        scale: "minor",
        intro: ["Em", "B7"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[
        ("drums", 2),
        ("bass", 2),
        ("chords", 2),
        ("balalaika", 2),
        ("violin", 1),
    ],
    fill: &["drums"],
    breakdown: &[("balalaika", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[
        ("drums", 3),
        ("bass", 3),
        ("chords", 3),
        ("balalaika", 3),
        ("violin", 2),
    ],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("balalaika", 3), ("violin", 1)],
    build: &[("drums", 3), ("bass", 3), ("chords", 3), ("balalaika", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[
        ("drums", 2),
        ("bass", 2),
        ("chords", 2),
        ("balalaika", 1),
        ("violin", 1),
    ],
    &[("drums", 3), ("bass", 3), ("chords", 3), ("balalaika", 3), ("riser", 1)],
);

/// The cossack dance's own band (sound upgrade, stage 2): a kazachok's oom-pah in two, the low brass on every beat
/// (root and fifth) and the bayan on the off-beats, the bass drum on the beat under a tambourine and a snare on
/// the off-beats, the stamps on the last beats of the two-bar phrase, and the balalaika's tremolo, a two-bar
/// figure moving off the top string and back.
pub static GROOVE: Groove = Groove {
    push_ms: 4.0,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                upto(1, Play::Drum(KICK), "X.......X......."),
                line(2, Play::Drum(KICK), "X...x...X...x...X...x...X.X.X..."),
                upto(1, Play::Drum(SNARE), "....x.......x..."),
                line(2, Play::Drum(SNARE), "..x...x...x...x...x...x...x...x."),
                only(3, Play::Drum(SNARE), "...g.......g.......g........X...").maybe(0.8),
                line(2, Play::Drum(HAT), "x.x.x.x.x.x.x.x."),
                only(3, Play::Drum(HAT), ".g.g.g.g.g.g.g.g"),
            ],
            fill: &[],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(1, Play::Root, "x..............."),
                upto(1, Play::Fifth, "........x......."),
                line(2, Play::Root, "X.......X......."),
                line(2, Play::Fifth, "....x.......x..."),
                only(3, Play::Third, "..............................x."),
                only(3, Play::Seventh, "..............g.................").maybe(0.5),
            ],
            fill: &[],
        },
        Part {
            layer: "chords",
            lines: &[
                upto(1, Play::Chord, "....x.......x..."),
                line(2, Play::Chord, "..x...x...x...x."),
                only(3, Play::Upper, "...g...g...g...g").maybe(0.7),
            ],
            fill: &[],
        },
        Part {
            layer: "balalaika",
            lines: &[
                upto(1, Play::Top, "x.x.x.x.x.x.x.x."),
                line(2, Play::Top, "XxoxXxoxXxoxXxox........XxoxXxox"),
                line(2, Play::Tone(2), "................XxoxXxox........"),
                only(3, Play::Tone(1), "x...x...x...x...x...x...x...x..."),
            ],
            fill: &[],
        },
    ],
    mix: &[("balalaika", -0.5), ("bayan", 2.0), ("brass", 3.5), ("drums", -2.5)],
    lows: &["brass"],
    ..STRAIGHT
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "drums",
            |_, m, _, v| match m {
                KICK => scaled(super::villain_march::bass_drum_boom(), 0.7 * v),
                SNARE => scaled(super::villain_march::march_snare_stroke(), 0.8 * v),
                HAT => scaled(super::big_top::tambourine(v, seed_of(v)), 0.5 * v),
                CRASH => scaled(synth::crash(1.4), 0.2 * v),
                _ => Vec::new(),
            },
            0.42,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "brass",
            |_, m, d, v| {
                scaled(
                    super::villain_march::brass(&[m], d.clamp(0.12, 0.3), 0.01, 900.0, 0.7),
                    v,
                )
            },
            0.35,
            0.0,
            (33, 45),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "bayan",
            |_, m, d, v| {
                scaled(
                    super::balkan_fire::reed(&[m], d.clamp(0.1, 0.25), 0.005, 0.05, 2000.0, 0.0),
                    v,
                )
            },
            0.15,
            -0.2,
            (52, 67),
        ),
        layer(
            "balalaika",
            LayerKind::Arp,
            "balalaika",
            |_, m, d, v| {
                scaled(
                    super::hot_jazz::pluck(m, d.clamp(0.05, 0.12), i64::from(m % 3), 0.9, 0.985),
                    v,
                )
            },
            0.24,
            0.3,
            (64, 79),
        ),
        layer(
            "violin",
            LayerKind::Pad,
            "violin",
            |_, m, d, v| scaled(super::baroque_storm::strng(m, d.min(1.5), 2600.0, 0.006, 0.05), v),
            0.07,
            0.1,
            (64, 76),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("cossack_dance", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::grooved("cossack_dance", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
