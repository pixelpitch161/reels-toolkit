//! Finding ffmpeg, and the loop-clean encode of `projects/maze_race/mux.py main()` with the frames piped in.
//!
//! ffmpeg is the program named by the environment variable `BALLFIGHT_FFMPEG`, else the first `ffmpeg.exe` on the
//! `PATH`; ffprobe is the one beside it, else the first on the `PATH`. Before any job it must say it has the
//! `libx264` and `aac` encoders.
//!
//! Every program is started directly with its arguments passed one by one (never through a shell, never pasted
//! into one command line), with its error output read on a thread of its own so its pipe can never fill and
//! block it, and it is always waited for: a program this module started never outlives the call or the value
//! that started it.

use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use bf_picture::Rgb8;
use bf_picture::config::{FPS, H, UP, W};

use crate::ExportError;

/// The environment variable that names the ffmpeg program to use.
pub const ENV: &str = "BALLFIGHT_FFMPEG";

/// How long ffmpeg may take to answer the version and encoder questions.
const ASK_LIMIT: Duration = Duration::from_secs(20);

/// How many of the last lines of a program's error output are kept for its error message.
const TAIL_LINES: usize = 6;

/// The ffmpeg (and ffprobe) a job uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ffmpeg {
    /// The ffmpeg program.
    pub ffmpeg: PathBuf,
    /// The ffprobe program.
    pub ffprobe: PathBuf,
    /// "ffmpeg 9.0.2": the version ffmpeg names in its first line (all of that line if it names none).
    pub version: String,
}

/// The ffmpeg to use: `BALLFIGHT_FFMPEG` if it is set and not empty, else the first on the `PATH`, with ffprobe
/// beside it (else the first on the `PATH`).
///
/// # Errors
/// [`ExportError::Ffmpeg`], in words naming the path: no ffmpeg there (or none on the `PATH`), no ffprobe, a
/// program that does not start or answer like ffmpeg, or one without the `libx264` or `aac` encoder.
pub fn find_ffmpeg() -> Result<Ffmpeg, ExportError> {
    find_with(std::env::var_os(ENV), std::env::var_os("PATH"))
}

/// The file name of program `name` on this system: `name.exe` on Windows.
fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// The first `file` in the folders of `path` (a `PATH` value).
fn on_path(file: &str, path: Option<&OsStr>) -> Option<PathBuf> {
    std::env::split_paths(path?)
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(|dir| dir.join(file))
        .find(|p| p.is_file())
}

/// [`find_ffmpeg`] with the variable and the `PATH` given.
pub(crate) fn find_with(var: Option<OsString>, path: Option<OsString>) -> Result<Ffmpeg, ExportError> {
    let fail = |s: String| ExportError::Ffmpeg(s);
    let ffmpeg = match var.filter(|v| !v.is_empty()) {
        Some(v) => {
            let p = PathBuf::from(v);
            let is_exe = p
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("exe") || !cfg!(windows));
            if p.is_file() && !is_exe {
                return Err(fail(format!(
                    "The {ENV} setting names {}, which is not a program file (.exe), so it cannot be used as ffmpeg; \
                     name ffmpeg.exe itself.",
                    p.display()
                )));
            }
            if !p.is_file() {
                return Err(fail(format!(
                    "ffmpeg was not found at {} (the program the {ENV} setting names), so no video can be made; \
                     install ffmpeg there or correct the setting.",
                    p.display()
                )));
            }
            p
        }
        None => on_path(&exe("ffmpeg"), path.as_deref()).ok_or_else(|| {
            fail(format!(
                "ffmpeg was not found on this PC (it is not on the PATH and {ENV} is not set), so no video can be \
                 made; install ffmpeg, then try again."
            ))
        })?,
    };
    check(ffmpeg, path.as_deref())
}

/// The [`Ffmpeg`] of the program `ffmpeg` (any file that runs, a script too: for tests), with its ffprobe and its
/// version, after asking it for its encoders.
///
/// # Errors
/// As [`find_ffmpeg`].
pub fn ffmpeg_at(ffmpeg: PathBuf) -> Result<Ffmpeg, ExportError> {
    check(ffmpeg, std::env::var_os("PATH").as_deref())
}

fn check(ffmpeg: PathBuf, path: Option<&OsStr>) -> Result<Ffmpeg, ExportError> {
    let fail = |s: String| ExportError::Ffmpeg(s);
    let ffprobe = ffmpeg
        .parent()
        .map(|dir| dir.join(exe("ffprobe")))
        .filter(|p| p.is_file())
        .or_else(|| on_path(&exe("ffprobe"), path))
        .ok_or_else(|| {
            fail(format!(
                "ffprobe was not found beside {} or on the PATH, so a finished video cannot be checked; install \
                 the whole ffmpeg package (it comes with ffprobe), then try again.",
                ffmpeg.display()
            ))
        })?;

    let ask = |what: &str| -> Result<String, ExportError> {
        let never = AtomicBool::new(false);
        let ran = run_tool(&ffmpeg, &["-hide_banner".as_ref(), what.as_ref()], ASK_LIMIT, &never).map_err(|why| {
            fail(format!(
                "The program {} could not be asked for its {} ({why}), so it cannot be used as ffmpeg; install \
                 ffmpeg or correct the {ENV} setting.",
                ffmpeg.display(),
                what.trim_start_matches('-')
            ))
        })?;
        if !ran.status.success() {
            return Err(fail(format!(
                "The program {} did not answer like ffmpeg when asked for its {} ({}), so it cannot be used; \
                 install ffmpeg or correct the {ENV} setting.",
                ffmpeg.display(),
                what.trim_start_matches('-'),
                ran.said()
            )));
        }
        Ok(String::from_utf8_lossy(&ran.stdout).into_owned())
    };
    let first = ask("-version")?.lines().next().unwrap_or("").trim().to_string();
    if !first.starts_with("ffmpeg version") {
        return Err(fail(format!(
            "The program {} does not call itself ffmpeg (it says \"{first}\"), so it cannot be used; install \
             ffmpeg or correct the {ENV} setting.",
            ffmpeg.display()
        )));
    }
    let version = version_of(&first);
    let encoders = ask("-encoders")?;
    for (kind, name) in [('V', "libx264"), ('A', "aac")] {
        if !has_encoder(&encoders, kind, name) {
            return Err(fail(format!(
                "The ffmpeg at {} has no {name} encoder, which the videos need; install a full build of ffmpeg \
                 (one with libx264 and aac), then try again.",
                ffmpeg.display()
            )));
        }
    }
    Ok(Ffmpeg {
        ffmpeg,
        ffprobe,
        version,
    })
}

/// "ffmpeg 9.0.2" from "ffmpeg version 9.0.2-full_build-www.gyan.dev Copyright ...": the version up to its
/// first "-"; the whole line if it has no version word.
fn version_of(first: &str) -> String {
    match first.split_whitespace().nth(2) {
        Some(v) => format!("ffmpeg {}", v.split('-').next().unwrap_or(v)),
        None => first.to_string(),
    }
}

/// Whether `ffmpeg -encoders` lists encoder `name` of kind `kind` (`V` video, `A` audio): a line whose first word
/// starts with the kind and whose second word is the name.
fn has_encoder(list: &str, kind: char, name: &str) -> bool {
    list.lines().any(|line| {
        let mut words = line.split_whitespace();
        matches!((words.next(), words.next()), (Some(flags), Some(n)) if flags.starts_with(kind) && n == name)
    })
}

// ---------------------------------------------------------------- running a program
/// What a program that ran to its end gave.
pub(crate) struct Ran {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    /// The last lines of its error output.
    pub tail: Vec<String>,
}

impl Ran {
    /// How it ended, in words, with its last words if it said any.
    pub fn said(&self) -> String {
        ended(&self.status, &self.tail)
    }
}

/// "exit code 1; it said: \"...\"": how a program ended, with its last lines of error output.
pub(crate) fn ended(status: &ExitStatus, tail: &[String]) -> String {
    let how = match status.code() {
        Some(code) => format!("exit code {code}"),
        None => "it was stopped".to_string(),
    };
    if tail.is_empty() {
        format!("{how}, without a message")
    } else {
        format!("{how}; it said: \"{}\"", tail.join(" / "))
    }
}

/// The last lines of a program's error output, read on a thread of its own until the pipe closes.
pub(crate) struct Tail {
    lines: Arc<Mutex<VecDeque<String>>>,
    thread: Option<JoinHandle<()>>,
}

impl Tail {
    /// Starts reading `pipe`.
    pub fn read(pipe: impl Read + Send + 'static) -> Tail {
        let lines: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        let keep = Arc::clone(&lines);
        let thread = std::thread::spawn(move || {
            let mut pipe = BufReader::new(pipe);
            let mut line = Vec::new();
            loop {
                line.clear();
                match pipe.read_until(b'\n', &mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let text = String::from_utf8_lossy(&line).trim().to_string();
                        if text.is_empty() {
                            continue;
                        }
                        let mut kept = keep.lock().unwrap_or_else(|e| e.into_inner());
                        if kept.len() == TAIL_LINES {
                            kept.pop_front();
                        }
                        kept.push_back(text);
                    }
                }
            }
        });
        Tail {
            lines,
            thread: Some(thread),
        }
    }

    /// Waits for the pipe to close (the program has ended) and gives the lines.
    pub fn finish(mut self) -> Vec<String> {
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        self.lines
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .cloned()
            .collect()
    }
}

/// Starts `program` with `args`, its input empty and its output piped.
pub(crate) fn spawn(program: &Path, args: &[&OsStr], stdin: Stdio) -> std::io::Result<Child> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.spawn()
}

/// Ends `child` (if it still runs) and waits for it, so it leaves nothing behind.
pub(crate) fn end(child: &mut Child) -> Option<ExitStatus> {
    if let Ok(Some(status)) = child.try_wait() {
        return Some(status);
    }
    let _ = child.kill();
    child.wait().ok()
}

/// Runs `program` with `args` to its end, at most `limit` long and only while `cancel` is not set (otherwise it
/// is ended), and gives what it printed.
///
/// # Errors
/// It could not be started, it ran longer than `limit`, or it was cancelled: in words.
pub(crate) fn run_tool(program: &Path, args: &[&OsStr], limit: Duration, cancel: &AtomicBool) -> Result<Ran, String> {
    let mut child = spawn(program, args, Stdio::null()).map_err(|e| format!("it could not be started: {e}"))?;
    let tail = Tail::read(child.stderr.take().expect("piped"));
    let mut out = child.stdout.take().expect("piped");
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = out.read_to_end(&mut bytes);
        bytes
    });
    let t0 = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if t0.elapsed() < limit && !cancel.load(Ordering::SeqCst) => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => {
                end(&mut child);
                break None;
            }
        }
    };
    let stdout = reader.join().unwrap_or_default();
    let tail = tail.finish();
    match status {
        Some(status) => Ok(Ran { status, stdout, tail }),
        None if cancel.load(Ordering::SeqCst) => Err("it was stopped because the render was cancelled".to_string()),
        None => Err(format!("it did not finish within {} s", limit.as_secs())),
    }
}

// ---------------------------------------------------------------- the encode
/// The arguments of the encode: `mux.py main()`'s after `ffmpeg`, with the frames read from the pipe: raw 360 x
/// 640 RGB at 30 fps, the soundtrack `wav` untrimmed, 3x nearest-neighbour upscale, libx264 slow CRF 18 yuv420p
/// high profile, tune animation, `mbtree=0`, keyframes forced on the first and the last frame (`nf` frames), AAC
/// 256k at 48 kHz, `+faststart`.
///
/// Two stated departures from the kit, both about the container only (no sample or pixel changes):
/// `-max_interleave_delta 0` before `-movflags`, so the muxer interleaves the two streams by their timestamps
/// alone and the file's bytes do not depend on when the frames arrive (ruling R42); and `-f mp4` before the
/// output, which is written as `<name>.mp4.part` and renamed once it is verified.
pub fn encode_args(wav: &Path, out: &Path, nf: usize) -> Vec<OsString> {
    let mut a: Vec<OsString> = Vec::new();
    let mut put = |s: &str| a.push(s.into());
    for s in ["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s"] {
        put(s);
    }
    put(&format!("{W}x{H}"));
    put("-r");
    put(&FPS.to_string());
    for s in ["-i", "pipe:0", "-i"] {
        put(s);
    }
    a.push(wav.as_os_str().to_os_string());
    let mut put = |s: &str| a.push(s.into());
    put("-vf");
    put(&format!("scale={}:{}:flags=neighbor", W * UP, H * UP));
    for s in [
        "-c:v",
        "libx264",
        "-preset",
        "slow",
        "-crf",
        "18",
        "-pix_fmt",
        "yuv420p",
        "-profile:v",
        "high",
        "-tune",
        "animation",
        "-x264-params",
        "mbtree=0",
        "-force_key_frames",
    ] {
        put(s);
    }
    put(&format!("expr:eq(n,0)+eq(n,{})", nf.saturating_sub(1)));
    for s in [
        "-c:a",
        "aac",
        "-b:a",
        "256k",
        "-ar",
        "48000",
        "-max_interleave_delta",
        "0",
        "-movflags",
        "+faststart",
        "-f",
        "mp4",
    ] {
        put(s);
    }
    a.push(out.as_os_str().to_os_string());
    a
}

/// A running encode: ffmpeg, with the frames going in through its input pipe.
///
/// Dropping an encoder that has not finished ends ffmpeg and waits for it; the half-written file is then the
/// caller's to remove.
pub struct Encoder {
    child: Child,
    stdin: Option<ChildStdin>,
    tail: Option<Tail>,
    nf: usize,
    pushed: usize,
    out: PathBuf,
    done: bool,
}

impl Encoder {
    /// Starts ffmpeg encoding `nf` frames and the soundtrack `wav` into `out`.
    ///
    /// # Errors
    /// [`ExportError::Ffmpeg`]: ffmpeg could not be started.
    pub fn start(ff: &Ffmpeg, wav: &Path, out: &Path, nf: usize) -> Result<Encoder, ExportError> {
        let args = encode_args(wav, out, nf);
        let args: Vec<&OsStr> = args.iter().map(OsString::as_os_str).collect();
        let mut child = spawn(&ff.ffmpeg, &args, Stdio::piped()).map_err(|e| {
            ExportError::Ffmpeg(format!(
                "ffmpeg at {} could not be started ({e}), so the video was not made; install ffmpeg again, then \
                 try again.",
                ff.ffmpeg.display()
            ))
        })?;
        let stdin = child.stdin.take();
        // nothing is printed on the output with these arguments; drain it all the same
        let stdout = child.stdout.take().expect("piped");
        std::thread::spawn(move || {
            let _ = std::io::copy(&mut { stdout }, &mut std::io::sink());
        });
        let tail = Tail::read(child.stderr.take().expect("piped"));
        Ok(Encoder {
            child,
            stdin,
            tail: Some(tail),
            nf,
            pushed: 0,
            out: out.to_path_buf(),
            done: false,
        })
    }

    /// The frames given so far.
    pub fn pushed(&self) -> usize {
        self.pushed
    }

    /// Gives ffmpeg the next frame (360 x 640 RGB).
    ///
    /// # Errors
    /// [`ExportError::Encode`]: the frame is not 360 x 640 RGB, all `nf` frames were already given, or ffmpeg
    /// stopped (its exit and last words are in the sentence; it has then been waited for).
    pub fn push(&mut self, frame: &Rgb8) -> Result<(), ExportError> {
        if (frame.w, frame.h, frame.c) != (W, H, 3) || frame.data.len() != W * H * 3 {
            return Err(ExportError::Encode(format!(
                "Frame {} is {} x {} with {} channels where the encoder takes {W} x {H} RGB, so the video was not \
                 made; this is a fault in the app, not in your fight.",
                self.pushed, frame.w, frame.h, frame.c
            )));
        }
        if self.pushed >= self.nf {
            return Err(ExportError::Encode(format!(
                "The encoder was given more than the {} frames it was started for, so the video was not made; this \
                 is a fault in the app, not in your fight.",
                self.nf
            )));
        }
        let wrote = match self.stdin.as_mut() {
            Some(pipe) => pipe.write_all(&frame.data),
            None => Err(std::io::ErrorKind::BrokenPipe.into()),
        };
        if let Err(e) = wrote {
            return Err(self.stopped(&format!("at frame {} of {}", self.pushed, self.nf), Some(&e)));
        }
        self.pushed += 1;
        Ok(())
    }

    /// Ends ffmpeg and gives the sentence for an encode that stopped `when`.
    fn stopped(&mut self, when: &str, e: Option<&std::io::Error>) -> ExportError {
        self.stdin = None;
        self.done = true;
        let status = end(&mut self.child);
        let tail = self.tail.take().map(Tail::finish).unwrap_or_default();
        let how = match status {
            Some(status) => ended(&status, &tail),
            None => match e {
                Some(e) => format!("the frames could not be given to it ({e})"),
                None => "it could not be waited for".to_string(),
            },
        };
        ExportError::Encode(format!(
            "ffmpeg stopped while it encoded the video ({when}: {how}), so nothing was saved; check that the drive \
             has free space, then try again."
        ))
    }

    /// Closes the input and waits for ffmpeg to write the rest of the file.
    ///
    /// # Errors
    /// [`ExportError::Encode`]: fewer than `nf` frames were given, ffmpeg failed (exit and last words in the
    /// sentence), or it left no file.
    pub fn finish(mut self) -> Result<(), ExportError> {
        if self.pushed != self.nf {
            let (pushed, nf) = (self.pushed, self.nf);
            self.abort();
            return Err(ExportError::Encode(format!(
                "Only {pushed} of the {nf} frames were given to the encoder, so the video was not made; this is a \
                 fault in the app, not in your fight."
            )));
        }
        self.stdin = None; // closes the pipe: ffmpeg finishes the file
        self.done = true;
        let status = self.child.wait();
        let tail = self.tail.take().map(Tail::finish).unwrap_or_default();
        match status {
            Ok(status) if status.success() => {}
            Ok(status) => {
                return Err(ExportError::Encode(format!(
                    "ffmpeg failed when it finished the video ({}), so nothing was saved; check that the drive has \
                     free space, then try again.",
                    ended(&status, &tail)
                )));
            }
            Err(e) => {
                return Err(ExportError::Encode(format!(
                    "ffmpeg could not be waited for when it finished the video ({e}), so nothing was saved; try \
                     again."
                )));
            }
        }
        match std::fs::metadata(&self.out) {
            Ok(m) if m.is_file() && m.len() > 0 => Ok(()),
            _ => Err(ExportError::Encode(format!(
                "ffmpeg ended without writing the video {}, so nothing was saved; try again.",
                self.out.display()
            ))),
        }
    }

    /// Ends ffmpeg at once and waits for it (a cancel). The half-written file is the caller's to remove.
    pub fn abort(mut self) {
        self.shut();
    }

    fn shut(&mut self) {
        if !self.done {
            self.stdin = None;
            self.done = true;
            end(&mut self.child);
            if let Some(t) = self.tail.take() {
                t.finish();
            }
        }
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        self.shut();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arguments_are_the_kits() {
        // mux.py's arguments token by token, with two stated departures that touch the container only:
        // `-max_interleave_delta 0` (ruling R42: the bytes do not depend on timing) and `-f mp4` (the output is
        // written as `.mp4.part`)
        let args = encode_args(Path::new(r"C:\a b!\audio.wav"), Path::new(r"C:\a b!\x.mp4.part"), 1830);
        let args: Vec<String> = args.iter().map(|a| a.to_string_lossy().into_owned()).collect();
        let want = [
            r"-y -loglevel error -f rawvideo -pix_fmt rgb24 -s 360x640 -r 30 -i pipe:0 -i C:\a b!\audio.wav",
            "-vf scale=1080:1920:flags=neighbor -c:v libx264 -preset slow -crf 18 -pix_fmt yuv420p",
            "-profile:v high -tune animation -x264-params mbtree=0 -force_key_frames",
            "expr:eq(n,0)+eq(n,1829) -c:a aac -b:a 256k -ar 48000 -max_interleave_delta 0 -movflags +faststart",
            r"-f mp4 C:\a b!\x.mp4.part",
        ]
        .join(" ");
        // the two paths are single arguments with their spaces
        assert_eq!(args[14], r"C:\a b!\audio.wav");
        assert_eq!(args.last().map(String::as_str), Some(r"C:\a b!\x.mp4.part"));
        assert_eq!(args.join(" "), want);
        assert_eq!(args.len(), 46);
    }

    #[test]
    fn only_a_program_file_is_taken_from_the_setting() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join(format!("bf-export not ffmpeg {}.cmd", std::process::id()));
        std::fs::write(&script, "@echo off\r\n").unwrap();
        let err = find_with(Some(script.clone().into_os_string()), None).unwrap_err();
        std::fs::remove_file(&script).unwrap();
        assert!(matches!(err, ExportError::Ffmpeg(_)));
        assert!(
            err.to_string().contains(".exe") && err.to_string().contains("not ffmpeg"),
            "{err}"
        );
    }

    #[test]
    fn versions_and_encoders_are_read() {
        assert_eq!(
            version_of("ffmpeg version 9.0.2-full_build-www.gyan.dev Copyright (c) 2000-2026"),
            "ffmpeg 9.0.2"
        );
        assert_eq!(version_of("ffmpeg version n7.1 Copyright"), "ffmpeg n7.1");
        let list = " V....D libx264              libx264 H.264\n V....D libx264rgb  x\n A....D aac   AAC\n";
        assert!(has_encoder(list, 'V', "libx264"));
        assert!(has_encoder(list, 'A', "aac"));
        assert!(!has_encoder(list, 'A', "libx264"));
        assert!(!has_encoder(" V....D libx264rgb x\n", 'V', "libx264"));
    }

    #[test]
    fn no_ffmpeg_on_the_path_is_named() {
        let empty = Path::new(env!("CARGO_MANIFEST_DIR")).join("no such folder 7f3a");
        let err = find_with(None, Some(empty.into_os_string())).unwrap_err();
        assert!(matches!(err, ExportError::Ffmpeg(_)));
        assert!(err.to_string().contains("PATH"), "{err}");
        let err = find_with(Some("C:\\nowhere\\ffmpeg.exe".into()), None).unwrap_err();
        assert!(err.to_string().contains("C:\\nowhere\\ffmpeg.exe"), "{err}");
    }
}
