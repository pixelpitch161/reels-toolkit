//! Where the app keeps what must survive a restart (its settings, the render queue, the shortlist cache), and the
//! one background thread that writes it, so the interface never waits on a disk.
//!
//! Every file is written whole to `<name>.tmp` in the same folder and then renamed over the old one, so a crash or a
//! power cut in the middle of a write leaves the previous file as it was. A file the app cannot use is moved aside
//! (`settings.toml` -> `settings.corrupt-1.toml`), never deleted.

pub mod library;
pub mod queue_file;
pub mod settings;
pub mod shortlist_cache;

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::model::Settings;
use crate::model::queue::Queue;

/// The app's two storage folders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    /// Settings and the render queue.
    pub config: PathBuf,
    /// What can be rebuilt (the shortlists).
    pub cache: PathBuf,
}

impl Paths {
    /// The per-user folders: on Windows `%APPDATA%\BALLFIGHT Studio\config` and
    /// `%LOCALAPPDATA%\BALLFIGHT Studio\cache`. Only works out the paths; nothing is created.
    pub fn system() -> Option<Paths> {
        let dirs = directories::ProjectDirs::from("", "", "BALLFIGHT Studio")?;
        Some(Paths {
            config: dirs.config_dir().to_path_buf(),
            cache: dirs.cache_dir().to_path_buf(),
        })
    }

    /// `<root>/config` and `<root>/cache`: for tests and `--data-dir`.
    pub fn under(root: &Path) -> Paths {
        Paths {
            config: root.join("config"),
            cache: root.join("cache"),
        }
    }

    /// `config/settings.toml`.
    pub fn settings(&self) -> PathBuf {
        self.config.join("settings.toml")
    }

    /// `config/queue.json`.
    pub fn queue(&self) -> PathBuf {
        self.config.join("queue.json")
    }

    /// `cache/shortlists`.
    pub fn shortlists(&self) -> PathBuf {
        self.cache.join("shortlists")
    }
}

/// How many times a file that another program holds open (an antivirus scan, an editor) is tried again. The pauses
/// before the tries are 25, 50, 100 and 200 ms, so at most 375 ms of waiting before the write is given up.
const BUSY_RETRIES: u32 = 4;

/// The default wait between tries of a busy file: sleeps, and says whether to try again.
fn pause_before_retry(attempt: u32) -> bool {
    if attempt >= BUSY_RETRIES {
        return false;
    }
    std::thread::sleep(Duration::from_millis(25 << attempt));
    true
}

/// Whether an error means "another program has the file open right now", which a short wait may cure.
fn is_busy(err: &io::Error) -> bool {
    if err.kind() == io::ErrorKind::PermissionDenied {
        return true;
    }
    // ERROR_SHARING_VIOLATION and ERROR_LOCK_VIOLATION.
    cfg!(windows) && matches!(err.raw_os_error(), Some(32 | 33))
}

/// Runs `op`, trying again while it fails because the file is busy and `wait(attempt)` says to go on.
fn retry_busy<T>(mut op: impl FnMut() -> io::Result<T>, wait: &mut dyn FnMut(u32) -> bool) -> io::Result<T> {
    let mut attempt = 0;
    loop {
        match op() {
            Err(err) if is_busy(&err) && wait(attempt) => attempt += 1,
            result => return result,
        }
    }
}

/// Reads a whole file, trying again for a moment if another program has it open.
pub(crate) fn read_patiently(path: &Path) -> io::Result<Vec<u8>> {
    retry_busy(|| fs::read(path), &mut pause_before_retry)
}

/// `<name>.<process>-<n>.tmp` beside `path`: a name of its own for every write, so two copies of the app (or two
/// writes in one) never write into the same temporary file.
fn tmp_path(path: &Path) -> io::Result<PathBuf> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "the path has no file name"))?;
    let mut tmp = OsString::from(name);
    tmp.push(format!(
        ".{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    Ok(path.with_file_name(tmp))
}

/// Settings files that `load_settings` could not open, so the app is running on its defaults while the owner's own
/// file is still there. Before the writer saves settings over one of these, it moves the old file aside as
/// `settings.unread-N.toml` (ruling M2-R9).
static UNREAD: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());

fn unread() -> MutexGuard<'static, BTreeSet<PathBuf>> {
    UNREAD.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Records that the file at `path` exists but could not be read.
pub(crate) fn mark_unread(path: &Path) {
    unread().insert(path.to_path_buf());
}

/// Records that the file at `path` was read (or is not there), so it may be written over.
pub(crate) fn clear_unread(path: &Path) {
    unread().remove(path);
}

/// Before settings are saved to `path`: if the file there could not be read at start, moves it aside as
/// `settings.unread-N.toml` first. An error means the old file could not be kept, and the save must not happen.
fn keep_unread(path: &Path) -> io::Result<()> {
    if !unread().contains(path) {
        return Ok(());
    }
    if !path.try_exists()? {
        clear_unread(path);
        return Ok(());
    }
    move_aside(path, "unread")?;
    clear_unread(path);
    Ok(())
}

/// Writes `bytes` to `path` so that the file is either the old one or the new one, never half of each: creates the
/// folder if it is missing, writes a temporary file of its own beside it (`<name>.<process>-<n>.tmp`), flushes it to
/// the disk and renames it over `path`. A file that another program holds open is tried again for up to 375 ms. On
/// failure no temporary file is left behind.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_atomic_with(path, bytes, &mut pause_before_retry)
}

/// `write_atomic` with the wait between tries of a busy file given by the caller (tests drive it).
fn write_atomic_with(path: &Path, bytes: &[u8], wait: &mut dyn FnMut(u32) -> bool) -> io::Result<()> {
    if let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        fs::create_dir_all(dir)?;
    }
    let tmp = tmp_path(path)?;
    let result = retry_busy(
        || {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&tmp, path)
        },
        wait,
    );
    if result.is_err() && tmp.is_file() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// The most numbers a set-aside name tries before it gives up.
const MAX_ASIDE: u32 = 9_999;

/// `settings.toml` with `tag` "corrupt" and `n` 1 -> `settings.corrupt-1.toml` (a name without an extension gets
/// `name.corrupt-1`).
fn aside_path(path: &Path, tag: &str, n: u32) -> PathBuf {
    let mut name = OsString::from(path.file_stem().unwrap_or_default());
    name.push(format!(".{tag}-{n}"));
    if let Some(ext) = path.extension() {
        name.push(".");
        name.push(ext);
    }
    path.with_file_name(name)
}

/// The first set-aside name not taken, as a name only (for a notice that says where a file would go).
pub(crate) fn next_aside_name(path: &Path, tag: &str) -> String {
    let n = (1..=MAX_ASIDE)
        .find(|&n| !aside_path(path, tag, n).exists())
        .unwrap_or(1);
    aside_path(path, tag, n)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// Tests only: a hook called with a set-aside name just before it is claimed.
#[cfg(test)]
type ClaimHook = Option<Box<dyn FnMut(&Path)>>;

#[cfg(test)]
thread_local! {
    /// Tests only: called with each set-aside name just before it is claimed, to play another program taking it.
    static BEFORE_CLAIM: std::cell::RefCell<ClaimHook> = const { std::cell::RefCell::new(None) };
}

/// Claims the first free set-aside name by creating it empty with `create_new`, which fails if the name exists,
/// even if another program made it a moment ago. Returns the name and the new, empty file.
fn claim_aside(path: &Path, tag: &str) -> io::Result<(PathBuf, fs::File)> {
    for n in 1..=MAX_ASIDE {
        let candidate = aside_path(path, tag, n);
        #[cfg(test)]
        BEFORE_CLAIM.with(|hook| {
            if let Some(hook) = hook.borrow_mut().as_mut() {
                hook(&candidate);
            }
        });
        match fs::OpenOptions::new().write(true).create_new(true).open(&candidate) {
            Ok(file) => return Ok((candidate, file)),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "every set-aside name is taken",
    ))
}

/// Moves a file the app cannot use out of the way without deleting it: `settings.toml` with `tag` "corrupt"
/// becomes `settings.corrupt-1.toml`, or the first free number after it. The name is claimed first, so a file of
/// that name made by anyone else is never written over. Returns the new path.
pub fn move_aside(path: &Path, tag: &str) -> io::Result<PathBuf> {
    let (target, claimed) = claim_aside(path, tag)?;
    drop(claimed);
    // The rename replaces only the empty file claimed above.
    match retry_busy(|| fs::rename(path, &target), &mut pause_before_retry) {
        Ok(()) => Ok(target),
        Err(err) => {
            let _ = fs::remove_file(&target);
            Err(err)
        }
    }
}

/// When `move_aside` fails (another program holds the file), keeps the bytes the app already read under the
/// set-aside name instead, so they are not lost when the file is written over later. Returns the new path.
pub(crate) fn keep_copy(path: &Path, tag: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    let (target, mut file) = claim_aside(path, tag)?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    match written {
        Ok(()) => Ok(target),
        Err(err) => {
            let _ = fs::remove_file(&target);
            Err(err)
        }
    }
}

/// Why a write failed, in words the owner can act on.
fn plain_reason(err: &io::Error) -> &'static str {
    if is_busy(err) {
        return "The file may be open in another program, or the file or its folder may be read-only.";
    }
    match err.kind() {
        io::ErrorKind::NotFound => "The folder could not be found.",
        io::ErrorKind::StorageFull => "The drive is full.",
        _ => "Check that the folder can be written to and that the drive has free space.",
    }
}

/// What the writer was asked to write and what it has done.
#[derive(Default)]
struct State {
    /// The newest settings not yet taken by the thread.
    settings: Option<Settings>,
    /// The newest queue not yet taken by the thread.
    queue: Option<Queue>,
    /// How many writes have been asked for.
    asked: u64,
    /// `asked` as it was when the newest finished round of writes began.
    written: u64,
    /// The writer is being dropped: finish what is pending, then stop.
    closing: bool,
    /// The thread has ended (or never started); `flush` must not wait for it.
    stopped: bool,
    /// What could not be written since `problems` was last called, one sentence each.
    problems: Vec<String>,
    /// Tests only: while set, the thread leaves pending values alone.
    #[cfg(test)]
    held: bool,
    /// Tests only: how many files the writer has written.
    #[cfg(test)]
    files_written: u64,
    /// Tests only: the next settings write panics, as a bug in the writer would.
    #[cfg(test)]
    panic_next: bool,
}

struct Shared {
    state: Mutex<State>,
    /// Woken when there is something to write or the writer closes.
    work: Condvar,
    /// Woken when a round of writes finishes or the thread stops.
    done: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Writes the settings and the queue on its own thread. Of each kind only the newest value is written: values asked
/// for while a write is under way replace one another, and the newest of them is written next. Dropping the writer
/// finishes every pending write and then ends the thread.
pub struct Writer {
    shared: Arc<Shared>,
    paths: Paths,
    thread: Option<JoinHandle<()>>,
}

impl Writer {
    /// Starts the writer's thread. If the system refuses a thread, the writer writes on the caller's thread when it
    /// is flushed or dropped instead.
    pub fn start(paths: Paths) -> Writer {
        let shared = Arc::new(Shared {
            state: Mutex::new(State::default()),
            work: Condvar::new(),
            done: Condvar::new(),
        });
        let thread = {
            let shared = Arc::clone(&shared);
            let paths = paths.clone();
            std::thread::Builder::new()
                .name("bf-store-writer".to_string())
                .spawn(move || run(&shared, &paths))
                .ok()
        };
        if thread.is_none() {
            shared.lock().stopped = true;
        }
        Writer { shared, paths, thread }
    }

    /// Asks for the settings to be saved. Returns at once.
    pub fn settings(&self, s: &Settings) {
        let mut state = self.shared.lock();
        state.settings = Some(s.clone());
        state.asked += 1;
        self.shared.work.notify_one();
    }

    /// Asks for the render queue to be saved. Returns at once.
    pub fn queue(&self, q: &Queue) {
        let mut state = self.shared.lock();
        state.queue = Some(q.clone());
        state.asked += 1;
        self.shared.work.notify_one();
    }

    /// Waits until everything asked for so far is on disk, or has failed and is in `problems`.
    pub fn flush(&self) {
        let mut state = self.shared.lock();
        let target = state.asked;
        while state.written < target && !state.stopped {
            state = self
                .shared
                .done
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        let stopped = state.stopped;
        drop(state);
        if stopped {
            write_pending(&self.shared, &self.paths);
        }
    }

    /// What could not be written since the last call, one sentence each; empty when all went well.
    pub fn problems(&self) -> Vec<String> {
        std::mem::take(&mut self.shared.lock().problems)
    }

    /// Tests only: while held, the thread leaves pending values alone, so a test can pile several up.
    #[cfg(test)]
    fn hold(&self, held: bool) {
        self.shared.lock().held = held;
        self.shared.work.notify_all();
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        {
            let mut state = self.shared.lock();
            state.closing = true;
            #[cfg(test)]
            {
                state.held = false;
            }
        }
        self.shared.work.notify_all();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        // Only does anything if the thread never started or died: it finishes its own work before it ends.
        write_pending(&self.shared, &self.paths);
    }
}

/// Marks the writer's thread as stopped when it ends, even by a panic, so `flush` never waits for a dead thread.
struct StopGuard<'a>(&'a Shared);

/// Said when the writer's thread ends by a fault rather than by the writer being dropped.
const WRITER_STOPPED: &str = "Saving in the background stopped because of a fault in the app, so your latest settings \
                              or render queue may not have been saved yet. Close the app and start it again: it saves \
                              them as it closes.";

impl Drop for StopGuard<'_> {
    fn drop(&mut self) {
        let mut state = self.0.lock();
        state.stopped = true;
        if std::thread::panicking() {
            state.problems.push(WRITER_STOPPED.to_string());
        }
        drop(state);
        self.0.done.notify_all();
    }
}

/// The writer's thread: waits for work, writes the newest of each kind, and repeats until the writer is dropped.
fn run(shared: &Shared, paths: &Paths) {
    let _guard = StopGuard(shared);
    loop {
        {
            let mut state = shared.lock();
            loop {
                let pending = state.settings.is_some() || state.queue.is_some();
                #[cfg(test)]
                let pending = pending && !state.held;
                if pending || state.closing {
                    break;
                }
                state = shared.work.wait(state).unwrap_or_else(|poisoned| poisoned.into_inner());
            }
            if state.closing && state.settings.is_none() && state.queue.is_none() {
                return;
            }
        }
        write_pending(shared, paths);
    }
}

/// Takes what is pending, writes it and records how it went.
fn write_pending(shared: &Shared, paths: &Paths) {
    let (settings, queue, mark) = {
        let mut state = shared.lock();
        (state.settings.take(), state.queue.take(), state.asked)
    };
    let mut problems = Vec::new();
    let mut files = 0u64;
    if let Some(s) = settings {
        let path = paths.settings();
        #[cfg(test)]
        if std::mem::take(&mut shared.lock().panic_next) {
            panic!("a test asked the writer to stop");
        }
        match keep_unread(&path) {
            Err(err) => problems.push(format!(
                "Your settings were not saved to {}, because the old settings file there could not be kept safe \
                 first. {} Change a setting again to try once more.",
                path.display(),
                plain_reason(&err)
            )),
            Ok(()) => match settings::save_settings(&path, &s) {
                Ok(()) => files += 1,
                Err(err) => problems.push(format!(
                    "Your settings could not be saved to {}. {}",
                    path.display(),
                    plain_reason(&err)
                )),
            },
        }
    }
    if let Some(q) = queue {
        // Task 11 swaps in `save_queue`.
        let path = paths.queue();
        let written = serde_json::to_vec_pretty(&q)
            .map_err(io::Error::other)
            .and_then(|bytes| write_atomic(&path, &bytes));
        match written {
            Ok(()) => files += 1,
            Err(err) => problems.push(format!(
                "The render queue could not be saved to {}. {}",
                path.display(),
                plain_reason(&err)
            )),
        }
    }
    let mut state = shared.lock();
    for problem in problems {
        if !state.problems.contains(&problem) {
            state.problems.push(problem);
        }
    }
    #[cfg(test)]
    {
        state.files_written += files;
    }
    #[cfg(not(test))]
    let _ = files;
    state.written = state.written.max(mark);
    shared.done.notify_all();
}

/// A fresh folder for one test under the system temp folder, removed when dropped. Its name holds spaces and letters
/// outside ASCII, as the owner's own folders may.
#[cfg(test)]
pub(crate) struct TempRoot(pub PathBuf);

#[cfg(test)]
impl TempRoot {
    pub(crate) fn new(test: &str) -> TempRoot {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("bf-app store Zoë Ångström {test} {}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        TempRoot(root)
    }

    pub(crate) fn paths(&self) -> Paths {
        Paths::under(&self.0)
    }
}

#[cfg(test)]
impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
pub(crate) fn files_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::settings::load_settings;
    use super::*;

    fn sample_settings(workers: usize) -> Settings {
        Settings {
            output_folder: PathBuf::from(r"D:\Vidéos du studio\BALLFIGHT"),
            workers,
        }
    }

    #[test]
    fn the_folders_sit_under_the_root() {
        let root = Path::new(r"C:\data dir");
        let paths = Paths::under(root);
        assert_eq!(paths.config, root.join("config"));
        assert_eq!(paths.cache, root.join("cache"));
        assert_eq!(paths.settings(), root.join("config").join("settings.toml"));
        assert_eq!(paths.queue(), root.join("config").join("queue.json"));
        assert_eq!(paths.shortlists(), root.join("cache").join("shortlists"));
    }

    #[test]
    fn the_system_folders_are_named_for_the_app() {
        // Works the paths out only: nothing is read or created.
        let paths = Paths::system().expect("this PC has per-user folders");
        assert!(paths.config.to_string_lossy().contains("BALLFIGHT Studio"));
        assert!(paths.cache.to_string_lossy().contains("BALLFIGHT Studio"));
    }

    #[test]
    fn write_atomic_leaves_no_tmp_file() {
        let root = TempRoot::new("write_atomic_leaves_no_tmp_file");
        let path = root.0.join("not yet").join("made é").join("file.json");
        write_atomic(&path, b"first").unwrap();
        write_atomic(&path, b"second").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        assert_eq!(files_in(path.parent().unwrap()), vec!["file.json"]);
    }

    #[test]
    fn every_write_has_its_own_temporary_name() {
        let path = Path::new(r"C:\data dir\config\settings.toml");
        let first = tmp_path(path).unwrap();
        let second = tmp_path(path).unwrap();
        assert_ne!(first, second);
        for tmp in [&first, &second] {
            assert_eq!(tmp.parent(), path.parent());
            let name = tmp.file_name().unwrap().to_string_lossy().into_owned();
            let prefix = format!("settings.toml.{}-", std::process::id());
            assert!(name.starts_with(&prefix) && name.ends_with(".tmp"), "{name}");
        }
    }

    #[test]
    fn write_atomic_replaces_an_existing_file() {
        let root = TempRoot::new("write_atomic_replaces_an_existing_file");
        let path = root.0.join("file.json");
        write_atomic(&path, b"a longer first version").unwrap();
        write_atomic(&path, b"second").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        assert_eq!(files_in(&root.0), vec!["file.json"]);
    }

    #[test]
    fn a_write_that_cannot_happen_leaves_what_was_there() {
        let root = TempRoot::new("a_write_that_cannot_happen_leaves_what_was_there");
        // A folder with something in it where the file should go: the rename fails.
        let path = root.0.join("file.json");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("inside"), b"old").unwrap();
        assert!(write_atomic_with(&path, b"new", &mut |_| false).is_err());
        assert_eq!(fs::read(path.join("inside")).unwrap(), b"old");
        assert_eq!(files_in(&root.0), vec!["file.json"]);
    }

    #[cfg(windows)]
    fn lock_exclusively(path: &Path) -> fs::File {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new().read(true).share_mode(0).open(path).unwrap()
    }

    #[cfg(windows)]
    #[test]
    fn a_briefly_locked_file_is_written_on_a_later_try() {
        let root = TempRoot::new("a_briefly_locked_file_is_written_on_a_later_try");
        let path = root.0.join("settings.toml");
        fs::write(&path, b"old").unwrap();
        let mut lock = Some(lock_exclusively(&path));
        let mut tries = Vec::new();
        // The scanner lets go after the first failed try.
        let result = write_atomic_with(&path, b"new", &mut |attempt| {
            tries.push(attempt);
            lock = None;
            true
        });
        result.unwrap();
        assert_eq!(tries, vec![0]);
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(files_in(&root.0), vec!["settings.toml"]);
    }

    #[cfg(windows)]
    #[test]
    fn a_file_that_stays_locked_fails_and_keeps_the_old_bytes() {
        let root = TempRoot::new("a_file_that_stays_locked_fails_and_keeps_the_old_bytes");
        let path = root.0.join("settings.toml");
        fs::write(&path, b"old").unwrap();
        let lock = lock_exclusively(&path);
        let mut tries = 0;
        let result = write_atomic_with(&path, b"new", &mut |attempt| {
            tries += 1;
            attempt < BUSY_RETRIES
        });
        assert!(is_busy(&result.unwrap_err()));
        assert_eq!(tries, BUSY_RETRIES + 1);
        drop(lock);
        assert_eq!(fs::read(&path).unwrap(), b"old");
        assert_eq!(files_in(&root.0), vec!["settings.toml"]);
    }

    #[test]
    fn move_aside_takes_the_first_free_number() {
        let root = TempRoot::new("move_aside_takes_the_first_free_number");
        let path = root.0.join("settings.toml");
        fs::write(&path, b"one").unwrap();
        assert_eq!(
            move_aside(&path, "corrupt").unwrap(),
            root.0.join("settings.corrupt-1.toml")
        );
        fs::write(&path, b"two").unwrap();
        assert_eq!(
            move_aside(&path, "corrupt").unwrap(),
            root.0.join("settings.corrupt-2.toml")
        );
        fs::write(&path, b"three").unwrap();
        assert_eq!(
            move_aside(&path, "newer").unwrap(),
            root.0.join("settings.newer-1.toml")
        );
        assert_eq!(fs::read(root.0.join("settings.corrupt-1.toml")).unwrap(), b"one");
        assert_eq!(fs::read(root.0.join("settings.corrupt-2.toml")).unwrap(), b"two");
        assert_eq!(fs::read(root.0.join("settings.newer-1.toml")).unwrap(), b"three");
        assert!(!path.exists());
        let bare = root.0.join("queue");
        fs::write(&bare, b"q").unwrap();
        assert_eq!(move_aside(&bare, "corrupt").unwrap(), root.0.join("queue.corrupt-1"));
    }

    #[test]
    fn move_aside_never_writes_over_a_file_already_there() {
        let root = TempRoot::new("move_aside_never_writes_over_a_file_already_there");
        let path = root.0.join("settings.toml");
        fs::write(root.0.join("settings.corrupt-1.toml"), b"kept before").unwrap();
        fs::write(&path, b"now").unwrap();
        assert_eq!(
            move_aside(&path, "corrupt").unwrap(),
            root.0.join("settings.corrupt-2.toml")
        );
        assert_eq!(
            fs::read(root.0.join("settings.corrupt-1.toml")).unwrap(),
            b"kept before"
        );
        assert_eq!(fs::read(root.0.join("settings.corrupt-2.toml")).unwrap(), b"now");
    }

    #[test]
    fn a_name_taken_at_the_last_moment_is_skipped() {
        let root = TempRoot::new("a_name_taken_at_the_last_moment_is_skipped");
        let path = root.0.join("settings.toml");
        fs::write(&path, b"mine").unwrap();
        // Another copy of the app makes settings.corrupt-1.toml just as this one is about to claim it.
        let mut first = true;
        BEFORE_CLAIM.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |candidate: &Path| {
                if std::mem::take(&mut first) {
                    fs::write(candidate, b"theirs").unwrap();
                }
            }));
        });
        let moved = move_aside(&path, "corrupt");
        BEFORE_CLAIM.with(|hook| *hook.borrow_mut() = None);
        assert_eq!(moved.unwrap(), root.0.join("settings.corrupt-2.toml"));
        assert_eq!(fs::read(root.0.join("settings.corrupt-1.toml")).unwrap(), b"theirs");
        assert_eq!(fs::read(root.0.join("settings.corrupt-2.toml")).unwrap(), b"mine");
        assert!(!path.exists());
    }

    #[cfg(windows)]
    #[test]
    fn a_move_that_fails_gives_back_the_name_it_claimed() {
        let root = TempRoot::new("a_move_that_fails_gives_back_the_name_it_claimed");
        let path = root.0.join("settings.toml");
        fs::write(&path, b"old").unwrap();
        let lock = lock_exclusively(&path);
        assert!(move_aside(&path, "unread").is_err());
        drop(lock);
        assert_eq!(files_in(&root.0), vec!["settings.toml"]);
    }

    #[test]
    fn a_fault_in_the_writer_is_reported_in_words() {
        let root = TempRoot::new("a_fault_in_the_writer_is_reported_in_words");
        let paths = root.paths();
        let writer = Writer::start(paths.clone());
        writer.shared.lock().panic_next = true;
        writer.settings(&sample_settings(2));
        writer.flush();
        assert_eq!(writer.problems(), vec![WRITER_STOPPED.to_string()]);
        // What was pending is still written, on the caller's thread.
        writer.settings(&sample_settings(3));
        writer.flush();
        assert_eq!(load_settings(&paths.settings(), 8).0, sample_settings(3));
    }

    #[test]
    fn the_writer_writes_the_newest() {
        let root = TempRoot::new("the_writer_writes_the_newest");
        let paths = root.paths();
        let writer = Writer::start(paths.clone());
        writer.settings(&sample_settings(1));
        writer.settings(&sample_settings(2));
        writer.settings(&sample_settings(3));
        writer.flush();
        assert_eq!(load_settings(&paths.settings(), 8), (sample_settings(3), None));
        assert!(writer.problems().is_empty());
    }

    #[test]
    fn rapid_saves_become_one_write() {
        let root = TempRoot::new("rapid_saves_become_one_write");
        let paths = root.paths();
        let writer = Writer::start(paths.clone());
        writer.hold(true);
        for workers in 1..=5 {
            writer.settings(&sample_settings(workers));
        }
        writer.hold(false);
        writer.flush();
        assert_eq!(writer.shared.lock().files_written, 1);
        assert_eq!(load_settings(&paths.settings(), 8).0, sample_settings(5));
    }

    #[test]
    fn the_writer_saves_the_queue() {
        let root = TempRoot::new("the_writer_saves_the_queue");
        let paths = root.paths();
        let writer = Writer::start(paths.clone());
        let queue = Queue {
            jobs: Vec::new(),
            paused: true,
            next_id: 4,
        };
        writer.queue(&queue);
        writer.flush();
        let text = fs::read_to_string(paths.queue()).unwrap();
        assert_eq!(serde_json::from_str::<Queue>(&text).unwrap(), queue);
        assert_eq!(files_in(&paths.config), vec!["queue.json"]);
    }

    #[test]
    fn dropping_the_writer_finishes_its_writes_and_ends_the_thread() {
        let root = TempRoot::new("dropping_the_writer_finishes_its_writes_and_ends_the_thread");
        let paths = root.paths();
        let writer = Writer::start(paths.clone());
        writer.hold(true);
        writer.settings(&sample_settings(6));
        let shared = Arc::clone(&writer.shared);
        drop(writer);
        // The thread held the only other reference; it has let go of it, so it has ended.
        assert_eq!(Arc::strong_count(&shared), 1);
        assert!(shared.lock().stopped);
        assert_eq!(load_settings(&paths.settings(), 8).0, sample_settings(6));
    }

    #[test]
    fn a_write_that_fails_is_reported_in_words() {
        let root = TempRoot::new("a_write_that_fails_is_reported_in_words");
        let paths = root.paths();
        // The config folder is a plain file, so nothing can be written under it.
        fs::write(&paths.config, b"not a folder").unwrap();
        let writer = Writer::start(paths.clone());
        writer.settings(&sample_settings(3));
        writer.flush();
        let problems = writer.problems();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(
            problems[0].starts_with("Your settings could not be saved to"),
            "{}",
            problems[0]
        );
        assert!(
            problems[0].contains(&paths.settings().display().to_string()),
            "{}",
            problems[0]
        );
        assert!(writer.problems().is_empty());
        assert_eq!(fs::read(&paths.config).unwrap(), b"not a folder");
    }

    #[test]
    fn the_same_problem_twice_is_said_once() {
        let root = TempRoot::new("the_same_problem_twice_is_said_once");
        let paths = root.paths();
        fs::write(&paths.config, b"not a folder").unwrap();
        let writer = Writer::start(paths.clone());
        writer.settings(&sample_settings(3));
        writer.flush();
        writer.settings(&sample_settings(4));
        writer.flush();
        assert_eq!(writer.problems().len(), 1);
    }
}
