//! The preview player (§7): one take's picture and sound, and the transport over them. The sound is the clock: while
//! the take's soundtrack sounds, the playhead is wherever the device has got to; before the sound is built, without a
//! device, or after the device is lost, it moves with the wall clock. Pictures come from the draw-ahead scheduler;
//! nothing here waits for one.

use std::time::Duration;

use crate::model::take::{AudioTake, Frame, PlayerCmd, SoundState, TransportAction, TransportView};
use crate::model::{NA, NF, SR, TakeId};
use crate::preview::audio::AudioOut;
use crate::preview::clock::{advance, frame_of, pos_of, step};
use crate::preview::scheduler::Scheduler;
use crate::seam::Waker;

/// The wall clock: any steadily rising time (the app passes the time since it started).
pub type Now = Box<dyn Fn() -> Duration + Send>;

/// How long the sound may report the same position while it is believed to be running before the device counts as
/// stalled (ruling M2-R12): it is then treated as lost.
const STALL: Duration = Duration::from_millis(500);

/// How long playback waits at most, after a `Load` with autoplay, for the take's first picture before its clock
/// starts anyway (ruling M2-R30). A first frame whose draw fails ends the wait at once.
pub const FIRST_PICTURE_WAIT: Duration = Duration::from_secs(3);

/// What the player counts about the frames it gives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerStats {
    /// The playhead's own frames given when they were kept (a held picture does not count).
    pub shown: u64,
    /// Frames not kept yet when the playhead reached them, once per frame. This also counts the first missing frame
    /// after a `Load` without autoplay, a `Step` or a scrub while paused, not only frames missed during playback.
    /// While playback waits for a take's first picture nothing is counted: the playhead is not moving.
    pub late: u64,
}

/// Where the playhead is right now, read without changing anything.
struct Reading {
    pos: u64,
    playing: bool,
    /// The sound was running and the device no longer answers (or has stalled).
    lost: bool,
}

/// Plays one take at a time. It owns one scheduler for its whole life and switches takes on it (widget rule 12).
///
/// Dropping a `Player` drops its scheduler, which joins the worker threads and can wait for a draw in progress, so it
/// is dropped only when the app closes, never on the interface thread while the app runs.
///
/// [`Player::has_picture`] says whether the shell should show a picture at all: false at the start, after `Clear`,
/// after a `Still` until `tick` has given it, and after a `Load` until that take's first frame has been given (so the
/// previous take's last picture is never shown under the new take's transport).
///
/// **Playback starts with the first picture** (ruling M2-R30). After a `Load` with autoplay the transport shows
/// playing, but the playhead stays on the start frame, and the scheduler is asked for that frame first, until `tick`
/// gives it: then the clock starts from the start frame. A soundtrack that arrives meanwhile is kept and started at
/// that moment, at the start frame's first sample, so from then on the audio position is the device's count from the
/// start frame and picture and sound are in step. The wait ends anyway when the start frame's draw fails (its
/// failed-frame status shows, widget rule 11) or after [`FIRST_PICTURE_WAIT`]. Pausing, stepping or scrubbing ends
/// the wait as they end playback; nothing is counted late while waiting, and the stall rule cannot fire since the
/// sound is not started.
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
    /// Whether `tick` has given a picture that belongs to what is loaded now (a still, or a frame of this take).
    picture: bool,
    /// While sounding: the last position the audio reported, and when (by `now`) it last moved or the audio started.
    heard: u64,
    heard_at: Duration,
    /// Playing after a `Load`, waiting for the take's first picture: the clock has not started. Since when.
    waiting: Option<Duration>,
    stats: PlayerStats,
}

impl Player {
    /// A player with `threads` drawing threads, sound through `audio`, wall time from `now`; `waker` is called when a
    /// frame is drawn (or fails), so the interface repaints.
    pub fn new(threads: usize, audio: Box<dyn AudioOut>, now: Now, waker: Waker) -> Player {
        Player::with(Scheduler::new(threads, waker), audio, now)
    }

    /// A player whose scheduler records every draw as taking `ms` milliseconds, so the lead and the stride it asks
    /// for do not depend on how fast this machine draws.
    #[cfg(test)]
    pub(crate) fn with_draw_ms(threads: usize, audio: Box<dyn AudioOut>, now: Now, waker: Waker, ms: f32) -> Player {
        Player::with(Scheduler::with_draw_ms(threads, waker, ms), audio, now)
    }

    fn with(scheduler: Scheduler, audio: Box<dyn AudioOut>, now: Now) -> Player {
        Player {
            scheduler,
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
            picture: false,
            heard: 0,
            heard_at: Duration::ZERO,
            waiting: None,
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
                self.picture = false;
                if autoplay {
                    // The clock starts with the first picture (`tick`).
                    self.waiting = Some((self.now)());
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
                // While waiting for the first picture the sound is kept and starts with the clock.
                if self.playing && self.waiting.is_none() {
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
                self.leave_scrub_paused();
                if self.playing {
                    self.halt();
                } else {
                    // On the last frame with loop off there is nothing left to play: start over.
                    if !self.looping && frame_of(self.pos) == NF - 1 {
                        self.pos = 0;
                    }
                    self.play();
                }
            }
            TransportAction::Step(n) => {
                self.leave_scrub_paused();
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

    /// Inside a scrub, a play/pause or a step is the owner's own choice: the scrub's end no longer resumes.
    fn leave_scrub_paused(&mut self) {
        if self.scrub.is_some() {
            self.scrub = Some(false);
        }
    }

    /// Whether the shell should show a picture: a still or a frame of the loaded take has been given by `tick`.
    /// False at the start, after `Clear`, after `Still` until it is given and after `Load` until the take's first
    /// frame is given.
    pub fn has_picture(&self) -> bool {
        self.picture
    }

    /// Once per repaint: moves the clock, asks the scheduler for the playhead's frame and gives a picture when the one
    /// to show has changed. A frame not kept yet (late, or failed) holds the latest kept one before it.
    pub fn tick(&mut self) -> Option<Frame> {
        if let Some(frame) = self.still.take() {
            self.picture = true;
            return Some(frame);
        }
        self.take?;
        self.settle();
        if let Some(since) = self.waiting {
            let f = frame_of(self.pos);
            let given_up = (self.now)().saturating_sub(since) >= FIRST_PICTURE_WAIT;
            if self.scheduler.get(f).is_some() || self.scheduler.failed(f) || given_up {
                // The clock starts now, from the start frame (and the sound with it, if it has arrived).
                self.waiting = None;
                self.play();
            } else {
                self.ask();
                return None;
            }
        }
        self.ask();
        let f = frame_of(self.pos);
        if self.shown == Some(f) {
            return None;
        }
        if let Some(frame) = self.scheduler.get(f) {
            self.shown = Some(f);
            self.late_at = None;
            self.picture = true;
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
        // The scheduler keeps only the loaded take's frames, so a held picture belongs to this take too.
        self.shown = Some(n);
        self.picture = true;
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
        self.picture = false;
    }

    /// Starts playback from `pos`: on the sound when it has arrived and starts, else on the wall clock.
    fn play(&mut self) {
        self.playing = true;
        self.sounding = false;
        if self.waiting.is_some() {
            // Neither the sound nor the wall clock starts before the first picture.
            return;
        }
        if let Some(audio) = &self.sound_take {
            match self.audio.start(audio.clone(), self.pos, self.looping) {
                Ok(()) => {
                    self.sounding = true;
                    self.sound = SoundState::Ready;
                    // The first movement gets the same allowance as any later one.
                    self.heard = self.pos;
                    self.heard_at = (self.now)();
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
        self.waiting = None;
    }

    /// Measures wall-clock playback from now and the current position.
    fn restart_wall(&mut self) {
        self.wall_from = (self.now)();
        self.wall_pos = self.pos;
    }

    /// Where the playhead is now, from the sound while it sounds, else from the wall clock. The sound counts as lost
    /// when the device no longer answers, or when it has answered the same position for `STALL` (a device that stopped
    /// calling back without an error).
    fn read(&self) -> Reading {
        if !self.playing {
            return Reading {
                pos: self.pos,
                playing: false,
                lost: false,
            };
        }
        if self.waiting.is_some() {
            // Playing, but the clock has not started: the playhead stays on the start frame.
            return Reading {
                pos: self.pos,
                playing: true,
                lost: false,
            };
        }
        let (pos, playing, lost) = if self.sounding {
            match self.audio.position() {
                Some(p) => {
                    let (pos, on) = advance(p, 0, self.looping);
                    let stalled = pos == self.heard && (self.now)().saturating_sub(self.heard_at) >= STALL;
                    (pos, on || stalled, stalled)
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

    /// Takes in what `read` sees: the new position, the end of playback, a lost or stalled device (the audio is
    /// stopped and playback goes on from the same position on the wall clock).
    fn settle(&mut self) {
        let r = self.read();
        self.pos = r.pos;
        if self.sounding && !r.lost && r.pos != self.heard {
            self.heard = r.pos;
            self.heard_at = (self.now)();
        }
        if r.lost {
            self.audio.stop();
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
            if self.waiting.is_some() {
                self.scheduler.want_start(frame_of(self.pos), self.looping);
            } else {
                self.scheduler.want(frame_of(self.pos), self.playing, self.looping);
            }
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

    use super::*;
    use crate::model::take::FrameSource;
    use crate::preview::audio::NoAudio;
    use crate::preview::scheduler::testing::{TestSource, wait_for};

    /// Every draw counts as this many milliseconds, so the lead and the stride are 1 however fast this machine draws.
    const DRAW_MS: f32 = 1.0;

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

    /// A player with two threads whose draws all count as `ms`, and the hand that moves its clock.
    fn player_with(audio: Box<dyn AudioOut>, ms: f32) -> (Player, Hand) {
        let t = Arc::new(AtomicU64::new(1_000_000_000));
        let t2 = t.clone();
        let now: Now = Box::new(move || Duration::from_nanos(t2.load(Ordering::SeqCst)));
        (Player::with_draw_ms(2, audio, now, Arc::new(|| {}), ms), Hand(t))
    }

    fn faked_with(ms: f32) -> (Player, Hand, Arc<Mutex<Log>>) {
        let log = Arc::new(Mutex::new(Log::default()));
        let (p, hand) = player_with(Box::new(FakeOut(log.clone())), ms);
        (p, hand, log)
    }

    fn faked() -> (Player, Hand, Arc<Mutex<Log>>) {
        faked_with(DRAW_MS)
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

    /// Waits until the take's first picture is given: after a `Load` with autoplay the clock starts then (ruling
    /// M2-R30).
    fn started(p: &mut Player) {
        wait_for("the first picture", || {
            p.tick();
            p.has_picture()
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
        started(&mut p);
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
        started(&mut p);
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
        let (mut p, hand) = player_with(Box::new(NoAudio::new("No sound device.")), DRAW_MS);
        load(&mut p, 1, TestSource::new(), 0, true);
        started(&mut p);
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
        started(&mut p);
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
        started(&mut p);
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
        started(&mut p);
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
        started(&mut p);
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
        started(&mut p);
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
        // Depends on the lead being 1 (frame 5 is asked for from frame 4): the draws count as `DRAW_MS`.
        let (mut p, _hand, log) = faked();
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
    }

    /// A test source that remembers which frames it drew, so a kept frame can be told apart from another take's
    /// (both draw the same bytes for the same frame).
    struct Recording {
        inner: Arc<TestSource>,
        drawn: Mutex<Vec<u32>>,
    }

    impl FrameSource for Recording {
        fn frames(&self) -> u32 {
            self.inner.frames()
        }

        fn draw(&self, f: u32) -> Frame {
            let frame = self.inner.draw(f);
            self.drawn.lock().unwrap().push(f);
            frame
        }
    }

    #[test]
    fn two_loads_use_one_scheduler() {
        // Every draw of this scheduler counts as 1,000 ms, and so do the 30 recorded here before any draw: a new
        // scheduler would report 33 ms.
        let (mut p, _hand, _log) = faked_with(1_000.0);
        for _ in 0..30 {
            p.scheduler.record_draw_ms(1_000.0);
        }
        let old = TestSource::gated();
        let _open = Opens(old.clone());
        load(&mut p, 1, old.clone(), 10, false);
        wait_for("the old take's draw to start", || old.calls() == 1);
        // Loading another take while a draw of the old one is held does not wait for that draw: the old draw stays
        // held (its gate opens only below), so a load that waited for it would never return. The load runs on
        // another thread and the test waits for it on a condition, whatever the machine's load.
        let new = Arc::new(Recording {
            inner: TestSource::new(),
            drawn: Mutex::new(Vec::new()),
        });
        let (tx, rx) = mpsc::channel();
        let new2: Arc<dyn FrameSource> = new.clone();
        std::thread::spawn(move || {
            load(&mut p, 2, new2, 20, false);
            _ = tx.send(p);
        });
        let mut returned = None;
        wait_for("the second load to return while the old draw is held", || {
            returned = returned.take().or_else(|| rx.try_recv().ok());
            returned.is_some()
        });
        let mut p = returned.expect("the second load returned");
        let _open_again = Opens(old.clone());
        assert_eq!(p.scheduler.frame_ms(), 1_000.0);
        assert_eq!(p.view().frame, 20);
        wait_for("frame 20 of the new take", || p.scheduler.get(20).is_some());
        assert_eq!(p.tick(), Some(TestSource::frame(20)));
        assert!(new.inner.calls() >= 1);
        // The old take's frame, finished after the switch, is thrown away. (A frame 10 may still be kept: between
        // the second load's new source and its new playhead, a free worker can draw frame 10 of the NEW take, which
        // is then rightly kept. Any kept frame 10 must be the new take's.)
        old.open();
        wait_for("the old draw to end", || p.scheduler.idle());
        let new_drew_10 = new.drawn.lock().unwrap().contains(&10);
        assert!(
            p.scheduler.get(10).is_none() || new_drew_10,
            "the old take's frame 10 was kept"
        );
        assert_eq!(p.tick(), None);
    }

    #[test]
    fn safe_zone_and_clear() {
        let (mut p, _hand, log) = faked();
        assert!(!p.view().safe_zone);
        assert!(!p.has_picture());
        load(&mut p, 1, TestSource::new(), 0, false);
        wait_for("frame 0", || p.scheduler.get(0).is_some());
        assert_eq!(p.tick(), Some(TestSource::frame(0)));
        assert!(p.has_picture());
        sound(&mut p, 1);
        act(&mut p, TransportAction::PlayPause);
        act(&mut p, TransportAction::SetSafeZone(true));
        assert!(p.view().safe_zone);
        p.run(PlayerCmd::Clear);
        let v = p.view();
        assert!(!v.enabled && !v.playing);
        assert_eq!(v.sound, SoundState::None);
        assert_eq!(log.lock().unwrap().stops, 1);
        assert!(!p.has_picture());
        assert_eq!(p.tick(), None);
        assert!(!p.has_picture());
        assert!(!p.wants_repaint());
    }

    #[test]
    fn has_picture_follows_the_still_and_the_take() {
        let (mut p, _hand, _log) = faked();
        // A still: there once `tick` has given it, and it stays (the transport is disabled, the picture is not).
        p.run(PlayerCmd::Still(TestSource::frame(9)));
        assert!(!p.has_picture());
        assert_eq!(p.tick(), Some(TestSource::frame(9)));
        assert!(p.has_picture());
        assert!(!p.view().enabled);
        assert_eq!(p.tick(), None);
        assert!(p.has_picture());
        // A take whose frames are held back: no picture until its first frame is given.
        let src = TestSource::gated();
        let _open = Opens(src.clone());
        load(&mut p, 1, src.clone(), 10, false);
        assert!(!p.has_picture());
        assert_eq!(p.tick(), None);
        assert!(!p.has_picture());
        src.open();
        wait_for("frame 10", || p.scheduler.get(10).is_some());
        assert_eq!(p.tick(), Some(TestSource::frame(10)));
        assert!(p.has_picture());
        // Another take: the old picture no longer counts until the new take's first frame.
        let src2 = TestSource::gated();
        let _open2 = Opens(src2.clone());
        load(&mut p, 2, src2.clone(), 30, false);
        assert!(!p.has_picture());
        assert_eq!(p.tick(), None);
        assert!(!p.has_picture());
        src2.open();
        wait_for("frame 30", || p.scheduler.get(30).is_some());
        assert_eq!(p.tick(), Some(TestSource::frame(30)));
        assert!(p.has_picture());
    }

    #[test]
    fn play_on_the_last_frame_without_loop_starts_over() {
        let (mut p, _hand, _log) = faked();
        act(&mut p, TransportAction::SetLoop(false));
        load(&mut p, 1, TestSource::new(), 1_000, false);
        // Stopped on the last frame by a step, well before the last sample.
        act(&mut p, TransportAction::Step(5_000));
        assert_eq!(p.view().frame, 1_829);
        act(&mut p, TransportAction::PlayPause);
        let v = p.view();
        assert!(v.playing);
        assert_eq!(v.frame, 0);
        // With loop on, play from the last frame plays on (and wraps).
        act(&mut p, TransportAction::Step(5_000));
        act(&mut p, TransportAction::SetLoop(true));
        act(&mut p, TransportAction::PlayPause);
        let v = p.view();
        assert!(v.playing);
        assert_eq!(v.frame, 1_829);
    }

    #[test]
    fn a_choice_inside_a_scrub_is_kept_at_its_end() {
        let (mut p, _hand, log) = faked();
        load(&mut p, 1, TestSource::new(), 0, true);
        sound(&mut p, 1);
        // A step inside a scrub that began while playing: the scrub's end does not resume.
        act(&mut p, TransportAction::ScrubTo(5.0));
        act(&mut p, TransportAction::Step(1));
        act(&mut p, TransportAction::ScrubEnd);
        assert!(!p.view().playing);
        assert_eq!(p.view().frame, 151);
        // Play inside a scrub, then pause: the scrub's end leaves it paused.
        act(&mut p, TransportAction::PlayPause);
        act(&mut p, TransportAction::ScrubTo(10.0));
        act(&mut p, TransportAction::PlayPause);
        assert!(p.view().playing);
        act(&mut p, TransportAction::PlayPause);
        act(&mut p, TransportAction::ScrubEnd);
        assert!(!p.view().playing);
        assert_eq!(p.view().frame, 300);
        // Play inside a scrub: still playing after its end, and the sound was not started twice.
        act(&mut p, TransportAction::PlayPause);
        act(&mut p, TransportAction::ScrubTo(20.0));
        act(&mut p, TransportAction::PlayPause);
        let starts = log.lock().unwrap().starts.len();
        act(&mut p, TransportAction::ScrubEnd);
        assert!(p.view().playing);
        assert_eq!(log.lock().unwrap().starts.len(), starts);
    }

    #[test]
    fn a_stalled_device_falls_back_to_the_wall_clock() {
        let (mut p, hand, log) = faked();
        load(&mut p, 1, TestSource::new(), 0, true);
        started(&mut p);
        sound(&mut p, 1);
        log.lock().unwrap().position = Some(pos_of(300));
        p.tick();
        assert_eq!(p.view().sound, SoundState::Ready);
        // The device stops calling back: its position stays where it was.
        hand.forward(Duration::from_millis(600));
        let v = p.view();
        assert_eq!(v.sound, SoundState::NoDevice);
        assert!(v.playing);
        assert_eq!(v.frame, 300);
        p.tick();
        assert_eq!(log.lock().unwrap().stops, 1);
        hand.forward(Duration::from_secs(1));
        p.tick();
        let v = p.view();
        assert_eq!(v.sound, SoundState::NoDevice);
        assert!(v.playing);
        assert_eq!(v.frame, 330);
    }

    #[test]
    fn a_moving_device_never_counts_as_stalled() {
        let (mut p, hand, log) = faked();
        load(&mut p, 1, TestSource::new(), 0, true);
        started(&mut p);
        sound(&mut p, 1);
        // Right after the start the position still answers the start value: the first movement gets 500 ms too.
        hand.forward(Duration::from_millis(450));
        p.tick();
        assert_eq!(p.view().sound, SoundState::Ready);
        for k in 1..=20u32 {
            log.lock().unwrap().position = Some(pos_of(k * 10));
            p.tick();
            hand.forward(Duration::from_millis(450));
            p.tick();
            let v = p.view();
            assert_eq!(v.sound, SoundState::Ready, "after move {k}");
            assert_eq!(v.frame, k * 10);
        }
        assert_eq!(log.lock().unwrap().stops, 0);
    }

    /// A source none of whose frames can be drawn.
    struct Broken;

    impl FrameSource for Broken {
        fn frames(&self) -> u32 {
            NF
        }

        fn draw(&self, f: u32) -> Frame {
            panic!("frame {f} cannot be drawn");
        }
    }

    #[test]
    fn the_clock_waits_for_the_first_picture() {
        let (mut p, hand, log) = faked();
        let src = TestSource::gated();
        let _open = Opens(src.clone());
        load(&mut p, 1, src.clone(), 108, true);
        // Playing, but nothing is drawn yet: the playhead stays on the start frame however long it takes.
        hand.forward(Duration::from_secs(2));
        assert_eq!(p.tick(), None);
        let v = p.view();
        assert!(v.playing);
        assert_eq!(v.frame, 108);
        assert!(!p.has_picture());
        assert!(p.wants_repaint());
        assert_eq!(p.stats(), PlayerStats::default(), "nothing counts late while waiting");
        // The sound arriving meanwhile does not start yet, and the 500 ms stall rule does not fire.
        sound(&mut p, 1);
        hand.forward(Duration::from_millis(600));
        assert_eq!(p.tick(), None);
        assert!(log.lock().unwrap().starts.is_empty());
        assert_eq!(p.view().sound, SoundState::Ready);
        assert_eq!(p.view().frame, 108);
        // The start frame is asked for first and is the first thing shown; the clock and the sound start with it,
        // at the start frame's first sample, so picture and sound are in step.
        src.open();
        wait_for("frame 108", || p.scheduler.get(108).is_some());
        assert_eq!(p.tick(), Some(TestSource::frame(108)));
        assert!(p.has_picture());
        assert_eq!(p.stats(), PlayerStats { shown: 1, late: 0 });
        assert_eq!(log.lock().unwrap().starts, [(pos_of(108), true)]);
        // From then on the device's count from the start frame is the playhead.
        log.lock().unwrap().position = Some(pos_of(108) + 48_000);
        p.tick();
        assert_eq!(p.view().frame, 138);
        assert_eq!(p.view().sound, SoundState::Ready);
    }

    #[test]
    fn without_sound_the_clock_runs_from_the_start_frame_after_the_first_picture() {
        let (mut p, hand, _log) = faked();
        let src = TestSource::gated();
        let _open = Opens(src.clone());
        load(&mut p, 1, src.clone(), 50, true);
        hand.forward(Duration::from_millis(700));
        assert_eq!(p.tick(), None);
        src.open();
        let mut first = None;
        wait_for("the first picture", || {
            first = first.take().or_else(|| p.tick());
            first.is_some()
        });
        assert_eq!(
            first,
            Some(TestSource::frame(50)),
            "the first frame shown is the start frame"
        );
        assert_eq!(p.view().frame, 50);
        hand.forward(Duration::from_secs(1));
        p.tick();
        assert_eq!(p.view().frame, 80);
        assert!(p.view().playing);
    }

    #[test]
    fn a_first_picture_that_cannot_be_drawn_does_not_hang() {
        // Every draw fails: the start frame's failure ends the wait, its status shows, and the clock runs.
        let (mut p, hand, _log) = faked();
        load(&mut p, 1, Arc::new(Broken), 10, true);
        wait_for("frame 10 to fail", || p.scheduler.failed(10));
        assert_eq!(p.tick(), None);
        assert_eq!(p.failed_frame(), Some(10));
        assert!(!p.has_picture());
        hand.forward(Duration::from_secs(1));
        p.tick();
        assert_eq!(p.view().frame, 40);
        assert!(p.view().playing);
    }

    #[test]
    fn a_first_picture_that_never_comes_starts_the_clock_after_the_wait() {
        let (mut p, hand, _log) = faked();
        let src = TestSource::gated();
        let _open = Opens(src.clone());
        load(&mut p, 1, src.clone(), 10, true);
        hand.forward(FIRST_PICTURE_WAIT - Duration::from_millis(1));
        assert_eq!(p.tick(), None);
        assert_eq!(p.view().frame, 10);
        hand.forward(Duration::from_millis(1));
        p.tick();
        hand.forward(Duration::from_secs(1));
        p.tick();
        assert_eq!(p.view().frame, 40);
        assert!(p.view().playing);
    }

    #[test]
    fn pausing_while_waiting_stays_on_the_start_frame() {
        let (mut p, hand, log) = faked();
        let src = TestSource::gated();
        let _open = Opens(src.clone());
        load(&mut p, 1, src.clone(), 10, true);
        sound(&mut p, 1);
        act(&mut p, TransportAction::PlayPause);
        assert!(!p.view().playing);
        src.open();
        wait_for("frame 10", || p.scheduler.get(10).is_some());
        assert_eq!(p.tick(), Some(TestSource::frame(10)));
        hand.forward(Duration::from_secs(1));
        p.tick();
        assert_eq!(p.view().frame, 10);
        assert!(log.lock().unwrap().starts.is_empty());
        // Play from there plays at once, as before.
        act(&mut p, TransportAction::PlayPause);
        assert_eq!(log.lock().unwrap().starts, [(pos_of(10), true)]);
    }
}
