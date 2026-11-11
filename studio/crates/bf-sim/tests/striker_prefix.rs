//! Striker parity with the Python kit: the FLUTE's baskets and cobras.
//!
//! Until the TROMBONE's lance first does something to the fight (a `hit` of sub-kind `blade`, a `swat` or a `block`)
//! or the first `ability` fires, what happens is the engine core plus the striker: the balls fly, bounce and clash,
//! baskets drop on the beat, cobras rise, lunge and bite. Over that span the Rust fight of the same seed and rules
//! must agree with the Python recording: `P`, `V`, `HP`, `BL` and the `COB` rows frame by frame, and every `basket`,
//! `strike`, `rope`, `clash` and `hit` (sub-kind `fang`) event.
//!
//! In the committed recordings that span is short (the lance lands its first hit two beats into the fight, before
//! any cobra has risen and found the enemy in reach), so it holds basket drops and `COB` rows but no lunge.

use std::path::PathBuf;

use bf_core::event::{Event, Value};
use bf_core::py;
use bf_core::recording::{Array, Recording};
use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{Rules, Setup, run};

/// Positions, velocities, hit points, the cobra rows and the float fields of events agree to this.
const TOL: f64 = 1e-9;

/// The five reference fights.
const FIVE: [&str; 5] = [
    "ep392.bfrec",
    "seed1.bfrec",
    "seed2.bfrec",
    "seed3.bfrec",
    "seed500.bfrec",
];

/// The other committed Python recordings: the fight with no K.O. and the scenario fights (rule overrides, and the
/// FLUTE in the right corner).
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

/// The largest difference between the first `frames` frames of a float array; panics past `TOL`.
fn compare_floats(name: &str, key: &str, rs: &Recording, py: &Recording, frames: usize) -> f64 {
    let ((rs_shape, rs_data), (py_shape, py_data)) = (floats(rs, key), floats(py, key));
    assert_eq!(&rs_shape[1..], &py_shape[1..], "{name}: the shape of {key}");
    assert!(
        rs_shape[0] >= frames,
        "{name}: the Rust fight has {} frames, {frames} are needed",
        rs_shape[0]
    );
    let per_frame: usize = py_shape[1..].iter().product();
    let mut worst = 0.0f64;
    for i in 0..frames * per_frame {
        let d = (rs_data[i] - py_data[i]).abs();
        assert!(
            d <= TOL,
            "{name}: {key} differs by {d:e} at frame {} (element {}): {} vs Python's {}",
            i / per_frame,
            i % per_frame,
            rs_data[i],
            py_data[i]
        );
        worst = worst.max(d);
    }
    worst
}

/// The `COB` rows of a recording: one entry per frame, each `[fighter 0's baskets, fighter 1's baskets]`.
fn cob(name: &str, who: &str, rec: &Recording) -> Vec<Value> {
    match rec.arrays.get("COB") {
        Some(Array::Nested(Value::List(rows))) => rows.clone(),
        other => panic!("{name}: the {who} recording's COB is {other:?}, not a list per frame"),
    }
}

/// The largest float difference between two values of the same layout (lists of the same length, the same whole
/// numbers and texts); panics where the layouts differ or a float differs by more than `TOL`.
fn compare_values(what: &str, a: &Value, b: &Value) -> f64 {
    match (a, b) {
        (Value::List(x), Value::List(y)) => {
            assert_eq!(x.len(), y.len(), "{what}: {a:?} vs Python's {b:?}");
            x.iter()
                .zip(y)
                .map(|(p, q)| compare_values(what, p, q))
                .fold(0.0, f64::max)
        }
        (Value::Float(x), Value::Float(y)) => {
            let d = (x - y).abs();
            assert!(d <= TOL, "{what}: {x} vs Python's {y} ({d:e} apart)");
            d
        }
        _ => {
            assert_eq!(a, b, "{what}");
            0.0
        }
    }
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

/// Whether an event ends the span: the lance did something (a `hit` of sub-kind `blade`, a `swat`, a `block`), or
/// an ability fired. Those are the work of the thrust blade and the abilities.
fn ends_the_span(e: &Event) -> bool {
    match e.kind.as_str() {
        "swat" | "block" | "ability" => true,
        "hit" => e.s(7) == "blade",
        _ => false,
    }
}

/// Whether an event is one the core or the striker makes: `rope`, `clash`, `basket`, `strike`, or a cobra's bite.
fn core_or_striker(e: &Event) -> bool {
    match e.kind.as_str() {
        "rope" | "clash" | "basket" | "strike" => true,
        "hit" => e.s(7) == "fang",
        _ => false,
    }
}

/// What one comparison covered.
struct Span {
    /// Frames compared, from `f0` on.
    frames: usize,
    /// The time of the event that ended the span.
    t_end: f64,
    /// The largest position difference in those frames.
    max_dp: f64,
    /// The largest hit point difference.
    max_dhp: f64,
    /// The largest difference in the cobra rows.
    max_dcob: f64,
    /// `basket` events matched.
    baskets: usize,
    /// Basket rows compared (a basket counts once per frame it is in).
    cob_rows: usize,
}

/// Runs the Rust fight of a Python recording and compares the span before the lance's first deed or the first
/// ability.
fn compare(name: &str, roster: &[FighterCfg]) -> Span {
    let py = python_recording(name);
    let rs = run(&setup_like(&py, roster), true);

    let stop = py
        .events
        .iter()
        .find(|e| ends_the_span(e))
        .unwrap_or_else(|| panic!("{name}: the Python fight has no lance hit"));
    // Frame i is the state after step i * steps_per_frame, at t_fight + i * steps_per_frame * DT; an event carries
    // its step's time rounded to 5 places. A frame is before the event when its rounded time is.
    let (t_fight, dt) = (rule(&py, "t_fight"), rule(&py, "DT"));
    let per_frame = py::round_half_even(1.0 / (rule(&py, "FPS") * dt));
    let frame_t = |i: usize| py::round_nd(t_fight + i as f64 * per_frame * dt, 5);
    let frames = (0..py.frames()).take_while(|&i| frame_t(i) < stop.t).count();
    assert!(
        frames >= 20,
        "{name}: only {frames} frames before the {} at {} s",
        stop.kind,
        stop.t
    );

    let max_dp = compare_floats(name, "P", &rs, &py, frames);
    let max_dv = compare_floats(name, "V", &rs, &py, frames);
    let max_dhp = compare_floats(name, "HP", &rs, &py, frames);
    // the reach: the cobras' grows with each bite
    compare_floats(name, "BL", &rs, &py, frames);

    let (rs_cob, py_cob) = (cob(name, "Rust", &rs), cob(name, "Python", &py));
    assert_eq!(rs_cob.len(), rs.frames(), "{name}: one COB entry per frame");
    let (mut max_dcob, mut cob_rows) = (0.0f64, 0);
    for i in 0..frames {
        let what = format!("{name}: COB at frame {i}");
        max_dcob = max_dcob.max(compare_values(&what, &rs_cob[i], &py_cob[i]));
        let Value::List(sides) = &py_cob[i] else {
            panic!("{what}: {:?}", py_cob[i])
        };
        assert_eq!(sides.len(), 2, "{what}: one list per fighter");
        for side in sides {
            let Value::List(baskets) = side else {
                panic!("{what}: {side:?}")
            };
            cob_rows += baskets.len();
        }
    }

    let wanted = |rec: &Recording| -> Vec<Event> {
        rec.events
            .iter()
            .filter(|e| e.t < stop.t && core_or_striker(e))
            .cloned()
            .collect()
    };
    let (rs_events, py_events) = (wanted(&rs), wanted(&py));
    assert_eq!(
        rs_events.len(),
        py_events.len(),
        "{name}: core and striker events before {} s:\n Rust   {rs_events:?}\n Python {py_events:?}",
        stop.t
    );
    for (a, b) in rs_events.iter().zip(&py_events) {
        assert!(same_event(a, b), "{name}: event differs:\n Rust   {a:?}\n Python {b:?}");
    }
    let count = |kind: &str| py_events.iter().filter(|e| e.kind == kind).count();
    println!(
        "{name}: agrees for {frames} frames, until the {} at {} s; max |dP| {max_dp:.3e}, |dV| {max_dv:.3e}, |dHP| \
         {max_dhp:.3e}, |dCOB| {max_dcob:.3e} over {cob_rows} basket rows; events matched: {} basket, {} strike, {} \
         fang hit, {} rope, {} clash",
        stop.kind,
        stop.t,
        count("basket"),
        count("strike"),
        count("hit"),
        count("rope"),
        count("clash"),
    );
    Span {
        frames,
        t_end: stop.t,
        max_dp,
        max_dhp,
        max_dcob,
        baskets: count("basket"),
        cob_rows,
    }
}

/// Brief, step 1: for the five reference recordings, `P`, `V`, `HP`, the `COB` rows and the events of kinds
/// `basket`, `strike`, `rope`, `clash` and `hit` (sub-kind `fang`) agree with Python to 1e-9 from `f0` until the
/// first event the trombone's lance causes or the first `ability`.
#[test]
fn striker_prefix() {
    let roster = roster();
    for name in FIVE {
        let span = compare(name, &roster);
        assert!(span.frames >= 20 && span.t_end > 4.4, "{name}");
        assert!(span.max_dp <= TOL && span.max_dhp <= TOL && span.max_dcob <= TOL);
        // the first basket drops on the fight's first beat, so it is in every frame but f0
        assert!(span.baskets >= 1, "{name}: the first basket");
        assert_eq!((span.baskets, span.cob_rows), (1, span.frames - 1), "{name}");
    }
}

/// The same for the fight with no K.O. (two baskets before the lance lands) and every scenario fight; in the swapped
/// ones the FLUTE is fighter 1, so its baskets are the second list of a `COB` row.
#[test]
fn the_other_recordings_agree_too() {
    let roster = roster();
    let mut most_baskets = 0;
    for name in OTHERS {
        let span = compare(name, &roster);
        assert!(span.cob_rows >= span.frames - 1, "{name}");
        most_baskets = most_baskets.max(span.baskets);
    }
    assert_eq!(most_baskets, 2, "no_ko drops its second basket before the lance lands");
}

/// The `COB` key is laid out as Python's: a list per frame, a list per fighter, a row of six per basket (the id a
/// whole number, the rest floats), and the fighter without baskets has an empty list. `f0` is before the first drop.
#[test]
fn cob_rows_have_the_python_layout() {
    let roster = roster();
    for name in ["ep392.bfrec", "scenarios/swapped_s1.bfrec"] {
        let py = python_recording(name);
        let rs = run(&setup_like(&py, &roster), true);
        let flute = py.names.iter().position(|n| n == "FLUTE").expect("the FLUTE fights");
        let rows = cob(name, "Rust", &rs);
        assert_eq!(rows.len(), rs.frames());
        let empty = || Value::List(vec![]);
        assert_eq!(rows[0], Value::List(vec![empty(), empty()]), "{name}: frame f0");
        assert_eq!(rows[0], cob(name, "Python", &py)[0]);
        let mut most = 0;
        for (i, row) in rows.iter().enumerate() {
            let Value::List(sides) = row else {
                panic!("{name}: COB[{i}] is {row:?}")
            };
            assert_eq!(sides.len(), 2, "{name}: COB[{i}]");
            assert_eq!(sides[1 - flute], empty(), "{name}: the TROMBONE has no baskets");
            let Value::List(baskets) = &sides[flute] else {
                panic!("{name}: COB[{i}] is {row:?}")
            };
            assert!(baskets.len() <= 2, "{name}: at most max_baskets = 2");
            most = most.max(baskets.len());
            for b in baskets {
                let Value::List(f) = b else { panic!("{name}: {b:?}") };
                assert!(
                    matches!(
                        f.as_slice(),
                        [
                            Value::Int(_),
                            Value::Float(_),
                            Value::Float(_),
                            Value::Float(_),
                            Value::Float(_),
                            Value::Float(_)
                        ]
                    ),
                    "{name}: COB[{i}] basket {b:?}"
                );
            }
        }
        assert_eq!(most, 2, "{name}");
        // frame 1 holds the first basket: where the FLUTE started, dropped at the first step's time
        let (t_fight, dt) = (rule(&py, "t_fight"), rule(&py, "DT"));
        let c = py.corner[flute];
        let first = Value::List(vec![
            Value::Int(0),
            Value::Float(c[0]),
            Value::Float(c[1]),
            Value::Float(c[0]),
            Value::Float(c[1]),
            Value::Float(t_fight + dt),
        ]);
        let mut want = vec![empty(), empty()];
        want[flute] = Value::List(vec![first]);
        assert_eq!(rows[1], Value::List(want), "{name}: frame 1");
        assert_eq!(
            rows[1],
            cob(name, "Python", &py)[1],
            "{name}: frame 1 is Python's, bit for bit"
        );
    }
}
