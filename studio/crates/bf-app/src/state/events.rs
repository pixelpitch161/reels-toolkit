//! What the engine's answers change. An answer for another generation, or for a take that is no longer wanted,
//! is late: it changes nothing and asks for nothing (Review Focus 3).

use super::{AppState, Effect};
use crate::model::draft::Draft;
use crate::model::roster::Roster;
use crate::model::take::PlayerCmd;
use crate::model::{FPS, NF, Step, View};
use crate::seam::Event;

impl AppState {
    pub(super) fn on_event(&mut self, event: Event, fx: &mut Vec<Effect>) {
        match event {
            Event::Roster(roster) => self.on_roster(roster),
            Event::FightInfo {
                r#gen,
                pair_rules,
                captions,
                title_card,
                problem,
            } => {
                // The description is of the fighters, so any generation since they were last picked is theirs.
                if r#gen < self.described || r#gen > self.generation {
                    return;
                }
                self.vm.pair_rules = pair_rules;
                if !self.vm.draft.captions_edited {
                    self.vm.draft.captions = captions;
                }
                self.title_card = title_card;
                if self.vm.take.story.is_none() {
                    fx.push(self.card_or_clear());
                }
                if let Some(text) = &problem {
                    self.vm.status = text.clone();
                }
                self.vm.fight_problem = problem;
            }
            Event::BalanceProgress { r#gen, done, total } => {
                if r#gen == self.generation && self.vm.balance.running.is_some() {
                    self.vm.balance.running = Some((done, total));
                }
            }
            Event::Balanced { r#gen, handicap, stats } => {
                if r#gen != self.generation || self.vm.balance.running.is_none() {
                    return;
                }
                self.vm.balance.running = None;
                if handicap != self.vm.draft.handicap {
                    // A new generation, whose numbers these are.
                    self.vm.draft.handicap = handicap;
                    self.settle(fx);
                }
                self.numbers(stats);
            }
            Event::Measured { r#gen, stats } => {
                if r#gen == self.generation {
                    self.numbers(stats);
                }
            }
            Event::Shortlist { r#gen, list } => {
                if r#gen == self.generation {
                    self.vm.shortlist = list;
                }
            }
            Event::TakePicture { take, story, frames } => {
                if self.wanted != Some(take) {
                    return;
                }
                let at = (story.t_fight * f64::from(FPS)).round();
                let start = if at.is_finite() && at > 0.0 {
                    (at as u32).min(NF - 1)
                } else {
                    0
                };
                fx.push(Effect::Player(PlayerCmd::Load {
                    take,
                    frames,
                    start,
                    autoplay: self.vm.view == View::Step(Step::Seeds),
                }));
                self.say(format!("Seed {} is ready", story.seed));
                self.vm.take.story = Some(story);
                self.vm.take.loading = false;
            }
            Event::TakeSound { take, audio, music } => {
                if self.wanted != Some(take) {
                    return;
                }
                self.vm.take.music = Some(music);
                fx.push(Effect::Player(PlayerCmd::Sound { take, audio }));
            }
            Event::TakeFailed { take, problem } => {
                if self.wanted != Some(take) {
                    return;
                }
                self.vm.take.story = None;
                self.vm.take.music = None;
                self.vm.take.loading = false;
                self.say(problem.clone());
                self.vm.take.problem = Some(problem);
                fx.push(self.card_or_clear());
            }
            // The queue, the output folder, the library and opened fights: Task 12 fills these arms.
            Event::Enqueued { .. } => {}
            Event::EnqueueFailed { .. } => {}
            Event::Job { .. } => {}
            Event::Library { .. } => {}
            Event::LibraryCover { .. } => {}
            Event::FightOpened { .. } => {}
            Event::Tools { .. } => {}
            Event::Folder { .. } => {}
            Event::FolderPicked { .. } => {}
            Event::Problem { .. } => {}
        }
    }

    /// The roster arrived: the draft starts from its starting pair. `settle` then begins the generation.
    fn on_roster(&mut self, roster: Roster) {
        let theme = roster.themes.first();
        let bars = theme.map_or(34, |t| 34u32.max(t.bars.0).min(t.bars.1));
        self.vm.draft = Draft {
            fighters: roster.start.clone(),
            theme: theme.map(|t| t.name.clone()).unwrap_or_default(),
            bars,
            handicap: [1.0, 1.0],
            seed: None,
            chord_loop: 0,
            bass: None,
            music_seed: 1,
            captions: self.vm.draft.captions.clone(),
            captions_edited: false,
            base: None,
        };
        self.vm.roster = Some(roster);
        self.vm.draft.bass = self.first_bass();
    }
}
