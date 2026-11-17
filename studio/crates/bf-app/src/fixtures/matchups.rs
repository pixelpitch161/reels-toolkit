//! The Matchups tab's view models: the starting history plus one new fight, a row naming a fighter the roster does
//! not know, and an empty history.

use super::base;
use crate::model::View;
use crate::model::history::History;
use crate::state::{AppState, ViewModel};
use crate::store::history::starter;

/// 2026-11-15 12:00 UTC: the new fight's date (12:00 UTC, so it reads the same in every time zone).
pub const NEW_FIGHT_MADE: u64 = 1_794_744_000;
/// The new fight's id and seed.
pub const NEW_FIGHT_ID: u64 = 27;
pub const NEW_FIGHT_SEED: u64 = 2210;

/// The 27 earlier episodes plus a new Flute vs Trombone (seed 2210, Flute won, not uploaded), which is selected.
pub fn vm() -> ViewModel {
    let mut history = starter();
    let id = history.record(
        ["FLUTE".to_string(), "TROMBONE".to_string()],
        bf_core::Engine::One,
        NEW_FIGHT_SEED,
        Some("FLUTE".to_string()),
        NEW_FIGHT_MADE,
    );
    let mut vm = on_matchups(history);
    vm.matchups.selected = Some(id);
    vm
}

/// One row whose left fighter, KAZOO, is not on any roster (a hand-edited file, say).
pub fn unknown_fighter() -> ViewModel {
    let mut history = History::default();
    history.record(
        ["KAZOO".to_string(), "FLUTE".to_string()],
        bf_core::Engine::One,
        77,
        Some("KAZOO".to_string()),
        NEW_FIGHT_MADE,
    );
    history.set_uploaded(0, true);
    on_matchups(history)
}

/// No fights at all.
pub fn empty() -> ViewModel {
    on_matchups(History::default())
}

fn on_matchups(history: History) -> ViewModel {
    let mut vm = base::vm();
    vm.view = View::Matchups;
    vm.history = history;
    vm.status = AppState::showing(vm.clone()).status();
    vm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_new_fight_is_the_selected_unticked_row() {
        let vm = vm();
        assert_eq!(vm.matchups.selected, Some(NEW_FIGHT_ID));
        assert!(!vm.history.fight(NEW_FIGHT_ID).unwrap().uploaded);
        assert_eq!(vm.history.summary(), (28, 26, 1));
    }
}
