//! Render jobs: `Enqueue`, `RunJob`, `CancelJob` and `CheckTools`.
//!
//! - `Enqueue` makes the draft's fight file (it must have a seed) and answers `Enqueued` with the job's spec, or
//!   `EnqueueFailed` with the reason in words. It is quick, so it is answered on the coordinator, in order.
//! - `RunJob` fights the job's fight file again (recording on) and renders it with `bf_export::run_job` on a thread
//!   of its own, with `ctx.workers()` drawing threads (the render keeps them while the preview draws: the preview is
//!   the one that may skip frames) and the engine's `clip` as the frames. Its [`JobEvent`]s become `Event::Job`
//!   updates through [`Steps`]: a stage makes its line `Now` and the lines before it `Done`, a check makes its line
//!   `Done` or `Failed` with the check's summary as the note, frame counts are sent ten a second at most (and the
//!   last one always), success makes every line `Done` and answers `Done`. A failure answers [`failure`]'s words.
//!   Starting the next job is the state's business: it sends one `RunJob` per job. A `RunJob` while another job
//!   renders cancels the older one quietly (nothing more is sent for it).
//! - `CancelJob` sets the running job's cancel flag; the job answers `Cancelled` once `run_job` has returned, so the
//!   half-made video's temporary folder is already gone. `run_job` looks at the flag within about a second
//!   everywhere except inside the soundtrack build (about 4 s), which runs to its end first.
//! - `CheckTools` looks for ffmpeg on a thread of its own: `Tools` with its version line ("ffmpeg 9.0.2"), or
//!   "ffmpeg was not found on this PC.".
//!
//! ffmpeg is found by `bf_export::find_ffmpeg` (`BALLFIGHT_FFMPEG`, then the `PATH`); `run_job` asks for it again
//! before any work, so a missing ffmpeg, like a folder that cannot be used, is refused before anything is written.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_export::{CheckResult, ExportError, JobEvent, JobOpts, run_job, stage};

use super::Ctx;
use super::convert::{fight_file, plain};
use crate::model::JobId;
use crate::model::draft::Draft;
use crate::model::queue::{FailHint, JobSpec, JobUpdate, STEP_ORDER, StepKey, StepState};
use crate::seam::{Command, Event};
use crate::wording;

/// Frame counts are sent at most this often (ten a second); the last one is always sent.
const FRAMES_EVERY: Duration = Duration::from_millis(100);

/// The six lines the checks before the render fill, in the check list's order.
const CHECK_LINES: [StepKey; 6] = [
    StepKey::SafeZone,
    StepKey::Focus,
    StepKey::Loudness,
    StepKey::Band,
    StepKey::Flashes,
    StepKey::LoopPoint,
];

/// What `CheckTools` answers when ffmpeg cannot be used.
const NO_FFMPEG: &str = "ffmpeg was not found on this PC.";

/// The headline of a job that stopped for a reason that is not a check, ffmpeg or the folder.
const NOT_MADE: &str = "The video could not be made";

/// One job that renders: its id, its cancel flag, and whether its one last word (`Done`, `Failed` or
/// `Cancelled`) has been sent; after it, nothing more is sent for the job.
struct Running {
    id: JobId,
    cancel: Arc<AtomicBool>,
    answered: Arc<AtomicBool>,
}

/// Sends `update` for job `id` unless the job has already had its last word. A last word (`Done`, `Failed`,
/// `Cancelled`) claims `answered` first, so each started job gets exactly one, whoever sends it (its thread, the
/// panic answer, or a newer `RunJob` that replaces it).
fn send_update(ctx: &Ctx, id: JobId, answered: &AtomicBool, update: JobUpdate) {
    let last = matches!(
        update,
        JobUpdate::Done { .. } | JobUpdate::Failed { .. } | JobUpdate::Cancelled
    );
    let may = if last {
        !answered.swap(true, Ordering::SeqCst)
    } else {
        !answered.load(Ordering::SeqCst)
    };
    if may && !ctx.stopping() {
        ctx.emit(Event::Job { id, update });
    }
}

/// What this handler keeps between commands: the jobs still running (normally one; a replaced job stays listed
/// until its thread has let go of its flag).
#[derive(Default)]
pub struct State {
    running: Vec<Running>,
}

/// Takes one command and returns at once; long work goes to its own thread (`Ctx::spawn`).
pub fn handle(state: &mut State, ctx: &Ctx, command: Command) {
    // A job whose thread has ended leaves the only clone of its flag here.
    state.running.retain(|r| Arc::strong_count(&r.cancel) > 1);
    match command {
        Command::Enqueue { draft, out_root } => ctx.emit(enqueue(&draft, out_root)),
        Command::RunJob {
            id,
            fight_toml,
            out_root,
        } => run(state, ctx, id, fight_toml, out_root),
        Command::CancelJob { id } => {
            for r in state.running.iter().filter(|r| r.id == id) {
                r.cancel.store(true, Ordering::SeqCst);
            }
        }
        Command::CheckTools => {
            ctx.spawn(
                "engine: tools",
                "looking for ffmpeg",
                |ctx| {
                    let ffmpeg = bf_export::find_ffmpeg()
                        .map(|f| f.version)
                        .map_err(|_| NO_FFMPEG.to_string());
                    if !ctx.stopping() {
                        ctx.emit(Event::Tools { ffmpeg });
                    }
                },
                |_| Event::Tools {
                    ffmpeg: Err(NO_FFMPEG.to_string()),
                },
            );
        }
        // The coordinator sends this handler nothing else.
        _ => {}
    }
}

/// The answer to `Enqueue`: the job's spec, fixed now, or why the draft cannot be queued.
fn enqueue(draft: &Draft, out_root: PathBuf) -> Event {
    let Some(seed) = draft.seed else {
        return Event::EnqueueFailed {
            problem: "Pick a seed first.".to_string(),
        };
    };
    let ff = match fight_file(draft) {
        Ok(ff) => ff,
        Err(problem) => return Event::EnqueueFailed { problem },
    };
    let colour = |name: &str| {
        bf_content::roster()
            .iter()
            .find(|f| f.name == name)
            .map_or([0x80; 3], |f| f.color)
    };
    let theme = bf_content::theme_card(&draft.theme).map_or_else(|| draft.theme.clone(), |t| t.display.to_string());
    Event::Enqueued {
        spec: JobSpec {
            title: wording::versus(&draft.fighters),
            colors: [colour(&draft.fighters[0]), colour(&draft.fighters[1])],
            theme,
            seed,
            fight_toml: ff.to_toml(),
            out_root,
        },
    }
}

/// Starts job `id` on a thread of its own. A job still running is cancelled and answered `Cancelled` here, at
/// once (as the sample engine does); its thread then sends nothing more.
fn run(state: &mut State, ctx: &Ctx, id: JobId, fight_toml: String, out_root: PathBuf) {
    let cancel = ctx.cancel_flag();
    let answered = Arc::new(AtomicBool::new(false));
    let (c, a, p) = (cancel.clone(), answered.clone(), answered.clone());
    let started = ctx.spawn(
        "engine: render",
        "rendering the video",
        move |ctx| render(&ctx, id, &fight_toml, out_root, &c, &a),
        move |text| {
            if p.swap(true, Ordering::SeqCst) {
                // The job had its last word already: the panic is told as a problem only.
                super::problem(text)
            } else {
                Event::Job {
                    id,
                    update: not_made(&text),
                }
            }
        },
    );
    if !started {
        return;
    }
    for older in state.running.drain(..) {
        older.cancel.store(true, Ordering::SeqCst);
        send_update(ctx, older.id, &older.answered, JobUpdate::Cancelled);
    }
    state.running.push(Running { id, cancel, answered });
}

/// "The video could not be made", with `why` and "Nothing was saved.".
fn not_made(why: &str) -> JobUpdate {
    JobUpdate::Failed {
        headline: NOT_MADE.to_string(),
        detail: format!("{} Nothing was saved.", plain(&why)),
        hint: FailHint::None,
    }
}

/// The job's thread: the fight again, then the render with its events as updates, then its end.
fn render(ctx: &Ctx, id: JobId, fight_toml: &str, out_root: PathBuf, cancel: &AtomicBool, answered: &AtomicBool) {
    let t0 = Instant::now();
    // Nothing after its last word (one a newer job may have sent for it), nor once the engine is ending.
    let send = |update: JobUpdate| send_update(ctx, id, answered, update);
    let (ff, rec) = match fight(fight_toml) {
        Ok(fought) if !cancel.load(Ordering::SeqCst) => fought,
        Ok(_) => return send(JobUpdate::Cancelled),
        Err(_) if cancel.load(Ordering::SeqCst) => return send(JobUpdate::Cancelled),
        Err(update) => return send(update),
    };
    let opts = JobOpts {
        out_root,
        workers: ctx.workers(),
        frames: ctx.config.clip.clone(),
    };
    let steps = Mutex::new(Steps::default());
    let on = |e: JobEvent| {
        let updates = super::lock(&steps).on(e, Instant::now());
        // A cancelled job's last word is its `Cancelled`, with nothing in between.
        if !cancel.load(Ordering::SeqCst) {
            updates.into_iter().for_each(send);
        }
    };
    let result = run_job(&ff, Arc::new(rec), &opts, &on, cancel);
    let (ends, problem) = {
        let mut steps = super::lock(&steps);
        match result {
            Ok(folder) => {
                // Saved, even if a cancel came in the last moment: the answer says what is on disk.
                let mut ends = steps.all_done();
                ends.push(JobUpdate::Done {
                    folder,
                    seconds: t0.elapsed().as_secs_f64(),
                });
                (ends, None)
            }
            Err(e) if matches!(e, ExportError::Cancelled(_)) || cancel.load(Ordering::SeqCst) => {
                (vec![JobUpdate::Cancelled], cancel_problem(&e))
            }
            Err(e) => {
                let mut ends = steps.failed_at(&e);
                ends.push(failure(&e, None));
                (ends, None)
            }
        }
    };
    // The guard is gone: nothing is sent under a lock.
    if let Some(text) = problem
        && !answered.load(Ordering::SeqCst)
        && !ctx.stopping()
    {
        ctx.emit(super::problem(text));
    }
    ends.into_iter().for_each(send);
}

/// What a cancelled job must still tell the owner: nothing when it ended as a plain cancel; otherwise the export's
/// own sentence (a temporary folder that could not be removed, say, or a failure that came with the cancel), so a
/// folder left behind is never hidden behind `Cancelled`.
fn cancel_problem(e: &ExportError) -> Option<String> {
    (*e != ExportError::cancelled()).then(|| plain(e))
}

/// The fight of the job's fight file with its recording, or why there is none (as the job's failure).
fn fight(fight_toml: &str) -> Result<(FightFile, Recording), JobUpdate> {
    let ff = FightFile::parse(fight_toml).map_err(|e| not_made(&format!("This fight cannot be used: {e}")))?;
    let Some(seed) = ff.seed else {
        return Err(not_made("This fight file names no seed."));
    };
    let setup = bf_sim::setup_from(&ff, bf_content::roster(), seed)
        .map_err(|e| not_made(&format!("This fight cannot be fought: {e}")))?;
    let rec = bf_sim::run(&setup, true).map_err(|e| not_made(&format!("Seed {seed} cannot be fought: {e}")))?;
    if rec.winner.is_none() || rec.t_ko.is_none() {
        return Err(JobUpdate::Failed {
            headline: NOT_MADE.to_string(),
            detail: format!(
                "Seed {seed} has no K.O. before 58 s, so it has no video. Nothing was saved. Try another seed, then \
                 queue it again."
            ),
            hint: FailHint::OpenInSeeds,
        });
    }
    Ok((ff, rec))
}

/// The line of the check list a check fills, by the check's `name`: `levels` and `loudness` the loudness line,
/// `band` the fighters-above-the-music line, `flash` the flashes, `loop_picture` and `loop_audio` the loop point,
/// `safezone` and `focus` their own, every check of the finished file (`mp4_*`) the last line.
pub fn step_of_check(name: &str) -> Option<StepKey> {
    Some(match name {
        "safezone" => StepKey::SafeZone,
        "focus" => StepKey::Focus,
        "levels" | "loudness" => StepKey::Loudness,
        "band" => StepKey::Band,
        "flash" => StepKey::Flashes,
        "loop_picture" | "loop_audio" => StepKey::LoopPoint,
        n if n.starts_with("mp4_") => StepKey::Verify,
        _ => return None,
    })
}

/// The line of the check list a stage of the job is: the soundtrack, the frames, the encode and the check of the
/// finished file. The checks before the render fill six lines at once, and the save has none: `None`.
pub fn step_of_stage(name: &str) -> Option<StepKey> {
    Some(match name {
        stage::SOUNDTRACK => StepKey::Soundtrack,
        stage::FRAMES => StepKey::Frames,
        stage::ENCODE => StepKey::Encode,
        stage::VERIFY => StepKey::Verify,
        _ => return None,
    })
}

/// A line of the check list as a failed job's headline names it ("Stopped at the loudness check").
pub fn check_title(key: StepKey) -> &'static str {
    match key {
        StepKey::Soundtrack => "building the soundtrack",
        StepKey::SafeZone => "the safe zone check",
        StepKey::Focus => "the focus check",
        StepKey::Loudness => "the loudness check",
        StepKey::Band => "the fighters-above-the-music check",
        StepKey::Flashes => "the flash check",
        StepKey::LoopPoint => "the loop check",
        StepKey::Frames => "drawing the frames",
        StepKey::Encode => "encoding",
        StepKey::Verify => "the check of the finished video",
    }
}

/// What a job that stopped with `error` tells the owner: what stopped it, the numbers, and what to do next.
/// `failed` is the check that failed when the caller knows it apart from the error; an `ExportError::Check`
/// carries its own.
pub fn failure(error: &ExportError, failed: Option<&CheckResult>) -> JobUpdate {
    let failed = failed.or(match error {
        ExportError::Check(r) => Some(r),
        _ => None,
    });
    if let Some(r) = failed
        && let Some(key) = step_of_check(r.name)
    {
        let (advice, hint) = match key {
            StepKey::Loudness | StepKey::Band => (
                "Open it in Music to see the levels, or try another seed, then queue it again.",
                FailHint::OpenInMusic,
            ),
            StepKey::Verify => (
                "Queue it again; if it stops here twice, try another seed.",
                FailHint::None,
            ),
            _ => ("Try another seed, then queue it again.", FailHint::OpenInSeeds),
        };
        return JobUpdate::Failed {
            headline: format!("Stopped at {}", check_title(key)),
            detail: format!("{} Nothing was saved. {advice}", plain(&r.summary)),
            hint,
        };
    }
    let text = plain(error);
    match error {
        ExportError::Ffmpeg(_) => JobUpdate::Failed {
            headline: "ffmpeg was not found".to_string(),
            detail: "Videos cannot be saved without it. Install ffmpeg, start the app again, then queue it again."
                .to_string(),
            hint: FailHint::InstallFfmpeg,
        },
        // The export's sentence already says whether anything was saved (or where a finished video was left)
        // and what to do: it is the whole detail.
        ExportError::Folder(_) => JobUpdate::Failed {
            headline: "The output folder cannot be used".to_string(),
            detail: text,
            hint: FailHint::CheckFolder,
        },
        ExportError::Encode(_) | ExportError::Frame(_) | ExportError::Clip(_) | ExportError::Cancelled(_) => {
            JobUpdate::Failed {
                headline: NOT_MADE.to_string(),
                detail: text,
                hint: FailHint::None,
            }
        }
        // "This fight cannot be rendered: ...": it does not say so itself.
        ExportError::Content(_) | ExportError::Check(_) => not_made(&text),
    }
}

/// The check list of one job as its events move it on: turns each [`JobEvent`] into the updates the state
/// expects, and nothing it would throw away (a line is sent only when it changes; frame counts ten a second).
#[derive(Default)]
struct Steps {
    /// Where each line is, as last sent (missing: to do).
    at: HashMap<StepKey, StepState>,
    /// The summaries of the checks that passed, by name.
    passed: BTreeMap<&'static str, String>,
    /// When the last frame count was sent.
    frames_sent: Option<Instant>,
}

impl Steps {
    fn state(&self, key: StepKey) -> StepState {
        self.at.get(&key).copied().unwrap_or(StepState::Todo)
    }

    /// Moves `key` to `state` with `note`, unless it is there already or has failed.
    fn set(&mut self, key: StepKey, state: StepState, note: Option<String>, out: &mut Vec<JobUpdate>) {
        let now = self.state(key);
        if now == state || now == StepState::Failed {
            return;
        }
        self.at.insert(key, state);
        out.push(JobUpdate::Step { key, state, note });
    }

    /// The lines in progress, in the check list's order.
    fn now(&self) -> Vec<StepKey> {
        STEP_ORDER
            .into_iter()
            .filter(|&k| self.state(k) == StepState::Now)
            .collect()
    }

    /// The updates for one event of the job, at time `t`.
    fn on(&mut self, e: JobEvent, t: Instant) -> Vec<JobUpdate> {
        let mut out = Vec::new();
        match e {
            JobEvent::Stage(name) => {
                // the lines before it are done
                for key in self.now() {
                    self.set(key, StepState::Done, None, &mut out);
                }
                if name == stage::CHECKS {
                    for key in CHECK_LINES {
                        self.set(key, StepState::Now, None, &mut out);
                    }
                } else if let Some(key) = step_of_stage(name) {
                    self.set(key, StepState::Now, None, &mut out);
                }
            }
            JobEvent::Check(r) => self.check(r, &mut out),
            JobEvent::Progress { done, total, seconds } => {
                let due = self
                    .frames_sent
                    .is_none_or(|last| t.saturating_duration_since(last) >= FRAMES_EVERY);
                if due || done >= total {
                    self.frames_sent = Some(t);
                    out.push(JobUpdate::Frames {
                        done: u32::try_from(done).unwrap_or(u32::MAX),
                        total: u32::try_from(total).unwrap_or(u32::MAX),
                        seconds: if seconds.is_finite() && seconds >= 0.0 {
                            seconds as f32
                        } else {
                            0.0
                        },
                    });
                }
            }
            // The job's own end answers `Done`, with the time it took.
            JobEvent::Done(_) => {}
        }
        out
    }

    /// A check's result: a failure marks its line at once; a line is done once every check it needs passed (the
    /// loudness line needs `levels` and `loudness` and shows the `levels` summary, the loop point both loop
    /// checks and shows the picture's). The finished file's line is done when its stage ends.
    fn check(&mut self, r: CheckResult, out: &mut Vec<JobUpdate>) {
        let Some(key) = step_of_check(r.name) else {
            return;
        };
        if !r.pass {
            self.set(key, StepState::Failed, Some(plain(&r.summary)), out);
            return;
        }
        let needs: &[&str] = match key {
            StepKey::Loudness => &["levels", "loudness"],
            StepKey::LoopPoint => &["loop_picture", "loop_audio"],
            StepKey::Verify => return,
            _ => &[],
        };
        let lead = needs.first().copied().unwrap_or(r.name);
        self.passed.insert(r.name, r.summary);
        if needs.iter().all(|n| self.passed.contains_key(n)) {
            let note = self.passed.get(lead).map(|s| plain(s));
            self.set(key, StepState::Done, note, out);
        }
    }

    /// The video is saved: every line not done yet is done, in the check list's order.
    fn all_done(&mut self) -> Vec<JobUpdate> {
        let mut out = Vec::new();
        for key in STEP_ORDER {
            self.set(key, StepState::Done, None, &mut out);
        }
        out
    }

    /// The job stopped with `e`: a failed check has marked its line already; anything else fails the first line
    /// in progress.
    fn failed_at(&mut self, e: &ExportError) -> Vec<JobUpdate> {
        let mut out = Vec::new();
        if !matches!(e, ExportError::Check(_))
            && let Some(&key) = self.now().first()
        {
            self.set(key, StepState::Failed, None, &mut out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;
    use std::path::Path;
    use std::process::Command as Process;

    use super::*;
    use crate::engine::{EngineConfig, wait_for};
    use crate::model::queue::{JobState, Queue};
    use crate::seam::EngineHandle;
    use crate::store::Paths;

    const EPISODE: &str = include_str!("../../../../fights/flute_vs_trombone.toml");

    /// The frames a test job renders.
    const CLIP: Range<usize> = 0..90;

    /// Every check `bf_export` runs, before the render (`checks::pre_render`) and on the finished file
    /// (`probe::verify_mp4`). `bf_export` keeps the names as private constants, so they are listed here from its
    /// source and its notes.
    const CHECK_NAMES: [&str; 15] = [
        "focus",
        "safezone",
        "levels",
        "band",
        "loudness",
        "flash",
        "loop_picture",
        "loop_audio",
        "mp4_size",
        "mp4_streams",
        "mp4_keyframes",
        "mp4_length",
        "mp4_flash",
        "mp4_loop_picture",
        "mp4_loop_audio",
    ];

    /// The tests that encode do it one at a time (they share the PC with other workers' builds).
    static ENCODES: Mutex<()> = Mutex::new(());

    fn one_encode() -> std::sync::MutexGuard<'static, ()> {
        ENCODES.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Whether ffmpeg can be used here; prints a notice when it cannot.
    fn have_ffmpeg() -> bool {
        match bf_export::find_ffmpeg() {
            Ok(_) => true,
            Err(e) => {
                eprintln!("notice: ffmpeg cannot be used here, so this test checks only what it can: {e}");
                false
            }
        }
    }

    /// An engine whose jobs render only [`CLIP`]: 4 workers, storage under a new empty temp folder (the test
    /// removes it), a waker that does nothing.
    fn clip_engine() -> (EngineHandle, PathBuf) {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("bf-app-jobs-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the test's temp folder");
        let config = EngineConfig {
            workers: 4,
            paths: Paths::under(&root),
            clip: Some(CLIP),
            search_limit: None,
        };
        (super::super::spawn(Arc::new(|| {}), config), root)
    }

    /// The draft of the episode's fight file (seed 392, a K.O. at 50 s).
    fn episode_draft() -> Draft {
        let ff = FightFile::parse(EPISODE).expect("the episode");
        super::super::convert::draft_of(&ff, EPISODE)
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .map(|d| {
                d.filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    fn enqueued(engine: &EngineHandle, draft: Draft, out_root: PathBuf) -> JobSpec {
        engine.send(Command::Enqueue { draft, out_root });
        match wait_for(engine, 10.0, |e| {
            matches!(e, Event::Enqueued { .. } | Event::EnqueueFailed { .. })
        }) {
            Event::Enqueued { spec } => spec,
            other => panic!("{other:?}"),
        }
    }

    /// Every update of job `id` until it ends (`Done`, `Failed` or `Cancelled`).
    fn job_until_end(engine: &EngineHandle, id: JobId, seconds: f64) -> Vec<JobUpdate> {
        let seen = std::cell::RefCell::new(Vec::new());
        wait_for(engine, seconds, |e| match e {
            Event::Job { id: j, update } if *j == id => {
                seen.borrow_mut().push(update.clone());
                matches!(
                    update,
                    JobUpdate::Done { .. } | JobUpdate::Failed { .. } | JobUpdate::Cancelled
                )
            }
            _ => false,
        });
        seen.into_inner()
    }

    /// The places in `STEP_ORDER` of the lines the updates start, in the order they start.
    fn started(updates: &[JobUpdate]) -> Vec<usize> {
        updates
            .iter()
            .filter_map(|u| match u {
                JobUpdate::Step {
                    key,
                    state: StepState::Now,
                    ..
                } => STEP_ORDER.iter().position(|k| k == key),
                _ => None,
            })
            .collect()
    }

    fn result(name: &'static str, pass: bool, summary: &str) -> CheckResult {
        CheckResult {
            name,
            pass,
            summary: summary.to_string(),
            numbers: Default::default(),
        }
    }

    fn spec_of(title: &str) -> JobSpec {
        JobSpec {
            title: title.to_string(),
            colors: [[0; 3]; 2],
            theme: "Big top".to_string(),
            seed: 1,
            fight_toml: String::new(),
            out_root: PathBuf::from("unused"),
        }
    }

    #[test]
    fn every_check_has_a_step() {
        for name in CHECK_NAMES {
            assert!(step_of_check(name).is_some(), "{name}");
        }
        // the finished file's checks are all the last line, never the flashes or the loop point
        for name in CHECK_NAMES.iter().filter(|n| n.starts_with("mp4_")) {
            assert_eq!(step_of_check(name), Some(StepKey::Verify), "{name}");
        }
        assert_eq!(step_of_check("levels"), Some(StepKey::Loudness));
        assert_eq!(step_of_check("loudness"), Some(StepKey::Loudness));
        assert_eq!(step_of_check("band"), Some(StepKey::Band));
        assert_eq!(step_of_check("flash"), Some(StepKey::Flashes));
        assert_eq!(step_of_check("unheard of"), None);
        for s in stage::ALL {
            let want = matches!(s, "soundtrack" | "frames" | "encode" | "verify");
            assert_eq!(step_of_stage(s).is_some(), want, "{s}");
        }
        assert_eq!(check_title(StepKey::Band), "the fighters-above-the-music check");
        assert_eq!(check_title(StepKey::Verify), "the check of the finished video");
    }

    // Review Focus 4
    #[test]
    fn a_failed_check_is_named_with_its_numbers() {
        let summary = "The opening is 2.1 dB quieter than the ending, and the limit is 1.5 dB.";
        let r = result("levels", false, summary);
        let JobUpdate::Failed { headline, detail, hint } = failure(&ExportError::Check(r.clone()), Some(&r)) else {
            panic!("not a failure");
        };
        assert_eq!(headline, "Stopped at the loudness check");
        assert!(
            detail.starts_with(summary) && detail.contains("Nothing was saved."),
            "{detail}"
        );
        assert_eq!(hint, FailHint::OpenInMusic);
        assert_eq!(
            detail,
            format!(
                "{summary} Nothing was saved. Open it in Music to see the levels, or try another seed, then queue it \
                 again."
            )
        );
        // the other kinds
        let r = result("safezone", false, "A caption sticks out at 3.20 s.");
        let JobUpdate::Failed { headline, hint, .. } = failure(&ExportError::Check(r), None) else {
            panic!("not a failure")
        };
        assert_eq!(
            (headline.as_str(), hint),
            ("Stopped at the safe zone check", FailHint::OpenInSeeds)
        );
        let r = result("mp4_length", false, "The video is 2 s short.");
        let JobUpdate::Failed { detail, hint, .. } = failure(&ExportError::Check(r), None) else {
            panic!("not a failure")
        };
        assert!(detail.ends_with("Queue it again; if it stops here twice, try another seed."));
        assert_eq!(hint, FailHint::None);
        let JobUpdate::Failed { headline, detail, hint } = failure(&ExportError::Ffmpeg("gone.".into()), None) else {
            panic!("not a failure")
        };
        assert_eq!(
            (headline.as_str(), detail.as_str(), hint),
            (
                "ffmpeg was not found",
                "Videos cannot be saved without it. Install ffmpeg, start the app again, then queue it again.",
                FailHint::InstallFfmpeg
            )
        );
        // The export's own sentence is the whole detail where it already says what was saved: once, and true.
        let said = |e: ExportError| match failure(&e, None) {
            JobUpdate::Failed { headline, detail, hint } => (headline, detail, hint),
            other => panic!("{other:?}"),
        };
        let encode = "ffmpeg stopped while it encoded the video (x), so nothing was saved; check that the drive has \
                      free space, then try again.";
        assert_eq!(
            said(ExportError::Encode(encode.into())),
            (NOT_MADE.to_string(), plain(&encode), FailHint::None)
        );
        let frame = "Frame 17 (at 0.57 s) could not be drawn (x), so nothing was saved; this is a fault in the app, \
                     not in your fight.";
        assert_eq!(said(ExportError::Frame(frame.into())).1, frame);
        let clip = "The frames 5 to 2 are not a stretch of the video's 1830 frames (0 to 1829), so nothing was made; \
                    ask for frames inside it.";
        assert_eq!(said(ExportError::Clip(clip.into())).1, clip);
        // a finished video left behind is never called "nothing was saved"
        let left = "A folder named X appeared in the output folder while the video was made, so the finished video \
                    was left in Y; move it out of there yourself.";
        let (headline, detail, hint) = said(ExportError::Folder(left.into()));
        assert_eq!(
            (headline.as_str(), detail.as_str(), hint),
            ("The output folder cannot be used", left, FailHint::CheckFolder)
        );
        let taken = "Both A and B are already taken in the output folder, so nothing was made; move or rename one of \
                     them, or choose another folder.";
        assert_eq!(said(ExportError::Folder(taken.into())).1, taken);
        // the content's sentence does not say it, so the detail adds it
        let content = "This fight cannot be rendered: no theme polka.";
        assert_eq!(
            said(ExportError::Content(content.into())).1,
            format!("{content} Nothing was saved.")
        );
        for e in [
            ExportError::Encode(encode.into()),
            ExportError::Frame(frame.into()),
            ExportError::Clip(clip.into()),
            ExportError::Folder(left.into()),
            ExportError::Folder(taken.into()),
            ExportError::Content(content.into()),
        ] {
            let detail = said(e).1.to_lowercase();
            let saying = detail.matches("nothing was saved").count() + detail.matches("nothing was made").count();
            assert!(saying <= 1, "{detail}");
        }
    }

    #[test]
    fn a_cancel_that_left_a_folder_says_so() {
        assert_eq!(cancel_problem(&ExportError::cancelled()), None);
        let left =
            ExportError::cancelled().with_clause("the temporary folder X could not be removed, so delete it yourself");
        assert_eq!(
            cancel_problem(&left).as_deref(),
            Some(
                "The render was cancelled, so nothing was saved, and the temporary folder X could not be removed, so \
                 delete it yourself."
            )
        );
        // a failure that came with the cancel is told too
        assert!(cancel_problem(&ExportError::Encode("ffmpeg stopped.".into())).is_some());
    }

    #[test]
    fn events_become_the_check_list() {
        let t = Instant::now();
        let mut s = Steps::default();
        let mut all = Vec::new();
        let at = |ms: u64| t + Duration::from_millis(ms);
        all.extend(s.on(JobEvent::Stage(stage::SOUNDTRACK), at(0)));
        all.extend(s.on(JobEvent::Stage(stage::CHECKS), at(0)));
        for name in &CHECK_NAMES[..8] {
            all.extend(s.on(JobEvent::Check(result(name, true, &format!("{name} fine"))), at(0)));
        }
        all.extend(s.on(JobEvent::Stage(stage::FRAMES), at(0)));
        // 100 counts in 1 s: about ten are sent, and the last one always
        for done in 1..=100_usize {
            let e = JobEvent::Progress {
                done,
                total: 100,
                seconds: done as f64 / 100.0,
            };
            all.extend(s.on(e, at(done as u64 * 10)));
        }
        all.extend(s.on(JobEvent::Stage(stage::ENCODE), at(1000)));
        all.extend(s.on(JobEvent::Stage(stage::VERIFY), at(1000)));
        all.extend(s.on(JobEvent::Check(result("mp4_size", true, "fine")), at(1000)));
        all.extend(s.on(JobEvent::Stage(stage::SAVE), at(1000)));
        all.extend(s.all_done());

        let frames: Vec<u32> = all
            .iter()
            .filter_map(|u| match u {
                JobUpdate::Frames { done, .. } => Some(*done),
                _ => None,
            })
            .collect();
        assert!((9..=12).contains(&frames.len()), "{frames:?}");
        assert_eq!(frames.last(), Some(&100));
        // the lines start in the check list's order, each once
        let order = started(&all);
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
        assert_eq!(order.len(), 10);
        // applied to a queued job, every line ends done; the loudness line carries the levels summary
        let mut q = Queue::new();
        let id = q.add(spec_of("x"));
        q.start(id);
        for u in all {
            q.apply(id, u);
        }
        let job = q.job(id).unwrap();
        assert!(job.steps.iter().all(|l| l.state == StepState::Done), "{:?}", job.steps);
        let loud = job.steps.iter().find(|l| l.key == StepKey::Loudness).unwrap();
        assert_eq!(loud.note.as_deref(), Some("Levels fine."));

        // a failed check marks its line; the loudness line waits for both of its checks
        let mut s = Steps::default();
        s.on(JobEvent::Stage(stage::CHECKS), t);
        let u = s.on(JobEvent::Check(result("levels", true, "ok")), t);
        assert!(u.is_empty(), "{u:?}");
        let u = s.on(JobEvent::Check(result("loop_picture", false, "A jump at the seam.")), t);
        assert_eq!(
            u,
            [JobUpdate::Step {
                key: StepKey::LoopPoint,
                state: StepState::Failed,
                note: Some("A jump at the seam.".to_string())
            }]
        );
        assert!(s.on(JobEvent::Check(result("loop_audio", true, "ok")), t).is_empty());
        let u = s.on(JobEvent::Check(result("loudness", true, "measured")), t);
        assert_eq!(
            u,
            [JobUpdate::Step {
                key: StepKey::Loudness,
                state: StepState::Done,
                note: Some("Ok.".to_string())
            }]
        );
        // an error that is not a check fails the first line in progress
        let mut s = Steps::default();
        s.on(JobEvent::Stage(stage::FRAMES), t);
        assert_eq!(
            s.failed_at(&ExportError::Encode("x".into())),
            [JobUpdate::Step {
                key: StepKey::Frames,
                state: StepState::Failed,
                note: None
            }]
        );
    }

    #[test]
    fn enqueue_needs_a_seed() {
        let (engine, root) = clip_engine();
        let mut d = super::super::tests::starting_draft();
        d.seed = None;
        engine.send(Command::Enqueue {
            draft: d.clone(),
            out_root: root.join("out"),
        });
        let Event::EnqueueFailed { problem } = wait_for(&engine, 5.0, |e| matches!(e, Event::EnqueueFailed { .. }))
        else {
            unreachable!()
        };
        assert_eq!(problem, "Pick a seed first.");
        // with a seed it is queued, fixed as it is now
        d.seed = Some(392);
        let spec = enqueued(&engine, d.clone(), root.join("out"));
        assert_eq!(
            (spec.title.as_str(), spec.theme.as_str(), spec.seed),
            ("Flute vs Trombone", "Big top", 392)
        );
        assert_eq!(spec.colors, [[222, 232, 246], [120, 240, 184]]);
        assert_eq!(spec.out_root, root.join("out"));
        let ff = FightFile::parse(&spec.fight_toml).expect("its fight file");
        assert_eq!(ff, fight_file(&d).expect("the draft's fight file"));
        // a draft that cannot fight says why
        d.handicap[0] = 0.0;
        engine.send(Command::Enqueue {
            draft: d,
            out_root: root.join("out"),
        });
        let Event::EnqueueFailed { problem } = wait_for(&engine, 5.0, |e| matches!(e, Event::EnqueueFailed { .. }))
        else {
            unreachable!()
        };
        assert!(problem.starts_with("This fight cannot be used: "), "{problem}");
        engine.send(Command::Shutdown);
        std::fs::remove_dir(&root).expect("nothing was written");
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine:: -- --include-ignored"
    )]
    fn a_queued_fight_renders_to_a_folder() {
        let _one = one_encode();
        let (engine, root) = clip_engine();
        let out = root.join("Videos");
        let spec = enqueued(&engine, episode_draft(), out.clone());
        if have_ffmpeg() {
            engine.send(Command::RunJob {
                id: 7,
                fight_toml: spec.fight_toml.clone(),
                out_root: out.clone(),
            });
            let updates = job_until_end(&engine, 7, 240.0);
            let Some(JobUpdate::Done { folder, seconds }) = updates.last().cloned() else {
                panic!("{updates:?}");
            };
            assert!(seconds > 0.0 && seconds.is_finite());
            // the lines start in the check list's order, the frames are counted, and every line ends done
            let order = started(&updates);
            assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
            let frames: Vec<(u32, u32)> = updates
                .iter()
                .filter_map(|u| match u {
                    JobUpdate::Frames { done, total, .. } => Some((*done, *total)),
                    _ => None,
                })
                .collect();
            assert!(!frames.is_empty());
            assert_eq!(frames.last(), Some(&(90, 90)));
            eprintln!("{} frame counts for 90 frames", frames.len());
            let mut q = Queue::new();
            let id = q.add(spec.clone());
            q.start(id);
            for u in updates.clone() {
                q.apply(id, u);
            }
            let job = q.job(id).unwrap();
            assert!(matches!(job.state, JobState::Done { .. }), "{:?}", job.state);
            assert!(job.steps.iter().all(|l| l.state == StepState::Done), "{:?}", job.steps);
            // a clip's folder holds only its video (ruling R41: no cover, caption or fight file)
            let name = "flute_vs_trombone_frames0-90";
            assert_eq!(folder, std::path::absolute(&out).unwrap().join(name));
            assert_eq!(entries(&folder), [format!("{name}.mp4")]);
            assert_eq!(entries(&out), [name]);
        }
        engine.send(Command::Shutdown);
        let _ = std::fs::remove_dir_all(&root);
    }

    // Review Focus 1
    #[test]
    fn a_root_that_cannot_be_written_fails_in_words() {
        let (engine, root) = clip_engine();
        let file = root.join("a file");
        std::fs::write(&file, b"not a folder").unwrap();
        let out = file.join("Videos");
        let spec = enqueued(&engine, episode_draft(), out.clone());
        engine.send(Command::RunJob {
            id: 3,
            fight_toml: spec.fight_toml,
            out_root: out,
        });
        let updates = job_until_end(&engine, 3, 60.0);
        let Some(JobUpdate::Failed { headline, detail, hint }) = updates.last().cloned() else {
            panic!("{updates:?}");
        };
        if have_ffmpeg() {
            assert_eq!(headline, "The output folder cannot be used");
            // the export's sentence, which says once that nothing was saved and what to do
            assert!(detail.starts_with("The output folder "), "{detail}");
            assert_eq!(
                detail.to_lowercase().matches("nothing was saved").count(),
                1,
                "{detail}"
            );
            assert!(detail.contains("choose another folder"), "{detail}");
            eprintln!("folder failure: {detail}");
            assert_eq!(hint, FailHint::CheckFolder);
        } else {
            assert_eq!(hint, FailHint::InstallFfmpeg);
        }
        // nothing is left behind
        assert_eq!(entries(&root), ["a file"]);
        engine.send(Command::Shutdown);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The marker that tells [`missing_ffmpeg_child`] it runs in the child process.
    const CHILD: &str = "BF_APP_JOBS_MISSING_FFMPEG_CHILD";

    /// `BALLFIGHT_FFMPEG` belongs to the whole process, and every other test runs in this one, so this test never
    /// sets it here: it runs [`missing_ffmpeg_child`] in a new process of this same test program, with the variable
    /// set for that process only (`Command::env`), where no other test can see it.
    #[test]
    fn missing_ffmpeg_fails_in_words() {
        let exe = std::env::current_exe().expect("this test program");
        let mut child = Process::new(exe)
            .args([
                "engine::jobs::tests::missing_ffmpeg_child",
                "--exact",
                "--include-ignored",
                "--test-threads=1",
                "--nocapture",
            ])
            .env("BALLFIGHT_FFMPEG", "/nonexistent")
            .env(CHILD, "1")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("the child test started");
        // Its output is read on threads of their own, so a full pipe can never stall it.
        let read = |pipe: Option<Box<dyn std::io::Read + Send>>| {
            std::thread::spawn(move || {
                let mut text = String::new();
                if let Some(mut p) = pipe {
                    let _ = p.read_to_string(&mut text);
                }
                text
            })
        };
        let stdout = read(
            child
                .stdout
                .take()
                .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
        );
        let stderr = read(
            child
                .stderr
                .take()
                .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
        );
        let deadline = Instant::now() + Duration::from_secs(180);
        let status = loop {
            if let Some(status) = child.try_wait().expect("the child can be waited for") {
                break status;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("the child test did not end within 180 s, so it was stopped");
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        let said = format!(
            "{}\n{}",
            stdout.join().unwrap_or_default(),
            stderr.join().unwrap_or_default()
        );
        assert!(status.success(), "{said}");
        assert!(said.contains("1 passed"), "{said}");
        assert!(said.contains(&format!("{CHILD}: every check ran")), "{said}");
    }

    /// The other half of [`missing_ffmpeg_fails_in_words`]; it does nothing in an ordinary run.
    #[test]
    #[ignore = "run by missing_ffmpeg_fails_in_words in a process of its own"]
    fn missing_ffmpeg_child() {
        if std::env::var_os(CHILD).is_none() {
            return;
        }
        let (engine, root) = clip_engine();
        engine.send(Command::CheckTools);
        let Event::Tools { ffmpeg } = wait_for(&engine, 30.0, |e| matches!(e, Event::Tools { .. })) else {
            unreachable!()
        };
        assert_eq!(ffmpeg, Err("ffmpeg was not found on this PC.".to_string()));
        let out = root.join("Videos");
        let spec = enqueued(&engine, episode_draft(), out.clone());
        engine.send(Command::RunJob {
            id: 1,
            fight_toml: spec.fight_toml,
            out_root: out.clone(),
        });
        let updates = job_until_end(&engine, 1, 60.0);
        let Some(JobUpdate::Failed { headline, hint, .. }) = updates.last().cloned() else {
            panic!("{updates:?}");
        };
        assert_eq!(
            (headline.as_str(), hint),
            ("ffmpeg was not found", FailHint::InstallFfmpeg)
        );
        assert!(!out.exists(), "nothing was made");
        engine.send(Command::Shutdown);
        std::fs::remove_dir(&root).expect("nothing was written");
        println!("{CHILD}: every check ran");
    }

    #[test]
    fn tools_are_named() {
        let (engine, root) = clip_engine();
        engine.send(Command::CheckTools);
        let Event::Tools { ffmpeg } = wait_for(&engine, 30.0, |e| matches!(e, Event::Tools { .. })) else {
            unreachable!()
        };
        match bf_export::find_ffmpeg() {
            Ok(f) => {
                assert!(f.version.starts_with("ffmpeg "), "{}", f.version);
                assert_eq!(ffmpeg, Ok(f.version));
            }
            Err(_) => assert_eq!(ffmpeg, Err(NO_FFMPEG.to_string())),
        }
        engine.send(Command::Shutdown);
        std::fs::remove_dir(&root).expect("nothing was written");
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "slow in debug: cargo test -p bf-app --release --lib engine:: -- --include-ignored"
    )]
    fn cancel_leaves_nothing() {
        let _one = one_encode();
        let (engine, root) = clip_engine();
        let out = root.join("Videos");
        let spec = enqueued(&engine, episode_draft(), out.clone());
        if have_ffmpeg() {
            engine.send(Command::RunJob {
                id: 5,
                fight_toml: spec.fight_toml,
                out_root: out.clone(),
            });
            wait_for(&engine, 120.0, |e| {
                matches!(
                    e,
                    Event::Job {
                        id: 5,
                        update: JobUpdate::Frames { .. }
                    }
                )
            });
            let asked = Instant::now();
            engine.send(Command::CancelJob { id: 5 });
            let updates = job_until_end(&engine, 5, 30.0);
            assert_eq!(updates.last(), Some(&JobUpdate::Cancelled), "{updates:?}");
            assert!(
                updates
                    .iter()
                    .all(|u| !matches!(u, JobUpdate::Done { .. } | JobUpdate::Failed { .. })),
                "{updates:?}"
            );
            eprintln!(
                "cancelled {:.2} s after the cancel was sent",
                asked.elapsed().as_secs_f64()
            );
            // bf-export removed its hidden temporary folder before the answer
            assert_eq!(entries(&out), Vec::<String>::new());
            // nothing more comes for it
            engine.send(Command::CheckTools);
            wait_for(&engine, 30.0, |e| {
                assert!(!matches!(e, Event::Job { id: 5, .. }), "{e:?}");
                matches!(e, Event::Tools { .. })
            });
        }
        engine.send(Command::Shutdown);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_file_that_cannot_be_fought_fails_in_words() {
        let (engine, root) = clip_engine();
        engine.send(Command::RunJob {
            id: 1,
            fight_toml: "not a fight file".to_string(),
            out_root: root.join("out"),
        });
        let updates = job_until_end(&engine, 1, 30.0);
        let [JobUpdate::Failed { headline, detail, hint }] = &updates[..] else {
            panic!("{updates:?}");
        };
        assert_eq!(
            (headline.as_str(), *hint),
            ("The video could not be made", FailHint::None)
        );
        assert!(
            detail.starts_with("This fight cannot be used: ") && detail.ends_with(" Nothing was saved."),
            "{detail}"
        );
        // cancelling a job that is not running does nothing
        engine.send(Command::CancelJob { id: 99 });
        engine.send(Command::Shutdown);
        std::fs::remove_dir(&root).expect("nothing was written");
    }

    #[test]
    fn a_replaced_job_gets_one_last_word() {
        // Without a coordinator: the handler and a context of its own.
        let (engine, side) = crate::seam::channel(Arc::new(|| {}));
        let root = std::env::temp_dir().join(format!("bf-app-jobs-replaced-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let config = EngineConfig {
            workers: 1,
            paths: Paths::under(&root),
            clip: Some(CLIP),
            search_limit: None,
        };
        let ctx = Ctx::new(side.events.clone(), config);
        let mut state = State::default();
        let toml = fight_file(&episode_draft()).expect("the episode").to_toml();
        let mut flags = Vec::new();
        for id in [1, 2] {
            handle_job(&mut state, &ctx, id, &toml, &root.join("out"));
            flags.push(state.running.last().expect("a running job").cancel.clone());
        }
        // the first one was replaced: its flag is set, and only the second is running
        assert!(flags[0].load(Ordering::SeqCst));
        assert_eq!(state.running.len(), 1);
        assert_eq!(state.running[0].id, 2);
        handle(&mut state, &ctx, Command::CancelJob { id: 2 });
        // wait until both jobs' threads have let go of their flags (they have ended); the test then holds the only
        // clones
        drop(state);
        let deadline = Instant::now() + Duration::from_secs(60);
        while flags.iter().any(|f| Arc::strong_count(f) > 1) {
            assert!(Instant::now() < deadline, "a job's thread did not end");
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut last_words: BTreeMap<JobId, Vec<JobUpdate>> = BTreeMap::new();
        while let Some(e) = engine.poll() {
            if let Event::Job { id, update } = e
                && matches!(
                    update,
                    JobUpdate::Done { .. } | JobUpdate::Failed { .. } | JobUpdate::Cancelled
                )
            {
                last_words.entry(id).or_default().push(update);
            }
        }
        // exactly one each: the replaced job's `Cancelled`, sent when it was replaced, and the second's own
        assert_eq!(last_words.get(&1), Some(&vec![JobUpdate::Cancelled]), "{last_words:?}");
        assert_eq!(last_words.get(&2).map(Vec::len), Some(1), "{last_words:?}");
        if have_ffmpeg() {
            assert_eq!(last_words[&2], [JobUpdate::Cancelled]);
        }
        // no video folder was left
        assert_eq!(entries(&root.join("out")), Vec::<String>::new());
        ctx.shutdown();
        let _ = std::fs::remove_dir_all(&root);
    }

    fn handle_job(state: &mut State, ctx: &Ctx, id: JobId, toml: &str, out: &Path) {
        handle(
            state,
            ctx,
            Command::RunJob {
                id,
                fight_toml: toml.to_string(),
                out_root: out.to_path_buf(),
            },
        );
    }
}
