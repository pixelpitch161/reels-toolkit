//! The render job (§9): build the soundtrack, run the checks, draw and encode every frame, check the finished
//! file, write the files beside it and move the folder into place.
//!
//! Stages, in order ([`JobEvent::Stage`]): [`stage::SOUNDTRACK`], [`stage::CHECKS`] (a whole video only),
//! [`stage::FRAMES`] (the frames drawn and piped to ffmpeg; [`JobEvent::Progress`] after each), [`stage::ENCODE`]
//! (ffmpeg finishes the file), [`stage::VERIFY`], [`stage::SAVE`]. Each check's result is sent as
//! [`JobEvent::Check`] when it is known; the job stops at the first that failed.
//!
//! Cancel is looked at between the stages, before each frame is drawn (in the pre-render checks too), every
//! 50 ms while a frame is awaited, and while the finished file is decoded: it acts within about a second
//! everywhere except inside the soundtrack build (about 4 s), which runs to its end.
//!
//! Frames are drawn on `workers` threads, at most `2 x workers` ahead of the encoder, and given to ffmpeg strictly
//! in order, so memory stays bounded and the video does not depend on the number of workers. Raw frames are never
//! written to disk.

use std::collections::BTreeMap;
use std::ops::Range;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use bf_content::{build_picture, build_soundtrack};
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_picture::Rgb8;
use bf_picture::config::{FPS, NF};
use bf_picture::ring::{Picture, upscale3};
use bf_sound::wav::wav16_bytes;
use bf_story::{BeatSheet, story};

use crate::checks::{BandPlan, CheckResult, panic_text, pre_render};
use crate::encode::{Encoder, Ffmpeg, find_ffmpeg};
use crate::output::{TempFolder, caption_text, clip_folder_name, folder_stem, png_bytes, prepare_root, taken};
use crate::probe::verify_until;
use crate::{ExportError, output};

/// The names of the stages a job goes through, in order.
pub mod stage {
    /// The soundtrack is built.
    pub const SOUNDTRACK: &str = "soundtrack";
    /// The checks before the render (a whole video only): focus, safe zone, levels, band, loudness, flashes, loop.
    pub const CHECKS: &str = "checks";
    /// The frames are drawn and given to ffmpeg.
    pub const FRAMES: &str = "frames";
    /// ffmpeg finishes the file.
    pub const ENCODE: &str = "encode";
    /// The finished file is checked.
    pub const VERIFY: &str = "verify";
    /// The cover, the caption and the fight file are written and the folder moved into place.
    pub const SAVE: &str = "save";
    /// All of them, in order.
    pub const ALL: [&str; 6] = [SOUNDTRACK, CHECKS, FRAMES, ENCODE, VERIFY, SAVE];
}

/// What a job is asked to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobOpts {
    /// The output folder; the video's own folder goes in it. Made if it is missing.
    pub out_root: PathBuf,
    /// The threads that draw frames (at least one is used).
    pub workers: usize,
    /// Only these frames (a clip, for tests and quick looks). A clip is never a finished video (ruling R41): its
    /// folder and file are `<a>_vs_<b>_frames<A>-<B>` and hold only the mp4 (no cover, caption or fight file),
    /// and the checks before the render and the two loop checks of the file are left out, since they are about
    /// the whole video. `None` for the whole video.
    pub frames: Option<Range<usize>>,
}

/// What a running job reports.
#[derive(Clone, Debug, PartialEq)]
pub enum JobEvent {
    /// A stage begins (one of [`stage::ALL`]).
    Stage(&'static str),
    /// `done` of `total` frames are drawn and given to ffmpeg, `seconds` after the first frame was started.
    Progress { done: usize, total: usize, seconds: f64 },
    /// A check's result.
    Check(CheckResult),
    /// The video is saved in this folder.
    Done(PathBuf),
}

/// Renders the fight `ff` from its recording `rec` into a new folder under `opts.out_root` and gives that folder,
/// with the ffmpeg [`find_ffmpeg`] finds. See [`run_job_using`].
///
/// # Errors
/// An [`ExportError`] in one plain sentence.
pub fn run_job(
    ff: &FightFile,
    rec: Arc<Recording>,
    opts: &JobOpts,
    on: &(dyn Fn(JobEvent) + Sync),
    cancel: &AtomicBool,
) -> Result<PathBuf, ExportError> {
    if cancel.load(Ordering::SeqCst) {
        return Err(ExportError::cancelled());
    }
    let ffmpeg = find_ffmpeg()?;
    run_job_using(&ffmpeg, ff, rec, opts, on, cancel)
}

/// [`run_job`] with the ffmpeg given (one found once by [`find_ffmpeg`], or a stand-in from
/// [`crate::encode::ffmpeg_at`] in tests).
///
/// Before any work: the fight file is checked, the output folder is made if missing and the video's folder name
/// is decided ([`output::folder_name`], [`output::clip_folder_name`]); a name already taken twice is an error.
/// Everything is written into a hidden temporary folder beside it, which a failure or a cancel removes.
///
/// # Errors
/// An [`ExportError`] in one plain sentence; nothing is left in the output folder (unless the temporary folder
/// could not be removed, which the sentence then says, or the finished video's name was taken at the last
/// moment, when the finished folder is kept and named).
pub fn run_job_using(
    ffmpeg: &Ffmpeg,
    ff: &FightFile,
    rec: Arc<Recording>,
    opts: &JobOpts,
    on: &(dyn Fn(JobEvent) + Sync),
    cancel: &AtomicBool,
) -> Result<PathBuf, ExportError> {
    let stop = || {
        if cancel.load(Ordering::SeqCst) {
            Err(ExportError::cancelled())
        } else {
            Ok(())
        }
    };
    stop()?;
    // validate() refuses any fight file without exactly two fighters
    bf_content::validate(ff)?;
    let names = [ff.fighters[0].as_str(), ff.fighters[1].as_str()];
    let seed = ff.seed.unwrap_or(rec.seed);
    let frames = match &opts.frames {
        None => 0..NF,
        Some(r) if r.start < r.end && r.end <= NF => r.clone(),
        Some(r) => {
            return Err(ExportError::Clip(format!(
                "The frames {} to {} are not a stretch of the video's {NF} frames (0 to {}), so nothing was made; \
                 ask for frames inside it.",
                r.start,
                r.end,
                NF - 1
            )));
        }
    };
    let whole = opts.frames.is_none();
    let root = std::path::absolute(&opts.out_root).unwrap_or_else(|_| opts.out_root.clone());
    prepare_root(&root)?;
    let (target, first) = if whole {
        (output::folder_name(names, seed, &root), root.join(folder_stem(names)))
    } else {
        let stem = folder_stem(names);
        (
            clip_folder_name(names, seed, &frames, &root),
            root.join(format!("{stem}_frames{}-{}", frames.start, frames.end)),
        )
    };
    if taken(&target) {
        return Err(ExportError::Folder(format!(
            "Both {} and {} are already taken in the output folder, so nothing was made; move or rename one of \
             them, or choose another folder.",
            first.display(),
            target.display()
        )));
    }
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| folder_stem(names));
    stop()?;
    let mut tmp = TempFolder::create(&root, &name)?;

    let job = Job {
        ff,
        rec: &rec,
        workers: opts.workers.max(1),
        frames,
        whole,
        ffmpeg,
        name: &name,
        on,
        cancel,
    };
    let made = job.make(&mut tmp, &stop).and_then(|()| tmp.move_to(&target));
    match made {
        Ok(()) => {
            on(JobEvent::Done(target.clone()));
            Ok(target)
        }
        Err(e) => match tmp.discard() {
            Ok(()) => Err(e),
            Err(left) => Err(e.with_clause(&left)),
        },
    }
}

/// One job's inputs.
struct Job<'a> {
    ff: &'a FightFile,
    rec: &'a Arc<Recording>,
    workers: usize,
    frames: Range<usize>,
    whole: bool,
    ffmpeg: &'a Ffmpeg,
    name: &'a str,
    on: &'a (dyn Fn(JobEvent) + Sync),
    cancel: &'a AtomicBool,
}

impl Job<'_> {
    /// Everything between the temporary folder and the move, written into `tmp`.
    fn make(&self, tmp: &mut TempFolder, stop: &dyn Fn() -> Result<(), ExportError>) -> Result<(), ExportError> {
        let (ff, rec, on, name) = (self.ff, self.rec, self.on, self.name);
        on(JobEvent::Stage(stage::SOUNDTRACK));
        let st = build_soundtrack(rec, ff)?;
        stop()?;
        let pic = build_picture(Arc::clone(rec), ff)?;
        stop()?;

        if self.whole {
            on(JobEvent::Stage(stage::CHECKS));
            let cfg = &pic.ctx().cfg;
            let show = &pic.ctx().show;
            let sheet = BeatSheet::series([&cfg[0], &cfg[1]]);
            let story = story(rec, &show.grid, &sheet, [&cfg[0].low.name, &cfg[1].low.name]);
            let plan = BandPlan::of(ff)?;
            let results = pre_render(&pic, &st, show, &story, &plan, self.workers, self.cancel)
                .ok_or_else(ExportError::cancelled)?;
            report(results, on)?;
            stop()?;
        }

        let wav = tmp.write("audio.wav", &wav16_bytes(&st.master))?;
        drop(st);
        let (part, mp4) = (format!("{name}.mp4.part"), format!("{name}.mp4"));
        let part_path = tmp.file(&part);
        let total = self.frames.len();
        on(JobEvent::Stage(stage::FRAMES));
        let mut enc = Encoder::start(self.ffmpeg, &wav, &part_path, total)?;
        let t0 = Instant::now();
        let first = self.frames.start;
        let drawn = draw_in_order(
            self.frames.clone(),
            self.workers,
            &|f| draw(&pic, f),
            &mut |f, img| {
                enc.push(&img)?;
                on(JobEvent::Progress {
                    done: f - first + 1,
                    total,
                    seconds: t0.elapsed().as_secs_f64(),
                });
                Ok(())
            },
            self.cancel,
        );
        if let Err(e) = drawn {
            enc.abort();
            return Err(e);
        }
        stop()?;
        on(JobEvent::Stage(stage::ENCODE));
        enc.finish()?;
        stop()?;

        on(JobEvent::Stage(stage::VERIFY));
        let results =
            verify_until(self.ffmpeg, &part_path, total, self.whole, self.cancel).ok_or_else(ExportError::cancelled)?;
        report(results, on)?;
        stop()?;

        on(JobEvent::Stage(stage::SAVE));
        tmp.rename(&part, &mp4)?;
        tmp.remove("audio.wav")?;
        if self.whole {
            let f = pic.card().cover_frame();
            let cover = draw(&pic, f).map_err(|why| frame_error(f, &why))?;
            tmp.write("cover.png", &png_bytes(&upscale3(&cover)))?;
            tmp.write("caption.txt", caption_text(ff).as_bytes())?;
            // the fight file last: a folder with it is a finished video
            tmp.write(&format!("{name}.toml"), ff.to_toml().as_bytes())?;
        }
        stop()
    }
}

/// Sends every result, then fails with the first that did not pass.
fn report(results: Vec<CheckResult>, on: &(dyn Fn(JobEvent) + Sync)) -> Result<(), ExportError> {
    let failed = results.iter().find(|r| !r.pass).cloned();
    for r in results {
        on(JobEvent::Check(r));
    }
    match failed {
        Some(r) => Err(ExportError::Check(r)),
        None => Ok(()),
    }
}

/// Frame `f` as the video shows it, or why it cannot be: a frame with a hull the drawing code refused is not
/// the kit's frame and is never written.
fn draw(pic: &Picture, f: usize) -> Result<Rgb8, String> {
    let (img, _, report) = pic.frame_reported(f);
    if report.hull_errors != 0 {
        return Err(format!(
            "{} of its shapes could not be drawn the way the reference does",
            report.hull_errors
        ));
    }
    Ok(img)
}

/// The sentence for frame `f` that could not be drawn.
fn frame_error(f: usize, why: &str) -> ExportError {
    ExportError::Frame(format!(
        "Frame {f} (at {:.2} s) could not be drawn ({}), so nothing was saved; this is a fault in the app, not in \
         your fight.",
        f as f64 / f64::from(FPS),
        why.replace(['\n', '\r'], " ").trim().trim_end_matches('.')
    ))
}

/// What the drawing threads and the writer share.
struct Line {
    /// The next frame to start.
    next: usize,
    /// The next frame to give to the writer.
    written: usize,
    /// Drawn frames not yet written.
    ready: BTreeMap<usize, Result<Rgb8, String>>,
    /// The writer has stopped: draw nothing more.
    stop: bool,
}

fn lock(m: &Mutex<Line>) -> MutexGuard<'_, Line> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Tells the drawing threads to stop when dropped, however the writer leaves (a return or a panic).
struct StopOnDrop<'a> {
    line: &'a Mutex<Line>,
    moved: &'a Condvar,
}

impl Drop for StopOnDrop<'_> {
    fn drop(&mut self) {
        lock(self.line).stop = true;
        self.moved.notify_all();
    }
}

/// Draws `frames` on `workers` threads, at most `2 x workers` frames ahead of the writer, and gives each to `sink`
/// in order, on the calling thread. A frame `draw` refuses (or panics on) stops it with that frame named;
/// `cancel` is looked at before each frame and every 50 ms while one is awaited. A panic in `sink` stops the
/// drawing threads and goes on up.
pub(crate) fn draw_in_order(
    frames: Range<usize>,
    workers: usize,
    draw: &(dyn Fn(usize) -> Result<Rgb8, String> + Sync),
    sink: &mut dyn FnMut(usize, Rgb8) -> Result<(), ExportError>,
    cancel: &AtomicBool,
) -> Result<(), ExportError> {
    let workers = workers.max(1);
    let window = 2 * workers;
    let line = Mutex::new(Line {
        next: frames.start,
        written: frames.start,
        ready: BTreeMap::new(),
        stop: false,
    });
    let moved = Condvar::new();
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let f = {
                        let mut l = lock(&line);
                        loop {
                            if l.stop || l.next >= frames.end {
                                return;
                            }
                            if l.next < l.written + window {
                                break;
                            }
                            l = moved.wait(l).unwrap_or_else(|e| e.into_inner());
                        }
                        l.next += 1;
                        l.next - 1
                    };
                    let img = catch_unwind(AssertUnwindSafe(|| draw(f))).unwrap_or_else(|p| Err(panic_text(p)));
                    lock(&line).ready.insert(f, img);
                    moved.notify_all();
                }
            });
        }
        let _stop = StopOnDrop {
            line: &line,
            moved: &moved,
        };
        for f in frames.clone() {
            let img = {
                let mut l = lock(&line);
                loop {
                    if cancel.load(Ordering::SeqCst) {
                        return Err(ExportError::cancelled());
                    }
                    if let Some(img) = l.ready.remove(&f) {
                        l.written = f + 1;
                        break img;
                    }
                    l = moved
                        .wait_timeout(l, Duration::from_millis(50))
                        .unwrap_or_else(|e| e.into_inner())
                        .0;
                }
            };
            moved.notify_all();
            sink(f, img.map_err(|why| frame_error(f, &why))?)?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;

    fn tiny(f: usize) -> Rgb8 {
        Rgb8::from_vec(1, 1, 3, vec![(f % 256) as u8, (f / 256) as u8, 0])
    }

    /// A drawer that takes a while, longer for some frames, and counts how many frames are out at once.
    fn slow(out: &AtomicUsize, most: &AtomicUsize, f: usize) -> Result<Rgb8, String> {
        let now = out.fetch_add(1, Ordering::SeqCst) + 1;
        most.fetch_max(now, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(((f * 7919) % 13) as u64));
        out.fetch_sub(1, Ordering::SeqCst);
        Ok(tiny(f))
    }

    #[test]
    fn frames_come_out_in_order_whatever_the_workers() {
        for workers in [1, 3, 8] {
            let (out, most) = (AtomicUsize::new(0), AtomicUsize::new(0));
            let mut got = Vec::new();
            draw_in_order(
                5..125,
                workers,
                &|f| slow(&out, &most, f),
                &mut |f, img| {
                    assert_eq!(img, tiny(f));
                    got.push(f);
                    Ok(())
                },
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(got, (5..125).collect::<Vec<_>>());
            assert!(most.load(Ordering::SeqCst) <= workers);
        }
    }

    #[test]
    fn the_drawers_never_run_far_ahead() {
        // a writer that is slow: the frames waiting for it stay within the window
        let started = AtomicUsize::new(0);
        let mut ahead = 0;
        draw_in_order(
            0..60,
            4,
            &|f| {
                started.fetch_max(f + 1, Ordering::SeqCst);
                Ok(tiny(f))
            },
            &mut |f, _| {
                std::thread::sleep(Duration::from_millis(2));
                ahead = ahead.max(started.load(Ordering::SeqCst) - f);
                Ok(())
            },
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(ahead <= 2 * 4 + 1, "{ahead} frames ahead");
    }

    #[test]
    fn a_broken_frame_is_named() {
        let err = draw_in_order(
            0..40,
            4,
            &|f| {
                if f == 17 {
                    panic!("a look fell over");
                }
                if f == 30 {
                    return Err("2 of its shapes could not be drawn".into());
                }
                Ok(tiny(f))
            },
            &mut |_, _| Ok(()),
            &AtomicBool::new(false),
        )
        .unwrap_err();
        let text = err.to_string();
        assert!(matches!(err, ExportError::Frame(_)));
        assert!(
            text.contains("Frame 17 (at 0.57 s)") && text.contains("a look fell over"),
            "{text}"
        );
    }

    #[test]
    fn cancel_stops_between_frames() {
        let cancel = AtomicBool::new(false);
        let mut seen = 0;
        let err = draw_in_order(
            0..1000,
            2,
            &|f| {
                std::thread::sleep(Duration::from_millis(5));
                Ok(tiny(f))
            },
            &mut |f, _| {
                seen = f + 1;
                if f == 9 {
                    cancel.store(true, Ordering::SeqCst);
                }
                Ok(())
            },
            &cancel,
        )
        .unwrap_err();
        assert!(matches!(err, ExportError::Cancelled(_)));
        assert_eq!(seen, 10);
    }

    #[test]
    fn a_failing_writer_stops_the_drawers() {
        let drawn = AtomicUsize::new(0);
        let err = draw_in_order(
            0..1000,
            4,
            &|f| {
                drawn.fetch_add(1, Ordering::SeqCst);
                Ok(tiny(f))
            },
            &mut |f, _| {
                if f == 3 {
                    Err(ExportError::Encode("ffmpeg stopped.".into()))
                } else {
                    Ok(())
                }
            },
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert!(matches!(err, ExportError::Encode(_)));
        assert!(drawn.load(Ordering::SeqCst) <= 4 + 2 * 4 + 4);
    }

    #[test]
    fn a_panicking_writer_returns() {
        // a progress consumer that panics (a closed pipe, a dropped receiver) must not leave the drawers waiting
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let got = catch_unwind(AssertUnwindSafe(|| {
                draw_in_order(
                    0..1000,
                    4,
                    &|f| Ok(tiny(f)),
                    &mut |f, _| {
                        if f == 5 {
                            panic!("the receiver is gone");
                        }
                        Ok(())
                    },
                    &AtomicBool::new(false),
                )
            }));
            let _ = tx.send(got.is_err());
        });
        let panicked = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("draw_in_order returned instead of hanging");
        assert!(panicked, "the writer's panic goes on up");
    }
}
