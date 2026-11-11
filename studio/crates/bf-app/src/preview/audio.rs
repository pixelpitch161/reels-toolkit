//! Sound output for the preview (§7): the take's soundtrack goes to the PC's default output device, and the device's
//! progress is the preview's clock. Without a device the preview plays silently on the wall clock.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::model::SR;
use crate::model::take::AudioTake;

/// What `open_audio` gives when the PC has no output device that works.
pub const NO_DEVICE: &str = "No sound device was found, so the preview is silent.";

/// Writes interleaved frames of `channels` samples from `src` into `out`, starting at source position `pos` and
/// moving `step` source samples per output frame (straight-line interpolation between neighbouring samples). One
/// channel gets the mean of left and right; channels past the second get 0. At the end of `src` the position wraps
/// to the start when `looping` (the last sample flows into the first); otherwise the rest is silence. Returns
/// whether the end was reached without looping, so nothing more will sound.
pub fn fill(out: &mut [f32], channels: usize, src: &[[f32; 2]], pos: &mut f64, step: f64, looping: bool) -> bool {
    let len = src.len();
    if len == 0 {
        out.fill(0.0);
        return true;
    }
    let end = len as f64;
    if !pos.is_finite() || *pos < 0.0 {
        *pos = 0.0;
    }
    for frame in out.chunks_mut(channels.max(1)) {
        if *pos >= end {
            if looping {
                *pos %= end;
            } else {
                frame.fill(0.0);
                continue;
            }
        }
        // `pos` is finite, non-negative and below `end`, so the cast is its whole part and a valid index.
        let i = *pos as usize;
        let t = (*pos - i as f64) as f32;
        let a = src[i];
        let b = if i + 1 < len {
            src[i + 1]
        } else if looping {
            src[0]
        } else {
            a
        };
        // A product and a sum, never fused.
        let l = a[0] + (b[0] - a[0]) * t;
        let r = a[1] + (b[1] - a[1]) * t;
        if let [only] = frame {
            *only = (l + r) * 0.5;
        } else {
            frame[0] = l;
            frame[1] = r;
            frame[2..].fill(0.0);
        }
        *pos += step;
    }
    !looping && *pos >= end
}

/// Where the preview's sound goes. Every call returns at once: a real device does its work on a thread of its own.
pub trait AudioOut: Send {
    /// Plays `audio` from source sample `at`.
    fn start(&mut self, audio: AudioTake, at: u64, looping: bool) -> Result<(), String>;
    fn stop(&mut self);
    /// The source sample now sounding; `None` when stopped or when the device was lost.
    fn position(&self) -> Option<u64>;
    fn set_looping(&mut self, on: bool);
}

/// No sound at all: every start fails with its text.
pub struct NoAudio(pub String);

impl AudioOut for NoAudio {
    fn start(&mut self, _audio: AudioTake, _at: u64, _looping: bool) -> Result<(), String> {
        Err(self.0.clone())
    }
    fn stop(&mut self) {}
    fn position(&self) -> Option<u64> {
        None
    }
    fn set_looping(&mut self, _on: bool) {}
}

/// The source sample reached after `device_frames` frames at `device_rate` from `start`:
/// `start + device_frames * 48_000 / device_rate`, rounded down (whole source samples), saturating.
pub fn source_pos(start: u64, device_frames: u64, device_rate: u32) -> u64 {
    let moved = u128::from(device_frames) * u128::from(SR) / u128::from(device_rate.max(1));
    start.saturating_add(u64::try_from(moved).unwrap_or(u64::MAX))
}

/// What the interface asks of the sound callback.
enum Cmd {
    Start {
        samples: Arc<[[f32; 2]]>,
        at: u64,
        looping: bool,
        epoch: u32,
    },
    Stop,
    Loop(bool),
}

/// What the callback and the interface share.
struct Shared {
    /// The epoch of the start being played (high 32 bits) and the source sample now sounding (low 32 bits), in one
    /// word so the two are always read together.
    sounding: AtomicU64,
    /// Set by the stream's error callback when the device is gone.
    lost: AtomicBool,
}

/// The default output device. The stream lives on a thread of its own (it is not `Send`); the interface talks to
/// the sound callback through a channel and reads its position from an atomic, so no call here waits for anything.
pub struct CpalOut {
    cmds: mpsc::Sender<Cmd>,
    shared: Arc<Shared>,
    /// Rises with every start, so a position the callback reports for an earlier start is not taken for this one.
    epoch: u32,
    /// Where the latest start began; the position until the callback has picked that start up.
    at: u64,
    running: bool,
    /// Dropping it ends the sound thread, which then closes the stream.
    _alive: mpsc::Sender<()>,
}

impl CpalOut {
    /// Opens the PC's default output device and starts a silent stream on it. Waits for the device to answer, so it
    /// is called once, at start, never inside a frame.
    pub fn open() -> Result<CpalOut, String> {
        let (cmds, cmd_rx) = mpsc::channel();
        let (alive, alive_rx) = mpsc::channel::<()>();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let shared = Arc::new(Shared {
            sounding: AtomicU64::new(0),
            lost: AtomicBool::new(false),
        });
        let s = shared.clone();
        std::thread::Builder::new()
            .name("preview-sound".into())
            .spawn(move || match stream(cmd_rx, s) {
                Ok(stream) => {
                    _ = ready_tx.send(Ok(()));
                    // Keeps the stream until the `CpalOut` is dropped.
                    _ = alive_rx.recv();
                    drop(stream);
                }
                Err(e) => _ = ready_tx.send(Err(e)),
            })
            .map_err(|e| e.to_string())?;
        ready_rx
            .recv()
            .map_err(|_| "the sound thread ended before the device answered".to_string())??;
        Ok(CpalOut {
            cmds,
            shared,
            epoch: 0,
            at: 0,
            running: false,
            _alive: alive,
        })
    }

    fn lost(&self) -> bool {
        self.shared.lost.load(Ordering::Acquire)
    }
}

impl AudioOut for CpalOut {
    fn start(&mut self, audio: AudioTake, at: u64, looping: bool) -> Result<(), String> {
        if self.lost() {
            self.running = false;
            return Err("The sound device was lost.".into());
        }
        self.epoch = self.epoch.wrapping_add(1);
        self.at = at;
        let cmd = Cmd::Start {
            samples: audio.samples,
            at,
            looping,
            epoch: self.epoch,
        };
        self.running = self.cmds.send(cmd).is_ok();
        if self.running {
            Ok(())
        } else {
            Err("The sound device stopped.".into())
        }
    }

    fn stop(&mut self) {
        self.running = false;
        _ = self.cmds.send(Cmd::Stop);
    }

    fn position(&self) -> Option<u64> {
        if !self.running || self.lost() {
            return None;
        }
        let word = self.shared.sounding.load(Ordering::Acquire);
        if (word >> 32) as u32 == self.epoch {
            Some(word & u64::from(u32::MAX))
        } else {
            Some(self.at)
        }
    }

    fn set_looping(&mut self, on: bool) {
        _ = self.cmds.send(Cmd::Loop(on));
    }
}

/// The take the callback is playing.
struct Playing {
    samples: Arc<[[f32; 2]]>,
    /// Where `fill` reads next.
    pos: f64,
    looping: bool,
    epoch: u32,
    /// The source sample the count of device frames starts from.
    from: u64,
    /// Device frames written since `from`.
    frames: u64,
}

/// Whether a stream error means the device is gone (rather than a glitch the stream survives).
fn is_loss(kind: cpal::ErrorKind) -> bool {
    !matches!(
        kind,
        cpal::ErrorKind::Xrun | cpal::ErrorKind::DeviceChanged | cpal::ErrorKind::RealtimeDenied
    )
}

/// Builds and starts the output stream on the default device: `f32` frames in the device's default layout, filled
/// from the current take, or silence.
fn stream(cmds: mpsc::Receiver<Cmd>, shared: Arc<Shared>) -> Result<cpal::Stream, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "there is no default output device".to_string())?;
    let config: cpal::StreamConfig = device.default_output_config().map_err(|e| e.to_string())?.into();
    let channels = usize::from(config.channels).max(1);
    let rate = config.sample_rate;
    if rate == 0 {
        return Err("the output device reports a rate of 0".into());
    }
    let step = f64::from(SR) / f64::from(rate);
    let on_error = shared.clone();
    let mut playing: Option<Playing> = None;
    let data = move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
        while let Ok(cmd) = cmds.try_recv() {
            match cmd {
                Cmd::Start {
                    samples,
                    at,
                    looping,
                    epoch,
                } => {
                    playing = Some(Playing {
                        samples,
                        pos: at as f64,
                        looping,
                        epoch,
                        from: at,
                        frames: 0,
                    });
                }
                Cmd::Stop => playing = None,
                Cmd::Loop(on) => {
                    if let Some(p) = &mut playing {
                        // Counting starts again from where `fill` is, so a wrap counted before stays counted.
                        p.looping = on;
                        p.from = p.pos as u64;
                        p.frames = 0;
                    }
                }
            }
        }
        let Some(p) = &mut playing else {
            out.fill(0.0);
            return;
        };
        // The first frame of this buffer is the one now starting to sound.
        let mut now = source_pos(p.from, p.frames, rate);
        let len = p.samples.len() as u64;
        if p.looping && len > 0 {
            now %= len;
        }
        let word = (u64::from(p.epoch) << 32) | now.min(u64::from(u32::MAX));
        shared.sounding.store(word, Ordering::Release);
        fill(out, channels, &p.samples, &mut p.pos, step, p.looping);
        p.frames += (out.len() / channels) as u64;
    };
    let error = move |err: cpal::Error| {
        if is_loss(err.kind()) {
            on_error.lost.store(true, Ordering::Release);
        }
    };
    let stream = device
        .build_output_stream(config, data, error, None)
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}

/// The PC's default output device, or [`NoAudio`] saying the preview is silent.
pub fn open_audio() -> Box<dyn AudioOut> {
    match CpalOut::open() {
        Ok(out) => Box::new(out),
        Err(_) => Box::new(NoAudio(NO_DEVICE.into())),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    /// A source whose sample `i` is `[i, i]`.
    fn ramp(n: usize) -> Vec<[f32; 2]> {
        (0..n).map(|i| [i as f32, i as f32]).collect()
    }

    fn left(out: &[f32]) -> Vec<f32> {
        out.chunks(2).map(|f| f[0]).collect()
    }

    #[test]
    fn step_one_copies() {
        let src = [[0.1, -0.2], [0.3, -0.4], [0.5, -0.6]];
        let mut out = [9.0; 6];
        let mut pos = 0.0;
        assert!(!fill(&mut out, 2, &src, &mut pos, 1.0, true));
        assert_eq!(out, [0.1, -0.2, 0.3, -0.4, 0.5, -0.6]);
        // The wrap happens when the next frame is written.
        assert_eq!(pos, 3.0);
    }

    #[test]
    fn one_channel_gets_the_mean() {
        let src = [[1.0, 0.0], [0.5, 0.25]];
        let mut out = [9.0; 2];
        let mut pos = 0.0;
        fill(&mut out, 1, &src, &mut pos, 1.0, true);
        assert_eq!(out, [0.5, 0.375]);
    }

    #[test]
    fn six_channels_get_two_and_silence() {
        let src = [[1.0, 2.0], [3.0, 4.0]];
        let mut out = [9.0; 12];
        let mut pos = 0.0;
        fill(&mut out, 6, &src, &mut pos, 1.0, true);
        assert_eq!(out, [1.0, 2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 4.0, 0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn half_step_interpolates() {
        let src = [[0.0, 0.0], [1.0, 1.0]];
        let mut out = [9.0; 6];
        let mut pos = 0.0;
        assert!(!fill(&mut out, 2, &src, &mut pos, 0.5, false));
        assert_eq!(left(&out), [0.0, 0.5, 1.0]);
        assert_eq!(pos, 1.5);
    }

    #[test]
    fn looping_wraps_exactly() {
        let src = ramp(4);
        let mut out = [9.0; 12];
        let mut pos = 0.0;
        assert!(!fill(&mut out, 2, &src, &mut pos, 1.0, true));
        assert_eq!(left(&out), [0.0, 1.0, 2.0, 3.0, 0.0, 1.0]);
        assert_eq!(pos, 2.0);
    }

    #[test]
    fn the_end_is_silence() {
        let src = ramp(4);
        let mut out = [9.0; 14];
        let mut pos = 0.0;
        assert!(fill(&mut out, 2, &src, &mut pos, 1.0, false));
        assert_eq!(
            out,
            [0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        );
        // Once at the end it stays silent.
        let mut out = [9.0; 4];
        assert!(fill(&mut out, 2, &src, &mut pos, 1.0, false));
        assert_eq!(out, [0.0; 4]);
    }

    #[test]
    fn source_pos_counts_device_frames_at_48_khz() {
        assert_eq!(source_pos(1_000, 44_100, 44_100), 49_000);
        assert_eq!(source_pos(0, 22_050, 44_100), 24_000);
        assert_eq!(source_pos(7, 48_000, 48_000), 48_007);
        // Rounded down: one frame at 44.1 kHz is 1.088 source samples.
        assert_eq!(source_pos(0, 1, 44_100), 1);
        assert_eq!(source_pos(0, 3, 96_000), 1);
    }

    #[test]
    fn no_audio_fails_with_its_text() {
        let mut out = NoAudio("No sound here.".into());
        let audio = AudioTake {
            samples: vec![[0.0; 2]; 4].into(),
        };
        assert_eq!(out.start(audio, 0, true), Err("No sound here.".to_string()));
        assert_eq!(out.position(), None);
    }

    /// Needs a sound device: run it by name on this PC with `--ignored`.
    #[test]
    #[ignore]
    fn this_pc_plays_silence() {
        let mut out = open_audio();
        let audio = AudioTake {
            samples: vec![[0.0; 2]; 48_000].into(),
        };
        out.start(audio, 0, false).expect("the sound starts");
        let until = Instant::now() + Duration::from_secs(2);
        loop {
            let pos = out.position().expect("the sound is running");
            if pos > 24_000 {
                break;
            }
            assert!(Instant::now() < until, "the position is only {pos} after 2 s");
            std::thread::sleep(Duration::from_millis(5));
        }
        out.stop();
        assert_eq!(out.position(), None);
    }
}
