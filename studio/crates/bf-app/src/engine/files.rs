//! Files and folders: `ScanLibrary`, `OpenFight`, `OpenFightText`, `CheckFolder`, `CreateFolder`, `PickFolder`
//! and `RevealFolder`.
//!
//! Every command is answered on a thread of its own, so a slow or unplugged drive never holds up the coordinator:
//!
//! - `ScanLibrary`: first [`clear_abandoned`] on the root with [`ABANDONED_AFTER`] (one day), each of its answers
//!   a `Problem` notice (how many hidden folders were removed; each folder left, and why, once per run of the
//!   app); then `Library` (the videos under the root, with each fight file read as the engine reads it; a root
//!   that is missing or cannot be listed gives no videos and the problem in words), then a `LibraryCover` for each
//!   of the newest `MAX_COVERS` videos whose `cover.png` can be read. A newer scan silences an older one: nothing
//!   more of the older is sent, and the older skips the fight files it has not read yet, but listing the folders
//!   (inside `library::scan`) cannot be interrupted.
//! - `OpenFight`, `OpenFightText`: the fight file read, parsed and checked against what this app has, then
//!   `FightOpened`, or `Problem` ("{file name} cannot be opened: {reason}"). One open is pending at a time: a newer
//!   open (the person's own request, ruling M2-R20) replaces an older one, whose answer is then never sent. Nothing
//!   else drops an open.
//! - `CheckFolder`: `Folder` with the root and name exactly as asked, so the state can tell a late answer.
//! - `CreateFolder`: makes the folder asked for (never its missing parents, never anything inside it), then
//!   `Folder` with an empty name; a folder that cannot be made is a `Problem` in words first (always sent). A path
//!   refused before anything is tried (relative or empty) is answered `Missing` without being checked.
//! - A newer `CheckFolder` or `CreateFolder` silences the older one still running: its `Folder` is never sent.
//! - `PickFolder`: the folder dialog, then `FolderPicked`. One dialog at a time: a `PickFolder` while one is open
//!   is ignored (the open one answers). Once [`set_parent_window`] has been given the app's window, the dialog
//!   belongs to it: it opens in front of the app, and the app's window takes no clicks while it is open.
//! - `RevealFolder`: shows the folder in the PC's file browser without waiting for it; only a folder that is there
//!   is shown (never a file, which the browser would open with its program), else a `Problem` in words.
//!
//! Nothing here overwrites anything, and the one thing that deletes is [`clear_abandoned`]: only the files an
//! unfinished render of this app wrote into its own hidden folder, by name, then that empty folder (its rules are
//! on the function). The two calls that touch the desktop (the dialog and starting the file browser) are the
//! functions in `desktop`; tests replace them, so no test opens a window.
//!
//! One gap is accepted: an older open that passed its last check a moment before a newer open finishes could still
//! send its answer after the newer one. The newer open sets the older one's flag before it starts, so the older
//! would have to stall between its check and its send for longer than the whole newer open takes.

use std::collections::HashSet;
use std::fmt::Display;
use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use bf_core::fightfile::FightFile;
use bf_export::output;

use super::convert::{draft_of, plain, title_of};
use super::{Ctx, lock, problem};
use crate::model::library::FolderState;
use crate::model::take::Frame;
use crate::seam::{Command, Event};
use crate::store::library::{self, FightSummary};
use crate::wording;

/// The largest fight file read: a real one is a few kilobytes.
const MAX_FIGHT_FILE: u64 = 1 << 20;

/// The largest cover read: a real one (1080 x 1920) is a few megabytes.
const MAX_COVER: u64 = 64 << 20;

/// Covers are sent for the newest this many videos only (each is a file of a few megabytes read and made into a thumbnail); every video is still
/// listed.
const MAX_COVERS: usize = 500;

/// A fighter this app does not know has no colour of its own: grey, since colour belongs to fighters.
const NO_COLOR: [u8; 3] = [0x80, 0x80, 0x80];

/// The folder dialog's title.
const DIALOG_TITLE: &str = "Choose the output folder";

/// What this handler keeps between commands.
#[derive(Default)]
pub struct State {
    /// The open waiting for its answer: set when a newer open replaces it.
    open: Option<Arc<AtomicBool>>,
    /// The scan in progress: set when a newer scan replaces it.
    scan: Option<Arc<AtomicBool>>,
    /// The newest `CheckFolder` or `CreateFolder`: set when a newer one replaces it, so its `Folder` answer is not
    /// sent after the newer one's.
    folder: Option<Arc<AtomicBool>>,
    /// True while a folder dialog is open.
    picking: Arc<AtomicBool>,
    /// The hidden folders `clear_abandoned` left that the person has been told about in this run of the app.
    told: Arc<Mutex<HashSet<PathBuf>>>,
}

/// Takes one command and returns at once; the work goes to its own thread (`Ctx::spawn`).
pub fn handle(state: &mut State, ctx: &Ctx, command: Command) {
    match command {
        Command::ScanLibrary { root } => scan(state, ctx, root),
        Command::OpenFight { path } => open(state, ctx, move || open_file(&path)),
        Command::OpenFightText { toml } => open(state, ctx, move || open_text(toml)),
        Command::CheckFolder { root, name } => folder(state, ctx, root, name, false),
        Command::CreateFolder { root } => folder(state, ctx, root, String::new(), true),
        Command::PickFolder { start } => pick(state, ctx, start),
        Command::RevealFolder { path } => {
            ctx.spawn(
                "engine: show folder",
                "showing the folder",
                move |ctx| {
                    if let Err(text) = reveal(&path) {
                        ctx.emit(problem(text));
                    }
                },
                problem,
            );
        }
        // The coordinator sends this handler only its own commands.
        _ => {}
    }
}

/// Whether the work that holds `flag` should send nothing more: replaced by newer work, or the engine is ending.
fn stopped(ctx: &Ctx, flag: &AtomicBool) -> bool {
    flag.load(Ordering::SeqCst) || ctx.stopping()
}

/// Sends `event` unless the work that holds `flag` was replaced or the engine is ending (checked right before the
/// send).
fn send_unless_stopped(ctx: &Ctx, flag: &AtomicBool, event: Event) {
    if !stopped(ctx, flag) {
        ctx.emit(event);
    }
}

/// Checks the folder `root` and the name `name` in it, after creating `root` first when `create` (then `name` is
/// empty). The older check or create still running is silenced: its `Folder` answer is never sent. A create's
/// `Problem` is always sent.
fn folder(state: &mut State, ctx: &Ctx, root: PathBuf, name: String, create: bool) {
    if let Some(older) = &state.folder {
        older.store(true, Ordering::SeqCst);
    }
    let flag = ctx.cancel_flag();
    let mine = flag.clone();
    let (r, n) = (root.clone(), name.clone());
    let what = if create {
        "creating the output folder"
    } else {
        "checking the output folder"
    };
    let started = ctx.spawn(
        "engine: folder",
        what,
        move |ctx| {
            let state = if create {
                match create_folder(&root) {
                    Ok(()) => library::folder_state(&root, &name),
                    Err(refusal) => {
                        ctx.emit(problem(refusal.text));
                        if refusal.probe {
                            library::folder_state(&root, &name)
                        } else {
                            FolderState::Missing
                        }
                    }
                }
            } else {
                library::folder_state(&root, &name)
            };
            send_unless_stopped(&ctx, &mine, Event::Folder { root, name, state });
        },
        move |text| Event::Folder {
            root: r,
            name: n,
            state: FolderState::NotWritable(text),
        },
    );
    if started {
        state.folder = Some(flag);
    }
}

/// Starts a scan of `root`, stopping the one before.
fn scan(state: &mut State, ctx: &Ctx, root: PathBuf) {
    if let Some(older) = &state.scan {
        older.store(true, Ordering::SeqCst);
    }
    let flag = ctx.cancel_flag();
    let mine = flag.clone();
    let r = root.clone();
    let told = state.told.clone();
    let started = ctx.spawn(
        "engine: library",
        "reading the Library",
        move |ctx| {
            clear_before_scan(&root, SystemTime::now(), &told, &mut |text| {
                let send = !stopped(&ctx, &mine);
                if send {
                    ctx.emit(problem(text));
                }
                send
            });
            scan_library(&ctx, root, &mine, MAX_COVERS);
        },
        move |text| Event::Library {
            root: r,
            entries: Vec::new(),
            problem: Some(text),
        },
    );
    if started {
        state.scan = Some(flag);
    }
}

/// The Library of `root`: `Library`, then the `LibraryCover` of each of the newest `covers` videos, unless `cancel`
/// is set meanwhile. Once `cancel` is set, the fight files still to be read are skipped (each is answered with an
/// empty problem that is never sent), but listing the folders cannot be stopped.
fn scan_library(ctx: &Ctx, root: PathBuf, cancel: &AtomicBool, covers: usize) {
    let read = |text: &str| {
        if stopped(ctx, cancel) {
            Err(String::new())
        } else {
            summary(text)
        }
    };
    let found = library::scan(&root, &read);
    if stopped(ctx, cancel) {
        return;
    }
    let entries = match found {
        Ok(entries) => entries,
        Err(text) => {
            ctx.emit(Event::Library {
                root,
                entries: Vec::new(),
                problem: Some(text),
            });
            return;
        }
    };
    // The entries come newest first.
    let folders: Vec<PathBuf> = entries.iter().take(covers).map(|e| e.folder.clone()).collect();
    ctx.emit(Event::Library {
        root,
        entries,
        problem: None,
    });
    for folder in folders {
        if stopped(ctx, cancel) {
            return;
        }
        // The Library shows its own empty picture for a cover it does not get.
        let Some(cover) = cover_of(&folder) else {
            continue;
        };
        send_unless_stopped(ctx, cancel, Event::LibraryCover { folder, cover });
    }
}

/// The thumbnail of `folder/cover.png`, or None when it is not a plain file (a link counts as missing, ruling
/// M2-R18), cannot be read or is not a cover.
fn cover_of(folder: &Path) -> Option<Frame> {
    let path = folder.join("cover.png");
    if !fs::symlink_metadata(&path).ok()?.is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    fs::File::open(&path)
        .ok()?
        .take(MAX_COVER + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_COVER {
        return None;
    }
    library::thumbnail(&bytes).ok()
}

/// How old an unfinished render's hidden folder must be, by its newest file, before a Library scan clears it.
pub const ABANDONED_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// What [`clear_abandoned`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cleared {
    /// How many hidden folders were removed.
    pub removed: usize,
    /// Each folder of the export's temporary form, old enough and of no running program, that was left as it is,
    /// with why and what to do, in words.
    pub left: Vec<(PathBuf, String)>,
}

/// One clearing at a time in this program, so two Library scans never work on the same folder at once.
static CLEARING: Mutex<()> = Mutex::new(());

/// Removes what unfinished renders left in the output folder `root`: their hidden temporary folders.
///
/// A folder is removed only when every one of these holds:
/// 1. it sits directly in `root` (only `root`'s own list is read; nothing below a folder that is refused);
/// 2. its name has exactly the export's form, `.tmp-<name>-<pid>` or `.tmp-<name>-<pid>-<n>` (n from 2 to 64;
///    `bf_export::output::temp_folder_owners`), where `<name>` holds `_vs_`;
/// 3. no program with the pid in its name is running (every pid the name can be read with), or the pid is this
///    program's own and none of its jobs is using a folder of that name (`temp_folder_in_use`);
/// 4. it is a real folder, not a link or junction, and holds no link, no junction and no folder;
/// 5. its newest time (the folder's and each file's, modified or created, whichever is later) is at least
///    `older_than` before `now`; a time that cannot be read counts as new;
/// 6. it holds no finished video (a file ending in `.mp4` in any letter case; a `.mp4.part` is not one) and no
///    fight file (ending in `.toml`);
/// 7. every file in it is one the export writes there (`temp_folder_files`: `audio.wav`, `<name>.mp4.part`,
///    `encoding.txt`, `cover.png`, `caption.txt`), with exactly that name.
///
/// Then those files are removed one by one by name, the folder is checked again before each, and the folder
/// itself is removed with `remove_dir` (which only removes an empty folder). There is no recursive delete, and each
/// removal is tried once. A folder that fails 2 or 3, or is too new, is passed over without a word: it is not this
/// app's leftover, or a render may still be using it. A folder that fails 4, 6 or 7, cannot be read, or could not
/// be removed is left as it is and named in [`Cleared::left`] with why and what to do. Elsewhere than Windows every
/// pid counts as running, so nothing is ever removed.
///
/// `Err` is the sentence for a `root` that cannot be listed (the Library says so in its own words too).
pub fn clear_abandoned(root: &Path, older_than: Duration, now: SystemTime) -> Result<Cleared, String> {
    let _one = CLEARING.lock().unwrap_or_else(|e| e.into_inner());
    let listing = fs::read_dir(root).map_err(|_| {
        format!(
            "The output folder {} could not be read, so what unfinished renders left in it was not looked at.",
            root.display()
        )
    })?;
    let mut cleared = Cleared::default();
    for entry in listing.filter_map(Result::ok) {
        // A name that is not Unicode is not one the export makes.
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let owners = output::temp_folder_owners(&name);
        if owners.is_empty() {
            continue;
        }
        let folder = root.join(&name);
        match judge(&folder, &name, &owners, older_than, now) {
            Verdict::PassOver => {}
            Verdict::Leave(why) => cleared.left.push((folder, why)),
            Verdict::Remove(files) => match remove_leftover(&folder, &files) {
                Ok(()) => cleared.removed += 1,
                Err(why) => cleared.left.push((folder, why)),
            },
        }
    }
    Ok(cleared)
}

/// What to do with one folder of the export's temporary form.
enum Verdict {
    /// Not this app's leftover, or perhaps still in use: not touched, not mentioned.
    PassOver,
    /// Left as it is, with why, in words.
    Leave(String),
    /// Abandoned: these files (all the export's own) and then the folder.
    Remove(Vec<PathBuf>),
}

/// Why a folder is left, from the most to the least important to say.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Held {
    Video,
    FightFile,
    Link,
    Folder,
    Other,
}

/// Whether the program with id `pid` might be using a temporary folder named `name`.
fn owner_running(pid: u32, name: &str) -> bool {
    if pid == std::process::id() {
        output::temp_folder_in_use(name)
    } else {
        process_running(pid)
    }
}

/// Whether a program with id `pid` is running. Anything but a clear "there is no such program" counts as running
/// (a program of another account that this one may not look at, say).
#[cfg(windows)]
fn process_running(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_INVALID_PARAMETER, GetLastError, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
    // SAFETY: OpenProcess and GetLastError take plain values. A handle that is not null is waited on with no wait
    // at all and closed exactly once, right after; it is never used again.
    unsafe {
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            // Windows answers "the parameter is incorrect" for an id no program has.
            return GetLastError() != ERROR_INVALID_PARAMETER;
        }
        let ended = WaitForSingleObject(handle, 0) == WAIT_OBJECT_0;
        CloseHandle(handle);
        !ended
    }
}

/// Elsewhere (the app is for Windows) every program counts as running, so nothing is ever cleared.
#[cfg(not(windows))]
fn process_running(_pid: u32) -> bool {
    true
}

/// Whether `meta` (read without following links) is a link, a junction or any other reparse point.
fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    meta.file_type().is_symlink()
}

/// The later of `meta`'s modified and created times, or None when either cannot be read.
fn newest_time(meta: &fs::Metadata) -> Option<SystemTime> {
    Some(meta.modified().ok()?.max(meta.created().ok()?))
}

/// Whether `folder` is still a real folder, not a link, right now.
fn real_folder(folder: &Path) -> bool {
    fs::symlink_metadata(folder).is_ok_and(|m| m.is_dir() && !is_link(&m))
}

/// The verdict on the folder `folder`, named `name`, which reads as the temporary folder of each of `owners`.
fn judge(folder: &Path, name: &str, owners: &[(String, u32)], older_than: Duration, now: SystemTime) -> Verdict {
    let Ok(meta) = fs::symlink_metadata(folder) else {
        return Verdict::PassOver;
    };
    if is_link(&meta) {
        return Verdict::Leave(format!(
            "The hidden folder {} is a link to somewhere else, so BALLFIGHT Studio did not touch it or what it \
             leads to. Delete it yourself if you do not need it.",
            folder.display()
        ));
    }
    if !meta.is_dir() || owners.iter().any(|(_, pid)| owner_running(*pid, name)) {
        return Verdict::PassOver;
    }
    let unreadable = || {
        Verdict::Leave(format!(
            "The hidden folder {}, left by an unfinished render, could not be read, so it was left as it is. \
             Delete it yourself if you do not need it.",
            folder.display()
        ))
    };
    let Ok(listing) = fs::read_dir(folder) else {
        return unreadable();
    };
    let ours: Vec<String> = owners.iter().flat_map(|(n, _)| output::temp_folder_files(n)).collect();
    let mut newest = newest_time(&meta);
    let mut files = Vec::new();
    let mut held: Option<(Held, String)> = None;
    for child in listing {
        let Ok(child) = child else {
            return unreadable();
        };
        let child_name = child.file_name();
        let shown = child_name.to_string_lossy().into_owned();
        let path = folder.join(&child_name);
        let Ok(m) = fs::symlink_metadata(&path) else {
            return unreadable();
        };
        newest = newest.zip(newest_time(&m)).map(|(a, b)| a.max(b));
        let lower = shown.to_lowercase();
        let kind = if is_link(&m) {
            Some(Held::Link)
        } else if m.is_dir() {
            Some(Held::Folder)
        } else if lower.ends_with(".mp4") {
            Some(Held::Video)
        } else if lower.ends_with(".toml") {
            Some(Held::FightFile)
        } else if m.is_file() && child_name.to_str().is_some_and(|n| ours.iter().any(|o| o == n)) {
            None
        } else {
            Some(Held::Other)
        };
        match kind {
            None => files.push(path),
            Some(kind) => {
                if held.as_ref().is_none_or(|(h, _)| kind < *h) {
                    held = Some((kind, shown));
                }
            }
        }
    }
    let old_enough = newest
        .and_then(|newest| now.duration_since(newest).ok())
        .is_some_and(|age| age >= older_than);
    if !old_enough {
        return Verdict::PassOver;
    }
    let Some((kind, what)) = held else {
        return Verdict::Remove(files);
    };
    let f = folder.display();
    Verdict::Leave(match kind {
        Held::Video => format!(
            "The hidden folder {f} holds a finished video ({what}), so BALLFIGHT Studio left it as it is. Move the \
             video somewhere else, then delete the folder yourself."
        ),
        Held::FightFile => format!(
            "The hidden folder {f} holds a fight file ({what}), so BALLFIGHT Studio left it as it is. Move the file \
             somewhere else if you want to keep it, then delete the folder yourself."
        ),
        Held::Link => format!(
            "The hidden folder {f} holds a link to somewhere else ({what}), so BALLFIGHT Studio left it as it is. \
             Delete the folder yourself if you do not need it."
        ),
        Held::Folder => format!(
            "The hidden folder {f} holds a folder that BALLFIGHT Studio did not make ({what}), so it was left as it \
             is. Look inside it, then delete it yourself if you do not need what it holds."
        ),
        Held::Other => format!(
            "The hidden folder {f} holds a file that BALLFIGHT Studio did not make ({what}), so it was left as it \
             is. Look inside it, then delete it yourself if you do not need what it holds."
        ),
    })
}

/// Removes `files` (each by its name, once) and then the folder `folder`, which must be empty by then. The folder
/// is checked again before each file. `Err` says what was left, in words.
fn remove_leftover(folder: &Path, files: &[PathBuf]) -> Result<(), String> {
    let f = folder.display();
    for file in files {
        if !real_folder(folder) {
            return Err(format!(
                "The hidden folder {f}, left by an unfinished render, changed while BALLFIGHT Studio cleared it, so \
                 the rest of it was left. Delete it yourself if you do not need it."
            ));
        }
        match fs::remove_file(file) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(format!(
                    "The file {} in the hidden folder {f}, left by an unfinished render, could not be removed, so \
                     the rest of the folder was left. Delete it yourself; if Windows says the file is in use, try \
                     again after restarting the PC.",
                    file.file_name().unwrap_or_default().to_string_lossy()
                ));
            }
        }
    }
    match fs::remove_dir(folder) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(format!(
            "The hidden folder {f}, left by an unfinished render, was emptied but could not be removed (something \
             may have been put in it meanwhile), so it was left. Look inside it, then delete it yourself."
        )),
    }
}

/// The sentence for `removed` hidden folders cleared.
fn removed_text(removed: usize) -> String {
    if removed == 1 {
        "BALLFIGHT Studio removed 1 hidden folder that an unfinished render left in the output folder more than a \
         day ago. It held no finished video."
            .to_string()
    } else {
        format!(
            "BALLFIGHT Studio removed {removed} hidden folders that unfinished renders left in the output folder \
             more than a day ago. They held no finished video."
        )
    }
}

/// Clears `root` before a scan and hands each sentence for the person to `send`: how many folders were removed
/// (when any were), then each folder left that this run of the app has not told about yet. A folder counts as told
/// only when `send` says it was sent. Nothing is said when `root` cannot be listed: the scan says that itself.
fn clear_before_scan(
    root: &Path,
    now: SystemTime,
    told: &Mutex<HashSet<PathBuf>>,
    send: &mut dyn FnMut(String) -> bool,
) {
    let Ok(cleared) = clear_abandoned(root, ABANDONED_AFTER, now) else {
        return;
    };
    if cleared.removed > 0 {
        send(removed_text(cleared.removed));
    }
    for (folder, why) in cleared.left {
        if lock(told).contains(&folder) {
            continue;
        }
        if send(why) {
            lock(told).insert(folder);
        }
    }
}

/// A fight file's text read and checked as the engine reads it: parsed, then checked against what this app has
/// (the fighters, the theme and its chords). The error is the engine's own text (`field: message`).
fn checked(text: &str) -> Result<FightFile, String> {
    let ff = FightFile::parse(text).map_err(|e| e.to_string())?;
    bf_content::validate(&ff).map_err(|e| e.to_string())?;
    Ok(ff)
}

/// What the Library shows of a video's fight file: "Flute vs Trombone", the fighters' colours and the seed; or why
/// the video cannot be reopened, in words.
fn summary(text: &str) -> Result<FightSummary, String> {
    let ff = checked(text).map_err(|e| plain(&format!("this video's fight file cannot be used: {e}")))?;
    let name = |i: usize| ff.fighters.get(i).cloned().unwrap_or_default();
    let names = [name(0), name(1)];
    Ok(FightSummary {
        title: wording::versus(&names),
        colors: [color_of(&names[0]), color_of(&names[1])],
        seed: ff.seed,
    })
}

/// A fighter's colour from the roster.
fn color_of(name: &str) -> [u8; 3] {
    bf_content::roster()
        .iter()
        .find(|f| f.name == name)
        .map_or(NO_COLOR, |f| f.color)
}

/// Starts an open, replacing the one waiting: the older one's answer is never sent.
fn open<R>(state: &mut State, ctx: &Ctx, read: R)
where
    R: FnOnce() -> Result<(FightFile, String), String> + Send + 'static,
{
    if let Some(older) = &state.open {
        older.store(true, Ordering::SeqCst);
    }
    let flag = ctx.cancel_flag();
    let mine = flag.clone();
    let started = ctx.spawn(
        "engine: open",
        "opening the fight",
        move |ctx| {
            let answer = match read() {
                Ok((ff, text)) => Event::FightOpened {
                    draft: draft_of(&ff, &text),
                    title: title_of(&ff),
                },
                Err(text) => problem(text),
            };
            send_unless_stopped(&ctx, &mine, answer);
        },
        problem,
    );
    if started {
        state.open = Some(flag);
    }
}

/// The fight file at `path` and its text, or "{file name} cannot be opened: {reason}".
fn open_file(path: &Path) -> Result<(FightFile, String), String> {
    let cannot = |reason: &dyn Display| format!("{} cannot be opened: {}", file_label(path), plain(reason));
    let text = fight_text(path).map_err(|reason| cannot(&reason))?;
    let ff = checked(&text).map_err(|e| cannot(&e))?;
    Ok((ff, text))
}

/// A queued video's fight text, or "This fight cannot be opened: {reason}".
fn open_text(toml: String) -> Result<(FightFile, String), String> {
    let ff = checked(&toml).map_err(|e| format!("This fight cannot be opened: {}", plain(&e)))?;
    Ok((ff, toml))
}

/// How a file is named in a sentence: its name, or the whole path when it has none.
fn file_label(path: &Path) -> String {
    match path.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None if path.as_os_str().is_empty() => "The fight file".to_string(),
        None => path.display().to_string(),
    }
}

/// A fight file's text (without a leading byte-order mark), or why it cannot be read, in words.
fn fight_text(path: &Path) -> Result<String, &'static str> {
    let unreadable = |e: io::Error| match e.kind() {
        io::ErrorKind::NotFound => "the file is not there any more. Refresh the Library to see what is there now",
        io::ErrorKind::PermissionDenied => "your Windows account is not allowed to read it",
        _ => "it could not be read. Check that its drive is connected",
    };
    // `symlink_metadata` does not follow a link: an open never reads through one (ruling M2-R18).
    let kind = fs::symlink_metadata(path).map_err(unreadable)?.file_type();
    if kind.is_dir() {
        return Err("it is a folder, not a fight file");
    }
    if !kind.is_file() {
        return Err("it is a link to somewhere else, not a fight file");
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(unreadable)?
        .take(MAX_FIGHT_FILE + 1)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;
    if bytes.len() as u64 > MAX_FIGHT_FILE {
        return Err("it is too big to be a fight file");
    }
    let text = String::from_utf8(bytes).map_err(|_| "it is not text, so it is not a fight file")?;
    Ok(text.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(text))
}

/// Makes the output folder `root` itself: never its missing parents (a mistyped path does not become a chain of
/// new folders), never anything inside it. A folder already there is fine. The error says why, in words.
fn create_folder(root: &Path) -> Result<(), Refusal> {
    let shown = root.display();
    // Refused before touching anything: no folder is probed for these either.
    let at_once = |text: String| Err(Refusal { text, probe: false });
    if root.as_os_str().is_empty() {
        return at_once("No output folder is set, so none was created. Choose a folder.".to_string());
    }
    if !root.is_absolute() {
        return at_once(format!(
            "The output folder {shown} does not start with its drive, so it was not created. Choose a folder."
        ));
    }
    let err = match fs::create_dir(root) {
        Ok(()) => return Ok(()),
        Err(err) => err,
    };
    if fs::metadata(root).is_ok_and(|m| m.is_dir()) {
        return Ok(());
    }
    let text = match err.kind() {
        io::ErrorKind::AlreadyExists => {
            format!("{shown} is a file, not a folder, so it cannot be the output folder. Choose another folder.")
        }
        io::ErrorKind::NotFound => match root.parent() {
            Some(parent) if parent.parent().is_some() => format!(
                "The folder {} that should hold the output folder is missing too, so {shown} was not created. \
                 Check that its drive is connected, or choose another folder.",
                parent.display()
            ),
            _ => format!(
                "The drive of {shown} is not there, so the output folder was not created. Check that the drive is \
                 connected, or choose another folder."
            ),
        },
        io::ErrorKind::PermissionDenied => {
            format!("Your Windows account is not allowed to create {shown}. Choose another folder.")
        }
        _ => format!(
            "The output folder {shown} could not be created. Check that its drive is connected and has free space, \
             or choose another folder."
        ),
    };
    Err(Refusal { text, probe: true })
}

/// Why `create_folder` made no folder: the sentence, and whether the folder is worth checking afterwards (false
/// for a path refused before anything was tried, which is answered `Missing` at once).
struct Refusal {
    text: String,
    probe: bool,
}

/// Clears the "a dialog is open" flag when dropped (also when the dialog's thread panics or never starts).
struct DialogOpen(Arc<AtomicBool>);

impl Drop for DialogOpen {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Every folder dialog starts from this: a dialog builder whose parent is the app's window, once
/// [`set_parent_window`] has been called.
static PARENT: OnceLock<rfd::FileDialog> = OnceLock::new();

/// Remembers the app's window as the folder dialog's parent, so the dialog opens in front of the app and the app
/// takes no clicks while it is open. `app::start_with` calls it once with the window's creation context; later
/// calls change nothing.
///
/// No unsafe code is needed: `rfd::FileDialog::set_parent` copies the window's handle (on Windows, its HWND, a
/// number) out of `cc` here, on the interface thread, and `rfd` makes `FileDialog` `Send` itself. The dialog is
/// still built from a clone of this and shown on the dialog's own thread, as `rfd`'s own `AsyncFileDialog` does on
/// Windows. The window outlives every dialog: it is the app's one window, made before the engine can be asked for a
/// dialog and closed only as the program ends, and while a dialog is open it takes no clicks, so it cannot be
/// closed from the window meanwhile. Should the handle ever be stale, Windows refuses it as a handle (it is never
/// read as memory) and the dialog shows no folder chosen. With no handle (a test's creation context) the dialog
/// has no parent, as before.
pub fn set_parent_window(cc: &eframe::CreationContext<'_>) {
    let _ = PARENT.set(rfd::FileDialog::new().set_parent(cc));
}

/// The folder dialog, from `template` (the one with the window as parent) when there is one: titled, opening at
/// `start` when given.
fn dialog_builder(template: Option<&rfd::FileDialog>, start: Option<&Path>) -> rfd::FileDialog {
    let dialog = template.cloned().unwrap_or_default().set_title(DIALOG_TITLE);
    match start {
        Some(start) => dialog.set_directory(start),
        None => dialog,
    }
}

/// Shows the folder dialog on its own thread, unless one is open already.
fn pick(state: &State, ctx: &Ctx, start: PathBuf) {
    if state.picking.swap(true, Ordering::SeqCst) {
        return;
    }
    let open = DialogOpen(state.picking.clone());
    // If the thread does not start, the work is dropped unrun and `open` with it, which clears the flag.
    ctx.spawn(
        "engine: folder dialog",
        "showing the folder dialog",
        move |ctx| {
            let path = desktop::pick_folder(dialog_start(&start).as_deref());
            drop(open);
            if !ctx.stopping() {
                ctx.emit(Event::FolderPicked { path });
            }
        },
        problem,
    );
}

/// Where the dialog opens: the nearest folder at or above `start` that is there, or None (the dialog's own
/// choice) when there is none, as on a drive that is not connected.
fn dialog_start(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|p| !p.as_os_str().is_empty() && p.is_dir())
        .map(Path::to_path_buf)
}

/// Shows `path` in the PC's file browser, or says why not.
fn reveal(path: &Path) -> Result<(), String> {
    let folder = reveal_target(path)?;
    desktop::show_folder(&folder).map_err(|_| {
        format!(
            "The app could not open {} in File Explorer. Open it from File Explorer yourself.",
            folder.display()
        )
    })
}

/// The folder to show for `path`, without a trailing separator; an error in words for anything but a folder that is
/// there (a file is never handed to the file browser, which would open it with its program).
fn reveal_target(path: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() {
        return Err("There is no folder to show.".to_string());
    }
    let folder: PathBuf = path.components().collect();
    match fs::metadata(&folder) {
        Ok(meta) if meta.is_dir() => Ok(std::path::absolute(&folder).unwrap_or(folder)),
        Ok(_) => Err(format!(
            "{} is a file, not a folder, so it was not shown.",
            path.display()
        )),
        Err(_) => Err(format!(
            "The folder {} is not there any more, so it cannot be shown. Check that its drive is connected.",
            path.display()
        )),
    }
}

/// The two calls that touch the desktop, kept as thin as they can be: everything around them is tested.
#[cfg(not(test))]
mod desktop {
    use std::io;
    use std::path::{Path, PathBuf};

    /// The PC's folder dialog, in front of the app's window once it is known, opening at `start`; waits until the
    /// person closes it.
    pub fn pick_folder(start: Option<&Path>) -> Option<PathBuf> {
        super::dialog_builder(super::PARENT.get(), start).pick_folder()
    }

    /// Starts File Explorer on `folder` and returns without waiting for it.
    #[cfg(windows)]
    pub fn show_folder(folder: &Path) -> io::Result<()> {
        use std::os::windows::process::CommandExt as _;
        let explorer = std::env::var_os("SystemRoot").map_or_else(
            || PathBuf::from("explorer.exe"),
            |root| PathBuf::from(root).join("explorer.exe"),
        );
        // One quoted argument, passed exactly: Explorer does not read its command line the usual way.
        let mut quoted = std::ffi::OsString::from("\"");
        quoted.push(folder);
        quoted.push("\"");
        std::process::Command::new(explorer).raw_arg(quoted).spawn().map(drop)
    }

    /// Starts the desktop's file browser on `folder` and returns without waiting for it.
    #[cfg(not(windows))]
    pub fn show_folder(folder: &Path) -> io::Result<()> {
        let mut child = std::process::Command::new("xdg-open").arg(folder).spawn()?;
        // Collected in the background, so no finished process is left behind.
        std::thread::spawn(move || child.wait());
        Ok(())
    }
}

/// Tests: the desktop calls record what they were asked and open nothing. A dialog started at a folder a test has
/// given an answer for stays open until the test sends it; any other dialog closes at once with no folder.
#[cfg(test)]
mod desktop {
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, mpsc};

    use crate::engine::lock;

    /// The start folder of every dialog shown.
    pub static DIALOGS: Mutex<Vec<Option<PathBuf>>> = Mutex::new(Vec::new());
    /// Every folder shown in the file browser.
    pub static SHOWN: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    /// A test's answer for the dialog started at a folder.
    pub static ANSWERS: Mutex<Vec<(PathBuf, mpsc::Receiver<Option<PathBuf>>)>> = Mutex::new(Vec::new());

    pub fn pick_folder(start: Option<&Path>) -> Option<PathBuf> {
        lock(&DIALOGS).push(start.map(Path::to_path_buf));
        let answer = {
            let mut answers = lock(&ANSWERS);
            let at = answers.iter().position(|(s, _)| Some(s.as_path()) == start)?;
            answers.remove(at).1
        };
        answer.recv().ok().flatten()
    }

    pub fn show_folder(folder: &Path) -> io::Result<()> {
        lock(&SHOWN).push(folder.to_path_buf());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::engine::{lock, test_engine, wait_for};
    use crate::model::library::LibraryEntry;
    use crate::seam::EngineHandle;
    use crate::store::{TempRoot, files_in};

    const FLUTE: [u8; 3] = [222, 232, 246];
    const TROMBONE: [u8; 3] = [120, 240, 184];

    /// The pair's fight with this seed and handicap, as the app writes it.
    fn fight_toml(seed: u64, handicap: [f64; 2]) -> String {
        let mut ff = bf_content::starter("FLUTE", "TROMBONE").expect("the pair's fight");
        ff.seed = Some(seed);
        ff.handicap = handicap.to_vec();
        ff.to_toml()
    }

    /// A 1080 x 1920 cover, the left half one colour and the right half another, made with the `png` crate.
    fn cover_png() -> Vec<u8> {
        let mut data = Vec::with_capacity(1080 * 1920 * 3);
        for _ in 0..1920 {
            for x in 0..1080 {
                data.extend(if x < 540 { [200, 10, 10] } else { [10, 10, 200] });
            }
        }
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, 1080, 1920);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&data).unwrap();
        writer.finish().unwrap();
        bytes
    }

    /// A video folder as a render leaves it: `cover.png` and `{name}.toml`.
    fn video(root: &Path, name: &str, toml: &str) -> PathBuf {
        let folder = root.join(name);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("cover.png"), cover_png()).unwrap();
        fs::write(folder.join(format!("{name}.toml")), toml).unwrap();
        folder
    }

    /// A temp folder for one test and, inside it, an output folder whose path holds spaces, "!" and letters outside
    /// ASCII, as the owner's own folders do.
    fn output_folder(test: &str) -> (TempRoot, PathBuf) {
        let temp = TempRoot::new(test);
        let videos = temp.0.join("Vidéos ! BALLFIGHT");
        fs::create_dir(&videos).unwrap();
        (temp, videos)
    }

    /// A drive letter that is not connected on this PC, as `Q:\`.
    fn missing_drive() -> Option<PathBuf> {
        if !cfg!(windows) {
            return None;
        }
        ('D'..='Z')
            .rev()
            .map(|letter| PathBuf::from(format!("{letter}:\\")))
            .find(|p| fs::metadata(p).is_err())
    }

    /// `path` written with a separator at the end.
    fn with_trailing_separator(path: &Path) -> PathBuf {
        let mut text = path.as_os_str().to_os_string();
        text.push(std::path::MAIN_SEPARATOR_STR);
        PathBuf::from(text)
    }

    /// The root of the drive the system's temp folder is on.
    fn drive_root() -> PathBuf {
        std::env::temp_dir().ancestors().last().unwrap().to_path_buf()
    }

    /// Ends the test engine and removes its own (empty) temp folder.
    fn finish(engine: EngineHandle, root: &Path) {
        engine.send(Command::Shutdown);
        drop(engine);
        fs::remove_dir(root).expect("the engine left its temp folder empty");
    }

    /// The `Library` answer for `root`.
    fn library_of(engine: &EngineHandle, root: &Path) -> (Vec<LibraryEntry>, Option<String>) {
        let Event::Library { entries, problem, .. } = wait_for(
            engine,
            10.0,
            |e| matches!(e, Event::Library { root: r, .. } if r == root),
        ) else {
            unreachable!()
        };
        (entries, problem)
    }

    /// The `Folder` answer for `root` and `name`.
    fn folder_of(engine: &EngineHandle, root: &Path, name: &str) -> FolderState {
        let Event::Folder { state, .. } = wait_for(
            engine,
            10.0,
            |e| matches!(e, Event::Folder { root: r, name: n, .. } if r == root && n == name),
        ) else {
            unreachable!()
        };
        state
    }

    /// The next `Problem`'s text.
    fn problem_text(engine: &EngineHandle) -> String {
        let Event::Problem { text } = wait_for(engine, 10.0, |e| matches!(e, Event::Problem { .. })) else {
            unreachable!()
        };
        text
    }

    /// Waits up to 10 s for `done`, checking every 10 ms.
    fn wait_until(what: &str, done: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !done() {
            assert!(Instant::now() < deadline, "{what} did not happen within 10 s");
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn a_rendered_folder_shows_in_the_library() {
        let (_temp, videos) = output_folder("a_rendered_folder_shows_in_the_library");
        let folder = video(&videos, "flute_vs_trombone", &fight_toml(7, [1.0, 1.0]));
        // What a render left behind is not a video, and it is not touched.
        let tmp = video(&videos, ".tmp-flute_vs_trombone-1", &fight_toml(8, [1.0, 1.0]));
        let (engine, root) = test_engine();
        engine.send(Command::ScanLibrary { root: videos.clone() });
        let (entries, problem) = library_of(&engine, &videos);
        assert_eq!(problem, None);
        assert_eq!(entries.len(), 1, "{entries:?}");
        let e = &entries[0];
        assert_eq!(e.folder, folder);
        assert_eq!(e.title, "Flute vs Trombone");
        assert_eq!(e.seed, Some(7));
        assert_eq!(e.colors, [FLUTE, TROMBONE]);
        assert_eq!(e.colors, [color_of("FLUTE"), color_of("TROMBONE")]);
        assert_eq!(e.fight_path, Some(folder.join("flute_vs_trombone.toml")));
        assert_eq!(e.problem, None);
        let Event::LibraryCover { folder: f, cover } =
            wait_for(&engine, 10.0, |e| matches!(e, Event::LibraryCover { .. }))
        else {
            unreachable!()
        };
        assert_eq!(f, folder);
        assert_eq!((cover.w, cover.h), (180, 320));
        assert_eq!(&cover.rgb[..3], &[200, 10, 10]);
        // Only one cover, and the scan wrote nothing.
        engine.send(Command::CheckFolder {
            root: videos.clone(),
            name: String::new(),
        });
        wait_for(&engine, 10.0, |e| {
            assert!(!matches!(e, Event::LibraryCover { .. }), "a second cover");
            matches!(e, Event::Folder { .. })
        });
        assert_eq!(files_in(&videos), [".tmp-flute_vs_trombone-1", "flute_vs_trombone"]);
        assert_eq!(files_in(&tmp).len(), 2);
        finish(engine, &root);
    }

    #[test]
    fn a_missing_output_folder_is_a_problem_not_a_crash() {
        let (temp, videos) = output_folder("a_missing_output_folder_is_a_problem_not_a_crash");
        let (engine, root) = test_engine();
        let missing = videos.join("Ünder ! here");
        engine.send(Command::ScanLibrary { root: missing.clone() });
        let (entries, problem) = library_of(&engine, &missing);
        assert!(entries.is_empty());
        assert_eq!(
            problem,
            Some(format!("The output folder {} is missing.", missing.display()))
        );
        engine.send(Command::CheckFolder {
            root: missing.clone(),
            name: "flute_vs_trombone".to_string(),
        });
        assert_eq!(folder_of(&engine, &missing, "flute_vs_trombone"), FolderState::Missing);
        // A file where the folder should be.
        let file = temp.0.join("a file ! é.txt");
        fs::write(&file, b"mine").unwrap();
        engine.send(Command::ScanLibrary { root: file.clone() });
        let (entries, problem) = library_of(&engine, &file);
        assert!(entries.is_empty());
        assert_eq!(
            problem,
            Some(format!("The output folder {} is missing.", file.display()))
        );
        engine.send(Command::CheckFolder {
            root: file.clone(),
            name: String::new(),
        });
        assert_eq!(folder_of(&engine, &file, ""), FolderState::Missing);
        assert_eq!(fs::read(&file).unwrap(), b"mine");
        // A drive that is not connected.
        if let Some(drive) = missing_drive() {
            let gone = drive.join("BALLFIGHT");
            engine.send(Command::ScanLibrary { root: gone.clone() });
            let (entries, problem) = library_of(&engine, &gone);
            assert!(entries.is_empty());
            let problem = problem.expect("a problem");
            assert!(problem.starts_with("The output folder "), "{problem}");
            assert!(problem.contains(&gone.display().to_string()), "{problem}");
            engine.send(Command::CheckFolder {
                root: gone.clone(),
                name: "x".to_string(),
            });
            let state = folder_of(&engine, &gone, "x");
            assert!(
                matches!(state, FolderState::Missing | FolderState::NotWritable(_)),
                "{state:?}"
            );
        }
        // A folder that is there, written with a separator at the end: answered as asked, with its videos.
        video(&videos, "flute_vs_trombone", &fight_toml(3, [1.0, 1.0]));
        let trailing = with_trailing_separator(&videos);
        engine.send(Command::ScanLibrary { root: trailing.clone() });
        let (entries, problem) = library_of(&engine, &trailing);
        assert_eq!((entries.len(), problem), (1, None));
        assert_eq!(entries[0].title, "Flute vs Trombone");
        // A drive's root is read like any folder (and nothing is written there).
        let drive = drive_root();
        engine.send(Command::ScanLibrary { root: drive.clone() });
        let (_, problem) = library_of(&engine, &drive);
        assert_eq!(problem, None);
        finish(engine, &root);
    }

    #[test]
    fn a_fight_from_a_newer_version_is_named() {
        let (_temp, videos) = output_folder("a_fight_from_a_newer_version_is_named");
        let toml = fight_toml(7, [1.0, 1.0]);
        assert!(toml.starts_with("format = 2\n"), "{toml}");
        let newer = toml.replacen("format = 2", "format = 3", 1);
        let folder = video(&videos, "flute_vs_trombone", &newer);
        let (engine, root) = test_engine();
        engine.send(Command::ScanLibrary { root: videos.clone() });
        let (entries, _) = library_of(&engine, &videos);
        assert_eq!(entries.len(), 1);
        let problem = entries[0].problem.clone().expect("a problem");
        assert!(
            problem.contains("cannot be used: This app reads fight files up to format 2"),
            "{problem}"
        );
        assert!(
            problem.starts_with("This video's fight file cannot be used: "),
            "{problem}"
        );
        assert!(problem.ends_with('.'), "{problem}");
        assert_eq!(entries[0].title, "flute_vs_trombone");
        // Opening it says the same, and the file is left as it was.
        engine.send(Command::OpenFight {
            path: folder.join("flute_vs_trombone.toml"),
        });
        let text = problem_text(&engine);
        assert!(text.starts_with("flute_vs_trombone.toml cannot be opened: "), "{text}");
        // the reason is a sentence of its own after the colon, as every reason an open gives
        assert!(text.contains("This app reads fight files up to format 2"), "{text}");
        assert_eq!(
            fs::read_to_string(folder.join("flute_vs_trombone.toml")).unwrap(),
            newer
        );
        finish(engine, &root);
    }

    #[test]
    fn reopening_gives_the_fight_back() {
        let (_temp, videos) = output_folder("reopening_gives_the_fight_back");
        let toml = fight_toml(7, [1.5, 0.8]);
        let folder = video(&videos, "flute_vs_trombone", &toml);
        let path = folder.join("flute_vs_trombone.toml");
        let (engine, root) = test_engine();
        for command in [
            Command::OpenFight { path: path.clone() },
            Command::OpenFightText { toml: toml.clone() },
        ] {
            engine.send(command);
            let Event::FightOpened { draft, title } =
                wait_for(&engine, 10.0, |e| matches!(e, Event::FightOpened { .. }))
            else {
                unreachable!()
            };
            assert_eq!(draft.fighters, ["FLUTE", "TROMBONE"]);
            assert_eq!(draft.handicap, [1.5, 0.8]);
            assert_eq!(draft.seed, Some(7));
            assert_eq!(draft.base.as_deref(), Some(toml.as_str()));
            assert_eq!(title, "Flute vs Trombone, seed 7");
        }
        // A file saved with a byte-order mark opens too.
        fs::write(&path, format!("\u{feff}{toml}")).unwrap();
        engine.send(Command::OpenFight { path: path.clone() });
        let Event::FightOpened { draft, .. } = wait_for(&engine, 10.0, |e| matches!(e, Event::FightOpened { .. }))
        else {
            unreachable!()
        };
        assert_eq!(draft.base.as_deref(), Some(toml.as_str()));
        finish(engine, &root);
    }

    #[test]
    fn a_broken_fight_file_is_a_problem() {
        let (_temp, videos) = output_folder("a_broken_fight_file_is_a_problem");
        let toml = fight_toml(7, [1.0, 1.0]);
        let (engine, root) = test_engine();
        let open = |name: &str, text: &[u8]| {
            let folder = videos.join(name);
            fs::create_dir_all(&folder).unwrap();
            let path = folder.join(format!("{name}.toml"));
            fs::write(&path, text).unwrap();
            engine.send(Command::OpenFight { path });
            problem_text(&engine)
        };
        let unknown = open("kazoo_vs_trombone", toml.replace("FLUTE", "KAZOO").as_bytes());
        assert!(
            unknown.starts_with("kazoo_vs_trombone.toml cannot be opened: "),
            "{unknown}"
        );
        assert!(unknown.contains("KAZOO") && unknown.ends_with('.'), "{unknown}");
        let retired = bf_core::fightfile::RETIRED[0];
        let gone = open("retired", toml.replace("FLUTE", retired).as_bytes());
        assert!(gone.starts_with("retired.toml cannot be opened: "), "{gone}");
        // the name as the screen writes it
        let shown = crate::wording::display_name(retired);
        assert!(gone.contains(&shown) && gone.contains("is retired"), "{gone}");
        let not_toml = open("not toml ! é", b"this is [not toml");
        assert!(
            not_toml.starts_with("not toml ! é.toml cannot be opened: ") && not_toml.ends_with('.'),
            "{not_toml}"
        );
        assert_eq!(
            open("binary", &[0xff, 0xfe, 0x00]),
            "binary.toml cannot be opened: It is not text, so it is not a fight file."
        );
        engine.send(Command::OpenFight {
            path: videos.join("gone.toml"),
        });
        assert_eq!(
            problem_text(&engine),
            "gone.toml cannot be opened: The file is not there any more. Refresh the Library to see what is there \
             now."
        );
        engine.send(Command::OpenFight { path: videos.clone() });
        assert_eq!(
            problem_text(&engine),
            "Vidéos ! BALLFIGHT cannot be opened: It is a folder, not a fight file."
        );
        engine.send(Command::OpenFightText {
            toml: toml.replace("FLUTE", "KAZOO"),
        });
        let text = problem_text(&engine);
        assert!(
            text.starts_with("This fight cannot be opened: ") && text.contains("KAZOO"),
            "{text}"
        );
        // The Library names the same problem in a sentence once the folder is a video.
        fs::write(videos.join("kazoo_vs_trombone").join("cover.png"), cover_png()).unwrap();
        engine.send(Command::ScanLibrary { root: videos.clone() });
        let (entries, _) = library_of(&engine, &videos);
        assert_eq!(entries.len(), 1);
        let problem = entries[0].problem.clone().expect("a problem");
        assert!(problem.contains("KAZOO") && problem.ends_with('.'), "{problem}");
        finish(engine, &root);
    }

    #[test]
    fn the_last_open_wins() {
        let (engine, root) = test_engine();
        engine.send(Command::OpenFightText {
            toml: fight_toml(7, [1.0, 1.0]),
        });
        engine.send(Command::OpenFightText {
            toml: fight_toml(8, [1.0, 1.0]),
        });
        wait_for(
            &engine,
            10.0,
            |e| matches!(e, Event::FightOpened { draft, .. } if draft.seed == Some(8)),
        );
        // Nothing comes after the newest.
        engine.send(Command::CheckFolder {
            root: root.join("unused"),
            name: String::new(),
        });
        wait_for(&engine, 10.0, |e| {
            assert!(!matches!(e, Event::FightOpened { .. }), "an older open answered late");
            matches!(e, Event::Folder { .. })
        });
        finish(engine, &root);
    }

    #[test]
    fn a_created_folder_is_ok() {
        let (_temp, videos) = output_folder("a_created_folder_is_ok");
        let (engine, root) = test_engine();
        let new = videos.join("Nëw ! folder");
        engine.send(Command::CreateFolder { root: new.clone() });
        assert_eq!(folder_of(&engine, &new, ""), FolderState::Ok { taken: false });
        assert!(new.is_dir());
        assert!(files_in(&new).is_empty(), "nothing is made inside it");
        // A folder already there is left as it is, also when written with a separator at the end.
        fs::write(new.join("mine.txt"), b"mine").unwrap();
        let trailing = with_trailing_separator(&new);
        engine.send(Command::CreateFolder { root: trailing.clone() });
        assert_eq!(folder_of(&engine, &trailing, ""), FolderState::Ok { taken: false });
        assert_eq!(files_in(&new), ["mine.txt"]);
        assert_eq!(fs::read(new.join("mine.txt")).unwrap(), b"mine");
        finish(engine, &root);
    }

    #[test]
    fn a_folder_that_cannot_be_created_is_named() {
        let (temp, videos) = output_folder("a_folder_that_cannot_be_created_is_named");
        let (engine, root) = test_engine();
        // Its parent is missing too: neither is made.
        let deep = videos.join("missing").join("BALLFIGHT");
        engine.send(Command::CreateFolder { root: deep.clone() });
        assert_eq!(
            problem_text(&engine),
            format!(
                "The folder {} that should hold the output folder is missing too, so {} was not created. Check that \
                 its drive is connected, or choose another folder.",
                videos.join("missing").display(),
                deep.display()
            )
        );
        assert_eq!(folder_of(&engine, &deep, ""), FolderState::Missing);
        assert!(files_in(&videos).is_empty());
        // A file in its place is kept.
        let file = temp.0.join("a file");
        fs::write(&file, b"mine").unwrap();
        engine.send(Command::CreateFolder { root: file.clone() });
        assert_eq!(
            problem_text(&engine),
            format!(
                "{} is a file, not a folder, so it cannot be the output folder. Choose another folder.",
                file.display()
            )
        );
        assert_eq!(folder_of(&engine, &file, ""), FolderState::Missing);
        assert_eq!(fs::read(&file).unwrap(), b"mine");
        // A path without its drive, and no path at all.
        for (path, text) in [
            (
                PathBuf::from("Videos"),
                "The output folder Videos does not start with its drive, so it was not created. Choose a folder.",
            ),
            (
                PathBuf::new(),
                "No output folder is set, so none was created. Choose a folder.",
            ),
        ] {
            // Refused before anything is tried, so answered `Missing` without checking the folder.
            assert!(create_folder(&path).is_err_and(|r| !r.probe && r.text == text));
            engine.send(Command::CreateFolder { root: path.clone() });
            assert_eq!(problem_text(&engine), text);
            assert_eq!(folder_of(&engine, &path, ""), FolderState::Missing);
        }
        assert!(!Path::new("Videos").exists());
        // The root of a drive that is not connected (a drive's root can never be made, so nothing can be written
        // even if the drive is connected meanwhile).
        if let Some(drive) = missing_drive() {
            engine.send(Command::CreateFolder { root: drive.clone() });
            let text = problem_text(&engine);
            assert!(text.contains(&drive.display().to_string()), "{text}");
            assert!(text.contains("drive") && text.ends_with('.'), "{text}");
            folder_of(&engine, &drive, "");
        }
        finish(engine, &root);
    }

    #[test]
    fn a_taken_name_is_reported() {
        let (_temp, videos) = output_folder("a_taken_name_is_reported");
        let (engine, root) = test_engine();
        let ask = |name: &str| {
            engine.send(Command::CheckFolder {
                root: videos.clone(),
                name: name.to_string(),
            });
            folder_of(&engine, &videos, name)
        };
        assert_eq!(ask("flute_vs_trombone"), FolderState::Ok { taken: false });
        video(&videos, "flute_vs_trombone", &fight_toml(7, [1.0, 1.0]));
        assert_eq!(ask("flute_vs_trombone"), FolderState::Ok { taken: true });
        assert_eq!(ask("trombone_vs_flute"), FolderState::Ok { taken: false });
        assert_eq!(ask(""), FolderState::Ok { taken: false });
        // The check leaves nothing behind.
        assert_eq!(files_in(&videos), ["flute_vs_trombone"]);
        finish(engine, &root);
    }

    #[test]
    fn one_folder_dialog_at_a_time() {
        let (_temp, videos) = output_folder("one_folder_dialog_at_a_time");
        let picked = videos.join("Picked ! é");
        let (answer, waiting) = mpsc::channel();
        lock(&desktop::ANSWERS).push((videos.clone(), waiting));
        let shown_at = || {
            lock(&desktop::DIALOGS)
                .iter()
                .filter(|s| s.as_deref() == Some(videos.as_path()))
                .count()
        };
        let (engine, root) = test_engine();
        // Asked from a folder that is not there: the dialog opens at the nearest folder that is.
        engine.send(Command::PickFolder {
            start: videos.join("gone").join("too"),
        });
        engine.send(Command::PickFolder { start: videos.clone() });
        engine.send(Command::CheckFolder {
            root: videos.clone(),
            name: String::new(),
        });
        // The coordinator goes on while the dialog is open.
        folder_of(&engine, &videos, "");
        wait_until("the dialog", || shown_at() == 1);
        assert!(engine.poll().is_none(), "no answer while the dialog is open");
        answer.send(Some(picked.clone())).unwrap();
        let Event::FolderPicked { path } = wait_for(&engine, 10.0, |e| matches!(e, Event::FolderPicked { .. })) else {
            unreachable!()
        };
        assert_eq!(path, Some(picked));
        assert_eq!(shown_at(), 1, "the second PickFolder showed no dialog");
        // Once it is closed, the next PickFolder shows one again (closed at once here, with no folder).
        engine.send(Command::PickFolder { start: videos.clone() });
        let Event::FolderPicked { path } = wait_for(&engine, 10.0, |e| matches!(e, Event::FolderPicked { .. })) else {
            unreachable!()
        };
        assert_eq!(path, None);
        assert_eq!(shown_at(), 2);
        assert_eq!(DIALOG_TITLE, "Choose the output folder");
        finish(engine, &root);
    }

    #[test]
    fn the_dialog_starts_at_the_nearest_folder_there() {
        let (_temp, videos) = output_folder("the_dialog_starts_at_the_nearest_folder_there");
        assert_eq!(dialog_start(&videos), Some(videos.clone()));
        assert_eq!(dialog_start(&videos.join("a").join("b")), Some(videos.clone()));
        let file = videos.join("a file.txt");
        fs::write(&file, b"mine").unwrap();
        assert_eq!(dialog_start(&file), Some(videos.clone()));
        assert_eq!(
            dialog_start(&with_trailing_separator(&videos)),
            Some(with_trailing_separator(&videos))
        );
        assert_eq!(dialog_start(Path::new("")), None);
        assert_eq!(dialog_start(&drive_root()), Some(drive_root()));
        if let Some(drive) = missing_drive() {
            assert_eq!(dialog_start(&drive.join("BALLFIGHT")), None);
        }
    }

    #[test]
    fn a_folder_is_shown_and_odd_ones_are_named() {
        let (_temp, videos) = output_folder("a_folder_is_shown_and_odd_ones_are_named");
        let folder = video(&videos, "flute_vs_trombone", &fight_toml(7, [1.0, 1.0]));
        let (engine, root) = test_engine();
        engine.send(Command::RevealFolder {
            path: with_trailing_separator(&folder),
        });
        wait_until("showing the folder", || {
            lock(&desktop::SHOWN)
                .iter()
                .any(|p| p.as_os_str() == folder.as_os_str())
        });
        let file = folder.join("cover.png");
        engine.send(Command::RevealFolder { path: file.clone() });
        assert_eq!(
            problem_text(&engine),
            format!("{} is a file, not a folder, so it was not shown.", file.display())
        );
        let gone = videos.join("gone ! é");
        engine.send(Command::RevealFolder { path: gone.clone() });
        assert_eq!(
            problem_text(&engine),
            format!(
                "The folder {} is not there any more, so it cannot be shown. Check that its drive is connected.",
                gone.display()
            )
        );
        engine.send(Command::RevealFolder { path: PathBuf::new() });
        assert_eq!(problem_text(&engine), "There is no folder to show.");
        if let Some(drive) = missing_drive() {
            engine.send(Command::RevealFolder { path: drive.clone() });
            assert!(problem_text(&engine).contains("is not there any more"));
        }
        let shown = lock(&desktop::SHOWN).clone();
        assert!(!shown.contains(&file) && !shown.contains(&gone));
        // A drive's root is shown as it is.
        assert_eq!(reveal_target(&drive_root()), Ok(drive_root()));
        finish(engine, &root);
    }

    /// A context without a coordinator, and the interface's end of its seam.
    fn bare_ctx() -> (Ctx, EngineHandle) {
        let (handle, side) = crate::seam::channel(Arc::new(|| {}));
        let config = super::super::EngineConfig {
            workers: 1,
            paths: crate::store::Paths::under(Path::new("unused")),
            clip: None,
            search_limit: None,
        };
        (Ctx::new(side.events.clone(), config), handle)
    }

    #[test]
    fn an_older_folder_answer_is_never_sent() {
        let (_temp, videos) = output_folder("an_older_folder_answer_is_never_sent");
        let (ctx, events) = bare_ctx();
        // A silenced answer is not sent; one that is not silenced is.
        let flag = AtomicBool::new(true);
        send_unless_stopped(&ctx, &flag, problem("late".to_string()));
        assert!(events.poll().is_none());
        flag.store(false, Ordering::SeqCst);
        send_unless_stopped(&ctx, &flag, problem("on time".to_string()));
        assert!(matches!(events.poll(), Some(Event::Problem { text }) if text == "on time"));
        // Each check or create silences the one before it, whichever kind it was.
        let mut state = State::default();
        let check = || Command::CheckFolder {
            root: videos.clone(),
            name: "flute_vs_trombone".to_string(),
        };
        handle(&mut state, &ctx, check());
        let first = state.folder.clone().expect("a flag for the check");
        handle(&mut state, &ctx, Command::CreateFolder { root: videos.clone() });
        let second = state.folder.clone().expect("a flag for the create");
        assert!(first.load(Ordering::SeqCst), "the check was silenced");
        assert!(!second.load(Ordering::SeqCst));
        handle(&mut state, &ctx, check());
        assert!(second.load(Ordering::SeqCst), "the create's answer was silenced");
        // The newest is answered.
        let state = folder_of(&events, &videos, "flute_vs_trombone");
        assert_eq!(state, FolderState::Ok { taken: false });
        // A create's problem is still sent when a newer check silences its answer.
        let mut state = State::default();
        let deep = videos.join("missing").join("BALLFIGHT");
        handle(&mut state, &ctx, Command::CreateFolder { root: deep });
        handle(&mut state, &ctx, check());
        assert!(problem_text(&events).starts_with("The folder "));
        ctx.shutdown();
    }

    #[test]
    fn covers_stop_at_the_newest_ones() {
        let (_temp, videos) = output_folder("covers_stop_at_the_newest_ones");
        let older = video(&videos, "flute_vs_trombone", &fight_toml(1, [1.0, 1.0]));
        let newer = video(&videos, "trombone_vs_flute", &fight_toml(2, [1.0, 1.0]));
        for (folder, secs) in [(&older, 1_794_000_000), (&newer, 1_794_100_000)] {
            let cover = fs::File::options().write(true).open(folder.join("cover.png")).unwrap();
            cover
                .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(secs))
                .unwrap();
        }
        assert_eq!(MAX_COVERS, 500);
        let (ctx, handle) = bare_ctx();
        let flag = AtomicBool::new(false);
        scan_library(&ctx, videos.clone(), &flag, 1);
        let Some(Event::Library { entries, .. }) = handle.poll() else {
            panic!("no Library");
        };
        assert_eq!(entries.len(), 2, "every video is listed");
        let Some(Event::LibraryCover { folder, .. }) = handle.poll() else {
            panic!("no cover");
        };
        assert_eq!(folder, newer);
        assert!(handle.poll().is_none(), "only the newest video's cover");
        // A scan silenced before it answers sends nothing at all.
        flag.store(true, Ordering::SeqCst);
        scan_library(&ctx, videos, &flag, MAX_COVERS);
        assert!(handle.poll().is_none());
    }

    #[cfg(windows)]
    #[test]
    fn an_open_never_reads_through_a_link() {
        let (temp, videos) = output_folder("an_open_never_reads_through_a_link");
        let outside = temp.0.join("outside");
        fs::create_dir(&outside).unwrap();
        let link = videos.join("linked.toml");
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&outside)
            .output()
            .unwrap();
        assert!(made.status.success(), "{made:?}");
        let (engine, root) = test_engine();
        engine.send(Command::OpenFight { path: link });
        assert_eq!(
            problem_text(&engine),
            "linked.toml cannot be opened: It is a link to somewhere else, not a fight file."
        );
        // Links to files need rights a plain account may not have; when this PC allows them, they are refused too.
        let real = outside.join("real.toml");
        fs::write(&real, fight_toml(7, [1.0, 1.0])).unwrap();
        let file_link = videos.join("file link.toml");
        match std::os::windows::fs::symlink_file(&real, &file_link) {
            Ok(()) => {
                engine.send(Command::OpenFight { path: file_link });
                assert!(problem_text(&engine).contains("It is a link to somewhere else"));
            }
            Err(err) => eprintln!("file links are not allowed here ({err}); that part was not run"),
        }
        finish(engine, &root);
    }
}

/// What an unfinished render leaves behind, and the folder dialog's parent. Every test works in a temp folder of its
/// own; nothing here touches the PC's own Videos folder, and no dialog is opened.
#[cfg(test)]
mod leftover_tests {
    #[cfg(windows)]
    use std::process::{Child, Command as Process, Stdio};

    use super::*;
    #[cfg(windows)]
    use crate::store::{TempRoot, files_in};

    /// The export's own partial video name for the pair.
    #[cfg(windows)]
    const PART: &str = "flute_vs_trombone.mp4.part";

    /// Two days after now: everything a test makes is "two days old" then.
    #[cfg(windows)]
    fn later() -> SystemTime {
        SystemTime::now() + Duration::from_secs(2 * 24 * 60 * 60)
    }

    /// The id of a program that has ended (started, waited for, and its handle let go).
    #[cfg(windows)]
    fn dead_pid() -> u32 {
        let mut child = Process::new("cmd")
            .args(["/C", "exit 0"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child.wait().unwrap();
        let pid = child.id();
        drop(child);
        assert!(!process_running(pid), "pid {pid} still runs");
        pid
    }

    /// A program that runs for up to two minutes; the test kills it.
    #[cfg(windows)]
    fn live_program() -> Child {
        let child = Process::new("cmd")
            .args(["/C", "ping -n 120 127.0.0.1"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        assert!(process_running(child.id()));
        child
    }

    /// The folder `name` in `root`, holding the given files.
    #[cfg(windows)]
    fn folder_with(root: &Path, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let folder = root.join(name);
        fs::create_dir(&folder).unwrap();
        for (file, bytes) in files {
            fs::write(folder.join(file), bytes).unwrap();
        }
        folder
    }

    /// An output folder inside a test's temp folder (with spaces, "!" and letters outside ASCII in its path).
    #[cfg(windows)]
    fn output(test: &str) -> (TempRoot, PathBuf) {
        let temp = TempRoot::new(test);
        let videos = temp.0.join("Vidéos ! BALLFIGHT");
        fs::create_dir(&videos).unwrap();
        (temp, videos)
    }

    /// Windows only: a junction at `link` to the folder `target` (no admin rights needed).
    #[cfg(windows)]
    fn junction(link: &Path, target: &Path) {
        let made = Process::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .unwrap();
        assert!(made.status.success(), "{made:?}");
    }

    /// What `cleared` says about `folder`.
    #[cfg(windows)]
    fn said(cleared: &Cleared, folder: &Path) -> String {
        cleared
            .left
            .iter()
            .find(|(f, _)| f == folder)
            .map(|(_, why)| why.clone())
            .unwrap_or_else(|| panic!("nothing said about {}: {cleared:?}", folder.display()))
    }

    #[cfg(windows)]
    #[test]
    fn old_leftovers_are_cleared_file_by_file() {
        let (_temp, videos) = output("old_leftovers_are_cleared_file_by_file");
        let dead = dead_pid();
        let own = std::process::id();
        folder_with(
            &videos,
            &format!(".tmp-flute_vs_trombone-{dead}"),
            &[("audio.wav", b"sound"), (PART, b"part"), ("encoding.txt", b"faster")],
        );
        folder_with(
            &videos,
            &format!(".tmp-flute_vs_trombone_frames0-90-{dead}-2"),
            &[("flute_vs_trombone_frames0-90.mp4.part", b"part")],
        );
        folder_with(&videos, &format!(".tmp-flute_vs_trombone_seed7-{dead}-3"), &[]);
        // This program's own, which no job of it is using.
        folder_with(&videos, &format!(".tmp-flute_vs_trombone-{own}"), &[(PART, b"part")]);
        // A video, a folder of the person's, and a file with a leftover's name: all stay.
        folder_with(
            &videos,
            "flute_vs_trombone",
            &[("cover.png", b"c"), ("flute_vs_trombone.toml", b"f")],
        );
        folder_with(&videos, ".keep", &[("mine.txt", b"mine")]);
        fs::write(videos.join(format!(".tmp-flute_vs_trombone-{dead}-4")), b"a file").unwrap();

        let cleared = clear_abandoned(&videos, ABANDONED_AFTER, later()).unwrap();
        assert_eq!(
            cleared,
            Cleared {
                removed: 4,
                left: Vec::new()
            }
        );
        assert_eq!(
            files_in(&videos),
            [
                ".keep".to_string(),
                format!(".tmp-flute_vs_trombone-{dead}-4"),
                "flute_vs_trombone".to_string()
            ]
        );
        assert_eq!(
            files_in(&videos.join("flute_vs_trombone")),
            ["cover.png", "flute_vs_trombone.toml"]
        );
        assert_eq!(fs::read(videos.join(".keep").join("mine.txt")).unwrap(), b"mine");
        // Nothing more to do the second time.
        assert_eq!(
            clear_abandoned(&videos, ABANDONED_AFTER, later()).unwrap(),
            Cleared::default()
        );
        // An output folder that cannot be listed is said in words.
        let missing = videos.join("missing");
        assert_eq!(
            clear_abandoned(&missing, ABANDONED_AFTER, later()).unwrap_err(),
            format!(
                "The output folder {} could not be read, so what unfinished renders left in it was not looked at.",
                missing.display()
            )
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_persons_own_file_keeps_the_folder() {
        let (_temp, videos) = output("a_persons_own_file_keeps_the_folder");
        let dead = dead_pid();
        let notes = folder_with(
            &videos,
            &format!(".tmp-flute_vs_trombone-{dead}"),
            &[("audio.wav", b"sound"), ("my notes.txt", b"mine")],
        );
        // The export's name in another letter case is not the export's file.
        let cased = folder_with(
            &videos,
            &format!(".tmp-flute_vs_trombone-{dead}-2"),
            &[("Audio.WAV", b"sound")],
        );
        // A folder inside.
        let inner = folder_with(&videos, &format!(".tmp-flute_vs_trombone-{dead}-3"), &[(PART, b"part")]);
        folder_with(&inner, "photos", &[("me.jpg", b"me")]);
        // A file meant for another video's name.
        let other = folder_with(
            &videos,
            &format!(".tmp-flute_vs_trombone_seed7-{dead}"),
            &[("tuba_vs_horn.mp4.part", b"p")],
        );

        let cleared = clear_abandoned(&videos, ABANDONED_AFTER, later()).unwrap();
        assert_eq!((cleared.removed, cleared.left.len()), (0, 4), "{cleared:?}");
        assert_eq!(
            said(&cleared, &notes),
            format!(
                "The hidden folder {} holds a file that BALLFIGHT Studio did not make (my notes.txt), so it was left \
                 as it is. Look inside it, then delete it yourself if you do not need what it holds.",
                notes.display()
            )
        );
        assert!(said(&cleared, &cased).contains("(Audio.WAV)"));
        assert!(said(&cleared, &inner).contains("holds a folder that BALLFIGHT Studio did not make (photos)"));
        assert!(said(&cleared, &other).contains("(tuba_vs_horn.mp4.part)"));
        // Nothing in them was touched, not even the export's own file beside the person's.
        assert_eq!(files_in(&notes), ["audio.wav", "my notes.txt"]);
        assert_eq!(fs::read(notes.join("my notes.txt")).unwrap(), b"mine");
        assert_eq!(files_in(&cased), ["Audio.WAV"]);
        assert_eq!(files_in(&inner), [PART, "photos"]);
        assert_eq!(fs::read(inner.join("photos").join("me.jpg")).unwrap(), b"me");
        assert_eq!(files_in(&other), ["tuba_vs_horn.mp4.part"]);
    }

    #[cfg(windows)]
    #[test]
    fn a_leftover_with_a_video_is_never_touched() {
        let (_temp, videos) = output("a_leftover_with_a_video_is_never_touched");
        let dead = dead_pid();
        let video = folder_with(
            &videos,
            &format!(".tmp-flute_vs_trombone-{dead}"),
            &[
                ("flute_vs_trombone.mp4", b"video"),
                ("audio.wav", b"sound"),
                ("cover.png", b"c"),
            ],
        );
        let upper = folder_with(
            &videos,
            &format!(".tmp-flute_vs_trombone-{dead}-2"),
            &[("OTHER.MP4", b"video")],
        );
        let fight = folder_with(
            &videos,
            &format!(".tmp-flute_vs_trombone-{dead}-3"),
            &[("flute_vs_trombone.toml", b"fight"), (PART, b"part")],
        );
        // A video and a person's file: the video is what is said.
        let both = folder_with(
            &videos,
            &format!(".tmp-flute_vs_trombone_seed7-{dead}"),
            &[("zz notes.txt", b"mine"), ("flute_vs_trombone.mp4", b"video")],
        );

        let cleared = clear_abandoned(&videos, ABANDONED_AFTER, later()).unwrap();
        assert_eq!((cleared.removed, cleared.left.len()), (0, 4), "{cleared:?}");
        assert_eq!(
            said(&cleared, &video),
            format!(
                "The hidden folder {} holds a finished video (flute_vs_trombone.mp4), so BALLFIGHT Studio left it as \
                 it is. Move the video somewhere else, then delete the folder yourself.",
                video.display()
            )
        );
        assert!(said(&cleared, &upper).contains("holds a finished video (OTHER.MP4)"));
        assert!(said(&cleared, &fight).contains("holds a fight file (flute_vs_trombone.toml)"));
        assert!(said(&cleared, &both).contains("holds a finished video (flute_vs_trombone.mp4)"));
        assert_eq!(files_in(&video), ["audio.wav", "cover.png", "flute_vs_trombone.mp4"]);
        assert_eq!(fs::read(video.join("flute_vs_trombone.mp4")).unwrap(), b"video");
        assert_eq!(files_in(&upper), ["OTHER.MP4"]);
        assert_eq!(files_in(&fight), [PART, "flute_vs_trombone.toml"]);
        assert_eq!(files_in(&both), ["flute_vs_trombone.mp4", "zz notes.txt"]);
    }

    #[cfg(windows)]
    #[test]
    fn links_are_never_followed() {
        let (temp, videos) = output("links_are_never_followed");
        let dead = dead_pid();
        // Outside the output folder, files with the export's own names.
        let outside = folder_with(&temp.0, "outside", &[("audio.wav", b"keep"), (PART, b"keep")]);
        // The leftover's name on a junction.
        let link = videos.join(format!(".tmp-flute_vs_trombone-{dead}"));
        junction(&link, &outside);
        // A junction inside a leftover.
        let holds = folder_with(
            &videos,
            &format!(".tmp-flute_vs_trombone-{dead}-2"),
            &[("audio.wav", b"sound")],
        );
        junction(&holds.join("inner"), &outside);
        // A file link with the export's name, when this account may make one.
        let file_link = folder_with(&videos, &format!(".tmp-flute_vs_trombone-{dead}-3"), &[]);
        let linked = std::os::windows::fs::symlink_file(outside.join("audio.wav"), file_link.join("audio.wav")).is_ok();

        let cleared = clear_abandoned(&videos, ABANDONED_AFTER, later()).unwrap();
        // Without a file link the third folder is empty, and an empty leftover is cleared.
        assert_eq!(cleared.removed, usize::from(!linked), "{cleared:?}");
        assert_eq!(
            said(&cleared, &link),
            format!(
                "The hidden folder {} is a link to somewhere else, so BALLFIGHT Studio did not touch it or what it \
                 leads to. Delete it yourself if you do not need it.",
                link.display()
            )
        );
        assert!(said(&cleared, &holds).contains("holds a link to somewhere else (inner)"));
        if linked {
            assert!(said(&cleared, &file_link).contains("holds a link to somewhere else (audio.wav)"));
            assert_eq!(files_in(&file_link), ["audio.wav"]);
        } else {
            eprintln!("file links are not allowed here; that part was not run");
        }
        // The outside folder and its files stay, and so do the links and the leftover's own file.
        assert_eq!(files_in(&outside), ["audio.wav", PART]);
        assert_eq!(fs::read(outside.join("audio.wav")).unwrap(), b"keep");
        assert_eq!(fs::read(outside.join(PART)).unwrap(), b"keep");
        assert!(fs::symlink_metadata(&link).is_ok());
        assert_eq!(files_in(&holds), ["audio.wav", "inner"]);
    }

    #[cfg(windows)]
    #[test]
    fn a_running_render_is_never_touched() {
        let (_temp, videos) = output("a_running_render_is_never_touched");
        let mut program = live_program();
        let pid = program.id();
        let busy = folder_with(&videos, &format!(".tmp-flute_vs_trombone-{pid}"), &[(PART, b"part")]);
        let cleared = clear_abandoned(&videos, ABANDONED_AFTER, later()).unwrap();
        let _ = program.kill();
        let _ = program.wait();
        drop(program);
        assert_eq!(
            cleared,
            Cleared::default(),
            "a running program's folder is passed over without a word"
        );
        assert_eq!(files_in(&busy), [PART]);
        // The program's id was the only reason: once it has ended, the folder goes.
        assert_eq!(clear_abandoned(&videos, ABANDONED_AFTER, later()).unwrap().removed, 1);
        assert_eq!(files_in(&videos), Vec::<String>::new());
    }

    #[cfg(windows)]
    #[test]
    fn a_young_leftover_waits() {
        let (_temp, videos) = output("a_young_leftover_waits");
        let dead = dead_pid();
        let young = folder_with(&videos, &format!(".tmp-flute_vs_trombone-{dead}"), &[(PART, b"part")]);
        // Made just now: too young today, and too young for three days two days from now.
        assert_eq!(
            clear_abandoned(&videos, ABANDONED_AFTER, SystemTime::now()).unwrap(),
            Cleared::default()
        );
        let three_days = Duration::from_secs(3 * 24 * 60 * 60);
        assert_eq!(
            clear_abandoned(&videos, three_days, later()).unwrap(),
            Cleared::default()
        );
        // The newest file counts, not the folder: a file dated three days ahead keeps its folder young.
        let newer = folder_with(
            &videos,
            &format!(".tmp-flute_vs_trombone-{dead}-2"),
            &[("audio.wav", b"s")],
        );
        let file = fs::OpenOptions::new()
            .write(true)
            .open(newer.join("audio.wav"))
            .unwrap();
        file.set_modified(SystemTime::now() + three_days).unwrap();
        drop(file);
        // A person's file in a young folder is not mentioned yet either.
        let mine = folder_with(
            &videos,
            &format!(".tmp-flute_vs_trombone-{dead}-3"),
            &[("mine.txt", b"m")],
        );
        assert_eq!(
            clear_abandoned(&videos, ABANDONED_AFTER, SystemTime::now()).unwrap(),
            Cleared::default()
        );
        let cleared = clear_abandoned(&videos, ABANDONED_AFTER, later()).unwrap();
        assert_eq!((cleared.removed, cleared.left.len()), (1, 1), "{cleared:?}");
        assert_eq!(cleared.left[0].0, mine);
        assert!(!young.exists());
        assert_eq!(files_in(&newer), ["audio.wav"]);
    }

    #[cfg(windows)]
    #[test]
    fn names_that_only_look_alike_are_left() {
        let (_temp, videos) = output("names_that_only_look_alike_are_left");
        let dead = dead_pid();
        let names = [
            format!(".tmp-x-{dead}"),
            ".tmp-flute_vs_trombone".to_string(),
            format!(".tmp-flute_vs_trombone-0{dead}"),
            format!(".tmp-flute_vs_trombone-{dead}a"),
            format!(".tmp-flute_vs_trombone-{dead}.old"),
            format!(".tmp-flute_vs_trombone-{dead} (2)"),
            format!(".tmp-flute_vs_trombone-{dead}-01"),
            format!(".TMP-flute_vs_trombone-{dead}"),
            format!("tmp-flute_vs_trombone-{dead}"),
            format!("x.tmp-flute_vs_trombone-{dead}"),
            format!(".tmp--{dead}"),
            "flute_vs_trombone_kept-1".to_string(),
        ];
        for name in &names {
            folder_with(&videos, name, &[("audio.wav", b"sound"), (PART, b"part")]);
        }
        let cleared = clear_abandoned(&videos, ABANDONED_AFTER, later()).unwrap();
        assert_eq!(cleared, Cleared::default());
        let mut sorted = names.to_vec();
        sorted.sort();
        assert_eq!(files_in(&videos), sorted);
        for name in &names {
            assert_eq!(files_in(&videos.join(name)), ["audio.wav", PART], "{name}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn the_person_is_told_once() {
        let (_temp, videos) = output("the_person_is_told_once");
        let dead = dead_pid();
        folder_with(&videos, &format!(".tmp-flute_vs_trombone-{dead}"), &[(PART, b"part")]);
        folder_with(&videos, &format!(".tmp-flute_vs_trombone-{dead}-2"), &[(PART, b"part")]);
        let kept = folder_with(
            &videos,
            &format!(".tmp-flute_vs_trombone-{dead}-3"),
            &[("mine.txt", b"m")],
        );
        let why = format!(
            "The hidden folder {} holds a file that BALLFIGHT Studio did not make (mine.txt), so it was left as it \
             is. Look inside it, then delete it yourself if you do not need what it holds.",
            kept.display()
        );
        let told = Mutex::new(HashSet::new());
        // Nothing could be sent (a newer scan took over): the folder left is not counted as told.
        let mut heard = Vec::new();
        clear_before_scan(&videos, later(), &told, &mut |text| {
            heard.push(text);
            false
        });
        assert_eq!(heard, [removed_text(2), why.clone()]);
        assert!(lock(&told).is_empty());
        // Sent: told once, and never again in this run.
        let mut heard = Vec::new();
        clear_before_scan(&videos, later(), &told, &mut |text| {
            heard.push(text);
            true
        });
        assert_eq!(heard, [why]);
        let mut heard = Vec::new();
        clear_before_scan(&videos, later(), &told, &mut |text| {
            heard.push(text);
            true
        });
        assert_eq!(heard, Vec::<String>::new());
        // Nothing is said about an output folder that cannot be listed (the Library says that itself).
        clear_before_scan(&videos.join("missing"), later(), &told, &mut |text| {
            panic!("{text}");
        });
        assert_eq!(files_in(&kept), ["mine.txt"]);
    }

    #[test]
    fn the_words_are_whole_sentences() {
        assert_eq!(
            removed_text(1),
            "BALLFIGHT Studio removed 1 hidden folder that an unfinished render left in the output folder more than \
             a day ago. It held no finished video."
        );
        assert_eq!(
            removed_text(3),
            "BALLFIGHT Studio removed 3 hidden folders that unfinished renders left in the output folder more than a \
             day ago. They held no finished video."
        );
    }

    #[test]
    fn the_dialog_has_a_parent_once_set() {
        // What `set_parent_window` keeps is what every dialog is built from (a mark stands in for the window,
        // which a test does not have).
        let template = rfd::FileDialog::new().set_can_create_directories(true);
        let start = std::env::temp_dir();
        let built = format!("{:?}", dialog_builder(Some(&template), Some(&start)));
        assert!(built.contains("can_create_directories: Some(true)"), "{built}");
        assert!(built.contains(&format!("title: Some({DIALOG_TITLE:?})")), "{built}");
        assert!(
            built.contains(&format!("starting_directory: Some({start:?})")),
            "{built}"
        );
        let plain = format!("{:?}", dialog_builder(None, None));
        assert!(
            plain.contains("parent: None") && plain.contains("starting_directory: None"),
            "{plain}"
        );
        // A creation context with no window (as a test's) leaves the dialog without a parent, and the first call
        // is the one kept.
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        set_parent_window(&cc);
        set_parent_window(&cc);
        let kept = format!("{:?}", PARENT.get().expect("set"));
        assert!(kept.contains("parent: None") && kept.contains("title: None"), "{kept}");
    }
}
