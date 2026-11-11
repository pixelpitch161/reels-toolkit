//! Files and folders: `ScanLibrary`, `OpenFight`, `OpenFightText`, `CheckFolder`, `CreateFolder`, `PickFolder`
//! and `RevealFolder`.
//!
//! Every command is answered on a thread of its own, so a slow or unplugged drive never holds up the coordinator:
//!
//! - `ScanLibrary`: `Library` (the videos under the root, with each fight file read as the engine reads it; a root
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
//!   is ignored (the open one answers).
//! - `RevealFolder`: shows the folder in the PC's file browser without waiting for it; only a folder that is there
//!   is shown (never a file, which the browser would open with its program), else a `Problem` in words.
//!
//! Nothing here deletes or overwrites anything. The two calls that touch the desktop (the dialog and starting the
//! file browser) are the functions in `desktop`; tests replace them, so no test opens a window.
//!
//! One gap is accepted: an older open that passed its last check a moment before a newer open finishes could still
//! send its answer after the newer one. The newer open sets the older one's flag before it starts, so the older
//! would have to stall between its check and its send for longer than the whole newer open takes.

use std::fmt::Display;
use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bf_core::fightfile::FightFile;

use super::convert::{draft_of, plain, title_of};
use super::{Ctx, problem};
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
    let started = ctx.spawn(
        "engine: library",
        "reading the Library",
        move |ctx| scan_library(&ctx, root, &mine, MAX_COVERS),
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

    /// The PC's folder dialog, opening at `start`; waits until the person closes it.
    pub fn pick_folder(start: Option<&Path>) -> Option<PathBuf> {
        let dialog = rfd::FileDialog::new().set_title(super::DIALOG_TITLE);
        match start {
            Some(start) => dialog.set_directory(start),
            None => dialog,
        }
        .pick_folder()
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
