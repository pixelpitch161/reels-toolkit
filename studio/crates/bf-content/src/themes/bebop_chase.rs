//! Theme: bebop chase, a Studio theme (the batch of 2026-11-18; no kit mood behind it). Breakneck and nervy: a walking upright bass, a ride cymbal, comping piano and a unison trumpet-and-sax head, a cartoon chase through a smoky club.
//! Built for Bb major; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "bebop_chase",
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
    &["Bb", "G7", "Cm7", "F7", "Dm7", "G7", "Cm7", "F7"],
    &["Bb", "G7", "Cm7", "F7"],
    &["Cm7", "F7", "Bb", "Bb"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (150.0, 172.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Bebop chase",
    text: "Breakneck and nervy: a walking upright bass, a ride cymbal, comping piano and a unison trumpet-and-sax head, a cartoon chase through a smoky club.",
    key: "Bb",
    scale: "major",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 44),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/bebop_chase/1",
        fighters: ["ALTO SAX", "TRUMPET"],
        bpm: 164.0,
        key: "Bb",
        scale: "major",
        intro: ["Bb", "F7"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/bebop_chase/2",
        fighters: ["ALTO SAX", "TRUMPET"],
        bpm: 164.0,
        key: "Bb",
        scale: "major",
        intro: ["Bb", "F7"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/bebop_chase/3",
        fighters: ["ALTO SAX", "TRUMPET"],
        bpm: 164.0,
        key: "Bb",
        scale: "major",
        intro: ["Cm7", "Bb"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("chords", 2), ("head", 2)],
    fill: &["drums"],
    breakdown: &[("head", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("chords", 3), ("head", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("head", 3)],
    build: &[("drums", 3), ("bass", 3), ("chords", 3), ("head", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("chords", 2), ("head", 1)],
    &[("drums", 3), ("bass", 3), ("chords", 3), ("head", 3), ("riser", 1)],
);

/// The bebop chase's own band (sound upgrade, stage 2): the ride's spang-a-lang over a feathered kick, the brush
/// on 2 and 4 where the hi-hat would be and the snare's comping, a walking upright on the quarters (a two-feel when
/// the band thins out), the piano comping rootless voicings on the Charleston and the off-beats, and a two-bar
/// unison head in swung eighths that ends on the and of 4.
pub static GROOVE: Groove = Groove {
    swing: 0.62,
    push_ms: 4.0,
    vel_spread: 0.09,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                upto(1, Play::Drum(HAT), "o...o...o...o..."),
                only(2, Play::Drum(HAT), "o...o.g.o...o.g."),
                only(3, Play::Drum(HAT), "o...x.o.o...x.o."),
                line(1, Play::Drum(KICK), "g...g...g...g..."),
                only(3, Play::Drum(KICK), "..........x...................x.").maybe(0.5),
                line(1, Play::Drum(SNARE), "....g.......g..."),
                line(2, Play::Drum(SNARE), "......o.......x...o.......o.....").maybe(0.6),
            ],
            fill: &[
                line(1, Play::Drum(KICK), "......x."),
                line(1, Play::Drum(SNARE), "x.xxx.xX"),
            ],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(1, Play::Root, "x_______........"),
                upto(1, Play::Fifth, "........x_______"),
                line(2, Play::Root, "X...............X..............."),
                line(2, Play::Third, "....x.......................x..."),
                line(2, Play::Fifth, "........x...........x..........."),
                line(2, Play::Seventh, "............x..................."),
                line(2, Play::Octave, "........................x......."),
                only(3, Play::Fifth, "..............g...............g.").maybe(0.5),
            ],
            fill: &[],
        },
        Part {
            layer: "chords",
            lines: &[
                upto(1, Play::Upper, "x.....x........."),
                line(2, Play::Upper, "x.....x.......o.......x...x....."),
                only(3, Play::Upper, "..........x.......o...........o.").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "head",
            lines: &[
                upto(1, Play::Tone(4), "x.............X."),
                upto(1, Play::Tone(2), "......x........."),
                line(2, Play::Tone(0), "........x......................."),
                line(2, Play::Tone(1), "......x...x....................."),
                line(2, Play::Tone(2), "....x.......x.......x..........."),
                line(2, Play::Tone(3), "..x.............x.....x........."),
                line(2, Play::Tone(4), "X.............x...............X."),
                line(2, Play::Tone(5), "..........................x....."),
                only(3, Play::Tone(2), "..................g.............").maybe(0.7),
                only(3, Play::Tone(4), "........................g.......").maybe(0.7),
            ],
            fill: &[],
        },
    ],
    mix: &[
        ("bass", -2.0),
        ("drums", 1.5),
        ("horns", 1.5),
        ("kit", 3.0),
        ("piano", 3.5),
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
            "kit",
            |_, m, _, v| match m {
                KICK => scaled(super::hot_jazz::kick_felt(), v),
                SNARE => scaled(super::heist::brush(seed_of(v)), 0.7 * v),
                HAT => scaled(super::noir_jazz::ride(v > 0.7), 0.45 * v),
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
            |_, m, d, v| super::noir_jazz::upright(m, d.min(0.4), v),
            0.6,
            0.0,
            (36, 48),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "piano",
            |_, m, d, v| super::fresh::voices::piano(m, d.clamp(0.1, 0.3), v),
            0.22,
            -0.25,
            (55, 70),
        ),
        layer(
            "head",
            LayerKind::Arp,
            "horns",
            |_, m, d, v| {
                scaled(
                    super::villain_march::brass(&[m], d.clamp(0.1, 0.22), 0.008, 3200.0, 1.0),
                    v,
                )
            },
            0.12,
            0.25,
            (65, 79),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("bebop_chase", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::grooved("bebop_chase", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
