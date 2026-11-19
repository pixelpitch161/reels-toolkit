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

/// ACCORDION's voice: one golden reed note.
#[derive(Clone, Copy, Debug, Default)]
pub struct AccordionVoice;

impl Voice for AccordionVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        reed(pp::midi_hz(midi), dur, vel, (midi as i64 * 7).max(0) as u64)
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "ACCORDION"
    }

    /// The reeds hold `max(.04, dur)`: every `dur` but a NaN.
    fn can_hold(&self, dur: f64) -> bool {
        !dur.is_nan()
    }
}
