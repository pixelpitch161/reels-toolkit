//! The second line's Composed arrangement (milestone 5, Task 5): the layer engine (`bf_sound::music::layers`) over
//! the theme's own instruments, its classic bed handing over every section the layers do not play.
//!
//! The parade kit (bass drum, snare, cymbal chick), the sousaphone on the TUBA's voice, the tambourine and claps.
//! The fight is a street parade with the sousaphone bouncing on eighths; a desperation move leaves the sousaphone
//! and the tambourine; the final exchange is the whole rhythm section flat out; the victory parades under the solo.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::py::round_nd;
use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{
    ArrangementTable, CLAP, CRASH, Cue, HAT, Instrument, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE,
};
use bf_sound::music::{MusicCtx, Samples};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::{TUBA, bass_drum, chick, clap, snare, tambourine};
use crate::ContentError;
use crate::themes::ThemeOptions;

/// The home key and scale.
pub const HOME: (&str, &str) = ("E", "harmonic minor");
/// The chord loops: the classic one first, then a gospel i - iv - i - V7 and a march-like i - VI7 - V7 - i.
pub const LOOPS: [&[&str]; 3] = [&super::LOOP, &["Em", "Am", "Em", "B7"], &["Em", "C7", "B7", "Em"]];
/// The tempo range it suits, in BPM.
pub const TEMPOS: (f64, f64) = (110.0, 132.0);

/// Which layers play where.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("kit", 2), ("sousa", 3), ("perc", 1)],
    fill: &["kit"],
    breakdown: &[("sousa", 2), ("perc", 1)],
    final_exchange: &[("kit", 3), ("sousa", 3), ("perc", 3)],
    ability: &["sousa", "kit"],
    victory: &[("kit", 2), ("sousa", 2), ("perc", 2)],
    build: &[("kit", 3), ("sousa", 3), ("perc", 3)],
};

/// The second line's own band (sound upgrade, stage 2): a swung New Orleans street beat, the bass drum on 1, the and
/// of 2 and 4 (the and of 3 in the second bar) with the cymbal on 2 and 4 and on top of the drum at full tilt, the
/// snare's accents among its ghost notes, rolling on at full tilt, the tambourine on the swung eighths and the claps
/// in the back half, and the sousaphone bouncing a two-bar line on the root and the fifth.
pub static GROOVE: Groove = Groove {
    swing: 0.62,
    push_ms: 5.0,
    vel_spread: 0.1,
    parts: &[
        Part {
            layer: "kit",
            lines: &[
                line(1, Play::Drum(KICK), "X.....x.....x...X.....x...x....."),
                line(1, Play::Drum(HAT), "....x.......x..."),
                only(3, Play::Drum(SHAKER), "X.....x.........X.....x...x.....").maybe(0.8),
                line(1, Play::Drum(SNARE), "...x..x....x..x."),
                line(2, Play::Drum(CLAP), "g.g.gg..g.g.gg.."),
                only(3, Play::Drum(CLAP), ".g.....g.g.....g").maybe(0.7),
            ],
            fill: &[
                line(1, Play::Drum(KICK), "x.....x."),
                line(1, Play::Drum(SNARE), "x.xgxgxX"),
            ],
        },
        Part {
            layer: "perc",
            lines: &[
                upto(1, Play::Drum(SHAKER), "....x.......x..."),
                line(2, Play::Drum(SHAKER), "o.g.X.g.o.g.X.g."),
                line(2, Play::Drum(CLAP), "............x..."),
                only(3, Play::Drum(CLAP), "....x..........."),
            ],
            fill: &[],
        },
        Part {
            layer: "sousa",
            lines: &[
                upto(1, Play::Root, "X_____..x_____.."),
                only(2, Play::Root, "X.....x.........X.....x........."),
                only(2, Play::Fifth, "........x_..................x..."),
                only(2, Play::Third, "........................x_......"),
                only(3, Play::Root, "X..o........x...X..o............"),
                only(3, Play::Fifth, "......x.x.....o.......x.....x..."),
                only(3, Play::Third, "..........x...................o."),
                only(3, Play::Octave, "........................x......."),
                only(3, Play::Seventh, "..........................x....."),
            ],
            fill: &[],
        },
    ],
    lows: &["sousa"],
    ..STRAIGHT
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    let (bd, sn, ch, crash, ghost, acc) = (
        bass_drum(false),
        snare(false, 1),
        chick(false, 1),
        synth::crash(1.6),
        snare(true, 2),
        chick(true, 3),
    );
    let kit = sound(move |_, m, _| {
        Ok(match m {
            KICK => scaled(&bd, 0.5),
            SNARE => scaled(&sn, 0.3),
            HAT => scaled(&ch, 0.14),
            CRASH => scaled(&crash, 0.08),
            CLAP => scaled(&ghost, 0.13),
            SHAKER => scaled(&acc, 0.08),
            _ => Vec::new(),
        })
    });
    let (tamb, cl) = (tambourine(1), clap(1));
    let perc = sound(move |_, m, _| {
        Ok(match m {
            SHAKER => scaled(&tamb, 0.07),
            CLAP => scaled(&cl, 0.2),
            _ => Vec::new(),
        })
    });
    let sousa = sound(|ctx, m, dur| {
        let d = dur.min(0.8);
        Ok(ctx
            .note(TUBA, f64::from(m), round_nd(d, 3), round_nd(d + 0.12, 2), 0.9)?
            .to_vec())
    });
    vec![
        layer("kit", LayerKind::Drums, kit, "drums", 1.0, 0.0, (0, 0)),
        layer("perc", LayerKind::Perc, perc, "perc", 2.0, 0.35, (0, 0)),
        layer("sousa", LayerKind::Bass, sousa, "sousa", 0.48, 0.0, (28, 40)),
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
