//! The preview player (§7): one take's picture and sound, and the transport over them. The sound is the clock: while
//! the take's soundtrack sounds, the playhead is wherever the device has got to; before the sound is built, without a
//! device, or after the device is lost, it moves with the wall clock. Pictures come from the draw-ahead scheduler;
//! nothing here waits for one.

use std::time::Duration;

use crate::model::take::{AudioTake, Frame, PlayerCmd, SoundState, TransportAction, TransportView};
use crate::model::{NA, SR, TakeId};
use crate::preview::audio::AudioOut;
use crate::preview::clock::{advance, frame_of, pos_of, step};
use crate::preview::scheduler::Scheduler;
use crate::seam::Waker;

/// The wall clock: any steadily rising time (the app passes the time since it started).
pub type Now = Box<dyn Fn() -> Duration + Send>;

/// Frames given on time (`shown`: the playhead's own frame, kept when asked) and frames that were not kept yet when
/// the playhead reached them (`late`, once per frame).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerStats {
    pub shown: u64,
    pub late: u64,
}

/// Where the playhead is right now, read without changing anything.
struct Reading {
    pos: u64,
    playing: bool,
    /// The sound was running and the device no longer answers.
    lost: bool,
}

/// Plays one take at a time. It owns one scheduler for its whole life and switches takes on it (widget rule 12).
pub struct Player {
    scheduler: Scheduler,
    audio: Box<dyn AudioOut>,
    now: Now,
    /// The loaded take; `None` with nothing loaded or a still up (the transport is disabled then).
    take: Option<TakeId>,
    /// The take's soundtrack, once it has arrived.
    sound_take: Option<AudioTake>,
    sound: SoundState,
    /// A still waiting to be given by `tick`.
    still: Option<Frame>,
    /// The playhead in soundtrack samples, as of the last command or tick.
    pos: u64,
    playing: bool,
    /// Whether the audio was started and is believed to be sounding: then it is the clock.
    sounding: bool,
    /// The wall-clock time and position playback was last measured from, while playing without sound.
    wall_from: Duration,
    wall_pos: u64,
    looping: bool,
    safe_zone: bool,
    /// Inside a scrub, and whether it was playing when the scrub began.
    scrub: Option<bool>,
    /// The number of the frame on screen (the playhead's own, or one held before it).
    shown: Option<u32>,
    /// The last frame counted late, so it is counted once.
    late_at: Option<u32>,
    stats: PlayerStats,
}

impl Player {
    /// A player with `threads` drawing threads, sound through `audio`, wall time from `now`; `waker` is called when a
    /// frame is drawn (or fails), so the interface repaints.
    pub fn new(threads: usize, audio: Box<dyn AudioOut>, now: Now, waker: Waker) -> Player {
        Player {
            scheduler: Scheduler::new(threads, waker),
            audio,
            now,
            take: None,
            sound_take: None,
            sound: SoundState::None,
            still: None,
            pos: 0,
            playing: false,
            sounding: false,
            wall_from: Duration::ZERO,
            wall_pos: 0,
            looping: true,
            safe_zone: false,
            scrub: None,
            shown: None,
            late_at: None,
            stats: PlayerStats::default(),
        }
    }

    /// Carries out one command. Never waits: the scheduler and the audio are only told.
    pub fn run(&mut self, cmd: PlayerCmd) {
        match cmd {
            PlayerCmd::Clear => self.unload(None),
            PlayerCmd::Still(frame) => self.unload(Some(frame)),
            PlayerCmd::Load {
                take,
                frames,
                start,
                autoplay,
            } => {
                self.halt();
                self.scheduler.set_source(take, Some(frames));
                self.take = Some(take);
                self.sound_take = None;
                self.sound = SoundState::Building;
                self.still = None;
                self.pos = pos_of(step(start, 0));
                self.scrub = None;
                self.shown = None;
                self.late_at = None;
                if autoplay {
                    self.play();
                }
                self.ask();
            }
            PlayerCmd::Sound { take, audio } => {
                if self.take != Some(take) {
                    return;
                }
                self.settle();
                self.sound_take = Some(audio);
                self.sound = SoundState::Ready;
                if self.playing {
                    // Joins at the playhead (or replaces a soundtrack already sounding).
                    self.halt();
                    self.play();
                }
            }
            PlayerCmd::Transport(action) => {
                // Without a take only the two settings change; playing and moving need something to play.
                let setting = matches!(action, TransportAction::SetLoop(_) | TransportAction::SetSafeZone(_));
                if self.take.is_none() && !setting {
                    return;
                }
                self.settle();
                self.transport(action);
                self.ask();
            }
        }
    }

    fn transport(&mut self, action: TransportAction) {
        match action {
            TransportAction::PlayPause => {
                if self.playing {
                    self.halt();
                } else {
                    if !self.looping && self.pos >= NA - 1 {
                        self.pos = 0;
                    }
                    self.play();
                }
            }
            TransportAction::Step(n) => {
                self.halt();
                self.pos = pos_of(step(frame_of(self.pos), n));
            }
            TransportAction::ScrubTo(t) => {
                if self.scrub.is_none() {
                    self.scrub = Some(self.playing);
                }
                self.halt();
                self.pos = scrub_pos(t);
            }
            TransportAction::ScrubEnd => {
                if self.scrub.take() == Some(true) {
                    self.play();
                }
            }
            TransportAction::SetLoop(on) => {
                self.looping = on;
                if self.sounding {
                    self.audio.set_looping(on);
                } else {
                    self.restart_wall();
                }
            }
            TransportAction::SetSafeZone(on) => self.safe_zone = on,
        }
    }

    /// Once per repaint: moves the clock, asks the scheduler for the playhead's frame and gives a picture when the one
    /// to show has changed. A frame not kept yet (late, or failed) holds the latest kept one before it.
    pub fn tick(&mut self) -> Option<Frame> {
        if let Some(frame) = self.still.take() {
            return Some(frame);
        }
        self.take?;
        self.settle();
        self.ask();
        let f = frame_of(self.pos);
        if self.shown == Some(f) {
            return None;
        }
        if let Some(frame) = self.scheduler.get(f) {
            self.shown = Some(f);
            self.late_at = None;
            self.stats.shown += 1;
            return Some(frame);
        }
        if self.late_at != Some(f) {
            self.late_at = Some(f);
            self.stats.late += 1;
        }
        let (n, frame) = self.scheduler.latest_at_or_before(f)?;
        // What is on screen is still the best picture if it is at or before the playhead and no older than `n`.
        if self.shown.is_some_and(|s| s <= f && s >= n) {
            return None;
        }
        self.shown = Some(n);
        Some(frame)
    }

    /// What the transport shows, as of now.
    pub fn view(&self) -> TransportView {
        let r = self.read();
        TransportView {
            enabled: self.take.is_some(),
            frame: frame_of(r.pos),
            playing: r.playing,
            looping: self.looping,
            safe_zone: self.safe_zone,
            sound: if r.lost { SoundState::NoDevice } else { self.sound },
        }
    }

    pub fn stats(&self) -> PlayerStats {
        self.stats
    }

    /// Playing, or waiting for the playhead's frame (a frame that failed is not waited for).
    pub fn wants_repaint(&self) -> bool {
        if self.still.is_some() || self.playing {
            return true;
        }
        let f = frame_of(self.pos);
        self.take.is_some() && self.shown != Some(f) && !self.scheduler.failed(f)
    }

    /// The playhead's frame when it could not be drawn (widget rule 11), else `None`.
    pub fn failed_frame(&self) -> Option<u32> {
        self.take?;
        let f = frame_of(self.read().pos);
        self.scheduler.failed(f).then_some(f)
    }

    /// Clears the take and shows `still`, if any.
    fn unload(&mut self, still: Option<Frame>) {
        self.halt();
        if let Some(take) = self.take.take() {
            // The same take keeps what is drawn, so nothing is thrown away here; drawing stops.
            self.scheduler.set_source(take, None);
        }
        self.sound_take = None;
        self.sound = SoundState::None;
        self.still = still;
        self.pos = 0;
        self.scrub = None;
        self.shown = None;
        self.late_at = None;
    }

    /// Starts playback from `pos`: on the sound when it has arrived and starts, else on the wall clock.
    fn play(&mut self) {
        self.playing = true;
        self.sounding = false;
        if let Some(audio) = &self.sound_take {
            match self.audio.start(audio.clone(), self.pos, self.looping) {
                Ok(()) => {
                    self.sounding = true;
                    self.sound = SoundState::Ready;
                }
                Err(_) => self.sound = SoundState::NoDevice,
            }
        }
        if !self.sounding {
            self.restart_wall();
        }
    }

    /// Stops playback (and the sound) where it is.
    fn halt(&mut self) {
        if self.sounding {
            self.audio.stop();
            self.sounding = false;
        }
        self.playing = false;
    }

    /// Measures wall-clock playback from now and the current position.
    fn restart_wall(&mut self) {
        self.wall_from = (self.now)();
        self.wall_pos = self.pos;
    }

    /// Where the playhead is now, from the sound while it sounds, else from the wall clock.
    fn read(&self) -> Reading {
        if !self.playing {
            return Reading {
                pos: self.pos,
                playing: false,
                lost: false,
            };
        }
        let (pos, playing, lost) = if self.sounding {
            match self.audio.position() {
                Some(p) => {
                    let (pos, on) = advance(p, 0, self.looping);
                    (pos, on, false)
                }
                None => (self.pos, true, true),
            }
        } else {
            let elapsed = (self.now)().saturating_sub(self.wall_from);
            let by = elapsed.as_nanos() * u128::from(SR) / 1_000_000_000;
            let (pos, on) = advance(self.wall_pos, u64::try_from(by).unwrap_or(u64::MAX), self.looping);
            (pos, on, false)
        };
        Reading { pos, playing, lost }
    }

    /// Takes in what `read` sees: the new position, the end of playback, a lost device (playback goes on from the
    /// same position on the wall clock).
    fn settle(&mut self) {
        let r = self.read();
        self.pos = r.pos;
        if r.lost {
            self.sounding = false;
            self.sound = SoundState::NoDevice;
            self.restart_wall();
        }
        if self.playing && !r.playing {
            self.halt();
        }
    }

    /// Tells the scheduler which frames are wanted now.
    fn ask(&self) {
        if self.take.is_some() {
            self.scheduler.want(frame_of(self.pos), self.playing, self.looping);
        }
    }
}

/// The position for a scrub to `t` seconds: `t * 48,000` rounded down, kept in `0..NA` (anything not a positive
/// number is 0).
fn scrub_pos(t: f32) -> u64 {
    if t > 0.0 {
        // `as` saturates, so a huge `t` lands on the last sample.
        ((f64::from(t) * f64::from(SR)) as u64).min(NA - 1)
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::Duration;

    use super::*;
    use crate::model::NA;
    use crate::model::take::{AudioTake, FrameSource, SoundState, TransportAction};
    use crate::preview::audio::NoAudio;
    use crate::preview::clock::pos_of;
    use crate::preview::scheduler::testing::{TestSource, wait_for};

    /// What a [`FakeOut`] was asked, and the position the test gives it.
    #[derive(Default)]
    struct Log {
        /// `(at, looping)` of every start.
        starts: Vec<(u64, bool)>,
        stops: usize,
        looping: Vec<bool>,
        position: Option<u64>,
    }

    /// A sound output that plays nothing: it records its calls, and its position is whatever the test sets (a start
    /// sets it to `at`, a stop to `None`, as a real device would).
    struct FakeOut(Arc<Mutex<Log>>);

    impl AudioOut for FakeOut {
        fn start(&mut self, _audio: AudioTake, at: u64, looping: bool) -> Result<(), String> {
            let mut log = self.0.lock().unwrap();
            log.starts.push((at, looping));
            log.position = Some(at);
            Ok(())
        }
        fn stop(&mut self) {
            let mut log = self.0.lock().unwrap();
            log.stops += 1;
            log.position = None;
        }
        fn position(&self) -> Option<u64> {
            self.0.lock().unwrap().position
        }
        fn set_looping(&mut self, on: bool) {
            self.0.lock().unwrap().looping.push(on);
        }
    }

    /// Opens a gated source when dropped. Declared after the player, it drops first, so a failing assertion cannot
    /// leave the scheduler's drop waiting for a held draw.
    struct Opens(Arc<TestSource>);

    impl Drop for Opens {
        fn drop(&mut self) {
            self.0.open();
        }
    }

    /// A clock the test moves by hand.
    struct Hand(Arc<AtomicU64>);

    impl Hand {
        fn forward(&self, by: Duration) {
            self.0.fetch_add(by.as_nanos() as u64, Ordering::SeqCst);
        }
    }

    /// `n` frames of playback on the wall clock: just over `n / 30` s, so whole samples land on the frame.
    fn frames(n: u64) -> Duration {
        Duration::from_nanos(n * 33_333_334)
    }

    fn player_with(threads: usize, audio: Box<dyn AudioOut>) -> (Player, Hand) {
        let t = Arc::new(AtomicU64::new(1_000_000_000));
        let t2 = t.clone();
        let now: Now = Box::new(move || Duration::from_nanos(t2.load(Ordering::SeqCst)));
        (Player::new(threads, audio, now, Arc::new(|| {})), Hand(t))
    }

    fn faked() -> (Player, Hand, Arc<Mutex<Log>>) {
        let log = Arc::new(Mutex::new(Log::default()));
        let (p, hand) = player_with(2, Box::new(FakeOut(log.clone())));
        (p, hand, log)
    }

    fn load(p: &mut Player, take: u64, frames: Arc<dyn FrameSource>, start: u32, autoplay: bool) {
        p.run(PlayerCmd::Load {
            take,
            frames,
            start,
            autoplay,
        });
    }

    fn sound(p: &mut Player, take: u64) {
        p.run(PlayerCmd::Sound {
            take,
            audio: AudioTake {
                samples: vec![[0.0; 2]; 16].into(),
            },
        });
    }

    fn act(p: &mut Player, a: TransportAction) {
        p.run(PlayerCmd::Transport(a));
    }

    #[test]
    fn a_still_is_shown_once_and_disables_the_transport() {
        let (mut p, _hand, log) = faked();
        let card = TestSource::frame(3);
        p.run(PlayerCmd::Still(card.clone()));
        assert!(p.wants_repaint());
        assert_eq!(p.tick(), Some(card));
        assert_eq!(p.tick(), None);
        assert!(!p.wants_repaint());
        let v = p.view();
        assert!(!v.enabled);
        assert!(!v.playing);
        assert_eq!(v.sound, SoundState::None);
        // The transport does nothing while a still is up.
        act(&mut p, TransportAction::PlayPause);
        assert!(!p.view().playing);
        assert!(log.lock().unwrap().starts.is_empty());
    }

    #[test]
    fn it_plays_silently_before_the_sound_arrives() {
        let (mut p, hand, log) = faked();
        load(&mut p, 1, TestSource::new(), 108, true);
        let v = p.view();
        assert!(v.enabled && v.playing);
        assert_eq!(v.frame, 108);
        assert_eq!(v.sound, SoundState::Building);
        hand.forward(Duration::from_secs(1));
        p.tick();
        assert_eq!(p.view().frame, 138);
        hand.forward(frames(1));
        assert_eq!(p.view().frame, 139);
        assert!(p.wants_repaint());
        assert!(log.lock().unwrap().starts.is_empty());
    }

    #[test]
    fn the_sound_joins_at_the_playhead() {
        let (mut p, hand, log) = faked();
        load(&mut p, 1, TestSource::new(), 108, true);
        hand.forward(Duration::from_secs(1));
        p.tick();
        // Sound for another take is ignored.
        sound(&mut p, 2);
        assert!(log.lock().unwrap().starts.is_empty());
        assert_eq!(p.view().sound, SoundState::Building);
        sound(&mut p, 1);
        {
            let log = log.lock().unwrap();
            assert_eq!(log.starts.len(), 1);
            let (at, looping) = log.starts[0];
            assert!((pos_of(138)..pos_of(139)).contains(&at), "the sound started at {at}");
            assert!(looping);
        }
        assert_eq!(p.view().sound, SoundState::Ready);
        // The sound is the clock: the wall clock no longer moves the playhead.
        log.lock().unwrap().position = Some(pos_of(300));
        hand.forward(Duration::from_secs(5));
        p.tick();
        assert_eq!(p.view().frame, 300);
        assert!(p.view().playing);
    }

    #[test]
    fn no_sound_device_still_plays() {
        let (mut p, hand) = player_with(2, Box::new(NoAudio("No sound device.".into())));
        load(&mut p, 1, TestSource::new(), 0, true);
        sound(&mut p, 1);
        assert_eq!(p.view().sound, SoundState::NoDevice);
        hand.forward(Duration::from_secs(1));
        p.tick();
        let v = p.view();
        assert_eq!(v.frame, 30);
        assert!(v.playing);
        assert_eq!(v.sound, SoundState::NoDevice);
    }

    #[test]
    fn step_pauses() {
        let (mut p, hand, log) = faked();
        load(&mut p, 1, TestSource::new(), 100, true);
        sound(&mut p, 1);
        log.lock().unwrap().position = Some(pos_of(115) + 10);
        act(&mut p, TransportAction::Step(2));
        let v = p.view();
        assert!(!v.playing);
        assert_eq!(v.frame, 117);
        assert_eq!(log.lock().unwrap().stops, 1);
        hand.forward(Duration::from_secs(1));
        p.tick();
        assert_eq!(p.view().frame, 117);
        act(&mut p, TransportAction::Step(-1));
        assert_eq!(p.view().frame, 116);
        act(&mut p, TransportAction::Step(-200));
        assert_eq!(p.view().frame, 0);
        act(&mut p, TransportAction::Step(5_000));
        assert_eq!(p.view().frame, 1_829);
        assert_eq!(log.lock().unwrap().starts.len(), 1);
    }

    #[test]
    fn scrub_stops_the_sound_and_resumes_after() {
        let (mut p, hand, log) = faked();
        load(&mut p, 1, TestSource::new(), 0, true);
        sound(&mut p, 1);
        act(&mut p, TransportAction::ScrubTo(10.0));
        assert_eq!(log.lock().unwrap().stops, 1);
        let v = p.view();
        assert!(!v.playing);
        assert_eq!(v.frame, 300);
        act(&mut p, TransportAction::ScrubTo(1_000.0));
        assert_eq!(p.view().frame, 1_829);
        act(&mut p, TransportAction::ScrubTo(-1.0));
        assert_eq!(p.view().frame, 0);
        act(&mut p, TransportAction::ScrubTo(f32::NAN));
        assert_eq!(p.view().frame, 0);
        act(&mut p, TransportAction::ScrubTo(20.0));
        hand.forward(Duration::from_secs(1));
        p.tick();
        assert_eq!(p.view().frame, 600);
        act(&mut p, TransportAction::ScrubEnd);
        assert!(p.view().playing);
        assert_eq!(log.lock().unwrap().starts.last(), Some(&(960_000, true)));

        // A scrub from pause stays paused.
        act(&mut p, TransportAction::PlayPause);
        assert!(!p.view().playing);
        act(&mut p, TransportAction::ScrubTo(5.0));
        act(&mut p, TransportAction::ScrubEnd);
        assert!(!p.view().playing);
        assert_eq!(p.view().frame, 150);
        assert_eq!(log.lock().unwrap().starts.len(), 2);
    }

    #[test]
    fn the_loop_wraps() {
        let (mut p, hand, log) = faked();
        assert!(p.view().looping);
        load(&mut p, 1, TestSource::new(), 1_825, true);
        hand.forward(Duration::from_secs(1));
        p.tick();
        let v = p.view();
        // (2,920,000 + 48,000) - 2,928,640 = 39,360: frame 24.
        assert_eq!(v.frame, 24);
        assert!(v.playing);
        // With sound, the device's position is the clock and is kept inside the soundtrack too.
        sound(&mut p, 1);
        log.lock().unwrap().position = Some(NA + pos_of(7));
        p.tick();
        assert_eq!(p.view().frame, 7);
        assert!(p.view().playing);
        // The loop setting reaches the sound while it plays.
        log.lock().unwrap().position = Some(pos_of(7));
        act(&mut p, TransportAction::SetLoop(false));
        act(&mut p, TransportAction::SetLoop(true));
        assert_eq!(log.lock().unwrap().looping, [false, true]);
    }

    #[test]
    fn without_loop_it_stops_on_the_last_frame() {
        let (mut p, hand, log) = faked();
        act(&mut p, TransportAction::SetLoop(false));
        load(&mut p, 1, TestSource::new(), 1_825, true);
        assert!(!p.view().looping);
        hand.forward(Duration::from_secs(1));
        p.tick();
        let v = p.view();
        assert_eq!(v.frame, 1_829);
        assert!(!v.playing);
        hand.forward(Duration::from_secs(1));
        p.tick();
        assert_eq!(p.view().frame, 1_829);
        // Play from the very end starts over.
        act(&mut p, TransportAction::PlayPause);
        let v = p.view();
        assert!(v.playing);
        assert_eq!(v.frame, 0);

        // With sound: the device running past the end stops the sound and the playback on the last frame.
        sound(&mut p, 1);
        assert_eq!(log.lock().unwrap().starts.last(), Some(&(0, false)));
        log.lock().unwrap().position = Some(NA + 300);
        p.tick();
        let v = p.view();
        assert_eq!(v.frame, 1_829);
        assert!(!v.playing);
        assert_eq!(log.lock().unwrap().stops, 1);
    }

    #[test]
    fn a_late_frame_counts_once() {
        let (mut p, _hand, _log) = faked();
        let src = TestSource::gated();
        let _open = Opens(src.clone());
        load(&mut p, 1, src.clone(), 10, false);
        assert_eq!(p.tick(), None);
        assert_eq!(p.stats(), PlayerStats { shown: 0, late: 1 });
        assert_eq!(p.tick(), None);
        assert_eq!(p.stats(), PlayerStats { shown: 0, late: 1 });
        assert!(p.wants_repaint());
        src.open();
        wait_for("frame 10", || p.scheduler.get(10).is_some());
        assert_eq!(p.tick(), Some(TestSource::frame(10)));
        assert_eq!(p.stats(), PlayerStats { shown: 1, late: 1 });
        assert_eq!(p.tick(), None);
        assert_eq!(p.stats(), PlayerStats { shown: 1, late: 1 });
        assert!(!p.wants_repaint());
    }

    #[test]
    fn a_missing_frame_holds_the_latest_one_before_it() {
        // Frame 10 never arrives (its draw fails), so what `tick` holds does not depend on how fast frames are drawn.
        let (mut p, _hand, _log) = faked();
        load(&mut p, 1, TestSource::panicking_at(10), 3, false);
        wait_for("frame 3", || p.scheduler.get(3).is_some());
        // Nothing on screen yet: the playhead's frame is missing, so the latest one before it is given.
        act(&mut p, TransportAction::Step(7));
        assert_eq!(p.tick(), Some(TestSource::frame(3)));
        assert_eq!(p.tick(), None);
        // A move asks for the new frame at once, before the next tick.
        act(&mut p, TransportAction::Step(10));
        wait_for("frame 20", || p.scheduler.get(20).is_some());
        assert_eq!(p.tick(), Some(TestSource::frame(20)));
        // Back to frame 10: the picture on screen is ahead of the playhead, so the latest one before it replaces it.
        act(&mut p, TransportAction::Step(-10));
        assert_eq!(p.tick(), Some(TestSource::frame(3)));
        assert_eq!(p.tick(), None);
        // Only frames shown on time count as shown.
        assert_eq!(p.stats().shown, 1);
    }

    #[test]
    fn a_lost_device_falls_back_to_the_wall_clock() {
        let (mut p, hand, log) = faked();
        load(&mut p, 1, TestSource::new(), 0, true);
        sound(&mut p, 1);
        log.lock().unwrap().position = Some(pos_of(300));
        p.tick();
        assert_eq!(p.view().frame, 300);
        // The device is unplugged.
        log.lock().unwrap().position = None;
        let v = p.view();
        assert_eq!(v.sound, SoundState::NoDevice);
        assert!(v.playing);
        assert_eq!(v.frame, 300);
        p.tick();
        hand.forward(Duration::from_secs(1));
        p.tick();
        let v = p.view();
        assert_eq!(v.sound, SoundState::NoDevice);
        assert!(v.playing);
        assert_eq!(v.frame, 330);
    }

    #[test]
    fn a_failed_frame_keeps_the_last_good_picture() {
        let (mut p, hand, log) = faked();
        load(&mut p, 1, TestSource::panicking_at(5), 4, false);
        sound(&mut p, 1);
        wait_for("frame 4", || p.scheduler.get(4).is_some());
        assert_eq!(p.tick(), Some(TestSource::frame(4)));
        act(&mut p, TransportAction::PlayPause);
        // Playing from frame 4 asks for 5 onwards; 5 fails.
        log.lock().unwrap().position = Some(pos_of(4));
        p.tick();
        wait_for("frame 5 to fail and 6 to be drawn", || {
            p.scheduler.failed(5) && p.scheduler.get(6).is_some()
        });
        assert_eq!(p.failed_frame(), None);
        log.lock().unwrap().position = Some(pos_of(5) + 100);
        assert_eq!(p.tick(), None);
        assert_eq!(p.failed_frame(), Some(5));
        let late = p.stats().late;
        for _ in 0..5 {
            assert_eq!(p.tick(), None);
        }
        assert_eq!(p.stats().late, late);
        let v = p.view();
        assert!(v.playing);
        assert_eq!(v.frame, 5);
        assert_eq!(v.sound, SoundState::Ready);
        assert_eq!(log.lock().unwrap().stops, 0);
        // The next good frame replaces the held picture.
        log.lock().unwrap().position = Some(pos_of(6));
        assert_eq!(p.tick(), Some(TestSource::frame(6)));
        assert_eq!(p.failed_frame(), None);
        assert!(p.view().playing);

        // Back on it, and then a new take clears it.
        act(&mut p, TransportAction::Step(-1));
        assert_eq!(p.failed_frame(), Some(5));
        load(&mut p, 2, TestSource::new(), 5, false);
        assert_eq!(p.failed_frame(), None);
        _ = hand;
    }

    #[test]
    fn two_loads_use_one_scheduler() {
        let (mut p, _hand, _log) = faked();
        // Draw times only this scheduler knows: a new one would report 33 ms, and one real draw (a few ms) cannot
        // bring the mean of 30 below 900.
        for _ in 0..30 {
            p.scheduler.record_draw_ms(1_000.0);
        }
        let old = TestSource::gated();
        let _open = Opens(old.clone());
        load(&mut p, 1, old.clone(), 10, false);
        wait_for("the old take's draw to start", || old.calls() == 1);
        // Loading another take while a draw of the old one is held returns at once.
        let new = TestSource::new();
        let (tx, rx) = mpsc::channel();
        let new2: Arc<dyn FrameSource> = new.clone();
        std::thread::spawn(move || {
            load(&mut p, 2, new2, 20, false);
            _ = tx.send(p);
        });
        let mut p = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("the second load returned");
        let _open_again = Opens(old.clone());
        let ms = p.scheduler.frame_ms();
        assert!(ms > 900.0, "the scheduler reports {ms} ms: it is a new one");
        assert_eq!(p.view().frame, 20);
        wait_for("frame 20 of the new take", || p.scheduler.get(20).is_some());
        assert_eq!(p.tick(), Some(TestSource::frame(20)));
        assert!(new.calls() >= 1);
        // The old take's frame, finished after the switch, is thrown away.
        old.open();
        wait_for("the old draw to end", || p.scheduler.idle());
        assert!(p.scheduler.get(10).is_none());
        assert_eq!(p.tick(), None);
    }

    #[test]
    fn safe_zone_and_clear() {
        let (mut p, _hand, log) = faked();
        assert!(!p.view().safe_zone);
        load(&mut p, 1, TestSource::new(), 0, true);
        sound(&mut p, 1);
        act(&mut p, TransportAction::SetSafeZone(true));
        assert!(p.view().safe_zone);
        p.run(PlayerCmd::Clear);
        let v = p.view();
        assert!(!v.enabled && !v.playing);
        assert_eq!(v.sound, SoundState::None);
        assert_eq!(log.lock().unwrap().stops, 1);
        assert_eq!(p.tick(), None);
        assert!(!p.wants_repaint());
    }
}
