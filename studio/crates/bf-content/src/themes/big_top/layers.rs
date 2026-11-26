//! The big top's Composed arrangement (milestone 5): the circus oom-pah, with the concert bass drum on 1 and 3, the
//! choked cymbal and the snare on 2 and 4, calliope chords on the off-beats over a low calliope bass, an organ
//! chord held under them, the glockenspiel running the chord and the tambourine shaking.

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{
    ArrangementTable, CLAP, CRASH, Cue, HAT, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE,
};
use bf_sound::synth;

use super::{CALLIOPE_ATT, CALLIOPE_VIB, THEME, bass_drum, calliope, choke, roll, snare, tambourine};
use crate::error::ContentError;
use crate::themes::ThemeOptions;
use crate::themes::balkan_fire::layers::{layer, riser, scaled, seed_of};

/// The home key and scale.
pub const KEY: (&str, &str) = ("E", "harmonic minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [
    &["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"],
    &["Em", "B7", "Em", "B7", "Am", "Em", "B7", "Em"],
    &["Em", "C", "Am", "B7"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (120.0, 144.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 1), ("bass", 2), ("pah", 1), ("organ", 1), ("glock", 1)],
    fill: &["drums", "tambourine"],
    breakdown: &[("bass", 1), ("organ", 2), ("glock", 1)],
    final_exchange: &[
        ("drums", 2),
        ("oom", 1),
        ("bass", 3),
        ("pah", 2),
        ("organ", 2),
        ("glock", 2),
        ("tambourine", 2),
    ],
    ability: &["pah", "drums"],
    victory: &[
        ("drums", 2),
        ("oom", 1),
        ("bass", 3),
        ("pah", 2),
        ("organ", 2),
        ("glock", 2),
        ("tambourine", 2),
    ],
    build: &[
        ("drums", 2),
        ("oom", 1),
        ("bass", 3),
        ("pah", 2),
        ("organ", 2),
        ("glock", 3),
        ("roll", 1),
        ("riser", 1),
    ],
};

/// The big top's own band (sound upgrade, stage 2): a circus march's oom-pah, the concert bass drum on 1 and 3
/// with the snare and the choked cymbal on 2 and 4 and a ruff before them, the calliope bass on the root and the
/// fifth (the octave in the second bar) walking in quarters at the top, the calliope's pah on 2 and 4 and the
/// off-beats, the glockenspiel turning round the chord and the tambourine on the eighths.
pub static GROOVE: Groove = Groove {
    push_ms: 4.0,
    vel_spread: 0.08,
    parts: &[
        Part {
            layer: "drums",
            lines: &[
                line(1, Play::Drum(KICK), "x.......x......."),
                line(1, Play::Drum(SNARE), "....x.......x..."),
                line(2, Play::Drum(SNARE), "...g.......g....").maybe(0.6),
                line(2, Play::Drum(HAT), "..x...x...x...x."),
                only(3, Play::Drum(HAT), "o...o...o...o..."),
            ],
            fill: &[
                line(1, Play::Drum(SNARE), "x.xxx.xX"),
                line(1, Play::Drum(KICK), "x......."),
            ],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(1, Play::Root, "x_.............."),
                upto(1, Play::Fifth, "........x_......"),
                line(2, Play::Root, "x_..............x_.............."),
                line(2, Play::Fifth, "........x_......................"),
                line(2, Play::Octave, "........................x_......"),
                only(3, Play::Third, "....o...........").maybe(0.7),
                only(3, Play::Fifth, "............o..."),
            ],
            fill: &[],
        },
        Part {
            layer: "pah",
            lines: &[
                line(1, Play::Chord, "....x.......x..."),
                line(2, Play::Upper, "..o...o...o...o."),
            ],
            fill: &[],
        },
        Part {
            layer: "glock",
            lines: &[
                line(1, Play::Tone(0), "x.......x......."),
                line(1, Play::Tone(2), "....x.......x..."),
                line(2, Play::Tone(1), "..x...x...x...x."),
                only(3, Play::Tone(3), ".o.o.o.o.o.o.o.o").maybe(0.7),
            ],
            fill: &[],
        },
        Part {
            layer: "tambourine",
            lines: &[
                line(1, Play::Drum(SHAKER), "o.x.o.x.o.x.o.x."),
                line(2, Play::Drum(CLAP), "....X.......X..."),
                only(3, Play::Drum(SHAKER), ".g.g.g.g.g.g.g.g"),
            ],
            fill: &[line(1, Play::Drum(CLAP), "x.x.xxxX")],
        },
    ],
    ..STRAIGHT
};

/// The layers, on the classic bed's instruments and buses.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "drums",
            LayerKind::Drums,
            "drums",
            |_, m, _, v| match m {
                KICK => bass_drum(v),
                SNARE => {
                    let mut x = snare(0.6 * v);
                    for (o, c) in x.iter_mut().zip(choke(0.9 * v, seed_of(v))) {
                        *o += c;
                    }
                    x
                }
                HAT => tambourine(0.5 * v, seed_of(v)),
                CRASH => scaled(synth::crash(1.0), 0.4 * v),
                _ => Vec::new(),
            },
            0.5,
            0.0,
            (0, 0),
        ),
        layer(
            "bass",
            LayerKind::Bass,
            "calliope",
            |_, m, d, v| calliope(&[m], d.min(0.4), v, 3, CALLIOPE_ATT, CALLIOPE_VIB),
            0.104,
            0.0,
            (40, 51),
        ),
        layer(
            "pah",
            LayerKind::Comp,
            "calliope",
            |_, m, d, v| calliope(&[m], d.min(0.22), v, 5, CALLIOPE_ATT, CALLIOPE_VIB),
            0.058,
            0.15,
            (59, 73),
        ),
        layer(
            "organ",
            LayerKind::Pad,
            "calliope",
            |_, m, d, v| calliope(&[m], d, v, 7, 0.08, CALLIOPE_VIB),
            0.019,
            -0.1,
            (52, 66),
        ),
        layer(
            "glock",
            LayerKind::Arp,
            "glock",
            |_, m, d, v| scaled(synth::chime(f64::from(m), d.clamp(0.2, 0.5)), v),
            0.018,
            0.25,
            (76, 91),
        ),
        layer(
            "tambourine",
            LayerKind::Perc,
            "drums",
            |_, m, _, v| tambourine(if m == CLAP { v } else { 0.6 * v }, seed_of(v)),
            0.12,
            0.3,
            (0, 0),
        ),
        layer(
            "roll",
            LayerKind::Bass,
            "drums",
            |ctx, _, d, v| roll(d + 0.01, 0.4, v, ctx.show.grid.s16 / 2.0),
            0.16,
            0.0,
            (40, 51),
        ),
        // the bass drum on every beat (an arpeggio at density 1 is one stroke a beat)
        layer(
            "oom",
            LayerKind::Arp,
            "drums",
            |_, _, _, _| bass_drum(1.0),
            0.4,
            0.0,
            (40, 51),
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
