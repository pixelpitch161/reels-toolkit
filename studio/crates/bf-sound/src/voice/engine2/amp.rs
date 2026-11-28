//! An electric guitar's or a bass guitar's amp and speaker cabinet (`docs/studio/voice-engine-2-instruments.md`,
//! "Electric guitar", "Bass guitar"; the sketch `pl.py`, `amp_chain`): the pickups' signal through gain stages that
//! round off its peaks (each a valve's soft, a little lopsided clip, so a chord's notes beat into one another and a
//! held note grows thick), a tone stack, the power stage's own softer limit, then the speaker: its roll-off at both
//! ends and its few bumps. The gain stages run at four times the sample rate, so the overtones they make over the
//! top fold back as little as a real amp's.
//!
//! An amp plays on a fighter's whole sound, as a real one does (the Composed polish, `crate::polish`, through the
//! voice's [`super::space::Space`]): how hard it is driven is set from the sound's own loudness against the level the
//! listening rounds drove it at, so it distorts the same however loud the part is mixed, and comes out as loud as it
//! went in. [`run`] is the amp itself at a fixed drive, for a player that drives it live.

use bf_core::dmath;

use super::eq::{self, Band};
use crate::dsp::{self, FilterKind, butter_sos};
use crate::heard;

/// An amp's make-up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Amp {
    /// The lows taken out before the gain (Hz, first order), so the bass doesn't swamp it.
    pub tight: f64,
    /// The gain stages: each its gain and its lopsidedness (a bias on the clip).
    pub stages: &'static [(f64, f64)],
    /// The low-pass after each stage, Hz (first order).
    pub stage_lp: f64,
    /// The power stage's drive.
    pub power: f64,
    /// The tone stack.
    pub tone: &'static [Band],
    /// The cabinet's roll-off under (second order) and over (fourth order), Hz, and its bumps.
    pub cab_hp: f64,
    pub cab_lp: f64,
    pub cab: &'static [Band],
    /// The loudness the listening rounds drove it at (LUFS of its input, both sides alike).
    pub drive_lufs: f64,
}

impl Amp {
    /// The electric's (E2): a high-gain amp, its lows tight, its middle scooped, a 4x12 cabinet.
    pub const HIGH_GAIN: Amp = Amp {
        tight: 200.0,
        stages: &[(30.0, 0.25), (6.0, 0.15), (2.0, 0.0)],
        stage_lp: 8000.0,
        power: 2.0,
        tone: &[
            Band::Low {
                hz: 100.0,
                db: 3.0,
                q: 0.7,
            },
            Band::Peak {
                hz: 650.0,
                db: -3.0,
                q: 0.7,
            },
            Band::High {
                hz: 3500.0,
                db: 2.0,
                q: 0.7,
            },
        ],
        cab_hp: 70.0,
        cab_lp: 5000.0,
        cab: &[
            Band::Peak {
                hz: 105.0,
                db: 3.0,
                q: 1.2,
            },
            Band::Peak {
                hz: 2600.0,
                db: 2.5,
                q: 1.2,
            },
            Band::Peak {
                hz: 4300.0,
                db: -4.0,
                q: 2.0,
            },
        ],
        drive_lufs: HIGH_GAIN_LUFS,
    };
    /// The bass's (B3): a bass amp pushed a little, for grit.
    pub const BASS_GRIT: Amp = Amp {
        tight: 35.0,
        stages: &[(4.0, 0.15), (1.3, 0.05)],
        stage_lp: 9000.0,
        power: 1.4,
        tone: &[
            Band::Low {
                hz: 80.0,
                db: 2.0,
                q: 0.7,
            },
            Band::Peak {
                hz: 800.0,
                db: 2.0,
                q: 0.8,
            },
            Band::High {
                hz: 2500.0,
                db: 1.5,
                q: 0.7,
            },
        ],
        cab_hp: 38.0,
        cab_lp: 4500.0,
        cab: &[
            Band::Peak {
                hz: 75.0,
                db: 1.5,
                q: 1.0,
            },
            Band::Peak {
                hz: 1500.0,
                db: 2.0,
                q: 1.0,
            },
        ],
        drive_lufs: BASS_GRIT_LUFS,
    };
}

/// The loudness of the sketch riffs' amp inputs (the pickups' signal times the sketch's drive: 8 for E2, 3 for B3),
/// LUFS, measured by [`heard::integrated`].
const HIGH_GAIN_LUFS: f64 = 6.5;
const BASS_GRIT_LUFS: f64 = -0.6;
/// How many times the sample rate the gain stages run at.
const OVER: usize = 4;
/// A signal far under hearing (about -360 dB, at a quarter of the sample rate) added before each filter: where
/// the sound stops, a filter's memory would otherwise fade through the tiny numbers a processor works a hundred
/// times slower on, and a fight's quiet stretches would take longer than its notes.
const GUARD: f64 = 1e-18;
/// Under this a sound is silence: no amp.
const SILENT: f64 = -70.0;

/// `x` (mono) through amp `a` at sample rate `fs`, as it is: its level is the drive.
pub fn run(x: &[f64], a: &Amp, fs: f64) -> Vec<f64> {
    if x.is_empty() {
        return Vec::new();
    }
    let fo = fs * OVER as f64;
    let top = |hz: f64, rate: f64| hz.min(0.45 * rate);
    let mut y = guarded(dsp::resample_poly(x, OVER, 1));
    y = dsp::sosfilt(&butter_sos(FilterKind::Hp, &[a.tight], 1, fo), &y);
    let lp = butter_sos(FilterKind::Lp, &[top(a.stage_lp, fo)], 1, fo);
    for &(g, bias) in a.stages {
        let rest = dmath::tanh(bias);
        y.iter_mut().for_each(|v| *v = dmath::tanh(g * *v + bias) - rest);
        y = dsp::sosfilt(&lp, &guarded(y));
    }
    let mut y = dsp::resample_poly(&y, 1, OVER);
    y.truncate(x.len());
    let mut y = eq::apply(&guarded(y), a.tone, fs);
    y.iter_mut().for_each(|v| *v = dmath::tanh(a.power * *v) / a.power);
    let y = dsp::sosfilt(&butter_sos(FilterKind::Hp, &[a.cab_hp], 2, fs), &guarded(y));
    let y = dsp::sosfilt(&butter_sos(FilterKind::Lp, &[top(a.cab_lp, fs)], 4, fs), &guarded(y));
    eq::apply(&guarded(y), a.cab, fs)
}

/// `y` with [`GUARD`] added.
fn guarded(mut y: Vec<f64>) -> Vec<f64> {
    const QUARTER: [f64; 4] = [GUARD, 0.0, -GUARD, 0.0];
    y.iter_mut().zip(QUARTER.iter().cycle()).for_each(|(v, g)| *v += g);
    y
}

/// The loudness of one side of a sound, LUFS (as if both sides were it).
fn loudness(x: &[f64]) -> f64 {
    let both: Vec<[f64; 2]> = x.iter().map(|&v| [v, v]).collect();
    heard::integrated(&both)
}

/// One side of a bus through amp `a`: driven as the listening rounds drove it, whatever its level, and given back
/// its own loudness.
fn side(x: &[f64], a: &Amp, fs: f64) -> Vec<f64> {
    let level = loudness(x);
    if level <= SILENT {
        return x.to_vec();
    }
    let drive = heard::gain(a.drive_lufs - level);
    let driven: Vec<f64> = x.iter().map(|v| v * drive).collect();
    let mut y = run(&driven, a, fs);
    let out = loudness(&y);
    if out <= SILENT {
        return vec![0.0; x.len()];
    }
    let back = heard::gain(level - out);
    y.iter_mut().for_each(|v| *v *= back);
    y
}

/// A fighter's bus `x` through amp `a` at sample rate `fs`, each side on its own (a sound panned to one side is
/// driven as hard as in the middle, and stays where it was panned). Silence stays silence.
pub fn on_bus(x: &[[f64; 2]], a: &Amp, fs: f64) -> Vec<[f64; 2]> {
    if x.is_empty() {
        return Vec::new();
    }
    let channel = |c: usize| -> Vec<f64> { x.iter().map(|f| f[c]).collect() };
    let (l, r) = std::thread::scope(|s| {
        let l = s.spawn(|| side(&channel(0), a, fs));
        let r = s.spawn(|| side(&channel(1), a, fs));
        (
            l.join().expect("the amp's left side"),
            r.join().expect("the amp's right side"),
        )
    });
    l.into_iter().zip(r).map(|(l, r)| [l, r]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SRF;

    fn tone(hz: &[f64], level: f64, secs: f64) -> Vec<f64> {
        (0..(secs * SRF) as usize)
            .map(|i| {
                let t = i as f64 / SRF;
                hz.iter().map(|f| level * dmath::sin(dmath::TAU * f * t)).sum::<f64>()
            })
            .collect()
    }

    /// The level of the component at `f` over the middle second, dB.
    fn at_hz(x: &[f64], f: f64) -> f64 {
        let seg = &x[(0.5 * SRF) as usize..(1.5 * SRF) as usize];
        let w = dmath::TAU * f / SRF;
        let (mut re, mut im) = (0.0, 0.0);
        for (i, v) in seg.iter().enumerate() {
            let hann = 0.5 - 0.5 * dmath::cos(dmath::TAU * i as f64 / seg.len() as f64);
            re += v * hann * dmath::cos(w * i as f64);
            im += v * hann * dmath::sin(w * i as f64);
        }
        10.0 * dmath::log10((re * re + im * im).max(1e-30))
    }

    #[test]
    fn a_quiet_note_stays_near_clean_and_a_loud_one_distorts() {
        let a = Amp::HIGH_GAIN;
        // the 3rd harmonic against the note, quiet and loud
        let ratio = |level: f64| {
            let y = run(&tone(&[220.0], level, 2.0), &a, SRF);
            at_hz(&y, 660.0) - at_hz(&y, 220.0)
        };
        assert!(
            ratio(1.0) > ratio(0.001) + 15.0,
            "{} against {}",
            ratio(1.0),
            ratio(0.001)
        );
        let b = Amp::BASS_GRIT;
        let y = run(&tone(&[55.0], 1.0, 2.0), &b, SRF);
        assert!(at_hz(&y, 110.0) - at_hz(&y, 55.0) > -40.0);
    }

    #[test]
    fn two_notes_together_make_difference_tones() {
        // a fifth, 220 and 330 Hz: driven hard, 110 Hz comes out that neither note has
        let a = Amp::HIGH_GAIN;
        let y = run(&tone(&[220.0, 330.0], 0.5, 2.0), &a, SRF);
        let x = run(&tone(&[220.0], 0.5, 2.0), &a, SRF);
        assert!(at_hz(&y, 110.0) > at_hz(&x, 110.0) + 20.0);
    }

    #[test]
    fn a_bus_is_driven_the_same_however_loud_and_keeps_its_level_and_pan() {
        let a = Amp::HIGH_GAIN;
        let x = tone(&[196.0, 294.0], 0.3, 2.0);
        let bus = |g: f64, l: f64, r: f64| -> Vec<[f64; 2]> { x.iter().map(|v| [g * l * v, g * r * v]).collect() };
        let quiet = on_bus(&bus(0.01, 0.8, 0.6), &a, SRF);
        let loud = on_bus(&bus(1.0, 0.8, 0.6), &a, SRF);
        for (q, l) in quiet.iter().zip(&loud).skip(4800).step_by(97) {
            assert!((q[0] * 100.0 - l[0]).abs() < 1e-6 * l[0].abs().max(1e-3));
        }
        let level = |y: &[[f64; 2]], c: usize| loudness(&y.iter().map(|f| f[c]).collect::<Vec<_>>());
        let input = bus(1.0, 0.8, 0.6);
        for c in 0..2 {
            assert!((level(&loud, c) - level(&input, c)).abs() < 0.2);
        }
        // silence stays silence
        let silent = vec![[0.0; 2]; 4800];
        assert_eq!(on_bus(&silent, &a, SRF), silent);
        assert!(on_bus(&[], &a, SRF).is_empty());
    }

    #[test]
    fn a_long_silence_after_a_note_holds_no_slow_numbers() {
        // a loud note, then ten seconds of nothing: no sample may fade into the subnormal numbers
        let mut x = tone(&[110.0], 1.0, 1.0);
        x.resize(11 * 48_000, 0.0);
        for a in [Amp::HIGH_GAIN, Amp::BASS_GRIT] {
            let y = run(&x, &a, SRF);
            let slow = y.iter().filter(|v| **v != 0.0 && v.abs() < f64::MIN_POSITIVE).count();
            assert_eq!(slow, 0);
            assert!(y[2 * 48_000..].iter().all(|v| v.abs() < 1e-12));
        }
    }

    #[test]
    fn the_amp_runs_at_other_rates() {
        for fs in [44_100.0, 96_000.0] {
            let x: Vec<f64> = (0..(fs as usize))
                .map(|i| 0.5 * dmath::sin(dmath::TAU * 220.0 * i as f64 / fs))
                .collect();
            for a in [Amp::HIGH_GAIN, Amp::BASS_GRIT] {
                let y = run(&x, &a, fs);
                assert_eq!(y.len(), x.len());
                assert!(y.iter().all(|v| v.is_finite()));
            }
        }
    }
}
