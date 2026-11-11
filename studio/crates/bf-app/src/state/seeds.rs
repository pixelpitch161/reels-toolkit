//! The search, its filters, the chosen seed and its take.

use super::{AppState, Effect, TakeView};
use crate::model::shortlist::{FilterSpec, SearchState};
use crate::model::{Step, View};
use crate::seam::Command;
use crate::wording;

/// The K.O. filter's bounds, in seconds.
const KO: (f64, f64) = (30.0, 58.0);

impl AppState {
    /// Starts (or goes on with) the search for the current generation.
    pub(super) fn start_search(&mut self, fx: &mut Vec<Effect>) {
        if !self.can_ask() {
            return;
        }
        self.vm.shortlist.state = SearchState::Searching;
        fx.push(Effect::Send(Command::StartSearch {
            r#gen: self.generation,
            draft: self.vm.draft.clone(),
            filters: self.vm.filters.clone(),
        }));
        let workers = self.vm.settings.workers as u64;
        self.say(format!("Searching on {}", wording::plural(workers, "core", "cores")));
    }

    pub(super) fn stop_search(&mut self, fx: &mut Vec<Effect>) {
        if self.vm.shortlist.state == SearchState::Searching {
            self.vm.shortlist.state = SearchState::Stopped;
            fx.push(Effect::Send(Command::StopSearch));
        }
    }

    /// Stores the filters, the K.O. bounds kept inside 30 to 58 s and in order, and has the engine apply them.
    pub(super) fn set_filters(&mut self, mut filters: FilterSpec, fx: &mut Vec<Effect>) {
        let old = self.vm.filters.ko;
        let keep = |v: f64, was: f64| if v.is_finite() { v.clamp(KO.0, KO.1) } else { was };
        let (a, b) = (keep(filters.ko.0, old.0), keep(filters.ko.1, old.1));
        filters.ko = (a.min(b), a.max(b));
        self.vm.filters = filters;
        if self.fed.is_some() {
            fx.push(Effect::Send(Command::SetFilters {
                r#gen: self.generation,
                filters: self.vm.filters.clone(),
            }));
        }
    }

    pub(super) fn select_seed(&mut self, seed: u64, fx: &mut Vec<Effect>) {
        if !self.can_ask() {
            return;
        }
        self.vm.draft.seed = Some(seed);
        self.ask_take(fx);
    }

    /// Asks for the take of the draft's seed as it stands; answers for any earlier take are late from now on.
    pub(crate) fn ask_take(&mut self, fx: &mut Vec<Effect>) {
        if !self.can_ask() || self.vm.draft.seed.is_none() {
            return;
        }
        self.last_take += 1;
        self.wanted = Some(self.last_take);
        self.vm.take = TakeView {
            loading: true,
            ..TakeView::default()
        };
        fx.push(Effect::Send(Command::LoadTake {
            take: self.last_take,
            draft: self.vm.draft.clone(),
        }));
    }

    /// Goes on to Music with the chosen seed, only when its fight has a winner.
    pub(super) fn use_seed(&mut self) {
        if self.vm.take.story.as_ref().is_some_and(|s| s.winner.is_some()) {
            self.vm.view = View::Step(Step::Music);
            self.vm.step = Step::Music;
        }
    }
}
