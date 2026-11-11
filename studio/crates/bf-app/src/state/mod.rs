//! The app's state: what the screens show (the [`ViewModel`]), what the owner does ([`Action`]) and what the state
//! asks of the engine, the preview player and the store in return ([`Effect`]).
//!
//! [`AppState::dispatch`] takes an action and [`AppState::apply`] an engine event; both only change values and
//! return effects, so the interface thread never waits. Both end in one place, `AppState::settle` (in `fight.rs`),
//! which compares what feeds the fight with what fed the current generation: when they differ a new generation
//! begins, and every answer still on its way for the old one is dropped when it lands (Review Focus 3).

pub mod events;
pub mod fight;
pub mod queue_run;
pub mod seeds;

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::model::balance::BalanceStats;
use crate::model::draft::{CaptionTexts, Draft};
use crate::model::library::{FolderState, LibraryEntry};
use crate::model::queue::Queue;
use crate::model::roster::{Roster, RosterEntry, ThemeEntry};
use crate::model::shortlist::{FilterSpec, SearchState, Shortlist};
use crate::model::story::StoryView;
use crate::model::take::{Frame, MusicView, PlayerCmd, SoundState, TransportAction, TransportView};
use crate::model::{Gen, JobId, Notice, Platform, Settings, Step, TakeId, View};
use crate::seam::{Command, Event};
use crate::wording;

/// The Balance screen's numbers, or how far a balance run has got.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BalanceView {
    pub stats: Option<BalanceStats>,
    /// A balance run in progress: steps done, of total (`(0, 0)` until the first progress arrives).
    pub running: Option<(u32, u32)>,
}

/// The take of the chosen seed: its story once the picture is ready, its music once the sound is.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TakeView {
    pub story: Option<StoryView>,
    pub music: Option<MusicView>,
    /// Asked for, and its picture has not arrived yet.
    pub loading: bool,
    /// Why this take cannot be made, in words.
    pub problem: Option<String>,
}

/// The Library: the videos found under the output folder and their covers.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LibraryView {
    pub entries: Vec<LibraryEntry>,
    pub covers: BTreeMap<PathBuf, Frame>,
    pub selected: Option<PathBuf>,
    pub scanning: bool,
    pub problem: Option<String>,
}

/// Everything the screens draw from.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewModel {
    pub view: View,
    /// The step last shown: the one the toolbar marks while the Library or Settings is open.
    pub step: Step,
    pub roster: Option<Roster>,
    pub draft: Draft,
    /// The pair rules switched on for the two fighters, in words.
    pub pair_rules: Vec<String>,
    /// Why these two cannot fight, in words.
    pub fight_problem: Option<String>,
    pub balance: BalanceView,
    pub filters: FilterSpec,
    pub shortlist: Shortlist,
    pub take: TakeView,
    pub transport: TransportView,
    pub caption_tab: Platform,
    pub folder: FolderState,
    /// ffmpeg's description, or why it was not found; `None` until the engine answers.
    pub ffmpeg: Option<Result<String, String>>,
    pub queue: Queue,
    pub selected_job: Option<JobId>,
    pub library: LibraryView,
    pub settings: Settings,
    /// The PC's cores.
    pub cores: usize,
    pub notices: Vec<Notice>,
    /// The left side of the status bar: what the app is doing or last did.
    pub status: String,
}

/// A neutral grey for a fighter the roster does not know (yet).
const NO_COLOR: [u8; 3] = [0x80, 0x80, 0x80];

impl ViewModel {
    /// The roster's entry for the fighter in corner `side` (0 left, 1 right).
    pub fn fighter(&self, side: usize) -> Option<&RosterEntry> {
        let name = self.draft.fighters.get(side)?;
        self.roster.as_ref()?.fighters.iter().find(|f| &f.name == name)
    }

    /// The two fighters' colours, left then right.
    pub fn colors(&self) -> [[u8; 3]; 2] {
        [0, 1].map(|side| self.fighter(side).map_or(NO_COLOR, |f| f.color))
    }

    /// The draft's theme.
    pub fn theme(&self) -> Option<&ThemeEntry> {
        self.roster.as_ref()?.themes.iter().find(|t| t.name == self.draft.theme)
    }

    /// The two fighters' names as shown: "Flute", "Trombone".
    pub fn names(&self) -> [String; 2] {
        [0, 1].map(|side| {
            self.fighter(side).map_or_else(
                || wording::display_name(&self.draft.fighters[side]),
                |f| f.display.clone(),
            )
        })
    }
}

/// What the owner does with the render queue.
#[derive(Clone, Debug, PartialEq)]
pub enum QueueAction {
    Pause,
    Resume,
    Cancel(JobId),
    Remove(JobId),
    Move { id: JobId, to: usize },
    Requeue(JobId),
    Select(JobId),
    OpenFolder(JobId),
    OpenIn { id: JobId, step: Step },
}

/// What the owner does.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Go(View),
    PickFighter {
        side: usize,
        name: String,
    },
    SwapSides,
    SetTheme(String),
    SetBars(u32),
    SetHandicap {
        side: usize,
        value: f64,
        released: bool,
    },
    ResetHandicap,
    AutoBalance,
    StopBalance,
    SetFilters(FilterSpec),
    StopSearch,
    /// Keep searching after a stop.
    SearchOn,
    SelectSeed(u64),
    UseSeed,
    SetChordLoop(usize),
    SetBass(usize),
    SetCaptionTab(Platform),
    SetCaption {
        platform: Platform,
        text: String,
    },
    ChangeFolder,
    CreateFolder,
    AddToQueue,
    Queue(QueueAction),
    Transport(TransportAction),
    RefreshLibrary,
    SelectVideo(PathBuf),
    Reopen(PathBuf),
    OpenVideoFolder(PathBuf),
    SetWorkers(usize),
    Dismiss(usize),
}

/// What the state asks for in return.
#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "a few effects per action, handled at once; `Send(Command)` is the shape the screens and the shell use"
)]
pub enum Effect {
    /// A command for the engine.
    Send(Command),
    /// A command for the preview player.
    Player(PlayerCmd),
    SaveSettings(Settings),
    SaveQueue(Queue),
}

/// What feeds the fight (§4): an answer belongs to the current fight only while this stays the same.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FightKey {
    pub fighters: [String; 2],
    pub bars: u32,
    pub handicap: [f64; 2],
    /// The chord loop and the music seed, when either fighter's notes are its weapon.
    pub music: Option<(usize, u64)>,
}

/// The music settings a take is made with: changing one asks for a new take of the same seed.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MusicKey {
    pub theme: String,
    pub bass: Option<usize>,
    pub chord_loop: usize,
    pub music_seed: u64,
}

/// The app's state: the view model plus what the screens never show.
#[derive(Debug)]
pub struct AppState {
    pub vm: ViewModel,
    /// The current generation; 0 until the roster arrives.
    generation: Gen,
    /// What fed the current generation; `None` until the roster arrives.
    fed: Option<FightKey>,
    /// The music settings the draft had when last settled.
    music: Option<MusicKey>,
    /// The take whose answers are wanted; any other take's are late.
    wanted: Option<TakeId>,
    /// The last take number given out.
    last_take: TakeId,
    /// The current pair's VS title card, shown while there is no take.
    title_card: Option<Frame>,
    /// A measurement was asked for in this generation and has not come back.
    measuring: bool,
    /// The generation the current fighters were described for (`DescribeFight`): a description for it or any
    /// later generation is of these fighters.
    described: Gen,
}

impl AppState {
    /// A new app: Match-up, waiting for the roster. Asks for the roster, ffmpeg and the output folder.
    pub fn new(settings: Settings, queue: Queue, notices: Vec<Notice>, cores: usize) -> (AppState, Vec<Effect>) {
        let effects = vec![
            Effect::Send(Command::LoadRoster),
            Effect::Send(Command::CheckTools),
            Effect::Send(Command::CheckFolder {
                root: settings.output_folder.clone(),
                name: String::new(),
            }),
        ];
        let vm = ViewModel {
            view: View::Step(Step::Matchup),
            step: Step::Matchup,
            roster: None,
            draft: empty_draft(),
            pair_rules: Vec::new(),
            fight_problem: None,
            balance: BalanceView::default(),
            filters: FilterSpec::default(),
            shortlist: idle_shortlist(settings.workers),
            take: TakeView::default(),
            transport: TransportView {
                enabled: false,
                frame: 0,
                playing: false,
                looping: true,
                safe_zone: false,
                sound: SoundState::None,
            },
            caption_tab: Platform::Instagram,
            folder: FolderState::Unknown,
            ffmpeg: None,
            queue,
            selected_job: None,
            library: LibraryView::default(),
            settings,
            cores,
            notices,
            status: String::new(),
        };
        (AppState::bare(vm), effects)
    }

    /// A state that shows `vm` as it is, for fixtures and screen tests. Its fight is generation 1 (0 without a
    /// roster), so the next change of what feeds it is generation 2; when it has a seed, its take is take 1.
    pub fn showing(vm: ViewModel) -> AppState {
        let mut state = AppState::bare(vm);
        state.fed = state.fight_key();
        if state.fed.is_some() {
            state.generation = 1;
            state.described = 1;
            state.music = Some(state.music_key());
        }
        if state.vm.draft.seed.is_some() {
            state.last_take = 1;
            state.wanted = Some(1);
        }
        state
    }

    fn bare(vm: ViewModel) -> AppState {
        AppState {
            vm,
            generation: 0,
            fed: None,
            music: None,
            wanted: None,
            last_take: 0,
            title_card: None,
            measuring: false,
            described: 0,
        }
    }

    /// The current generation.
    pub fn generation(&self) -> Gen {
        self.generation
    }

    /// Takes one thing the owner did.
    pub fn dispatch(&mut self, action: Action) -> Vec<Effect> {
        let mut fx = Vec::new();
        match action {
            Action::Go(view) => self.go(view, &mut fx),
            Action::PickFighter { side, name } => self.pick_fighter(side, &name),
            Action::SwapSides => self.swap_sides(),
            Action::SetTheme(name) => self.set_theme(&name),
            Action::SetBars(bars) => self.set_bars(bars),
            Action::SetHandicap { side, value, released } => self.set_handicap(side, value, released, &mut fx),
            Action::ResetHandicap => self.reset_handicap(&mut fx),
            Action::AutoBalance => self.auto_balance(&mut fx),
            Action::StopBalance => self.stop_balance(&mut fx),
            Action::SetFilters(filters) => self.set_filters(filters, &mut fx),
            Action::StopSearch => self.stop_search(&mut fx),
            Action::SearchOn => self.start_search(&mut fx),
            Action::SelectSeed(seed) => self.select_seed(seed, &mut fx),
            Action::UseSeed => self.use_seed(),
            Action::SetChordLoop(i) => self.set_chord_loop(i),
            Action::SetBass(side) => self.set_bass(side),
            Action::SetCaptionTab(platform) => self.vm.caption_tab = platform,
            Action::SetCaption { platform, text } => self.set_caption(platform, text),
            Action::Transport(a) => fx.push(Effect::Player(PlayerCmd::Transport(a))),
            // The output folder, the queue, the library and the settings: Task 12 fills these arms.
            Action::ChangeFolder => {}
            Action::CreateFolder => {}
            Action::AddToQueue => {}
            Action::Queue(_) => {}
            Action::RefreshLibrary => {}
            Action::SelectVideo(_) => {}
            Action::Reopen(_) => {}
            Action::OpenVideoFolder(_) => {}
            Action::SetWorkers(_) => {}
            Action::Dismiss(_) => {}
        }
        self.settle(&mut fx);
        fx
    }

    /// Takes one event from the engine.
    pub fn apply(&mut self, event: Event) -> Vec<Effect> {
        let mut fx = Vec::new();
        self.on_event(event, &mut fx);
        self.settle(&mut fx);
        fx
    }

    /// Shows a step, the Library or Settings. Opening Balance with nothing measured balances or measures; opening
    /// Seeds with no search asked for starts one.
    fn go(&mut self, view: View, fx: &mut Vec<Effect>) {
        self.vm.view = view;
        match view {
            View::Step(step) => {
                self.vm.step = step;
                match step {
                    Step::Balance => self.open_balance(fx),
                    Step::Seeds if self.vm.shortlist.state == SearchState::Idle => self.start_search(fx),
                    _ => {}
                }
            }
            // Task 12 adds the library scan here.
            View::Library => {}
            View::Settings => {}
        }
    }

    /// Sets the status line, unless the pair cannot fight: then the status line says why.
    pub(crate) fn say(&mut self, text: String) {
        self.vm.status = self.vm.fight_problem.clone().unwrap_or(text);
    }
}

/// The draft before the roster arrives: nothing chosen yet.
fn empty_draft() -> Draft {
    Draft {
        fighters: [String::new(), String::new()],
        theme: String::new(),
        bars: 34,
        handicap: [1.0, 1.0],
        seed: None,
        chord_loop: 0,
        bass: None,
        music_seed: 1,
        captions: CaptionTexts {
            instagram: String::new(),
            tiktok: String::new(),
            shorts: String::new(),
        },
        captions_edited: false,
        base: None,
    }
}

/// A shortlist with nothing in it and no search asked for.
pub(crate) fn idle_shortlist(workers: usize) -> Shortlist {
    Shortlist {
        rows: Vec::new(),
        fits_total: 0,
        checked: 0,
        state: SearchState::Idle,
        workers,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::model::shortlist::Finish;
    use crate::model::take::AudioTake;
    use crate::seam::sample;

    fn settings() -> Settings {
        Settings {
            output_folder: PathBuf::from("Videos").join("BALLFIGHT"),
            workers: 7,
        }
    }

    fn queue() -> Queue {
        Queue {
            jobs: Vec::new(),
            paused: false,
            next_id: 1,
        }
    }

    /// The commands among `effects`, in order.
    fn sent(effects: &[Effect]) -> Vec<Command> {
        effects
            .iter()
            .filter_map(|e| match e {
                Effect::Send(c) => Some(c.clone()),
                _ => None,
            })
            .collect()
    }

    /// The player commands among `effects`, in order.
    fn played(effects: &[Effect]) -> Vec<&PlayerCmd> {
        effects
            .iter()
            .filter_map(|e| match e {
                Effect::Player(p) => Some(p),
                _ => None,
            })
            .collect()
    }

    fn count(effects: &[Effect], what: fn(&Command) -> bool) -> usize {
        sent(effects).iter().filter(|c| what(c)).count()
    }

    fn is_measure(c: &Command) -> bool {
        matches!(c, Command::Measure { .. })
    }

    fn is_auto_balance(c: &Command) -> bool {
        matches!(c, Command::AutoBalance { .. })
    }

    fn is_start_search(c: &Command) -> bool {
        matches!(c, Command::StartSearch { .. })
    }

    fn is_describe(c: &Command) -> bool {
        matches!(c, Command::DescribeFight { .. })
    }

    /// A new app that has its roster: FLUTE vs TROMBONE, generation 1.
    fn ready() -> AppState {
        let (mut s, _) = AppState::new(settings(), queue(), Vec::new(), 8);
        s.apply(Event::Roster(sample::roster()));
        s
    }

    fn go(s: &mut AppState, step: Step) -> Vec<Effect> {
        s.dispatch(Action::Go(View::Step(step)))
    }

    fn handicap(side: usize, value: f64, released: bool) -> Action {
        Action::SetHandicap { side, value, released }
    }

    fn pick(side: usize, name: &str) -> Action {
        Action::PickFighter {
            side,
            name: name.to_string(),
        }
    }

    fn fight_info(generation: Gen, captions: CaptionTexts) -> Event {
        Event::FightInfo {
            r#gen: generation,
            pair_rules: vec!["A live lance knocks a risen cobra flat.".to_string()],
            captions,
            title_card: None,
            problem: None,
        }
    }

    fn sample_list() -> Shortlist {
        Shortlist {
            rows: sample::stories(),
            fits_total: 212,
            checked: 18_240,
            state: SearchState::Searching,
            workers: 7,
        }
    }

    fn story_with(seed: u64, t_fight: f64, winner: Option<usize>) -> StoryView {
        StoryView {
            t_fight,
            winner,
            ..sample::story(seed)
        }
    }

    fn picture(take: TakeId, story: StoryView) -> Event {
        Event::TakePicture {
            take,
            story,
            frames: sample::frames([[0xde, 0xe8, 0xf6], [0x78, 0xf0, 0xb8]]),
        }
    }

    fn quiet() -> AudioTake {
        AudioTake {
            samples: Arc::from(vec![[0.0f32; 2]; 4]),
        }
    }

    /// The take number the last `LoadTake` among `effects` asked for.
    fn take_of(effects: &[Effect]) -> TakeId {
        sent(effects)
            .iter()
            .rev()
            .find_map(|c| match c {
                Command::LoadTake { take, .. } => Some(*take),
                _ => None,
            })
            .expect("a LoadTake")
    }

    #[test]
    fn starts_by_asking_for_the_roster() {
        let (s, fx) = AppState::new(settings(), queue(), Vec::new(), 8);
        assert_eq!(s.vm.view, View::Step(Step::Matchup));
        assert_eq!(
            sent(&fx),
            vec![
                Command::LoadRoster,
                Command::CheckTools,
                Command::CheckFolder {
                    root: settings().output_folder,
                    name: String::new(),
                },
            ]
        );
        assert_eq!(fx.len(), 3);
        assert_eq!(s.generation(), 0);
    }

    #[test]
    fn the_roster_sets_the_starting_pair() {
        let (mut s, _) = AppState::new(settings(), queue(), Vec::new(), 8);
        let fx = s.apply(Event::Roster(sample::roster()));
        let d = &s.vm.draft;
        assert_eq!(d.fighters, ["FLUTE".to_string(), "TROMBONE".to_string()]);
        assert_eq!(d.theme, "big_top");
        assert_eq!(d.bars, 34);
        assert_eq!(d.handicap, [1.0, 1.0]);
        assert_eq!(d.seed, None);
        assert_eq!(d.chord_loop, 0);
        assert_eq!(d.music_seed, 1);
        assert_eq!(d.bass, Some(1));
        assert_eq!(s.generation(), 1);
        assert_eq!(count(&fx, is_describe), 1);
        assert!(matches!(&sent(&fx)[..], [Command::DescribeFight { r#gen: 1, draft }] if draft == &s.vm.draft));
        assert_eq!(s.vm.status, "Flute vs Trombone, not balanced yet");
    }

    #[test]
    fn picking_the_other_corners_fighter_swaps() {
        let mut s = ready();
        s.dispatch(handicap(0, 2.0, false));
        s.dispatch(handicap(1, 0.4, false));
        s.dispatch(pick(0, "TROMBONE"));
        let d = &s.vm.draft;
        assert_eq!(d.fighters, ["TROMBONE".to_string(), "FLUTE".to_string()]);
        assert_eq!(d.handicap, [0.4, 2.0]);
        assert_eq!(d.bass, Some(0));
    }

    #[test]
    fn a_new_fighter_resets_the_handicap_and_the_base() {
        let mut s = ready();
        s.dispatch(handicap(0, 2.0, false));
        s.vm.draft.base = Some("format = 1".to_string());
        s.vm.draft.captions_edited = true;
        let fx = s.dispatch(pick(1, "PIANO"));
        let d = &s.vm.draft;
        assert_eq!(d.fighters, ["FLUTE".to_string(), "PIANO".to_string()]);
        assert_eq!(d.handicap, [1.0, 1.0]);
        assert_eq!(d.base, None);
        assert!(!d.captions_edited);
        assert_eq!(d.bass, None, "neither FLUTE nor PIANO can play bass");
        assert_eq!(count(&fx, is_describe), 1);
        // Picking the fighter already in that corner changes nothing.
        let generation = s.generation();
        assert!(s.dispatch(pick(1, "PIANO")).is_empty());
        assert_eq!(s.generation(), generation);
    }

    #[test]
    fn every_fight_input_starts_a_new_generation() {
        let actions = [
            pick(0, "OBOE"),
            Action::SwapSides,
            Action::SetBars(30),
            handicap(1, 0.7, false),
        ];
        for action in actions {
            let mut s = ready();
            go(&mut s, Step::Seeds);
            s.apply(Event::Shortlist {
                r#gen: s.generation(),
                list: sample_list(),
            });
            s.dispatch(Action::SelectSeed(18433));
            s.vm.balance.stats = Some(sample::stats());
            go(&mut s, Step::Matchup);
            let before = s.generation();
            let fx = s.dispatch(action.clone());
            assert_eq!(s.generation(), before + 1, "{action:?}");
            assert_eq!(s.vm.draft.seed, None, "{action:?}");
            assert!(s.vm.shortlist.rows.is_empty(), "{action:?}");
            assert_eq!(s.vm.shortlist.state, SearchState::Idle, "{action:?}");
            assert_eq!(s.vm.balance.stats, None, "{action:?}");
            assert_eq!(s.vm.take, TakeView::default(), "{action:?}");
            assert_eq!(count(&fx, |c| matches!(c, Command::StopSearch)), 1, "{action:?}");
            assert_eq!(count(&fx, is_start_search), 0, "{action:?}: not on Seeds");
            assert_eq!(played(&fx).len(), 1, "{action:?}");
            let fighters_changed = !matches!(action, Action::SetBars(_) | Action::SetHandicap { .. });
            assert_eq!(count(&fx, is_describe), usize::from(fighters_changed), "{action:?}");
        }
    }

    #[test]
    fn music_settings_keep_the_generation() {
        let mut s = ready();
        let before = s.generation();
        for action in [
            Action::SetTheme("big_top".to_string()),
            Action::SetBass(1),
            Action::SetChordLoop(1),
        ] {
            let fx = s.dispatch(action.clone());
            assert_eq!(s.generation(), before, "{action:?}");
            assert!(sent(&fx).is_empty(), "{action:?}");
        }
        assert_eq!(s.vm.draft.chord_loop, 1);

        // With a seed chosen, a music setting asks for a new take of the same seed.
        s.dispatch(Action::SelectSeed(18433));
        let fx = s.dispatch(Action::SetChordLoop(0));
        assert_eq!(s.generation(), before);
        assert_eq!(s.vm.draft.seed, Some(18433));
        assert!(matches!(
            &sent(&fx)[..],
            [Command::LoadTake { take: 2, draft }] if draft.seed == Some(18433) && draft.chord_loop == 0
        ));
    }

    #[test]
    fn notes_that_are_a_weapon_feed_the_fight() {
        let mut s = ready();
        s.dispatch(pick(1, "PIANO"));
        let before = s.generation();
        s.dispatch(Action::SetChordLoop(1));
        assert_eq!(s.generation(), before + 1);
        assert_eq!(s.vm.draft.chord_loop, 1);
    }

    #[test]
    fn late_results_are_dropped() {
        let mut s = ready();
        go(&mut s, Step::Seeds);
        let first = take_of(&s.dispatch(Action::SelectSeed(20418)));
        let second = take_of(&s.dispatch(Action::SelectSeed(7731)));
        assert_ne!(first, second);
        let old_take = [
            picture(first, sample::story(20418)),
            Event::TakeSound {
                take: first,
                audio: quiet(),
                music: sample::music(),
            },
            Event::TakeFailed {
                take: first,
                problem: "Too late.".to_string(),
            },
        ];
        for event in old_take {
            let before = s.vm.clone();
            let shown = format!("{event:?}");
            let fx = s.apply(event);
            assert_eq!(s.vm, before, "{shown}");
            assert!(fx.is_empty(), "{shown}");
        }

        let old = s.generation();
        s.dispatch(Action::SwapSides);
        assert_ne!(s.generation(), old);
        s.dispatch(Action::AutoBalance);
        let old_fight = [
            fight_info(old, sample::captions()),
            Event::BalanceProgress {
                r#gen: old,
                done: 3,
                total: 29,
            },
            Event::Balanced {
                r#gen: old,
                handicap: [2.0, 0.4],
                stats: sample::stats(),
            },
            Event::Measured {
                r#gen: old,
                stats: sample::stats(),
            },
            Event::Shortlist {
                r#gen: old,
                list: sample_list(),
            },
        ];
        for event in old_fight {
            let before = s.vm.clone();
            let shown = format!("{event:?}");
            let fx = s.apply(event);
            assert_eq!(s.vm, before, "{shown}");
            assert!(fx.is_empty(), "{shown}");
        }
        // After the swap, the second seed's take is late too.
        for event in [
            picture(second, sample::story(7731)),
            Event::TakeSound {
                take: second,
                audio: quiet(),
                music: sample::music(),
            },
            Event::TakeFailed {
                take: second,
                problem: "Too late.".to_string(),
            },
        ] {
            let before = s.vm.clone();
            let shown = format!("{event:?}");
            let fx = s.apply(event);
            assert_eq!(s.vm, before, "{shown}");
            assert!(fx.is_empty(), "{shown}");
        }
    }

    #[test]
    fn releasing_the_handicap_measures() {
        let mut s = ready();
        let fx = s.dispatch(handicap(0, 2.004, false));
        assert!(sent(&fx).is_empty());
        assert_eq!(s.vm.draft.handicap, [2.0, 1.0]);
        let fx = s.dispatch(handicap(0, 2.0, true));
        assert_eq!(sent(&fx).len(), 1);
        assert!(matches!(&sent(&fx)[0], Command::Measure { r#gen, .. } if *r#gen == s.generation()));
        s.dispatch(handicap(1, 99.0, false));
        assert_eq!(s.vm.draft.handicap, [2.0, 12.0]);
        s.dispatch(handicap(1, 0.0, false));
        assert_eq!(s.vm.draft.handicap, [2.0, 0.05]);

        let fx = s.dispatch(Action::ResetHandicap);
        assert_eq!(s.vm.draft.handicap, [1.0, 1.0]);
        assert_eq!(count(&fx, is_measure), 1);
        assert!(matches!(&sent(&fx)[..], [Command::Measure { r#gen, draft }]
            if *r#gen == s.generation() && draft.handicap == [1.0, 1.0]));
    }

    #[test]
    fn opening_balance_balances_once() {
        let mut s = ready();
        let fx = go(&mut s, Step::Balance);
        assert_eq!(s.vm.view, View::Step(Step::Balance));
        assert_eq!(s.vm.step, Step::Balance);
        assert_eq!(count(&fx, is_auto_balance), 1);
        assert_eq!(sent(&fx).len(), 1);
        assert!(s.vm.balance.running.is_some());
        go(&mut s, Step::Matchup);
        assert!(sent(&go(&mut s, Step::Balance)).is_empty());

        // With a handicap of its own the fight is measured, once.
        let mut s = ready();
        s.dispatch(handicap(0, 2.0, false));
        let fx = go(&mut s, Step::Balance);
        assert_eq!(count(&fx, is_measure), 1);
        assert_eq!(sent(&fx).len(), 1);
        go(&mut s, Step::Matchup);
        assert!(sent(&go(&mut s, Step::Balance)).is_empty());

        // With numbers already there, nothing is asked.
        let mut s = ready();
        s.apply(Event::Measured {
            r#gen: s.generation(),
            stats: sample::stats(),
        });
        assert!(sent(&go(&mut s, Step::Balance)).is_empty());
    }

    #[test]
    fn a_balanced_handicap_keeps_its_numbers() {
        let mut s = ready();
        go(&mut s, Step::Balance);
        let old = s.generation();
        s.apply(Event::BalanceProgress {
            r#gen: old,
            done: 9,
            total: 29,
        });
        assert_eq!(s.vm.balance.running, Some((9, 29)));
        let fx = s.apply(Event::Balanced {
            r#gen: old,
            handicap: [2.0, 0.4],
            stats: sample::stats(),
        });
        assert_eq!(s.vm.draft.handicap, [2.0, 0.4]);
        assert_eq!(s.generation(), old + 1);
        assert_eq!(s.vm.balance.stats, Some(sample::stats()));
        assert_eq!(s.vm.balance.running, None);
        assert!(
            sent(&fx).is_empty(),
            "a finished balance is not stopped, and Balance starts no search"
        );
        assert_eq!(s.vm.status, "Balanced on 3,000 fights in 1.4 s");

        // The numbers belong to the new generation: a measurement for the old one is late.
        let before = s.vm.clone();
        s.apply(Event::Measured {
            r#gen: old,
            stats: BalanceStats {
                win_share: 0.9,
                ..sample::stats()
            },
        });
        assert_eq!(s.vm, before);

        // The same handicap keeps the generation.
        let mut s = ready();
        go(&mut s, Step::Balance);
        let generation = s.generation();
        s.apply(Event::Balanced {
            r#gen: generation,
            handicap: [1.0, 1.0],
            stats: sample::stats(),
        });
        assert_eq!(s.generation(), generation);
        assert_eq!(s.vm.balance.stats, Some(sample::stats()));
    }

    #[test]
    fn a_stopped_balance_ignores_what_was_on_its_way() {
        let mut s = ready();
        go(&mut s, Step::Balance);
        let fx = s.dispatch(Action::StopBalance);
        assert!(matches!(&sent(&fx)[..], [Command::StopBalance]));
        assert_eq!(s.vm.balance.running, None);
        let before = s.vm.clone();
        for event in [
            Event::BalanceProgress {
                r#gen: s.generation(),
                done: 10,
                total: 29,
            },
            Event::Balanced {
                r#gen: s.generation(),
                handicap: [2.0, 0.4],
                stats: sample::stats(),
            },
        ] {
            assert!(s.apply(event).is_empty());
            assert_eq!(s.vm, before);
        }
    }

    #[test]
    fn opening_seeds_starts_one_search() {
        let mut s = ready();
        let fx = go(&mut s, Step::Seeds);
        assert_eq!(count(&fx, is_start_search), 1);
        assert_eq!(sent(&fx).len(), 1);
        assert!(matches!(&sent(&fx)[0], Command::StartSearch { r#gen, filters, .. }
            if *r#gen == s.generation() && *filters == FilterSpec::default()));
        assert_eq!(s.vm.shortlist.state, SearchState::Searching);
        assert_eq!(s.vm.status, "Searching on 7 cores");
        assert!(sent(&go(&mut s, Step::Seeds)).is_empty());
        go(&mut s, Step::Music);
        assert!(sent(&go(&mut s, Step::Seeds)).is_empty());

        // The shortlist arrives; stopping and going on.
        s.apply(Event::Shortlist {
            r#gen: s.generation(),
            list: sample_list(),
        });
        assert_eq!(s.vm.shortlist, sample_list());
        let fx = s.dispatch(Action::StopSearch);
        assert!(matches!(&sent(&fx)[..], [Command::StopSearch]));
        assert_eq!(s.vm.shortlist.state, SearchState::Stopped);
        let fx = s.dispatch(Action::SearchOn);
        assert_eq!(count(&fx, is_start_search), 1);
        assert_eq!(s.vm.shortlist.state, SearchState::Searching);
    }

    #[test]
    fn a_new_fight_on_seeds_restarts_the_search() {
        let mut s = ready();
        s.dispatch(Action::AutoBalance);
        go(&mut s, Step::Seeds);
        let old = s.generation();
        let fx = s.apply(Event::Balanced {
            r#gen: old,
            handicap: [2.0, 0.4],
            stats: sample::stats(),
        });
        assert_eq!(count(&fx, is_start_search), 1);
        assert!(
            sent(&fx)
                .iter()
                .any(|c| matches!(c, Command::StartSearch { r#gen, draft, .. }
            if *r#gen == old + 1 && draft.handicap == [2.0, 0.4]))
        );
        assert_eq!(s.vm.shortlist.state, SearchState::Searching);

        let mut s = ready();
        go(&mut s, Step::Balance);
        let fx = s.apply(Event::Balanced {
            r#gen: s.generation(),
            handicap: [2.0, 0.4],
            stats: sample::stats(),
        });
        assert_eq!(count(&fx, is_start_search), 0);
        assert_eq!(s.vm.shortlist.state, SearchState::Idle);
        // It restarts when Seeds is next shown.
        assert_eq!(count(&go(&mut s, Step::Seeds), is_start_search), 1);
    }

    #[test]
    fn choosing_a_seed_asks_for_its_take() {
        let mut s = ready();
        go(&mut s, Step::Seeds);
        let fx = s.dispatch(Action::SelectSeed(18433));
        assert_eq!(s.vm.draft.seed, Some(18433));
        assert!(s.vm.take.loading);
        assert!(matches!(&sent(&fx)[..], [Command::LoadTake { take: 1, draft }] if draft.seed == Some(18433)));
        let fx = s.dispatch(Action::SelectSeed(20418));
        assert!(matches!(&sent(&fx)[..], [Command::LoadTake { take: 2, draft }] if draft.seed == Some(20418)));
    }

    #[test]
    fn the_picture_starts_at_fight() {
        for (step, autoplay) in [(Step::Seeds, true), (Step::Music, false)] {
            let mut s = ready();
            go(&mut s, step);
            let take = take_of(&s.dispatch(Action::SelectSeed(18433)));
            let fx = s.apply(picture(take, story_with(18433, 3.6, Some(1))));
            match &played(&fx)[..] {
                [
                    PlayerCmd::Load {
                        take: t,
                        start,
                        autoplay: a,
                        ..
                    },
                ] => {
                    assert_eq!(*t, take);
                    assert_eq!(*start, 108);
                    assert_eq!(*a, autoplay, "{step:?}");
                }
                other => panic!("expected one Load, got {other:?}"),
            }
            assert!(!s.vm.take.loading);
            assert_eq!(s.vm.take.story.as_ref().map(|t| t.seed), Some(18433));
            assert_eq!(s.vm.status, "Seed 18433 is ready");

            let fx = s.apply(Event::TakeSound {
                take,
                audio: quiet(),
                music: sample::music(),
            });
            assert!(matches!(&played(&fx)[..], [PlayerCmd::Sound { take: t, .. }] if *t == take));
            assert_eq!(s.vm.take.music, Some(sample::music()));
        }
    }

    #[test]
    fn a_failed_take_says_why() {
        let mut s = ready();
        let take = take_of(&s.dispatch(Action::SelectSeed(613)));
        let fx = s.apply(Event::TakeFailed {
            take,
            problem: "Seed 613 has no K.O. before 58 s.".to_string(),
        });
        assert_eq!(s.vm.take.problem.as_deref(), Some("Seed 613 has no K.O. before 58 s."));
        assert!(!s.vm.take.loading);
        assert_eq!(s.vm.status, "Seed 613 has no K.O. before 58 s.");
        assert!(matches!(&played(&fx)[..], [PlayerCmd::Clear]), "no title card yet");
    }

    #[test]
    fn a_seed_with_no_winner_cannot_be_used() {
        let mut s = ready();
        go(&mut s, Step::Seeds);
        s.dispatch(Action::UseSeed);
        assert_eq!(s.vm.view, View::Step(Step::Seeds), "no take yet");
        let take = take_of(&s.dispatch(Action::SelectSeed(902)));
        s.apply(picture(take, story_with(902, 3.6, None)));
        s.dispatch(Action::UseSeed);
        assert_eq!(s.vm.view, View::Step(Step::Seeds));
        let take = take_of(&s.dispatch(Action::SelectSeed(18433)));
        s.apply(picture(take, story_with(18433, 3.6, Some(1))));
        s.dispatch(Action::UseSeed);
        assert_eq!(s.vm.view, View::Step(Step::Music));
        assert_eq!(s.vm.step, Step::Music);
    }

    #[test]
    fn drafted_captions_arrive_until_edited() {
        let mut s = ready();
        s.apply(fight_info(s.generation(), sample::captions()));
        assert_eq!(s.vm.draft.captions, sample::captions());
        assert_eq!(
            s.vm.pair_rules,
            vec!["A live lance knocks a risen cobra flat.".to_string()]
        );
        s.dispatch(Action::SetCaption {
            platform: Platform::Shorts,
            text: "Mine".to_string(),
        });
        assert!(s.vm.draft.captions_edited);
        s.dispatch(Action::SetBars(30));
        let drafted = CaptionTexts {
            instagram: "Other".to_string(),
            tiktok: "Other".to_string(),
            shorts: "Other".to_string(),
        };
        s.apply(fight_info(s.generation(), drafted.clone()));
        assert_eq!(s.vm.draft.captions.shorts, "Mine");
        assert_eq!(s.vm.draft.captions.instagram, sample::captions().instagram);

        // A new fighter forgets the edit, so the next drafted captions arrive.
        s.dispatch(pick(0, "OBOE"));
        s.apply(fight_info(s.generation(), drafted.clone()));
        assert_eq!(s.vm.draft.captions, drafted);
    }

    #[test]
    fn a_description_is_of_the_fighters() {
        // The fighters were described for generation 1; dragging a handicap starts generation 2, and the
        // description is still of these two fighters, so it is not dropped.
        let mut s = ready();
        s.dispatch(handicap(0, 2.0, false));
        assert_eq!(s.generation(), 2);
        s.apply(fight_info(1, sample::captions()));
        assert_eq!(s.vm.draft.captions, sample::captions());
        assert_eq!(s.vm.pair_rules.len(), 1);
    }

    #[test]
    fn the_caption_line_is_shared() {
        let mut s = ready();
        s.apply(fight_info(s.generation(), sample::captions()));
        s.dispatch(Action::SetCaption {
            platform: Platform::TikTok,
            text: "New line\n#fyp".to_string(),
        });
        let c = &s.vm.draft.captions;
        assert_eq!(c.tiktok, "New line\n#fyp");
        let (_, tags) = sample::captions()
            .instagram
            .split_once('\n')
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .unwrap();
        assert_eq!(c.instagram, format!("New line\n{tags}"));
        assert_eq!(c.shorts, sample::captions().shorts);

        s.dispatch(Action::SetCaption {
            platform: Platform::Instagram,
            text: "Other line\n#ig".to_string(),
        });
        assert_eq!(s.vm.draft.captions.tiktok, "Other line\n#fyp");
    }

    #[test]
    fn filters_are_kept_in_bounds_and_sent() {
        let mut s = ready();
        let spec = FilterSpec {
            ko: (60.0, 20.0),
            finish: Finish::Close,
            ..FilterSpec::default()
        };
        let fx = s.dispatch(Action::SetFilters(spec));
        assert_eq!(s.vm.filters.ko, (30.0, 58.0));
        assert_eq!(s.vm.filters.finish, Finish::Close);
        assert!(matches!(&sent(&fx)[..], [Command::SetFilters { r#gen, filters }]
            if *r#gen == s.generation() && *filters == s.vm.filters));
        let fx = s.dispatch(Action::SetFilters(FilterSpec {
            ko: (52.0, 50.0),
            ..FilterSpec::default()
        }));
        assert_eq!(s.vm.filters.ko, (50.0, 52.0));
        assert_eq!(sent(&fx).len(), 1);
    }

    #[test]
    fn a_fight_problem_holds_every_request() {
        let mut s = ready();
        s.apply(Event::FightInfo {
            r#gen: s.generation(),
            pair_rules: Vec::new(),
            captions: sample::captions(),
            title_card: None,
            problem: Some("These two cannot fight yet.".to_string()),
        });
        assert_eq!(s.vm.fight_problem.as_deref(), Some("These two cannot fight yet."));
        assert_eq!(s.vm.status, "These two cannot fight yet.");
        assert!(sent(&go(&mut s, Step::Balance)).is_empty());
        assert!(sent(&s.dispatch(Action::AutoBalance)).is_empty());
        assert!(sent(&s.dispatch(handicap(0, 2.0, true))).is_empty());
        assert!(sent(&s.dispatch(Action::ResetHandicap)).is_empty());
        assert!(sent(&go(&mut s, Step::Seeds)).is_empty());
        assert!(sent(&s.dispatch(Action::SearchOn)).is_empty());
        assert!(sent(&s.dispatch(Action::SelectSeed(18433))).is_empty());
        assert_eq!(s.vm.shortlist.state, SearchState::Idle);
        assert_eq!(s.vm.balance.running, None);
        assert_eq!(s.vm.status, "These two cannot fight yet.");
    }

    #[test]
    fn the_title_card_shows_while_there_is_no_take() {
        let mut s = ready();
        let card = sample::title_card(s.vm.colors(), [&sample::mask("FLUTE"), &sample::mask("TROMBONE")]);
        let fx = s.apply(Event::FightInfo {
            r#gen: s.generation(),
            pair_rules: Vec::new(),
            captions: sample::captions(),
            title_card: Some(card.clone()),
            problem: None,
        });
        assert!(matches!(&played(&fx)[..], [PlayerCmd::Still(f)] if *f == card));
        let fx = s.dispatch(handicap(0, 2.0, false));
        assert!(matches!(&played(&fx)[..], [PlayerCmd::Still(f)] if *f == card));
        let fx = s.dispatch(Action::SwapSides);
        assert!(
            matches!(&played(&fx)[..], [PlayerCmd::Clear]),
            "the card shows the other order"
        );
    }

    #[test]
    fn transport_goes_to_the_player_only() {
        let mut s = ready();
        let before = s.vm.clone();
        let fx = s.dispatch(Action::Transport(TransportAction::Step(1)));
        assert!(matches!(
            &fx[..],
            [Effect::Player(PlayerCmd::Transport(TransportAction::Step(1)))]
        ));
        assert_eq!(s.vm, before);
    }

    #[test]
    fn the_view_model_names_its_fighters() {
        let s = ready();
        assert_eq!(s.vm.names(), ["Flute".to_string(), "Trombone".to_string()]);
        assert_eq!(s.vm.colors(), [[0xde, 0xe8, 0xf6], [0x78, 0xf0, 0xb8]]);
        assert_eq!(s.vm.theme().map(|t| t.display.as_str()), Some("Big top"));
        assert_eq!(s.vm.fighter(1).map(|f| f.name.as_str()), Some("TROMBONE"));
        assert!(s.vm.fighter(2).is_none());
    }

    #[test]
    fn showing_keeps_the_view_model() {
        let vm = ready().vm;
        let mut s = AppState::showing(vm.clone());
        assert_eq!(s.vm, vm);
        assert_eq!(s.generation(), 1);
        let fx = s.dispatch(Action::SetCaptionTab(Platform::TikTok));
        assert!(fx.is_empty(), "a fixture's fight is not a new one");
        assert_eq!(s.vm.caption_tab, Platform::TikTok);
    }
}
