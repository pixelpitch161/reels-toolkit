//! The hot jazz's Composed arrangement (milestone 5): a two-beat on the felt bass drum, brushes and the swung
//! cymbal, the tuba on the root, the rhythm guitar's short chops and a woodblock for the breakdown. The final
//! exchange adds snare press rolls; the victory swings it out.

use bf_core::py::round_nd;
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
    Ok(LayeredBed::new((THEME.classic)(options)?, layers(), TABLE, cues, seed)?)
}
