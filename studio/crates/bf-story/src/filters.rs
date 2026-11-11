//! The shortlist's filters: which candidates of a search a person asked to see. The kit has none (its search
//! prints the fits and a person reads them); these are the Studio's own (the design's section on seeds).

use crate::search::Candidate;

/// A close finish: the winner ends on at most this many hit points.
pub const CLOSE_FINISH_HP: f64 = 12.0;
/// A comeback: the winner was at least this many hit points behind ...
pub const COMEBACK_DEFICIT: f64 = 20.0;
/// ... at some time after this many seconds on the video clock,
pub const COMEBACK_AFTER: f64 = 15.0;
/// or was the first of the two to fall below this many hit points (the rules' `LOW_HP`, where a fighter's
/// desperation move comes up).
pub const COMEBACK_LOW_HP: f64 = 25.0;

/// The room left for rounding when hit points are worked back from [`crate::Story::lead`] (sums of a hundred or so
/// differences of numbers below 100: off by 1e-12 at most). A fighter on 25 HP to the last bit is not below 25.
const HP_SLACK: f64 = 1e-9;

/// What to keep of a shortlist. The default keeps every fight that fits the beat sheet; each filter that is set
/// narrows it further (a candidate must pass them all).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Filters {
    /// Only fights this fighter won (0 is the left corner).
    pub winner: Option<usize>,
    /// Only fights whose K.O. came in this time range, in seconds on the video clock, both ends included.
    pub ko: Option<(f64, f64)>,
    /// Only fights with at least this many lead changes.
    pub min_lead_changes: Option<u32>,
    /// Only close finishes: the winner ends on [`CLOSE_FINISH_HP`] or less.
    pub close_finish: bool,
    /// Only comebacks: the winner trailed by [`COMEBACK_DEFICIT`] HP or more at some time after [`COMEBACK_AFTER`]
    /// seconds, or was the first to fall below [`COMEBACK_LOW_HP`] HP.
    pub comeback: bool,
    /// Only fights in which this ability was used (its name as the roster has it, such as `PICCOLO`).
    pub ability: Option<String>,
    /// Also show fights that do not fit the beat sheet.
    pub show_misfits: bool,
}

impl Filters {
    /// Whether the shortlist shows this candidate. A filter about the winner or the finish never keeps a fight
    /// with no K.O.
    pub fn keep(&self, c: &Candidate) -> bool {
        let st = &c.story;
        if !self.show_misfits && !c.verdict.fits {
            return false;
        }
        if self.winner.is_some_and(|w| st.winner != Some(w)) {
            return false;
        }
        if let Some((lo, hi)) = self.ko
            && !st.t_ko.is_some_and(|t| lo <= t && t <= hi)
        {
            return false;
        }
        if self.min_lead_changes.is_some_and(|n| st.lead_changes < n) {
            return false;
        }
        if self.close_finish && !(st.winner.is_some() && st.winner_hp <= CLOSE_FINISH_HP) {
            return false;
        }
        if self.comeback && !comeback(st.winner, &st.lead) {
            return false;
        }
        if let Some(name) = &self.ability
            && !st.firsts.iter().any(|(_, _, used)| used == name)
        {
            return false;
        }
        true
    }
}

/// Whether the winner came from behind, read off the lead ribbon (`Story::lead`: after each hit, the left
/// fighter's HP less the right fighter's).
///
/// - **Trailed by 20 or more after 15 s.** The lead after a hit stands until the next hit, so a deficit counts when
///   it still stood at some moment later than 15 s: the hit after it came after 15 s (the last entry is the K.O.,
///   where the winner cannot be behind).
/// - **First below 25 HP.** A hit takes HP from one fighter, so a step down of the lead is the left fighter's loss
///   and a step up the right fighter's: the two HP lines are worked back from the ribbon, starting at 100 each, and
///   the first of the two to be below 25 is looked for (a `heal` between two hits, which no ported fighter has,
///   would be read as part of the next hit).
fn comeback(winner: Option<usize>, lead: &[(f64, f64)]) -> bool {
    let Some(w) = winner else {
        return false;
    };
    // the winner's deficit is the lead seen from the other side
    let behind = |d: f64| if w == 0 { -d } else { d };
    let trailed = lead.iter().enumerate().any(|(i, &(_, d))| {
        let until = lead.get(i + 1).map_or(f64::INFINITY, |next| next.0);
        until > COMEBACK_AFTER && behind(d) >= COMEBACK_DEFICIT
    });
    if trailed {
        return true;
    }
    let mut hp = [100.0f64, 100.0];
    let mut before = 0.0;
    for &(_, d) in lead {
        let step = d - before;
        before = d;
        if step < 0.0 {
            hp[0] += step;
        } else {
            hp[1] -= step;
        }
        let low = [hp[0] < COMEBACK_LOW_HP - HP_SLACK, hp[1] < COMEBACK_LOW_HP - HP_SLACK];
        if low[0] || low[1] {
            // one hit lowers one fighter, so only one of them can be newly below
            return low[w] && !low[1 - w];
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ribbon_with_no_hits_is_no_comeback() {
        assert!(!comeback(Some(0), &[]));
        assert!(!comeback(None, &[(20.0, -50.0)]));
    }

    #[test]
    fn the_hp_lines_come_back_out_of_the_ribbon() {
        // left 100 -> 60 -> 24.5, right 100 -> 90 -> 30, all before 15 s: the left fighter is first below 25 ...
        let start = [(5.0, -40.0), (8.0, -30.0), (12.0, -65.5), (13.0, -5.5)];
        // ... and wins (the right fighter falls to 0)
        let mut lead = start.to_vec();
        lead.push((50.0, 24.5));
        assert!(comeback(Some(0), &lead));
        // ... or loses (it falls to 0 itself): the winner was never behind after 15 s and never first below 25
        let mut lead = start.to_vec();
        lead.push((50.0, -30.0));
        assert!(!comeback(Some(1), &lead));
    }

    #[test]
    fn rounding_in_the_ribbon_does_not_make_25_hp_low() {
        // 75 hits of 1.0 on the left fighter leave exactly 25 HP; the ribbon's differences are off in the last bits
        let mut lead = Vec::new();
        let mut d = 0.0f64;
        for i in 0..75 {
            d -= 1.0 + 1e-14 * f64::from(i % 3);
            lead.push((4.0 + 0.1 * f64::from(i), d));
        }
        lead.push((14.0, d + 80.0)); // then the right fighter falls to 20
        assert!(
            !comeback(Some(0), &lead),
            "the left fighter stood on 25, the right one was first below"
        );
        assert!(comeback(Some(1), &lead));
    }
}
