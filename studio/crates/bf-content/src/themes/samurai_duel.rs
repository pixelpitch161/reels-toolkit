//! Theme: samurai duel, a Studio theme (the batch of 2026-11-18; no kit mood behind it). A tense standoff, then flashing steel: a breathy bamboo flute, koto plucks, taiko hits and swelling strings.
//! Built for A minor; any chord works (the layers voice the chord's tones in their ranges). Both arrangements are
//! layered beds over the Studio themes' [frame](super::fresh): the Classic one a plain cut of the Composed one, with
//! no story cues.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
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
    name: "samurai_duel",
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
    &["Am", "Am", "F", "G", "Am", "Am", "Dm", "E"],
    &["Am", "F", "G", "Am"],
    &["Am", "Dm", "Am", "E"],
];

/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (104.0, 132.0);

/// What the interface says about the theme.
pub static CARD: ThemeCard = ThemeCard {
    display: "Samurai duel",
    text: "A tense standoff, then flashing steel: a breathy bamboo flute, koto plucks, taiko hits and swelling strings.",
    key: "A",
    scale: "minor",
    chord_loops: &LOOPS,
    presets: &PRESETS,
    bars: (24, 42),
};

/// The pair the theme was made for, once on each of its loops (no kit config: a Studio theme).
pub static PRESETS: [Preset; 3] = [
    Preset {
        config: "studio/samurai_duel/1",
        fighters: ["FLUTE", "HARP"],
        bpm: 118.0,
        key: "A",
        scale: "minor",
        intro: ["Am", "E"],
        chord_loop: LOOPS[0],
        tail: &[],
    },
    Preset {
        config: "studio/samurai_duel/2",
        fighters: ["FLUTE", "HARP"],
        bpm: 118.0,
        key: "A",
        scale: "minor",
        intro: ["Am", "Am"],
        chord_loop: LOOPS[1],
        tail: &[],
    },
    Preset {
        config: "studio/samurai_duel/3",
        fighters: ["FLUTE", "HARP"],
        bpm: 118.0,
        key: "A",
        scale: "minor",
        intro: ["Am", "E"],
        chord_loop: LOOPS[2],
        tail: &[],
    },
];

/// The Composed arrangement's table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("chords", 2), ("flute", 2), ("strings", 1)],
    fill: &["drums"],
    breakdown: &[("flute", 2), ("bass", 1), ("chords", 1)],
    final_exchange: &[("drums", 3), ("bass", 3), ("chords", 3), ("flute", 3), ("strings", 2)],
    ability: &["chords", "drums"],
    victory: &[("drums", 2), ("bass", 2), ("flute", 3), ("strings", 1)],
    build: &[("drums", 3), ("bass", 3), ("chords", 3), ("flute", 3), ("riser", 1)],
};

/// The Classic arrangement's table: the band at one steady level.
pub const PLAIN: ArrangementTable = fresh::plain(
    &[("drums", 2), ("bass", 2), ("chords", 2), ("flute", 1), ("strings", 1)],
    &[("drums", 3), ("bass", 3), ("chords", 3), ("flute", 3), ("riser", 1)],
);

/// The samurai duel's own band (sound upgrade, stage 2): the taiko's two-bar call (don on 1 and 3, the small drum
/// answering, the rim's ka between) with a flurry of ghost strokes at the top, the low strings held through the
/// standoff and driving spiccato eighths when steel flashes, the koto's plucked two-bar figure, and the bamboo
/// flute's long breaths turning to running eighths.
pub static GROOVE: Groove = Groove {
    push_ms: 6.0,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(KICK), "X.......X.......X.....x.X......."),
                line(2, Play::Drum(SNARE), "....x.......x.x.....x.......xx.x"),
                line(2, Play::Drum(HAT), "..x...x...x....."),
                only(3, Play::Drum(KICK), "..........x....."),
                only(3, Play::Drum(SNARE), ".g.g.g.g.g.g....").maybe(0.5),
            ],
            fill: &[
                line(1, Play::Drum(KICK), "X..X..XX"),
                line(1, Play::Drum(SNARE), ".xx.xx.."),
            ],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(1, Play::Root, "X_______________"),
                only(2, Play::Root, "X.......X..x...."),
                only(2, Play::Fifth, "..............x."),
                only(3, Play::Root, "X.x.x.x.X.x.x.x."),
                only(3, Play::Fifth, ".......g.......g").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "chords",
            lines: &[
                upto(1, Play::Tone(0), "x..............................."),
                upto(1, Play::Tone(1), "................x..............."),
                upto(1, Play::Tone(2), "........x......................."),
                upto(1, Play::Tone(4), "........................x......."),
                line(2, Play::Tone(0), "x...............x..............."),
                line(2, Play::Tone(2), "...x........o......x..........o."),
                line(2, Play::Tone(3), "........x...................o..."),
                line(2, Play::Tone(4), "......x...............x...o....."),
                line(2, Play::Tone(5), "........................x......."),
                only(3, Play::Tone(1), ".o...o.....o...o").maybe(0.7),
            ],
            fill: &[],
        },
        Part {
            layer: "flute",
            lines: &[
                upto(2, Play::Tone(0), "............o___................"),
                upto(2, Play::Tone(1), "........x___............x_______"),
                upto(2, Play::Tone(2), "x_______..............o_........"),
                upto(2, Play::Tone(3), "................x_____.........."),
                only(3, Play::Tone(0), "....x.......................x___"),
                only(3, Play::Tone(1), "..o...o...............o...o....."),
                only(3, Play::Tone(2), "x.......x...........x...x......."),
                only(3, Play::Tone(3), "..........o...o...o............."),
                only(3, Play::Tone(4), "............x...x..............."),
            ],
            fill: &[],
        },
    ],
    mix: &[("taiko", -2.5), ("koto", 8.0), ("low", 5.0), ("strings", 3.5)],
    lows: &["taiko"],
    ..STRAIGHT
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "taiko",
            |_, m, _, v| match m {
                KICK => scaled(super::brass_trailer::taiko(true, seed_of(v)), v),
                SNARE => scaled(super::brass_trailer::taiko(false, seed_of(v)), 0.7 * v),
                HAT => scaled(super::brass_trailer::tick(seed_of(v)), 0.4 * v),
                CRASH => scaled(synth::crash(2.0), 0.15 * v),
                _ => Vec::new(),
            },
            0.45,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "low",
            |_, m, d, v| super::brass_trailer::strings_low(m, d.min(1.5), v),
            0.4,
            0.0,
            (33, 45),
        ),
        layer(
            "chords",
            LayerKind::Comp,
            "koto",
            |_, m, d, v| {
                scaled(
                    super::bossa_noir::pluck(m, d.clamp(0.1, 0.3), i64::from(m % 4), 0.9, 0.99),
                    v,
                )
            },
            0.22,
            -0.25,
            (57, 72),
        ),
        layer(
            "flute",
            LayerKind::Arp,
            "flute",
            |_, m, d, v| super::fresh::voices::breath_flute(m, d.clamp(0.2, 0.8), v),
            0.12,
            0.25,
            (69, 84),
        ),
        layer(
            "strings",
            LayerKind::Pad,
            "strings",
            |_, m, d, v| scaled(super::brass_trailer::tremolo(&[m], d.min(3.0), 0.3), v),
            0.05,
            0.1,
            (57, 69),
        ),
        riser(),
    ]
}

/// The Classic bed.
///
/// # Errors
/// None in practice (the table names only the theme's layers).
pub fn classic() -> Result<Box<dyn Bed>, ContentError> {
    fresh::bed("samurai_duel", layers, ROOM, PLAIN, Vec::new(), 0)
}

/// The Composed bed for the fight's story `cues` and the music `seed`.
///
/// # Errors
/// As [`classic`].
pub fn composed(cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    fresh::grooved("samurai_duel", layers, ROOM, TABLE, cues, seed, &GROOVE)
}
