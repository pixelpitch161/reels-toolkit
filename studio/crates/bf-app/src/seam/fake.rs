//! The sample engine: a stand-in engine on its own thread that answers every [`Command`] from the sample data
//! (`seam::sample`), so the interface can be built and tested before the real engine is behind the seam.
//!
//! What it answers:
//! - `LoadRoster`: the sample roster.
//! - `DescribeFight`: the pair rule "A live lance knocks a risen cobra flat." when FLUTE meets TROMBONE (in either
//!   corner), none otherwise; the sample captions; the two fighters' VS title card. It looks at the fighters only.
//! - `AutoBalance`: 29 `BalanceProgress`, then `Balanced` with the handicap ×2.00 and ×0.40 and the sample numbers.
//!   `Measure`: `Measured` with the sample numbers, the win share moved by `0.1 * ln(left / right)` from 0.47 and kept
//!   in 0.05 to 0.95 (0.47 when the handicaps give no number).
//! - `StartSearch`: fourteen `Shortlist`s, each one sample fight longer in ranked order, `checked` rising by 1,303
//!   from 1,301 to the mock-up's 18,240 and `fits_total` to its 212, the last one `Stopped`. The rows pass through the
//!   filters: winner, K.O. bounds and finish only (`Close` is a winner at 12 HP or less, `Comeback` keeps none); the
//!   lead changes and the ability keep every row (interface reference, section 11). `SetFilters`: the list again,
//!   through the new filters, which the search then goes on with; for a generation with no search, an empty, idle
//!   list.
//! - `LoadTake`: `TakePicture` (the seed's sample story and frames), then `TakeSound` (the sample soundtrack and
//!   music); for a seed ending in 13, `TakeFailed` instead.
//! - `Enqueue`: `Enqueued` with a job built from the draft, whose fight file is a short sample TOML holding the seed,
//!   the theme, the tempo, the music settings and the two fighters with their handicaps.
//! - `RunJob`: the ten check lines in order (each `Now`, then `Done`), with 30 `Frames` counts (every 61 frames,
//!   `seconds` rising by 0.2) while the frames are drawn, then `Done` in `<out_root>\<left>_vs_<right>` after 6 s. A job
//!   whose fight file's seed ends in 2 fails at the loudness check instead. `CancelJob` for the job running:
//!   `Cancelled`, and nothing more for it.
//! - `ScanLibrary`: the six sample videos, placed under the asked-for folder, then a `LibraryCover` for each.
//! - `OpenFight`, `OpenFightText`: `FightOpened` with the starting draft on seed 18433, titled
//!   "Flute vs Trombone, seed 18433".
//! - `CheckTools`: `Ok("ffmpeg (sample), found on this PC")`. `CheckFolder` and `CreateFolder`: the folder can be written and the name
//!   is free, with the command's root and name (empty for `CreateFolder`). `PickFolder`: no folder picked.
//! - `StopBalance`, `StopSearch`: the run stops and sends nothing more. `SetWorkers`: the searches' "cores" from then
//!   on (7, the mock-up's, until told). `PreviewBusy`, `RevealFolder`: nothing. `Shutdown`: the thread ends.
//!
//! It never reads or writes a file and never touches a folder: `OpenFight` does not open the path, `CreateFolder`
//! creates nothing, `RevealFolder` shows nothing, `PickFolder` shows no dialog, a job saves nothing.
//!
//! Every answer about the fight carries the generation of the command it answers, and every answer about a take or a
//! job its take number or job id. A newer command of the same kind replaces the run in progress: a new balance run,
//! search or take drops what was left of the old one, and a new job ends the one running with `Cancelled`.
//!
//! With [`Pace::Instant`] every answer to a command is sent before the next command is taken, so tests see the same
//! events in the same order whatever the machine's speed. With [`Pace::Live`] the runs are spread out in time: balance
//! progress, shortlists and a take's picture and sound are 40 ms apart, and a job takes 6 s (each check line but
//! "Drawing frames" 40 ms, and its 30 frame counts 188 ms apart). The thread ends when the [`EngineHandle`] is dropped
//! or on `Shutdown`, at once even in the middle of a run.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use super::{Command, EngineHandle, EngineSide, Event, Waker, channel, sample};
use crate::model::balance::BalanceStats;
use crate::model::draft::Draft;
use crate::model::library::FolderState;
use crate::model::queue::{FailHint, JobSpec, JobUpdate, STEP_ORDER, StepKey, StepState};
use crate::model::roster::{Roster, RosterEntry};
use crate::model::shortlist::{FilterSpec, Finish, SearchState, Shortlist};
use crate::model::story::StoryView;
use crate::model::take::AudioTake;
use crate::model::{Gen, JobId, NF, TakeId};
use crate::wording;

/// How fast the sample engine answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pace {
    /// Every answer at once: for tests.
    Instant,
    /// Spread out in time like a real engine: for `--fixtures`.
    Live,
}

/// Starts the sample engine on its own thread. It answers every command from the sample data until the handle is
/// dropped or `Shutdown` is sent.
pub fn spawn(waker: Waker, pace: Pace) -> EngineHandle {
    let (handle, side) = channel(waker);
    // Should the PC refuse a new thread, the engine's end is dropped with the closure: commands are then dropped
    // quietly and no answer comes, as for an engine that has stopped.
    _ = std::thread::Builder::new()
        .name("sample engine".to_string())
        .spawn(move || Engine::new(side, pace).run());
    handle
}

/// The gap between progress events in `Pace::Live`.
const TICK: Duration = Duration::from_millis(40);
/// The gap between a job's frame counts in `Pace::Live`: with its nine other lines at `TICK`, a job takes 6 s.
const FRAME_TICK: Duration = Duration::from_millis(188);
/// Progress events before a balance run lands.
const BALANCE_STEPS: u32 = 29;
/// The handicap a balance run lands on (the Balance mock-up's ×2.00 and ×0.40).
const BALANCED: [f64; 2] = [2.0, 0.4];
/// The win share the sample numbers give at an even handicap.
const EVEN_SHARE: f64 = 0.47;
const PAIR_RULE: &str = "A live lance knocks a risen cobra flat.";
/// Seeds checked by the first shortlist, and by each one after it: the last of the fourteen reads 18,240.
const CHECKED_FIRST: u64 = 1_301;
const CHECKED_STEP: u64 = 1_303;
/// Fits found by the time the search has stopped (the mock-up's footer).
const FITS: usize = 212;
/// The search's threads until `SetWorkers` says otherwise (the mock-up's "7 cores").
const WORKERS: usize = 7;
/// A job reports its frames every this many.
const FRAMES_EVERY: u32 = 61;
/// How long a sample job takes, in seconds.
const JOB_SECONDS: f64 = 6.0;
/// The seed an opened fight file is on.
const OPENED_SEED: u64 = 18433;
/// A fighter the roster does not know.
const GREY: [u8; 3] = [0x80, 0x80, 0x80];

/// A run of answers sent one after another, each its gap after the one before.
struct Script {
    /// When the next answer is due.
    due: Instant,
    items: VecDeque<(Duration, Event)>,
}

impl Script {
    fn new(items: Vec<(Duration, Event)>) -> Script {
        let first = items.first().map_or(Duration::ZERO, |(gap, _)| *gap);
        Script {
            due: Instant::now() + first,
            items: items.into(),
        }
    }

    fn due(&self) -> Option<Instant> {
        self.items.front().map(|_| self.due)
    }

    fn pop(&mut self) -> Option<Event> {
        let (_, e) = self.items.pop_front()?;
        if let Some((gap, _)) = self.items.front() {
            self.due += *gap;
        }
        Some(e)
    }
}

/// The background search: how many sample fights it has found, through which filters.
struct Search {
    r#gen: Gen,
    filters: FilterSpec,
    found: usize,
    running: bool,
    due: Instant,
}

/// Which run's answer is next.
#[derive(Clone, Copy)]
enum Run {
    Balance,
    Search,
    Take,
    Job,
}

struct Engine {
    side: EngineSide,
    pace: Pace,
    roster: Roster,
    /// The sample fights, best first.
    stories: Vec<StoryView>,
    workers: usize,
    /// The sample soundtrack, built the first time a take needs it.
    audio: Option<AudioTake>,
    balance: Option<Script>,
    search: Option<Search>,
    take: Option<Script>,
    job: Option<(JobId, Script)>,
}

impl Engine {
    fn new(side: EngineSide, pace: Pace) -> Engine {
        Engine {
            side,
            pace,
            roster: sample::roster(),
            stories: sample::stories(),
            workers: WORKERS,
            audio: None,
            balance: None,
            search: None,
            take: None,
            job: None,
        }
    }

    /// Answers commands until the handle is dropped or `Shutdown` comes, sending each run's answers when due.
    fn run(mut self) {
        loop {
            self.send_due();
            let next = self.next_due();
            let command = match next {
                None => self.side.commands.recv().map_err(|_| RecvTimeoutError::Disconnected),
                Some(at) => self
                    .side
                    .commands
                    .recv_timeout(at.saturating_duration_since(Instant::now())),
            };
            match command {
                Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => return,
                Ok(c) => self.take_command(c),
                Err(RecvTimeoutError::Timeout) => {}
            }
        }
    }

    /// The gap `live` in `Pace::Live`, none in `Pace::Instant`.
    fn gap(&self, live: Duration) -> Duration {
        match self.pace {
            Pace::Instant => Duration::ZERO,
            Pace::Live => live,
        }
    }

    fn script(&self, items: Vec<(Duration, Event)>) -> Script {
        Script::new(items.into_iter().map(|(gap, e)| (self.gap(gap), e)).collect())
    }

    fn dues(&self) -> [(Run, Option<Instant>); 4] {
        [
            (Run::Balance, self.balance.as_ref().and_then(Script::due)),
            (Run::Search, self.search.as_ref().filter(|s| s.running).map(|s| s.due)),
            (Run::Take, self.take.as_ref().and_then(Script::due)),
            (Run::Job, self.job.as_ref().and_then(|(_, s)| s.due())),
        ]
    }

    fn next_due(&self) -> Option<Instant> {
        self.dues().iter().filter_map(|(_, due)| *due).min()
    }

    /// Sends every answer that is due, earliest first.
    fn send_due(&mut self) {
        loop {
            let now = Instant::now();
            let Some((run, _)) = self
                .dues()
                .into_iter()
                .filter_map(|(run, due)| due.filter(|&d| d <= now).map(|d| (run, d)))
                .min_by_key(|&(_, d)| d)
            else {
                return;
            };
            match run {
                Run::Balance => {
                    if let Some(e) = self.balance.as_mut().and_then(Script::pop) {
                        self.side.emit(e);
                    }
                    if self.balance.as_ref().is_some_and(|s| s.due().is_none()) {
                        self.balance = None;
                    }
                }
                Run::Search => self.search_found_one(),
                Run::Take => {
                    if let Some(e) = self.take.as_mut().and_then(Script::pop) {
                        self.side.emit(e);
                    }
                    if self.take.as_ref().is_some_and(|s| s.due().is_none()) {
                        self.take = None;
                    }
                }
                Run::Job => {
                    if let Some(e) = self.job.as_mut().and_then(|(_, s)| s.pop()) {
                        self.side.emit(e);
                    }
                    if self.job.as_ref().is_some_and(|(_, s)| s.due().is_none()) {
                        self.job = None;
                    }
                }
            }
        }
    }

    fn take_command(&mut self, c: Command) {
        match c {
            Command::LoadRoster => self.side.emit(Event::Roster(self.roster.clone())),
            Command::DescribeFight { r#gen, draft } => self.describe(r#gen, &draft),
            Command::AutoBalance { r#gen, .. } => {
                let mut items: Vec<(Duration, Event)> = (1..=BALANCE_STEPS)
                    .map(|done| {
                        (
                            TICK,
                            Event::BalanceProgress {
                                r#gen,
                                done,
                                total: BALANCE_STEPS,
                            },
                        )
                    })
                    .collect();
                items.push((
                    TICK,
                    Event::Balanced {
                        r#gen,
                        handicap: BALANCED,
                        stats: sample::stats(),
                    },
                ));
                self.balance = Some(self.script(items));
            }
            Command::Measure { r#gen, draft } => self.side.emit(Event::Measured {
                r#gen,
                stats: BalanceStats {
                    win_share: win_share(draft.handicap),
                    ..sample::stats()
                },
            }),
            Command::StopBalance => self.balance = None,
            Command::StartSearch { r#gen, filters, .. } => {
                self.search = Some(Search {
                    r#gen,
                    filters,
                    found: 0,
                    running: true,
                    due: Instant::now() + self.gap(TICK),
                });
            }
            Command::StopSearch => {
                if let Some(s) = &mut self.search {
                    s.running = false;
                }
            }
            Command::SetFilters { r#gen, filters } => self.set_filters(r#gen, filters),
            Command::LoadTake { take, draft } => {
                let items = self.take_items(take, &draft);
                self.take = Some(self.script(items));
            }
            Command::Enqueue { draft, out_root } => self.enqueue(&draft, out_root),
            Command::RunJob {
                id,
                fight_toml,
                out_root,
            } => {
                if let Some((old, _)) = self.job.take() {
                    self.side.emit(Event::Job {
                        id: old,
                        update: JobUpdate::Cancelled,
                    });
                }
                let items = job_items(id, &fight_toml, &out_root);
                self.job = Some((id, self.script(items)));
            }
            Command::CancelJob { id } => {
                if self.job.as_ref().is_some_and(|(running, _)| *running == id) {
                    self.job = None;
                    self.side.emit(Event::Job {
                        id,
                        update: JobUpdate::Cancelled,
                    });
                }
            }
            Command::ScanLibrary { root } => self.scan_library(root),
            Command::OpenFight { .. } | Command::OpenFightText { .. } => {
                let draft = self.starting_draft();
                let title = format!("{}, seed {OPENED_SEED}", self.versus(&draft.fighters));
                self.side.emit(Event::FightOpened { draft, title });
            }
            Command::SetWorkers(n) => self.workers = n.max(1),
            Command::CheckTools => self.side.emit(Event::Tools {
                ffmpeg: Ok("ffmpeg (sample), found on this PC".to_string()),
            }),
            Command::CheckFolder { root, name } => self.side.emit(Event::Folder {
                root,
                name,
                state: FolderState::Ok { taken: false },
            }),
            Command::CreateFolder { root } => self.side.emit(Event::Folder {
                root,
                name: String::new(),
                state: FolderState::Ok { taken: false },
            }),
            Command::PickFolder { .. } => self.side.emit(Event::FolderPicked { path: None }),
            Command::PreviewBusy(_) | Command::RevealFolder { .. } | Command::Shutdown => {}
        }
    }

    fn entry(&self, name: &str) -> Option<&RosterEntry> {
        self.roster.fighters.iter().find(|f| f.name == name)
    }

    fn colors(&self, fighters: &[String; 2]) -> [[u8; 3]; 2] {
        [0, 1].map(|side| self.entry(&fighters[side]).map_or(GREY, |f| f.color))
    }

    /// "Flute vs Trombone".
    fn versus(&self, fighters: &[String; 2]) -> String {
        let [left, right] = [0, 1].map(|side| {
            self.entry(&fighters[side])
                .map_or_else(|| wording::display_name(&fighters[side]), |f| f.display.clone())
        });
        format!("{left} vs {right}")
    }

    fn describe(&self, r#gen: Gen, draft: &Draft) {
        let f = &draft.fighters;
        let meets = |a: &str, b: &str| f[0] == a && f[1] == b;
        let pair_rules = if meets("FLUTE", "TROMBONE") || meets("TROMBONE", "FLUTE") {
            vec![PAIR_RULE.to_string()]
        } else {
            Vec::new()
        };
        let masks = [0, 1].map(|side| {
            self.entry(&f[side])
                .map_or_else(|| sample::mask(&f[side]), |e| e.mask.clone())
        });
        self.side.emit(Event::FightInfo {
            r#gen,
            pair_rules,
            captions: sample::captions(),
            title_card: Some(sample::title_card(self.colors(f), [&masks[0], &masks[1]])),
            problem: None,
        });
    }

    /// One more sample fight found; the last one stops the search.
    fn search_found_one(&mut self) {
        let gap = self.gap(TICK);
        let Some(s) = &mut self.search else {
            return;
        };
        s.found = (s.found + 1).min(self.stories.len());
        s.running = s.found < self.stories.len();
        s.due += gap;
        let (r#gen, list) = (s.r#gen, self.list());
        self.side.emit(Event::Shortlist { r#gen, list });
    }

    /// The search's list as it stands, through its filters.
    fn list(&self) -> Shortlist {
        let Some(s) = &self.search else {
            return self.idle_list();
        };
        let found = s.found.min(self.stories.len());
        Shortlist {
            rows: self.stories[..found]
                .iter()
                .filter(|story| passes(story, &s.filters))
                .cloned()
                .collect(),
            fits_total: (FITS * found / self.stories.len().max(1)) as u32,
            checked: match found {
                0 => 0,
                n => CHECKED_FIRST + CHECKED_STEP * (n as u64 - 1),
            },
            state: if s.running {
                SearchState::Searching
            } else {
                SearchState::Stopped
            },
            workers: self.workers,
        }
    }

    fn idle_list(&self) -> Shortlist {
        Shortlist {
            rows: Vec::new(),
            fits_total: 0,
            checked: 0,
            state: SearchState::Idle,
            workers: self.workers,
        }
    }

    fn set_filters(&mut self, r#gen: Gen, filters: FilterSpec) {
        let list = match &mut self.search {
            Some(s) if s.r#gen == r#gen => {
                s.filters = filters;
                self.list()
            }
            // No search for that generation: nothing found.
            _ => self.idle_list(),
        };
        self.side.emit(Event::Shortlist { r#gen, list });
    }

    fn take_items(&mut self, take: TakeId, draft: &Draft) -> Vec<(Duration, Event)> {
        let failed = |problem: String| vec![(TICK, Event::TakeFailed { take, problem })];
        let Some(seed) = draft.seed else {
            return failed("Pick a seed first.".to_string());
        };
        if seed % 100 == 13 {
            return failed(format!(
                "Seed {seed} has no K.O. before 58 s, so it cannot be made into a video. Pick another seed."
            ));
        }
        let audio = self.audio.get_or_insert_with(sample::audio).clone();
        vec![
            (
                TICK,
                Event::TakePicture {
                    take,
                    story: sample::story(seed),
                    frames: sample::frames(self.colors(&draft.fighters)),
                },
            ),
            (
                TICK,
                Event::TakeSound {
                    take,
                    audio,
                    music: sample::music(),
                },
            ),
        ]
    }

    fn enqueue(&self, draft: &Draft, out_root: PathBuf) {
        let (Some(seed), Some(fight_toml)) = (draft.seed, fight_toml(draft)) else {
            let problem = match draft.seed {
                None => "Pick a seed first.".to_string(),
                Some(_) => "This fight could not be written as a fight file.".to_string(),
            };
            self.side.emit(Event::EnqueueFailed { problem });
            return;
        };
        let theme = self
            .roster
            .themes
            .iter()
            .find(|t| t.name == draft.theme)
            .map_or_else(|| draft.theme.clone(), |t| t.display.clone());
        self.side.emit(Event::Enqueued {
            spec: JobSpec {
                title: self.versus(&draft.fighters),
                colors: self.colors(&draft.fighters),
                theme,
                seed,
                fight_toml,
                out_root,
            },
        });
    }

    /// The sample videos, found under `root`, then their covers.
    fn scan_library(&self, root: PathBuf) {
        let videos: Vec<_> = sample::library()
            .into_iter()
            .map(|(mut entry, cover)| {
                if let Some(name) = entry.folder.file_name().map(|n| n.to_os_string()) {
                    let folder = root.join(&name);
                    entry.fight_path = entry
                        .fight_path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|file| folder.join(file));
                    entry.folder = folder;
                }
                (entry, cover)
            })
            .collect();
        self.side.emit(Event::Library {
            root,
            entries: videos.iter().map(|(e, _)| e.clone()).collect(),
            problem: None,
        });
        for (entry, cover) in videos {
            self.side.emit(Event::LibraryCover {
                folder: entry.folder,
                cover,
            });
        }
    }

    /// The draft the app starts with when the roster arrives, on seed 18433.
    fn starting_draft(&self) -> Draft {
        let theme = self.roster.themes.first();
        let fighters = self.roster.start.clone();
        let bass = (0..2).find(|&side| {
            self.entry(&fighters[side])
                .and_then(|f| f.card.as_ref())
                .is_some_and(|card| card.can_play_bass)
        });
        Draft {
            theme: theme.map(|t| t.name.clone()).unwrap_or_default(),
            bars: theme.map_or(34, |t| 34u32.max(t.bars.0).min(t.bars.1)),
            fighters,
            handicap: [1.0, 1.0],
            seed: Some(OPENED_SEED),
            chord_loop: 0,
            bass,
            music_seed: 1,
            captions: sample::captions(),
            captions_edited: false,
            base: None,
        }
    }
}

/// The win share at a handicap: 0.47 moved by `0.1 * ln(left / right)`, kept in 0.05 to 0.95; 0.47 when the
/// handicaps give no number. A sample value for the Balance screen only; it reaches no fight.
fn win_share(handicap: [f64; 2]) -> f64 {
    let share = EVEN_SHARE + 0.1 * (handicap[0] / handicap[1]).ln();
    if share.is_nan() {
        EVEN_SHARE
    } else {
        share.clamp(0.05, 0.95)
    }
}

/// Whether a sample fight passes the filters the sample engine applies: the winner, the K.O. bounds and the finish.
fn passes(story: &StoryView, filters: &FilterSpec) -> bool {
    let winner = filters.winner.is_none_or(|w| story.winner == Some(w));
    let (lo, hi) = filters.ko;
    let ko = story.t_ko.is_some_and(|t| lo <= t && t <= hi);
    let finish = match filters.finish {
        Finish::Any => true,
        Finish::Close => story.winner.is_some() && story.winner_hp <= 12.0,
        Finish::Comeback => false,
    };
    winner && ko && finish
}

/// The sample fight file of a draft: its seed, theme, tempo and music settings, and each fighter with its handicap.
/// `None` when a number does not fit in a TOML integer.
fn fight_toml(draft: &Draft) -> Option<String> {
    let int = |n: u64| i64::try_from(n).ok().map(toml::Value::Integer);
    let mut file = toml::Table::new();
    file.insert("seed".into(), int(draft.seed?)?);
    file.insert("theme".into(), draft.theme.clone().into());
    file.insert("bars".into(), int(u64::from(draft.bars))?);
    file.insert("chord_loop".into(), int(draft.chord_loop as u64)?);
    file.insert("music_seed".into(), int(draft.music_seed)?);
    if let Some(side) = draft.bass {
        file.insert("bass".into(), int(side as u64)?);
    }
    for (side, key) in ["left", "right"].into_iter().enumerate() {
        let mut fighter = toml::Table::new();
        fighter.insert("name".into(), draft.fighters[side].clone().into());
        fighter.insert("handicap".into(), draft.handicap[side].into());
        file.insert(key.into(), toml::Value::Table(fighter));
    }
    toml::to_string(&file).ok()
}

/// A job's answers: the ten check lines in order, the frame counts while the frames are drawn, then `Done`; or, for
/// a fight file whose seed ends in 2, the failed loudness check. A file that cannot be read renders all the same.
fn job_items(id: JobId, fight_toml: &str, out_root: &Path) -> Vec<(Duration, Event)> {
    let file = fight_toml.parse::<toml::Table>().ok();
    let seed = file
        .as_ref()
        .and_then(|f| f.get("seed"))
        .and_then(toml::Value::as_integer);
    let name = |key: &str| {
        file.as_ref()
            .and_then(|f| f.get(key))
            .and_then(|t| t.get("name"))
            .and_then(toml::Value::as_str)
            .map(str::to_string)
    };
    let folder = match (name("left"), name("right")) {
        (Some(left), Some(right)) => out_root.join(wording::folder_stem(&[left, right])),
        _ => out_root.join("sample_video"),
    };
    let fails = seed.is_some_and(|s| s.rem_euclid(10) == 2);

    let job = |update| Event::Job { id, update };
    let line = |key, state, note: Option<&str>| {
        job(JobUpdate::Step {
            key,
            state,
            note: note.map(str::to_string),
        })
    };
    let mut items = Vec::new();
    for key in STEP_ORDER {
        items.push((Duration::ZERO, line(key, StepState::Now, None)));
        match key {
            StepKey::Frames => {
                for k in 1..=NF / FRAMES_EVERY {
                    items.push((
                        FRAME_TICK,
                        job(JobUpdate::Frames {
                            done: k * FRAMES_EVERY,
                            total: NF,
                            seconds: k as f32 / 5.0,
                        }),
                    ));
                }
                items.push((Duration::ZERO, line(key, StepState::Done, None)));
            }
            StepKey::Loudness if fails => {
                items.push((
                    TICK,
                    line(key, StepState::Failed, Some("Opening 2.1 dB quieter than the ending")),
                ));
                items.push((
                    Duration::ZERO,
                    job(JobUpdate::Failed {
                        headline: "Stopped at the loudness check".to_string(),
                        detail: "The opening is 2.1 dB quieter than the ending, and the limit is 1.5 dB. Nothing was \
                                 saved. Open it in Music to see the levels, or try another seed, then queue it again."
                            .to_string(),
                        hint: FailHint::OpenInMusic,
                    }),
                ));
                return items;
            }
            StepKey::Loudness => {
                items.push((
                    TICK,
                    line(key, StepState::Done, Some("Opening within 0.6 dB of the ending")),
                ));
            }
            StepKey::Flashes => {
                items.push((
                    TICK,
                    line(key, StepState::Done, Some("Busiest stretch peaks at 17% of the frame")),
                ));
            }
            _ => items.push((TICK, line(key, StepState::Done, None))),
        }
    }
    items.push((
        Duration::ZERO,
        job(JobUpdate::Done {
            folder,
            seconds: JOB_SECONDS,
        }),
    ));
    items
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use super::*;
    use crate::model::draft::Draft;
    use crate::model::library::FolderState;
    use crate::model::queue::{FailHint, JobSpec, JobUpdate, STEP_ORDER, StepKey, StepState};
    use crate::model::shortlist::{FilterSpec, Finish, SearchState, Shortlist};
    use crate::model::{JobId, NF};
    use crate::seam::{Command, Event, sample};

    /// The longest any test waits for an answer it expects. Answers come far sooner; this only bounds a failure.
    const WAIT: Duration = Duration::from_secs(10);
    /// Quiet for this long after the last event: nothing more is coming.
    const QUIET: Duration = Duration::from_millis(50);

    const FLUTE: [u8; 3] = [0xde, 0xe8, 0xf6];
    const TROMBONE: [u8; 3] = [0x78, 0xf0, 0xb8];

    fn engine(pace: Pace) -> EngineHandle {
        spawn(Arc::new(|| {}), pace)
    }

    /// Everything that arrives: waits (up to `WAIT`) for the first event, then polls until nothing has arrived for
    /// 50 ms.
    fn drain(handle: &EngineHandle) -> Vec<Event> {
        let mut events = Vec::new();
        let start = Instant::now();
        let mut last = None::<Instant>;
        loop {
            if let Some(e) = handle.poll() {
                events.push(e);
                last = Some(Instant::now());
                continue;
            }
            match last {
                Some(at) if at.elapsed() >= QUIET => return events,
                None if start.elapsed() >= WAIT => return events,
                _ => std::thread::sleep(Duration::from_millis(1)),
            }
        }
    }

    /// Everything that arrives up to and including the first event `done` accepts (waiting up to `WAIT` for it),
    /// then whatever follows until 50 ms of quiet.
    fn drain_until(handle: &EngineHandle, done: impl Fn(&Event) -> bool) -> Vec<Event> {
        let mut events = Vec::new();
        let start = Instant::now();
        loop {
            match handle.poll() {
                Some(e) => {
                    let last = done(&e);
                    events.push(e);
                    if last {
                        break;
                    }
                }
                None if start.elapsed() >= WAIT => panic!("the awaited event never came; got {events:?}"),
                None => std::thread::sleep(Duration::from_millis(1)),
            }
        }
        let mut quiet = Instant::now();
        while quiet.elapsed() < QUIET {
            match handle.poll() {
                Some(e) => {
                    events.push(e);
                    quiet = Instant::now();
                }
                None => std::thread::sleep(Duration::from_millis(1)),
            }
        }
        events
    }

    /// Everything that arrives within `span`.
    fn collect_for(handle: &EngineHandle, span: Duration) -> Vec<Event> {
        let mut events = Vec::new();
        let start = Instant::now();
        while start.elapsed() < span {
            match handle.poll() {
                Some(e) => events.push(e),
                None => std::thread::sleep(Duration::from_millis(1)),
            }
        }
        events
    }

    /// Waits (up to `WAIT`) for `ok` to hold.
    fn wait_for(ok: impl Fn() -> bool) -> bool {
        let start = Instant::now();
        while !ok() {
            if start.elapsed() >= WAIT {
                return false;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        true
    }

    fn draft(left: &str, right: &str, seed: Option<u64>) -> Draft {
        Draft {
            fighters: [left.to_string(), right.to_string()],
            theme: "big_top".to_string(),
            bars: 34,
            handicap: [1.0, 1.0],
            seed,
            chord_loop: 0,
            bass: Some(1),
            music_seed: 1,
            captions: sample::captions(),
            captions_edited: false,
            base: None,
        }
    }

    fn flute_trombone(seed: Option<u64>) -> Draft {
        draft("FLUTE", "TROMBONE", seed)
    }

    /// A folder of this test's own under the PC's temp folder, which the sample engine must never create.
    fn temp_root(test: &str) -> PathBuf {
        std::env::temp_dir().join(format!("bf-app-fake-{test}-{}", std::process::id()))
    }

    fn is_roster(e: &Event) -> bool {
        matches!(e, Event::Roster(_))
    }

    fn lists(events: &[Event]) -> Vec<(u64, Shortlist)> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::Shortlist { r#gen, list } => Some((*r#gen, list.clone())),
                _ => None,
            })
            .collect()
    }

    fn is_stopped_list(e: &Event) -> bool {
        matches!(e, Event::Shortlist { list, .. } if list.state == SearchState::Stopped)
    }

    fn job_updates(events: &[Event], want: JobId) -> Vec<JobUpdate> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::Job { id, update } if *id == want => Some(update.clone()),
                _ => None,
            })
            .collect()
    }

    fn enqueue(handle: &EngineHandle, seed: u64, out_root: &Path) -> JobSpec {
        handle.send(Command::Enqueue {
            draft: flute_trombone(Some(seed)),
            out_root: out_root.to_path_buf(),
        });
        let events = drain_until(handle, |e| matches!(e, Event::Enqueued { .. }));
        assert_eq!(events.len(), 1, "{events:?}");
        match &events[0] {
            Event::Enqueued { spec } => spec.clone(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_roster_is_answered() {
        let wakes = Arc::new(AtomicUsize::new(0));
        let w = wakes.clone();
        let handle = spawn(Arc::new(move || _ = w.fetch_add(1, Ordering::SeqCst)), Pace::Instant);
        handle.send(Command::LoadRoster);
        let events = drain(&handle);
        assert_eq!(events.len(), 1, "{events:?}");
        match &events[0] {
            Event::Roster(r) => assert_eq!(*r, sample::roster()),
            other => panic!("{other:?}"),
        }
        // The waker is called after the event is sent.
        assert!(wait_for(|| wakes.load(Ordering::SeqCst) == 1));
    }

    #[test]
    fn a_fight_is_described_with_its_generation() {
        let handle = engine(Pace::Instant);
        handle.send(Command::DescribeFight {
            r#gen: 4,
            draft: flute_trombone(None),
        });
        let events = drain(&handle);
        assert_eq!(events.len(), 1, "{events:?}");
        let Event::FightInfo {
            r#gen,
            pair_rules,
            captions,
            title_card,
            problem,
        } = &events[0]
        else {
            panic!("{:?}", events[0]);
        };
        assert_eq!(*r#gen, 4);
        assert_eq!(pair_rules, &["A live lance knocks a risen cobra flat.".to_string()]);
        assert_eq!(*captions, sample::captions());
        let (left, right) = (sample::mask("FLUTE"), sample::mask("TROMBONE"));
        assert_eq!(
            title_card.as_ref(),
            Some(&sample::title_card([FLUTE, TROMBONE], [&left, &right]))
        );
        assert_eq!(*problem, None);

        handle.send(Command::DescribeFight {
            r#gen: 5,
            draft: draft("FLUTE", "OBOE", None),
        });
        let events = drain(&handle);
        assert_eq!(events.len(), 1, "{events:?}");
        let Event::FightInfo {
            r#gen,
            pair_rules,
            title_card,
            problem,
            ..
        } = &events[0]
        else {
            panic!("{:?}", events[0]);
        };
        assert_eq!(*r#gen, 5);
        assert!(pair_rules.is_empty(), "{pair_rules:?}");
        let (left, right) = (sample::mask("FLUTE"), sample::mask("OBOE"));
        assert_eq!(
            title_card.as_ref(),
            Some(&sample::title_card([FLUTE, [0x9c, 0xb0, 0x34]], [&left, &right]))
        );
        assert_eq!(*problem, None);
    }

    #[test]
    fn balance_reports_then_lands() {
        let handle = engine(Pace::Instant);
        handle.send(Command::AutoBalance {
            r#gen: 3,
            draft: flute_trombone(None),
        });
        let events = drain_until(&handle, |e| matches!(e, Event::Balanced { .. }));
        assert_eq!(events.len(), 30, "{events:?}");
        for (i, e) in events[..29].iter().enumerate() {
            match e {
                Event::BalanceProgress { r#gen, done, total } => {
                    assert_eq!((*r#gen, *done, *total), (3, i as u32 + 1, 29));
                }
                other => panic!("{other:?}"),
            }
        }
        match &events[29] {
            Event::Balanced { r#gen, handicap, stats } => {
                assert_eq!(*r#gen, 3);
                assert_eq!(*handicap, [2.0, 0.4]);
                assert_eq!(*stats, sample::stats());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn measure_moves_the_win_share() {
        let handle = engine(Pace::Instant);
        let share = |r#gen: u64, handicap: [f64; 2]| {
            let mut d = flute_trombone(None);
            d.handicap = handicap;
            handle.send(Command::Measure { r#gen, draft: d });
            let events = drain(&handle);
            assert_eq!(events.len(), 1, "{events:?}");
            match &events[0] {
                Event::Measured { r#gen: g, stats } => {
                    assert_eq!(*g, r#gen);
                    // Everything else is the sample's.
                    assert_eq!(
                        *stats,
                        crate::model::balance::BalanceStats {
                            win_share: stats.win_share,
                            ..sample::stats()
                        }
                    );
                    stats.win_share
                }
                other => panic!("{other:?}"),
            }
        };
        assert_eq!(share(1, [1.0, 1.0]), 0.47);
        assert!((share(2, [2.0, 0.4]) - (0.47 + 0.1 * 5.0f64.ln())).abs() < 1e-12);
        assert!((share(3, [0.4, 2.0]) - (0.47 - 0.1 * 5.0f64.ln())).abs() < 1e-12);
        // Kept in 0.05 to 0.95.
        assert_eq!(share(4, [12.0, 0.05]), 0.95);
        assert_eq!(share(5, [0.05, 12.0]), 0.05);
        // A handicap that is not a number does not give one.
        assert_eq!(share(6, [f64::NAN, 1.0]), 0.47);
        assert_eq!(share(7, [0.0, 0.0]), 0.47);
    }

    #[test]
    fn a_search_grows_to_fourteen_rows_and_stops() {
        let handle = engine(Pace::Instant);
        handle.send(Command::StartSearch {
            r#gen: 2,
            draft: flute_trombone(None),
            filters: FilterSpec::default(),
        });
        let events = drain_until(&handle, is_stopped_list);
        assert_eq!(events.len(), 14, "{events:?}");
        let stories = sample::stories();
        let lists = lists(&events);
        assert_eq!(lists.len(), 14);
        for (k, (r#gen, list)) in lists.iter().enumerate() {
            assert_eq!(*r#gen, 2);
            assert_eq!(list.rows, stories[..=k], "list {k}");
            assert_eq!(list.checked, 1_301 + 1_303 * k as u64);
            let want = if k == 13 {
                SearchState::Stopped
            } else {
                SearchState::Searching
            };
            assert_eq!(list.state, want);
            assert_eq!(list.workers, 7);
        }
        // The mock-up's footer: 212 fits from 18,240 checked.
        let last = &lists[13].1;
        assert_eq!((last.fits_total, last.checked), (212, 18_240));
        assert!(lists.windows(2).all(|w| w[0].1.fits_total <= w[1].1.fits_total));
    }

    #[test]
    fn filters_narrow_the_list() {
        let handle = engine(Pace::Instant);
        handle.send(Command::StartSearch {
            r#gen: 2,
            draft: flute_trombone(None),
            filters: FilterSpec::default(),
        });
        drain_until(&handle, is_stopped_list);
        let through = |filters: FilterSpec| {
            handle.send(Command::SetFilters { r#gen: 2, filters });
            let events = drain(&handle);
            let lists = lists(&events);
            assert_eq!((lists.len(), events.len()), (1, 1), "{events:?}");
            assert_eq!(lists[0].0, 2);
            assert_eq!(lists[0].1.state, SearchState::Stopped);
            assert_eq!(lists[0].1.checked, 18_240);
            lists[0].1.rows.clone()
        };

        let rows = through(FilterSpec {
            winner: Some(1),
            ..FilterSpec::default()
        });
        assert_eq!(rows.len(), 7);
        assert!(rows.iter().all(|r| r.winner == Some(1)));
        assert!(rows.windows(2).all(|w| w[0].score >= w[1].score));

        let rows = through(FilterSpec {
            finish: Finish::Close,
            ..FilterSpec::default()
        });
        assert_eq!(rows.len(), 6);
        assert!(rows.iter().all(|r| r.winner_hp <= 12.0));

        let rows = through(FilterSpec {
            finish: Finish::Comeback,
            ..FilterSpec::default()
        });
        assert!(rows.is_empty());

        // The K.O. bounds.
        let want: Vec<u64> = sample::stories()
            .iter()
            .filter(|s| s.t_ko.is_some_and(|t| (50.0..=52.0).contains(&t)))
            .map(|s| s.seed)
            .collect();
        assert!(!want.is_empty() && want.len() < 14);
        let rows = through(FilterSpec {
            ko: (50.0, 52.0),
            ..FilterSpec::default()
        });
        assert_eq!(rows.iter().map(|r| r.seed).collect::<Vec<_>>(), want);

        // The lead changes and the ability keep every row.
        let rows = through(FilterSpec {
            min_lead_changes: 5,
            ability: Some("FLUTTER TONGUE".to_string()),
            ..FilterSpec::default()
        });
        assert_eq!(rows, sample::stories());

        // A search started with filters is filtered from its first list on.
        handle.send(Command::StartSearch {
            r#gen: 3,
            draft: flute_trombone(None),
            filters: FilterSpec {
                winner: Some(1),
                ..FilterSpec::default()
            },
        });
        let lists = lists(&drain_until(&handle, is_stopped_list));
        assert_eq!(lists.len(), 14);
        assert!(
            lists
                .iter()
                .all(|(g, l)| *g == 3 && l.rows.iter().all(|r| r.winner == Some(1)))
        );
        assert_eq!(lists[13].1.rows.len(), 7);
    }

    #[test]
    fn a_take_arrives_picture_then_sound() {
        let handle = engine(Pace::Instant);
        handle.send(Command::LoadTake {
            take: 9,
            draft: flute_trombone(Some(18433)),
        });
        let events = drain_until(&handle, |e| matches!(e, Event::TakeSound { .. }));
        assert_eq!(events.len(), 2, "{events:?}");
        match &events[0] {
            Event::TakePicture { take, story, frames } => {
                assert_eq!(*take, 9);
                assert_eq!(*story, sample::story(18433));
                let want = sample::frames([FLUTE, TROMBONE]);
                assert_eq!(frames.frames(), NF);
                assert_eq!(frames.draw(0), want.draw(0));
                assert_eq!(frames.draw(942), want.draw(942));
            }
            other => panic!("{other:?}"),
        }
        match &events[1] {
            Event::TakeSound { take, audio, music } => {
                assert_eq!(*take, 9);
                assert!(*audio == sample::audio());
                assert_eq!(*music, sample::music());
            }
            other => panic!("{other:?}"),
        }

        // Any seed has a take: one the mock-ups never listed too.
        handle.send(Command::LoadTake {
            take: 10,
            draft: flute_trombone(Some(77)),
        });
        let events = drain_until(&handle, |e| matches!(e, Event::TakeSound { .. }));
        assert!(matches!(&events[0], Event::TakePicture { take: 10, story, .. } if *story == sample::story(77)));
        assert!(matches!(&events[1], Event::TakeSound { take: 10, .. }));
    }

    #[test]
    fn seed_13_has_no_ko() {
        let handle = engine(Pace::Instant);
        for (take, seed) in [(3, 13), (4, 20_413)] {
            handle.send(Command::LoadTake {
                take,
                draft: flute_trombone(Some(seed)),
            });
            let events = drain(&handle);
            assert_eq!(events.len(), 1, "{events:?}");
            match &events[0] {
                Event::TakeFailed { take: t, problem } => {
                    assert_eq!(*t, take);
                    assert_eq!(
                        problem,
                        &format!(
                            "Seed {seed} has no K.O. before 58 s, so it cannot be made into a video. Pick another seed."
                        )
                    );
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn a_job_runs_its_ten_steps() {
        let handle = engine(Pace::Instant);
        let root = temp_root("job");
        let spec = enqueue(&handle, 18433, &root);
        assert_eq!(spec.title, "Flute vs Trombone");
        assert_eq!(spec.colors, [FLUTE, TROMBONE]);
        assert_eq!(spec.theme, "Big top");
        assert_eq!(spec.seed, 18433);
        assert_eq!(spec.out_root, root);
        let table: toml::Table = spec.fight_toml.parse().unwrap();
        assert_eq!(table["seed"].as_integer(), Some(18433));

        handle.send(Command::RunJob {
            id: 7,
            fight_toml: spec.fight_toml.clone(),
            out_root: root.clone(),
        });
        let events = drain_until(&handle, |e| {
            matches!(
                e,
                Event::Job {
                    update: JobUpdate::Done { .. },
                    ..
                }
            )
        });
        let updates = job_updates(&events, 7);
        assert_eq!(updates.len(), events.len(), "every update is job 7's");

        // Each step starts (Now) and is done, in the check list's order.
        let started: Vec<StepKey> = updates
            .iter()
            .filter_map(|u| match u {
                JobUpdate::Step {
                    key,
                    state: StepState::Now,
                    ..
                } => Some(*key),
                _ => None,
            })
            .collect();
        assert_eq!(started, STEP_ORDER.to_vec());
        let finished: Vec<StepKey> = updates
            .iter()
            .filter_map(|u| match u {
                JobUpdate::Step {
                    key,
                    state: StepState::Done,
                    ..
                } => Some(*key),
                _ => None,
            })
            .collect();
        assert_eq!(finished, STEP_ORDER.to_vec());

        // 30 frame counts, every 61 frames, all while "Drawing frames" is the step under way.
        let frames_at = |state| {
            updates
                .iter()
                .position(|u| matches!(u, JobUpdate::Step { key: StepKey::Frames, state: s, .. } if *s == state))
                .unwrap()
        };
        let (from, to) = (frames_at(StepState::Now), frames_at(StepState::Done));
        let counts: Vec<(usize, u32, u32, f32)> = updates
            .iter()
            .enumerate()
            .filter_map(|(i, u)| match u {
                JobUpdate::Frames { done, total, seconds } => Some((i, *done, *total, *seconds)),
                _ => None,
            })
            .collect();
        assert_eq!(counts.len(), 30);
        for (k, &(i, done, total, seconds)) in counts.iter().enumerate() {
            let k = k as u32 + 1;
            assert!(from < i && i < to, "frame count {k} outside the frames step");
            assert_eq!((done, total), (61 * k, NF));
            assert!((seconds - 0.2 * k as f32).abs() < 1e-4, "{seconds}");
        }

        match updates.last().unwrap() {
            JobUpdate::Done { folder, seconds } => {
                assert_eq!(*folder, root.join("flute_vs_trombone"));
                assert_eq!(*seconds, 6.0);
            }
            other => panic!("{other:?}"),
        }
        // It wrote nothing: not even the output folder exists.
        assert!(!root.exists());
    }

    #[test]
    fn a_seed_ending_in_2_fails_at_loudness() {
        let handle = engine(Pace::Instant);
        let root = temp_root("loudness");
        let spec = enqueue(&handle, 18432, &root);
        handle.send(Command::RunJob {
            id: 2,
            fight_toml: spec.fight_toml,
            out_root: root.clone(),
        });
        let events = drain_until(&handle, |e| {
            matches!(
                e,
                Event::Job {
                    update: JobUpdate::Failed { .. },
                    ..
                }
            )
        });
        let updates = job_updates(&events, 2);
        assert_eq!(updates.len(), events.len());
        match updates.last().unwrap() {
            JobUpdate::Failed { headline, detail, hint } => {
                assert_eq!(headline, "Stopped at the loudness check");
                assert_eq!(
                    detail,
                    "The opening is 2.1 dB quieter than the ending, and the limit is 1.5 dB. Nothing was saved. Open \
                     it in Music to see the levels, or try another seed, then queue it again."
                );
                assert_eq!(*hint, FailHint::OpenInMusic);
            }
            other => panic!("{other:?}"),
        }
        // The loudness line fails; nothing after it starts, no frame is drawn and nothing is done.
        assert!(updates.iter().any(|u| matches!(
            u,
            JobUpdate::Step {
                key: StepKey::Loudness,
                state: StepState::Failed,
                ..
            }
        )));
        let started: Vec<StepKey> = updates
            .iter()
            .filter_map(|u| match u {
                JobUpdate::Step {
                    key,
                    state: StepState::Now,
                    ..
                } => Some(*key),
                _ => None,
            })
            .collect();
        assert_eq!(started, STEP_ORDER[..4].to_vec());
        assert!(
            !updates
                .iter()
                .any(|u| matches!(u, JobUpdate::Frames { .. } | JobUpdate::Done { .. }))
        );
        assert!(!root.exists());
    }

    #[test]
    fn cancel_answers_cancelled() {
        let handle = engine(Pace::Live);
        let root = temp_root("cancel");
        let spec = enqueue(&handle, 18433, &root);
        handle.send(Command::RunJob {
            id: 4,
            fight_toml: spec.fight_toml,
            out_root: root.clone(),
        });
        drain_until(&handle, |e| {
            matches!(
                e,
                Event::Job {
                    update: JobUpdate::Frames { .. },
                    ..
                }
            )
        });
        handle.send(Command::CancelJob { id: 4 });
        let events = drain_until(&handle, |e| {
            matches!(
                e,
                Event::Job {
                    update: JobUpdate::Cancelled,
                    ..
                }
            )
        });
        assert_eq!(job_updates(&events, 4).last(), Some(&JobUpdate::Cancelled));
        // Nothing more for that job: no Done.
        let after = collect_for(&handle, Duration::from_secs(1));
        assert!(job_updates(&after, 4).is_empty(), "{after:?}");
        assert!(!root.exists());
    }

    #[test]
    fn the_library_has_six_videos_and_covers() {
        let handle = engine(Pace::Instant);
        let root = temp_root("library");
        handle.send(Command::ScanLibrary { root: root.clone() });
        let covers = AtomicUsize::new(0);
        let events = drain_until(&handle, |e| {
            matches!(e, Event::LibraryCover { .. }) && covers.fetch_add(1, Ordering::SeqCst) == 5
        });
        assert_eq!(events.len(), 7, "{events:?}");
        let want = sample::library();
        let Event::Library {
            root: r,
            entries,
            problem,
        } = &events[0]
        else {
            panic!("{:?}", events[0]);
        };
        assert_eq!(*r, root);
        assert_eq!(*problem, None);
        assert_eq!(entries.len(), 6);
        for (entry, (sample_entry, _)) in entries.iter().zip(&want) {
            // The sample's videos, found under the folder asked about.
            let name = sample_entry.folder.file_name().unwrap();
            assert_eq!(entry.folder, root.join(name));
            assert_eq!(
                entry.fight_path,
                Some(root.join(name).join(format!("{}.toml", name.to_string_lossy())))
            );
            assert_eq!(
                (&entry.title, entry.colors, entry.seed, entry.modified, &entry.problem),
                (
                    &sample_entry.title,
                    sample_entry.colors,
                    sample_entry.seed,
                    sample_entry.modified,
                    &sample_entry.problem
                )
            );
        }
        for (e, (entry, (_, cover))) in events[1..].iter().zip(entries.iter().zip(&want)) {
            match e {
                Event::LibraryCover { folder, cover: c } => {
                    assert_eq!(*folder, entry.folder);
                    assert_eq!(c, cover);
                }
                other => panic!("{other:?}"),
            }
        }
        assert!(!root.exists());
    }

    #[test]
    fn a_folder_check_echoes_root_and_name() {
        let handle = engine(Pace::Instant);
        let root = temp_root("folder");
        handle.send(Command::CheckFolder {
            root: root.clone(),
            name: "flute_vs_trombone".to_string(),
        });
        let events = drain(&handle);
        assert_eq!(events.len(), 1, "{events:?}");
        match &events[0] {
            Event::Folder { root: r, name, state } => {
                assert_eq!(*r, root);
                assert_eq!(name, "flute_vs_trombone");
                assert_eq!(*state, FolderState::Ok { taken: false });
            }
            other => panic!("{other:?}"),
        }
        // Creating the folder only says it can be written: the sample engine makes nothing on disk.
        handle.send(Command::CreateFolder { root: root.clone() });
        let events = drain(&handle);
        assert_eq!(events.len(), 1, "{events:?}");
        match &events[0] {
            Event::Folder { root: r, name, state } => {
                assert_eq!(*r, root);
                assert_eq!(name, "");
                assert_eq!(*state, FolderState::Ok { taken: false });
            }
            other => panic!("{other:?}"),
        }
        assert!(!root.exists());
    }

    #[test]
    fn tools_folders_and_opened_fights() {
        let handle = engine(Pace::Instant);
        handle.send(Command::CheckTools);
        let events = drain(&handle);
        assert!(
            matches!(&events[..], [Event::Tools { ffmpeg: Ok(t) }] if t == "ffmpeg (sample), found on this PC"),
            "{events:?}"
        );

        handle.send(Command::PickFolder {
            start: temp_root("pick"),
        });
        let events = drain(&handle);
        assert!(
            matches!(&events[..], [Event::FolderPicked { path: None }]),
            "{events:?}"
        );

        let starting = Draft {
            seed: Some(18433),
            ..flute_trombone(None)
        };
        for c in [
            Command::OpenFight {
                path: temp_root("open").join("flute_vs_trombone.toml"),
            },
            Command::OpenFightText {
                toml: "seed = 9120\n".to_string(),
            },
        ] {
            handle.send(c);
            let events = drain(&handle);
            assert_eq!(events.len(), 1, "{events:?}");
            match &events[0] {
                Event::FightOpened { draft, title } => {
                    assert_eq!(*draft, starting);
                    assert_eq!(title, "Flute vs Trombone, seed 18433");
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn commands_with_no_answer_and_odd_ones_never_stop_it() {
        let handle = engine(Pace::Instant);
        for c in [
            Command::StopBalance,
            Command::StopSearch,
            Command::CancelJob { id: 99 },
            Command::PreviewBusy(true),
            Command::SetWorkers(3),
            Command::RevealFolder {
                path: temp_root("reveal"),
            },
        ] {
            handle.send(c);
        }
        handle.send(Command::LoadRoster);
        let events = drain_until(&handle, is_roster);
        assert_eq!(events.len(), 1, "only the roster answers: {events:?}");

        // Filters for a generation with no search: an empty list for that generation.
        handle.send(Command::SetFilters {
            r#gen: 8,
            filters: FilterSpec::default(),
        });
        let lists = lists(&drain(&handle));
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].0, 8);
        assert!(lists[0].1.rows.is_empty());
        assert_eq!(lists[0].1.state, SearchState::Idle);
        assert_eq!(lists[0].1.workers, 3);

        // A take with no seed, a draft with no seed to queue and a job whose file is not a fight file.
        handle.send(Command::LoadTake {
            take: 1,
            draft: flute_trombone(None),
        });
        let events = drain(&handle);
        assert!(matches!(&events[..], [Event::TakeFailed { take: 1, .. }]), "{events:?}");
        handle.send(Command::Enqueue {
            draft: flute_trombone(None),
            out_root: temp_root("odd"),
        });
        let events = drain(&handle);
        assert!(matches!(&events[..], [Event::EnqueueFailed { .. }]), "{events:?}");
        handle.send(Command::RunJob {
            id: 5,
            fight_toml: "not = [a fight".to_string(),
            out_root: temp_root("odd"),
        });
        let events = drain_until(&handle, |e| {
            matches!(
                e,
                Event::Job {
                    update: JobUpdate::Done { .. },
                    ..
                }
            )
        });
        assert_eq!(job_updates(&events, 5).len(), events.len());

        // Strange fighters are described, with a grey card.
        handle.send(Command::DescribeFight {
            r#gen: 9,
            draft: draft("NOBODY", "", None),
        });
        let events = drain(&handle);
        assert!(matches!(&events[..], [Event::FightInfo { r#gen: 9, .. }]), "{events:?}");

        // Still answering.
        handle.send(Command::LoadRoster);
        assert_eq!(drain_until(&handle, is_roster).len(), 1);
    }

    #[test]
    fn a_newer_command_replaces_the_work_in_progress() {
        let handle = engine(Pace::Live);
        // A search for generation 1, then one for generation 2: nothing of the first arrives after the second
        // begins, and only the second finishes.
        handle.send(Command::StartSearch {
            r#gen: 1,
            draft: flute_trombone(None),
            filters: FilterSpec::default(),
        });
        drain_until(&handle, |e| matches!(e, Event::Shortlist { r#gen: 1, .. }));
        handle.send(Command::StartSearch {
            r#gen: 2,
            draft: flute_trombone(None),
            filters: FilterSpec::default(),
        });
        let events = drain_until(&handle, is_stopped_list);
        let lists = lists(&events);
        let first_new = lists.iter().position(|(g, _)| *g == 2).unwrap();
        assert!(lists[first_new..].iter().all(|(g, _)| *g == 2), "{lists:?}");
        assert!(lists.iter().all(|(g, l)| *g == 2 || l.state == SearchState::Searching));
        assert_eq!(lists[first_new..].len(), 14);

        // The same for a balance run.
        handle.send(Command::AutoBalance {
            r#gen: 1,
            draft: flute_trombone(None),
        });
        drain_until(&handle, |e| matches!(e, Event::BalanceProgress { r#gen: 1, .. }));
        handle.send(Command::AutoBalance {
            r#gen: 2,
            draft: flute_trombone(None),
        });
        let events = drain_until(&handle, |e| matches!(e, Event::Balanced { .. }));
        let gens: Vec<u64> = events
            .iter()
            .filter_map(|e| match e {
                Event::BalanceProgress { r#gen, .. } | Event::Balanced { r#gen, .. } => Some(*r#gen),
                _ => None,
            })
            .collect();
        let first_new = gens.iter().position(|g| *g == 2).unwrap();
        assert!(gens[first_new..].iter().all(|g| *g == 2), "{gens:?}");
        assert!(matches!(events.last(), Some(Event::Balanced { r#gen: 2, .. })));

        // And for a take: the newer take's picture and sound, and nothing of the older one after it.
        handle.send(Command::LoadTake {
            take: 1,
            draft: flute_trombone(Some(18433)),
        });
        handle.send(Command::LoadTake {
            take: 2,
            draft: flute_trombone(Some(5627)),
        });
        let events = drain_until(&handle, |e| matches!(e, Event::TakeSound { take: 2, .. }));
        let takes: Vec<u64> = events
            .iter()
            .filter_map(|e| match e {
                Event::TakePicture { take, .. } | Event::TakeSound { take, .. } => Some(*take),
                _ => None,
            })
            .collect();
        let first_new = takes.iter().position(|t| *t == 2).unwrap();
        assert_eq!(&takes[first_new..], &[2, 2]);
    }

    #[test]
    fn a_stop_ends_the_work_in_progress() {
        let handle = engine(Pace::Live);
        let marker = |handle: &EngineHandle| {
            // Commands are taken in order, so once the roster is back the stop before it has been taken.
            handle.send(Command::LoadRoster);
            drain_until(handle, is_roster);
        };
        handle.send(Command::AutoBalance {
            r#gen: 1,
            draft: flute_trombone(None),
        });
        drain_until(&handle, |e| matches!(e, Event::BalanceProgress { .. }));
        handle.send(Command::StopBalance);
        marker(&handle);
        handle.send(Command::StartSearch {
            r#gen: 1,
            draft: flute_trombone(None),
            filters: FilterSpec::default(),
        });
        drain_until(&handle, |e| matches!(e, Event::Shortlist { .. }));
        handle.send(Command::StopSearch);
        marker(&handle);
        // Once stopped, neither sends anything more.
        let after = collect_for(&handle, Duration::from_millis(300));
        assert!(after.is_empty(), "{after:?}");
        // The stopped search's list is still there for new filters, marked stopped.
        handle.send(Command::SetFilters {
            r#gen: 1,
            filters: FilterSpec::default(),
        });
        let lists = lists(&drain(&handle));
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].1.state, SearchState::Stopped);
        assert!(!lists[0].1.rows.is_empty());
    }

    /// Sends the state's commands and gives it the answers, until it asks for nothing more. `CheckTools` follows
    /// each batch as a marker: in `Pace::Instant` every answer to the commands before it has arrived by its own.
    fn pump(handle: &EngineHandle, state: &mut crate::state::AppState, mut fx: Vec<crate::state::Effect>) {
        use crate::state::Effect;
        for _ in 0..100 {
            let commands: Vec<Command> = fx
                .drain(..)
                .filter_map(|e| match e {
                    Effect::Send(c) => Some(c),
                    _ => None,
                })
                .collect();
            if commands.is_empty() {
                return;
            }
            for c in commands {
                handle.send(c);
            }
            handle.send(Command::CheckTools);
            for e in drain_until(handle, |e| matches!(e, Event::Tools { .. })) {
                fx.extend(state.apply(e));
            }
        }
        panic!("the state kept asking");
    }

    #[test]
    fn the_app_state_runs_on_it() {
        use crate::model::queue::{JobState, Queue};
        use crate::model::{Settings, Step, View};
        use crate::state::{Action, AppState};

        let handle = engine(Pace::Instant);
        let root = temp_root("state");
        let settings = Settings {
            output_folder: root.clone(),
            workers: 7,
        };
        let (mut s, fx) = AppState::new(settings, Queue::new(), Vec::new(), 8);
        pump(&handle, &mut s, fx);
        assert_eq!(s.vm.roster, Some(sample::roster()));
        assert_eq!(s.vm.pair_rules, [PAIR_RULE.to_string()]);
        assert_eq!(s.vm.folder, FolderState::Ok { taken: false });
        assert_eq!(s.vm.ffmpeg, Some(Ok("ffmpeg (sample), found on this PC".to_string())));

        let fx = s.dispatch(Action::Go(View::Step(Step::Balance)));
        pump(&handle, &mut s, fx);
        assert_eq!(s.vm.draft.handicap, BALANCED);
        assert_eq!(s.vm.balance.stats, Some(sample::stats()));

        let fx = s.dispatch(Action::Go(View::Step(Step::Seeds)));
        pump(&handle, &mut s, fx);
        assert_eq!(s.vm.shortlist.rows, sample::stories());
        assert_eq!(s.vm.shortlist.state, SearchState::Stopped);

        let fx = s.dispatch(Action::SelectSeed(18433));
        pump(&handle, &mut s, fx);
        assert_eq!(s.vm.take.story, Some(sample::story(18433)));
        assert_eq!(s.vm.take.music, Some(sample::music()));
        assert_eq!(s.vm.status, "Seed 18433 is ready");

        let fx = s.dispatch(Action::AddToQueue);
        pump(&handle, &mut s, fx);
        let job = &s.vm.queue.jobs[0];
        assert_eq!(job.spec.title, "Flute vs Trombone");
        assert_eq!(
            job.state,
            JobState::Done {
                folder: root.join("flute_vs_trombone"),
                seconds: 6.0,
            }
        );
        assert_eq!(s.vm.library.entries.len(), 6);
        assert_eq!(s.vm.library.covers.len(), 6);
        assert!(!root.exists());
    }

    #[test]
    fn the_thread_ends_when_the_handle_is_dropped() {
        // The waker is held by the engine's thread: once it has ended, this is the only holder left.
        let waker: Waker = Arc::new(|| {});
        let handle = spawn(waker.clone(), Pace::Live);
        let root = temp_root("drop");
        let spec = enqueue(&handle, 18433, &root);
        handle.send(Command::RunJob {
            id: 1,
            fight_toml: spec.fight_toml,
            out_root: root,
        });
        handle.send(Command::StartSearch {
            r#gen: 1,
            draft: flute_trombone(None),
            filters: FilterSpec::default(),
        });
        drain_until(&handle, |e| {
            matches!(
                e,
                Event::Job {
                    update: JobUpdate::Frames { .. },
                    ..
                }
            )
        });
        assert!(Arc::strong_count(&waker) > 1);
        drop(handle);
        assert!(
            wait_for(|| Arc::strong_count(&waker) == 1),
            "the engine's thread is still running"
        );

        // `Shutdown` ends it too, with the handle still held; later commands are dropped quietly.
        let waker: Waker = Arc::new(|| {});
        let handle = spawn(waker.clone(), Pace::Live);
        handle.send(Command::Shutdown);
        assert!(
            wait_for(|| Arc::strong_count(&waker) == 1),
            "the engine's thread is still running"
        );
        handle.send(Command::LoadRoster);
        assert!(handle.poll().is_none());
    }
}
