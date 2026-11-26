//! The dark drumline's Composed arrangement (milestone 5): the marching snares (accents on 2 and 4, taps on the
//! eighths), the tuned bass drums on the chord, the quads running it, the cymbal crunch, and the pit (marimba and
//! timpani) under them.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, LayeredBed, SNARE};

use super::{THEME, bassdrum, choke, marimba, msnare, tenor, timp};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, riser, scaled, seed_of};

/// The home key and scale.
pub const KEY: (&str, &str) = ("C", "minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [
    &["Cm", "Ab", "Fm", "G"],
    &["Cm", "Fm", "Ab", "G"],
    &["Cm", "Eb", "Ab", "G"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (116.0, 136.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("snares", 2), ("basses", 2), ("pit", 1), ("timp", 1)],
    fill: &["snares", "quads"],
    breakdown: &[("timp", 1), ("pit", 1), ("cymbals", 1)],
    final_exchange: &[
        ("snares", 3),
        ("basses", 3),
        ("unison", 1),
        ("quads", 2),
        ("pit", 2),
        ("timp", 2),
        ("cymbals", 2),
    ],
    ability: &["snares", "timp"],
    victory: &[
        ("snares", 3),
        ("basses", 3),
        ("unison", 1),
        ("quads", 1),
        ("pit", 3),
        ("timp", 2),
        ("cymbals", 1),
    ],
    build: &[
        ("snares", 3),
        ("basses", 3),
        ("unison", 1),
        ("quads", 3),
        ("pit", 3),
        ("timp", 2),
        ("riser", 1),
    ],
};

/// The dark drumline's own band (sound upgrade, stage 2): a battle cadence, the snares' accents on 2 and 4 over
/// taps on the eighths, a two-bar accent figure over sixteenth taps and ghosts at the top and diddled fills, the
/// bass drums splitting the chord (root on 1 and 3, the fifth and the third between), the quads sweeping down the
/// drums, the cymbal crunch on the downbeat, the marimba turning round the chord and the timpani on 1 and 3.
pub static GROOVE: Groove = Groove {
    push_ms: 3.0,
    vel_spread: 0.08,
    parts: &[
        Part {
            layer: "snares",
            lines: &[
                line(1, Play::Drum(KICK), "x.......x......."),
                upto(2, Play::Drum(SNARE), "....X.......X..."),
                upto(2, Play::Drum(HAT), "x.x.x.xgx.x.x.xg"),
                only(3, Play::Drum(SNARE), "....X..X....X...X...X.X...X..XX."),
                only(3, Play::Drum(HAT), "xgxg.gxgxgxg.gxg"),
            ],
            fill: &[
                line(1, Play::Drum(SNARE), "xxXxxXxX"),
                line(1, Play::Drum(KICK), "X......."),
            ],
        },
        Part {
            layer: "cymbals",
            lines: &[
                upto(1, Play::Drum(CLAP), "X..............."),
                line(2, Play::Drum(CLAP), "X.......x...x..."),
                line(2, Play::Drum(HAT), "..g...g...g...g.").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "basses",
            lines: &[
                line(1, Play::Root, "x.......x......."),
                line(2, Play::Fifth, "...x.......x...."),
                line(2, Play::Third, "......x.......x."),
                only(3, Play::Octave, ".x...x...x...x..").maybe(0.7),
            ],
            fill: &[],
        },
        Part {
            layer: "quads",
            lines: &[
                line(1, Play::Tone(0), "x.......x......."),
                line(1, Play::Tone(2), "....x.......x..."),
                line(2, Play::Tone(1), "..x...x...x...x."),
                only(3, Play::Tone(3), ".o.o.o.o.o.o.o.o"),
            ],
            fill: &[
                line(1, Play::Tone(3), "x...x..."),
                line(1, Play::Tone(2), ".x...x.."),
                line(1, Play::Tone(1), "..x...x."),
                line(1, Play::Tone(0), "...x...X"),
            ],
        },
        Part {
            layer: "pit",
            lines: &[
                line(1, Play::Tone(0), "x.......x......."),
                line(1, Play::Tone(2), "....x.......x..."),
                line(2, Play::Tone(1), "..x.......x.....x.x.......x....."),
                line(2, Play::Tone(3), "......x.......x.......x.......x."),
                only(3, Play::Tone(4), ".o.o.o.o.o.o.o.o").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "timp",
            lines: &[
                line(1, Play::Root, "x_.............."),
                line(1, Play::Fifth, "........x_......"),
                line(2, Play::Root, "............o.o.").maybe(0.6),
            ],
            fill: &[],
        },
    ],
    ..STRAIGHT
};

/// The layers, on the classic bed's instruments and buses.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "snares",
            LayerKind::Drums,
            "drums",
            |_, m, _, v| match m {
                SNARE => scaled(msnare('A'), v),
                HAT => scaled(msnare(if v > 0.5 { 't' } else { 'g' }), v),
                KICK => scaled(bassdrum(60), 0.5 * v),
                CRASH => scaled(choke(seed_of(v)), v),
                _ => Vec::new(),
            },
            0.42,
            0.05,
            (0, 0),
        ),
        layer(
            "cymbals",
            LayerKind::Perc,
            "drums",
            |_, m, _, v| match m {
                CLAP => scaled(choke(seed_of(v)), v),
                _ => scaled(msnare('g'), 0.6 * v),
            },
            0.168,
            0.0,
            (0, 0),
        ),
        layer(
            "basses",
            LayerKind::Bass,
            "drums",
            |_, m, _, v| scaled(bassdrum(m + 24), v),
            0.252,
            -0.15,
            (36, 48),
        ),
        layer(
            "quads",
            LayerKind::Arp,
            "drums",
            |_, m, _, v| scaled(tenor(m.rem_euclid(4) as usize), v),
            0.1,
            0.2,
            (60, 72),
        ),
        layer(
            "pit",
            LayerKind::Arp,
            "pit",
            |_, m, _, v| scaled(marimba(m), v),
            0.14,
            0.3,
            (60, 79),
        ),
        layer(
            "timp",
            LayerKind::Bass,
            "pit",
            |_, m, _, v| scaled(timp(m), v),
            0.26,
            0.0,
            (36, 48),
        ),
        // the bass drums in unison on every beat (an arpeggio at density 1 is one stroke a beat)
        layer(
            "unison",
            LayerKind::Arp,
            "drums",
            |_, m, _, _| bassdrum(m + 24),
            0.22,
            0.0,
            (36, 48),
        ),
        riser(),
    ]
}

/// The Composed bed for the fight's `options`, its story `cues` and the music `seed`.
///
/// # Errors
/// The classic bed refuses the options.
pub fn composed(options: &ThemeOptions, cues: Vec<Cue>, seed: u64) -> Result<LayeredBed, ContentError> {
    Ok(LayeredBed::new((THEME.classic)(options)?, layers(), TABLE, cues, seed)?.with_groove(&GROOVE)?)
}
