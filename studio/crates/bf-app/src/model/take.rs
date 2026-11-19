use std::sync::Arc;

use super::TakeId;

/// One picture of the video, `w` x `h` pixels of RGB: 360 x 640 unless said.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub w: u32,
    pub h: u32,
    pub rgb: Arc<[u8]>,
}

/// Draws the frames of one take on demand, on whichever thread asks (the preview draws ahead on several).
pub trait FrameSource: Send + Sync {
    /// How many frames the take has.
    fn frames(&self) -> u32;
    /// Draws frame `f` (0-based).
    fn draw(&self, f: u32) -> Frame;
}

impl std::fmt::Debug for dyn FrameSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameSource")
            .field("frames", &self.frames())
            .finish_non_exhaustive()
    }
}

/// A take's soundtrack: 48 kHz stereo, `NA` samples long.
#[derive(Clone, PartialEq)]
pub struct AudioTake {
    pub samples: Arc<[[f32; 2]]>,
}

impl std::fmt::Debug for AudioTake {
    /// The length only: a soundtrack has almost three million samples.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioTake")
            .field("samples", &self.samples.len())
            .finish()
    }
}

/// One of the take's checks (such as "Loudness"), with what it found.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckLine {
    pub title: String,
    pub pass: bool,
    pub note: String,
}

/// Which group of the Music screen's lanes a lane is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LaneGroup {
    Fighters,
    Band,
}

/// One part of the soundtrack and where it sounds.
#[derive(Clone, Debug, PartialEq)]
pub struct Lane {
    pub group: LaneGroup,
    pub name: String,
    /// A fighter's colour; band lanes have none.
    pub color: Option<[u8; 3]>,
    /// (start, end) in seconds.
    pub clips: Vec<(f32, f32)>,
}

/// What the Music screen shows about a take's soundtrack.
#[derive(Clone, Debug, PartialEq)]
pub struct MusicView {
    pub checks: Vec<CheckLine>,
    pub lanes: Vec<Lane>,
    /// (seconds, label) of the fight's moments.
    pub markers: Vec<(f32, String)>,
}

/// What the owner does with the transport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransportAction {
    PlayPause,
    /// Frames forward (or back, when negative).
    Step(i32),
    /// Seconds into the video.
    ScrubTo(f32),
    ScrubEnd,
    SetLoop(bool),
    SetSafeZone(bool),
}

/// Whether the preview can be heard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoundState {
    None,
    Building,
    Ready,
    NoDevice,
}

/// What the transport shows, copied from the player.
#[derive(Clone, Debug, PartialEq)]
pub struct TransportView {
    pub enabled: bool,
    pub frame: u32,
    pub playing: bool,
    pub looping: bool,
    pub safe_zone: bool,
    pub sound: SoundState,
}

/// What the interface tells the preview player.
#[derive(Clone, Debug)]
pub enum PlayerCmd {
    /// Show nothing.
    Clear,
    /// Show one picture and stop (such as the title card).
    Still(Frame),
    /// Play a take's frames from `start`.
    Load {
        take: TakeId,
        frames: Arc<dyn FrameSource>,
        start: u32,
        autoplay: bool,
    },
    /// The take's sound, when it is built; ignored for any other take.
    Sound {
        take: TakeId,
        audio: AudioTake,
    },
    Transport(TransportAction),
}
