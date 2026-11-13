//! Core parity with the Python kit: until the first hit, block or ability of a fight the balls only fly, bounce off
//! the ropes and clash, which is the engine core. Over that span the Rust fight of the same seed and rules must agree
//! with the Python recording: positions and velocities frame by frame, and every `rope` and `clash` event.
//!
//! The span ends at the first `hit`, `block` or `ability` event of the Python recording because those are the first
//! things a weapon or an ability changes about the balls; so the tests stay valid as the kinds are ported.

use std::path::PathBuf;

use bf_core::event::{Event, Value};
use bf_core::py;
use bf_core::recording::{Array, Recording, Source};
use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{Rules, Setup, run};

/// Positions, velocities and the float fields of events agree to this.
const TOL: f64 = 1e-9;
/// At least this many frames must be compared in every fight.
const MIN_FRAMES: usize = 20;

/// The five reference fights.
const FIVE: [&str; 5] = [
    "ep392.bfrec",
    "seed1.bfrec",
    "seed2.bfrec",
    "seed3.bfrec",
    "seed500.bfrec",
];

/// The other committed Python recordings: the fight with no K.O. and the scenario fights (rule overrides, and
/// TROMBONE in the left corner).
const OTHERS: [&str; 16] = [
    "no_ko.bfrec",
    "scenarios/early_ko_s1.bfrec",
    "scenarios/early_ko_s2.bfrec",
    "scenarios/early_ko_s3.bfrec",
    "scenarios/early_levels_s1.bfrec",
    "scenarios/early_levels_s2.bfrec",
    "scenarios/early_levels_s3.bfrec",
    "scenarios/early_low_s1.bfrec",
    "scenarios/early_low_s2.bfrec",
    "scenarios/early_low_s3.bfrec",
    "scenarios/plain_s7.bfrec",
    "scenarios/plain_s8.bfrec",
    "scenarios/plain_s9.bfrec",
    "scenarios/swapped_s1.bfrec",
    "scenarios/swapped_s2.bfrec",
    "scenarios/swapped_s3.bfrec",
];

/// The per-frame keys every recording has (`fight.py` `run`), with the member kind and the shape after the frame
/// count.
const BASE_KEYS: [(&str, &str, &[usize]); 15] = [
    ("P", "f8", &[2, 2]),
    ("V", "f8", &[2, 2]),
    ("TH", "f8", &[2]),
    ("BL", "f8", &[2]),
    ("FR", "i8", &[2]),
    ("NV", "i8", &[2]),
    ("HP", "f8", &[2]),
    ("LV", "i8", &[2]),
    ("SH", "f8", &[2]),
    ("HA", "f8", &[2]),
    ("SPIN", "f8", &[2]),
    ("OM", "f8", &[2]),
    ("K", "f8", &[72, 7]),
    ("WAVES", "json", &[]),
    ("SLAMS", "json", &[]),
];

fn python_recording(rel: &str) -> Recording {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference/fights/flute_vs_trombone")
        .join(rel);
    Recording::read(&path).unwrap_or_else(|e| panic!("{e}"))
}

fn roster() -> Vec<FighterCfg> {
    parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads")
}

fn rule(rec: &Recording, key: &str) -> f64 {
    match rec.rules.get(key) {
        Some(Value::Float(x)) => *x,
        Some(Value::Int(n)) => *n as f64,
        other => panic!("rules.{key} is {other:?}"),
    }
}

/// The same fight as a Python recording: its fighters in its order, its rules, its seed.
fn setup_like<'a>(py: &Recording, roster: &'a [FighterCfg]) -> Setup<'a> {
    let find = |name: &str| {
        roster
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("{name} is not in the roster"))
    };
    let mut rules = Rules::new(rule(py, "bpm"), rule(py, "t_fight"));
    rules.apply(&py.rules).expect("the recording's rules apply");
    Setup {
        fighters: [find(&py.names[0]), find(&py.names[1])],
        rules,
        seed: py.seed,
    }
}

fn floats<'a>(rec: &'a Recording, key: &str) -> (&'a [usize], &'a [f64]) {
    match rec.arrays.get(key) {
        Some(Array::F64 { shape, data }) => (shape, data),
        other => panic!("{key} is not a float array: {other:?}"),
    }
}

/// What one comparison covered.
struct Span {
    /// Frames compared, from `f0` on.
    frames: usize,
    /// The largest position difference in those frames.
    max_dp: f64,
    /// The largest velocity difference.
    max_dv: f64,
    /// `rope` events matched.
    ropes: usize,
    /// `clash` events matched.
    clashes: usize,
}

/// The largest difference between the first `frames` frames of a (frames, 2, 2) array; panics past `TOL`.
fn compare_frames(name: &str, key: &str, rs: &Recording, py: &Recording, frames: usize) -> f64 {
    let ((rs_shape, rs_data), (py_shape, py_data)) = (floats(rs, key), floats(py, key));
    assert_eq!(&rs_shape[1..], &[2, 2], "{name}: {key}");
    assert_eq!(&py_shape[1..], &[2, 2], "{name}: {key}");
    assert!(
        rs_shape[0] >= frames,
        "{name}: the Rust fight has {} frames, {frames} are needed",
        rs_shape[0]
    );
    let mut worst = 0.0f64;
    for i in 0..frames * 4 {
        let d = (rs_data[i] - py_data[i]).abs();
        assert!(
            d <= TOL,
            "{name}: {key} differs by {d:e} at frame {} (fighter {}, axis {}): {} vs Python's {}",
            i / 4,
            i / 2 % 2,
            i % 2,
            rs_data[i],
            py_data[i]
        );
        worst = worst.max(d);
    }
    worst
}

/// Whether two events are the same: kind and time exactly, whole numbers and texts exactly, floats to `TOL`.
fn same_event(a: &Event, b: &Event) -> bool {
    a.kind == b.kind
        && a.t.to_bits() == b.t.to_bits()
        && a.fields.len() == b.fields.len()
        && a.fields.iter().zip(&b.fields).all(|(x, y)| match (x, y) {
            (Value::Float(x), Value::Float(y)) => (x - y).abs() <= TOL,
            _ => x == y,
        })
}

/// Runs the Rust fight of a Python recording and compares the span before the first hit, block or ability.
fn compare(name: &str, roster: &[FighterCfg]) -> Span {
    let py = python_recording(name);
    let rs = run(&setup_like(&py, roster), true).expect("the engine runs this fight");

    let first = py
        .events
        .iter()
        .find(|e| matches!(e.kind.as_str(), "hit" | "block" | "ability"))
        .unwrap_or_else(|| panic!("{name}: the Python fight has no hit"));
    // Frame i is the state after step i * steps_per_frame, at t_fight + i * steps_per_frame * DT; an event carries
    // its step's time rounded to 5 places. A frame is before the event when its rounded time is.
    let (t_fight, dt) = (rule(&py, "t_fight"), rule(&py, "DT"));
    let per_frame = py::round_half_even(1.0 / (rule(&py, "FPS") * dt));
    let frame_t = |i: usize| py::round_nd(t_fight + i as f64 * per_frame * dt, 5);
    let frames = (0..py.frames()).take_while(|&i| frame_t(i) < first.t).count();
    assert!(
        frames >= MIN_FRAMES,
        "{name}: only {frames} frames before the first {} at {} s",
        first.kind,
        first.t
    );

    let max_dp = compare_frames(name, "P", &rs, &py, frames);
    let max_dv = compare_frames(name, "V", &rs, &py, frames);

    let core = |rec: &Recording| -> Vec<Event> {
        rec.events
            .iter()
            .filter(|e| e.t < first.t && matches!(e.kind.as_str(), "rope" | "clash"))
            .cloned()
            .collect()
    };
    let (rs_events, py_events) = (core(&rs), core(&py));
    assert_eq!(
        rs_events.len(),
        py_events.len(),
        "{name}: rope and clash events before {} s:\n Rust   {rs_events:?}\n Python {py_events:?}",
        first.t
    );
    for (a, b) in rs_events.iter().zip(&py_events) {
        assert!(same_event(a, b), "{name}: event differs:\n Rust   {a:?}\n Python {b:?}");
    }
    let count = |kind: &str| py_events.iter().filter(|e| e.kind == kind).count();
    let span = Span {
        frames,
        max_dp,
        max_dv,
        ropes: count("rope"),
        clashes: count("clash"),
    };
    println!(
        "{name}: {} frames before the first {} at {} s; max |dP| {:.3e}, max |dV| {:.3e}; {} rope and {} clash \
         events match",
        span.frames, first.kind, first.t, span.max_dp, span.max_dv, span.ropes, span.clashes
    );
    span
}

/// Brief, step 1: for each of the five reference recordings, `P` and `V` agree frame by frame up to the frame before
/// the first `hit`, `block` or `ability` of the Python recording, to 1e-9 over at least 20 frames, and the `rope`
/// and `clash` events in that span match in order, time and fields.
#[test]
fn motion_agrees_until_first_hit() {
    let roster = roster();
    let mut clashes = 0;
    for name in FIVE {
        let span = compare(name, &roster);
        assert!(span.frames >= MIN_FRAMES);
        clashes += span.clashes;
    }
    assert!(clashes >= 4, "four of the five fights open with a clash: {clashes}");
}

/// The same for the fight with no K.O. and every scenario fight: rule overrides do not touch the opening, the
/// swapped pair launches from the other corners, and `no_ko` flies for 81 frames and bounces off two ropes first (the
/// seeded rope give is drawn from the fight's generator).
#[test]
fn the_other_recordings_agree_too() {
    let roster = roster();
    let (mut ropes, mut longest) = (0, 0);
    for name in OTHERS {
        let span = compare(name, &roster);
        ropes += span.ropes;
        longest = longest.max(span.frames);
    }
    assert!(
        ropes >= 2,
        "no_ko bounces off the ropes twice before its first hit: {ropes}"
    );
    assert!(
        longest >= 80,
        "no_ko flies for 81 frames before its first hit: {longest}"
    );
}

/// The recording the engine writes has the Python recording's base keys with the same kinds and shapes, the same
/// header fields, and its first frame.
#[test]
fn the_recording_has_the_python_keys_kinds_and_shapes() {
    let roster = roster();
    let py = python_recording("ep392.bfrec");
    let rs = run(&setup_like(&py, &roster), true).expect("the engine runs this fight");

    assert_eq!(rs.source, Source::studio());
    assert_eq!(rs.format, py.format);
    assert_eq!((&rs.names, rs.seed, rs.f0, rs.fps), (&py.names, py.seed, py.f0, py.fps));
    assert_eq!(rs.corner, py.corner);
    assert_eq!(
        rs.rules, py.rules,
        "the rules are written under the Python names, with the Python types"
    );
    assert_eq!((rs.f0, rs.fps), (108, 30));

    let frames = rs.frames();
    assert!(frames >= 27, "{frames}");
    for (key, kind, tail) in BASE_KEYS {
        let (r, p) = (
            rs.arrays
                .get(key)
                .unwrap_or_else(|| panic!("the Rust recording has no {key}")),
            py.arrays
                .get(key)
                .unwrap_or_else(|| panic!("the Python recording has no {key}")),
        );
        match (kind, r, p) {
            ("f8", Array::F64 { shape, data }, Array::F64 { shape: py_shape, .. }) => {
                assert_eq!(shape[0], frames, "{key}");
                assert_eq!((&shape[1..], &py_shape[1..]), (tail, tail), "{key}");
                assert_eq!(data.len(), frames * tail.iter().product::<usize>(), "{key}");
            }
            ("i8", Array::I64 { shape, data }, Array::I64 { shape: py_shape, .. }) => {
                assert_eq!(shape[0], frames, "{key}");
                assert_eq!((&shape[1..], &py_shape[1..]), (tail, tail), "{key}");
                assert_eq!(data.len(), frames * tail.iter().product::<usize>(), "{key}");
            }
            ("json", Array::Nested(Value::List(rows)), Array::Nested(Value::List(_))) => {
                assert_eq!(rows.len(), frames, "{key}");
            }
            _ => panic!("{key}: expected {kind} in both, found {r:?} and {p:?}"),
        }
    }
    // every key the engine writes is one the Python recording has, of the same kind
    for (key, a) in &rs.arrays {
        let p = py
            .arrays
            .get(key)
            .unwrap_or_else(|| panic!("the Python recording has no {key}"));
        assert_eq!(
            std::mem::discriminant(a),
            std::mem::discriminant(p),
            "{key} is stored differently"
        );
    }
    // the lance thrusts and SAD TROMBONE slows, so the three opt-in keys are there too
    for key in ["EXT", "SLW", "CRV"] {
        let ((shape, _), (py_shape, _)) = (floats(&rs, key), floats(&py, key));
        assert_eq!(
            (shape[0], &shape[1..], &py_shape[1..]),
            (frames, &[2][..], &[2][..]),
            "{key}"
        );
    }

    // frame 0, before the first step: every array equals Python's (floats that come from sin, cos and atan2 to 1e-12)
    for (key, a) in &rs.arrays {
        match (a, &py.arrays[key]) {
            (Array::F64 { shape, data }, Array::F64 { data: py_data, .. }) => {
                let n: usize = shape[1..].iter().product();
                for i in 0..n {
                    let same = data[i].to_bits() == py_data[i].to_bits() || (data[i] - py_data[i]).abs() <= 1e-12;
                    assert!(same, "{key}[0] element {i}: {} vs Python's {}", data[i], py_data[i]);
                }
            }
            (Array::I64 { shape, data }, Array::I64 { data: py_data, .. }) => {
                let n: usize = shape[1..].iter().product();
                assert_eq!(data[..n], py_data[..n], "{key}[0]");
            }
            (Array::Nested(Value::List(rows)), Array::Nested(Value::List(py_rows))) => {
                assert_eq!(rows[0], py_rows[0], "{key}[0]");
            }
            _ => unreachable!("the kinds were compared above"),
        }
    }
    // no projectiles: every slot of K is numpy's NaN, bit for bit
    let (_, k) = floats(&rs, "K");
    assert!(k.iter().all(|x| x.to_bits() == 0x7FF8_0000_0000_0000));
    let (_, py_k) = floats(&py, "K");
    assert_eq!(py_k[0].to_bits(), 0x7FF8_0000_0000_0000);
}

/// A recording the engine made can be written and read back unchanged, and the same setup always gives the same
/// recording (the content hash says so).
#[test]
fn a_recording_round_trips_and_repeats() {
    let roster = roster();
    let py = python_recording("seed1.bfrec");
    let setup = setup_like(&py, &roster);
    let rs = run(&setup, true).expect("the engine runs this fight");
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("core_prefix_seed1.bfrec");
    rs.write(&path).unwrap_or_else(|e| panic!("{e}"));
    let back = Recording::read(&path).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, rs);
    assert_eq!(back.hash(), rs.hash());
    let _ = std::fs::remove_file(&path);

    assert_eq!(
        run(&setup, true).expect("the engine runs this fight"),
        rs,
        "the same setup gives the same recording"
    );
    let mut other = setup.clone();
    other.seed = 2;
    assert_ne!(
        run(&other, true).expect("the engine runs this fight").hash(),
        rs.hash(),
        "another seed is another fight"
    );

    // without recording: the same events, winner and K.O. time, and no per-frame arrays
    let quiet = run(&setup, false).expect("the engine runs this fight");
    assert!(quiet.arrays.is_empty());
    assert_eq!(quiet.frames(), 0);
    assert_eq!(
        (&quiet.events, quiet.winner, quiet.t_ko.map(f64::to_bits)),
        (&rs.events, rs.winner, rs.t_ko.map(f64::to_bits))
    );
    assert_eq!((quiet.f0, quiet.fps, &quiet.rules), (rs.f0, rs.fps, &rs.rules));
}
