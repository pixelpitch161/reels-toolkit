//! The saved level of every fighter under the Composed arrangement (sound upgrade, M1), and what its sound is made
//! of. Written by `ballfight levels --write`, never by hand: each fighter is rendered against the same reference
//! fight and measured ([`bf_sound::heard`]).
//!
//! - `gain`: the bus gain that puts the fighter [`bf_sound::polish::OVER_BAND`] dB over the reference band as small
//!   speakers give it.
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

/// The line the table starts after and the one it ends before, for `ballfight levels --write`.
pub const TABLE_OPENS: &str = "pub const LEVELS: &[(&str, f64, f64, f64)] = &[";
pub const TABLE_CLOSES: &str = "];";

/// A row of the table as the source holds it.
pub fn row(name: &str, saved: Saved) -> String {
    format!(
        "    ({name:?}, {:.4}, {:.3}, {:.3}),",
        saved.gain, saved.rumble, saved.piercing
    )
}

/// `(fighter, gain, rumble, piercing)`, in roster order.
#[rustfmt::skip]
pub const LEVELS: &[(&str, f64, f64, f64)] = &[
    ("PIANO", 1.8483, 0.021, 0.010),
    ("DRUMS", 4.1099, 0.677, 0.019),
    ("VIOLIN", 1.9553, 0.000, 0.260),
    ("BAGPIPES", 3.6904, 0.000, 0.660),
    ("ACCORDION", 3.6028, 0.061, 0.049),
    ("TUBA", 2.1296, 0.703, 0.000),
    ("FLUTE", 3.3003, 0.000, 0.029),
    ("TROMBONE", 2.1486, 0.009, 0.027),
    ("HORN", 4.1022, 0.000, 0.004),
    ("ORGAN", 2.6980, 0.092, 0.127),
    ("HARP", 1.9522, 0.003, 0.052),
    ("UKULELE", 2.5630, 0.000, 0.134),
    ("ACOUSTIC", 1.6875, 0.100, 0.076),
    ("TRUMPET", 4.4231, 0.000, 0.393),
    ("TURNTABLES", 2.6692, 0.050, 0.039),
    ("SYNTH", 2.8853, 0.002, 0.066),
    ("CLARINET", 1.9890, 0.061, 0.224),
    ("OBOE", 2.7636, 0.000, 0.055),
    ("BASS CLARINET", 3.2819, 0.661, 0.032),
    ("EUPHONIUM", 2.3382, 0.000, 0.000),
    ("BASSOON", 5.0031, 0.000, 0.002),
    ("BARI SAX", 2.0331, 0.124, 0.011),
    ("VIOLA", 1.3618, 0.004, 0.097),
    ("ELECTRIC", 2.2111, 0.003, 0.448),
    ("CELLO", 1.6055, 0.047, 0.188),
    ("CONTRABASS", 2.5547, 0.903, 0.001),
    ("ALTO SAX", 1.8828, 0.001, 0.079),
    ("TENOR SAX", 2.7085, 0.045, 0.020),
    ("SOUSAPHONE", 2.3547, 0.754, 0.000),
    ("BASS DRUM", 1.4933, 0.990, 0.000),
    ("CORNET", 5.7117, 0.000, 0.116),
    ("CONTRABASS CLARINET", 1.7196, 0.670, 0.011),
    ("MARIMBA", 0.9530, 0.000, 0.052),
    ("PICCOLO", 1.5024, 0.000, 0.572),
    ("SOPRANO SAX", 5.8615, 0.000, 0.243),
    ("MELLOPHONE", 8.0429, 0.000, 0.032),
    ("TIMPANI", 2.1958, 0.771, 0.000),
    ("CONTRABASSOON", 4.1806, 0.008, 0.003),
    ("FLUGELHORN", 3.8765, 0.007, 0.174),
    ("SNARE DRUM", 5.0797, 0.241, 0.353),
    ("OCTOBASS", 7.8159, 0.904, 0.000),
    ("BASS GUITAR", 6.9373, 0.360, 0.021),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Every row is a roster fighter, once, with a gain above zero and shares that are shares.
    #[test]
    fn the_table_holds_roster_fighters() {
        for (i, &(name, gain, rumble, piercing)) in LEVELS.iter().enumerate() {
            assert!(
                crate::roster().iter().any(|f| f.name == name),
                "{name} is not in the roster"
            );
            assert!(LEVELS[..i].iter().all(|(n, ..)| *n != name), "{name} is listed twice");
            assert!(gain > 0.0 && gain.is_finite(), "{name}: gain {gain}");
            assert!(
                (0.0..=1.0).contains(&rumble) && (0.0..=1.0).contains(&piercing),
                "{name}"
            );
        }
        assert_eq!(of("no such fighter"), None);
        let saved = Saved {
            gain: 1.5,
            rumble: 0.61234,
            piercing: 0.0,
        };
        assert_eq!(row("TUBA", saved), "    (\"TUBA\", 1.5000, 0.612, 0.000),");
        assert!(saved.low() && !saved.piercing());
    }
}
