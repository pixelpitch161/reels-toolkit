//! The matchup history: every distinct fight the app has made (and the episodes made before it existed), kept
//! between runs in `config/history.toml` (`store::history`).
//!
//! One row is one fight: the same fighters on the same sides with the same seed is the same fight, so rendering it
//! again moves its date on instead of adding a row. A pairing ignores sides: Flute vs Trombone and Trombone vs Flute
//! are one pairing. The roster chips count uploaded fights only.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// One fight made.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FightRecord {
    /// The row's own number, never reused.
    pub id: u64,
    /// The roster names, left then right ("BASS CLARINET").
    pub fighters: [String; 2],
    /// The seed; missing for an old episode whose seed is not known.
    #[serde(default)]
    pub seed: Option<u64>,
    /// The winner's roster name; missing when not known.
    #[serde(default)]
    pub winner: Option<String>,
    /// When it was made, in seconds since 1970 UTC.
    pub made: u64,
    /// The owner ticked it as uploaded.
    #[serde(default)]
    pub uploaded: bool,
}

/// Every fight made, in the order they were first made.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct History {
    #[serde(default)]
    pub fights: Vec<FightRecord>,
    /// The id the next new row gets.
    #[serde(default)]
    pub next_id: u64,
}

impl History {
    /// Notes that the fight `fighters` (left, right) with `seed` was made at `made`, won by `winner`. A row with the
    /// same fighters and seed has its date and winner moved on and keeps its uploaded tick; otherwise a new row is
    /// added, not uploaded. Returns the row's id.
    pub fn record(&mut self, fighters: [String; 2], seed: u64, winner: Option<String>, made: u64) -> u64 {
        if let Some(row) = self
            .fights
            .iter_mut()
            .find(|row| row.fighters == fighters && row.seed == Some(seed))
        {
            row.made = made;
            row.winner = winner;
            return row.id;
        }
        // Never reuse an id still in the list, even if a hand-edited file has `next_id` too low.
        let id = self
            .fights
            .iter()
            .map(|row| row.id.saturating_add(1))
            .max()
            .unwrap_or(0)
            .max(self.next_id);
        self.fights.push(FightRecord {
            id,
            fighters,
            seed: Some(seed),
            winner,
            made,
            uploaded: false,
        });
        self.next_id = id.saturating_add(1);
        id
    }

    /// Ticks or unticks a row. False when there is no row `id`.
    pub fn set_uploaded(&mut self, id: u64, uploaded: bool) -> bool {
        match self.fights.iter_mut().find(|row| row.id == id) {
            Some(row) => {
                row.uploaded = uploaded;
                true
            }
            None => false,
        }
    }

    /// Removes a row. False when there is no row `id`.
    pub fn delete(&mut self, id: u64) -> bool {
        let before = self.fights.len();
        self.fights.retain(|row| row.id != id);
        self.fights.len() != before
    }

    /// The row with `id`.
    pub fn fight(&self, id: u64) -> Option<&FightRecord> {
        self.fights.iter().find(|row| row.id == id)
    }

    /// The rows, newest made first (a tie: the higher id first).
    pub fn newest_first(&self) -> Vec<&FightRecord> {
        let mut rows: Vec<&FightRecord> = self.fights.iter().collect();
        rows.sort_by(|a, b| b.made.cmp(&a.made).then(b.id.cmp(&a.id)));
        rows
    }

    /// For every opponent `fighter` has an uploaded fight with (on either side), how many.
    pub fn uploaded_counts(&self, fighter: &str) -> BTreeMap<String, u32> {
        let mut counts = BTreeMap::new();
        for row in self.fights.iter().filter(|row| row.uploaded) {
            let [left, right] = &row.fighters;
            let opponent = if left == fighter {
                right
            } else if right == fighter {
                left
            } else {
                continue;
            };
            *counts.entry(opponent.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// (fights, distinct pairings ignoring sides, fights not uploaded).
    pub fn summary(&self) -> (usize, usize, usize) {
        let pairings: BTreeSet<[&str; 2]> = self
            .fights
            .iter()
            .map(|row| {
                let (a, b) = (row.fighters[0].as_str(), row.fighters[1].as_str());
                if a <= b { [a, b] } else { [b, a] }
            })
            .collect();
        let not_uploaded = self.fights.iter().filter(|row| !row.uploaded).count();
        (self.fights.len(), pairings.len(), not_uploaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(a: &str, b: &str) -> [String; 2] {
        [a.to_string(), b.to_string()]
    }

    #[test]
    fn recording_a_new_fight_adds_a_row() {
        let mut h = History::default();
        let id = h.record(pair("FLUTE", "TROMBONE"), 2210, Some("FLUTE".to_string()), 100);
        assert_eq!(id, 0);
        assert_eq!(h.fights.len(), 1);
        assert!(!h.fights[0].uploaded);
        assert_eq!(h.fights[0].seed, Some(2210));
        assert_eq!(h.next_id, 1);
    }

    #[test]
    fn the_same_fighters_and_seed_update_the_row() {
        let mut h = History::default();
        let id = h.record(pair("FLUTE", "TROMBONE"), 2210, Some("FLUTE".to_string()), 100);
        assert!(h.set_uploaded(id, true));
        let again = h.record(pair("FLUTE", "TROMBONE"), 2210, Some("TROMBONE".to_string()), 200);
        assert_eq!(again, id);
        assert_eq!(h.fights.len(), 1);
        assert_eq!(h.fights[0].made, 200);
        assert_eq!(h.fights[0].winner.as_deref(), Some("TROMBONE"));
        assert!(h.fights[0].uploaded);
    }

    #[test]
    fn a_new_seed_or_swapped_sides_is_a_new_row() {
        let mut h = History::default();
        h.record(pair("FLUTE", "TROMBONE"), 2210, None, 100);
        h.record(pair("FLUTE", "TROMBONE"), 2211, None, 100);
        assert_eq!(h.fights.len(), 2);
        h.record(pair("TROMBONE", "FLUTE"), 2210, None, 100);
        assert_eq!(h.fights.len(), 3);
        assert_eq!(h.next_id, 3);
    }

    #[test]
    fn a_new_row_never_takes_an_id_in_use() {
        let mut h = History::default();
        h.record(pair("FLUTE", "TROMBONE"), 1, None, 100);
        h.record(pair("FLUTE", "TROMBONE"), 2, None, 100);
        h.next_id = 0;
        assert_eq!(h.record(pair("FLUTE", "TROMBONE"), 3, None, 100), 2);
    }

    fn three_rows() -> History {
        let mut h = History::default();
        let a = h.record(pair("FLUTE", "TROMBONE"), 1, None, 100);
        let b = h.record(pair("TROMBONE", "FLUTE"), 2, None, 100);
        h.record(pair("FLUTE", "TUBA"), 3, None, 100);
        h.set_uploaded(a, true);
        h.set_uploaded(b, true);
        h
    }

    #[test]
    fn counts_use_uploaded_fights_only_and_ignore_sides() {
        let h = three_rows();
        let flute: BTreeMap<String, u32> = [("TROMBONE".to_string(), 2)].into();
        let trombone: BTreeMap<String, u32> = [("FLUTE".to_string(), 2)].into();
        assert_eq!(h.uploaded_counts("FLUTE"), flute);
        assert_eq!(h.uploaded_counts("TROMBONE"), trombone);
        assert!(h.uploaded_counts("TUBA").is_empty());
    }

    #[test]
    fn summary_counts_fights_pairings_and_not_uploaded() {
        assert_eq!(three_rows().summary(), (3, 2, 1));
        assert_eq!(History::default().summary(), (0, 0, 0));
    }

    #[test]
    fn newest_first_orders_by_made_then_id() {
        let mut h = History::default();
        h.record(pair("A", "B"), 1, None, 100);
        h.record(pair("A", "B"), 2, None, 300);
        h.record(pair("A", "B"), 3, None, 300);
        let order: Vec<u64> = h.newest_first().iter().map(|row| row.id).collect();
        assert_eq!(order, vec![2, 1, 0]);
    }

    #[test]
    fn delete_and_set_uploaded_report_missing_ids() {
        let mut h = three_rows();
        assert!(!h.set_uploaded(99, true));
        assert!(!h.delete(99));
        assert!(h.delete(0));
        assert_eq!(h.fights.len(), 2);
        assert!(h.fight(0).is_none());
    }
}
