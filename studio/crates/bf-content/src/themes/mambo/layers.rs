//! The mambo's Composed arrangement (milestone 5): the timbales' cascara on the shell, the bombo and the cowbell,
//! congas, the contrabass's tumbao on the root, the piano montuno in octaves and the sax section's pad. A
//! desperation move drops to congas, bass and the montuno; the final exchange is the whole orchestra.

use bf_core::py::round_nd;
use bf_sound::music::MusicCtx;
use bf_sound::music::layers::{
    ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE,
};
use bf_sound::synth;

use super::{ALTO, Conga, THEME, bell, bombo, conga, shell, timbale};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, scaled};

/// The home key and scale.
pub const KEY: (&str, &str) = ("G", "harmonic minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [
    &["Gm", "Cm", "D7", "Gm", "Gm", "Cm", "Eb7", "D7"],
    &["Gm", "Cm", "D7", "Gm"],
    &["Gm", "Eb7", "D7", "Gm"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (120.0, 150.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("timbales", 2), ("congas", 1), ("bass", 2), ("piano", 2), ("horns", 1)],
    fill: &["timbales", "congas"],
    breakdown: &[("congas", 2), ("bass", 1), ("montuno", 1)],
    final_exchange: &[
        ("timbales", 3),
        ("congas", 2),
        ("bass", 3),
        ("piano", 3),
        ("montuno", 2),
        ("horns", 2),
    ],
    ability: &["piano", "timbales"],
    victory: &[
        ("timbales", 2),
        ("congas", 2),
        ("bass", 3),
        ("montuno", 2),
        ("horns", 2),
    ],
    build: &[
        ("timbales", 3),
        ("congas", 3),
        ("bass", 3),
        ("piano", 3),
        ("montuno", 3),
    ],
};

/// One note of a Pixel Pitch voice at a fixed velocity (so the voice's cache holds it), scaled by `vel`.
fn voice(ctx: &MusicCtx, name: &str, m: i32, dur: f64, vel: f64) -> Vec<f64> {
    let x = ctx.note(name, f64::from(m), round_nd(dur, 3), round_nd(dur + 0.18, 2), 0.9);
    scaled(x.map(|x| x.to_vec()).unwrap_or_default(), vel)
}

/// The layers, on the classic bed's instruments and buses.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "timbales",
            LayerKind::Drums,
            "perc",
            |_, m, _, v| match m {
                KICK => scaled(bombo(), v),
                SNARE => scaled(timbale(false, true), 0.5 * v),
                HAT => scaled(shell(), 0.3 * v),
                CRASH => scaled(synth::crash(1.2), 0.12 * v),
                _ => Vec::new(),
            },
            0.4,
            -0.2,
            (0, 0),
        ),
        layer(
            "congas",
            LayerKind::Perc,
            "perc",
            |_, m, _, v| match m {
                CLAP => scaled(conga(Conga::Slap, false), v),
                SHAKER if v > 0.6 => scaled(conga(Conga::Open, false), 0.8 * v),
                SHAKER => scaled(conga(Conga::Muff, true), 0.8 * v),
                _ => scaled(bell(true), 0.5 * v),
            },
            0.16,
            0.35,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |ctx, m, d, v| voice(ctx, "CONTRABASS", m, d.clamp(0.15, 0.45), v),
            0.6,
            0.0,
            (36, 48),
        ),
        layer(
            "piano",
            LayerKind::Comp,
            "piano",
            |ctx, m, d, v| voice(ctx, "PIANO", m, d.clamp(0.1, 0.3), v),
            0.2,
            0.22,
            (55, 72),
        ),
        layer(
            "montuno",
            LayerKind::Arp,
            "piano",
            |ctx, m, _, v| {
                let a = voice(ctx, "PIANO", m, 0.14, v);
                let b = voice(ctx, "PIANO", m + 12, 0.14, 0.6 * v);
                (0..a.len().max(b.len()))
                    .map(|i| a.get(i).copied().unwrap_or(0.0) + b.get(i).copied().unwrap_or(0.0))
                    .collect()
            },
            0.16,
            0.22,
            (62, 76),
        ),
        layer(
            "horns",
            LayerKind::Pad,
            "sax",
            |ctx, m, d, v| voice(ctx, ALTO, m, d, 0.55 * v),
            0.16,
            -0.05,
            (57, 70),
        ),
    ]
}

/// The Composed bed for the fight's `options`, its story `cues` and the music `seed`.
///
/// # Errors
/// The classic bed refuses the options.
pub fn composed(options: &ThemeOptions, cues: Vec<Cue>, seed: u64) -> Result<LayeredBed, ContentError> {
    Ok(LayeredBed::new((THEME.classic)(options)?, layers(), TABLE, cues, seed)?)
}
