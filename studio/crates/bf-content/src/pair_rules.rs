//! The pair-rule table (Task 7): opt-in engine rules the series switches on for specific pairs of fighters, whichever
//! side each stands on (spec section 4, "Match-up": switched on automatically and shown read-only).

use std::collections::BTreeMap;

use bf_core::event::Value;

/// `(fighter, fighter, rule, value)`: the rules the kit's episodes turn on for a pair, for the fighters ported so far.
/// FLUTE vs TROMBONE: the lance swats cobras (`fight.py` #41; the flute_vs_trombone example's `RULES`).
const TABLE: [(&str, &str, &str, bool); 1] = [("FLUTE", "TROMBONE", "swat_cobras", true)];

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

    #[test]
    fn pair_rule_is_symmetric() {
        for (a, b) in [("FLUTE", "TROMBONE"), ("TROMBONE", "FLUTE")] {
            let rules = pair_rules(a, b);
            assert_eq!(rules.len(), 1, "{a} vs {b}");
            assert_eq!(rules["swat_cobras"], Value::Bool(true), "{a} vs {b}");
        }
    }

    #[test]
    fn other_pairs_have_no_rules() {
        assert!(pair_rules("FLUTE", "TUBA").is_empty());
        assert!(pair_rules("TROMBONE", "TROMBONE").is_empty());
        assert!(pair_rules("FLUTE", "FLUTE").is_empty());
        assert!(pair_rules("", "").is_empty());
    }
}
