//! The Matchups tab's actions: picking a row, ticking it as uploaded, and deleting it after asking once. Every
//! change to the history is saved at once (`Effect::SaveHistory`); an action about a row the history does not hold
//! changes nothing and saves nothing.

use super::{Action, AppState, Effect};

impl AppState {
    pub(super) fn history_action(&mut self, action: Action, fx: &mut Vec<Effect>) {
        let view = &mut self.vm.matchups;
        match action {
            Action::SelectFight(id) => {
                if self.vm.history.fight(id).is_some() && view.selected != Some(id) {
                    view.selected = Some(id);
                    view.asking_delete = false;
                }
            }
            Action::SetUploaded { id, uploaded } => {
                if self.vm.history.fight(id).is_some_and(|row| row.uploaded != uploaded) {
                    self.vm.history.set_uploaded(id, uploaded);
                    fx.push(Effect::SaveHistory(self.vm.history.clone()));
                }
            }
            Action::AskDeleteFight => {
                view.asking_delete = view.selected.is_some_and(|id| self.vm.history.fight(id).is_some());
            }
            Action::KeepFight => view.asking_delete = false,
            Action::DeleteFight(id) if self.vm.history.delete(id) => {
                if view.selected == Some(id) {
                    view.selected = None;
                }
                view.asking_delete = false;
                fx.push(Effect::SaveHistory(self.vm.history.clone()));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::Settings;
    use crate::model::queue::Queue;
    use crate::state::{Action, AppState, Effect};

    fn app() -> AppState {
        let settings = Settings {
            output_folder: "Videos".into(),
            workers: 1,
            encode: crate::model::EncodeChoice::Best,
        };
        let (mut s, _) = AppState::new(settings, Queue::new(), Vec::new(), 1);
        s.vm.history = crate::store::history::starter();
        s.vm.history
            .record(["FLUTE".to_string(), "TROMBONE".to_string()], 2210, None, 1_794_744_000);
        s
    }

    fn saves(fx: &[Effect]) -> usize {
        fx.iter().filter(|e| matches!(e, Effect::SaveHistory(_))).count()
    }

    #[test]
    fn ticking_a_row_saves_it() {
        let mut s = app();
        let fx = s.dispatch(Action::SetUploaded { id: 27, uploaded: true });
        assert!(s.vm.history.fight(27).unwrap().uploaded);
        match fx.iter().find(|e| matches!(e, Effect::SaveHistory(_))) {
            Some(Effect::SaveHistory(h)) => assert!(h.fight(27).unwrap().uploaded),
            _ => panic!("{fx:?}"),
        }
    }

    #[test]
    fn an_unknown_row_changes_nothing() {
        let mut s = app();
        let before = s.vm.history.clone();
        assert_eq!(saves(&s.dispatch(Action::SetUploaded { id: 99, uploaded: true })), 0);
        assert_eq!(saves(&s.dispatch(Action::DeleteFight(99))), 0);
        s.dispatch(Action::SelectFight(99));
        assert_eq!(s.vm.matchups.selected, None);
        assert_eq!(s.vm.history, before);
    }

    #[test]
    fn asking_then_deleting_removes_the_row() {
        let mut s = app();
        s.dispatch(Action::SelectFight(27));
        s.dispatch(Action::AskDeleteFight);
        assert!(s.vm.matchups.asking_delete);
        let fx = s.dispatch(Action::DeleteFight(27));
        assert_eq!(saves(&fx), 1);
        assert!(s.vm.history.fight(27).is_none());
        assert_eq!(s.vm.matchups.selected, None);
        assert!(!s.vm.matchups.asking_delete);
    }

    #[test]
    fn keep_it_keeps_the_row_and_another_pick_drops_the_question() {
        let mut s = app();
        s.dispatch(Action::SelectFight(27));
        s.dispatch(Action::AskDeleteFight);
        s.dispatch(Action::KeepFight);
        assert!(!s.vm.matchups.asking_delete);
        assert!(s.vm.history.fight(27).is_some());
        s.dispatch(Action::AskDeleteFight);
        s.dispatch(Action::SelectFight(3));
        assert!(!s.vm.matchups.asking_delete);
        assert_eq!(s.vm.matchups.selected, Some(3));
    }

    #[test]
    fn asking_with_nothing_selected_asks_nothing() {
        let mut s = app();
        s.dispatch(Action::AskDeleteFight);
        assert!(!s.vm.matchups.asking_delete);
    }
}
