//! The saved level of every fighter under the Composed arrangement (sound upgrade, M1), and what its sound is made
//! of. Written by `ballfight levels --write`, never by hand: each fighter is rendered against the same reference
//! fight and measured ([`bf_sound::heard`]).
//!
//! - `gain`: the bus gain that puts the fighter 5 dB over the reference band as small speakers give it.
//! - `rumble`: the part of its energy under 200 Hz, before any treatment. Over 0.5 it is a low instrument: its
//!   lows are made audible on a phone (M2), it sends little to the room, and as the bass it takes the band's bass
//!   layers' place (C6).
//! - `piercing`: the part of its energy between 2 and 6 kHz. Over 0.25 its highs are tamed (M3).

/// A fighter's saved level and make-up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Saved {
    pub gain: f64,
    pub rumble: f64,
    pub piercing: f64,
}

impl Saved {
    /// A low instrument (M2, C6).
    pub fn low(&self) -> bool {
        self.rumble > 0.5
    }

    /// A piercing instrument (M3).
    pub fn piercing(&self) -> bool {
        self.piercing > 0.25
    }
}

/// The saved level of fighter `name`, if it has one (a fighter added since the table was written has none, and
/// plays at its part's own bus gain until the table is written again).
pub fn of(name: &str) -> Option<Saved> {
    LEVELS
        .iter()
        .find(|(n, ..)| *n == name)
        .map(|&(_, gain, rumble, piercing)| Saved { gain, rumble, piercing })
}

/// `(fighter, gain, rumble, piercing)`, in roster order.
#[rustfmt::skip]
pub const LEVELS: &[(&str, f64, f64, f64)] = &[
];
