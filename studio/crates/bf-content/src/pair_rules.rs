//! The pair-rule table (Task 7, milestone 3 Task 3): opt-in engine rules the series switches on for specific pairs of
//! fighters, whichever side each stands on (spec section 4, "Match-up": switched on automatically and shown
//! read-only).

use std::collections::BTreeMap;

use bf_core::event::Value;

/// `(fighter, fighter, rule, value)`: the rules the kit's episodes turn on for a pair, the series' table (one row per
/// link between two fighters):
///
/// - FLUTE and TROMBONE: a live lance swats a risen cobra (`swat_cobras`, `fight.py` #41; the flute_vs_trombone
///   example's `RULES`).
/// - EUPHONIUM and TROMBONE: a live lance swats a falling shell (`swat_shells`, #48, `fight.py` 755 to 756 and 1243 to
///   1271; `ballfight_euphonium_vs_trombone`'s `RULES`).
/// - BASS DRUM and TUBA: a swallowing bell can gulp a rolling drum (`swallow_rollers`, #56, `fight.py` 1100;
///   `ballfight_bassdrum_vs_tuba`'s `RULES`).
const TABLE: [(&str, &str, &str, bool); 3] = [
    ("FLUTE", "TROMBONE", "swat_cobras", true),
    ("EUPHONIUM", "TROMBONE", "swat_shells", true),
    ("BASS DRUM", "TUBA", "swallow_rollers", true),
];

/// The pair rules for `a` against `b`, in either order (none for a pair the table does not list).
pub fn pair_rules(a: &str, b: &str) -> BTreeMap<String, Value> {
    TABLE
        .iter()
        .filter(|(x, y, _, _)| (a == *x && b == *y) || (a == *y && b == *x))
        .map(|(_, _, rule, on)| (rule.to_string(), Value::Bool(*on)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROWS: [(&str, &str, &str); 3] = [
        ("FLUTE", "TROMBONE", "swat_cobras"),
        ("EUPHONIUM", "TROMBONE", "swat_shells"),
        ("BASS DRUM", "TUBA", "swallow_rollers"),
    ];

    #[test]
    fn pair_rule_is_symmetric() {
        for (x, y, rule) in ROWS {
            for (a, b) in [(x, y), (y, x)] {
                let rules = pair_rules(a, b);
                assert_eq!(rules.len(), 1, "{a} vs {b}");
                assert_eq!(rules[rule], Value::Bool(true), "{a} vs {b}");
            }
        }
    }

    /// Every other pair of the 20 kit fighters (a fighter against itself too) has no rule. (The roster's older ten
    /// are `tests/older_pair_rules.rs`'s.)
    #[test]
    fn other_pairs_have_no_rules() {
        let roster = bf_core::roster::parse_roster(include_str!("../data/roster.json")).expect("the roster loads");
        let roster: Vec<_> = roster
            .into_iter()
            .filter(|f| !crate::fighters::OLDER.contains(&f.name.as_str()))
            .collect();
        assert_eq!(roster.len(), 20);
        let mut with_rules = 0;
        for a in &roster {
            for b in &roster {
                let listed = ROWS
                    .iter()
                    .any(|(x, y, _)| (a.name == *x && b.name == *y) || (a.name == *y && b.name == *x));
                let rules = pair_rules(&a.name, &b.name);
                if listed {
                    with_rules += 1;
                } else {
                    assert!(rules.is_empty(), "{} vs {}: {rules:?}", a.name, b.name);
                }
            }
        }
        assert_eq!(with_rules, 6, "three rows, both orders");
        assert!(pair_rules("", "").is_empty());
    }
}
