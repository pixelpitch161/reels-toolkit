//! The stadium rock's Composed arrangement (milestone 5, Task 5): the layer engine (`bf_sound::music::layers`)
//! over the theme's own instruments, its classic bed handing over every section the layers do not play.
//!
//! The plugged band (the kick on its own bus, the kit, fuzz power chords chugging on eighths, the bass) with the
//! unplugged one's acoustic strum on the ACOUSTIC's voice laid over it. The fight is the full rock band; a desperation
//! move leaves the acoustic strum over a held bass; the final exchange is everything at full density; the victory
//! opens up under the solo.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_sound::music::layers::{
    ArrangementTable, CLAP, CRASH, Cue, HAT, Instrument, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE,
};
use bf_sound::music::{MusicCtx, Samples};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::{kick, pbass, pgtr, snare, tamb, tom};
use crate::ContentError;
use crate::themes::ThemeOptions;

/// The acoustic guitar's Pixel Pitch voice id, which the strum plays.
const ACOUSTIC: &str = "ACOUSTIC";

/// The home key and scale.
pub const HOME: (&str, &str) = ("E", "minor");
/// The chord loops: the classic one first, then an anthem i - bVI - bIII - bVII and a pop-rock i - bVII - bVI - V.
pub const LOOPS: [&[&str]; 3] = [&super::LOOP, &["Em", "C", "G", "D"], &["Em", "D", "C", "B7"]];
/// The tempo range it suits, in BPM.
pub const TEMPOS: (f64, f64) = (112.0, 140.0);

/// Which layers play where.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("kick", 2), ("kit", 2), ("gtr", 3), ("bass", 3), ("strum", 1)],
    fill: &["kit", "kick"],
    breakdown: &[("bass", 1), ("strum", 2)],
    final_exchange: &[("kick", 3), ("kit", 3), ("gtr", 3), ("bass", 3), ("strum", 2)],
    ability: &["gtr", "kit"],
    victory: &[("kick", 1), ("kit", 1), ("gtr", 2), ("bass", 2), ("strum", 2)],
    build: &[("kick", 3), ("kit", 3), ("gtr", 3), ("bass", 3)],
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    let k = kick();
    let kick = sound(move |_, m, _| Ok(if m == KICK { scaled(&k, 0.55) } else { Vec::new() }));
    let (s, h, crash, t, tb) = (snare(), synth::hat(0.03, false), synth::crash(1.8), tom(45), tamb());
    let kit = sound(move |_, m, _| {
        Ok(match m {
            SNARE => scaled(&s, 0.42),
            HAT => scaled(&h, 0.1),
            CRASH => scaled(&crash, 0.16),
            CLAP => scaled(&t, 0.22),
            SHAKER => scaled(&tb, 0.1),
            _ => Vec::new(),
        })
    });
    let gtr = sound(|_, m, dur| Ok(pgtr(m, dur.min(0.8), dur < 0.3, false)));
    let bass = sound(|_, m, dur| Ok(pbass(m, dur.min(0.9))));
    let strum = sound(|ctx, m, dur| {
        let x = ctx.note(ACOUSTIC, f64::from(m), dur.min(0.8), 0.5, 0.7)?;
        Ok(synth::hp(&x, 250.0, synth::FILTER_O))
    });
    vec![
        layer("kick", LayerKind::Drums, kick, "kick", 1.0, 0.0, (0, 0)),
        layer("kit", LayerKind::Drums, kit, "p_drums", 1.8, 0.0, (0, 0)),
        layer("gtr", LayerKind::Bass, gtr, "p_gtr", 0.3, 0.0, (40, 52)),
        layer("bass", LayerKind::Bass, bass, "p_bass", 0.42, 0.0, (28, 40)),
        layer("strum", LayerKind::Comp, strum, "u_gtr", 0.06, 0.2, (52, 71)),
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
