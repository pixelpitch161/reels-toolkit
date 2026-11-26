//! Voice Engine 2's built voices (`docs/studio/2026-11-26-voice-engine-2-design.md`, "Revision: built engines"):
//! PIANO on the struck-string engine and ORGAN on the pipe engine, as the Composed arrangement plays them, held to
//! what Aedan picked by ear (P16 and O8, 2026-11-26) and to the real instruments' range.
//!
//! - The piano's riff (the listening rounds' own) has its octave bands inside the range of four real pianos (VCSL
//!   Steinway B, Splendid Grand, Salamander, Headroom), each measured the same way, give or take 1.5 dB.
//! - The riff's tune an octave up keeps its energy over 4 kHz low: the high notes Aedan heard as "very
//!   synthesized" (P12) had 9.5 dB under the whole there, the real Steinway 32.6, P16 27.2.
//! - The organ's riff keeps O8's balance as built (within 2 dB of each band).
//! - Both voices' repeats differ, as a player's do.

use bf_sound::SRF;
use bf_sound::dsp::fft::Fft;
use bf_sound::voice::engine2::play::Play;
use bf_sound::voice::engine2::{self, Instrument, Parts};

fn instrument(name: &str) -> &'static Instrument {
    bf_content::fighters::voice(name)
        .and_then(|v| v.engine2())
        .unwrap_or_else(|| panic!("{name} has no engine 2 instrument"))
}

/// An eighth at 132 beats a minute, and a quarter at 100: the riffs' units.
const E: f64 = 60.0 / 132.0 / 2.0;
const O: f64 = 60.0 / 100.0 / 2.0;

/// A note of a riff: start, MIDI, held, velocity, how many start with it.
type Note = (f64, f64, f64, f64, u8);

/// The piano riff of the listening rounds: a bass line under a tune, a held chord at the end.
fn piano_riff() -> Vec<Note> {
    let mut n = Vec::new();
    let mut t = 0.25;
    let mel: [(f64, f64, f64); 17] = [
        (67.0, 1.0, 0.8),
        (75.0, 1.0, 0.65),
        (74.0, 1.0, 0.6),
        (72.0, 1.0, 0.62),
        (67.0, 2.0, 0.75),
        (70.0, 1.0, 0.6),
        (72.0, 1.0, 0.62),
        (74.0, 1.0, 0.66),
        (75.0, 1.0, 0.7),
        (77.0, 1.0, 0.72),
        (74.0, 1.0, 0.62),
        (79.0, 4.0, 0.85),
        (72.0, 1.0, 0.7),
        (72.0, 1.0, 0.55),
        (72.0, 1.0, 0.62),
        (70.0, 1.0, 0.58),
        (67.0, 2.0, 0.7),
    ];
    n.push((t, 48.0, 8.0 * E * 0.95, 0.7, 2));
    n.push((t + 8.0 * E, 55.0, 8.0 * E * 0.95, 0.62, 1));
    n.push((t + 16.0 * E, 51.0, 8.0 * E * 0.95, 0.66, 1));
    for (m, l, v) in mel {
        n.push((t, m, l * E * 0.9, v, 1));
        t += l * E;
    }
    t += E;
    for (m, v) in [(48.0, 0.75), (55.0, 0.62), (63.0, 0.66), (67.0, 0.7), (72.0, 0.74)] {
        n.push((t, m, 2.2, v, 5));
    }
    n
}

/// The riff's tune an octave up, ending on a high held chord: where the high notes are heard on their own.
fn high_tune() -> Vec<Note> {
    let mut n: Vec<Note> = piano_riff()
        .into_iter()
        .filter(|&(_, m, d, _, _)| d < 4.0 * E && m >= 60.0)
        .map(|(t, m, d, v, _)| (t, m + 12.0, d, v, 1))
        .collect();
    let t = n.iter().map(|x| x.0).fold(0.0, f64::max) + 2.0 * E;
    for (m, v) in [(72.0, 0.66), (79.0, 0.66), (84.0, 0.7), (87.0, 0.7), (91.0, 0.74)] {
        n.push((t, m, 2.2, v, 5));
    }
    n
}

/// The organ riff of the listening rounds: a run, a falling line, a big chord with the pedal.
fn organ_riff() -> Vec<Note> {
    let mut n = Vec::new();
    let mut t = 0.25;
    for m in [60.0, 63.0, 67.0, 72.0, 75.0, 72.0, 67.0, 63.0] {
        n.push((t, m, O / 2.0 * 0.92, 0.8, 1));
        t += O / 2.0;
    }
    for (m, l) in [
        (70.0, 1.0),
        (68.0, 1.0),
        (67.0, 1.0),
        (65.0, 1.0),
        (63.0, 1.0),
        (62.0, 1.0),
        (60.0, 2.0),
    ] {
        n.push((t, m, l * O * 0.95, 0.8, 1));
        t += l * O;
    }
    t += O * 0.5;
    for m in [36.0, 48.0, 51.0, 55.0, 60.0] {
        n.push((t, m, 2.6, 0.8, 5));
    }
    n
}

/// The riff on `inst`, mono and dry, each note as long as it rings, a key's repeats taking turns.
fn play(inst: &Instrument, notes: &[Note]) -> Vec<f64> {
    let mut out = vec![0.0; (20.0 * SRF) as usize];
    let mut seen = std::collections::BTreeMap::new();
    for &(t, m, d, v, together) in notes {
        let turn = seen.entry(m as i64).or_insert(0u64);
        let x = engine2::play(
            inst,
            &Play {
                together,
                ..Play::plain(m, d, v)
            },
            0.0,
            *turn % 3,
        );
        *turn += 1;
        let s = (t * SRF) as usize;
        for (o, y) in out.iter_mut().skip(s).zip(&x) {
            *o += y;
        }
    }
    let end = out.iter().rposition(|v| v.abs() > 1e-9).map_or(0, |i| i + 1);
    out.truncate(end);
    out
}

/// The share of `x`'s power in each of `edges`' bands, dB against the power from the first edge to the last
/// (Welch's average of 8192-sample periodic Hann windows, half overlapping, each less its mean: `scipy.signal.welch`
/// as the listening rounds measured).
fn bands(x: &[f64], edges: &[f64]) -> Vec<f64> {
    const N: usize = 8192;
    let fft = Fft::new(N);
    let mut p = vec![0.0; N / 2 + 1];
    let mut start = 0;
    while start + N <= x.len() {
        let seg = &x[start..start + N];
        let mean = seg.iter().sum::<f64>() / N as f64;
        let mut re: Vec<f64> = seg
            .iter()
            .enumerate()
            .map(|(i, v)| (v - mean) * (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / N as f64).cos()))
            .collect();
        let mut im = vec![0.0; N];
        fft.forward(&mut re, &mut im);
        for (k, q) in p.iter_mut().enumerate() {
            *q += re[k] * re[k] + im[k] * im[k];
        }
        start += N / 2;
    }
    let hz = |k: usize| k as f64 * SRF / N as f64;
    let sum = |lo: f64, hi: f64| {
        p.iter()
            .enumerate()
            .filter(|(k, _)| (lo..hi).contains(&hz(*k)))
            .map(|(_, q)| q)
            .sum::<f64>()
    };
    let total = sum(edges[0], edges[edges.len() - 1]).max(1e-300);
    edges
        .windows(2)
        .map(|w| 10.0 * (sum(w[0], w[1]) / total).log10())
        .collect()
}

/// The piano bands' edges, and the four real pianos' range in each (their riffs, the listening rounds' measure).
const PIANO_EDGES: [f64; 9] = [60.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16_000.0];
const REAL_LOW: [f64; 8] = [-23.95, -11.21, -7.78, -3.71, -15.77, -23.52, -35.75, -58.09];
const REAL_HIGH: [f64; 8] = [-18.62, -8.73, -3.76, -1.90, -10.46, -15.22, -24.01, -43.10];
/// How far outside the real range a band may lie, dB.
const PIANO_MARGIN: f64 = 1.5;

#[test]
fn both_fighters_play_their_built_engines() {
    assert!(matches!(instrument("PIANO").parts, Parts::Strings(_)));
    assert!(matches!(instrument("ORGAN").parts, Parts::Pipes(_)));
    assert!(!instrument("PIANO").parts.has_recipe() && !instrument("ORGAN").parts.has_recipe());
}

#[test]
fn the_piano_riff_sits_inside_the_real_pianos() {
    let b = bands(&play(instrument("PIANO"), &piano_riff()), &PIANO_EDGES);
    for (i, v) in b.iter().enumerate() {
        assert!(
            *v > REAL_LOW[i] - PIANO_MARGIN && *v < REAL_HIGH[i] + PIANO_MARGIN,
            "band {} to {} Hz: {v:.2} dB, the real pianos {} to {}",
            PIANO_EDGES[i],
            PIANO_EDGES[i + 1],
            REAL_LOW[i],
            REAL_HIGH[i]
        );
    }
}

#[test]
fn the_high_notes_are_not_too_bright() {
    let b = bands(&play(instrument("PIANO"), &high_tune()), &[60.0, 4000.0, 16_000.0]);
    assert!(b[1] < -20.0 && b[1] > -38.0, "over 4 kHz: {:.2} dB", b[1]);
}

/// The organ's bands' edges, and O8 as built (2026-11-26), each band within [`ORGAN_MARGIN`].
const ORGAN_EDGES: [f64; 10] = [
    31.25, 62.5, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16_000.0,
];
const O8_BUILT: [f64; 9] = [-14.22, -8.48, -7.40, -6.31, -7.96, -8.15, -11.42, -17.28, -35.90];
const ORGAN_MARGIN: f64 = 2.0;

#[test]
fn the_organ_riff_keeps_o8s_balance() {
    let b = bands(&play(instrument("ORGAN"), &organ_riff()), &ORGAN_EDGES);
    for (i, v) in b.iter().enumerate() {
        assert!(
            (v - O8_BUILT[i]).abs() < ORGAN_MARGIN,
            "band {} to {} Hz: {v:.2} dB, O8 {}",
            ORGAN_EDGES[i],
            ORGAN_EDGES[i + 1],
            O8_BUILT[i]
        );
    }
}

#[test]
fn repeats_differ_and_the_same_note_is_the_same() {
    for name in ["PIANO", "ORGAN"] {
        let inst = instrument(name);
        let note = |variant| engine2::play(inst, &Play::plain(62.0, 0.4, 0.7), 0.8, variant);
        assert_eq!(note(0), note(0), "{name}");
        assert_ne!(note(0), note(1), "{name}");
        assert!(note(0).iter().all(|v| v.is_finite()), "{name}");
    }
}
