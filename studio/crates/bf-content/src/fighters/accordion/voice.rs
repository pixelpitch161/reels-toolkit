//! ACCORDION's instrument: the free reeds of `series/instrument-history/accordion60_lowpoly.py` (`reed_wt`, `reed`)
//! in its `golden` tone (three reeds a few cents apart and the low octave: the full accordion #30 plays), computed
//! straight at 48,000 Hz (close enough for the ear, ruling M4-R16).
//!
//! [`AccordionVoice`] is `pp_bridge.voice('ACCORDION', midi, vel, dur)`: one golden reed note. The part (`sound.rs`)
//! builds its chords from [`reed`] directly, as #30's `audio.py` did. Each note's phases are its own generator's
//! (`default_rng(seed)`); the series' shared noise stream is not touched.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use bf_core::dmath::{self, PI};
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::voice::engine2::reed::ReedParts;
use bf_sound::voice::engine2::space::{Home, Space};
use bf_sound::voice::engine2::{Instrument, Parts};
use bf_sound::voice::real::{Correction, Fill, Rework};
use bf_sound::voice::{PpNoise, Voice, pp};

/// ACCORDION's register in `pp_bridge`'s roster.
pub const REGISTER: (i32, i32) = (55, 79);

/// `TONES['golden']`: `(cents, amp)` of each reed.
const GOLDEN: [(f64, f64); 4] = [(-9.0, 1.0), (0.0, 1.0), (9.0, 1.0), (-1200.0, 0.4)];
/// Its `tilt`.
const TILT: f64 = 1.05;
/// Its `nas`.
const NAS: f64 = 0.55;
/// Its `attack` (s).
const ATTACK: f64 = 0.012;
/// Its `rel` (s).
const REL: f64 = 0.05;

/// `reed_wt(K, tilt, nas)`: a 2,048-sample wavetable of `K` harmonics `k ** -tilt` with a nasal bump around the 4th,
/// normalised to a peak of 1.
fn reed_wt(kn: usize) -> Arc<Vec<f64>> {
    static MEMO: OnceLock<Mutex<HashMap<usize, Arc<Vec<f64>>>>> = OnceLock::new();
    let memo = MEMO.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(t) = memo.lock().expect("reed tables").get(&kn) {
        return t.clone();
    }
    let mut tb = vec![0.0; 2048];
    for k in 1..=kn {
        let kf = k as f64;
        let amp = dmath::pow(kf, -TILT) * (1.0 + NAS * dmath::exp(-((kf - 4.0) / 2.2) * ((kf - 4.0) / 2.2)));
        for (i, v) in tb.iter_mut().enumerate() {
            *v += amp * dmath::sin(2.0 * PI * kf * i as f64 / 2048.0);
        }
    }
    let peak = tb.iter().fold(0.0f64, |m, v| m.max(v.abs())).max(1e-12);
    let tb = Arc::new(tb.into_iter().map(|v| v / peak).collect::<Vec<f64>>());
    memo.lock().expect("reed tables").insert(kn, tb.clone());
    tb
}

/// `reed(f, dur, v, seed=seed, **TONES['golden'])` at 48 kHz: the golden reeds held `max(.04, dur)` with a short
/// attack, a release and the bellows' slow sway.
pub fn reed(f: f64, dur: f64, v: f64, seed: u64) -> Vec<f64> {
    let d = dur.max(0.04);
    let n = ((d + REL * 6.0 + 0.02) * SRF) as usize;
    let mut rg = Generator::new(seed);
    let mut x = vec![0.0; n];
    for (cents, a) in GOLDEN {
        let fr = f * dmath::pow(2.0, cents / 1200.0);
        let kn = (10000.0 / fr).clamp(1.0, 40.0) as usize;
        let tb = reed_wt(kn);
        let ph0 = rg.random();
        for (i, o) in x.iter_mut().enumerate() {
            let ph = (fr * i as f64 / SRF + ph0).rem_euclid(1.0);
            *o += a * tb[((ph * 2048.0) as usize) & 2047];
        }
    }
    let sway = seed as f64;
    for (i, o) in x.iter_mut().enumerate() {
        let t = i as f64 / SRF;
        let env = (t / ATTACK).min(1.0) * if t < d { 1.0 } else { dmath::exp(-(t - d) / REL) };
        let env = env * (1.0 + 0.05 * dmath::sin(2.0 * PI * 1.1 * t + sway));
        *o *= env * v * 0.3;
    }
    x
}

/// The ACCORDION's added overtones: the harmonics its own note barely has, laid in at a real accordion's level (dB
/// under the fundamental, `NONE` for none), note by note. Written with the [`CORRECTION`].
#[rustfmt::skip]
pub const FILL: &Fill = &[
    (55, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (56, [-99.0, -99.0, 2.5, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (57, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (58, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (59, [-99.0, -99.0, 6.4, -99.0, 0.4, -99.0, -6.1, -9.4]),
    (60, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (61, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (62, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (63, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (64, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (65, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (66, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (67, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (68, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (69, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (70, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (71, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (72, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (73, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (74, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (75, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (76, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (77, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (78, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
    (79, [-99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0, -99.0]),
];

/// The ACCORDION's overtone correction toward a real accordion: gains in dB for its first eight harmonics, note by
/// note (FluidR3, `bf-harness/tests/reference/fluidr3.json`). Written by
/// `cargo run --release -p bf-harness --example references -- correct "ACCORDION"`.
#[rustfmt::skip]
pub const CORRECTION: &Correction = &[
    (55, [-3.4, -6.4, 8.5, 0.0, 5.8, 1.6, 6.1, 5.2]),
    (56, [0.1, -0.7, 0.0, 1.4, 9.6, 6.2, 11.5, 10.4]),
    (57, [-4.4, -3.0, 2.8, -3.4, 4.3, 0.4, 4.0, 3.0]),
    (58, [-4.5, -2.9, 0.0, -6.2, 2.9, -2.5, 2.5, 0.4]),
    (59, [1.7, 13.6, 0.0, 14.2, 1.3, 14.3, 1.6, 1.3]),
    (60, [-5.6, -6.0, 8.3, -2.3, 5.9, -0.6, 3.2, 0.6]),
    (61, [0.0, 2.7, 7.5, -0.4, -1.1, 1.6, 1.3, -10.9]),
    (62, [0.0, -1.6, 8.6, -0.1, -1.5, 0.6, -2.4, -11.9]),
    (63, [0.0, -9.0, 3.7, -6.6, -6.9, -4.3, -9.0, -17.4]),
    (64, [0.0, -9.6, 0.8, -7.9, -8.0, -6.0, -11.6, -19.0]),
    (65, [0.0, -8.4, 3.1, -5.6, -7.1, -6.2, -9.8, -15.5]),
    (66, [0.0, -11.7, 2.6, -7.8, -8.5, -10.0, -10.2, -15.6]),
    (67, [0.0, -4.2, 8.2, -2.7, -5.0, -5.9, -3.0, -5.5]),
    (68, [0.0, -2.3, 9.3, -0.5, -5.2, -3.3, 0.3, -1.5]),
    (69, [0.0, -0.4, 7.8, -1.5, -6.6, -3.6, 2.5, 0.2]),
    (70, [0.0, 3.8, 9.2, -1.3, -6.1, -0.1, 6.0, 3.2]),
    (71, [0.0, -0.2, 9.7, -3.0, -6.1, 1.6, 6.3, 4.0]),
    (72, [0.0, -1.8, 5.4, -8.2, -9.2, 0.7, 4.8, 1.4]),
    (73, [0.0, -4.7, -0.1, -12.6, -13.4, -5.5, 1.2, -3.9]),
    (74, [0.0, -9.1, -1.8, -16.8, -14.9, -4.5, -1.7, -5.7]),
    (75, [0.0, 0.1, 4.8, -10.1, -5.4, 3.4, 6.6, 1.1]),
    (76, [0.0, 0.1, 2.6, -9.9, -3.4, 5.0, 7.6, 2.3]),
    (77, [0.0, -7.7, -4.5, -15.7, -8.0, 0.2, 1.7, -6.1]),
    (78, [0.0, -4.0, -1.1, -9.8, -1.0, 4.8, 8.5, -1.1]),
    (79, [0.0, -19.1, 3.2, -4.1, 4.8, -0.8, 1.2, -11.8]),
];

/// The Composed arrangement's ACCORDION (sound upgrade, stage 3, S1): its own note with the overtones of the real
/// instrument, the ones it lacks added; its own start and stop, already near the real one's.
pub const REWORK: Rework = Rework {
    fill: FILL,
    correction: CORRECTION,
    speak: 0.0,
    stop: 0.0,
    keep_shape: false,
    tail: &[],
};

/// ACCORDION's voice: one golden reed note.
#[derive(Clone, Copy, Debug, Default)]
pub struct AccordionVoice;

/// The Composed arrangement's ACCORDION (Voice Engine 2): built from how it works, on the reed engine (Aedan's pick
/// Ac1, the musette accordion: three reeds to a note, one a little flat and one a little sharp, so they beat; its bass
/// side's buttons for the low notes), with every technique it has, in a light room of its own
/// (`docs/studio/voice-engine-2-instruments.md`, "The reeds"). Classic keeps the note below.
pub static ENGINE2: Instrument = Instrument::built(
    "ACCORDION",
    Parts::Reed(ReedParts::AC1),
    Space {
        width: 0.0,
        tilt: 0.0,
        home: Some(Home {
            length: 2.6,
            under: 14.0,
            hp: 120.0,
        }),
        amp: None,
    },
);

impl Voice for AccordionVoice {
    fn engine2(&self) -> Option<&'static Instrument> {
        Some(&ENGINE2)
    }

    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        reed(pp::midi_hz(midi), dur, vel, (midi as i64 * 7).max(0) as u64)
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn rework(&self) -> Option<&'static Rework> {
        Some(&REWORK)
    }

    fn name(&self) -> &str {
        "ACCORDION"
    }

    /// The reeds hold `max(.04, dur)`: every `dur` but a NaN.
    fn can_hold(&self, dur: f64) -> bool {
        !dur.is_nan()
    }
}
