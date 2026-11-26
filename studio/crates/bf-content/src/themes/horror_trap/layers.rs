//! The horror trap's Composed arrangement (milestone 5): the trap kit (kick, clap, rattling hats), the sub 808 on
//! the root, the detuned music box up high, the string pad and the reverse swell into frame 0. A desperation move
//! leaves the music box alone over the strings; the final exchange brings every layer in.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, LayeredBed, SNARE};
use bf_sound::synth;

use super::{BOX_HI, BOX_LO, STRINGS_LO, THEME, b808, clap, hat, kick, rev_swell, strings, tine};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, scaled, seed_of};

/// The home key and scale.
pub const KEY: (&str, &str) = ("E", "minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [
    &["Em", "Em", "C", "B", "Em", "Em", "Am", "B"],
    &["Em", "C", "Am", "B"],
    &["Em", "F", "Em", "B"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (130.0, 150.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("808", 2), ("musicbox", 1), ("strings", 1)],
    fill: &["drums"],
    breakdown: &[("musicbox", 2), ("strings", 1)],
    final_exchange: &[("drums", 3), ("808", 3), ("musicbox", 2), ("strings", 2)],
    ability: &["808", "drums"],
    victory: &[("drums", 1), ("808", 2), ("musicbox", 2), ("strings", 2)],
    build: &[("drums", 3), ("808", 2), ("musicbox", 3), ("swell", 1)],
};

/// The horror trap's own band (sound upgrade, stage 2): the half-time trap kit (the clap on 3, the kick moving over
/// two bars, hats in quarters, eighths and then sixteenths), the sub 808 riding the kick and the detuned music box
/// picking a slow, eerie figure over the strings; a light sixteenth swing.
pub static GROOVE: Groove = Groove {
    swing: 0.54,
    swing16: true,
    push_ms: 2.5,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(KICK), "X.........x....."),
                line(1, Play::Drum(SNARE), "........X......."),
                line(1, Play::Drum(HAT), "x...x...x...x..."),
                line(2, Play::Drum(HAT), "..x...x...x...x."),
                line(2, Play::Drum(KICK), "......x............x.........x.."),
                only(3, Play::Drum(HAT), ".g.g.g.g.g.g.g.g"),
                only(3, Play::Drum(SNARE), "...............g").maybe(0.4),
            ],
            fill: &[],
        },
        Part {
            layer: "808",
            lines: &[
                upto(1, Play::Root, "X_______________"),
                line(2, Play::Root, "X_____x___x_____X_____x___x__x__"),
                only(3, Play::Octave, "..............x.").maybe(0.5),
            ],
            fill: &[],
        },
        Part {
            layer: "musicbox",
            lines: &[
                line(1, Play::Top, "x.........x....."),
                line(1, Play::Tone(1), "....x..........."),
                line(2, Play::Tone(2), "......x.......x."),
                only(3, Play::Tone(0), "..x.....x......."),
            ],
            fill: &[],
        },
    ],
    lows: &["808"],
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
                SNARE | CLAP => scaled(clap(seed_of(v)), 0.9 * v),
                HAT => scaled(hat(false, seed_of(v)), 0.35 * v),
                CRASH => scaled(synth::crash(1.6), 0.2 * v),
                _ => Vec::new(),
            },
            0.5,
            0.05,
            (0, 0),
        ),
        layer(
            "808",
            LayerKind::Bass,
            "808",
            |_, m, d, v| scaled(b808(m, d.clamp(0.3, 1.2), None, 0.0), v),
            0.5,
            0.0,
            (28, 40),
        ),
        layer(
            "musicbox",
            LayerKind::Arp,
            "musicbox",
            |_, m, d, v| scaled(tine(m, d.clamp(0.2, 0.6), 0.18), v),
            0.13,
            -0.3,
            (BOX_LO, BOX_HI),
        ),
        layer(
            "strings",
            LayerKind::Pad,
            "strings",
            |_, m, d, v| scaled(strings(&[m], d, false), v),
            0.2,
            0.0,
            (STRINGS_LO, STRINGS_LO + 19),
        ),
        // the build into frame 0: the classic bed's reverse swell, once a bar
        layer(
            "swell",
            LayerKind::Bass,
            "swell",
            |_, _, d, v| scaled(rev_swell(d, None), v),
            0.18,
            0.0,
            (28, 40),
        ),
    ]
}

/// The Composed bed for the fight's `options`, its story `cues` and the music `seed`.
///
/// # Errors
/// The classic bed refuses the options.
pub fn composed(options: &ThemeOptions, cues: Vec<Cue>, seed: u64) -> Result<LayeredBed, ContentError> {
    Ok(LayeredBed::new((THEME.classic)(options)?, layers(), TABLE, cues, seed)?.with_groove(&GROOVE)?)
}
