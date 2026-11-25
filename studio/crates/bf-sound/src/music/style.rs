//! The melody styles of the Composed arrangement (sound upgrade, C11) and what a Composed build carries beside its
//! lines ([`Polish`]): the style, and how each fighter's notes are treated when they are placed.

use std::collections::BTreeMap;

/// A way of writing the fighters' melodies: every fight on the Composed arrangement plays in one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Style {
    /// The fighter's own motif, its answer, the echo and a run into the cadence.
    Signature,
    /// A short rising figure that starts higher each bar.
    Climbing,
    /// One close figure three times on the changing chords, then a changed ending.
    Riff,
    /// Up the scale and back on every 8th.
    Runs,
    /// Up and down the notes of the bar's chord.
    BrokenChords,
    /// One note keeps returning while a second voice moves against it.
    Anchor,
}

impl Style {
    /// Every style, in the order the app lists them.
    pub const ALL: [Style; 6] = [
        Style::Signature,
        Style::Climbing,
        Style::Riff,
        Style::Runs,
        Style::BrokenChords,
        Style::Anchor,
    ];

    /// The word a fight file holds for the style (`music.melody_style`).
    pub fn key(self) -> &'static str {
        match self {
            Style::Signature => "signature",
            Style::Climbing => "climbing",
            Style::Riff => "riff",
            Style::Runs => "runs",
            Style::BrokenChords => "broken_chords",
            Style::Anchor => "anchor",
        }
    }

    /// The style a fight file's word names.
    pub fn from_key(key: &str) -> Option<Style> {
        Style::ALL.into_iter().find(|s| s.key() == key)
    }

    /// The name the app shows.
    pub fn title(self) -> &'static str {
        match self {
            Style::Signature => "Signature",
            Style::Climbing => "Climbing",
            Style::Riff => "Riff",
            Style::Runs => "Runs",
            Style::BrokenChords => "Broken chords",
            Style::Anchor => "Anchor note",
        }
    }

    /// Whether the style never plays a note twice in a row (the others play their line as written).
    pub fn moving(self) -> bool {
        matches!(self, Style::Climbing | Style::Runs | Style::BrokenChords)
    }
}

/// How one fighter's notes are treated under the Composed arrangement.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PartNote {
    /// The instrument sounds one note at a time (wind, brass, bowed strings): a new note ends the one before it.
    pub one_voice: bool,
    /// Most of its energy is under 200 Hz (tuba, contrabass): its lows are made audible on small speakers.
    pub low: bool,
    /// Much of its energy lies between 2 and 6 kHz (piccolo, bagpipes): its top notes are softened.
    pub piercing: bool,
    /// The lowest and highest note of its written line.
    pub register: (i32, i32),
}

/// What a Composed build carries beside the fighters' lines. A Classic build has none
/// ([`super::MusicCtx::polish`] is `None`), and every part then plays as the kit's does.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Polish {
    /// The melody style of the fight.
    pub style: Option<Style>,
    /// The two fighters, by name.
    pub parts: BTreeMap<String, PartNote>,
    /// The fighter with the bass role is a true bass instrument: the band's own bass layers stay silent from
    /// FIGHT! to the K.O. and leave the low end to it.
    pub hand_bass: bool,
}

impl Polish {
    /// The style, Signature where none is set.
    pub fn style(&self) -> Style {
        self.style.unwrap_or(Style::Signature)
    }
}

/// The level of an ordinary note of a piercing instrument (M3): 1 up to C6 (MIDI 84), then 1.5 dB less for every
/// semitone above it, 9 dB at most.
pub fn soften(midi: i32) -> f64 {
    let over = f64::from((midi - 84).clamp(0, 6));
    bf_core::dmath::pow(10.0, -1.5 * over / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_style_reads_back_from_its_word() {
        for s in Style::ALL {
            assert_eq!(Style::from_key(s.key()), Some(s));
            assert!(!s.title().is_empty());
        }
        assert_eq!(Style::from_key("polka"), None);
        let moving: Vec<Style> = Style::ALL.into_iter().filter(|s| s.moving()).collect();
        assert_eq!(moving, [Style::Climbing, Style::Runs, Style::BrokenChords]);
        assert_eq!(Polish::default().style(), Style::Signature);
    }

    #[test]
    fn notes_above_c6_are_softer() {
        assert_eq!(soften(60), 1.0);
        assert_eq!(soften(84), 1.0);
        assert!((soften(86) - 0.7079).abs() < 1e-3);
        assert!((soften(90) - 0.3548).abs() < 1e-3);
        assert_eq!(soften(100), soften(90));
    }
}
