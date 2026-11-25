//! The hot jazz's Composed arrangement (milestone 5): a two-beat on the felt bass drum, brushes and the swung
//! cymbal, the tuba on the root, the rhythm guitar's short chops and a woodblock for the breakdown. The final
//! exchange adds snare press rolls; the victory swings it out.

use bf_core::py::round_nd;
use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{
    ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE,
};
use bf_sound::synth;

use super::{PLUCK_RING, THEME, brush, cymbal, kick_felt, pluck, press, woodblock};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, riser, scaled, seed_of};

/// The home key and scale.
pub const KEY: (&str, &str) = ("D", "harmonic minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [
    &["Dm", "Dm", "Gm", "Gm", "Bb7", "A7", "Dm", "A7"],
    &["Dm", "Gm", "A7", "Dm"],
    &["Dm", "D7", "Gm", "Gm", "E7", "A7", "Dm", "A7"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (128.0, 160.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("tuba", 2), ("guitar", 2)],
    fill: &["drums", "press"],
    breakdown: &[("tuba", 1), ("guitar", 1), ("woodblock", 2)],
    final_exchange: &[("drums", 3), ("press", 2), ("tuba", 3), ("guitar", 3)],
    ability: &["guitar", "drums"],
    victory: &[("drums", 2), ("tuba", 3), ("guitar", 2), ("woodblock", 1)],
    build: &[("drums", 3), ("press", 3), ("tuba", 3), ("guitar", 3), ("riser", 1)],
};

/// The hot jazz's own band (sound upgrade, stage 2): a swung two-beat, the felt bass drum on 1 and 3, the brushes
/// and the cymbal choke on 2 and 4, press rolls leading into the next bar, the tuba on the root and the fifth (four
/// to the bar in the last chorus), the rhythm guitar's four chops with the weight on 2 and 4, and the woodblock's
/// clip-clop.
pub static GROOVE: Groove = Groove {
    swing: 0.6,
    push_ms: 5.0,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(KICK), "X.......x......."),
                line(1, Play::Drum(SNARE), "....x.......x..."),
                only(3, Play::Drum(SNARE), "......g.......g.").maybe(0.6),
                upto(2, Play::Drum(HAT), "....x.......x..."),
                only(3, Play::Drum(HAT), "x...x.o.x...x.o."),
            ],
            fill: &[
                line(1, Play::Drum(KICK), "x......."),
                line(1, Play::Drum(SNARE), "x.x.xxxX"),
            ],
        },
        Part {
            layer: "press",
            lines: &[
                line(1, Play::Drum(SHAKER), "..g...g...g...g."),
                line(2, Play::Drum(CLAP), "............x.................x."),
                only(3, Play::Drum(CLAP), "....x...........").maybe(0.6),
            ],
            fill: &[line(1, Play::Drum(CLAP), "x...x.X.")],
        },
        Part {
            layer: "tuba",
            lines: &[
                upto(1, Play::Root, "x_______........"),
                upto(1, Play::Fifth, "........x_______"),
                only(2, Play::Root, "X_______........X_______........"),
                only(2, Play::Fifth, "........x_______................"),
                only(2, Play::Third, "........................x_______"),
                only(3, Play::Root, "X...............X..............."),
                only(3, Play::Third, "....x.......x..................."),
                only(3, Play::Fifth, "........x...........x.......x..."),
                only(3, Play::Octave, "........................x......."),
            ],
            fill: &[],
        },
        Part {
            layer: "guitar",
            lines: &[
                upto(1, Play::Chord, "....x.......x..."),
                line(2, Play::Chord, "o...X...o...X..."),
                only(3, Play::Chord, "......g.......g.").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "woodblock",
            lines: &[
                line(1, Play::Drum(CLAP), "x.......x......."),
                line(1, Play::Drum(SHAKER), "....x.......x..."),
                line(2, Play::Drum(SHAKER), "..o...o...o...o."),
            ],
            fill: &[],
        },
    ],
    mix: &[("drums", -0.5), ("guitar", 2.0), ("tuba", -1.0)],
    lows: &["tuba"],
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
                KICK => scaled(kick_felt(), v),
                SNARE => scaled(brush(seed_of(v)), 0.35 * v),
                HAT => scaled(cymbal(seed_of(v) % 2), 0.12 * v),
                CRASH => scaled(synth::crash(1.0), 0.12 * v),
                _ => Vec::new(),
            },
            0.5,
            0.0,
            (0, 0),
        ),
        layer(
            "press",
            LayerKind::Perc,
            "drums",
            |_, m, _, v| match m {
                CLAP => press(0.2, v, seed_of(v), None),
                SHAKER => scaled(brush(seed_of(v) + 1), 0.6 * v),
                _ => Vec::new(),
            },
            0.14,
            -0.12,
            (0, 0),
        ),
        layer(
            "tuba",
            LayerKind::Bass,
            "tuba",
            |ctx, m, d, v| {
                let d = d.clamp(0.15, 0.5);
                let x = ctx.note("TUBA", f64::from(m), round_nd(d, 3), round_nd(d + 0.12, 2), 0.9);
                scaled(x.map(|x| x.to_vec()).unwrap_or_default(), v)
            },
            0.6,
            0.0,
            (36, 47),
        ),
        layer(
            "guitar",
            LayerKind::Comp,
            "guitar",
            |_, m, _, v| scaled(pluck(m, 0.18, i64::from(m % 4), 0.55, PLUCK_RING), v),
            0.2,
            0.18,
            (52, 67),
        ),
        layer(
            "woodblock",
            LayerKind::Perc,
            "drums",
            |_, m, _, v| scaled(woodblock(m == CLAP), v),
            0.08,
            0.25,
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
