//! Voice Engine 2's built voices (`docs/studio/2026-11-26-voice-engine-2-design.md`, "Revision: built engines"):
//! PIANO on the struck-string engine and ORGAN on the pipe engine, as the Composed arrangement plays them, held to
//! what Aedan picked by ear (P16 and O8, 2026-11-26) and to the real instruments' range.
//!
//! - The piano's riff (the listening rounds' own) has its octave bands inside the range of four real pianos (VCSL
//!   Steinway B, Splendid Grand, Salamander, Headroom), each measured the same way, give or take 1.5 dB.
//! - The riff's tune an octave up keeps its energy over 4 kHz low: the high notes Aedan heard as "very
//!   synthesized" (P12) had 9.5 dB under the whole there, the real Steinway 32.6, P16 27.2.
//! - The organ's riff keeps O8's balance as built (within 2 dB of each band).
//! - ACOUSTIC, ELECTRIC, BASS GUITAR, UKULELE and HARP on the plucked-string engine (A6, E2, B3, U6, H3,
//!   2026-11-27): each one's riff of the listening rounds (`tests/reference/plucked_riffs.txt`, written from the
//!   sketch's own) keeps the sketch's octave bands within 2 dB, through its amp where it has one.
//! - VIOLIN, VIOLA, CELLO, CONTRABASS and OCTOBASS on the bowed-string engine (Vn3, Va3, Vc1, Cb1, Oc2,
//!   2026-11-27): each one's riff of the bowed rounds (`tests/reference/bowed_riffs.txt`, the sketch's own) keeps
//!   the sketch's octave bands within 2 dB.
//! - SOPRANO SAX, ALTO SAX, TENOR SAX, BARI SAX, CLARINET, BASS CLARINET, CONTRABASS CLARINET, OBOE, BASSOON,
//!   CONTRABASSOON, BAGPIPES and ACCORDION on the reed engine (Ss2, As2, Ts2, Bs2, Cl1, Bc1, Cc1, Ob1, Bn1, Cf2, Bp1,
//!   Ac1, 2026-11-27): each one's riff of the reed rounds (`tests/reference/reed_riffs.txt`, the sketch's own) keeps
//!   the sketch's octave bands within 2 dB, but where the sketch's whole lines carry the bore from note to note
//!   ([`REED_HISTORY`]).
//! - TRUMPET, CORNET, FLUGELHORN, HORN, MELLOPHONE, TROMBONE, EUPHONIUM, TUBA and SOUSAPHONE on the brass engine,
//!   FLUTE and PICCOLO on the flute engine (Tp1, Co1, Fh1, Hn1, Me1, Tb1, Eu1, Tu1, Su1, Fl1, Pc1, 2026-11-27): each
//!   one's riff of the brass and flutes round (`tests/reference/wind_riffs.txt`, the sketch's own) keeps the sketch's
//!   octave bands as heard (after its limiter, mutes and hand, dry) within 2 dB.
//! - Every voice's repeats differ, as a player's do.

use bf_sound::SRF;
use bf_sound::dsp::fft::Fft;
use bf_sound::voice::engine2::play::Vibrato;
use bf_sound::voice::engine2::play::{Bend, Mute, Play, Register, Swell, Technique};
use bf_sound::voice::engine2::{self, Instrument, Parts, amp};

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
fn the_fighters_play_their_built_engines() {
    assert!(matches!(instrument("PIANO").parts, Parts::Strings(_)));
    assert!(matches!(instrument("ORGAN").parts, Parts::Pipes(_)));
    for (_, name, _) in PLUCKED {
        assert!(matches!(instrument(name).parts, Parts::Plucked(_)), "{name}");
    }
    for (_, name, _) in BOWED {
        assert!(matches!(instrument(name).parts, Parts::Bowed(_)), "{name}");
        assert_eq!(instrument(name).space.amp, None, "{name}");
    }
    for (_, name, _) in REEDS {
        assert!(matches!(instrument(name).parts, Parts::Reed(_)), "{name}");
        assert_eq!(instrument(name).space.amp, None, "{name}");
    }
    for (code, name, _) in WINDS {
        match &instrument(name).parts {
            Parts::Brass(b) => assert_eq!(b.name, code, "{name}"),
            Parts::Flute(f) => assert_eq!(f.name, code, "{name}"),
            _ => panic!("{name} plays neither wind engine"),
        }
        assert_eq!(instrument(name).space.amp, None, "{name}");
    }
    for name in ["PIANO", "ORGAN"]
        .into_iter()
        .chain(PLUCKED.map(|p| p.1))
        .chain(BOWED.map(|p| p.1))
        .chain(REEDS.map(|p| p.1))
        .chain(WINDS.map(|p| p.1))
    {
        assert!(!instrument(name).parts.has_recipe(), "{name}");
    }
    // the electric and the bass play through their amps, the rest through none
    assert_eq!(instrument("ELECTRIC").space.amp, Some(amp::Amp::HIGH_GAIN));
    assert_eq!(instrument("BASS GUITAR").space.amp, Some(amp::Amp::BASS_GRIT));
    for name in ["ACOUSTIC", "UKULELE", "HARP", "PIANO", "ORGAN"] {
        assert_eq!(instrument(name).space.amp, None, "{name}");
    }
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

/// The plucked riffs: the sketch's code, the fighter, and the sketch's own bands (its riff dry, mono; the acoustic's
/// and the ukulele's two microphones' powers averaged) over [`ORGAN_EDGES`].
const PLUCKED: [(&str, &str, [f64; 9]); 5] = [
    ("A6", "ACOUSTIC", SKETCH[0]),
    ("E2", "ELECTRIC", SKETCH[1]),
    ("B3", "BASS GUITAR", SKETCH[2]),
    ("U6", "UKULELE", SKETCH[3]),
    ("H3", "HARP", SKETCH[4]),
];
#[rustfmt::skip]
const SKETCH: [[f64; 9]; 5] = [
    [-43.22, -8.63, -4.47, -6.99, -11.16, -9.00, -10.63, -17.91, -32.93],
    [-35.26, -17.24, -10.98, -6.49, -6.37, -5.70, -7.83, -19.27, -42.94],
    [-10.32, -3.50, -6.93, -9.85, -10.36, -12.96, -19.75, -33.13, -66.98],
    [-94.38, -59.20, -27.83, -2.85, -4.92, -10.35, -12.15, -23.07, -44.99],
    [-56.15, -26.02, -11.99, -5.47, -2.78, -9.47, -20.08, -48.81, -84.12],
];
/// A band this far under the whole is too quiet to hold to the sketch (a ukulele's lowest octaves).
const QUIET: f64 = -45.0;
const PLUCKED_MARGIN: f64 = 2.0;

/// The riff `code` of `tests/reference/plucked_riffs.txt` on `inst`, mono and dry, each note as long as it rings, a
/// pitch's repeats taking turns, through the instrument's amp if it has one. A line is the code, then the note's
/// start, MIDI, held length, velocity, `pick` or `fingers`, `open`, `palm` or `dead`, a bend's cents, start and
/// length, a vibrato's cents, rate and wait, and the pitch it slides from (`-` for none).
fn plucked_riff(inst: &Instrument, code: &str) -> Vec<f64> {
    let text = include_str!("reference/plucked_riffs.txt");
    let mut out = vec![0.0; (20.0 * SRF) as usize];
    let mut seen = std::collections::BTreeMap::new();
    for line in text.lines().filter(|l| l.split_whitespace().next() == Some(code)) {
        let w: Vec<&str> = line.split_whitespace().collect();
        let num = |i: usize| -> Option<f64> { (w[i] != "-").then(|| w[i].parse().expect("a number")) };
        let at = |i: usize| num(i).expect("a value");
        let play = Play {
            technique: if w[5] == "pick" {
                Technique::Picked
            } else {
                Technique::Fingers
            },
            mute: match w[6] {
                "palm" => Mute::Muted,
                "dead" => Mute::Choked,
                _ => Mute::Open,
            },
            bend: num(7).map(|cents| Bend {
                cents,
                at: at(8),
                over: at(9),
            }),
            vibrato: num(10).map(|cents| Vibrato {
                cents,
                hz: at(11),
                wait: at(12),
            }),
            slur_from: num(13),
            ..Play::plain(at(2), at(3), at(4))
        };
        let turn = seen.entry(play.midi as i64).or_insert(0u64);
        let x = engine2::play(inst, &play, 0.0, *turn % 3);
        *turn += 1;
        let s = (at(1) * SRF) as usize;
        for (o, y) in out.iter_mut().skip(s).zip(&x) {
            *o += y;
        }
    }
    let end = out.iter().rposition(|v| v.abs() > 1e-9).map_or(0, |i| i + 1);
    out.truncate(end);
    match &inst.space.amp {
        Some(a) => {
            let both: Vec<[f64; 2]> = out.iter().map(|&v| [v, v]).collect();
            amp::on_bus(&both, a, SRF).iter().map(|f| f[0]).collect()
        }
        None => out,
    }
}

#[test]
fn the_plucked_riffs_keep_the_sketches_balance() {
    for (code, name, sketch) in PLUCKED {
        let x = plucked_riff(instrument(name), code);
        assert!(x.len() > (5.0 * SRF) as usize, "{name}");
        let b = bands(&x, &ORGAN_EDGES);
        for (i, v) in b.iter().enumerate() {
            if sketch[i] < QUIET {
                continue;
            }
            assert!(
                (v - sketch[i]).abs() < PLUCKED_MARGIN,
                "{name} ({code}), band {} to {} Hz: {v:.2} dB, the sketch {}",
                ORGAN_EDGES[i],
                ORGAN_EDGES[i + 1],
                sketch[i]
            );
        }
    }
}

/// The bowed riffs: the sketch's code, the fighter, and the sketch's own bands (its riff through the sketch's body,
/// the two microphones' powers averaged) over [`ORGAN_EDGES`].
const BOWED: [(&str, &str, [f64; 9]); 5] = [
    ("Vn3", "VIOLIN", BOWED_SKETCH[0]),
    ("Va3", "VIOLA", BOWED_SKETCH[1]),
    ("Vc1", "CELLO", BOWED_SKETCH[2]),
    ("Cb1", "CONTRABASS", BOWED_SKETCH[3]),
    ("Oc2", "OCTOBASS", BOWED_SKETCH[4]),
];
#[rustfmt::skip]
const BOWED_SKETCH: [[f64; 9]; 5] = [
    [-83.09, -65.68, -42.68, -22.17, -4.08, -4.72, -6.64, -13.31, -28.46],
    [-67.73, -37.31, -11.89, -7.79, -3.50, -6.42, -10.57, -22.28, -38.82],
    [-40.30, -5.95, -2.51, -9.57, -14.67, -15.96, -18.79, -26.42, -41.43],
    [-6.76, -3.45, -6.69, -11.86, -14.41, -17.04, -27.80, -43.69, -56.30],
    [-1.50, -6.14, -15.67, -19.03, -21.40, -27.61, -40.15, -55.45, -67.37],
];

/// The riff `code` of `tests/reference/bowed_riffs.txt` on `inst`, mono and dry, each note as long as it rings, a
/// pitch's repeats taking turns. A line is the code, then the note's start, MIDI, held length and velocity, then the
/// sketch's asks: `tech=` (pizz, snap, lh, slap, spicc, legno, tratto, trem, harm, harm3, arco), `acc=` an accent,
/// `tie=1`, `slur=` the pitch it is slurred from, `str=` a string, `end=stop` a martele, `swell=` from,to,over.
fn bowed_riff(inst: &Instrument, code: &str) -> Vec<f64> {
    let text = include_str!("reference/bowed_riffs.txt");
    let mut out = vec![0.0; (20.0 * SRF) as usize];
    let mut seen = std::collections::BTreeMap::new();
    for line in text.lines().filter(|l| l.split_whitespace().next() == Some(code)) {
        let w: Vec<&str> = line.split_whitespace().collect();
        let at = |i: usize| -> f64 { w[i].parse().expect("a number") };
        let mut play = Play::plain(at(2), at(3), at(4));
        for kv in &w[5..] {
            let (k, v) = kv.split_once('=').expect("a key=value");
            let num = || -> f64 { v.parse().expect("a number") };
            match k {
                "tech" => {
                    play.technique = match v {
                        "pizz" => Technique::Pizzicato,
                        "snap" => Technique::SnapPizz,
                        "lh" => Technique::LeftHandPizz,
                        "slap" => Technique::Slap,
                        "spicc" => Technique::Spiccato,
                        "legno" => Technique::ColLegno,
                        "tratto" => Technique::ColLegnoTratto,
                        "trem" => Technique::Tremolo,
                        "harm" => Technique::Harmonic(2),
                        "harm3" => Technique::Harmonic(3),
                        "arco" => Technique::Arco,
                        other => panic!("a technique {other:?}"),
                    }
                }
                "acc" => play.accent = num(),
                "tie" => play.tie = v == "1",
                "slur" => play.slur_from = Some(num()),
                "str" => play.string = Some(num() as u8),
                "end" if v == "stop" => play.technique = Technique::Martele,
                "swell" => {
                    let q: Vec<f64> = v.split(',').map(|x| x.parse().expect("a number")).collect();
                    play.swell = Some(Swell {
                        from: q[0],
                        to: q[1],
                        over: q[2],
                    });
                }
                other => panic!("an ask {other:?}"),
            }
        }
        let turn = seen.entry(play.midi as i64).or_insert(0u64);
        let x = engine2::play(inst, &play, 0.0, *turn % 3);
        *turn += 1;
        let s = (at(1) * SRF) as usize;
        for (o, y) in out.iter_mut().skip(s).zip(&x) {
            *o += y;
        }
    }
    let end = out.iter().rposition(|v| v.abs() > 1e-9).map_or(0, |i| i + 1);
    out.truncate(end);
    out
}

#[test]
fn the_bowed_riffs_keep_the_sketches_balance() {
    for (code, name, sketch) in BOWED {
        let x = bowed_riff(instrument(name), code);
        assert!(x.len() > (5.0 * SRF) as usize, "{name}");
        let b = bands(&x, &ORGAN_EDGES);
        for (i, v) in b.iter().enumerate() {
            if sketch[i] < QUIET {
                continue;
            }
            assert!(
                (v - sketch[i]).abs() < PLUCKED_MARGIN,
                "{name} ({code}), band {} to {} Hz: {v:.2} dB, the sketch {}",
                ORGAN_EDGES[i],
                ORGAN_EDGES[i + 1],
                sketch[i]
            );
        }
    }
}

/// The reed riffs: the sketch's code, the fighter, and the sketch's own bands (its riff as the sketch played it, every
/// part of a line through one bore, dry) over [`ORGAN_EDGES`].
const REEDS: [(&str, &str, [f64; 9]); 12] = [
    ("Ss2", "SOPRANO SAX", REED_SKETCH[0]),
    ("As2", "ALTO SAX", REED_SKETCH[1]),
    ("Ts2", "TENOR SAX", REED_SKETCH[2]),
    ("Bs2", "BARI SAX", REED_SKETCH[3]),
    ("Cl1", "CLARINET", REED_SKETCH[4]),
    ("Bc1", "BASS CLARINET", REED_SKETCH[5]),
    ("Cc1", "CONTRABASS CLARINET", REED_SKETCH[6]),
    ("Ob1", "OBOE", REED_SKETCH[7]),
    ("Bn1", "BASSOON", REED_SKETCH[8]),
    ("Cf2", "CONTRABASSOON", REED_SKETCH[9]),
    ("Bp1", "BAGPIPES", REED_SKETCH[10]),
    ("Ac1", "ACCORDION", REED_SKETCH[11]),
];
#[rustfmt::skip]
const REED_SKETCH: [[f64; 9]; 12] = [
    [-67.59, -61.04, -51.40, -14.10, -4.16, -3.08, -10.94, -23.60, -34.30],
    [-63.51, -56.76, -21.96, -5.77, -2.57, -8.61, -14.46, -28.77, -32.85],
    [-62.70, -31.61, -16.12, -5.41, -2.35, -10.70, -17.40, -30.70, -32.15],
    [-35.69, -13.29, -5.27, -3.79, -7.19, -14.07, -21.98, -30.80, -30.01],
    [-63.25, -55.98, -14.38, -7.98, -2.91, -5.51, -19.68, -34.58, -35.64],
    [-40.16, -25.61, -10.36, -3.49, -4.18, -11.29, -29.64, -35.51, -31.69],
    [-27.43, -12.29, -8.54, -2.53, -6.36, -20.24, -38.01, -39.41, -35.70],
    [-64.50, -58.26, -33.01, -15.78, -3.03, -3.39, -17.95, -32.35, -39.27],
    [-42.56, -23.26, -12.10, -1.64, -6.65, -15.30, -28.67, -38.53, -40.46],
    [-20.86, -11.13, -5.14, -3.27, -9.30, -17.27, -33.03, -36.02, -33.78],
    [-80.49, -17.17, -9.96, -10.89, -3.36, -5.65, -12.11, -25.46, -38.55],
    [-43.40, -10.27, -9.45, -9.05, -5.98, -4.97, -10.99, -19.48, -21.85],
];

/// Where the reed riffs' bands lie further from the sketch's than [`PLUCKED_MARGIN`]: the code, the band (by its
/// place in [`ORGAN_EDGES`]) and how far, dB. The sketch plays a riff as whole lines, one bore through every note,
/// and the low bassoons' and the tenor's later notes take over the swing the bore had (a fuller fundamental, a
/// little louder); the app plays every note on its own, as the line's first note starts (the port's ruling: the
/// bassoon's bands under 250 Hz 7 to 9 dB under the sketch's, the contrabassoon's 7 to 14, and its top 6 to 7, the
/// tenor's 62 to 125 Hz 3.7). Each is the measured distance and a dB more.
const REED_HISTORY: [(&str, usize, f64); 12] = [
    ("Ts2", 1, 4.75),
    ("Bn1", 0, 9.7),
    ("Bn1", 1, 8.2),
    ("Bn1", 2, 8.7),
    ("Cf2", 0, 15.2),
    ("Cf2", 1, 12.8),
    ("Cf2", 2, 8.3),
    ("Cf2", 3, 3.6),
    ("Cf2", 4, 3.3),
    ("Cf2", 6, 4.0),
    ("Cf2", 7, 7.3),
    ("Cf2", 8, 8.4),
];

/// The riff `code` of `tests/reference/reed_riffs.txt` on `inst`, mono and dry, each note as long as it rings, a
/// pitch's repeats taking turns, each at its line's level. A line is the code, then the note's start, MIDI, held
/// length and velocity, then the sketch's asks: `tech=` (breath, click, growl, flutter, subtone, slap, air, trem, back,
/// soft, shake, sung<n>, harm<n>), `acc=` an accent, `tie=1`, `slur=` the pitch it is slurred from, `mute=choke` or
/// `mute=clip` a stop on the tongue, `swell=` from,to,over, `vib=` cents,hz,wait (0 for none), `bend=` and `scoop=`
/// cents,at,over, `glide=` a slur's glide over that long, `bank=` an accordion's register (one, two, musette, low,
/// oct, master, bass, chord; the bagpipe's drones are its bass), `gain=` the line's level.
fn reed_riff(inst: &Instrument, code: &str) -> Vec<f64> {
    let text = include_str!("reference/reed_riffs.txt");
    let mut out = vec![0.0; (20.0 * SRF) as usize];
    let mut seen = std::collections::BTreeMap::new();
    for line in text.lines().filter(|l| l.split_whitespace().next() == Some(code)) {
        let w: Vec<&str> = line.split_whitespace().collect();
        let at = |i: usize| -> f64 { w[i].parse().expect("a number") };
        let mut play = Play::plain(at(2), at(3), at(4));
        let mut gain = 1.0;
        for kv in &w[5..] {
            let (k, v) = kv.split_once('=').expect("a key=value");
            let num = || -> f64 { v.parse().expect("a number") };
            let list = || -> Vec<f64> { v.split(',').map(|x| x.parse().expect("a number")).collect() };
            match k {
                "tech" => {
                    play.technique = match v {
                        "breath" => Technique::Breath,
                        "click" => Technique::KeyClick,
                        "growl" => Technique::Growl,
                        "flutter" => Technique::Flutter,
                        "subtone" => Technique::Subtone,
                        "slap" => Technique::Slap,
                        "air" => Technique::AirTone,
                        "trem" => Technique::Tremolo,
                        "back" => Technique::BackTongue,
                        "soft" => Technique::SoftTongue,
                        "shake" => Technique::Shake,
                        t if t.starts_with("sung") => Technique::Sung(t[4..].parse().expect("a number")),
                        t if t.starts_with("harm") => Technique::Harmonic(t[4..].parse().expect("a number")),
                        other => panic!("a technique {other:?}"),
                    }
                }
                "acc" => play.accent = num(),
                "tie" => play.tie = v == "1",
                "slur" => play.slur_from = Some(num()),
                "mute" if v == "choke" => play.mute = Mute::Choked,
                "mute" if v == "clip" => play.mute = Mute::Clipped,
                "swell" => {
                    let q = list();
                    play.swell = Some(Swell {
                        from: q[0],
                        to: q[1],
                        over: q[2],
                    });
                }
                "vib" => {
                    let q = list();
                    play.vibrato = Some(Vibrato {
                        cents: q[0],
                        hz: *q.get(1).unwrap_or(&5.0),
                        wait: *q.get(2).unwrap_or(&0.3),
                    });
                }
                "bend" | "scoop" => {
                    let q = list();
                    let b = Some(Bend {
                        cents: q[0],
                        at: q[1],
                        over: q[2],
                    });
                    if k == "bend" {
                        play.bend = b;
                    } else {
                        play.scoop = b;
                    }
                }
                "glide" => {
                    play.bend = Some(Bend {
                        cents: 0.0,
                        at: 0.0,
                        over: num(),
                    })
                }
                "bank" => {
                    play.register = Some(match v {
                        "one" => Register::One,
                        "two" => Register::Two,
                        "musette" => Register::Musette,
                        "low" => Register::Low,
                        "oct" => Register::Octave,
                        "master" => Register::Master,
                        "bass" => Register::Bass,
                        "chord" => Register::Chord,
                        other => panic!("a bank {other:?}"),
                    })
                }
                "gain" => gain = num(),
                other => panic!("an ask {other:?}"),
            }
        }
        let turn = seen.entry(play.midi as i64).or_insert(0u64);
        let x = engine2::play(inst, &play, 0.0, *turn % 3);
        *turn += 1;
        let s = (at(1) * SRF) as usize;
        for (o, y) in out.iter_mut().skip(s).zip(&x) {
            *o += gain * y;
        }
    }
    let end = out.iter().rposition(|v| v.abs() > 1e-9).map_or(0, |i| i + 1);
    out.truncate(end);
    out
}

#[test]
fn the_reed_riffs_keep_the_sketches_balance() {
    for (code, name, sketch) in REEDS {
        let x = reed_riff(instrument(name), code);
        assert!(x.len() > (5.0 * SRF) as usize, "{name}");
        let b = bands(&x, &ORGAN_EDGES);
        for (i, v) in b.iter().enumerate() {
            if sketch[i] < QUIET {
                continue;
            }
            let margin = REED_HISTORY
                .iter()
                .find(|(c, band, _)| *c == code && *band == i)
                .map_or(PLUCKED_MARGIN, |h| h.2);
            assert!(
                (v - sketch[i]).abs() < margin,
                "{name} ({code}), band {} to {} Hz: {v:.2} dB, the sketch {}",
                ORGAN_EDGES[i],
                ORGAN_EDGES[i + 1],
                sketch[i]
            );
        }
    }
}

/// The brass and flutes' riffs: the sketch's code, the fighter, and the sketch's own bands (its riff as heard: through
/// its pulse limiter, mutes and hand, dry) over [`ORGAN_EDGES`].
const WINDS: [(&str, &str, [f64; 9]); 11] = [
    ("Tp1", "TRUMPET", WINDS_SKETCH[0]),
    ("Co1", "CORNET", WINDS_SKETCH[1]),
    ("Fh1", "FLUGELHORN", WINDS_SKETCH[2]),
    ("Hn1", "HORN", WINDS_SKETCH[3]),
    ("Me1", "MELLOPHONE", WINDS_SKETCH[4]),
    ("Tb1", "TROMBONE", WINDS_SKETCH[5]),
    ("Eu1", "EUPHONIUM", WINDS_SKETCH[6]),
    ("Tu1", "TUBA", WINDS_SKETCH[7]),
    ("Su1", "SOUSAPHONE", WINDS_SKETCH[8]),
    ("Fl1", "FLUTE", WINDS_SKETCH[9]),
    ("Pc1", "PICCOLO", WINDS_SKETCH[10]),
];
#[rustfmt::skip]
const WINDS_SKETCH: [[f64; 9]; 11] = [
    [-90.43, -80.41, -67.65, -19.57, -2.12, -6.77, -9.21, -14.26, -21.65],
    [-66.96, -62.48, -47.00, -7.93, -2.01, -7.60, -15.42, -22.07, -32.21],
    [-83.71, -72.64, -39.71, -4.41, -2.49, -11.48, -25.76, -37.77, -51.86],
    [-53.79, -48.50, -14.23, -2.22, -5.77, -10.84, -18.36, -32.18, -51.73],
    [-78.45, -66.68, -55.07, -11.85, -2.60, -6.21, -9.58, -15.01, -23.60],
    [-42.03, -24.57, -8.20, -3.44, -5.17, -11.55, -18.12, -25.98, -34.84],
    [-58.14, -22.26, -7.71, -3.24, -5.17, -13.57, -25.57, -39.14, -54.68],
    [-20.02, -8.77, -4.65, -3.30, -13.75, -24.40, -32.69, -41.19, -48.76],
    [-19.16, -8.34, -4.89, -3.27, -13.94, -23.41, -30.77, -38.28, -45.33],
    [-72.78, -64.52, -54.41, -26.15, -5.45, -1.82, -13.59, -20.02, -28.42],
    [-86.01, -76.74, -67.61, -58.75, -12.99, -3.79, -2.97, -16.02, -25.88],
];

/// The riff `code` of `tests/reference/wind_riffs.txt` on `inst`, mono and dry, each note as long as it rings, a
/// pitch's repeats taking turns. A line is the code, then the note's start, MIDI, held length and velocity, then the
/// sketch's asks: `tech=` (back, soft, breath, shake, flutter, growl, half, whistle, jet, click, air, harm<n>,
/// sung<n>), `str=` the fingering, `acc=` an accent, `tie=1`, `slur=` the pitch it is slurred from, `bend=`,
/// `scoop=` cents,at,over, `swell=`, `expr=` from,to,over, `vib=` cents,hz,wait, `mute=` (straight, cup, harmon,
/// stem, plunger, practice, stopped, stopmute, muted, choked, clipped).
fn wind_riff(inst: &Instrument, code: &str) -> Vec<f64> {
    let text = include_str!("reference/wind_riffs.txt");
    let mut out = vec![0.0; (20.0 * SRF) as usize];
    let mut seen = std::collections::BTreeMap::new();
    for line in text.lines().filter(|l| l.split_whitespace().next() == Some(code)) {
        let w: Vec<&str> = line.split_whitespace().collect();
        let at = |i: usize| -> f64 { w[i].parse().expect("a number") };
        let mut play = Play::plain(at(2), at(3), at(4));
        for kv in &w[5..] {
            let (k, v) = kv.split_once('=').expect("a key=value");
            let num = || -> f64 { v.parse().expect("a number") };
            let three = || -> Vec<f64> { v.split(',').map(|x| x.parse().expect("a number")).collect() };
            match k {
                "tech" => {
                    play.technique = match v {
                        "back" => Technique::BackTongue,
                        "soft" => Technique::SoftTongue,
                        "breath" => Technique::Breath,
                        "shake" => Technique::Shake,
                        "flutter" => Technique::Flutter,
                        "growl" => Technique::Growl,
                        "half" => Technique::HalfValve,
                        "whistle" => Technique::WhistleTone,
                        "jet" => Technique::JetWhistle,
                        "click" => Technique::KeyClick,
                        "air" => Technique::AirTone,
                        t if t.starts_with("harm") => Technique::Harmonic(t[4..].parse().expect("a harmonic")),
                        t if t.starts_with("sung") => Technique::Sung(t[4..].parse().expect("an interval")),
                        other => panic!("a technique {other:?}"),
                    }
                }
                "str" => play.string = Some(num() as u8),
                "acc" => play.accent = num(),
                "tie" => play.tie = v == "1",
                "slur" => play.slur_from = Some(num()),
                "bend" | "scoop" => {
                    let q = three();
                    let b = Some(Bend {
                        cents: q[0],
                        at: q[1],
                        over: q[2],
                    });
                    if k == "bend" {
                        play.bend = b;
                    } else {
                        play.scoop = b;
                    }
                }
                "swell" | "expr" => {
                    let q = three();
                    let s = Some(Swell {
                        from: q[0],
                        to: q[1],
                        over: q[2],
                    });
                    if k == "swell" {
                        play.swell = s;
                    } else {
                        play.expression = s;
                    }
                }
                "vib" => {
                    let q = three();
                    play.vibrato = Some(Vibrato {
                        cents: q[0],
                        hz: q[1],
                        wait: q[2],
                    });
                }
                "mute" => {
                    play.mute = match v {
                        "straight" => Mute::Straight,
                        "muted" => Mute::Muted,
                        "cup" => Mute::Cup,
                        "harmon" => Mute::Harmon,
                        "stem" => Mute::HarmonStem,
                        "plunger" => Mute::Plunger,
                        "practice" => Mute::Practice,
                        "stopped" => Mute::Stopped,
                        "stopmute" => Mute::StopMute,
                        "choked" => Mute::Choked,
                        "clipped" => Mute::Clipped,
                        other => panic!("a mute {other:?}"),
                    }
                }
                other => panic!("an ask {other:?}"),
            }
        }
        let turn = seen.entry(play.midi as i64).or_insert(0u64);
        let x = engine2::play(inst, &play, 0.0, *turn % 3);
        *turn += 1;
        let s = (at(1) * SRF) as usize;
        for (o, y) in out.iter_mut().skip(s).zip(&x) {
            *o += y;
        }
    }
    let end = out.iter().rposition(|v| v.abs() > 1e-9).map_or(0, |i| i + 1);
    out.truncate(end);
    out
}

#[test]
fn the_wind_riffs_keep_the_sketches_balance() {
    for (code, name, sketch) in WINDS {
        let x = wind_riff(instrument(name), code);
        assert!(x.len() > (5.0 * SRF) as usize, "{name}");
        let b = bands(&x, &ORGAN_EDGES);
        for (i, v) in b.iter().enumerate() {
            if sketch[i] < QUIET {
                continue;
            }
            assert!(
                (v - sketch[i]).abs() < PLUCKED_MARGIN,
                "{name} ({code}), band {} to {} Hz: {v:.2} dB, the sketch {}",
                ORGAN_EDGES[i],
                ORGAN_EDGES[i + 1],
                sketch[i]
            );
        }
    }
}

#[test]
fn repeats_differ_and_the_same_note_is_the_same() {
    for name in ["PIANO", "ORGAN"]
        .into_iter()
        .chain(PLUCKED.map(|p| p.1))
        .chain(BOWED.map(|p| p.1))
        .chain(REEDS.map(|p| p.1))
        .chain(WINDS.map(|p| p.1))
    {
        let inst = instrument(name);
        let note = |variant| engine2::play(inst, &Play::plain(62.0, 0.4, 0.7), 0.8, variant);
        assert_eq!(note(0), note(0), "{name}");
        assert_ne!(note(0), note(1), "{name}");
        assert!(note(0).iter().all(|v| v.is_finite()), "{name}");
    }
}
