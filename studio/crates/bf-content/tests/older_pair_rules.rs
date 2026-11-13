//! Milestone 4, Task 3: none of the older ten fighters' kinds has a `Rules` flag the series sets per pair (`fight.py`
//! reads `getattr(self.R, ...)` only for `dmg_side`, `roll_rows`, `roll_start8` and milestone 3's `swallow_rollers`,
//! `swat_cobras`, `swat_shells`, `swat_shell_h`, `swat_shell_r`, `swat_beats`), so the pair-rule table stays as it is:
//! every pair with an older fighter in it, either way round and against each of the 30, has no rule.

use bf_content::fighters::OLDER;
use bf_content::pair_rules;

#[test]
fn every_pair_with_an_older_fighter_has_no_pair_rule() {
    let roster = bf_core::roster::parse_roster(include_str!("../data/roster.json")).expect("the roster loads");
    let (older, kit): (Vec<_>, Vec<_>) = roster.iter().partition(|f| OLDER.contains(&f.name.as_str()));
    assert_eq!((kit.len(), older.len()), (20, 10));
    let all: Vec<&str> = roster.iter().map(|f| f.name.as_str()).collect();
    let mut pairs = 0;
    for a in older.iter().map(|f| f.name.as_str()) {
        for &b in &all {
            assert!(pair_rules(a, b).is_empty(), "{a} vs {b}");
            assert!(pair_rules(b, a).is_empty(), "{b} vs {a}");
            pairs += 1;
        }
    }
    assert_eq!(pairs, 10 * 30);
}
