//! The brass house's Composed arrangement (milestone 5): four on the floor, the clap on 2 and 4, closed hats on
//! the eighths and a shaker, saw stabs on the off-beats and the minor pad pumping under the kick.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, LayeredBed, SNARE};
use bf_sound::synth;

use super::{THEME, hat, kick, pad_bar, ride, stab};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, riser, scaled, seed_of};

/// The home key and scale.
pub const KEY: (&str, &str) = ("F", "minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [
    &["Fm", "Db", "Bbm", "C"],
    &["Fm", "Eb", "Db", "C"],
    &["Fm", "Bbm", "Eb", "Ab"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (120.0, 128.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("kick", 1), ("kit", 2), ("stabs", 1), ("pad", 1)],
    fill: &["kit", "shaker"],
    breakdown: &[("pad", 2), ("stabs", 1)],
    final_exchange: &[("kick", 1), ("kit", 3), ("stabs", 2), ("pad", 2), ("shaker", 2)],
    ability: &["stabs", "kit"],
    victory: &[
        ("kick", 1),
        ("kit", 3),
        ("lift", 3),
        ("stabs", 3),
        ("pad", 3),
        ("shaker", 2),
    ],
    build: &[
        ("kick", 1),
        ("kit", 3),
        ("lift", 3),
        ("stabs", 3),
        ("pad", 2),
        ("shaker", 2),
        ("riser", 1),
    ],
};

/// The brass house's own band (sound upgrade, stage 2): the four on the floor stays, the clap on 2 and 4 over
/// closed hats in eighths (sixteenths and the ride on the off-beats at the top), the open hat on every off-beat,
/// the saw stabs in a two-bar off-beat figure and the victory lift doubling kick, clap and open hat; a light
/// sixteenth swing.
pub static GROOVE: Groove = Groove {
    swing: 0.54,
    swing16: true,
    push_ms: 2.5,
    vel_spread: 0.08,
    parts: &[
        Part {
            layer: "kit",
            lines: &[
                line(1, Play::Drum(SNARE), "....X.......X..."),
                line(1, Play::Drum(HAT), "o.x.o.x.o.x.o.x."),
                only(3, Play::Drum(HAT), ".g.g.g.g.g.g.g.g"),
                only(3, Play::Drum(CRASH), "..o...o...o...o."),
            ],
            fill: &[],
        },
        Part {
            layer: "shaker",
            lines: &[
                line(1, Play::Drum(HAT), "..x...x...x...x."),
                only(3, Play::Drum(CLAP), "...............g").maybe(0.5),
            ],
            fill: &[],
        },
        Part {
            layer: "stabs",
            lines: &[
                upto(1, Play::Chord, "..x.......x....."),
                line(2, Play::Chord, "..x..x....x.......x..x....x..x.x"),
                only(3, Play::Upper, ".......x........").maybe(0.5),
            ],
            fill: &[],
        },
        Part {
            layer: "lift",
            lines: &[
                line(1, Play::Drum(KICK), "X...X...X...X..."),
                line(1, Play::Drum(SNARE), "....x.......x..."),
                line(1, Play::Drum(HAT), "..x...x...x...x."),
            ],
            fill: &[],
        },
    ],
    mix: &[("drums", 4.0), ("kit", -1.5), ("pad", 7.5), ("stabs", 4.5)],
    ..STRAIGHT
};

/// The layers, on the classic bed's instruments and buses.
pub fn layers() -> Vec<Layer> {
    vec![
        // an arpeggio at density 1 is one stroke a beat: the four on the floor
        layer(
            "kick",
            LayerKind::Arp,
            "kit",
            |_, _, _, v| kick(v.max(0.8)),
            0.801,
            0.0,
            (36, 47),
        ),
        layer(
            "kit",
            LayerKind::Drums,
            "kit",
            |_, m, _, v| match m {
                SNARE => scaled(synth::clap(), 0.8 * v),
                HAT => hat(v, false, seed_of(v)),
                CRASH => ride(v, seed_of(v)),
                _ => Vec::new(),
            },
            0.3,
            0.0,
            (0, 0),
        ),
        layer(
            "shaker",
            LayerKind::Perc,
            "kit",
            |_, m, _, v| match m {
                CLAP => scaled(synth::clap(), v),
                _ => hat(v, true, seed_of(v)),
            },
            0.117,
            0.2,
            (0, 0),
        ),
        layer(
            "stabs",
            LayerKind::Comp,
            "stabs",
            |_, m, d, v| scaled(stab(&[m], d.min(0.22), 3200.0), v),
            0.05,
            0.1,
            (60, 74),
        ),
        layer(
            "pad",
            LayerKind::Pad,
            "pad",
            |ctx, m, d, v| scaled(pad_bar(&[m], 0.0, ctx.show.grid.beat, d, 1800.0, 0.05), v),
            0.105,
            0.0,
            (53, 67),
        ),
        // the victory lift: the kick, the clap and an open hat doubled
        layer(
            "lift",
            LayerKind::Drums,
            "kit",
            |_, m, _, v| match m {
                KICK => kick(v),
                SNARE => scaled(synth::clap(), 0.8 * v),
                HAT => hat(v, true, seed_of(v)),
                _ => Vec::new(),
            },
            0.44,
            0.0,
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
