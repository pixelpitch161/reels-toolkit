//! The Balkan fire's Composed arrangement (milestone 5): its layers over the classic bed's instruments and its
//! arrangement table. Still no drums of its own: the oom-pah is the upright on 1 and 3 and the accordion on the
//! off-beats, the cimbalom shimmer comes in as the fight heats up, and the sticks only click in the fills, the final
//! exchange and the build into frame 0.
//!
//! The small helpers here ([`Play`], [`layer`], [`scaled`], [`seed_of`]) are shared by the other themes' arrangements.

use std::sync::Arc;

use bf_sound::music::groove::{Groove, Part, Play as Note, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{ArrangementTable, CLAP, Cue, Instrument, Layer, LayerKind, LayeredBed, SHAKER};
use bf_sound::music::{MusicCtx, RISER_OVER, RISER_SEED, Samples};

use super::{MUSETTE, THEME, cimbalom, reed, roll, stab, sticks, upright};
use crate::error::ContentError;
use crate::themes::ThemeOptions;

/// The home key and scale.
pub const KEY: (&str, &str) = ("D", "harmonic minor");
/// The chord loops, the classic one first.
pub static LOOPS: [&[&str]; 3] = [
    &["Dm", "Dm", "Gm", "A"],
    &["Dm", "Gm", "C", "F", "Bb", "Gm", "A", "A"],
    &["Dm", "A", "Dm", "Gm", "Dm", "Bb", "A", "A"],
];
/// The tempo range, bpm.
pub const TEMPO: (f64, f64) = (126.0, 150.0);

/// The arrangement table.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("bass", 2), ("stabs", 1), ("pad", 1), ("cimbalom", 1)],
    fill: &["cimbalom", "sticks"],
    breakdown: &[("bass", 1), ("pad", 2), ("cimbalom", 1)],
    final_exchange: &[("bass", 3), ("stabs", 2), ("pad", 2), ("cimbalom", 3), ("sticks", 2)],
    ability: &["stabs"],
    victory: &[("bass", 3), ("stabs", 3), ("pad", 2), ("cimbalom", 3), ("sticks", 1)],
    build: &[
        ("bass", 3),
        ("stabs", 3),
        ("pad", 2),
        ("cimbalom", 3),
        ("roll", 1),
        ("riser", 1),
    ],
};

/// The Balkan fire's own band (sound upgrade, stage 2): a brass-band oom-pah, the upright on 1 and 3 (root and
/// fifth) breaking into the 3-3-2 of a čoček when the fight heats up, the accordion's pah on the off-beats, the
/// cimbalom running its chord in eighths and then tremolo sixteenths, and the sticks clicking the off-beats with a
/// clap on 2 and 4.
pub static GROOVE: Groove = Groove {
    push_ms: 5.0,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "bass",
            lines: &[
                upto(1, Note::Root, "x_____..........x_____.........."),
                upto(1, Note::Fifth, "........x_____..........x___..g."),
                only(2, Note::Root, "x.......x.......x.......x......."),
                only(2, Note::Fifth, "....x.......x.......x.......x.g."),
                only(3, Note::Root, "X..x..x.X..x....X..x..x.X..x...."),
                only(3, Note::Fifth, "............x.x.............x.x."),
            ],
            fill: &[],
        },
        Part {
            layer: "stabs",
            lines: &[
                line(1, Note::Upper, "..x...x...x...x."),
                line(2, Note::Upper, "...g.......g....").maybe(0.6),
                only(3, Note::Chord, "......X.......X."),
            ],
            fill: &[],
        },
        Part {
            layer: "cimbalom",
            lines: &[
                upto(1, Note::Tone(0), "x.......x.......x.......x......."),
                upto(1, Note::Tone(1), "..x.......x.......x.......x....."),
                upto(1, Note::Tone(2), "....x.......x.........x.......x."),
                upto(1, Note::Tone(3), "......x.......x.....x.......x..."),
                line(2, Note::Tone(0), "X.......o.......X.......o......."),
                line(2, Note::Tone(1), ".x.....o.x.....o.x.....o.x.....o"),
                line(2, Note::Tone(2), "..x...o...x...o...x...o...x...o."),
                line(2, Note::Tone(3), "...x.o.....x.o.....x.o.....x.o.."),
                line(2, Note::Tone(4), "....x.......x.......x.......x..."),
            ],
            fill: &[
                line(1, Note::Tone(4), "x......."),
                line(1, Note::Tone(3), ".x......"),
                line(1, Note::Tone(2), "..x....."),
                line(1, Note::Tone(1), "...x...."),
                line(1, Note::Tone(0), "....x..."),
                line(1, Note::Tone(1), ".....x.."),
                line(1, Note::Tone(2), "......x."),
                line(1, Note::Tone(3), ".......X"),
            ],
        },
        Part {
            layer: "sticks",
            lines: &[
                line(1, Note::Drum(SHAKER), "..x...x...x...x."),
                line(1, Note::Drum(CLAP), "....x.......x..."),
                line(2, Note::Drum(SHAKER), "g...g..g.g..g..g").maybe(0.7),
                only(3, Note::Drum(CLAP), "......x.......xx"),
            ],
            fill: &[],
        },
    ],
    mix: &[
        ("accordion", 10.0),
        ("bass", -2.0),
        ("cimbalom", 10.0),
        ("pad", 4.5),
        ("perc", 10.0),
    ],
    lows: &["bass"],
    ..STRAIGHT
};

/// One of a theme's sounds as a layer instrument: a function of the context, the note (a drum number for a kit), the
/// length and the velocity.
pub(crate) struct Play(pub fn(&MusicCtx, i32, f64, f64) -> Vec<f64>);

impl Instrument for Play {
    fn strike(&self, ctx: &MusicCtx, midi: i32, dur: f64, vel: f64) -> Samples {
        Ok((self.0)(ctx, midi, dur, vel))
    }
}

/// A layer `name` of kind `kind` playing `f` on bus `bus` at `gain` and `pan`, over the notes `range`.
pub(crate) fn layer(
    name: &'static str,
    kind: LayerKind,
    bus: &'static str,
    f: fn(&MusicCtx, i32, f64, f64) -> Vec<f64>,
    gain: f64,
    pan: f64,
    range: (i32, i32),
) -> Layer {
    Layer {
        name,
        kind,
        inst: Arc::new(Play(f)),
        bus,
        bus_mix: (1.0, 0.2),
        gain,
        pan,
        range,
    }
}

/// `x` times `vel`.
pub(crate) fn scaled(x: Vec<f64>, vel: f64) -> Vec<f64> {
    x.into_iter().map(|v| v * vel).collect()
}

/// A noise seed from a stroke's velocity (the only variation of a drum that draws noise).
pub(crate) fn seed_of(vel: f64) -> u64 {
    (vel * 1000.0) as u64
}

/// A riser on the `fx` bus, once a bar (a one-note layer): the build into frame 0 of the themes whose classic bed has
/// one.
pub(crate) fn riser() -> Layer {
    layer(
        "riser",
        LayerKind::Bass,
        "fx",
        |_, _, d, v| scaled(bf_sound::music::riser(d, RISER_OVER, RISER_SEED).unwrap_or_default(), v),
        0.26,
        0.0,
        (36, 47),
    )
}

/// The layers, on the classic bed's instruments and buses.
pub fn layers() -> Vec<Layer> {
    vec![
        layer(
            "bass",
            LayerKind::Bass,
            "bass",
            |_, m, d, v| upright(m, d.min(0.45), v),
            0.66,
            0.0,
            (36, 47),
        ),
        layer(
            "stabs",
            LayerKind::Comp,
            "accordion",
            |_, m, d, v| scaled(stab(&[m], d.min(0.2)), v),
            0.12,
            0.0,
            (55, 69),
        ),
        layer(
            "pad",
            LayerKind::Pad,
            "pad",
            |_, m, d, v| scaled(reed(&[m], d, 0.25, 0.3, 1400.0, MUSETTE), v),
            0.045,
            0.0,
            (50, 64),
        ),
        layer(
            "cimbalom",
            LayerKind::Arp,
            "cimbalom",
            |_, m, _, v| cimbalom(m, v),
            0.075,
            0.2,
            (73, 86),
        ),
        layer(
            "sticks",
            LayerKind::Perc,
            "perc",
            |_, m, _, v| sticks(if m == CLAP { v } else { 0.5 * v }),
            0.16,
            0.0,
            (0, 0),
        ),
        // the build into frame 0: the classic bed's snare roll and riser
        layer(
            "roll",
            LayerKind::Bass,
            "perc",
            |ctx, _, d, v| scaled(roll(d + 0.03, 0.15, ctx.show.grid.s16 / 2.0), v),
            0.36,
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
