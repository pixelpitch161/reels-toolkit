//! The musical personality of each fighter (milestone 5, spec section 8): what the composer needs to write a line
//! and a victory solo that sound like that fighter. Each fighter's own lives in `fighters/<slug>/personality.rs`;
//! [`personality`] looks one up by the fighter's roster name.

/// The touch a fighter puts on its notes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ornament {
    /// A quick note just before the main one.
    Grace,
    /// A bend or glide into the note.
    Slide,
    /// A fast wobble with the note above.
    Trill,
    /// A short, hard accent.
    Stab,
    /// Plain notes.
    None,
}

/// A part a fighter can play in the fight's music.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The low line under everything.
    Bass,
    /// The top tune.
    Melody,
    /// Short replies between the other fighter's phrases.
    Answer,
}

/// One note of a motif: a scale degree from the home key (0 is the home note, 7 the octave above, negative is
/// below) and its length in sixteenths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub deg: i8,
    pub len: u8,
}

/// A fighter's musical personality.
#[derive(Debug)]
pub struct Personality {
    /// The home key's note (MIDI), where scale degree 0 sits.
    pub home: u8,
    /// The home scale, as semitones above the home note.
    pub scale: [u8; 7],
    /// The lowest and highest MIDI note it plays.
    pub register: (u8, u8),
    /// A few one-bar rhythm cells: the sixteenths (0 to 15) a note starts on.
    pub rhythms: &'static [&'static [u8]],
    /// Its favourite leaps, in semitones, most used first.
    pub intervals: &'static [u8],
    /// Its ornament.
    pub ornament: Ornament,
    /// The roles it can take.
    pub roles: &'static [Role],
    /// 2 to 4 signature motifs, each from one of its episodes.
    pub motifs: &'static [&'static [Step]],
}

impl Personality {
    /// The MIDI note of scale degree `deg` in the home key.
    pub fn note(&self, deg: i8) -> i32 {
        let (oct, i) = (i32::from(deg).div_euclid(7), i32::from(deg).rem_euclid(7));
        i32::from(self.home) + 12 * oct + i32::from(self.scale[i as usize])
    }
}

macro_rules! fighters {
    ($($name:literal => $m:ident),* $(,)?) => {
        /// The personality of the fighter `fighter` (its roster name, such as `BASS CLARINET`), or `None` for a
        /// name that is not one of the thirty.
        pub fn personality(fighter: &str) -> Option<&'static Personality> {
            Some(match fighter {
                $($name => &crate::fighters::$m::personality::PERSONALITY,)*
                _ => return None,
            })
        }
    };
}

fighters! {
    "ACOUSTIC" => acoustic, "ALTO SAX" => alto_sax, "BASS CLARINET" => bass_clarinet, "BASS DRUM" => bass_drum,
    "BASSOON" => bassoon, "CELLO" => cello, "CLARINET" => clarinet, "CONTRABASS" => contrabass,
    "ELECTRIC" => electric, "EUPHONIUM" => euphonium, "FLUTE" => flute, "HORN" => horn, "OBOE" => oboe,
    "ORGAN" => organ, "PIANO" => piano, "SOUSAPHONE" => sousaphone, "TENOR SAX" => tenor_sax,
    "TROMBONE" => trombone, "TRUMPET" => trumpet, "TUBA" => tuba, "ACCORDION" => accordion,
    "BAGPIPES" => bagpipes, "BARI SAX" => bari_sax, "DRUMS" => drums, "HARP" => harp, "SYNTH" => synth,
    "TURNTABLES" => turntables, "UKULELE" => ukulele, "VIOLA" => viola, "VIOLIN" => violin,
    "CONTRABASSOON" => contrabassoon,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fighters::{OLDER, entry};
    use crate::roster;

    #[test]
    fn all_thirty_have_a_playable_personality() {
        let mut names: Vec<String> = roster().iter().map(|f| f.name.clone()).collect();
        names.extend(OLDER.iter().map(|s| s.to_string()));
        names.retain(|n| entry(n).is_some());
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 31);
        for n in &names {
            let p = personality(n).unwrap_or_else(|| panic!("{n}"));
            assert!((2..=4).contains(&p.motifs.len()), "{n}");
            assert!(!p.roles.is_empty(), "{n}");
            assert!(
                !p.rhythms.is_empty() && p.rhythms.iter().all(|c| c.iter().all(|&x| x < 16)),
                "{n}"
            );
            let (lo, hi) = (i32::from(p.register.0), i32::from(p.register.1));
            for m in p.motifs {
                assert!(!m.is_empty(), "{n}");
                for st in *m {
                    let note = p.note(st.deg);
                    assert!(
                        (lo..=hi).contains(&note),
                        "{n}: degree {} is {note}, outside {lo}..={hi}",
                        st.deg
                    );
                }
            }
        }
        assert!(personality("GUITAR").is_none() && personality("SAX").is_none());
    }
}
