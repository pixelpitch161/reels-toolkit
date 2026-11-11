//! The one way to start a program. Every child program is started directly with its arguments one by one (never
//! pasted into a command line), with no console window, no input, its output piped, its error output read on a
//! thread, and always waited for. `tests/scan.rs` fails on any other `Command` start in this crate.

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
    let name = program
        .file_name()
        .unwrap_or(program.as_os_str())
        .to_string_lossy()
        .into_owned();
    let mut child = quiet(program, args)
        .spawn()
        .map_err(|e| format!("{name} could not be started: {e}."))?;
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
            let took = match limit.as_secs() {
                0 => format!("{} milliseconds", limit.as_millis()),
                1 => "1 second".to_string(),
                s => format!("{s} seconds"),
            };
            break Err(format!("{name} took longer than {took} and was stopped."));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let stdout = out_reader.join().unwrap_or_default();
    let tail = err_reader.join().unwrap_or_default();
    ended.map(|status| Ran { status, stdout, tail })
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
