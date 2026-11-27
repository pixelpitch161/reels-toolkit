//! Bowed strings: the violin, the viola, the cello, the contrabass and the octobass
//! (`docs/studio/voice-engine-2-instruments.md`; the sketch `bw.rs`, Aedan's picks Vn3, Va3, Vc1, Cb1 and Oc2).
//!
//! A bowed note is a string held by the bow: two delay lines meeting at the bow, one running to the bridge and one to
//! the stopping finger (or the nut), each end a lossy reflection (a gain and a one-pole low-pass, set so the string
//! rings as long as it should at its fundamental and at 4 kHz). The bow holds the string by friction: while the string
//! moves with the bow it sticks, and when the force that would take passes what the friction can hold it slips, the
//! friction falling as it slips faster (a hyperbolic friction curve, the slip kept until it can stick again). That
//! stick and slip is the Helmholtz motion: one corner running round the string, a sawtooth push on the bridge.
//!
//! The bow also turns the string's surface, so it twists: those waves run several times quicker than the string's
//! and die fast, and they soak up the small ripples that would make the bow slip twice a period. Their speed is set
//! per instrument so that their own resonances fall between the note's overtones: on them, the same overtones would
//! drop out of every note and the pattern would follow the melody like a second instrument playing along (Aedan's
//! remark, 2026-11-27). The player's ear hears the time from one slip to the next and moves the finger a hair to
//! bring the note to pitch. When the bow leaves the string, it rings on freely; when the finger lifts, it is damped.
//!
//! The bow's force jitters a little (the rosin), the friction too while it slips. A stopped stroke (martelé) lets its
//! weight off with its speed, since weight left on a bow at rest pins the string like a fret; an accent comes with
//! speed and weight together, a strong one quicker; an open string is bowed a little heavier, since with no finger
//! on it it is less damped and slips twice a period at a weight a stopped string bows cleanly. The string runs at a
//! rate of its own (192 kHz for the violin and the viola, 96 kHz for the cello, the heard rate for the basses, as
//! near as a whole multiple of the sample rate goes) and is brought down with a windowed-sinc decimator.
//!
//! Plucked and struck notes (pizzicato, a snap, a left-hand pizzicato, the basses' slap, col legno battuto) are the
//! same string as its own modes, pulled aside or struck and let go, losing energy to the air, to its own damping, to
//! the finger and to the body where the body moves most readily.
//!
//! The force on the bridge goes through the instrument's body ([`body`]), a mute (con sordino) on the bridge first if
//! one is asked for.

use bf_core::dmath;

use super::body::{self, BodyKind};
use super::eq::{self, Band};
use super::play::{Mute, Play, Technique};
use crate::dsp::{self, FilterKind, butter_sos};

/// A bowed string instrument's make-up (the sketch's settings, by its names in brackets).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BowedParts {
    /// Its open strings from the lowest: (MIDI, tension in newtons).
    pub strings: &'static [(f64, f64)],
    /// The open string's length, m.
    pub length: f64,
    /// The open string's stiffness: how much sharper each overtone of a plucked note runs (`b_open`).
    pub stiffness: f64,
    /// Where it is bowed, as a part of the sounding length from the bridge.
    pub beta: f64,
    /// The bow's speed at full strength, m/s (`vb`).
    pub speed: f64,
    /// The bow's weight as a part of the most it can be and still give a clean note (Schelleng's); an open string's
    /// times `open_press`.
    pub press: f64,
    pub open_press: f64,
    /// The friction: holding (`mus`), slipping fast (`mud`), and how quickly it falls with the slip's speed, m/s.
    pub mu_s: f64,
    pub mu_d: f64,
    pub v0: f64,
    /// The rosin: the bow's force jitters by this part, this quickly (Hz); the friction by `slip_noise` while slipping.
    pub noise: f64,
    pub noise_hz: f64,
    pub slip_noise: f64,
    /// How long an open string rings at low frequencies, s, and where a plucked one's losses start to rise, Hz; a
    /// stopped string rings `finger` as long, its losses rising `finger_hz` as soon (`fc`, `fingerc`).
    pub t60: f64,
    pub loss_hz: f64,
    pub finger: f64,
    pub finger_hz: f64,
    /// How long the bowed string's overtones at 4 kHz ring, s, and the bridge end's share of its losses (the rest at
    /// the finger).
    pub t60_4k: f64,
    pub bridge_share: f64,
    /// An open string's ring once the bow leaves it, seconds to fall 60 dB at the note (a stopped one's times
    /// `finger`), and its overtones' at 4 kHz as a part of that.
    pub ring: f64,
    pub ring_hi: f64,
    /// The string's twist: its waves' impedance against the string's (`tors_z`), how much quicker they run, what one
    /// keeps at each end, and how much its top is softened there.
    pub twist_z: f64,
    pub twist_speed: f64,
    pub twist_keep: f64,
    pub twist_soft: f64,
    /// How quickly the player's ear brings the note to pitch, s.
    pub intone: f64,
    /// How long the bow takes to get going, and to let off at the end, s (`att`, `rel`).
    pub attack: f64,
    pub release: f64,
    /// An accent: its extra weight at full accent (`accent_k`), how quickly that comes off, s (`bite_t`), whether the
    /// bow's speed takes it too (`bite_v`), and how much quicker a strong accent's bow gets going (`att_acc`).
    pub accent: f64,
    pub bite: f64,
    pub bite_speed: bool,
    pub quick: f64,
    /// A stopped stroke's weight left on the string as the bow comes to rest (`stop_k`).
    pub stop_weight: f64,
    /// The player's own vibrato: cents, Hz, and how long into a note it comes in, s.
    pub vib_cents: f64,
    pub vib_hz: f64,
    pub vib_wait: f64,
    /// The string's own rate, Hz: the sample rate's nearest whole multiple (`os`).
    pub rate: f64,
    /// A plucked note's highest overtone, Hz (`fmax`).
    pub top: f64,
    /// A pizzicato: where it is plucked, as a part of the sounding length from the bridge, how far it is pulled (a
    /// part of the sketch's), and the finger's width, m.
    pub pizz_at: f64,
    pub pizz_level: f64,
    pub pizz_width: f64,
    /// The body's pull on a plucked string's modes: its strongest modes `(Hz, Q, share)`, and the most it drains, per
    /// second.
    pub drain_modes: &'static [(f64, f64, f64)],
    pub drain: f64,
    /// The body.
    pub body: BodyKind,
    /// Where the bridge carries the tone most readily (the bridge hill), Hz: a mute on the bridge takes it away.
    pub bridge_hz: f64,
    /// An orchestra's bass (the contrabass and the octobass): a plain note under half a second is plucked, as
    /// today's parts were written for, and a slap is a slap (else a snap).
    pub bass: bool,
    /// The note's level, from the force on the bridge.
    pub gain: f64,
}

/// The violin's open strings: G, D, A, E.
const VIOLIN: [(f64, f64); 4] = [(55.0, 44.0), (62.0, 40.0), (69.0, 52.0), (76.0, 75.0)];
/// The viola's: C, G, D, A.
const VIOLA: [(f64, f64); 4] = [(48.0, 40.0), (55.0, 38.0), (62.0, 36.0), (69.0, 48.0)];
/// The cello's: C, G, D, A.
const CELLO: [(f64, f64); 4] = [(36.0, 135.0), (43.0, 130.0), (50.0, 140.0), (57.0, 170.0)];
/// The contrabass's: E, A, D, G.
const CONTRABASS: [(f64, f64); 4] = [(28.0, 280.0), (33.0, 290.0), (38.0, 300.0), (43.0, 300.0)];
/// The octobass's: C, G, C (the sketch's tuning, a fifth and a fourth).
const OCTOBASS: [(f64, f64); 3] = [(22.0, 450.0), (29.0, 450.0), (36.0, 450.0)];

/// The bodies' pull on a plucked string: the violin's air and corpus modes and its bridge hill...
const VIOLIN_DRAIN: [(f64, f64, f64); 5] = [
    (275.0, 12.0, 1.0),
    (405.0, 30.0, 0.6),
    (470.0, 25.0, 0.9),
    (545.0, 25.0, 1.0),
    (2400.0, 3.0, 0.5),
];
/// ... the viola's ...
const VIOLA_DRAIN: [(f64, f64, f64); 5] = [
    (230.0, 12.0, 1.0),
    (340.0, 30.0, 0.6),
    (390.0, 25.0, 0.9),
    (450.0, 25.0, 1.0),
    (2000.0, 3.0, 0.5),
];
/// ... the cello's ...
const CELLO_DRAIN: [(f64, f64, f64); 4] = [
    (100.0, 12.0, 1.0),
    (145.0, 25.0, 0.9),
    (190.0, 25.0, 1.0),
    (1000.0, 2.5, 0.5),
];
/// ... the contrabass's ...
const CONTRABASS_DRAIN: [(f64, f64, f64); 4] = [
    (60.0, 10.0, 1.0),
    (100.0, 20.0, 0.9),
    (130.0, 20.0, 0.8),
    (500.0, 2.0, 0.5),
];
/// ... and the octobass's.
const OCTOBASS_DRAIN: [(f64, f64, f64); 4] = [
    (40.0, 10.0, 1.0),
    (70.0, 20.0, 0.9),
    (95.0, 20.0, 0.8),
    (350.0, 2.0, 0.5),
];

impl BowedParts {
    /// The violin Aedan picked (Vn3): warm, bowed a little further from the bridge with a lighter weight, a wide slow
    /// vibrato, its top softened.
    pub const VN3: BowedParts = BowedParts {
        strings: &VIOLIN,
        length: 0.328,
        stiffness: 4e-5,
        beta: 0.12,
        speed: 0.45,
        press: 0.55,
        open_press: 1.15,
        mu_s: 0.8,
        mu_d: 0.35,
        v0: 0.1,
        noise: 0.04,
        noise_hz: 800.0,
        slip_noise: 0.02,
        t60: 3.0,
        loss_hz: 1200.0,
        finger: 0.4,
        finger_hz: 0.6,
        t60_4k: 0.15,
        bridge_share: 0.35,
        ring: 0.5,
        ring_hi: 0.45,
        twist_z: 2.5,
        twist_speed: 6.2,
        twist_keep: 0.85,
        twist_soft: 0.3,
        intone: 0.06,
        attack: 0.05,
        release: 0.05,
        accent: 0.8,
        bite: 0.02,
        bite_speed: true,
        quick: 0.7,
        stop_weight: 0.0,
        vib_cents: 24.0,
        vib_hz: 5.0,
        vib_wait: 0.18,
        rate: 192_000.0,
        top: 14_000.0,
        pizz_at: 0.22,
        pizz_level: 1.0,
        pizz_width: 0.012,
        drain_modes: &VIOLIN_DRAIN,
        drain: 6.0,
        body: BodyKind::ViolinWarm,
        bridge_hz: 2500.0,
        bass: false,
        gain: 0.2,
    };
    /// The viola (Va3): big and warm, as the violin's bowing, its accents lighter and shorter.
    pub const VA3: BowedParts = BowedParts {
        strings: &VIOLA,
        length: 0.375,
        speed: 0.42,
        t60_4k: 0.12,
        ring: 0.6,
        accent: 0.4,
        bite: 0.012,
        vib_cents: 22.0,
        vib_hz: 5.0,
        drain_modes: &VIOLA_DRAIN,
        body: BodyKind::ViolaBig,
        bridge_hz: 2000.0,
        twist_speed: 6.4,
        gain: 0.13,
        ..BowedParts::VN3
    };
    /// The cello (Vc1): a concert cello.
    pub const VC1: BowedParts = BowedParts {
        strings: &CELLO,
        length: 0.69,
        stiffness: 3e-5,
        beta: 0.09,
        speed: 0.38,
        press: 0.6,
        t60: 4.0,
        loss_hz: 900.0,
        t60_4k: 0.12,
        ring: 0.8,
        twist_speed: 8.0,
        accent: 0.4,
        bite: 0.02,
        vib_cents: 16.0,
        vib_hz: 5.2,
        rate: 96_000.0,
        top: 12_000.0,
        pizz_at: 0.2,
        pizz_width: 0.015,
        drain_modes: &CELLO_DRAIN,
        body: BodyKind::Cello,
        bridge_hz: 1200.0,
        gain: 0.027,
        ..BowedParts::VN3
    };
    /// The contrabass (Cb1): an orchestral double bass.
    pub const CB1: BowedParts = BowedParts {
        strings: &CONTRABASS,
        length: 1.05,
        stiffness: 2e-5,
        beta: 0.1,
        speed: 0.32,
        press: 0.56,
        t60: 6.0,
        loss_hz: 600.0,
        t60_4k: 0.06,
        ring: 1.2,
        twist_speed: 7.0,
        attack: 0.07,
        release: 0.07,
        accent: 1.6,
        bite: 0.035,
        bite_speed: false,
        quick: 0.4,
        vib_cents: 14.0,
        vib_hz: 5.0,
        rate: 48_000.0,
        top: 9000.0,
        pizz_at: 0.18,
        pizz_width: 0.02,
        drain_modes: &CONTRABASS_DRAIN,
        body: BodyKind::Contrabass,
        bridge_hz: 700.0,
        bass: true,
        gain: 0.018,
        ..BowedParts::VC1
    };
    /// The octobass (Oc2): bowed nearer the bridge and pressed harder, a growl in the low middle; not bowed heavier
    /// on an open string (pressed hard already, more creaks).
    pub const OC2: BowedParts = BowedParts {
        strings: &OCTOBASS,
        length: 2.1,
        stiffness: 1.5e-5,
        beta: 0.07,
        speed: 0.3,
        press: 0.58,
        open_press: 1.0,
        t60: 8.0,
        loss_hz: 400.0,
        t60_4k: 0.045,
        ring: 1.5,
        twist_speed: 8.5,
        attack: 0.1,
        release: 0.09,
        vib_cents: 12.0,
        vib_hz: 4.6,
        top: 7000.0,
        pizz_at: 0.16,
        pizz_width: 0.03,
        drain_modes: &OCTOBASS_DRAIN,
        body: BodyKind::OctobassGrowl,
        bridge_hz: 500.0,
        gain: 0.0065,
        ..BowedParts::CB1
    };
}

/// How a bowed note's bow moves.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Stroke {
    /// Drawn: a plain stroke (détaché), a stopped one (martelé), slurred or tied.
    Drawn,
    /// Dropped and bouncing off (spiccato).
    Bounced,
    /// Back and forth as fast as it goes (tremolo).
    Tremolo,
    /// The stick's wood drawn across (col legno tratto).
    Wood,
    /// Touched at a part of the string (a harmonic), its number.
    Touched(f64),
}

/// How a plucked or struck note is set going.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Pluck {
    /// Pizzicato.
    Finger,
    /// A snap pizzicato: pulled up and let slap onto the fingerboard.
    Snap,
    /// A left-hand pizzicato: the stopping hand's finger, lighter and thinner.
    LeftHand,
    /// The basses' slap: pulled away and let slap back, the hand slapping the strings.
    Slap,
    /// Col legno battuto: struck with the stick.
    Stick,
}

/// A note as the string plays it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Way {
    Bowed(Stroke),
    Plucked(Pluck),
}

/// How `play` is played on `p`.
fn way(p: &BowedParts, play: &Play) -> Way {
    match play.technique {
        Technique::Normal => {
            if p.bass && play.dur < SHORT && !play.tie && play.slur_from.is_none() {
                Way::Plucked(Pluck::Finger)
            } else {
                Way::Bowed(Stroke::Drawn)
            }
        }
        Technique::Arco | Technique::Martele => Way::Bowed(Stroke::Drawn),
        Technique::Spiccato => Way::Bowed(Stroke::Bounced),
        Technique::Tremolo => Way::Bowed(Stroke::Tremolo),
        Technique::ColLegnoTratto => Way::Bowed(Stroke::Wood),
        Technique::Harmonic(n) => Way::Bowed(Stroke::Touched(f64::from(n.clamp(2, 6)))),
        Technique::ColLegno => Way::Plucked(Pluck::Stick),
        Technique::SnapPizz | Technique::Pop => Way::Plucked(Pluck::Snap),
        Technique::Slap => Way::Plucked(if p.bass { Pluck::Slap } else { Pluck::Snap }),
        Technique::LeftHandPizz | Technique::Tapped | Technique::Hammer => Way::Plucked(Pluck::LeftHand),
        // the plucked string's own ways are a pizzicato on a bowed one
        Technique::Pizzicato
        | Technique::Plucked
        | Technique::Fingers
        | Technique::Picked
        | Technique::Nails
        | Technique::Thumb
        | Technique::PinchHarmonic => Way::Plucked(Pluck::Finger),
    }
}

/// A plain note under this long is plucked on an orchestra's bass, s.
pub const SHORT: f64 = 0.5;
/// A martelé's accent at the least, and a clipped note's longest stroke, s.
const MARTELE: f64 = 0.9;
const CLIPPED: f64 = 0.06;
/// A slurred note's bow is already moving: it starts this long before the note, at the note before's pitch, s.
const SLUR_PRE: f64 = 0.2;
/// How quickly a slur's finger moves to the new note, s (a portamento: the bend's `over`).
const SLUR_GLIDE: f64 = 0.006;
/// A slurred note comes in as the one tied to it goes out, over this long (equal power), s.
const JOIN: f64 = 0.012;
/// The bow leaves the string this long after the note; the free ring's losses take over in this long, s.
const LEAVES: f64 = 0.01;
const TO_FREE: f64 = 0.02;
/// The stopping finger lifts this long after the note, s, and the string then rings this long (T60) at the note and
/// at 4 kHz, in a share of the losses.
const LIFTS: f64 = 0.25;
const LIFTED: (f64, f64, f64) = (0.05, 0.01, 0.65);
/// A choked note's finger (and hand) damps the string at once.
const CHOKED: f64 = 0.01;
/// The longest a bowed note rings past its end, s (the sketch's).
const TAIL: f64 = 1.5;
/// A spiccato's bounce: its time on the string at full and at no strength, s, and its peak weight.
const BOUNCE: (f64, f64) = (0.04, 0.07);
const BOUNCE_WEIGHT: f64 = 1.6;
/// A tremolo's strokes a second, and the least of the bow's weight left at a turn.
const TREMOLO_HZ: f64 = 14.0;
const TREMOLO_WEIGHT: f64 = 0.05;
/// What is under this part of the note is taken out of a tremolo's push on the bridge.
const TREMOLO_UNDER: f64 = 0.6;
/// Col legno tratto: the stick's weight (a part of the bow's) and its friction.
const WOOD_PRESS: f64 = 0.5;
/// How much lighter a bow nearer the bridge than usual goes: the weight times (bowing point / usual) to this power.
const PONT_LIGHT: f64 = 0.75;
/// How much slower a bow nearer the bridge than usual goes: the speed times (bowing point / usual) to this power.
const PONT_SLOW: f64 = 0.5;
/// How much slower a bow nearer the fingerboard than usual goes: the speed over (bowing point / usual) to this power.
const TASTO_SLOW: f64 = 0.5;
const WOOD_MU: (f64, f64) = (0.45, 0.3);
/// A harmonic's touching finger: its resistance, in the string's own (2 z).
const TOUCH: f64 = 1.5;
/// The player's ear keeps the note within this part of its pitch, and listens only to slips this near a period.
const EAR_RANGE: f64 = 0.04;
const EAR_NEAR: f64 = 0.05;
/// The bow's speed and the vibrato's depth wander slowly (Hz), by this part.
const WANDER_HZ: f64 = 3.0;
const WANDER_SPEED: f64 = 0.02;
const WANDER_VIB: f64 = 0.15;
/// The vibrato's rate differs a little from note to note, and comes in over this long, s.
const VIB_RATE_SPREAD: f64 = 0.04;
const VIB_IN: f64 = 0.35;
/// The bridge's steady sideways push is not heard: high-passed at this, Hz.
const DC_HZ: f64 = 12.0;
/// The decimator: taps per step of the string's rate, and its edge as a part of the heard rate's half.
const DOWN_TAPS: usize = 32;
const DOWN_EDGE: f64 = 0.45;
/// A pizzicato's pull at full strength, as a part of the open string's length; a snap's and a slap's times that.
const PULL: f64 = 0.004;
const SNAP_PULL: f64 = 2.2;
const SLAP_PULL: f64 = 1.8;
/// A left-hand pizzicato: its pull, where it is plucked (near the stopping finger), and the fingertip's width, as
/// parts of a pizzicato's.
const LEFT_PULL: f64 = 0.4;
const LEFT_AT: f64 = 0.1;
const LEFT_WIDTH: f64 = 0.7;
/// Col legno battuto: the stick's blow (in the string's own impedance, at full strength) and how long it lasts, s.
const STICK_BLOW: f64 = 7.0;
const STICK_TIME: f64 = 0.0012;
/// The clicks: a snap's and the stick's level (a part of the note's peak), and the slap's clack and its band, Hz.
const SNAP_CLICK: f64 = 0.35;
const STICK_CLICK: f64 = 0.5;
const SLAP_CLACK: f64 = 0.6;
const SLAP_CLACK_HZ: f64 = 2500.0;
const CLICK_TIME: f64 = 0.004;
const CLICK_DECAY: f64 = 0.0007;
const SLAP_DECAY: f64 = 0.0025;
const SLAP_TIME: f64 = 0.012;
/// A plucked note is damped this much once let go (per second, more for its overtones), choked this much.
const LET_GO: f64 = 30.0;
const CHOKE: f64 = 120.0;
/// An accent on a plucked note pulls it harder, by this part of a full velocity.
const PLUCK_ACCENT: f64 = 0.12;
/// The most modes a plucked note has, and how near the heard rate's half they may go.
const MOST_MODES: usize = 400;
const NYQUIST_SHARE: f64 = 0.45;
/// The plucked modes are set again every this many samples (a bend, a vibrato, the let-go).
const BLOCK: usize = 64;
/// A mute on the bridge: how much it takes from the bridge hill and from what is above it (dB).
const SORDINO_HILL: f64 = -4.0;
const SORDINO_TOP: f64 = -9.0;
/// The note's last 20 ms faded (less for a very short note), so a note cut short ends without a click.
const END_FADE: f64 = 0.02;

fn hz(m: f64) -> f64 {
    440.0 * dmath::pow(2.0, (m - 69.0) / 12.0)
}

/// A smooth step from 0 to 1 over `0..1`.
fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A strength asked for, 0 to 1 (nothing for one that isn't a number).
fn unit(x: f64) -> f64 {
    if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) }
}

/// The note's pitch, held to a sensible range.
fn key_of(play: &Play) -> f64 {
    if play.midi.is_finite() {
        play.midi.clamp(0.0, 127.0)
    } else {
        60.0
    }
}

/// The bow's and the rosin's random draws (the sketch's xorshift and Box-Muller, so a note is the sketch's own).
struct Rosin(u64);

impl Rosin {
    fn new(seed: u64) -> Rosin {
        Rosin(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn uniform(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn gauss(&mut self) -> f64 {
        let u1 = self.uniform().max(1e-12);
        let u2 = self.uniform();
        (-2.0 * dmath::ln(u1)).sqrt() * dmath::cos(dmath::TAU * u2)
    }
}

/// A delay line read anywhere between samples (third-order Lagrange).
struct Line {
    buf: Vec<f64>,
    w: usize,
}

impl Line {
    fn new(n: usize) -> Line {
        Line {
            buf: vec![0.0; n.next_power_of_two()],
            w: 0,
        }
    }

    fn push(&mut self, x: f64) {
        self.w = (self.w + 1) & (self.buf.len() - 1);
        self.buf[self.w] = x;
    }

    /// The value `d` samples ago (at least 1): the newest pushed is 0 samples ago.
    fn read(&self, d: f64) -> f64 {
        self.read_tap(&Tap::at(d))
    }

    /// The value a [`Tap`]'s samples ago.
    fn read_tap(&self, t: &Tap) -> f64 {
        let m = self.buf.len() - 1;
        let at = |k: usize| self.buf[self.w.wrapping_add(self.buf.len()).wrapping_sub(k) & m];
        let i = t.i;
        let (xm, x0, x1, x2) = (at(i - 1), at(i), at(i.wrapping_add(1)), at(i.wrapping_add(2)));
        let [hm, h0, h1, h2] = t.h;
        hm * xm + h0 * x0 + h1 * x1 + h2 * x2
    }
}

/// Where a [`Line`] is read: the whole samples back and the four Lagrange weights, worked out once for a read that
/// stays put.
struct Tap {
    i: usize,
    h: [f64; 4],
}

impl Tap {
    /// `d` samples back (at least 1).
    fn at(d: f64) -> Tap {
        let d = d.max(1.0);
        // (d is at least 1, so the cast is its floor)
        let i = d as usize;
        let mu = d - i as f64;
        Tap {
            i,
            h: [
                -mu * (mu - 1.0) * (mu - 2.0) / 6.0,
                (mu + 1.0) * (mu - 1.0) * (mu - 2.0) / 2.0,
                -(mu + 1.0) * mu * (mu - 2.0) / 2.0,
                (mu + 1.0) * mu * (mu - 1.0) / 6.0,
            ],
        }
    }
}

/// A reflection's loss: a gain `g` and a one-pole low-pass with pole `a`.
#[derive(Clone, Copy, Debug)]
struct Loss {
    g: f64,
    a: f64,
    y: f64,
}

impl Loss {
    fn run(&mut self, x: f64) -> f64 {
        self.y = self.g * (1.0 - self.a) * x + self.a * self.y;
        self.y
    }

    /// Its phase delay at `w` (radians a sample), in samples.
    fn delay_at(&self, w: f64) -> f64 {
        let a = self.a;
        dmath::atan2(a * dmath::sin(w), 1.0 - a * dmath::cos(w)) / w
    }

    fn gain_at(&self, w: f64) -> f64 {
        let a = self.a;
        self.g * (1.0 - a) / (1.0 - 2.0 * a * dmath::cos(w) + a * a).sqrt()
    }

    /// The loss of a reflection that takes `share` of a round trip's, so that the string rings `t60_lo` seconds at
    /// `f0` and `t60_hi` at `fh` (Hz, at the string's rate `fs`).
    fn for_ring(f0: f64, t60_lo: f64, t60_hi: f64, fh: f64, fs: f64, share: f64) -> Loss {
        let g_lo = dmath::pow(10.0, -3.0 / (t60_lo * f0) * share);
        let g_hi = dmath::pow(10.0, -3.0 / (t60_hi.max(1e-3) * f0) * share);
        let want = (g_hi / g_lo).min(0.999_999);
        let w = dmath::TAU * fh / fs;
        let w0 = dmath::TAU * f0 / fs;
        let (mut lo, mut hi) = (0.0f64, 0.99f64);
        for _ in 0..60 {
            let a = 0.5 * (lo + hi);
            let l = Loss { g: 1.0, a, y: 0.0 };
            // normalised so its gain at f0 is 1
            let k = l.gain_at(w) / l.gain_at(w0);
            if k > want {
                lo = a;
            } else {
                hi = a;
            }
        }
        let a = 0.5 * (lo + hi);
        let l = Loss { g: 1.0, a, y: 0.0 };
        // the gain at f0 is the ring time's; under it never over 1 (it would grow)
        Loss {
            g: (g_lo / l.gain_at(w0)).min(dmath::pow(0.9995, share)),
            a,
            y: 0.0,
        }
    }
}

/// The string a note is played on: the asked one if it is tuned at or under the note, else the highest that is
/// (the lowest for a note under them all).
fn string_of(p: &BowedParts, play: &Play, m: f64) -> usize {
    let asked = play
        .string
        .map(usize::from)
        .filter(|&s| p.strings.get(s).is_some_and(|&(o, _)| o <= m + 1e-9));
    asked.unwrap_or_else(|| p.strings.iter().rposition(|&(o, _)| o <= m + 1e-9).unwrap_or(0))
}

/// What a note's string is: its open pitch and tension, its impedance, its mass a metre, whether a finger stops it.
#[derive(Clone, Copy, Debug)]
struct Wire {
    open: f64,
    tension: f64,
    z: f64,
    rho: f64,
    stopped: bool,
}

fn wire(p: &BowedParts, play: &Play, m: f64) -> Wire {
    let (open, tension) = p.strings.get(string_of(p, play, m)).copied().unwrap_or((m, 100.0));
    let f_open = hz(open);
    let len = 2.0 * p.length * f_open;
    let rho = tension / (len * len);
    Wire {
        open,
        tension,
        z: (tension * rho).sqrt(),
        rho,
        stopped: m > open + 0.01,
    }
}

/// The note's held length, as played: a clipped note as short as a stroke goes.
fn held(play: &Play) -> f64 {
    let d = play.dur.max(0.0);
    if play.mute == Mute::Clipped { d.min(CLIPPED) } else { d }
}

/// A pitch's bend and asked-for vibrato, cents from the note at `t` seconds (the plucked string's).
fn asked_cents(play: &Play, t: f64) -> f64 {
    let ease = |u: f64| 0.5 - 0.5 * dmath::cos(dmath::PI * u.clamp(0.0, 1.0));
    let mut c = 0.0;
    if let Some(b) = play.bend
        && b.cents.is_finite()
        && b.at.is_finite()
        && b.over.is_finite()
    {
        c += b.cents * ease((t - b.at) / b.over.max(1e-3));
    }
    c.clamp(-2400.0, 2400.0)
}

/// A slur's glide: how long the finger takes to the new note (a bend of 0 cents with it is a portamento over the
/// bend's `over`).
fn glide(play: &Play) -> f64 {
    match play.bend {
        Some(b) if b.cents == 0.0 && b.over.is_finite() && b.over > 0.0 => b.over,
        _ => SLUR_GLIDE,
    }
}

/// The note's vibrato: as asked, or the player's own (cents, Hz, wait).
fn vibrato(p: &BowedParts, play: &Play) -> (f64, f64, f64) {
    match play.vibrato {
        Some(v) if v.cents.is_finite() && v.hz.is_finite() && v.wait.is_finite() => (v.cents, v.hz, v.wait.max(0.0)),
        _ => (p.vib_cents, p.vib_hz, p.vib_wait),
    }
}

/// A swell's (or the expression pedal's) part at `t` seconds.
fn swelled(s: Option<super::play::Swell>, t: f64) -> f64 {
    match s {
        Some(s) if s.from.is_finite() && s.to.is_finite() && s.over.is_finite() => {
            let (a, b) = (s.from.clamp(0.0, 1.5), s.to.clamp(0.0, 1.5));
            a + (b - a) * smooth(t / s.over.max(1e-4))
        }
        _ => 1.0,
    }
}

/// How many steps of the string's rate to one of the heard rate `fs`.
fn steps(p: &BowedParts, fs: f64) -> usize {
    ((p.rate / fs).round() as usize).max(1)
}

/// The seed of repeat `variant` of a note at `m` (the sketch's: note `variant` of a riff).
fn seed(variant: u64, m: f64) -> u64 {
    variant.wrapping_mul(7919).wrapping_add(17) ^ ((m * 1000.0) as u64)
}

/// How long a bowed note rings past its held length, s: the bow's release, then the free ring until the finger lifts
/// (a stopped string) or until it has died away (an open one), at most [`TAIL`].
fn bowed_tail(p: &BowedParts, play: &Play, w: &Wire) -> f64 {
    if play.tie {
        return JOIN + 0.005;
    }
    if play.mute == Mute::Choked {
        return CHOKED + 0.1;
    }
    let tail = if w.stopped {
        LIFTS + 1.4 * LIFTED.0 + 0.05
    } else {
        LEAVES + 1.4 * p.ring + 0.05
    };
    tail.min(TAIL)
}

/// How long the note `play` rings on `p` with no length asked for, seconds, at most `longest`.
pub fn rings_for(p: &BowedParts, play: &Play, longest: f64) -> f64 {
    let m = key_of(play);
    let w = wire(p, play, m);
    let dur = held(play);
    let ring = match way(p, play) {
        Way::Bowed(_) => dur + bowed_tail(p, play, &w),
        Way::Plucked(_) => {
            let t60 = if w.stopped { p.t60 * p.finger } else { p.t60 };
            let damped = if play.mute == Mute::Choked { CHOKE } else { LET_GO };
            dur.min(1.4 * t60) + 9.5 / damped + 0.05
        }
    };
    ring.clamp(0.05, longest.max(0.05))
}

/// A bowed note's force on the bridge at the string's rate, `n` samples from the note's start.
fn bowed(p: &BowedParts, play: &Play, stroke: Stroke, n: usize, variant: u64, fs: f64) -> Vec<f64> {
    let m = key_of(play);
    let w = wire(p, play, m);
    let (z, stopped) = (w.z, w.stopped);
    let y = 0.5 / z;
    let f0 = hz(m);
    let dt = 1.0 / fs;
    let mut rng = Rosin::new(seed(variant, m));
    let dur = held(play);
    let martele = play.technique == Technique::Martele;
    let acc = if martele {
        unit(play.accent).max(MARTELE)
    } else {
        unit(play.accent)
    };
    let stop = martele || matches!(play.mute, Mute::Clipped | Mute::Choked);
    let tie = play.tie && stroke != Stroke::Bounced;
    let slur = play.slur_from.filter(|s| s.is_finite()).map(|s| s.clamp(0.0, 127.0));
    let beta = play
        .position
        .filter(|x| x.is_finite())
        .map_or(p.beta, |x| x.clamp(0.02, 0.3));
    let (t60, ring) = if stopped {
        (p.t60 * p.finger, p.ring * p.finger)
    } else {
        (p.t60, p.ring)
    };
    let fh = 4000.0f64.min(0.4 * fs);
    let t60_hi = p.t60_4k.min(t60 * 0.9);
    let share = p.bridge_share;
    let mut lb = Loss::for_ring(f0, t60, t60_hi, fh, fs, share);
    let mut ln = Loss::for_ring(f0, t60, t60_hi, fh, fs, 1.0 - share);
    let mut ln_off = Loss::for_ring(f0, LIFTED.0, LIFTED.1, fh, fs, LIFTED.2);
    let (lb_bow, ln_bow) = (lb, ln);
    let lb_free = Loss::for_ring(f0, ring, ring * p.ring_hi, fh, fs, share);
    let ln_free = Loss::for_ring(f0, ring, ring * p.ring_hi, fh, fs, 1.0 - share);
    ln_off.y = 0.0;
    // the bow sits a fixed way from the bridge: beta of the note's own length
    let w0 = dmath::TAU * f0 / fs;
    let (mut dlb, mut dln) = (lb.delay_at(w0), ln.delay_at(w0));
    let db = (beta * fs / f0 / 2.0 - dlb / 2.0).max(2.0);
    // long enough for the lowest pitch the note reaches (a slur from under it, a bend down, the ear, the vibrato)
    let lowest = m.min(slur.unwrap_or(m)) - play.bend.map_or(0.0, |b| b.cents.abs().min(2400.0) / 100.0) - 2.0;
    let longest = (fs / hz(lowest)).ceil() as usize;
    let mut bl = Line::new(2 * longest + 64);
    let mut nl = Line::new(2 * longest + 64);
    let vel = unit(play.vel);
    // bowed nearer the bridge than usual (sul ponticello) a player lightens the bow, as the weight that holds the
    // string there narrows to a sliver (too much and it sticks fast, a hair less and it sings glassy, the fundamental
    // gone thin), and slows it a little (the string swings further for the same bow there); nearer the fingerboard
    // (sul tasto) the bow goes slower
    let (near, over) = ((beta / p.beta).min(1.0), (beta / p.beta).max(1.0));
    let v_full = p.speed * (0.25 + 0.75 * vel) * dmath::pow(near, PONT_SLOW) / dmath::pow(over, TASTO_SLOW);
    let wood = stroke == Stroke::Wood;
    let press = p.press
        * dmath::pow(near, PONT_LIGHT)
        * if wood { WOOD_PRESS } else { 1.0 }
        * if stopped { 1.0 } else { p.open_press };
    let (mus, mud) = if wood { WOOD_MU } else { (p.mu_s, p.mu_d) };
    let touched = match stroke {
        Stroke::Touched(h) => Some(h),
        _ => None,
    };
    // a harmonic sounds the string's h-th mode: the bow sits h times further along each of its loops, so it holds
    // the string with the weight for that bowing point
    let beta_heard = touched.map_or(beta, |h| (beta * h).min(0.5));
    let fb_full = press * 2.0 * z * v_full / (beta_heard * (mus - mud));
    let pre = if slur.is_some() { SLUR_PRE } else { 0.0 };
    let start = (pre * fs) as usize;
    let ns = start + n;
    let mut out = vec![0.0; ns];
    let (vib_c, vib_r, vib_d) = vibrato(p, play);
    let vib_r = vib_r * (1.0 + VIB_RATE_SPREAD * rng.gauss());
    let mut ph = rng.uniform() * dmath::TAU;
    let mut nz = 0.0;
    let a_nz = dmath::exp(-dmath::TAU * p.noise_hz * dt);
    let nz_g = (1.0 - a_nz * a_nz).sqrt();
    let mut wander = 0.0;
    let a_w = dmath::exp(-dmath::TAU * WANDER_HZ * dt);
    let wander_g = (1.0 - a_w * a_w).sqrt();
    // an accent's bow starts quicker; only a strong one (a martelé, accent 0.6 and up) takes the full `quick`, a
    // light one 0.4 (a quick start on a light accent can catch the string two or three times a period)
    let acc_q = 0.4 + (p.quick - 0.4) * smooth((acc - 0.5) / 0.4);
    let att = p.attack * (1.0 - acc_q * acc) * (1.3 - 0.5 * vel);
    let rel = p.release;
    let glide = glide(play);
    let mut tl = Line::new(if touched.is_some() { 2 * longest + 64 } else { 4 });
    let mut hret = Line::new(if touched.is_some() { 2 * longest + 64 } else { 4 });
    let r_touch = 2.0 * z * TOUCH;
    let mut tune = 1.0f64;
    let mut last_slip: Option<usize> = None;
    let yr = if p.twist_z > 0.0 { 0.5 / (z * p.twist_z) } else { 0.0 };
    let yt = y + yr;
    let mut trb = Line::new(longest + 64);
    let mut trn = Line::new(longest + 64);
    let mut tlb = Loss {
        g: 1.0,
        a: p.twist_soft,
        y: 0.0,
    };
    let mut tln = tlb;
    let mut slipping = false;
    let let_go = if play.mute == Mute::Choked { CHOKED } else { LIFTS };
    // the two reads at the bow's fixed place
    let (at_bow, at_bridge) = (Tap::at(2.0 * db - 1.0), Tap::at(db));
    let tc = BOUNCE.0 + (BOUNCE.1 - BOUNCE.0) * (1.0 - vel);
    for (s, o) in out.iter_mut().enumerate() {
        let tt = s as f64 * dt - pre;
        // the pitch
        let mut cents = asked_cents(play, tt);
        if let Some(from) = slur {
            cents += (from - m) * 100.0 * (1.0 - smooth(tt / glide.max(1e-4)));
        }
        let vib_on = smooth((tt - vib_d) / VIB_IN);
        ph += dmath::TAU * vib_r * dt;
        // (before the vibrato comes in it adds nothing, and a note at its own pitch needs no power of two)
        if vib_on > 0.0 {
            cents += vib_c * vib_on * (1.0 + WANDER_VIB * wander) * dmath::sin(ph);
        }
        let per = if cents == 0.0 {
            fs / f0
        } else {
            fs / (f0 * dmath::pow(2.0, cents / 1200.0))
        };
        if !tie && tt > dur && tt < dur + TO_FREE + 0.01 {
            let k = smooth((tt - dur) / TO_FREE);
            lb.g = lb_bow.g + (lb_free.g - lb_bow.g) * k;
            lb.a = lb_bow.a + (lb_free.a - lb_bow.a) * k;
            ln.g = ln_bow.g + (ln_free.g - ln_bow.g) * k;
            ln.a = ln_bow.a + (ln_free.a - ln_bow.a) * k;
            dlb = lb.delay_at(w0);
            dln = ln.delay_at(w0);
        }
        let dn_total = (per * tune / 2.0 - db - dlb / 2.0 - dln / 2.0).max(2.0);
        // once the bow has left the string for good it no longer pulls on it, nor on its twist (whose waves reach
        // the sound only through the bow): the string rings on alone
        let gone = match stroke {
            Stroke::Bounced => tt >= tc,
            _ => !tie && tt > dur + LEAVES,
        };
        // the bow
        let (vb, fb) = match stroke {
            _ if gone => (0.0, 0.0),
            Stroke::Bounced => {
                let fb = if (0.0..tc).contains(&tt) {
                    fb_full * BOUNCE_WEIGHT * dmath::sin(dmath::PI * tt / tc)
                } else {
                    0.0
                };
                (v_full, fb)
            }
            _ => {
                let ts = tt + pre;
                let up = if slur.is_some() { 1.0 } else { smooth(ts / att) };
                let sw = swelled(play.swell, tt) * swelled(play.expression, tt);
                let end = if tie {
                    1.0
                } else {
                    1.0 - smooth((tt - (dur - rel)) / rel)
                };
                // (past 60 of its times the bite adds under a part in 10^26: nothing at all to the 1)
                let bite = if ts / p.bite > 60.0 {
                    1.0
                } else {
                    1.0 + acc * p.accent * dmath::exp(-ts / p.bite)
                };
                // a tremolo's strokes: the bow's speed swings back and forth as a sine, its weight following the
                // speed, so the bow's pull on the string swings as one too (a weight held through each turn would
                // flip the string's sideways push at every stroke, a thump under the note)
                let (dir, weight) = if stroke == Stroke::Tremolo {
                    let swing = dmath::sin(dmath::PI * tt.max(0.0) * TREMOLO_HZ);
                    (swing, swing.abs().max(TREMOLO_WEIGHT))
                } else {
                    (1.0, 1.0)
                };
                let mut vb = v_full * sw * up * (1.0 + WANDER_SPEED * wander) * if stop { end } else { 1.0 } * dir;
                if p.bite_speed {
                    vb *= bite;
                }
                let lift = bite * (0.6 + 0.4 * up);
                let held = if stop {
                    p.stop_weight + (1.0 - p.stop_weight) * end
                } else {
                    end
                };
                let mut fb = fb_full * sw * lift * held * weight;
                if tt > dur + LEAVES && !tie {
                    fb = 0.0;
                }
                (vb, fb)
            }
        };
        // the waves arriving at the bow
        let v_ib = -lb.run(bl.read_tap(&at_bow));
        let v_in = match touched {
            Some(h) => {
                // a finger touching lightly at 1/h of the string from the nut end: a small damper there
                // (the nut's reflection is part of the stretch from the touch to the nut)
                let d_touch_nut = ((per * tune / 2.0) / h - dln / 2.0).max(1.0);
                let d_bow_touch = (dn_total - d_touch_nut).max(2.0);
                let a_in = nl.read(d_bow_touch);
                let b_in = -ln.run(tl.read(2.0 * d_touch_nut));
                let vt = (a_in + b_in) * (2.0 * z) / (2.0 * z + r_touch);
                let dlt = vt - (a_in + b_in);
                tl.push(a_in + dlt);
                hret.push(b_in + dlt);
                // (one sample less: the bow's own step, as the plain string's read)
                hret.read(d_bow_touch - 1.0)
            }
            None => {
                let x = nl.read(2.0 * dn_total - 1.0);
                if (stopped || play.mute == Mute::Choked) && tt > dur + let_go {
                    -ln_off.run(x)
                } else {
                    -ln.run(x)
                }
            }
        };
        // the string's twist (at the top notes the bridge side is only a sample or so)
        let (r_ib, r_in) = if gone {
            (0.0, 0.0)
        } else {
            let pr = per / p.twist_speed;
            let dbr = (beta * pr / 2.0).max(1.0);
            let dnr = (pr / 2.0 - dbr).max(1.0);
            (
                -p.twist_keep * tlb.run(trb.read(2.0 * dbr - 1.0)),
                -p.twist_keep * tln.run(trn.read(2.0 * dnr - 1.0)),
            )
        };
        let r_t = if p.twist_z > 0.0 { r_ib + r_in } else { 0.0 };
        if fb > 0.0 {
            nz = a_nz * nz + nz_g * rng.gauss();
        }
        let fbn = fb * (1.0 + p.noise * nz).max(0.0);
        let vh = v_ib + v_in + r_t;
        let mut f = 0.0;
        if fb > 0.0 {
            let k = yt * fbn;
            let a = (vh - vb).abs();
            let sgn = if vh - vb >= 0.0 { 1.0 } else { -1.0 };
            let bq = p.v0 - a + k * mud;
            let c = p.v0 * (k * mus - a);
            let disc = bq * bq - 4.0 * c;
            let slip_root = if disc >= 0.0 {
                let x = (-bq + disc.sqrt()) / 2.0;
                (x > 0.0).then_some(x)
            } else {
                None
            };
            if a > k * mus || (slipping && slip_root.is_some()) {
                let x = slip_root.unwrap_or(0.0);
                let mu = (mud + (mus - mud) * p.v0 / (p.v0 + x)) * (1.0 + p.slip_noise * rng.gauss());
                f = -sgn * fbn * mu;
                if !slipping {
                    // the player's ear: the time from one slip to the next is the note's period; the finger moves
                    // a hair to bring it to the note
                    if let Some(ls) = last_slip {
                        let gap = (s - ls) as f64;
                        // (a harmonic's slips come at its own period, a part of the string's)
                        let r = per / touched.unwrap_or(1.0) / gap;
                        if (r - 1.0).abs() < EAR_NEAR && p.intone > 0.0 && tt >= 0.0 {
                            let k = 1.0 - dmath::exp(-(gap / fs) / p.intone);
                            tune = (tune * (1.0 + k * (r - 1.0))).clamp(1.0 - EAR_RANGE, 1.0 + EAR_RANGE);
                        }
                    }
                    last_slip = Some(s);
                }
                slipping = true;
            } else {
                f = (vb - vh) / yt;
                slipping = false;
            }
        } else {
            slipping = false;
        }
        wander = a_w * wander + wander_g * rng.gauss();
        let dv = f * y;
        bl.push(v_in + dv);
        nl.push(v_ib + dv);
        if !gone {
            let dr = f * yr;
            trb.push(r_in + dr);
            trn.push(r_ib + dr);
        }
        // the push on the bridge: the wave arriving there
        *o = 2.0 * z * bl.read_tap(&at_bridge);
    }
    // the bridge's steady sideways push is not heard
    let a_dc = dmath::exp(-dmath::TAU * DC_HZ / fs);
    let (mut xp, mut yp) = (0.0, 0.0);
    for v in &mut out {
        let y = *v - xp + a_dc * yp;
        xp = *v;
        yp = y;
        *v = y;
    }
    let mut x = out.split_off(start);
    // a slurred note comes in as the one tied to it goes out
    let k = (JOIN * fs) as usize;
    if slur.is_some() {
        for (s, v) in x.iter_mut().take(k).enumerate() {
            *v *= dmath::sin(0.5 * dmath::PI * s as f64 / k as f64);
        }
    }
    if tie {
        let a = (dur * fs) as usize;
        for (s, v) in x.iter_mut().enumerate().skip(a) {
            let u = ((s - a) as f64 / k as f64).min(1.0);
            *v *= dmath::cos(0.5 * dmath::PI * u);
        }
    }
    x
}

/// `x` at `os` times the heard rate brought down to it: a windowed-sinc (Blackman) low-pass, every `os`-th sample
/// kept, `n` of them.
fn down(x: &[f64], os: usize, n: usize) -> Vec<f64> {
    if os == 1 {
        let mut y = x.to_vec();
        y.resize(n, 0.0);
        return y;
    }
    let taps = DOWN_TAPS * os + 1;
    let c = taps / 2;
    let fc = DOWN_EDGE / os as f64;
    let last = (taps - 1) as f64;
    let h: Vec<f64> = (0..taps)
        .map(|k| {
            let m = k as f64 - c as f64;
            let s = if k == c {
                2.0 * fc
            } else {
                dmath::sin(dmath::TAU * fc * m) / (dmath::PI * m)
            };
            let w = 0.42 - 0.5 * dmath::cos(dmath::TAU * k as f64 / last)
                + 0.08 * dmath::cos(2.0 * dmath::TAU * k as f64 / last);
            s * w
        })
        .collect();
    (0..n)
        .map(|i| {
            let centre = i * os;
            let lo = c.saturating_sub(centre);
            let hi = taps.min(x.len() + c - centre.min(x.len() + c));
            // h[k] x[centre + k - c] for k in lo..hi, in four running sums (the adds need not wait on each other)
            let from = x.get(centre + lo - c..).unwrap_or(&[]);
            let k = (hi - lo).min(from.len());
            let (hs, xs) = (&h[lo..lo + k], &from[..k]);
            let ((ha, hr), (xa, xr)) = (hs.as_chunks::<4>(), xs.as_chunks::<4>());
            let mut acc = [0.0; 4];
            for (a, b) in ha.iter().zip(xa) {
                for ((s, a), b) in acc.iter_mut().zip(a).zip(b) {
                    *s += a * b;
                }
            }
            let rest: f64 = hr.iter().zip(xr).map(|(a, b)| a * b).sum();
            (acc[0] + acc[1]) + (acc[2] + acc[3]) + rest
        })
        .collect()
}

/// The body's pull on a plucked string's mode at `f` Hz (the sketch's `adm`).
fn pull(modes: &[(f64, f64, f64)], f: f64) -> f64 {
    modes
        .iter()
        .map(|&(fm, q, g)| {
            let r = f / fm;
            let a = 1.0 - r * r;
            g * (r / q) / (a * a + (r / q) * (r / q))
        })
        .sum()
}

/// The strongest pull of `modes` from 20 Hz up (the sketch's grid).
fn strongest(modes: &[(f64, f64, f64)]) -> f64 {
    let mut f = 20.0;
    let mut most = 0.0f64;
    for _ in 0..2000 {
        most = most.max(pull(modes, f));
        f *= 1.005;
    }
    most.max(1e-9)
}

/// A plucked or struck note's force on the bridge, `n` samples at the heard rate `fs`: the string's modes, each a
/// damped oscillator (exact for its frequency and loss), pulled aside (or struck by the stick) and let go.
fn plucked(p: &BowedParts, play: &Play, how: Pluck, n: usize, variant: u64, fs: f64) -> Vec<f64> {
    let m = key_of(play);
    let wr = wire(p, play, m);
    let f0 = hz(m);
    let dt = 1.0 / fs;
    let r = hz(wr.open) / f0;
    let l = p.length * r.min(1.0);
    let b = p.stiffness / (r * r);
    let at = match how {
        Pluck::Stick => p.beta,
        Pluck::LeftHand => LEFT_AT,
        _ => p.pizz_at,
    };
    let beta = play
        .position
        .filter(|x| x.is_finite())
        .map_or(at, |x| x.clamp(0.02, 0.5));
    let (t60, fc) = if wr.stopped {
        (p.t60 * p.finger, p.loss_hz * p.finger_hz)
    } else {
        (p.t60, p.loss_hz)
    };
    let count = ((p.top.min(NYQUIST_SHARE * fs) / f0) as usize).clamp(1, MOST_MODES);
    let mass = wr.rho * l / 2.0;
    let peak = strongest(p.drain_modes);
    let mut fr = Vec::with_capacity(count);
    let mut sig = Vec::with_capacity(count);
    let mut bb = Vec::with_capacity(count);
    let mut kbr = Vec::with_capacity(count);
    for i in 0..count {
        let k = (i + 1) as f64;
        let f = k * f0 * (1.0 + b * k * k).sqrt();
        let t = t60 / (1.0 + dmath::pow(f / fc, 1.5));
        fr.push(f);
        sig.push(6.91 / t + p.drain * pull(p.drain_modes, f) / peak);
        let phi = dmath::sin(k * dmath::PI * beta);
        bb.push(phi * dt / mass);
        kbr.push(wr.tension * k * dmath::PI / l);
    }
    let vel = unit(unit(play.vel) + PLUCK_ACCENT * unit(play.accent));
    let mut q = vec![0.0; count];
    let mut qd = vec![0.0; count];
    if how != Pluck::Stick {
        let (pull, width) = match how {
            Pluck::Snap => (SNAP_PULL, 1.0),
            Pluck::Slap => (SLAP_PULL, 1.0),
            Pluck::LeftHand => (LEFT_PULL, LEFT_WIDTH),
            _ => (1.0, 1.0),
        };
        let h = p.pizz_level * PULL * p.length * (0.25 + 0.75 * vel) * pull;
        let wf = p.pizz_width * width / l;
        for (i, qi) in q.iter_mut().enumerate() {
            let k = (i + 1) as f64;
            let a = k * dmath::PI * wf / 2.0;
            let soft = if a > 1e-9 { dmath::sin(a) / a } else { 1.0 };
            *qi = 2.0 * h / (k * k * dmath::PI * dmath::PI * beta * (1.0 - beta))
                * dmath::sin(k * dmath::PI * beta)
                * soft;
        }
    }
    let dur = held(play);
    let damped = if play.mute == Mute::Choked { CHOKE } else { LET_GO };
    let blow = STICK_BLOW * wr.z * (0.3 + 0.7 * vel);
    let mut out = vec![0.0; n];
    let (mut m11, mut m12, mut m21, mut m22) = (vec![0.0; count], vec![0.0; count], vec![0.0; count], vec![0.0; count]);
    let (mut last_pm, mut last_extra) = (f64::NAN, f64::NAN);
    let mut s0 = 0;
    while s0 < n {
        let t = s0 as f64 * dt;
        let mut cents = asked_cents(play, t);
        if let Some(v) = play.vibrato
            && v.cents.is_finite()
            && v.hz.is_finite()
            && v.wait.is_finite()
        {
            let since = (t - v.wait).max(0.0);
            cents += v.cents * (since / 0.25).min(1.0) * dmath::sin(dmath::TAU * v.hz * since);
        }
        let pm = dmath::pow(2.0, cents / 1200.0);
        let extra = if t - dur > 0.0 { damped } else { 0.0 };
        if pm != last_pm || extra != last_extra {
            for i in 0..count {
                let w = dmath::TAU * fr[i] * pm;
                let s = sig[i] + extra * (1.0 + (fr[i] / 1000.0) * (fr[i] / 1000.0));
                let wd = (w * w - s * s).max(1.0).sqrt();
                let e = dmath::exp(-s * dt);
                let (sn, cs) = (dmath::sin(wd * dt), dmath::cos(wd * dt));
                m11[i] = e * (cs + s / wd * sn);
                m12[i] = e * sn / wd;
                m21[i] = -e * w * w / wd * sn;
                m22[i] = e * (cs - s / wd * sn);
            }
            (last_pm, last_extra) = (pm, extra);
        }
        let s1 = (s0 + BLOCK).min(n);
        for (s, o) in out.iter_mut().enumerate().take(s1).skip(s0) {
            let tt = s as f64 * dt;
            let f = if how == Pluck::Stick && tt < STICK_TIME {
                blow * dmath::sin(dmath::PI * tt / STICK_TIME)
            } else {
                0.0
            };
            let mut sum = 0.0;
            for i in 0..count {
                let v = qd[i] + bb[i] * f;
                sum += kbr[i] * q[i];
                let qn = m11[i] * q[i] + m12[i] * v;
                qd[i] = m21[i] * q[i] + m22[i] * v;
                q[i] = qn;
            }
            *o = sum;
        }
        s0 = s1;
    }
    // the clicks: the snap's string on the fingerboard, the stick's wood, the slap's hand and the fingerboard
    let click = match how {
        Pluck::Snap => Some((SNAP_CLICK, CLICK_TIME, CLICK_DECAY, None)),
        Pluck::Stick => Some((STICK_CLICK, CLICK_TIME, CLICK_DECAY, None)),
        Pluck::Slap => Some((SLAP_CLACK, SLAP_TIME, SLAP_DECAY, Some(SLAP_CLACK_HZ))),
        _ => None,
    };
    if let Some((level, time, decay, top)) = click {
        let mut r2 = Rosin::new(variant.wrapping_mul(7919).wrapping_add(17).wrapping_add(99));
        let pk = out.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        let g = level * pk;
        let a_top = top.map(|h| dmath::exp(-dmath::TAU * h.min(NYQUIST_SHARE * fs) / fs));
        let (mut lp, mut soft) = (0.0, 0.0);
        for (s, o) in out.iter_mut().enumerate().take((time * fs) as usize) {
            let t = s as f64 * dt;
            let w = r2.gauss();
            let hp = w - lp;
            lp += 0.3 * (w - lp);
            let v = match a_top {
                Some(a) => {
                    soft = (1.0 - a) * hp + a * soft;
                    soft
                }
                None => hp,
            };
            *o += g * v * dmath::exp(-t / decay);
        }
    }
    out
}

/// The force on the bridge of the note `play` on `p`: `len` samples at sample rate `fs`, as repeat `variant`.
pub fn bridge(p: &BowedParts, play: &Play, len: usize, variant: u64, fs: f64) -> Vec<f64> {
    if len == 0 {
        return Vec::new();
    }
    match way(p, play) {
        Way::Bowed(stroke) => {
            let os = steps(p, fs);
            let fs_s = fs * os as f64;
            // the string rings no longer than its tail; the decimator looks a little past the end
            let w = wire(p, play, key_of(play));
            let rings = held(play) + bowed_tail(p, play, &w);
            let n = (len * os).min((rings * fs_s) as usize) + DOWN_TAPS * os;
            let x = down(&bowed(p, play, stroke, n, variant, fs_s), os, len);
            if stroke == Stroke::Tremolo {
                // the strokes' slow push on the bridge, under the note, which no body sends out
                let under = TREMOLO_UNDER * hz(key_of(play)).min(NYQUIST_SHARE * fs / TREMOLO_UNDER);
                dsp::sosfilt(&butter_sos(FilterKind::Hp, &[under], 4, fs), &x)
            } else {
                x
            }
        }
        Way::Plucked(how) => {
            let rings = rings_for(p, play, f64::INFINITY);
            let mut x = plucked(p, play, how, len.min((rings * fs) as usize + 1), variant, fs);
            x.resize(len, 0.0);
            x
        }
    }
}

/// A mute on `p`'s bridge (con sordino): the bridge hill taken away and the top veiled.
fn sordino(p: &BowedParts) -> [Band; 2] {
    [
        Band::Peak {
            hz: p.bridge_hz,
            db: SORDINO_HILL,
            q: 0.9,
        },
        Band::High {
            hz: 0.6 * p.bridge_hz,
            db: SORDINO_TOP,
            q: 0.7,
        },
    ]
}

/// The note `play` on the bowed string instrument `p`: `len` samples at sample rate `fs`, as repeat `variant` (each
/// repeat bowed a hair differently).
pub fn note(p: &BowedParts, play: &Play, len: usize, variant: u64, fs: f64) -> Vec<f64> {
    if len == 0 {
        return Vec::new();
    }
    let mut x = bridge(p, play, len, variant, fs);
    if play.mute == Mute::Muted {
        x = eq::apply(&x, &sordino(p), fs);
    }
    let mut y = body::apply(&x, p.body, fs);
    y.iter_mut().for_each(|v| *v *= p.gain);
    end_fade(&mut y, fs);
    y
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
    use crate::dsp::fft::Fft;
    use crate::voice::engine2::play::{Bend, Swell, Vibrato};

    fn hz_of(m: f64) -> f64 {
        440.0 * dmath::pow(2.0, (m - 69.0) / 12.0)
    }

    fn cents(f: f64, of: f64) -> f64 {
        1200.0 * dmath::log2(f / of)
    }

    fn seg(x: &[f64], a: f64, b: f64, fs: f64) -> &[f64] {
        let j = ((b * fs) as usize).min(x.len());
        &x[((a * fs) as usize).min(j)..j]
    }

    /// The level over `a..b` seconds, dB.
    fn db(x: &[f64], a: f64, b: f64, fs: f64) -> f64 {
        let s = seg(x, a, b, fs);
        let e = s.iter().map(|v| v * v).sum::<f64>() / s.len().max(1) as f64;
        10.0 * dmath::log10(e.max(1e-30))
    }

    /// `x` with a two-pole high-pass at `hz` (two one-pole splits).
    fn highs(x: &[f64], hz: f64, fs: f64) -> Vec<f64> {
        let k = dmath::exp(-dmath::TAU * hz / fs);
        let (mut l1, mut l2) = (0.0, 0.0);
        x.iter()
            .map(|&v| {
                l1 = (1.0 - k) * v + k * l1;
                let h = v - l1;
                l2 = (1.0 - k) * h + k * l2;
                h - l2
            })
            .collect()
    }

    /// The share of the energy over `a..b` seconds above `hz`, dB.
    fn bright(x: &[f64], a: f64, b: f64, hz: f64, fs: f64) -> f64 {
        db(&highs(x, hz, fs), a, b, fs) - db(x, a, b, fs)
    }

    /// The Hann-windowed power of the component at `f` over `a..b` seconds, dB.
    fn at_hz(x: &[f64], f: f64, a: f64, b: f64, fs: f64) -> f64 {
        let s = seg(x, a, b, fs);
        let w = dmath::TAU * f / fs;
        let (mut re, mut im) = (0.0, 0.0);
        let n = s.len() as f64;
        for (i, v) in s.iter().enumerate() {
            let hann = 0.5 - 0.5 * dmath::cos(dmath::TAU * i as f64 / n);
            re += v * hann * dmath::cos(w * i as f64);
            im += v * hann * dmath::sin(w * i as f64);
        }
        10.0 * dmath::log10((re * re + im * im).max(1e-30))
    }

    /// The strongest frequency between `lo` and `hi` Hz over `a..b` seconds, in half-cent steps.
    fn peak_in(x: &[f64], lo: f64, hi: f64, a: f64, b: f64, fs: f64) -> f64 {
        let (mut best, mut at, mut g) = (f64::MIN, lo, lo);
        while g <= hi {
            let p = at_hz(x, g, a, b, fs);
            if p > best {
                best = p;
                at = g;
            }
            g *= 1.0 + 0.5 / 1731.0;
        }
        at
    }

    /// How far the strongest frequency within 3% of `f` over `a..b` seconds is from `f`, cents.
    fn off(x: &[f64], f: f64, a: f64, b: f64, fs: f64) -> f64 {
        cents(peak_in(x, f * 0.97, f * 1.03, a, b, fs), f)
    }

    /// How far a note near `f0` sounds from it over `a..b` seconds, cents, measured on an overtone for a low note (the
    /// 2nd under 300 Hz, the 3rd under 150 Hz) for a finer reading in a short stretch, within 6% either way.
    fn pitch(x: &[f64], f0: f64, a: f64, b: f64, fs: f64) -> f64 {
        let h = if f0 < 150.0 {
            3.0
        } else if f0 < 300.0 {
            2.0
        } else {
            1.0
        };
        cents(peak_in(x, h * f0 * 0.94, h * f0 * 1.06, a, b, fs), h * f0)
    }

    /// The levels of the first `k` harmonics of a note near `f0` over `t0..t1` seconds, dB: 0.1 s Hann windows at
    /// half overlap, each transformed at 2^15 points (at 48 kHz), its fundamental found within 3% by the first six
    /// harmonics together, each harmonic the strongest bin within 1% of it; the mean over the windows (`nl2.py`).
    fn harmonics(x: &[f64], f0: f64, t0: f64, t1: f64, k: usize, fs: f64) -> Vec<f64> {
        let w = (0.1 * fs) as usize;
        let n = (w * 6).next_power_of_two();
        let fft = Fft::new(n);
        let bin = fs / n as f64;
        let mut sum = vec![0.0; k];
        let mut rows = 0;
        let mut s = (t0 * fs) as usize;
        while s + w <= ((t1 * fs) as usize).min(x.len()) {
            let mut re = vec![0.0; n];
            let mut im = vec![0.0; n];
            for i in 0..w {
                re[i] = x[s + i] * (0.5 - 0.5 * dmath::cos(dmath::TAU * i as f64 / (w - 1) as f64));
            }
            fft.forward(&mut re, &mut im);
            let mag: Vec<f64> = (0..n / 2).map(|i| (re[i] * re[i] + im[i] * im[i]).sqrt()).collect();
            let at = |f: f64| {
                let u = f / bin;
                let i = u.floor() as usize;
                if i + 1 >= mag.len() {
                    0.0
                } else {
                    mag[i] + (mag[i + 1] - mag[i]) * (u - i as f64)
                }
            };
            let (mut fe, mut most) = (f0, f64::MIN);
            for c in 0..121 {
                let g = f0 * (0.97 + 0.06 * c as f64 / 120.0);
                let v: f64 = (1..=6).map(|h| at(h as f64 * g)).sum();
                if v > most {
                    most = v;
                    fe = g;
                }
            }
            for (h, acc) in sum.iter_mut().enumerate() {
                let f = (h + 1) as f64 * fe;
                let span = (0.01 * f).max(2.0 * bin);
                let lo = ((f - span) / bin).ceil().max(0.0) as usize;
                let hi = (((f + span) / bin).floor() as usize).min(mag.len() - 1);
                let top = (lo..=hi).map(|i| mag[i]).fold(0.0, f64::max);
                *acc += 20.0 * dmath::log10(top + 1e-12);
            }
            rows += 1;
            s += w / 2;
        }
        sum.iter().map(|v| v / rows.max(1) as f64).collect()
    }

    /// Least squares: the coefficients of `cols` (each a column) best fitting `y`.
    fn fit(cols: &[Vec<f64>], y: &[f64]) -> Vec<f64> {
        let m = cols.len();
        let mut a = vec![vec![0.0; m + 1]; m];
        for i in 0..m {
            for j in 0..m {
                a[i][j] = cols[i].iter().zip(&cols[j]).map(|(u, v)| u * v).sum();
            }
            a[i][m] = cols[i].iter().zip(y).map(|(u, v)| u * v).sum();
        }
        for i in 0..m {
            let p = (i..m)
                .max_by(|&r, &s| a[r][i].abs().total_cmp(&a[s][i].abs()))
                .unwrap_or(i);
            a.swap(i, p);
            for r in 0..m {
                if r != i {
                    let q = a[r][i] / a[i][i];
                    let pivot = a[i].clone();
                    for (v, w) in a[r].iter_mut().zip(&pivot).skip(i) {
                        *v -= q * w;
                    }
                }
            }
        }
        (0..m).map(|i| a[i][m] / a[i][i]).collect()
    }

    /// How far single overtone numbers stand out or drop out of every note alike, dB (the note-locked score of
    /// `nl2.py`): each note's harmonic levels less their fit on [1, log2 n], the mean of those over the notes, and
    /// the RMS of what a quadratic in log2 n leaves of that mean (the fundamental aside).
    fn locked(levels: &[Vec<f64>]) -> f64 {
        let k = levels[0].len();
        let l2: Vec<f64> = (1..=k).map(|n| dmath::log2(n as f64)).collect();
        let mut pat = vec![0.0; k];
        for l in levels {
            let c = fit(&[vec![1.0; k], l2.clone()], l);
            for n in 0..k {
                pat[n] += (l[n] - c[0] - c[1] * l2[n]) / levels.len() as f64;
            }
        }
        let cols = [
            vec![1.0; k - 1],
            l2[1..].to_vec(),
            l2[1..].iter().map(|v| v * v).collect(),
        ];
        let c = fit(&cols, &pat[1..]);
        let rough: Vec<f64> = (1..k)
            .map(|n| pat[n] - c[0] - c[1] * l2[n] - c[2] * l2[n] * l2[n])
            .collect();
        (rough.iter().map(|v| v * v).sum::<f64>() / rough.len() as f64).sqrt()
    }

    /// The worst burst of highs (over 5 kHz, 4 ms windows) within 6 ms of `t`, dB over the median of the 0.2 s
    /// before it.
    fn burst(x: &[f64], t: f64, fs: f64) -> f64 {
        let hp = highs(x, 5000.0, fs);
        let e = |u: f64| db(&hp, u - 0.002, u + 0.002, fs);
        let around = (0..7).map(|i| e(t - 0.006 + 0.002 * i as f64)).fold(f64::MIN, f64::max);
        let mut before: Vec<f64> = (0..50).map(|i| e(t - 0.25 + 0.004 * i as f64)).collect();
        before.sort_by(f64::total_cmp);
        around - before[25]
    }

    /// No vibrato (senza vibrato).
    const SENZA: Option<Vibrato> = Some(Vibrato {
        cents: 0.0,
        hz: 5.0,
        wait: 0.0,
    });

    /// Each instrument with its held notes across its range (`nl2.py`'s) and a note in its middle.
    const ALL: [(BowedParts, [f64; 10], f64); 5] = [
        (
            BowedParts::VN3,
            [55.0, 57.0, 60.0, 64.0, 67.0, 69.0, 72.0, 76.0, 79.0, 81.0],
            69.0,
        ),
        (
            BowedParts::VA3,
            [48.0, 50.0, 53.0, 55.0, 57.0, 60.0, 62.0, 64.0, 67.0, 69.0],
            62.0,
        ),
        (
            BowedParts::VC1,
            [36.0, 38.0, 41.0, 43.0, 45.0, 48.0, 50.0, 53.0, 55.0, 57.0],
            50.0,
        ),
        (
            BowedParts::CB1,
            [28.0, 31.0, 33.0, 36.0, 38.0, 40.0, 43.0, 45.0, 47.0, 50.0],
            40.0,
        ),
        (
            BowedParts::OC2,
            [22.0, 24.0, 26.0, 29.0, 31.0, 34.0, 36.0, 38.0, 41.0, 43.0],
            34.0,
        ),
    ];

    /// The violin, the viola and the cello (Aedan's squeak notes were on these).
    const UPPER: usize = 3;

    fn sound(p: &BowedParts, play: &Play, secs: f64, fs: f64) -> Vec<f64> {
        note(p, play, (secs * fs) as usize, 0, fs)
    }

    fn force(p: &BowedParts, play: &Play, secs: f64) -> Vec<f64> {
        bridge(p, play, (secs * SRF) as usize, 0, SRF)
    }

    fn with(t: Technique, m: f64, dur: f64, vel: f64) -> Play {
        Play {
            technique: t,
            vibrato: SENZA,
            ..Play::plain(m, dur, vel)
        }
    }

    #[test]
    fn every_instrument_bows_in_tune_at_48_and_44_1_khz() {
        for (p, _, m) in &ALL {
            for fs in [SRF, 44100.0] {
                let x = sound(p, &with(Technique::Arco, *m, 1.5, 0.7), 1.6, fs);
                let c = off(&x, hz_of(*m), 0.4, 1.4, fs);
                assert!(c.abs() < 5.0, "{m} at {fs}: {c:.1} cents");
            }
        }
    }

    #[test]
    fn a_bowed_note_slips_once_a_period() {
        // the Helmholtz motion's sawtooth on the bridge: each overtone weaker than the one before (about -6, -9.5 dB)
        for (p, _, m) in &ALL {
            let x = force(p, &with(Technique::Arco, *m, 1.5, 0.7), 1.6);
            let g = peak_in(&x, hz_of(*m) * 0.97, hz_of(*m) * 1.03, 0.4, 1.4, SRF);
            let h: Vec<f64> = (1..=3).map(|n| at_hz(&x, n as f64 * g, 0.4, 1.4, SRF)).collect();
            assert!(h[1] < h[0] - 4.0 && h[2] < h[0] - 7.0, "{m}: {h:?}");
        }
    }

    #[test]
    fn with_the_ear_off_a_held_note_bows_near_its_pitch() {
        // fix 4: twist speeds that keep each note's bowed pitch near the note, so the player's ear has no scoop to
        // make (at the round's speeds the cello's upper notes bowed 40 cents flat). The approved sketch: the violin
        // and viola within 9 cents, the cello 13, the contrabass 15, the octobass's lowest notes up to 53 flat.
        let sets: [(&[f64], f64); 5] = [
            (&[55.0, 62.0, 69.0, 76.0, 81.0, 88.0], 10.0),
            (&[48.0, 55.0, 62.0, 69.0, 74.0, 81.0], 10.0),
            (&[36.0, 43.0, 50.0, 57.0, 64.0, 69.0], 15.0),
            (&[28.0, 33.0, 38.0, 43.0, 50.0, 55.0], 20.0),
            (&[22.0, 26.0, 29.0, 34.0, 41.0], 60.0),
        ];
        for ((p, _, _), (notes, most)) in ALL.iter().zip(sets) {
            let deaf = BowedParts { intone: 0.0, ..*p };
            for m in notes {
                for vel in [0.55, 0.8] {
                    let x = force(&deaf, &with(Technique::Arco, *m, 0.6, vel), 0.7);
                    let c = pitch(&x, hz_of(*m), 0.15, 0.55, SRF);
                    assert!(c.abs() < most, "{m} at {vel}: {c:.1} cents");
                }
            }
        }
    }

    #[test]
    fn an_accent_settles_on_its_pitch_with_one_slip_a_period() {
        // fix 4: an accent's bow speed and weight come together, so from 50 ms it sounds its note, cleanly
        for (p, notes, _) in &ALL[..UPPER] {
            for m in [notes[0], notes[5], notes[9]] {
                let f = hz_of(m);
                let x = force(
                    p,
                    &Play {
                        accent: 0.9,
                        ..with(Technique::Arco, m, 0.6, 0.9)
                    },
                    0.7,
                );
                for i in 0..5 {
                    let a = 0.05 + 0.1 * i as f64;
                    let c = pitch(&x, f, a, a + 0.1, SRF);
                    assert!(c.abs() < 20.0, "{m} at {a:.2} s: {c:.1} cents");
                }
                let h2 = at_hz(&x, 2.0 * f, 0.05, 0.5, SRF) - at_hz(&x, f, 0.05, 0.5, SRF);
                assert!(h2 < -3.0, "{m}: the 2nd overtone {h2:+.1} dB on the fundamental");
            }
        }
    }

    #[test]
    fn a_martele_stops_the_string_and_never_sounds_above_the_note() {
        // fix 4: a stopped bow lets its weight off with its speed; weight left on a bow at rest pinned the string
        // like a fret, and the pitch jumped two semitones
        for (p, notes, _) in &ALL[..UPPER] {
            for m in [notes[0], notes[5], notes[9]] {
                let f = hz_of(m);
                let x = force(p, &with(Technique::Martele, m, 0.3, 0.8), 0.6);
                for i in 0..8 {
                    let a = 0.24 + 0.02 * i as f64;
                    let c = pitch(&x, f, a, a + 0.06, SRF);
                    assert!(c < 30.0, "{m} at {a:.2} s: {c:+.0} cents");
                }
                let arco = sound(p, &with(Technique::Arco, m, 0.3, 0.8), 0.8, SRF);
                let mart = sound(p, &with(Technique::Martele, m, 0.3, 0.8), 0.8, SRF);
                assert!(
                    db(&mart, 0.35, 0.5, SRF) < db(&arco, 0.35, 0.5, SRF) - 2.0,
                    "{m}: rings on after the stop"
                );
            }
        }
    }

    #[test]
    fn the_overtones_follow_no_pattern_locked_to_the_notes() {
        // fix 3 (Aedan: "the sustained tone is gone"): no overtone number stands out or drops out on every note
        // alike, a pattern that would follow the melody like a synth playing along (the real violin 1.2 dB)
        for (p, notes, _) in &ALL {
            let levels: Vec<Vec<f64>> = notes
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let x = note(p, &Play::plain(*m, 1.6, 0.7), (1.7 * SRF) as usize, i as u64, SRF);
                    harmonics(&x, hz_of(*m), 0.5, 1.5, 16, SRF)
                })
                .collect();
            let score = locked(&levels);
            assert!(score < 2.0, "{:?}: {score:.2} dB", p.body);
        }
    }

    #[test]
    fn a_harder_bow_is_louder_and_no_darker() {
        for (p, _, m) in &ALL {
            let soft = sound(p, &with(Technique::Arco, *m, 1.0, 0.3), 1.0, SRF);
            let hard = sound(p, &with(Technique::Arco, *m, 1.0, 0.9), 1.0, SRF);
            assert!(db(&hard, 0.3, 0.9, SRF) > db(&soft, 0.3, 0.9, SRF) + 4.0, "{m}");
            assert!(
                bright(&hard, 0.3, 0.9, 2000.0, SRF) > bright(&soft, 0.3, 0.9, 2000.0, SRF) - 1.5,
                "{m}"
            );
        }
    }

    #[test]
    fn a_spiccato_speaks_at_once_and_leaves_the_string() {
        for (p, _, m) in &ALL {
            let arco = sound(p, &with(Technique::Arco, *m, 0.3, 0.7), 0.6, SRF);
            let spicc = sound(p, &with(Technique::Spiccato, *m, 0.3, 0.7), 0.6, SRF);
            assert!(
                db(&spicc, 0.0, 0.05, SRF) > db(&arco, 0.0, 0.05, SRF) + 3.0,
                "{m}: slow to speak"
            );
            assert!(
                db(&spicc, 0.3, 0.4, SRF) < db(&arco, 0.3, 0.4, SRF) - 4.0,
                "{m}: not short"
            );
        }
    }

    #[test]
    fn a_tremolo_strokes_fourteen_times_a_second_without_a_thump() {
        for (p, _, m) in &ALL {
            let trem = sound(p, &with(Technique::Tremolo, *m, 1.5, 0.7), 1.6, SRF);
            let arco = sound(p, &with(Technique::Arco, *m, 1.5, 0.7), 1.6, SRF);
            // the strokes: the envelope's strongest rate between 5 and 40 a second
            let step = (SRF / 1000.0) as usize;
            let env: Vec<f64> = trem
                .chunks(step)
                .map(|c| c.iter().map(|v| v * v).sum::<f64>().sqrt())
                .collect();
            let mean = env[300..1300].iter().sum::<f64>() / 1000.0;
            let env: Vec<f64> = env.iter().map(|v| v - mean).collect();
            let rate = peak_in(&env, 5.0, 40.0, 0.3, 1.3, 1000.0);
            assert!((rate - TREMOLO_HZ).abs() < 1.0, "{m}: {rate:.1} strokes a second");
            // each turn of the bow flips the string's push: no square wave at the stroke rate under the note
            let thump = |x: &[f64]| at_hz(x, 14.0, 0.3, 1.3, SRF).max(at_hz(x, 28.0, 0.3, 1.3, SRF));
            assert!(thump(&trem) < thump(&arco) + 3.0, "{m}: a thump at the strokes");
        }
    }

    #[test]
    fn a_harmonic_sounds_its_overtone_with_the_fundamental_gone() {
        for (p, _, m) in &ALL {
            for h in [2u8, 3] {
                let x = sound(p, &with(Technique::Harmonic(h), *m, 1.5, 0.7), 1.6, SRF);
                let (f, fh) = (hz_of(*m), hz_of(*m) * f64::from(h));
                let c = off(&x, fh, 0.4, 1.4, SRF);
                assert!(c.abs() < 5.0, "{m} harmonic {h}: {c:.1} cents");
                let over = at_hz(&x, fh, 0.4, 1.4, SRF) - at_hz(&x, f, 0.4, 1.4, SRF);
                assert!(over > 25.0, "{m} harmonic {h}: only {over:.0} dB over the fundamental");
            }
        }
    }

    #[test]
    fn a_pizzicato_is_plucked_in_tune_and_dies_away() {
        for (p, _, m) in &ALL {
            let x = sound(p, &with(Technique::Pizzicato, *m, 1.0, 0.7), 1.2, SRF);
            let c = off(&x, hz_of(*m), 0.05, 0.4, SRF);
            assert!(c.abs() < 6.0, "{m}: {c:.1} cents");
            assert!(
                db(&x, 0.0, 0.1, SRF) > db(&x, 0.8, 1.0, SRF) + 12.0,
                "{m}: rings like a bowed note"
            );
        }
    }

    #[test]
    fn a_snap_is_louder_a_left_hand_pluck_softer_and_the_wood_no_louder_than_the_bow() {
        for (p, _, m) in &ALL {
            let pl = |t| sound(p, &with(t, *m, 1.0, 0.7), 1.2, SRF);
            let (pizz, snap, left, wood) = (
                pl(Technique::Pizzicato),
                pl(Technique::SnapPizz),
                pl(Technique::LeftHandPizz),
                pl(Technique::ColLegno),
            );
            let arco = sound(p, &with(Technique::Arco, *m, 1.0, 0.7), 1.2, SRF);
            let start = |x: &[f64]| db(x, 0.0, 0.1, SRF);
            assert!(start(&snap) > start(&pizz) + 4.0, "{m}: the snap");
            assert!(start(&left) < start(&pizz) - 0.5, "{m}: the left hand louder");
            assert!(
                bright(&left, 0.0, 0.1, 2000.0, SRF) > bright(&pizz, 0.0, 0.1, 2000.0, SRF) + 1.0,
                "{m}: the left hand duller"
            );
            assert!(
                start(&wood) < db(&arco, 0.3, 0.9, SRF) + 4.5,
                "{m}: the wood over the bow"
            );
            assert!(start(&wood) < start(&pizz) + 4.0, "{m}: the wood over the pizzicato");
        }
    }

    #[test]
    fn a_mute_darkens_the_tone() {
        for (p, _, m) in &ALL {
            let open = sound(p, &with(Technique::Arco, *m, 1.0, 0.7), 1.0, SRF);
            let muted = sound(
                p,
                &Play {
                    mute: Mute::Muted,
                    ..with(Technique::Arco, *m, 1.0, 0.7)
                },
                1.0,
                SRF,
            );
            let darker = bright(&muted, 0.3, 0.9, 2000.0, SRF) - bright(&open, 0.3, 0.9, 2000.0, SRF);
            assert!(darker < -3.0, "{m}: {darker:+.1} dB over 2 kHz");
            assert!(db(&muted, 0.3, 0.9, SRF) < db(&open, 0.3, 0.9, SRF) + 1.0, "{m}");
        }
    }

    #[test]
    fn the_bowing_point_changes_the_tone_and_every_point_sounds() {
        // sul ponticello (by the bridge) to sul tasto (over the fingerboard): a lighter, slower bow by the bridge
        // (with the usual weight it held the string fast and went silent there), a slower one over the fingerboard
        for (p, _, m) in &ALL {
            let usual = force(p, &with(Technique::Arco, *m, 1.0, 0.7), 1.0);
            let f = hz_of(*m);
            let shape = |x: &[f64]| -> Vec<f64> { (1..=6).map(|n| at_hz(x, n as f64 * f, 0.3, 0.9, SRF)).collect() };
            let s0 = shape(&usual);
            for b in [0.02, 0.03, 0.05, 0.18, 0.25, 0.3] {
                let play = Play {
                    position: Some(b),
                    ..with(Technique::Arco, *m, 1.0, 0.7)
                };
                let x = sound(p, &play, 1.0, SRF);
                let heard = db(&x, 0.3, 0.9, SRF)
                    - db(&sound(p, &with(Technique::Arco, *m, 1.0, 0.7), 1.0, SRF), 0.3, 0.9, SRF);
                assert!(heard > -20.0 && heard < 8.0, "{m} at {b}: {heard:+.1} dB");
                if b < 0.025 || b > 0.15 {
                    let s = shape(&force(p, &play, 1.0));
                    let moved = (1..6)
                        .map(|n| ((s[n] - s[0]) - (s0[n] - s0[0])).abs())
                        .fold(0.0, f64::max);
                    assert!(moved > 3.0, "{m} at {b}: the same tone");
                }
            }
        }
    }

    #[test]
    fn the_free_ring_dies_with_its_highs_first() {
        // an open string, bowed half a second and left to ring
        for (p, notes, _) in &ALL {
            let m = notes[0];
            let x = sound(p, &with(Technique::Arco, m, 0.5, 0.7), 1.5, SRF);
            assert!(db(&x, 0.9, 1.0, SRF) < db(&x, 0.6, 0.7, SRF) - 6.0, "{m}: rings on");
            assert!(
                bright(&x, 0.9, 1.0, 2000.0, SRF) < bright(&x, 0.3, 0.5, 2000.0, SRF) - 4.0,
                "{m}: its highs last"
            );
        }
    }

    #[test]
    fn a_slur_or_a_tie_joins_without_a_click_or_a_gap() {
        for (p, _, m) in &ALL {
            for glide in [None, Some(0.15)] {
                let a = Play {
                    tie: true,
                    ..Play::plain(*m, 0.5, 0.7)
                };
                let b = Play {
                    slur_from: Some(*m),
                    bend: glide.map(|over| Bend {
                        cents: 0.0,
                        at: 0.0,
                        over,
                    }),
                    ..Play::plain(*m + 2.0, 0.8, 0.7)
                };
                let la = (rings_for(p, &a, 12.0) * SRF) as usize;
                let lb = (rings_for(p, &b, 12.0) * SRF) as usize;
                let at = (0.5 * SRF) as usize;
                let mut x = vec![0.0; at + lb];
                for (i, v) in note(p, &a, la, 0, SRF).iter().enumerate() {
                    x[i] += v;
                }
                for (i, v) in note(p, &b, lb, 1, SRF).iter().enumerate() {
                    x[at + i] += v;
                }
                let click = burst(&x, 0.5, SRF);
                assert!(click < 6.0, "{m} {glide:?}: a burst {click:+.1} dB at the join");
                let dip = db(&x, 0.49, 0.53, SRF) - db(&x, 0.3, 0.45, SRF);
                assert!(dip > -6.0, "{m} {glide:?}: a gap {dip:+.1} dB at the join");
            }
        }
    }

    #[test]
    fn the_same_note_twice_is_the_same_and_its_repeats_differ() {
        for (p, _, m) in &ALL {
            let a = note(p, &Play::plain(*m, 0.5, 0.7), 30000, 0, SRF);
            assert_eq!(a, note(p, &Play::plain(*m, 0.5, 0.7), 30000, 0, SRF));
            let b = note(p, &Play::plain(*m, 0.5, 0.7), 30000, 1, SRF);
            let apart: f64 = a.iter().zip(&b).map(|(u, v)| (u - v) * (u - v)).sum::<f64>().sqrt();
            let size: f64 = a.iter().map(|u| u * u).sum::<f64>().sqrt();
            assert!(apart > 0.05 * size, "{m}: repeats alike");
        }
    }

    #[test]
    fn every_technique_and_mute_is_finite_and_heard() {
        let techniques = [
            Technique::Normal,
            Technique::Arco,
            Technique::Martele,
            Technique::Spiccato,
            Technique::Tremolo,
            Technique::ColLegnoTratto,
            Technique::Harmonic(2),
            Technique::Harmonic(9),
            Technique::ColLegno,
            Technique::SnapPizz,
            Technique::Slap,
            Technique::LeftHandPizz,
            Technique::Pizzicato,
        ];
        for (p, _, m) in &ALL {
            for t in techniques {
                for mute in [Mute::Open, Mute::Muted, Mute::Choked, Mute::Clipped] {
                    let play = Play {
                        technique: t,
                        mute,
                        ..Play::plain(*m, 0.4, 0.7)
                    };
                    let x = note(p, &play, (rings_for(p, &play, 12.0) * SRF) as usize, 0, SRF);
                    assert!(x.iter().all(|v| v.is_finite()), "{m} {t:?} {mute:?}");
                    assert!(db(&x, 0.0, 0.3, SRF) > -60.0, "{m} {t:?} {mute:?}: silent");
                }
            }
        }
    }

    #[test]
    fn notes_at_the_edges_and_odd_asks_stay_finite() {
        for (p, _, m) in &ALL {
            let mut plays: Vec<Play> = [0.0, 10.0, 127.0, f64::NAN, f64::INFINITY]
                .iter()
                .map(|&k| Play::plain(k, 0.4, 0.7))
                .collect();
            let odd = [
                Play::plain(*m, 0.4, 0.0),
                Play {
                    accent: 1.0,
                    ..Play::plain(*m, 0.4, 1.0)
                },
                Play {
                    accent: 5.0,
                    ..Play::plain(*m, 0.4, 3.0)
                },
                Play {
                    accent: f64::NAN,
                    ..Play::plain(*m, 0.4, f64::NAN)
                },
                Play {
                    swell: Some(Swell {
                        from: 0.0,
                        to: 1.0,
                        over: 0.4,
                    }),
                    ..Play::plain(*m, 0.6, 1.0)
                },
                Play {
                    expression: Some(Swell {
                        from: 0.0,
                        to: 0.0,
                        over: 0.1,
                    }),
                    ..Play::plain(*m, 0.6, 0.7)
                },
                Play::plain(*m, 0.0, 0.7),
                Play::plain(*m, f64::NAN, 0.7),
                Play {
                    tie: true,
                    slur_from: Some(*m - 7.0),
                    ..Play::plain(*m, 0.4, 0.7)
                },
                Play {
                    slur_from: Some(f64::NAN),
                    ..Play::plain(*m, 0.4, 0.7)
                },
                Play {
                    position: Some(0.0),
                    ..Play::plain(*m, 0.4, 0.7)
                },
                Play {
                    position: Some(1.0),
                    ..Play::plain(*m, 0.4, 0.7)
                },
                Play {
                    string: Some(9),
                    ..Play::plain(*m, 0.4, 0.7)
                },
            ];
            plays.extend(odd);
            for play in &plays {
                let r = rings_for(p, play, 12.0);
                assert!(r.is_finite() && r > 0.0, "{play:?}");
                let x = note(p, play, (r * SRF) as usize, 0, SRF);
                assert!(x.iter().all(|v| v.is_finite() && v.abs() < 2.0), "{play:?}");
            }
        }
    }

    #[test]
    fn the_basses_pluck_short_plain_notes_and_bow_the_rest() {
        for (p, _, m) in &ALL {
            let at = |t, dur| {
                note(
                    p,
                    &Play {
                        technique: t,
                        ..Play::plain(*m, dur, 0.7)
                    },
                    60000,
                    0,
                    SRF,
                )
            };
            let short = at(Technique::Normal, 0.3);
            if p.bass {
                assert_eq!(short, at(Technique::Pizzicato, 0.3), "{m}");
            } else {
                assert_eq!(short, at(Technique::Arco, 0.3), "{m}");
            }
            assert_eq!(at(Technique::Normal, 0.8), at(Technique::Arco, 0.8), "{m}");
        }
    }

    #[test]
    fn a_note_rings_for_its_tail_and_ends_quietly() {
        for (p, notes, m) in &ALL {
            for play in [
                Play::plain(*m, 1.0, 0.7),
                Play::plain(notes[0], 0.6, 0.7),
                Play {
                    technique: Technique::Pizzicato,
                    ..Play::plain(*m, 0.3, 0.7)
                },
            ] {
                let r = rings_for(p, &play, 12.0);
                assert!(r > play.dur && r <= play.dur + TAIL + 0.6, "{play:?}: {r:.2} s");
                let x = note(p, &play, (r * SRF) as usize, 0, SRF);
                let end = db(&x, r - 0.12, r - 0.02, SRF) - db(&x, 0.0, play.dur, SRF);
                assert!(end < -25.0, "{play:?}: {end:+.0} dB at its end");
            }
            assert!(rings_for(p, &Play::plain(*m, 60.0, 0.7), 12.0) <= 12.0);
        }
    }
}
