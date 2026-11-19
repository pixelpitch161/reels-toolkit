//! The drift phonk's Composed arrangement (milestone 5): the hard kick and the clap, rattling hats, the distorted
//! 808, the cowbell riffing the chord in sixteenths and a dark pad.

use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, LayeredBed, SNARE};
use bf_sound::synth;

use super::{THEME, b808, cowbell, kick, roll};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, riser, scaled};

/// The home key and scale.
pub const KEY: (&str, &str) = ("E", "minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [&["E", "E", "C", "B"], &["Em", "C", "Am", "B"], &["Em", "Em", "G", "B"]];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (124.0, 140.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("808", 2), ("cowbell", 2), ("pad", 1)],
    fill: &["drums", "rattle"],
    breakdown: &[("pad", 2), ("cowbell", 1)],
    final_exchange: &[("drums", 3), ("rattle", 2), ("808", 3), ("cowbell", 3), ("pad", 2)],
    ability: &["drums", "808"],
    victory: &[("drums", 3), ("808", 3), ("cowbell", 3), ("pad", 2), ("rattle", 1)],
    build: &[
        ("drums", 3),
        ("rattle", 3),
        ("808", 3),
        ("cowbell", 3),
        ("pad", 2),
        ("roll", 1),
        ("riser", 1),
    ],
};

/// The layers, on the classic bed's instruments and buses.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "drums",
            |_, m, _, v| match m {
                KICK => scaled(kick(1.0), v),
                SNARE => scaled(synth::clap(), 0.7 * v),
                HAT => scaled(synth::hat(0.03, false), 0.3 * v),
                CRASH => scaled(synth::crash(1.4), 0.25 * v),
                _ => Vec::new(),
            },
            0.96,
            0.0,
            (0, 0),
        ),
        layer(
            "rattle",
            LayerKind::Perc,
            "drums",
            |_, m, _, v| match m {
                CLAP => scaled(synth::clap(), v),
                _ => scaled(synth::hat(0.02, false), v),
            },
            0.16,
            0.25,
            (0, 0),
        ),
        layer(
            "808",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| scaled(b808(m, m, d.max(0.15), 0.0), v),
            0.77,
            0.0,
            (28, 40),
        ),
        layer(
            "cowbell",
            LayerKind::Arp,
            "cowbell",
            |_, m, _, v| scaled(cowbell(m, 0.1), v),
            0.24,
            -0.3,
            (76, 88),
        ),
        layer(
            "pad",
            LayerKind::Pad,
            "pad",
            |_, m, d, v| scaled(synth::pad(&[f64::from(m)], d, 900.0, 0.4), v),
            0.05,
            0.0,
            (52, 66),
        ),
        layer(
            "roll",
            LayerKind::Bass,
            "drums",
            |ctx, _, d, v| scaled(roll(d, 0.15, ctx.show.grid.s16 / 2.0), v),
            0.2,
            0.0,
            (28, 40),
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
