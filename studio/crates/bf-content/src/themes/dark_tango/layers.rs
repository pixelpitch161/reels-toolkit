//! The dark tango's Composed arrangement (milestone 5): the marcato on every beat (the guitar's chop), the
//! bandoneon low on the root and stabbing the off-beats, a held bandoneon chord, the knuckle on the bass's body and
//! the chicharra scraping when the fight heats up.

use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, KICK, Layer, LayerKind, LayeredBed, SNARE};
use bf_sound::synth::{self, FILTER_O};

use super::{PLUCK_RING, REED_ATT, REED_BARK, THEME, chicharra, golpe, latigo, pluck, reed};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, riser, scaled, seed_of};

/// The home key and scale.
pub const KEY: (&str, &str) = ("D", "harmonic minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [
    &["Dm", "A7", "A7", "Dm", "Gm", "Dm", "E7", "A7"],
    &["Dm", "Gm", "A7", "Dm"],
    &["Dm", "Bb", "Gm", "A7"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (110.0, 128.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("marcato", 1), ("bass", 2), ("bandoneon", 1), ("held", 1), ("golpe", 1)],
    fill: &["golpe", "chicharra"],
    breakdown: &[("held", 2), ("bass", 1)],
    final_exchange: &[
        ("marcato", 2),
        ("bass", 3),
        ("bandoneon", 2),
        ("held", 2),
        ("golpe", 2),
        ("chicharra", 2),
    ],
    ability: &["bandoneon", "golpe"],
    victory: &[
        ("marcato", 1),
        ("bass", 3),
        ("bandoneon", 2),
        ("held", 2),
        ("golpe", 1),
        ("chicharra", 1),
    ],
    build: &[
        ("marcato", 2),
        ("bass", 3),
        ("bandoneon", 3),
        ("held", 2),
        ("golpe", 2),
        ("chicharra", 2),
        ("riser", 1),
    ],
};

/// The layers, on the classic bed's instruments and buses.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "marcato",
            LayerKind::Arp,
            "guitar",
            |_, m, d, v| scaled(pluck(m, d.min(0.2) + 0.05, i64::from(m), 0.3, PLUCK_RING), v),
            0.16,
            0.2,
            (52, 64),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bandoneon",
            |ctx, m, d, v| {
                let x = reed(m, d.min(0.45 * ctx.show.grid.beat * 2.0), 0.02, REED_BARK);
                scaled(synth::lp(&x, 1800.0, FILTER_O), v)
            },
            0.3,
            -0.12,
            (38, 50),
        ),
        layer(
            "bandoneon",
            LayerKind::Comp,
            "bandoneon",
            |_, m, d, v| scaled(reed(m, d.min(0.18), REED_ATT, REED_BARK), v),
            0.09,
            -0.12,
            (57, 70),
        ),
        layer(
            "held",
            LayerKind::Pad,
            "bandoneon",
            |_, m, d, v| scaled(reed(m, d, 0.3, 0.3), v),
            0.035,
            0.1,
            (50, 64),
        ),
        layer(
            "golpe",
            LayerKind::Drums,
            "perc",
            |_, m, _, v| match m {
                KICK => scaled(golpe(seed_of(v)), v),
                SNARE => scaled(golpe(seed_of(v) + 1), 0.6 * v),
                CRASH => scaled(latigo(), v),
                _ => Vec::new(),
            },
            0.3,
            0.0,
            (0, 0),
        ),
        layer(
            "chicharra",
            LayerKind::Perc,
            "perc",
            |_, m, _, v| match m {
                CLAP => scaled(golpe(seed_of(v)), v),
                _ => scaled(chicharra(seed_of(v)), v),
            },
            0.08,
            0.15,
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
