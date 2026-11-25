//! The industrial's Composed arrangement (milestone 5): the factory floor's kick and metal backbeat, ticking
//! valves, tuned pipes for the chords, the grinder's gated power chord on the root and anvil strikes. A desperation
//! move empties the floor to the pipes and the anvil; the final exchange runs every machine.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{
    ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE,
};

use super::{THEME, anvil, backbeat, grind, kick, pipe, sheet, tick};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, riser, scaled, seed_of};

/// The home key and scale.
pub const KEY: (&str, &str) = ("G", "minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [
    &["Gm", "Eb", "Cm", "D"],
    &["Gm", "Gm", "Ab", "D"],
    &["Gm", "F", "Eb", "D", "Gm", "Bb", "Cm", "D"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (116.0, 136.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("grind", 1), ("pipes", 1)],
    fill: &["drums", "anvil"],
    breakdown: &[("pipes", 2), ("anvil", 1)],
    final_exchange: &[("drums", 3), ("grind", 2), ("pipes", 2), ("anvil", 2)],
    ability: &["grind", "anvil"],
    victory: &[("drums", 2), ("grind", 1), ("pipes", 2), ("anvil", 1)],
    build: &[("drums", 3), ("grind", 2), ("pipes", 3), ("anvil", 2), ("riser", 1)],
};

/// The industrial's own band (sound upgrade, stage 2): the kick on 1 and 3 with the metal backbeat on 2 and 4, a
/// second kick moving over two bars, the valves ticking in eighths and then sixteenths with a sheet-metal crash
/// now and then, the grinder's gated power chord chugging, the pipes ringing out a sparse figure and the anvil.
pub static GROOVE: Groove = Groove {
    push_ms: 2.0,
    vel_spread: 0.08,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(KICK), "X.......X......."),
                line(1, Play::Drum(SNARE), "....X.......X..."),
                line(2, Play::Drum(KICK), "......x.......x.......x...x...x."),
                line(2, Play::Drum(HAT), "x.o.x.o.x.o.x.o."),
                only(3, Play::Drum(HAT), ".g.g.g.g.g.g.g.g"),
                only(3, Play::Drum(CRASH), "X...............").maybe(0.25),
            ],
            fill: &[],
        },
        Part {
            layer: "grind",
            lines: &[
                upto(1, Play::Root, "X.x...x.X.x...x."),
                line(2, Play::Root, "X.xxX.x.X.xxX.xx"),
                only(3, Play::Fifth, "...x.......x...."),
            ],
            fill: &[],
        },
        Part {
            layer: "pipes",
            lines: &[
                line(1, Play::Tone(0), "x.........x....."),
                line(1, Play::Tone(2), "......x........."),
                line(2, Play::Tone(1), "...x.......x..x."),
                only(3, Play::Top, "........x...............x...x..."),
            ],
            fill: &[],
        },
        Part {
            layer: "anvil",
            lines: &[
                line(1, Play::Drum(CLAP), "..........X....."),
                line(2, Play::Drum(SHAKER), "..x...x...x...x."),
                only(3, Play::Drum(CLAP), "...x............").maybe(0.5),
            ],
            fill: &[line(1, Play::Drum(CLAP), "X.x.XxXX")],
        },
    ],
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
                SNARE => scaled(backbeat(seed_of(v)), 0.6 * v),
                HAT => scaled(tick(seed_of(v), false), 0.25 * v),
                CRASH => scaled(sheet(seed_of(v) % 5, 1.2), 0.5 * v),
                _ => Vec::new(),
            },
            0.55,
            0.0,
            (0, 0),
        ),
        layer(
            "grind",
            LayerKind::Bass,
            "grind",
            |_, m, d, v| scaled(grind(m, d.clamp(0.06, 0.25), seed_of(v)), v),
            0.12,
            0.2,
            (62, 73),
        ),
        layer(
            "pipes",
            LayerKind::Arp,
            "metal",
            |_, m, _, v| scaled(pipe(m, seed_of(v)), v),
            0.12,
            -0.15,
            (67, 86),
        ),
        layer(
            "anvil",
            LayerKind::Perc,
            "drums",
            |_, m, _, v| match m {
                CLAP => scaled(anvil(seed_of(v)), v),
                SHAKER => scaled(tick(seed_of(v), true), 0.4 * v),
                _ => Vec::new(),
            },
            0.14,
            -0.2,
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
