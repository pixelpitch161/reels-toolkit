//! `ballfight`: the BALLFIGHT Studio command line, every step without a window.
//!
//! Every command takes a fight file. `--rec <file.bfrec>` uses a recording instead of fighting the seed again,
//! `--workers N` sets the threads of the commands that fight or draw many (search, balance, check, render; the
//! default is all cores but one) and `--json` prints one JSON value on stdout instead of text. Progress goes to
//! stderr.
//!
//! Exit code 0 on success; 1 on bad input, a refused fight, a failed check or a stopped run, with the reason as one
//! line on stderr (`ballfight: ...`). Nothing a person can type makes the program panic; were it to happen, the
//! panic is reported as that one line too. Output files are never replaced: a command refuses a path that exists.

use std::fmt::Display;
use std::io::{self, Write};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use bf_content::roster;
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::roster::FighterCfg;
use clap::{CommandFactory, Parser, Subcommand};
use serde_json::Value;

mod cmd {
    pub mod balance;
    pub mod check;
    pub mod frame;
    pub mod hash;
    pub mod render;
    pub mod search;
    pub mod sim;
    pub mod sound;
    pub mod story;
}

/// The most worker threads a command starts.
const MAX_WORKERS: u64 = 256;

#[derive(Parser)]
#[command(
    name = "ballfight",
    version,
    about = "BALLFIGHT Studio without a window: fight, search, balance, draw, sound, check and render a fight file.",
    after_help = "Exit code 0 on success, 1 on bad input or a failed check, with the reason in one line on stderr."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Threads for the commands that fight or draw many (search, balance, check, render); default: all cores but
    /// one.
    #[arg(long, global = true, value_name = "N", value_parser = clap::value_parser!(u64).range(1..=MAX_WORKERS))]
    workers: Option<u64>,
    /// Print one JSON value on stdout instead of text.
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Fight one seed and save its recording (.bfrec).
    Sim(cmd::sim::Args),
    /// Tell the story of one fight and whether it fits the series.
    Story(cmd::story::Args),
    /// Search the seeds for fights that fit and list the best.
    Search(cmd::search::Args),
    /// Find the damage handicap that makes the pairing even.
    Balance(cmd::balance::Args),
    /// Draw one frame of the video as a PNG.
    Frame(cmd::frame::Args),
    /// Build the soundtrack and save it as a .wav.
    Sound(cmd::sound::Args),
    /// Run every check a video must pass before it is rendered.
    Check(cmd::check::Args),
    /// Render the video into a new folder.
    Render(cmd::render::Args),
    /// Print the replay line of each seed: winner, K.O. time, events and the recording's hash.
    Hash(cmd::hash::Args),
}

/// Why a command did not succeed: one sentence for stderr, exit code 1.
#[derive(Debug)]
pub struct Fail(pub String);

impl Fail {
    pub fn new(why: impl Display) -> Fail {
        Fail(why.to_string())
    }
}

pub type Done = Result<(), Fail>;

/// What every command gets besides its own arguments.
pub struct Ctx {
    /// Worker threads (at least 1).
    pub workers: usize,
    /// `--json`.
    pub json: bool,
    /// Set by Ctrl+C: the long commands stop and clean up.
    pub cancel: &'static AtomicBool,
}

static CANCEL: AtomicBool = AtomicBool::new(false);

/// The message of the last panic (the hook keeps it instead of printing it).
static PANIC: Mutex<Option<String>> = Mutex::new(None);

fn main() -> ExitCode {
    panic::set_hook(Box::new(|info| {
        let what = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "no message".to_string());
        let at = info
            .location()
            .map(|l| format!(" at {}:{}", l.file(), l.line()))
            .unwrap_or_default();
        if let Ok(mut slot) = PANIC.lock() {
            *slot = Some(format!("{what}{at}"));
        }
    }));
    stop_on_ctrl_c();
    match panic::catch_unwind(AssertUnwindSafe(run)) {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(Fail(why))) => {
            note(&format!("ballfight: {}", one_line(&why)));
            ExitCode::FAILURE
        }
        Err(_) => {
            let what = PANIC.lock().ok().and_then(|s| s.clone()).unwrap_or_default();
            note(&format!(
                "ballfight: the program met a fault of its own and stopped ({}); this is a bug, so please report it \
                 with the command you ran.",
                one_line(&what)
            ));
            ExitCode::FAILURE
        }
    }
}

fn run() -> Done {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            use clap::error::ErrorKind;
            if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                say(e.render().to_string().trim_end());
                return Ok(());
            }
            return Err(Fail(format!("{} (see ballfight --help).", clap_line(&e))));
        }
    };
    let Some(command) = cli.command else {
        let names: Vec<String> = Cli::command()
            .get_subcommands()
            .map(|c| c.get_name().to_string())
            .collect();
        return Err(Fail(format!(
            "give a command ({}); ballfight --help says what each one does.",
            names.join(", ")
        )));
    };
    let workers = match cli.workers {
        Some(n) => usize::try_from(n).unwrap_or(1),
        None => std::thread::available_parallelism().map_or(1, |n| n.get().saturating_sub(1).max(1)),
    };
    let ctx = Ctx {
        workers,
        json: cli.json,
        cancel: &CANCEL,
    };
    match command {
        Command::Sim(a) => cmd::sim::run(&a, &ctx),
        Command::Story(a) => cmd::story::run(&a, &ctx),
        Command::Search(a) => cmd::search::run(&a, &ctx),
        Command::Balance(a) => cmd::balance::run(&a, &ctx),
        Command::Frame(a) => cmd::frame::run(&a, &ctx),
        Command::Sound(a) => cmd::sound::run(&a, &ctx),
        Command::Check(a) => cmd::check::run(&a, &ctx),
        Command::Render(a) => cmd::render::run(&a, &ctx),
        Command::Hash(a) => cmd::hash::run(&a, &ctx),
    }
}

/// A clap error as one line: its text before the usage, without the "error:" mark and the tips.
fn clap_line(e: &clap::Error) -> String {
    let text = e.render().to_string();
    let words: Vec<&str> = text
        .lines()
        .take_while(|l| !l.starts_with("Usage:") && !l.starts_with("For more information"))
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("tip:"))
        .collect();
    let line = words.join(" ");
    let line = line.strip_prefix("error: ").unwrap_or(&line);
    line.trim_end_matches('.').to_string()
}

/// Text as one line.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// On Windows, Ctrl+C (or Ctrl+Break) sets the stop flag the first time, so a render removes its temporary folder
/// and a search shows what it found; a second press ends the program at once.
fn stop_on_ctrl_c() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
        use windows_sys::core::BOOL;

        unsafe extern "system" fn handler(kind: u32) -> BOOL {
            // CTRL_C_EVENT 0, CTRL_BREAK_EVENT 1; the others (closing the window, logging off) end the program
            if kind <= 1 && !CANCEL.swap(true, Ordering::SeqCst) {
                note("ballfight: stopping (press Ctrl+C again to end at once)");
                1
            } else {
                0
            }
        }
        // SAFETY: `handler` is a plain function that only touches an atomic and stderr, as the call requires.
        unsafe {
            SetConsoleCtrlHandler(Some(handler), 1);
        }
    }
}

// ---------------------------------------------------------------- output

/// One line on stdout. A closed pipe ends the program quietly (exit code 0: whoever read the output stopped
/// reading); any other failure to write ends it with exit code 1 and a line on stderr.
pub fn say(text: &str) {
    let mut out = io::stdout().lock();
    let wrote = writeln!(out, "{text}").and_then(|()| out.flush());
    if let Err(e) = wrote {
        if e.kind() == io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        note(&format!("ballfight: the output could not be written ({e})."));
        std::process::exit(1);
    }
}

/// `value` as JSON on stdout: the whole of stdout for a `--json` command.
pub fn say_json(value: &Value) {
    say(&json_text(value));
}

/// `value` as the JSON text `--json` prints.
pub fn json_text(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".to_string())
}

/// One line on stderr (progress and the reason of a failure); a closed stderr is ignored.
pub fn note(text: &str) {
    let mut err = io::stderr().lock();
    let _ = writeln!(err, "{text}");
}

// ---------------------------------------------------------------- inputs

/// The fight file at `path`, read, parsed and checked against what this app has (the fighters, the theme, the
/// chords, the parts in the score), with its text.
pub fn load_fight_text(path: &Path) -> Result<(FightFile, String), Fail> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| Fail(format!("the fight file {} cannot be read ({e}).", path.display())))?;
    let ff = FightFile::parse(&text).map_err(|e| Fail(format!("{}: {e}.", path.display())))?;
    bf_content::validate(&ff).map_err(|e| Fail(format!("{}: {e}.", path.display())))?;
    Ok((ff, text))
}

/// [`load_fight_text`] without the text.
pub fn load_fight(path: &Path) -> Result<FightFile, Fail> {
    load_fight_text(path).map(|(ff, _)| ff)
}

/// The two fighters' roster entries (the fight file has passed [`load_fight`]).
pub fn fighters(ff: &FightFile) -> Result<[&'static FighterCfg; 2], Fail> {
    let find = |k: usize| {
        let name = ff.fighters.get(k).map(String::as_str).unwrap_or_default();
        roster()
            .iter()
            .find(|f| f.name == name)
            .ok_or_else(|| Fail(format!("fighters[{k}]: {name:?} is not in the roster.")))
    };
    Ok([find(0)?, find(1)?])
}

/// The seed to fight: `--seed`, else the fight file's.
pub fn seed_of(ff: &FightFile, path: &Path, seed: Option<u64>) -> Result<u64, Fail> {
    seed.or(ff.seed).ok_or_else(|| {
        Fail(format!(
            "the fight file {} has no seed, so give one with --seed N (ballfight search lists seeds that fit).",
            path.display()
        ))
    })
}

/// The fight of `seed` under the fight file.
pub fn fight(ff: &FightFile, path: &Path, seed: u64, record: bool) -> Result<Recording, Fail> {
    let setup = bf_sim::setup_from(ff, roster(), seed).map_err(|e| Fail(format!("{}: {e}.", path.display())))?;
    bf_sim::run(&setup, record).map_err(|e| Fail(format!("{}: seed {seed}: {e}.", path.display())))
}

/// The fight a command works on: the recording `rec` if given (it must be a fight of the fight file's two
/// fighters), else the fight of `seed` (or the fight file's seed).
pub fn the_fight(
    ff: &FightFile,
    path: &Path,
    rec: Option<&Path>,
    seed: Option<u64>,
    record: bool,
) -> Result<Recording, Fail> {
    let Some(file) = rec else {
        return fight(ff, path, seed_of(ff, path, seed)?, record);
    };
    let rec = Recording::read(file).map_err(|e| Fail(format!("{e}.")))?;
    if rec.names != ff.fighters {
        return Err(Fail(format!(
            "the recording {} is a fight of {}, but the fight file {} is {}, so they do not belong together.",
            file.display(),
            rec.names.join(" against "),
            path.display(),
            ff.fighters.join(" against ")
        )));
    }
    Ok(rec)
}

/// Refuses a fight that has no K.O.: it has no show, so no video, sound or checks.
pub fn need_ko(rec: &Recording) -> Done {
    if rec.winner.is_none() || rec.t_ko.is_none() {
        return Err(Fail(format!(
            "seed {} has no K.O. before 58 s, so it has no video; choose another seed (ballfight search lists \
             seeds that fit).",
            rec.seed
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------- files

/// Refuses an output path that already exists, or whose folder does not.
pub fn new_file(path: &Path) -> Done {
    if std::fs::symlink_metadata(path).is_ok() {
        return Err(Fail(format!(
            "{} already exists, and ballfight never replaces a file; choose another name or delete it first.",
            path.display()
        )));
    }
    let dir = folder_of(path);
    if !dir.is_dir() {
        return Err(Fail(format!(
            "the folder {} does not exist, so {} cannot be written there.",
            dir.display(),
            path.display()
        )));
    }
    Ok(())
}

/// The folder a file goes in (`.` for a bare name).
fn folder_of(path: &Path) -> PathBuf {
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// A temporary name beside `path`, for writing it whole before it appears.
pub fn temp_beside(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    folder_of(path).join(format!(".{name}.{}.tmp", std::process::id()))
}

/// Writes a new file at `path` whole or not at all, never replacing one: `write` fills a temporary file beside
/// it, which then gets the final name only if that is still free.
pub fn write_new_with(path: &Path, write: impl FnOnce(&Path) -> Result<(), String>) -> Done {
    new_file(path)?;
    let tmp = temp_beside(path);
    let failed = |why: String| Fail(format!("{} could not be written ({why}).", path.display()));
    if let Err(why) = write(&tmp) {
        let _ = std::fs::remove_file(&tmp);
        return Err(failed(why));
    }
    // a hard link never replaces anything; where the drive has none (FAT), check the name, then rename
    let placed = match std::fs::hard_link(&tmp, path) {
        Ok(()) => std::fs::remove_file(&tmp).or(Ok(())),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Err(e),
        Err(_) if std::fs::symlink_metadata(path).is_ok() => Err(io::Error::from(io::ErrorKind::AlreadyExists)),
        Err(_) => std::fs::rename(&tmp, path),
    };
    match placed {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            if e.kind() == io::ErrorKind::AlreadyExists {
                Err(Fail(format!(
                    "{} appeared while ballfight was working, and ballfight never replaces a file; nothing was \
                     written.",
                    path.display()
                )))
            } else {
                Err(failed(e.to_string()))
            }
        }
    }
}

/// [`write_new_with`] for bytes.
pub fn write_new(path: &Path, bytes: &[u8]) -> Done {
    write_new_with(path, |tmp| write_whole(tmp, bytes))
}

/// Writes `bytes` to a new file at `path` and makes sure they are on the disk.
pub fn write_whole(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    f.write_all(bytes)
        .and_then(|()| f.sync_all())
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- shared wording

/// A time in seconds with two decimals.
pub fn secs(t: f64) -> String {
    format!("{t:.2} s")
}

/// A finite number as JSON (`null` otherwise).
pub fn num(x: f64) -> Value {
    serde_json::Number::from_f64(x).map_or(Value::Null, Value::Number)
}

/// Stops with the stop message when Ctrl+C was pressed.
pub fn stopped(ctx: &Ctx, what: &str) -> Done {
    if ctx.cancel.load(Ordering::SeqCst) {
        return Err(Fail(format!("{what} was stopped (Ctrl+C).")));
    }
    Ok(())
}
