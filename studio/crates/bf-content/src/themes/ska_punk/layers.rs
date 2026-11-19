//! The ska punk's Composed arrangement (milestone 5, Task 5): the layer engine (`bf_sound::music::layers`) over the
//! theme's own instruments, its classic bed handing over every section the layers do not play.
//!
//! The punk kit, the guitar's choked upstroke skanking on every off-beat, the bubbling organ and the ring ropes
//! plucked as a bass. The fight skanks over driving eighth-note bass; a desperation move leaves the skank and a held
//! root; the final exchange is everything flat out.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_sound::music::layers::{
    ArrangementTable, CLAP, CRASH, Cue, HAT, Instrument, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE,
};
use bf_sound::music::{MusicCtx, Samples};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::{kick, organ, ride, rope_pluck, skank, snare, tom};
use crate::ContentError;
use crate::themes::ThemeOptions;

/// The home key and scale.
pub const HOME: (&str, &str) = ("E", "minor");
/// The chord loops: the classic one first, then a punk i - bVI - bIII - bVII and a two-chord i - bVII vamp.
pub const LOOPS: [&[&str]; 3] = [&super::LOOP, &["Em", "C", "G", "D"], &["Em", "D", "Em", "D"]];
/// The tempo range it suits, in BPM.
pub const TEMPOS: (f64, f64) = (136.0, 176.0);

/// Which layers play where.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("kit", 2), ("bass", 3), ("skank", 2), ("organ", 1)],
    fill: &["kit"],
    breakdown: &[("bass", 1), ("skank", 2)],
    final_exchange: &[("kit", 3), ("bass", 3), ("skank", 3), ("organ", 2)],
    ability: &["skank", "kit"],
    victory: &[("kit", 1), ("bass", 2), ("skank", 2), ("organ", 1)],
    build: &[("kit", 3), ("bass", 3), ("skank", 2), ("organ", 2)],
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    let (k, s, h, crash, rd, t) = (
        kick(),
        snare(),
        synth::hat(0.03, false),
        synth::crash(1.4),
        ride(1),
        tom(45),
    );
    let kit = sound(move |_, m, _| {
        Ok(match m {
            KICK => scaled(&k, 0.42),
            SNARE => scaled(&s, 0.36),
            HAT => scaled(&h, 0.13),
            CRASH => scaled(&crash, 0.07),
            CLAP => scaled(&t, 0.22),
            SHAKER => scaled(&rd, 0.08),
            _ => Vec::new(),
        })
    });
    let bass = sound(|_, m, dur| Ok(rope_pluck(m, dur.clamp(0.2, 0.6), 1.0, 7)));
    let up = sound(|_, m, _| Ok(skank(&[m], 1.0, m.unsigned_abs().into())));
    let org = sound(|_, m, dur| Ok(organ(&[m], dur, 1.0)));
    vec![
        layer("kit", LayerKind::Drums, kit, "kit", 1.0, 0.0, (0, 0)),
        layer("bass", LayerKind::Bass, bass, "ropes", 0.135, 0.0, (40, 51)),
        layer("skank", LayerKind::Comp, up, "skank", 0.37, -0.35, (59, 71)),
        layer("organ", LayerKind::Pad, org, "bubble", 0.06, 0.35, (52, 67)),
    ]
}

/// The Composed bed for the fight's `options`, its story `cues` and its music seed.
///
/// # Errors
/// The classic bed refuses the options.
pub fn composed(options: &ThemeOptions, cues: Vec<Cue>, seed: u64) -> Result<Box<dyn Bed>, ContentError> {
    let classic = (super::THEME.classic)(options)?;
    Ok(Box::new(LayeredBed::new(classic, layers(), TABLE, cues, seed)?))
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
