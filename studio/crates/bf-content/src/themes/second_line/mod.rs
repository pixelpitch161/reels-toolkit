//! Theme: second line (`projects/ballfight/kit/moods/second_line.py`, #48 EUPHONIUM vs TROMBONE). A dark minor New
//! Orleans brass-band street groove with no horns of its own (the two brass fighters split the front line): the
//! parade bass drum with a cymbal on top (a syncopated two-bar figure, the cymbal's chick on every beat with its accent
//! on 2 and 4), the parade snare's stick shots on 2 and 4, swung ghost notes and press rolls into the phrases; the
//! sousaphone (the TUBA's Pixel Pitch voice) low and simple; and in the back half a tambourine on the off-beats and
//! hand claps on 2 and 4. The 16ths swing ([`SWING16`]). Built for E minor (Em Am Em C7 | B7 Em Am B7); any chord works
//! (roots and fifths by rule).
//!
//! Here: the theme's harmony helpers and its instruments as the kit computes them. The arrangement that reproduces
//! the kit's bed is [`SecondLineClassic`], in `classic.rs`; it also holds the kit's per-bed cache of sounds (keyed
//! loosely: a snare's noise is its seed modulo 4, a tambourine's and a clap's modulo 3, a press roll's length and
//! crescendo rounded to 3 decimals).
//!
//! **Voicings.** The mood has no method that voices one chord: the sousaphone plays [`root`] and [`fifth`], both
//! numbers, so the theme has no voicing vectors.
//!
//! **The TUBA's voice.** The sousaphone is `M.note('TUBA', ...)` with the mood's own rounding (see
//! `SecondLineClassic::sousa`): the TUBA fighter's ported voice (`bf_content::fighters::tuba`), handed to the music
//! context through [`Theme::voices`], never the TUBA's part.

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_core::py::py_max;
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::synth::{self, FILTER_O, clip};

pub use self::classic::SecondLineClassic;
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options; its bed plays the TUBA's voice (the sousaphone).
pub static THEME: Theme = Theme {
    name: "second_line",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[TUBA],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(SecondLineClassic::new())),
};

/// The Pixel Pitch voice the sousaphone plays (`M.note('TUBA', ...)`).
pub const TUBA: &str = "TUBA";

/// #48's loop in E (harmonic) minor.
const LOOP: [&str; 8] = ["Em", "Am", "Em", "C7", "B7", "Em", "Am", "B7"];

/// What the interface says about the second line.
pub static CARD: ThemeCard = ThemeCard {
    display: "Second line",
    text: "A dark New Orleans second line: the parade bass drum with a cymbal on top, the snare's ghost notes and \
           press rolls, a low sousaphone, a tambourine and claps in the back half. No horns of its own: leaves the \
           leads to the fighters.",
    key: "E",
    scale: "harmonic minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The kit configs that play the second line (Table S4): their music as each `episode.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_euphonium_vs_trombone",
    fighters: ["EUPHONIUM", "TROMBONE"],
    bpm: 122.0,
    key: "E",
    scale: "harmonic minor",
    intro: ["Em", "B7"],
    chord_loop: &LOOP,
    tail: &["Em", "Am", "Em", "C7", "B7"],
}];

/// `SWING16`: the off 16th at 60 % of the 8th (the second line's swung 16ths).
pub const SWING16: f64 = 0.60;
/// `BD_A`: the bass drum in bar A (even bars): 1, the and of 2, 4 (16ths).
pub const BD_A: [u32; 3] = [0, 6, 12];
/// `BD_B`: in bar B (odd bars): 1, the and of 2, 3, the and of 4 (pushing into the bar line).
pub const BD_B: [u32; 4] = [0, 6, 8, 14];
/// `GH_A`: the snare's ghost notes (16ths) in bar A.
pub const GH_A: [u32; 4] = [2, 7, 9, 15];
/// `GH_B`: in bar B.
pub const GH_B: [u32; 4] = [3, 6, 9, 11];
/// `GH_X`: the extra ghosts of the back half.
pub const GH_X: [u32; 5] = [1, 5, 10, 13, 14];

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `default_rng(base + seed)`.
///
/// # Panics
/// On a seed below zero (numpy refuses one; the bed checks its seeds before it makes a sound).
fn rng(base: i64, seed: i64) -> Generator {
    let s = base + seed;
    Generator::new(u64::try_from(s).unwrap_or_else(|_| panic!("a noise seed of {s}: numpy refuses a negative seed")))
}

/// `Bed.root(ch)`: the sousaphone's root, `T.root_above(ch, 36)` (C2 to B2).
///
/// # Errors
/// Where Python raises: `ch` does not parse.
pub fn root(harmony: &Harmony, ch: &str) -> Result<i32, HarmonyError> {
    harmony.root_above(ch, 36)
}

/// `Bed.fifth(ch)`: the root plus the chord's fifth (its third pitch class above its first, modulo 12; 7 for a
/// chord of fewer than three), an octave down when it would be above D3 (50).
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn fifth(harmony: &Harmony, ch: &str) -> Result<i32, HarmonyError> {
    let r = root(harmony, ch)?;
    let pcs = harmony.chord_pcs(ch)?;
    let f = r + if pcs.len() > 2 {
        (pcs[2] - pcs[0]).rem_euclid(12)
    } else {
        7
    };
    Ok(if f <= 50 { f } else { f - 12 })
}

/// `np.where(t < dur, 1.0, np.exp(-(t - dur) / rel))`.
fn release(t: f64, dur: f64, rel: f64) -> f64 {
    if t < dur { 1.0 } else { dmath::exp(-(t - dur) / rel) }
}

/// `Bed.bass_drum(1.0, big)` before the cache: the parade bass drum, a deep loose head, high-passed at 38 Hz so the
/// sousaphone stays on top, then cut to 0.6 s (big) or 0.4 s.
pub fn bass_drum(big: bool) -> Vec<f64> {
    let x = if big {
        synth::kick(0.75, 0.30, 95.0, 46.0)
    } else {
        synth::kick(0.55, 0.22, 95.0, 46.0)
    };
    let mut y = synth::hp(&x, 38.0, FILTER_O);
    y.truncate(length(if big { 0.6 } else { 0.4 } * SRF));
    y
}

/// `Bed.chick(1.0, accent, seed)` before the cache: the cymbal on top of the bass drum, seven inharmonic partials
/// and `21 + seed`'s band-passed noise, choked (70 ms) or let ring on the accents (220 ms).
pub fn chick(accent: bool, seed: i64) -> Vec<f64> {
    const PARTS: [f64; 7] = [431.0, 1093.0, 2239.0, 3517.0, 4871.0, 6907.0, 8123.0];
    let n = length(0.5 * SRF);
    let t = synth::tt(n);
    // Python's `sum` starts from 0; `0.86 ** i` is a Python float to an int power (C's pow).
    let mut x = vec![0.0; n];
    for (i, f) in PARTS.into_iter().enumerate() {
        let i = i as f64;
        let a = dmath::pow(0.86, i);
        for (o, &t) in x.iter_mut().zip(&t) {
            *o += dmath::sin(2.0 * PI * f * t + 1.7 * i) * a;
        }
    }
    let k = PARTS.len() as f64;
    let mut z = vec![0.0; n];
    rng(21, seed).fill_normal(0.0, 1.0, &mut z);
    let noise = synth::bp(&z, 3000.0, 11000.0, FILTER_O);
    let dec = if accent { 0.22 } else { 0.07 };
    (0..n)
        .map(|i| {
            let t = t[i];
            (x[i] / k * 0.7 + noise[i] * 0.8) * dmath::exp(-t / dec) * clip(t / 0.002, 0.0, 1.0)
        })
        .collect()
}

/// `Bed.snare(1.0, ghost, seed)` before the cache: a parade snare, a stick shot (head and snares) or a ghost tap,
/// from `50 + seed`'s band-passed noise and two sines of the drum's body.
pub fn snare(ghost: bool, seed: i64) -> Vec<f64> {
    let n = length(0.25 * SRF);
    let t = synth::tt(n);
    let mut z = vec![0.0; n];
    rng(50, seed).fill_normal(0.0, 1.0, &mut z);
    let wires = synth::bp(&z, 1800.0, 9000.0, FILTER_O);
    let (dec, w, b, g) = if ghost {
        (0.045, 0.8, 0.25, 0.32)
    } else {
        (0.12, 1.0, 0.7, 1.0)
    };
    (0..n)
        .map(|i| {
            let t = t[i];
            let wires = wires[i] * dmath::exp(-t / dec);
            let body =
                (dmath::sin(2.0 * PI * 186.0 * t) + 0.5 * dmath::sin(2.0 * PI * 331.0 * t)) * dmath::exp(-t / 0.05);
            let x = wires * w + body * b;
            x * clip(t / 0.001, 0.0, 1.0) * g
        })
        .collect()
}

/// `Bed.press(dur, v, seed, v0)` before the cache: a snare press roll, a buzz of `60 + seed`'s band-passed noise
/// under a rattle (its phase the generator's next draw, a uniform one) with the drum's body under it. With `v0` it is
/// a crescendo from `v0` to `v` (`v0 / max(v, 1e-6)` of the way up at its start), else an accent on its front.
pub fn press(dur: f64, v: f64, seed: i64, v0: Option<f64>) -> Vec<f64> {
    let n = length((dur + 0.08) * SRF);
    let t = synth::tt(n);
    let mut r = rng(60, seed);
    let mut z = vec![0.0; n];
    r.fill_normal(0.0, 1.0, &mut z);
    let buzz = synth::bp(&z, 1600.0, 9000.0, FILTER_O);
    let phase = r.uniform(0.0, 3.0);
    let rise = v0.map(|v0| v0 / py_max(v, 1e-6));
    (0..n)
        .map(|i| {
            let t = t[i];
            let rattle = 0.55 + 0.45 * dmath::sin(PI * 30.0 * t + phase).abs();
            let body = dmath::sin(2.0 * PI * 186.0 * t) * dmath::exp(-t / 0.05) * 0.5;
            let env = clip(t / 0.004, 0.0, 1.0) * release(t, dur, 0.03);
            let env = match rise {
                None => env * (0.55 + 0.45 * dmath::exp(-t / 0.06)),
                Some(a) => env * (a + (1.0 - a) * dmath::pow(clip(t / dur, 0.0, 1.05), 1.4)),
            };
            (buzz[i] * rattle + body) * env
        })
        .collect()
}

/// `Bed.tambourine(1.0, seed)` before the cache: four jingle sines and `70 + seed`'s high band of noise.
pub fn tambourine(seed: i64) -> Vec<f64> {
    let n = length(0.18 * SRF);
    let t = synth::tt(n);
    // Python's `sum` starts from 0.
    let mut jingle = vec![0.0; n];
    for (i, f) in [5100.0, 6300.0, 7400.0, 8900.0].into_iter().enumerate() {
        let i = i as f64;
        for (o, &t) in jingle.iter_mut().zip(&t) {
            *o += dmath::sin(2.0 * PI * f * t + i);
        }
    }
    let mut z = vec![0.0; n];
    rng(70, seed).fill_normal(0.0, 1.0, &mut z);
    let noise = synth::bp(&z, 5000.0, 12000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            (jingle[i] / 4.0 * 0.5 + noise[i]) * dmath::exp(-t / 0.05) * clip(t / 0.002, 0.0, 1.0)
        })
        .collect()
}

/// `Bed.clap(1.0, seed)` before the cache: a few hands clapping together, three band-passed bursts of `90 + seed`'s
/// noise a few ms apart (every draw from the one generator, in the kit's order).
pub fn clap(seed: i64) -> Vec<f64> {
    let n = length(0.2 * SRF);
    let t = synth::tt(n);
    let mut r = rng(90, seed);
    let mut x = vec![0.0; n];
    for (j, d) in [0.0, 0.007, 0.016].into_iter().enumerate() {
        let a = length(d * SRF);
        let mut z = vec![0.0; n - a];
        r.fill_normal(0.0, 1.0, &mut z);
        let burst = synth::bp(&z, 900.0, 4500.0, FILTER_O);
        let dec = if j < 2 { 0.012 } else { 0.04 };
        let g = if j < 2 { 0.8 } else { 1.0 };
        // `A.tt(n - a)` starts again from 0 at the burst's first sample.
        for (i, (o, v)) in x[a..].iter_mut().zip(&burst).enumerate() {
            *o += v * dmath::exp(-t[i] / dec) * g;
        }
    }
    x.iter().map(|v| v * 0.6).collect()
}

/// `title_rest`'s cymbal swell: `A.hp(default_rng(5).normal(0, 1, n), 5000) * np.linspace(0.2, 1.0, n) ** 2`
/// (numpy squares by a product).
pub fn swell(n: usize) -> Vec<f64> {
    let x = synth::hp(&synth::noise(5, n), 5000.0, FILTER_O);
    let ramp = bf_sound::dsp::linspace(0.2, 1.0, n);
    x.iter().zip(&ramp).map(|(v, r)| v * (r * r)).collect()
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    fn harmony(chords: &[&str]) -> Harmony {
        let g = Grid::new(122.0);
        let all: Vec<String> = chords.iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2.min(all.len())], &all, &[], g.n_bars);
        Harmony::new(&g, "E", scale("harmonic minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    #[test]
    fn roots_and_fifths_by_rule() {
        let h = harmony(&["Em", "B7", "Am", "C7"]);
        // E2, B2, A2, C2
        assert_eq!(
            [root(&h, "Em"), root(&h, "B7"), root(&h, "Am"), root(&h, "C7")].map(Result::unwrap),
            [40, 47, 45, 36]
        );
        // B2; F#3 (54) is over D3, so F#2; E3 (52), so E2; G2
        assert_eq!(
            [fifth(&h, "Em"), fifth(&h, "B7"), fifth(&h, "Am"), fifth(&h, "C7")].map(Result::unwrap),
            [47, 42, 40, 43]
        );
        assert_eq!(fifth(&h, "F#m"), Err(HarmonyError::UnknownChord("F#m".into())));
    }

    #[test]
    fn instrument_lengths_follow_pythons_int() {
        // the big drum's kick is shorter than its 0.6 s cut, so the slice keeps all of it
        assert_eq!(synth::kick(0.75, 0.30, 95.0, 46.0).len(), 24000);
        assert_eq!((bass_drum(false).len(), bass_drum(true).len()), (19200, 24000));
        assert_eq!((chick(false, 0).len(), snare(true, 3).len()), (24000, 12000));
        assert_eq!((tambourine(1).len(), clap(2).len()), (8640, 9600));
        assert_eq!(press(0.2, 1.0, 6, Some(0.25)).len(), 13440);
        assert_eq!(swell(100).len(), 100);
    }

    #[test]
    fn a_press_roll_with_v0_grows() {
        let x = press(0.4, 1.0, 1, Some(0.3));
        let rms = |a: &[f64]| (a.iter().map(|v| v * v).sum::<f64>() / a.len() as f64).sqrt();
        assert!(rms(&x[480..2400]) < rms(&x[16000..18000]));
        // the noise is the seed's: another seed is another roll
        assert_ne!(press(0.4, 1.0, 2, Some(0.3)), x);
    }
}
