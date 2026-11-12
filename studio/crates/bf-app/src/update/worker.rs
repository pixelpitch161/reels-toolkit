//! The update worker (plan M6, Task 12): a thread of its own, like `store::Writer`, so the interface never waits.
//! `send` only queues a request (and `StopBuild` only raises a flag), and `view` only copies the current state, so
//! neither ever waits on git, cargo or the disk.
//!
//! One request runs at a time, so two updates never run at once in this app; `GitSource`'s lock keeps a second copy
//! of the app (another `--data-dir`) out of the same source copy and build folder. The build runs at below-normal
//! priority and may run while a video renders; "Restart now" and "Go back" are refused by the interface while one
//! renders (`RENDERING`, Task 13). Closing the app stops a build (the flag is raised, and the build's programs are in
//! a job that ends with the app).
//!
//! Task 13 starts it once with `Worker::start(Setup::for_this_copy(), waker)` and sends: "Look again" and "Try
//! again" `Check`, "Build version {v}" `Build`, "Stop building" `StopBuild`, "Restart now" `Restart`, "Go back to
//! version {w}" `GoBack`, "Open the guide" `OpenGuide`, "Remove the app" `Remove`. While the state is `Building`, it
//! repaints once a second (the seconds come from `view`). When `view().close` is set, the app saves and closes.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bf_pack::version::Version;

use super::stage::{NEXT, check_staged, staged_version};
use super::{
    Built, Installed, Release, Source, UpdateState, UpdateView, build_failed, check_failed, check_staged_failed,
    stage_failed,
};
use crate::seam::Waker;

/// What the interface asks of the worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    Check,
    Build,
    StopBuild,
    Restart,
    GoBack,
    OpenGuide,
    Remove,
}

/// A step that the worker calls with the installed copy.
pub type InstalledStep = Box<dyn Fn(&Installed) -> Result<(), String> + Send>;
/// A step that the worker calls with a file.
pub type PathStep = Box<dyn Fn(&Path) -> Result<(), String> + Send>;
/// Packaging and checking a build: `Ok` once `.next` holds the checked new version; `Err` is a whole sentence.
pub type StageStep = Box<dyn Fn(&Installed, &Built, &Release, &Version) -> Result<(), String> + Send>;

/// Everything the worker uses. `for_this_copy` is the real one; tests give their own steps.
pub struct Setup {
    /// The installed copy, or `None` for a build that was not installed (it then never updates).
    pub installed: Option<Installed>,
    /// Where new versions come from, or the sentence saying why none can (`NO_TOOLS`).
    pub source: Result<Box<dyn Source>, String>,
    /// The running version.
    pub current: Version,
    /// The fault reports folder, shown in Settings.
    pub logs: Option<PathBuf>,
    pub stage: StageStep,
    /// "Restart now": `swap::launch_finish`.
    pub restart: InstalledStep,
    /// "Go back": `swap::go_back`.
    pub go_back: InstalledStep,
    /// Opens the guide's page with Windows' default program for it.
    pub open: PathStep,
    /// Removes the app (Task 11's `remove_app`); gives the sentences to show before closing.
    pub remove: Box<dyn Fn() -> Result<Vec<String>, String> + Send>,
    /// Set by the interface while a video renders (Task 13, through `Worker::set_rendering`): Restart, Go back and
    /// Remove are then refused with `RENDERING`.
    pub rendering: Arc<AtomicBool>,
}

impl Setup {
    /// The running copy as it is on this PC: its install (if any), route A, the real folders.
    pub fn for_this_copy() -> Setup {
        let installed = super::installed();
        let source: Result<Box<dyn Source>, String> = match &installed {
            Some(inst) => super::git::GitSource::new(inst.config.clone()).map(|s| Box::new(s) as Box<dyn Source>),
            None => Err(String::new()),
        };
        let current = super::running_version().unwrap_or(Version {
            major: 0,
            minor: 0,
            patch: 0,
            rc: None,
        });
        Setup {
            installed,
            source,
            current,
            logs: crate::store::Paths::system().map(|p| p.logs),
            stage: Box::new(stage_and_check),
            restart: Box::new(super::swap::launch_finish),
            go_back: Box::new(super::swap::go_back),
            open: Box::new(open_with_windows),
            remove: Box::new(|| {
                let layout = bf_pack::install::Layout::system()
                    .ok_or("Windows did not say where the app's folders are, so nothing was removed.")?;
                let exe = std::env::current_exe().map_err(|e| format!("This program's own file is not known: {e}."))?;
                bf_pack::remove::remove_app(&layout, &exe)
                    .map(|r| bf_pack::remove::removed_words(&r))
                    .map_err(|e| e.to_string())
            }),
            rendering: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// The real staging: `stage::stage`, then `stage::check_staged`, each failure in words.
fn stage_and_check(inst: &Installed, built: &Built, r: &Release, current: &Version) -> Result<(), String> {
    let (v, w) = (r.version.to_string(), current.to_string());
    let next = super::stage::stage(inst, &built.exe, &built.tree).map_err(|e| stage_failed(&v, &w, &e))?;
    check_staged(&next, r).map_err(|e| check_staged_failed(&v, &w, &e))
}

/// Opens `file` with Windows' default program for it, through `explorer.exe <file>` from Windows' own folder (never
/// from `PATH`). Only an existing file is opened.
pub fn open_with_windows(file: &Path) -> Result<(), String> {
    if !file.is_file() || bf_pack::package::is_link(file) {
        return Err(format!("{} is not there, so it could not be opened.", file.display()));
    }
    let root = std::env::var_os("SystemRoot").unwrap_or_default();
    let text = root.to_string_lossy();
    let b = text.as_bytes();
    let drive_windows =
        b.len() == 10 && b[0].is_ascii_alphabetic() && b[1..3] == *b":\\" && text[3..].eq_ignore_ascii_case("Windows");
    let explorer = PathBuf::from(&root).join("explorer.exe");
    if !drive_windows || !explorer.is_file() {
        return Err("Windows' own explorer.exe was not found, so the file could not be opened.".to_string());
    }
    let dir = file.parent().unwrap_or(file);
    bf_pack::run::start_detached(&explorer, &[file.as_os_str()], dir).map(|_| ())
}

/// Where the worker stands (the interface sees it as `UpdateView`).
#[derive(Clone)]
enum State {
    NotInstalled,
    Checking,
    UpToDate,
    Available(Release),
    Building {
        release: Release,
        started: Instant,
        doing: String,
    },
    Ready(Release),
    /// The release found is the version kept in `.previous` (after going back), and it still verifies: "Restart
    /// now" brings it back as it is, without building it again.
    ReadyKept(Release),
    Failed(String),
}

struct Inner {
    state: State,
    previous: Option<String>,
    close: Option<Vec<String>>,
    note: Option<String>,
}

/// The update worker: one thread, its requests, and what it shows.
pub struct Worker {
    requests: Sender<(Request, Instant)>,
    inner: Arc<Mutex<Inner>>,
    cancel: Arc<AtomicBool>,
    reports: Option<PathBuf>,
    guide: Option<PathBuf>,
    rendering: Arc<AtomicBool>,
    running: String,
}

impl Worker {
    /// Starts the worker. For an installed copy it looks for a newer version at once.
    pub fn start(setup: Setup, waker: Waker) -> Worker {
        let installed = setup.installed.is_some();
        let inner = Arc::new(Mutex::new(Inner {
            state: if installed {
                State::Checking
            } else {
                State::NotInstalled
            },
            previous: setup
                .installed
                .as_ref()
                .and_then(|i| super::swap::previous_version(&i.dir)),
            close: None,
            note: None,
        }));
        let cancel = Arc::new(AtomicBool::new(false));
        let guide = setup
            .installed
            .as_ref()
            .map(|i| i.dir.join(bf_pack::package::GUIDE_DIR).join("index.html"))
            .filter(|g| g.is_file());
        let reports = setup.logs.clone();
        let rendering = setup.rendering.clone();
        let running = setup.current.to_string();
        let (requests, rx) = channel();
        if installed {
            let _ = requests.send((Request::Check, Instant::now()));
        }
        let (i, c) = (inner.clone(), cancel.clone());
        let g = guide.clone();
        let spawned = std::thread::Builder::new()
            .name("update".to_string())
            .spawn(move || work(setup, rx, &i, &c, g.as_deref(), &waker));
        if spawned.is_err() {
            inner.lock().unwrap_or_else(|p| p.into_inner()).state = State::Failed(
                "The update helper could not be started, so updates are off until the next start.".into(),
            );
        }
        Worker {
            requests,
            inner,
            cancel,
            reports,
            guide,
            rendering,
            running,
        }
    }

    /// Tells the worker whether a video is rendering (Restart, Go back and Remove wait while one is).
    pub fn set_rendering(&self, on: bool) {
        self.rendering.store(on, Ordering::Relaxed);
    }

    /// Queues a request; never waits. `StopBuild` takes effect at once.
    pub fn send(&self, r: Request) {
        if r == Request::StopBuild {
            self.cancel.store(true, Ordering::Relaxed);
        }
        let _ = self.requests.send((r, Instant::now()));
    }

    /// What to show now.
    pub fn view(&self) -> UpdateView {
        let inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let state = match &inner.state {
            State::NotInstalled => UpdateState::NotInstalled,
            State::Checking => UpdateState::Checking,
            State::UpToDate => UpdateState::UpToDate,
            State::Available(r) => UpdateState::Available {
                version: r.version.to_string(),
            },
            State::Building {
                release,
                started,
                doing,
            } => UpdateState::Building {
                version: release.version.to_string(),
                seconds: started.elapsed().as_secs(),
                doing: doing.clone(),
            },
            State::Ready(r) | State::ReadyKept(r) => UpdateState::Ready {
                version: r.version.to_string(),
            },
            State::Failed(text) => UpdateState::Failed { text: text.clone() },
        };
        UpdateView {
            state,
            previous_button: inner
                .previous
                .as_deref()
                .map(|w| super::swap::kept_button(w, &self.running)),
            previous: inner.previous.clone(),
            reports: self.reports.clone(),
            guide: self.guide.clone(),
            close: inner.close.clone(),
            note: inner.note.clone(),
        }
    }
}

impl Drop for Worker {
    /// Closing the app stops a build that is running; the thread is not waited for.
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// The worker's thread: one request at a time until the app closes.
fn work(
    setup: Setup,
    rx: Receiver<(Request, Instant)>,
    inner: &Mutex<Inner>,
    cancel: &AtomicBool,
    guide: Option<&Path>,
    waker: &Waker,
) {
    let set = |f: &dyn Fn(&mut Inner)| {
        f(&mut inner.lock().unwrap_or_else(|p| p.into_inner()));
        waker();
    };
    let state = || inner.lock().unwrap_or_else(|p| p.into_inner()).state.clone();
    let Some(inst) = setup.installed.as_ref() else {
        // Not installed: nothing ever changes, and every request is let go.
        for _ in rx {}
        return;
    };
    // A "Build" sent while a build was running (a second click) is not a new request once that build has ended.
    let mut build_ended: Option<Instant> = None;
    for (r, sent) in rx {
        set(&|i| i.note = None);
        if r == Request::Build && build_ended.is_some_and(|ended| sent < ended) {
            continue;
        }
        match r {
            Request::Check => {
                if matches!(state(), State::Building { .. }) {
                    continue;
                }
                set(&|i| i.state = State::Checking);
                let found = match &setup.source {
                    Err(text) => State::Failed(text.clone()),
                    Ok(source) => match source.newest(&setup.current) {
                        Ok(None) => State::UpToDate,
                        Ok(Some(r)) => {
                            let v = r.version.to_string();
                            // A version built earlier and staged whole is offered for a restart again; so is the
                            // version kept in .previous (after going back), when it still verifies.
                            let staged = staged_version(&inst.dir) == Some(v.clone())
                                && check_staged(&inst.dir.join(NEXT), &r).is_ok();
                            let kept = !staged
                                && super::swap::previous_version(&inst.dir) == Some(v.clone())
                                && super::stage::check_named(
                                    &inst.dir.join(super::swap::PREVIOUS),
                                    &v,
                                    r.commit.get(..7),
                                    true,
                                )
                                .is_ok();
                            if staged {
                                State::Ready(r)
                            } else if kept {
                                State::ReadyKept(r)
                            } else {
                                State::Available(r)
                            }
                        }
                        Err(e) => State::Failed(check_failed(&e)),
                    },
                };
                let previous = super::swap::previous_version(&inst.dir);
                set(&|i| {
                    i.state = found.clone();
                    i.previous = previous.clone();
                });
            }
            Request::Build => {
                let State::Available(release) = state() else { continue };
                let Ok(source) = &setup.source else { continue };
                cancel.store(false, Ordering::Relaxed);
                let begun = State::Building {
                    release: release.clone(),
                    started: Instant::now(),
                    doing: "Getting ready.".to_string(),
                };
                set(&|i| i.state = begun.clone());
                let progress = |text: String| {
                    set(&|i| {
                        if let State::Building { doing, .. } = &mut i.state {
                            doing.clone_from(&text);
                        }
                    })
                };
                let (v, w) = (release.version.to_string(), setup.current.to_string());
                let built = source.build(&release, &progress, cancel);
                let stopped = cancel.load(Ordering::Relaxed);
                let after = match built {
                    Err(_) if stopped => State::Available(release.clone()),
                    Err(e) => State::Failed(build_failed(&v, &w, &e)),
                    Ok(built) => {
                        progress("Checking the new program.".to_string());
                        let notes = built.notes.join(" ");
                        if !notes.is_empty() {
                            set(&|i| i.note = Some(notes.clone()));
                        }
                        // `built` (and with it the source's lock) is held until the stage is done.
                        let staged = (setup.stage)(inst, &built, &release, &setup.current);
                        drop(built);
                        match staged {
                            Ok(()) if cancel.load(Ordering::Relaxed) => {
                                let _ = super::stage::clear_next(&inst.dir);
                                State::Available(release.clone())
                            }
                            Ok(()) => State::Ready(release.clone()),
                            Err(text) => State::Failed(text),
                        }
                    }
                };
                cancel.store(false, Ordering::Relaxed);
                build_ended = Some(Instant::now());
                set(&|i| i.state = after.clone());
            }
            Request::StopBuild => {}
            Request::Restart => {
                let kept = matches!(state(), State::ReadyKept(_));
                if !matches!(state(), State::Ready(_) | State::ReadyKept(_)) {
                    continue;
                }
                if setup.rendering.load(Ordering::Relaxed) {
                    set(&|i| i.note = Some(super::RENDERING.to_string()));
                    continue;
                }
                let step = if kept { &setup.go_back } else { &setup.restart };
                match step(inst) {
                    Ok(()) => set(&|i| i.close = Some(Vec::new())),
                    Err(text) => set(&|i| i.state = State::Failed(text.clone())),
                }
            }
            Request::GoBack => {
                if inner.lock().unwrap_or_else(|p| p.into_inner()).previous.is_none()
                    || matches!(state(), State::Building { .. })
                {
                    continue;
                }
                if setup.rendering.load(Ordering::Relaxed) {
                    set(&|i| i.note = Some(super::RENDERING.to_string()));
                    continue;
                }
                match (setup.go_back)(inst) {
                    Ok(()) => set(&|i| i.close = Some(Vec::new())),
                    Err(text) => set(&|i| i.note = Some(text.clone())),
                }
            }
            Request::OpenGuide => {
                let opened = match guide {
                    Some(g) => (setup.open)(g),
                    None => Err("This copy has no guide.".to_string()),
                };
                if let Err(text) = opened {
                    set(&|i| i.note = Some(text.clone()));
                }
            }
            Request::Remove if setup.rendering.load(Ordering::Relaxed) => {
                set(&|i| i.note = Some(super::RENDERING.to_string()));
            }
            Request::Remove => match (setup.remove)() {
                Ok(words) => set(&|i| i.close = Some(words.clone())),
                Err(text) => set(&|i| i.note = Some(text.clone())),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::git::{GitSource, find_cargo, standin};
    use super::super::stage::tests::{installed, snapshot};
    use super::super::testkit::{TempDir, git};
    use super::*;
    use bf_pack::version::parse_version;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    fn v(s: &str) -> Version {
        parse_version(s).unwrap()
    }

    fn release(s: &str) -> Release {
        Release {
            tag: format!("studio-v{s}"),
            version: v(s),
            commit: "c".repeat(40),
        }
    }

    type BuildFn = dyn Fn(&Release, &dyn Fn(String), &AtomicBool) -> Result<Built, String> + Send + Sync;

    /// A source whose answers the test gives, counting builds and how many ran at once.
    struct Fake {
        newest: Result<Option<Release>, String>,
        build: Arc<BuildFn>,
        now: Arc<AtomicUsize>,
        most: Arc<AtomicUsize>,
        builds: Arc<AtomicUsize>,
    }

    impl Source for Fake {
        fn newest(&self, _current: &Version) -> Result<Option<Release>, String> {
            self.newest.clone()
        }
        fn build(&self, r: &Release, progress: &dyn Fn(String), cancel: &AtomicBool) -> Result<Built, String> {
            let n = self.now.fetch_add(1, Ordering::SeqCst) + 1;
            self.most.fetch_max(n, Ordering::SeqCst);
            self.builds.fetch_add(1, Ordering::SeqCst);
            let out = (self.build)(r, progress, cancel);
            self.now.fetch_sub(1, Ordering::SeqCst);
            out
        }
    }

    struct Counters {
        most: Arc<AtomicUsize>,
        builds: Arc<AtomicUsize>,
    }

    fn fake(newest: Result<Option<Release>, String>, build: Arc<BuildFn>) -> (Box<dyn Source>, Counters) {
        let most = Arc::new(AtomicUsize::new(0));
        let builds = Arc::new(AtomicUsize::new(0));
        let f = Fake {
            newest,
            build,
            now: Arc::new(AtomicUsize::new(0)),
            most: most.clone(),
            builds: builds.clone(),
        };
        (Box::new(f), Counters { most, builds })
    }

    fn quick_build() -> Arc<BuildFn> {
        Arc::new(|_, progress, _| {
            progress("Building bf-app.".into());
            Ok(Built::new(PathBuf::from("built.exe"), PathBuf::from("tree")))
        })
    }

    /// A setup with the given install and source and steps that only record what they were asked.
    fn setup(
        installed: Option<Installed>,
        source: Result<Box<dyn Source>, String>,
        log: Arc<Mutex<Vec<String>>>,
    ) -> Setup {
        let (l1, l2, l3, l4, l5) = (log.clone(), log.clone(), log.clone(), log.clone(), log);
        Setup {
            installed,
            source,
            current: v("0.6.0"),
            logs: Some(PathBuf::from(r"C:\logs")),
            stage: Box::new(move |_, _, r, _| {
                l1.lock().unwrap().push(format!("stage {}", r.version));
                Ok(())
            }),
            restart: Box::new(move |_| {
                l2.lock().unwrap().push("restart".into());
                Ok(())
            }),
            go_back: Box::new(move |_| {
                l3.lock().unwrap().push("go back".into());
                Ok(())
            }),
            open: Box::new(move |p| {
                l4.lock().unwrap().push(format!("open {}", p.display()));
                Ok(())
            }),
            remove: Box::new(move || {
                l5.lock().unwrap().push("remove".into());
                Ok(vec!["BALLFIGHT Studio was removed.".into()])
            }),
            rendering: Arc::new(AtomicBool::new(false)),
        }
    }

    fn no_wake() -> Waker {
        Arc::new(|| {})
    }

    /// Waits up to 10 s (or `secs`) for the view to satisfy `ok`.
    fn until(w: &Worker, secs: u64, ok: impl Fn(&UpdateView) -> bool) -> UpdateView {
        let deadline = Instant::now() + Duration::from_secs(secs);
        loop {
            let view = w.view();
            if ok(&view) || Instant::now() > deadline {
                return view;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn a_fake_install(root: &Path) -> Installed {
        installed(root, "0.6.0", None)
    }

    /// The exe of the install `a_fake_install` made under `root`.
    fn w_dir_exe(root: &Path) -> PathBuf {
        bf_pack::install::Layout::under(&root.join("pc")).exe()
    }

    #[test]
    fn not_installed_says_so() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let (src, _) = fake(Ok(Some(release("0.6.1"))), quick_build());
        let w = Worker::start(setup(None, Ok(src), log.clone()), no_wake());
        for r in [Request::Check, Request::Build, Request::Restart, Request::GoBack] {
            w.send(r);
        }
        std::thread::sleep(Duration::from_millis(100));
        let view = w.view();
        assert_eq!(view.state, UpdateState::NotInstalled);
        assert_eq!((view.previous, view.guide, view.close), (None, None, None));
        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn no_newer_tag_is_up_to_date() {
        let dir = TempDir::new("worker-uptodate");
        let log = Arc::new(Mutex::new(Vec::new()));
        let (src, _) = fake(Ok(None), quick_build());
        let w = Worker::start(setup(Some(a_fake_install(dir.path())), Ok(src), log), no_wake());
        let view = until(&w, 10, |v| v.state != UpdateState::Checking);
        assert_eq!(view.state, UpdateState::UpToDate);
        assert_eq!(view.reports, Some(PathBuf::from(r"C:\logs")));
        assert!(
            view.guide
                .is_some_and(|g| g.ends_with("guide/index.html") || g.ends_with(r"guide\index.html"))
        );
    }

    #[test]
    fn a_newer_tag_is_available_and_builds_to_ready() {
        let dir = TempDir::new("worker-available");
        let log = Arc::new(Mutex::new(Vec::new()));
        let (src, counts) = fake(Ok(Some(release("0.6.1"))), quick_build());
        let w = Worker::start(setup(Some(a_fake_install(dir.path())), Ok(src), log.clone()), no_wake());
        let view = until(&w, 10, |v| v.state != UpdateState::Checking);
        assert_eq!(
            view.state,
            UpdateState::Available {
                version: "0.6.1".into()
            }
        );
        w.send(Request::Build);
        let view = until(&w, 10, |v| matches!(v.state, UpdateState::Ready { .. }));
        assert_eq!(
            view.state,
            UpdateState::Ready {
                version: "0.6.1".into()
            }
        );
        assert_eq!(counts.builds.load(Ordering::SeqCst), 1);
        w.send(Request::Restart);
        let view = until(&w, 10, |v| v.close.is_some());
        assert_eq!(view.close, Some(Vec::new()));
        assert_eq!(*log.lock().unwrap(), ["stage 0.6.1", "restart"]);
    }

    #[test]
    fn a_failed_check_says_so_in_words() {
        let dir = TempDir::new("worker-failed");
        let log = Arc::new(Mutex::new(Vec::new()));
        let (src, _) = fake(
            Err("Git could not fetch the releases. Git said \"no network\".".into()),
            quick_build(),
        );
        let w = Worker::start(setup(Some(a_fake_install(dir.path())), Ok(src), log), no_wake());

        let view = until(&w, 10, |v| v.state != UpdateState::Checking);
        assert_eq!(
            view.state,
            UpdateState::Failed {
                text: "BALLFIGHT Studio could not look for a newer version, so nothing changed. Git could not fetch \
                       the releases. Git said \"no network\"."
                    .into()
            }
        );
        // Without Git or Rust, the sentence says so.
        let inst = super::super::installed_at(&w_dir_exe(dir.path())).unwrap();
        let w = Worker::start(
            setup(Some(inst), Err(super::super::NO_TOOLS.into()), Arc::default()),
            no_wake(),
        );
        let view = until(&w, 10, |v| v.state != UpdateState::Checking);
        assert_eq!(
            view.state,
            UpdateState::Failed {
                text: super::super::NO_TOOLS.into()
            }
        );
    }

    /// A build that runs until it is stopped (at most 30 s), saying so.
    fn endless_build() -> Arc<BuildFn> {
        Arc::new(|_, progress, cancel| {
            progress("Building eframe.".into());
            let deadline = Instant::now() + Duration::from_secs(30);
            while !cancel.load(Ordering::Relaxed) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err("The build was stopped.".into())
        })
    }

    #[test]
    fn the_worker_never_blocks_its_caller() {
        let dir = TempDir::new("worker-blocks");
        let log = Arc::new(Mutex::new(Vec::new()));
        let (src, counts) = fake(Ok(Some(release("0.6.1"))), endless_build());
        let w = Worker::start(setup(Some(a_fake_install(dir.path())), Ok(src), log), no_wake());
        until(&w, 10, |v| v.state != UpdateState::Checking);
        w.send(Request::Build);
        let view = until(
            &w,
            10,
            |v| matches!(&v.state, UpdateState::Building { doing, .. } if doing == "Building eframe."),
        );
        assert!(matches!(view.state, UpdateState::Building { .. }), "{view:?}");
        // While the build runs, sending and looking take no time at all; more builds are not started.
        let mut slowest = Duration::ZERO;
        for r in [
            Request::Check,
            Request::Build,
            Request::Build,
            Request::GoBack,
            Request::Check,
        ] {
            let t = Instant::now();
            w.send(r);
            let _ = w.view();
            slowest = slowest.max(t.elapsed());
        }
        assert!(slowest < Duration::from_millis(1), "{slowest:?}");
        // Stopping it: back to Available within 2 s, and only one build ever ran.
        let t = Instant::now();
        w.send(Request::StopBuild);
        let view = until(&w, 2, |v| matches!(v.state, UpdateState::Available { .. }));
        assert_eq!(
            view.state,
            UpdateState::Available {
                version: "0.6.1".into()
            }
        );
        assert!(t.elapsed() < Duration::from_secs(2));
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(counts.builds.load(Ordering::SeqCst), 1);
        assert_eq!(counts.most.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn nothing_restarts_or_goes_while_a_video_renders() {
        let dir = TempDir::new("worker-render");
        let inst = a_fake_install(dir.path());
        let prev = inst.dir.join(".previous");
        std::fs::create_dir(&prev).unwrap();
        std::fs::copy(inst.dir.join("MANIFEST.txt"), prev.join("MANIFEST.txt")).unwrap();
        let log = Arc::new(Mutex::new(Vec::new()));
        let (src, _) = fake(Ok(Some(release("0.6.1"))), quick_build());
        let w = Worker::start(setup(Some(inst), Ok(src), log.clone()), no_wake());
        until(&w, 10, |v| v.state != UpdateState::Checking);
        w.send(Request::Build);
        until(&w, 10, |v| matches!(v.state, UpdateState::Ready { .. }));
        w.set_rendering(true);
        for r in [Request::Restart, Request::GoBack, Request::Remove] {
            w.send(r);
            let view = until(&w, 10, |v| v.note.is_some());
            assert_eq!(view.note.as_deref(), Some(super::super::RENDERING), "{r:?}");
            assert_eq!(view.close, None);
            // Any next request clears the note.
            w.send(Request::OpenGuide);
            until(&w, 10, |v| v.note.is_none());
        }
        assert!(
            log.lock()
                .unwrap()
                .iter()
                .all(|l| l.starts_with("stage") || l.starts_with("open")),
            "{:?}",
            log.lock().unwrap()
        );
        // When the render is over, the restart goes ahead.
        w.set_rendering(false);
        w.send(Request::Restart);
        let view = until(&w, 10, |v| v.close.is_some());
        assert_eq!(view.close, Some(Vec::new()));
        assert!(log.lock().unwrap().iter().any(|l| l == "restart"));
    }

    #[test]
    fn after_going_back_the_kept_newer_version_comes_back_without_a_build() {
        let Some(g) = git() else { return };
        let Some(cargo) = find_cargo() else { return };
        let dir = TempDir::new("worker-forward");
        // The stand-in of 0.6.1, built from a detached commit so it names that commit, kept in .previous.
        let tree = dir.path().join("tree-0.6.1");
        super::super::testkit::new_repo(&g, &tree);
        standin::write(&tree, "0.6.1", &standin::Kind::default());
        let commit = super::super::testkit::commit_all(&g, &tree, "0.6.1");
        super::super::testkit::git_in(&g, &tree, &["checkout", "-q", "--detach"]);
        let target = dir.path().join("target");
        let never = AtomicBool::new(false);
        let how = bf_pack::run::Family {
            dir: Some(&tree.join("studio")),
            env: &[("CARGO_TARGET_DIR", target.as_os_str())],
            env_remove: &["RUSTUP_TOOLCHAIN"],
            scrub_git: true,
            below_normal: true,
        };
        let o = std::ffi::OsStr::new;
        let ran = bf_pack::run::run_family(
            &cargo,
            &[o("build"), o("--release"), o("-p"), o("bf-app")],
            &how,
            Duration::from_secs(300),
            &never,
            &mut |_| {},
        )
        .unwrap();
        assert!(ran.status.success(), "{:?}", ran.tail);
        let inst = a_fake_install(dir.path());
        let checked = standin::checked_ffmpeg(&dir.path().join("ff"));
        let exe = target.join("release").join(bf_pack::package::EXE_NAME);
        bf_pack::package::package(&exe, &tree, &checked, &inst.dir.join(".previous")).unwrap();

        let log = Arc::new(Mutex::new(Vec::new()));
        let mut r = release("0.6.1");
        r.commit = commit;
        let (src, counts) = fake(Ok(Some(r)), quick_build());
        let w = Worker::start(setup(Some(inst), Ok(src), log.clone()), no_wake());
        let view = until(&w, 30, |v| v.state != UpdateState::Checking);
        assert_eq!(view.previous_button.as_deref(), Some("Go forward to version 0.6.1"));
        assert_eq!(
            view.state,
            UpdateState::Ready {
                version: "0.6.1".into()
            }
        );
        // "Restart now" brings the kept version back as it is: the go-back step, no build.
        w.send(Request::Restart);
        until(&w, 10, |v| v.close.is_some());
        assert_eq!(counts.builds.load(Ordering::SeqCst), 0);
        assert_eq!(*log.lock().unwrap(), ["go back"]);
        assert_eq!(
            super::super::swap::kept_button("0.6.0", "0.6.1"),
            "Go back to version 0.6.0"
        );
    }

    #[test]
    fn go_back_open_and_remove_do_what_they_say() {
        let dir = TempDir::new("worker-steps");
        let inst = a_fake_install(dir.path());
        let log = Arc::new(Mutex::new(Vec::new()));
        // Without a kept version, "Go back" does nothing.
        let (src, _) = fake(Ok(None), quick_build());
        let w = Worker::start(setup(Some(inst.clone()), Ok(src), log.clone()), no_wake());
        until(&w, 10, |v| v.state != UpdateState::Checking);
        w.send(Request::GoBack);
        w.send(Request::OpenGuide);
        until(&w, 10, |_| log.lock().unwrap().len() == 1);
        let guide = inst.dir.join("guide").join("index.html");
        assert_eq!(*log.lock().unwrap(), [format!("open {}", guide.display())]);
        w.send(Request::Remove);
        let view = until(&w, 10, |v| v.close.is_some());
        assert_eq!(view.close, Some(vec!["BALLFIGHT Studio was removed.".to_string()]));
        // With one kept, it does.
        let prev = inst.dir.join(".previous");
        std::fs::create_dir(&prev).unwrap();
        std::fs::copy(inst.dir.join("MANIFEST.txt"), prev.join("MANIFEST.txt")).unwrap();
        let log = Arc::new(Mutex::new(Vec::new()));
        let (src, _) = fake(Ok(None), quick_build());
        let w = Worker::start(setup(Some(inst), Ok(src), log.clone()), no_wake());
        let view = until(&w, 10, |v| v.state != UpdateState::Checking);
        assert_eq!(view.previous.as_deref(), Some("0.6.0"));
        w.send(Request::GoBack);
        let view = until(&w, 10, |v| v.close.is_some());
        assert_eq!(view.close, Some(Vec::new()));
        assert_eq!(*log.lock().unwrap(), ["go back"]);
    }

    /// The real route A for `inst`: real git and cargo on the install's copy of the source, real staging (the
    /// stand-in's pinned ffmpeg comes from `checked`), and steps that touch nothing outside the test's folders.
    fn real(inst: &Installed, checked: PathBuf, repo: &Path) -> Setup {
        // The install's update.toml names an https address; the test's source copy comes from a local repository.
        let mut config = inst.config.clone();
        config.remote = repo.to_string_lossy().into_owned();
        let source = GitSource::for_tests(config, git().unwrap(), find_cargo().unwrap());
        Setup {
            installed: Some(inst.clone()),
            source: Ok(Box::new(source)),
            current: v("0.6.0"),
            logs: None,
            stage: Box::new(move |inst, built, r, current| {
                let (v, w) = (r.version.to_string(), current.to_string());
                let next = super::super::stage::stage_with(inst, &built.exe, &built.tree, &|_| Ok(checked.clone()))
                    .map_err(|e| stage_failed(&v, &w, &e))?;
                check_staged(&next, r).map_err(|e| check_staged_failed(&v, &w, &e))
            }),
            restart: Box::new(|_| Ok(())),
            go_back: Box::new(|_| Ok(())),
            open: Box::new(|_| Ok(())),
            remove: Box::new(|| Err("Not in a test.".into())),
            rendering: Arc::new(AtomicBool::new(false)),
        }
    }

    /// The real route A, end to end with the stand-in: a local repository with a tag, the install's copy of it.
    fn real_setup(root: &Path, kind: standin::Kind) -> Option<(Setup, Installed)> {
        let (Some(g), Some(cargo)) = (git(), find_cargo()) else {
            eprintln!("git or cargo is not on this PC, so this test checks nothing");
            return None;
        };
        let inst = a_fake_install(root);
        let repo = root.join("upstream");
        standin::upstream(&g, &cargo, &repo, &[("0.6.1", kind, true)]);
        standin::source_copy(&g, &repo, &inst.config.source);
        let checked = standin::checked_ffmpeg(&root.join("ffmpeg-checked"));
        Some((real(&inst, checked, &repo), inst))
    }

    #[test]
    fn a_whole_update_is_built_staged_and_checked() {
        let dir = TempDir::new("worker-real");
        let Some((s, inst)) = real_setup(dir.path(), standin::Kind::default()) else {
            return;
        };
        let before = snapshot(&inst.dir);
        let w = Worker::start(s, no_wake());
        let view = until(&w, 60, |v| v.state != UpdateState::Checking);
        assert_eq!(
            view.state,
            UpdateState::Available {
                version: "0.6.1".into()
            }
        );
        let started = Instant::now();
        w.send(Request::Build);
        let view = until(&w, 300, |v| {
            !matches!(v.state, UpdateState::Available { .. } | UpdateState::Building { .. })
        });
        eprintln!("built, staged and checked the stand-in in {:?}", started.elapsed());
        assert_eq!(
            view.state,
            UpdateState::Ready {
                version: "0.6.1".into()
            }
        );
        let next = inst.dir.join(NEXT);
        bf_pack::package::check_package(&next).unwrap();
        // Outside .next, the install is exactly as it was.
        let mut now = snapshot(&inst.dir);
        now.retain(|(p, _)| !p.starts_with("/.next"));
        assert_eq!(now, before);
        // A new start finds the staged version and offers the restart without building again.
        drop(w);
        let w = Worker::start(
            real(&inst, dir.path().join("ffmpeg-checked"), &dir.path().join("upstream")),
            no_wake(),
        );
        let view = until(&w, 60, |v| v.state != UpdateState::Checking);
        assert_eq!(
            view.state,
            UpdateState::Ready {
                version: "0.6.1".into()
            }
        );
    }
    #[test]
    fn a_build_that_fails_leaves_everything() {
        let dir = TempDir::new("worker-broken");
        let broken = standin::Kind {
            broken: true,
            ..standin::Kind::default()
        };
        let Some((s, inst)) = real_setup(dir.path(), broken) else {
            return;
        };
        let before = snapshot(&inst.dir);
        let w = Worker::start(s, no_wake());
        until(&w, 60, |v| v.state != UpdateState::Checking);
        w.send(Request::Build);
        let view = until(&w, 300, |v| matches!(v.state, UpdateState::Failed { .. }));
        let UpdateState::Failed { text } = view.state else {
            panic!("{view:?}")
        };
        assert!(
            text.starts_with(
                "Version 0.6.1 could not be built on this PC, so nothing changed: this is still version 0.6.0. Cargo \
                 said \"error"
            ),
            "{text}"
        );
        assert!(!inst.dir.join(NEXT).exists());
        assert_eq!(snapshot(&inst.dir), before, "the install folder is unchanged");
    }
}
