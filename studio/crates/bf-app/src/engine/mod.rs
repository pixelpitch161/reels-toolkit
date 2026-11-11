//! The real engine behind the seam: one coordinator thread that takes the interface's commands and hands each to
//! the code that answers it.
//!
//! The coordinator never does long work itself. Describing a fight, loading the roster, and everything the five
//! handlers start (balance, search, takes, jobs, files) runs on a thread of its own, started with [`Ctx::spawn`],
//! and answers through the [`EventSender`] in [`Ctx`]. The coordinator's routing is written once, here:
//!
//! - `LoadRoster`, `DescribeFight`, `SetWorkers`, `PreviewBusy` and `Shutdown` are answered here;
//! - `AutoBalance`, `Measure` and `StopBalance` go to [`balance::handle`];
//! - `StartSearch`, `StopSearch` and `SetFilters` go to [`search::handle`];
//! - `LoadTake` goes to [`takes::handle`];
//! - `Enqueue`, `RunJob`, `CancelJob` and `CheckTools` go to [`jobs::handle`];
//! - `ScanLibrary`, `OpenFight`, `OpenFightText`, `CheckFolder`, `CreateFolder`, `PickFolder` and `RevealFolder` go
//!   to [`files::handle`].
//!
//! **Ending.** `Shutdown`, or the interface dropping its [`EngineHandle`], ends the coordinator. As it ends it sets
//! every cancel flag made with [`Ctx::cancel_flag`] and the engine's own stop flag ([`Ctx::stopping`]), refuses
//! to start new threads, and waits up to [`STOP_WAIT`] for every thread it started to finish.
//!
//! **Panics.** A panic on a thread started with [`Ctx::spawn`] is caught there and becomes the event the caller
//! asked for, carrying a plain sentence ("Something went wrong while ..., so it stopped. Try again."). A panic in
//! a handler's `handle` (on the coordinator) is caught by the coordinator and becomes `Event::Problem`. Either way
//! the engine keeps taking commands.
//!
//! **The newest description wins.** `DescribeFight` goes to one describing thread that keeps only the newest
//! request waiting. It answers in the order it was asked and drops an answer when a newer request is already
//! waiting, so an older description never arrives after a newer one (the engine's side). The interface also
//! ignores a description older than the fighters it shows (the state's side, by generation).

pub mod balance;
pub mod content;
pub mod convert;
pub mod files;
pub mod jobs;
pub mod search;
pub mod takes;

use std::ops::Range;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::model::Gen;
use crate::model::draft::Draft;
use crate::seam::{self, Command, EngineHandle, EngineSide, Event, EventSender, Waker};
use crate::store::Paths;

/// How long the ending coordinator waits for the threads it started to see their cancel flags and finish. A
/// thread still running after that is left to finish by itself.
pub const STOP_WAIT: Duration = Duration::from_secs(3);

/// What the engine is started with.
#[derive(Clone, Debug, PartialEq)]
pub struct EngineConfig {
    /// Threads the engine may use for searching, balancing and rendering (changed later by `SetWorkers`).
    pub workers: usize,
    /// Where settings, the queue and the shortlists are kept.
    pub paths: Paths,
    /// Tests: a job renders only these frames.
    pub clip: Option<Range<usize>>,
    /// Tests: a search stops once this many seeds are checked.
    pub search_limit: Option<u64>,
}

/// What every handler is given: the way to answer, the configuration, the shared settings, and the way to start
/// threads and make cancel flags that the engine's ending reaches. Cheap to clone; every clone is the same engine.
#[derive(Clone)]
pub struct Ctx {
    /// Sends an event to the interface and wakes it.
    pub events: EventSender,
    pub config: Arc<EngineConfig>,
    /// Threads the engine may use now: `config.workers` at start, then each `SetWorkers` (read with
    /// [`Ctx::workers`]).
    pub workers: Arc<AtomicUsize>,
    /// The preview is drawing (`PreviewBusy`), so background work may give way.
    pub preview_busy: Arc<AtomicBool>,
    /// Set once the engine is ending.
    stop: Arc<AtomicBool>,
    shared: Arc<Mutex<Shared>>,
}

/// What the coordinator keeps of the work it started.
#[derive(Default)]
struct Shared {
    threads: Vec<JoinHandle<()>>,
    cancels: Vec<Weak<AtomicBool>>,
}

impl std::fmt::Debug for Ctx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ctx")
            .field("config", &self.config)
            .field("workers", &self.workers())
            .field("preview_busy", &self.preview_busy())
            .field("stopping", &self.stopping())
            .finish_non_exhaustive()
    }
}

/// A lock that a panic elsewhere cannot poison for good: what it guards stays usable.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// `Event::Problem` with `text`.
pub fn problem(text: String) -> Event {
    Event::Problem { text }
}

/// The sentence for work that panicked while `what` ("describing the fight").
fn panic_sentence(what: &str) -> String {
    format!("Something went wrong while {what}, so it stopped. Try again.")
}

impl Ctx {
    pub(crate) fn new(events: EventSender, config: EngineConfig) -> Ctx {
        Ctx {
            events,
            workers: Arc::new(AtomicUsize::new(config.workers)),
            config: Arc::new(config),
            preview_busy: Arc::new(AtomicBool::new(false)),
            stop: Arc::new(AtomicBool::new(false)),
            shared: Arc::default(),
        }
    }

    /// Sends an event to the interface and wakes it.
    pub fn emit(&self, e: Event) {
        self.events.emit(e);
    }

    /// Threads the engine may use now, at least 1.
    pub fn workers(&self) -> usize {
        self.workers.load(Ordering::SeqCst).max(1)
    }

    /// Whether the preview is drawing.
    pub fn preview_busy(&self) -> bool {
        self.preview_busy.load(Ordering::SeqCst)
    }

    /// Whether the engine is ending: long work stops at its next check.
    pub fn stopping(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    /// A new cancel flag, false, for one piece of work: the handler sets it to stop that work (a `Stop...` or a
    /// newer command), and the engine's ending sets every flag still in use. A flag made while the engine is ending
    /// starts set. It is a plain `AtomicBool`, so it can be handed to `bf_story`'s `stats_until` and the like.
    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        let mut shared = lock(&self.shared);
        let flag = Arc::new(AtomicBool::new(self.stopping()));
        shared.cancels.retain(|w| w.strong_count() > 0);
        shared.cancels.push(Arc::downgrade(&flag));
        flag
    }

    /// Runs `work` on a new thread named `name`, with a clone of this context. `what` says in words what it does,
    /// such as "searching seeds". If `work` panics, the panic is caught and `on_panic` is sent with the sentence
    /// "Something went wrong while {what}, so it stopped. Try again." (so a take can answer `TakeFailed` and a job
    /// its own failure); `on_panic` is also sent, with a sentence saying so, if the PC cannot start the thread.
    ///
    /// Returns false, starting nothing and sending nothing, once the engine is ending.
    pub fn spawn<W, P>(&self, name: &str, what: &str, work: W, on_panic: P) -> bool
    where
        W: FnOnce(Ctx) + Send + 'static,
        P: FnOnce(String) -> Event + Send + 'static,
    {
        let mut shared = lock(&self.shared);
        if self.stopping() {
            return false;
        }
        shared.threads.retain(|t| !t.is_finished());
        let on_panic = Arc::new(Mutex::new(Some(on_panic)));
        let (ctx, answer, words) = (self.clone(), on_panic.clone(), what.to_string());
        let started = thread::Builder::new().name(name.to_string()).spawn(move || {
            let events = ctx.events.clone();
            if catch_unwind(AssertUnwindSafe(move || work(ctx))).is_err()
                && let Some(answer) = lock(&answer).take()
            {
                events.emit(answer(panic_sentence(&words)));
            }
        });
        match started {
            Ok(t) => {
                shared.threads.push(t);
                true
            }
            Err(_) => {
                drop(shared);
                if let Some(answer) = lock(&on_panic).take() {
                    self.emit(answer(format!(
                        "The engine could not start a thread for {what}, so it did not start. Close other programs \
                         and try again."
                    )));
                }
                false
            }
        }
    }

    /// The engine is ending: set the stop flag and every cancel flag, start nothing more, and wait up to
    /// [`STOP_WAIT`] for the threads already started.
    fn shutdown(&self) {
        let threads = {
            let mut shared = lock(&self.shared);
            self.stop.store(true, Ordering::SeqCst);
            for flag in shared.cancels.drain(..).filter_map(|w| w.upgrade()) {
                flag.store(true, Ordering::SeqCst);
            }
            std::mem::take(&mut shared.threads)
        };
        let deadline = Instant::now() + STOP_WAIT;
        let mut waiting = threads;
        while !waiting.is_empty() && Instant::now() < deadline {
            let (done, running): (Vec<_>, Vec<_>) = waiting.into_iter().partition(|t| t.is_finished());
            for t in done {
                // A panic there was already caught and answered inside the thread.
                _ = t.join();
            }
            waiting = running;
            if !waiting.is_empty() {
                thread::sleep(Duration::from_millis(2));
            }
        }
    }
}

/// Starts the engine: the coordinator thread, answering through `waker`'s seam. Returns at once.
pub fn spawn(waker: Waker, config: EngineConfig) -> EngineHandle {
    let (handle, side) = seam::channel(waker);
    let events = side.events.clone();
    let started = thread::Builder::new()
        .name("engine".to_string())
        .spawn(move || coordinate(side, config));
    if started.is_err() {
        events.emit(problem(
            "The engine could not start, so nothing can be balanced, searched or rendered. Close other programs and \
             start the app again."
                .to_string(),
        ));
    }
    handle
}

/// What the five handlers keep between commands.
#[derive(Default)]
struct Handlers {
    balance: balance::State,
    search: search::State,
    takes: takes::State,
    jobs: jobs::State,
    files: files::State,
}

/// The coordinator: takes commands until `Shutdown` or until the interface has gone, then ends the engine.
fn coordinate(side: EngineSide, config: EngineConfig) {
    let ctx = Ctx::new(side.events.clone(), config);
    let mut handlers = Handlers::default();
    let describer = Describer::default();
    while let Ok(command) = side.commands.recv() {
        if command == Command::Shutdown {
            break;
        }
        let routed = catch_unwind(AssertUnwindSafe(|| route(&mut handlers, &describer, &ctx, command)));
        if routed.is_err() {
            ctx.emit(problem(
                "Something went wrong inside the engine, so the last request was dropped. Try again.".to_string(),
            ));
        }
    }
    describer.close();
    ctx.shutdown();
}

/// Hands one command to the code that answers it.
fn route(h: &mut Handlers, describer: &Describer, ctx: &Ctx, command: Command) {
    match command {
        Command::LoadRoster => {
            ctx.spawn(
                "engine: roster",
                "loading the fighters",
                |ctx| ctx.emit(Event::Roster(content::roster())),
                problem,
            );
        }
        Command::DescribeFight { r#gen, draft } => describer.ask(ctx, r#gen, draft),
        Command::SetWorkers(n) => ctx.workers.store(n, Ordering::SeqCst),
        Command::PreviewBusy(busy) => ctx.preview_busy.store(busy, Ordering::SeqCst),
        // The coordinator ends before routing it.
        Command::Shutdown => {}
        c @ (Command::AutoBalance { .. } | Command::Measure { .. } | Command::StopBalance) => {
            balance::handle(&mut h.balance, ctx, c)
        }
        c @ (Command::StartSearch { .. } | Command::StopSearch | Command::SetFilters { .. }) => {
            search::handle(&mut h.search, ctx, c)
        }
        c @ Command::LoadTake { .. } => takes::handle(&mut h.takes, ctx, c),
        c @ (Command::Enqueue { .. } | Command::RunJob { .. } | Command::CancelJob { .. } | Command::CheckTools) => {
            jobs::handle(&mut h.jobs, ctx, c)
        }
        c @ (Command::ScanLibrary { .. }
        | Command::OpenFight { .. }
        | Command::OpenFightText { .. }
        | Command::CheckFolder { .. }
        | Command::CreateFolder { .. }
        | Command::PickFolder { .. }
        | Command::RevealFolder { .. }) => files::handle(&mut h.files, ctx, c),
    }
}

/// The describing thread's mailbox: only the newest request waits.
#[derive(Default)]
struct Mailbox {
    next: Option<(Gen, Draft)>,
    closed: bool,
}

/// Describes fights on one thread of its own, started on the first `DescribeFight`.
#[derive(Default)]
struct Describer {
    mailbox: Arc<(Mutex<Mailbox>, Condvar)>,
    started: std::cell::Cell<bool>,
}

impl Describer {
    /// Leaves the request for the describing thread, replacing any older one still waiting.
    fn ask(&self, ctx: &Ctx, r#gen: Gen, draft: Draft) {
        let (mailbox, bell) = &*self.mailbox;
        lock(mailbox).next = Some((r#gen, draft));
        bell.notify_all();
        if !self.started.get() {
            let mailbox = self.mailbox.clone();
            let started = ctx.spawn(
                "engine: describe",
                "describing the fight",
                move |ctx| describe_loop(&ctx, &mailbox),
                problem,
            );
            self.started.set(started);
        }
    }

    /// No more requests: the describing thread ends after the one it is drawing.
    fn close(&self) {
        let (mailbox, bell) = &*self.mailbox;
        lock(mailbox).closed = true;
        bell.notify_all();
    }
}

/// Answers the newest request, again and again, until the mailbox is closed or the engine is ending.
fn describe_loop(ctx: &Ctx, mailbox: &(Mutex<Mailbox>, Condvar)) {
    let (mailbox, bell) = mailbox;
    loop {
        let (generation, draft) = {
            let mut m = lock(mailbox);
            loop {
                if m.closed || ctx.stopping() {
                    return;
                }
                if let Some(request) = m.next.take() {
                    break request;
                }
                m = bell.wait(m).unwrap_or_else(|e| e.into_inner());
            }
        };
        match catch_unwind(AssertUnwindSafe(|| content::describe(generation, &draft))) {
            // A newer request is waiting: its answer replaces this one.
            Ok(_) if lock(mailbox).next.is_some() => {}
            Ok(answer) => ctx.emit(answer),
            Err(_) => ctx.emit(problem(panic_sentence("describing the fight"))),
        }
    }
}

/// Waits up to `seconds` for an event that `want` accepts and returns it, polling every 10 ms; the events it
/// passes over are dropped.
///
/// # Panics
/// When the time runs out, listing the events it saw.
#[cfg(test)]
pub(crate) fn wait_for(handle: &EngineHandle, seconds: f64, want: impl Fn(&Event) -> bool) -> Event {
    let deadline = Instant::now() + Duration::from_secs_f64(seconds);
    let mut seen: Vec<String> = Vec::new();
    loop {
        while let Some(e) = handle.poll() {
            if want(&e) {
                return e;
            }
            // Pictures print every byte: keep the start of each.
            seen.push(format!("{e:?}").chars().take(160).collect());
        }
        assert!(
            Instant::now() < deadline,
            "the awaited event did not come within {seconds} s; saw {seen:#?}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

/// An engine for a test: 4 workers, storage under a new folder in the system's temp folder (created empty; the
/// test removes it), a waker that does nothing.
#[cfg(test)]
pub(crate) fn test_engine() -> (EngineHandle, std::path::PathBuf) {
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!("bf-app-engine-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&root).expect("the test's temp folder");
    let config = EngineConfig {
        workers: 4,
        paths: Paths::under(&root),
        clip: None,
        search_limit: None,
    };
    (spawn(Arc::new(|| {}), config), root)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::Settings;
    use crate::model::queue::Queue;
    use crate::state::AppState;

    /// The draft the app starts from with the real roster (FLUTE vs TROMBONE, 34 bars, the trombone on bass).
    pub(crate) fn starting_draft() -> Draft {
        let settings = Settings {
            output_folder: std::path::PathBuf::from("Videos"),
            workers: 4,
        };
        let (mut s, _) = AppState::new(settings, Queue::new(), Vec::new(), 8);
        s.apply(Event::Roster(content::roster()));
        s.vm.draft.clone()
    }

    /// Waits up to 5 s until nothing but the test holds `probe`, so every engine thread has ended.
    fn wait_until_alone(probe: &Waker) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Arc::strong_count(probe) > 1 {
            assert!(Instant::now() < deadline, "an engine thread is still running");
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn is_info(e: &Event, generation: Gen) -> bool {
        matches!(e, Event::FightInfo { r#gen, .. } if *r#gen == generation)
    }

    #[test]
    fn the_starting_draft() {
        let d = starting_draft();
        assert_eq!(d.fighters, ["FLUTE", "TROMBONE"]);
        assert_eq!(
            (d.theme.as_str(), d.bars, d.bass, d.seed),
            ("big_top", 34, Some(1), None)
        );
    }

    #[test]
    fn the_engine_answers_the_roster_and_the_fight() {
        let (engine, root) = test_engine();
        engine.send(Command::LoadRoster);
        let Event::Roster(roster) = wait_for(&engine, 5.0, |e| matches!(e, Event::Roster(_))) else {
            unreachable!()
        };
        assert_eq!(roster, content::roster());
        let draft = starting_draft();
        engine.send(Command::DescribeFight {
            r#gen: 4,
            draft: draft.clone(),
        });
        let Event::FightInfo {
            r#gen,
            pair_rules,
            captions,
            title_card,
            problem,
        } = wait_for(&engine, 5.0, |e| matches!(e, Event::FightInfo { .. }))
        else {
            unreachable!()
        };
        assert_eq!(r#gen, 4);
        assert_eq!(pair_rules, ["A live lance knocks a risen cobra flat."]);
        let drafted = bf_content::draft_captions("FLUTE", "TROMBONE");
        assert_eq!(captions, convert::caption_texts(&drafted));
        assert!(captions.instagram.starts_with(&drafted.caption));
        let card = title_card.expect("a title card");
        assert_eq!((card.w, card.h), (360, 640));
        assert_eq!(problem, None);
        engine.send(Command::Shutdown);
        std::fs::remove_dir(&root).expect("the engine left its temp folder empty");
    }

    #[test]
    fn the_newest_description_wins() {
        let (engine, root) = test_engine();
        let mut d = starting_draft();
        for generation in 1..=6 {
            d.fighters.swap(0, 1);
            engine.send(Command::DescribeFight {
                r#gen: generation,
                draft: d.clone(),
            });
        }
        let gens = std::cell::RefCell::new(Vec::new());
        wait_for(&engine, 10.0, |e| {
            if let Event::FightInfo { r#gen, .. } = e {
                gens.borrow_mut().push(*r#gen);
            }
            is_info(e, 6)
        });
        // in the order asked, ending with the newest; the ones overtaken while waiting are skipped
        let gens = gens.into_inner();
        assert!(gens.windows(2).all(|w| w[0] < w[1]), "{gens:?}");
        assert_eq!(gens.last(), Some(&6));
        // nothing older comes after it
        engine.send(Command::LoadRoster);
        wait_for(&engine, 5.0, |e| {
            assert!(!matches!(e, Event::FightInfo { .. }), "a late description");
            matches!(e, Event::Roster(_))
        });
        drop(engine);
        std::fs::remove_dir(&root).expect("the engine left its temp folder empty");
    }

    #[test]
    fn shutdown_ends_every_thread() {
        let waker: Waker = Arc::new(|| {});
        let config = EngineConfig {
            workers: 2,
            paths: Paths::under(std::path::Path::new("unused")),
            clip: None,
            search_limit: None,
        };
        let engine = spawn(waker.clone(), config);
        engine.send(Command::LoadRoster);
        engine.send(Command::DescribeFight {
            r#gen: 1,
            draft: starting_draft(),
        });
        engine.send(Command::Shutdown);
        wait_until_alone(&waker);
        // a command after the end is dropped quietly
        engine.send(Command::LoadRoster);
    }

    #[test]
    fn dropping_the_handle_ends_every_thread() {
        let waker: Waker = Arc::new(|| {});
        let config = EngineConfig {
            workers: 2,
            paths: Paths::under(std::path::Path::new("unused")),
            clip: None,
            search_limit: None,
        };
        let engine = spawn(waker.clone(), config);
        engine.send(Command::DescribeFight {
            r#gen: 1,
            draft: starting_draft(),
        });
        drop(engine);
        wait_until_alone(&waker);
    }

    #[test]
    fn every_command_is_taken_and_the_engine_keeps_answering() {
        let (engine, root) = test_engine();
        let d = starting_draft();
        let unused = root.join("unused");
        let commands = [
            Command::SetWorkers(0),
            Command::SetWorkers(3),
            Command::PreviewBusy(true),
            Command::PreviewBusy(false),
            Command::AutoBalance {
                r#gen: 1,
                draft: d.clone(),
            },
            Command::Measure {
                r#gen: 1,
                draft: d.clone(),
            },
            Command::StopBalance,
            Command::StartSearch {
                r#gen: 1,
                draft: d.clone(),
                filters: Default::default(),
            },
            Command::SetFilters {
                r#gen: 1,
                filters: Default::default(),
            },
            Command::StopSearch,
            Command::LoadTake {
                take: 1,
                draft: d.clone(),
            },
            Command::Enqueue {
                draft: d.clone(),
                out_root: unused.clone(),
            },
            Command::RunJob {
                id: 1,
                fight_toml: String::new(),
                out_root: unused.clone(),
            },
            Command::CancelJob { id: 1 },
            Command::CheckTools,
            Command::ScanLibrary { root: unused.clone() },
            Command::OpenFight { path: unused.clone() },
            Command::OpenFightText { toml: String::new() },
            Command::CheckFolder {
                root: unused.clone(),
                name: String::new(),
            },
            Command::CreateFolder { root: unused.clone() },
            Command::PickFolder { start: unused.clone() },
            Command::RevealFolder { path: unused.clone() },
            Command::LoadRoster,
        ];
        for c in commands {
            engine.send(c);
        }
        wait_for(&engine, 5.0, |e| {
            assert!(!matches!(e, Event::Problem { .. }), "{e:?}");
            matches!(e, Event::Roster(_))
        });
        assert!(!unused.exists(), "nothing was written");
        engine.send(Command::Shutdown);
        std::fs::remove_dir(&root).expect("the engine left its temp folder empty");
    }

    #[test]
    fn a_panic_on_a_worker_becomes_a_problem() {
        let (handle, side) = seam::channel(Arc::new(|| {}));
        let config = EngineConfig {
            workers: 0,
            paths: Paths::under(std::path::Path::new("unused")),
            clip: None,
            search_limit: None,
        };
        let ctx = Ctx::new(side.events.clone(), config);
        assert_eq!(ctx.workers(), 1, "at least one worker");
        assert!(ctx.spawn("test: panics", "testing the engine", |_| panic!("on purpose"), problem));
        let Event::Problem { text } = wait_for(&handle, 5.0, |e| matches!(e, Event::Problem { .. })) else {
            unreachable!()
        };
        assert_eq!(
            text,
            "Something went wrong while testing the engine, so it stopped. Try again."
        );
        // the context still starts work, and work that ends well sends only what it sends
        assert!(ctx.spawn(
            "test: answers",
            "testing the engine",
            |ctx| ctx.emit(problem("done".to_string())),
            |_| problem("not this".to_string()),
        ));
        let Event::Problem { text } = wait_for(&handle, 5.0, |_| true) else {
            unreachable!()
        };
        assert_eq!(text, "done");
        // the ending sets every cancel flag still in use and refuses new work
        let flag = ctx.cancel_flag();
        let running = ctx.cancel_flag();
        let seen = running.clone();
        assert!(ctx.spawn(
            "test: waits",
            "testing the engine",
            move |_| {
                while !seen.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(1));
                }
            },
            problem,
        ));
        assert!(!flag.load(Ordering::SeqCst) && !ctx.stopping());
        let started = Instant::now();
        ctx.shutdown();
        assert!(started.elapsed() < STOP_WAIT, "the waiting thread saw its flag");
        assert!(flag.load(Ordering::SeqCst) && running.load(Ordering::SeqCst) && ctx.stopping());
        assert!(
            ctx.cancel_flag().load(Ordering::SeqCst),
            "a flag made while ending starts set"
        );
        assert!(!ctx.spawn("test: late", "testing the engine", |_| {}, problem));
        assert!(handle.poll().is_none());
    }
}
