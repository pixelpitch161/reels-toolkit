//! The preview's benchmark (`ballfight-studio --preview-bench`): plays a take the way the app does, with no window
//! and no sound, and measures it against the plan's "Performance budget for the preview".
//!
//! [`run`] drives a real [`Player`] on the wall clock: it loads the take at `start` with autoplay, calls `tick()`
//! every 4 ms for `seconds`, and counts what the player counts (`shown`, `late`). It then pauses, lets the draws in
//! progress finish, and asks for 50 frames that were never drawn, one after another, through `ScrubTo`, timing each
//! from the request until `tick()` gives that frame (the interface repaints on the scheduler's waker; here `tick()`
//! is called every millisecond instead).
//!
//! Every draw is timed by a thin wrapper around the take's frame source, which hands on the source's own frame
//! untouched, so the bench sees the same draw times the scheduler does:
//! - `frame_ms` is the mean of the last 30 draws of the playback, which is what the scheduler feeds to
//!   [`pace`]; `lead` and `stride` are `pace(frame_ms, threads)`, the pace it was drawing at when playback ended;
//! - `frame_ms_p95` is the 95th percentile of the scrub draws, each drawn while no other frame was being drawn
//!   (one frame on one thread, as milestone 1 measured `frame_ms_p95`); `play_ms_p95` is that of the playback's
//!   draws, which share the PC's cores with each other;
//! - `first_picture_ms` is the time from the `Load` until the first picture is given, and `late_after_start` the
//!   frames counted late after it, which tells playback's own misses from those of its start (the clock runs from
//!   the `Load`, while a fresh take's first draws are still going on).
//!
//! When every frame was drawn during playback (a run as long as the video), there is nothing left to scrub to and
//! the three scrub figures are 0.
//!
//! [`starter_take`] makes the take through the real engine, the way the app asks for one: `engine::spawn`, then
//! `LoadTake`, timing `TakePicture` and `TakeSound`.

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::engine::{self, EngineConfig, content};
use crate::model::draft::Draft;
use crate::model::queue::Queue;
use crate::model::take::{Frame, FrameSource, PlayerCmd, TransportAction};
use crate::model::{FPS, TakeId};
use crate::preview::audio::NoAudio;
use crate::preview::player::{Now, Player};
use crate::preview::scheduler::pace;
use crate::seam::{Command, Event};
use crate::state::AppState;
use crate::store::Paths;
use crate::store::settings::default_settings;

/// How often the playback loop calls `tick()`.
const TICK: Duration = Duration::from_millis(4);
/// How often a scrub's wait calls `tick()`.
const SCRUB_TICK: Duration = Duration::from_millis(1);
/// How many frames are scrubbed to.
const SCRUBS: usize = 50;
/// How many of the latest draws `frame_ms` averages (as the scheduler does).
const TIMES: usize = 30;
/// The longest the bench waits for one scrubbed frame, or for the draws in progress to end.
const PATIENCE: Duration = Duration::from_secs(30);
/// The longest the bench waits for the take's picture and for its sound.
const TAKE_WAIT: Duration = Duration::from_secs(180);
/// The handicap of the starter fight the bench plays (the kit's handicap for this pair).
pub const HANDICAP: [f64; 2] = [2.0, 0.4];

/// What one bench run measured. The first nine fields are the plan's; the rest say what they rest on and what the
/// budget tests read (`frame_ms_p95`, `start`, `end_frame`).
#[derive(Clone, Debug, PartialEq)]
pub struct BenchReport {
    /// The mean of the playback's last 30 draws, in ms: what the scheduler paces itself by.
    pub frame_ms: f32,
    /// The preview's drawing threads.
    pub threads: usize,
    /// `pace(frame_ms, threads)`: how far ahead the first wanted frame is, and how many frames apart they are.
    pub lead: u32,
    pub stride: u32,
    /// What the player counted over the playback (`PlayerStats`).
    pub shown: u64,
    pub late: u64,
    /// `shown / (shown + late)`.
    pub on_time: f32,
    /// From a scrub's request until `tick()` gave the asked frame, in ms.
    pub scrub_median_ms: f32,
    pub scrub_worst_ms: f32,
    /// The 95th percentile of the scrub draws (one frame on one thread), in ms.
    pub frame_ms_p95: f32,
    /// The 95th percentile of the playback's draws (many at once), in ms.
    pub play_ms_p95: f32,
    /// Frames drawn during the playback.
    pub drawn: u64,
    /// The frame playback started from, and the playhead's frame when it ended.
    pub start: u32,
    pub end_frame: u32,
    /// From the `Load` until the first picture of the take was given, in ms (infinite if none was).
    pub first_picture_ms: f32,
    /// Frames counted late after that first picture: playback's own misses, without its start.
    pub late_after_start: u64,
}

impl BenchReport {
    /// One line per field, `name value`.
    pub fn lines(&self) -> Vec<String> {
        vec![
            format!("frame_ms {:.1}", self.frame_ms),
            format!("threads {}", self.threads),
            format!("lead {}", self.lead),
            format!("stride {}", self.stride),
            format!("shown {}", self.shown),
            format!("late {}", self.late),
            format!("on_time {:.4}", self.on_time),
            format!("scrub_median_ms {:.1}", self.scrub_median_ms),
            format!("scrub_worst_ms {:.1}", self.scrub_worst_ms),
            format!("frame_ms_p95 {:.1}", self.frame_ms_p95),
            format!("play_ms_p95 {:.1}", self.play_ms_p95),
            format!("drawn {}", self.drawn),
            format!("start {}", self.start),
            format!("end_frame {}", self.end_frame),
            format!("first_picture_ms {:.1}", self.first_picture_ms),
            format!("late_after_start {}", self.late_after_start),
        ]
    }
}

/// One timed draw.
#[derive(Clone, Copy, Debug)]
struct Draw {
    f: u32,
    ms: f32,
}

/// The take's frame source with every draw timed. The frames are the source's own.
struct Timed {
    inner: Arc<dyn FrameSource>,
    /// Finished draws, in the order they finished.
    log: Mutex<Vec<Draw>>,
    /// Draws in progress.
    drawing: AtomicUsize,
}

/// Counts a draw out when it ends, by a panic too (a frame that could not be drawn).
struct Out<'a>(&'a AtomicUsize);

impl Drop for Out<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Timed {
    fn log(&self) -> Vec<Draw> {
        self.log.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

impl FrameSource for Timed {
    fn frames(&self) -> u32 {
        self.inner.frames()
    }

    fn draw(&self, f: u32) -> Frame {
        self.drawing.fetch_add(1, Ordering::SeqCst);
        let _out = Out(&self.drawing);
        let began = Instant::now();
        let frame = self.inner.draw(f);
        let ms = began.elapsed().as_secs_f32() * 1000.0;
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Draw { f, ms });
        frame
    }
}

/// The `q`-th percentile of `v` with linear interpolation between neighbours (numpy's default, as `ballfight
/// bench` computes it); 0 for no values.
fn percentile(v: &[f32], q: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(f32::total_cmp);
    let at = q / 100.0 * (s.len() - 1) as f32;
    let (lo, hi) = (at.floor() as usize, at.ceil() as usize);
    s[lo] + (s[hi] - s[lo]) * (at - lo as f32)
}

/// Waits, ticking, until no draw is in progress (at most `PATIENCE`).
fn settle(player: &mut Player, timed: &Timed) {
    let until = Instant::now() + PATIENCE;
    while timed.drawing.load(Ordering::SeqCst) > 0 && Instant::now() < until {
        player.tick();
        std::thread::sleep(SCRUB_TICK);
    }
}

/// The frames to scrub to: `n` frames below `frames` not in `drawn`, picked by a fixed sequence of numbers (the
/// same frames on every run of the same take), in the order they are asked.
fn scrub_frames(frames: u32, drawn: &HashSet<u32>, n: usize) -> Vec<u32> {
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut out: Vec<u32> = Vec::with_capacity(n);
    let free = (0..frames).filter(|f| !drawn.contains(f)).count();
    while out.len() < n.min(free) {
        // Knuth's 64-bit linear congruential step.
        x = x
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let f = ((x >> 33) % u64::from(frames.max(1))) as u32;
        if !drawn.contains(&f) && !out.contains(&f) {
            out.push(f);
        }
    }
    out
}

/// Plays `frames` from `start` for `seconds` on `threads` drawing threads with no sound, then scrubs to 50 frames
/// one after another (see the module's description).
pub fn run(frames: Arc<dyn FrameSource>, start: u32, seconds: f32, threads: usize) -> BenchReport {
    let count = frames.frames();
    let timed = Arc::new(Timed {
        inner: frames,
        log: Mutex::new(Vec::new()),
        drawing: AtomicUsize::new(0),
    });
    let origin = Instant::now();
    let now: Now = Box::new(move || origin.elapsed());
    let mut player = Player::new(
        threads,
        Box::new(NoAudio::new("The bench plays without sound.")),
        now,
        Arc::new(|| {}),
    );
    const TAKE: TakeId = 1;

    // Playback, from `start`, as the app plays a take it has just been given.
    let length = Duration::from_secs_f32(if seconds.is_finite() { seconds.max(0.0) } else { 0.0 });
    let began = Instant::now();
    player.run(PlayerCmd::Load {
        take: TAKE,
        frames: timed.clone(),
        start,
        autoplay: true,
    });
    // When the first picture was given, and how many frames were late by then.
    let mut first: Option<(Duration, u64)> = None;
    while began.elapsed() < length {
        player.tick();
        if first.is_none() && player.has_picture() {
            first = Some((began.elapsed(), player.stats().late));
        }
        std::thread::sleep(TICK);
    }
    player.tick();
    let end_frame = player.view().frame;
    let stats = player.stats();
    player.run(PlayerCmd::Transport(TransportAction::PlayPause));
    settle(&mut player, &timed);
    let played = timed.log();
    let play_ms: Vec<f32> = played.iter().map(|d| d.ms).collect();
    let last = &play_ms[play_ms.len().saturating_sub(TIMES)..];
    let frame_ms = if last.is_empty() {
        0.0
    } else {
        last.iter().sum::<f32>() / last.len() as f32
    };
    let (lead, stride) = pace(frame_ms, threads);

    // Scrubs, each to a frame that was never drawn.
    let drawn: HashSet<u32> = played.iter().map(|d| d.f).collect();
    let mut waits = Vec::with_capacity(SCRUBS);
    for f in scrub_frames(count, &drawn, SCRUBS) {
        let before = player.stats().shown;
        let asked = Instant::now();
        player.run(PlayerCmd::Transport(TransportAction::ScrubTo(
            (f as f32 + 0.5) / FPS as f32,
        )));
        loop {
            player.tick();
            let given = player.stats().shown > before;
            if given || player.failed_frame() == Some(f) || asked.elapsed() > PATIENCE {
                break;
            }
            std::thread::sleep(SCRUB_TICK);
        }
        waits.push(asked.elapsed().as_secs_f32() * 1000.0);
    }
    player.run(PlayerCmd::Transport(TransportAction::ScrubEnd));
    settle(&mut player, &timed);
    let scrub_ms: Vec<f32> = timed.log()[played.len()..].iter().map(|d| d.ms).collect();

    let counted = stats.shown + stats.late;
    BenchReport {
        frame_ms,
        threads,
        lead,
        stride,
        shown: stats.shown,
        late: stats.late,
        on_time: if counted == 0 {
            0.0
        } else {
            stats.shown as f32 / counted as f32
        },
        scrub_median_ms: percentile(&waits, 50.0),
        scrub_worst_ms: waits.iter().copied().fold(0.0, f32::max),
        frame_ms_p95: percentile(&scrub_ms, 95.0),
        play_ms_p95: percentile(&play_ms, 95.0),
        drawn: played.len() as u64,
        start,
        end_frame,
        first_picture_ms: first.map_or(f32::INFINITY, |(t, _)| t.as_secs_f32() * 1000.0),
        late_after_start: first.map_or(stats.late, |(_, late)| stats.late - late),
    }
}

/// The take the bench plays, and how long the engine took to give it.
pub struct Take {
    pub seed: u64,
    pub frames: Arc<dyn FrameSource>,
    /// The frame of FIGHT!, where the app starts playing a take.
    pub start: u32,
    /// From `LoadTake` to `TakePicture`, and to `TakeSound`.
    pub to_picture: Duration,
    pub to_sound: Duration,
}

/// The threads the app gives the preview on this PC: the settings' default `workers`, the PC's threads minus one.
pub fn workers() -> usize {
    default_settings(cores()).workers
}

/// The PC's threads, as the app counts them.
pub fn cores() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
}

/// The draft the app starts from with the real roster, under [`HANDICAP`].
fn starter_draft() -> Draft {
    let (mut state, _) = AppState::new(default_settings(cores()), Queue::new(), Vec::new(), cores());
    state.apply(Event::Roster(content::roster()));
    let mut draft = state.vm.draft.clone();
    draft.handicap = HANDICAP;
    draft
}

/// The first seed from 1 whose fight under `draft` fits the story, by the search's own test.
fn first_fitting_seed(draft: &Draft) -> Result<u64, String> {
    let ff = engine::convert::fight_file(draft)?;
    let base =
        bf_sim::setup_from(&ff, bf_content::roster(), 0).map_err(|e| format!("This fight cannot be fought: {e}"))?;
    let sheet = bf_story::BeatSheet::series(base.fighters);
    for seed in 1..10_000 {
        let setup = bf_sim::Setup { seed, ..base.clone() };
        let c = bf_story::fight(&setup, &sheet).map_err(|e| format!("Seed {seed} cannot be fought: {e}"))?;
        if c.verdict.fits {
            return Ok(seed);
        }
    }
    Err("No seed under 10,000 fits the story.".to_string())
}

/// The starter fight's take (FLUTE against TROMBONE at [`HANDICAP`]) for `seed`, or for the first fitting seed,
/// made by the real engine through `LoadTake`. Its files go in a temp folder of its own, removed before this
/// returns. Waits for the sound too, so the bench then plays on a PC the take no longer works on.
pub fn starter_take(seed: Option<u64>) -> Result<Take, String> {
    let mut draft = starter_draft();
    let seed = match seed {
        Some(s) => s,
        None => first_fitting_seed(&draft)?,
    };
    draft.seed = Some(seed);
    let root = std::env::temp_dir().join(format!("bf-preview-bench-{}", std::process::id()));
    std::fs::create_dir_all(&root).map_err(|_| "The bench could not make its temp folder.".to_string())?;
    let config = EngineConfig {
        workers: workers(),
        paths: Paths::under(&root),
        clip: None,
        search_limit: None,
    };
    let handle = engine::spawn(Arc::new(|| {}), config);
    const TAKE: TakeId = 1;
    let asked = Instant::now();
    handle.send(Command::LoadTake { take: TAKE, draft });
    let mut picture = None;
    let result = loop {
        if asked.elapsed() > TAKE_WAIT {
            break Err(format!(
                "The take of seed {seed} did not arrive within {} s.",
                TAKE_WAIT.as_secs()
            ));
        }
        match handle.poll() {
            Some(Event::TakePicture { take, story, frames }) if take == TAKE => {
                let at = (story.t_fight * f64::from(FPS)).round();
                let start = if at.is_finite() && at > 0.0 {
                    (at as u32).min(frames.frames().saturating_sub(1))
                } else {
                    0
                };
                picture = Some((frames, start, asked.elapsed()));
            }
            Some(Event::TakeSound { take, .. }) if take == TAKE => {
                let to_sound = asked.elapsed();
                break match picture.take() {
                    Some((frames, start, to_picture)) => Ok(Take {
                        seed,
                        frames,
                        start,
                        to_picture,
                        to_sound,
                    }),
                    None => Err("The take's sound arrived before its picture.".to_string()),
                };
            }
            Some(Event::TakeFailed { take, problem }) if take == TAKE => break Err(problem),
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(1)),
        }
    };
    handle.send(Command::Shutdown);
    drop(handle);
    _ = std::fs::remove_dir_all(&root);
    result
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use super::*;
    use crate::preview::scheduler::testing::TestSource;

    #[test]
    fn the_bench_counts_frames() {
        let r = run(TestSource::new(), 0, 2.0, 2);
        let counted = r.shown + r.late;
        assert!((55..=61).contains(&counted), "{r:?}");
        assert_eq!(r.on_time, r.shown as f32 / counted as f32, "{r:?}");
    }

    #[test]
    fn percentiles_are_numpys() {
        // np.percentile([1, 2, 4, 8], q) for q = 0, 50, 95, 100
        let v = [8.0, 1.0, 4.0, 2.0];
        assert_eq!(percentile(&v, 0.0), 1.0);
        assert_eq!(percentile(&v, 50.0), 3.0);
        assert!((percentile(&v, 95.0) - 7.4).abs() < 1e-5);
        assert_eq!(percentile(&v, 100.0), 8.0);
        assert_eq!(percentile(&[], 95.0), 0.0);
    }

    #[test]
    fn scrubs_go_to_frames_never_drawn() {
        let drawn: HashSet<u32> = (100..700).collect();
        let a = scrub_frames(1_830, &drawn, 50);
        assert_eq!(a.len(), 50);
        assert!(a.iter().all(|f| *f < 1_830 && !drawn.contains(f)));
        assert_eq!(a.iter().collect::<HashSet<_>>().len(), 50, "no frame twice");
        assert_eq!(a, scrub_frames(1_830, &drawn, 50), "the same frames every run");
        // Fewer free frames than asked: each free frame once.
        let most: HashSet<u32> = (0..1_825).collect();
        assert_eq!(scrub_frames(1_830, &most, 50).len(), 5);
    }

    /// One bench of the real take on `workers` threads, shared by the two budget tests (two at once would measure
    /// each other).
    fn real() -> &'static BenchReport {
        static RUN: OnceLock<BenchReport> = OnceLock::new();
        RUN.get_or_init(|| {
            let take = starter_take(None).expect("the starter fight's take");
            let r = run(take.frames, take.start, 20.0, workers());
            eprintln!("seed {}: {r:?}", take.seed);
            r
        })
    }

    #[test]
    #[ignore = "plays the real take for 20 s; run by name in release"]
    fn playback_meets_the_budget() {
        let r = real();
        let workers = workers();
        assert_eq!(r.threads, workers);
        let p = (1.25 * r.frame_ms / 33.3).ceil() as usize;
        if p <= workers {
            assert!(r.on_time >= 0.99, "P = {p}, workers = {workers}: {r:?}");
            assert_eq!(r.stride, 1, "{r:?}");
        } else {
            assert_eq!(r.stride, pace(r.frame_ms, workers).1, "{r:?}");
        }
        // The clock does not wait for frames: the playhead is where 20 s of playback puts it.
        let expected = r.start + 600;
        assert!(
            r.end_frame.abs_diff(expected) <= 1,
            "ended on {} not {expected}: {r:?}",
            r.end_frame
        );
    }

    #[test]
    #[ignore = "plays the real take for 20 s; run by name in release"]
    fn scrubbing_meets_the_budget() {
        let r = real();
        assert!(r.scrub_worst_ms <= r.frame_ms_p95 + 20.0, "{r:?}");
    }
}
