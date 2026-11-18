//! Theme: glam stomp, a Studio theme (the batch of 2026-11-18; no kit mood behind it). Stomp, stomp, clap: a glittering stadium chant of fuzz guitar, pounded piano and big floor toms.
//! Built for D major; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
//! layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of the Composed one, with
//! no story cues.

use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, SHAKER, SNARE};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::balkan_fire::layers::{layer, riser, scaled, seed_of};
use super::fresh::{self, ROOM};
use super::{Preset, Theme};
use crate::cards::ThemeCard;
use crate::error::ContentError;

/// The theme as the registry lists it: no options, no voices of its own.
pub static THEME: Theme = Theme {
    name: "glam_stomp",
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
    &["D", "D", "G", "D", "D", "D", "A", "A"],
    &["D", "G", "A", "D"],
    &["D", "C", "G", "D"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (118.0, 138.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Glam stomp",
    text: "Stomp, stomp, clap: a glittering stadium chant of fuzz guitar, pounded piano and big floor toms.",
    key: "D",
    scale: "major",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/glam_stomp/1",
        fighters: ["DRUMS", "ELECTRIC"],
        bpm: 128.0,
        key: "D",
        scale: "major",
        intro: ["D", "A"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/glam_stomp/2",
        fighters: ["DRUMS", "ELECTRIC"],
        bpm: 128.0,
        key: "D",
        scale: "major",
        intro: ["D", "D"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/glam_stomp/3",
        fighters: ["DRUMS", "ELECTRIC"],
        bpm: 128.0,
        key: "D",
        scale: "major",
        intro: ["D", "D"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("toms", 1), ("bass", 2), ("chords", 2), ("piano", 2)],
    fill: &["drums", "toms"],
    breakdown: &[("piano", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("toms", 3), ("bass", 3), ("chords", 3), ("piano", 3)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("toms", 2), ("bass", 2), ("piano", 3)],
    build: &[
        ("drums", 3),
        ("toms", 3),
        ("bass", 3),
        ("chords", 3),
        ("piano", 3),
        ("riser", 1),
    ],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("toms", 1), ("bass", 2), ("chords", 2), ("piano", 1)],
    &[
        ("drums", 3),
        ("toms", 3),
        ("bass", 3),
        ("chords", 3),
        ("piano", 3),
        ("riser", 1),
    ],
);

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "kit",
            |_, m, _, v| match m {
                KICK => scaled(super::sea_shanty::stomp(v, seed_of(v)), 1.0),
                SNARE => scaled(super::sea_shanty::gang_clap_hands(), 0.8 * v),
                HAT => scaled(synth::hat(0.05, false), 0.35 * v),
                CRASH => scaled(synth::crash(1.6), 0.3 * v),
                _ => Vec::new(),
            },
            0.5,
            0.0,
            (0, 0),
        ),
        layer(
            "toms",
            LayerKind::Perc,
            "kit",
            |_, m, _, v| match m {
                CLAP => scaled(super::stadium_rock::tom(43), v),
                SHAKER => scaled(super::stadium_rock::tom(48), 0.6 * v),
                _ => Vec::new(),
            },
            0.3,
            -0.15,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| scaled(super::stadium_rock::pbass(m, d.clamp(0.12, 0.5)), v),
            0.5,
            0.0,
            (38, 50),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "guitar",
            |_, m, d, v| scaled(super::stadium_rock::pgtr(m, d.clamp(0.1, 0.4), false, true), v),
            0.22,
            -0.25,
            (40, 52),
        ),
        layer(
            "piano",
            LayerKind::Arp,
            "piano",
            |_, m, d, v| super::fresh::voices::piano(m, d.clamp(0.1, 0.3), v),
            0.2,
            0.25,
            (57, 72),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("glam_stomp", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("glam_stomp", layers, ROOM, TABLE, cues, seed)
}
