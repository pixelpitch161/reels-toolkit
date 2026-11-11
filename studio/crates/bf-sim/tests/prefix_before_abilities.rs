//! Parity with the Python kit until the first ability: the engine core, the FLUTE's cobras and the TROMBONE's lance.
//!
//! Until the first `ability` event nothing acts in a fight of FLUTE against TROMBONE but what the engine has ported:
//! the balls fly, bounce and clash, baskets drop and cobras rise, lunge and bite, and the lance tracks, thrusts, hits,
//! grows and swats cobras. Over that window the Rust fight of the same fighters, rules and seed must agree with the
//! Python recording: EVERY event, in order (kind, time and whole numbers exactly, floats to 1e-6), and EVERY
//! per-frame key the recording has (`P`, `V`, `TH`, `BL`, `FR`, `HP`, `LV`, `EXT`, the `COB` rows and the rest; whole
//! numbers exactly, floats to 1e-6).
//!
//! The window ends at the first `ability` event (the abilities are the next task's): the events before it in the
//! list, and the frames recorded before its step.
//!
//! The bar is a tolerance, not equal bits: the two engines' maths differ in the last bit now and then (the
//! transcendentals, and numpy's fused dot product of two 2-vectors on the reference PC, where the engine takes the
//! plain sum). Each test prints, per recording, how long the window is, what ended it, how many events of each kind
//! it holds and the largest differences seen: run with `--nocapture` to read them.

mod support;

use std::collections::BTreeMap;

use bf_core::event::{Event, Value};
use bf_core::py;
use bf_core::recording::{Array, Recording};
use bf_sim::roster::FighterCfg;
use bf_sim::run;
use support::{TOL, compare_array, compare_event, count_kinds, kind_of, rule};

/// The five reference fights.
const FIVE: [&str; 5] = [
    "ep392.bfrec",
    "seed1.bfrec",
    "seed2.bfrec",
    "seed3.bfrec",
    "seed500.bfrec",
];

/// The other committed Python recordings: the fight with no K.O. and the scenario fights (rule overrides, and the
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

fn python_recording(name: &str) -> Recording {
    support::python_recording(&format!("flute_vs_trombone/{name}"))
}

/// What one comparison covered.
struct Window {
    /// Frames compared, from `f0` on.
    frames: usize,
    /// Events compared: all of them before the first `ability`.
    events: usize,
    /// Those events by kind (`hit` and `block` with what hit).
    kinds: BTreeMap<String, usize>,
    /// The largest difference per per-frame key.
    worst: BTreeMap<String, f64>,
    /// The largest difference in a float of an event.
    worst_event: f64,
}

impl Window {
    fn count(&self, kind: &str) -> usize {
        self.kinds.get(kind).copied().unwrap_or(0)
    }
}

/// The time of frame `i` of a recording as an event would carry it: the frame is the state after step
/// `i * steps_per_frame`, at `t_fight + i * steps_per_frame * DT`, and an event carries its step's time rounded to 5
/// places.
fn frame_time(py: &Recording, i: usize) -> f64 {
    let (t_fight, dt) = (rule(py, "t_fight"), rule(py, "DT"));
    let per_frame = py::round_half_even(1.0 / (rule(py, "FPS") * dt));
    py::round_nd(t_fight + i as f64 * per_frame * dt, 5)
}

/// The largest position difference over the frames before time `t`, without judging it: what a failure reports.
fn position_gap(rs: &Recording, py: &Recording, t: f64) -> f64 {
    let (Some(Array::F64 { data: a, .. }), Some(Array::F64 { data: b, .. })) = (rs.arrays.get("P"), py.arrays.get("P"))
    else {
        return f64::NAN;
    };
    let frames = (0..py.frames()).take_while(|&i| frame_time(py, i) < t).count();
    a.iter()
        .zip(b)
        .take(frames * 4)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// Runs the Rust fight of a Python recording and compares everything before the first ability.
fn compare(name: &str, roster: &[FighterCfg]) -> Window {
    let py = python_recording(name);
    let rs = run(&support::setup_like(&py, roster), true);

    let first = py
        .events
        .iter()
        .position(|e| e.kind == "ability")
        .unwrap_or_else(|| panic!("{name}: the Python fight has no ability"));
    let stop = &py.events[first];

    // every event before it, in order
    let mut worst_event = 0.0f64;
    for (i, b) in py.events[..first].iter().enumerate() {
        // what a break reports: the events around it and how far apart the balls are by then
        let context = || {
            let around = |events: &[Event]| -> String {
                let (from, upto) = (i.saturating_sub(3).min(events.len()), (i + 3).min(events.len()));
                (from..upto).map(|j| format!("\n   {j}: {:?}", events[j])).collect()
            };
            format!(
                "{name}: event {i} of {first} before the {} at {} s differs; the largest position difference in the \
                 frames before {} s is {:e}\n Rust:{}\n Python:{}",
                stop.s(3),
                stop.t,
                b.t,
                position_gap(&rs, &py, b.t),
                around(&rs.events),
                around(&py.events),
            )
        };
        let Some(a) = rs.events.get(i) else {
            panic!("the Rust fight has only {i} events\n{}", context())
        };
        if a.kind != b.kind || a.t.to_bits() != b.t.to_bits() || a.fields.len() != b.fields.len() {
            panic!("{}", context());
        }
        worst_event = worst_event.max(compare_event(&format!("{name}: event {i}"), a, b));
    }

    // every per-frame key, over the frames recorded before the ability's step
    let frames = (0..py.frames()).take_while(|&i| frame_time(&py, i) < stop.t).count();
    assert_eq!(
        rs.arrays.keys().collect::<Vec<_>>(),
        py.arrays.keys().collect::<Vec<_>>(),
        "{name}: the per-frame keys"
    );
    let worst: BTreeMap<String, f64> = py
        .arrays
        .iter()
        .map(|(key, py_array)| {
            let d = compare_array(&format!("{name}: {key}"), &rs.arrays[key], py_array, frames);
            (key.clone(), d)
        })
        .collect();

    let kinds = count_kinds(&py.events[..first]);
    println!(
        "{name}: agrees for {frames} frames and {first} events, until {} s ({:.2} s of fighting), ended by {} \
         ({}, fighter {}); events {kinds:?}; max |d|: events {worst_event:.1e}, P {:.1e}, V {:.1e}, HP {:.1e}, BL \
         {:.1e}, COB {:.1e}, TH {:.1e}, EXT {:.1e}",
        stop.t,
        stop.t - rule(&py, "t_fight"),
        stop.s(3),
        stop.s(4),
        stop.i(2),
        worst["P"],
        worst["V"],
        worst["HP"],
        worst["BL"],
        worst["COB"],
        worst["TH"],
        worst["EXT"],
    );
    Window {
        frames,
        events: first,
        kinds,
        worst,
        worst_event,
    }
}

/// The totals of some windows, by event kind.
fn totals(windows: &[Window]) -> BTreeMap<String, usize> {
    let mut all = BTreeMap::new();
    for w in windows {
        for (kind, n) in &w.kinds {
            *all.entry(kind.clone()).or_insert(0) += n;
        }
    }
    all
}

/// Brief, step 1: for the five reference recordings, positions, HP, `BL`, `COB` and the full event list agree with
/// Python (1e-6 on floats, exact on kinds and integers) from `f0` until the first `ability` event. Each window holds
/// both weapons at work: cobra lunges and bites, thrusts and lance hits; together they hold swats too.
#[test]
fn prefix_before_abilities() {
    let roster = support::roster();
    let windows: Vec<Window> = FIVE.iter().map(|name| compare(name, &roster)).collect();
    for (name, w) in FIVE.iter().zip(&windows) {
        // the first ability comes 7.6 to 12.6 s into these fights
        assert!(
            w.frames >= 229 && w.events >= 64,
            "{name}: {} frames, {} events",
            w.frames,
            w.events
        );
        for key in ["P", "HP", "BL", "COB"] {
            assert!(w.worst[key] <= TOL, "{name}: {key}");
        }
        assert!(w.worst_event <= TOL, "{name}");
        // the real recordings pin the lunges, the bites, the thrusts and the lance's hits
        for kind in ["strike", "hit/fang", "thrust", "hit/blade", "basket", "rope"] {
            assert!(
                w.count(kind) >= 1,
                "{name}: no {kind} before the first ability: {:?}",
                w.kinds
            );
        }
    }
    let all = totals(&windows);
    println!("the five reference recordings, events before the first ability: {all:?}");
    // What the five Python windows hold, all of it matched above. Neither fighter has a shield, so no thrust and no
    // bite is ever blocked in these fights: there is no `block`.
    let want = [
        ("basket", 35),
        ("clash", 9),
        ("hit/blade", 41),
        ("hit/fang", 32),
        ("level", 6),
        ("rope", 97),
        ("strike", 46),
        ("swat", 9),
        ("thrust", 126),
    ];
    assert_eq!(all, want.into_iter().map(|(k, n)| (k.to_string(), n)).collect());
}

/// The same for the fight with no K.O. and every scenario fight (early levels, an early `low`, an early K.O., other
/// seeds, and the TROMBONE in the left corner, where its lance steps before the cobras bite).
#[test]
fn the_other_recordings_agree_too() {
    let roster = support::roster();
    let windows: Vec<Window> = OTHERS.iter().map(|name| compare(name, &roster)).collect();
    for (name, w) in OTHERS.iter().zip(&windows) {
        // the shortest windows are the early-level scenarios: 2.7 s of fighting
        assert!(
            w.frames >= 80 && w.events >= 15,
            "{name}: {} frames, {} events",
            w.frames,
            w.events
        );
        assert!(
            w.count("thrust") >= 6 && w.count("hit/blade") >= 2,
            "{name}: {:?}",
            w.kinds
        );
    }
    let all = totals(&windows);
    println!("the 16 other recordings, events before the first ability: {all:?}");
    let want = [
        ("basket", 85),
        ("clash", 18),
        ("hit/blade", 95),
        ("hit/fang", 36),
        ("level", 20),
        ("rope", 228),
        ("strike", 69),
        ("swat", 23),
        ("thrust", 320),
    ];
    assert_eq!(all, want.into_iter().map(|(k, n)| (k.to_string(), n)).collect());
    // the swapped fights: the TROMBONE is fighter 0, and its lance swats there too
    let swapped: Vec<&Window> = OTHERS
        .iter()
        .zip(&windows)
        .filter(|(name, _)| name.contains("swapped"))
        .map(|(_, w)| w)
        .collect();
    assert_eq!(swapped.len(), 3);
    assert!(swapped.iter().all(|w| w.count("swat") >= 2 && w.count("hit/fang") >= 2));
}

/// The lance's per-frame keys are laid out as Python's: `BL`, `TH` and `EXT` are (frames, 2) floats and `FR` is
/// (frames, 2) whole numbers, one column per fighter in the order of `names`. Frame `f0` holds the lance closed
/// (`EXT` = `rest`) and aimed at the enemy, bit for bit as Python has it; after the first landed thrust the lance's
/// column of `BL` is one `grow` longer and `FR` one more, in the frame Python shows it.
#[test]
fn the_lance_rows_have_the_python_layout() {
    let roster = support::roster();
    for name in ["ep392.bfrec", "scenarios/swapped_s1.bfrec"] {
        let py = python_recording(name);
        let rs = run(&support::setup_like(&py, &roster), true);
        let lance = py
            .names
            .iter()
            .position(|n| n == "TROMBONE")
            .expect("the TROMBONE fights");
        let floats = |rec: &Recording, key: &str| match rec.arrays.get(key) {
            Some(Array::F64 { shape, data }) => {
                assert_eq!(shape[..], [rec.frames(), 2], "{name}: the shape of {key}");
                data.clone()
            }
            other => panic!("{name}: {key} is {other:?}"),
        };
        let wholes = |rec: &Recording, key: &str| match rec.arrays.get(key) {
            Some(Array::I64 { shape, data }) => {
                assert_eq!(shape[..], [rec.frames(), 2], "{name}: the shape of {key}");
                data.clone()
            }
            other => panic!("{name}: {key} is {other:?}"),
        };
        let (bl, th, ext, fr) = (
            floats(&rs, "BL"),
            floats(&rs, "TH"),
            floats(&rs, "EXT"),
            wholes(&rs, "FR"),
        );
        let (py_bl, py_th, py_ext, py_fr) = (
            floats(&py, "BL"),
            floats(&py, "TH"),
            floats(&py, "EXT"),
            wholes(&py, "FR"),
        );
        // frame f0: the FLUTE's reach and the lance's, the lance closed and aimed, as Python has them bit for bit
        let mut want = [1.4, 1.4];
        want[lance] = 0.95;
        assert_eq!(bl[..2], want, "{name}");
        assert_eq!(fr[lance], 1, "{name}: frets0");
        assert_eq!(
            (fr[1 - lance], ext[1 - lance]),
            (0, 0.0),
            "{name}: the FLUTE has no lance"
        );
        assert_eq!(ext[lance], 0.3, "{name}: rest");
        for (key, rs_row, py_row) in [("BL", &bl, &py_bl), ("TH", &th, &py_th), ("EXT", &ext, &py_ext)] {
            let bits = |row: &[f64]| [row[0].to_bits(), row[1].to_bits()];
            assert_eq!(bits(rs_row), bits(py_row), "{name}: {key} at frame f0");
        }
        assert_eq!(fr[..2], py_fr[..2], "{name}: FR at frame f0");
        // the first landed thrust: the frame that first shows the longer lance is the same one, with the same row
        let grown = |bl: &[f64]| (0..bl.len() / 2).find(|i| bl[2 * i + lance] > 0.95);
        let (i, py_i) = (
            grown(&bl).expect("the lance lands"),
            grown(&py_bl).expect("the lance lands"),
        );
        assert_eq!(i, py_i, "{name}: the frame of the first growth");
        assert_eq!(bl[2 * i + lance], 1.03, "{name}: base + grow");
        assert_eq!((fr[2 * i + lance], py_fr[2 * i + lance]), (2, 2), "{name}");
        assert_eq!(fr[2 * i + 1 - lance], 0, "{name}: the FLUTE's column stays 0");
        // the hit that grew it carries the fret count from before the growth as its sub
        let first_hit = |rec: &Recording| {
            let e = rec
                .events
                .iter()
                .find(|e| kind_of(e) == "hit/blade")
                .expect("a lance hit");
            (e.t, e.i(2), e.i(3), e.at(8).clone())
        };
        assert_eq!(first_hit(&rs), first_hit(&py), "{name}");
        assert_eq!(first_hit(&rs).3, Value::Int(1), "{name}");
    }
}
