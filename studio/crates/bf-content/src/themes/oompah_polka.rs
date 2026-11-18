//! Theme: oompah polka, a Studio theme (the batch of 2026-11-18; no kit mood behind it). Cheeky, bouncy beer-hall fun: a tuba oom, an accordion pah, clarinet runs and a trumpet tune over a snare.
//! Built for Bb major; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "oompah_polka",
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
    &["Bb", "Bb", "F7", "F7", "F7", "F7", "Bb", "Bb"],
    &["Bb", "Eb", "F7", "Bb"],
    &["Bb", "F7", "Bb", "F7"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (116.0, 140.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Oompah polka",
    text: "Cheeky, bouncy beer-hall fun: a tuba oom, an accordion pah, clarinet runs and a trumpet tune over a snare.",
    key: "Bb",
    scale: "major",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/oompah_polka/1",
        fighters: ["TUBA", "ACCORDION"],
        bpm: 128.0,
        key: "Bb",
        scale: "major",
        intro: ["Bb", "Bb"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/oompah_polka/2",
        fighters: ["TUBA", "ACCORDION"],
        bpm: 128.0,
        key: "Bb",
        scale: "major",
        intro: ["Bb", "Bb"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/oompah_polka/3",
        fighters: ["TUBA", "ACCORDION"],
        bpm: 128.0,
        key: "Bb",
        scale: "major",
        intro: ["Bb", "F7"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[
        ("drums", 2),
        ("bass", 2),
        ("chords", 2),
        ("clarinet", 2),
        ("trumpet", 1),
    ],
    fill: &["drums"],
    breakdown: &[("clarinet", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[
        ("drums", 3),
        ("bass", 3),
        ("chords", 3),
        ("clarinet", 3),
        ("trumpet", 2),
    ],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("clarinet", 3), ("trumpet", 1)],
    build: &[("drums", 3), ("bass", 3), ("chords", 3), ("clarinet", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[
        ("drums", 2),
        ("bass", 2),
        ("chords", 2),
        ("clarinet", 1),
        ("trumpet", 1),
    ],
    &[("drums", 3), ("bass", 3), ("chords", 3), ("clarinet", 3), ("riser", 1)],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "drums",
            |_, m, _, v| match m {
                KICK => scaled(super::big_top::bass_drum(v), 1.0),
                SNARE => scaled(super::big_top::snare(v), 0.8),
                HAT => scaled(super::big_top::tambourine(v, seed_of(v)), 0.3 * v),
                CRASH => scaled(synth::crash(1.2), 0.2 * v),
                _ => Vec::new(),
            },
            0.45,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "tuba",
            |_, m, d, v| {
                scaled(
                    super::villain_march::brass(&[m], d.clamp(0.12, 0.3), 0.01, 800.0, 0.5),
                    v,
                )
            },
            0.4,
            0.0,
            (34, 46),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "accordion",
            |_, m, d, v| {
                scaled(
                    super::balkan_fire::reed(
                        &[m],
                        d.clamp(0.08, 0.18),
                        0.005,
                        0.04,
                        2400.0,
                        super::balkan_fire::MUSETTE,
                    ),
                    v,
                )
            },
            0.15,
            -0.25,
            (55, 69),
        ),
        layer(
            "clarinet",
            LayerKind::Arp,
            "clarinet",
            |_, m, d, v| scaled(super::dark_tango::reed(m, d.clamp(0.1, 0.25), 0.01, 0.2), v),
            0.12,
            0.3,
            (67, 82),
        ),
        layer(
            "trumpet",
            LayerKind::Pad,
            "trumpet",
            |_, m, d, v| scaled(super::villain_march::brass(&[m], d.min(1.5), 0.02, 2800.0, 0.9), v),
            0.05,
            0.15,
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
    fresh::bed("oompah_polka", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("oompah_polka", layers, ROOM, TABLE, cues, seed)
}
