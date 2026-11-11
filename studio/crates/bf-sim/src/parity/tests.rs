//! Unit tests of the agreement window: a Python recording against itself, then against copies changed by hand in
//! one place, so each kind of break is told apart from the others and reported where it is.

use super::*;
use crate::testing;

/// A fight with a K.O. (340 frames, 117 events) and a copy of it to change.
fn pair() -> (Recording, Recording) {
    let py = testing::python_recording("scenarios/early_ko_s1.bfrec");
    (py.clone(), py)
}

fn floats<'a>(rec: &'a mut Recording, key: &str) -> &'a mut Vec<f64> {
    match rec.arrays.get_mut(key) {
        Some(Array::F64 { data, .. }) => data,
        other => panic!("{key} is {other:?}"),
    }
}

/// The time of regular frame `i` of the reference fights.
fn frame_time(rec: &Recording, i: usize) -> f64 {
    let dt = rule(rec, "DT").expect("DT");
    rule(rec, "t_fight").expect("t_fight") + i as f64 * 4.0 * dt
}

fn words(report: &PrefixReport) -> &str {
    report.first_break.as_deref().expect("a break")
}

#[test]
fn a_recording_agrees_with_itself_to_its_end() {
    let (rs, py) = pair();
    let report = prefix_window(&rs, &py);
    assert_eq!(report.first_break, None);
    assert_eq!(report.kind, None);
    // to the K.O.
    assert_eq!(report.agree_until_s, 14.879901960784313);
    assert_eq!((report.frames, report.events, report.worst), (340, 117, 0.0));

    // a fight with no K.O. agrees to its last frame
    let py = testing::python_recording("no_ko.bfrec");
    let report = prefix_window(&py.clone(), &py);
    assert_eq!((report.first_break.as_ref(), report.kind), (None, None));
    assert_eq!(report.agree_until_s, frame_time(&py, 1632));
    assert_eq!((report.frames, report.events), (1633, 456));
}

#[test]
fn differences_inside_the_bar_are_agreement() {
    let (mut rs, py) = pair();
    floats(&mut rs, "P")[4 * 100] += 5e-7;
    floats(&mut rs, "V")[4 * 200 + 3] -= 2.5e-7;
    let i = rs.events.iter().position(|e| e.kind == "rope").expect("a rope bounce");
    let Value::Float(x) = &mut rs.events[i].fields[3] else {
        panic!("a rope event's x is a float")
    };
    *x += 9e-7;
    let report = prefix_window(&rs, &py);
    assert_eq!((report.first_break.as_ref(), report.kind), (None, None));
    assert_eq!((report.frames, report.events), (340, 117));
    assert!((report.worst - 9e-7).abs() < 1e-12, "{}", report.worst);
}

/// A per-frame value that jumps outside the bar (the frame before inside `DRIFT_FROM`) is a logic break at that
/// frame's time; the words name the key, what the other keys do there and at the frame before, and how far the event
/// lists stay the same.
#[test]
fn a_value_outside_the_bar_is_a_logic_break() {
    let (mut rs, py) = pair();
    floats(&mut rs, "P")[4 * 100 + 1] += 2e-6;
    floats(&mut rs, "V")[4 * 100] += 3e-7;
    floats(&mut rs, "P")[4 * 99 + 1] += 6e-8;
    // a difference inside the bar in an event AFTER that frame: outside the window, so it does not count
    let later = py.events.iter().rposition(|e| e.kind == "rope").expect("a rope bounce");
    assert!(py.events[later].t > 10.0);
    let Value::Float(x) = &mut rs.events[later].fields[3] else {
        panic!("a rope event's x is a float")
    };
    *x += 9e-7;
    let report = prefix_window(&rs, &py);
    assert_eq!(report.kind, Some(BreakKind::Logic));
    assert_eq!(report.agree_until_s, frame_time(&py, 100));
    assert_eq!(report.frames, 100);
    // the events before that frame's step
    let n = py.events.iter().filter(|e| e.t <= 6.92157).count();
    assert_eq!(report.events, n);
    let w = words(&report);
    assert!(
        w.starts_with("logic break (values): at frame 100 (6.92157 s) outside 1e-6: P (2.0e-6);"),
        "{w}"
    );
    assert!(w.contains("inside it at that frame: V 3.0e-7;"), "{w}");
    assert!(w.contains("these keys at the frame before: P 6.0e-8;"), "{w}");
    assert!(
        w.ends_with("and the event lists are the same in kind, time and whole numbers to the end"),
        "{w}"
    );
    assert!(
        (report.worst - 6e-8).abs() < 1e-12,
        "the largest difference inside the window: {}",
        report.worst
    );

    // a whole number that differs, and a row with another layout, are outside the bar too
    let (mut rs, py) = pair();
    let Some(Array::I64 { data, .. }) = rs.arrays.get_mut("LV") else {
        panic!("LV is whole numbers")
    };
    data[2 * 7] = 2;
    let report = prefix_window(&rs, &py);
    assert_eq!((report.kind, report.frames), (Some(BreakKind::Logic), 7));
    assert!(
        words(&report).contains("outside 1e-6: LV ([2, 1] against Python's [1, 1])"),
        "{}",
        words(&report)
    );

    let (mut rs, py) = pair();
    let Some(Array::Nested(Value::List(rows))) = rs.arrays.get_mut("WAVES") else {
        panic!("WAVES is rows")
    };
    rows[3] = Value::List(vec![Value::List(vec![Value::Int(0)])]);
    let report = prefix_window(&rs, &py);
    assert_eq!((report.kind, report.frames), (Some(BreakKind::Logic), 3));
    assert!(
        words(&report).contains("WAVES (1 items against Python's 0"),
        "{}",
        words(&report)
    );
}

/// A float that passes the bar while the same key was already more than `DRIFT_FROM` apart in the frame before is
/// drift, in a frame and in an event's float field; a whole number or a layout that differs never is.
#[test]
fn a_value_that_drifts_past_the_bar_is_drift() {
    let (mut rs, py) = pair();
    floats(&mut rs, "P")[4 * 100 + 1] += 2e-6;
    floats(&mut rs, "P")[4 * 99 + 1] += 4e-7;
    let report = prefix_window(&rs, &py);
    assert_eq!((report.kind, report.frames), (Some(BreakKind::Drift), 100));
    assert_eq!(report.agree_until_s, frame_time(&py, 100));
    let w = words(&report);
    assert!(
        w.starts_with(
            "drift (values past 1e-6, the frame before at 4.0e-7): at frame 100 (6.92157 s) outside 1e-6: P (2.0e-6);"
        ),
        "{w}"
    );
    assert!(w.contains("these keys at the frame before: P 4.0e-7;"), "{w}");
    // only the keys outside the bar count: another key that was apart in the frame before does not make it drift
    let (mut rs, py) = pair();
    floats(&mut rs, "P")[4 * 100 + 1] += 2e-6;
    floats(&mut rs, "V")[4 * 99] += 4e-7;
    assert_eq!(prefix_window(&rs, &py).kind, Some(BreakKind::Logic));
    // a whole number that differs is a logic break, however far apart the floats were in the frame before
    let (mut rs, py) = pair();
    floats(&mut rs, "P")[4 * 6 + 1] += 4e-7;
    let Some(Array::I64 { data, .. }) = rs.arrays.get_mut("LV") else {
        panic!("LV is whole numbers")
    };
    data[2 * 7] = 2;
    assert_eq!(prefix_window(&rs, &py).kind, Some(BreakKind::Logic));

    // an event's float field past the bar: drift when the frame before is more than DRIFT_FROM apart
    let (mut rs, py) = pair();
    let i = py.events.iter().position(|e| e.kind == "hit").expect("a hit");
    let before = (0..340)
        .filter(|&f| py::round_nd(frame_time(&py, f), 5) < py.events[i].t)
        .count();
    let Value::Float(x) = &mut rs.events[i].fields[3] else {
        panic!("a hit's x is a float")
    };
    *x += 2e-6;
    floats(&mut rs, "V")[4 * (before - 1) + 2] += 3e-7;
    let report = prefix_window(&rs, &py);
    assert_eq!(
        (report.kind, report.events, report.frames),
        (Some(BreakKind::Drift), i, before)
    );
    assert!(
        words(&report).starts_with(&format!(
            "drift (a float field of an event past 1e-6, the frame before at 3.0e-7): event {i}: a float field is \
             2.0e-6 apart"
        )),
        "{}",
        words(&report)
    );
}

/// An event one engine made and the other did not, while the frames before it agree, is a knife-edge: the words name
/// the event, who has it, and for a clash how close to R + R the balls are around it.
#[test]
fn an_event_only_one_engine_made_is_a_knife_edge() {
    let (mut rs, py) = pair();
    let i = py.events.iter().position(|e| e.kind == "clash").expect("a clash");
    let clash = rs.events.remove(i);
    let report = prefix_window(&rs, &py);
    assert_eq!(report.kind, Some(BreakKind::KnifeEdge));
    assert_eq!((report.agree_until_s, report.events), (clash.t, i));
    let frames = (0..340)
        .filter(|&f| py::round_nd(frame_time(&py, f), 5) < clash.t)
        .count();
    assert_eq!(report.frames, frames);
    let w = words(&report);
    assert!(
        w.starts_with(&format!(
            "knife-edge (an event break while every per-frame value before it is inside 1e-6): a \"clash\" at {} s, \
             event {i}: Python has it and Rust has not",
            clash.t
        )),
        "{w}"
    );
    assert!(w.contains("the largest position difference in the"), "{w}");
    assert!(
        w.contains("the balls' distance less R + R: in the frame before, "),
        "{w}"
    );

    // the other way round: an event Rust made and Python did not
    let (mut rs, py) = pair();
    let mut extra = py.events[i].clone();
    extra.kind = "rope".to_string();
    extra.t = py.events[i - 1].t;
    rs.events.insert(i, extra);
    let report = prefix_window(&rs, &py);
    assert_eq!(report.kind, Some(BreakKind::KnifeEdge));
    assert_eq!((report.agree_until_s, report.events), (py.events[i - 1].t, i));
    let w = words(&report);
    assert!(
        w.contains("a \"rope\" at") && w.contains("Rust has it and Python has not"),
        "{w}"
    );
    assert!(!w.contains("R + R"), "only a clash has that margin: {w}");

    // a list that stops short: the fight's last hit and what follows are missing
    let (mut rs, py) = pair();
    let last_hit = py.events.iter().rposition(|e| e.kind == "hit").expect("a hit");
    rs.events.truncate(last_hit);
    let report = prefix_window(&rs, &py);
    assert_eq!(report.kind, Some(BreakKind::KnifeEdge));
    assert_eq!((report.agree_until_s, report.events), (py.events[last_hit].t, last_hit));
    let w = words(&report);
    assert!(
        w.contains("a \"hit\" at") && w.contains("Python has it and Rust has not"),
        "{w}"
    );
    assert!(w.contains("Rust:   no more events"), "{w}");
}

/// An event only one engine made is no knife-edge when its kind follows the clock and whole hit points: no last bit
/// makes or unmakes an ability, a level, a basket, a thrust, a slam or a K.O.
#[test]
fn a_missing_event_that_no_last_bit_decides_is_a_logic_break() {
    for kind in ["ability", "level", "basket", "thrust", "ko"] {
        let (mut rs, py) = pair();
        let i = py
            .events
            .iter()
            .position(|e| e.kind == kind)
            .unwrap_or_else(|| panic!("a {kind}"));
        let gone = rs.events.remove(i);
        let report = prefix_window(&rs, &py);
        assert_eq!(report.kind, Some(BreakKind::Logic), "{kind}");
        assert_eq!((report.agree_until_s, report.events), (gone.t, i), "{kind}");
        let w = words(&report);
        assert!(
            w.starts_with(&format!(
                "logic break (an event only one engine made, of a kind no last bit decides): a {kind:?} at {} s, \
                 event {i}: Python has it and Rust has not",
                gone.t
            )),
            "{w}"
        );
    }
    // the kinds a comparison of positions decides
    for kind in ["clash", "rope", "hit", "strike", "swat"] {
        let (mut rs, py) = pair();
        let i = py
            .events
            .iter()
            .position(|e| e.kind == kind)
            .unwrap_or_else(|| panic!("a {kind}"));
        rs.events.remove(i);
        assert_eq!(prefix_window(&rs, &py).kind, Some(BreakKind::KnifeEdge), "{kind}");
    }
    for kind in ["slow", "block", "parry", "cleared"] {
        assert!(positions_decide(kind), "{kind}");
    }
    assert!(!positions_decide("slam"));

    // two different events at the same time, neither simply skipped by the other engine: a knife-edge when a
    // comparison of positions decides either of them, a logic break when the clock decides both
    let (mut rs, py) = pair();
    let i = py.events.iter().position(|e| e.kind == "rope").expect("a rope bounce");
    rs.events[i].kind = "clash".to_string();
    let report = prefix_window(&rs, &py);
    assert_eq!((report.kind, report.events), (Some(BreakKind::KnifeEdge), i));
    assert!(
        words(&report).contains("the two engines made different events there"),
        "{}",
        words(&report)
    );
    let (mut rs, py) = pair();
    let i = py.events.iter().position(|e| e.kind == "ability").expect("an ability");
    rs.events[i].kind = "level".to_string();
    let report = prefix_window(&rs, &py);
    assert_eq!((report.kind, report.events), (Some(BreakKind::Logic), i));
    assert!(
        words(&report).contains("the two engines made different events there"),
        "{}",
        words(&report)
    );
}

/// The same kind of event at the same time with other fields is a logic break, whatever the frames say.
#[test]
fn an_event_with_different_fields_is_a_logic_break() {
    let (mut rs, py) = pair();
    let i = py.events.iter().position(|e| e.kind == "hit").expect("a hit");
    // the sub-number of the hit
    rs.events[i].fields[6] = Value::Int(7);
    let report = prefix_window(&rs, &py);
    assert_eq!(report.kind, Some(BreakKind::Logic));
    assert_eq!((report.agree_until_s, report.events), (py.events[i].t, i));
    let w = words(&report);
    assert!(
        w.starts_with(&format!(
            "logic break (an event with different fields at the same time): event {i}: Int(7) against Python's Int(1)"
        )),
        "{w}"
    );

    // a float field outside the bar
    let (mut rs, py) = pair();
    let Value::Float(x) = &mut rs.events[i].fields[3] else {
        panic!("a hit's x is a float")
    };
    *x += 2e-6;
    let report = prefix_window(&rs, &py);
    assert_eq!((report.kind, report.events), (Some(BreakKind::Logic), i));
    assert!(
        words(&report).contains("a float field is 2.0e-6 apart"),
        "{}",
        words(&report)
    );

    // one field more
    let (mut rs, py) = pair();
    rs.events[i].fields.push(Value::Int(0));
    let report = prefix_window(&rs, &py);
    assert_eq!((report.kind, report.events), (Some(BreakKind::Logic), i));
    assert!(
        words(&report).contains("8 items against Python's 7"),
        "{}",
        words(&report)
    );
}

/// The first break in time order ends the window, and an event comes before the frame of its own step.
#[test]
fn the_first_break_in_time_order_counts() {
    let (rs0, py) = pair();
    let i = py.events.iter().position(|e| e.kind == "clash").expect("a clash");
    let t = py.events[i].t;
    let before = (0..340).filter(|&f| py::round_nd(frame_time(&py, f), 5) < t).count();
    assert!(before > 10);

    // a frame outside the bar well before the missing event: the frame ends the window
    let mut rs = rs0.clone();
    rs.events.remove(i);
    floats(&mut rs, "P")[4 * (before - 5)] += 1e-3;
    let report = prefix_window(&rs, &py);
    assert_eq!((report.kind, report.frames), (Some(BreakKind::Logic), before - 5));
    assert!(report.agree_until_s < t);

    // a frame outside the bar at or after the missing event: the event ends it
    let mut rs = rs0.clone();
    rs.events.remove(i);
    floats(&mut rs, "P")[4 * before] += 1e-3;
    let report = prefix_window(&rs, &py);
    assert_eq!(
        (report.kind, report.frames, report.events),
        (Some(BreakKind::KnifeEdge), before, i)
    );
    assert_eq!(report.agree_until_s, t);
}

#[test]
fn recordings_of_different_fights_do_not_compare() {
    let (mut rs, py) = pair();
    rs.seed = 2;
    let report = prefix_window(&rs, &py);
    assert_eq!(report.kind, Some(BreakKind::Logic));
    assert_eq!(
        (report.agree_until_s, report.frames, report.events),
        (3.5882352941176467, 0, 0)
    );
    assert!(words(&report).starts_with("not the same fight"), "{}", words(&report));

    let (mut rs, py) = pair();
    rs.names.reverse();
    assert!(words(&prefix_window(&rs, &py)).starts_with("not the same fight"));

    // a key missing, or of another shape
    let (mut rs, py) = pair();
    rs.arrays.remove("COB");
    assert!(words(&prefix_window(&rs, &py)).starts_with("the per-frame keys differ"));
    let (mut rs, py) = pair();
    rs.arrays.insert(
        "TH".to_string(),
        Array::F64 {
            shape: vec![340, 3],
            data: vec![0.0; 340 * 3],
        },
    );
    let report = prefix_window(&rs, &py);
    assert_eq!(words(&report), "TH is floats [3] per frame against Python's floats [2]");

    // one frame short, with the same events
    let (mut rs, py) = pair();
    for a in rs.arrays.values_mut() {
        match a {
            Array::F64 { shape, data } => {
                shape[0] -= 1;
                data.truncate(data.len() - shape[1..].iter().product::<usize>());
            }
            Array::I64 { shape, data } => {
                shape[0] -= 1;
                data.truncate(data.len() - shape[1..].iter().product::<usize>());
            }
            Array::Nested(Value::List(rows)) => _ = rows.pop(),
            Array::Nested(_) => panic!("rows are a list"),
        }
    }
    let report = prefix_window(&rs, &py);
    assert_eq!(
        (report.kind, report.frames, report.events),
        (Some(BreakKind::Logic), 339, 117)
    );
    assert!(
        words(&report).contains("339 frames against Python's 340"),
        "{}",
        words(&report)
    );
}

/// `clash_margin`: below 0 in every step this engine clashes in, from 0 up in the steps it does not; nothing for a
/// time no step has.
#[test]
fn clash_margin_is_the_distance_less_two_radii() {
    let bare = testing::bare_pair();
    let mut setup = testing::bare_setup(&bare, 3);
    setup.rules.t_max = setup.rules.t_fight + 20.0;
    let rec = crate::run_with_unported(&setup, true).expect("the bare pair runs");
    let clashes: Vec<f64> = rec.events.iter().filter(|e| e.kind == "clash").map(|e| e.t).collect();
    assert!(clashes.len() >= 2, "seed 3 clashes in its first 20 s: {clashes:?}");
    for &t in &clashes {
        let m = clash_margin(&setup, t).expect("a step of the fight");
        assert!(
            (-0.06..0.0).contains(&m),
            "the balls overlap by less than one step's flight: {m}"
        );
    }
    // the step before a clash: the balls are still apart, by less than two steps' flight
    let dt = setup.rules.dt;
    let before = py::round_nd(
        setup.rules.t_fight + ((clashes[0] - setup.rules.t_fight) / dt).round() * dt - dt,
        5,
    );
    let m = clash_margin(&setup, before).expect("a step of the fight");
    assert!((0.0..0.12).contains(&m), "{m}");
    // the first step: the balls are in their corners, 3.7 * sqrt(2) apart less what they flew
    let m = clash_margin(&setup, py::round_nd(setup.rules.t_fight + dt, 5)).expect("the first step");
    assert!((m - (3.7 * 2f64.sqrt() - 1.2)).abs() < 0.05, "{m}");
    assert_eq!(clash_margin(&setup, 1.0), None, "before the fight");
    assert_eq!(clash_margin(&setup, 99.0), None, "after it");
    assert_eq!(clash_margin(&setup, clashes[0] + 1e-5), None, "between two steps");
    // a step with a rope bounce: the replay of one ball would draw from the generator out of turn
    let ropes: Vec<f64> = rec.events.iter().filter(|e| e.kind == "rope").map(|e| e.t).collect();
    assert!(!ropes.is_empty());
    for &t in &ropes {
        assert_eq!(clash_margin(&setup, t), None, "a rope bounce at {t}");
    }
    // rules the engine cannot run
    let mut bad = setup.clone();
    bad.rules.dt = 0.0;
    assert_eq!(clash_margin(&bad, clashes[0]), None);
}
