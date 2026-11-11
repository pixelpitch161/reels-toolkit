//! A recording's `rules` from a fight file equal Python's `vars(rules)` for the two attributes only an episode adds.
//!
//! The Python recordings are the `rules_map` export (`studio/reference/fights/rules_map/`, made by
//! `tools/studio_export/rules_map.py`): the example episode with no `DMG_SIDE`, and with `RULES =
//! dict(swat_cobras=False)`, each fought for 3 s. The Rust fight of each comes from the example fight file with the
//! same change made to its TEXT (the `handicap` line taken out; the pair rule written `swat_cobras = false`), through
//! `setup_from`. Its recorded `rules` must be the Python map exactly (the same keys, types and bits: the content
//! hash covers them), and the two fights must agree over the whole window.

mod support;

use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_sim::parity::prefix_window;
use bf_sim::{Rules, run, setup_from};
use support::rule;

const EXAMPLE: &str = include_str!("../../../fights/flute_vs_trombone.toml");

/// The example fight file with `old` replaced by `new` in its text.
fn example_with(old: &str, new: &str) -> FightFile {
    assert_eq!(EXAMPLE.matches(old).count(), 1, "{old:?} is in the example once");
    FightFile::parse(&EXAMPLE.replace(old, new)).expect("the changed example parses")
}

/// The Rust fight of a case: the fight file's setup at the recording's seed, cut at the recording's `t_max` (the
/// export's window; `t_max` is an attribute of every Rules, so both maps hold it).
fn compare(case: &str, ff: &FightFile) -> (Recording, Recording) {
    let py = support::python_recording(&format!("rules_map/{case}.bfrec"));
    let roster = support::roster();
    let mut setup = setup_from(ff, &roster, py.seed).expect("the changed example sets up");
    setup.rules.t_max = rule(&py, "t_max");
    let rs = run(&setup, true).expect("the engine runs it");
    assert_eq!(rs.rules, py.rules, "{case}: the recorded rules");
    let report = prefix_window(&rs, &py);
    assert_eq!(report.first_break, None, "{case}: the two fights agree over the window");
    assert_eq!((report.frames, report.events), (py.frames(), py.events.len()), "{case}");
    println!(
        "{case}: worst {:e} rust hash {} python hash {}",
        report.worst,
        rs.hash(),
        py.hash()
    );
    (rs, py)
}

#[test]
fn a_fight_file_with_no_handicap_records_no_dmg_side() {
    let ff = example_with("handicap = [2.0, 0.4]\n", "");
    assert_eq!(ff.handicap, vec![1.0, 1.0], "no handicap line is 1.0 each");
    let (rs, _) = compare("no_handicap", &ff);
    assert!(!rs.rules.contains_key("dmg_side"));
    // an explicit 1.0 each is the same fight file, and so the same rules
    let explicit = example_with("handicap = [2.0, 0.4]", "handicap = [1.0, 1.0]");
    assert_eq!(explicit, ff);
    let roster = support::roster();
    let a = setup_from(&ff, &roster, 1).expect("sets up");
    assert_eq!(a.rules.dmg_side, None);
    // any other handicap is a dmg_side, as before
    let one_sided = example_with("handicap = [2.0, 0.4]", "handicap = [1.0, 0.5]");
    let b = setup_from(&one_sided, &roster, 1).expect("sets up");
    assert_eq!(b.rules.dmg_side, Some([1.0, 0.5]));
}

#[test]
fn a_pair_rule_set_to_false_is_recorded() {
    let ff = example_with("swat_cobras = true", "swat_cobras = false");
    let (rs, _) = compare("swat_off", &ff);
    assert_eq!(rs.rules.get("swat_cobras"), Some(&bf_core::event::Value::Bool(false)));
    // and without the pair rule there is no such attribute at all
    let roster = support::roster();
    let none = example_with("[pair_rules]\nswat_cobras = true\n", "");
    let setup = setup_from(&none, &roster, 1).expect("sets up");
    assert_eq!(setup.rules.swat_cobras, None);
    assert!(!setup.rules.to_values().contains_key("swat_cobras"));
    // the fight is the same with the rule off and with no rule: only the map differs
    let mut off = setup_from(&ff, &roster, 1).expect("sets up");
    off.rules.t_max = off.rules.t_fight + 10.0;
    let mut absent = off.clone();
    absent.rules = Rules {
        swat_cobras: None,
        ..off.rules.clone()
    };
    let (a, b) = (run(&off, false).expect("runs"), run(&absent, false).expect("runs"));
    assert_eq!((a.events, a.winner), (b.events, b.winner));
}
