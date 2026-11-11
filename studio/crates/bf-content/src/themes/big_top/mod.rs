//! Theme: sinister big top (`projects/ballfight/kit/moods/big_top.py`, #41 Flute vs Trombone). A dark circus
//! with no brass and no flute of its own: a band organ / calliope (breathy, slightly out-of-tune steam pipes)
//! chunking the chords on the off-beats, a concert bass drum on the "oom" (beats 1 and 3), a choked cymbal on 2
//! and 4, snare rolls into the sections, crashes on the phrase lines; the back half adds the march snare's 8ths,
//! a tambourine, a held band-organ chord and a glockenspiel glint. It leaves the bass register to a fighter (the
//! trombone in #41) and the top to the other (the flute). Built for E harmonic minor (Em Am B7 C).
//!
//! Here: the theme's voicings and its instruments, with the kit's names. The arrangement that reproduces the
//! kit's bed is [`BigTopClassic`], in `classic.rs`.
//!
//! **Voicings.** The kit has two tables, the calliope's off-beat chords ([`VOICE`]) and the held organ chord
//! ([`ORGAN`]), for the four chords of #41, and voices any other chord "by the same rule" (the chord's tones in
//! the calliope's octave; the tables are what the rule gives for #41's chords). [`voice`] and [`organ`] are
//! that, table first, then the rule. What a fight file may ask for is narrower: the registry lets through only
//! the chords the tables hold ([`has_voicing`]), as the plan of Milestone 1 has it; the kit itself would play
//! any chord it can parse.
//!
//! **Instruments.** Each is the kit's expression sample for sample. They take every argument of the kit's
//! method in its order (the defaults are the constants `<INSTRUMENT>_<ARGUMENT>`); `roll` takes the rate the kit
//! derives from the episode's 16th, and leaves out the kit's `seed`, which it never reads. Like the generators
//! of `bf_sound::synth` they panic where Python raises (a length of no samples handed to a filter).

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::harmony::{Harmony, HarmonyError};
use bf_core::py::py_max;
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::BigTopClassic;
use super::Theme;

/// The theme as the registry lists it.
pub static THEME: Theme = Theme {
    name: "big_top",
    voices: has_voicing,
    classic: || Box::new(BigTopClassic),
};

/// `VOICE`: the calliope's off-beat chords (E4 to C5: under the melody), as #41 voiced them.
pub const VOICE: [(&str, &[i32]); 4] = [
    ("Em", &[64, 67, 71]),
    ("Am", &[64, 69, 72]),
    ("B7", &[63, 66, 69, 71]),
    ("C", &[64, 67, 72]),
];

/// `ORGAN`: the back half's held organ chord, as #41 voiced it.
pub const ORGAN: [(&str, &[i32]); 4] = [
    ("Em", &[59, 64, 67]),
    ("Am", &[60, 64, 69]),
    ("B7", &[59, 63, 69]),
    ("C", &[60, 64, 67]),
];

/// `calliope`'s default velocity.
pub const CALLIOPE_VEL: f64 = 1.0;
/// `calliope`'s default noise seed.
pub const CALLIOPE_SEED: u64 = 0;
/// `calliope`'s default attack, in seconds.
pub const CALLIOPE_ATT: f64 = 0.008;
/// `calliope`'s default vibrato depth, as a fraction of the frequency.
pub const CALLIOPE_VIB: f64 = 0.004;
/// `roll`'s default velocity at its start.
pub const ROLL_V0: f64 = 0.4;
/// `roll`'s default velocity at its end.
pub const ROLL_V1: f64 = 1.0;

/// Whether the theme has a voicing written for `chord`: it is in both of the kit's tables.
pub fn has_voicing(chord: &str) -> bool {
    VOICE.iter().any(|(c, _)| *c == chord) && ORGAN.iter().any(|(c, _)| *c == chord)
}

fn table(of: &[(&str, &[i32])], chord: &str) -> Option<Vec<i32>> {
    of.iter().find(|(c, _)| *c == chord).map(|(_, notes)| notes.to_vec())
}

/// `Bed.voice(ch)`: the calliope's off-beat chord: `VOICE.get(ch) or tuple(T.chord_tones(63, 72, ch))`.
///
/// # Errors
/// Where Python raises: `chord` is in neither the table nor the episode's chords.
pub fn voice(harmony: &Harmony, chord: &str) -> Result<Vec<i32>, HarmonyError> {
    match table(&VOICE, chord) {
        Some(notes) => Ok(notes),
        None => harmony.chord_tones(63, 72, chord),
    }
}

/// `Bed.organ(ch)`: the held organ chord: the table's, else the chord's tones from B3 to A4, without the fifth
/// of a chord of four notes or more (`pcs[2]`).
///
/// # Errors
/// Where Python raises: `chord` is in neither the table nor the episode's chords.
pub fn organ(harmony: &Harmony, chord: &str) -> Result<Vec<i32>, HarmonyError> {
    if let Some(notes) = table(&ORGAN, chord) {
        return Ok(notes);
    }
    let pcs = harmony.chord_pcs(chord)?;
    let fifth = if pcs.len() > 3 { Some(pcs[2]) } else { None };
    Ok(harmony
        .chord_tones(59, 69, chord)?
        .into_iter()
        .filter(|m| Some(m.rem_euclid(12)) != fifth)
        .collect())
}

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `Bed.calliope(ms, dur, vel, seed, att, vib)`: the band organ / calliope: breathy steam pipes (a sine with a
/// little second and third harmonic), each pipe a hair out of tune, a fast shallow vibrato, chiff noise on the
/// attack. `ms` are the MIDI notes of the chord, `dur` the held length in seconds (the note is 60 ms longer).
///
/// One generator of `seed` gives, in this order, each pipe's detuning (a uniform draw), the chiff's noise and
/// the air's.
pub fn calliope(ms: &[i32], dur: f64, vel: f64, seed: u64, att: f64, vib: f64) -> Vec<f64> {
    let n = length((dur + 0.06) * SRF);
    let t = synth::tt(n);
    let mut rng = Generator::new(seed);
    let mut x = vec![0.0; n];
    for (j, &m) in ms.iter().enumerate() {
        let f = synth::mtof(f64::from(m)) * (1.0 + rng.uniform(-0.005, 0.005));
        let lag = 1.7 * j as f64;
        let fv: Vec<f64> = t
            .iter()
            .map(|&t| f * (1.0 + vib * dmath::sin(2.0 * PI * 6.3 * t + lag)))
            .collect();
        for (v, c) in x.iter_mut().zip(dsp::cumsum(&fv)) {
            let ph = 2.0 * PI * c / SRF;
            *v += dmath::sin(ph) + 0.28 * dmath::sin(2.0 * ph) + 0.10 * dmath::sin(3.0 * ph);
        }
    }
    // x /= max(1, len(ms))
    let pipes = ms.len().max(1) as f64;
    let mut draw = vec![0.0; n];
    rng.fill_normal(0.0, 1.0, &mut draw);
    let chiff = synth::bp(&draw, 1200.0, 5000.0, FILTER_O);
    rng.fill_normal(0.0, 1.0, &mut draw);
    let air = synth::bp(&draw, 800.0, 3000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let chiff = chiff[i] * dmath::exp(-t[i] / 0.015) * 0.25;
            let air = air[i] * 0.04;
            let env = clip(t[i] / att, 0.0, 1.0) * clip((dur + 0.06 - t[i]) / 0.06, 0.0, 1.0);
            (x[i] / pipes + chiff + air) * env * vel
        })
        .collect()
}

/// `Bed.bass_drum(vel)`: the concert bass drum (the "oom"): a soft boom, over in about 0.2 s and high-passed so
/// a bass line stays on top.
pub fn bass_drum(vel: f64) -> Vec<f64> {
    let x = synth::kick(0.7, 0.2, 105.0, 50.0);
    synth::hp(&x[..length(0.36 * SRF)], 46.0, FILTER_O)
        .iter()
        .map(|v| v * vel)
        .collect()
}

/// `Bed.choke(vel, seed)`: the "pah" cymbal: a crash choked by hand after about 0.12 s.
pub fn choke(vel: f64, seed: u64) -> Vec<f64> {
    let n = length(0.22 * SRF);
    let t = synth::tt(n);
    let hiss = synth::hp(&noise(seed, n), 4200.0, FILTER_O);
    (0..n)
        .map(|i| {
            let mut x = hiss[i] * dmath::exp(-t[i] / 0.05);
            x += 0.15 * dmath::sin(2.0 * PI * 4130.0 * t[i]) * dmath::exp(-t[i] / 0.06);
            x * clip((0.2 - t[i]) / 0.06, 0.0, 1.0) * vel
        })
        .collect()
}

/// `Bed.snare(vel)`: `A.snare(0.13) * vel`.
pub fn snare(vel: f64) -> Vec<f64> {
    synth::snare(0.13).iter().map(|v| v * vel).collect()
}

/// `Bed.roll(dur, v0, v1, rate)`: a snare roll of `dur` seconds, a stroke every `rate` seconds (the kit's
/// default is a 32nd of the episode, `T.S16 / 2`), growing from `v0` to `v1`; every other stroke is lighter.
///
/// # Panics
/// When `rate` is not above zero (the kit would never stop adding strokes).
pub fn roll(dur: f64, v0: f64, v1: f64, rate: f64) -> Vec<f64> {
    assert!(rate > 0.0, "roll: a stroke every {rate} s");
    let n = length((dur + 0.15) * SRF);
    let mut out = vec![0.0; n];
    let stroke = synth::snare(0.13);
    let span = py_max(dur, 1e-6);
    let mut k = 0usize;
    while (k as f64) * rate < dur - 1e-6 {
        let v = v0 + (v1 - v0) * (k as f64 * rate / span);
        // self.snare(0.6 + 0.4 * ((k % 2) == 0)) * v
        let vel = 0.6 + 0.4 * if k.is_multiple_of(2) { 1.0 } else { 0.0 };
        let a = length(k as f64 * rate * SRF);
        let b = n.min(a + stroke.len());
        for (o, s) in out[a.min(b)..b].iter_mut().zip(&stroke) {
            *o += s * vel * v;
        }
        k += 1;
    }
    out
}

/// `Bed.tambourine(vel, seed)`: a short shake: band-passed noise over three high partials.
pub fn tambourine(vel: f64, seed: u64) -> Vec<f64> {
    let n = length(0.12 * SRF);
    let t = synth::tt(n);
    let shake = synth::bp(&noise(seed, n), 5500.0, 12000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let mut x = shake[i] * dmath::exp(-t[i] / 0.04);
            // Python's `sum(...)` over the partials starts from 0.
            let mut jingle = 0.0;
            for f in [6300.0, 7900.0, 9100.0] {
                jingle += dmath::sin(2.0 * PI * f * t[i]);
            }
            x += 0.2 * jingle * dmath::exp(-t[i] / 0.05);
            x * vel
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// A harmony whose bars hold every chord of `chords` (the intro and the loop are filled with them).
    fn harmony(chords: &[&str]) -> Harmony {
        let g = Grid::new(132.0);
        let all = names(chords);
        let bars = chords_for(&all[..2.min(all.len())], &all, g.n_bars);
        Harmony::new(&g, "E", scale("harmonic minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    /// The kit's tables are what its rule gives for #41's chords; for any other chord the rule speaks. As the
    /// kit printed `bed.voice(ch)` and `bed.organ(ch)` (with the chord added to `T.CHORD_PCS`).
    #[test]
    fn voicings_are_the_kits_tables_then_its_rule() {
        #[rustfmt::skip]
        let cases: [(&str, &[i32], &[i32]); 22] = [
            ("Em", &[64, 67, 71], &[59, 64, 67]),
            ("Am", &[64, 69, 72], &[60, 64, 69]),
            ("B7", &[63, 66, 69, 71], &[59, 63, 69]),
            ("C", &[64, 67, 72], &[60, 64, 67]),
            ("F#m", &[66, 69], &[61, 66, 69]),
            ("D", &[66, 69], &[62, 66, 69]),
            ("G", &[67, 71], &[59, 62, 67]),
            ("Gm", &[67, 70], &[62, 67]),
            ("Eb", &[63, 67, 70], &[63, 67]),
            ("Cm", &[63, 67, 72], &[60, 63, 67]),
            ("Bbm", &[65, 70], &[61, 65]),
            ("Db", &[65, 68], &[61, 65, 68]),
            ("G7", &[65, 67, 71], &[59, 65, 67]),
            ("Ab7", &[63, 66, 68, 72], &[60, 66, 68]),
            ("F", &[65, 69, 72], &[60, 65, 69]),
            ("E", &[64, 68, 71], &[59, 64, 68]),
            ("Bdim7", &[65, 68, 71], &[59, 62, 68]),
            ("Dm7b5", &[65, 68, 72], &[60, 62, 65]),
            ("Cmaj7", &[64, 67, 71, 72], &[59, 60, 64]),
            ("A6", &[64, 66, 69], &[61, 66, 69]),
            ("Esus4", &[64, 69, 71], &[59, 64, 69]),
            ("Caug", &[64, 68, 72], &[60, 64, 68]),
        ];
        let all: Vec<&str> = cases.iter().map(|c| c.0).collect();
        let h = harmony(&all);
        for (chord, v, o) in cases {
            assert_eq!(voice(&h, chord).unwrap(), v, "voice({chord})");
            assert_eq!(organ(&h, chord).unwrap(), o, "organ({chord})");
        }
        // The tables answer without the episode's chords; the rule needs the chord to be one of them.
        let plain = harmony(&["Em", "B7"]);
        assert_eq!(voice(&plain, "C").unwrap(), [64, 67, 72]);
        assert_eq!(organ(&plain, "Am").unwrap(), [60, 64, 69]);
        assert_eq!(voice(&plain, "F#m"), Err(HarmonyError::UnknownChord("F#m".into())));
        assert_eq!(organ(&plain, "F#m"), Err(HarmonyError::UnknownChord("F#m".into())));
        // A fight file may ask for the tables' chords only.
        for (chord, _, _) in cases {
            assert_eq!(has_voicing(chord), ["Em", "Am", "B7", "C"].contains(&chord), "{chord}");
        }
        assert!(!has_voicing("") && !has_voicing("em"));
    }

    #[test]
    fn instrument_lengths_follow_pythons_int() {
        // int((dur + 0.06) * SR)
        assert_eq!(
            calliope(&[64, 67, 71], 0.13, 1.0, 0, CALLIOPE_ATT, CALLIOPE_VIB).len(),
            9120
        );
        assert_eq!(calliope(&[], 0.3, 1.0, 7, CALLIOPE_ATT, CALLIOPE_VIB).len(), 17280);
        assert_eq!(bass_drum(1.0).len(), 17280);
        assert_eq!(choke(1.0, 0).len(), 10560);
        assert_eq!(snare(1.0).len(), 16800);
        assert_eq!(tambourine(1.0, 0).len(), 5760);
        // int((dur + 0.15) * SR)
        assert_eq!(roll(0.5, ROLL_V0, ROLL_V1, 0.05).len(), 31200);
    }

    #[test]
    fn the_calliope_is_its_pipes_under_an_envelope() {
        let (ms, dur) = ([64, 67, 71], 0.13);
        let x = calliope(&ms, dur, 1.0, 3, CALLIOPE_ATT, CALLIOPE_VIB);
        // It starts from silence and ends in it (the release is the last 60 ms).
        assert_eq!((x[0], x[x.len() - 1].abs() < 1e-3), (0.0, true));
        assert!(x.iter().all(|v| v.is_finite()) && x.iter().any(|v| v.abs() > 0.3));
        // The velocity scales it; the seed changes the detuning and the breath.
        let soft = calliope(&ms, dur, 0.5, 3, CALLIOPE_ATT, CALLIOPE_VIB);
        assert!(x.iter().zip(&soft).all(|(a, b)| (a * 0.5 - b).abs() < 1e-15));
        assert!(calliope(&ms, dur, 1.0, 4, CALLIOPE_ATT, CALLIOPE_VIB) != x);
        // No pipes: the chiff and the air alone (Python divides by `max(1, len(ms))`).
        let breath = calliope(&[], dur, 1.0, 3, CALLIOPE_ATT, CALLIOPE_VIB);
        assert!(breath.iter().all(|v| v.is_finite()) && breath.iter().any(|v| *v != 0.0));
        assert!(breath.iter().all(|v| v.abs() < 0.5));
    }

    #[test]
    fn a_roll_grows_and_alternates() {
        let stroke = synth::snare(0.13);
        let rate = 0.1;
        let x = roll(0.3, 0.4, 1.0, rate);
        // Strokes at 0, 0.1 and 0.2 s (not at 0.3: `k * rate < dur - 1e-6`), at 0.4, 0.6 and 0.8 of the way up,
        // the odd one lighter.
        let at = |k: usize, i: usize| {
            stroke[i]
                * (0.6 + 0.4 * if k.is_multiple_of(2) { 1.0 } else { 0.0 })
                * (0.4 + 0.6 * (k as f64 * rate / 0.3))
        };
        assert_eq!(x[100], at(0, 100));
        assert_eq!(x[4800 + 100], at(0, 4900) + at(1, 100));
        assert_eq!(x[9600 + 100], at(0, 9700) + at(1, 4900) + at(2, 100));
        // The first stroke has rung out by 0.36 s; no fourth stroke starts at 0.3 s.
        assert_eq!(x[17000], at(1, 17000 - 4800) + at(2, 17000 - 9600));
        // int((0.3 + 0.15) * 48000): the sum is a hair under 0.45.
        assert_eq!(x.len(), 21599);
    }

    #[test]
    #[should_panic(expected = "roll: a stroke every 0 s")]
    fn a_roll_needs_a_rate() {
        roll(0.3, 0.4, 1.0, 0.0);
    }
}
