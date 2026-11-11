//! The Balance screen's view models: the mock-up's numbers, and a balance run half way.

use crate::fixtures::base;
use crate::model::{Step, View};
use crate::seam::sample;
use crate::state::{AppState, ViewModel};

/// The base on Balance, handicap `[2.0, 0.4]`, with the mock-up's numbers (`sample::stats()`): Flute wins 47 % of
/// 3,000 fights. The status line is the state's own for it.
pub fn vm() -> ViewModel {
    let mut vm = on_balance();
    vm.draft.handicap = [2.0, 0.4];
    vm.balance.stats = Some(sample::stats());
    vm.status = AppState::showing(vm.clone()).status();
    vm
}

/// The base on Balance while a balance run is at step 9 of 29, with no numbers yet.
pub fn running() -> ViewModel {
    let mut vm = on_balance();
    vm.balance.running = Some((9, 29));
    vm.status = AppState::showing(vm.clone()).status();
    vm
}

/// The base with Balance shown.
fn on_balance() -> ViewModel {
    let mut vm = base::vm();
    vm.view = View::Step(Step::Balance);
    vm.step = Step::Balance;
    vm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_balance_fixture_is_the_mock_up() {
        let vm = vm();
        assert_eq!(vm.view, View::Step(Step::Balance));
        assert_eq!(vm.step, Step::Balance);
        assert_eq!(vm.draft.handicap, [2.0, 0.4]);
        assert_eq!(vm.balance.stats, Some(sample::stats()));
        assert_eq!(vm.balance.running, None);
        assert_eq!(vm.status, "Balanced on 3,000 fights in 1.4 s");
        assert_eq!(AppState::showing(vm.clone()).vm, vm);
    }

    #[test]
    fn the_running_fixture_has_no_numbers() {
        let vm = running();
        assert_eq!(vm.view, View::Step(Step::Balance));
        assert_eq!(vm.balance.running, Some((9, 29)));
        assert_eq!(vm.balance.stats, None);
        assert_eq!(vm.status, "Flute vs Trombone, not balanced yet");
        assert_eq!(AppState::showing(vm.clone()).vm, vm);
    }
}
