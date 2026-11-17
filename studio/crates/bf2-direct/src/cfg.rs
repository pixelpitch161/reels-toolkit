//! The director's tuning numbers.

/// Frames per second of the video.
pub const FPS: f64 = 30.0;

/// Frames in the 61 s video.
pub const FRAMES: usize = 1830;

/// The director's numbers. Times are fight seconds unless they say video seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct DirectCfg {
    /// The most video seconds slow-mo and the K.O. cinematic may add, all together.
    pub max_extra: f64,
    /// A slow-mo ramps in over this many fight seconds before its cue.
    pub ramp_in: f64,
    /// A slow-mo holds this many fight seconds past its move's Recovery.
    pub hold_after: f64,
    /// A slow-mo ramps back to normal speed over this many fight seconds.
    pub ramp_out: f64,
    /// The freeze at the K.O., in video seconds.
    pub ko_freeze: f64,
    /// The speed after the K.O. freeze (fight seconds per video second).
    pub ko_slow: f64,
    /// How long the K.O. slow lasts, in fight seconds, before it ramps out.
    pub ko_slow_len: f64,
    /// Video seconds between two big shots (punch-in, slam, K.O.), unless the later one has a higher priority.
    pub big_shot_gap: f64,
    /// False turns every slow-mo cue off (the K.O. cinematic stays): for tests and a later switch.
    pub slowmo: bool,
    /// How far the shots and the shake move the camera from the base shot (the current engine's camera): 1 is
    /// each shot in full (a punch-in to 0.62 of the distance, a 12 degree K.O. orbit, 18 px of shake), 0 none.
    /// Aedan, E7: "very subtle for now".
    pub intensity: f64,
}

impl Default for DirectCfg {
    fn default() -> Self {
        DirectCfg {
            max_extra: 4.0,
            ramp_in: 0.15,
            hold_after: 0.25,
            ramp_out: 0.2,
            ko_freeze: 0.35,
            ko_slow: 0.3,
            ko_slow_len: 0.8,
            big_shot_gap: 2.5,
            slowmo: true,
            intensity: 0.2,
        }
    }
}
