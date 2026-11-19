//! The beat sheet and the score: a port of `projects/ballfight/kit/story.py` `score()`, with the series defaults of
//! `kit/episode.py` (`T_KO`, `MIN_LEAD_CHANGES`, `MAX_WINNER_HP`, `CAP_GAP`, `MAX_GAP`, `CAP_TARGETS`).
//!
//! The kit returns only `(fits, score)`; [`Verdict::misses`] adds the reasons in plain words, one per failed window.

use std::collections::BTreeMap;

use bf_core::py::{py_max, round_nd};
use bf_core::roster::FighterCfg;

use crate::story::Story;

/// The beat sheet: the hard windows a fight must meet to fit, and the caption targets the score aims at.
#[derive(Clone, Debug, PartialEq)]
pub struct BeatSheet {
    /// `T_KO`: the K.O. must land in this window, in seconds on the video clock (49 to 53).
    pub t_ko: (f64, f64),
    /// `MIN_LEAD_CHANGES`: at least this many lead changes (2).
    pub min_lead_changes: u32,
    /// `MAX_WINNER_HP`: the winner ends with at most this many hit points (30).
    pub max_winner_hp: f64,
    /// `CAP_GAP`: the least time between two kept ability captions (4.0 s).
    pub cap_gap: f64,
    /// `MAX_GAP`: the longest allowed stretch without a hit (4.0 s).
    pub max_gap: f64,
    /// At least this many ability captions (5, a literal in `score()`).
    pub min_caps: usize,
    /// `CAP_TARGETS`: when each ability's caption should come, by ability name (the series default: unlock1 at
    /// 12 s, unlock2 at 24 s, low at 40 s). An ability without a target costs nothing.
    pub cap_targets: BTreeMap<String, f64>,
}

impl BeatSheet {
    /// The series beat sheet for two fighters (left first): the `kit/episode.py` default windows (`T_KO` 49 to 53,
    /// `MIN_LEAD_CHANGES` 2, `MAX_WINNER_HP` 30, `CAP_GAP` 4.0, `MAX_GAP` 4.0, 5 captions) and its default
    /// `CAP_TARGETS`, `{ROSTER[n]['abilities'][s]['name']: v for n in FIGHTERS for s, v in (('unlock1', 12),
    /// ('unlock2', 24), ('low', 40))}` (a name met twice keeps the later value, as the dict does).
    ///
    /// There is deliberately no `Default`: a sheet without the fighters' caption targets would drop the score's
    /// caption terms without a word and mis-rank fights.
    pub fn series(fighters: [&FighterCfg; 2]) -> BeatSheet {
        let mut cap_targets = BTreeMap::new();
        for f in fighters {
            for (ability, v) in [(&f.unlock1, 12.0), (&f.unlock2, 24.0), (&f.low, 40.0)] {
                cap_targets.insert(ability.name.clone(), v);
            }
        }
        BeatSheet {
            t_ko: (49.0, 53.0),
            min_lead_changes: 2,
            max_winner_hp: 30.0,
            cap_gap: 4.0,
            max_gap: 4.0,
            min_caps: 5,
            cap_targets,
        }
    }
}

/// `x` written with two decimals, or more when two would round it onto the right side of its limit: the shown
/// value always still fails (`above`: it must show greater than `limit`; else less than it).
fn past(x: f64, limit: f64, above: bool) -> String {
    for digits in 2..=17 {
        let text = format!("{x:.digits$}");
        let shown: f64 = text.parse().unwrap_or(x);
        if (above && shown > limit) || (!above && shown < limit) {
            return text;
        }
    }
    format!("{x:?}")
}

/// Whether a fight fits the beat sheet, its score, and why it does not fit.
#[derive(Clone, Debug, PartialEq)]
pub struct Verdict {
    /// All hard windows are met.
    pub fits: bool,
    /// The soft story score, `round(s, 2)`: higher is a better story.
    pub score: f64,
    /// One plain-words reason per failed window, in the order K.O., lead changes, winner HP, captions, longest gap
    /// (empty exactly when `fits`). A fight with no K.O. gives only "no K.O.": the other windows mean nothing then.
    pub misses: Vec<String>,
}

/// `kit/story.py` `score(st)`: the hard windows, then the soft score, term by term in the kit's order, rounded
/// with Python's `round(s, 2)`.
pub fn score(st: &Story, sheet: &BeatSheet) -> Verdict {
    // tko = st['t_ko'] or 99
    let tko = st.t_ko.filter(|&t| t != 0.0).unwrap_or(99.0);
    let ko_ok = sheet.t_ko.0 <= tko && tko <= sheet.t_ko.1;
    let lc_ok = st.lead_changes >= sheet.min_lead_changes;
    let hp_ok = st.winner_hp <= sheet.max_winner_hp;
    let caps_ok = st.caps.len() >= sheet.min_caps;
    let gap_ok = st.max_gap <= sheet.max_gap;
    let fits = ko_ok && lc_ok && hp_ok && caps_ok && gap_ok;

    let lc = i64::from(st.lead_changes);
    let mut s = 0.0;
    s += (sheet.max_winner_hp - st.winner_hp) / 3.0; // a close finish
    s += 2.0 * 4.min(lc - 2) as f64;
    s += 3.0 * f64::from(u8::from(st.lc_t.iter().any(|&t| t > 40.0))); // a late lead change
    s += 1.5 * (st.caps.len() as i64 - 5) as f64;
    s += 2.0 * f64::from(u8::from(st.lows.len() == 2));
    for (t, _, name) in &st.caps {
        // T.CAP_TARGETS.get(name, t)
        let target = sheet.cap_targets.get(name).copied().unwrap_or(*t);
        s -= (t - target).abs() / 4.0;
    }
    s -= 2.0 * py_max(0.0, st.max_gap - 2.5);
    s -= 2.0 * (tko - 51.0).abs();

    let mut misses = Vec::new();
    if !ko_ok && st.t_ko.is_none() {
        misses.push("no K.O.".to_string());
    } else {
        if !ko_ok {
            let shown = if tko < sheet.t_ko.0 {
                past(tko, sheet.t_ko.0, false)
            } else {
                past(tko, sheet.t_ko.1, true)
            };
            misses.push(format!(
                "K.O. at {shown} s (needs {} to {} s)",
                sheet.t_ko.0, sheet.t_ko.1
            ));
        }
        if !lc_ok {
            let plural = if st.lead_changes == 1 { "" } else { "s" };
            misses.push(format!(
                "{} lead change{plural} (needs {})",
                st.lead_changes, sheet.min_lead_changes
            ));
        }
        if !hp_ok {
            misses.push(format!(
                "winner HP {} (needs {} or less)",
                past(st.winner_hp, sheet.max_winner_hp, true),
                sheet.max_winner_hp
            ));
        }
        if !caps_ok {
            let plural = if st.caps.len() == 1 { "" } else { "s" };
            misses.push(format!(
                "{} ability caption{plural} (needs {})",
                st.caps.len(),
                sheet.min_caps
            ));
        }
        if !gap_ok {
            misses.push(format!(
                "{} s without a hit (needs {} s or less)",
                past(st.max_gap, sheet.max_gap, true),
                sheet.max_gap
            ));
        }
    }
    Verdict {
        fits,
        score: round_nd(s, 2),
        misses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ep392's story (the kit's numbers, rounded where the exact bits do not matter here): it fits.
    fn fit() -> Story {
        Story {
            seed: 392,
            winner: Some(1),
            t_ko: Some(50.021568627450975),
            lead_changes: 2,
            lc_t: vec![22.47157, 42.8799],
            winner_hp: 4.080000000000105,
            firsts: vec![],
            caps: vec![
                (11.21324, 0, "FLUTTER TONGUE".into()),
                (16.1549, 1, "SAD TROMBONE".into()),
                (26.02157, 1, "POWER SLIDE".into()),
                (30.0549, 0, "SNAKE DANCE".into()),
                (42.61324, 0, "PICCOLO".into()),
            ],
            max_gap: 1.3499999999999996,
            lows: vec!["PEDAL TONE".into(), "PICCOLO".into()],
            lead: vec![],
        }
    }

    /// The example episode's sheet, written out (the parity test builds it with `series` from the roster).
    fn sheet() -> BeatSheet {
        BeatSheet {
            t_ko: (49.0, 53.0),
            min_lead_changes: 2,
            max_winner_hp: 30.0,
            cap_gap: 4.0,
            max_gap: 4.0,
            min_caps: 5,
            cap_targets: [
                ("FLUTTER TONGUE", 12.0),
                ("SAD TROMBONE", 12.0),
                ("POWER SLIDE", 24.0),
                ("SNAKE DANCE", 24.0),
                ("PICCOLO", 40.0),
                ("PEDAL TONE", 40.0),
            ]
            .into_iter()
            .map(|(n, v)| (n.to_string(), v))
            .collect(),
        }
    }

    #[test]
    fn a_fit_has_no_misses() {
        let v = score(&fit(), &sheet());
        assert!(v.fits);
        assert_eq!(v.score, 7.78);
        assert!(v.misses.is_empty());
    }

    #[test]
    fn each_failed_window_gives_one_reason_in_a_fixed_order() {
        let mut st = fit();
        st.t_ko = Some(47.2);
        st.lead_changes = 1;
        st.winner_hp = 46.6;
        st.caps.truncate(3);
        st.max_gap = 4.26;
        let v = score(&st, &sheet());
        assert!(!v.fits);
        assert_eq!(
            v.misses,
            [
                "K.O. at 47.20 s (needs 49 to 53 s)",
                "1 lead change (needs 2)",
                "winner HP 46.60 (needs 30 or less)",
                "3 ability captions (needs 5)",
                "4.26 s without a hit (needs 4 s or less)",
            ]
        );
        for one in 0..5 {
            let mut st = fit();
            match one {
                0 => st.t_ko = Some(53.5),
                1 => st.lead_changes = 0,
                2 => st.winner_hp = 30.5,
                3 => st.caps.truncate(4),
                _ => st.max_gap = 4.01,
            }
            let v = score(&st, &sheet());
            assert!(!v.fits);
            assert_eq!(v.misses.len(), 1, "{one}: {:?}", v.misses);
        }
    }

    #[test]
    fn no_ko_is_the_only_reason_without_a_ko() {
        let mut st = fit();
        st.winner = None;
        st.t_ko = None;
        st.winner_hp = 100.0;
        st.lead_changes = 0;
        let v = score(&st, &sheet());
        assert!(!v.fits);
        assert_eq!(v.misses, ["no K.O."]);
        // tko counts as 99 s: 2 * |99 - 51| = 96 off the score
        assert!(v.score < -90.0, "{}", v.score);
    }

    #[test]
    fn a_window_that_admits_99_s_lets_a_no_ko_fight_fit_as_in_the_kit() {
        let mut st = fit();
        st.t_ko = None;
        let mut sh = sheet();
        sh.t_ko = (49.0, 100.0);
        let v = score(&st, &sh);
        assert!(v.fits);
        assert!(v.misses.is_empty());
    }

    #[test]
    fn an_ability_without_a_target_costs_nothing() {
        let mut sh = sheet();
        let with = score(&fit(), &sh).score;
        sh.cap_targets.remove("PICCOLO");
        // PICCOLO at 42.61324 against 40: |2.61324| / 4 = 0.65331 less off the score (both rounded to 2 places)
        let without = score(&fit(), &sh).score;
        assert!((without - with - 0.65331).abs() < 0.0051, "{with} -> {without}");
    }

    /// The one miss of `st` under the example sheet.
    fn only_miss(st: &Story) -> String {
        let v = score(st, &sheet());
        assert!(!v.fits);
        assert_eq!(v.misses.len(), 1, "{:?}", v.misses);
        v.misses[0].clone()
    }

    #[test]
    fn a_miss_never_shows_a_value_that_meets_its_limit() {
        let mut st = fit();
        st.winner_hp = 30.04;
        assert_eq!(only_miss(&st), "winner HP 30.04 (needs 30 or less)");
        st.winner_hp = 30.001;
        assert_eq!(only_miss(&st), "winner HP 30.001 (needs 30 or less)");

        let mut st = fit();
        st.max_gap = 4.02;
        assert_eq!(only_miss(&st), "4.02 s without a hit (needs 4 s or less)");
        st.max_gap = 4.000_000_7;
        assert_eq!(only_miss(&st), "4.000001 s without a hit (needs 4 s or less)");

        let mut st = fit();
        st.t_ko = Some(48.97);
        assert_eq!(only_miss(&st), "K.O. at 48.97 s (needs 49 to 53 s)");
        st.t_ko = Some(48.998);
        assert_eq!(only_miss(&st), "K.O. at 48.998 s (needs 49 to 53 s)");
        st.t_ko = Some(53.004);
        assert_eq!(only_miss(&st), "K.O. at 53.004 s (needs 49 to 53 s)");
    }

    #[test]
    fn past_shows_the_least_digits_that_stay_past_the_limit() {
        assert_eq!(past(30.04, 30.0, true), "30.04");
        assert_eq!(past(30.0001, 30.0, true), "30.0001");
        assert_eq!(past(48.9999, 49.0, false), "48.9999");
        assert_eq!(past(46.6, 30.0, true), "46.60");
        // the next float above 4.0 needs every digit
        let next = f64::from_bits(4.0f64.to_bits() + 1);
        let text = past(next, 4.0, true);
        assert!(text.parse::<f64>().unwrap() > 4.0, "{text}");
    }
}
