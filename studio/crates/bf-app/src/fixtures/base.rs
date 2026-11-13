//! The view model every screen fixture starts from: the app as it stands once the sample engine has sent its roster
//! and described the starting pair.

use std::path::PathBuf;

use crate::model::queue::Queue;
use crate::model::take::{SoundState, TransportView};
use crate::model::{Settings, Step, View};
use crate::seam::Event;
use crate::seam::sample;
use crate::state::{AppState, TakeView, ViewModel};

/// The sample roster, FLUTE vs TROMBONE, big_top, 34 bars, handicap `[1.0, 1.0]`, no seed, on Match-up, with the
/// drafted sample captions; output folder `Videos\BALLFIGHT`, 7 workers on 8 cores, an empty queue.
pub fn vm() -> ViewModel {
    let settings = Settings {
        output_folder: PathBuf::from("Videos").join("BALLFIGHT"),
        workers: 7,
        encode: crate::model::EncodeChoice::Best,
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
        music: Some(sample::pair_music(&state.vm.draft)),
    });
    state.vm
}

/// The base with a take: on Seeds, handicap `[2.0, 0.4]`, the first sample story chosen (seed 18433, won by
/// Trombone at 49.1 s) with its music; the transport enabled and paused at frame 942 (0:31.40), loop on, safe zone
/// off, the sound ready. The status line is the state's own for it.
pub fn with_take() -> ViewModel {
    let mut vm = vm();
    let story = sample::stories().swap_remove(0);
    vm.view = View::Step(Step::Seeds);
    vm.step = Step::Seeds;
    vm.draft.handicap = [2.0, 0.4];
    vm.draft.seed = Some(story.seed);
    vm.take = TakeView {
        story: Some(story),
        music: Some(sample::music()),
        loading: false,
        problem: None,
    };
    vm.transport = TransportView {
        enabled: true,
        frame: 942,
        playing: false,
        looping: true,
        safe_zone: false,
        sound: SoundState::Ready,
    };
    vm.status = AppState::showing(vm.clone()).status();
    vm
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn the_take_base_is_seed_18433_at_frame_942() {
        let vm = with_take();
        assert_eq!(vm.view, View::Step(Step::Seeds));
        assert_eq!(vm.draft.handicap, [2.0, 0.4]);
        assert_eq!(vm.draft.seed, Some(18433));
        let story = vm.take.story.as_ref().expect("a story");
        assert_eq!(story.seed, 18433);
        assert_eq!(story.winner, Some(1), "Trombone wins");
        assert_eq!(story.t_ko.map(|t| (t * 10.0).round() / 10.0), Some(49.1));
        assert!(vm.take.music.is_some() && !vm.take.loading && vm.take.problem.is_none());
        assert!(vm.transport.enabled && vm.transport.looping && !vm.transport.safe_zone);
        assert_eq!(vm.transport.frame, 942);
        assert_eq!(vm.transport.sound, SoundState::Ready);
        assert_eq!(vm.status, "Seed 18433 is ready");
        assert_eq!(AppState::showing(vm.clone()).vm, vm);
    }
}
