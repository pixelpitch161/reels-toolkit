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
