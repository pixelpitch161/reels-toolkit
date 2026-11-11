//! The view model every screen fixture starts from: the app as it stands once the sample engine has sent its roster
//! and described the starting pair.

use std::path::PathBuf;

use crate::model::Settings;
use crate::model::queue::Queue;
use crate::seam::Event;
use crate::seam::sample;
use crate::state::{AppState, ViewModel};

/// The sample roster, FLUTE vs TROMBONE, big_top, 34 bars, handicap `[1.0, 1.0]`, no seed, on Match-up, with the
/// drafted sample captions; output folder `Videos\BALLFIGHT`, 7 workers on 8 cores, an empty queue.
pub fn vm() -> ViewModel {
    let settings = Settings {
        output_folder: PathBuf::from("Videos").join("BALLFIGHT"),
        workers: 7,
    };
    let (mut state, _) = AppState::new(settings, Queue::new(), Vec::new(), 8);
    state.apply(Event::Roster(sample::roster()));
    let masks = [0, 1].map(|side| {
        state
            .vm
            .fighter(side)
            .map(|f| f.mask.clone())
            .unwrap_or_else(|| sample::mask(""))
    });
    let card = sample::title_card(state.vm.colors(), [&masks[0], &masks[1]]);
    state.apply(Event::FightInfo {
        r#gen: state.generation(),
        pair_rules: Vec::new(),
        captions: sample::captions(),
        title_card: Some(card),
        problem: None,
    });
    state.vm
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Step, View};

    #[test]
    fn the_base_is_the_starting_pair() {
        let vm = vm();
        assert_eq!(vm.view, View::Step(Step::Matchup));
        assert_eq!(vm.draft.fighters, ["FLUTE".to_string(), "TROMBONE".to_string()]);
        assert_eq!(vm.draft.theme, "big_top");
        assert_eq!(vm.draft.bars, 34);
        assert_eq!(vm.draft.handicap, [1.0, 1.0]);
        assert_eq!(vm.draft.seed, None);
        assert_eq!(vm.draft.captions, sample::captions());
        assert_eq!(vm.status, "Flute vs Trombone, not balanced yet");
        assert_eq!(AppState::showing(vm.clone()).vm, vm);
    }
}
