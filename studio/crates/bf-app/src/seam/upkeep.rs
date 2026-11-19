//! The seam between the interface and the app's own upkeep (plan M6, Task 13): looking for, building and putting in
//! place a newer version, going back, opening the guide, and removing the app from this PC.
//!
//! The state holds one [`Upkeep`] (`AppState::set_upkeep`). Like the engine's seam, nothing on it waits: `send` only
//! queues a request, `view` and `answered` only copy what the work has got to, and `plan_removal` starts working out
//! what a removal would do on a thread of its own (it reads and checks every file of the install).
//!
//! **Which one runs.**
//! - The state starts with [`SampleUpkeep`]: it shows what it was made with (`NotInstalled` for a new state, the
//!   fixture's view for `AppState::showing`), keeps the requests it is sent for tests to read, and answers each one
//!   without doing anything (a removal: "This is the sample app, so nothing was removed."). It never starts a
//!   program, reads the network, git or cargo, or touches a file.
//! - [`for_this_start`] is called from one place, `main.rs`, with the folder the start's fault reports go to. It
//!   gives the real one, `live::LiveUpkeep`, only for a real start (not `--fixtures`, not `--smoke`) of an installed
//!   copy (`update::installed()`); every other start gets the sample, not installed, with that reports folder.
//!   Only the live one calls `update::worker::Setup::for_this_copy` (where the worker's real steps,
//!   `bf_pack::remove::remove_app` among them, are made) and `bf_pack::install::Layout::system`.
//!
//! **What guarantees that no test or developer build touches an install** is `update::installed()` (a program in
//! cargo's target folder is never an installed copy) and `remove_app`'s own refusal unless the running exe is the
//! installed one. The test `only_main_reaches_the_real_upkeep` is a tripwire on top: it scans the crate's sources
//! and tests and fails when the names that lead to the real upkeep turn up anywhere new, or are renamed with `as`.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::update::worker::Request;
use crate::update::{UpdateState, UpdateView};

/// What the sample says when asked to remove the app.
pub const SAMPLE_KEPT: &str = "This is the sample app, so nothing was removed.";
/// When Windows gives no folders for the app, so no removal can be planned.
pub const NO_FOLDERS: &str = "Windows did not say where the app's folders are, so nothing can be removed.";
/// When the removal plan could not be started.
pub const NO_PLAN: &str = "The app could not work out what removing it would do, so nothing was removed.";

/// What the app can be asked to do about itself, and what it shows of it.
pub trait Upkeep: Send + fmt::Debug {
    /// Queues a request; never waits.
    fn send(&mut self, r: Request);
    /// What to show now; never waits.
    fn view(&self) -> UpdateView;
    /// How many requests have been answered: it rises by one after each, whatever came of it, so two answers with
    /// the same words are still two. Never waits.
    fn answered(&self) -> u64;
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

/// An upkeep that shows what it was made with and answers every request without doing anything: the state's own
/// until `main.rs` gives it one for the start, every screen test's, and `--fixtures`'.
#[derive(Debug)]
pub struct SampleUpkeep {
    view: UpdateView,
    removal: Result<Vec<String>, String>,
    asked: bool,
    answered: u64,
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

/// `text` without Windows' "(os error N)" tails, wherever they stand.
pub fn without_os_errors(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(" (os error ") {
        let after = &rest[at + " (os error ".len()..];
        let digits = after.chars().take_while(char::is_ascii_digit).count();
        if digits > 0 && after[digits..].starts_with(')') {
            out.push_str(&rest[..at]);
            rest = &after[digits + 1..];
        } else {
            out.push_str(&rest[..at + 1]);
            rest = &rest[at + 1..];
        }
    }
    out.push_str(rest);
    out
}

impl SampleUpkeep {
    /// A copy that was not installed: it never updates and cannot be removed. Its fault reports go to `reports`.
    pub fn not_installed(reports: Option<PathBuf>) -> SampleUpkeep {
        SampleUpkeep::showing(UpdateView {
            state: UpdateState::NotInstalled,
            previous: None,
            previous_button: None,
            reports,
            guide: None,
            close: None,
            note: None,
        })
    }

    /// One that shows `view`; asked about a removal, it gives [`sample_removal_words`] for an installed copy.
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
            answered: 0,
            sent: Arc::default(),
        }
    }
}

impl Upkeep for SampleUpkeep {
    fn send(&mut self, r: Request) {
        self.sent.lock().unwrap_or_else(|p| p.into_inner()).push(r);
        // Answered at once, and nothing is done: a removal keeps the app and says so.
        self.view.note = (r == Request::Remove).then(|| SAMPLE_KEPT.to_string());
        self.answered += 1;
    }

    fn view(&self) -> UpdateView {
        self.view.clone()
    }

    fn answered(&self) -> u64 {
        self.answered
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

/// The upkeep for this start, woken through `ctx`, its fault reports in `reports` (the folder the start's fault hook
/// writes to). `real` is false for `--smoke` and `--fixtures`. The real one only for a real start of an installed
/// copy; any other start (a developer's build in `target\`, a copy not installed by `ballfight-pack install`) gets
/// the sample, not installed. Called by `main.rs` only.
pub fn for_this_start(ctx: &egui::Context, real: bool, reports: Option<PathBuf>) -> Box<dyn Upkeep> {
    if !real || crate::update::installed().is_none() {
        return Box::new(SampleUpkeep::not_installed(reports));
    }
    let ctx = ctx.clone();
    let waker: crate::seam::Waker = Arc::new(move || ctx.request_repaint());
    Box::new(live::LiveUpkeep::start(waker, reports))
}

/// The real upkeep: Task 12's update worker, and Task 11's removal plan.
mod live {
    use std::fmt;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use super::{NO_FOLDERS, NO_PLAN, Upkeep, without_os_errors};
    use crate::seam::Waker;
    use crate::update::worker::{Request, Setup, Worker};
    use crate::update::{UpdateState, UpdateView};

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
        /// Starts the update worker for this copy, as it is on this PC (once: `for_this_start` is called once), with
        /// the start's own reports folder when it has one (a start with `--data-dir` writes its reports there).
        pub(super) fn start(waker: Waker, reports: Option<PathBuf>) -> LiveUpkeep {
            let mut setup = Setup::for_this_copy();
            if reports.is_some() {
                setup.logs = reports;
            }
            LiveUpkeep {
                worker: Worker::start(setup, waker.clone()),
                plan: Arc::default(),
                waker,
            }
        }
    }

    /// What removing the app would do on this PC, in Task 11's words, or why it cannot be removed.
    fn plan_here() -> Result<Vec<String>, String> {
        let layout = bf_pack::install::Layout::system().ok_or(NO_FOLDERS)?;
        let plan = bf_pack::remove::removal_plan(&layout).map_err(|e| without_os_errors(&e.to_string()))?;
        Ok(bf_pack::remove::removal_words(&layout, &plan))
    }

    impl Upkeep for LiveUpkeep {
        fn send(&mut self, r: Request) {
            self.worker.send(r);
        }

        /// The worker's view, with no "(os error N)" tail in any sentence.
        fn view(&self) -> UpdateView {
            let mut view = self.worker.view();
            view.note = view.note.as_deref().map(without_os_errors);
            if let UpdateState::Failed { text } = &mut view.state {
                *text = without_os_errors(text);
            }
            if let Some(words) = &mut view.close {
                for w in words.iter_mut() {
                    *w = without_os_errors(w);
                }
            }
            view
        }

        fn answered(&self) -> u64 {
            self.worker.answered()
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
                *self.plan.lock().unwrap_or_else(|p| p.into_inner()) = Some(Err(NO_PLAN.to_string()));
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

    fn is_ident(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    /// Whether `line` holds `word` with no identifier character just before it, and none just after its last
    /// identifier character (so `for_this_copy` matches `for_this_copy(` and `for_this_copy;` but not
    /// `for_this_copy_2`).
    fn has_word(line: &str, word: &str) -> bool {
        line.match_indices(word).any(|(at, _)| {
            let before = line[..at].chars().next_back();
            let after = line[at + word.len()..].chars().next();
            !before.is_some_and(is_ident) && (word.ends_with(|c: char| !is_ident(c)) || !after.is_some_and(is_ident))
        })
    }

    /// The code lines of `text` (comment lines left out).
    fn code(text: &str) -> impl Iterator<Item = &str> {
        text.lines().map(str::trim_start).filter(|l| !l.starts_with("//"))
    }

    /// The files whose code names `word` (as a whole word), sorted.
    fn users(all: &[(String, String)], word: &str) -> Vec<String> {
        let mut found: Vec<String> = all
            .iter()
            .filter(|(_, text)| code(text).any(|l| has_word(l, word)))
            .map(|(rel, _)| rel.clone())
            .collect();
        found.sort();
        found
    }

    /// A tripwire for the module notes (the guarantee itself is `update::installed()` and `remove_app`'s own check):
    /// the real upkeep, the update worker's real steps, the removal, the PC's own install layout and the installed
    /// check are named only where they are defined, in the update engine itself and in `main.rs`'s one call; no
    /// test, fixture or screen names them, and nothing renames them with `as`.
    #[test]
    fn only_main_reaches_the_real_upkeep() {
        let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut all = Vec::new();
        sources(crate_dir, "src", &mut all);
        sources(crate_dir, "tests", &mut all);
        assert!(all.len() > 50, "the sources were read: {}", all.len());
        // Each name is written here in two halves, so this test's own text does not count.
        let w = |a: &str, b: &str| format!("{a}{b}");
        let this = "src/seam/upkeep.rs";
        let worker = "src/update/worker.rs";
        let expect = |word: String, files: &[&str]| {
            let mut want: Vec<String> = files.iter().map(|f| f.to_string()).collect();
            want.sort();
            assert_eq!(users(&all, &word), want, "{word}");
        };
        expect(w("for_this", "_start"), &["src/main.rs", this]);
        expect(w("Live", "Upkeep"), &[this]);
        expect(w("for_this", "_copy"), &[this, worker]);
        expect(w("remove", "_app"), &[worker]);
        expect(w("Layout::", "system"), &[this, worker]);
        // `update::installed()` and the update engine's own helpers of that name (its tests' fake installs).
        expect(
            w("installed", "("),
            &[
                "src/main.rs",
                this,
                "src/update/mod.rs",
                "src/update/stage.rs",
                "src/update/swap.rs",
                worker,
            ],
        );
        // No renames that would hide them.
        for name in ["Setup", "Layout", "remove", "worker", "upkeep"] {
            let renamed = w(name, " as ");
            assert_eq!(users(&all, &renamed), Vec::<String>::new(), "{renamed}");
        }
        // In this file, they stand only in the live module and in `for_this_start`, before the tests.
        let text = &all.iter().find(|(rel, _)| rel == this).unwrap().1;
        let made = text.find(&w("pub fn for_this", "_start")).unwrap();
        let live = text.find("mod live {").unwrap();
        let tests = text.find("#[cfg(test)]").unwrap();
        let only_between = |word: &str, from: usize| {
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
        assert!(only_between(&w("Live", "Upkeep"), made));
        assert!(only_between(&w("Setup::for_this", "_copy"), live));
        assert!(only_between(&w("Layout::", "system"), live));
    }

    #[test]
    fn the_word_match_bites() {
        assert!(has_word("let s = Setup::make_it();", "make_it"));
        assert!(has_word("use worker::make_it;", "make_it"));
        assert!(has_word("f = make_it", "make_it"));
        assert!(!has_word("make_it_2()", "make_it"));
        assert!(!has_word("not_make_it()", "make_it"));
        assert!(has_word("use Thing as T;", "Thing as "));
        assert!(has_word("x = update::installed()", "installed("));
        assert!(!has_word("x = not_installed()", "installed("));
    }

    #[test]
    fn the_sample_answers_and_does_nothing() {
        let mut sample = SampleUpkeep::not_installed(Some(PathBuf::from("logs")));
        assert_eq!(sample.view().state, UpdateState::NotInstalled);
        assert_eq!(sample.view().reports, Some(PathBuf::from("logs")));
        assert_eq!(sample.removal(), None, "not asked yet");
        sample.plan_removal();
        assert_eq!(
            sample.removal(),
            Some(Err(crate::state::upkeep::NOTHING_TO_REMOVE.to_string()))
        );
        sample.send(Request::Check);
        assert_eq!((sample.answered(), sample.view().note), (1, None));
        sample.set_rendering(true);
        sample.send(Request::Remove);
        assert_eq!(sample.sent(), [Request::Check, Request::Remove]);
        assert_eq!(sample.answered(), 2);
        assert_eq!(
            sample.view().note.as_deref(),
            Some(SAMPLE_KEPT),
            "a removal is answered, never done"
        );
        assert_eq!(sample.view().close, None);
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
        let programs = r"C:\Users\owner\AppData\Local\Programs\BALLFIGHT Studio";
        assert!(
            words[0].starts_with(&format!(
                "Removing takes away the program in {programs} and its Start menu and desktop shortcuts"
            )),
            "{words:?}"
        );
        assert!(sample_removed_words()[0].starts_with("BALLFIGHT Studio was removed"));
    }

    #[test]
    fn no_os_error_tail_reaches_the_screen() {
        assert_eq!(
            without_os_errors(r"Could not read C:\x\MANIFEST.txt: Access is denied. (os error 5)"),
            r"Could not read C:\x\MANIFEST.txt: Access is denied."
        );
        assert_eq!(
            without_os_errors("First (os error 32) and second (os error 2) gone."),
            "First and second gone."
        );
        assert_eq!(without_os_errors("A (os error x) stays."), "A (os error x) stays.");
        assert_eq!(without_os_errors("Plain."), "Plain.");
    }
}
