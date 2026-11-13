//! The Music screen's view models: the mock-up's take, and the screen before a seed is picked.

use super::base;
use crate::model::{Step, View};
use crate::seam::sample;
use crate::state::{AppState, TakeView, ViewModel};

/// The mock-up's Music screen: the take base (seed 18433, handicap `[2.0, 0.4]`, paused at 0:31.40) on Music with
/// the sample music, Trombone on bass and the music seed 417 (shown "0417"). The status line is the state's own.
pub fn vm() -> ViewModel {
    let mut vm = base::with_take();
    vm.view = View::Step(Step::Music);
    vm.step = Step::Music;
    vm.take.music = Some(sample::music());
    vm.draft.bass = Some(1);
    vm.draft.music_seed = 417;
    vm.status = AppState::showing(vm.clone()).status();
    vm
}

/// Music before a seed is picked: the base on Music with handicap `[2.0, 0.4]`, no seed and no take, Trombone on
/// bass, the music seed 417.
pub fn no_seed() -> ViewModel {
    let mut vm = base::vm();
    vm.view = View::Step(Step::Music);
    vm.step = Step::Music;
    vm.draft.handicap = [2.0, 0.4];
    vm.draft.seed = None;
    vm.draft.bass = Some(1);
    vm.draft.music_seed = 417;
    vm.take = TakeView::default();
    vm.status = AppState::showing(vm.clone()).status();
    vm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_music_fixture_is_the_mock_up() {
        let vm = vm();
        assert_eq!(vm.view, View::Step(Step::Music));
        assert_eq!(vm.draft.seed, Some(18433));
        assert_eq!(vm.draft.bass, Some(1));
        assert_eq!(vm.draft.chord_loop, 0);
        assert_eq!(vm.take.music, Some(sample::music()));
        assert_eq!(vm.status, "Seed 18433 is ready");
        assert_eq!(AppState::showing(vm.clone()).vm, vm);
    }

    #[test]
    fn the_no_seed_fixture_has_no_take() {
        let vm = no_seed();
        assert_eq!(vm.view, View::Step(Step::Music));
        assert_eq!(vm.draft.seed, None);
        assert!(vm.take.story.is_none() && vm.take.music.is_none());
        assert_eq!(vm.draft.bass, Some(1));
        assert_eq!(AppState::showing(vm.clone()).vm, vm);
    }
}
