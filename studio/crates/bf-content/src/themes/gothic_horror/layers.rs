//! The gothic horror's Composed arrangement (milestone 5): the choir's "aah" held over a low drone, staccato
//! cellos driving the eighths, the heartbeat kick and the timpani, the clock ticking, and the church bell tolling
//! the chord when the fight turns grim.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only};
use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE};
use bf_sound::synth;

use super::{AAH, THEME, bell, cello, choir, drone, heart, roll, tick, timp};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, riser, scaled, seed_of};

/// The home key and scale.
pub const KEY: (&str, &str) = ("D", "minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [
    &["Dm", "Dm", "Bb", "A"],
    &["Dm", "Gm", "Bb", "A"],
    &["Dm", "C", "Bb", "A"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (116.0, 136.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("choir", 1), ("drone", 1), ("cellos", 2), ("heart", 1), ("clock", 1)],
    fill: &["heart", "clock"],
    breakdown: &[("choir", 2), ("drone", 1), ("bell", 1)],
    final_exchange: &[
        ("choir", 2),
        ("drone", 1),
        ("cellos", 3),
        ("heart", 2),
        ("pound", 1),
        ("clock", 2),
        ("bell", 1),
    ],
    ability: &["heart", "choir"],
    victory: &[
        ("choir", 2),
        ("drone", 1),
        ("cellos", 3),
        ("pound", 1),
        ("clock", 1),
        ("bell", 1),
    ],
    build: &[
        ("choir", 2),
        ("cellos", 3),
        ("pound", 1),
        ("clock", 3),
        ("roll", 1),
        ("riser", 1),
    ],
};

/// The gothic horror's own band (sound upgrade, stage 2): the heartbeat (lub-dub on 1 and 3) with the timpani on
/// 2 and 4 when the fight turns, the clock's tick and tock (its eighths and sixteenths as the tension grows), the
/// staccato cellos driving the eighths, and the church bell tolling the chord's top and its root.
pub static GROOVE: Groove = Groove {
    push_ms: 5.0,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "heart",
            lines: &[
                line(1, Play::Drum(KICK), "X.o.....X.o....."),
                only(2, Play::Drum(SNARE), "....x.......x..."),
                only(3, Play::Drum(SNARE), "....x.......x.x."),
            ],
            fill: &[line(1, Play::Drum(SNARE), "g.g.xxxX")],
        },
        Part {
            layer: "clock",
            lines: &[
                line(1, Play::Drum(CLAP), "x.......x......."),
                line(1, Play::Drum(SHAKER), "....x.......x..."),
                line(2, Play::Drum(SHAKER), "..g...g...g...g."),
                only(3, Play::Drum(SHAKER), ".g.g.g.g.g.g.g.g"),
            ],
            fill: &[],
        },
        Part {
            layer: "cellos",
            lines: &[
                line(1, Play::Root, "X.x.x.x.X.x.x..."),
                line(1, Play::Fifth, "..............x."),
                only(3, Play::Root, ".g.g.g.g.g.g.g.."),
            ],
            fill: &[],
        },
        Part {
            layer: "bell",
            lines: &[
                line(1, Play::Top, "X_______........"),
                line(1, Play::Tone(0), "........x_______"),
            ],
            fill: &[],
        },
    ],
    mix: &[
        ("drone", -2.5),
        ("drums", -2.5),
        ("bell", 9.5),
        ("choir", 9.5),
        ("perc", 9.5),
        ("strings", 6.0),
    ],
    lows: &["drone", "drums"],
    ..STRAIGHT
};

/// The layers, on the classic bed's instruments and buses.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "choir",
            LayerKind::Pad,
            "choir",
            |_, m, d, v| choir(&[m], d, 0.3, 0.5, AAH, v, m.unsigned_abs().into()),
            0.084,
            0.0,
            (57, 69),
        ),
        layer(
            "drone",
            LayerKind::Bass,
            "drone",
            |_, m, d, v| drone(m, d + 0.3, v),
            0.14,
            0.0,
            (26, 37),
        ),
        layer(
            "cellos",
            LayerKind::Bass,
            "strings",
            |_, m, d, v| cello(m, d.min(0.22), v),
            0.08,
            0.0,
            (38, 50),
        ),
        layer(
            "heart",
            LayerKind::Drums,
            "drums",
            |ctx, m, _, v| match m {
                KICK => heart(v),
                SNARE => timp(38 + (ctx.harmony.key_pc - 2).rem_euclid(12), 0.6 * v, 1.0),
                CRASH => scaled(synth::crash(1.4), 0.25 * v),
                _ => Vec::new(),
            },
            0.45,
            0.0,
            (0, 0),
        ),
        layer(
            "clock",
            LayerKind::Perc,
            "perc",
            |_, m, _, v| match m {
                CLAP => tick(v, false),
                _ => tick(v, seed_of(v).is_multiple_of(2)),
            },
            0.12,
            0.2,
            (0, 0),
        ),
        layer(
            "bell",
            LayerKind::Arp,
            "bell",
            |_, m, _, v| bell(m, 2.0, v),
            0.14,
            0.0,
            (62, 74),
        ),
        layer(
            "roll",
            LayerKind::Bass,
            "drums",
            |ctx, m, d, v| scaled(roll(d, m, 0.15, ctx.show.grid.s16 / 2.0), v),
            0.25,
            0.0,
            (38, 50),
        ),
        // the climax: a kick on every beat (an arpeggio at density 1 is one stroke a beat)
        layer(
            "pound",
            LayerKind::Arp,
            "drums",
            |_, _, _, _| synth::kick(1.0, 0.32, 150.0, 46.0),
            0.5,
            0.0,
            (36, 47),
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
