//! How a note is played: everything a player does with one note besides choosing its pitch and length. Today's parts
//! play plain notes ([`Play::plain`]); the composition system can ask for any of the rest (see
//! `docs/studio/voice-engine-2-playing.md`). An instrument takes what it can do and leaves the rest: a piano has no
//! vibrato, an organ no accent.

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
    /// can (a wind, a bowed string).
    pub slur_from: Option<f64>,
    /// A slide of the pitch: a bend, a fall, a scoop.
    pub bend: Option<Bend>,
    /// A swell of the strength over the note: an organ's stops coming in, a wind's crescendo.
    pub swell: Option<Swell>,
    /// A vibrato.
    pub vibrato: Option<Vibrato>,
    /// A piano's sustain pedal down: the note rings on past the key, with the other strings ringing along.
    pub pedal: bool,
    /// A mute or a short stop.
    pub mute: Mute,
    /// How many notes sound together with this one, itself included (an organ's wind sags a hair under a big chord).
    pub together: u8,
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
    /// With a mute (brass) or damped by hand (strings).
    Muted,
    /// Stopped dead at the key's release (a cymbal grabbed, a string choked).
    Choked,
    /// Cut short, as short as the instrument allows.
    Clipped,
}

impl Play {
    /// A plain note: no accent, slur, slide, swell or vibrato, no pedal, open, on its own.
    pub fn plain(midi: f64, dur: f64, vel: f64) -> Play {
        Play {
            midi,
            dur,
            vel,
            accent: 0.0,
            slur_from: None,
            bend: None,
            swell: None,
            vibrato: None,
            pedal: false,
            mute: Mute::Open,
            together: 1,
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
        match self.swell {
            Some(x) => k.extend([1, b(x.from), b(x.to), b(x.over)]),
            None => k.push(0),
        }
        match self.vibrato {
            Some(x) => k.extend([1, b(x.cents), b(x.hz), b(x.wait)]),
            None => k.push(0),
        }
        k.extend([u64::from(self.pedal), self.mute as u64, u64::from(self.together)]);
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
        assert_ne!(
            Play {
                pedal: true,
                ..a.clone()
            }
            .key(),
            a.key()
        );
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
}
