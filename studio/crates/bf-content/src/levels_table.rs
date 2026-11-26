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
    ("VIOLIN", 1.6314, 0.000, 0.260),
    ("BAGPIPES", 3.3933, 0.000, 0.423),
    ("ACCORDION", 3.7100, 0.114, 0.075),
    ("TUBA", 2.2903, 0.967, 0.000),
    ("FLUTE", 3.0963, 0.000, 0.020),
    ("TROMBONE", 3.1920, 0.483, 0.006),
    ("HORN", 3.9523, 0.000, 0.009),
    ("ORGAN", 3.4603, 0.195, 0.101),
    ("HARP", 2.0133, 0.213, 0.023),
    ("UKULELE", 2.2333, 0.000, 0.098),
    ("ACOUSTIC", 1.3329, 0.017, 0.205),
    ("TRUMPET", 3.3516, 0.000, 0.145),
    ("TURNTABLES", 2.6692, 0.050, 0.039),
    ("SYNTH", 2.7679, 0.002, 0.076),
    ("CLARINET", 3.1232, 0.255, 0.006),
    ("OBOE", 2.6913, 0.000, 0.115),
    ("BASS CLARINET", 5.4753, 0.661, 0.002),
    ("EUPHONIUM", 2.1166, 0.000, 0.011),
    ("BASSOON", 3.6607, 0.000, 0.011),
    ("BARI SAX", 2.0647, 0.164, 0.012),
    ("VIOLA", 1.1932, 0.003, 0.006),
    ("ELECTRIC", 1.2154, 0.001, 0.169),
    ("CELLO", 1.8074, 0.046, 0.030),
    ("CONTRABASS", 2.4747, 0.903, 0.001),
    ("ALTO SAX", 1.9587, 0.001, 0.044),
    ("TENOR SAX", 2.8468, 0.122, 0.010),
    ("SOUSAPHONE", 2.3009, 0.913, 0.001),
    ("BASS DRUM", 1.4415, 0.990, 0.000),
    ("CORNET", 4.4308, 0.000, 0.005),
    ("CONTRABASS CLARINET", 2.9584, 0.670, 0.006),
    ("MARIMBA", 0.9232, 0.000, 0.007),
    ("PICCOLO", 1.3318, 0.000, 0.572),
    ("SOPRANO SAX", 6.2637, 0.000, 0.102),
    ("MELLOPHONE", 7.7454, 0.000, 0.029),
    ("TIMPANI", 2.1355, 0.784, 0.000),
    ("CONTRABASSOON", 3.8258, 0.009, 0.013),
    ("FLUGELHORN", 5.2615, 0.034, 0.004),
    ("SNARE DRUM", 4.7019, 0.229, 0.347),
    ("OCTOBASS", 7.5143, 0.904, 0.000),
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
