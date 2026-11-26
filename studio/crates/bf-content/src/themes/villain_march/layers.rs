//! The villain march's Composed arrangement (milestone 5, Task 5): the layer engine (`bf_sound::music::layers`)
//! over the theme's own instruments, its classic bed handing over every section the layers do not play.
//!
//! The march kit (the bass drum's boom, snare strokes on every eighth), the timpani walking the roots, brass stabs,
//! the horn pad and the creepy music box. The fight marches under brass stabs; a desperation move leaves the music
//! box alone over the horns; the final exchange is the whole army; the victory lets the music box play under the
//! solo.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{
    ArrangementTable, CLAP, CRASH, Cue, HAT, Instrument, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE,
};
use bf_sound::music::{MusicCtx, Samples};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::{bass_drum_boom, brass, horn_pad, march_snare_stroke, music_box, timpani};
use crate::ContentError;
use crate::themes::ThemeOptions;

/// The home key and scale.
pub const HOME: (&str, &str) = ("C", "harmonic minor");
/// The chord loops: the classic one first, then a menacing i - bII - i - V and a stalking i - iv - bVI - V.
pub const LOOPS: [&[&str]; 3] = [&super::LOOP, &["Cm", "Db", "Cm", "G"], &["Cm", "Fm", "Ab", "G"]];
/// The tempo range it suits, in BPM.
pub const TEMPOS: (f64, f64) = (108.0, 132.0);

/// Which layers play where.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("kit", 2), ("timp", 2), ("brass", 1), ("pad", 1)],
    fill: &["kit"],
    breakdown: &[("pad", 1), ("musicbox", 2)],
    final_exchange: &[("kit", 3), ("timp", 3), ("brass", 3), ("pad", 2)],
    ability: &["brass", "kit"],
    victory: &[("kit", 1), ("timp", 1), ("pad", 2), ("musicbox", 2)],
    build: &[("kit", 3), ("timp", 2), ("brass", 2), ("musicbox", 3)],
};

/// The villain march's own band (sound upgrade, stage 2): the bass drum on every beat (on 1 and 3 at the lightest),
/// the march snare in dotted figures with sixteenth ghosts and accents at the top and a roll for the fill, the
/// timpani walking root and fifth with pickups, the brass's march stabs (three quarters, then the dotted eighth and
/// sixteenth into a held chord) and the music box turning down the chord.
pub static GROOVE: Groove = Groove {
    push_ms: 4.0,
    vel_spread: 0.08,
    parts: &[
        Part {
            layer: "kit",
            lines: &[
                line(1, Play::Drum(KICK), "X.......X......."),
                line(2, Play::Drum(KICK), "....x.......x..."),
                upto(1, Play::Drum(SNARE), "....x.......x..."),
                line(2, Play::Drum(SNARE), "x..xx.x.x..xx.x."),
                only(3, Play::Drum(HAT), ".g.g.g.g.g.g.g.g"),
                only(3, Play::Drum(CLAP), "....x.......x..."),
                only(3, Play::Drum(CRASH), "X...............................").maybe(0.5),
            ],
            fill: &[
                line(1, Play::Drum(SNARE), "xxxxxxxX"),
                line(1, Play::Drum(KICK), "X...X..."),
            ],
        },
        Part {
            layer: "timp",
            lines: &[
                upto(1, Play::Root, "x.......x......."),
                line(2, Play::Root, "x.......x......."),
                line(2, Play::Fifth, "....x.......x..."),
                only(3, Play::Root, "...g.......g....").maybe(0.7),
            ],
            fill: &[],
        },
        Part {
            layer: "brass",
            lines: &[
                upto(1, Play::Chord, "x_......x_......"),
                line(2, Play::Chord, "x_..x_..x_.xx___x_.xx___x_.xx___"),
                only(3, Play::Upper, "..o...o...o...o.").maybe(0.6),
            ],
            fill: &[],
        },
        Part {
            layer: "musicbox",
            lines: &[
                line(1, Play::Tone(3), "x.......x......."),
                line(1, Play::Tone(1), "....x.......x..."),
                line(2, Play::Tone(2), "..x.......x....."),
                line(2, Play::Tone(0), "......x.......x."),
                only(3, Play::Tone(4), "...o.......o....").maybe(0.6),
            ],
            fill: &[],
        },
    ],
    ..STRAIGHT
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    let (boom, stroke, crash) = (bass_drum_boom(), march_snare_stroke(), synth::crash(1.6));
    let kit = sound(move |_, m, _| {
        Ok(match m {
            KICK => scaled(&boom, 0.45),
            SNARE => scaled(&stroke, 0.25),
            HAT => scaled(&stroke, 0.08),
            CRASH => scaled(&crash, 0.07),
            CLAP => scaled(&stroke, 0.2),
            SHAKER => scaled(&stroke, 0.06),
            _ => Vec::new(),
        })
    });
    let timp = sound(|_, m, _| Ok(timpani(m, 0.9)));
    let stabs = sound(|_, m, dur| Ok(brass(&[m], dur.max(0.16), 0.012, 1500.0, 1.0)));
    let pad = sound(|_, m, dur| Ok(horn_pad(&[m], dur)));
    let mbox = sound(|_, m, _| Ok(music_box(m, 1.0)));
    vec![
        layer("kit", LayerKind::Drums, kit, "drums", 0.6, -0.1, (0, 0)),
        layer("timp", LayerKind::Bass, timp, "drums", 0.3, 0.0, (36, 48)),
        layer("brass", LayerKind::Comp, stabs, "brass", 0.08, 0.0, (48, 67)),
        layer("pad", LayerKind::Pad, pad, "pad", 0.06, 0.0, (48, 64)),
        layer("musicbox", LayerKind::Arp, mbox, "musicbox", 0.16, 0.0, (72, 88)),
    ]
}

/// The Composed bed for the fight's `options`, its story `cues` and its music seed.
///
/// # Errors
/// The classic bed refuses the options.
pub fn composed(options: &ThemeOptions, cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    let classic = (super::THEME.classic)(options)?;
    Ok(Box::new(
        LayeredBed::new(classic, layers(), TABLE, cues, seed)?.with_groove(&GROOVE)?,
    ))
}

/// One of the bed's sounds: `make(ctx, note, seconds)` at full velocity, made once per note and length and scaled
/// by each strike's velocity.
struct Sound<F> {
    make: F,
    made: Mutex<Made>,
}

/// The sounds made so far, by note and length in milliseconds.
type Made = BTreeMap<(i32, i64), Arc<[f64]>>;

impl<F: Fn(&MusicCtx, i32, f64) -> Samples + Send + Sync> Instrument for Sound<F> {
    fn strike(&self, ctx: &MusicCtx, midi: i32, dur: f64, vel: f64) -> Samples {
        let key = (midi, (dur * 1e3).round() as i64);
        let mut made = self.made.lock().unwrap_or_else(|e| e.into_inner());
        let x = match made.get(&key) {
            Some(x) => x.clone(),
            None => {
                let x: Arc<[f64]> = (self.make)(ctx, midi, dur)?.into();
                made.insert(key, x.clone());
                x
            }
        };
        Ok(x.iter().map(|v| v * vel).collect())
    }
}

fn sound(make: impl Fn(&MusicCtx, i32, f64) -> Samples + Send + Sync + 'static) -> Arc<dyn Instrument> {
    Arc::new(Sound {
        make,
        made: Mutex::new(BTreeMap::new()),
    })
}

/// A layer on bus `bus` (one of the classic bed's, whose level it keeps).
fn layer(
    name: &'static str,
    kind: LayerKind,
    inst: Arc<dyn Instrument>,
    bus: &'static str,
    gain: f64,
    pan: f64,
    range: (i32, i32),
) -> Layer {
    Layer {
        name,
        kind,
        inst,
        bus,
        bus_mix: (1.0, 0.2),
        gain,
        pan,
        range,
    }
}

fn scaled(x: &[f64], g: f64) -> Vec<f64> {
    x.iter().map(|v| v * g).collect()
}
