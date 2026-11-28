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
    ("VIOLIN", 1.2228, 0.001, 0.264),
    ("BAGPIPES", 2.6176, 0.000, 0.660),
    ("ACCORDION", 3.6649, 0.054, 0.098),
    ("TUBA", 2.1296, 0.703, 0.000),
    ("FLUTE", 3.3003, 0.000, 0.029),
    ("TROMBONE", 2.1486, 0.009, 0.027),
    ("HORN", 4.1022, 0.000, 0.004),
    ("ORGAN", 2.6980, 0.092, 0.127),
    ("HARP", 1.6227, 0.019, 0.008),
    ("UKULELE", 1.8992, 0.001, 0.219),
    ("ACOUSTIC", 3.2540, 0.479, 0.063),
    ("TRUMPET", 4.4231, 0.000, 0.393),
    ("TURNTABLES", 2.6692, 0.050, 0.039),
    ("SYNTH", 2.8853, 0.002, 0.066),
    ("CLARINET", 2.3633, 0.075, 0.016),
    ("OBOE", 2.9460, 0.000, 0.018),
    ("BASS CLARINET", 2.8101, 0.661, 0.006),
    ("EUPHONIUM", 2.3382, 0.000, 0.000),
    ("BASSOON", 3.7328, 0.001, 0.001),
    ("BARI SAX", 2.1403, 0.164, 0.005),
    ("VIOLA", 1.8749, 0.001, 0.062),
    ("ELECTRIC", 1.8690, 0.003, 0.209),
    ("CELLO", 1.6413, 0.115, 0.034),
    ("CONTRABASS", 1.7978, 0.821, 0.001),
    ("ALTO SAX", 2.2483, 0.000, 0.034),
    ("TENOR SAX", 2.2731, 0.018, 0.016),
    ("SOUSAPHONE", 2.3547, 0.754, 0.000),
    ("BASS DRUM", 1.4933, 0.990, 0.000),
    ("CORNET", 5.7117, 0.000, 0.116),
    ("CONTRABASS CLARINET", 2.2473, 0.670, 0.000),
    ("MARIMBA", 0.9530, 0.000, 0.052),
    ("PICCOLO", 1.5024, 0.000, 0.572),
    ("SOPRANO SAX", 7.3537, 0.000, 0.095),
    ("MELLOPHONE", 8.0429, 0.000, 0.032),
    ("TIMPANI", 2.1958, 0.771, 0.000),
    ("CONTRABASSOON", 1.6637, 0.005, 0.001),
    ("FLUGELHORN", 3.8765, 0.007, 0.174),
    ("SNARE DRUM", 5.0797, 0.241, 0.353),
    ("OCTOBASS", 6.4414, 0.928, 0.000),
    ("BASS GUITAR", 6.5038, 0.549, 0.003),
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
