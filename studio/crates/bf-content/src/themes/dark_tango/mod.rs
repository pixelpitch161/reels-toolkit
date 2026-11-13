//! Theme: dark tango (`projects/ballfight/kit/moods/dark_tango.py`, #47 CELLO vs CONTRABASS). A dramatic minor tango
//! orquesta with no piano and no bass of its own (the fighters are the leads: the double bass's footfalls are the
//! walking line, the cello sings the melody): the bandoneon's marcato (a chord stab on every beat, two reeds beating,
//! a nasal bark, accents on 1 and 3; in the back half the 3-3-2 sincopa on odd bars and an arrastre, a dragged slide,
//! into every phrase); the strings' chop on a muted nylon guitar (a choked strum on 2 and 4, ghost strokes on 1 and 3
//! later); and no drum kit: the golpe (a knuckle on the bass's body), the chicharra (a bow scraped below the bridge)
//! and the latigo (a whip glissando). Built for D minor (Dm A7 Gm E7); any chord works (voicings by rule).
//!
//! **The tango stops.** On every stop a fighter asks for (a part's `stops()`: the CELLO's rope slams), the whole band
//! hits that beat and cuts until the next one, so the fighter's own crash rings alone. The score gathers the parts'
//! stops into [`bf_sound::music::MusicCtx::fighter_stops`]; the bed's `fight` sorts and filters them
//! ([`DarkTangoClassic::stop_times`]) and cuts its own buses ([`DarkTangoClassic::apply_stops`]).
//!
//! **The theme line.** Under the winner's solo the bandoneon takes up the line of the fighter whose role is
//! `melody` (the fight file's roles and lines: [`DarkTangoClassic::theme`]).
//!
//! Here: the theme's harmony helpers and its instruments as the kit computes them. The arrangement that reproduces
//! the kit's bed is [`DarkTangoClassic`], in `classic.rs`; it also holds the kit's per-bed cache of sounds (keyed
//! loosely: a duration is `round(dur, 3)`), through which the stab, the bellows and the chop are built from the
//! cached reeds and strings, as the kit builds them.
//!
//! **Voicings.** The mood has no table: [`voicing`] is its rule for every chord the harmony parses (the chord's tones
//! from D4 up, each a new note, a fourth an octave over the lowest when the chord has three).
//!
//! **The nylon string.** `pluck` is the Karplus-Strong loop of the bossa noir's nylon strings
//! ([`crate::themes::bossa_noir::karplus_strong`], scipy's `lfilter` to the bit), with this mood's own burst.

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::DarkTangoClassic;
use super::bossa_noir::{karplus_strong, loop_length};
use super::{Preset, Theme};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: no options, no voice of a fighter in its bed.
pub static THEME: Theme = Theme {
    name: "dark_tango",
    ported: true,
    card: &CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: |_| Ok(Box::new(DarkTangoClassic::new())),
};

/// #47's loop in D harmonic minor.
const LOOP: [&str; 8] = ["Dm", "A7", "A7", "Dm", "Gm", "Dm", "E7", "A7"];

/// What the interface says about the dark tango.
pub static CARD: ThemeCard = ThemeCard {
    display: "Dark tango",
    text: "A dramatic minor tango: the bandoneon's marcato and sincopa, a muted nylon chop, knuckle golpes and a \
           scraped chicharra. The band stops dead on a fighter's big hits. Leaves the melody and the bass to the \
           fighters.",
    key: "D",
    scale: "harmonic minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The kit configs that play the dark tango (Table S4): their music as each `episode.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_cello_vs_contrabass",
    fighters: ["CELLO", "CONTRABASS"],
    bpm: 118.0,
    key: "D",
    scale: "harmonic minor",
    intro: ["Dm", "A7"],
    chord_loop: &LOOP,
    tail: &["Gm", "Dm", "E7", "A7"],
}];

/// `voicing`'s default `lo` (D4).
pub const VOICING_LO: i32 = 62;
/// `root`'s default `lo` (D2).
pub const ROOT_LO: i32 = 38;
/// `reed`'s default `att`.
pub const REED_ATT: f64 = 0.006;
/// `reed`'s default `bark`.
pub const REED_BARK: f64 = 1.0;
/// `pluck`'s default `ring` (the chop never passes another).
pub const PLUCK_RING: f64 = 0.992;
/// `chop`'s default `dur`.
pub const CHOP_DUR: f64 = 0.12;

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `Bed.root(ch, lo)`: `T.root_above(ch, lo)`.
///
/// # Errors
/// Where Python raises: `ch` does not parse.
pub fn root(harmony: &Harmony, ch: &str, lo: i32) -> Result<i32, HarmonyError> {
    harmony.root_above(ch, lo)
}

/// `Bed.voicing(ch)`: the bandoneon's right-hand chord ([`voicing_from`] at D4).
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn voicing(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    voicing_from(harmony, ch, VOICING_LO)
}

/// `Bed.voicing(ch, lo)`: each of the chord's tones at or above `lo` in the chord's order, an octave up while that
/// note is taken; a fourth note an octave over the lowest when there are three; sorted, the lowest four.
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn voicing_from(harmony: &Harmony, ch: &str, lo: i32) -> Result<Vec<i32>, HarmonyError> {
    let mut out: Vec<i32> = Vec::new();
    for &pc in harmony.chord_pcs(ch)? {
        let mut m = lo + (pc - lo).rem_euclid(12);
        while out.contains(&m) {
            m += 12;
        }
        out.push(m);
    }
    if out.len() < 4 {
        // `min(out) + 12` (a chord has at least one tone)
        let low = *out.iter().min().expect("a chord's tones");
        out.push(low + 12);
    }
    out.sort_unstable();
    out.truncate(4);
    Ok(out)
}

/// The chop's chord (`chop`'s `vo`): `sorted(set(50 + (pc - 50) % 12 for pc in pcs))[:4]`, from D3 up within an
/// octave.
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn chop_voicing(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    let set: std::collections::BTreeSet<i32> = harmony
        .chord_pcs(ch)?
        .iter()
        .map(|&pc| 50 + (pc - 50).rem_euclid(12))
        .collect();
    Ok(set.into_iter().take(4).collect())
}

/// `Bed.reed(m, dur, 1.0, att, bark)` before the cache: one bandoneon note, two free reeds 7 cents apart (the
/// beating), each a sum of partials up to 7 kHz with a nasal formant at 1250 Hz and another at 2600 Hz; normalised;
/// a bellows envelope (the bark on the attack); low-passed at 6 kHz.
pub fn reed(m: i32, dur: f64, att: f64, bark: f64) -> Vec<f64> {
    let f = synth::mtof(f64::from(m));
    let n = length((dur + 0.08) * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    for (det, ph) in [(-3.5, 0.0), (3.5, 1.3)] {
        let ff = f * dmath::pow(2.0, det / 1200.0);
        for k in 1..40 {
            let kf = f64::from(k);
            let fk = ff * kf;
            if fk > 7000.0 {
                break;
            }
            // `1.0 / k ** 0.75 * (1 + 1.4 * math.exp(-(log(fk / 1250) / 0.35) ** 2) + 0.7 * math.exp(...))`
            let a = dmath::ln(fk / 1250.0) / 0.35;
            let b = dmath::ln(fk / 2600.0) / 0.3;
            let g = 1.0 / dmath::pow(kf, 0.75)
                * (1.0 + 1.4 * dmath::exp(-dmath::pow(a, 2.0)) + 0.7 * dmath::exp(-dmath::pow(b, 2.0)));
            // `2 * np.pi * fk * t + ph * k`
            let w = 2.0 * PI * fk;
            let p = ph * kf;
            for (o, &t) in x.iter_mut().zip(&t) {
                *o += g * dmath::sin(w * t + p);
            }
        }
    }
    // `x /= np.abs(x).max() + 1e-9`
    let peak = x
        .iter()
        .fold(f64::NEG_INFINITY, |a, v| if v.abs() > a { v.abs() } else { a });
    let d = peak + 1e-9;
    let y: Vec<f64> = x
        .iter()
        .zip(&t)
        .map(|(v, &t)| {
            let v = v / d;
            let hold = if t < dur {
                0.72 + 0.28 * dmath::exp(-t / 0.05) * bark
            } else {
                1.0
            };
            let mut env = clip(t / att, 0.0, 1.0) * hold;
            env *= if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.025) };
            v * env
        })
        .collect();
    synth::lp(&y, 6000.0, FILTER_O)
}

/// `Bed.pluck(m, dur, seed, bright, ring)` before the cache: a nylon string (Karplus-Strong): `N` normals from a new
/// generator of `2000 + m + 37 * seed`, low-passed at `1200 + 4000 * bright` Hz, through a tuned loop of `N` samples.
///
/// # Panics
/// Where Python raises: the note is shorter than its loop (`exc[:N] = ...` would not fit), or the seed is negative.
pub fn pluck(m: i32, dur: f64, seed: i64, bright: f64, ring: f64) -> Vec<f64> {
    let period = loop_length(m);
    let n = length(dur * SRF);
    assert!(n >= period, "pluck: {n} samples hold no loop of {period}");
    let burst_seed = 2000 + i64::from(m) + 37 * seed;
    let burst = noise(u64::try_from(burst_seed).expect("a noise seed"), period);
    let mut exc = vec![0.0; n];
    exc[..period].copy_from_slice(&synth::lp(&burst, 1200.0 + 4000.0 * bright, FILTER_O));
    karplus_strong(&exc, period, -0.5 * ring)
}

/// `Bed.golpe(1.0, seed)` before the cache: a knuckle on the bass's body, a falling wooden sine and a knock of
/// `90 + seed`'s band-passed noise, high-passed at 110 Hz.
pub fn golpe(seed: u64) -> Vec<f64> {
    let n = length(0.18 * SRF);
    let t = synth::tt(n);
    let knock = synth::bp(&noise(90 + seed, n), 350.0, 1400.0, FILTER_O);
    let x: Vec<f64> = (0..n)
        .map(|i| {
            let t = t[i];
            let body = dmath::sin(2.0 * PI * (150.0 + 60.0 * dmath::exp(-t / 0.01)) * t) * dmath::exp(-t / 0.045);
            let knock = knock[i] * dmath::exp(-t / 0.012);
            body * 0.9 + knock * 0.8
        })
        .collect();
    synth::hp(&x, 110.0, FILTER_O)
}

/// `Bed.chicharra(1.0, seed)` before the cache: a bow scraped below the bridge, `120 + seed`'s band-passed noise
/// gated by a 42 Hz grain.
pub fn chicharra(seed: u64) -> Vec<f64> {
    let n = length(0.13 * SRF);
    let t = synth::tt(n);
    let x = synth::bp(&noise(120 + seed, n), 1800.0, 6500.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            // `(np.sin(2 * np.pi * 42 * t) > 0.2).astype(float)`
            let grain = if dmath::sin(2.0 * PI * 42.0 * t) > 0.2 {
                1.0
            } else {
                0.0
            };
            x[i] * grain * clip(t / 0.008, 0.0, 1.0) * dmath::exp(-t / 0.06)
        })
        .collect()
}

/// `Bed.latigo(1.0)` before the cache: a whip glissando, a saw sweeping from 500 Hz to 3 kHz in 120 ms, low-passed
/// at 5 kHz.
pub fn latigo() -> Vec<f64> {
    let n = length(0.16 * SRF);
    let t = synth::tt(n);
    // `f = 500 * (6.0 ** np.clip(t / 0.12, 0, 1))`
    let f: Vec<f64> = t
        .iter()
        .map(|&t| 500.0 * dmath::pow(6.0, clip(t / 0.12, 0.0, 1.0)))
        .collect();
    let two_pi = 2.0 * PI;
    let saw: Vec<f64> = dsp::cumsum(&f)
        .iter()
        .map(|&c| {
            // `ph = 2 * np.pi * np.cumsum(f) / SR`; `2 * ((ph / (2 * np.pi)) % 1) - 1` (ph is never negative, so
            // numpy's remainder is the C fmod)
            let ph = two_pi * c / SRF;
            2.0 * ((ph / two_pi) % 1.0) - 1.0
        })
        .collect();
    synth::lp(&saw, 5000.0, FILTER_O)
        .iter()
        .zip(&t)
        .map(|(v, &t)| {
            // `np.maximum(t - 0.1, 0)`
            let late = t - 0.1;
            let late = if late >= 0.0 { late } else { 0.0 };
            v * clip(t / 0.01, 0.0, 1.0) * dmath::exp(-late / 0.02) * 0.6
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    fn harmony(chords: &[&str]) -> Harmony {
        let g = Grid::new(118.0);
        let all: Vec<String> = chords.iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2.min(all.len())], &all, &[], g.n_bars);
        Harmony::new(&g, "D", scale("harmonic minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    #[test]
    fn the_rule_voices_the_tango_chords() {
        let h = harmony(&["Dm", "A7", "Gm", "E7", "C"]);
        // D4 F4 A4, then D5 over the lowest
        assert_eq!(voicing(&h, "Dm").unwrap(), [62, 65, 69, 74]);
        // A4 C#5 E4 G4, sorted
        assert_eq!(voicing(&h, "A7").unwrap(), [64, 67, 69, 73]);
        assert_eq!(voicing(&h, "E7").unwrap(), [62, 64, 68, 71]);
        assert_eq!(chop_voicing(&h, "A7").unwrap(), [52, 55, 57, 61]);
        assert_eq!(chop_voicing(&h, "Dm").unwrap(), [50, 53, 57]);
        assert_eq!((root(&h, "Dm", 38).unwrap(), root(&h, "Gm", 45).unwrap()), (38, 55));
        assert_eq!(voicing(&h, "F#m"), Err(HarmonyError::UnknownChord("F#m".into())));
    }

    #[test]
    fn instrument_lengths_follow_pythons_int() {
        assert_eq!(reed(62, 0.42, REED_ATT, REED_BARK).len(), 24000);
        assert_eq!(pluck(50, 0.17, 0, 0.4, PLUCK_RING).len(), 8160);
        assert_eq!((golpe(0).len(), chicharra(0).len(), latigo().len()), (8640, 6240, 7680));
    }

    #[test]
    fn a_reed_is_normalised_before_its_envelope() {
        let x = reed(69, 0.4, REED_ATT, REED_BARK);
        let peak = x.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        assert!(peak > 0.5 && peak < 1.2, "{peak}");
    }
}
