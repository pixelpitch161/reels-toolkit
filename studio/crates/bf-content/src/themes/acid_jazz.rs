//! Theme: acid jazz groove, a Studio theme (the batch of 2026-11-18; no kit mood behind it). Cool, funky and head-nodding: jazz horns and a Rhodes over a breakbeat, a wah guitar and a round bass.
//! Built for D dorian; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
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
    name: "acid_jazz",
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
    &["Dm7", "G7", "Dm7", "G7", "Dm7", "G7", "Em7", "A7"],
    &["Dm7", "G7", "Dm7", "G7"],
    &["Dm7", "Em7", "Fmaj7", "G7"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (104.0, 122.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Acid jazz groove",
    text: "Cool, funky and head-nodding: jazz horns and a Rhodes over a breakbeat, a wah guitar and a round bass.",
    key: "D",
    scale: "dorian",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/acid_jazz/1",
        fighters: ["FLUGELHORN", "TENOR SAX"],
        bpm: 112.0,
        key: "D",
        scale: "dorian",
        intro: ["Dm7", "A7"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/acid_jazz/2",
        fighters: ["FLUGELHORN", "TENOR SAX"],
        bpm: 112.0,
        key: "D",
        scale: "dorian",
        intro: ["Dm7", "G7"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/acid_jazz/3",
        fighters: ["FLUGELHORN", "TENOR SAX"],
        bpm: 112.0,
        key: "D",
        scale: "dorian",
        intro: ["Dm7", "G7"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("chords", 2), ("wah", 2), ("horns", 1)],
    fill: &["drums"],
    breakdown: &[("wah", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("chords", 3), ("wah", 3), ("horns", 2)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("wah", 3), ("horns", 1)],
    build: &[("drums", 3), ("bass", 3), ("chords", 3), ("wah", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("chords", 2), ("wah", 1), ("horns", 1)],
    &[("drums", 3), ("bass", 3), ("chords", 3), ("wah", 3), ("riser", 1)],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "breaks",
            |_, m, _, v| match m {
                KICK => super::dark_breakbeat::kick(v),
                SNARE => scaled(super::dark_breakbeat::snare_hit(), 0.7 * v),
                HAT => super::brass_house::hat(0.45 * v, false, seed_of(v)),
                CRASH => scaled(synth::crash(1.4), 0.15 * v),
                _ => Vec::new(),
            },
            0.42,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| super::fresh::voices::slap(m, d.clamp(0.1, 0.4), v),
            0.4,
            0.0,
            (33, 45),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "rhodes",
            |_, m, d, v| super::fresh::voices::rhodes(m, d.clamp(0.15, 0.6), v),
            0.22,
            -0.2,
            (55, 69),
        ),
        layer(
            "wah",
            LayerKind::Arp,
            "guitar",
            |_, m, d, v| scaled(super::stadium_rock::pgtr(m, d.clamp(0.06, 0.15), true, false), v),
            0.18,
            0.3,
            (52, 64),
        ),
        layer(
            "horns",
            LayerKind::Pad,
            "horns",
            |_, m, d, v| scaled(super::villain_march::horn_pad(&[m], d.min(3.0)), v),
            0.05,
            0.1,
            (55, 67),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("acid_jazz", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("acid_jazz", layers, ROOM, TABLE, cues, seed)
}
