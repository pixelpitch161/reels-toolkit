//! The dark disco's Composed arrangement (milestone 5): four on the floor, the clap on 2 and 4, closed hats and
//! the open hat on the off-beats, the filtered-saw bass in eighths, string stabs, a pad and the mirror-ball glints.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{ArrangementTable, CLAP, CRASH, Cue, HAT, Layer, LayerKind, LayeredBed, SNARE};
use bf_sound::synth;

use super::{THEME, bass, clap, glint, hat, kick};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, riser, scaled, seed_of};

/// The home key and scale.
pub const KEY: (&str, &str) = ("A", "minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [
    &["Am", "F", "Dm", "E7", "Am", "F", "G", "E7"],
    &["Am", "Dm", "G", "C", "F", "Dm", "E7", "E7"],
    &["Am", "G", "F", "E7"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (112.0, 124.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[
        ("kick", 1),
        ("drums", 2),
        ("open", 1),
        ("bass", 3),
        ("strings", 1),
        ("pad", 1),
    ],
    fill: &["drums"],
    breakdown: &[("pad", 2), ("glints", 1), ("bass", 1)],
    final_exchange: &[
        ("kick", 1),
        ("drums", 3),
        ("open", 1),
        ("bass", 3),
        ("strings", 2),
        ("pad", 2),
        ("glints", 2),
    ],
    ability: &["strings", "drums"],
    victory: &[
        ("kick", 1),
        ("drums", 2),
        ("open", 1),
        ("bass", 3),
        ("strings", 2),
        ("pad", 2),
        ("glints", 1),
    ],
    build: &[
        ("kick", 1),
        ("drums", 3),
        ("bass", 3),
        ("strings", 3),
        ("pad", 2),
        ("glints", 3),
        ("riser", 1),
    ],
};

/// The dark disco's own band (sound upgrade, stage 2): the four on the floor stays, the clap on 2 and 4, closed
/// hats on the beat with sixteenths between and the open hat on every off-beat, the saw bass in octave eighths
/// (with sixteenth pickups at the top), the strings stabbing a two-bar off-beat figure and the glints sparkling
/// over it.
pub static GROOVE: Groove = Groove {
    push_ms: 2.5,
    vel_spread: 0.08,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(SNARE), "....X.......X..."),
                line(1, Play::Drum(HAT), "x.o.x.o.x.o.x.o."),
                line(2, Play::Drum(HAT), ".x.o.x.o.x.o.x.o"),
                only(3, Play::Drum(SNARE), "...........g...g").maybe(0.5),
            ],
            fill: &[],
        },
        Part {
            layer: "open",
            lines: &[line(1, Play::Drum(HAT), "..X..xX...X..xX.")],
            fill: &[],
        },
        Part {
            layer: "bass",
            lines: &[
                line(1, Play::Root, "x...o...x...o..."),
                line(2, Play::Octave, "..g...g...g...g."),
                only(3, Play::Root, "...g.......g....").maybe(0.7),
                only(3, Play::Fifth, ".......g.......g").maybe(0.5),
            ],
            fill: &[],
        },
        Part {
            layer: "strings",
            lines: &[
                line(1, Play::Chord, "X_.X_.X_.X_.X_.X..X_.X_.X_.X_.X_"),
                line(1, Play::Top, "X..X..X..X..X..X..X..X..X..X..X."),
                line(2, Play::Upper, ".......x........"),
                only(3, Play::Top, ".........x......").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "pad",
            lines: &[
                upto(1, Play::Chord, "o_______________"),
                line(2, Play::Chord, "x_______________"),
            ],
            fill: &[],
        },
        Part {
            layer: "glints",
            lines: &[
                line(1, Play::Top, "x...........x..."),
                line(1, Play::Tone(1), "......x........."),
                line(2, Play::Tone(2), "...x.......x...."),
                only(3, Play::Tone(0), ".x...x...x...x.."),
            ],
            fill: &[],
        },
    ],
    mix: &[
        ("bass", -4.0),
        ("drums", -4.0),
        ("pad", 6.0),
        ("shimmer", 8.0),
        ("strings", 8.0),
    ],
    lows: &["bass", "drums"],
    ..STRAIGHT
};

/// The layers, on the classic bed's instruments and buses.
pub fn layers() -> Vec<Layer> {
    vec![
        // an arpeggio at density 1 is one stroke a beat: the four on the floor
        layer(
            "kick",
            LayerKind::Arp,
            "drums",
            |_, _, _, _| kick(false),
            0.55,
            0.0,
            (36, 47),
        ),
        layer(
            "drums",
            LayerKind::Drums,
            "drums",
            |_, m, _, v| match m {
                SNARE => scaled(clap(seed_of(v)), v),
                HAT => scaled(hat(false, seed_of(v)), 0.4 * v),
                CRASH => scaled(synth::crash(1.2), 0.3 * v),
                _ => Vec::new(),
            },
            0.28,
            -0.05,
            (0, 0),
        ),
        layer(
            "open",
            LayerKind::Perc,
            "drums",
            |_, m, _, v| match m {
                CLAP => scaled(clap(seed_of(v)), v),
                _ => scaled(hat(true, seed_of(v)), v),
            },
            0.09,
            0.25,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| scaled(bass(m, d.min(0.3)), v),
            0.5,
            0.0,
            (33, 45),
        ),
        layer(
            "strings",
            LayerKind::Comp,
            "strings",
            |_, m, d, v| scaled(synth::pad(&[f64::from(m)], d.min(0.2), 3200.0, 0.01), v),
            0.08,
            0.15,
            (57, 72),
        ),
        layer(
            "pad",
            LayerKind::Pad,
            "pad",
            |_, m, d, v| scaled(synth::pad(&[f64::from(m)], d, 1600.0, 0.3), v),
            0.096,
            0.0,
            (52, 67),
        ),
        layer(
            "glints",
            LayerKind::Arp,
            "shimmer",
            |_, m, _, v| scaled(glint(m), v),
            0.108,
            0.3,
            (76, 93),
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
