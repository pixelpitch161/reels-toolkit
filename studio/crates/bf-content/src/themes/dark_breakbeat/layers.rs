//! The dark breakbeat's Composed arrangement (milestone 5): a crunchy break (kick, snare and hats through the
//! classic bed's grit), the Reese bass, the cold pad and a chime arpeggio when the fight heats up.

use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, LayeredBed, SNARE};
use bf_sound::synth;

use super::{THEME, cold_pad, crunch, kick, reese_plain, snare_hit};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, riser, scaled};

/// The home key and scale.
pub const KEY: (&str, &str) = ("F", "minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [
    &["Fm", "Db", "Bbm", "C"],
    &["Fm", "Fm", "Db", "Eb"],
    &["Fm", "Ab", "Eb", "Db"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (130.0, 150.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("break", 3), ("bass", 2), ("pad", 1)],
    fill: &["break", "ghosts"],
    breakdown: &[("pad", 2), ("bells", 1)],
    final_exchange: &[("break", 3), ("ghosts", 2), ("bass", 3), ("pad", 2), ("bells", 2)],
    ability: &["break", "bass"],
    victory: &[("break", 3), ("bass", 3), ("pad", 2), ("bells", 1)],
    build: &[
        ("break", 3),
        ("ghosts", 3),
        ("bass", 3),
        ("pad", 2),
        ("bells", 3),
        ("riser", 1),
    ],
};

/// The layers, on the classic bed's instruments and buses.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "break",
            LayerKind::Drums,
            "break",
            |_, m, _, v| match m {
                KICK => crunch(&kick(v)),
                SNARE => scaled(crunch(&snare_hit()), 0.8 * v),
                HAT => scaled(synth::hat(0.04, false), 0.45 * v),
                CRASH => scaled(synth::crash(1.6), 0.25 * v),
                _ => Vec::new(),
            },
            0.5,
            0.0,
            (0, 0),
        ),
        layer(
            "ghosts",
            LayerKind::Perc,
            "break",
            |_, m, _, v| match m {
                CLAP => scaled(crunch(&snare_hit()), 0.5 * v),
                _ => scaled(synth::hat(0.03, false), 0.4 * v),
            },
            0.3,
            0.25,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| reese_plain(m, d.max(0.2), v),
            0.5,
            0.0,
            (29, 40),
        ),
        layer(
            "pad",
            LayerKind::Pad,
            "pad",
            |_, m, d, v| scaled(cold_pad(&[m], d, 1200.0), v),
            0.036,
            0.0,
            (53, 67),
        ),
        layer(
            "bells",
            LayerKind::Arp,
            "pad",
            |_, m, _, v| scaled(synth::chime(f64::from(m), 0.25), v),
            0.018,
            0.25,
            (77, 89),
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
