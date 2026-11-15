//! The prog duel's Composed arrangement (milestone 5, Task 5): the layer engine (`bf_sound::music::layers`) over
//! the theme's own instruments, its classic bed handing over every section the layers do not play.
//!
//! The rock kit with its toms and ride, the driven bass, and the same bass voice played high and harder as a
//! keyboard arpeggio. The fight drives on eighth-note bass; a desperation move leaves the arpeggio alone over a held
//! root; the final exchange is every layer at full density, toms included.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_sound::music::layers::{
    ArrangementTable, CLAP, CRASH, Cue, HAT, Instrument, KICK, Layer, LayerKind, LayeredBed, SHAKER, SNARE,
};
use bf_sound::music::{MusicCtx, Samples};
use bf_sound::synth;
use bf_sound::traits::Bed;

use super::{bass_note, hat, kick, ride, snare, tom};
use crate::ContentError;
use crate::themes::ThemeOptions;

/// The home key and scale.
pub const HOME: (&str, &str) = ("A", "minor");
/// The chord loops: the classic one first, then an epic i - bVI - bIII - bVII and a climbing i - bVII - bVI - V.
pub const LOOPS: [&[&str]; 3] = [&super::LOOP, &["Am", "F", "C", "G"], &["Am", "G", "F", "E7"]];
/// The tempo range it suits, in BPM.
pub const TEMPOS: (f64, f64) = (120.0, 150.0);

/// Which layers play where.
pub const TABLE: ArrangementTable = ArrangementTable {
    fight: &[("kit", 2), ("bass", 3), ("arp", 1)],
    fill: &["kit", "toms"],
    breakdown: &[("bass", 1), ("arp", 2)],
    final_exchange: &[("kit", 3), ("bass", 3), ("arp", 3), ("toms", 3)],
    ability: &["arp", "kit"],
    victory: &[("kit", 1), ("bass", 2), ("arp", 1)],
    build: &[("kit", 3), ("bass", 3), ("arp", 2), ("toms", 2)],
};

/// The layers.
pub fn layers() -> Vec<Layer> {
    let (k, s, h, crash, rd, t1, t2) = (
        kick(),
        snare(1),
        hat(false),
        synth::crash(1.6),
        ride(),
        tom(131),
        tom(98),
    );
    let t3 = t1.clone();
    let kit = sound(move |_, m, _| {
        Ok(match m {
            KICK => scaled(&k, 0.6),
            SNARE => scaled(&s, 0.4),
            HAT => scaled(&h, 0.09),
            CRASH => scaled(&crash, 0.08),
            CLAP => scaled(&t1, 0.25),
            SHAKER => scaled(&rd, 0.1),
            _ => Vec::new(),
        })
    });
    let toms = sound(move |_, m, _| {
        Ok(match m {
            SHAKER => scaled(&t2, 0.12),
            CLAP => scaled(&t3, 0.3),
            _ => Vec::new(),
        })
    });
    let bass = sound(|_, m, dur| Ok(bass_note(m, dur.min(0.9), 1.0, 1.2)));
    let arp = sound(|_, m, dur| Ok(bass_note(m, dur.min(0.5), 0.8, 2.0)));
    vec![
        layer("kit", LayerKind::Drums, kit, "drums", 1.0, 0.0, (0, 0)),
        layer("toms", LayerKind::Perc, toms, "drums", 1.0, 0.1, (0, 0)),
        layer("bass", LayerKind::Bass, bass, "bass", 0.42, 0.0, (33, 45)),
        layer("arp", LayerKind::Arp, arp, "bass", 0.16, 0.25, (57, 76)),
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
