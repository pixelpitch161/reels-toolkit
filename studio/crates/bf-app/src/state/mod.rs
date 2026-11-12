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
pub mod upkeep;

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
use crate::model::{EncodeChoice, Gen, JobId, Notice, Platform, Settings, Step, TakeId, View};
use crate::seam::upkeep::{SampleUpkeep, Upkeep};
use crate::seam::{Command, Event};
use crate::update::UpdateView;
use crate::update::worker::Request;
use crate::wording;
use crate::wording::NO_DEVICE;

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
    /// Updates, the guide, the fault reports and the closing words, as the upkeep seam last said
    /// (`seam::upkeep`; copied in by `upkeep::sync_upkeep`).
    pub update: UpdateView,
    /// Settings' "Version" row: `version::line()`, such as "0.6.0 (a842e11, 12 Nov 2026)".
    pub app_line: String,
    /// Settings' "Remove" row: the button, the question, or the removal on its way.
    pub removal: upkeep::RemovalView,
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
    /// Settings' "Video encoding": applies to the next video that starts.
    SetEncode(EncodeChoice),
    /// A request for the upkeep seam ("Look again", "Build version {v}", "Restart now", "Remove the app", ...),
    /// unless the rules of `upkeep.rs` refuse it.
    Upkeep(Request),
    /// "Remove BALLFIGHT Studio from this PC": shows what would be removed, and asks.
    AskRemove,
    /// "Keep it": the question goes away and nothing is removed.
    KeepApp,
    /// "Open the reports folder": the fault reports folder in the PC's file browser (`RevealFolder`).
    OpenReports,
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
    /// The theme (which gives the key and scale, and whose chord loops `chord_loop` indexes), the chord loop and
    /// the music seed, when either fighter's notes are its weapon (§4, ruling M2-R14).
    pub music: Option<(String, usize, u64)>,
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
    /// The owner stopped a balance run in this generation, or a balance run or measurement failed
    /// (`Event::BalanceFailed`), so opening Balance does not start another by itself.
    balance_stopped: bool,
    /// The generation the current fighters were described for (`DescribeFight`): a description for it or any
    /// later generation is of these fighters.
    described: Gen,
    /// What the queue, folder and library rules remember (`queue_run.rs`).
    run: queue_run::QueueRun,
    /// The playhead's frame when the preview could not draw it, as the player last said (`player_view`).
    failed_frame: Option<u32>,
    /// What updates and removal go through: a sample that is always what `vm.update` was when the state was made,
    /// until `set_upkeep` gives the real one (`seam::upkeep`).
    upkeep: Box<dyn Upkeep>,
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
            update: SampleUpkeep::not_installed(None).view(),
            app_line: crate::version::line(),
            removal: upkeep::RemovalView::Closed,
        };
        let mut state = AppState::bare(vm);
        state.after_restart();
        state.folder_checked_as_shown();
        (state, effects)
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
        // What `vm.folder` says is taken to be about the folder and pair shown.
        state.folder_checked_as_shown();
        state
    }

    fn bare(vm: ViewModel) -> AppState {
        let upkeep = Box::new(SampleUpkeep::showing(vm.update.clone()));
        AppState {
            upkeep,
            vm,
            generation: 0,
            fed: None,
            music: None,
            wanted: None,
            last_take: 0,
            title_card: None,
            measuring: false,
            balance_stopped: false,
            described: 0,
            run: queue_run::QueueRun::default(),
            failed_frame: None,
        }
    }

    /// The current generation.
    pub fn generation(&self) -> Gen {
        self.generation
    }

    /// Takes one thing the owner did.
    pub fn dispatch(&mut self, action: Action) -> Vec<Effect> {
        let mut fx = Vec::new();
        let generation = self.generation;
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
            Action::SearchOn => self.search_on(&mut fx),
            Action::SelectSeed(seed) => self.select_seed(seed, &mut fx),
            Action::UseSeed => self.use_seed(),
            Action::SetChordLoop(i) => self.set_chord_loop(i),
            Action::SetBass(side) => self.set_bass(side),
            Action::SetCaptionTab(platform) => self.vm.caption_tab = platform,
            Action::SetCaption { platform, text } => self.set_caption(platform, text),
            Action::Transport(a) => fx.push(Effect::Player(PlayerCmd::Transport(a))),
            // The output folder, the queue, the library and the settings (`queue_run.rs`).
            a @ Action::ChangeFolder => self.queue_action(a, &mut fx),
            a @ Action::CreateFolder => self.queue_action(a, &mut fx),
            a @ Action::AddToQueue => self.queue_action(a, &mut fx),
            a @ Action::Queue(_) => self.queue_action(a, &mut fx),
            a @ Action::RefreshLibrary => self.queue_action(a, &mut fx),
            a @ Action::SelectVideo(_) => self.queue_action(a, &mut fx),
            a @ Action::Reopen(_) => self.queue_action(a, &mut fx),
            a @ Action::OpenVideoFolder(_) => self.queue_action(a, &mut fx),
            a @ Action::SetWorkers(_) => self.queue_action(a, &mut fx),
            a @ Action::Dismiss(_) => self.queue_action(a, &mut fx),
            a @ Action::SetEncode(_) => self.queue_action(a, &mut fx),
            // Updates, the guide and removal (`upkeep.rs`).
            Action::Upkeep(r) => self.upkeep_request(r),
            Action::AskRemove => self.ask_remove(),
            Action::KeepApp => self.keep_app(),
            Action::OpenReports => {
                if let Some(path) = self.vm.update.reports.clone() {
                    fx.push(Effect::Send(Command::RevealFolder { path }));
                }
            }
        }
        self.settle(&mut fx);
        self.after_queue_run(Some(generation), &mut fx);
        self.poll_upkeep();
        self.vm.status = self.status();
        fx
    }

    /// Takes one event from the engine.
    pub fn apply(&mut self, event: Event) -> Vec<Effect> {
        let mut fx = Vec::new();
        self.on_event(event, &mut fx);
        self.settle(&mut fx);
        self.after_queue_run(None, &mut fx);
        self.poll_upkeep();
        self.vm.status = self.status();
        fx
    }

    /// What the preview player says, once per frame (the shell copies it): the transport as it stands and the
    /// playhead's frame when it could not be drawn. Only the player knows these, so they come in here rather than
    /// through an event; the status line follows them like any other change.
    pub fn player_view(&mut self, transport: TransportView, failed_frame: Option<u32>) {
        if self.vm.transport == transport && self.failed_frame == failed_frame {
            return;
        }
        self.vm.transport = transport;
        self.failed_frame = failed_frame;
        self.vm.status = self.status();
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
            // The library is scanned each time it opens.
            View::Library => self.scan_library(fx),
            View::Settings => {}
        }
    }

    /// The status line, worked out from the state after every action and event and whenever the player's view
    /// changes (ruling M2-R17). The first of these that holds:
    /// 1. a problem: why the pair cannot fight, else why the take cannot be made;
    /// 2. the preview could not draw the playhead's frame: "Frame {n} could not be drawn. Try another seed." (`n`
    ///    counted from 0, as the transport counts frames; widget rule 11);
    /// 3. a video is rendering: "Rendering {title}, about {n} s left", or "Rendering {title}" until its frame count
    ///    gives a time (a render is the longer, costlier wait, so it comes before the search);
    /// 4. a search is running: "Searching on {workers} cores";
    /// 5. a fight file was opened: "Opened {title}", until the draft or the view changes;
    /// 6. the sound device was not found or was lost: "No sound device was found, so the preview is silent."
    ///    (widget rule 14; it stands where the take's own line would);
    /// 7. a take is loaded: "Seed {n} is ready";
    /// 8. there are balance numbers: "Balanced on {n} fights in {s} s" (without the time when it is not a real one,
    ///    widget rule 9);
    /// 9. "{Left} vs {Right}, not balanced yet".
    ///
    /// Before the roster arrives it is empty.
    pub(crate) fn status(&self) -> String {
        if self.fed.is_none() {
            return String::new();
        }
        let vm = &self.vm;
        if let Some(problem) = vm.fight_problem.as_ref().or(vm.take.problem.as_ref()) {
            return problem.clone();
        }
        if let Some(f) = self.failed_frame {
            return format!("Frame {f} could not be drawn. Try another seed.");
        }
        if let Some(rendering) = self.rendering_status() {
            return rendering;
        }
        if vm.shortlist.state == SearchState::Searching {
            return format!(
                "Searching on {}",
                wording::plural(vm.shortlist.workers as u64, "core", "cores")
            );
        }
        if let Some(opened) = self.opened_status() {
            return opened;
        }
        if vm.transport.sound == SoundState::NoDevice {
            return NO_DEVICE.to_string();
        }
        if let Some(story) = &vm.take.story {
            return format!("Seed {} is ready", story.seed);
        }
        if let Some(stats) = &vm.balance.stats {
            let n = wording::thousands(u64::from(stats.fights));
            return if stats.seconds.is_finite() && stats.seconds.is_sign_positive() {
                format!("Balanced on {n} fights in {}", wording::seconds(stats.seconds))
            } else {
                format!("Balanced on {n} fights")
            };
        }
        let [left, right] = vm.names();
        format!("{left} vs {right}, not balanced yet")
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
mod tests;
