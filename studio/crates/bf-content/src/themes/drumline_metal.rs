//! Theme: drumline metal, a Studio theme (the batch of 2026-11-16; no kit mood behind it): a marching snare, quad toms and a pitched bass drum, palm-muted drop-D chugs, a driven bass and a low brass pad.
//! Built for D minor; any chord works (the layers voice the chord's tones in their ranges). Both
//! arrangements are layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of
//! the Composed one, with no story cues.

use bf_sound::music::layers::{ArrangementTable, CRASH, Cue, HAT, KICK, Layer, LayerKind, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled};
use super::dark_drumline::{bassdrum, msnare, tenor};
use super::fresh::{self, ROOM};
use super::prog_duel::bass_note;
use super::stadium_rock::pgtr;
use super::villain_march::horn_pad;
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "drumline_metal",
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
    &["Dm", "Dm", "Bb", "A", "Dm", "Dm", "Bb", "C"],
    &["Dm", "Bb", "C", "A"],
    &["Dm", "Gm", "Bb", "A"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (132.0, 160.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Drumline metal",
    text: "Heavy and military: a marching drumline cadence locked to drop-D guitar chugs, double kicks and low brass.",
    key: "D",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for (no kit config: a Studio theme).
pub static PRESETS: [Preset; 1] = [Preset {
    config: "studio/drumline_metal",
    fighters: ["TIMPANI", "BASS DRUM"],
    bpm: 144.0,
    key: "D",
    scale: "minor",
    intro: ["Dm", "C"],
    chord_loop: LOOPS[0],
    tail: &[],
}];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 3), ("quads", 1), ("bass", 3), ("chug", 3), ("chords", 1)],
    fill: &["drums", "quads"],
    breakdown: &[("drums", 1), ("quads", 3), ("chords", 1)],
    final_exchange: &[("drums", 3), ("quads", 3), ("bass", 3), ("chug", 3), ("chords", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("quads", 2), ("bass", 2), ("chords", 2)],
    build: &[
        ("drums", 3),
        ("quads", 3),
        ("bass", 3),
        ("chug", 3),
        ("chords", 2),
        ("riser", 1),
    ],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 3), ("bass", 3), ("chug", 3), ("chords", 1)],
    &[
        ("drums", 3),
        ("quads", 3),
        ("bass", 3),
        ("chug", 3),
        ("chords", 2),
        ("riser", 1),
    ],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "line",
            |_, m, _, v| match m {
                KICK => scaled(bassdrum(38), v),
                SNARE => scaled(msnare(if v > 0.6 { 'A' } else { 't' }), v),
                HAT => scaled(msnare('g'), 0.8 * v),
                CRASH => scaled(synth::crash(1.6), 0.3 * v),
                _ => Vec::new(),
            },
            0.42,
            0.0,
            (0, 0),
        ),
        layer(
            "quads",
            LayerKind::Arp,
            "line",
            |_, m, _, v| scaled(tenor(m.rem_euclid(4) as usize), v),
            0.2,
            0.15,
            (50, 62),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| bass_note(m, d.clamp(0.1, 0.4), v, 0.8),
            0.3,
            0.0,
            (26, 38),
        ),
        layer(
            "chug",
            LayerKind::Bass,
            "guitar",
            |_, m, d, v| scaled(pgtr(m, d.clamp(0.08, 0.3), true, false), v),
            0.6,
            -0.25,
            (38, 50),
        ),
        layer(
            "chords",
            LayerKind::Pad,
            "brass",
            |_, m, d, v| scaled(horn_pad(&[m], d.min(3.0)), v),
            0.08,
            0.2,
            (45, 60),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("drumline_metal", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("drumline_metal", layers, ROOM, TABLE, cues, seed)
}
