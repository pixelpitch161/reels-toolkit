//! The sea shanty's Composed arrangement (milestone 5, Task 5): the layer engine (`bf_sound::music::layers`) over
//! the theme's own instruments, its classic bed handing over every section the layers do not play.
//!
//! Boots stomping the deck, the crew's gang claps, the frame drum, the low male choir humming the chords, an open
//! "oh" on the stabs, and the bass voice. The fight is a stomp-and-clap sing-along; a desperation move leaves the
//! hum and the bass voice; the final exchange is the whole crew; the victory hums under the solo.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_sound::music::groove::{Groove, Part, Play, STRAIGHT, line, only, upto};
use bf_sound::music::layers::{
    ArrangementTable, CLAP, CRASH, Cue, HAT, Instrument, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE,
};
use bf_sound::music::{MusicCtx, Samples};
use bf_sound::traits::Bed;

use super::{Vowel, bass_voice, choir, frame_drum, gang_clap_hands, stomp};
use crate::ContentError;
use crate::themes::ThemeOptions;

/// The home key and scale.
pub const HOME: (&str, &str) = ("A", "dorian");
/// The chord loops: the classic one first, then a dorian i - IV - i - bVII and a rolling i - bVII - bVI - bVII.
pub const LOOPS: [&[&str]; 3] = [
    &["Am", "G", "Am", "Em"],
    &["Am", "D", "Am", "G"],
    &["Am", "G", "F", "G"],
];
/// The tempo range it suits, in BPM.
pub const TEMPOS: (f64, f64) = (108.0, 132.0);

/// Which layers play where.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("deck", 2), ("drum", 2), ("bass", 2), ("hum", 1), ("hey", 1)],
    fill: &["deck", "drum"],
    breakdown: &[("bass", 1), ("hum", 2)],
    final_exchange: &[("deck", 3), ("drum", 3), ("bass", 3), ("hum", 2), ("hey", 2)],
    ability: &["hey", "deck"],
    victory: &[("deck", 1), ("bass", 2), ("hum", 2)],
    build: &[("deck", 3), ("drum", 2), ("bass", 2), ("hey", 2)],
};

/// The sea shanty's own band (sound upgrade, stage 2): a capstan sing-along, light boots on 1 and 3 (a push on the
/// and of 2 at full crew), the gang claps on 2 and 4, the frame drum tapping eighths through the fight (it carries
/// the crew on a phone, where the boots and the low voices fade) and a boom to open the two-bar phrase at full
/// crew, the bass voice's short "doom" on root and fifth, and the crew's call and answer on the open "oh", an
/// octave up, "hey ... ho" at the end of the phrase.
pub static GROOVE: Groove = Groove {
    push_ms: 6.0,
    vel_spread: 0.12,
    parts: &[
        Part {
            layer: "deck",
            lines: &[
                upto(1, Play::Drum(KICK), "o.......g......."),
                only(2, Play::Drum(KICK), "o.......g......."),
                only(3, Play::Drum(KICK), "o.....g.g......."),
                only(3, Play::Drum(KICK), "..............o."),
                line(1, Play::Drum(SNARE), "....X.......X..."),
                only(3, Play::Drum(HAT), "..g.......g.....").maybe(0.7),
                // the crew's frame-drum taps and an answering clap carry the stomp on a phone
                line(2, Play::Drum(SHAKER), "X.x.X.x.X.x.X.x."),
                line(2, Play::Drum(CLAP), "......x.......x."),
                only(3, Play::Drum(CLAP), "....X.......X..x"),
                only(3, Play::Drum(CRASH), "o..............................."),
            ],
            fill: &[],
        },
        Part {
            layer: "drum",
            lines: &[
                upto(1, Play::Drum(SHAKER), "x...x...x...x..."),
                line(2, Play::Drum(SHAKER), "X.x.X.x.X.x.X.x."),
                line(2, Play::Drum(SHAKER), ".g.g.g.g.g.g.g.g").maybe(0.7),
                line(2, Play::Drum(CLAP), "....x.......x..."),
            ],
            fill: &[],
        },
        Part {
            layer: "bass",
            lines: &[
                upto(1, Play::Root, "o___....o___...."),
                only(2, Play::Root, "o__.............o__............."),
                only(2, Play::Fifth, "........g_..............g_....g_"),
                only(3, Play::Root, "x__.o_.o_x__...."),
                only(3, Play::Fifth, "..............o_"),
            ],
            fill: &[],
        },
        Part {
            layer: "hum",
            lines: &[line(1, Play::Upper, "g_______________")],
            fill: &[],
        },
        Part {
            layer: "hey",
            lines: &[
                upto(1, Play::Chord, "....x_..............x___....x___"),
                line(2, Play::Chord, "....x_......x_......x___....X___"),
            ],
            fill: &[],
        },
    ],
    mix: &[("choir", -8.0), ("drum", 4.0), ("stomp", -8.0)],
    lift: &[("bass", 12), ("hey", 12)],
    lows: &["choir", "stomp"],
    ..STRAIGHT
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    let (boot, soft, clap, tap, boom) = (
        stomp(1.0, 1),
        stomp(0.5, 3),
        gang_clap_hands(),
        frame_drum(false),
        frame_drum(true),
    );
    let (clap2, tap2) = (clap.clone(), tap.clone());
    let deck = sound(move |_, m, _| {
        Ok(match m {
            KICK => scaled(&boot, 0.5),
            SNARE => scaled(&clap, 0.3),
            HAT => scaled(&soft, 0.12),
            CRASH => scaled(&boom, 0.5),
            CLAP => scaled(&clap, 0.22),
            SHAKER => scaled(&tap, 0.1),
            _ => Vec::new(),
        })
    });
    let drum = sound(move |_, m, _| {
        Ok(match m {
            SHAKER => scaled(&tap2, 0.08),
            CLAP => scaled(&clap2, 0.2),
            _ => Vec::new(),
        })
    });
    let bass = sound(|_, m, dur| Ok(bass_voice(m, dur.clamp(0.25, 0.6), 1.0)));
    let hum = sound(|_, m, dur| Ok(choir(&[m], dur + 0.1, 0.3, 0.4, Vowel::M)));
    let hey = sound(|_, m, dur| Ok(choir(&[m], dur.max(0.18), 0.01, 0.2, Vowel::O)));
    vec![
        layer("deck", LayerKind::Drums, deck, "stomp", 1.0, 0.0, (0, 0)),
        layer("drum", LayerKind::Perc, drum, "drum", 1.0, 0.0, (0, 0)),
        layer("bass", LayerKind::Bass, bass, "choir", 0.22, 0.0, (33, 45)),
        layer("hum", LayerKind::Pad, hum, "choir", 0.05, 0.0, (45, 64)),
        layer("hey", LayerKind::Comp, hey, "choir", 0.06, 0.0, (52, 67)),
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
