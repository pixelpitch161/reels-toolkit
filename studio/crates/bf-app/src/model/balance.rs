//! What a balance run or a measurement found.

/// The numbers of a set of fights on seeds the search never uses.
#[derive(Clone, Debug, PartialEq)]
pub struct BalanceStats {
    pub fights: u32,
    /// The left fighter's share of decided fights.
    pub win_share: f64,
    /// K.O. times in seconds: the median and the 10th and 90th percentiles.
    pub ko_median: f64,
    pub ko_p10: f64,
    pub ko_p90: f64,
    /// The share of fights with no K.O.
    pub no_ko_share: f64,
    pub lead_changes_mean: f64,
    /// The share of fights that fit the beat sheet.
    pub fit_rate: f64,
    /// K.O. counts in 28 one-second bins, 30 s to 58 s.
    pub ko_hist: Vec<u32>,
    /// How long the run took.
    pub seconds: f64,
}

impl BalanceStats {
    /// The win split is fair enough: the left fighter wins between 40 and 60 percent of decided fights.
    pub fn in_range(&self) -> bool {
        (0.40..=0.60).contains(&self.win_share)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(win_share: f64) -> BalanceStats {
        BalanceStats {
            fights: 3_000,
            win_share,
            ko_median: 50.6,
            ko_p10: 44.2,
            ko_p90: 56.9,
            no_ko_share: 0.0,
            lead_changes_mean: 2.6,
            fit_rate: 0.08,
            ko_hist: vec![0; 28],
            seconds: 1.4,
        }
    }

    #[test]
    fn the_win_range_is_40_to_60() {
        assert!(stats(0.40).in_range());
        assert!(stats(0.60).in_range());
        assert!(stats(0.47).in_range());
        assert!(!stats(0.39).in_range());
        assert!(!stats(0.61).in_range());
    }
}
