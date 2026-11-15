//! The Match-up screen's view model: the base with one pair rule switched on.

use crate::fixtures::base;
use crate::state::ViewModel;

/// The base (FLUTE vs TROMBONE, big_top, 34 bars, on Match-up) with the pair rule "A live lance knocks a risen cobra
/// flat."
pub fn vm() -> ViewModel {
    let mut vm = base::vm();
    vm.pair_rules = vec!["A live lance knocks a risen cobra flat.".to_string()];
    vm
}

/// [`vm`] with the matchup history: the 27 earlier episodes, a second uploaded Flute vs Clarinet and an unticked
/// Flute vs Oboe, so Flute's chips count Clarinet 2, Sousaphone 1 and Tuba 1, and Oboe nothing.
pub fn counted() -> ViewModel {
    let mut vm = vm();
    let mut history = crate::store::history::starter();
    let again = history.record(["FLUTE".to_string(), "CLARINET".to_string()], 515, None, 1_794_744_000);
    history.set_uploaded(again, true);
    history.record(["FLUTE".to_string(), "OBOE".to_string()], 516, None, 1_794_744_000);
    vm.history = history;
    vm
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Step, View};

    #[test]
    fn the_match_up_fixture_is_the_base_with_one_pair_rule() {
        let vm = vm();
        assert_eq!(vm.view, View::Step(Step::Matchup));
        assert_eq!(vm.pair_rules, ["A live lance knocks a risen cobra flat."]);
        assert_eq!(vm.fight_problem, None);
        assert_eq!(vm.draft.bars, 34);
    }
}
