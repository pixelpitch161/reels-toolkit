//! Voice Engine 2's yardstick (`docs/studio/2026-11-26-voice-engine-2-design.md`): PIANO and ORGAN, as the
//! Composed arrangement plays them, against the recordings they were measured from and the real instruments studied
//! beside them (`tests/reference/engine2/<NAME>.json`, written by `cargo run --release -p bf-harness --example
//! recipes -- reference <NAME>`).
//!
//! Every note is measured the way its recording was ([`e2measure::yard`]). The bars are named per instrument below,
//! each with the worst the reference run measured (2026-11-26) beside it.
//!
//! - A piano's tone is compared overtone by overtone in its first 0.3 s, before its strings beat apart; after that,
//!   two or three strings of one note beat in and out of step, so only the overtones' median is held to a bar.
//! - An organ's tone is compared as held (each overtone's power over 0.3 to 2.5 s): its ranks of pipes beat against
//!   each other all through the note. Its start is not compared with the recording's: the recordings take up to a
//!   second to come up as the church's echo builds under them, and the engine's pipes speak within a tenth of a
//!   second (`bf_sound::voice::engine2::organ`, "Speech").
//! - Each note is nearer its own recording than the rivals it could be taken for (FluidR3's notes, as in
//!   `real_voices.rs`). FluidR3's own piano and church organ are not the yardstick: the recordings themselves are
//!   farther from them than from FluidR3's harp, accordion or trumpet on many notes (the reference run's table).

use std::collections::BTreeMap;

use bf_harness::e2measure::{self, Yard, YardDiff};
use bf_harness::timbre::{self, Fingerprint};
use bf_sound::voice::engine2::Instrument;
use serde_json::Value;

/// One instrument's bars.
struct Bars {
    name: &'static str,
    /// The one overtone in the first 0.3 s (piano) or of the held tone (organ), dB.
    partial_db: f64,
    /// The median over the notes of that, dB: the design's 2 to 3 dB per overtone, typically.
    typical_db: f64,
    /// The overtones' median at any moment, dB.
    median_db: f64,
    /// The start, seconds (`None`: not compared).
    start_s: Option<f64>,
    /// The stop after the key comes up against the measured damper or pallet, seconds.
    stop_s: f64,
    /// The noise shares, early and held.
    noise_early: f64,
    noise_steady: f64,
    /// The fingerprint distance to its own recording, dB.
    own_db: f64,
    /// The rivals it must be farther from, in FluidR3's set.
    rivals: &'static [&'static str],
}

const PIANO: Bars = Bars {
    name: "PIANO",
    // worst 19.6 (80 vl3, the 5th overtone at 30 ms); median 3.6
    partial_db: 23.0,
    typical_db: 4.5,
    // worst 9.5 (82 vl4)
    median_db: 11.0,
    // worst 0.020
    start_s: Some(0.03),
    // worst 0.275 (76 vl2)
    stop_s: 0.33,
    // worst 0.047 and 0.008
    noise_early: 0.06,
    noise_steady: 0.015,
    // worst 1.4; the nearest rival 5.0 further
    own_db: 3.0,
    rivals: &["harp", "marimba", "nylon guitar"],
};

const ORGAN: Bars = Bars {
    name: "ORGAN",
    // worst 14.9; median 3.9
    partial_db: 18.0,
    typical_db: 4.5,
    // worst 5.3 (the held tone; its first 0.3 s are the pipes speaking)
    median_db: 6.5,
    start_s: None,
    // worst 0.063
    stop_s: 0.08,
    // worst 0.222 (the church's wind and room under a quiet low note) and 0.045
    noise_early: 0.27,
    noise_steady: 0.06,
    // worst 8.5 (42 quiet: the speech lifts the recording's slow start, whose tone differs); the nearest rival 3.6
    // further
    own_db: 10.0,
    rivals: &["accordion", "drawbar organ", "trumpet"],
};

/// How far past the real instruments' spread a note may lie, as a share of the spread's width.
const SPREAD_MARGIN: f64 = 0.1;

fn instrument(name: &str) -> &'static Instrument {
    bf_content::fighters::voice(name)
        .and_then(|v| v.engine2())
        .unwrap_or_else(|| panic!("{name} has no engine 2 instrument"))
}

fn reference(name: &str) -> Value {
    let path = format!("{}/tests/reference/engine2/{name}.json", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(&path).expect("the yardstick")).expect("JSON")
}

/// A recorded note of the yardstick.
struct Recorded {
    midi: f64,
    layer: String,
    strength: f64,
    grid_f0: f64,
    b: f64,
    release: f64,
    yard: Yard,
    print: Fingerprint,
}

fn recorded(name: &str) -> Vec<Recorded> {
    let v = reference(name);
    let notes: Vec<Recorded> = v["notes"]
        .as_array()
        .expect("notes")
        .iter()
        .map(|n| Recorded {
            midi: n["midi"].as_f64().expect("midi"),
            layer: n["layer"].as_str().expect("layer").to_string(),
            strength: n["strength"].as_f64().expect("strength"),
            grid_f0: n["grid_f0"].as_f64().expect("grid_f0"),
            b: n["b"].as_f64().expect("b"),
            release: n["release"].as_f64().expect("release"),
            yard: Yard::from_json(&n["yard"]).expect("a yard"),
            print: Fingerprint::from_json(&n["fingerprint"]).expect("a fingerprint"),
        })
        .collect();
    assert!(notes.len() >= 30, "{name}: only {} recordings", notes.len());
    notes
}

/// Runs `f` over `items` on every core, keeping the order.
fn par_map<T: Sync, U: Send>(items: &[T], f: impl Fn(&T) -> U + Sync) -> Vec<U> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = items.len().div_ceil(threads).max(1);
    std::thread::scope(|s| {
        let hs: Vec<_> = items
            .chunks(chunk)
            .map(|c| s.spawn(|| c.iter().map(&f).collect::<Vec<U>>()))
            .collect();
        hs.into_iter().flat_map(|h| h.join().expect("a worker")).collect()
    })
}

fn matches_its_recordings(bars: &Bars) {
    let inst = instrument(bars.name);
    let notes = recorded(bars.name);
    let organ = bars.name == "ORGAN";
    let diffs: Vec<(String, YardDiff, Option<f64>, f64)> = par_map(&notes, |n| {
        // held 4 s, then released; 6 s long
        let x = e2measure::engine2_note(inst, n.midi, n.strength, 4.0, 6.0, 0);
        let ours = e2measure::yard(&x, n.grid_f0, n.b, Some(4.0));
        // the stop wanted: the measured time constant to 40 dB down
        let want = n.release * std::f64::consts::LN_10 * 2.0;
        (
            format!("{:.0} {}", n.midi, n.layer),
            e2measure::yard_diff(&n.yard, &ours),
            ours.stop,
            want,
        )
    });
    let mut bad = Vec::new();
    let mut partials = Vec::new();
    for (note, d, stop, want) in &diffs {
        let partial = if organ { d.steady_db } else { d.early_db };
        let median = if organ { d.late_median_db } else { d.median_db };
        partials.push(partial);
        if partial > bars.partial_db {
            bad.push(format!("{note}: an overtone {partial:.1} dB off"));
        }
        if median > bars.median_db {
            bad.push(format!("{note}: the overtones' median {median:.1} dB off"));
        }
        if let Some(s) = bars.start_s
            && d.start_s.abs() > s
        {
            bad.push(format!("{note}: starts {:+.0} ms off", d.start_s * 1000.0));
        }
        match stop {
            Some(stop) if (stop - want).abs() <= bars.stop_s => {}
            _ => bad.push(format!(
                "{note}: stops in {stop:?} s, the recording's damper {want:.3} s"
            )),
        }
        if d.noise_early.abs() > bars.noise_early || d.noise_steady.abs() > bars.noise_steady {
            bad.push(format!(
                "{note}: noise {:+.3} early, {:+.3} held",
                d.noise_early, d.noise_steady
            ));
        }
    }
    partials.sort_by(f64::total_cmp);
    let typical = partials[partials.len() / 2];
    assert!(
        typical <= bars.typical_db,
        "{}: typically {typical:.1} dB off",
        bars.name
    );
    assert!(bad.is_empty(), "{}:\n{}", bars.name, bad.join("\n"));
}

#[test]
fn every_recorded_note_matches_its_recording_over_time() {
    matches_its_recordings(&PIANO);
    matches_its_recordings(&ORGAN);
}

/// The register a MIDI note is in, as the reference's spread has it.
fn region(midi: f64) -> &'static str {
    if midi < 48.0 {
        "low"
    } else if midi < 72.0 {
        "middle"
    } else {
        "high"
    }
}

fn inside_the_spread(bars: &Bars) {
    let inst = instrument(bars.name);
    let organ = bars.name == "ORGAN";
    let spread = &reference(bars.name)["spread"];
    let (lo, hi) = bf_content::fighters::voice(bars.name).expect("a voice").register();
    let tones: Vec<i32> = (lo..=hi).step_by(2).collect();
    // a piano struck at the parts' usual 0.8; an organ on its main chorus
    let strength = if organ { 1.0 } else { 0.8 };
    let set = inst.set();
    let yards: Vec<(i32, Yard)> = par_map(&tones, |&m| {
        let f0 = timbre::hz(f64::from(m));
        let b: f64 = set
            .nearest(f64::from(m), strength)
            .iter()
            .map(|(i, w)| w * set.notes[i].b)
            .sum();
        let x = e2measure::engine2_note(inst, f64::from(m), strength, 4.0, 6.0, 0);
        (m, e2measure::yard(&x, if organ { f0 / 2.0 } else { f0 }, b, None))
    });
    let mut bad = Vec::new();
    for (m, y) in &yards {
        let f0 = timbre::hz(f64::from(*m));
        let s = &spread[region(f64::from(*m))];
        for (what, v) in [
            ("bright_early", y.bright_early / f0),
            ("bright_late", y.bright_late / f0),
            ("fall", y.fall),
            ("noise_early", y.noise_early),
        ] {
            let (a, z) = (
                s[what][0].as_f64().expect("a low end"),
                s[what][1].as_f64().expect("a high end"),
            );
            let w = SPREAD_MARGIN * (z - a);
            if v < a - w || v > z + w {
                bad.push(format!("{m}: {what} {v:.3} outside {a:.3} to {z:.3}"));
            }
        }
    }
    assert!(bad.is_empty(), "{}:\n{}", bars.name, bad.join("\n"));
}

#[test]
fn every_note_sits_inside_the_real_instruments_spread() {
    inside_the_spread(&PIANO);
    inside_the_spread(&ORGAN);
}

/// FluidR3's fingerprints, by reference and MIDI note.
fn fluidr3() -> BTreeMap<String, BTreeMap<i32, Fingerprint>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/reference/fluidr3.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("the fingerprints")).expect("JSON");
    v["references"]
        .as_object()
        .expect("the references")
        .iter()
        .map(|(name, r)| {
            let notes = r["notes"]
                .as_array()
                .expect("its notes")
                .iter()
                .filter_map(Fingerprint::from_json)
                .filter(|f| f.harmonics.iter().any(|&h| h > -200.0))
                .map(|f| (f.midi, f))
                .collect();
            (name.clone(), notes)
        })
        .collect()
}

fn nearer_its_own(bars: &Bars) {
    let inst = instrument(bars.name);
    let refs = fluidr3();
    let notes = recorded(bars.name);
    let rows: Vec<Vec<String>> = par_map(&notes, |n| {
        let m = n.midi.round() as i32;
        // held a second, as the fingerprints were
        let x = e2measure::engine2_note(inst, n.midi, n.strength, 1.0, 2.0, 0);
        let print = timbre::fingerprint(&x, m, 1.0, true);
        let own = timbre::distance(&print.harmonics, &n.print.harmonics);
        let mut bad = Vec::new();
        if own > bars.own_db {
            bad.push(format!("{m} {}: {own:.1} from its recording", n.layer));
        }
        for r in bars.rivals {
            if let Some(rr) = refs[*r].get(&m) {
                let d = timbre::distance(&print.harmonics, &rr.harmonics);
                if d <= own {
                    bad.push(format!(
                        "{m} {}: {own:.1} from its recording, {d:.1} from the real {r}",
                        n.layer
                    ));
                }
            }
        }
        bad
    });
    let bad: Vec<String> = rows.into_iter().flatten().collect();
    assert!(bad.is_empty(), "{}:\n{}", bars.name, bad.join("\n"));
}

#[test]
fn each_note_is_nearer_its_own_instrument_than_its_rivals() {
    nearer_its_own(&PIANO);
    nearer_its_own(&ORGAN);
}

/// A note's level over its first half second (dB) and brightness (its spectral centroid over 50 to 500 ms, Hz).
fn level_and_brightness(x: &[f64], f0: f64) -> (f64, f64) {
    let n = 24_000.min(x.len());
    let ms = x[..n].iter().map(|v| v * v).sum::<f64>() / n as f64;
    (
        10.0 * ms.max(1e-30).log10(),
        e2measure::yard(x, f0, 0.0, None).bright_early,
    )
}

fn repeats_differ(bars: &Bars) {
    let inst = instrument(bars.name);
    let (lo, hi) = bf_content::fighters::voice(bars.name).expect("a voice").register();
    let strength = if bars.name == "ORGAN" { 1.0 } else { 0.8 };
    for m in [lo, 60, hi] {
        let f0 = timbre::hz(f64::from(m));
        let notes: Vec<Vec<f64>> = (0..3)
            .map(|v| e2measure::engine2_note(inst, f64::from(m), strength, 0.5, 1.5, v))
            .collect();
        let (l0, b0) = level_and_brightness(&notes[0], f0);
        for (v, x) in notes.iter().enumerate().skip(1) {
            assert_ne!(x, &notes[0], "{} {m}: repeat {v} is the first note again", bars.name);
            assert_eq!(
                x,
                &e2measure::engine2_note(inst, f64::from(m), strength, 0.5, 1.5, v as u64)
            );
            let (l, b) = level_and_brightness(x, f0);
            assert!(
                (l - l0).abs() < 1.5,
                "{} {m}: repeat {v} is {:+.1} dB",
                bars.name,
                l - l0
            );
            assert!(
                (b / b0 - 1.0).abs() < 0.12,
                "{} {m}: repeat {v} is {:.2}x as bright",
                bars.name,
                b / b0
            );
        }
    }
}

#[test]
fn repeats_differ_within_bounds() {
    repeats_differ(&PIANO);
    repeats_differ(&ORGAN);
}

/// A note's level from `a` to `b` seconds, dB (silence as -300).
fn level_over(x: &[f64], a: f64, b: f64) -> f64 {
    let (i, j) = ((a * 48_000.0) as usize, ((b * 48_000.0) as usize).min(x.len()));
    if j <= i {
        return -300.0;
    }
    let ms = x[i..j].iter().map(|v| v * v).sum::<f64>() / (j - i) as f64;
    10.0 * ms.max(1e-30).log10()
}

/// The largest change in level between the 50 ms before and the 50 ms after any moment from `a` to `b` (every 10
/// ms), dB.
fn worst_step(x: &[f64], a: f64, b: f64) -> f64 {
    let mut worst = 0.0f64;
    let mut t = a;
    while t < b {
        worst = worst.max((level_over(x, t, t + 0.05) - level_over(x, t - 0.05, t)).abs());
        t += 0.01;
    }
    worst
}

/// Every recorded note held past its recipe's end (Review Focus 3 of the stage 1 plan): across the end it moves no
/// more in 50 ms than it moved in the second before (by 1.5 dB at most), and a piano's tone keeps dying at least as
/// fast as `decay_floor` (within 20%, for the strings beating against each other).
fn carries_on(bars: &Bars) {
    let inst = instrument(bars.name);
    let set = inst.set();
    let end = set.grid[set.grid.len() - 1];
    let piano = bars.name == "PIANO";
    let notes = recorded(bars.name);
    let found = par_map(&notes, |n| {
        let x = e2measure::engine2_note(inst, n.midi, n.strength, end + 6.0, end + 4.5, 0);
        let (own, across) = (
            worst_step(&x, end - 1.0, end - 0.1),
            worst_step(&x, end - 0.05, end + 0.6),
        );
        let fall = (level_over(&x, end + 1.0, end + 1.1) - level_over(&x, end + 4.0, end + 4.1)) / 3.0;
        let heard = level_over(&x, end + 4.0, end + 4.1) > level_over(&x, 0.0, 1.0) - 80.0;
        (n.midi, n.layer.clone(), own, across, fall, heard)
    });
    for (midi, layer, own, across, fall, heard) in found {
        assert!(
            across <= own + 1.5,
            "{} {midi} {layer}: {across:.2} dB in 50 ms across the recipe's end ({own:.2} dB before it)",
            bars.name
        );
        let floor = bf_sound::voice::engine2::player::decay_floor(midi);
        if piano && heard {
            assert!(
                fall >= 0.8 * floor,
                "{} {midi} {layer}: dies {fall:.2} dB a second past its recipe (at least {floor:.2})",
                bars.name
            );
        }
    }
}

#[test]
fn notes_carry_on_past_their_recipes_without_a_step() {
    carries_on(&PIANO);
    carries_on(&ORGAN);
}
