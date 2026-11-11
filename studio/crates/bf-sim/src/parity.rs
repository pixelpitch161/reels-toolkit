//! Parity with the Python kit: how long this engine's fight and the Python recording of the same fight agree.
//!
//! The two engines are not equal bit for bit: the maths differ in the last bit now and then (numpy's dot product of
//! two 2-vectors is fused on the reference PC, the engine's is the plain sum), and a fight is chaotic, so the two
//! drift apart. The bar is a window: from frame `f0`, every event and every per-frame value agree to
//! [`PREFIX_TOL`] for at least [`PREFIX_FLOOR_S`] seconds of fighting. [`prefix_window`] measures the window of one
//! fight and says in words what ended it:
//!
//! - **drift**: a float (a per-frame value, or a float field of an event that is otherwise the same) passes the bar
//!   while the frame before was already more than [`DRIFT_FROM`] apart: the two fights have been parting slowly, as
//!   the last-bit differences grow. That is how a window of a correct port ends.
//! - **a knife-edge**: one engine made an event the other did not make there, while every per-frame value before it
//!   is still inside the bar, and the event is of a kind that a comparison of positions decides (a `clash`, a `rope`
//!   bounce, a `hit`, ...). That is what a last-bit difference does when such a comparison is decided by the last
//!   bit (two balls that travel side by side exactly `R + R` apart clash or not on it). The words name the event.
//!   Only a `clash` carries a margin: the words say how close to `R + R` the balls are in the frames around it, and
//!   [`clash_margin`] replays the step and gives the margin of the comparison itself. A missing event is ALSO what a
//!   mistake in the port looks like, so a knife-edge before the floor is never passed silently: the margin tells the
//!   two apart.
//! - **a logic break**: a float outside the bar where the frame before was close (inside [`DRIFT_FROM`]: a jump, not
//!   a drift); a whole number, a text or the layout of a row or an event that differs; or an event only one engine
//!   made of a kind that the clock and whole hit points decide (an `ability`, a `level`, a `basket`, a `thrust`, a
//!   `slam`, a `ko`), which no last bit can make or unmake.
//!
//! [`prefix_window`] is three steps: the event scan (the first event that is not the same in both lists), the frame
//! scan (the frames before that event, up to the first one outside the bar), and the classification of whichever
//! break comes first.

use std::collections::BTreeMap;

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py;
use bf_core::recording::{Array, Recording};

use crate::fight::{Fight, Setup};
use crate::schedule::Schedule;

/// The floor of the agreement window: seconds of fighting (counted from `t_fight`) over which this engine and a
/// Python recording of the same fight must agree. A fight that ends sooner must agree to its end.
pub const PREFIX_FLOOR_S: f64 = 10.0;

/// The parity bar of a fight: positions, hit points and every other float of a frame or an event agree to this.
/// Whole numbers, texts, event kinds and event times are compared exactly.
pub const PREFIX_TOL: f64 = 1e-6;

/// A float that passes [`PREFIX_TOL`] is drift, not a jump, when the frame before was already more than this apart.
pub const DRIFT_FROM: f64 = 1e-7;

/// What ended an agreement window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreakKind {
    /// A float passed the bar (a per-frame value, or a float field of an event that is otherwise the same) while the
    /// frame before was already more than [`DRIFT_FROM`] apart: the slow parting of two fights whose maths differ in
    /// the last bit.
    Drift,
    /// One engine made an event the other did not make there (the next events differ in kind or in time), of a kind
    /// that a comparison of positions decides, while every per-frame value before it is inside the bar.
    KnifeEdge,
    /// A float jumped outside the bar (the frame before was inside [`DRIFT_FROM`]), a whole number, a text or a
    /// layout differs, or only one engine made an event of a kind that the clock and whole hit points decide.
    Logic,
}

/// How long two recordings of the same fight agree.
#[derive(Clone, Debug, PartialEq)]
pub struct PrefixReport {
    /// The time, on the video clock, of the first break of either kind; the end of the fight (its K.O., else its
    /// last frame) when the two agree throughout.
    pub agree_until_s: f64,
    /// What broke first, in words: which kind of break, where, with the numbers. `None` when the two agree
    /// throughout.
    pub first_break: Option<String>,
    /// The kind of the first break.
    pub kind: Option<BreakKind>,
    /// Frames that agree, from `f0` on (those before the break).
    pub frames: usize,
    /// Events that agree (those before the break).
    pub events: usize,
    /// The largest float difference among those frames and events.
    pub worst: f64,
}

/// A number of a recording's rules.
fn rule(rec: &Recording, key: &str) -> Option<f64> {
    match rec.rules.get(key) {
        Some(Value::Float(x)) => Some(*x),
        Some(Value::Int(n)) => Some(*n as f64),
        _ => None,
    }
}

/// When each frame of a recording was taken: frame `i` is the state after `i * steps_per_frame` steps, and the last
/// frame of a fight with a K.O. is the state at the K.O.
struct Clock {
    t_fight: f64,
    /// `steps_per_frame * DT`.
    per_frame: f64,
}

impl Clock {
    fn of(rec: &Recording) -> Option<Clock> {
        let (t_fight, dt, fps) = (rule(rec, "t_fight")?, rule(rec, "DT")?, rule(rec, "FPS")?);
        let steps_per_frame = py::round_half_even(1.0 / (fps * dt));
        Some(Clock {
            t_fight,
            per_frame: steps_per_frame * dt,
        })
    }

    /// The time of frame `i` of `rec`.
    fn frame(&self, rec: &Recording, i: usize) -> f64 {
        match rec.t_ko {
            Some(t_ko) if i + 1 == rec.frames() => t_ko,
            _ => self.t_fight + i as f64 * self.per_frame,
        }
    }
}

/// The largest float difference between two values of the same layout, or what differs in their layout (a list's
/// length, a whole number, a text, a kind of value).
fn value_gap(a: &Value, b: &Value) -> Result<f64, String> {
    match (a, b) {
        (Value::List(x), Value::List(y)) => {
            if x.len() != y.len() {
                return Err(format!("{} items against Python's {}", x.len(), y.len()));
            }
            let mut worst = 0.0f64;
            for (p, q) in x.iter().zip(y) {
                worst = worst.max(value_gap(p, q)?);
            }
            Ok(worst)
        }
        (Value::Float(x), Value::Float(y)) => Ok(float_gap(*x, *y)),
        _ if a == b => Ok(0.0),
        _ => Err(format!("{a:?} against Python's {b:?}")),
    }
}

/// How far apart two floats are: 0 for the same bits (the NaN of an unused slot equals itself), infinitely far when
/// only one is a number.
fn float_gap(a: f64, b: f64) -> f64 {
    if a.to_bits() == b.to_bits() {
        return 0.0;
    }
    let d = (a - b).abs();
    if d.is_nan() { f64::INFINITY } else { d }
}

/// How far apart frame `i` of one key is in the two recordings: the largest float difference, or what differs
/// otherwise (a whole number, a row's layout).
fn frame_gap(rs: &Array, py: &Array, i: usize) -> Result<f64, String> {
    match (rs, py) {
        (Array::F64 { shape, data: a }, Array::F64 { data: b, .. }) => {
            let n: usize = shape[1..].iter().product();
            let (a, b) = (&a[i * n..(i + 1) * n], &b[i * n..(i + 1) * n]);
            Ok(a.iter().zip(b).map(|(x, y)| float_gap(*x, *y)).fold(0.0, f64::max))
        }
        (Array::I64 { shape, data: a }, Array::I64 { data: b, .. }) => {
            let n: usize = shape[1..].iter().product();
            let (a, b) = (&a[i * n..(i + 1) * n], &b[i * n..(i + 1) * n]);
            if a == b {
                Ok(0.0)
            } else {
                Err(format!("{a:?} against Python's {b:?}"))
            }
        }
        (Array::Nested(Value::List(a)), Array::Nested(Value::List(b))) => {
            value_gap(&a[i], &b[i]).map_err(|what| format!("{what}: {:?} against Python's {:?}", a[i], b[i]))
        }
        _ => Err("not the same kind of array".to_string()),
    }
}

/// The shape of one frame of an array (the kind of array and its dimensions after the first).
fn frame_shape(a: &Array) -> String {
    match a {
        Array::F64 { shape, .. } => format!("floats {:?}", shape.get(1..).unwrap_or(&[])),
        Array::I64 { shape, .. } => format!("whole numbers {:?}", shape.get(1..).unwrap_or(&[])),
        Array::Nested(_) => "rows".to_string(),
    }
}

/// How two events at the same place of the two lists compare.
enum EventGap {
    /// The same event: the largest difference of its float fields (inside the bar).
    Same(f64),
    /// The same kind at the same time, the same whole numbers, texts and layout, a float field outside the bar: the
    /// largest difference.
    Float(f64),
    /// The same kind at the same time with a whole number, a text or a layout that differs: what differs.
    Fields(String),
    /// Another kind or another time: one engine made an event the other did not make there.
    Other,
}

fn event_gap(a: &Event, b: &Event) -> EventGap {
    if a.kind != b.kind || a.t.to_bits() != b.t.to_bits() {
        return EventGap::Other;
    }
    match value_gap(&Value::List(a.fields.clone()), &Value::List(b.fields.clone())) {
        Ok(d) if d <= PREFIX_TOL => EventGap::Same(d),
        Ok(d) => EventGap::Float(d),
        Err(what) => EventGap::Fields(what),
    }
}

/// Where the two event lists stop telling the same story: the index and time of the first event that differs in
/// kind, time, a whole number or a text (floats are not looked at). `None` when they never do.
fn same_story_until(rust: &Recording, python: &Recording) -> Option<(usize, f64)> {
    let story = |e: &Event| -> Vec<Value> {
        let whole = e.fields.iter().filter(|v| !matches!(v, Value::Float(_)));
        whole.cloned().collect()
    };
    let n = rust.events.len().max(python.events.len());
    (0..n).find_map(|i| match (rust.events.get(i), python.events.get(i)) {
        (Some(a), Some(b)) => {
            let same = a.kind == b.kind && a.t.to_bits() == b.t.to_bits() && a.fields.len() == b.fields.len();
            (!same || story(a) != story(b)).then(|| (i, a.t.min(b.t)))
        }
        (Some(e), None) | (None, Some(e)) => Some((i, e.t)),
        (None, None) => None,
    })
}

/// Whether an event of this kind is made or not by a comparison of positions (the balls overlap, a ball is past a
/// rope, a weapon or a ring reaches the enemy), which a last-bit difference between the engines can decide. The
/// other kinds follow the beat grid, the clock and sums of damage, which are the same numbers in both engines as
/// long as the events before agree: `ability`, `level`, `basket`, `thrust`, `slam` (its time; whether it hits is a
/// field) and `ko`.
fn positions_decide(kind: &str) -> bool {
    matches!(
        kind,
        "clash" | "rope" | "hit" | "block" | "strike" | "swat" | "slow" | "parry" | "cleared"
    )
}

/// The distance between the two balls in frame `i` of a recording, less `R + R`.
fn apart(rec: &Recording, i: usize) -> Option<f64> {
    let Some(Array::F64 { data, .. }) = rec.arrays.get("P") else {
        return None;
    };
    let p = data.get(4 * i..4 * i + 4)?;
    Some(dmath::hypot(p[2] - p[0], p[3] - p[1]) - 2.0 * rule(rec, "R")?)
}

/// A report that ends at `t` with a break.
fn broken(t: f64, kind: BreakKind, words: String, frames: usize, events: usize, worst: f64) -> PrefixReport {
    PrefixReport {
        agree_until_s: t,
        first_break: Some(words),
        kind: Some(kind),
        frames,
        events,
        worst,
    }
}

/// Measures how long a fight of this engine (`rust`) and the Python recording of the same fight (`python`) agree:
/// every event in order (kind, time, whole numbers and texts exactly, floats to [`PREFIX_TOL`]) and every per-frame
/// key (whole numbers and the layout of rows exactly, floats to [`PREFIX_TOL`]).
///
/// The first break in time order ends the window; an event comes before the frame of its own step. The report says
/// which kind of break it is ([`BreakKind`]) and, in words, where it is and how far apart the two are there. Both
/// recordings must hold their per-frame arrays.
pub fn prefix_window(rust: &Recording, python: &Recording) -> PrefixReport {
    let t_start = rule(python, "t_fight").unwrap_or(0.0);
    let clock = match comparable(rust, python) {
        Ok(clock) => clock,
        Err(what) => return broken(t_start, BreakKind::Logic, what, 0, 0, 0.0),
    };
    let events = scan_events(rust, python);
    let t_event = events.brk.as_ref().map_or(f64::INFINITY, |b| b.t);
    let frames = scan_frames(rust, python, &clock, t_event);
    if let Some(at) = &frames.brk {
        return frame_break(rust, python, &frames, at, &events.gaps);
    }
    match events.brk {
        Some(at) => event_break(rust, python, &frames, at, &events.gaps),
        None => no_break(rust, python, &clock, &frames, &events.gaps, t_start),
    }
}

/// Whether the two recordings are of the same fight with the same per-frame keys, each of the same shape; the clock
/// of their frames if so, else what differs.
fn comparable(rust: &Recording, python: &Recording) -> Result<Clock, String> {
    if rust.names != python.names || rust.seed != python.seed {
        return Err(format!(
            "not the same fight: {:?} seed {} against Python's {:?} seed {}",
            rust.names, rust.seed, python.names, python.seed
        ));
    }
    let clock = Clock::of(python).ok_or("the Python recording's rules lack t_fight, DT or FPS")?;
    let (rs_keys, py_keys): (Vec<&String>, Vec<&String>) =
        (rust.arrays.keys().collect(), python.arrays.keys().collect());
    if rs_keys != py_keys || py_keys.is_empty() {
        return Err(format!(
            "the per-frame keys differ: {rs_keys:?} against Python's {py_keys:?}"
        ));
    }
    for (key, py_array) in &python.arrays {
        let (a, b) = (frame_shape(&rust.arrays[key]), frame_shape(py_array));
        if a != b {
            return Err(format!("{key} is {a} per frame against Python's {b}"));
        }
    }
    Ok(clock)
}

/// The event scan: how far apart each pair of events is, up to the first one that is not the same in both lists.
struct EventScan {
    /// The largest float difference of each pair of events before the break.
    gaps: Vec<f64>,
    /// The first event that is not the same, if any.
    brk: Option<EventBreak>,
}

/// The first event that is not the same in both lists.
struct EventBreak {
    /// Its index in the lists.
    i: usize,
    /// Its time: Python's for an event of the same kind at the same time, else the earlier of the two.
    t: f64,
    /// How the two compare there: never `Same`; `Other` also when one list has no event left.
    gap: EventGap,
}

fn scan_events(rust: &Recording, python: &Recording) -> EventScan {
    let mut gaps = Vec::new();
    for i in 0.. {
        let (gap, t) = match (rust.events.get(i), python.events.get(i)) {
            (Some(a), Some(b)) => match event_gap(a, b) {
                EventGap::Same(d) => {
                    gaps.push(d);
                    continue;
                }
                EventGap::Other => (EventGap::Other, a.t.min(b.t)),
                gap => (gap, b.t),
            },
            (Some(e), None) | (None, Some(e)) => (EventGap::Other, e.t),
            (None, None) => break,
        };
        return EventScan {
            gaps,
            brk: Some(EventBreak { i, t, gap }),
        };
    }
    EventScan { gaps, brk: None }
}

/// The largest difference in the first `n` events.
fn in_events(gaps: &[f64], n: usize) -> f64 {
    gaps[..n.min(gaps.len())].iter().copied().fold(0.0, f64::max)
}

/// The frame scan: the frames before the first event break (an event's time is its step's, rounded to 5 places: so
/// are the frame times here), up to the first one with a value outside the bar.
struct FrameScan<'a> {
    /// Frames that agree.
    agreed: usize,
    /// The frames both recordings hold.
    common: usize,
    /// The largest difference in the frames that agree.
    worst: f64,
    /// The largest difference in their positions (`P`).
    worst_p: f64,
    /// The largest difference in the last frame that agrees (0 when none does).
    last: f64,
    /// The first frame with a value outside the bar, if it comes before the event break.
    brk: Option<FrameBreak<'a>>,
}

/// A frame with a value outside the bar.
struct FrameBreak<'a> {
    /// The frame.
    i: usize,
    /// Its time.
    t: f64,
    /// How far apart each key is at that frame.
    gaps: BTreeMap<&'a str, Result<f64, String>>,
}

fn scan_frames<'a>(rust: &Recording, python: &'a Recording, clock: &Clock, t_event: f64) -> FrameScan<'a> {
    let common = rust.frames().min(python.frames());
    let mut scan = FrameScan {
        agreed: 0,
        common,
        worst: 0.0,
        worst_p: 0.0,
        last: 0.0,
        brk: None,
    };
    for i in 0..common {
        let t = clock.frame(python, i);
        if py::round_nd(t, 5) >= t_event {
            break;
        }
        let gaps: BTreeMap<&str, Result<f64, String>> = python
            .arrays
            .iter()
            .map(|(key, py_array)| (key.as_str(), frame_gap(&rust.arrays[key], py_array, i)))
            .collect();
        if gaps.values().any(outside) {
            scan.brk = Some(FrameBreak { i, t, gaps });
            break;
        }
        scan.last = 0.0;
        for (key, gap) in &gaps {
            let d = *gap.as_ref().expect("inside the bar");
            scan.last = scan.last.max(d);
            if *key == "P" {
                scan.worst_p = scan.worst_p.max(d);
            }
        }
        scan.worst = scan.worst.max(scan.last);
        scan.agreed += 1;
    }
    scan
}

/// Whether a frame's gap is outside the bar (a float past it, or anything else that differs).
fn outside(gap: &Result<f64, String>) -> bool {
    !matches!(gap, Ok(d) if *d <= PREFIX_TOL)
}

/// The report of a frame with a value outside the bar: drift when every value outside is a float and the same keys
/// were already more than [`DRIFT_FROM`] apart in the frame before, else a logic break.
fn frame_break(
    rust: &Recording,
    python: &Recording,
    scan: &FrameScan,
    at: &FrameBreak,
    event_gaps: &[f64],
) -> PrefixReport {
    let FrameBreak { i, t, gaps } = at;
    let (i, t) = (*i, *t);
    // every key at this frame, the ones outside the bar first; then the frame before, and the events
    let show = |gap: &Result<f64, String>| match gap {
        Ok(d) => format!("{d:.1e}"),
        Err(what) => what.clone(),
    };
    let out: Vec<String> = gaps
        .iter()
        .filter(|(_, gap)| outside(gap))
        .map(|(key, gap)| format!("{key} ({})", show(gap)))
        .collect();
    let rest: Vec<String> = gaps
        .iter()
        .filter(|(_, gap)| !outside(gap))
        .filter(|(_, gap)| !matches!(gap, Ok(d) if *d == 0.0))
        .map(|(key, gap)| format!("{key} {}", show(gap)))
        .collect();
    // the keys outside the bar, at the frame before
    let before: Vec<(&str, Result<f64, String>)> = match i.checked_sub(1) {
        Some(j) => gaps
            .iter()
            .filter(|(_, gap)| outside(gap))
            .map(|(key, _)| (*key, frame_gap(&rust.arrays[*key], &python.arrays[*key], j)))
            .collect(),
        None => Vec::new(),
    };
    let floats_only = gaps.values().filter(|gap| outside(gap)).all(Result::is_ok);
    let was = before
        .iter()
        .filter_map(|(_, gap)| gap.as_ref().ok())
        .copied()
        .fold(0.0, f64::max);
    let drift = floats_only && !before.is_empty() && was > DRIFT_FROM;
    // the events up to this frame all agree (an event break at or before it would have ended the window first);
    // what the lists do afterwards says whether the two fights still tell the same story
    let n_before = python.events.iter().take_while(|e| e.t <= py::round_nd(t, 5)).count();
    let story = match same_story_until(rust, python) {
        Some((j, t_j)) => format!("stay the same in kind, time and whole numbers until event {j} at {t_j} s"),
        None => "are the same in kind, time and whole numbers to the end".to_string(),
    };
    let head = if drift {
        format!("drift (values past 1e-6, the frame before at {was:.1e})")
    } else {
        "logic break (values)".to_string()
    };
    let words = format!(
        "{head}: at frame {i} ({t:.5} s) outside 1e-6: {}; inside it at that frame: {}; these keys at the frame \
         before: {}; the {n_before} events up to that frame agree, and the event lists {story}",
        out.join(", "),
        if rest.is_empty() {
            "nothing else differs".to_string()
        } else {
            rest.join(", ")
        },
        if before.is_empty() {
            "(none)".to_string()
        } else {
            let shown: Vec<String> = before.iter().map(|(key, gap)| format!("{key} {}", show(gap))).collect();
            shown.join(", ")
        },
    );
    let kind = if drift { BreakKind::Drift } else { BreakKind::Logic };
    broken(
        t,
        kind,
        words,
        scan.agreed,
        n_before,
        scan.worst.max(in_events(event_gaps, n_before)),
    )
}

/// The report of the first event that is not the same in both lists, when every frame before it agrees.
fn event_break(
    rust: &Recording,
    python: &Recording,
    scan: &FrameScan,
    at: EventBreak,
    event_gaps: &[f64],
) -> PrefixReport {
    let EventBreak { i, t, gap } = at;
    let (agreed, worst_p) = (scan.agreed, scan.worst_p);
    let worst = scan.worst.max(in_events(event_gaps, i));
    let show = |e: Option<&Event>| match e {
        Some(e) => format!("{:?} at {} s {:?}", e.kind, e.t, e.fields),
        None => "no more events".to_string(),
    };
    let (a, b) = (rust.events.get(i), python.events.get(i));
    let tail = format!(
        "\n Rust:   {}\n Python: {}\n the largest position difference in the {agreed} frames before is {worst_p:.1e}",
        show(a),
        show(b)
    );
    match gap {
        EventGap::Float(d) if agreed > 0 && scan.last > DRIFT_FROM => {
            let words = format!(
                "drift (a float field of an event past 1e-6, the frame before at {:.1e}): event {i}: a float field \
                 is {d:.1e} apart{tail}",
                scan.last
            );
            return broken(t, BreakKind::Drift, words, agreed, i, worst);
        }
        EventGap::Float(d) => {
            let words = format!(
                "logic break (an event with different fields at the same time): event {i}: a float field is {d:.1e} \
                 apart{tail}"
            );
            return broken(t, BreakKind::Logic, words, agreed, i, worst);
        }
        EventGap::Fields(what) => {
            let words =
                format!("logic break (an event with different fields at the same time): event {i}: {what}{tail}");
            return broken(t, BreakKind::Logic, words, agreed, i, worst);
        }
        EventGap::Other | EventGap::Same(_) => {}
    }
    // One engine made an event the other did not make there. The earlier of the two is the one the other engine
    // lacks; of two at the same time, the one the other engine's list simply skips.
    let ta = a.map_or(f64::INFINITY, |e| e.t);
    let tb = b.map_or(f64::INFINITY, |e| e.t);
    let same = |x: Option<&Event>, y: Option<&Event>| match (x, y) {
        (Some(x), Some(y)) => matches!(event_gap(x, y), EventGap::Same(_)),
        _ => false,
    };
    let (odd, whose, by_positions) = if ta < tb || (ta == tb && same(rust.events.get(i + 1), b)) {
        (
            a,
            "Rust has it and Python has not",
            a.is_some_and(|e| positions_decide(&e.kind)),
        )
    } else if tb < ta || same(a, python.events.get(i + 1)) {
        (
            b,
            "Python has it and Rust has not",
            b.is_some_and(|e| positions_decide(&e.kind)),
        )
    } else {
        let either = [a, b].iter().flatten().any(|e| positions_decide(&e.kind));
        (b, "the two engines made different events there", either)
    };
    let odd = odd.expect("the earlier event exists");
    if !by_positions {
        let words = format!(
            "logic break (an event only one engine made, of a kind no last bit decides): a {:?} at {} s, event {i}: \
             {whose}{tail}",
            odd.kind, odd.t
        );
        return broken(t, BreakKind::Logic, words, agreed, i, worst);
    }
    let mut words = format!(
        "knife-edge (an event break while every per-frame value before it is inside 1e-6): a {:?} at {} s, event {i}: \
         {whose}{tail}",
        odd.kind, odd.t
    );
    if odd.kind == "clash" {
        // the comparison that makes a clash is `dist < R + R`: how close to it the balls are in the frames around
        let f = |x: Option<f64>| x.map_or("?".to_string(), |x| format!("{x:.1e}"));
        let after = (agreed < scan.common).then_some(agreed);
        words.push_str(&format!(
            "\n the balls' distance less R + R: in the frame before, {} (Rust) and {} (Python); in the frame after, {} \
             (Rust) and {} (Python)",
            f(agreed.checked_sub(1).and_then(|j| apart(rust, j))),
            f(agreed.checked_sub(1).and_then(|j| apart(python, j))),
            f(after.and_then(|j| apart(rust, j))),
            f(after.and_then(|j| apart(python, j))),
        ));
    }
    broken(t, BreakKind::KnifeEdge, words, agreed, i, worst)
}

/// The report when every event agrees, and every frame both recordings hold: the window is the whole fight, unless
/// one recording has more frames.
fn no_break(
    rust: &Recording,
    python: &Recording,
    clock: &Clock,
    scan: &FrameScan,
    event_gaps: &[f64],
    t_start: f64,
) -> PrefixReport {
    let n_events = event_gaps.len();
    let worst = scan.worst.max(in_events(event_gaps, n_events));
    let frames = scan.common;
    if rust.frames() != python.frames() {
        let t = clock.frame(if rust.frames() > frames { rust } else { python }, frames);
        let words = format!(
            "logic break (values): {} frames against Python's {}, with the same events",
            rust.frames(),
            python.frames()
        );
        return broken(t, BreakKind::Logic, words, scan.agreed, n_events, worst);
    }
    let end = match (python.t_ko, frames.checked_sub(1)) {
        (Some(t_ko), _) => t_ko,
        (None, Some(last)) => clock.frame(python, last),
        (None, None) => t_start,
    };
    PrefixReport {
        agree_until_s: end,
        first_break: None,
        kind: None,
        frames: scan.agreed,
        events: n_events,
        worst,
    }
}

/// The margin of the comparison that makes a `clash` (`dist < R + R`, `fight.py` 3179 to 3188) in the step of this
/// engine's fight whose events carry the time `t`: the distance between the two balls after both have flown (and
/// bounced), less `R + R`. Below 0 the engine clashes in that step, by that much; from 0 up it does not.
///
/// It replays the fight to that step, then flies each ball once more with the other out of its way: a ball's flight
/// depends on nothing but itself, except for the fight's generator, which a rope bounce draws from. So the margin is
/// `None` when that step has a `rope` event (the replay of one ball would draw out of turn there and fly it
/// elsewhere); also when the fight has no such step, or the engine cannot run the setup.
///
/// A diagnostic for the parity suites only (it is how a knife-edge `clash` is shown to be one); a fighter of a kind
/// not ported is let in, as the test entry [`Fight::with_unported`] does.
#[doc(hidden)]
pub fn clash_margin(setup: &Setup, t: f64) -> Option<f64> {
    let mut fight = Fight::with_unported(setup, false).ok()?;
    let mut sched = Schedule::new(&mut fight);
    fight.t = fight.rules.t_fight;
    while fight.t < fight.rules.t_max {
        let before = (fight.clone(), sched.clone());
        sched.step(&mut fight);
        let now = py::round_nd(fight.t, 5);
        if now > t {
            return None;
        }
        if now == t {
            if fight.events[before.0.events.len()..].iter().any(|e| e.kind == "rope") {
                return None;
            }
            let flown = |k: usize| {
                let (mut alone, mut sched) = before.clone();
                // far outside the ring: nothing of ball `k` can touch it, and it cannot clash
                alone.balls[1 - k].p = [1e6, 1e6];
                sched.step(&mut alone);
                alone.balls[k].p
            };
            let (p, q) = (flown(0), flown(1));
            return Some(dmath::hypot(q[0] - p[0], q[1] - p[1]) - 2.0 * fight.rules.r);
        }
        if fight.balls.iter().any(|b| b.hp <= 0.0) {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests;
