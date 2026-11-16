//! Theme: samba bateria, a Studio theme (the batch of 2026-11-16; no kit mood behind it): a surdo pair, a caixa snare, tamborims rattling the sixteenths, agogô bells, a cavaquinho chopping the chord and a bass.
//! Built for A harmonic minor; any chord works (the layers voice the chord's tones in their ranges). Both
//! arrangements are layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of
//! the Composed one, with no story cues.

use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, SHAKER, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled, seed_of};
use super::bossa_noir::{pluck, shaker};
use super::dark_drumline::{bassdrum, msnare};
use super::fresh::{self, ROOM};
use super::heist::upright;
use super::mambo::{bell, timbale};
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "samba_bateria",
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
    &["Am", "Am", "Dm", "E7", "Am", "F", "E7", "E7"],
    &["Am", "Dm", "E7", "Am"],
    &["Am", "F", "E7", "Am"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (98.0, 112.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Samba bateria",
    text: "Carnival percussion in full roar: surdos on the downbeat, a rattling caixa and tamborims, agogô bells and a cavaquinho.",
    key: "A",
    scale: "harmonic minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for (no kit config: a Studio theme).
pub static PRESETS: [Preset; 1] = [Preset {
    config: "studio/samba_bateria",
    fighters: ["MARIMBA", "SNARE DRUM"],
    bpm: 104.0,
    key: "A",
    scale: "harmonic minor",
    intro: ["Am", "E7"],
    chord_loop: LOOPS[0],
    tail: &[],
}];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 3), ("tamborim", 2), ("agogo", 1), ("bass", 2), ("chords", 2)],
    fill: &["drums", "agogo"],
    breakdown: &[("tamborim", 3), ("agogo", 2)],
    final_exchange: &[("drums", 3), ("tamborim", 3), ("agogo", 3), ("bass", 3), ("chords", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("tamborim", 2), ("agogo", 2), ("bass", 2), ("chords", 2)],
    build: &[
        ("drums", 3),
        ("tamborim", 3),
        ("agogo", 3),
        ("bass", 3),
        ("chords", 3),
        ("riser", 1),
    ],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 3), ("tamborim", 2), ("bass", 2), ("chords", 2)],
    &[
        ("drums", 3),
        ("tamborim", 3),
        ("agogo", 3),
        ("bass", 3),
        ("chords", 3),
        ("riser", 1),
    ],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "bateria",
            |_, m, _, v| match m {
                KICK => scaled(bassdrum(if v > 0.87 { 33 } else { 40 }), v),
                SNARE => scaled(msnare(if v > 0.6 { 'A' } else { 'g' }), v),
                HAT => scaled(shaker(seed_of(v), v > 0.5), 0.7 * v),
                CRASH => scaled(synth::crash(1.0), 0.2 * v),
                _ => Vec::new(),
            },
            0.42,
            0.0,
            (0, 0),
        ),
        layer(
            "tamborim",
            LayerKind::Arp,
            "bateria",
            |_, m, _, v| scaled(timbale(m % 2 == 0, true), v),
            0.12,
            0.25,
            (60, 72),
        ),
        layer(
            "agogo",
            LayerKind::Perc,
            "perc",
            |_, m, _, v| match m {
                CLAP => scaled(bell(true), v),
                SHAKER => scaled(bell(false), 0.6 * v),
                _ => Vec::new(),
            },
            0.5,
            -0.25,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| scaled(upright(m, d.min(0.45)), v),
            0.55,
            0.0,
            (33, 45),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "cavaco",
            |_, m, d, v| scaled(pluck(m, d.clamp(0.08, 0.2), i64::from(m % 4), 0.85, 0.99), v),
            0.3,
            -0.2,
            (62, 76),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("samba_bateria", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("samba_bateria", layers, ROOM, TABLE, cues, seed)
}
