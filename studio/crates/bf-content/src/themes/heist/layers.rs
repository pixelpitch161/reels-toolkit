//! The heist's Composed arrangement (milestone 5): spy jazz on brushes, the kick, the rim and the ride, the upright
//! on the root and the fifth, pizzicato chords on the off-beats, muted vibes, the high string tremolo held for the
//! tension and the clock ticking under it.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{
    ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE,
};
use bf_sound::synth;

use super::{THEME, brush, kick, pizz, ride, rim, tick, trem, upright, vibes};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, riser, scaled, seed_of};

/// The home key and scale.
pub const KEY: (&str, &str) = ("D", "minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [
    &["Dm", "Dm", "Bb", "A7", "Dm", "Gm", "Bb", "A7"],
    &["Dm", "Bb", "Gm", "A7"],
    &["Dm", "Gm", "Dm", "A7"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (110.0, 128.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("pizz", 1), ("vibes", 1), ("tension", 1)],
    fill: &["drums", "brushes"],
    breakdown: &[("bass", 1), ("tension", 2), ("ticks", 2)],
    final_exchange: &[
        ("drums", 3),
        ("brushes", 2),
        ("bass", 3),
        ("pizz", 2),
        ("vibes", 2),
        ("tension", 2),
        ("ticks", 1),
    ],
    ability: &["pizz", "drums"],
    victory: &[
        ("drums", 2),
        ("brushes", 1),
        ("bass", 3),
        ("pizz", 2),
        ("vibes", 2),
        ("tension", 1),
    ],
    build: &[
        ("drums", 3),
        ("brushes", 2),
        ("bass", 3),
        ("pizz", 3),
        ("vibes", 2),
        ("ticks", 3),
        ("riser", 1),
    ],
};

/// The heist's own band (sound upgrade, stage 2): spy jazz with a light swing, the ride's spang-a-lang soft over a
/// kick on 1 and 3 and the rim clicking on 4, the brushes swishing the eighths, a walking upright (a two-feel when
/// the band thins out), pizzicato plucks on the off-beats, a muted two-bar vibes figure that sneaks down and back
/// up, the tremolo held over the bar, and the clock ticking the quarters, then the eighths, then everything it has.
pub static GROOVE: Groove = Groove {
    swing: 0.62,
    push_ms: 5.0,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                upto(1, Play::Drum(HAT), "o...o...o...o..."),
                only(2, Play::Drum(HAT), "o...o.g.o...o.g."),
                only(3, Play::Drum(HAT), "o...x.o.o...x.o."),
                line(1, Play::Drum(KICK), "o.......o......."),
                only(3, Play::Drum(KICK), "..............g.").maybe(0.6),
                line(1, Play::Drum(SNARE), "............x..."),
                only(3, Play::Drum(SNARE), "......g...............g.........").maybe(0.6),
            ],
            fill: &[
                line(1, Play::Drum(KICK), "x......."),
                line(1, Play::Drum(SNARE), "..x.x.xX"),
            ],
        },
        Part {
            layer: "brushes",
            lines: &[
                upto(1, Play::Drum(SHAKER), "o.g.o.g.o.g.o.g."),
                line(2, Play::Drum(SHAKER), "o.g.x.g.o.g.x.gg"),
            ],
            fill: &[line(1, Play::Drum(SHAKER), "g.o.x.xX")],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(1, Play::Root, "x_______........"),
                upto(1, Play::Fifth, "........x_______"),
                line(2, Play::Root, "x...............x..............."),
                line(2, Play::Third, "....x.......................x..."),
                line(2, Play::Fifth, "........x...............x......."),
                line(2, Play::Octave, "............x..................."),
                line(2, Play::Seventh, "....................x..........."),
                only(3, Play::Fifth, "..............g...............g.").maybe(0.5),
            ],
            fill: &[],
        },
        Part {
            layer: "pizz",
            lines: &[
                line(1, Play::Chord, "..x...x...x...x."),
                line(2, Play::Upper, "x.......................x......."),
                only(3, Play::Upper, "....o...............o...........").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "vibes",
            lines: &[
                line(1, Play::Tone(0), "..........x....................."),
                line(1, Play::Tone(1), "......x.......x...o............."),
                line(1, Play::Tone(2), "..o.x...........X.............o."),
                line(1, Play::Tone(3), "X.....................x.....x..."),
                line(1, Play::Tone(4), "........................X......."),
                line(2, Play::Tone(4), "........g.................g....."),
            ],
            fill: &[],
        },
        Part {
            layer: "tension",
            lines: &[
                upto(1, Play::Chord, "o_______________"),
                line(2, Play::Chord, "x_______________"),
            ],
            fill: &[],
        },
        Part {
            layer: "ticks",
            lines: &[
                upto(1, Play::Drum(KICK), "x...x...x...x..."),
                only(2, Play::Drum(KICK), "x.o.x.o.x.o.x.o."),
                only(3, Play::Drum(KICK), "x.ogx.ogx.ogx.og"),
            ],
            fill: &[],
        },
    ],
    mix: &[
        ("bass", -2.5),
        ("drums", -2.5),
        ("pizz", 9.5),
        ("tension", 9.5),
        ("vibes", 9.5),
    ],
    lows: &["bass"],
    ..STRAIGHT
};

/// The layers, on the classic bed's instruments and buses.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "drums",
            |_, m, _, v| match m {
                KICK => scaled(kick(), v),
                SNARE => scaled(rim(seed_of(v)), 0.3 * v),
                HAT => scaled(ride(), 0.2 * v),
                CRASH => scaled(synth::crash(1.2), 0.15 * v),
                _ => Vec::new(),
            },
            0.55,
            0.0,
            (0, 0),
        ),
        layer(
            "brushes",
            LayerKind::Perc,
            "drums",
            |_, m, _, v| match m {
                CLAP => scaled(rim(seed_of(v)), v),
                _ => scaled(brush(seed_of(v)), v),
            },
            0.12,
            -0.1,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| scaled(upright(m, d.min(0.6)), v),
            0.5,
            0.0,
            (38, 50),
        ),
        layer(
            "pizz",
            LayerKind::Comp,
            "pizz",
            |_, m, _, v| scaled(pizz(&[m]), v),
            0.17,
            -0.25,
            (55, 69),
        ),
        layer(
            "vibes",
            LayerKind::Arp,
            "vibes",
            |_, m, _, v| scaled(vibes(&[m], 0.4), v),
            0.1,
            0.3,
            (62, 76),
        ),
        layer(
            "tension",
            LayerKind::Pad,
            "tension",
            |_, m, d, v| scaled(trem(m, d), v),
            0.06,
            0.0,
            (69, 81),
        ),
        layer(
            "ticks",
            LayerKind::Perc,
            "tension",
            |_, _, _, v| scaled(tick(), v),
            0.08,
            0.35,
            (0, 0),
        ),
        riser(),
    ]
}

/// The Composed bed for the fight's `options`, its story `cues` and the music `seed`.
///
/// # Errors
/// The classic bed refuses the options.
pub fn composed(options: &ThemeOptions, cues: Vec<Cue>, seed: u64) -> Result<LayeredBed, ContentError> {
    Ok(LayeredBed::new((THEME.classic)(options)?, layers(), TABLE, cues, seed)?.with_groove(&GROOVE)?)
}
