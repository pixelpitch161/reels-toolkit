//! Theme: amapiano log drum, a Studio theme (the batch of 2026-11-18; no kit mood behind it). Laid-back, deep and groovy: a bouncing log-drum bass, shakers, jazzy piano chords, airy pads and a soft flute line.
//! Built for F minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
//! layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of the Composed one, with
//! no story cues.

use bf_sound::music::layers::{ArrangementTable, CRASH, Cue, HAT, KICK, Layer, LayerKind, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled, seed_of};
use super::fresh::{self, ROOM};
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "amapiano",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| classic(),
    composed: Some(|_, c, s| composed(c, s)),
};

/// The chord loops, the first one the preset's.
pub static LOOPS: [&[&str]; 3] = [
    &["Fm7", "Fm7", "Bbm7", "Bbm7", "Dbmaj7", "Dbmaj7", "C7", "C7"],
    &["Fm7", "Bbm7", "Dbmaj7", "C7"],
    &["Fm7", "Ebmaj7", "Dbmaj7", "C7"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (108.0, 116.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Amapiano log drum",
    text: "Laid-back, deep and groovy: a bouncing log-drum bass, shakers, jazzy piano chords, airy pads and a soft flute line.",
    key: "F",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/amapiano/1",
        fighters: ["PIANO", "MARIMBA"],
        bpm: 112.0,
        key: "F",
        scale: "minor",
        intro: ["Fm7", "C7"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/amapiano/2",
        fighters: ["PIANO", "MARIMBA"],
        bpm: 112.0,
        key: "F",
        scale: "minor",
        intro: ["Fm7", "C7"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/amapiano/3",
        fighters: ["PIANO", "MARIMBA"],
        bpm: 112.0,
        key: "F",
        scale: "minor",
        intro: ["Fm7", "C7"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("chords", 2), ("pad", 1), ("flute", 2)],
    fill: &["drums"],
    breakdown: &[("flute", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("chords", 3), ("pad", 2), ("flute", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("pad", 1), ("flute", 3)],
    build: &[("drums", 3), ("bass", 3), ("chords", 3), ("flute", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("chords", 2), ("pad", 1), ("flute", 1)],
    &[("drums", 3), ("bass", 3), ("chords", 3), ("flute", 3), ("riser", 1)],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "kit",
            |_, m, _, v| match m {
                KICK => super::brass_house::kick(0.8 * v),
                SNARE => scaled(super::orchestral_drill::rim(v, seed_of(v)), 0.6),
                HAT => super::brass_house::shaker(0.6 * v, seed_of(v)),
                CRASH => scaled(synth::crash(1.4), 0.12 * v),
                _ => Vec::new(),
            },
            0.42,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "logdrum",
            |_, m, d, v| super::fresh::voices::log_drum(m, d, v),
            0.45,
            0.0,
            (33, 45),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "piano",
            |_, m, d, v| super::fresh::voices::piano(m, d.clamp(0.15, 0.5), v),
            0.2,
            -0.2,
            (55, 70),
        ),
        layer(
            "pad",
            LayerKind::Pad,
            "pad",
            |_, m, d, v| scaled(super::dark_breakbeat::cold_pad(&[m], d, 2400.0), v),
            0.1,
            0.1,
            (55, 67),
        ),
        layer(
            "flute",
            LayerKind::Arp,
            "flute",
            |_, m, d, v| super::fresh::voices::breath_flute(m, d.clamp(0.15, 0.5), v),
            0.08,
            0.3,
            (72, 84),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("amapiano", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("amapiano", layers, ROOM, TABLE, cues, seed)
}
