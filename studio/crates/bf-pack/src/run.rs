//! The one way to start a program. Every child program is started directly with its arguments one by one (never
//! pasted into a command line), with no console window, no input, its output piped, its error output read on a
//! thread, and always waited for (`run`, `run_with_env`, `run_family`) or handed off on purpose (`start_detached`).
//! `tests/scan.rs` fails on any other `Command` start in this crate.

use std::collections::VecDeque;
use std::ffi::OsStr;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Windows' `CREATE_NO_WINDOW` process flag. `quiet` sets it; a caller that sets more flags with `creation_flags`
/// (which replaces them) must include it again.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// How many of the last error-output lines `run` keeps.
pub const TAIL_LINES: usize = 20;

/// What a finished program left: how it ended, everything it printed, and the last lines of its error output.
#[derive(Debug)]
pub struct Ran {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub tail: Vec<String>,
}

/// A program ready to start: no console window, input from nothing, output and error output piped.
pub fn quiet(program: &Path, args: &[&OsStr]) -> Command {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Starts `program` with `args`, waits for it, and gives what it left. It is stopped (and this is an error) when it
/// runs longer than `limit` or when `cancel` turns true.
pub fn run(program: &Path, args: &[&OsStr], limit: Duration, cancel: &AtomicBool) -> Result<Ran, String> {
    run_with_env(program, args, &[], limit, cancel)
}

/// `run`, with these environment variables added for the program (each a name and its value; everything else it
/// inherits as `run` does).
pub fn run_with_env(
    program: &Path,
    args: &[&OsStr],
    env: &[(&str, &OsStr)],
    limit: Duration,
    cancel: &AtomicBool,
) -> Result<Ran, String> {
    let name = program
        .file_name()
        .unwrap_or(program.as_os_str())
        .to_string_lossy()
        .into_owned();
    let mut cmd = quiet(program, args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().map_err(|e| format!("{name} could not be started: {e}."))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut out) = stdout {
            let _ = out.read_to_end(&mut bytes);
        }
        bytes
    });
    let err_reader = std::thread::spawn(move || {
        let mut tail = VecDeque::with_capacity(TAIL_LINES);
        if let Some(err) = stderr {
            let mut reader = BufReader::new(err);
            let mut line = Vec::new();
            loop {
                line.clear();
                match reader.read_until(b'\n', &mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                let text = String::from_utf8_lossy(&line).trim_end().to_string();
                if text.is_empty() {
                    continue;
                }
                if tail.len() == TAIL_LINES {
                    tail.pop_front();
                }
                tail.push_back(text);
            }
        }
        Vec::from(tail)
    });

    let started = Instant::now();
    let ended = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => {}
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(format!("{name} could not be waited for: {e}."));
            }
        }
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            break Err(format!("{name} was stopped."));
        }
        if started.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            break Err(format!(
                "{name} took longer than {} and was stopped.",
                took_words(limit)
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let stdout = out_reader.join().unwrap_or_default();
    let tail = err_reader.join().unwrap_or_default();
    ended.map(|status| Ran { status, stdout, tail })
}

/// A time limit in words: "300 milliseconds", "1 second", "20 seconds".
fn took_words(limit: Duration) -> String {
    match limit.as_secs() {
        0 => format!("{} milliseconds", limit.as_millis()),
        1 => "1 second".to_string(),
        s => format!("{s} seconds"),
    }
}

/// How `run_family` starts its program.
#[derive(Clone, Copy, Default)]
pub struct Family<'a> {
    /// The folder it runs in (otherwise this program's own).
    pub dir: Option<&'a Path>,
    /// Environment variables added for it, each a name and its value (after the ones taken away).
    pub env: &'a [(&'a str, &'a OsStr)],
    /// Environment variables taken away for it.
    pub env_remove: &'a [&'a str],
    /// Whether every variable whose name starts with `GIT_` is taken away for it (before `env` is added), so that
    /// nothing in this program's own environment can point git at another repository, index or settings.
    pub scrub_git: bool,
    /// Whether it, and every program it starts, runs at Windows' below-normal priority.
    pub below_normal: bool,
}

/// Windows' `BELOW_NORMAL_PRIORITY_CLASS` process flag.
pub const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
/// Windows' `CREATE_SUSPENDED` process flag: the program's first thread waits until it is resumed.
const CREATE_SUSPENDED: u32 = 0x0000_0004;
/// Windows' `CREATE_BREAKAWAY_FROM_JOB` process flag.
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;

/// The names of every variable in this program's environment that starts with `GIT_` (as Windows compares names).
pub fn git_variables() -> Vec<std::ffi::OsString> {
    std::env::vars_os()
        .map(|(k, _)| k)
        .filter(|k| k.to_string_lossy().to_ascii_uppercase().starts_with("GIT_"))
        .collect()
}

#[cfg(test)]
thread_local! {
    /// For a test only: wait this long between starting a program and putting it in its job, so a test can show
    /// that a program cannot start anything of its own before it is held.
    static HOLD_DELAY_MS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// `run` for a program that starts programs of its own (the update build: cargo, through rustup's proxy, starts the
/// real cargo, rustc and build scripts). On Windows the program is started suspended, put in a job of its own, and
/// only then let run, so every program it starts joins that job: when it is stopped (by `cancel` or the time limit)
/// the whole family is stopped at once, and so is anything of the family still running when it ends or when this
/// program ends. If Windows gives no job, or the program cannot be put in it, the program is ended before it ran and
/// this is an error in words. Each line of its error output goes to `on_line` as it comes (on the calling thread).
/// No shell is involved at any point.
pub fn run_family(
    program: &Path,
    args: &[&OsStr],
    how: &Family,
    limit: Duration,
    cancel: &AtomicBool,
    on_line: &mut dyn FnMut(&str),
) -> Result<Ran, String> {
    let name = program
        .file_name()
        .unwrap_or(program.as_os_str())
        .to_string_lossy()
        .into_owned();
    let mut cmd = quiet(program, args);
    if how.scrub_git {
        for k in git_variables() {
            cmd.env_remove(k);
        }
    }
    for k in how.env_remove {
        cmd.env_remove(k);
    }
    for (k, v) in how.env {
        cmd.env(k, v);
    }
    if let Some(dir) = how.dir {
        cmd.current_dir(dir);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // `creation_flags` replaces what `quiet` set, so no-window is given again.
        let priority = if how.below_normal {
            BELOW_NORMAL_PRIORITY_CLASS
        } else {
            0
        };
        cmd.creation_flags(CREATE_SUSPENDED | CREATE_NO_WINDOW | priority);
    }
    let not_held = || format!("{name} could not be held so that it can be stopped, so it was not run.");
    let job = family::Job::new(how.below_normal);
    let mut child = cmd.spawn().map_err(|e| format!("{name} could not be started: {e}."))?;
    #[cfg(test)]
    std::thread::sleep(Duration::from_millis(HOLD_DELAY_MS.with(|d| d.get())));
    let held = job.as_ref().is_some_and(|j| j.hold(&child)) && family::resume(child.id());
    if !held {
        if let Some(job) = &job {
            job.stop();
        }
        let _ = child.kill();
        let _ = child.wait();
        return Err(not_held());
    }
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut out) = stdout {
            let _ = out.read_to_end(&mut bytes);
        }
        bytes
    });
    let (lines_tx, lines) = std::sync::mpsc::channel::<String>();
    let err_reader = std::thread::spawn(move || {
        if let Some(err) = stderr {
            let mut reader = BufReader::new(err);
            let mut line = Vec::new();
            loop {
                line.clear();
                match reader.read_until(b'\n', &mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                let text = String::from_utf8_lossy(&line).trim_end().to_string();
                if !text.is_empty() && lines_tx.send(text).is_err() {
                    break;
                }
            }
        }
    });

    let mut tail: VecDeque<String> = VecDeque::with_capacity(TAIL_LINES);
    let mut take_lines = |tail: &mut VecDeque<String>| {
        for text in lines.try_iter() {
            on_line(&text);
            if tail.len() == TAIL_LINES {
                tail.pop_front();
            }
            tail.push_back(text);
        }
    };
    let stop = |child: &mut std::process::Child| {
        if let Some(job) = &job {
            job.stop();
        }
        let _ = child.kill();
        let _ = child.wait();
    };
    let started = Instant::now();
    let ended = loop {
        take_lines(&mut tail);
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => {}
            Err(e) => {
                stop(&mut child);
                break Err(format!("{name} could not be waited for: {e}."));
            }
        }
        if cancel.load(Ordering::Relaxed) {
            stop(&mut child);
            break Err(format!("{name} was stopped."));
        }
        if started.elapsed() > limit {
            stop(&mut child);
            break Err(format!(
                "{name} took longer than {} and was stopped.",
                took_words(limit)
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    // Anything of the family still running ends with the job, so its pipes close and the readers finish.
    if let Some(job) = &job {
        job.stop();
    }
    drop(job);
    let stdout = out_reader.join().unwrap_or_default();
    let _ = err_reader.join();
    take_lines(&mut tail);
    ended.map(|status| Ran {
        status,
        stdout,
        tail: Vec::from(tail),
    })
}

/// Waits up to `limit` for the program with process id `pid` to end. True when it has ended (or no such program is
/// running); false when it still runs after `limit`.
pub fn wait_for_exit(pid: u32, limit: Duration) -> bool {
    family::wait_for_exit(pid, limit)
}

/// Starts `program` with `args` in the folder `dir` and hands it off on purpose: it is not waited for and keeps
/// running after this program ends (the app's own exe, started again after an update; or `explorer.exe` opening a
/// file). No console window, and no input or output is connected to it. It is started outside any job this program
/// is in when Windows allows that (so closing this program's job does not end it), else inside it. Gives its id.
pub fn start_detached(program: &Path, args: &[&OsStr], dir: &Path) -> Result<u32, String> {
    let name = program
        .file_name()
        .unwrap_or(program.as_os_str())
        .to_string_lossy()
        .into_owned();
    let make = |breakaway: bool| {
        let mut cmd = quiet(program, args);
        cmd.stdout(Stdio::null()).stderr(Stdio::null()).current_dir(dir);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let extra = if breakaway { CREATE_BREAKAWAY_FROM_JOB } else { 0 };
            cmd.creation_flags(CREATE_NO_WINDOW | extra);
        }
        #[cfg(not(windows))]
        let _ = breakaway;
        cmd
    };
    let child = make(true)
        .spawn()
        .or_else(|_| make(false).spawn())
        .map_err(|e| format!("{name} could not be started: {e}."))?;
    let pid = child.id();
    // Dropping the handle leaves the program running.
    drop(child);
    Ok(pid)
}

/// The Windows calls behind `run_family` and `wait_for_exit`; plain fallbacks elsewhere.
mod family {
    use std::time::Duration;

    /// A Windows job: every program in it is stopped when it is stopped or closed.
    #[cfg(windows)]
    pub(super) struct Job(windows_sys::Win32::Foundation::HANDLE);

    #[cfg(windows)]
    impl Job {
        /// A new job that stops its programs when it is closed, and holds them at below-normal priority when asked.
        pub(super) fn new(below_normal: bool) -> Option<Job> {
            use windows_sys::Win32::System::JobObjects::{
                CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PRIORITY_CLASS,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation, SetInformationJobObject,
            };
            // SAFETY: both arguments may be null (default security, no name); the handle is checked and owned by the
            // `Job`, which closes it once.
            let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if handle.is_null() {
                return None;
            }
            let job = Job(handle);
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if below_normal {
                info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_PRIORITY_CLASS;
                info.BasicLimitInformation.PriorityClass = super::BELOW_NORMAL_PRIORITY_CLASS;
            }
            // SAFETY: the handle is a live job; `info` is a whole JOBOBJECT_EXTENDED_LIMIT_INFORMATION that lives for
            // the call, and its size is given exactly.
            let set = unsafe {
                SetInformationJobObject(
                    job.0,
                    JobObjectExtendedLimitInformation,
                    (&raw const info).cast(),
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };
            (set != 0).then_some(job)
        }

        /// Puts a started program in the job; the programs it starts from then on join it too.
        pub(super) fn hold(&self, child: &std::process::Child) -> bool {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;
            // SAFETY: both handles are live: the job is owned by `self`, the process handle by `child`, which
            // outlives the call.
            unsafe { AssignProcessToJobObject(self.0, child.as_raw_handle().cast()) != 0 }
        }

        /// Stops every program in the job.
        pub(super) fn stop(&self) {
            use windows_sys::Win32::System::JobObjects::TerminateJobObject;
            // SAFETY: the job handle is live (owned by `self`).
            unsafe {
                TerminateJobObject(self.0, 1);
            }
        }
    }

    #[cfg(windows)]
    impl Drop for Job {
        fn drop(&mut self) {
            // SAFETY: the handle came from CreateJobObjectW and is closed exactly once, here.
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.0);
            }
        }
    }

    /// Lets every thread of the suspended program `pid` run (a program started suspended has exactly one). True when
    /// at least one was resumed.
    #[cfg(windows)]
    pub(super) fn resume(pid: u32) -> bool {
        use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
        };
        use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};
        // SAFETY: a snapshot of the threads of every process; the handle is checked and closed once at the end.
        let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
        if snap == INVALID_HANDLE_VALUE || snap.is_null() {
            return false;
        }
        let mut entry = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        let mut resumed = false;
        // SAFETY: `entry` is a whole THREADENTRY32 with its size set, as both calls require; the snapshot is live.
        let mut more = unsafe { Thread32First(snap, &raw mut entry) } != 0;
        while more {
            if entry.th32OwnerProcessID == pid {
                // SAFETY: plain values in; the thread handle is checked, used once and closed once.
                unsafe {
                    let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                    if !thread.is_null() {
                        resumed |= ResumeThread(thread) != u32::MAX;
                        CloseHandle(thread);
                    }
                }
            }
            // SAFETY: as for Thread32First.
            more = unsafe { Thread32Next(snap, &raw mut entry) } != 0;
        }
        // SAFETY: the snapshot handle is closed exactly once, here.
        unsafe {
            CloseHandle(snap);
        }
        resumed
    }

    #[cfg(windows)]
    pub(super) fn wait_for_exit(pid: u32, limit: Duration) -> bool {
        use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
        use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
        // SAFETY: OpenProcess takes plain values; a null result means there is no such program to wait for.
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        if handle.is_null() {
            return true;
        }
        let ms = u32::try_from(limit.as_millis())
            .unwrap_or(u32::MAX - 1)
            .min(u32::MAX - 1);
        // SAFETY: the handle is live for the wait and is closed right after it, exactly once.
        let waited = unsafe {
            let waited = WaitForSingleObject(handle, ms);
            CloseHandle(handle);
            waited
        };
        waited == WAIT_OBJECT_0
    }

    // Elsewhere there are no jobs and nothing is started suspended: the program is held as it is.
    #[cfg(not(windows))]
    pub(super) struct Job;

    #[cfg(not(windows))]
    impl Job {
        pub(super) fn new(_below_normal: bool) -> Option<Job> {
            Some(Job)
        }
        pub(super) fn hold(&self, _child: &std::process::Child) -> bool {
            true
        }
        pub(super) fn stop(&self) {}
    }

    #[cfg(not(windows))]
    pub(super) fn resume(_pid: u32) -> bool {
        true
    }

    #[cfg(not(windows))]
    pub(super) fn wait_for_exit(_pid: u32, _limit: Duration) -> bool {
        true
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::time::Instant;

    fn system32(exe: &str) -> PathBuf {
        let root = std::env::var_os("SystemRoot").expect("SystemRoot is set on Windows");
        PathBuf::from(root).join("System32").join(exe)
    }

    fn os(args: &[&'static str]) -> Vec<&'static OsStr> {
        args.iter().map(|a| OsStr::new(*a)).collect()
    }

    #[test]
    fn a_program_runs_and_its_output_comes_back() {
        let never = AtomicBool::new(false);
        let ran = run(
            &system32("curl.exe"),
            &os(&["--version"]),
            Duration::from_secs(20),
            &never,
        )
        .unwrap();
        assert!(ran.status.success());
        assert!(String::from_utf8_lossy(&ran.stdout).starts_with("curl "));
    }

    #[test]
    fn error_output_keeps_its_last_lines() {
        let never = AtomicBool::new(false);
        // curl names an option it does not know on its error output and ends with an error code.
        let ran = run(
            &system32("curl.exe"),
            &os(&["--no-such-option"]),
            Duration::from_secs(20),
            &never,
        )
        .unwrap();
        assert!(!ran.status.success());
        assert!(!ran.tail.is_empty() && ran.tail.len() <= TAIL_LINES);
        assert!(ran.tail.iter().any(|l| l.contains("no-such-option")), "{:?}", ran.tail);
    }

    #[test]
    fn a_program_that_runs_too_long_is_stopped() {
        let never = AtomicBool::new(false);
        let started = Instant::now();
        // Ten pings of this PC itself, one a second: about 9 s unless stopped.
        let err = run(
            &system32("PING.EXE"),
            &os(&["-n", "10", "127.0.0.1"]),
            Duration::from_millis(300),
            &never,
        )
        .unwrap_err();
        assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
        assert!(err.contains("took longer than"), "{err}");
    }

    #[test]
    fn cancel_stops_a_program() {
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let setter = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            flag.store(true, Ordering::Relaxed);
        });
        let started = Instant::now();
        let err = run(
            &system32("PING.EXE"),
            &os(&["-n", "10", "127.0.0.1"]),
            Duration::from_secs(60),
            &cancel,
        )
        .unwrap_err();
        setter.join().unwrap();
        assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
        assert!(err.contains("was stopped"), "{err}");
    }

    #[test]
    fn added_environment_reaches_the_program() {
        let never = AtomicBool::new(false);
        // cmd's `set NAME` prints that variable; the value comes in the environment, not on the command line.
        let ran = run_with_env(
            &system32("cmd.exe"),
            &os(&["/d", "/c", "set", "BF_RUN_TEST_VALUE"]),
            &[("BF_RUN_TEST_VALUE", OsStr::new("given"))],
            Duration::from_secs(20),
            &never,
        )
        .unwrap();
        assert!(ran.status.success());
        assert!(String::from_utf8_lossy(&ran.stdout).contains("BF_RUN_TEST_VALUE=given"));
    }

    #[test]
    fn a_familys_error_lines_come_as_they_are_printed() {
        let never = AtomicBool::new(false);
        let mut seen = Vec::new();
        let ran = run_family(
            &system32("cmd.exe"),
            &os(&["/d", "/c", "echo first 1>&2 && echo second 1>&2 && echo out"]),
            &Family::default(),
            Duration::from_secs(20),
            &never,
            &mut |l| seen.push(l.to_string()),
        )
        .unwrap();
        assert!(ran.status.success());
        assert_eq!(seen, ["first", "second"]);
        assert_eq!(ran.tail, ["first", "second"]);
        assert!(String::from_utf8_lossy(&ran.stdout).contains("out"));
    }

    #[test]
    fn a_family_gets_its_folder_and_environment() {
        let never = AtomicBool::new(false);
        let dir = std::env::temp_dir();
        // The variables change for the child only; this process's own environment is never touched.
        let ran = run_family(
            &system32("cmd.exe"),
            &os(&["/d", "/c", "cd & set BF_FAMILY_ADDED & set USERNAME"]),
            &Family {
                dir: Some(&dir),
                env: &[("BF_FAMILY_ADDED", OsStr::new("yes"))],
                env_remove: &["USERNAME"],
                scrub_git: false,
                below_normal: false,
            },
            Duration::from_secs(20),
            &never,
            &mut |_| {},
        )
        .unwrap();
        let out = String::from_utf8_lossy(&ran.stdout).to_string();
        assert!(out.contains("BF_FAMILY_ADDED=yes"), "{out}");
        assert!(!out.contains("USERNAME="), "{out}");
        let first = out.lines().next().unwrap_or("").trim().to_lowercase();
        let want = dir.to_string_lossy().trim_end_matches('\\').to_lowercase();
        assert_eq!(first, want);
    }

    fn powershell() -> PathBuf {
        system32(r"WindowsPowerShell\v1.0\powershell.exe")
    }

    #[test]
    fn a_family_runs_below_normal_priority_all_the_way_down() {
        let never = AtomicBool::new(false);
        let ps = powershell().to_string_lossy().into_owned();
        // cmd starts PowerShell, which reports its own priority: the grandchild is held below normal too.
        let line = format!("{ps} -NoProfile -NonInteractive -Command (Get-Process -Id $PID).PriorityClass");
        let ran = run_family(
            &system32("cmd.exe"),
            &[OsStr::new("/d"), OsStr::new("/c"), OsStr::new(&line)],
            &Family {
                below_normal: true,
                ..Family::default()
            },
            Duration::from_secs(60),
            &never,
            &mut |_| {},
        )
        .unwrap();
        let out = String::from_utf8_lossy(&ran.stdout).to_string();
        assert_eq!(out.trim(), "BelowNormal", "{out}");
    }

    #[test]
    fn stopping_a_family_stops_the_programs_it_started() {
        let dir = std::env::temp_dir().join(format!("bf-pack-family-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("pings.txt");
        // cmd starts ping, which writes into pings.txt (held open) once a second for about 30 s.
        let line = "PING.EXE -n 30 127.0.0.1 > pings.txt";
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let watched = out.clone();
        let stopper = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(20);
            while Instant::now() < deadline {
                let lines = std::fs::read_to_string(&watched)
                    .map(|t| t.lines().count())
                    .unwrap_or(0);
                if lines >= 3 {
                    // While ping runs, the file it writes cannot be removed.
                    let held = std::fs::remove_file(&watched).is_err();
                    flag.store(true, Ordering::Relaxed);
                    return held;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            flag.store(true, Ordering::Relaxed);
            false
        });
        let started = Instant::now();
        // As rustup's cargo proxy does, cmd starts its own program at once. With a pause between starting cmd and
        // putting it in the job, a cmd that was already running would have started ping outside the job by then;
        // started suspended, it cannot.
        HOLD_DELAY_MS.with(|d| d.set(1500));
        let err = run_family(
            &system32("cmd.exe"),
            &[OsStr::new("/d"), OsStr::new("/c"), OsStr::new(line)],
            &Family {
                dir: Some(&dir),
                ..Family::default()
            },
            Duration::from_secs(60),
            &cancel,
            &mut |_| {},
        )
        .unwrap_err();
        HOLD_DELAY_MS.with(|d| d.set(0));
        assert!(err.contains("was stopped"), "{err}");
        assert!(stopper.join().unwrap(), "ping held its file while it ran");
        assert!(started.elapsed() < Duration::from_secs(25));
        // Within 2 s ping is gone too: its file can be removed.
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut removed = false;
        while Instant::now() < deadline {
            if std::fs::remove_file(&out).is_ok() || !out.exists() {
                removed = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert!(removed, "ping was still running after its family was stopped");
    }

    #[test]
    fn a_started_program_runs_on_and_can_be_waited_for() {
        let dir = std::env::temp_dir();
        let pid = start_detached(&system32("PING.EXE"), &os(&["-n", "3", "127.0.0.1"]), &dir).unwrap();
        // About 2 s: still running after 100 ms, over within 10 s.
        assert!(!wait_for_exit(pid, Duration::from_millis(100)));
        let started = Instant::now();
        assert!(wait_for_exit(pid, Duration::from_secs(10)));
        assert!(started.elapsed() < Duration::from_secs(9));
        // A program that is not there has nothing to wait for.
        assert!(wait_for_exit(pid, Duration::from_millis(10)));
        let err = start_detached(Path::new(r"C:\no\such\program.exe"), &[], &dir).unwrap_err();
        assert!(err.contains("could not be started"), "{err}");
    }

    #[test]
    fn a_missing_program_says_so() {
        let never = AtomicBool::new(false);
        let err = run(
            Path::new(r"C:\no\such\program.exe"),
            &[],
            Duration::from_secs(5),
            &never,
        )
        .unwrap_err();
        assert!(err.contains("could not be started"), "{err}");
    }
}
