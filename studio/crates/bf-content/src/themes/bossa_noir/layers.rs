//! The bossa noir's Composed arrangement (milestone 5): brushes, the felt kick and the cross-stick, a soft upright
//! on the root and the fifth, nylon chords on the off-beats, a dark Rhodes held under them and a nylon arpeggio
//! when the fight heats up.

use bf_core::dmath::{PI, exp, sin};
use bf_sound::SRF;
use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, LayeredBed, SNARE};
use bf_sound::synth::{self, FILTER_O, clip};

use super::{PLUCK_RING, THEME, brush, choke, kick_felt, pluck, rim, shaker};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, riser, scaled, seed_of};

/// The home key and scale.
pub const KEY: (&str, &str) = ("D", "harmonic minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 2] = [
    &["Dm7", "Dm6", "Gm7", "C7", "Fmaj7", "Bbmaj7", "Em7b5", "A7"],
    &["Dm7", "Gm7", "Em7b5", "A7"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (118.0, 140.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("nylon", 2), ("rhodes", 1)],
    fill: &["drums", "shaker"],
    breakdown: &[("bass", 1), ("rhodes", 2), ("arp", 1)],
    final_exchange: &[
        ("drums", 3),
        ("bass", 3),
        ("nylon", 2),
        ("rhodes", 2),
        ("arp", 2),
        ("shaker", 2),
    ],
    ability: &["rhodes", "drums"],
    victory: &[
        ("drums", 2),
        ("bass", 3),
        ("nylon", 2),
        ("rhodes", 2),
        ("arp", 2),
        ("shaker", 1),
    ],
    build: &[
        ("drums", 3),
        ("bass", 3),
        ("nylon", 3),
        ("rhodes", 2),
        ("arp", 3),
        ("riser", 1),
    ],
};

/// A soft upright: a sine body, a little of the string, darkened.
fn upright(m: i32, dur: f64, v: f64) -> Vec<f64> {
    let st = pluck(m, dur + 0.1, 3, 0.10, 0.993);
    let f = synth::mtof(f64::from(m));
    let x: Vec<f64> = synth::tt(st.len())
        .iter()
        .zip(&st)
        .map(|(&t, s)| {
            let body = (sin(2.0 * PI * f * t) + 0.22 * sin(4.0 * PI * f * t)) * exp(-t / 0.55);
            let rel = if t < dur { 1.0 } else { exp(-(t - dur) / 0.05) };
            (0.9 * body + 0.25 * s) * clip(t / 0.006, 0.0, 1.0) * rel
        })
        .collect();
    scaled(synth::lp(&x, 520.0, FILTER_O), v)
}

/// A Rhodes tine: FM, the index falling from bright to round, a slow tremolo.
fn rhodes(m: i32, dur: f64, v: f64) -> Vec<f64> {
    let f = synth::mtof(f64::from(m));
    let n = ((dur + 0.3) * SRF) as usize;
    let x: Vec<f64> = synth::tt(n)
        .iter()
        .map(|&t| {
            let idx = 0.4 + 1.6 * exp(-t / 0.25);
            let tine = sin(2.0 * PI * f * t + idx * sin(2.0 * PI * f * t));
            let trem = 1.0 - 0.22 * (0.5 + 0.5 * sin(2.0 * PI * 4.6 * t));
            let rel = if t < dur { 1.0 } else { exp(-(t - dur) / 0.08) };
            tine * trem * clip(t / 0.008, 0.0, 1.0) * exp(-t / (dur * 1.4)) * rel
        })
        .collect();
    scaled(synth::lp(&x, 2200.0, FILTER_O), v)
}

/// The layers, on the classic bed's instruments and buses.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "drums",
            |_, m, _, v| match m {
                KICK => scaled(kick_felt(), v),
                SNARE => scaled(rim(seed_of(v)), 0.25 * v),
                HAT => scaled(brush(seed_of(v), false), 0.3 * v),
                CRASH => choke(0.5 * v, seed_of(v)),
                _ => Vec::new(),
            },
            0.45,
            0.0,
            (0, 0),
        ),
        layer(
            "shaker",
            LayerKind::Perc,
            "drums",
            |_, m, _, v| match m {
                CLAP => scaled(rim(seed_of(v)), v),
                _ => scaled(shaker(seed_of(v), v > 0.35), v),
            },
            0.06,
            0.35,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| upright(m, d, v),
            0.6,
            0.0,
            (38, 50),
        ),
        layer(
            "nylon",
            LayerKind::Comp,
            "guitar",
            |_, m, d, v| scaled(pluck(m, d + 0.15, i64::from(m), 0.34, PLUCK_RING), v),
            0.18,
            0.2,
            (52, 67),
        ),
        layer(
            "rhodes",
            LayerKind::Pad,
            "keys",
            |_, m, d, v| rhodes(m, d, v),
            0.18,
            -0.2,
            (55, 69),
        ),
        layer(
            "arp",
            LayerKind::Arp,
            "guitar",
            |_, m, d, v| scaled(pluck(m, d + 0.2, i64::from(m) + 10, 0.45, PLUCK_RING), v),
            0.15,
            0.3,
            (62, 77),
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
