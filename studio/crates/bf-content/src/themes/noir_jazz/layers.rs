//! The noir jazz's Composed arrangement (milestone 5, Task 5): the layer engine (`bf_sound::music::layers`) over
//! the theme's own instruments, its classic bed handing over every section the layers do not play.
//!
//! Brushes, the walking upright, the muted piano on the PIANO's voice and the dark pad. The fight walks on brushes
//! with the piano comping lightly; a desperation move drops to the bass and the pad; the final exchange is the whole
//! combo at full tilt; the victory falls back to brushes under the solo.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::py::round_nd;
use bf_sound::music::layers::{
    ArrangementTable, CLAP, CRASH, Cue, HAT, Instrument, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE,
};
use bf_sound::music::{MusicCtx, Samples};
use bf_sound::synth::{self, FILTER_O};
use bf_sound::traits::Bed;

use super::{PIANO, brush_tap, dark_pad, ride, upright};
use crate::ContentError;
use crate::themes::ThemeOptions;

/// The home key and scale.
pub const HOME: (&str, &str) = ("C", "minor");
/// The chord loops: the classic one first, then a minor blues turnaround and a darker i - iv - bII7 - V7.
pub const LOOPS: [&[&str]; 3] = [&super::LOOP, &["Cm", "Fm", "Cm", "G7"], &["Cm", "Fm", "Db7", "G7"]];
/// The tempo range it suits, in BPM.
pub const TEMPOS: (f64, f64) = (116.0, 150.0);

/// Which layers play where.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("drums", 2), ("bass", 2), ("comp", 1), ("pad", 1)],
    fill: &["drums"],
    breakdown: &[("bass", 1), ("pad", 2)],
    final_exchange: &[("drums", 3), ("bass", 3), ("comp", 3), ("pad", 2)],
    ability: &["comp", "drums"],
    victory: &[("drums", 1), ("bass", 2), ("comp", 1), ("pad", 2)],
    build: &[("drums", 3), ("bass", 2), ("comp", 2), ("pad", 2)],
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    let (kick, tap, rd, crash, sn, soft) = (
        synth::kick(0.5, 0.2, synth::KICK_F0, synth::KICK_F1),
        brush_tap(0.9, 3),
        ride(false),
        synth::crash(1.6),
        synth::snare(0.09),
        brush_tap(0.5, 5),
    );
    let kit = sound(move |_, m, _| {
        Ok(match m {
            KICK => scaled(&kick, 0.14),
            SNARE => scaled(&tap, 0.24),
            HAT => scaled(&rd, 0.09),
            CRASH => scaled(&crash, 0.08),
            CLAP => scaled(&sn, 0.06),
            SHAKER => scaled(&soft, 0.10),
            _ => Vec::new(),
        })
    });
    let bass = sound(|_, m, dur| Ok(upright(m, dur.clamp(0.3, 0.9), 1.0)));
    let piano = sound(|ctx, m, dur| {
        let x = ctx.note(PIANO, f64::from(m), round_nd(dur, 3), round_nd(dur + 0.25, 2), 0.6)?;
        Ok(synth::lp(&x, 2400.0, FILTER_O))
    });
    let pad = sound(|_, m, dur| Ok(dark_pad(&[m], dur + 0.05, 900.0)));
    vec![
        layer("drums", LayerKind::Drums, kit, "drums", 1.0, 0.0, (0, 0)),
        layer("bass", LayerKind::Bass, bass, "bass", 0.6, 0.0, (36, 48)),
        layer("comp", LayerKind::Comp, piano, "comp", 0.14, -0.2, (56, 75)),
        layer("pad", LayerKind::Pad, pad, "pad", 0.032, 0.0, (41, 63)),
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
