//! How each fighter's notes are treated under the Composed arrangement (sound upgrade, 2026-11-25): what the build
//! hands the parts through `MusicCtx::polish`.

use bf_sound::music::{PartNote, Polish, Style};

use crate::levels_table;
use crate::personality::personality;

/// The instruments that sound one note at a time: wind, brass and bowed strings (M5). A new ordinary note of
/// theirs ends the one before it; a piano's, a harp's or a marimba's notes ring through each other as they do on
/// the instrument.
pub const ONE_VOICE: [&str; 27] = [
    "ALTO SAX",
    "BAGPIPES",
    "BARI SAX",
    "BASS CLARINET",
    "BASSOON",
    "CELLO",
    "CLARINET",
    "CONTRABASS",
    "CONTRABASS CLARINET",
    "CONTRABASSOON",
    "CORNET",
    "EUPHONIUM",
    "FLUGELHORN",
    "FLUTE",
    "HORN",
    "MELLOPHONE",
    "OBOE",
    "OCTOBASS",
    "PICCOLO",
    "SOPRANO SAX",
    "SOUSAPHONE",
    "TENOR SAX",
    "TROMBONE",
    "TRUMPET",
    "TUBA",
    "VIOLA",
    "VIOLIN",
];

/// The melody style a fight file asks for: its `music.melody_style`, Signature for a file without one or with a
/// word this app does not know.
pub fn style_of(melody_style: Option<&str>) -> Style {
    melody_style.and_then(Style::from_key).unwrap_or(Style::Signature)
}

/// How fighter `name`'s notes are treated.
pub fn part_note(name: &str) -> PartNote {
    let saved = levels_table::of(name);
    PartNote {
        one_voice: ONE_VOICE.contains(&name),
        low: saved.is_some_and(|s| s.low()),
        piercing: saved.is_some_and(|s| s.piercing()),
        register: personality(name).map_or((0, 127), |p| {
            (i32::from(p.register.0), i32::from(p.register.1.max(p.register.0)))
        }),
    }
}

/// What a Composed build of `names` carries: the style, each fighter's treatment, and whether the fighter with
/// the bass role (`bass`, if any) takes the band's bass layers' place (C6: a true bass instrument does).
pub fn polish(names: &[String; 2], style: Style, bass: Option<&str>) -> Polish {
    Polish {
        style: Some(style),
        parts: names.iter().map(|n| (n.clone(), part_note(n))).collect(),
        hand_bass: bass.and_then(levels_table::of).is_some_and(|s| s.low()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roster;

    #[test]
    fn every_one_voice_fighter_is_in_the_roster() {
        for name in ONE_VOICE {
            assert!(roster().iter().any(|f| f.name == name), "{name}");
        }
        assert_eq!(style_of(None), Style::Signature);
        assert_eq!(style_of(Some("runs")), Style::Runs);
        assert_eq!(style_of(Some("polka")), Style::Signature);
        let p = part_note("FLUTE");
        assert!(p.one_voice && p.register == (71, 93));
        assert!(!part_note("PIANO").one_voice);
        // a name that is no fighter: plain treatment, the whole keyboard
        assert_eq!(
            part_note("KAZOO"),
            PartNote {
                register: (0, 127),
                ..PartNote::default()
            }
        );
    }
}
