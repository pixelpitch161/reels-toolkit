//! The draft and what feeds the fight: the fighters, the tempo, the handicap and the music settings, the balance
//! run, the captions, and the one place where a new generation begins (`AppState::settle`).

use super::{AppState, BalanceView, Effect, FightKey, MusicKey, TakeView, idle_shortlist};
use crate::model::Platform;
use crate::model::balance::BalanceStats;
use crate::model::draft::Draft;
use crate::model::shortlist::SearchState;
use crate::model::take::PlayerCmd;
use crate::model::{Step, View};
use crate::seam::Command;

/// The lowest and highest damage multiplier.
const HANDICAP: (f64, f64) = (0.05, 12.0);

impl AppState {
    /// The one place every action and event goes through, at its end (and wherever an action must ask the engine
    /// something about the fight it has just made). When what feeds the fight differs from what fed the current
    /// generation, a new generation begins. When only the music settings changed and a seed is chosen, a new take
    /// of that seed is asked for. Calling it again with nothing changed does nothing.
    pub(crate) fn settle(&mut self, fx: &mut Vec<Effect>) {
        let Some(key) = self.fight_key() else {
            return;
        };
        if self.fed.as_ref() != Some(&key) {
            let fighters_changed = self.fed.as_ref().is_none_or(|fed| fed.fighters != key.fighters);
            self.fed = Some(key);
            self.begin_generation(fighters_changed, None, fx);
        }
        let music = self.music_key();
        if self.music.as_ref() != Some(&music) {
            let had = self.music.replace(music).is_some();
            if had && self.vm.draft.seed.is_some() {
                self.ask_take(fx);
            }
        }
    }

    /// A whole new draft (the roster's starting draft, or a fight file that was opened): always a new generation,
    /// as if the fighters had changed, even when it feeds the same fight as the draft before, so the fighters are
    /// described again and everything found or asked for before is dropped. The draft's own seed is kept; the
    /// caller asks for its take (`ask_take`) when it has one. Task 12 calls this for `Event::FightOpened`.
    pub(super) fn open_draft(&mut self, draft: Draft, fx: &mut Vec<Effect>) {
        let seed = draft.seed;
        self.vm.draft = draft;
        self.fed = self.fight_key();
        if self.fed.is_none() {
            return;
        }
        self.begin_generation(true, seed, fx);
        // The take, if any, is the caller's to ask for, so `settle` must not ask for one as a music change.
        self.music = Some(self.music_key());
    }

    /// What feeds the fight (§4): the two fighters in order, the bars and the handicap; and, when either fighter's
    /// notes are its weapon, the theme (it gives the key and the chord loops), the chord loop and the music seed
    /// (ruling M2-R14). `None` until the roster arrives.
    pub(crate) fn fight_key(&self) -> Option<FightKey> {
        self.vm.roster.as_ref()?;
        let d = &self.vm.draft;
        let notes_are_a_weapon = (0..2).any(|side| self.vm.fighter(side).is_some_and(|f| f.music_feeds_fight));
        Some(FightKey {
            fighters: d.fighters.clone(),
            bars: d.bars,
            handicap: d.handicap,
            music: notes_are_a_weapon.then(|| (d.theme.clone(), d.chord_loop, d.music_seed)),
        })
    }

    /// The music settings a take is made with.
    pub(crate) fn music_key(&self) -> MusicKey {
        let d = &self.vm.draft;
        MusicKey {
            theme: d.theme.clone(),
            bass: d.bass,
            chord_loop: d.chord_loop,
            music_seed: d.music_seed,
        }
    }

    /// The fight changed: forget everything found for the old one and stop what is still working on it. The draft's
    /// seed becomes `seed` (`None` unless a whole draft was opened).
    fn begin_generation(&mut self, fighters_changed: bool, seed: Option<u64>, fx: &mut Vec<Effect>) {
        self.generation += 1;
        self.stop_running(fx);
        self.vm.draft.seed = seed;
        self.vm.shortlist = idle_shortlist(self.vm.settings.workers);
        self.vm.balance = BalanceView::default();
        self.measuring = false;
        self.balance_stopped = false;
        self.vm.take = TakeView::default();
        self.wanted = None;
        if fighters_changed {
            // The pair rules, the problem and the title card were the other pair's (or the other order's).
            self.title_card = None;
            self.vm.pair_rules.clear();
            self.vm.pair_music = None;
            self.vm.fight_problem = None;
            self.described = self.generation;
        }
        fx.push(self.card_or_clear());
        if fighters_changed {
            fx.push(Effect::Send(Command::DescribeFight {
                r#gen: self.generation,
                draft: self.vm.draft.clone(),
            }));
        }
        if self.vm.view == View::Step(Step::Seeds) {
            // §4: the search restarts. On any other view it restarts when Seeds is next shown.
            self.start_search(fx);
        }
    }

    /// Stops a running search and a running balance, and marks them stopped.
    pub(super) fn stop_running(&mut self, fx: &mut Vec<Effect>) {
        if self.vm.shortlist.state == SearchState::Searching {
            self.vm.shortlist.state = SearchState::Stopped;
            fx.push(Effect::Send(Command::StopSearch));
        }
        if self.vm.balance.running.take().is_some() {
            fx.push(Effect::Send(Command::StopBalance));
        }
        self.measuring = false;
    }

    /// The title card when there is one, else an empty preview.
    pub(crate) fn card_or_clear(&self) -> Effect {
        Effect::Player(match &self.title_card {
            Some(card) => PlayerCmd::Still(card.clone()),
            None => PlayerCmd::Clear,
        })
    }

    /// The engine may be asked about the fight: the roster is there and nothing says the pair cannot fight.
    pub(crate) fn can_ask(&self) -> bool {
        self.fed.is_some() && self.vm.fight_problem.is_none()
    }

    /// The first of the two fighters whose part can play the bass line.
    pub(crate) fn first_bass(&self) -> Option<usize> {
        (0..2).find(|&side| {
            self.vm
                .fighter(side)
                .and_then(|f| f.card.as_ref())
                .is_some_and(|card| card.can_play_bass)
        })
    }

    pub(super) fn pick_fighter(&mut self, side: usize, name: &str) {
        if side > 1 {
            return;
        }
        let known = self
            .vm
            .roster
            .as_ref()
            .is_some_and(|r| r.fighters.iter().any(|f| f.name == name && f.available));
        if !known || self.vm.draft.fighters[side] == name {
            return;
        }
        if self.vm.draft.fighters[1 - side] == name {
            self.swap_sides();
            return;
        }
        let d = &mut self.vm.draft;
        d.fighters[side] = name.to_string();
        d.handicap = [1.0, 1.0];
        d.base = None;
        d.captions_edited = false;
        self.vm.draft.bass = self.first_bass();
        self.start_pair();
    }

    /// The draft's two fighters begin a new match-up from their starter (`Roster::start_of`, the engine's own
    /// rule): the kit pair's handicap (ruling M3-R6), the fighter whose part is the bass on bass, and, while the
    /// draft is in the starter's theme, the starter's chord loop. When the draft's theme cannot be played by the
    /// two (ruling M3-R2), the draft moves to the starter's theme, and a notice says so. A pair the roster knows no
    /// start for keeps the even handicap and the first fighter that can play bass, and moves to the first theme it
    /// can play if it must.
    pub(super) fn start_pair(&mut self) {
        let Some(roster) = self.vm.roster.as_ref() else {
            return;
        };
        let fighters = self.vm.draft.fighters.clone();
        let start = roster.start_of(&fighters);
        // What was said about the other pair's music is not this pair's.
        self.vm.pair_music = None;
        let offered: Vec<String> = roster.themes_for(&fighters).iter().map(|t| t.name.clone()).collect();
        let old = self.vm.draft.theme.clone();
        let mut notice = None;
        if !offered.contains(&old) {
            let new = start
                .as_ref()
                .map(|s| s.theme.clone())
                .filter(|t| offered.contains(t))
                .or_else(|| offered.first().cloned());
            if let Some(new) = new {
                let display = |name: &str| {
                    roster
                        .themes
                        .iter()
                        .find(|t| t.name == name)
                        .map_or_else(|| name.to_string(), |t| t.display.clone())
                };
                let refused: Vec<String> = roster
                    .themes
                    .iter()
                    .find(|t| t.name == old)
                    .map(|t| {
                        fighters
                            .iter()
                            .filter(|f| t.refuses.contains(f))
                            .map(|f| crate::wording::display_name(f))
                            .collect()
                    })
                    .unwrap_or_default();
                let who = if refused.is_empty() {
                    crate::wording::versus(&fighters)
                } else {
                    refused.join(" and ")
                };
                notice = Some(format!(
                    "{who} cannot play in {}, so the theme is now {}.",
                    display(&old),
                    display(&new)
                ));
                self.set_theme(&new);
            }
        }
        if let Some(start) = start {
            let d = &mut self.vm.draft;
            d.handicap = start.handicap;
            if let Some(side) = start.roles.iter().position(|r| r == crate::model::roster::BASS_ROLE) {
                d.bass = Some(side);
            } else {
                d.bass = None;
            }
            if d.theme == start.theme {
                d.chord_loop = start.chord_loop;
            }
        }
        if let Some(text) = notice {
            self.vm.notices.push(crate::model::Notice { text });
        }
    }

    /// The fighters change corners; each keeps its handicap and, if it plays bass, the bass. When each order has a kit
    /// config of its own (HORN against TROMBONE) and the draft is not a file that was opened, the match-up starts from
    /// the new order's config instead (its handicap, parts and loop).
    pub(super) fn swap_sides(&mut self) {
        let d = &mut self.vm.draft;
        d.fighters.swap(0, 1);
        d.handicap.swap(0, 1);
        d.bass = d.bass.map(|side| 1 - side);
        let now = self.vm.draft.fighters.clone();
        let before = [now[1].clone(), now[0].clone()];
        // both orders written apart: the other order is another kit config, not this one swapped
        let own = self
            .vm
            .roster
            .as_ref()
            .is_some_and(|r| r.has_own_config(&now) && r.has_own_config(&before));
        if own && self.vm.draft.base.is_none() {
            self.start_pair();
        }
    }

    pub(super) fn set_theme(&mut self, name: &str) {
        let Some(theme) = self
            .vm
            .roster
            .as_ref()
            .and_then(|r| r.themes.iter().find(|t| t.name == name))
        else {
            return;
        };
        let (bars, loops) = (theme.bars, theme.chord_loops.len());
        // Back in the fight file's own theme, its own chord loop is the one chosen.
        let own = self.vm.pair_music.as_ref().filter(|m| m.theme == name).map(|m| {
            theme
                .chord_loops
                .iter()
                .position(|l| *l == m.chord_loop)
                .unwrap_or(loops)
        });
        let d = &mut self.vm.draft;
        let other = d.theme != name;
        d.theme = name.to_string();
        d.bars = d.bars.max(bars.0).min(bars.1);
        if let Some(own) = own {
            d.chord_loop = own;
        } else if other || d.chord_loop >= loops {
            // a new theme starts on its own first loop
            d.chord_loop = 0;
        }
    }

    /// The tempo, kept inside the theme's range.
    pub(super) fn set_bars(&mut self, bars: u32) {
        let range = self.vm.theme().map(|t| t.bars);
        self.vm.draft.bars = match range {
            Some((lo, hi)) => bars.max(lo).min(hi),
            None => bars,
        };
    }

    /// One fighter's damage multiplier, kept in 0.05 to 12.0 and rounded to two decimals; measured on release.
    pub(super) fn set_handicap(&mut self, side: usize, value: f64, released: bool, fx: &mut Vec<Effect>) {
        if side > 1 || !value.is_finite() {
            return;
        }
        self.vm.draft.handicap[side] = (value.clamp(HANDICAP.0, HANDICAP.1) * 100.0).round() / 100.0;
        if released {
            self.settle(fx);
            self.measure(fx);
        }
    }

    /// The kit pair's own handicap again (ruling M3-R6); nothing for a pair with no kit config.
    pub(super) fn use_series_handicap(&mut self, fx: &mut Vec<Effect>) {
        let Some(series) = self
            .vm
            .roster
            .as_ref()
            .and_then(|r| r.series_handicap(&self.vm.draft.fighters))
        else {
            return;
        };
        self.vm.draft.handicap = series;
        self.settle(fx);
        self.measure(fx);
    }

    pub(super) fn reset_handicap(&mut self, fx: &mut Vec<Effect>) {
        self.vm.draft.handicap = [1.0, 1.0];
        self.settle(fx);
        self.measure(fx);
    }

    /// The Balance step opens: with no numbers and nothing running, balance an even fight, measure any other; but
    /// not when the owner stopped a balance run in this generation.
    pub(super) fn open_balance(&mut self, fx: &mut Vec<Effect>) {
        let b = &self.vm.balance;
        if b.stats.is_some() || b.running.is_some() || self.measuring || self.balance_stopped {
            return;
        }
        if self.vm.draft.handicap == [1.0, 1.0] {
            self.auto_balance(fx);
        } else {
            self.measure(fx);
        }
    }

    fn measure(&mut self, fx: &mut Vec<Effect>) {
        if !self.can_ask() {
            return;
        }
        self.measuring = true;
        fx.push(Effect::Send(Command::Measure {
            r#gen: self.generation,
            draft: self.vm.draft.clone(),
        }));
    }

    pub(super) fn auto_balance(&mut self, fx: &mut Vec<Effect>) {
        if !self.can_ask() || self.vm.balance.running.is_some() {
            return;
        }
        self.vm.balance.running = Some((0, 0));
        fx.push(Effect::Send(Command::AutoBalance {
            r#gen: self.generation,
            draft: self.vm.draft.clone(),
        }));
    }

    /// Stops a balance run; its progress and result, if still on their way, are then ignored. Opening Balance again
    /// in this generation does not start another by itself (ruling M2-R16).
    pub(super) fn stop_balance(&mut self, fx: &mut Vec<Effect>) {
        if self.vm.balance.running.take().is_some() {
            self.balance_stopped = true;
            fx.push(Effect::Send(Command::StopBalance));
        }
    }

    /// A balance run or a measurement came back with numbers for the current fight.
    pub(super) fn numbers(&mut self, stats: BalanceStats) {
        self.measuring = false;
        self.vm.balance.stats = Some(stats);
    }

    pub(super) fn set_chord_loop(&mut self, i: usize) {
        if self.vm.theme().is_some_and(|t| i < t.chord_loops.len()) {
            self.vm.draft.chord_loop = i;
        }
    }

    /// Who plays bass: only a fighter whose part can.
    pub(super) fn set_bass(&mut self, side: usize) {
        let can = self
            .vm
            .fighter(side)
            .and_then(|f| f.card.as_ref())
            .is_some_and(|card| card.can_play_bass);
        if can {
            self.vm.draft.bass = Some(side);
        }
    }

    /// The owner's own caption. The fight file has one caption line, so the Instagram and TikTok texts share their
    /// first line: editing either one's first line rewrites the other's, and each keeps its own tags.
    pub(super) fn set_caption(&mut self, platform: Platform, text: String) {
        let c = &mut self.vm.draft.captions;
        match platform {
            Platform::Instagram => {
                c.tiktok = with_first_line(&c.tiktok, first_line(&text));
                c.instagram = text;
            }
            Platform::TikTok => {
                c.instagram = with_first_line(&c.instagram, first_line(&text));
                c.tiktok = text;
            }
            Platform::Shorts => c.shorts = text,
        }
        self.vm.draft.captions_edited = true;
    }
}

fn first_line(text: &str) -> &str {
    text.split('\n').next().unwrap_or("")
}

/// `text` with its first line replaced by `line`.
fn with_first_line(text: &str, line: &str) -> String {
    match text.find('\n') {
        Some(i) => format!("{line}{}", &text[i..]),
        None => line.to_string(),
    }
}
