//! The orchestral drill's Composed arrangement (milestone 5, Task 5): the layer engine (`bf_sound::music::layers`)
//! over the theme's own instruments, its classic bed handing over every section the layers do not play.
//!
//! The drill kit (kick, clap, rim, hats), the sliding 808, staccato cellos on the CELLO's voice and the contrabass
//! section. The fight rides the 808 under cello stabs; a desperation move strips to the contrabasses and the 808;
//! the final exchange adds the cello ostinato; the victory leaves the strings and a light kit under the solo.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::py::round_nd;
use bf_sound::music::groove::{Groove, Part, Play, line, only, upto};
use bf_sound::music::layers::{
    ArrangementTable, CLAP, CRASH, Cue, HAT, Instrument, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE,
};
use bf_sound::music::{MusicCtx, Samples};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::{CELLO, clap, contrabass, hat, rim, sub808};
use crate::ContentError;
use crate::themes::ThemeOptions;

/// The home key and scale.
pub const HOME: (&str, &str) = ("C", "harmonic minor");
/// The chord loops: the classic one first, then a faster i - bVI - bVII - V and a brooding i - iv - bVI - V.
pub const LOOPS: [&[&str]; 3] = [
    &super::LOOP,
    &["Cm", "Ab", "Bb", "G"],
    &["Cm", "Cm", "Fm", "Fm", "Ab", "Ab", "G", "G"],
];
/// The tempo range it suits, in BPM.
pub const TEMPOS: (f64, f64) = (130.0, 150.0);

/// Which layers play where.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("kit", 2), ("sub", 2), ("cellos", 1), ("basses", 1)],
    fill: &["kit"],
    breakdown: &[("sub", 1), ("basses", 2)],
    final_exchange: &[("kit", 3), ("sub", 3), ("cellos", 3), ("ostinato", 2), ("basses", 2)],
    ability: &["cellos", "kit"],
    victory: &[("kit", 1), ("sub", 2), ("basses", 2), ("ostinato", 1)],
    build: &[("kit", 3), ("sub", 2), ("cellos", 2), ("ostinato", 3)],
};

/// The orchestral drill's own band (sound upgrade, stage 2): a UK drill kit in a light sixteenth swing (the kick's
/// two-bar bounce, the clap on 3 with its late answer in the second bar, ghost claps, hats in threes and threes and
/// twos with rolls at the top), the 808 bouncing from the root to the fifth and the octave over two bars, the cellos
/// (an octave up, so a phone carries them) stabbing the same three-three-two, and the cello ostinato in threes
/// against the beat.
pub static GROOVE: Groove = Groove {
    swing: 0.55,
    swing16: true,
    push_ms: 3.0,
    vel_spread: 0.08,
    parts: &[
        Part {
            layer: "kit",
            lines: &[
                line(1, Play::Drum(KICK), "X......x..x.....X.....x.....x..."),
                line(1, Play::Drum(SNARE), "........X...............X....x.."),
                line(2, Play::Drum(HAT), "x..x..x.x..x..x."),
                line(2, Play::Drum(CLAP), "...g.......g...."),
                only(3, Play::Drum(HAT), "............xx.x").maybe(0.6),
                only(3, Play::Drum(SHAKER), "......x.......x.").maybe(0.5),
            ],
            fill: &[
                line(1, Play::Drum(HAT), "xgxgxxxx"),
                line(1, Play::Drum(SNARE), "....X..."),
            ],
        },
        Part {
            layer: "sub",
            lines: &[
                upto(1, Play::Root, "X_____......x__."),
                line(2, Play::Root, "x___......o_....x___............"),
                line(2, Play::Fifth, "......................o__......."),
                line(2, Play::Octave, "............................o_.."),
                only(3, Play::Octave, ".......g........................"),
                only(3, Play::Root, "..............x................."),
            ],
            fill: &[],
        },
        Part {
            layer: "cellos",
            lines: &[
                upto(1, Play::Chord, "x..x..x.x..x..x."),
                only(2, Play::Chord, "X..x..x.X..x..x."),
                only(3, Play::Chord, "X..x..x.X..x.xx."),
            ],
            fill: &[],
        },
        Part {
            layer: "ostinato",
            lines: &[
                upto(1, Play::Tone(0), "x.......x......."),
                upto(1, Play::Tone(1), "...o.......o...."),
                upto(1, Play::Tone(2), "......o.......o."),
                only(2, Play::Tone(0), "X..X..X..X..X..."),
                only(2, Play::Tone(1), ".o..o..o..o..o.o"),
                only(2, Play::Tone(2), "..o..o..o..o..X."),
                only(3, Play::Tone(0), "X..X..X..X..X..................."),
                only(3, Play::Tone(1), ".o..o..o..o..o.oX..X..X..X..X..."),
                only(3, Play::Tone(2), "..o..o..o..o..X..o..o..o..o..o.o"),
                only(3, Play::Tone(3), "..................o..o..o..o..X."),
            ],
            fill: &[],
        },
    ],
    mix: &[("bass808", -2.5), ("drums", -2.5), ("cellos", 9.5)],
    lift: &[("cellos", 12)],
    lows: &["bass808"],
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    let mut kick = synth::kick(0.9, 0.18, 140.0, 55.0);
    kick.truncate((0.3 * bf_sound::SRF) as usize);
    let (cl, ghost, hh, crash, rm) = (clap(1.0), clap(0.5), hat(0.55, false), synth::crash(1.6), rim(0.8, 1));
    let kit = sound(move |_, m, _| {
        Ok(match m {
            KICK => scaled(&kick, 0.48),
            SNARE => scaled(&cl, 0.34),
            HAT => scaled(&hh, 0.09),
            CRASH => scaled(&crash, 0.08),
            CLAP => scaled(&ghost, 0.14),
            SHAKER => scaled(&rm, 0.06),
            _ => Vec::new(),
        })
    });
    let sub = sound(|_, m, dur| Ok(sub808(m, m, dur.min(1.2), 0.0, 1.0)));
    let cello = |cap: f64| {
        sound(move |ctx: &MusicCtx, m, dur: f64| {
            let d = dur.min(cap);
            Ok(ctx
                .note(CELLO, f64::from(m), round_nd(d, 3), round_nd(d + 0.2, 2), 0.9)?
                .to_vec())
        })
    };
    let basses = sound(|_, m, dur| Ok(contrabass(m, dur, 0.8)));
    vec![
        layer("kit", LayerKind::Drums, kit, "drums", 1.0, 0.0, (0, 0)),
        layer("sub", LayerKind::Bass, sub, "bass808", 0.6, 0.0, (36, 47)),
        layer("cellos", LayerKind::Comp, cello(0.13), "cellos", 0.11, -0.2, (48, 64)),
        layer("ostinato", LayerKind::Arp, cello(0.11), "cellos", 0.15, 0.2, (55, 72)),
        layer("basses", LayerKind::Pad, basses, "basses", 0.125, 0.0, (36, 52)),
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
