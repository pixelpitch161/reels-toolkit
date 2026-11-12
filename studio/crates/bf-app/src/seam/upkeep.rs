//! The seam between the interface and the app's own upkeep (plan M6, Task 13): looking for, building and putting in
//! place a newer version, going back, opening the guide, and removing the app from this PC.
//!
//! The state holds one [`Upkeep`] (`AppState::set_upkeep`). Like the engine's seam, nothing on it waits: `send` only
//! queues a request, `view` only copies what the work has got to, and `plan_removal` starts working out what a
//! removal would do on a thread of its own (it reads and checks every file of the install).
//!
//! **Which one runs, and why no test can reach the real one.**
//! - The state starts with [`SampleUpkeep`]: it is always what it was made with (`NotInstalled` for a new state, the
//!   fixture's view for `AppState::showing`), keeps the requests it is sent for tests to read, and does nothing
//!   else. It never starts a program, reads the network, git or cargo, or touches a file.
//! - The real one, [`live::LiveUpkeep`], is made only by [`for_installed_copy`], which gives `None` unless this
//!   program is an installed copy (`update::installed()`), and which is called from exactly one place: `main.rs`,
//!   for a real start (not `--fixtures`, not `--smoke`). Only it calls `update::worker::Setup::for_this_copy` (the
//!   one place the worker's real steps, `bf_pack::remove::remove_app` among them, are made) and
//!   `bf_pack::install::Layout::system`. The test `only_main_reaches_the_real_upkeep` scans the crate's sources and
//!   tests and fails if any of these names is used anywhere else.

use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::update::worker::Request;
use crate::update::{UpdateState, UpdateView};

/// What the app can be asked to do about itself, and what it shows of it.
pub trait Upkeep: Send + fmt::Debug {
    /// Queues a request; never waits.
    fn send(&mut self, r: Request);
    /// What to show now; never waits.
    fn view(&self) -> UpdateView;
    /// Whether a video is rendering: "Restart now", "Go back" and "Remove the app" wait while one is.
    fn set_rendering(&mut self, on: bool);
    /// Starts working out what removing the app would do, in words ([`Upkeep::removal`] gives it once known).
    fn plan_removal(&mut self);
    /// What removing the app would do, as whole sentences, or why it cannot be removed; `None` until it is known
    /// (or when it was never asked for).
    fn removal(&self) -> Option<Result<Vec<String>, String>>;
    /// The requests sent so far, for tests and the sample; the real upkeep keeps none.
    fn sent(&self) -> Vec<Request> {
        Vec::new()
    }
}

/// An upkeep that is always what it was made with and does nothing: the state's own until `main.rs` gives it the
/// real one, every screen test's, and `--fixtures`'.
#[derive(Debug)]
pub struct SampleUpkeep {
    view: UpdateView,
    removal: Result<Vec<String>, String>,
    asked: bool,
    sent: Arc<Mutex<Vec<Request>>>,
}

/// What the sample says before a removal of an installed copy: Task 11's own words for a made-up install (nothing on
/// this PC is looked at but whether that made-up folder's `data\build` and `data\downloads` exist, and they do not).
pub fn sample_removal_words() -> Vec<String> {
    let layout = sample_layout();
    let plan = bf_pack::remove::Removal {
        files: vec![
            (layout.start_menu_link(), String::new()),
            (layout.desktop_link(), String::new()),
        ],
        dirs: Vec::new(),
        kept: Vec::new(),
    };
    bf_pack::remove::removal_words(&layout, &plan)
}

/// What the sample says after it removed the app (Task 11's own words, for the made-up install).
pub fn sample_removed_words() -> Vec<String> {
    let layout = sample_layout();
    let removed = bf_pack::remove::Removed {
        programs: layout.programs.clone(),
        went: Vec::new(),
        stayed: Vec::new(),
        moved_exe: Some(Path::new(r"C:\Users\owner\AppData\Local\Temp").join("BALLFIGHT Studio removed 4120.exe")),
        exe_left: None,
    };
    bf_pack::remove::removed_words(&removed)
}

/// A made-up install for the sample's words, under a folder that is not on this PC.
fn sample_layout() -> bf_pack::install::Layout {
    bf_pack::install::Layout::under(Path::new(r"C:\Users\owner\AppData\Local"))
}

impl SampleUpkeep {
    /// A copy that was not installed: it never updates and cannot be removed.
    pub fn not_installed() -> SampleUpkeep {
        SampleUpkeep::showing(UpdateView {
            state: UpdateState::NotInstalled,
            previous: None,
            previous_button: None,
            reports: None,
            guide: None,
            close: None,
            note: None,
        })
    }

    /// One that always shows `view`; asked about a removal, it gives [`sample_removal_words`] for an installed copy.
    pub fn showing(view: UpdateView) -> SampleUpkeep {
        let removal = if view.state == UpdateState::NotInstalled {
            Err(crate::state::upkeep::NOTHING_TO_REMOVE.to_string())
        } else {
            Ok(sample_removal_words())
        };
        SampleUpkeep {
            view,
            removal,
            asked: false,
            sent: Arc::default(),
        }
    }
}

impl Upkeep for SampleUpkeep {
    fn send(&mut self, r: Request) {
        self.sent.lock().unwrap_or_else(|p| p.into_inner()).push(r);
    }

    fn view(&self) -> UpdateView {
        self.view.clone()
    }

    fn set_rendering(&mut self, _on: bool) {}

    fn plan_removal(&mut self) {
        self.asked = true;
    }

    fn removal(&self) -> Option<Result<Vec<String>, String>> {
        self.asked.then(|| self.removal.clone())
    }

    fn sent(&self) -> Vec<Request> {
        self.sent.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
}

/// The real upkeep for an installed copy, woken through `ctx`; `None` for any other copy (a developer's build in
/// `target\`, anything not installed by `ballfight-pack install`). Called by `main.rs` only, for a real start.
pub fn for_installed_copy(ctx: &egui::Context) -> Option<Box<dyn Upkeep>> {
    crate::update::installed()?;
    let ctx = ctx.clone();
    let waker: crate::seam::Waker = Arc::new(move || ctx.request_repaint());
    Some(Box::new(live::LiveUpkeep::start(waker)))
}

/// The real upkeep: Task 12's update worker, and Task 11's removal plan.
mod live {
    use std::fmt;
    use std::sync::{Arc, Mutex};

    use super::Upkeep;
    use crate::seam::Waker;
    use crate::update::UpdateView;
    use crate::update::worker::{Request, Setup, Worker};

    /// What removing would do, once worked out.
    type Plan = Arc<Mutex<Option<Result<Vec<String>, String>>>>;

    pub(super) struct LiveUpkeep {
        worker: Worker,
        plan: Plan,
        waker: Waker,
    }

    impl fmt::Debug for LiveUpkeep {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("LiveUpkeep").finish_non_exhaustive()
        }
    }

    impl LiveUpkeep {
        /// Starts the update worker for this copy, as it is on this PC (once: `for_installed_copy` is called once).
        pub(super) fn start(waker: Waker) -> LiveUpkeep {
            LiveUpkeep {
                worker: Worker::start(Setup::for_this_copy(), waker.clone()),
                plan: Arc::default(),
                waker,
            }
        }
    }

    /// What removing the app would do on this PC, in Task 11's words, or why it cannot be removed.
    fn plan_here() -> Result<Vec<String>, String> {
        let layout = bf_pack::install::Layout::system()
            .ok_or("Windows did not say where the app's folders are, so nothing can be removed.")?;
        let plan = bf_pack::remove::removal_plan(&layout).map_err(|e| e.to_string())?;
        Ok(bf_pack::remove::removal_words(&layout, &plan))
    }

    impl Upkeep for LiveUpkeep {
        fn send(&mut self, r: Request) {
            self.worker.send(r);
        }

        fn view(&self) -> UpdateView {
            self.worker.view()
        }

        fn set_rendering(&mut self, on: bool) {
            self.worker.set_rendering(on);
        }

        fn plan_removal(&mut self) {
            *self.plan.lock().unwrap_or_else(|p| p.into_inner()) = None;
            let (plan, waker) = (self.plan.clone(), self.waker.clone());
            let spawned = std::thread::Builder::new()
                .name("removal plan".to_string())
                .spawn(move || {
                    let words = plan_here();
                    *plan.lock().unwrap_or_else(|p| p.into_inner()) = Some(words);
                    waker();
                });
            if spawned.is_err() {
                *self.plan.lock().unwrap_or_else(|p| p.into_inner()) = Some(Err(
                    "The app could not work out what removing it would do, so nothing was removed.".to_string(),
                ));
            }
        }

        fn removal(&self) -> Option<Result<Vec<String>, String>> {
            self.plan.lock().unwrap_or_else(|p| p.into_inner()).clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    /// Every `.rs` file under `dir`, with its path relative to the crate.
    fn sources(crate_dir: &Path, dir: &str, out: &mut Vec<(String, String)>) {
        let mut stack = vec![crate_dir.join(dir)];
        while let Some(at) = stack.pop() {
            for entry in std::fs::read_dir(&at).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let rel: PathBuf = path.strip_prefix(crate_dir).unwrap().to_path_buf();
                    let rel = rel.to_string_lossy().replace('\\', "/");
                    out.push((rel, std::fs::read_to_string(&path).unwrap()));
                }
            }
        }
    }

    /// The files whose code (not comments) names `word`.
    fn users(all: &[(String, String)], word: &str) -> Vec<String> {
        let mut found: Vec<String> = all
            .iter()
            .filter(|(_, text)| {
                text.lines()
                    .map(str::trim_start)
                    .filter(|l| !l.starts_with("//"))
                    .any(|l| l.contains(word))
            })
            .map(|(rel, _)| rel.clone())
            .collect();
        found.sort();
        found
    }

    /// The guarantee of the module notes: the real upkeep, the update worker's real steps, the removal and the PC's
    /// own install layout are named only where they are defined and in `main.rs`'s one call; no test, fixture or
    /// screen names them.
    #[test]
    fn only_main_reaches_the_real_upkeep() {
        let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut all = Vec::new();
        sources(crate_dir, "src", &mut all);
        sources(crate_dir, "tests", &mut all);
        assert!(all.len() > 50, "the sources were read: {}", all.len());
        let this = "src/seam/upkeep.rs".to_string();
        // Written here in two halves, so this test's own text does not count.
        let name = |a: &str, b: &str| format!("{a}{b}");
        assert_eq!(
            users(&all, &name("for_installed", "_copy(")),
            ["src/main.rs".to_string(), this.clone()]
        );
        assert_eq!(
            users(&all, &name("LiveUpkeep::", "start(")),
            std::slice::from_ref(&this),
            "made only by for_installed_copy"
        );
        assert_eq!(
            users(&all, &name("Setup::for_this", "_copy(")),
            std::slice::from_ref(&this),
            "the worker's real steps are made only by the live upkeep"
        );
        assert_eq!(
            users(&all, &name("fn for_this", "_copy(")),
            ["src/update/worker.rs".to_string()]
        );
        assert_eq!(
            users(&all, &name("remove", "_app(")),
            ["src/update/worker.rs".to_string()],
            "only inside Setup::for_this_copy"
        );
        assert_eq!(
            users(&all, &name("Layout::", "system(")),
            [this.clone(), "src/update/worker.rs".to_string()]
        );
        // In this file, they stand only in the live module and in `for_installed_copy`, before the tests.
        let text = &all.iter().find(|(rel, _)| *rel == this).unwrap().1;
        let live = text.find("mod live {").unwrap();
        let tests = text.find("#[cfg(test)]").unwrap();
        let only_after = |word: &str, from: usize| {
            text.match_indices(word)
                .filter(|(at, _)| {
                    !text[..*at]
                        .rsplit('\n')
                        .next()
                        .unwrap_or("")
                        .trim_start()
                        .starts_with("//")
                })
                .all(|(at, _)| at > from && at < tests)
        };
        let made = text.find("pub fn for_installed_copy").unwrap();
        assert!(only_after(&name("LiveUpkeep::", "start("), made));
        assert!(only_after(&name("Setup::for_this", "_copy("), live));
        assert!(only_after(&name("Layout::", "system("), live));
    }

    #[test]
    fn the_sample_does_nothing_but_keep_what_it_is_sent() {
        let mut sample = SampleUpkeep::not_installed();
        assert_eq!(sample.view().state, UpdateState::NotInstalled);
        assert_eq!(sample.removal(), None, "not asked yet");
        sample.plan_removal();
        assert_eq!(
            sample.removal(),
            Some(Err(crate::state::upkeep::NOTHING_TO_REMOVE.to_string()))
        );
        sample.send(Request::Check);
        sample.set_rendering(true);
        sample.send(Request::Remove);
        assert_eq!(sample.sent(), [Request::Check, Request::Remove]);
        assert_eq!(
            sample.view().state,
            UpdateState::NotInstalled,
            "it stays as it was made"
        );
        let mut view = sample.view();
        view.state = UpdateState::UpToDate;
        let mut installed = SampleUpkeep::showing(view);
        installed.plan_removal();
        assert_eq!(installed.removal(), Some(Ok(sample_removal_words())));
    }

    #[test]
    fn the_sample_words_are_task_11s() {
        let words = sample_removal_words();
        assert!(
            words[0].starts_with("Removing takes away the program in C:\\Users\\owner\\AppData\\Local\\Programs\\BALLFIGHT Studio and its Start menu and desktop shortcuts"),
            "{words:?}"
        );
        assert!(sample_removed_words()[0].starts_with("BALLFIGHT Studio was removed"));
    }
}
