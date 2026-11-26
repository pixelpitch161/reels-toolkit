//! The spaghetti western's Composed arrangement (milestone 5, Task 5): the layer engine (`bf_sound::music::layers`)
//! over the theme's own instruments, its classic bed handing over every section the layers do not play.
//!
//! The galloping coconut hooves and the low frame drum, the baritone guitar's twang (its low strings as the bass,
//! its high ones arpeggiated), the wordless choir, the tremolo strings and the church bell. The fight gallops under
//! the twang; a desperation move leaves the choir and the shivering strings; the final exchange is the whole
//! showdown; each captioned ability tolls the bell.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{
    ArrangementTable, CLAP, CRASH, Cue, HAT, Instrument, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE,
};
use bf_sound::music::{MusicCtx, Samples};
use bf_sound::traits::Bed;

use super::{bell, choir, hoof, thump, tremolo_strings, twang, whip};
use crate::ContentError;
use crate::themes::ThemeOptions;

/// The home key and scale.
pub const HOME: (&str, &str) = ("A", "minor");
/// The chord loops: the classic one first, then a desert i - bVII - bVI - bVII and a tense i - iv - V - i.
pub const LOOPS: [&[&str]; 3] = [&["Am", "G", "F", "E"], &["Am", "G", "F", "G"], &["Am", "Dm", "E", "Am"]];
/// The tempo range it suits, in BPM.
pub const TEMPOS: (f64, f64) = (116.0, 140.0);

/// Which layers play where.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("hooves", 2), ("bass", 2), ("arp", 1), ("choir", 1)],
    fill: &["hooves"],
    breakdown: &[("choir", 2), ("strings", 1)],
    final_exchange: &[("hooves", 3), ("bass", 3), ("arp", 3), ("choir", 2), ("strings", 2)],
    ability: &["bell", "hooves"],
    victory: &[("hooves", 1), ("bass", 2), ("choir", 2), ("strings", 1)],
    build: &[("hooves", 3), ("bass", 2), ("arp", 2), ("strings", 3)],
};

/// The spaghetti western's own band (sound upgrade, stage 2): the coconut hooves' gallop (a hoof on each beat and
/// two light ones after it, a full canter at the top) over the frame drum's thump on 1 and 3 and the whip cracking
/// at the end of the second bar, the baritone guitar's low twang on the root and the fifth, and its high strings
/// climbing and falling through the chord in quarters, eighths and then a turn.
pub static GROOVE: Groove = Groove {
    push_ms: 5.0,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "hooves",
            lines: &[
                line(1, Play::Drum(SNARE), "X...x...X...x..."),
                line(1, Play::Drum(HAT), "..o...o...o...o."),
                line(2, Play::Drum(HAT), "...o...o...o...o"),
                line(2, Play::Drum(KICK), "X.......X......."),
                only(3, Play::Drum(SHAKER), ".g...g...g...g.."),
                only(3, Play::Drum(CRASH), "..............................X.").maybe(0.5),
            ],
            fill: &[
                line(1, Play::Drum(SNARE), "x.xxx.x."),
                line(1, Play::Drum(CRASH), ".......X"),
            ],
        },
        Part {
            layer: "bass",
            lines: &[
                line(1, Play::Root, "X.....x........."),
                line(1, Play::Fifth, "........X......."),
                only(3, Play::Octave, "..........x....."),
                only(3, Play::Fifth, "..............x."),
            ],
            fill: &[],
        },
        Part {
            layer: "arp",
            lines: &[
                upto(1, Play::Tone(0), "x..............................."),
                upto(1, Play::Tone(1), "....o.......................o..."),
                upto(1, Play::Tone(2), "........x...............x......."),
                upto(1, Play::Tone(3), "............o.......o..........."),
                upto(1, Play::Tone(4), "................x..............."),
                only(2, Play::Tone(0), "x..............."),
                only(2, Play::Tone(1), "..o...........o."),
                only(2, Play::Tone(2), "....x.......x..."),
                only(2, Play::Tone(3), "......o...o....."),
                only(2, Play::Tone(4), "........x......."),
                only(3, Play::Tone(0), "X.............x."),
                only(3, Play::Tone(1), "..o.........o..."),
                only(3, Play::Tone(2), "....x.....x....."),
                only(3, Play::Tone(3), "......o.o......."),
                only(3, Play::Tone(4), ".......x........"),
            ],
            fill: &[],
        },
    ],
    ..STRAIGHT
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    let (th, hi, lo, cr, cl, soft) = (
        thump(1.0),
        hoof(1.0, true, 1),
        hoof(0.55, false, 2),
        whip(1.0, 3),
        hoof(0.8, true, 4),
        hoof(0.45, false, 5),
    );
    let hooves = sound(move |_, m, _| {
        Ok(match m {
            KICK => scaled(&th, 0.36),
            SNARE => scaled(&hi, 0.24),
            HAT => scaled(&lo, 0.2),
            CRASH => scaled(&cr, 0.25),
            CLAP => scaled(&cl, 0.2),
            SHAKER => scaled(&soft, 0.13),
            _ => Vec::new(),
        })
    });
    let bass = sound(|_, m, dur| Ok(twang(m, dur.min(0.9), 1.0)));
    let arp = sound(|_, m, dur| Ok(twang(m, dur.min(0.5), 0.8)));
    let ch = sound(|_, m, dur| Ok(choir(&[m], dur + 0.15, 0.45, 0.5)));
    let strings = sound(|ctx, m, dur| Ok(tremolo_strings(&[m], dur, 0.5, 1.0, ctx.show.grid.s16)));
    let toll = sound(|_, m, _| Ok(bell(m, 1.0, 4.0)));
    vec![
        layer("hooves", LayerKind::Drums, hooves, "perc", 1.0, 0.0, (0, 0)),
        layer("bass", LayerKind::Bass, bass, "twang", 0.42, 0.0, (45, 57)),
        layer("arp", LayerKind::Arp, arp, "twang", 0.22, 0.2, (57, 76)),
        layer("choir", LayerKind::Pad, ch, "choir", 0.25, 0.0, (52, 69)),
        layer("strings", LayerKind::Pad, strings, "strings", 0.06, 0.0, (57, 76)),
        layer("bell", LayerKind::Comp, toll, "bell", 0.1, 0.0, (69, 81)),
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
