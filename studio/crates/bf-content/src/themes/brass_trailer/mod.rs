//! Theme: `brass_trailer` (`projects/ballfight/kit/moods/brass_trailer.py`). Being ported (milestone 3, Template T):
//! the voicings are here; the card, the option `sequel` and the classic bed (`classic.rs`) follow.

mod classic;

use bf_core::harmony::{Harmony, HarmonyError};

use super::{NOT_PORTED_CARD, Theme};

/// The theme as the registry lists it.
pub static THEME: Theme = Theme {
    name: "brass_trailer",
    ported: false,
    card: &NOT_PORTED_CARD,
    options: &[],
    voices: &[],
    own_voice: super::no_own_voice,
    classic: classic::classic,
};

/// `Bed.trem(ch)`: the high tremolo's voicing, `tuple(T.chord_tones(73, 84, ch)[:3])`.
///
/// # Errors
/// Where Python raises: `chord` is not one of the episode's chords.
pub fn trem(harmony: &Harmony, chord: &str) -> Result<Vec<i32>, HarmonyError> {
    let mut t = harmony.chord_tones(73, 84, chord)?;
    t.truncate(3);
    Ok(t)
}

/// `Bed.pad_notes(ch)`: the sequel's cello pad, `tuple(T.chord_tones(45, 57, ch)[:3])`.
///
/// # Errors
/// Where Python raises: `chord` is not one of the episode's chords.
pub fn pad_notes(harmony: &Harmony, chord: &str) -> Result<Vec<i32>, HarmonyError> {
    let mut t = harmony.chord_tones(45, 57, chord)?;
    t.truncate(3);
    Ok(t)
}
