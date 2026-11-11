//! Parity with the Python kit over whole fights: the agreement window of every reference recording.
//!
//! For each committed Python recording of FLUTE against TROMBONE (the five reference fights, the fight with no K.O.
//! and the fifteen scenario fights) the Rust fight of the same fighters, rules and seed is run to its end, and
//! [`prefix_window`] measures how long the two agree: EVERY event in order (kind, time, whole numbers and texts
//! exactly, floats to 1e-6) and EVERY per-frame key (whole numbers and the layout of rows exactly, floats to 1e-6).
//! The window must last [`PREFIX_FLOOR_S`] seconds of fighting, or the whole fight when that is shorter.
//!
//! The two engines are not equal bit for bit (the maths differ in the last bit now and then, and a fight is
//! chaotic), so every window ends somewhere unless the fight ends first. What ends it is printed for every fight:
//! run with `--nocapture`. A window that ends before the floor fails the test with the numbers, whatever ended it; a
//! knife-edge on a `clash` is reported with the margin of the comparison that decided it.
//!
//! `prefix_before_abilities.rs` is the tighter test of the span before the first ability (exact event totals); this
//! one adds everything after it: the six abilities, their waves, slams, slows and hastes.
//!
//! No window of the 21 fights is ended by a knife-edge or a logic break: every one that ends, ends on drift (a float
//! passing 1e-6 where it was already past 1e-7 a frame before), and the test holds that. The last test shows what a
//! knife-edge looks like on a real fight: a
//! core-only Python recording (`studio/reference/fights/knife_edge/`, the `knife_edge` export) in which the balls
//! travel side by side exactly `R + R` apart.

mod support;

use std::collections::BTreeMap;

use bf_core::recording::Recording;
use bf_sim::parity::{BreakKind, PREFIX_FLOOR_S, PrefixReport, clash_margin, prefix_window};
use bf_sim::{run, run_with_unported};
use support::{count_kinds, rule};

/// Every committed Python recording of FLUTE against TROMBONE, with the scenario it belongs to: the five reference
/// fights and the fight with no K.O. under the episode's own rules, then the scenario fights (rule overrides, and
/// the TROMBONE in the left corner).
const FIGHTS: [(&str, &str); 21] = [
    ("ep392.bfrec", "episode"),
    ("seed1.bfrec", "episode"),
    ("seed2.bfrec", "episode"),
    ("seed3.bfrec", "episode"),
    ("seed500.bfrec", "episode"),
    ("no_ko.bfrec", "episode, no K.O."),
    ("scenarios/early_ko_s1.bfrec", "early_ko"),
    ("scenarios/early_ko_s2.bfrec", "early_ko"),
    ("scenarios/early_ko_s3.bfrec", "early_ko"),
    ("scenarios/early_levels_s1.bfrec", "early_levels"),
    ("scenarios/early_levels_s2.bfrec", "early_levels"),
    ("scenarios/early_levels_s3.bfrec", "early_levels"),
    ("scenarios/early_low_s1.bfrec", "early_low"),
    ("scenarios/early_low_s2.bfrec", "early_low"),
    ("scenarios/early_low_s3.bfrec", "early_low"),
    ("scenarios/plain_s7.bfrec", "plain"),
    ("scenarios/plain_s8.bfrec", "plain"),
    ("scenarios/plain_s9.bfrec", "plain"),
    ("scenarios/swapped_s1.bfrec", "swapped"),
    ("scenarios/swapped_s2.bfrec", "swapped"),
    ("scenarios/swapped_s3.bfrec", "swapped"),
];

/// One fight's window.
struct Window {
    name: &'static str,
    report: PrefixReport,
    /// Seconds of fighting the window lasts.
    lasts: f64,
    /// Seconds of fighting the Python fight lasts.
    fight: f64,
    /// The events inside the window, by kind (`hit` with what hit), and the abilities among them by name; a `+`
    /// after the name marks a use that is not the first.
    kinds: BTreeMap<String, usize>,
    /// What to say when the window is too short: the report's words and, for a knife-edge on a clash, the margin.
    trouble: String,
}

fn measure(name: &'static str, scenario: &str, roster: &[bf_sim::roster::FighterCfg]) -> Window {
    let py: Recording = support::python_recording(&format!("flute_vs_trombone/{name}"));
    let setup = support::setup_like(&py, roster);
    let rs = run(&setup, true).expect("the engine runs this fight");
    let report = prefix_window(&rs, &py);
    let t_fight = rule(&py, "t_fight");
    let lasts = report.agree_until_s - t_fight;
    let fight = py.t_ko.unwrap_or(rule(&py, "t_max")) - t_fight;

    let inside = &py.events[..report.events];
    let mut kinds = count_kinds(inside);
    for e in inside.iter().filter(|e| e.kind == "ability") {
        let again = if e.i(5) == 1 { "" } else { "+" };
        *kinds.entry(format!("ability/{}{again}", e.s(3))).or_insert(0) += 1;
    }

    let mut trouble = report.first_break.clone().unwrap_or_default();
    if report.kind == Some(BreakKind::KnifeEdge) && trouble.contains("a \"clash\" at") {
        // the comparison that decides a clash is `dist < R + R`: its margin in the Rust fight, at that step
        match clash_margin(&setup, report.agree_until_s) {
            Some(m) => trouble.push_str(&format!(
                "\n in the Rust fight the balls are {m:e} from R + R after that step's flight (below 0: a clash)"
            )),
            None => trouble.push_str("\n the Rust fight has no step at that time"),
        }
    }
    let ended = match (&report.first_break, report.kind) {
        (None, _) => "nothing: the two agree to the end of the fight".to_string(),
        (Some(_), _) => trouble.replace('\n', " |"),
    };
    println!(
        "{name} ({scenario}): agrees until {:.5} s, {lasts:.2} s of the fight's {fight:.2} s; {} frames, {} events, \
         max |d| {:.1e}; ended by {ended}",
        report.agree_until_s, report.frames, report.events, report.worst
    );
    Window {
        name,
        report,
        lasts,
        fight,
        kinds,
        trouble,
    }
}

fn windows() -> Vec<Window> {
    let roster = support::roster();
    FIGHTS
        .iter()
        .map(|(name, scenario)| measure(name, scenario, &roster))
        .collect()
}

/// Brief, step 1: for the five reference fights and every scenario, the Rust fight agrees with the Python recording
/// for at least `PREFIX_FLOOR_S` seconds of fighting, or for the whole fight if that is shorter. One line per fight
/// is printed with its window.
#[test]
fn agrees_with_python() {
    let windows = windows();
    let short: Vec<String> = windows
        .iter()
        .filter(|w| w.report.first_break.is_some() && w.lasts < PREFIX_FLOOR_S)
        .map(|w| {
            format!(
                "{}: the window ends after {:.3} s of fighting, at {} s ({} frames and {} events agree):\n {}",
                w.name, w.lasts, w.report.agree_until_s, w.report.frames, w.report.events, w.trouble
            )
        })
        .collect();
    assert!(
        short.is_empty(),
        "{} of {} fights break before the floor of {PREFIX_FLOOR_S} s:\n\n{}",
        short.len(),
        windows.len(),
        short.join("\n\n")
    );
    // a window never claims more than the fight
    for w in &windows {
        assert!(w.lasts <= w.fight + 0.01, "{}: {} s of {} s", w.name, w.lasts, w.fight);
        assert!(w.report.worst <= 1e-6, "{}", w.name);
    }
    // every window of these fights that ends, ends on drift: the two fights part slowly, never by a jump, a whole
    // number or an event that one engine made (a logic break there is a bug until shown otherwise)
    let not_drift: Vec<String> = windows
        .iter()
        .filter(|w| w.report.kind.is_some_and(|k| k != BreakKind::Drift))
        .map(|w| format!("{}: {}", w.name, w.trouble))
        .collect();
    assert!(
        not_drift.is_empty(),
        "windows not ended by drift:\n{}",
        not_drift.join("\n")
    );
}

/// The windows are not empty of what this suite is for: inside them (so matched event for event with Python) every
/// one of the six abilities fires, FLUTTER TONGUE and SAD TROMBONE fire a second time (the `every_bars` timer), and
/// the waves, the shockwave with its slow, and the slam land.
#[test]
fn the_windows_hold_every_ability() {
    let windows = windows();
    let mut all: BTreeMap<String, usize> = BTreeMap::new();
    for w in &windows {
        for (kind, n) in &w.kinds {
            *all.entry(kind.clone()).or_insert(0) += n;
        }
    }
    println!("inside the 21 windows: {all:?}");
    for item in [
        "ability/FLUTTER TONGUE",
        "ability/SNAKE DANCE",
        "ability/PICCOLO",
        "ability/SAD TROMBONE",
        "ability/POWER SLIDE",
        "ability/PEDAL TONE",
        "ability/FLUTTER TONGUE+",
        "ability/SAD TROMBONE+",
        "hit/wave",
        "hit/shockwave",
        "hit/slam",
        "slow",
        "slam",
        "hit/fang",
        "hit/blade",
        "swat",
        "level",
        "ko",
    ] {
        assert!(
            all.get(item).copied().unwrap_or(0) >= 1,
            "no {item} inside any window: {all:?}"
        );
    }
}

/// A real knife-edge is called one, with the event and the margin of the comparison that decided it. In the
/// core-only fight of `studio/reference/fights/knife_edge/` the balls clash and then fly on side by side, `R + R`
/// apart to rounding, so on every following step `dist < R + R` is decided by the last bit of the distance: Python
/// makes a `clash` on 49 steps in a row, the Rust engine on some of them, while every position of the two agrees far
/// inside the bar. That is not a mistake of the port, and it must not look like one (nor a mistake like it).
///
/// What is pinned is the kind of break and the size of its numbers, not the step it falls on: that step depends on
/// the last bits of this engine's arithmetic. If a change to the arithmetic makes this fight agree throughout, the
/// sample no longer shows a knife-edge: pick another seed in the export (`tools/studio_export/knife_edge.py`).
#[test]
fn a_knife_edge_clash_is_told_from_a_logic_break() {
    let path = support::reference("knife_edge/index.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let bags = bf_sim::roster::parse_roster(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let doc: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let row = &doc["fights"][0];
    let side_by_side = |key: &str| row["side_by_side"][key].as_f64().expect("a time");
    let py = support::python_recording(row["file"].as_str().expect("a file").trim_start_matches("fights/"));
    assert_eq!(Some(py.hash().as_str()), row["hash"].as_str());
    assert!(
        py.events.iter().all(|e| e.kind == "rope" || e.kind == "clash"),
        "core only"
    );

    let setup = support::setup_like(&py, &bags);
    let rs = run_with_unported(&setup, true).expect("the engine runs the bags (kinds not ported)");
    let report = prefix_window(&rs, &py);
    let words = report
        .first_break
        .clone()
        .expect("the two engines differ in this fight");
    println!("knife_edge/core_s4: {words}");
    assert_eq!(report.kind, Some(BreakKind::KnifeEdge), "{words}");
    assert!(words.starts_with("knife-edge ("), "{words}");
    assert!(words.contains("a \"clash\" at"), "{words}");
    // inside the run of clashes on consecutive steps: the balls are travelling side by side
    let t = report.agree_until_s;
    assert!(side_by_side("first") < t && t <= side_by_side("last"), "{t}: {words}");
    // every per-frame value and event before it agrees far inside the bar (1e-6)
    assert!(report.worst < 1e-9, "{:e}", report.worst);
    assert!(
        report.frames >= 300 && report.events >= 40,
        "{} frames, {} events",
        report.frames,
        report.events
    );
    // the margin of the comparison that decided it, `dist < R + R` in the Rust fight at that step: a last bit (the
    // spacing of doubles at R + R = 1.2 is 2.2e-16), on the side that fits who made the clash
    let margin = clash_margin(&setup, t).expect("a step of the Rust fight");
    println!("knife_edge/core_s4: in the Rust fight the balls are {margin:e} from R + R after the flight of that step");
    assert!(margin.abs() <= 4.0 * f64::EPSILON, "{margin:e}");
    if words.contains("Python has it and Rust has not") {
        assert!(margin >= 0.0, "Rust made no clash there: {margin:e}");
    } else {
        assert!(words.contains("Rust has it and Python has not"), "{words}");
        assert!(margin < 0.0, "Rust made a clash there: {margin:e}");
    }
    // and the words carry the distance in the frames around it, as far as the recordings show it
    assert!(
        words.contains("the balls' distance less R + R: in the frame before, "),
        "{words}"
    );
}
