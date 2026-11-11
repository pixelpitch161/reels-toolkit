//! The draw-ahead scheduler (§7). Worker threads draw the take's frames on the CPU a little ahead of the playhead
//! into a small cache; the sound is the clock, so the player only ever asks for frames and shows what is kept.

//!
//! The scheduler only decides when a frame is drawn, never what it looks like: every kept frame is exactly what the
//! source returned for it.

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::Instant;

use crate::model::take::{Frame, FrameSource};
use crate::model::{NF, TakeId};
use crate::seam::Waker;

/// Frames drawn ahead of the playhead while playing (0.4 s).
pub const AHEAD: u32 = 12;
/// Frames kept at most (about 44 MB at 360 x 640).
pub const KEEP: usize = 64;

/// The draw time `frame_ms` reports before anything has been drawn.
const FIRST_FRAME_MS: f32 = 33.0;
/// How many of the latest draw times `frame_ms` averages.
const TIMES: usize = 30;

/// How far ahead the first wanted frame is and how many frames apart the wanted frames are, for a draw time of
/// `frame_ms` on `threads` threads. With `f = frame_ms * 30 / 1000` (frames of playback one draw takes), the lead is
/// `ceil(f - 0.001)` kept in 1 to 12, so a frame asked for now is ready when the playhead reaches it, and the stride
/// is `ceil(1.25 * f / threads - 0.001)`, at least 1: when the threads cannot draw every frame, every second (or
/// third) one is drawn and held. The small margin keeps a draw time of exactly three frames from counting as four.
pub fn pace(frame_ms: f32, threads: usize) -> (u32, u32) {
    let f = frame_ms * 30.0 / 1000.0;
    // `as u32` saturates: NaN and negative numbers become 0, which the bounds then lift to 1.
    let lead = ((f - 1e-3).ceil() as u32).clamp(1, AHEAD);
    let stride = ((1.25 * f / threads.max(1) as f32 - 1e-3).ceil() as u32).max(1);
    (lead, stride)
}

/// The frames to draw, most urgent first. Paused, it is the playhead alone (a scrub). Playing, it is
/// `playhead + lead`, then every `stride`-th frame after it up to `playhead + AHEAD`, wrapping past 1,829 to 0 when
/// `looping` and stopping at 1,829 when not.
pub fn wanted(playhead: u32, playing: bool, looping: bool, lead: u32, stride: u32) -> Vec<u32> {
    if !playing {
        return vec![playhead];
    }
    let mut out = Vec::new();
    let mut ahead = lead.max(1);
    while ahead <= AHEAD {
        let f = u64::from(playhead) + u64::from(ahead);
        if looping {
            out.push((f % u64::from(NF)) as u32);
        } else if f < u64::from(NF) {
            out.push(f as u32);
        } else {
            break;
        }
        ahead = ahead.saturating_add(stride.max(1));
    }
    out
}

/// The frames to draw when playback is about to start from `playhead` (ruling M2-R30: after a `Load` the clock
/// waits for the take's first picture): the playhead's own frame first, so it is the first thing shown, then the
/// frames playback will want, as [`wanted`] gives them while playing.
pub fn wanted_from(playhead: u32, looping: bool, lead: u32, stride: u32) -> Vec<u32> {
    let mut out = vec![playhead];
    out.extend(wanted(playhead, true, looping, lead, stride));
    out
}

/// Which of the `cached` frames to drop so that `keep` remain: the frames farthest behind the playhead first, then
/// the farthest ahead. The playhead's own frame counts as ahead, by nothing, so it goes last.
pub fn evict(cached: &[u32], playhead: u32, keep: usize) -> Vec<u32> {
    let Some(drop) = cached.len().checked_sub(keep).filter(|&n| n > 0) else {
        return Vec::new();
    };
    let mut behind: Vec<u32> = cached.iter().copied().filter(|&f| f < playhead).collect();
    let mut ahead: Vec<u32> = cached.iter().copied().filter(|&f| f >= playhead).collect();
    behind.sort_unstable();
    ahead.sort_unstable_by(|a, b| b.cmp(a));
    behind.into_iter().chain(ahead).take(drop).collect()
}

/// What the workers and the interface share, behind one lock.
struct State {
    take: TakeId,
    source: Option<Arc<dyn FrameSource>>,
    /// `source.frames()`, read once when the source is set (0 without one).
    frames: u32,
    /// Rises whenever the take changes; a frame finished for an older one is thrown away.
    epoch: u64,
    /// The wanted frames, most urgent first; a new `want` replaces them.
    queue: Vec<u32>,
    playhead: u32,
    /// The kept frames of the current take.
    cache: BTreeMap<u32, Frame>,
    /// Frames of the current take a worker is drawing.
    drawing: HashSet<u32>,
    /// Frames of the current source whose draw panicked; not tried again until a source is set.
    failed: HashSet<u32>,
    /// Workers inside a draw, for any take.
    busy: usize,
    /// The latest draw times in milliseconds.
    times: VecDeque<f32>,
    stop: bool,
}

impl State {
    /// The next frame a worker should draw: the first wanted frame that is neither kept, nor being drawn, nor failed.
    fn next(&self) -> Option<u32> {
        self.source.as_ref()?;
        self.queue.iter().copied().find(|f| {
            *f < self.frames && !self.cache.contains_key(f) && !self.drawing.contains(f) && !self.failed.contains(f)
        })
    }

    /// Drops frames beyond `KEEP`, never one that is wanted now (at the loop point, frames 0 to 11 are wanted while
    /// being "behind" a playhead near 1,829 by number).
    fn trim(&mut self) {
        if self.cache.len() <= KEEP {
            return;
        }
        let spare: Vec<u32> = self.cache.keys().copied().filter(|f| !self.queue.contains(f)).collect();
        let keep = KEEP.saturating_sub(self.cache.len() - spare.len());
        for f in evict(&spare, self.playhead, keep) {
            self.cache.remove(&f);
        }
    }
}

struct Shared {
    state: Mutex<State>,
    work: Condvar,
    /// When set (tests only), every draw is recorded as taking this many milliseconds instead of its measured time,
    /// so the feedback from `frame_ms` through `pace` into `want` does not depend on the machine.
    draw_ms: Option<f32>,
}

impl Shared {
    /// The state. A worker never panics while holding the lock (draws happen outside it and are caught), so a
    /// poisoned lock still holds consistent state and is used as it is.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Draws the current take's frames on worker threads, ahead of the playhead, and keeps the latest `KEEP` of them.
/// Nothing here waits for a draw: the interface asks and reads, the workers draw outside the lock.
pub struct Scheduler {
    shared: Arc<Shared>,
    workers: Vec<JoinHandle<()>>,
}

impl Scheduler {
    /// Starts `threads` workers (at least one). `waker` is called after every frame that is kept and after every
    /// draw that fails, outside the lock.
    pub fn new(threads: usize, waker: Waker) -> Scheduler {
        Scheduler::start(threads, waker, None)
    }

    /// A scheduler whose draws are all recorded as taking `ms` milliseconds, whatever they really take.
    #[cfg(test)]
    pub(crate) fn with_draw_ms(threads: usize, waker: Waker, ms: f32) -> Scheduler {
        Scheduler::start(threads, waker, Some(ms))
    }

    /// Records a draw time as if a draw had taken `ms` milliseconds.
    #[cfg(test)]
    pub(crate) fn record_draw_ms(&self, ms: f32) {
        record(&mut self.shared.lock().times, ms);
    }

    fn start(threads: usize, waker: Waker, draw_ms: Option<f32>) -> Scheduler {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                take: 0,
                source: None,
                frames: 0,
                epoch: 0,
                queue: Vec::new(),
                playhead: 0,
                cache: BTreeMap::new(),
                drawing: HashSet::new(),
                failed: HashSet::new(),
                busy: 0,
                times: VecDeque::with_capacity(TIMES),
                stop: false,
            }),
            work: Condvar::new(),
            draw_ms,
        });
        let workers = (0..threads.max(1))
            .map(|i| {
                let shared = shared.clone();
                let waker = waker.clone();
                std::thread::Builder::new()
                    .name(format!("preview-draw-{i}"))
                    .spawn(move || work(&shared, &waker))
                    .expect("the preview's drawing thread could not start")
            })
            .collect();
        Scheduler { shared, workers }
    }

    /// Draws from `frames` from now on. A different `take` forgets every frame of the old one, and a frame still
    /// being drawn for it is thrown away when it finishes; the same `take` keeps its frames. `None` draws nothing
    /// (and, for the same take, keeps what is drawn). Every call forgets which frames failed, even for the same
    /// take: a take set again (for example when its sound arrives) gets a fresh try.
    pub fn set_source(&self, take: TakeId, frames: Option<Arc<dyn FrameSource>>) {
        // Asked outside the lock; the workers read the cached count.
        let count = frames.as_ref().map_or(0, |s| s.frames());
        let mut st = self.shared.lock();
        if take != st.take {
            st.take = take;
            st.epoch += 1;
            st.cache.clear();
            st.drawing.clear();
        }
        st.failed.clear();
        st.frames = count;
        let old = std::mem::replace(&mut st.source, frames);
        drop(st);
        self.shared.work.notify_all();
        // A large old source is freed here, outside the lock.
        drop(old);
    }

    /// Asks for the frames around `playhead`; replaces what was asked before, so older scrubs are skipped.
    pub fn want(&self, playhead: u32, playing: bool, looping: bool) {
        let threads = self.workers.len();
        let mut st = self.shared.lock();
        let (lead, stride) = pace(mean(&st.times), threads);
        st.queue = wanted(playhead, playing, looping, lead, stride);
        st.playhead = playhead;
        drop(st);
        self.shared.work.notify_all();
    }

    /// Asks for `playhead`'s own frame first and then the frames playback from it will want ([`wanted_from`]):
    /// what the player asks while it waits for a take's first picture. Replaces what was asked before.
    pub fn want_start(&self, playhead: u32, looping: bool) {
        let threads = self.workers.len();
        let mut st = self.shared.lock();
        let (lead, stride) = pace(mean(&st.times), threads);
        st.queue = wanted_from(playhead, looping, lead, stride);
        st.playhead = playhead;
        drop(st);
        self.shared.work.notify_all();
    }

    /// Frame `f` of the current take, if it is kept.
    pub fn get(&self, f: u32) -> Option<Frame> {
        self.shared.lock().cache.get(&f).cloned()
    }

    /// The latest kept frame at or before `f`, with its number (what to hold while `f` is not drawn yet).
    pub fn latest_at_or_before(&self, f: u32) -> Option<(u32, Frame)> {
        let st = self.shared.lock();
        st.cache.range(..=f).next_back().map(|(&n, frame)| (n, frame.clone()))
    }

    /// The mean of the last 30 draw times in milliseconds; 33.0 before the first.
    pub fn frame_ms(&self) -> f32 {
        mean(&self.shared.lock().times)
    }

    /// Whether frame `f` could not be drawn (its source panicked). Scoped to the current take and source: setting a
    /// source forgets every failure. A failed frame is not tried again until then.
    pub fn failed(&self, f: u32) -> bool {
        self.shared.lock().failed.contains(&f)
    }

    /// How many frames of the current take and source could not be drawn.
    pub fn failed_count(&self) -> usize {
        self.shared.lock().failed.len()
    }

    /// Whether nothing is being drawn and nothing that is wanted is left to draw.
    pub fn idle(&self) -> bool {
        let st = self.shared.lock();
        st.busy == 0 && st.next().is_none()
    }
}

impl Drop for Scheduler {
    /// Stops the workers and waits for them: a worker inside a draw finishes that frame (and throws it away) first.
    fn drop(&mut self) {
        self.shared.lock().stop = true;
        self.shared.work.notify_all();
        for w in self.workers.drain(..) {
            // A worker catches every panic of the source and the waker, so this only fails if the thread itself
            // could not run; either way it has ended.
            _ = w.join();
        }
    }
}

impl std::fmt::Debug for Scheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let st = self.shared.lock();
        f.debug_struct("Scheduler")
            .field("threads", &self.workers.len())
            .field("take", &st.take)
            .field("kept", &st.cache.len())
            .finish_non_exhaustive()
    }
}

/// The mean of `times`, or 33.0 when there are none.
fn mean(times: &VecDeque<f32>) -> f32 {
    if times.is_empty() {
        FIRST_FRAME_MS
    } else {
        times.iter().sum::<f32>() / times.len() as f32
    }
}

/// Adds a draw time, keeping the latest 30.
fn record(times: &mut VecDeque<f32>, ms: f32) {
    if times.len() == TIMES {
        times.pop_front();
    }
    times.push_back(ms);
}

/// One worker: takes the next wanted frame, draws it outside the lock, keeps it if its take is still current.
fn work(shared: &Shared, waker: &Waker) {
    loop {
        let mut st = shared.lock();
        let (epoch, f, source) = loop {
            if st.stop {
                return;
            }
            if let (Some(f), Some(source)) = (st.next(), st.source.clone()) {
                break (st.epoch, f, source);
            }
            st = shared.work.wait(st).unwrap_or_else(PoisonError::into_inner);
        };
        st.drawing.insert(f);
        st.busy += 1;
        drop(st);

        let started = Instant::now();
        let drawn = catch_unwind(AssertUnwindSafe(|| source.draw(f)));
        let ms = shared
            .draw_ms
            .unwrap_or_else(|| started.elapsed().as_secs_f32() * 1000.0);
        drop(source);

        let mut st = shared.lock();
        st.busy -= 1;
        // Whether the interface should look again: a frame was kept, or one failed.
        let mut wake = false;
        if let Ok(frame) = &drawn {
            record(&mut st.times, ms);
            if epoch == st.epoch {
                st.cache.insert(f, frame.clone());
                st.trim();
                wake = true;
            }
        }
        if epoch == st.epoch {
            st.drawing.remove(&f);
            if drawn.is_err() {
                st.failed.insert(f);
                wake = true;
            }
        }
        drop(st);
        // A panic payload is freed outside the lock too.
        drop(drawn);
        if wake {
            _ = catch_unwind(AssertUnwindSafe(|| waker()));
        }
    }
}

/// A frame source and a wait for the tests here and in the player's and the bench's tests (Tasks 16 and 32).
#[cfg(test)]
pub(crate) mod testing {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::{Duration, Instant};

    use super::Scheduler;
    use crate::model::NF;
    use crate::model::take::{Frame, FrameSource};

    /// Draws 360 x 640 frames whose every byte is `(f % 251) as u8` and counts its calls. A gated source holds every
    /// draw until [`TestSource::open`]; a source made with [`TestSource::panicking_at`] panics on that one frame.
    pub(crate) struct TestSource {
        calls: AtomicUsize,
        gate: Option<(Mutex<bool>, Condvar)>,
        panics_on: Option<u32>,
    }

    impl TestSource {
        pub(crate) fn new() -> Arc<TestSource> {
            Arc::new(TestSource {
                calls: AtomicUsize::new(0),
                gate: None,
                panics_on: None,
            })
        }

        /// A source whose draws wait until [`TestSource::open`] is called.
        pub(crate) fn gated() -> Arc<TestSource> {
            Arc::new(TestSource {
                calls: AtomicUsize::new(0),
                gate: Some((Mutex::new(false), Condvar::new())),
                panics_on: None,
            })
        }

        /// A source that panics while drawing frame `f`.
        pub(crate) fn panicking_at(f: u32) -> Arc<TestSource> {
            Arc::new(TestSource {
                calls: AtomicUsize::new(0),
                gate: None,
                panics_on: Some(f),
            })
        }

        /// How many draws have started.
        pub(crate) fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }

        /// Lets every held draw, and every later one, finish.
        pub(crate) fn open(&self) {
            if let Some((open, cv)) = &self.gate {
                *open.lock().unwrap() = true;
                cv.notify_all();
            }
        }

        /// The frame this source draws for `f`.
        pub(crate) fn frame(f: u32) -> Frame {
            Frame {
                w: 360,
                h: 640,
                rgb: vec![(f % 251) as u8; 360 * 640 * 3].into(),
            }
        }
    }

    impl FrameSource for TestSource {
        fn frames(&self) -> u32 {
            NF
        }

        fn draw(&self, f: u32) -> Frame {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if let Some((open, cv)) = &self.gate {
                let mut open = open.lock().unwrap();
                while !*open {
                    open = cv.wait(open).unwrap();
                }
            }
            if self.panics_on == Some(f) {
                panic!("the test source fails on frame {f}");
            }
            TestSource::frame(f)
        }
    }

    /// Waits until `cond` holds, at most 5 s; panics with `what` when the time runs out.
    pub(crate) fn wait_for(what: &str, mut cond: impl FnMut() -> bool) {
        let until = Instant::now() + Duration::from_secs(5);
        while !cond() {
            assert!(Instant::now() < until, "waited 5 s for {what}");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// Waits until the scheduler has nothing left to draw, at most 5 s.
    ///
    /// The waker is called after the lock is released, so the scheduler can be idle before the waker for its last
    /// frame has run: never assert on the waker straight after this; wait for it with [`wait_for`].
    pub(crate) fn wait_idle(s: &Scheduler) {
        wait_for("the scheduler to be idle", || s.idle());
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    use super::testing::{TestSource, wait_for, wait_idle};
    use super::*;
    use crate::model::NF;

    fn scheduler(threads: usize) -> Scheduler {
        Scheduler::new(threads, Arc::new(|| {}))
    }

    fn kept(s: &Scheduler) -> Vec<u32> {
        (0..NF).filter(|&f| s.get(f).is_some()).collect()
    }

    /// Drops `s` on another thread and says whether that returned within 2 s.
    fn drops_within_2_s(s: Scheduler, meanwhile: impl FnOnce()) -> bool {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            drop(s);
            _ = tx.send(());
        });
        meanwhile();
        rx.recv_timeout(Duration::from_secs(2)).is_ok()
    }

    #[test]
    fn pace_gives_the_lead_and_the_stride() {
        assert_eq!(pace(20.0, 7), (1, 1));
        assert_eq!(pace(100.0, 7), (3, 1));
        assert_eq!(pace(300.0, 7), (9, 2));
        assert_eq!(pace(26.0, 1), (1, 1));
        assert_eq!(pace(30.0, 1), (1, 2));
    }

    #[test]
    fn pace_keeps_the_lead_in_1_to_12_and_the_stride_at_least_1() {
        assert_eq!(pace(0.0, 7), (1, 1));
        assert_eq!(pace(10_000.0, 7).0, 12);
        assert_eq!(pace(f32::NAN, 7), (1, 1));
        assert_eq!(pace(-5.0, 0), (1, 1));
        assert!(pace(100.0, 0).1 >= 1);
    }

    #[test]
    fn wanted_is_the_playhead_when_paused() {
        assert_eq!(wanted(100, false, true, 1, 1), [100]);
        assert_eq!(wanted(100, false, false, 3, 2), [100]);
    }

    #[test]
    fn wanted_runs_ahead_while_playing() {
        assert_eq!(wanted(100, true, true, 1, 1), (101..=112).collect::<Vec<_>>());
        assert_eq!(wanted(100, true, true, 3, 2), [103, 105, 107, 109, 111]);
        assert_eq!(wanted(100, true, true, 12, u32::MAX), [112]);
    }

    #[test]
    fn wanted_wraps_when_looping_and_stops_at_the_end_when_not() {
        assert_eq!(
            wanted(1_825, true, true, 1, 1),
            [1826, 1827, 1828, 1829, 0, 1, 2, 3, 4, 5, 6, 7]
        );
        assert_eq!(wanted(1_825, true, false, 1, 1), [1826, 1827, 1828, 1829]);
        assert_eq!(wanted(1_829, true, false, 1, 1), Vec::<u32>::new());
    }

    #[test]
    fn wanted_from_starts_with_the_playhead() {
        let mut ahead = vec![100];
        ahead.extend(101..=112);
        assert_eq!(wanted_from(100, true, 1, 1), ahead);
        assert_eq!(wanted_from(100, true, 3, 2), [100, 103, 105, 107, 109, 111]);
        assert_eq!(wanted_from(1_829, false, 1, 1), [1_829]);
        assert_eq!(wanted_from(1_829, true, 11, 1), [1_829, 10, 11]);
    }

    /// A test source that remembers the order frames were asked in.
    struct Order {
        inner: Arc<TestSource>,
        asked: std::sync::Mutex<Vec<u32>>,
    }

    impl FrameSource for Order {
        fn frames(&self) -> u32 {
            self.inner.frames()
        }

        fn draw(&self, f: u32) -> Frame {
            self.asked.lock().unwrap().push(f);
            self.inner.draw(f)
        }
    }

    #[test]
    fn want_start_draws_the_playhead_first_then_ahead() {
        let s = Scheduler::with_draw_ms(1, Arc::new(|| {}), 1.0);
        let gate = TestSource::gated();
        let src = Arc::new(Order {
            inner: gate.clone(),
            asked: std::sync::Mutex::new(Vec::new()),
        });
        s.set_source(1, Some(src.clone()));
        // A draw time of 1 ms, so the lead and the stride are 1 from the first `want_start`.
        s.record_draw_ms(1.0);
        s.want_start(100, true);
        wait_for("the first draw", || gate.calls() == 1);
        gate.open();
        wait_idle(&s);
        let asked = src.asked.lock().unwrap().clone();
        assert_eq!(asked.first(), Some(&100), "{asked:?}");
        assert_eq!(kept(&s), (100..=112).collect::<Vec<_>>());
    }

    #[test]
    fn evict_drops_the_farthest_behind_then_the_farthest_ahead() {
        assert_eq!(evict(&[1, 2, 3, 50, 51], 50, 3), [1, 2]);
        assert_eq!(evict(&[1, 2, 3, 50, 51], 50, 5), Vec::<u32>::new());
        assert_eq!(evict(&[60, 50, 70, 40], 50, 2), [40, 70]);
        assert_eq!(evict(&[52, 50, 51], 50, 1), [52, 51]);
    }

    #[test]
    fn it_draws_ahead_while_playing() {
        let s = scheduler(4);
        s.set_source(1, Some(TestSource::new()));
        s.want(100, true, true);
        wait_idle(&s);
        for f in 101..=112 {
            assert!(s.get(f).is_some(), "frame {f} is drawn");
        }
        assert!(s.get(200).is_none());
    }

    #[test]
    fn a_scrub_draws_just_that_frame() {
        let src = TestSource::new();
        let s = scheduler(4);
        s.set_source(1, Some(src.clone()));
        s.want(500, false, true);
        wait_idle(&s);
        assert_eq!(kept(&s), [500]);
        assert_eq!(src.calls(), 1);
        assert_eq!(s.latest_at_or_before(600).map(|(f, _)| f), Some(500));
        assert_eq!(s.latest_at_or_before(500).map(|(f, _)| f), Some(500));
        assert!(s.latest_at_or_before(499).is_none());
    }

    #[test]
    fn scrubs_skip_to_the_newest() {
        // The draws are held until every request is in (a gate rather than the brief's 5 ms sleep, so the test
        // does not depend on timing): at most one draw per thread starts before the last request replaces the
        // queue, and after that only the newest frame is drawn.
        let src = TestSource::gated();
        let s = scheduler(2);
        s.set_source(1, Some(src.clone()));
        for f in 0..50 {
            s.want(f, false, true);
        }
        src.open();
        wait_idle(&s);
        assert!(s.get(49).is_some());
        assert!(src.calls() < 20, "the source drew {} frames", src.calls());
        assert!(src.calls() <= 3, "the source drew {} frames", src.calls());
    }

    #[test]
    fn frames_are_the_sources_own_bytes() {
        let s = scheduler(2);
        s.set_source(1, Some(TestSource::new()));
        for f in [7, 300] {
            s.want(f, false, true);
            wait_idle(&s);
            assert_eq!(s.get(f), Some(TestSource::frame(f)));
        }
        let frame = s.get(300).unwrap();
        assert_eq!((frame.w, frame.h, frame.rgb.len()), (360, 640, 360 * 640 * 3));
        assert!(frame.rgb.iter().all(|&b| b == 49));
    }

    #[test]
    fn another_take_forgets_the_old_frames() {
        let s = scheduler(2);
        s.set_source(1, Some(TestSource::new()));
        s.want(10, false, true);
        wait_idle(&s);
        assert!(s.get(10).is_some());
        // The same take keeps its frames, with another source or with none.
        s.set_source(1, Some(TestSource::new()));
        assert!(s.get(10).is_some());
        s.set_source(1, None);
        assert!(s.get(10).is_some());
        assert_eq!(s.latest_at_or_before(NF).map(|(f, _)| f), Some(10));
        s.set_source(2, Some(TestSource::new()));
        assert!(s.get(10).is_none());
        assert!(s.latest_at_or_before(NF).is_none());
    }

    #[test]
    fn a_frame_finished_for_an_old_take_is_thrown_away() {
        let old = TestSource::gated();
        let woken = Arc::new(AtomicUsize::new(0));
        let w = woken.clone();
        let s = Scheduler::new(2, Arc::new(move || _ = w.fetch_add(1, Ordering::SeqCst)));
        s.set_source(1, Some(old.clone()));
        s.want(20, false, true);
        wait_for("the old take's draw to start", || old.calls() == 1);
        s.set_source(2, None);
        old.open();
        wait_idle(&s);
        assert!(s.get(20).is_none());
        assert_eq!(woken.load(Ordering::SeqCst), 0);
        // The new take draws the frame afresh.
        let new = TestSource::new();
        s.set_source(2, Some(new.clone()));
        s.want(20, false, true);
        wait_idle(&s);
        assert!(s.get(20).is_some());
        assert_eq!(new.calls(), 1);
        // The waker runs after the lock is released, so it may still be on its way when the scheduler is idle.
        wait_for("the waker", || woken.load(Ordering::SeqCst) == 1);
    }

    #[test]
    fn it_keeps_at_most_64() {
        let s = scheduler(4);
        s.set_source(1, Some(TestSource::new()));
        for f in 0..100 {
            s.want(f, false, true);
            wait_idle(&s);
        }
        assert_eq!(kept(&s), (36..=99).collect::<Vec<_>>());
    }

    #[test]
    fn the_frames_it_wants_are_never_dropped_at_the_loop_point() {
        // 64 frames behind the playhead are kept; playing on from 1,825 wants 0 to 7, which are "behind" 1,825 by
        // number but are what plays next. They must stay, or they would be drawn and dropped for ever. Every draw
        // counts as 1 ms, so the lead and the stride are 1 however fast this machine draws.
        let src = TestSource::new();
        let s = Scheduler::with_draw_ms(4, Arc::new(|| {}), 1.0);
        s.set_source(1, Some(src.clone()));
        for f in 1_761..=1_824 {
            s.want(f, false, true);
            wait_idle(&s);
        }
        s.want(1_825, true, true);
        wait_idle(&s);
        let kept = kept(&s);
        assert_eq!(kept.len(), KEEP);
        for f in [1826, 1827, 1828, 1829, 0, 1, 2, 3, 4, 5, 6, 7] {
            assert!(kept.contains(&f), "frame {f} is kept");
        }
        assert_eq!(src.calls(), 64 + 12);
    }

    #[test]
    fn a_panicking_source_leaves_the_scheduler_working() {
        let src = TestSource::panicking_at(5);
        let woken = Arc::new(AtomicUsize::new(0));
        let w = woken.clone();
        let s = Scheduler::new(2, Arc::new(move || _ = w.fetch_add(1, Ordering::SeqCst)));
        s.set_source(1, Some(src.clone()));
        assert!(!s.failed(5));
        s.want(5, false, true);
        wait_idle(&s);
        assert!(s.get(5).is_none());
        assert!(s.failed(5));
        assert!(!s.failed(6));
        assert_eq!(s.failed_count(), 1);
        // A failed draw wakes the interface too, so it can ask.
        wait_for("the waker after the failed draw", || woken.load(Ordering::SeqCst) == 1);
        // The frame that failed is not drawn again for this source; the others still are.
        s.want(5, false, true);
        s.want(6, false, true);
        wait_idle(&s);
        assert!(s.get(6).is_some());
        assert_eq!(src.calls(), 2);
        assert!(!s.failed(6));
        // Setting the source again, even for the same take, forgets the failure and tries once more.
        s.set_source(1, Some(src.clone()));
        assert!(!s.failed(5));
        assert_eq!(s.failed_count(), 0);
        assert!(s.get(6).is_some());
        s.want(5, false, true);
        wait_idle(&s);
        assert_eq!(src.calls(), 3);
        assert!(s.failed(5));
        // Another take starts with no failures.
        s.set_source(2, Some(TestSource::new()));
        assert_eq!(s.failed_count(), 0);
        assert!(drops_within_2_s(s, || {}));
    }

    #[test]
    fn a_panic_in_the_waker_is_survived() {
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let s = Scheduler::new(
            1,
            Arc::new(move || {
                c.fetch_add(1, Ordering::SeqCst);
                panic!("the test waker fails");
            }),
        );
        s.set_source(1, Some(TestSource::new()));
        for f in [1, 2] {
            s.want(f, false, true);
            wait_idle(&s);
            assert!(s.get(f).is_some(), "frame {f} is drawn");
        }
        wait_for("both wakers", || calls.load(Ordering::SeqCst) == 2);
        assert!(drops_within_2_s(s, || {}));
    }

    #[test]
    fn frame_ms_is_33_before_the_first_draw() {
        let s = scheduler(1);
        assert_eq!(s.frame_ms(), 33.0);
        s.set_source(1, Some(TestSource::new()));
        s.want(3, false, true);
        wait_idle(&s);
        let ms = s.frame_ms();
        assert!(ms.is_finite() && ms >= 0.0, "frame_ms is {ms}");
    }

    #[test]
    fn frame_ms_is_the_mean_of_the_last_30_draws() {
        let s = scheduler(1);
        for ms in 1..=40 {
            s.record_draw_ms(ms as f32);
        }
        // The last 30 are 11 to 40.
        assert_eq!(s.frame_ms(), 25.5);
        // A real draw is recorded as it is timed (here, fixed at 7 ms).
        let s = Scheduler::with_draw_ms(1, Arc::new(|| {}), 7.0);
        s.set_source(1, Some(TestSource::new()));
        for f in [3, 4] {
            s.want(f, false, true);
            wait_idle(&s);
        }
        assert_eq!(s.frame_ms(), 7.0);
    }

    #[test]
    fn a_slow_draw_moves_the_lead_and_draws_every_second_frame() {
        // Draws of 300 ms on 7 threads: pace gives a lead of 9 and a stride of 2. The draw time comes from a real
        // draw (a scrub) and reaches the next `want` through `frame_ms`.
        let s = Scheduler::with_draw_ms(7, Arc::new(|| {}), 300.0);
        s.set_source(1, Some(TestSource::new()));
        s.want(0, false, true);
        wait_idle(&s);
        assert_eq!(s.frame_ms(), 300.0);
        s.want(100, true, true);
        wait_idle(&s);
        assert_eq!(kept(&s), [0, 109, 111]);

        // One thread that takes 30 ms a frame cannot draw every frame: every second one is drawn (and held).
        let s = scheduler(1);
        s.record_draw_ms(30.0);
        s.set_source(1, Some(TestSource::new()));
        s.want(100, true, true);
        wait_idle(&s);
        assert_eq!(kept(&s), [101, 103, 105, 107, 109, 111]);
    }

    #[test]
    fn idle_without_a_source() {
        let s = scheduler(2);
        assert!(s.idle());
        s.want(10, true, true);
        assert!(s.idle());
    }

    #[test]
    fn dropping_it_stops_the_threads() {
        let src = TestSource::new();
        let s = scheduler(4);
        s.set_source(1, Some(src.clone()));
        s.want(100, true, true);
        wait_idle(&s);
        assert!(drops_within_2_s(s, || {}));
        // Every thread has ended, so no one holds the source any more.
        assert_eq!(Arc::strong_count(&src), 1);
    }

    #[test]
    fn dropping_it_while_frames_are_drawn_ends_after_those_frames() {
        let src = TestSource::gated();
        let s = scheduler(2);
        s.set_source(1, Some(src.clone()));
        s.want(100, true, true);
        wait_for("two draws to start", || src.calls() == 2);
        assert!(drops_within_2_s(s, || src.open()));
        assert_eq!(Arc::strong_count(&src), 1);
    }
}
