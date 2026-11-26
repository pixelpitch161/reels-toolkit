//! The baroque storm's Composed arrangement (milestone 5): a bowed continuo under harpsichord arpeggios, string
//! chops and a held string chord, the timpani and the punch keeping the pulse; the harpsichord runs in sixteenths
//! when the storm breaks.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{ArrangementTable, CRASH, Cue, KICK, Layer, LayerKind, LayeredBed, SNARE};
use bf_sound::synth;

use super::{STRNG_ATT, STRNG_VIB, THEME, harpsi, punch, strng, timp};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, riser, scaled};

/// The home key and scale.
pub const KEY: (&str, &str) = ("D", "minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [
    &["Dm", "Gm", "C", "F", "Bb", "Gm", "A", "A"],
    &["Dm", "A", "Bb", "F", "Gm", "Dm", "A", "A"],
    &["Dm", "C", "Bb", "A"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (126.0, 150.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("continuo", 2), ("harpsi", 2), ("chops", 1), ("held", 1), ("drums", 1)],
    fill: &["drums", "harpsi"],
    breakdown: &[("continuo", 1), ("held", 2), ("harpsi", 1)],
    final_exchange: &[
        ("continuo", 3),
        ("harpsi", 3),
        ("chops", 2),
        ("held", 2),
        ("drums", 2),
        ("timpani", 3),
    ],
    ability: &["chops", "drums"],
    victory: &[
        ("continuo", 3),
        ("harpsi", 3),
        ("chops", 2),
        ("held", 2),
        ("drums", 2),
        ("timpani", 3),
    ],
    build: &[
        ("continuo", 3),
        ("harpsi", 3),
        ("chops", 3),
        ("held", 2),
        ("drums", 2),
        ("timpani", 3),
        ("riser", 1),
    ],
};

/// The baroque storm's own band (sound upgrade, stage 2): the continuo in half notes, then walking eighths, then
/// the storm's repeated sixteenths; the harpsichord's broken chords in quarters, eighths and running sixteenths
/// (a run up the chord into a lead change); the string chops on the off-beats and then the eighths; the punch on 1
/// and 3 under the timpani's thunder, rolling on the root and the fifth when everything is in.
pub static GROOVE: Groove = Groove {
    push_ms: 4.0,
    vel_spread: 0.08,
    parts: &[
        Part {
            layer: "continuo",
            lines: &[
                upto(1, Play::Root, "X_______x_______"),
                only(2, Play::Root, "X.x.....X.x....."),
                only(2, Play::Octave, "....x.x........."),
                only(2, Play::Fifth, "............x.x."),
                only(3, Play::Root, "Xxxx....Xxxx...."),
                only(3, Play::Fifth, "....xxxx........"),
                only(3, Play::Octave, "............xxxx"),
            ],
            fill: &[],
        },
        Part {
            layer: "harpsi",
            lines: &[
                upto(1, Play::Tone(0), "x..............."),
                upto(1, Play::Tone(1), "........x......."),
                upto(1, Play::Tone(2), "....o.......o..."),
                only(2, Play::Tone(0), "X...............X..............."),
                only(2, Play::Tone(1), "..o...........o.....x.........o."),
                only(2, Play::Tone(2), "....x.......x.....o.....x......."),
                only(2, Play::Tone(3), "......o...o...........o.....x..."),
                only(2, Play::Tone(4), "........x.................o....."),
                only(3, Play::Tone(0), "X.......X......................."),
                only(3, Play::Tone(1), ".o.....o.o.....oX.......X......."),
                only(3, Play::Tone(2), "..x...x...x...x..o.....o.o.....o"),
                only(3, Play::Tone(3), "...o.o.....o.o....x...x...x...x."),
                only(3, Play::Tone(4), "....x.......x......o.o.....o.o.."),
                only(3, Play::Tone(5), "....................x.......x..."),
            ],
            fill: &[
                line(1, Play::Tone(0), "x......."),
                line(1, Play::Tone(1), ".x......"),
                line(1, Play::Tone(2), "..x....."),
                line(1, Play::Tone(3), "...x...."),
                line(1, Play::Tone(4), "....x..."),
                line(1, Play::Tone(5), ".....x.."),
                line(1, Play::Tone(6), "......x."),
                line(1, Play::Tone(7), ".......X"),
            ],
        },
        Part {
            layer: "chops",
            lines: &[
                upto(1, Play::Chord, "..x...x...x...x."),
                line(2, Play::Chord, "X.x.x.x.X.x.x.x."),
                only(3, Play::Chord, ".g.g.g.g.g.g.g.g").maybe(0.5),
            ],
            fill: &[],
        },
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(KICK), "X.......X......."),
                upto(1, Play::Drum(SNARE), "............x..."),
                line(2, Play::Drum(SNARE), "....x.......x.xx"),
                line(2, Play::Drum(KICK), "......x........."),
            ],
            fill: &[],
        },
        Part {
            layer: "timpani",
            lines: &[
                line(1, Play::Root, "X.......X......."),
                line(1, Play::Fifth, "......x.....x..."),
                only(3, Play::Root, "...x.......x..xx"),
            ],
            fill: &[],
        },
    ],
    mix: &[("drums", -2.0), ("harpsi", 2.5), ("strings", 2.5)],
    lows: &["drums"],
    ..STRAIGHT
};

/// The layers, on the classic bed's instruments and buses.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "continuo",
            LayerKind::Bass,
            "strings",
            |_, m, d, v| scaled(strng(m, d.min(0.6), 900.0, STRNG_VIB, STRNG_ATT), v),
            0.16,
            0.1,
            (36, 47),
        ),
        layer(
            "harpsi",
            LayerKind::Arp,
            "harpsi",
            |_, m, d, v| scaled(harpsi(m, d.max(0.3)), v),
            0.3,
            -0.15,
            (62, 79),
        ),
        layer(
            "chops",
            LayerKind::Comp,
            "strings",
            |_, m, d, v| scaled(strng(m, d.min(0.12), 2600.0, STRNG_VIB, STRNG_ATT), v),
            0.05,
            0.25,
            (57, 71),
        ),
        layer(
            "held",
            LayerKind::Pad,
            "strings",
            |_, m, d, v| scaled(strng(m, d, 1800.0, 0.004, 0.08), v),
            0.025,
            0.0,
            (50, 65),
        ),
        layer(
            "drums",
            LayerKind::Drums,
            "drums",
            |ctx, m, _, v| match m {
                KICK => scaled(punch(), v),
                SNARE => scaled(timp(38 + (ctx.harmony.key_pc - 2).rem_euclid(12) - 5), 0.7 * v),
                CRASH => scaled(synth::crash(1.2), 0.25 * v),
                _ => Vec::new(),
            },
            0.336,
            0.0,
            (0, 0),
        ),
        layer(
            "timpani",
            LayerKind::Bass,
            "drums",
            |_, m, _, v| scaled(timp(m), v),
            0.3,
            0.0,
            (36, 47),
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
