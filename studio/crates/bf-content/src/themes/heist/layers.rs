//! The heist's Composed arrangement (milestone 5): spy jazz on brushes, the kick, the rim and the ride, the upright
//! on the root and the fifth, pizzicato chords on the off-beats, muted vibes, the high string tremolo held for the
//! tension and the clock ticking under it.

use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, LayeredBed, SNARE};
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
    Ok(LayeredBed::new((THEME.classic)(options)?, layers(), TABLE, cues, seed)?)
}
