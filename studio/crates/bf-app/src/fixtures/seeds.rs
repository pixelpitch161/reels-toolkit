//! The Seeds screen's view models: the mock-up's shortlist, and a shortlist with nothing in it yet.

use super::base;
use crate::model::shortlist::{SearchState, Shortlist};
use crate::model::{Step, View};
use crate::seam::sample;
use crate::state::{AppState, TakeView, ViewModel};

/// The mock-up's Seeds screen: the take base (seed 18433 chosen, handicap `[2.0, 0.4]`) with the 14 sample stories
/// ranked, 212 fits from 18,240 checked, still searching on 7 workers. The status line is the state's own for it.
pub fn vm() -> ViewModel {
    let mut vm = base::with_take();
    vm.shortlist = Shortlist {
        rows: sample::stories(),
        fits_total: 212,
        checked: 18_240,
        state: SearchState::Searching,
        workers: 7,
    };
    vm.status = AppState::showing(vm.clone()).status();
    vm
}

/// Seeds with an empty shortlist in `state`: the base on Seeds with handicap `[2.0, 0.4]`, no seed chosen, no fits
/// from 2,048 checked (fewer than `screens::seeds::NO_FIT_AFTER`, so the search may still find one), 7 workers.
pub fn empty(state: SearchState) -> ViewModel {
    let mut vm = base::vm();
    vm.view = View::Step(Step::Seeds);
    vm.step = Step::Seeds;
    vm.draft.handicap = [2.0, 0.4];
    vm.draft.seed = None;
    vm.take = TakeView::default();
    vm.shortlist = Shortlist {
        rows: Vec::new(),
        fits_total: 0,
        checked: 2_048,
        state,
        workers: 7,
    };
    vm.status = AppState::showing(vm.clone()).status();
    vm
}

/// Seeds when the search found no fight that fits in 5,120 seeds, still searching: the match-up is likely too uneven
/// at its handicap (milestone 3 Task 60, note (a)).
pub fn uneven() -> ViewModel {
    let mut vm = empty(SearchState::Searching);
    vm.shortlist.checked = 5_120;
    vm.status = AppState::showing(vm.clone()).status();
    vm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seeds_fixture_is_the_mock_up() {
        let vm = vm();
        assert_eq!(vm.view, View::Step(Step::Seeds));
        assert_eq!(vm.shortlist.rows.len(), 14);
        assert_eq!(vm.shortlist.rows[0].seed, 18433);
        assert_eq!(vm.draft.seed, Some(18433));
        assert_eq!(vm.shortlist.fits_total, 212);
        assert_eq!(vm.shortlist.checked, 18_240);
        assert_eq!(vm.shortlist.state, SearchState::Searching);
        assert_eq!(vm.status, "Searching on 7 cores");
        assert_eq!(AppState::showing(vm.clone()).vm, vm);
    }

    #[test]
    fn the_empty_fixture_has_no_rows_and_no_seed() {
        for state in [SearchState::Searching, SearchState::Stopped] {
            let vm = empty(state);
            assert!(vm.shortlist.rows.is_empty());
            assert_eq!(vm.shortlist.state, state);
            assert_eq!(vm.draft.seed, None);
            assert!(vm.take.story.is_none());
            assert_eq!(AppState::showing(vm.clone()).vm, vm);
        }
    }
}
