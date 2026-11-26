//! Where a Voice Engine 2 voice sits in the stereo picture, and its own room: applied to its bus by the Composed
//! polish (`crate::polish`).

/// A voice's place in the stereo picture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Space {
    /// How wide it spreads, 0 (a point) to about 0.5.
    pub width: f64,
    /// How far its bass leans left and its treble right, as a player hears a piano from the bench (0 for none).
    pub tilt: f64,
    /// Its own room, if it has one.
    pub home: Option<Home>,
}

/// A voice's own room: `length` seconds long, `under` LU under the dry sound, high-passed at `hp` Hz so it never
/// muddies the bass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Home {
    pub length: f64,
    pub under: f64,
    pub hp: f64,
}
