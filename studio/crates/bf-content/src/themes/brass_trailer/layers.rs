//! The brass trailer's Composed arrangement (milestone 5): spiccato low strings driving the eighths, taiko on the
//! downbeats, trailer ticks, the high string tremolo and the dark pad, and the braam with its sub held under the
//! final exchange and the build.

use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, LayeredBed, SNARE};
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
    Ok(LayeredBed::new((THEME.classic)(options)?, layers(), TABLE, cues, seed)?)
}
