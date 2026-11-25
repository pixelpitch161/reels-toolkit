//! The brass trailer's Composed arrangement (milestone 5): spiccato low strings driving the eighths, taiko on the
//! downbeats, trailer ticks, the high string tremolo and the dark pad, and the braam with its sub held under the
//! final exchange and the build.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{
    ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE,
};
use bf_sound::synth;

use super::{BRAAM_SWELL, THEME, braam, pad, strings_low, sub, taiko, tick, tremolo};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, riser, scaled, seed_of};

/// The home key and scale.
pub const KEY: (&str, &str) = ("D", "minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [
    &["Dm", "Bb", "Gm", "A"],
    &["Dm", "Bb", "Gm", "A", "Dm", "Bb", "Eb", "A"],
    &["Dm", "F", "C", "Bb"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (110.0, 130.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("low", 3), ("taiko", 1), ("ticks", 1), ("pad", 1), ("tremolo", 1)],
    fill: &["taiko"],
    breakdown: &[("pad", 2), ("tremolo", 1), ("ticks", 1)],
    final_exchange: &[
        ("low", 3),
        ("taiko", 3),
        ("boom", 1),
        ("ticks", 3),
        ("pad", 2),
        ("tremolo", 2),
        ("braam", 1),
    ],
    ability: &["taiko"],
    victory: &[("low", 3), ("taiko", 3), ("ticks", 1), ("pad", 2), ("tremolo", 2)],
    build: &[
        ("low", 3),
        ("taiko", 3),
        ("boom", 1),
        ("ticks", 2),
        ("pad", 2),
        ("tremolo", 2),
        ("braam", 1),
        ("riser", 1),
    ],
};

/// The brass trailer's own band (sound upgrade, stage 2): the spiccato strings' sixteenths in octaves, accented
/// three, three, two (turning to the fifth at the end of the second bar), moved up an octave so a phone carries
/// them; the taiko once a bar under the fight and pounding the three-three-two with the small drum on the backbeat
/// when everything is in; the trailer ticks in sixteenths with the small taiko on 2 and 4; and the high string
/// tremolo bowed on the bar and the half bar.
pub static GROOVE: Groove = Groove {
    push_ms: 4.0,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "low",
            lines: &[
                line(1, Play::Root, "XxxXxxXxXxxXxxXxXxxXxxXxXxxX...."),
                line(1, Play::Fifth, "............................xxxX"),
                line(2, Play::Octave, "XxxXxxXxXxxXxxXxXxxXxxXxXxxX...."),
            ],
            fill: &[],
        },
        Part {
            layer: "tremolo",
            lines: &[
                upto(1, Play::Chord, "X_______x_______"),
                line(2, Play::Chord, "X_______X_______"),
            ],
            fill: &[],
        },
        Part {
            layer: "taiko",
            lines: &[
                upto(1, Play::Drum(KICK), "X..............."),
                upto(1, Play::Drum(SNARE), "..........o...o."),
                line(2, Play::Drum(KICK), "X.....x.X......."),
                line(2, Play::Drum(SNARE), "....x.......x..."),
            ],
            fill: &[
                line(1, Play::Drum(KICK), "X...X.X."),
                line(1, Play::Drum(SNARE), ".gxg.x.X"),
            ],
        },
        Part {
            layer: "ticks",
            lines: &[
                line(1, Play::Drum(SHAKER), "X.x.X.x.X.x.X.x."),
                line(1, Play::Drum(SHAKER), ".x.x.x.x.x.x.x.x"),
                line(2, Play::Drum(CLAP), "....x.......x..."),
                only(3, Play::Drum(CLAP), "..............g.").maybe(0.5),
            ],
            fill: &[],
        },
    ],
    mix: &[("strings", 12.0), ("pad", 12.0), ("tremolo", 12.0)],
    lift: &[("low", 12)],
    lows: &["taiko", "braam"],
    ..STRAIGHT
};

/// The layers, on the classic bed's instruments and buses.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "low",
            LayerKind::Bass,
            "strings",
            |_, m, d, v| strings_low(m, d.min(0.25), v),
            0.132,
            0.0,
            (38, 50),
        ),
        layer(
            "taiko",
            LayerKind::Drums,
            "taiko",
            |_, m, _, v| match m {
                KICK => scaled(taiko(true, seed_of(v)), v),
                SNARE => scaled(taiko(false, seed_of(v)), 0.7 * v),
                CRASH => scaled(synth::crash(1.6), 0.3 * v),
                _ => Vec::new(),
            },
            0.36,
            0.0,
            (0, 0),
        ),
        layer(
            "ticks",
            LayerKind::Perc,
            "taiko",
            |_, m, _, v| match m {
                CLAP | HAT => scaled(taiko(false, seed_of(v)), 0.5 * v),
                _ => scaled(tick(seed_of(v)), v),
            },
            0.08,
            0.2,
            (0, 0),
        ),
        layer(
            "pad",
            LayerKind::Pad,
            "pad",
            |_, m, d, v| scaled(pad(&[m], d), v),
            0.04,
            0.0,
            (50, 65),
        ),
        layer(
            "tremolo",
            LayerKind::Pad,
            "tremolo",
            |_, m, d, v| scaled(tremolo(&[m], d, 0.3), v),
            0.023,
            0.0,
            (69, 84),
        ),
        layer(
            "braam",
            LayerKind::Bass,
            "braam",
            |_, m, d, v| {
                let mut x = braam(m, d.max(0.6), v, BRAAM_SWELL);
                for (o, s) in x.iter_mut().zip(sub(m, d.max(0.6))) {
                    *o += s;
                }
                x
            },
            0.16,
            0.0,
            (38, 49),
        ),
        // the big taiko on every beat (an arpeggio at density 1 is one stroke a beat)
        layer(
            "boom",
            LayerKind::Arp,
            "taiko",
            |_, m, _, _| taiko(true, m.unsigned_abs().into()),
            0.3,
            0.0,
            (38, 50),
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
