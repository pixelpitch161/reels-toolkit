//! The render job end to end (Review Focus 3): ffmpeg missing or failing, an output folder that cannot be written,
//! a name already taken, a cancel, and short clips of the example episode (FLUTE against TROMBONE, seed 392)
//! encoded for real. A clip is never named or filled like a finished video (ruling R41); the plain name and the
//! seed suffix of a whole video are pinned by `full_render_passes_every_check` and `folder_name`.
//!
//! Every test writes only under its own folder in Cargo's temporary folder for tests (`CARGO_TARGET_TMPDIR`, under
//! `studio/target`), named with spaces, a "!" and letters outside ASCII so such paths are exercised. The tests that
//! need ffmpeg print a notice and pass when it is not installed (all but `missing_ffmpeg_is_a_clear_error`).
//! `BALLFIGHT_FFMPEG` belongs to the whole process, so every test holds one lock while it runs.
//!
//! `full_render_passes_every_check` is the whole 61 s video with every check; it takes about a minute and a half
//! in release, so it is ignored: run it by name with
//! `cargo test --release -p bf-export --test job -- --ignored full_render --nocapture`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_export::{
    ExportError, JobEvent, JobOpts, clip_folder_name, ffmpeg_at, find_ffmpeg, folder_name, run_job, run_job_using,
    stage,
};
use bf_testdata::{reference, repo_root};

// ---------------------------------------------------------------- shared set-up
/// Held by every test: `BALLFIGHT_FFMPEG` is the process's, and the encodes are heavy enough to run one at a time.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

struct Episode {
    rec: Arc<Recording>,
    ff: FightFile,
}

static EPISODE: LazyLock<Episode> = LazyLock::new(|| {
    let rec = Recording::read(&reference("fights/flute_vs_trombone/ep392.bfrec")).expect("the recording reads");
    let path = repo_root().join("studio/fights/flute_vs_trombone.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    Episode {
        rec: Arc::new(rec),
        ff: FightFile::parse(&text).expect("the fight file parses"),
    }
});

/// A folder of this test's own, removed when dropped (only this folder, made by the test itself).
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(test: &str) -> TempRoot {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("bf-export job! Zoë Ångström {test} {}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        TempRoot(root)
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Sets `BALLFIGHT_FFMPEG` while alive (the caller holds [`LOCK`]) and removes it when dropped.
struct FfmpegVar;

impl FfmpegVar {
    fn set(value: &OsStr) -> FfmpegVar {
        // SAFETY: every test of this file holds LOCK while the variable is set, and nothing else in this test
        // program reads the environment from outside Rust's std, which guards its own environment access.
        unsafe { std::env::set_var("BALLFIGHT_FFMPEG", value) };
        FfmpegVar
    }
}

impl Drop for FfmpegVar {
    fn drop(&mut self) {
        // SAFETY: as in `set`.
        unsafe { std::env::remove_var("BALLFIGHT_FFMPEG") };
    }
}

/// Whether ffmpeg can be used here; prints a notice when it cannot.
fn have_ffmpeg(test: &str) -> bool {
    match find_ffmpeg() {
        Ok(_) => true,
        Err(e) => {
            println!("{test}: skipped, ffmpeg is not usable here: {e}");
            false
        }
    }
}

fn opts(root: &Path, workers: usize, frames: Option<std::ops::Range<usize>>) -> JobOpts {
    JobOpts {
        out_root: root.to_path_buf(),
        workers,
        frames,
    }
}

fn workers() -> usize {
    std::thread::available_parallelism().map_or(4, usize::from).clamp(1, 15)
}

/// The names in a folder, sorted.
fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Every event a job sent, in order.
#[derive(Default)]
struct Events(Mutex<Vec<JobEvent>>);

impl Events {
    fn push(&self, e: JobEvent) {
        self.0.lock().unwrap().push(e);
    }

    fn take(&self) -> Vec<JobEvent> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

/// `ffprobe -v error <args> <file>` as text.
fn ffprobe(args: &[&str], file: &Path) -> String {
    let probe = find_ffmpeg().expect("ffmpeg").ffprobe;
    let out = Command::new(probe)
        .arg("-v")
        .arg("error")
        .args(args)
        .arg(file)
        .output()
        .expect("ffprobe runs");
    assert!(
        out.status.success(),
        "ffprobe: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("ffprobe prints text")
}

// ---------------------------------------------------------------- Review Focus 3
#[test]
fn missing_ffmpeg_is_a_clear_error() {
    let _guard = lock();
    let _var = FfmpegVar::set(OsStr::new("/nonexistent"));
    let err = find_ffmpeg().expect_err("there is no ffmpeg at /nonexistent");
    let text = err.to_string();
    assert!(text.contains("ffmpeg") && text.contains("/nonexistent"), "{text}");
    assert!(matches!(err, ExportError::Ffmpeg(_)), "{err:?}");

    // the job refuses before it makes anything
    let root = TempRoot::new("missing");
    let ep = &*EPISODE;
    let err = run_job(
        &ep.ff,
        Arc::clone(&ep.rec),
        &opts(&root.0, 2, Some(0..3)),
        &|_| {},
        &AtomicBool::new(false),
    )
    .expect_err("no ffmpeg, no video");
    assert!(err.to_string().contains("/nonexistent"), "{err}");
    assert_eq!(entries(&root.0), Vec::<String>::new());
}

/// A stand-in ffmpeg: it answers the version and encoder questions like ffmpeg, and fails every encode with a
/// line on its error output.
fn stub_ffmpeg(dir: &Path) -> PathBuf {
    let stub = dir.join("fake ffmpeg!.cmd");
    std::fs::write(
        &stub,
        "@echo off\r\n\
         if \"%~1\"==\"-hide_banner\" goto info\r\n\
         echo stub encoder: the disk caught fire 1>&2\r\n\
         exit /b 1\r\n\
         :info\r\n\
         echo ffmpeg version 0.0-stub Copyright (c) the stub\r\n\
         echo  V....D libx264              stub H.264\r\n\
         echo  A....D aac                  stub AAC\r\n\
         exit /b 0\r\n",
    )
    .unwrap();
    stub
}

#[test]
fn failing_ffmpeg_leaves_nothing() {
    let _guard = lock();
    if !have_ffmpeg("failing_ffmpeg_leaves_nothing") {
        return;
    }
    let tools = TempRoot::new("stub tools");
    let stub = stub_ffmpeg(&tools.0);
    {
        // outside tests BALLFIGHT_FFMPEG must name a program file: a script would run through cmd.exe and leave an
        // ffmpeg that cannot be ended
        let _var = FfmpegVar::set(stub.as_os_str());
        let err = find_ffmpeg().expect_err("a .cmd is refused");
        assert!(matches!(err, ExportError::Ffmpeg(_)), "{err:?}");
        assert!(
            err.to_string().contains(".exe") && err.to_string().contains("fake ffmpeg!.cmd"),
            "{err}"
        );
    }
    let found = ffmpeg_at(stub).expect("the stub passes as an ffmpeg with libx264 and aac");
    assert_eq!(found.version, "ffmpeg 0.0");

    let root = TempRoot::new("failing");
    let ep = &*EPISODE;
    let err = run_job_using(
        &found,
        &ep.ff,
        Arc::clone(&ep.rec),
        &opts(&root.0, 2, Some(0..30)),
        &|_| {},
        &AtomicBool::new(false),
    )
    .expect_err("the encode fails");
    let text = err.to_string();
    assert!(matches!(err, ExportError::Encode(_)), "{err:?}");
    assert!(
        text.contains("ffmpeg") && text.contains("the disk caught fire"),
        "{text}"
    );
    assert!(!text.contains('\n'), "one line: {text}");
    assert_eq!(
        entries(&root.0),
        Vec::<String>::new(),
        "nothing is left in the output folder"
    );
}

/// Windows only: denies this user some rights on a folder, and gives them back when dropped.
struct Denied {
    path: PathBuf,
    user: String,
}

impl Denied {
    fn new(path: &Path, rights: &str) -> Denied {
        let user = format!(
            "{}\\{}",
            std::env::var("USERDOMAIN").unwrap(),
            std::env::var("USERNAME").unwrap()
        );
        let out = Command::new("icacls")
            .arg(path)
            .arg("/deny")
            .arg(format!("{user}:({rights})"))
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
        Denied {
            path: path.to_path_buf(),
            user,
        }
    }
}

impl Drop for Denied {
    fn drop(&mut self) {
        let out = Command::new("icacls")
            .arg(&self.path)
            .arg("/remove:d")
            .arg(&self.user)
            .output();
        if !matches!(&out, Ok(o) if o.status.success()) {
            eprintln!("could not give back the rights on {}: {out:?}", self.path.display());
        }
    }
}

#[test]
fn unwritable_root_is_a_clear_error() {
    let _guard = lock();
    if !have_ffmpeg("unwritable_root_is_a_clear_error") {
        return;
    }
    let ep = &*EPISODE;
    let root = TempRoot::new("unwritable");
    let videos = root.0.join("Videos");
    std::fs::create_dir(&videos).unwrap();
    {
        let _denied = Denied::new(&videos, "WD,AD");
        let t0 = Instant::now();
        let err = run_job(
            &ep.ff,
            Arc::clone(&ep.rec),
            &opts(&videos, 2, Some(0..3)),
            &|_| {},
            &AtomicBool::new(false),
        )
        .expect_err("the folder cannot be written");
        let text = err.to_string();
        assert!(matches!(err, ExportError::Folder(_)), "{err:?}");
        assert!(
            text.contains(&videos.display().to_string()) && text.contains("cannot be written"),
            "{text}"
        );
        assert!(
            t0.elapsed() < Duration::from_secs(3),
            "refused before any work: {:?}",
            t0.elapsed()
        );
    }
    assert_eq!(entries(&videos), Vec::<String>::new());

    // a file where the output folder should be
    let file = root.0.join("not a folder");
    std::fs::write(&file, b"x").unwrap();
    let err = run_job(
        &ep.ff,
        Arc::clone(&ep.rec),
        &opts(&file, 2, Some(0..3)),
        &|_| {},
        &AtomicBool::new(false),
    )
    .expect_err("a file is no folder");
    assert!(matches!(err, ExportError::Folder(_)), "{err:?}");
    assert!(err.to_string().contains("not a folder"), "{err}");
    assert_eq!(std::fs::read(&file).unwrap(), b"x");
}

#[test]
fn cancel_leaves_nothing() {
    let _guard = lock();
    if !have_ffmpeg("cancel_leaves_nothing") {
        return;
    }
    let ep = &*EPISODE;
    let root = TempRoot::new("cancel");

    // cancelled before it starts
    let err = run_job(
        &ep.ff,
        Arc::clone(&ep.rec),
        &opts(&root.0, 2, Some(0..3)),
        &|_| {},
        &AtomicBool::new(true),
    )
    .expect_err("cancelled");
    assert!(matches!(err, ExportError::Cancelled(_)), "{err:?}");
    assert_eq!(entries(&root.0), Vec::<String>::new());

    // cancelled while the frames are drawn and encoded
    let cancel = AtomicBool::new(false);
    let at: Mutex<Option<Instant>> = Mutex::new(None);
    let on = |e: JobEvent| {
        if let JobEvent::Progress { done, .. } = e
            && done >= 8
            && !cancel.swap(true, Ordering::SeqCst)
        {
            *at.lock().unwrap() = Some(Instant::now());
        }
    };
    let err = run_job(
        &ep.ff,
        Arc::clone(&ep.rec),
        &opts(&root.0, workers(), Some(0..300)),
        &on,
        &cancel,
    )
    .expect_err("cancelled");
    let late = at.lock().unwrap().expect("the frames started").elapsed();
    assert!(matches!(err, ExportError::Cancelled(_)), "{err:?}");
    assert!(
        late < Duration::from_secs(3),
        "the job stopped {late:?} after the cancel"
    );
    assert_eq!(entries(&root.0), Vec::<String>::new(), "the temporary folder is gone");
}

// ---------------------------------------------------------------- real clips
#[test]
fn clip_is_valid() {
    let _guard = lock();
    if !have_ffmpeg("clip_is_valid") {
        return;
    }
    let ep = &*EPISODE;
    let root = TempRoot::new("clip");
    // an earlier video under the plain name, and an earlier clip of the same frames: neither is touched, and the
    // clip takes the seed name of a clip
    let plain = root.0.join("flute_vs_trombone");
    std::fs::create_dir(&plain).unwrap();
    std::fs::write(plain.join("keep me.txt"), b"an earlier video").unwrap();
    let earlier = root.0.join("flute_vs_trombone_frames0-91");
    std::fs::create_dir(&earlier).unwrap();
    assert_eq!(
        folder_name(["FLUTE", "TROMBONE"], 392, &root.0),
        root.0.join("flute_vs_trombone_seed392")
    );
    let name = "flute_vs_trombone_seed392_frames0-91";
    assert_eq!(
        clip_folder_name(["FLUTE", "TROMBONE"], 392, &(0..91), &root.0),
        root.0.join(name)
    );
    let events = Events::default();
    let t0 = Instant::now();
    let folder = run_job(
        &ep.ff,
        Arc::clone(&ep.rec),
        &opts(&root.0, workers(), Some(0..91)),
        &|e| events.push(e),
        &AtomicBool::new(false),
    )
    .expect("the clip renders");
    println!("clip of 91 frames: {:.1} s", t0.elapsed().as_secs_f64());
    assert_eq!(folder, root.0.join(name));
    assert_eq!(
        entries(&root.0),
        ["flute_vs_trombone", "flute_vs_trombone_frames0-91", name]
    );
    // a clip holds its mp4 only: no cover, caption or fight file, so the Library never lists it as a video
    assert_eq!(entries(&folder), [format!("{name}.mp4")]);
    assert_eq!(entries(&plain), ["keep me.txt"]);
    assert_eq!(std::fs::read(plain.join("keep me.txt")).unwrap(), b"an earlier video");
    assert_eq!(entries(&earlier), Vec::<String>::new());
    let mp4 = folder.join(format!("{name}.mp4"));

    // the events: the stages in order, the frames counted up to the last, every check passed, then done
    let events = events.take();
    let stages: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            JobEvent::Stage(s) => Some(*s),
            _ => None,
        })
        .collect();
    assert_eq!(
        stages,
        [
            stage::SOUNDTRACK,
            stage::FRAMES,
            stage::ENCODE,
            stage::VERIFY,
            stage::SAVE
        ]
    );
    let done: Vec<(usize, usize)> = events
        .iter()
        .filter_map(|e| match e {
            JobEvent::Progress { done, total, seconds } => {
                assert!(seconds.is_finite() && *seconds >= 0.0);
                Some((*done, *total))
            }
            _ => None,
        })
        .collect();
    assert_eq!(done, (1..=91).map(|d| (d, 91)).collect::<Vec<_>>());
    let checks: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            JobEvent::Check(c) => Some(c),
            _ => None,
        })
        .collect();
    assert!(!checks.is_empty());
    for c in &checks {
        assert!(c.pass, "{}: {}", c.name, c.summary);
        assert!(c.name.starts_with("mp4_"), "{}", c.name);
    }
    assert!(matches!(events.last(), Some(JobEvent::Done(p)) if *p == folder));

    // ffprobe's own view: h264 1080 x 1920 at 30 fps, aac 48 kHz stereo, nothing else
    let streams = ffprobe(
        &[
            "-show_entries",
            "stream=codec_type,codec_name,width,height,r_frame_rate,sample_rate,channels,nb_frames",
            "-of",
            "compact=p=0",
        ],
        &mp4,
    );
    let lines: Vec<&str> = streams.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 2, "{streams}");
    assert!(
        lines[0].contains("codec_name=h264")
            && lines[0].contains("codec_type=video")
            && lines[0].contains("width=1080")
            && lines[0].contains("height=1920")
            && lines[0].contains("r_frame_rate=30/1")
            && lines[0].contains("nb_frames=91"),
        "{}",
        lines[0]
    );
    assert!(
        lines[1].contains("codec_name=aac")
            && lines[1].contains("codec_type=audio")
            && lines[1].contains("sample_rate=48000")
            && lines[1].contains("channels=2"),
        "{}",
        lines[1]
    );
    // the first and the last frame shown are keyframes
    let packets = ffprobe(
        &[
            "-select_streams",
            "v:0",
            "-show_entries",
            "packet=pts,flags",
            "-of",
            "csv=p=0",
        ],
        &mp4,
    );
    let mut frames: Vec<(i64, bool)> = packets
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let (pts, flags) = l.trim().split_once(',').expect("pts,flags");
            (pts.parse().expect("a pts"), flags.starts_with('K'))
        })
        .collect();
    frames.sort();
    assert_eq!(frames.len(), 91);
    assert!(
        frames[0].1 && frames[90].1,
        "keyframes: {:?}",
        frames.iter().filter(|f| f.1).collect::<Vec<_>>()
    );

    // both clip names taken: refused at once, in words, and nothing changes
    let t0 = Instant::now();
    let err = run_job(
        &ep.ff,
        Arc::clone(&ep.rec),
        &opts(&root.0, 2, Some(0..91)),
        &|_| {},
        &AtomicBool::new(false),
    )
    .expect_err("no free name");
    assert!(matches!(err, ExportError::Folder(_)), "{err:?}");
    assert!(err.to_string().contains(name), "{err}");
    assert!(
        t0.elapsed() < Duration::from_secs(3),
        "refused before any work: {:?}",
        t0.elapsed()
    );
    assert_eq!(
        entries(&root.0),
        ["flute_vs_trombone", "flute_vs_trombone_frames0-91", name]
    );
}

#[test]
fn the_video_does_not_depend_on_the_workers() {
    let _guard = lock();
    if !have_ffmpeg("the_video_does_not_depend_on_the_workers") {
        return;
    }
    let ep = &*EPISODE;
    let ffmpeg = find_ffmpeg().expect("ffmpeg").ffmpeg;
    // the MD5 of every packet of one stream, copied out of the file as it is
    let packets = |mp4: &Path, map: &str| {
        let out = Command::new(&ffmpeg)
            .args(["-v", "error", "-i"])
            .arg(mp4)
            .args(["-map", map, "-c", "copy", "-f", "md5", "-"])
            .output()
            .expect("ffmpeg runs");
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8(out.stdout).unwrap()
    };
    let mut got = Vec::new();
    for w in [1, 8] {
        let root = TempRoot::new(&format!("workers {w}"));
        let folder = run_job(
            &ep.ff,
            Arc::clone(&ep.rec),
            &opts(&root.0, w, Some(1500..1512)),
            &|_| {},
            &AtomicBool::new(false),
        )
        .expect("the clip renders");
        let mp4 = folder.join("flute_vs_trombone_frames1500-1512.mp4");
        got.push((packets(&mp4, "0:v"), packets(&mp4, "0:a"), std::fs::read(&mp4).unwrap()));
    }
    assert!(got[0].0.starts_with("MD5=") && got[0].2.len() > 100_000);
    assert_eq!(got[0].0, got[1].0, "the video stream differs between 1 and 8 workers");
    assert_eq!(got[0].1, got[1].1, "the sound stream differs between 1 and 8 workers");
    // the streams first (the clearer message), then the whole file: `-max_interleave_delta 0` makes the muxer's
    // chunks follow the timestamps alone, not the timing of the frames' arrival (ruling R42)
    assert!(
        got[0].2 == got[1].2,
        "the mp4 files differ between 1 and 8 workers though every packet is the same"
    );
}

#[test]
#[ignore = "the whole 61 s video with every check: about a minute and a half in release; run it by name"]
fn full_render_passes_every_check() {
    let _guard = lock();
    if !have_ffmpeg("full_render_passes_every_check") {
        return;
    }
    let ep = &*EPISODE;
    let root = TempRoot::new("full");
    let t0 = Instant::now();
    let stamps: Mutex<Vec<(String, f64)>> = Mutex::new(Vec::new());
    let on = |e: JobEvent| match e {
        JobEvent::Stage(s) => stamps.lock().unwrap().push((s.to_string(), t0.elapsed().as_secs_f64())),
        JobEvent::Check(c) => println!("  {} {}: {}", if c.pass { "ok  " } else { "FAIL" }, c.name, c.summary),
        _ => {}
    };
    let folder = run_job(
        &ep.ff,
        Arc::clone(&ep.rec),
        &opts(&root.0, workers(), None),
        &on,
        &AtomicBool::new(false),
    )
    .unwrap_or_else(|e| panic!("the full render: {e}"));
    let total = t0.elapsed().as_secs_f64();
    for (s, t) in stamps.lock().unwrap().iter() {
        println!("stage {s} from {t:.1} s");
    }
    // the plain name, holding the four files of a finished video
    assert_eq!(folder, root.0.join("flute_vs_trombone"));
    assert_eq!(entries(&root.0), ["flute_vs_trombone"]);
    assert_eq!(
        entries(&folder),
        [
            "caption.txt",
            "cover.png",
            "flute_vs_trombone.mp4",
            "flute_vs_trombone.toml"
        ]
    );
    let (w, h, c, _) = bf_testdata::png::read_png(&folder.join("cover.png"));
    assert_eq!((w, h, c), (1080, 1920, 3));
    let caption = std::fs::read_to_string(folder.join("caption.txt")).unwrap();
    let cap = &ep.ff.captions;
    let want = format!(
        "{}\ninstagram: {}\ntiktok: {}\nshorts: {}\n",
        cap.caption,
        cap.instagram,
        cap.tiktok,
        cap.shorts.join("\n")
    );
    assert_eq!(caption, want);
    let toml = std::fs::read_to_string(folder.join("flute_vs_trombone.toml")).unwrap();
    assert_eq!(FightFile::parse(&toml).expect("the saved fight file reads"), ep.ff);
    let mp4 = folder.join("flute_vs_trombone.mp4");
    let size = std::fs::metadata(&mp4).unwrap().len();
    println!("full render: {total:.1} s with {} workers, {size} bytes", workers());
    let duration = ffprobe(&["-show_entries", "format=duration", "-of", "csv=p=0"], &mp4);
    println!("ffprobe duration: {}", duration.trim());
    assert!(duration.trim().starts_with("61.01"), "{duration}");
}
