//! The seam between the interface and the engine: plain-data commands one way, events the other, over channels.
//!
//! The interface thread never waits on the engine. It sends a [`Command`] with [`EngineHandle::send`], which returns
//! at once, and later takes whatever has arrived with [`EngineHandle::poll`], which never blocks. The engine's
//! threads answer through an [`EventSender`], which also calls the [`Waker`] so the interface repaints and polls.
//!
//! An answer may arrive after the fight has changed. Every command about the fight carries the generation ([`Gen`])
//! it was asked for, every command about a take carries its [`TakeId`], and the answering event echoes them, so the
//! interface recognises a late answer and drops it.
//!
//! `gen` is a reserved word in Rust 2024, so the generation fields are written `r#gen` (in patterns too:
//! `Event::Shortlist { r#gen, list }`); the field's name is still `gen`.

pub mod fake;
pub mod sample;
pub mod upkeep;

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::time::{Duration, Instant};

use crate::model::balance::BalanceStats;
use crate::model::draft::{CaptionTexts, Draft};
use crate::model::library::{FolderState, LibraryEntry};
use crate::model::queue::{JobSpec, JobUpdate};
use crate::model::roster::Roster;
use crate::model::shortlist::{FilterSpec, Shortlist};
use crate::model::story::StoryView;
use crate::model::take::{AudioTake, Frame, FrameSource, MusicView};
use crate::model::{EncodeChoice, Gen, JobId, TakeId};

/// What the interface asks of the engine.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Answered by `Event::Roster`.
    LoadRoster,
    /// The pair rules, drafted captions and title card of the draft's fighters: `Event::FightInfo`.
    /// The description must depend on the fighters and their order only (never on the tempo, the handicap or the
    /// music): the interface keeps it for every generation until the fighters or their order change.
    DescribeFight {
        r#gen: Gen,
        draft: Draft,
    },
    /// Find a fair handicap: `Event::BalanceProgress`, then `Event::Balanced` (or `Event::BalanceFailed`).
    AutoBalance {
        r#gen: Gen,
        draft: Draft,
    },
    /// Measure the handicap as it stands: `Event::Measured` (or `Event::BalanceFailed`).
    Measure {
        r#gen: Gen,
        draft: Draft,
    },
    StopBalance,
    /// Search seeds in the background: `Event::Shortlist`, again and again as it grows.
    StartSearch {
        r#gen: Gen,
        draft: Draft,
        filters: FilterSpec,
    },
    StopSearch,
    /// Show the search's results through other filters: `Event::Shortlist`.
    SetFilters {
        r#gen: Gen,
        filters: FilterSpec,
    },
    /// The picture and sound of the draft's seed: `Event::TakePicture`, then `Event::TakeSound`, or
    /// `Event::TakeFailed`.
    LoadTake {
        take: TakeId,
        draft: Draft,
    },
    /// The preview is drawing (true) or idle (false), so background work can give way.
    PreviewBusy(bool),
    /// Make the fight file of a draft for the queue: `Event::Enqueued` or `Event::EnqueueFailed`.
    Enqueue {
        draft: Draft,
        out_root: PathBuf,
    },
    /// Render one queued video: a stream of `Event::Job`. `speed` is Settings' "Video encoding" as it stood when the
    /// job started.
    RunJob {
        id: JobId,
        fight_toml: String,
        out_root: PathBuf,
        speed: EncodeChoice,
    },
    CancelJob {
        id: JobId,
    },
    /// The videos under the output folder: `Event::Library`, then an `Event::LibraryCover` per video.
    ScanLibrary {
        root: PathBuf,
    },
    /// Open a fight file: `Event::FightOpened`.
    OpenFight {
        path: PathBuf,
    },
    /// Open a fight file's text: `Event::FightOpened`.
    OpenFightText {
        toml: String,
    },
    SetWorkers(usize),
    /// Find ffmpeg: `Event::Tools`.
    CheckTools,
    /// Whether `root` can be written and `name` in it is free: `Event::Folder`.
    CheckFolder {
        root: PathBuf,
        name: String,
    },
    CreateFolder {
        root: PathBuf,
    },
    /// Show the folder dialog: `Event::FolderPicked`.
    PickFolder {
        start: PathBuf,
    },
    /// Show a folder in the PC's file browser.
    RevealFolder {
        path: PathBuf,
    },
    /// The app is closing: stop everything.
    Shutdown,
}

/// What the engine tells the interface.
#[derive(Clone, Debug)]
pub enum Event {
    Roster(Roster),
    FightInfo {
        r#gen: Gen,
        pair_rules: Vec<String>,
        captions: CaptionTexts,
        title_card: Option<Frame>,
        /// Why this pair cannot fight, in words.
        problem: Option<String>,
        /// What the fight file the draft is laid over says about its music (its parts and theme options), or
        /// `None` when there is none to say (the pair cannot fight, or the sample engine).
        music: Option<crate::model::roster::PairMusic>,
    },
    BalanceProgress {
        r#gen: Gen,
        done: u32,
        total: u32,
    },
    Balanced {
        r#gen: Gen,
        handicap: [f64; 2],
        stats: BalanceStats,
    },
    Measured {
        r#gen: Gen,
        stats: BalanceStats,
    },
    /// A balance run or a measurement of generation `gen` could not be made or broke off, in words. It ends that
    /// work on screen (ruling M2-R29): nothing more comes for it.
    BalanceFailed {
        r#gen: Gen,
        text: String,
    },
    Shortlist {
        r#gen: Gen,
        list: Shortlist,
    },
    TakePicture {
        take: TakeId,
        story: StoryView,
        frames: Arc<dyn FrameSource>,
    },
    TakeSound {
        take: TakeId,
        audio: AudioTake,
        music: MusicView,
    },
    TakeFailed {
        take: TakeId,
        problem: String,
    },
    Enqueued {
        spec: JobSpec,
    },
    EnqueueFailed {
        problem: String,
    },
    Job {
        id: JobId,
        update: JobUpdate,
    },
    Library {
        root: PathBuf,
        entries: Vec<LibraryEntry>,
        problem: Option<String>,
    },
    LibraryCover {
        folder: PathBuf,
        cover: Frame,
    },
    FightOpened {
        draft: Draft,
        title: String,
    },
    /// ffmpeg's description, or why it was not found.
    Tools {
        ffmpeg: Result<String, String>,
    },
    /// The answer to `CheckFolder` or `CreateFolder`, with the root and name it was asked about.
    Folder {
        root: PathBuf,
        name: String,
        state: FolderState,
    },
    FolderPicked {
        path: Option<PathBuf>,
    },
    /// Something went wrong that no other event covers, in words.
    Problem {
        text: String,
    },
}

/// Called after every event is sent, from the engine's thread, so the interface wakes up and polls.
pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// Whether the engine's end of a seam ([`EngineSide`]) has been dropped, which is how an engine says it has
/// stopped: the real engine drops it only after its own threads have ended (or its wait for them ran out), the
/// sample engine when its thread ends.
#[derive(Debug, Default)]
struct Stopped {
    done: Mutex<bool>,
    bell: Condvar,
}

/// Held by the [`EngineSide`]: dropping it marks the engine stopped.
#[derive(Debug)]
struct StopMark(Arc<Stopped>);

impl Drop for StopMark {
    fn drop(&mut self) {
        *self.0.done.lock().unwrap_or_else(|e| e.into_inner()) = true;
        self.0.bell.notify_all();
    }
}

/// The interface's end of the seam.
pub struct EngineHandle {
    commands: Sender<Command>,
    events: Receiver<Event>,
    stopped: Arc<Stopped>,
}

impl EngineHandle {
    /// Sends a command and returns at once. A command for an engine that has stopped is dropped.
    pub fn send(&self, c: Command) {
        // The channel has no bound, so this never waits; an error only means the engine has gone.
        _ = self.commands.send(c);
    }

    /// The next event that has arrived, if any; never waits.
    pub fn poll(&self) -> Option<Event> {
        self.events.try_recv().ok()
    }

    /// Whether the engine has stopped (its end of the seam is gone).
    pub fn has_stopped(&self) -> bool {
        *self.stopped.done.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Sends `Shutdown` and waits up to `within` for the engine to stop. Returns whether it stopped in time. The one
    /// call that waits: it is for the app's exit, when nothing is drawn any more.
    pub fn stop(&self, within: Duration) -> bool {
        self.send(Command::Shutdown);
        let deadline = Instant::now() + within;
        let mut done = self.stopped.done.lock().unwrap_or_else(|e| e.into_inner());
        while !*done {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            done = self
                .stopped
                .bell
                .wait_timeout(done, left)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
        true
    }
}

impl std::fmt::Debug for EngineHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineHandle").finish_non_exhaustive()
    }
}

/// What an engine thread keeps to answer the interface. Clone it for every thread that answers.
#[derive(Clone)]
pub struct EventSender {
    events: Sender<Event>,
    waker: Waker,
}

impl EventSender {
    /// Sends an event, then calls the waker. An event for an interface that has gone is dropped.
    pub fn emit(&self, e: Event) {
        if self.events.send(e).is_ok() {
            (self.waker)();
        }
    }
}

impl std::fmt::Debug for EventSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventSender").finish_non_exhaustive()
    }
}

/// The engine's end of the seam. Dropping it tells the interface the engine has stopped
/// ([`EngineHandle::stop`]), so an engine keeps it until all its work has ended.
#[derive(Debug)]
pub struct EngineSide {
    pub commands: Receiver<Command>,
    pub events: EventSender,
    /// Never read: dropping it is what marks the engine stopped.
    _stopped: StopMark,
}

impl EngineSide {
    /// Sends an event through `events`, then calls the waker.
    pub fn emit(&self, e: Event) {
        self.events.emit(e);
    }
}

/// Both ends of a new seam. `waker` is called after every event the engine sends.
pub fn channel(waker: Waker) -> (EngineHandle, EngineSide) {
    let (command_tx, command_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    let stopped = Arc::new(Stopped::default());
    (
        EngineHandle {
            commands: command_tx,
            events: event_rx,
            stopped: stopped.clone(),
        },
        EngineSide {
            commands: command_rx,
            events: EventSender {
                events: event_tx,
                waker,
            },
            _stopped: StopMark(stopped),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn counting_waker() -> (Waker, Arc<AtomicUsize>) {
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        (Arc::new(move || _ = c.fetch_add(1, Ordering::SeqCst)), count)
    }

    #[test]
    fn events_reach_the_other_side() {
        let (waker, wakes) = counting_waker();
        let (handle, side) = channel(waker);
        assert!(handle.poll().is_none());

        side.emit(Event::Problem {
            text: "one".to_string(),
        });
        match handle.poll() {
            Some(Event::Problem { text }) => assert_eq!(text, "one"),
            other => panic!("expected the problem, got {other:?}"),
        }
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
        assert!(handle.poll().is_none());

        let events = side.events.clone();
        std::thread::spawn(move || {
            events.emit(Event::Problem {
                text: "two".to_string(),
            })
        })
        .join()
        .unwrap();
        match handle.poll() {
            Some(Event::Problem { text }) => assert_eq!(text, "two"),
            other => panic!("expected the second problem, got {other:?}"),
        }
        assert_eq!(wakes.load(Ordering::SeqCst), 2);

        handle.send(Command::StopSearch);
        assert_eq!(side.commands.try_recv(), Ok(Command::StopSearch));
    }

    #[test]
    fn stop_waits_until_the_engine_side_is_gone() {
        let (handle, side) = channel(Arc::new(|| {}));
        assert!(!handle.has_stopped());
        // an engine that does not stop: the wait is bounded
        let began = Instant::now();
        assert!(!handle.stop(Duration::from_millis(50)));
        assert!(began.elapsed() >= Duration::from_millis(50));
        assert_eq!(side.commands.try_recv(), Ok(Command::Shutdown));
        // an engine that stops a moment after it is asked
        let engine = std::thread::spawn(move || {
            assert_eq!(side.commands.recv(), Ok(Command::Shutdown));
            std::thread::sleep(Duration::from_millis(30));
            drop(side);
        });
        assert!(handle.stop(Duration::from_secs(10)));
        assert!(handle.has_stopped());
        engine.join().unwrap();
        // once stopped, asking again answers at once
        let began = Instant::now();
        assert!(handle.stop(Duration::from_secs(10)));
        assert!(began.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn a_closed_seam_drops_quietly() {
        let (waker, wakes) = counting_waker();
        let (handle, side) = channel(waker);
        let events = side.events.clone();
        drop(side);
        handle.send(Command::Shutdown);
        drop(handle);
        events.emit(Event::Problem {
            text: "late".to_string(),
        });
        assert_eq!(wakes.load(Ordering::SeqCst), 0);
    }
}
