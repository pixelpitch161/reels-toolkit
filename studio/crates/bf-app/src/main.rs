//! BALLFIGHT Studio.
//!
//! - `--fixtures` runs the whole interface on the sample engine (`seam::fake`, at a real engine's pace) with real
//!   sound out, its files in a temp folder of its own (removed when the window has closed), and the status bar's
//!   right side reading "Sample data, not a real fight".
//! - Without it the app runs on the real engine: its settings, queue and shortlists in the PC's own app folders
//!   (`Paths::system`), the settings and the queue read once before the first frame (with their notices), the
//!   engine started with the settings' worker threads, the PC's sound device, and the writer that saves. Closing
//!   the window stops the engine first (a render in progress is cancelled and its ffmpeg ended), waiting a few
//!   seconds at most.
//! - `--data-dir <folder>` keeps the settings, the queue, the cache and the logs in `<folder>` instead (created if
//!   missing; a folder that cannot be used stops the app with one plain sentence and exit code 2).
//! - `--smoke` opens the window, paints 30 frames, closes it and exits with code 0; it keeps its files in a temp
//!   folder of its own and has no sound. It runs the real app that way, or with `--fixtures` the sample one.
//! - `--version` prints "BALLFIGHT Studio 0.6.0 (a842e11, 12 Nov 2026)" (`version::version_text`) and exits 0. It
//!   opens no window. The release build has no console of its own (`windows_subsystem = "windows"`), so the line
//!   goes to the output the starting program gave it (a file or a pipe, as `check_staged` and `cmd`'s `> file`
//!   give), or, when there is none or it is unusable (`GetFileType` unknown), to the console of the program that
//!   started it (`AttachConsole`), or nowhere. PowerShell 5.1's `>` gives a windowed program no file.
//! - `--finish-update <pid>` is how an update is put in place (`update::swap`): the new version, started from the
//!   program folder's `.next`, waits for program `<pid>` to end, swaps itself in and starts the app again. It opens
//!   no window and exits 0 when the swap worked, else 1. It refuses (exit 1, nothing read further, written or
//!   started) unless this exe sits in a folder named `.next` whose parent is a real installed copy (see `finish_dir`).
//! - `--test-fault-report` (in no help text) makes the interface thread fault on purpose after the first painted
//!   frame, so the release checklist can see a real fault report and its message box from the shipped exe.
//! - `--preview-bench [--seed N] [--seconds 20] [--threads T]` opens no window: it makes the starter fight's take
//!   (FLUTE against TROMBONE at handicap 2.0 / 0.4, the first seed that fits the story unless `--seed` names one)
//!   with the real engine, plays it from FIGHT! with no sound for `--seconds` on `--threads` drawing threads (the
//!   settings' default workers unless given), scrubs to 50 frames, and prints the measurements to stdout, one
//!   `name value` line each (`preview::bench`). Its engine's files go in a temp folder, removed at the end. A bad
//!   value stops it with one plain sentence and exit code 2.
//!
//! # The start-up, step by step
//!
//! Each step's decision is a function of what it found ([`mode`], [`data_folders`], [`second_copy`],
//! [`looks_at_updates`], [`after_update_check`], [`graphics_notice`], [`finish_dir`]), tested on its own; `run` only
//! carries them out. A step that stops the start says why in plain words: in a message box (`report::tell`) where
//! there is no window yet, and on stderr. Nothing on this path panics on purpose (except `--test-fault-report`).
//!
//! 1. **The command line** ([`mode`]). `--version` prints and exits 0 before anything else. A word the app does
//!    not know is told, exit 2.
//! 2. **`--finish-update <pid>`**: only for a staged update of a real install ([`finish_dir`]); the swap
//!    (`update::swap::finish_at`), exit 0 or 1; its failure goes to the PC's own `logs\finish.txt` (the started app
//!    rewrites `start.txt`). **`--preview-bench`** runs here too (no window, no hook, no claim, no update check).
//! 3. **The fault hook** (`report::install_hook`), on the interface thread, writing into this run's `logs` folder:
//!    the PC's own, the `--data-dir` folder's, or a `--smoke` / `--fixtures` run's temp folder.
//! 4. **The data folders.** A `--data-dir` folder that cannot be used stops the start (exit 2). Then `config`,
//!    `cache` and `logs` are each checked (`startup::check_data_folder`): `config` that cannot be written stops the
//!    start (exit 2), since settings and the queue could not be kept; `cache` or `logs` that cannot be written is a
//!    notice in the window, and the app runs. A stop here comes before the claim, so it writes no `start.txt` (an
//!    open copy's record survives): the words go to the person only.
//! 5. **One copy at a time** (`startup::claim` on this run's `config` folder). A second copy brings the first
//!    window forward and exits 0; when there is no window to bring forward it says `ALREADY_OPEN`. A second copy
//!    writes no `start.txt` (it would replace the open copy's). The claim is held to the end of `run`. A smoke or
//!    fixtures run without `--data-dir` has a temp `config` of its own, so it never meets the real app's claim; with
//!    `--data-dir` it shares that folder's claim, as the real app does.
//! 6. **The unfinished update**, only for the installed copy's own start ([`looks_at_updates`]): `swap::at_start`
//!    gives nothing, a notice, `Resume` (hand the swap to `.next`'s exe, exit 0), `Busy` (`UPDATE_BUSY`, exit 0) or
//!    `Restart` (the claim let go, the earlier version started, exit 0).
//! 7. **The window**, with the app's icon. When it cannot be opened, the error goes to `start.txt` and the person is
//!    told `startup::no_window_text`, exit 1.
//! 8. **In the window's creation**: the graphics card in words (`startup::window_graphics`; software drawing is the
//!    notice `SOFTWARE_DRAWING`), then the app; then, off the interface thread, ffmpeg (`startup::check_ffmpeg`) and
//!    the output folder (`startup::check_output_folder`; a missing one at a first start is only a line, as the
//!    Render and Library screens already say what to do), each a line in `start.txt` when it answers.
//!
//! **What counts as installed** (`update::installed`): this exe is named `ballfight-studio.exe` and its folder holds
//! a readable `MANIFEST.txt` and `update.toml`. Only then does a start look at `.next`, `.previous` or the swap lock.
//! A build in `target\` has neither, so a developer's build and every test behave as before. (`startup`'s
//! `expected_own_folder`, `MANIFEST.txt` or an `ffmpeg` folder beside the exe, decides only how a missing ffmpeg is
//! worded.)
//!
//! `start.txt` (in the run's `logs`) records the last start: the version, whether this copy is installed, the data
//! folders, the update check, the graphics card, ffmpeg and the output folder, and any error. The line naming the
//! graphics adapter also still goes to stderr.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

use bf_app::app::{self, EngineKind, Started};
use bf_app::model::Settings;
use bf_app::preview::audio::{NO_DEVICE, NoAudio, open_audio};
use bf_app::preview::bench;
use bf_app::startup::{self, ALREADY_OPEN, Folder, Found, Graphics, Instance};
use bf_app::store::Paths;
use bf_app::store::settings::{default_settings, save_settings};
use bf_app::update::swap::{
    self, AtStart, FINISH_FLAG, FINISH_WORDS, NOT_STAGED, RESTART_FAILED, RESUME_FAILED, UPDATE_BUSY,
};
use bf_app::{icon, report, update, version};

/// The window's title, and the title of every message box the start-up shows.
const TITLE: &str = "BALLFIGHT Studio";
/// How many painted frames `--smoke` waits for before it closes the window.
const SMOKE_FRAMES: u32 = 30;
/// The status bar's right side in `--fixtures`.
const SAMPLE_NOTE: &str = "Sample data, not a real fight";
/// The option that makes the interface thread fault on purpose (the release checklist's look at a real report).
const FAULT_FLAG: &str = "--test-fault-report";
/// The fault `--test-fault-report` makes.
const FAULT_MESSAGE: &str = "A fault made on purpose by --test-fault-report, to check that fault reports work.";

/// What the command line asks for, for a start with a window.
#[derive(Debug, Default, PartialEq, Eq)]
struct Args {
    smoke: bool,
    fixtures: bool,
    data_dir: Option<PathBuf>,
    test_fault: bool,
}

/// What the program is asked to do.
#[derive(Debug, PartialEq)]
enum Mode {
    /// `--version`: print the version and exit.
    Version,
    /// `--finish-update <pid>`: put the staged update in place.
    FinishUpdate(u32),
    /// `--preview-bench`: measure the preview.
    Bench(BenchArgs),
    /// Open the window.
    Run(Args),
}

/// Reads the whole command line (without the program's own name): `--version` anywhere wins; `--finish-update` must
/// be alone with its number; `--preview-bench` goes with its own options ([`parse_bench`]); everything else is a
/// start with a window ([`parse`]). `cores` bounds `--threads`.
fn mode(args: Vec<OsString>, cores: usize) -> Result<Mode, String> {
    if args.iter().any(|a| a == "--version") {
        return Ok(Mode::Version);
    }
    if args.iter().any(|a| a == FINISH_FLAG) {
        return match args.as_slice() {
            [flag, pid] if flag == FINISH_FLAG => pid
                .to_str()
                .and_then(|p| p.parse::<u32>().ok())
                .filter(|&p| p > 0)
                .map(Mode::FinishUpdate)
                .ok_or_else(|| FINISH_WORDS.to_string()),
            _ => Err(FINISH_WORDS.to_string()),
        };
    }
    if args.iter().any(|a| a == "--preview-bench") {
        return parse_bench(args, cores).map(Mode::Bench);
    }
    parse(args).map(Mode::Run)
}

/// Reads the command line of a start with a window (without the program's own name). A word it does not know, or
/// `--data-dir` with no folder after it, is an error in one plain sentence.
fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Args, String> {
    let mut parsed = Args::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--smoke") => parsed.smoke = true,
            Some("--fixtures") => parsed.fixtures = true,
            Some(FAULT_FLAG) => parsed.test_fault = true,
            Some("--data-dir") => match args.next() {
                // An option in its place ("--data-dir --fixtures") is not a folder.
                Some(dir) if !dir.is_empty() && !dir.to_string_lossy().starts_with("--") => {
                    parsed.data_dir = Some(PathBuf::from(dir));
                }
                _ => return Err("--data-dir needs a folder after it.".to_string()),
            },
            _ => {
                return Err(format!(
                    "BALLFIGHT Studio does not know the option {}.",
                    arg.to_string_lossy()
                ));
            }
        }
    }
    Ok(parsed)
}

/// What `--preview-bench` is asked for.
#[derive(Debug, PartialEq)]
struct BenchArgs {
    seed: Option<u64>,
    seconds: f32,
    threads: Option<usize>,
}

/// How long `--preview-bench` plays unless `--seconds` says.
const BENCH_SECONDS: f32 = 20.0;
/// The longest `--seconds` may be: the whole video.
const BENCH_MOST_SECONDS: f32 = 61.0;

/// Reads a command line that holds `--preview-bench`: only `--seed`, `--seconds` and `--threads` may go with it.
/// `cores` bounds `--threads`. A bad value is an error in one plain sentence.
fn parse_bench(args: impl IntoIterator<Item = OsString>, cores: usize) -> Result<BenchArgs, String> {
    let mut parsed = BenchArgs {
        seed: None,
        seconds: BENCH_SECONDS,
        threads: None,
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = |words: &str| {
            args.next()
                .and_then(|v| v.to_str().map(str::to_string))
                .ok_or_else(|| words.to_string())
        };
        match arg.to_str() {
            Some("--preview-bench") => {}
            Some("--seed") => {
                let words = "--seed needs a whole number after it, such as --seed 12.";
                parsed.seed = Some(value(words)?.parse::<u64>().map_err(|_| words.to_string())?);
            }
            Some("--seconds") => {
                let words =
                    format!("--seconds needs a number of seconds above 0 and up to {BENCH_MOST_SECONDS} after it.");
                let s = value(&words)?.parse::<f32>().map_err(|_| words.clone())?;
                if !(s > 0.0 && s <= BENCH_MOST_SECONDS) {
                    return Err(words);
                }
                parsed.seconds = s;
            }
            Some("--threads") => {
                let words = format!("--threads needs a whole number from 1 to {cores} after it.");
                let t = value(&words)?.parse::<usize>().map_err(|_| words.clone())?;
                if !(1..=cores).contains(&t) {
                    return Err(words);
                }
                parsed.threads = Some(t);
            }
            _ => {
                return Err(format!(
                    "--preview-bench goes only with --seed, --seconds and --threads, not with {}.",
                    arg.to_string_lossy()
                ));
            }
        }
    }
    Ok(parsed)
}

// ---------------------------------------------------------------------------------------------------------------
// The start-up's decisions: each takes what a step found and says what to do.

/// What a start-up step decided.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// Go on to the next step.
    GoOn,
    /// Go on, and show this notice in the window.
    Notice(String),
    /// Stop with this exit code, telling the person `tell` first when there is something to say.
    Stop { code: u8, tell: Option<String> },
    /// An unfinished update is handed to `.next`'s exe to finish (`swap::resume`); then exit 0.
    Resume,
    /// An update was taken back while this program (the new version) runs: let the claim go, start the earlier
    /// version (`swap::start_installed`), exit 0.
    Restart,
}

/// What the checks of the `config`, `cache` and `logs` folders mean (in that order): `config` that is not ready
/// stops the start with its words; `cache` or `logs` that is not ready is a notice (its words), and the app runs.
fn data_folders(found: [&Found<Folder>; 3]) -> Result<Vec<String>, String> {
    let [config, cache, logs] = found;
    if config.kind != Folder::Ready {
        return Err(config.words.clone());
    }
    Ok([cache, logs]
        .into_iter()
        .filter(|f| f.kind != Folder::Ready)
        .map(|f| f.words.clone())
        .collect())
}

/// What a second copy for the same `config` folder tells the person before it steps aside (always exit 0): nothing
/// when the first copy's window was brought forward, else `ALREADY_OPEN` (the first is still closing, or hung).
fn second_copy(brought_forward: bool) -> Option<&'static str> {
    (!brought_forward).then_some(ALREADY_OPEN)
}

/// Whether this start looks at an unfinished update (`swap::at_start`, which may move files in the program folder
/// and takes the swap lock): only an installed copy's real start, never a smoke or fixtures run.
fn looks_at_updates(installed: bool, args: &Args) -> bool {
    installed && !args.smoke && !args.fixtures
}

/// What the start does about `swap::at_start`'s answer.
fn after_update_check(at: AtStart) -> Outcome {
    match at {
        AtStart::Nothing => Outcome::GoOn,
        AtStart::Notice(text) => Outcome::Notice(text),
        AtStart::Resume => Outcome::Resume,
        AtStart::Busy => Outcome::Stop {
            code: 0,
            tell: Some(UPDATE_BUSY.to_string()),
        },
        AtStart::Restart => Outcome::Restart,
    }
}

/// The notice for the window's graphics: software drawing is `SOFTWARE_DRAWING`; a graphics card says nothing.
fn graphics_notice(found: &Found<Graphics>) -> Option<String> {
    (found.kind == Graphics::Software).then(|| found.words.clone())
}

/// The program folder `--finish-update` swaps into, for the exe at `exe`: the parent of the exe's folder, when that
/// folder is named `.next` and the parent is a real installed copy (`update::installed_at` on its
/// `ballfight-studio.exe`: a `MANIFEST.txt` of ours that reads, and an `update.toml` that `read_config` accepts).
/// The swap keeps a manifest of ours in the program folder at every step and never moves `update.toml`, so a
/// stopped swap's folder still passes. Anything else is refused before anything is read further or written, so a
/// build in `target\`, or a folder made to look like a staged update, touches nothing.
fn finish_dir(exe: &Path) -> Result<PathBuf, String> {
    let refuse = || NOT_STAGED.to_string();
    let staged = exe.parent().ok_or_else(refuse)?;
    let named_next = staged
        .file_name()
        .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(".next"));
    let dir = staged.parent().ok_or_else(refuse)?;
    if named_next && update::installed_at(&dir.join(bf_pack::package::EXE_NAME)).is_some() {
        Ok(dir.to_path_buf())
    } else {
        Err(refuse())
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The record of this start (`logs\start.txt`).

/// The lines of `start.txt`, written again whenever a line is added once the log is open.
struct StartLog {
    logs: Option<PathBuf>,
    inner: Mutex<LogLines>,
}

#[derive(Default)]
struct LogLines {
    lines: Vec<String>,
    /// Whether lines are written as they come (from the claim on; a second copy never writes).
    open: bool,
    /// Whether the last write worked.
    written: bool,
}

impl StartLog {
    fn new(logs: Option<PathBuf>) -> StartLog {
        StartLog {
            logs,
            inner: Mutex::new(LogLines::default()),
        }
    }

    /// Adds `line`; writes `start.txt` when the log is open.
    fn add(&self, line: impl Into<String>) {
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        inner.lines.push(line.into());
        if inner.open {
            self.write(&mut inner);
        }
    }

    /// From now on every line is written, starting with those so far.
    fn open(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        inner.open = true;
        self.write(&mut inner);
    }

    fn write(&self, inner: &mut LogLines) {
        inner.written = match &self.logs {
            Some(logs) => report::write_start(logs, &inner.lines).is_ok(),
            None => false,
        };
    }

    /// `start.txt`'s path, when it was written.
    fn file(&self) -> Option<PathBuf> {
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let logs = self.logs.as_ref().filter(|_| inner.written)?;
        Some(logs.join("start.txt"))
    }
}

/// Stops the start with exit `code`: `tell` (when there is one) goes to `start.txt` when the log is open, to
/// stderr, and to the person in a message box.
fn stop(log: &StartLog, code: u8, tell: Option<String>) -> ExitCode {
    if let Some(text) = tell {
        log.add(text.clone());
        err_line(&text);
        report::tell(TITLE, &text);
    }
    ExitCode::from(code)
}

/// `text` and a line end on stderr, ignoring a failed write (never a panic, unlike `eprintln!`).
fn err_line(text: &str) {
    use std::io::Write as _;
    let _ = writeln!(std::io::stderr().lock(), "{text}");
}

/// `text` and a line end on stdout, ignoring a failed write (never a panic, unlike `println!`).
fn out_line(text: &str) {
    use std::io::Write as _;
    let _ = writeln!(std::io::stdout().lock(), "{text}");
}

// ---------------------------------------------------------------------------------------------------------------
// Where `--version` writes.

/// Writes `text` and a line end where the person or program that started this one can read it: the output it was
/// given (a file, a pipe or a console); else, in the release build, which has no console of its own, the console
/// of the program that started it; else nowhere. A failed write is ignored.
fn say(text: &str) {
    #[cfg(windows)]
    if let Some(mut console) = console::parent() {
        use std::io::Write as _;
        let _ = writeln!(console, "{text}");
        return;
    }
    out_line(text);
}

/// The starting program's console, for a program that was given no output of its own.
#[cfg(windows)]
mod console {
    use std::ffi::c_void;
    use std::fs::{File, OpenOptions};

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetStdHandle(which: u32) -> *mut c_void;
        fn GetFileType(file: *mut c_void) -> u32;
        fn AttachConsole(process: u32) -> i32;
    }

    /// `FILE_TYPE_UNKNOWN`: what `GetFileType` gives for a handle that is not a file, a pipe or a console (or no
    /// usable handle at all).
    const FILE_TYPE_UNKNOWN: u32 = 0;

    /// `STD_OUTPUT_HANDLE`, (DWORD)-11.
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    /// `ATTACH_PARENT_PROCESS`, (DWORD)-1.
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    /// `INVALID_HANDLE_VALUE`.
    const INVALID: isize = -1;

    /// `None` when this program has a usable output already (a console, a file or a pipe: stdout reaches it), or
    /// when the starting program has no console either; else that console, opened for writing. A handle that is
    /// there but is none of those (`FILE_TYPE_UNKNOWN`) counts as no output.
    pub fn parent() -> Option<File> {
        // SAFETY: GetStdHandle takes a constant and only reads the process's standard handles.
        let out = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
        // SAFETY: GetFileType only asks about the handle, which is checked for null and INVALID first.
        if !out.is_null() && out as isize != INVALID && unsafe { GetFileType(out) } != FILE_TYPE_UNKNOWN {
            return None;
        }
        // SAFETY: AttachConsole takes a constant; it fails harmlessly when there is no such console.
        if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } == 0 {
            return None;
        }
        OpenOptions::new().write(true).open("CONOUT$").ok()
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The runs.

/// `--preview-bench`: makes the starter fight's take, and the next fitting seed's, with the real engine; plays the
/// first on a new player and then the second on a player that has just played the first (`warm_` lines), each with
/// no window and no sound, scrubs each, and prints what it measured, one line per field. Writes nothing outside a
/// temp folder.
fn preview_bench(args: &BenchArgs) -> ExitCode {
    let cores = bench::cores();
    let threads = args.threads.unwrap_or_else(bench::workers);
    out_line(&format!("pc_threads {cores}"));
    out_line(&format!("workers {}", bench::workers()));
    // The first take, and a second (the next fitting seed) for a player that has already played one.
    let takes = bench::starter_take(args.seed).and_then(|a| {
        let b = bench::starter_take(Some(bench::fitting_seed_after(a.seed)?))?;
        Ok((a, b))
    });
    let (a, b) = match takes {
        Ok(takes) => takes,
        Err(problem) => {
            err_line(&format!("The bench could not make its take: {problem}"));
            return ExitCode::FAILURE;
        }
    };
    out_line(&format!("seed {}", a.seed));
    out_line(&format!("to_picture_s {:.3}", a.to_picture.as_secs_f32()));
    out_line(&format!("to_sound_s {:.2}", a.to_sound.as_secs_f32()));
    out_line(&format!("seconds {}", args.seconds));
    let first = bench::run(a.frames.clone(), a.start, args.seconds, threads);
    for line in first.lines() {
        out_line(&line);
    }
    // The app's player lives as long as the app: from the second seed on, a take is loaded on a scheduler that
    // already knows its draw times.
    out_line(&format!("warm_seed {}", b.seed));
    let warm = bench::run_after(Some((a.frames, a.start)), b.frames, b.start, args.seconds, threads);
    for line in warm.lines() {
        out_line(&format!("warm_{line}"));
    }
    ExitCode::SUCCESS
}

/// The record `--finish-update` leaves in the logs folder when its swap failed (the started app rewrites
/// `start.txt`, so the swap's words get a file of their own).
const FINISH_LOG: &str = "finish.txt";

/// `--finish-update <pid>` for the exe at `exe`, leaving its notice and its record in `logs` (`main` passes the PC's
/// own logs folder, where the next start looks): refuses, writing nothing anywhere and starting nothing, unless the
/// exe is a staged update of a real install ([`finish_dir`]); else the swap. A failed swap's words go to
/// `logs\finish.txt` (the swap has already left its notice for the next start, and started the version in place).
fn finish_update(exe: &Path, pid: u32, logs: Option<&Path>) -> ExitCode {
    let dir = match finish_dir(exe) {
        Ok(dir) => dir,
        Err(problem) => {
            err_line(&problem);
            return ExitCode::FAILURE;
        }
    };
    match swap::finish_at(&dir, pid, logs) {
        Ok(()) => ExitCode::SUCCESS,
        Err(problem) => {
            err_line(&problem);
            if let Some(logs) = logs {
                let text = format!("{}\r\n{problem}\r\n", version::version_text());
                let _ = bf_app::store::write_atomic(&logs.join(FINISH_LOG), text.as_bytes());
            }
            ExitCode::FAILURE
        }
    }
}

/// Makes sure the app can keep its files in `dir`: creates it when missing and checks that a file can be written
/// there. Says why not in one plain sentence.
fn usable(dir: &Path) -> Result<(), String> {
    let refuse = || {
        format!(
            "BALLFIGHT Studio cannot keep its files in {}, so it did not start.",
            dir.display()
        )
    };
    std::fs::create_dir_all(dir).map_err(|_| refuse())?;
    if !dir.is_dir() {
        return Err(refuse());
    }
    let probe = dir.join(format!(".bf-write-check-{}", std::process::id()));
    std::fs::write(&probe, b"").map_err(|_| refuse())?;
    let _ = std::fs::remove_file(&probe);
    Ok(())
}

/// Writes the settings a smoke run starts with into its temp folder `root`: the output folder `root\Videos` and the
/// default workers. Best effort: if it cannot be written the run starts on the defaults, as any first start does.
fn smoke_settings(root: &Path, paths: &Paths) {
    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let settings = Settings {
        output_folder: root.join("Videos"),
        ..default_settings(cores)
    };
    if std::fs::create_dir_all(&paths.config).is_ok() {
        let _ = save_settings(&paths.settings(), &settings);
    }
}

/// Runs `app`; with `close_after`, closes its window after that many painted frames (`--smoke`); with `fault`,
/// faults on the interface thread once the first frame was painted (`--test-fault-report`).
struct Wrapped {
    app: Box<dyn eframe::App>,
    painted: u32,
    close_after: Option<u32>,
    fault: bool,
}

impl eframe::App for Wrapped {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.logic(ctx, frame);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if self.fault && self.painted >= 1 {
            panic!("{FAULT_MESSAGE}");
        }
        self.app.ui(ui, frame);
        self.painted += 1;
        if self.close_after.is_some_and(|n| self.painted >= n) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        } else if self.close_after.is_some() || self.fault {
            ui.ctx().request_repaint();
        }
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        self.app.clear_color(visuals)
    }

    fn on_exit(&mut self) {
        self.app.on_exit();
    }
}

/// A start with a window: steps 3 to 8 of the module notes. A smoke or fixtures run without `--data-dir` keeps its
/// files in a temp folder of its own, removed at the end.
fn run(args: Args) -> ExitCode {
    // A smoke or fixtures run never touches the owner's own settings folder: its files go in a folder of its own
    // under the system temp folder, removed when the window has closed. `--data-dir` names the folder instead.
    let temp_root = (args.data_dir.is_none() && (args.smoke || args.fixtures)).then(|| {
        let kind = if args.smoke { "smoke" } else { "fixtures" };
        std::env::temp_dir().join(format!("bf-{kind}-{}", std::process::id()))
    });
    let paths = match (&args.data_dir, &temp_root) {
        (Some(dir), _) | (None, Some(dir)) => Some(Paths::under(dir)),
        (None, None) => Paths::system(),
    };
    let look: Arc<Mutex<Option<JoinHandle<()>>>> = Arc::new(Mutex::new(None));
    let code = start(args, paths, temp_root.as_deref(), &look);
    if let Some(root) = temp_root {
        // The late checks write into the temp folder: they end first. The wait is bounded by their own waits,
        // `startup::FFMPEG_WAIT` and then `startup::FOLDER_WAIT` (10 s + 5 s), one after the other.
        if let Some(handle) = look.lock().unwrap_or_else(PoisonError::into_inner).take() {
            let _ = handle.join();
        }
        let _ = std::fs::remove_dir_all(root);
    }
    code
}

/// Steps 3 to 8 (see the module notes), for `args` keeping its files in `paths`. `temp_root` is a smoke or fixtures
/// run's own folder. The late checks' thread is put in `look`.
fn start(
    args: Args,
    paths: Option<Paths>,
    temp_root: Option<&Path>,
    look: &Arc<Mutex<Option<JoinHandle<()>>>>,
) -> ExitCode {
    // 3. The fault hook.
    report::install_hook(paths.as_ref().map(|p| p.logs.clone()));
    let log = Arc::new(StartLog::new(paths.as_ref().map(|p| p.logs.clone())));
    log.add(version::version_text());
    let installed = update::installed();
    log.add(match &installed {
        Some(inst) => format!("This is the copy installed in {}.", inst.dir.display()),
        None => "This copy is not an installed one, so it does not look at updates.".to_string(),
    });

    // 4. The data folders. A stop here comes before the claim, so it writes no start.txt (an open copy's record
    // must survive): its words go to the person only.
    if let Some(dir) = &args.data_dir
        && let Err(problem) = usable(dir)
    {
        return stop(&log, 2, Some(problem));
    }
    let mut notices = Vec::new();
    if let Some(p) = &paths {
        let found = [&p.config, &p.cache, &p.logs].map(|dir| startup::check_data_folder(dir));
        for f in &found {
            log.add(f.words.clone());
        }
        match data_folders([&found[0], &found[1], &found[2]]) {
            Ok(more) => notices.extend(more),
            Err(problem) => return stop(&log, 2, Some(problem)),
        }
    }
    // A smoke run's videos would go to its own temp folder too: it never looks at the owner's Videos folder.
    if let (true, Some(root), Some(paths)) = (args.smoke, temp_root, &paths) {
        smoke_settings(root, paths);
    }

    // 5. One copy at a time. The claim is held until this function returns.
    let mut claim = match paths.as_ref().map(|p| startup::claim(&p.config)) {
        Some(Instance::Second) => {
            err_line("Another copy of BALLFIGHT Studio is open with these settings, so this one stepped aside.");
            // Not `stop`: a second copy never writes the open copy's start.txt.
            if let Some(text) = second_copy(startup::bring_forward(TITLE)) {
                err_line(text);
                report::tell(TITLE, text);
            }
            return ExitCode::SUCCESS;
        }
        Some(Instance::First(claim)) => Some(claim),
        None => None,
    };
    log.open();

    // 6. The unfinished update (the installed copy only).
    if let Some(inst) = installed.as_ref().filter(|_| looks_at_updates(true, &args)) {
        // The logs folder `swap::finish` leaves its notices in.
        let logs = Paths::system().map(|p| p.logs);
        match after_update_check(swap::at_start(inst, logs.as_deref())) {
            Outcome::GoOn => log.add("No update is waiting to be finished."),
            Outcome::Notice(text) => {
                log.add(text.clone());
                notices.push(text);
            }
            Outcome::Stop { code, tell } => return stop(&log, code, tell),
            Outcome::Resume => {
                log.add("An unfinished update was handed to the new version to finish.");
                return match swap::resume(inst) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(problem) => {
                        log.add(problem);
                        stop(&log, 1, Some(RESUME_FAILED.to_string()))
                    }
                };
            }
            Outcome::Restart => {
                log.add("An update was taken back, so the earlier version is started in this one's place.");
                // Let the claim go first, so the earlier version does not take this program for an open copy.
                drop(claim.take());
                return match swap::start_installed(inst) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(problem) => {
                        log.add(problem);
                        stop(&log, 1, Some(RESTART_FAILED.to_string()))
                    }
                };
            }
        }
    }

    // 7. The window.
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(TITLE)
            .with_icon(icon::window_icon())
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    // DirectX 12 alone where the PC has a DirectX 12 card, so overlay and capture hooks that ride on Vulkan and
    // OpenGL stay out of the Studio (see `app::prefer_dx12`).
    app::prefer_dx12(&mut options.wgpu_options);
    let Args {
        smoke,
        fixtures,
        test_fault,
        ..
    } = args;
    let (window_log, window_look) = (Arc::clone(&log), Arc::clone(look));
    let result = eframe::run_native(
        TITLE,
        options,
        Box::new(move |cc| {
            // 8. In the window's creation.
            let mut started = Started { notices, adapter: None };
            if let Some(render) = &cc.wgpu_render_state {
                let info = render.adapter.get_info();
                eprintln!(
                    "BALLFIGHT Studio draws with {} ({:?}, {:?})",
                    info.name, info.backend, info.device_type
                );
                let found = startup::window_graphics(&info);
                window_log.add(found.words.clone());
                started.notices.extend(graphics_notice(&found));
                started.adapter = Some(found.words);
            }
            // The theme is installed, and the settings and the queue are read, here, before the first frame; the
            // engine is made here too, where the window's context exists for its waker.
            let kind = if fixtures { EngineKind::Sample } else { EngineKind::Real };
            let reports = paths.as_ref().map(|p| p.logs.clone());
            let mut app = if smoke {
                app::start_with(cc, paths, || Box::new(NoAudio::new(NO_DEVICE)), kind, started)
            } else {
                app::start_with(cc, paths, open_audio, kind, started)
            };
            if fixtures {
                app.status_note = Some(SAMPLE_NOTE.to_string());
            }
            let real = !(smoke || fixtures);
            app.state
                .set_upkeep(bf_app::seam::upkeep::for_this_start(&cc.egui_ctx, real, reports));
            // ffmpeg and the output folder, off the interface thread: each a line in start.txt when it answers.
            let output = app.state.vm.settings.output_folder.clone();
            let late_log = Arc::clone(&window_log);
            let handle = std::thread::Builder::new()
                .name("startup: late checks".to_string())
                .spawn(move || {
                    late_log.add(startup::check_ffmpeg().words);
                    late_log.add(startup::check_output_folder(&output).words);
                })
                .ok();
            *window_look.lock().unwrap_or_else(PoisonError::into_inner) = handle;
            let app: Box<dyn eframe::App> = Box::new(Wrapped {
                app: Box::new(app),
                painted: 0,
                close_after: smoke.then_some(SMOKE_FRAMES),
                fault: test_fault,
            });
            Ok(app)
        }),
    );
    let code = match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            log.add(format!("The window could not be opened: {err}"));
            stop(&log, 1, Some(startup::no_window_text(log.file().as_deref())))
        }
    };
    drop(claim);
    code
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    match mode(args, bench::cores()) {
        Ok(Mode::Version) => {
            say(&version::version_text());
            ExitCode::SUCCESS
        }
        Ok(Mode::FinishUpdate(pid)) => match std::env::current_exe() {
            Ok(exe) => finish_update(&exe, pid, Paths::system().map(|p| p.logs).as_deref()),
            Err(_) => {
                err_line(NOT_STAGED);
                ExitCode::FAILURE
            }
        },
        Ok(Mode::Bench(args)) => preview_bench(&args),
        Ok(Mode::Run(args)) => run(args),
        Err(problem) => {
            err_line(&problem);
            report::tell(TITLE, &problem);
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(line: &[&str]) -> Vec<OsString> {
        line.iter().map(OsString::from).collect()
    }

    #[test]
    fn the_command_line() {
        assert_eq!(parse(words(&[])), Ok(Args::default()));
        assert_eq!(
            parse(words(&["--fixtures", "--smoke"])),
            Ok(Args {
                smoke: true,
                fixtures: true,
                ..Args::default()
            })
        );
        assert_eq!(
            parse(words(&["--data-dir", "D:\\Studio files", "--fixtures"])),
            Ok(Args {
                fixtures: true,
                data_dir: Some(PathBuf::from("D:\\Studio files")),
                ..Args::default()
            })
        );
        assert_eq!(
            parse(words(&["--test-fault-report", "--data-dir", "X"])),
            Ok(Args {
                data_dir: Some(PathBuf::from("X")),
                test_fault: true,
                ..Args::default()
            })
        );
        assert_eq!(
            parse(words(&["--data-dir"])),
            Err("--data-dir needs a folder after it.".to_string())
        );
        assert_eq!(
            parse(words(&["--data-dir", "--fixtures"])),
            Err("--data-dir needs a folder after it.".to_string())
        );
        assert_eq!(
            parse(words(&["--data-dir", ""])),
            Err("--data-dir needs a folder after it.".to_string())
        );
        assert_eq!(
            parse(words(&["--fixture"])),
            Err("BALLFIGHT Studio does not know the option --fixture.".to_string())
        );
    }

    #[test]
    fn the_modes() {
        let m = |line: &[&str]| mode(words(line), 16);
        assert_eq!(m(&["--version"]), Ok(Mode::Version));
        // --version wins over everything else, even words the app does not know.
        assert_eq!(m(&["--smoke", "--version"]), Ok(Mode::Version));
        assert_eq!(m(&["--nonsense", "--version"]), Ok(Mode::Version));
        assert_eq!(m(&["--finish-update", "1234"]), Ok(Mode::FinishUpdate(1234)));
        let finish = Err(FINISH_WORDS.to_string());
        for bad in [
            &["--finish-update"][..],
            &["--finish-update", "0"],
            &["--finish-update", "-3"],
            &["--finish-update", "twelve"],
            &["--finish-update", "12", "--smoke"],
            &["--smoke", "--finish-update", "12"],
            &["1234", "--finish-update"],
        ] {
            assert_eq!(m(bad), finish, "{bad:?}");
        }
        assert_eq!(
            m(&["--preview-bench", "--seconds", "5"]),
            Ok(Mode::Bench(BenchArgs {
                seed: None,
                seconds: 5.0,
                threads: None
            }))
        );
        assert_eq!(
            m(&["--preview-bench", "--smoke"]),
            Err("--preview-bench goes only with --seed, --seconds and --threads, not with --smoke.".to_string())
        );
        assert_eq!(
            m(&["--smoke", "--data-dir", "X"]),
            Ok(Mode::Run(Args {
                smoke: true,
                data_dir: Some(PathBuf::from("X")),
                ..Args::default()
            }))
        );
        assert_eq!(m(&[]), Ok(Mode::Run(Args::default())));
        assert_eq!(
            m(&["--Version"]),
            Err("BALLFIGHT Studio does not know the option --Version.".to_string())
        );
    }

    #[test]
    fn the_bench_command_line() {
        let bench = |line: &[&str]| parse_bench(words(line), 16);
        assert_eq!(
            bench(&["--preview-bench"]),
            Ok(BenchArgs {
                seed: None,
                seconds: 20.0,
                threads: None
            })
        );
        assert_eq!(
            bench(&["--seed", "255", "--preview-bench", "--seconds", "61", "--threads", "16"]),
            Ok(BenchArgs {
                seed: Some(255),
                seconds: 61.0,
                threads: Some(16)
            })
        );
        assert_eq!(
            bench(&["--preview-bench", "--seconds", "2.5"]).map(|a| a.seconds),
            Ok(2.5)
        );
        let seed = Err("--seed needs a whole number after it, such as --seed 12.".to_string());
        assert_eq!(bench(&["--preview-bench", "--seed"]), seed);
        assert_eq!(bench(&["--preview-bench", "--seed", "-3"]), seed);
        assert_eq!(bench(&["--preview-bench", "--seed", "ten"]), seed);
        let seconds = Err("--seconds needs a number of seconds above 0 and up to 61 after it.".to_string());
        for bad in ["0", "-1", "61.5", "NaN", "inf", "soon"] {
            assert_eq!(bench(&["--preview-bench", "--seconds", bad]), seconds, "{bad}");
        }
        let threads = Err("--threads needs a whole number from 1 to 16 after it.".to_string());
        for bad in ["0", "17", "1.5", "all"] {
            assert_eq!(bench(&["--preview-bench", "--threads", bad]), threads, "{bad}");
        }
        assert_eq!(
            bench(&["--preview-bench", "--fixtures"]),
            Err("--preview-bench goes only with --seed, --seconds and --threads, not with --fixtures.".to_string())
        );
    }

    fn found(kind: Folder, words: &str) -> Found<Folder> {
        Found {
            kind,
            words: words.to_string(),
        }
    }

    #[test]
    fn the_data_folders_decide() {
        let ready = found(Folder::Ready, "ready");
        assert_eq!(data_folders([&ready, &ready, &ready]), Ok(Vec::new()));
        for kind in [
            Folder::NotWritable,
            Folder::NotAFolder,
            Folder::NoAnswer,
            Folder::Missing,
        ] {
            let bad = found(kind, "Settings cannot be kept.");
            // The settings folder stops the start, whatever the others say.
            assert_eq!(
                data_folders([&bad, &ready, &ready]),
                Err("Settings cannot be kept.".to_string()),
                "{kind:?}"
            );
            assert_eq!(
                data_folders([&bad, &bad, &bad]),
                Err("Settings cannot be kept.".to_string())
            );
        }
        let cache = found(Folder::NotWritable, "The cache cannot be written.");
        let logs = found(Folder::NoAnswer, "The logs did not answer.");
        assert_eq!(
            data_folders([&ready, &cache, &ready]),
            Ok(vec!["The cache cannot be written.".to_string()])
        );
        assert_eq!(
            data_folders([&ready, &ready, &logs]),
            Ok(vec!["The logs did not answer.".to_string()])
        );
        assert_eq!(
            data_folders([&ready, &cache, &logs]),
            Ok(vec![
                "The cache cannot be written.".to_string(),
                "The logs did not answer.".to_string()
            ])
        );
    }

    #[test]
    fn a_second_copy_steps_aside() {
        assert_eq!(second_copy(true), None);
        assert_eq!(second_copy(false), Some(ALREADY_OPEN));
    }

    #[test]
    fn only_the_installed_copys_own_start_looks_at_updates() {
        let run = Args::default();
        let with_dir = Args {
            data_dir: Some(PathBuf::from("X")),
            ..Args::default()
        };
        let fault = Args {
            test_fault: true,
            ..Args::default()
        };
        let smoke = Args {
            smoke: true,
            ..Args::default()
        };
        let fixtures = Args {
            fixtures: true,
            ..Args::default()
        };
        for args in [&run, &with_dir, &fault] {
            assert!(looks_at_updates(true, args), "{args:?}");
        }
        for args in [&smoke, &fixtures] {
            assert!(!looks_at_updates(true, args), "{args:?}");
        }
        for args in [&run, &with_dir, &fault, &smoke, &fixtures] {
            assert!(!looks_at_updates(false, args), "{args:?}");
        }
    }

    #[test]
    fn each_update_answer_has_its_step() {
        assert_eq!(after_update_check(AtStart::Nothing), Outcome::GoOn);
        assert_eq!(
            after_update_check(AtStart::Notice("The update to version 0.6.1 is finished.".to_string())),
            Outcome::Notice("The update to version 0.6.1 is finished.".to_string())
        );
        assert_eq!(after_update_check(AtStart::Resume), Outcome::Resume);
        assert_eq!(after_update_check(AtStart::Restart), Outcome::Restart);
        assert_eq!(
            after_update_check(AtStart::Busy),
            Outcome::Stop {
                code: 0,
                tell: Some(UPDATE_BUSY.to_string())
            }
        );
    }

    #[test]
    fn software_drawing_is_a_notice() {
        let said = |kind| Found {
            kind,
            words: "words".to_string(),
        };
        assert_eq!(graphics_notice(&said(Graphics::Software)), Some("words".to_string()));
        for kind in [
            Graphics::Card,
            Graphics::NoCard,
            Graphics::NoAnswer,
            Graphics::NotWindows,
        ] {
            assert_eq!(graphics_notice(&said(kind)), None, "{kind:?}");
        }
    }

    /// A temp folder of the test's own, removed at the end.
    struct Temp(PathBuf);

    impl Temp {
        fn new(name: &str) -> Temp {
            let dir = std::env::temp_dir().join(format!("bf-app main {name} {}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Temp(dir)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Makes `program` a real installed copy as Task 11 leaves one (its exe, a manifest of ours, `update.toml`).
    fn install_at(program: &Path) {
        use bf_pack::package::{EXE_NAME, MANIFEST_NAME, Manifest};
        std::fs::create_dir_all(program).unwrap();
        let exe = program.join(EXE_NAME);
        std::fs::write(&exe, b"pretend").unwrap();
        let m = Manifest {
            version: "0.6.0".into(),
            files: vec![(EXE_NAME.into(), 7, bf_pack::sha::sha256_file(&exe).unwrap())],
        };
        std::fs::write(program.join(MANIFEST_NAME), m.text()).unwrap();
        let config = bf_pack::config::UpdateConfig {
            source: program.join("data").join("source"),
            remote: "https://github.com/someone/repo.git".into(),
            tag_prefix: "studio-v".into(),
            build_dir: program.join("data").join("build"),
            downloads: program.join("data").join("downloads"),
        };
        bf_pack::config::write_config(&program.join(bf_pack::config::CONFIG_NAME), &config).unwrap();
    }

    /// Every file and folder under `dir`, as paths relative to it, sorted.
    fn everything(dir: &Path) -> Vec<PathBuf> {
        fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let p = entry.path();
                out.push(p.strip_prefix(root).unwrap().to_path_buf());
                if p.is_dir() {
                    walk(root, &p, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(dir, dir, &mut out);
        out.sort();
        out
    }

    #[test]
    fn only_a_staged_update_of_a_real_install_finishes() {
        let temp = Temp::new("finish");
        let program = temp.0.join("BALLFIGHT Studio");
        let next = program.join(".next");
        std::fs::create_dir_all(&next).unwrap();
        let exe = next.join("ballfight-studio.exe");
        // Nothing beside .next: not an installed copy's.
        assert_eq!(finish_dir(&exe), Err(NOT_STAGED.to_string()));
        // An empty update.toml with no manifest is a folder made to look like one: refused.
        std::fs::write(program.join("update.toml"), b"").unwrap();
        assert_eq!(finish_dir(&exe), Err(NOT_STAGED.to_string()));
        // A real install (as `launch_finish` and `Resume` find it: the manifest of ours is always in place).
        install_at(&program);
        assert_eq!(finish_dir(&exe), Ok(program.clone()));
        assert_eq!(
            finish_dir(&program.join(".NEXT").join("ballfight-studio.exe")),
            Ok(program.clone())
        );
        // The exe in the program folder itself, or in another folder, is not a staged update.
        assert_eq!(
            finish_dir(&program.join("ballfight-studio.exe")),
            Err(NOT_STAGED.to_string())
        );
        let other = program.join("next");
        std::fs::create_dir_all(&other).unwrap();
        assert_eq!(
            finish_dir(&other.join("ballfight-studio.exe")),
            Err(NOT_STAGED.to_string())
        );
        // A build in target\ never is.
        let built = std::env::current_exe().unwrap();
        assert_eq!(finish_dir(&built), Err(NOT_STAGED.to_string()));
        assert_eq!(
            finish_dir(Path::new("ballfight-studio.exe")),
            Err(NOT_STAGED.to_string())
        );
    }

    #[test]
    fn a_made_up_staged_update_writes_nothing_and_starts_nothing() {
        let temp = Temp::new("finish made up");
        let program = temp.0.join("X");
        let next = program.join(".next");
        std::fs::create_dir_all(&next).unwrap();
        std::fs::write(program.join("update.toml"), b"").unwrap();
        let exe = next.join("ballfight-studio.exe");
        std::fs::write(&exe, b"pretend").unwrap();
        let logs = temp.0.join("logs");
        let before = everything(&temp.0);
        // The pid of this test program: were the swap to run, it would wait for it.
        let code = finish_update(&exe, std::process::id(), Some(&logs));
        assert_eq!(code, ExitCode::FAILURE);
        assert!(!logs.exists(), "no logs folder made, no notice, no record");
        assert_eq!(everything(&temp.0), before, "nothing anywhere changed");
    }

    #[test]
    fn the_start_log_is_written_once_open() {
        let temp = Temp::new("start log");
        let logs = temp.0.join("logs");
        let log = StartLog::new(Some(logs.clone()));
        log.add("one");
        log.add("two");
        assert!(!logs.join("start.txt").exists(), "nothing is written before the claim");
        assert_eq!(log.file(), None);
        // A line added before the claim is not written: the open copy's record survives.
        std::fs::create_dir_all(&logs).unwrap();
        std::fs::write(logs.join("start.txt"), "the open copy's record\n").unwrap();
        // What `stop` adds to the log before the claim (the exe test `a_data_folder_stop_writes_no_start_record`
        // covers the whole stop; `stop` itself would show a message box here).
        log.add("Settings cannot be kept.");
        assert_eq!(
            std::fs::read_to_string(logs.join("start.txt")).unwrap(),
            "the open copy's record\n"
        );
        log.open();
        assert_eq!(
            std::fs::read_to_string(logs.join("start.txt")).unwrap(),
            "one\ntwo\nSettings cannot be kept.\n"
        );
        log.add("three");
        assert_eq!(
            std::fs::read_to_string(logs.join("start.txt")).unwrap(),
            "one\ntwo\nSettings cannot be kept.\nthree\n"
        );
        assert_eq!(log.file(), Some(logs.join("start.txt")));

        // No logs folder: nothing written, no file named.
        let none = StartLog::new(None);
        none.open();
        none.add("x");
        assert_eq!(none.file(), None);
        // A logs folder that cannot be made (a file in its place): no file named, no fault.
        std::fs::write(temp.0.join("a file"), b"").unwrap();
        let blocked = StartLog::new(Some(temp.0.join("a file").join("logs")));
        blocked.open();
        blocked.add("x");
        assert_eq!(blocked.file(), None);
    }

    #[test]
    fn the_fault_message_reads_plainly() {
        // The texts a person reads are in `update::swap` and checked by `tests/ui/wording.rs`; this one goes only
        // into a fault report.
        assert!(FAULT_MESSAGE.starts_with("A fault"));
        assert!(FAULT_MESSAGE.ends_with('.'));
    }

    #[test]
    fn a_smoke_run_keeps_its_videos_in_its_own_folder() {
        let root = std::env::temp_dir().join(format!("bf-app smoke settings {}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let paths = Paths::under(&root);
        smoke_settings(&root, &paths);
        let (settings, notice) = bf_app::store::settings::load_settings(&paths.settings(), 8);
        assert_eq!(notice, None);
        assert_eq!(settings.output_folder, root.join("Videos"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_data_folder_is_made_or_refused_in_words() {
        let root = std::env::temp_dir().join(format!("bf-app data-dir {}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("made").join("for the studio");
        assert_eq!(usable(&dir), Ok(()));
        assert!(dir.is_dir(), "a missing folder is created");
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            0,
            "the write check leaves nothing behind"
        );
        // A file where the folder should be cannot be used.
        let file = root.join("a file");
        std::fs::write(&file, b"not a folder").unwrap();
        let problem = usable(&file).expect_err("a file is not a folder");
        assert_eq!(
            problem,
            format!(
                "BALLFIGHT Studio cannot keep its files in {}, so it did not start.",
                file.display()
            )
        );
        assert_eq!(problem.matches(". ").count(), 0, "one sentence");
        let _ = std::fs::remove_dir_all(&root);
    }

    // -----------------------------------------------------------------------------------------------------------
    // The built exe, run by name after `cargo build --release -p bf-app` (or a debug build for a debug test run):
    // each test runs `target\<profile>\ballfight-studio.exe` with a `--data-dir` in a temp folder of its own, and
    // with `BF_APP_REPORT_CHILD` set, so `report::tell` writes to stderr and no message box waits for a click. Every
    // run is waited for at most a fixed time, and its output is read as it comes, so a chatty child never blocks.

    use std::process::{Child, Output, Stdio};
    use std::time::{Duration, Instant};

    /// The built exe beside this test program's folder (`target\<profile>\deps\..`).
    fn built_exe() -> PathBuf {
        let me = std::env::current_exe().unwrap();
        let exe = me.parent().and_then(Path::parent).unwrap().join("ballfight-studio.exe");
        assert!(
            exe.is_file(),
            "build the app first ({} is missing): cargo build -p bf-app, with --release for a release test run",
            exe.display()
        );
        exe
    }

    /// The exe with `args`, quiet (no console window), its output piped, and message boxes kept off the screen.
    fn the_exe(args: &[&str], no_box: &Path) -> std::process::Command {
        let args: Vec<&std::ffi::OsStr> = args.iter().map(std::ffi::OsStr::new).collect();
        let mut cmd = bf_pack::run::quiet(&built_exe(), &args);
        cmd.env("BF_APP_REPORT_CHILD", no_box);
        cmd
    }

    /// Writes settings for a data folder `root` whose output folder is `root\Videos`, so a run never looks at the
    /// owner's own Videos folder.
    fn data_dir(root: &Path) -> PathBuf {
        let dir = root.join("data");
        smoke_settings(root, &Paths::under(&dir));
        dir
    }

    /// Reads `pipe` to its end on a thread of its own.
    fn drain(pipe: Option<impl std::io::Read + Send + 'static>) -> std::thread::JoinHandle<Vec<u8>> {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut bytes);
            }
            bytes
        })
    }

    /// Waits at most `limit` for `child`, reading its output meanwhile; ends it and fails when it runs longer.
    fn wait_within(mut child: Child, limit: Duration, what: &str) -> Output {
        let (out, err) = (drain(child.stdout.take()), drain(child.stderr.take()));
        let began = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if began.elapsed() > limit {
                let _ = child.kill();
                let _ = child.wait();
                let stderr = err.join().unwrap_or_default();
                panic!(
                    "{what} ran longer than {limit:?}; stderr: {}",
                    String::from_utf8_lossy(&stderr)
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        Output {
            status,
            stdout: out.join().unwrap_or_default(),
            stderr: err.join().unwrap_or_default(),
        }
    }

    /// Runs the exe with `args` to its end, at most `limit`.
    fn run_exe(args: &[&str], no_box: &Path, limit: Duration) -> Output {
        wait_within(the_exe(args, no_box).spawn().unwrap(), limit, &args.join(" "))
    }

    #[test]
    #[ignore = "runs the built exe: build it first, then run by name"]
    fn version_prints_and_exits() {
        let temp = Temp::new("version");
        let out = run_exe(&["--version"], &temp.0, Duration::from_secs(2));
        assert_eq!(out.status.code(), Some(0));
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim_end(), version::version_text());
        // Into a file, as `> file` gives it.
        let file = temp.0.join("version.txt");
        let child = the_exe(&["--version"], &temp.0)
            .stdout(Stdio::from(std::fs::File::create(&file).unwrap()))
            .spawn()
            .unwrap();
        let out = wait_within(child, Duration::from_secs(2), "--version into a file");
        assert_eq!(out.status.code(), Some(0));
        assert_eq!(
            std::fs::read_to_string(&file).unwrap().trim_end(),
            version::version_text()
        );
        // It opens nothing and writes nothing.
        assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 1);
    }

    #[test]
    #[ignore = "runs the built exe: build it first, then run by name"]
    fn a_finish_outside_an_install_is_refused() {
        let temp = Temp::new("finish refused");
        let out = run_exe(&["--finish-update", "1234"], &temp.0, Duration::from_secs(5));
        assert_eq!(out.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&out.stderr).contains(NOT_STAGED));
        let out = run_exe(&["--finish-update"], &temp.0, Duration::from_secs(5));
        assert_eq!(out.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&out.stderr).contains(FINISH_WORDS));
    }

    #[test]
    #[ignore = "runs the built exe: build it first, then run by name"]
    fn a_data_folder_stop_writes_no_start_record() {
        let temp = Temp::new("config stop");
        let dir = temp.0.join("data");
        std::fs::create_dir_all(&dir).unwrap();
        // A file where the settings folder goes: the start stops before the claim.
        std::fs::write(dir.join("config"), b"in the way").unwrap();
        let out = run_exe(
            &["--data-dir", &dir.to_string_lossy()],
            &temp.0,
            Duration::from_secs(20),
        );
        assert_eq!(out.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("a file of that name is in the way"), "{stderr}");
        assert!(!dir.join("logs").join("start.txt").exists(), "no start record");
    }

    #[test]
    #[ignore = "runs the built exe and opens windows: build it first, then run by name"]
    fn a_second_copy_steps_aside_from_the_exe() {
        let temp = Temp::new("second copy");
        let dir = data_dir(&temp.0);
        let dir_text = dir.to_string_lossy().to_string();
        let mut first = the_exe(&["--smoke", "--data-dir", &dir_text], &temp.0).spawn().unwrap();
        let (first_out, first_err) = (drain(first.stdout.take()), drain(first.stderr.take()));
        // Wait until the first has its claim (its start.txt is written from the claim on).
        let began = Instant::now();
        while !dir.join("logs").join("start.txt").exists() {
            if began.elapsed() > Duration::from_secs(20) {
                let _ = first.kill();
                panic!("the first copy never started");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let out = run_exe(&["--smoke", "--data-dir", &dir_text], &temp.0, Duration::from_secs(2));
        assert_eq!(out.status.code(), Some(0));
        assert!(String::from_utf8_lossy(&out.stderr).contains("stepped aside"));
        let out = wait_within(first, Duration::from_secs(60), "the first copy");
        assert_eq!(out.status.code(), Some(0));
        let (_, _) = (first_out.join(), first_err.join());
        let start = std::fs::read_to_string(dir.join("logs").join("start.txt")).unwrap();
        assert!(start.contains("The window draws with"), "{start}");

        // Another data folder: both run.
        let other = data_dir(&temp.0.join("other"));
        let one = the_exe(&["--smoke", "--data-dir", &dir_text], &temp.0).spawn().unwrap();
        let two = the_exe(&["--smoke", "--data-dir", &other.to_string_lossy()], &temp.0)
            .spawn()
            .unwrap();
        for (child, d) in [(one, &dir), (two, &other)] {
            let out = wait_within(child, Duration::from_secs(60), "a copy");
            assert_eq!(out.status.code(), Some(0));
            assert!(!String::from_utf8_lossy(&out.stderr).contains("stepped aside"));
            let start = std::fs::read_to_string(d.join("logs").join("start.txt")).unwrap();
            assert!(start.contains("The window draws with"), "{start}");
        }
    }

    #[test]
    #[ignore = "runs the built exe with no graphics: build it first, then run by name"]
    fn no_window_is_told() {
        let temp = Temp::new("no window");
        let dir = data_dir(&temp.0);
        // Metal names no adapter on Windows, so the window cannot be made.
        let child = the_exe(&["--data-dir", &dir.to_string_lossy()], &temp.0)
            .env("WGPU_BACKEND", "metal")
            .spawn()
            .unwrap();
        let out = wait_within(child, Duration::from_secs(60), "a start with no graphics");
        assert_eq!(out.status.code(), Some(1));
        let start = std::fs::read_to_string(dir.join("logs").join("start.txt")).unwrap();
        assert!(start.contains("The window could not be opened"), "{start}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains(startup::NO_GRAPHICS), "{stderr}");
        assert!(stderr.contains("The details were saved in"), "{stderr}");
    }

    #[test]
    #[ignore = "runs the built exe and opens a window: build it first, then run by name"]
    fn a_fault_report_from_the_exe() {
        let temp = Temp::new("fault");
        let dir = data_dir(&temp.0);
        let out = run_exe(
            &["--test-fault-report", "--data-dir", &dir.to_string_lossy()],
            &temp.0,
            Duration::from_secs(60),
        );
        assert_ne!(out.status.code(), Some(0));
        let reports: Vec<_> = std::fs::read_dir(dir.join("logs"))
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with("crash-"))
            .collect();
        assert_eq!(reports.len(), 1, "one report");
        let report = std::fs::read_to_string(reports[0].path()).unwrap();
        assert!(report.contains(FAULT_MESSAGE), "{report}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("A report was saved as"), "{stderr}");
    }
}
