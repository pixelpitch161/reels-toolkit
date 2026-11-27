//! How a note is played: everything a player does with one note besides choosing its pitch and length. Today's parts
//! play plain notes ([`Play::plain`]); the composition system can ask for any of the rest (see
//! `docs/studio/voice-engine-2-playing.md`). An instrument takes what it can do and leaves the rest: a piano has no
//! vibrato, an organ no accent on its pipes' loudness. Every instrument can play every one of its techniques
//! (Aedan's rule): what is a single note's business is asked for here; what is notes in a row (trills, runs,
//! repeated notes, glissandos, clusters) is the composition system's.

/// One note as played.
#[derive(Clone, Debug, PartialEq)]
pub struct Play {
    /// The pitch, MIDI (a fraction is a pitch between keys).
    pub midi: f64,
    /// How long the key is held, seconds; the note rings on after it as the instrument does.
    pub dur: f64,
    /// How hard, 0 to 1 (as the kit's `vel`).
    pub vel: f64,
    /// An accent on top of `vel`, 0 (none) to 1 (as hard as it goes): a harder, noisier start.
    pub accent: f64,
    /// A slur from the note before, at this MIDI pitch: the note joins it without a new start, where the instrument
    /// can (a wind, a bowed string). On a plucked string the string keeps ringing without a new pluck and slides from
    /// that pitch to this one over 60 ms, or with [`Technique::Hammer`] changes at once: a slide, a hammer-on, a
    /// pull-off. A bend adds to it: a bend let down is the bent pitch as both the note and `slur_from`, with a bend
    /// down from it.
    pub slur_from: Option<f64>,
    /// A slide of the pitch: a bend, a fall (down), a doit (up). With `slur_from` and 0 cents, a slur that glides
    /// over the bend's `over` (a portamento, a wind's smear, a trombone's slide).
    pub bend: Option<Bend>,
    /// A scoop into the note: it starts the bend's `cents` off its pitch (under it when negative), stays there `at`
    /// seconds and reaches its pitch over `over` (a wind, a voice; a bend after it is a fall or a doit).
    pub scoop: Option<Bend>,
    /// A swell of the strength over the note: an organ's stops coming in, a wind's crescendo (a sforzando: an accent
    /// with a swell from 1 down).
    pub swell: Option<Swell>,
    /// A vibrato. On a bowed string or a wind, none asked is the player's own (it comes in after the start, if the
    /// instrument's players use one) and one of 0 cents none at all.
    pub vibrato: Option<Vibrato>,
    /// The pedals a piano (or a vibraphone) has.
    pub pedals: Pedals,
    /// A mute or a short stop.
    pub mute: Mute,
    /// How many notes sound together with this one, itself included (an organ's wind sags a hair under a big chord).
    pub together: u8,
    /// A special way of sounding the note.
    pub technique: Technique,
    /// An organ's registration: which stops sound (none asked: the instrument's own, the organ's grand chorus).
    pub stops: Option<Stops>,
    /// An accordion's register: which banks of reeds sound (none asked: the instrument's own).
    pub register: Option<Register>,
    /// Where along a plucked string it is plucked, as a fraction of the sounding string from the bridge (0.5 the
    /// middle): near the bridge is bright and thin (sul ponticello, a harp's près de la table), over the neck round
    /// and soft (sul tasto). None: where the player usually plucks. On a bowed string, where the bow sits, from the
    /// bridge (0.02 to 0.3): by the bridge glassy and thin, over the fingerboard soft and sweet.
    pub position: Option<f64>,
    /// Which string a fretted note is played on, counted as a player numbers them from the chin down, from 0 (a
    /// guitar's low E, a ukulele's high G); none, or a string tuned over the note: the lowest position that has it.
    /// The same pitch higher up a lower string is rounder. On a bowed string counted from the lowest (0 the violin's
    /// G, the cello's C); none, or a string tuned over the note: the highest string that has it. On brass, the
    /// fingering: how many semitones the valves (or the trombone's slide, 0 its first position) lower the horn, plus
    /// 100 for a double horn's other side; none, or one that cannot reach the note: the player's usual fingering.
    pub string: Option<u8>,
    /// A volume pedal over the note, 0 (shut) to 1 (open), with no new start: an organ's swell box, quieter and
    /// darker shut; on a bowed string, the bow's speed and weight over the note; on brass with
    /// [`Mute::Plunger`], how far the plunger is off the bell (none: closed).
    pub expression: Option<Swell>,
    /// The next note is slurred on from this one: the bow keeps moving (a wind's breath keeps flowing) and the note
    /// hands over to the next instead of stopping (which comes with `slur_from` set to this note's pitch).
    pub tie: bool,
}

/// A piano's three pedals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pedals {
    /// The sustain (damper) pedal, 0 (up) to 1 (down): down, the dampers leave the strings, the note rings on past
    /// the key and the other strings ring along; part way (a half pedal) the dampers only touch the strings.
    pub sustain: f64,
    /// The sustain and middle pedals come up this many seconds into the note (none: held down as long as it rings).
    pub lift: Option<f64>,
    /// The soft pedal (una corda): the hammers shift, striking one string fewer with softer felt.
    pub soft: bool,
    /// The middle pedal (sostenuto): this note's dampers stay up, the other strings stay damped.
    pub sostenuto: bool,
}

impl Pedals {
    /// No pedal down.
    pub const UP: Pedals = Pedals {
        sustain: 0.0,
        lift: None,
        soft: false,
        sostenuto: false,
    };
    /// The sustain pedal all the way down, held.
    pub const SUSTAIN: Pedals = Pedals {
        sustain: 1.0,
        ..Pedals::UP
    };
}

/// A special way of sounding a note. Later families add their own (pizzicato, flutter tongue, rimshot...).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Technique {
    /// As the instrument is usually played (a plucked string: the electric and the bass with a pick, the rest with
    /// the fingers).
    Normal,
    /// A harmonic: the string touched at 1/n of its length (2 to 6) as it is played, so only every n-th overtone
    /// rings on (2 sounds an octave up, 3 an octave and a fifth). On a wind, the note played on the n-th resonance of
    /// its tube (from 1): a flute's harmonic (a lower note's fingering overblown, hollow), a reed's register (a
    /// clarinet's 1 chalumeau, 3 clarion, 5 altissimo), a brass player's natural note n (with `string` its valves).
    Harmonic(u8),
    /// Plucked: a piano's string pulled aside by a finger inside the piano and let go.
    Plucked,
    /// A plucked string played with the flesh of the finger: round and warm.
    Fingers,
    /// A plucked string played with a pick: a sharp, bright start.
    Picked,
    /// A plucked string played with the fingernails: bright and crisp.
    Nails,
    /// A plucked string played with the side of the thumb: deep and soft.
    Thumb,
    /// A bass string struck by the thumb against the last frets: a bright knock with the frets' buzz. On a reed, a
    /// slap tongue: the tongue pulls the reed onto the mouthpiece and lets go with a pop.
    Slap,
    /// A bass string pulled up and let snap against the frets: a sharp, metallic crack.
    Pop,
    /// A string sounded by a fingertip hammered onto the fret, with no pluck.
    Tapped,
    /// A hammer-on or pull-off: the note before keeps ringing into this one, a fretting finger changing the pitch
    /// (with `slur_from`; without it, as [`Technique::Tapped`]).
    Hammer,
    /// A pinch harmonic: the pick and the thumb's edge together, so a high overtone squeals out (an electric's).
    PinchHarmonic,
    /// Bowed (arco), whatever the note's length: a contrabass's plain short notes are plucked otherwise.
    Arco,
    /// Pizzicato: a bowed string plucked by the finger.
    Pizzicato,
    /// A snap pizzicato (Bartók's): the string pulled up and let slap back onto the fingerboard.
    SnapPizz,
    /// A left-hand pizzicato: plucked by a finger of the hand that stops the strings, lighter and thinner.
    LeftHandPizz,
    /// Spiccato: the bow dropped onto the string and bouncing off, a short, light, bouncing note.
    Spiccato,
    /// Martelé: a bitten start, the bow stopped on the string before the next note.
    Martele,
    /// Tremolo: the bow moved back and forth as fast as it goes, all through the note. On an accordion, the
    /// bellows shake: the bellows pushed and pulled in quick turns.
    Tremolo,
    /// Col legno battuto: the string struck with the bow's wooden stick, a dry click with a short pitch.
    ColLegno,
    /// Col legno tratto: the stick drawn across the string, a thin, whispering tone.
    ColLegnoTratto,
    /// A wind's note started by the breath alone, with no tongue: a soft, breathy start (a flute's "hoo").
    Breath,
    /// A wind's note started by the back of the tongue, the "k" of double and triple tonguing ("tu-ku", "tu-tu-ku"):
    /// a little softer than the tip's "t".
    BackTongue,
    /// A soft legato tongue ("d"): the breath keeps flowing and the tongue just touches the reed or the lips.
    SoftTongue,
    /// Flutter tongue: the tongue rolling an "r" through the note, a fast buzzing flutter.
    Flutter,
    /// A growl: the player hums or growls in the throat while playing, a rough buzz on the tone.
    Growl,
    /// A saxophone's subtone: the lip damps the reed, a soft, breathy, dark tone (in the low notes most of all).
    Subtone,
    /// An air tone: breath through the instrument with the reed (the lips, the jet) hardly sounding, a pitched
    /// rushing air.
    AirTone,
    /// Key clicks: the keys (pads) slapped shut with no breath, a small pitched pop of the tube.
    KeyClick,
    /// A brass lip shake: the note shaken fast to the natural note above and back.
    Shake,
    /// Half valve: a valve pressed half way down, a choked, nasal, wobbly tone.
    HalfValve,
    /// A flute's whistle tone: hardly any breath over the hole, a very soft, pure, high whistle.
    WhistleTone,
    /// A flute's jet whistle: a hard burst of breath straight into the hole, a rushing sweep.
    JetWhistle,
    /// Sung while played: the player hums a note this many semitones from the played one into the instrument (a
    /// flute's "singing": the voice and the tone beat against each other).
    Sung(i8),
}

/// An organ's registration: which stops sound, as bits of [`Stops::MANUAL`] and [`Stops::PEDAL`], and its octave
/// couplers (the manual's notes also an octave under, or over). Notes under G2 are played on the pedal board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stops {
    pub manual: u16,
    pub pedal: u8,
    /// The sub-octave coupler: every manual note also an octave under.
    pub sub: bool,
    /// The octave (super) coupler: every manual note also an octave over.
    pub octave: bool,
}

impl Stops {
    /// The manual's stops, bit 0 first.
    pub const MANUAL: [&'static str; 14] = [
        "Praestant 16'",
        "Bourdon 16'",
        "Principal 8'",
        "Gedackt 8'",
        "Salicional 8'",
        "Voix celeste 8'",
        "Octave 4'",
        "Rohrflute 4'",
        "Quint 2 2/3'",
        "Superoctave 2'",
        "Mixture IV",
        "Cymbal III",
        "Trumpet 8'",
        "Oboe 8'",
    ];
    /// The pedal board's stops, bit 0 first.
    pub const PEDAL: [&'static str; 4] = ["Subbass 16'", "Octavebass 8'", "Choralbass 4'", "Posaune 16'"];

    /// The stops named, manual and pedal (a name of neither is left out).
    pub fn of(manual: &[&str], pedal: &[&str]) -> Stops {
        let bits = |names: &[&str], all: &[&str]| {
            names
                .iter()
                .filter_map(|n| all.iter().position(|a| a == n))
                .fold(0u16, |b, i| b | (1 << i))
        };
        Stops {
            manual: bits(manual, &Stops::MANUAL),
            pedal: bits(pedal, &Stops::PEDAL) as u8,
            sub: false,
            octave: false,
        }
    }

    /// A quiet stopped flute, the softest the organ has.
    pub fn soft() -> Stops {
        Stops::of(&["Gedackt 8'"], &["Subbass 16'"])
    }
    /// Flutes at 8' and 4'.
    pub fn flutes() -> Stops {
        Stops::of(&["Gedackt 8'", "Rohrflute 4'"], &["Subbass 16'"])
    }
    /// The two string stops, one a hair sharp, beating gently.
    pub fn strings() -> Stops {
        Stops::of(&["Salicional 8'", "Voix celeste 8'"], &["Subbass 16'"])
    }
    /// The principal alone.
    pub fn principal() -> Stops {
        Stops::of(&["Principal 8'"], &["Subbass 16'", "Octavebass 8'"])
    }
    /// The principal chorus, 8' to the mixture.
    pub fn chorus() -> Stops {
        Stops::of(
            &[
                "Principal 8'",
                "Octave 4'",
                "Quint 2 2/3'",
                "Superoctave 2'",
                "Mixture IV",
            ],
            &["Subbass 16'", "Octavebass 8'"],
        )
    }
    /// The grand chorus, the 16' stops under the chorus (the organ Aedan picked, O8).
    pub fn grand() -> Stops {
        Stops::of(
            &[
                "Praestant 16'",
                "Bourdon 16'",
                "Principal 8'",
                "Octave 4'",
                "Quint 2 2/3'",
                "Superoctave 2'",
                "Mixture IV",
            ],
            &["Subbass 16'", "Octavebass 8'", "Choralbass 4'"],
        )
    }
    /// The full organ: the grand chorus with the cymbal and the trumpet, the pedal's trombone.
    pub fn full() -> Stops {
        let g = Stops::grand();
        let more = Stops::of(&["Cymbal III", "Trumpet 8'"], &["Posaune 16'"]);
        Stops {
            manual: g.manual | more.manual,
            pedal: g.pedal | more.pedal,
            ..g
        }
    }
    /// A trumpet solo over a flute.
    pub fn trumpet() -> Stops {
        Stops::of(&["Trumpet 8'", "Gedackt 8'"], &["Subbass 16'", "Octavebass 8'"])
    }
    /// An oboe solo.
    pub fn oboe() -> Stops {
        Stops::of(&["Oboe 8'"], &["Subbass 16'"])
    }
}

/// A slide of the pitch: by `cents` (up when positive), starting `at` seconds into the note, taking `over` seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bend {
    pub cents: f64,
    pub at: f64,
    pub over: f64,
}

/// A swell of the strength from `from` to `to` (0 to 1) over the note's first `over` seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swell {
    pub from: f64,
    pub to: f64,
    pub over: f64,
}

/// A vibrato: `cents` either way, `hz` times a second, starting after `wait` seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vibrato {
    pub cents: f64,
    pub hz: f64,
    pub wait: f64,
}

/// A mute or a short stop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mute {
    /// Played open.
    Open,
    /// With a mute (brass: its straight mute; a bowed string's con sordino on its bridge) or damped by hand (plucked
    /// strings).
    Muted,
    /// Stopped dead at the key's release (a cymbal grabbed, a string choked; a wind's tongue stopping the note).
    Choked,
    /// Cut short, as short as the instrument allows.
    Clipped,
    /// Brass, a straight mute: a cone in the bell, bright, nasal and buzzy.
    Straight,
    /// Brass, a cup mute: the cone with a cup over the bell's mouth, round and muffled.
    Cup,
    /// Brass, a harmon mute with its stem out: thin, buzzy and close (the jazz trumpet's ballad sound).
    Harmon,
    /// Brass, a harmon mute with its stem in: a strong nasal ring, the "wah-wah" with a hand over the stem.
    HarmonStem,
    /// Brass, a plunger (a rubber cup) held over the bell, opened and closed by the hand as `expression` says: the
    /// "wah-wah".
    Plunger,
    /// Brass, a practice mute: the bell almost shut, quiet and stuffy.
    Practice,
    /// A horn's stopped note: the hand closes the bell almost completely, a thin, brassy buzz.
    Stopped,
    /// A horn's brass stopping mute: the stopped buzz, a little brighter.
    StopMute,
}

/// An accordion's register: which banks of reeds sound for a key (the switches over the keys, or the buttons of the
/// bass side).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Register {
    /// One reed at pitch ("clarinet").
    One,
    /// Two reeds at pitch, one a little sharp ("violin"), beating gently.
    Two,
    /// The musette: three reeds at pitch, one flat, one sharp, beating fast (the Paris café sound).
    Musette,
    /// The low reed alone, an octave under ("bassoon").
    Low,
    /// The low and the middle reeds, an octave apart ("bandoneon").
    Octave,
    /// The master: every bank, low, middle and high.
    Master,
    /// A bass button: the note with its octaves over.
    Bass,
    /// A chord button's reeds: the note and its octave over.
    Chord,
}

impl Play {
    /// A plain note: no accent, slur, slide, swell or vibrato, no pedal, open, on its own, played the usual way on
    /// the instrument's own stops.
    pub fn plain(midi: f64, dur: f64, vel: f64) -> Play {
        Play {
            midi,
            dur,
            vel,
            accent: 0.0,
            slur_from: None,
            bend: None,
            scoop: None,
            swell: None,
            vibrato: None,
            pedals: Pedals::UP,
            mute: Mute::Open,
            together: 1,
            technique: Technique::Normal,
            stops: None,
            register: None,
            expression: None,
            position: None,
            string: None,
            tie: false,
        }
    }

    /// The note's key for a cache: every field, as bits (`-0.0` as `0.0`, so equal notes share a key).
    pub fn key(&self) -> Vec<u64> {
        let b = |x: f64| if x == 0.0 { 0 } else { x.to_bits() };
        let mut k = vec![b(self.midi), b(self.dur), b(self.vel), b(self.accent)];
        match self.slur_from {
            Some(m) => k.extend([1, b(m)]),
            None => k.push(0),
        }
        match self.bend {
            Some(x) => k.extend([1, b(x.cents), b(x.at), b(x.over)]),
            None => k.push(0),
        }
        match self.scoop {
            Some(x) => k.extend([1, b(x.cents), b(x.at), b(x.over)]),
            None => k.push(0),
        }
        match self.swell {
            Some(x) => k.extend([1, b(x.from), b(x.to), b(x.over)]),
            None => k.push(0),
        }
        match self.vibrato {
            Some(x) => k.extend([1, b(x.cents), b(x.hz), b(x.wait)]),
            None => k.push(0),
        }
        let p = &self.pedals;
        k.extend([b(p.sustain), u64::from(p.soft), u64::from(p.sostenuto)]);
        match p.lift {
            Some(x) => k.extend([1, b(x)]),
            None => k.push(0),
        }
        k.extend([self.mute as u64, u64::from(self.together)]);
        k.push(match self.technique {
            Technique::Normal => 0,
            Technique::Harmonic(n) => 0x100 | u64::from(n),
            Technique::Plucked => 2,
            Technique::Fingers => 3,
            Technique::Picked => 4,
            Technique::Nails => 5,
            Technique::Thumb => 6,
            Technique::Slap => 7,
            Technique::Pop => 8,
            Technique::Tapped => 9,
            Technique::Hammer => 10,
            Technique::PinchHarmonic => 11,
            Technique::Arco => 12,
            Technique::Pizzicato => 13,
            Technique::SnapPizz => 14,
            Technique::LeftHandPizz => 15,
            Technique::Spiccato => 16,
            Technique::Martele => 17,
            Technique::Tremolo => 18,
            Technique::ColLegno => 19,
            Technique::ColLegnoTratto => 20,
            Technique::Breath => 21,
            Technique::BackTongue => 22,
            Technique::SoftTongue => 23,
            Technique::Flutter => 24,
            Technique::Growl => 25,
            Technique::Subtone => 26,
            Technique::AirTone => 27,
            Technique::KeyClick => 28,
            Technique::Shake => 29,
            Technique::HalfValve => 30,
            Technique::WhistleTone => 31,
            Technique::JetWhistle => 32,
            Technique::Sung(n) => 0x200 | u64::from(n as u8),
        });
        match self.stops {
            Some(s) => k.extend([
                1,
                u64::from(s.manual),
                u64::from(s.pedal),
                u64::from(s.sub),
                u64::from(s.octave),
            ]),
            None => k.push(0),
        }
        match self.register {
            Some(r) => k.extend([1, r as u64]),
            None => k.push(0),
        }
        match self.expression {
            Some(x) => k.extend([1, b(x.from), b(x.to), b(x.over)]),
            None => k.push(0),
        }
        match self.position {
            Some(x) => k.extend([1, b(x)]),
            None => k.push(0),
        }
        match self.string {
            Some(x) => k.extend([1, u64::from(x)]),
            None => k.push(0),
        }
        k.push(u64::from(self.tie));
        k
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_notes_share_a_key_and_different_ones_do_not() {
        let a = Play::plain(60.0, 0.5, 0.8);
        assert_eq!(
            a.key(),
            Play {
                vel: 0.8,
                ..Play::plain(60.0, 0.5, 0.8)
            }
            .key()
        );
        assert_eq!(
            Play {
                accent: -0.0,
                ..a.clone()
            }
            .key(),
            a.key()
        );
        for other in [
            Play {
                pedals: Pedals::SUSTAIN,
                ..a.clone()
            },
            Play {
                pedals: Pedals {
                    sustain: 0.5,
                    ..Pedals::UP
                },
                ..a.clone()
            },
            Play {
                pedals: Pedals {
                    lift: Some(0.2),
                    ..Pedals::SUSTAIN
                },
                ..a.clone()
            },
            Play {
                pedals: Pedals {
                    soft: true,
                    ..Pedals::UP
                },
                ..a.clone()
            },
            Play {
                pedals: Pedals {
                    sostenuto: true,
                    ..Pedals::UP
                },
                ..a.clone()
            },
            Play {
                technique: Technique::Harmonic(2),
                ..a.clone()
            },
            Play {
                technique: Technique::Harmonic(3),
                ..a.clone()
            },
            Play {
                technique: Technique::Plucked,
                ..a.clone()
            },
            Play {
                stops: Some(Stops::grand()),
                ..a.clone()
            },
            Play {
                stops: Some(Stops {
                    octave: true,
                    ..Stops::grand()
                }),
                ..a.clone()
            },
            Play {
                expression: Some(Swell {
                    from: 0.0,
                    to: 1.0,
                    over: 1.0,
                }),
                ..a.clone()
            },
            Play {
                technique: Technique::Picked,
                ..a.clone()
            },
            Play {
                technique: Technique::Slap,
                ..a.clone()
            },
            Play {
                technique: Technique::Hammer,
                ..a.clone()
            },
            Play {
                position: Some(0.1),
                ..a.clone()
            },
            Play {
                string: Some(2),
                ..a.clone()
            },
            Play { tie: true, ..a.clone() },
            Play {
                technique: Technique::Arco,
                ..a.clone()
            },
            Play {
                technique: Technique::Pizzicato,
                ..a.clone()
            },
            Play {
                technique: Technique::Spiccato,
                ..a.clone()
            },
            Play {
                technique: Technique::Tremolo,
                ..a.clone()
            },
            Play {
                technique: Technique::ColLegnoTratto,
                ..a.clone()
            },
            Play {
                technique: Technique::Flutter,
                ..a.clone()
            },
            Play {
                technique: Technique::Sung(-12),
                ..a.clone()
            },
            Play {
                technique: Technique::Sung(-7),
                ..a.clone()
            },
            Play {
                mute: Mute::Plunger,
                ..a.clone()
            },
            Play {
                mute: Mute::Stopped,
                ..a.clone()
            },
            Play {
                scoop: Some(Bend {
                    cents: -100.0,
                    at: 0.0,
                    over: 0.1,
                }),
                ..a.clone()
            },
            Play {
                register: Some(Register::Musette),
                ..a.clone()
            },
            Play {
                register: Some(Register::One),
                ..a.clone()
            },
        ] {
            assert_ne!(other.key(), a.key(), "{other:?}");
        }
        assert_ne!(
            Play {
                swell: Some(Swell {
                    from: 0.5,
                    to: 1.0,
                    over: 1.0
                }),
                ..a.clone()
            }
            .key(),
            a.key()
        );
        assert_ne!(
            Play {
                together: 3,
                ..a.clone()
            }
            .key(),
            a.key()
        );
    }

    #[test]
    fn the_named_registrations_are_their_stops() {
        assert_eq!(Stops::grand().manual.count_ones(), 7);
        assert_eq!(Stops::grand().pedal, 0b111);
        assert_eq!(Stops::full().manual, Stops::grand().manual | (1 << 11) | (1 << 12));
        assert_eq!(Stops::full().pedal, 0b1111);
        assert_eq!(Stops::soft(), Stops::of(&["Gedackt 8'"], &["Subbass 16'"]));
        assert_eq!(Stops::of(&["no such stop"], &[]).manual, 0);
        for s in [
            Stops::soft(),
            Stops::flutes(),
            Stops::strings(),
            Stops::principal(),
            Stops::chorus(),
            Stops::trumpet(),
            Stops::oboe(),
        ] {
            assert!(s.manual != 0 && s.pedal != 0, "{s:?}");
        }
    }
}
