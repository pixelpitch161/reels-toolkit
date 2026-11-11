//! Fault reports: what the app leaves behind when it stops because of a fault in its own code, so the owner can be
//! asked to send one file.
//!
//! [`install_hook`] is called once, from `main` on the interface thread. From then on every panic, on any thread,
//! writes one report `crash-YYYYMMDD-HHMMSS.txt` into the app's own `logs` folder (`store::Paths::logs`, never
//! anywhere else), after the standard library's usual line on stderr. A panic on the interface thread also shows a
//! message box ([`CRASHED`]). A panic on a worker thread only writes its report: the engine already catches it and
//! turns it into a sentence on screen (`engine::Ctx::spawn`), and the hook runs once per panic, before that catch,
//! so the fault is recorded once. The hook never ends the process itself.
//!
//! **What a report holds:** the app's version and commit, Windows' version numbers, the time, the thread's name, the
//! panic's message (at most [`MAX_MESSAGE`] characters), its file and line, and the stack. **What it never holds:**
//! settings values, file contents, environment variables, captions or anything else typed into the app: the hook
//! adds nothing but those seven things. A message or stack that names a path under the owner's user folder
//! (`%USERPROFILE%`) has that folder written as `%USERPROFILE%`, so the owner's user name does not appear. A report
//! is at most [`MAX_BYTES`] long, and only the newest [`KEEP`] are kept.
//!
//! **Writing is best effort and never makes things worse.** A folder that cannot be written or a full disk only means
//! no report, and the message box then leaves out "A report was saved as". The hook's own code is written not to
//! panic (a panic inside a panic hook ends the process at once), waits on no lock that the panicking code could
//! hold, never retries, and never sleeps. The message box is shown from a thread of its own, so the frozen window's
//! messages are never handled while the hook runs.

use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::thread::{self, ThreadId};

/// The message box for a fault on the interface thread, when the report was saved; `{file}` is the report's path.
pub const CRASHED: &str = "BALLFIGHT Studio stopped because of a fault in the app. A report was saved as {file}. \
    Send that file to whoever builds the app, then start BALLFIGHT Studio again: your queue and settings are kept.";

/// The message box for a fault on the interface thread when no report could be saved.
pub const CRASHED_UNSAVED: &str = "BALLFIGHT Studio stopped because of a fault in the app. Start BALLFIGHT Studio \
    again: your queue and settings are kept.";

/// How many fault reports are kept: the newest.
pub const KEEP: usize = 20;

/// The most bytes one report holds; a longer stack is cut short.
pub const MAX_BYTES: usize = 64 * 1024;

/// The most characters of a panic's message a report holds.
pub const MAX_MESSAGE: usize = 1000;

/// The most bytes `start.txt` holds.
pub const MAX_START_BYTES: usize = 16 * 1024;

/// What a cut-short text ends with.
const CUT: &str = "\n(cut short)\n";

/// What the hook knows, worked out once when it is installed so that the hook itself does as little as it can.
struct Hook {
    logs: Option<PathBuf>,
    home: Vec<String>,
    interface: ThreadId,
}

static HOOK: OnceLock<Hook> = OnceLock::new();

thread_local! {
    /// Whether this thread is inside the hook already.
    static IN_HOOK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Sets the panic hook (see the module's notes). `logs` is where reports go; `None` writes none and still shows
/// the message box. The thread that calls it is the interface thread. Only the first call counts; a second does
/// nothing.
pub fn install_hook(logs: Option<PathBuf>) {
    let hook = Hook {
        logs,
        home: home_forms(user_folder().as_deref()),
        interface: thread::current().id(),
    };
    if HOOK.set(hook).is_err() {
        return;
    }
    let standard = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        standard(info);
        if let Some(hook) = HOOK.get() {
            on_panic(hook, info);
        }
    }));
}

/// What the hook does after the standard line: write the report, and on the interface thread show the box.
fn on_panic(hook: &Hook, info: &std::panic::PanicHookInfo<'_>) {
    // `try_with` fails (and nothing is done) only while the thread's own storage is being torn down.
    let Ok(false) = IN_HOOK.try_with(|inside| inside.replace(true)) else {
        return;
    };
    let current = thread::current();
    let message = match info.payload().downcast_ref::<&str>() {
        Some(text) => (*text).to_string(),
        None => match info.payload().downcast_ref::<String>() {
            Some(text) => text.clone(),
            None => "(the fault gave no message)".to_string(),
        },
    };
    let mut location = String::new();
    match info.location() {
        Some(at) => _ = write!(location, "{}:{}", at.file(), at.line()),
        None => location.push_str("unknown"),
    }
    let mut backtrace = String::new();
    _ = write!(backtrace, "{}", std::backtrace::Backtrace::force_capture());
    let now = chrono::Local::now();
    let mut when = String::new();
    _ = write!(when, "{}", now.format("%Y-%m-%d %H:%M:%S"));
    // Shortened before `crash_text` cuts anything, so a cut never leaves half of the user folder's name.
    let text = crash_text(
        &shorten(&message, &hook.home),
        &shorten(&location, &hook.home),
        current.name().unwrap_or("unnamed"),
        &shorten(&backtrace, &hook.home),
        &when,
    );
    let saved = hook.logs.as_deref().and_then(|logs| write_crash(logs, &text, now).ok());
    if current.id() == hook.interface {
        let words = crashed_text(saved.as_deref());
        // On a thread of its own: the interface thread is stopped inside its window's code, and a message box on it
        // would hand that window's messages back to the code that just failed. Shown here only if no thread starts.
        let shown = thread::Builder::new().name("fault message".to_string()).spawn({
            let words = words.clone();
            move || tell("BALLFIGHT Studio", &words)
        });
        match shown {
            Ok(box_thread) => _ = box_thread.join(),
            Err(_) => tell("BALLFIGHT Studio", &words),
        }
    }
    _ = IN_HOOK.try_with(|inside| inside.set(false));
}

/// The message box's text: [`CRASHED`] naming `report`, or [`CRASHED_UNSAVED`] when there is none.
pub fn crashed_text(report: Option<&Path>) -> String {
    match report {
        Some(file) => CRASHED.replace("{file}", &file.display().to_string()),
        None => CRASHED_UNSAVED.to_string(),
    }
}

/// The text of one report, at most [`MAX_BYTES`] long (the stack is cut short first). Paths are written as given:
/// the hook shortens the user folder after this.
pub fn crash_text(message: &str, location: &str, thread: &str, backtrace: &str, when: &str) -> String {
    let message: String = if message.chars().count() > MAX_MESSAGE {
        let mut cut: String = message.chars().take(MAX_MESSAGE).collect();
        cut.push_str(" (cut short)");
        cut
    } else {
        message.to_string()
    };
    let mut text = String::new();
    _ = writeln!(text, "BALLFIGHT Studio fault report");
    _ = writeln!(text);
    _ = writeln!(text, "App: {}", crate::version::version_text());
    _ = writeln!(text, "Windows: {}", windows_version());
    _ = writeln!(text, "Time: {when}");
    _ = writeln!(text, "Thread: {thread}");
    _ = writeln!(text, "Place: {location}");
    _ = writeln!(text, "Message: {message}");
    _ = writeln!(text);
    _ = writeln!(text, "Where the app was (the stack):");
    text.push_str(backtrace);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    cap(text, MAX_BYTES)
}

/// `text`, or its start ending with [`CUT`] so that the whole is at most `max` bytes, never splitting a letter.
fn cap(mut text: String, max: usize) -> String {
    if text.len() <= max {
        return text;
    }
    let mut end = max.saturating_sub(CUT.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text.push_str(CUT);
    text
}

/// Windows' version numbers, such as "10.0.26100", or "unknown".
fn windows_version() -> String {
    #[cfg(windows)]
    {
        if let Some((major, minor, build)) = win::version() {
            return format!("{major}.{minor}.{build}");
        }
    }
    "unknown".to_string()
}

/// `crash-YYYYMMDD-HHMMSS` and its number (1 for the plain name, n for `-n`), when `name` is a report's name.
fn report_name(name: &str) -> Option<(&str, u32)> {
    let rest = name.strip_prefix("crash-")?.strip_suffix(".txt")?;
    let b = rest.as_bytes();
    if b.len() < 15
        || !b[..8].iter().all(u8::is_ascii_digit)
        || b[8] != b'-'
        || !b[9..15].iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    let n = match rest.get(15..)? {
        "" => 1,
        more => more.strip_prefix('-')?.parse::<u32>().ok().filter(|n| *n >= 2)?,
    };
    Some((rest.get(..15)?, n))
}

/// The most reports one second may hold.
const MAX_IN_ONE_SECOND: u32 = 99;

/// Writes `text` as a new report in `logs` (made if missing), named by `now` (`crash-20261112-093005.txt`, then
/// `-2`, `-3` for more in the same second; nothing is ever replaced), and then keeps only the newest [`KEEP`]
/// reports. Never waits and never retries: a failed write removes what it began and gives the error.
pub fn write_crash(logs: &Path, text: &str, now: chrono::DateTime<chrono::Local>) -> io::Result<PathBuf> {
    fs::create_dir_all(logs)?;
    let mut stamp = String::new();
    _ = write!(stamp, "{}", now.format("crash-%Y%m%d-%H%M%S"));
    let mut written = None;
    for n in 1..=MAX_IN_ONE_SECOND {
        let name = if n == 1 {
            format!("{stamp}.txt")
        } else {
            format!("{stamp}-{n}.txt")
        };
        let path = logs.join(name);
        match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                let bytes = cap(text.to_string(), MAX_BYTES);
                if let Err(err) = file.write_all(bytes.as_bytes()).and_then(|()| file.flush()) {
                    drop(file);
                    _ = fs::remove_file(&path);
                    return Err(err);
                }
                written = Some(path);
                break;
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err),
        }
    }
    let path = written.ok_or_else(|| io::Error::new(io::ErrorKind::AlreadyExists, "too many reports in one second"))?;
    prune(logs);
    Ok(path)
}

/// Removes all but the newest [`KEEP`] reports in `logs`; any other file is left alone. Best effort.
fn prune(logs: &Path) {
    let Ok(entries) = fs::read_dir(logs) else { return };
    let mut found: Vec<(String, u32, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let (stamp, n) = report_name(&name)?;
            Some((stamp.to_string(), n, entry.path()))
        })
        .collect();
    if found.len() <= KEEP {
        return;
    }
    found.sort();
    let extra = found.len() - KEEP;
    for (_, _, path) in found.into_iter().take(extra) {
        _ = fs::remove_file(path);
    }
}

/// Writes `start.txt` in `logs` (made if missing), replacing the last start's: one line each, at most
/// [`MAX_START_BYTES`], with the user folder written as `%USERPROFILE%`.
pub fn write_start(logs: &Path, lines: &[String]) -> io::Result<()> {
    let mut text = String::new();
    for line in lines {
        text.push_str(line);
        text.push('\n');
        if text.len() > MAX_START_BYTES {
            break;
        }
    }
    let text = cap(shorten(&text, &home_forms(user_folder().as_deref())), MAX_START_BYTES);
    crate::store::write_atomic(&logs.join("start.txt"), text.as_bytes())
}

/// Tells a person something went wrong: a Windows message box with the error icon, which waits for its OK;
/// elsewhere a line on stderr.
pub fn tell(title: &str, text: &str) {
    #[cfg(windows)]
    win::message_box(title, text);
    #[cfg(not(windows))]
    {
        _ = writeln!(io::stderr(), "{title}: {text}");
    }
}

/// The owner's user folder, as Windows names it (`%USERPROFILE%`; `HOME` elsewhere).
fn user_folder() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

/// The forms of the user folder `home` that [`shorten`] replaces: as written with `\` and with `/`. None for a
/// folder with fewer than two parts below its drive (such as `C:\`), which would match too much.
fn home_forms(home: Option<&Path>) -> Vec<String> {
    let Some(home) = home else { return Vec::new() };
    let parts = home
        .components()
        .filter(|c| matches!(c, std::path::Component::Normal(_)))
        .count();
    if parts < 2 {
        return Vec::new();
    }
    let text = home.to_string_lossy();
    let text = text.trim_end_matches(['\\', '/']);
    let mut forms = vec![text.replace('/', "\\"), text.replace('\\', "/")];
    forms.dedup();
    forms
}

/// `text` with every mention of the user folder (any form of `home`, in any mix of capitals) written as
/// `%USERPROFILE%`. A mention counts only where the folder's name ends (the next character is not a letter, a
/// digit, `_` or `-`): `C:\Users\al` is not found in `C:\Users\alice`.
fn shorten(text: &str, home: &[String]) -> String {
    let mut out = text.to_string();
    for form in home {
        let wanted: Vec<char> = form.chars().flat_map(char::to_lowercase).collect();
        let mut result = String::with_capacity(out.len());
        let chars: Vec<(usize, char)> = out.char_indices().collect();
        let mut i = 0;
        while i < chars.len() {
            if let Some(used) = matches_at(&chars, i, &wanted) {
                let next = chars.get(i + used).map(|&(_, c)| c);
                if next.is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '-')) {
                    result.push_str("%USERPROFILE%");
                    i += used;
                    continue;
                }
            }
            result.push(chars[i].1);
            i += 1;
        }
        out = result;
    }
    out
}

/// How many characters of `chars`, from `at`, spell `wanted` (lower-cased, `\` and `/` the same), if they do.
fn matches_at(chars: &[(usize, char)], at: usize, wanted: &[char]) -> Option<usize> {
    let mut w = 0;
    let mut i = at;
    while w < wanted.len() {
        let &(_, c) = chars.get(i)?;
        for lower in c.to_lowercase() {
            let same = lower == *wanted.get(w)? || (matches!(lower, '\\' | '/') && matches!(wanted[w], '\\' | '/'));
            if !same {
                return None;
            }
            w += 1;
        }
        i += 1;
    }
    Some(i - at)
}

/// The two Windows calls, declared here so that this file needs no new dependency features.
#[cfg(windows)]
mod win {
    use std::ffi::c_void;

    #[repr(C)]
    struct OsVersionInfoW {
        size: u32,
        major: u32,
        minor: u32,
        build: u32,
        platform: u32,
        service_pack: [u16; 128],
    }

    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn RtlGetVersion(info: *mut OsVersionInfoW) -> i32;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(owner: *mut c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
    }

    const MB_ICONERROR: u32 = 0x0000_0010;
    const MB_SETFOREGROUND: u32 = 0x0001_0000;
    const MB_TOPMOST: u32 = 0x0004_0000;

    /// Windows' real version numbers (`RtlGetVersion` is not shimmed the way `GetVersionEx` is).
    pub(super) fn version() -> Option<(u32, u32, u32)> {
        let mut info = OsVersionInfoW {
            size: size_of::<OsVersionInfoW>() as u32,
            major: 0,
            minor: 0,
            build: 0,
            platform: 0,
            service_pack: [0; 128],
        };
        // SAFETY: `info` is a properly sized, writable OSVERSIONINFOW with its size field set, as the call needs.
        let status = unsafe { RtlGetVersion(&mut info) };
        (status == 0).then_some((info.major, info.minor, info.build))
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16()
            .filter(|&u| u != 0)
            .chain(std::iter::once(0))
            .collect()
    }

    /// A message box with no owner window, the error icon and one OK button, in front of other windows.
    pub(super) fn message_box(title: &str, text: &str) {
        let (title, text) = (wide(title), wide(text));
        // SAFETY: both strings are NUL-terminated UTF-16 that outlive the call; a null owner is allowed.
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_ICONERROR | MB_SETFOREGROUND | MB_TOPMOST,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{TempRoot, files_in};
    use chrono::TimeZone;
    use std::ffi::OsStr;
    use std::fs;
    use std::io::Read;
    use std::time::{Duration, Instant};

    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> chrono::DateTime<chrono::Local> {
        chrono::Local.with_ymd_and_hms(y, mo, d, h, mi, s).single().unwrap()
    }

    fn reports(dir: &Path) -> Vec<String> {
        if !dir.is_dir() {
            return Vec::new();
        }
        files_in(dir).into_iter().filter(|n| n.starts_with("crash-")).collect()
    }

    #[test]
    fn a_report_holds_what_is_needed() {
        let text = crash_text(
            "index out of bounds",
            r"crates\bf-app\src\file.rs:12",
            "engine: jobs",
            "   0: bf_app::thing\n   1: main",
            "2026-11-12 09:30:05",
        );
        for wanted in [
            crate::version::version_text().as_str(),
            "index out of bounds",
            "file.rs:12",
            "engine: jobs",
            "stack",
            "bf_app::thing",
            "2026-11-12 09:30:05",
            "Windows",
        ] {
            assert!(text.contains(wanted), "{wanted:?} is missing from:\n{text}");
        }
    }

    #[test]
    fn a_report_is_kept_short() {
        let long_message = "m".repeat(MAX_MESSAGE * 3);
        let long_stack = "frame\n".repeat(MAX_BYTES);
        let text = crash_text(&long_message, "a.rs:1", "main", &long_stack, "now");
        assert!(text.len() <= MAX_BYTES, "{}", text.len());
        assert!(text.contains(&"m".repeat(MAX_MESSAGE)));
        assert!(!text.contains(&"m".repeat(MAX_MESSAGE + 1)));
        assert!(text.contains("cut short"));
        // a cut never splits a letter
        let wide = "é".repeat(MAX_BYTES);
        let text = crash_text("x", "a.rs:1", "main", &wide, "now");
        assert!(text.len() <= MAX_BYTES, "{}", text.len());
    }

    #[test]
    fn the_user_folder_is_shortened() {
        let forms = home_forms(Some(Path::new(r"C:\Users\Zoë Ångström")));
        let text = "opened C:\\Users\\Zoë Ångström\\Videos\\a.mp4 and c:/users/zoë ångström/.cargo/x.rs, \
                    not C:\\Users\\Zoë Ångströmer\\b; at the end C:\\Users\\Zoë Ångström";
        let short = shorten(text, &forms);
        assert!(short.contains(r"%USERPROFILE%\Videos\a.mp4"), "{short}");
        assert!(short.contains("%USERPROFILE%/.cargo/x.rs"), "{short}");
        assert!(short.ends_with("at the end %USERPROFILE%"), "{short}");
        // another user whose name merely starts the same way is not cut in half
        assert!(short.contains(r"C:\Users\Zoë Ångströmer\b"), "{short}");
        assert_eq!(short.matches("%USERPROFILE%").count(), 3, "{short}");
        // a user folder that is a whole drive or nothing is not replaced everywhere
        assert!(home_forms(Some(Path::new(r"C:\"))).is_empty());
        assert!(home_forms(Some(Path::new(""))).is_empty());
        assert!(home_forms(None).is_empty());
    }

    #[test]
    fn a_report_is_named_by_its_time() {
        let root = TempRoot::new("a_report_is_named_by_its_time");
        let logs = root.0.join("logs");
        let first = write_crash(&logs, "first", at(2026, 11, 12, 9, 30, 5)).unwrap();
        assert_eq!(first, logs.join("crash-20261112-093005.txt"));
        assert_eq!(fs::read_to_string(&first).unwrap(), "first");
        // a second fault in the same second gets a name of its own and replaces nothing
        let second = write_crash(&logs, "second", at(2026, 11, 12, 9, 30, 5)).unwrap();
        assert_eq!(second, logs.join("crash-20261112-093005-2.txt"));
        assert_eq!(fs::read_to_string(&first).unwrap(), "first");
        assert_eq!(fs::read_to_string(&second).unwrap(), "second");
    }

    #[test]
    fn the_newest_twenty_are_kept() {
        let root = TempRoot::new("the_newest_twenty_are_kept");
        let logs = root.0.join("logs");
        fs::create_dir_all(&logs).unwrap();
        // files that are not reports are never touched
        fs::write(logs.join("start.txt"), "start").unwrap();
        fs::write(logs.join("crash-notes.txt"), "mine").unwrap();
        for minute in 0..25 {
            write_crash(&logs, "x", at(2026, 11, 12, 10, minute, 0)).unwrap();
        }
        // two in one second: the one with the number is the newer
        write_crash(&logs, "y", at(2026, 11, 12, 10, 59, 0)).unwrap();
        write_crash(&logs, "z", at(2026, 11, 12, 10, 59, 0)).unwrap();
        let kept = reports(&logs);
        assert_eq!(kept.len(), KEEP + 1, "{kept:?}");
        assert!(kept.contains(&"crash-notes.txt".to_string()));
        assert!(kept.contains(&"crash-20261112-105900.txt".to_string()), "{kept:?}");
        assert!(kept.contains(&"crash-20261112-105900-2.txt".to_string()), "{kept:?}");
        // 27 written, 20 kept: minutes 7 to 24 and the two at 10:59
        assert!(!kept.contains(&"crash-20261112-100600.txt".to_string()), "{kept:?}");
        assert!(kept.contains(&"crash-20261112-100700.txt".to_string()), "{kept:?}");
        assert!(logs.join("start.txt").is_file());
    }

    #[test]
    fn only_report_names_are_counted() {
        assert_eq!(report_name("crash-20261112-093005.txt"), Some(("20261112-093005", 1)));
        assert_eq!(
            report_name("crash-20261112-093005-12.txt"),
            Some(("20261112-093005", 12))
        );
        for other in [
            "crash-20261112-093005-1.txt",
            "crash-20261112-093005-x.txt",
            "crash-2026111-0930055.txt",
            "crash-notes.txt",
            "crash-éééééééé-093005.txt",
            "crash-20261112-09300é.txt",
            "start.txt",
            "crash-20261112-093005.txt.tmp",
        ] {
            assert_eq!(report_name(other), None, "{other}");
        }
    }

    #[test]
    fn a_folder_that_cannot_be_written_is_survived() {
        let root = TempRoot::new("a_folder_that_cannot_be_written_is_survived");
        let plain = root.0.join("a plain file");
        fs::write(&plain, "not a folder").unwrap();
        let logs = plain.join("logs");
        assert!(write_crash(&logs, "x", at(2026, 11, 12, 9, 30, 5)).is_err());
        assert!(write_start(&logs, &["a line".to_string()]).is_err());
        assert_eq!(fs::read_to_string(&plain).unwrap(), "not a folder");
    }

    #[test]
    fn the_start_record_is_replaced_each_start() {
        let root = TempRoot::new("the_start_record_is_replaced_each_start");
        let logs = root.0.join("logs");
        write_start(&logs, &["first start".to_string()]).unwrap();
        write_start(&logs, &["second start".to_string(), "adapter: test".to_string()]).unwrap();
        assert_eq!(
            fs::read_to_string(logs.join("start.txt")).unwrap(),
            "second start\nadapter: test\n"
        );
        assert_eq!(files_in(&logs), ["start.txt"]);
        let many: Vec<String> = (0..10_000).map(|i| format!("line {i}")).collect();
        write_start(&logs, &many).unwrap();
        let len = fs::metadata(logs.join("start.txt")).unwrap().len();
        assert!(len <= MAX_START_BYTES as u64, "{len}");
        assert!(
            fs::read_to_string(logs.join("start.txt"))
                .unwrap()
                .contains("cut short")
        );
    }

    #[test]
    fn the_texts_read_plainly() {
        let file = Path::new(r"C:\Users\x\AppData\Local\BALLFIGHT Studio\data\logs\crash-20261112-093005.txt");
        let saved = crashed_text(Some(file));
        assert!(saved.starts_with("BALLFIGHT Studio stopped because of a fault in the app. A report was saved as"));
        assert!(saved.contains(&format!("saved as {}. Send", file.display())), "{saved}");
        assert!(!saved.contains("{file}"));
        let unsaved = crashed_text(None);
        assert_eq!(unsaved, CRASHED_UNSAVED);
        assert!(!unsaved.contains("report"));
        for text in [CRASHED, CRASHED_UNSAVED] {
            assert!(text.ends_with('.'), "{text}");
            assert_eq!(capitals_out_of_place(text), Vec::<String>::new(), "{text}");
        }
    }

    /// The sentence-case rule of `tests/ui/wording.rs` (private to that test program), cut down to what these
    /// texts need: after its first word, a word may start with a capital only after ". " or ": ", when it holds a
    /// digit, a backslash, an underscore or a `{` (a place to fill), or when it is "BALLFIGHT" or "Studio".
    fn capitals_out_of_place(text: &str) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut bad = Vec::new();
        for pair in words.windows(2) {
            let (before, word) = (pair[0], pair[1]);
            if !word.chars().next().is_some_and(char::is_uppercase)
                || before.ends_with('.')
                || before.ends_with(':')
                || word
                    .chars()
                    .any(|c| c.is_ascii_digit() || c == '\\' || c == '_' || c == '{')
                || ["BALLFIGHT", "Studio"].contains(&word.trim_end_matches([',', '.', ':']))
            {
                continue;
            }
            bad.push(word.to_string());
        }
        bad
    }

    #[test]
    fn the_rule_here_bites() {
        assert_eq!(capitals_out_of_place("Start BALLFIGHT Studio Again"), ["Again"]);
        assert!(capitals_out_of_place("It stopped. Send it: Then go").is_empty());
    }

    /// Tells the child half of a hook test (a new process of this test program) where its reports go.
    const CHILD: &str = "BF_APP_REPORT_CHILD";

    /// The start of the child's panic message.
    const ON_PURPOSE: &str = "a fault on purpose";

    /// Runs the child test `name` of this test program with `CHILD` set to `logs`, and gives its exit code and what
    /// it printed. The panic hook belongs to the whole process and every other test runs in this one, so no test
    /// installs the hook here: only these children do, each in a process of its own that ends with its one test.
    fn run_child(name: &str, logs: &Path) -> (Option<i32>, String) {
        let exe = std::env::current_exe().expect("this test program");
        let args: [&OsStr; 5] = [
            name.as_ref(),
            "--exact".as_ref(),
            "--include-ignored".as_ref(),
            "--test-threads=1".as_ref(),
            "--nocapture".as_ref(),
        ];
        let mut child = bf_pack::run::quiet(&exe, &args)
            .env(CHILD, logs)
            .env_remove("RUST_BACKTRACE")
            .spawn()
            .expect("the child test started");
        // Its output is read on threads of their own, so a full pipe can never stall it.
        let read = |pipe: Option<Box<dyn Read + Send>>| {
            std::thread::spawn(move || {
                let mut text = String::new();
                if let Some(mut p) = pipe {
                    let _ = p.read_to_string(&mut text);
                }
                text
            })
        };
        let stdout = read(child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
        let stderr = read(child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
        let deadline = Instant::now() + Duration::from_secs(120);
        let status = loop {
            if let Some(status) = child.try_wait().expect("the child can be waited for") {
                break status;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("the child test did not end within 120 s, so it was stopped (the hook may hang)");
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let said = format!(
            "{}\n{}",
            stdout.join().unwrap_or_default(),
            stderr.join().unwrap_or_default()
        );
        (status.code(), said)
    }

    /// The hook as `main` installs it, but from a thread of its own that ends at once, so the child's test thread
    /// is not the interface thread and no message box waits for a click. `None` in an ordinary run.
    fn install_in_child() -> Option<PathBuf> {
        let logs = PathBuf::from(std::env::var_os(CHILD)?);
        let given = logs.clone();
        std::thread::Builder::new()
            .name("interface".to_string())
            .spawn(move || install_hook(Some(given)))
            .unwrap()
            .join()
            .unwrap();
        Some(logs)
    }

    fn home() -> Option<PathBuf> {
        std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
    }

    #[test]
    fn the_hook_writes_a_report() {
        let root = TempRoot::new("the_hook_writes_a_report");
        let logs = root.0.join("logs");
        let (code, said) = run_child("report::tests::child_panics", &logs);
        // 101 is a test that failed by panicking: the process ended as usual, not aborted inside the hook
        assert_eq!(code, Some(101), "{said}");
        assert!(
            said.contains("panicked at"),
            "the standard line on stderr is kept: {said}"
        );
        let found = reports(&logs);
        assert_eq!(found.len(), 1, "{found:?}\n{said}");
        let text = fs::read_to_string(logs.join(&found[0])).unwrap();
        for wanted in [
            crate::version::version_text().as_str(),
            ON_PURPOSE,
            "report.rs:",
            "stack",
            "%USERPROFILE%",
        ] {
            assert!(text.contains(wanted), "{wanted:?} is missing from:\n{text}");
        }
        if let Some(home) = home() {
            let home = home.to_string_lossy().to_lowercase();
            assert!(
                !text.to_lowercase().contains(&home),
                "the user folder is named:\n{text}"
            );
        }
    }

    #[test]
    fn a_caught_panic_is_recorded_once_and_the_app_goes_on() {
        let root = TempRoot::new("a_caught_panic_is_recorded_once");
        let logs = root.0.join("logs");
        let (code, said) = run_child("report::tests::child_catches", &logs);
        assert_eq!(code, Some(0), "{said}");
        assert!(said.contains("the engine went on"), "{said}");
        let found = reports(&logs);
        assert_eq!(found.len(), 1, "{found:?}\n{said}");
        let text = fs::read_to_string(logs.join(&found[0])).unwrap();
        assert!(text.contains("engine: test worker"), "{text}");
        assert!(text.contains(ON_PURPOSE), "{text}");
    }

    #[test]
    fn the_hook_survives_a_folder_it_cannot_write() {
        let root = TempRoot::new("the_hook_survives_a_folder_it_cannot_write");
        let plain = root.0.join("a plain file");
        fs::write(&plain, "not a folder").unwrap();
        let (code, said) = run_child("report::tests::child_panics", &plain.join("logs"));
        assert_eq!(code, Some(101), "{said}");
        assert!(said.contains("panicked at"), "{said}");
        assert!(!said.contains("while processing panic"), "{said}");
        assert_eq!(fs::read_to_string(&plain).unwrap(), "not a folder");
    }

    /// The child half of [`the_hook_writes_a_report`]: installs the hook and panics on its own test thread, with a
    /// path under the user folder in the message.
    #[test]
    #[ignore = "the child half of the_hook_writes_a_report, which runs it in a process of its own"]
    fn child_panics() {
        if install_in_child().is_none() {
            return;
        }
        let near = home().unwrap_or_default().join("secret folder").join("notes.txt");
        panic!("{ON_PURPOSE} near {}", near.display());
    }

    /// The child half of [`a_caught_panic_is_recorded_once_and_the_app_goes_on`]: a panic on an engine worker,
    /// which the engine catches and turns into a sentence, and the engine goes on.
    #[test]
    #[ignore = "the child half of a_caught_panic_is_recorded_once_and_the_app_goes_on"]
    fn child_catches() {
        use crate::engine::{Ctx, EngineConfig, problem, wait_for};
        use crate::seam::{self, Event};
        let Some(logs) = install_in_child() else { return };
        let (handle, side) = seam::channel(std::sync::Arc::new(|| {}));
        let config = EngineConfig {
            workers: 1,
            paths: crate::store::Paths::under(logs.parent().unwrap()),
            clip: None,
            search_limit: None,
        };
        let ctx = Ctx::new(side.events.clone(), config);
        assert!(ctx.spawn(
            "engine: test worker",
            "testing reports",
            |_| panic!("{ON_PURPOSE}"),
            problem
        ));
        let Event::Problem { text } = wait_for(&handle, 30.0, |e| matches!(e, Event::Problem { .. })) else {
            unreachable!()
        };
        assert_eq!(
            text,
            "Something went wrong while testing reports, so it stopped. Try again."
        );
        // the engine still starts work after the fault
        assert!(ctx.spawn(
            "engine: after",
            "testing reports",
            |ctx| ctx.emit(problem("done".to_string())),
            problem
        ));
        let Event::Problem { text } = wait_for(&handle, 30.0, |_| true) else {
            unreachable!()
        };
        assert_eq!(text, "done");
        println!("the engine went on");
    }
}
