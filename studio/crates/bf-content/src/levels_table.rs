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
    ("PIANO", 1.8564, 0.031, 0.054),
    ("DRUMS", 3.8737, 0.677, 0.020),
    ("VIOLIN", 1.9553, 0.000, 0.260),
    ("BAGPIPES", 3.6913, 0.000, 0.661),
    ("ACCORDION", 3.7100, 0.114, 0.075),
    ("TUBA", 2.1296, 0.703, 0.000),
    ("FLUTE", 3.3003, 0.000, 0.029),
    ("TROMBONE", 2.1486, 0.009, 0.027),
    ("HORN", 4.1033, 0.000, 0.004),
    ("ORGAN", 3.4603, 0.195, 0.101),
    ("HARP", 2.0133, 0.213, 0.023),
    ("UKULELE", 2.2333, 0.000, 0.098),
    ("ACOUSTIC", 1.3329, 0.017, 0.205),
    ("TRUMPET", 4.3689, 0.000, 0.361),
    ("TURNTABLES", 2.6692, 0.050, 0.039),
    ("SYNTH", 2.7679, 0.002, 0.076),
    ("CLARINET", 2.1785, 0.100, 0.145),
    ("OBOE", 2.7636, 0.000, 0.055),
    ("BASS CLARINET", 3.6299, 0.661, 0.011),
    ("EUPHONIUM", 2.3207, 0.000, 0.000),
    ("BASSOON", 3.6607, 0.000, 0.011),
    ("BARI SAX", 2.0331, 0.124, 0.011),
    ("VIOLA", 1.3618, 0.004, 0.097),
    ("ELECTRIC", 1.2154, 0.001, 0.169),
    ("CELLO", 1.6055, 0.047, 0.188),
    ("CONTRABASS", 2.5547, 0.903, 0.001),
    ("ALTO SAX", 1.8828, 0.001, 0.079),
    ("TENOR SAX", 2.7085, 0.045, 0.020),
    ("SOUSAPHONE", 2.3547, 0.754, 0.000),
    ("BASS DRUM", 1.4933, 0.990, 0.000),
    ("CORNET", 5.7044, 0.000, 0.104),
    ("CONTRABASS CLARINET", 1.9018, 0.670, 0.013),
    ("MARIMBA", 0.9232, 0.000, 0.007),
    ("PICCOLO", 1.5024, 0.000, 0.572),
    ("SOPRANO SAX", 5.8615, 0.000, 0.243),
    ("MELLOPHONE", 8.0429, 0.000, 0.032),
    ("TIMPANI", 2.1958, 0.771, 0.000),
    ("CONTRABASSOON", 3.8258, 0.009, 0.013),
    ("FLUGELHORN", 3.8961, 0.007, 0.159),
    ("SNARE DRUM", 4.7019, 0.229, 0.347),
    ("OCTOBASS", 7.8159, 0.904, 0.000),
    ("BASS GUITAR", 4.2046, 0.128, 0.092),
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
