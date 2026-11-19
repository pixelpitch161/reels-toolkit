//! Theme: klezmer freylekh, a Studio theme (the batch of 2026-11-18; no kit mood behind it). A whirling wedding dance that spins ever faster: a wailing clarinet and a violin over an oom-pah accordion, a tuba and a frame drum.
//! Built for D harmonic minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "klezmer",
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
    &["Dm", "Dm", "A7", "A7", "Gm", "Dm", "A7", "Dm"],
    &["Dm", "Gm", "A7", "Dm"],
    &["Dm", "C", "Bb", "A7"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (120.0, 150.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Klezmer freylekh",
    text: "A whirling wedding dance that spins ever faster: a wailing clarinet and a violin over an oom-pah accordion, a tuba and a frame drum.",
    key: "D",
    scale: "harmonic minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/klezmer/1",
        fighters: ["CLARINET", "VIOLIN"],
        bpm: 136.0,
        key: "D",
        scale: "harmonic minor",
        intro: ["Dm", "Dm"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/klezmer/2",
        fighters: ["CLARINET", "VIOLIN"],
        bpm: 136.0,
        key: "D",
        scale: "harmonic minor",
        intro: ["Dm", "Dm"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/klezmer/3",
        fighters: ["CLARINET", "VIOLIN"],
        bpm: 136.0,
        key: "D",
        scale: "harmonic minor",
        intro: ["Dm", "A7"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("chords", 2), ("fiddle", 2), ("clarinet", 1)],
    fill: &["drums"],
    breakdown: &[("fiddle", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("chords", 3), ("fiddle", 3), ("clarinet", 2)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("fiddle", 3), ("clarinet", 1)],
    build: &[("drums", 3), ("bass", 3), ("chords", 3), ("fiddle", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("chords", 2), ("fiddle", 1), ("clarinet", 1)],
    &[("drums", 3), ("bass", 3), ("chords", 3), ("fiddle", 3), ("riser", 1)],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "drum",
            |_, m, _, v| match m {
                KICK => scaled(super::sea_shanty::frame_drum(true), v),
                SNARE => scaled(super::sea_shanty::frame_drum(false), 0.8 * v),
                HAT => scaled(super::big_top::tambourine(v, seed_of(v)), 0.4 * v),
                CRASH => scaled(synth::crash(1.0), 0.12 * v),
                _ => Vec::new(),
            },
            0.5,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "tuba",
            |_, m, d, v| {
                scaled(
                    super::villain_march::brass(&[m], d.clamp(0.12, 0.35), 0.01, 900.0, 0.6),
                    v,
                )
            },
            0.35,
            0.0,
            (33, 45),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "accordion",
            |_, m, d, v| {
                scaled(
                    super::balkan_fire::reed(
                        &[m],
                        d.clamp(0.08, 0.2),
                        0.005,
                        0.05,
                        2200.0,
                        super::balkan_fire::MUSETTE,
                    ),
                    v,
                )
            },
            0.14,
            -0.25,
            (55, 69),
        ),
        layer(
            "fiddle",
            LayerKind::Arp,
            "fiddle",
            |_, m, d, v| {
                scaled(
                    super::baroque_storm::strng(m, d.clamp(0.1, 0.3), 3000.0, 0.006, 0.02),
                    v,
                )
            },
            0.16,
            0.3,
            (67, 81),
        ),
        layer(
            "clarinet",
            LayerKind::Pad,
            "clarinet",
            |_, m, d, v| scaled(super::dark_tango::reed(m, d.min(1.5), 0.04, 0.3), v),
            0.07,
            0.1,
            (62, 74),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("klezmer", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("klezmer", layers, ROOM, TABLE, cues, seed)
}
