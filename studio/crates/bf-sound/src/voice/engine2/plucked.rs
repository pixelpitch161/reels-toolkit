//! Plucked strings: the acoustic guitar, the electric guitar, the bass guitar, the ukulele and the harp
//! (`docs/studio/voice-engine-2-instruments.md`; the sketch `pl.py`, Aedan's picks A6, E2, B3, U6 and H3).
//!
//! A note is a stiff string's modes, each ringing in two directions: across the top (where the bridge moves most
//! readily, so the body drains it sooner) and along it (a hair sharp, ringing longer, a quieter share). The string is
//! pulled aside at a point by a finger, a nail, the thumb or a pick: the point sets which overtones ring (one at the
//! middle leaves out every second), the width of what plucks it and how quickly it lets go set how bright. Each mode
//! loses energy to the air and to the string's own stiffness (the high ones far sooner) and to the body where the
//! body takes it; when the note is let go, the fretting finger lifts (or the hand damps the string) over a few
//! milliseconds. A fretted note is played on one of the instrument's strings (the lowest position that has it,
//! unless one is asked for), stopped shorter, so the pluck point and the finger are a larger part of what sounds; a
//! harp's strings shorten with pitch.
//!
//! The force on the bridge goes through the body ([`body`]: a guitar's, a ukulele's or a harp's box) or is heard by
//! the pickups (where they sit along the string, and their coil's soft resonant top); an electric's or a bass's amp
//! plays on the fighter's whole sound ([`super::amp`]), so a chord's notes distort together.
//!
//! Notes run in blocks of [`BLOCK`] samples at any sample rate: each mode a phasor turned sample by sample, its
//! pitch and loss set once a block (a bend, a slide, a vibrato, the release).

use bf_core::dmath;
use bf_core::rng::Generator;

use super::body::{self, BodyKind};
use super::eq::{self, Band};
use super::play::{Mute, Play, Technique};
use crate::dsp::{self, FilterKind, butter_sos};

/// A string's make-up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wire {
    /// How long the fundamental rings at `fref` Hz (seconds to fall 60 dB), and how that falls with pitch (the
    /// power of the frequency ratio).
    pub t60: f64,
    pub fref: f64,
    pub tslope: f64,
    /// The loss growing with the square of the frequency (the string's own damping), per second per Hz squared.
    pub b3: f64,
    /// How much sharper each overtone is than a whole multiple (`f_n = n f0 sqrt(1 + b n^2)`).
    pub stiffness: f64,
    /// How strongly the body drains the motion across the top, per second (at the body's most open frequency).
    pub drain: f64,
}

/// A plucked string instrument's make-up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PluckedParts {
    pub wire: Wire,
    /// Its open strings, MIDI, in the order a player numbers them from the chin down (a guitar's low E first, a
    /// ukulele's high G first). Empty: a harp, one string for each note.
    pub open: &'static [f64],
    /// Where it is usually plucked, as a part of the open string (a harp's: of each string) from the bridge.
    pub pluck: f64,
    /// The width of the finger's pad, as a part of the open string.
    pub width: f64,
    /// How soft the finger's pad is: overtones far over this (Hz, at a middling strength) are let go gently. None:
    /// no softening.
    pub soft: Option<f64>,
    /// How strongly the string is damped once the note is let go, per second (more for higher overtones), and how
    /// long the damping takes to land, seconds.
    pub damp: f64,
    pub ramp: f64,
    /// How loud the pluck's knock is (the finger or the pick leaving the string).
    pub knock: f64,
    /// The highest overtone it has, Hz.
    pub top: f64,
    /// The body the force goes through (none: the pickups and their coil).
    pub body: Option<BodyKind>,
    /// The pickups: where each sits, as a part of the string from the bridge, and its share.
    pub pickups: &'static [(f64, f64)],
    /// The pickups' coil and cable: a soft resonant top at `(Hz, Q)`.
    pub coil: Option<(f64, f64)>,
    /// Whether it is usually played with a pick (else with the fingers).
    pub picked: bool,
    /// Its studio polish, on each note.
    pub polish: &'static [Band],
}

/// A guitar's open strings.
const GUITAR: [f64; 6] = [40.0, 45.0, 50.0, 55.0, 59.0, 64.0];
/// A four-string bass's.
const BASS: [f64; 4] = [28.0, 33.0, 38.0, 43.0];
/// A ukulele's, re-entrant (the G above the C).
const UKULELE: [f64; 4] = [67.0, 60.0, 64.0, 69.0];

/// A6's polish: a little less boom and box, more presence and air.
pub const GUITAR_POLISH: [Band; 4] = [
    Band::Low {
        hz: 90.0,
        db: -1.0,
        q: 0.7,
    },
    Band::Peak {
        hz: 250.0,
        db: -1.5,
        q: 0.8,
    },
    Band::Peak {
        hz: 3500.0,
        db: 2.0,
        q: 1.0,
    },
    Band::High {
        hz: 10_000.0,
        db: 3.0,
        q: 0.7,
    },
];
/// U6's.
pub const UKULELE_POLISH: [Band; 3] = [
    Band::Peak {
        hz: 300.0,
        db: -1.0,
        q: 0.8,
    },
    Band::Peak {
        hz: 3500.0,
        db: 2.0,
        q: 1.0,
    },
    Band::High {
        hz: 10_000.0,
        db: 3.0,
        q: 0.7,
    },
];

impl PluckedParts {
    /// The acoustic guitar Aedan picked (A6): a dreadnought with new steel strings, fingers, polished.
    pub const A6: PluckedParts = PluckedParts {
        wire: Wire {
            t60: 8.0,
            fref: 82.0,
            tslope: 0.6,
            b3: 0.6e-6,
            stiffness: 1.2e-4,
            drain: 5.0,
        },
        open: &GUITAR,
        pluck: 0.16,
        width: 0.016,
        soft: Some(9000.0),
        damp: 12.0,
        ramp: 0.015,
        knock: 0.018,
        top: 16_000.0,
        body: Some(BodyKind::Dreadnought),
        pickups: &[],
        coil: None,
        picked: false,
        polish: &GUITAR_POLISH,
    };
    /// The electric guitar (E2): a pick by the bridge pickup, into a high-gain amp.
    pub const E2: PluckedParts = PluckedParts {
        wire: Wire {
            t60: 12.0,
            fref: 82.0,
            tslope: 0.5,
            b3: 1.4e-6,
            stiffness: 1.0e-4,
            drain: 0.4,
        },
        open: &GUITAR,
        pluck: 0.12,
        width: 0.004,
        soft: None,
        damp: 14.0,
        ramp: 0.05,
        knock: 0.01,
        top: 14_000.0,
        body: None,
        pickups: &[(0.065, 1.0)],
        coil: Some((3000.0, 2.0)),
        picked: true,
        polish: &[],
    };
    /// The bass guitar (B3): picked, one pickup, into an amp with a little grit.
    pub const B3: PluckedParts = PluckedParts {
        wire: Wire {
            t60: 16.0,
            fref: 41.0,
            tslope: 0.3,
            b3: 6.0e-6,
            stiffness: 2.2e-4,
            drain: 0.4,
        },
        open: &BASS,
        pluck: 0.14,
        width: 0.004,
        soft: None,
        damp: 20.0,
        ramp: 0.03,
        knock: 0.02,
        top: 9000.0,
        body: None,
        pickups: &[(0.21, 1.0)],
        coil: Some((2600.0, 1.8)),
        picked: true,
        polish: &[],
    };
    /// The ukulele (U6): a soprano with new fluorocarbon strings, fingers, polished.
    pub const U6: PluckedParts = PluckedParts {
        wire: Wire {
            t60: 4.0,
            fref: 262.0,
            tslope: 0.6,
            b3: 1.3e-6,
            stiffness: 4e-5,
            drain: 6.0,
        },
        open: &UKULELE,
        pluck: 0.22,
        width: 0.026,
        soft: Some(9000.0),
        damp: 6.0,
        ramp: 0.015,
        knock: 0.014,
        top: 16_000.0,
        body: Some(BodyKind::Soprano),
        pickups: &[],
        coil: None,
        picked: false,
        polish: &UKULELE_POLISH,
    };
    /// The harp (H3): a concert pedal harp's gut strings, plucked a little nearer the soundboard.
    pub const H3: PluckedParts = PluckedParts {
        wire: Wire {
            t60: 9.0,
            fref: 165.0,
            tslope: 0.95,
            b3: 1.0e-6,
            stiffness: 3e-5,
            drain: 1.0,
        },
        open: &[],
        pluck: 0.33,
        width: 0.035,
        soft: Some(3200.0),
        damp: 1.5,
        ramp: 0.015,
        knock: 0.008,
        top: 12_000.0,
        body: Some(BodyKind::PedalHarp),
        pickups: &[],
        coil: None,
        picked: false,
        polish: &[],
    };
}

/// The block a note's pitch and losses are set for, samples.
pub const BLOCK: usize = 32;
/// A harp's string at this pitch (Hz) has the finger's own width; its strings shorten as the `HARP_WIDTH_POWER`
/// of the pitch.
const HARP_WIDTH_HZ: f64 = 165.0;
const HARP_WIDTH_POWER: f64 = 0.8;
/// How far the pluck point wanders from one pluck to the next, as a part of the string.
const PLUCK_WANDER: f64 = 0.008;
/// The most overtones a note has.
const MOST_MODES: usize = 400;
/// The overtones stop under this part of the sample rate.
const NYQUIST_SHARE: f64 = 0.45;
/// An accent adds this much strength.
const ACCENT: f64 = 0.12;
/// The width of a pick, a nail (of the finger's), a thumb and a slapping thumb, as parts of the open string.
const PICK_WIDTH: f64 = 0.004;
const NAIL_SHARE: f64 = 0.4;
const THUMB_WIDTH: f64 = 0.03;
const SLAP_WIDTH: f64 = 0.02;
/// Where a slap, a pop and a tap meet the string, as parts of the sounding string from the bridge (a slap and a
/// pop over the last frets, a tap right by the fret).
const SLAP_AT: f64 = 0.3;
const POP_AT: f64 = 0.25;
const TAP_AT: f64 = 0.08;
/// How loud a tapped note is against a plucked one.
const TAP_LEVEL: f64 = 0.7;
/// What a harmonic leaves of the overtones it stops, and what a pinch harmonic leaves (the thumb touches only
/// briefly).
const HARMONIC_LEAVES: f64 = 0.02;
const PINCH_LEAVES: f64 = 0.15;
/// A palm mute's losses: `PALM_DB + PALM_DB_KHZ (f / 1 kHz)^1.3` more a second (in nepers), its top rolled off
/// over `PALM_HZ`.
const PALM: f64 = 9.0;
const PALM_KHZ: f64 = 18.0;
const PALM_HZ: f64 = 1800.0;
/// A dead (choked) note's: `DEAD + DEAD_KHZ (f / 1 kHz)` more a second, both directions alike.
const DEAD: f64 = 60.0;
const DEAD_KHZ: f64 = 40.0;
/// The longest a clipped note is held, seconds.
const CLIPPED: f64 = 0.05;
/// The motion along the top: its share at most and at least (times [`ALONG_SHARE`]), its detuning (a part of the
/// pitch), and how much of the body's drain it feels.
const ALONG_SHARE: f64 = 0.35;
const ALONG_MIX: (f64, f64) = (0.25, 0.5);
const ALONG_SHARP: (f64, f64) = (0.2e-3, 0.6e-3);
const ALONG_DRAIN: f64 = 0.15;
/// How far each overtone's start wanders, radians.
const PHASE_WANDER: f64 = 0.15;
/// A slide from the note before takes this long, seconds; a slurred note starts over this, at this level.
const SLIDE: f64 = 0.06;
const SLUR_START: f64 = 0.006;
const SLUR_LEVEL: f64 = 0.8;
/// The knock's length, seconds, and its band, Hz (a pick's and a nail's brighter).
const KNOCK_LEN: f64 = 0.004;
const KNOCK_LOW: f64 = 400.0;
/// A pluck's first moment (the string leaving the finger) and the note's last, seconds.
const START: f64 = 0.0008;
const END_FADE: f64 = 0.02;
/// A note rings until about 80 dB down (11 nepers).
const RING_NEPERS: f64 = 11.0;
/// A mode is let go once it is this many nepers down.
const GONE_NEPERS: f64 = 16.0;
/// The notes' level.
const GAIN: f64 = 0.25;

fn hz(m: f64) -> f64 {
    440.0 * dmath::pow(2.0, (m - 69.0) / 12.0)
}

/// `np.sinc`: `sin(pi x) / (pi x)`.
fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        let p = dmath::PI * x;
        dmath::sin(p) / p
    }
}

/// The note's pitch, held to a sensible range.
fn key_of(play: &Play) -> f64 {
    if play.midi.is_finite() {
        play.midi.clamp(0.0, 127.0)
    } else {
        60.0
    }
}

/// How the string is set going: a pick, the flesh, a nail, the thumb, a slap, a pop or a tap.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Hand {
    /// The width that sets it going, as a part of the open string.
    width: f64,
    /// The pad's softness, Hz (none: a pick's or a nail's edge).
    soft: Option<f64>,
    /// How much a quick release lifts the top (a pick's 1).
    lift: f64,
    /// Struck (the string given a speed: each overtone `n` times as strong as plucked) rather than pulled aside.
    struck: bool,
    /// Where it meets the sounding string, if not where the player usually plucks.
    at: Option<f64>,
    /// The knock's level (times the instrument's) and the top of its band, Hz; none for no knock.
    knock: Option<(f64, f64)>,
    level: f64,
}

/// How technique `t` sets instrument `p`'s string going.
fn hand(p: &PluckedParts, t: Technique) -> Hand {
    let fingers = Hand {
        width: p.width,
        soft: p.soft,
        lift: 0.0,
        struck: false,
        at: None,
        knock: Some((1.0, 2500.0)),
        level: 1.0,
    };
    let pick = Hand {
        width: PICK_WIDTH,
        soft: None,
        lift: 1.0,
        knock: Some((2.0, 6000.0)),
        ..fingers
    };
    match t {
        Technique::Picked | Technique::PinchHarmonic => pick,
        Technique::Fingers | Technique::Plucked => fingers,
        Technique::Normal | Technique::Harmonic(_) => {
            if p.picked {
                pick
            } else {
                fingers
            }
        }
        Technique::Nails => Hand {
            width: (NAIL_SHARE * p.width).max(PICK_WIDTH),
            soft: p.soft.map(|s| 2.0 * s),
            lift: 0.3,
            knock: Some((1.5, 5000.0)),
            ..fingers
        },
        Technique::Thumb => Hand {
            width: THUMB_WIDTH,
            ..fingers
        },
        Technique::Slap => Hand {
            width: SLAP_WIDTH,
            soft: None,
            struck: true,
            at: Some(SLAP_AT),
            knock: Some((4.0, 5000.0)),
            ..fingers
        },
        Technique::Pop => Hand {
            width: PICK_WIDTH,
            soft: None,
            lift: 1.5,
            at: Some(POP_AT),
            knock: Some((4.0, 8000.0)),
            ..fingers
        },
        Technique::Tapped | Technique::Hammer => Hand {
            width: SLAP_WIDTH,
            struck: true,
            at: Some(TAP_AT),
            knock: Some((0.5, 1500.0)),
            level: TAP_LEVEL,
            ..fingers
        },
    }
}

/// A note made ready: its modes, how each rings and how it is let go.
struct Setup {
    vel: f64,
    dur: f64,
    hand: Hand,
    /// The overtones' frequencies (Hz), starting levels and loss rates across and along the top, and their
    /// release damping (all per second, in nepers).
    f: Vec<f64>,
    amp: Vec<f64>,
    sig_v: Vec<f64>,
    sig_h: Vec<f64>,
    damp: Vec<f64>,
    /// Whether the note joins the one before without a new pluck, and a slide's start, cents from the note.
    slur: bool,
    slide: Option<f64>,
    /// The motion along the top: its share, how sharp it runs (a factor); each overtone's starting phase.
    pol: f64,
    sharp: f64,
    ph0: Vec<f64>,
}

impl Setup {
    /// Note `play` on `p` at sample rate `fs`, its random draws from `rng` in the sketch's order (the pluck point's
    /// wander, the motion along the top, the overtones' phases).
    fn new(p: &PluckedParts, play: &Play, rng: &mut Generator, fs: f64) -> Setup {
        let m = key_of(play);
        let f0 = hz(m);
        let vel = (play.vel + ACCENT * play.accent.clamp(0.0, 1.0)).clamp(0.0, 1.0);
        let dur = match play.mute {
            Mute::Clipped => play.dur.clamp(0.0, CLIPPED),
            _ => play.dur.max(0.0),
        };
        let t = play.technique;
        let hand = hand(p, t);
        // which string, how far up it is stopped: the pluck point and the width are parts of the sounding length,
        // which shortens as the note is stopped higher (a harp's strings shorten with pitch)
        let short = if p.open.is_empty() {
            dmath::pow(f0 / HARP_WIDTH_HZ, HARP_WIDTH_POWER)
        } else {
            let asked = play
                .string
                .and_then(|s| p.open.get(usize::from(s)).copied())
                .filter(|&o| o <= m);
            let near = p
                .open
                .iter()
                .copied()
                .filter(|&o| (0.0..=4.0).contains(&(m - o)))
                .fold(f64::NEG_INFINITY, f64::max);
            let below = p
                .open
                .iter()
                .copied()
                .filter(|&o| o <= m)
                .fold(f64::NEG_INFINITY, f64::max);
            let lowest = p.open.iter().copied().fold(f64::INFINITY, f64::min);
            let o = asked.unwrap_or(if near.is_finite() {
                near
            } else if below.is_finite() {
                below
            } else {
                lowest
            });
            dmath::pow(2.0, (m - o) / 12.0)
        };
        let wander = rng.uniform(-PLUCK_WANDER, PLUCK_WANDER);
        let fretted = !p.open.is_empty();
        let beta = match (play.position, hand.at) {
            (Some(x), _) if x.is_finite() => x.clamp(0.01, 0.5),
            (_, Some(x)) => x,
            _ => (p.pluck * if fretted { short } else { 1.0 } + wander).min(0.5),
        };
        let width = hand.width * short;
        // how quickly the string leaves the finger: a harder pluck sooner and brighter
        let w_eff = width * (1.35 - 0.6 * vel);
        let top = p.top.min(NYQUIST_SHARE * fs);
        let count = ((top / f0) as usize).clamp(1, MOST_MODES);
        let b = p.wire.stiffness;
        let (mut f, mut amp) = (Vec::with_capacity(count), Vec::with_capacity(count));
        let harmonic = match t {
            Technique::Harmonic(h) => Some((h.clamp(2, 6), HARMONIC_LEAVES)),
            Technique::PinchHarmonic => Some((((1.0 / beta).round() as u8).clamp(3, 6), PINCH_LEAVES)),
            _ => None,
        };
        for k in 1..=count {
            let n = k as f64;
            let fk = n * f0 * (1.0 + b * n * n).sqrt();
            if fk >= top {
                continue;
            }
            let mut a = dmath::sin(dmath::PI * n * beta) / n * sinc(n * w_eff);
            if hand.struck {
                a *= n;
            }
            if let Some((h, leaves)) = harmonic
                && k % usize::from(h) != 0
            {
                a *= leaves;
            }
            if hand.lift > 0.0 {
                a *= 1.0 + 0.6 * hand.lift * vel * (fk / 3000.0).clamp(0.0, 2.0);
            }
            if !p.pickups.is_empty() {
                let heard: f64 = p
                    .pickups
                    .iter()
                    .map(|&(alpha, g)| g * dmath::sin(dmath::PI * n * alpha) * sinc(n * 0.01))
                    .sum();
                a *= heard;
            }
            if let Some(soft) = hand.soft {
                let fc = soft * (0.6 + 0.8 * vel);
                let r = fk / fc;
                a /= (1.0 + r * r * r * r).sqrt();
            }
            f.push(fk);
            amp.push(a * dmath::pow(vel, 1.1) * hand.level);
        }
        // the losses: the air and the string's own damping, the body's drain across the top
        let w = p.wire;
        let t60 = w.t60 * dmath::pow(f0 / w.fref, -w.tslope);
        let sig: Vec<f64> = f.iter().map(|fk| 6.91 / t60 + w.b3 * fk * fk).collect();
        let mut sig_v: Vec<f64> = match p.body {
            Some(kind) => body::admittance(kind, &f)
                .iter()
                .zip(&sig)
                .map(|(y, s)| s + w.drain * y)
                .collect(),
            None => sig.iter().map(|s| s + w.drain).collect(),
        };
        let mut sig_h: Vec<f64> = sig.iter().zip(&sig_v).map(|(s, v)| s + ALONG_DRAIN * (v - s)).collect();
        match play.mute {
            Mute::Muted => {
                for (i, fk) in f.iter().enumerate() {
                    let extra = PALM + PALM_KHZ * dmath::pow(fk / 1000.0, 1.3);
                    sig_v[i] += extra;
                    sig_h[i] += extra;
                    let r = fk / PALM_HZ;
                    amp[i] /= 1.0 + r * r;
                }
            }
            Mute::Choked => {
                for (i, fk) in f.iter().enumerate() {
                    sig_v[i] += DEAD + DEAD_KHZ * fk / 1000.0;
                    sig_h[i] = sig_v[i];
                }
            }
            Mute::Open | Mute::Clipped => {}
        }
        let damp = f.iter().map(|fk| p.damp * (1.0 + fk / 1500.0)).collect();
        let slur = play.slur_from.is_some_and(f64::is_finite);
        let slide = play
            .slur_from
            .filter(|s| s.is_finite() && t != Technique::Hammer && (s - m).abs() > 1e-9)
            .map(|s| 100.0 * (s.clamp(0.0, 127.0) - m));
        let pol = rng.uniform(ALONG_MIX.0, ALONG_MIX.1);
        let sharp = 1.0 + rng.uniform(ALONG_SHARP.0, ALONG_SHARP.1);
        let ph0 = (0..f.len()).map(|_| rng.uniform(-PHASE_WANDER, PHASE_WANDER)).collect();
        Setup {
            vel,
            dur,
            hand,
            f,
            amp,
            sig_v,
            sig_h,
            damp,
            slur,
            slide,
            pol,
            sharp,
            ph0,
        }
    }

    /// How long it rings, seconds, at most `longest`: until about 80 dB down, through the release when it is
    /// damped, else by the string's own losses.
    fn rings(&self, p: &PluckedParts, longest: f64) -> f64 {
        let slowest = self.sig_h.iter().copied().fold(f64::INFINITY, f64::min);
        let released = self
            .sig_h
            .iter()
            .zip(&self.damp)
            .map(|(s, d)| s + d)
            .fold(f64::INFINITY, f64::min);
        let ring = if slowest.is_finite() {
            let natural = RING_NEPERS / slowest.max(1e-3);
            let released = self.dur + p.ramp + RING_NEPERS / released.max(1e-3);
            natural.min(released) + 0.05
        } else {
            0.0
        };
        ring.max(self.dur + 0.25).min(longest).max(0.05)
    }

    /// The pitch at `t` seconds, cents from the note: a slide from the note before, a bend, a vibrato.
    fn cents(&self, play: &Play, t: f64) -> f64 {
        let ease = |u: f64| 0.5 - 0.5 * dmath::cos(dmath::PI * u.clamp(0.0, 1.0));
        let mut c = 0.0;
        if let Some(from) = self.slide {
            c += from * (1.0 - ease(t / SLIDE));
        }
        if let Some(b) = play.bend
            && b.cents.is_finite()
            && b.at.is_finite()
            && b.over.is_finite()
        {
            c += b.cents * ease((t - b.at) / b.over.max(1e-3));
        }
        if let Some(v) = play.vibrato
            && v.cents.is_finite()
            && v.hz.is_finite()
            && v.wait.is_finite()
        {
            let since = (t - v.wait).max(0.0);
            c += v.cents * (since / 0.25).min(1.0) * dmath::sin(dmath::TAU * v.hz * since);
        }
        c.clamp(-2400.0, 2400.0)
    }
}

/// How long the note `play` rings on `p` with no length asked for, seconds, at most `longest`.
pub fn rings_for(p: &PluckedParts, play: &Play, longest: f64) -> f64 {
    let mut rng = Generator::new(0);
    Setup::new(p, play, &mut rng, crate::SRF).rings(p, longest)
}

/// The modes rung out for `len` samples: each a phasor turned sample by sample, its pitch and loss set once a
/// block, let go once it has died away.
fn ring_out(s: &Setup, play: &Play, p: &PluckedParts, len: usize, fs: f64) -> Vec<f64> {
    struct Mode {
        f: f64,
        sig: f64,
        damp: f64,
        z: (f64, f64),
        gone: f64,
        step: (f64, f64),
    }
    let mut modes: Vec<Mode> = Vec::with_capacity(2 * s.f.len());
    for (share, along) in [(1.0, false), (s.pol * ALONG_SHARE, true)] {
        for i in 0..s.f.len() {
            let a = s.amp[i] * share;
            if a == 0.0 || !a.is_finite() {
                continue;
            }
            modes.push(Mode {
                f: if along { s.f[i] * s.sharp } else { s.f[i] },
                sig: if along { s.sig_h[i] } else { s.sig_v[i] },
                damp: s.damp[i],
                z: (a * dmath::cos(s.ph0[i]), a * dmath::sin(s.ph0[i])),
                gone: (a * dmath::exp(-GONE_NEPERS)).abs(),
                step: (1.0, 0.0),
            });
        }
    }
    let mut out = vec![0.0; len];
    let (mut last_rho, mut last_rel) = (f64::NAN, f64::NAN);
    let mut a = 0;
    while a < len && !modes.is_empty() {
        let b = (a + BLOCK).min(len);
        let mid = (a as f64 + 0.5 * (b - a) as f64) / fs;
        let rho = dmath::pow(2.0, s.cents(play, mid) / 1200.0);
        let rel = if mid <= s.dur {
            0.0
        } else {
            let u = ((mid - s.dur) / p.ramp.max(1e-6)).min(1.0);
            0.5 - 0.5 * dmath::cos(dmath::PI * u)
        };
        if rho != last_rho || rel != last_rel {
            for md in &mut modes {
                let r = dmath::exp(-(md.sig + md.damp * rel) / fs);
                let w = dmath::TAU * md.f * rho / fs;
                md.step = (r * dmath::cos(w), r * dmath::sin(w));
            }
            (last_rho, last_rel) = (rho, rel);
        }
        let block = &mut out[a..b];
        for md in &mut modes {
            let (sr, si) = md.step;
            let (mut zr, mut zi) = md.z;
            for v in block.iter_mut() {
                *v += zi;
                (zr, zi) = (zr * sr - zi * si, zr * si + zi * sr);
            }
            md.z = (zr, zi);
        }
        modes.retain(|md| md.z.0.abs() + md.z.1.abs() > md.gone);
        a = b;
    }
    out
}

/// The note `play` on the plucked string instrument `p`: `len` samples at sample rate `fs`, as repeat `variant`
/// (each repeat plucked a hair differently).
pub fn note(p: &PluckedParts, play: &Play, len: usize, variant: u64, fs: f64) -> Vec<f64> {
    if len == 0 {
        return Vec::new();
    }
    let m = key_of(play);
    // numpy's default_rng(1000 * variant + int(7 m)) in the sketch
    let mut rng = Generator::new(1000u64.wrapping_mul(variant).wrapping_add((m * 7.0) as u64));
    let s = Setup::new(p, play, &mut rng, fs);
    let mut x = ring_out(&s, play, p, len, fs);
    // the knock: the finger or the pick leaving the string (none on a dead note, or one joined to the note before)
    if let (Some((level, cut)), false, false) = (s.hand.knock, s.slur, play.mute == Mute::Choked) {
        let k = ((KNOCK_LEN * fs) as usize).min(len);
        if k > 1 {
            let noise: Vec<f64> = (0..k)
                .map(|i| {
                    let hann = 0.5 - 0.5 * dmath::cos(dmath::TAU * i as f64 / (k - 1) as f64);
                    rng.standard_normal() * hann
                })
                .collect();
            let sos = butter_sos(FilterKind::Bp, &[KNOCK_LOW, cut.min(NYQUIST_SHARE * fs)], 2, fs);
            let noise = dsp::sosfilt(&sos, &noise);
            for (v, n) in x.iter_mut().zip(noise) {
                *v += p.knock * s.vel * n * level;
            }
        }
    }
    if s.slur {
        // the string already moving: no new pluck, a soft start
        let n = SLUR_START * fs;
        for (i, v) in x.iter_mut().enumerate() {
            *v *= SLUR_LEVEL * (i as f64 / n).min(1.0);
        }
    }
    let a = ((START * fs) as usize).min(len);
    for (i, v) in x[..a].iter_mut().enumerate() {
        *v *= i as f64 / a as f64;
    }
    if let Some((f, q)) = p.coil {
        x = coil(&x, f, q, fs);
    }
    if let Some(kind) = p.body {
        x = body::apply(&x, kind, fs);
    }
    let mut y = eq::apply(&x, p.polish, fs);
    y.iter_mut().for_each(|v| *v *= GAIN);
    end_fade(&mut y, fs);
    y
}

/// The pickups' coil and cable: a soft resonant low-pass at `f` Hz with quality `q`.
fn coil(x: &[f64], f: f64, q: f64, fs: f64) -> Vec<f64> {
    let w = dmath::TAU * f.min(NYQUIST_SHARE * fs) / fs;
    let al = dmath::sin(w) / (2.0 * q);
    let c = dmath::cos(w);
    let b = [
        (1.0 - c) / 2.0 / (1.0 + al),
        (1.0 - c) / (1.0 + al),
        (1.0 - c) / 2.0 / (1.0 + al),
    ];
    let a = [1.0, -2.0 * c / (1.0 + al), (1.0 - al) / (1.0 + al)];
    dsp::lfilter(&b, &a, x)
}

/// The note's last 20 ms faded (less for a very short note), so a note cut short ends without a click.
fn end_fade(x: &mut [f64], fs: f64) {
    let n = ((END_FADE * fs) as usize).min(x.len() / 2);
    let len = x.len();
    for i in 0..n {
        x[len - 1 - i] *= i as f64 / n as f64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SRF;
    use crate::voice::engine2::play::{Bend, Vibrato};

    fn sound(p: &PluckedParts, play: &Play, secs: f64) -> Vec<f64> {
        note(p, play, (secs * SRF) as usize, 0, SRF)
    }

    /// The level over `a..b` seconds, dB.
    fn db(x: &[f64], a: f64, b: f64) -> f64 {
        let (i, j) = ((a * SRF) as usize, ((b * SRF) as usize).min(x.len()));
        let e = x[i..j].iter().map(|v| v * v).sum::<f64>() / (j - i).max(1) as f64;
        10.0 * dmath::log10(e.max(1e-30))
    }

    /// The share of the energy over `a..b` seconds above `hz`, dB (a one-pole split, enough to compare).
    fn bright(x: &[f64], a: f64, b: f64, hz: f64) -> f64 {
        let k = dmath::exp(-dmath::TAU * hz / SRF);
        let mut lp = 0.0;
        let hi: Vec<f64> = x
            .iter()
            .map(|&v| {
                lp = (1.0 - k) * v + k * lp;
                v - lp
            })
            .collect();
        db(&hi, a, b) - db(x, a, b)
    }

    /// The strongest frequency within 3% of `f` over `a..b` seconds, Hz.
    fn peak_near(x: &[f64], f: f64, a: f64, b: f64) -> f64 {
        let seg = &x[(a * SRF) as usize..((b * SRF) as usize).min(x.len())];
        let (mut best, mut at) = (0.0, f);
        let mut g = f * 0.97;
        while g < f * 1.03 {
            let w = dmath::TAU * g / SRF;
            let (mut re, mut im) = (0.0, 0.0);
            for (i, v) in seg.iter().enumerate() {
                let hann = 0.5 - 0.5 * dmath::cos(dmath::TAU * i as f64 / seg.len() as f64);
                re += v * hann * dmath::cos(w * i as f64);
                im += v * hann * dmath::sin(w * i as f64);
            }
            let p = re * re + im * im;
            if p > best {
                best = p;
                at = g;
            }
            g *= 1.0 + 0.5 / 1200.0;
        }
        at
    }

    /// The level of the component at exactly `f` over `a..b` seconds, dB.
    fn at_hz(x: &[f64], f: f64, a: f64, b: f64) -> f64 {
        let seg = &x[(a * SRF) as usize..((b * SRF) as usize).min(x.len())];
        let w = dmath::TAU * f / SRF;
        let (mut re, mut im) = (0.0, 0.0);
        for (i, v) in seg.iter().enumerate() {
            let hann = 0.5 - 0.5 * dmath::cos(dmath::TAU * i as f64 / seg.len() as f64);
            re += v * hann * dmath::cos(w * i as f64);
            im += v * hann * dmath::sin(w * i as f64);
        }
        10.0 * dmath::log10((re * re + im * im).max(1e-30))
    }

    const ALL: [PluckedParts; 5] = [
        PluckedParts::A6,
        PluckedParts::E2,
        PluckedParts::B3,
        PluckedParts::U6,
        PluckedParts::H3,
    ];

    #[test]
    fn every_instrument_plays_in_tune() {
        for (p, m) in ALL.iter().zip([52.0, 57.0, 40.0, 69.0, 60.0]) {
            let x = sound(p, &Play::plain(m, 1.5, 0.7), 1.6);
            let f = peak_near(&x, hz(m), 0.2, 1.2);
            let off = 1200.0 * dmath::log2(f / hz(m));
            assert!(off.abs() < 6.0, "{p:?} at {m}: {off:.1} cents");
        }
    }

    #[test]
    fn a_harder_pluck_is_louder_and_no_darker() {
        for p in [PluckedParts::A6, PluckedParts::U6, PluckedParts::H3] {
            let soft = sound(&p, &Play::plain(64.0, 1.0, 0.3), 1.0);
            let hard = sound(&p, &Play::plain(64.0, 1.0, 0.9), 1.0);
            assert!(db(&hard, 0.0, 0.3) > db(&soft, 0.0, 0.3) + 6.0);
            // the finger leaves a harder pluck sooner: a touch brighter (as the sketch, a fraction of a decibel)
            assert!(bright(&hard, 0.0, 0.2, 2000.0) > bright(&soft, 0.0, 0.2, 2000.0));
        }
    }

    #[test]
    fn a_pick_is_brighter_than_the_fingers_and_the_thumb_darker() {
        let p = PluckedParts::A6;
        let with = |t| {
            sound(
                &p,
                &Play {
                    technique: t,
                    ..Play::plain(57.0, 1.0, 0.7)
                },
                1.0,
            )
        };
        let (fingers, pick, thumb) = (
            with(Technique::Fingers),
            with(Technique::Picked),
            with(Technique::Thumb),
        );
        let b = |x: &[f64]| bright(x, 0.0, 0.3, 2500.0);
        assert!(b(&pick) > b(&fingers) + 1.0, "{} against {}", b(&pick), b(&fingers));
        assert!(b(&thumb) < b(&fingers) - 0.5, "{} against {}", b(&thumb), b(&fingers));
        // a plain note on the acoustic is fingers; on the electric, a pick
        assert_eq!(sound(&p, &Play::plain(57.0, 1.0, 0.7), 1.0), fingers);
        let e = PluckedParts::E2;
        let plain = sound(&e, &Play::plain(57.0, 1.0, 0.7), 1.0);
        let picked = sound(
            &e,
            &Play {
                technique: Technique::Picked,
                ..Play::plain(57.0, 1.0, 0.7)
            },
            1.0,
        );
        assert_eq!(plain, picked);
    }

    #[test]
    fn a_palm_mute_is_short_and_dull_and_a_dead_note_shorter() {
        for p in [PluckedParts::E2, PluckedParts::B3, PluckedParts::A6] {
            let open = sound(&p, &Play::plain(45.0, 1.0, 0.7), 1.2);
            let muted = sound(
                &p,
                &Play {
                    mute: Mute::Muted,
                    ..Play::plain(45.0, 1.0, 0.7)
                },
                1.2,
            );
            let dead = sound(
                &p,
                &Play {
                    mute: Mute::Choked,
                    ..Play::plain(45.0, 1.0, 0.7)
                },
                1.2,
            );
            // how far each has fallen by 0.4 s
            let fall = |x: &[f64]| db(x, 0.0, 0.05) - db(x, 0.4, 0.5);
            assert!(fall(&muted) > fall(&open) + 10.0, "{p:?}");
            assert!(fall(&dead) > fall(&muted) + 10.0, "{p:?}");
            assert!(
                bright(&muted, 0.0, 0.1, 1500.0) < bright(&open, 0.0, 0.1, 1500.0) - 1.0,
                "{p:?}"
            );
        }
    }

    #[test]
    fn a_note_let_go_is_damped_and_a_harp_rings_on() {
        let g = PluckedParts::A6;
        let short = sound(&g, &Play::plain(57.0, 0.2, 0.7), 1.5);
        let long = sound(&g, &Play::plain(57.0, 1.4, 0.7), 1.5);
        assert!(db(&long, 0.9, 1.1) > db(&short, 0.9, 1.1) + 20.0);
        // a harpist rarely damps: a short note rings on where the guitar's is gone
        let h = PluckedParts::H3;
        let x = sound(&h, &Play::plain(60.0, 0.2, 0.7), 2.0);
        let fall = |x: &[f64]| db(x, 0.3, 0.5) - db(x, 1.0, 1.2);
        assert!(
            fall(&x) < 35.0 && fall(&short) > fall(&x) + 40.0,
            "{} {}",
            fall(&x),
            fall(&short)
        );
        assert!(rings_for(&h, &Play::plain(60.0, 0.2, 0.7), 12.0) > 2.0);
        assert!(rings_for(&g, &Play::plain(57.0, 0.2, 0.7), 12.0) < 1.5);
    }

    #[test]
    fn a_harmonic_sounds_an_octave_up() {
        let p = PluckedParts::A6;
        let m = 45.0;
        let x = sound(
            &p,
            &Play {
                technique: Technique::Harmonic(2),
                ..Play::plain(m, 1.0, 0.7)
            },
            1.0,
        );
        let plain = sound(&p, &Play::plain(m, 1.0, 0.7), 1.0);
        let (f1, f2) = (hz(m), 2.0 * hz(m));
        // the fundamental all but gone, the octave there
        assert!(at_hz(&x, f2, 0.1, 0.6) - at_hz(&x, f1, 0.1, 0.6) > 20.0);
        assert!(at_hz(&plain, f2, 0.1, 0.6) - at_hz(&plain, f1, 0.1, 0.6) < 10.0);
    }

    #[test]
    fn a_bend_reaches_its_pitch_and_a_vibrato_wavers() {
        let p = PluckedParts::E2;
        let m = 64.0;
        let bend = Play {
            bend: Some(Bend {
                cents: 200.0,
                at: 0.05,
                over: 0.12,
            }),
            ..Play::plain(m, 1.2, 0.7)
        };
        let x = sound(&p, &bend, 1.2);
        let f = peak_near(&x, hz(m + 2.0), 0.3, 1.0);
        assert!((1200.0 * dmath::log2(f / hz(m + 2.0))).abs() < 6.0, "{f}");
        let vib = Play {
            vibrato: Some(Vibrato {
                cents: 30.0,
                hz: 5.5,
                wait: 0.0,
            }),
            ..Play::plain(m, 1.2, 0.7)
        };
        let y = sound(&p, &vib, 1.2);
        assert_ne!(y, sound(&p, &Play::plain(m, 1.2, 0.7), 1.2));
        assert!(y.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn a_slide_from_the_note_before_arrives_without_a_new_pluck() {
        let p = PluckedParts::A6;
        let slide = Play {
            slur_from: Some(55.0),
            ..Play::plain(57.0, 1.0, 0.7)
        };
        let x = sound(&p, &slide, 1.0);
        let f = peak_near(&x, hz(57.0), 0.2, 0.9);
        assert!((1200.0 * dmath::log2(f / hz(57.0))).abs() < 6.0, "{f}");
        // it starts softly: no knock, a gentle start
        let plucked = sound(&p, &Play::plain(57.0, 1.0, 0.7), 1.0);
        assert!(db(&x, 0.0, 0.003) < db(&plucked, 0.0, 0.003) - 3.0);
        // a hammer-on changes the pitch at once: in tune from the start
        let hammer = Play {
            technique: Technique::Hammer,
            ..slide
        };
        let h = sound(&p, &hammer, 1.0);
        let f = peak_near(&h, hz(57.0), 0.02, 0.3);
        assert!((1200.0 * dmath::log2(f / hz(57.0))).abs() < 10.0, "{f}");
    }

    #[test]
    fn the_same_note_twice_is_the_same_and_its_repeats_differ() {
        for p in ALL {
            let play = Play::plain(60.0, 0.5, 0.7);
            let a = note(&p, &play, 24_000, 0, SRF);
            assert_eq!(a, note(&p, &play, 24_000, 0, SRF));
            assert_ne!(a, note(&p, &play, 24_000, 1, SRF));
        }
    }

    #[test]
    fn every_technique_and_mute_is_finite_and_heard() {
        let techniques = [
            Technique::Normal,
            Technique::Fingers,
            Technique::Picked,
            Technique::Nails,
            Technique::Thumb,
            Technique::Slap,
            Technique::Pop,
            Technique::Tapped,
            Technique::Hammer,
            Technique::PinchHarmonic,
            Technique::Harmonic(3),
        ];
        for p in ALL {
            for t in techniques {
                for mute in [Mute::Open, Mute::Muted, Mute::Choked, Mute::Clipped] {
                    let play = Play {
                        technique: t,
                        mute,
                        position: Some(0.1),
                        string: Some(1),
                        ..Play::plain(50.0, 0.3, 0.8)
                    };
                    let x = sound(&p, &play, 0.6);
                    assert!(x.iter().all(|v| v.is_finite()), "{p:?} {t:?} {mute:?}");
                    assert!(db(&x, 0.0, 0.1) > -80.0, "{p:?} {t:?} {mute:?}");
                }
            }
        }
    }

    #[test]
    fn notes_past_the_strings_and_odd_asks_stay_finite() {
        for p in ALL {
            for m in [0.0, 12.0, 20.0, 110.0, 127.0, f64::NAN] {
                for vel in [0.0, 1.0, 2.0] {
                    let play = Play {
                        accent: 1.0,
                        string: Some(9),
                        position: Some(f64::NAN),
                        slur_from: Some(f64::INFINITY),
                        ..Play::plain(m, 0.4, vel)
                    };
                    let x = sound(&p, &play, 0.5);
                    assert_eq!(x.len(), 24_000);
                    assert!(x.iter().all(|v| v.is_finite()), "{p:?} at {m}, {vel}");
                    let r = rings_for(&p, &play, 12.0);
                    assert!(r.is_finite() && r > 0.0 && r <= 12.0);
                }
            }
            assert!(note(&p, &Play::plain(60.0, 1.0, 0.7), 0, 0, SRF).is_empty());
        }
    }

    #[test]
    fn a_note_cut_short_ends_quietly() {
        for p in ALL {
            let x = note(&p, &Play::plain(48.0, 2.0, 0.9), (0.3 * SRF) as usize, 0, SRF);
            let last = x[x.len() - 3..].iter().fold(0.0f64, |a, v| a.max(v.abs()));
            let most = x.iter().fold(0.0f64, |a, v| a.max(v.abs()));
            assert!(last < 0.01 * most, "{p:?}: {last} against {most}");
        }
    }

    #[test]
    fn a_higher_string_or_a_pluck_by_the_bridge_changes_the_tone() {
        let p = PluckedParts::A6;
        let m = 57.0;
        let low = sound(
            &p,
            &Play {
                string: Some(0),
                ..Play::plain(m, 1.0, 0.7)
            },
            1.0,
        );
        let own = sound(&p, &Play::plain(m, 1.0, 0.7), 1.0);
        // up the low E at the 17th fret against the open A: the pluck point a bigger part of a shorter string
        assert_ne!(low, own);
        let bridge = sound(
            &p,
            &Play {
                position: Some(0.04),
                ..Play::plain(m, 1.0, 0.7)
            },
            1.0,
        );
        let neck = sound(
            &p,
            &Play {
                position: Some(0.45),
                ..Play::plain(m, 1.0, 0.7)
            },
            1.0,
        );
        assert!(bright(&bridge, 0.0, 0.3, 2000.0) > bright(&neck, 0.0, 0.3, 2000.0) + 3.0);
    }

    #[test]
    fn notes_play_at_other_rates() {
        for fs in [44_100.0, 96_000.0] {
            for p in ALL {
                let x = note(&p, &Play::plain(57.0, 0.5, 0.7), (0.6 * fs) as usize, 0, fs);
                assert!(x.iter().all(|v| v.is_finite()));
                let at48 = note(&p, &Play::plain(57.0, 0.5, 0.7), (0.6 * SRF) as usize, 0, SRF);
                let level = |x: &[f64], fs: f64| {
                    let n = (0.3 * fs) as usize;
                    10.0 * dmath::log10(x[..n].iter().map(|v| v * v).sum::<f64>() / n as f64)
                };
                assert!((level(&x, fs) - level(&at48, SRF)).abs() < 1.5, "{p:?} at {fs}");
            }
        }
    }
}
