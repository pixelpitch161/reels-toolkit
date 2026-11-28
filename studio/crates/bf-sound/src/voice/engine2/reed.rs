//! Reeds: the saxophones, the clarinets, the oboe, the bassoons, the bagpipes and the accordion
//! (`docs/studio/voice-engine-2-instruments.md`; the sketch `rd.rs`, Aedan's picks Ss2, As2, Ts2, Bs2, Cl1, Bc1,
//! Cc1, Ob1, Bn1, Cf2, Bp1 and Ac1).
//!
//! A blown reed (one cane reed on a mouthpiece, or the two blades of a double reed) is a valve that the player's
//! breath pushes shut: the pressure difference between the mouth and the bore bends the reed (a mass on a spring,
//! damped by the lip), and the air squeezed through the gap is the flow (Bernoulli: it grows as the square root of
//! the pressure difference, times the gap), solved exactly each sample; the reed beats against the lay when it shuts.
//! The flow drives the bore, a set of tuned resonances: a cone (the saxophones, the oboe, the bassoons, the chanter)
//! has every multiple, thinned by a comb from the cone's missing tip; a cylinder (the clarinets, the drones) has the
//! odd multiples only. The resonances fall away above the open end's cutoff (the tone holes and the bell), and the
//! bore's own impedance carries what lies above them; the mouthpiece (or the bocal) adds one broad resonance of its
//! own, let go when the note itself reaches it. The register key is a small vent that spoils the resonances that are
//! not multiples of the register's, so the tube plays an octave (a cone) or a twelfth (a cylinder) up; on a slur across
//! registers each resonance hands its swing to the nearest new one. What comes out is the pressure's swing rising with
//! frequency up to the cutoff, a little breath heard directly, the pads' knocks, then the formants the bell and the
//! mouthpiece give.
//!
//! The player tunes each note before playing it (a trial run at its pressure, the fingering moved until it plays in
//! tune, and if it will not speak a little more or less breath, a firmer or looser lip, another throat, up to nine
//! ways), and while it holds the player's ear brings it to pitch in about 35 ms (letting go during bends, scoops,
//! smears and growls). A tongued note starts with the tongue leaving the reed, a little push of the air held behind
//! it and the reed swinging a little wider for a few cycles; an accent is split into more breath and more loudness at
//! the bell. Breath noise rides in the flow.
//!
//! A free reed (the accordion) is a brass tongue swinging through a slot with no bore to rule it: its pitch is its
//! own, the bellows' pressure sets how far it swings, the air is chopped into puffs each time the tongue clears the
//! slot (a plate of fixed thickness, so how long it stays open changes with the swing), and the reed block's cavity
//! colours it. Each key has a bank of reeds (one; two; three a few cents apart for the musette; an octave down...).
//!
//! The reed and its bore run at twice the sample rate (96 kHz at 48 kHz) and are brought down with a windowed-sinc
//! decimator. The sketch plays a whole line through one bore; here a note is played at a time: each note is the
//! sketch's line of that note alone, a slurred one with the note it comes from as a short lead-in (its bore and its
//! breath carried into the slur), a tied one handing over to the next. [`line`] is the sketch's line itself, for
//! measuring the port against it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use bf_core::dmath;

use super::play::{Mute, Play, Register, Technique};

/// The speed of sound, m/s.
const C: f64 = 343.0;
/// How many times the sample rate the reed and the bore run at.
const OS: usize = 2;
/// How long before the note the tongue lands on the reed and the breath starts to build, s.
const PRE: f64 = 0.04;
/// A slurred note's lead-in: the note it comes from, played this long first, s.
const LEAD: f64 = 0.12;
/// A slur's hand-over: the note slurred to comes in, and the one tied to it goes out, over this long, s.
const JOIN: f64 = 0.012;
/// How long a released note rings on after its breath stops, s (the sketch's lines end half a second after the
/// last note; the bore has died away well within this).
const RING: f64 = 0.25;
/// A bagpipe's grace note: the chanter's high G before each note, s (the sketch's `GRACE`).
const GRACE: f64 = 0.032;
/// The amounts the techniques are played at (the round's riffs').
const GROWL: f64 = 0.8;
const FLUTTER: f64 = 0.8;
const SUBTONE: f64 = 0.6;
const SLAP: f64 = 1.0;
const AIR_TONE: f64 = 1.0;
/// Singing into the horn: the throat's hum at the sung note, as much as a growl's this part.
const SUNG: f64 = 0.6;
/// A back tongue ("k"): its push and the reed's wider swing this part of a tip tongue's, the tongue this much slower.
const BACK_KICK: f64 = 0.6;
const BACK_SLOW: f64 = 1.6;
/// A soft legato tongue ("d"): its push and the reed's wider swing this part of a tip tongue's, the tongue this much
/// slower.
const SOFT_KICK: f64 = 0.5;
const SOFT_SLOW: f64 = 1.3;
/// A saxophone's shake: the jaw shaking the note this many cents, so many times a second, from this soon.
const SHAKE: (f64, f64, f64) = (70.0, 7.0, 0.05);
/// The accordion's bellows shake: turns a second, and how far the pressure dips at each.
const BELLOWS_SHAKE: (f64, f64) = (7.0, 0.85);
/// The accordion's air button: the bellows' noise with no reed, this many times its usual level.
const AIR_BUTTON: f64 = 12.0;
/// The note's last 20 ms faded (less for a very short note), so a note cut short ends without a click.
const END_FADE: f64 = 0.02;

/// One reed instrument (or a bagpipe's chanter, or its drones): the sketch's `Inst`, by its names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reed {
    /// A free reed (the accordion) instead of a reed on a bore.
    pub free: bool,
    /// A cylindrical bore (odd overtones) instead of a cone.
    pub cyl: bool,
    /// A cone's missing tip, m (the mouthpiece stands in for it).
    pub x0: f64,
    /// How much the cone's missing tip thins the resonances near its node (0 none, 1 all).
    pub comb: f64,
    /// The registers: (the highest MIDI of the register, which resonance it plays on: 1, 2, 3...).
    pub regs: &'static [(f64, f64)],
    /// What the register vent leaves of the resonances under the one played...
    pub vent_low: f64,
    /// ... and of those over it that are not its overtones.
    pub vent_high: f64,
    /// The bore's resonances' height at 200 Hz (in units of its own impedance), falling as f to the power `peak_p`.
    pub peak: f64,
    pub peak_p: f64,
    /// The resonances' sharpness, as a part of what their losses give.
    pub qk: f64,
    /// The resonances' stretch (a part per overtone).
    pub inh: f64,
    /// Resonances are kept up to this many times the cutoff.
    pub top: f64,
    /// The bore's own impedance over its resonances (a high-pass at `rdir_f`, or at the cutoff for 0).
    pub rdir: f64,
    pub rdir_f: f64,
    /// The radiation's second order (0: rising 6 dB an octave below `rad_f`; 1: 12).
    pub rad2: f64,
    /// The mouthpiece's (and the neck's) broad resonance, Hz, its Q and its height.
    pub mp_f: f64,
    pub mp_q: f64,
    pub mp_h: f64,
    /// The embouchure: how much air the open reed lets through.
    pub zeta: f64,
    /// The blowing pressure at velocity 0 and at 1, as a part of what shuts the reed.
    pub g_lo: f64,
    pub g_hi: f64,
    /// The reed's own resonance (with the lip on it), Hz, and its damping (the lip).
    pub reed_f: f64,
    pub reed_d: f64,
    /// The open end's cutoff (the tone-hole lattice and the bell), Hz.
    pub fc: f64,
    /// The radiation's corner (below it the sound comes out rising with frequency), Hz.
    pub rad_f: f64,
    /// The turbulence at the reed (a flow noise), and the breath heard directly around the mouthpiece.
    pub breath: f64,
    pub air: f64,
    /// The turbulence's band, Hz.
    pub noise_f: (f64, f64),
    /// The pads' knock, and how long the keys take to move, s.
    pub keys: f64,
    pub key_t: f64,
    /// How long the tongue takes to leave the reed, s.
    pub tongue_t: f64,
    /// An accent's extra pressure, a tongued start's small extra push, an accent's extra loudness at the bell, and
    /// a tongued start's wider reed opening (a part of `zeta`).
    pub acc_k: f64,
    pub tkick: f64,
    pub acc_out: f64,
    pub zkick: f64,
    /// How quickly the player's ear brings a held note to pitch (0 none, 1 about 35 ms).
    pub ear: f64,
    /// The vibrato's kind: the jaw (the pitch) or the breath (the pressure).
    pub breath_vibrato: bool,
    /// The breath's slow unsteadiness, and the pitch's slow drift, cents.
    pub jitter: f64,
    pub drift: f64,
    /// The sound's colour as heard (Hz, dB, Q): the mouthpiece's baffle, the bell.
    pub form: &'static [(f64, f64, f64)],
    /// A darkening of what is heard, Hz (0 none), and the high-pass under it, Hz.
    pub lp_out: f64,
    pub hp_out: f64,
    pub gain: f64,
    /// A breath attack's rise, and a breath release, s.
    pub att_breath: f64,
    pub rel: f64,
    /// The throat's hum for a growl, Hz.
    pub growl_f: f64,
    /// Blown from a bag: never tongued, every note joined to the last.
    pub bag: bool,
    /// A free reed's bank: (semitones from the note, cents, gain).
    pub bank: &'static [(f64, f64, f64)],
    /// The reed block's cavity: Hz, Q, dB.
    pub cav_f: f64,
    pub cav_q: f64,
    pub cav_db: f64,
    /// How far the tongue swings inside the slot before air passes (a part of its swing), and the slot's opening on
    /// the inward swing (a part of the outward).
    pub tongue_th: f64,
    pub tongue_in: f64,
    /// How many periods a free reed takes to speak.
    pub free_tau: f64,
    /// Above this the free reed's puffs radiate as their rate of change, Hz.
    pub fsh: f64,
    /// The bellows' noise, and the pallet's (the key's valve) knock.
    pub bellows_noise: f64,
    pub pallet: f64,
}

/// A bagpipe's drones: their reed and bore, and each drone's (semitones over the note asked, its level, how long
/// after the first it strikes in, s).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drones {
    pub reed: Reed,
    pub lines: [(f64, f64, f64); 3],
}

/// A reed instrument's make-up: its reed (a bagpipe's chanter), a bagpipe's drones, and the player's own vibrato
/// `(cents, Hz, wait)` (0 cents: none).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReedParts {
    pub reed: &'static Reed,
    pub drones: Option<&'static Drones>,
    pub vibrato: (f64, f64, f64),
}

/// The sketch's `base()`: what every reed starts from.
const BASE: Reed = Reed {
    free: false,
    cyl: false,
    x0: 0.15,
    comb: 0.6,
    regs: &[(64.0, 1.0), (80.0, 2.0), (99.0, 3.0)],
    vent_low: 0.08,
    vent_high: 0.4,
    peak: 25.0,
    peak_p: 0.35,
    qk: 1.0,
    inh: 0.0,
    top: 3.0,
    rdir: 0.5,
    rdir_f: 0.0,
    rad2: 0.0,
    mp_f: 1100.0,
    mp_q: 2.0,
    mp_h: 0.0,
    zeta: 0.32,
    g_lo: 0.38,
    g_hi: 0.72,
    reed_f: 2000.0,
    reed_d: 0.35,
    fc: 1300.0,
    rad_f: 1300.0,
    breath: 0.012,
    air: 0.004,
    noise_f: (600.0, 6000.0),
    keys: 0.008,
    key_t: 0.006,
    tongue_t: 0.0025,
    acc_k: 0.15,
    tkick: 0.12,
    acc_out: 0.35,
    zkick: 0.1,
    ear: 1.0,
    breath_vibrato: false,
    jitter: 0.004,
    drift: 1.0,
    form: &[],
    lp_out: 0.0,
    hp_out: 40.0,
    gain: 1.0,
    att_breath: 0.09,
    rel: 0.045,
    growl_f: 97.0,
    bag: false,
    bank: &[(0.0, 0.0, 1.0)],
    cav_f: 2200.0,
    cav_q: 1.5,
    cav_db: 5.0,
    tongue_th: 0.12,
    tongue_in: 0.2,
    free_tau: 4.0,
    fsh: 900.0,
    bellows_noise: 0.002,
    pallet: 0.02,
};

/// The saxophones' Jazz-rock mouthpiece (round 1's version 2): a brighter baffle with an edge at 2.7 kHz, a more
/// open reed, more breath.
const JAZZ: Reed = Reed {
    mp_h: 1.8,
    zeta: 0.35,
    form: &[(2700.0, 5.0, 1.2)],
    breath: 0.016,
    air: 0.006,
    ..BASE
};

/// The saxophones (a cone, a single reed): the soprano's, the alto's, the tenor's and the baritone's make-up with
/// the Jazz-rock mouthpiece, its chamber tuned to each size.
const SOPRANO: Reed = Reed {
    x0: 0.09,
    regs: &[(71.0, 1.0), (87.0, 2.0), (99.0, 3.0)],
    reed_f: 2600.0,
    fc: 1800.0,
    rad_f: 1800.0,
    rdir: 1.0,
    rad2: 0.7,
    mp_f: 1300.0,
    mp_q: 1.5,
    ..JAZZ
};
const ALTO: Reed = Reed {
    regs: &[(64.0, 1.0), (80.0, 2.0), (99.0, 3.0)],
    rdir: 1.0,
    rad2: 0.7,
    mp_f: 850.0,
    mp_q: 1.5,
    ..JAZZ
};
const TENOR: Reed = Reed {
    x0: 0.22,
    regs: &[(59.0, 1.0), (75.0, 2.0), (99.0, 3.0)],
    reed_f: 1700.0,
    fc: 1000.0,
    rad_f: 1000.0,
    rdir: 1.0,
    rad2: 0.7,
    mp_f: 650.0,
    mp_q: 1.5,
    ..JAZZ
};
const BARI: Reed = Reed {
    tkick: 0.06,
    x0: 0.32,
    regs: &[(52.0, 1.0), (68.0, 2.0), (99.0, 3.0)],
    reed_f: 1400.0,
    fc: 750.0,
    rad_f: 750.0,
    key_t: 0.008,
    rdir: 1.0,
    rad2: 0.7,
    mp_f: 450.0,
    mp_q: 1.5,
    ..JAZZ
};

/// The clarinets (a cylinder, a single reed).
const CLARINET: Reed = Reed {
    cyl: true,
    zkick: 0.35,
    rdir: 0.4,
    rad2: 0.5,
    peak_p: 0.45,
    regs: &[(68.0, 1.0), (84.0, 3.0), (99.0, 5.0)],
    zeta: 0.3,
    g_lo: 0.4,
    g_hi: 0.68,
    reed_f: 2500.0,
    reed_d: 0.3,
    fc: 1500.0,
    rad_f: 1500.0,
    ..BASE
};
const BASS_CLARINET: Reed = Reed {
    regs: &[(56.0, 1.0), (72.0, 3.0), (99.0, 5.0)],
    rad2: 1.0,
    reed_f: 1700.0,
    fc: 850.0,
    rad_f: 850.0,
    key_t: 0.008,
    ..CLARINET
};
const CONTRABASS_CLARINET: Reed = Reed {
    regs: &[(44.0, 1.0), (60.0, 3.0), (99.0, 5.0)],
    rad2: 1.0,
    rdir: 0.55,
    reed_f: 1100.0,
    fc: 550.0,
    rad_f: 450.0,
    key_t: 0.01,
    ..CLARINET
};

/// The double reeds (a cone).
const OBOE: Reed = Reed {
    x0: 0.05,
    regs: &[(75.0, 1.0), (87.0, 2.0), (99.0, 3.0)],
    zeta: 0.22,
    g_lo: 0.42,
    g_hi: 0.75,
    reed_f: 3200.0,
    reed_d: 0.4,
    fc: 1400.0,
    rad_f: 1400.0,
    breath_vibrato: true,
    breath: 0.01,
    form: &[(2900.0, 3.0, 2.0)],
    rdir: 0.8,
    rad2: 1.0,
    mp_f: 1150.0,
    mp_q: 2.0,
    mp_h: 3.0,
    tkick: 0.05,
    zkick: 0.0,
    ..BASE
};
const BASSOON: Reed = Reed {
    x0: 0.14,
    regs: &[(53.0, 1.0), (65.0, 2.0), (99.0, 3.0)],
    zeta: 0.25,
    g_lo: 0.42,
    g_hi: 0.75,
    reed_f: 1900.0,
    reed_d: 0.4,
    fc: 650.0,
    rad_f: 650.0,
    breath_vibrato: true,
    form: &[(1150.0, 3.0, 1.6), (2600.0, 2.0, 2.0)],
    key_t: 0.008,
    rdir: 1.2,
    rad2: 1.0,
    mp_f: 480.0,
    mp_q: 2.0,
    mp_h: 5.0,
    ..BASE
};
/// The contrabassoon, rough and buzzy (Cf2: a wider reed, a brighter bocal, an edge at 1.2 kHz).
const CONTRABASSOON: Reed = Reed {
    x0: 0.25,
    regs: &[(41.0, 1.0), (53.0, 2.0), (99.0, 3.0)],
    zeta: 0.27,
    g_lo: 0.42,
    g_hi: 0.75,
    reed_f: 1300.0,
    reed_d: 0.4,
    fc: 420.0,
    rad_f: 420.0,
    breath_vibrato: true,
    form: &[(700.0, 3.0, 1.5), (1700.0, 2.0, 2.0), (1200.0, 6.0, 1.5)],
    key_t: 0.01,
    rdir: 1.0,
    rad2: 1.0,
    mp_f: 300.0,
    mp_q: 2.0,
    mp_h: 3.5,
    ..BASE
};

/// The Highland bagpipe's chanter: a conical double reed blown from the bag (no tongue, never stops).
const CHANTER: Reed = Reed {
    x0: 0.04,
    regs: &[(99.0, 1.0)],
    zeta: 0.25,
    g_lo: 0.55,
    g_hi: 0.75,
    reed_f: 3800.0,
    reed_d: 0.3,
    fc: 2400.0,
    rad_f: 2400.0,
    bag: true,
    key_t: 0.002,
    keys: 0.0,
    breath: 0.006,
    air: 0.0,
    jitter: 0.002,
    drift: 0.5,
    form: &[(3500.0, 3.0, 1.5)],
    rdir: 1.0,
    rad2: 0.5,
    mp_f: 1600.0,
    mp_q: 1.2,
    mp_h: 2.0,
    ..BASE
};
/// A drone: a cylindrical pipe sounded by a single cane reed.
const DRONE: Reed = Reed {
    cyl: true,
    regs: &[(99.0, 1.0)],
    zeta: 0.3,
    g_lo: 0.55,
    g_hi: 0.7,
    reed_f: 2200.0,
    reed_d: 0.25,
    fc: 1800.0,
    rad_f: 1800.0,
    bag: true,
    keys: 0.0,
    breath: 0.004,
    air: 0.0,
    jitter: 0.002,
    drift: 0.4,
    ..BASE
};

/// The musette accordion: three reeds a key, one 14 cents flat and one 14 sharp, a bright reed block.
const MUSETTE: Reed = Reed {
    free: true,
    bank: &[(0.0, -14.0, 0.85), (0.0, 0.0, 1.0), (0.0, 14.0, 0.85)],
    cav_f: 2400.0,
    cav_db: 6.0,
    ..BASE
};

impl ReedParts {
    /// Ss2, the soprano saxophone, Jazz-rock: quick and bright.
    pub const SS2: ReedParts = ReedParts {
        reed: &SOPRANO,
        drones: None,
        vibrato: (18.0, 5.8, 0.25),
    };
    /// As2, the alto saxophone, Jazz-rock.
    pub const AS2: ReedParts = ReedParts {
        reed: &ALTO,
        drones: None,
        vibrato: (16.0, 5.4, 0.3),
    };
    /// Ts2, the tenor saxophone, Jazz-rock.
    pub const TS2: ReedParts = ReedParts {
        reed: &TENOR,
        drones: None,
        vibrato: (18.0, 5.0, 0.35),
    };
    /// Bs2, the baritone saxophone, Jazz-rock.
    pub const BS2: ReedParts = ReedParts {
        reed: &BARI,
        drones: None,
        vibrato: (15.0, 4.8, 0.35),
    };
    /// Cl1, the French Boehm clarinet: dark low, clear top, no vibrato of its own.
    pub const CL1: ReedParts = ReedParts {
        reed: &CLARINET,
        drones: None,
        vibrato: (0.0, 5.0, 0.3),
    };
    /// Bc1, the bass clarinet: deep and woody.
    pub const BC1: ReedParts = ReedParts {
        reed: &BASS_CLARINET,
        drones: None,
        vibrato: (0.0, 5.0, 0.3),
    };
    /// Cc1, the contrabass clarinet.
    pub const CC1: ReedParts = ReedParts {
        reed: &CONTRABASS_CLARINET,
        drones: None,
        vibrato: (0.0, 5.0, 0.3),
    };
    /// Ob1, the French oboe, with a breath vibrato.
    pub const OB1: ReedParts = ReedParts {
        reed: &OBOE,
        drones: None,
        vibrato: (16.0, 5.4, 0.3),
    };
    /// Bn1, the German (Heckel) bassoon: warm and round, a breath vibrato.
    pub const BN1: ReedParts = ReedParts {
        reed: &BASSOON,
        drones: None,
        vibrato: (15.0, 5.0, 0.3),
    };
    /// Cf2, the contrabassoon, rough and buzzy.
    pub const CF2: ReedParts = ReedParts {
        reed: &CONTRABASSOON,
        drones: None,
        vibrato: (12.0, 4.6, 0.35),
    };
    /// Bp1, the Great Highland bagpipe: its chanter, and two tenor drones an octave over the bass drone (the second
    /// tenor 3 cents sharp, so they beat), struck in one after another.
    pub const BP1: ReedParts = ReedParts {
        reed: &CHANTER,
        drones: Some(&Drones {
            reed: DRONE,
            lines: [(12.0, 0.42, 0.0), (12.03, 0.42, 0.03), (0.0, 0.5, 0.07)],
        }),
        vibrato: (0.0, 5.0, 0.3),
    };
    /// Ac1, the musette accordion.
    pub const AC1: ReedParts = ReedParts {
        reed: &MUSETTE,
        drones: None,
        vibrato: (0.0, 5.0, 0.3),
    };
}

// =====================================================================================================================
// small parts

/// The sketch's noise: a xorshift with its uniform and its normal draws.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn uni(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn gauss(&mut self) -> f64 {
        let u1 = self.uni().max(1e-12);
        let u2 = self.uni();
        (-2.0 * dmath::ln(u1)).sqrt() * dmath::cos(2.0 * dmath::PI * u2)
    }
}

fn hz(m: f64) -> f64 {
    440.0 * dmath::pow(2.0, (m - 69.0) / 12.0)
}

/// `e^-x`, or 0 once it is too small to move `1 + c e^-x` from 1 (`c` under a thousand): what a note's start
/// adds to its breath or loudness, `1 + c e^-x`, is then exactly 1 either way, and the exponential is not worked out.
fn gone(x: f64) -> f64 {
    if x > 45.0 { 0.0 } else { dmath::exp(-x) }
}

/// A smooth step from 0 to 1 over `0..1`.
fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A state-variable filter (topology-preserving): low-pass, band-pass and high-pass at once.
#[derive(Clone, Copy, Default)]
struct Svf {
    g: f64,
    r: f64,
    s1: f64,
    s2: f64,
    /// The frequency and rate `g` was set for (so an unchanged one is not worked out again).
    f_at: f64,
    fs_at: f64,
}

impl Svf {
    fn new(f: f64, r: f64, fs: f64) -> Svf {
        let mut s = Svf::default();
        s.set(f, r, fs);
        s
    }
    fn set(&mut self, f: f64, r: f64, fs: f64) {
        self.r = r;
        if f.to_bits() == self.f_at.to_bits() && fs.to_bits() == self.fs_at.to_bits() {
            return;
        }
        self.g = dmath::tan(dmath::PI * (f / fs).min(0.49));
        (self.f_at, self.fs_at) = (f, fs);
    }
    /// Set as `o` is (the same as [`Svf::set`] with its frequency, sharpness and rate).
    fn set_as(&mut self, o: &Svf) {
        (self.g, self.r, self.f_at, self.fs_at) = (o.g, o.r, o.f_at, o.fs_at);
    }
    fn tick(&mut self, x: f64) -> (f64, f64, f64) {
        let g = self.g;
        let hp = (x - (2.0 * self.r + g) * self.s1 - self.s2) / (1.0 + 2.0 * self.r * g + g * g);
        let bp = g * hp + self.s1;
        let lp = g * bp + self.s2;
        self.s1 = g * hp + bp;
        self.s2 = g * bp + lp;
        (lp, bp, hp)
    }
}

/// A one-pole low-pass.
#[derive(Clone, Copy, Default)]
struct One {
    a: f64,
    y: f64,
}

impl One {
    fn new(f: f64, fs: f64) -> One {
        One {
            a: dmath::exp(-2.0 * dmath::PI * f / fs),
            y: 0.0,
        }
    }
    fn lp(&mut self, x: f64) -> f64 {
        self.y = x + self.a * (self.y - x);
        self.y
    }
}

/// An RBJ biquad.
#[derive(Clone, Copy, Default)]
struct Bq {
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}

impl Bq {
    fn peak(f: f64, db: f64, q: f64, fs: f64) -> Bq {
        let a = dmath::pow(10.0, db / 40.0);
        let w = 2.0 * dmath::PI * f / fs;
        let al = dmath::sin(w) / (2.0 * q);
        let c = dmath::cos(w);
        let a0 = 1.0 + al / a;
        Bq {
            b: [(1.0 + al * a) / a0, -2.0 * c / a0, (1.0 - al * a) / a0],
            a: [-2.0 * c / a0, (1.0 - al / a) / a0],
            z: [0.0; 2],
        }
    }
    fn lowpass(f: f64, q: f64, fs: f64) -> Bq {
        let w = 2.0 * dmath::PI * f.min(0.45 * fs) / fs;
        let al = dmath::sin(w) / (2.0 * q);
        let c = dmath::cos(w);
        let a0 = 1.0 + al;
        Bq {
            b: [(1.0 - c) / 2.0 / a0, (1.0 - c) / a0, (1.0 - c) / 2.0 / a0],
            a: [-2.0 * c / a0, (1.0 - al) / a0],
            z: [0.0; 2],
        }
    }
    fn highpass(f: f64, q: f64, fs: f64) -> Bq {
        let w = 2.0 * dmath::PI * f / fs;
        let al = dmath::sin(w) / (2.0 * q);
        let c = dmath::cos(w);
        let a0 = 1.0 + al;
        Bq {
            b: [(1.0 + c) / 2.0 / a0, -(1.0 + c) / a0, (1.0 + c) / 2.0 / a0],
            a: [-2.0 * c / a0, (1.0 - al) / a0],
            z: [0.0; 2],
        }
    }
    fn tick(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}

// =====================================================================================================================
// a line of notes, as the sketch has it

/// How a note of a line starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    /// Tongued.
    Tongue,
    /// Slurred from the note before.
    Slur,
    /// The breath alone.
    Breath,
    /// The keys slapped shut, no breath.
    Click,
}

/// A note of a line (the sketch's `Note`, its names in brackets where they differ).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineNote {
    /// When it starts, s; its pitch, MIDI; how long it is held, s; how hard, 0 to 1.
    pub t: f64,
    pub midi: f64,
    pub dur: f64,
    pub vel: f64,
    /// How it starts (`art`).
    pub start: Start,
    /// An accent (`acc`).
    pub accent: f64,
    /// Stopped by the tongue (`end=t`).
    pub stop_tongue: bool,
    /// Tied into a note that is not in the line: the breath goes on past its end.
    pub tie: bool,
    /// A swell of the strength (from, to, over s).
    pub swell: Option<(f64, f64, f64)>,
    /// A vibrato (cents, Hz, wait s).
    pub vib: Option<(f64, f64, f64)>,
    /// A bend (cents, at s, over s).
    pub bend: Option<(f64, f64, f64)>,
    /// A scoop up into the note: from so many cents under, held so long, over so long (`scoop`, with its hold).
    pub scoop: Option<(f64, f64, f64)>,
    /// A fall after the end (cents down, over s); a negative one is a doit.
    pub fall: Option<(f64, f64)>,
    /// A smear from the note before, over s (`gliss`).
    pub gliss: Option<f64>,
    /// A growl, its hum's pitch (none: the instrument's `growl_f`), flutter tongue, slap tongue, an air tone, a
    /// subtone: each 0 to 1.
    pub growl: f64,
    pub growl_hz: Option<f64>,
    pub flutter: f64,
    pub slap: f64,
    pub airtone: f64,
    pub sub: f64,
    /// A back or soft tongue: the tongue's push and the reed's wider swing times this, the tongue this much slower.
    pub kick: f64,
    pub slow: f64,
    /// The register forced (which resonance), if any (`reg`).
    pub reg: Option<f64>,
    /// An accordion's bellows shake (turns a second, depth).
    pub shake: Option<(f64, f64)>,
    /// An accordion's bank (none: its own).
    pub bank: Option<Register>,
    /// An accordion's bellows vibrato (a part of the pressure, Hz), and its bend (cents, at s, over s).
    pub wobble: Option<(f64, f64)>,
}

impl LineNote {
    /// A plain tongued note.
    pub fn plain(t: f64, midi: f64, dur: f64, vel: f64) -> LineNote {
        LineNote {
            t,
            midi,
            dur,
            vel,
            start: Start::Tongue,
            accent: 0.0,
            stop_tongue: false,
            tie: false,
            swell: None,
            vib: None,
            bend: None,
            scoop: None,
            fall: None,
            gliss: None,
            growl: 0.0,
            growl_hz: None,
            flutter: 0.0,
            slap: 0.0,
            airtone: 0.0,
            sub: 0.0,
            kick: 1.0,
            slow: 1.0,
            reg: None,
            shake: None,
            bank: None,
            wobble: None,
        }
    }
}

/// What one note asks of the bore (the fingering): the bore's lowest resonance, which resonance it plays on, the
/// note, and the player's voicing for it (pressure, lip and throat).
#[derive(Clone, Copy, Debug)]
struct Finger {
    fb: f64,
    k: f64,
    f: f64,
    gs: f64,
    zs: f64,
    rs: f64,
}

/// The most resonances a bore keeps.
const MODES: usize = 160;
/// How far (a natural log of the ratio) a resonance moves at a change of register, at least.
const JUMP: f64 = 0.2;

/// The bore as the reed sees it: its resonances (each a damped mode, its height and sharpness set by the bore's
/// losses at its frequency, falling away above the open end's cutoff), the bore's own impedance for what lies above
/// them, the mouthpiece's broad resonance.
struct Bore<'a> {
    inst: &'a Reed,
    fs: f64,
    n: usize,
    g: [f64; MODES],
    r: [f64; MODES],
    amp: [f64; MODES],
    s1: [f64; MODES],
    s2: [f64; MODES],
    /// Each resonance's filter worked out once a fingering is set: `1 / (1 + 2rg + g^2)`, `2r + g`, and the weights
    /// of its two states in the bore's pressure (`amp (1 - g (2r + g) / d)`, `-amp g / d`).
    inv: [f64; MODES],
    c: [f64; MODES],
    w1: [f64; MODES],
    w2: [f64; MODES],
    /// The resonances' part of the bore's response to the flow (the sum of `amp g / d`).
    a_modes: f64,
    /// Each overtone's place over the lowest resonance, `h (1 + inh (h - 1))`, raised to `-peak_p`.
    tilt: [f64; MODES],
    mp: Svf,
    mp_k: f64,
    /// The mouthpiece's resonance's parts of the bore's response, worked out once (its filter never changes) and
    /// when its weight `mp_k` does: `1 + 2rg + g^2`, `2r + g`, its weight `mp_h mp_k 2r` and that weight's part of the
    /// response to the flow (`weight g / d`).
    mp_d: f64,
    mp_c: f64,
    mp_a: f64,
    mp_aa: f64,
    x_prev: f64,
    reed: Svf,
    rad: One,
    rad_b: One,
    hp_a: f64,
    hp_y: f64,
    hp_v: f64,
    last_fb: f64,
    last_mask: f64,
    /// The bounds of a resonance's move that make a change of register (`e^JUMP`, `e^-JUMP`).
    jump: (f64, f64),
    /// A trial run's bore, without breath or air.
    quiet: bool,
    /// The register vent's masks of the resonances (and each one's part of its sharpness) for the fingering
    /// `masks_at` (`ka`, `kb`, `w`), worked out for the first `masks_n`.
    masks: [f64; MODES],
    masks_q: [f64; MODES],
    masks_at: (f64, f64, f64),
    masks_n: usize,
    /// `1 / fc`.
    inv_fc: f64,
    /// Each resonance's overtone number `h`, and its stretch `1 + inh (h - 1)`.
    hs: [f64; MODES],
    stretch: [f64; MODES],
    /// The highest resonance kept, Hz.
    top: f64,
    /// The bounds of a move of the lowest resonance inside which no resonance can move by a change of register's
    /// [`JUMP`] (a resonance's tangent moves by at most `2x / sin 2x` times the lowest's ratio, `x` its angle at the
    /// top; the bounds are half of that), so the resonances are not each compared.
    calm: (f64, f64),
    /// [`Bore::set_modes`]'s working space (only its first `n` places are written and read each time, so it is not
    /// cleared between calls).
    work: Work,
}

/// What [`Bore::set_modes`] works out for each resonance on the way: its frequency, its angle's cosine and sine, the
/// sine of a cone's comb angle, its tangent, half its inverse sharpness, its weight, its comb and its height.
struct Work {
    fr: [f64; MODES],
    tc: [f64; MODES],
    ts: [f64; MODES],
    zs: [f64; MODES],
    g: [f64; MODES],
    rr: [f64; MODES],
    amp: [f64; MODES],
    comb: [f64; MODES],
    height: [f64; MODES],
}

/// How much the register vent leaves of a resonance (overtone number `h`) when the note plays on resonance `k`.
fn vent_mask(inst: &Reed, h: f64, k: f64) -> f64 {
    if k <= 1.0 {
        return 1.0;
    }
    let r = h / k;
    if (r - r.round()).abs() < 1e-6 {
        1.0
    } else if h < k {
        inst.vent_low
    } else {
        inst.vent_high
    }
}

impl<'a> Bore<'a> {
    fn new(inst: &'a Reed, fs: f64) -> Bore<'a> {
        let mp = Svf::new(inst.mp_f, 0.5 / inst.mp_q, fs);
        let mut b = Bore {
            inst,
            fs,
            n: 0,
            g: [0.0; MODES],
            r: [0.0; MODES],
            amp: [0.0; MODES],
            s1: [0.0; MODES],
            s2: [0.0; MODES],
            inv: [0.0; MODES],
            c: [0.0; MODES],
            w1: [0.0; MODES],
            w2: [0.0; MODES],
            a_modes: 0.0,
            tilt: std::array::from_fn(|n| {
                let h = if inst.cyl { (2 * n + 1) as f64 } else { (n + 1) as f64 };
                dmath::pow(h * (1.0 + inst.inh * (h - 1.0)), -inst.peak_p)
            }),
            mp,
            mp_k: 1.0,
            mp_d: 1.0 + 2.0 * mp.r * mp.g + mp.g * mp.g,
            mp_c: 2.0 * mp.r + mp.g,
            mp_a: 0.0,
            mp_aa: 0.0,
            x_prev: 0.0,
            reed: Svf::new(inst.reed_f, inst.reed_d, fs),
            rad: One::new(inst.rad_f, fs),
            rad_b: One::new(inst.rad_f, fs),
            hp_a: {
                let fc = if inst.rdir_f > 0.0 { inst.rdir_f } else { inst.fc };
                1.0 / (1.0 + 2.0 * dmath::PI * fc / fs)
            },
            hp_y: 0.0,
            hp_v: 0.0,
            last_fb: -1.0,
            last_mask: -1.0,
            jump: (dmath::exp(JUMP), dmath::exp(-JUMP)),
            quiet: false,
            masks: [0.0; MODES],
            masks_q: [0.0; MODES],
            masks_at: (f64::NAN, f64::NAN, f64::NAN),
            masks_n: 0,
            inv_fc: 1.0 / inst.fc,
            hs: std::array::from_fn(|n| if inst.cyl { (2 * n + 1) as f64 } else { (n + 1) as f64 }),
            stretch: std::array::from_fn(|n| {
                let h = if inst.cyl { (2 * n + 1) as f64 } else { (n + 1) as f64 };
                1.0 + inst.inh * (h - 1.0)
            }),
            top: (inst.fc * inst.top).min(0.42 * fs).min(9000.0),
            calm: {
                let x = (dmath::PI * (inst.fc * inst.top).min(0.42 * fs).min(9000.0) / fs).clamp(1e-6, 1.5);
                let d = JUMP * 0.5 / (2.0 * x / dmath::sin(2.0 * x));
                (dmath::exp(d), dmath::exp(-d))
            },
            work: Work {
                fr: [0.0; MODES],
                tc: [0.0; MODES],
                ts: [0.0; MODES],
                zs: [0.0; MODES],
                g: [0.0; MODES],
                rr: [0.0; MODES],
                amp: [0.0; MODES],
                comb: [0.0; MODES],
                height: [0.0; MODES],
            },
        };
        b.weigh_mp();
        b
    }

    /// The mouthpiece's weight in the bore's response, after `mp_k` is set.
    fn weigh_mp(&mut self) {
        self.mp_a = self.inst.mp_h * self.mp_k * 2.0 * self.mp.r;
        self.mp_aa = self.mp_a * self.mp.g / self.mp_d;
    }

    /// The resonances for a fingering: the bore's lowest resonance `fb`, a register vent mixed between two fingerings
    /// (`ka` and `kb`, weight `w`), the resonances above the lowest weakened by `hd` (a slur down out of a higher
    /// register: the lip and throat let the upper resonance go so that the low one takes over).
    fn set_modes(&mut self, fb: f64, ka: f64, kb: f64, w: f64, hd: f64) {
        let key = fb * 1000.0 + ka * 10.0 + kb + w * 0.001 + hd * 1e-7;
        if (fb - self.last_fb).abs() < 1e-9 && (key - self.last_mask).abs() < 1e-9 {
            return;
        }
        let old_fb = self.last_fb;
        self.last_fb = fb;
        self.last_mask = key;
        let old_n = self.n;
        let i = self.inst;
        let kk = ka.max(kb);
        {
            // the sounding pitch: the mouthpiece's broad resonance would capture a note that sits on it
            let f0 = fb * kk;
            let r = f0 / i.mp_f.max(1.0);
            self.mp_k = (1.0 - (r - 0.92) / 0.15).clamp(0.0, 1.0);
        }
        self.weigh_mp();
        let top = self.top;
        let base = dmath::pow(fb / 200.0, -i.peak_p);
        if (ka, kb, w) != self.masks_at {
            self.masks_at = (ka, kb, w);
            self.masks_n = 0;
        }
        // (each resonance's working numbers in the bore's own working space: only the first `n` are written and read)
        let Work {
            fr,
            tc,
            ts,
            zs,
            g,
            rr,
            amp,
            comb,
            height,
        } = &mut self.work;
        // the resonances under the top, and their frequencies
        let mut n = 0;
        while n < MODES {
            let f = self.hs[n] * fb * self.stretch[n];
            if f > top {
                break;
            }
            fr[n] = f;
            n += 1;
        }
        // the overtones' angles (each resonance's tangent, a cone's comb) turned on from the lowest, one overtone
        // step at a time, as each is a whole multiple of the lowest's (the same, to rounding, as each worked out on
        // its own, at a fraction of the cost)
        let beta = 2.0 * fb * i.x0 / C;
        let (th, zb) = (dmath::PI * fb / self.fs, dmath::PI * beta);
        let step = if i.cyl { 2.0 } else { 1.0 };
        let turn = |a: f64| (dmath::cos(a), dmath::sin(a));
        // (a cone's step is the lowest's own angle; a cylinder has no comb, so its comb's angles are not wanted)
        let combed = !(i.cyl || i.comb <= 0.0);
        let mut t_at = turn(th);
        let t_step = if step == 1.0 { t_at } else { turn(step * th) };
        let (mut z_at, z_step) = if combed {
            let z = turn(zb);
            (z, if step == 1.0 { z } else { turn(step * zb) })
        } else {
            ((1.0, 0.0), (1.0, 0.0))
        };
        let rot = |(c, s): (f64, f64), (dc, ds): (f64, f64)| (c * dc - s * ds, s * dc + c * ds);
        for j in 0..n {
            if j > 0 {
                t_at = rot(t_at, t_step);
                z_at = rot(z_at, z_step);
            }
            (tc[j], ts[j], zs[j]) = (t_at.0, t_at.1, z_at.1);
        }
        for j in self.masks_n..n {
            let h = self.hs[j];
            self.masks[j] = vent_mask(i, h, ka) * (1.0 - w) + vent_mask(i, h, kb) * w;
            self.masks_q[j] = dmath::PI / if i.cyl { 4.0 } else { 2.0 } * i.qk * self.masks[j].sqrt();
        }
        self.masks_n = self.masks_n.max(n);
        // each resonance's height, sharpness and tangent, in passes of their own over all the resonances, every
        // choice made after both sides are worked out (so the divisions go two at a time)
        // a cone: the reed sits x0 from where the cone's tip would be, a part beta of the whole cone, so the
        // resonances whose pressure has a node near there are weaker (the comb moves with the note)
        if combed {
            for ((c, &h), &s) in comb[..n].iter_mut().zip(&self.hs[..n]).zip(&zs[..n]) {
                let z = dmath::PI * h * beta;
                let s = s / z;
                let sc = if z.abs() < 1e-9 { 1.0 } else { s * s };
                *c = if h <= kk { 1.0 } else { 1.0 - i.comb + i.comb * sc };
            }
        } else {
            comb[..n].fill(1.0);
        }
        for j in 0..n {
            let h = self.hs[j];
            // the bore's resonance at `fr[j]` (in units of the bore's own impedance)
            let q = fr[j] * self.inv_fc;
            let q2 = q * q;
            let at = i.peak * base * self.tilt[j] / (1.0 + q2 * q2);
            height[j] = at * self.masks[j] * comb[j] * if h > 1.0 { hd } else { 1.0 };
        }
        for j in 0..n {
            let q = (height[j] * self.hs[j] * self.masks_q[j]).clamp(3.0, 180.0);
            let inv_q = 1.0 / q;
            rr[j] = 0.5 * inv_q;
            amp[j] = height[j] * inv_q;
        }
        if i.inh == 0.0 {
            for ((g, s), c) in g[..n].iter_mut().zip(&ts[..n]).zip(&tc[..n]) {
                *g = s / c;
            }
        } else {
            for (g, f) in g[..n].iter_mut().zip(&fr[..n]) {
                *g = dmath::tan(dmath::PI * f / self.fs);
            }
        }
        // when the set of resonances jumps (a change of register), each resonance takes over the swing of the old
        // one nearest it in pitch, so the sound carries on where it was instead of leaping to another resonance
        let (up, down) = self.jump;
        let moved = fb / old_fb;
        let calm = old_fb > 0.0 && moved < self.calm.0 && moved > self.calm.1;
        let jumped = !calm
            && (0..n.min(old_n)).any(|j| {
                let r = g[j] / self.g[j];
                r > up || r < down
            });
        if jumped && old_n > 0 {
            // a one-to-one matching (nearest pairs first), so no swing is lost or doubled; what is left over keeps
            // its place in the order
            let mut pairs: Vec<(f64, usize, usize)> = Vec::with_capacity(n * old_n);
            for (j, &gj) in g.iter().enumerate().take(n) {
                for (jo, &go) in self.g.iter().enumerate().take(old_n) {
                    pairs.push((dmath::ln(gj / go).abs(), j, jo));
                }
            }
            pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut new_of: Vec<Option<usize>> = vec![None; n];
            let mut used = vec![false; old_n];
            for &(d, j, jo) in &pairs {
                if d < 0.25 && new_of[j].is_none() && !used[jo] {
                    new_of[j] = Some(jo);
                    used[jo] = true;
                }
            }
            let mut rest = (0..old_n).filter(|&jo| !used[jo]);
            for slot in new_of.iter_mut() {
                if slot.is_none() {
                    *slot = rest.next();
                }
            }
            // (scaled so that each resonance's part of the mouthpiece pressure carries on without a step)
            let (o1, o2, oa) = (self.s1, self.s2, self.amp);
            for j in 0..n {
                let (a, b) = new_of[j].map_or((0.0, 0.0), |jo| {
                    let k = oa[jo] / amp[j].max(1e-12);
                    (o1[jo] * k, o2[jo] * k)
                });
                self.s1[j] = a;
                self.s2[j] = b;
            }
        }
        for j in n..old_n {
            self.s1[j] = 0.0;
            self.s2[j] = 0.0;
        }
        self.g[..n].copy_from_slice(&g[..n]);
        self.r[..n].copy_from_slice(&rr[..n]);
        self.amp[..n].copy_from_slice(&amp[..n]);
        self.n = n;
        for j in 0..n {
            let (g, r, a) = (g[j], rr[j], amp[j]);
            let inv = 1.0 / (1.0 + 2.0 * r * g + g * g);
            let c = 2.0 * r + g;
            self.inv[j] = inv;
            self.c[j] = c;
            self.w1[j] = a * (1.0 - g * c * inv);
            self.w2[j] = -a * g * inv;
        }
        // (the sum of `amp g / d`, which is each `-w2`, added in order)
        self.a_modes = self.w2[..n].iter().fold(0.0, |s, w| s + -w);
    }

    /// One inner sample: `gamma` the mouth pressure, `tau` the tongue on the reed (0 to 1); the reed's opening, its
    /// resonance and damping; breath noise, air heard directly, a flow impulse (a key click), and the breath's and the
    /// air's noise this sample (`noise`, from [`LineNoise`]; 0 in a trial run). Returns the pressure in the
    /// mouthpiece and the sound radiated.
    #[allow(clippy::too_many_arguments)]
    fn tick(
        &mut self,
        gamma: f64,
        tau: f64,
        zeta: f64,
        reed_f: f64,
        reed_d: f64,
        breath: f64,
        air: f64,
        kick: f64,
        noise: (f64, f64),
    ) -> (f64, f64) {
        // the reed: pushed shut by the pressure difference, and by the tongue
        self.reed.set(reed_f, reed_d, self.fs);
        let (xi, _, _) = self.reed.tick(-(gamma - self.x_prev) - 1.5 * tau);
        let o = (1.0 + xi).max(0.0) * (1.0 - tau).max(0.0);
        let zo = zeta * o;
        // the bore: x = A v + B; its own impedance above the resonances a high-pass of the flow
        let n = self.n;
        let mut aa = self.inst.rdir * self.hp_a + self.a_modes;
        let mut bb = self.inst.rdir * self.hp_a * (self.hp_y - self.hp_v);
        let mut two = [0.0; 2];
        let (s1, s2) = (self.s1[..n].as_chunks::<2>().0, self.s2[..n].as_chunks::<2>().0);
        let (w1, w2) = (self.w1[..n].as_chunks::<2>().0, self.w2[..n].as_chunks::<2>().0);
        for (((s1, s2), w1), w2) in s1.iter().zip(s2).zip(w1).zip(w2) {
            two[0] += w1[0] * s1[0] + w2[0] * s2[0];
            two[1] += w1[1] * s1[1] + w2[1] * s2[1];
        }
        bb += two[0] + two[1];
        if n % 2 == 1 {
            let j = n - 1;
            bb += self.w1[j] * self.s1[j] + self.w2[j] * self.s2[j];
        }
        let mpg = self.mp.g;
        aa += self.mp_aa;
        bb += self.mp_a * (self.mp.s1 - mpg * (self.mp_c * self.mp.s1 + self.mp.s2) / self.mp_d);
        let dd = gamma - bb;
        let z2 = zo * zo;
        let v = if zo <= 0.0 {
            0.0
        } else if dd >= 0.0 {
            0.5 * (-z2 * aa + (z2 * z2 * aa * aa + 4.0 * z2 * dd).sqrt())
        } else {
            0.5 * (z2 * aa - (z2 * z2 * aa * aa - 4.0 * z2 * dd).sqrt())
        };
        // turbulence: a flow noise that follows the flow
        let nb = if self.quiet { 0.0 } else { noise.0 };
        let vn = v + breath * v.abs().sqrt() * nb * 2.0 + kick;
        let x = aa * vn + bb;
        // (by index over the whole arrays, so the resonances go two at a time)
        let (s1, s2) = (&mut self.s1, &mut self.s2);
        let (gs, cs, invs) = (&self.g, &self.c, &self.inv);
        for j in 0..n.min(MODES) {
            let (g, a, b) = (gs[j], s1[j], s2[j]);
            let hp = (vn - cs[j] * a - b) * invs[j];
            let bp = g * hp + a;
            let lp = g * bp + b;
            s1[j] = g * hp + bp;
            s2[j] = g * bp + lp;
        }
        self.mp.tick(vn);
        self.hp_y = self.hp_a * (self.hp_y + vn - self.hp_v);
        self.hp_v = vn;
        self.x_prev = x;
        // what comes out of the bell and the open holes: the pressure's swing, rising with frequency up to the cutoff
        let low = self.rad.lp(x);
        let mut out = x - low;
        if self.inst.rad2 > 0.0 {
            let l2 = self.rad_b.lp(out);
            out -= self.inst.rad2 * l2;
        }
        if !self.quiet {
            out += air * (gamma.max(0.0) * o).sqrt() * noise.1;
        }
        (x, out)
    }
}

/// Which resonance a note plays on, by the instrument's registers.
fn reg_of(inst: &Reed, midi: f64) -> f64 {
    for &(top, k) in inst.regs {
        if midi <= top + 1e-9 {
            return k;
        }
    }
    inst.regs.last().map_or(1.0, |r| r.1)
}

fn gamma_of(inst: &Reed, vel: f64) -> f64 {
    inst.g_lo + (inst.g_hi - inst.g_lo) * vel.clamp(0.0, 1.2)
}

/// The fingering for a pitch on a register, before tuning.
fn finger_raw(midi: f64, k: f64) -> Finger {
    let f = hz(midi);
    Finger {
        fb: f / k,
        k,
        f,
        gs: 1.0,
        zs: 1.0,
        rs: 1.0,
    }
}

/// The pitch a fingering plays at a steady pressure, Hz (0 if it does not speak, or plays another resonance).
fn measure(inst: &Reed, fg: &Finger, gamma: f64, zeta: f64, fs: f64) -> f64 {
    let mut b = Bore::new(inst, fs);
    // no breath noise and no air in a trial run (the noise would be multiplied away)
    b.quiet = true;
    b.set_modes(fg.fb, fg.k, fg.k, 0.0, 1.0);
    let n = (0.32 * fs) as usize;
    let mut xs = Vec::with_capacity(n);
    let gamma = gamma * fg.gs;
    let zeta = zeta * fg.zs;
    for i in 0..n {
        let t = i as f64 / fs;
        // as a tongued note starts: the breath built up behind the tongue, then the tongue lets go
        let g = gamma * smooth(t / 0.025);
        let tau = 1.0 - smooth((t - 0.03) / inst.tongue_t);
        let (x, _) = b.tick(
            g,
            tau,
            zeta,
            inst.reed_f * fg.rs,
            inst.reed_d,
            0.0,
            0.0,
            0.0,
            (0.0, 0.0),
        );
        xs.push(x);
    }
    let seg = &xs[(0.17 * fs) as usize..];
    let f = pitch_of(seg, fg.f, fs);
    if f > 0.0 && plays_its_note(seg, f, fs) { f } else { 0.0 }
}

/// Whether a stretch is periodic at `f` itself (its first overtone is there), not at a multiple of it.
fn plays_its_note(x: &[f64], f: f64, fs: f64) -> bool {
    let m = x.iter().sum::<f64>() / x.len() as f64;
    let n = x.len();
    // the Hann window of this length, worked out once
    static WINDOWS: OnceLock<Mutex<HashMap<usize, Arc<[f64]>>>> = OnceLock::new();
    let windows = WINDOWS.get_or_init(|| Mutex::new(HashMap::new()));
    let made = windows.lock().ok().and_then(|w| w.get(&n).cloned());
    let win = made.unwrap_or_else(|| {
        let w: Arc<[f64]> = (0..n)
            .map(|i| 0.5 - 0.5 * dmath::cos(2.0 * dmath::PI * i as f64 / n as f64))
            .collect();
        if let Ok(mut m) = windows.lock() {
            m.insert(n, Arc::clone(&w));
        }
        w
    });
    let lev = |fr: f64| -> f64 {
        let (mut c, mut s) = (0.0, 0.0);
        // the phase `2 pi fr i / fs` turned on one sample at a time (worked out afresh every 1024, so the turning
        // never drifts)
        let d = 2.0 * dmath::PI * fr / fs;
        let (dc, ds) = (dmath::cos(d), dmath::sin(d));
        let (mut pc, mut ps) = (1.0, 0.0);
        for ((i, v), w) in x.iter().enumerate().zip(win.iter()) {
            if i % 1024 == 0 {
                let ph = d * i as f64;
                (pc, ps) = (dmath::cos(ph), dmath::sin(ph));
            }
            c += (v - m) * w * pc;
            s += (v - m) * w * ps;
            (pc, ps) = (pc * dc - ps * ds, ps * dc + pc * ds);
        }
        (c * c + s * s).sqrt()
    };
    let h1 = lev(f);
    let best = (1..8)
        .map(|k| if k == 1 { h1 } else { lev(f * f64::from(k)) })
        .fold(0.0, f64::max);
    h1 > best * 0.06
}

/// The pitch of a steady stretch, by autocorrelation near the expected period.
fn pitch_of(x: &[f64], f_guess: f64, fs: f64) -> f64 {
    let m = x.iter().sum::<f64>() / x.len() as f64;
    let y: Vec<f64> = x.iter().map(|v| v - m).collect();
    let e: f64 = y.iter().map(|v| v * v).sum::<f64>();
    if e < 1e-12 * y.len() as f64 {
        return 0.0;
    }
    let t0 = fs / f_guess;
    let lo = (t0 * 0.8) as usize;
    let hi = (t0 * 1.25) as usize + 2;
    if hi + 1 >= y.len() || lo < 1 {
        return 0.0;
    }
    let n = y.len() - hi - 1;
    // (four running sums, so the products are added four at a time)
    let ac = |l: usize| -> f64 {
        let ((ca, ra), (cb, rb)) = (y[..n].as_chunks::<4>(), y[l..l + n].as_chunks::<4>());
        let mut s = [0.0; 4];
        for (p, q) in ca.iter().zip(cb) {
            for k in 0..4 {
                s[k] += p[k] * q[k];
            }
        }
        let mut t = (s[0] + s[1]) + (s[2] + s[3]);
        for (p, q) in ra.iter().zip(rb) {
            t += p * q;
        }
        t
    };
    // four lags at a time, each lag's products added in the same order as `ac` adds them (the same numbers, with the
    // sixteen sums not waiting on each other)
    let ac4 = |l: usize| -> [f64; 4] {
        let (ca, ra) = y[..n].as_chunks::<4>();
        let b: [(&[[f64; 4]], &[f64]); 4] = std::array::from_fn(|d| y[l + d..l + d + n].as_chunks::<4>());
        let mut s = [[0.0; 4]; 4];
        let lags = b[0].0.iter().zip(b[1].0).zip(b[2].0).zip(b[3].0);
        for (p, (((q0, q1), q2), q3)) in ca.iter().zip(lags) {
            for k in 0..4 {
                s[0][k] += p[k] * q0[k];
                s[1][k] += p[k] * q1[k];
                s[2][k] += p[k] * q2[k];
                s[3][k] += p[k] * q3[k];
            }
        }
        std::array::from_fn(|d| {
            let mut t = (s[d][0] + s[d][1]) + (s[d][2] + s[d][3]);
            for (p, q) in ra.iter().zip(b[d].1) {
                t += p * q;
            }
            t
        })
    };
    let mut vals: Vec<f64> = Vec::with_capacity(hi + 2 - lo);
    let mut l = lo;
    while l + 3 <= hi + 1 {
        vals.extend(ac4(l));
        l += 4;
    }
    vals.extend((l..=hi + 1).map(ac));
    let mut best = lo;
    let mut bv = f64::MIN;
    for l in lo + 1..=hi {
        let v = vals[l - lo];
        if v > bv && v >= vals[l - lo - 1] && v >= vals[l - lo + 1] {
            bv = v;
            best = l;
        }
    }
    if best == lo || bv < 0.3 * e * n as f64 / y.len() as f64 {
        return 0.0;
    }
    let (a, b, c) = (vals[best - lo - 1], vals[best - lo], vals[best - lo + 1]);
    let den = a - 2.0 * b + c;
    let off = if den.abs() > 1e-20 { 0.5 * (a - c) / den } else { 0.0 };
    fs / (best as f64 + off)
}

/// The ways a player voices a note that will not speak cleanly: pressure, lip and throat, in the order tried.
const VOICINGS: [(f64, f64, f64); 9] = [
    (1.0, 1.0, 1.0),
    (0.93, 1.0, 1.0),
    (1.0, 0.85, 1.0),
    (0.93, 0.85, 1.15),
    (1.07, 1.0, 1.0),
    (1.0, 1.15, 0.85),
    (0.87, 0.8, 1.2),
    (1.0, 1.0, 0.75),
    (1.12, 1.1, 1.0),
];

/// The fingering tuned so that it plays its note at this pressure (as a maker voices the tone holes), and the
/// player's voicing for it: if the note does not speak cleanly, a player blows a little harder or softer, firms or
/// loosens the lip, or changes the throat, until it does. Each is worked out once and kept: the same question always
/// has the same answer.
fn tuned(inst: &Reed, midi: f64, k: f64, gamma: f64, zeta: f64, fs: f64) -> Finger {
    type Key = Vec<u64>;
    static TUNED: OnceLock<Mutex<HashMap<Key, Finger>>> = OnceLock::new();
    let key: Key = [
        midi,
        k,
        gamma,
        zeta,
        fs,
        inst.x0,
        inst.comb,
        inst.vent_low,
        inst.vent_high,
        inst.peak,
        inst.peak_p,
        inst.qk,
        inst.inh,
        inst.top,
        inst.rdir,
        inst.rdir_f,
        inst.mp_f,
        inst.mp_q,
        inst.mp_h,
        inst.reed_f,
        inst.reed_d,
        inst.fc,
        inst.tongue_t,
        f64::from(u8::from(inst.cyl)),
    ]
    .iter()
    .map(|v| v.to_bits())
    .collect();
    let memo = TUNED.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(f) = memo.lock().ok().and_then(|m| m.get(&key).copied()) {
        return f;
    }
    let f = tune(inst, midi, k, gamma, zeta, fs);
    if let Ok(mut m) = memo.lock() {
        m.insert(key, f);
    }
    f
}

/// [`tuned`]'s trial runs.
fn tune(inst: &Reed, midi: f64, k: f64, gamma: f64, zeta: f64, fs: f64) -> Finger {
    for &(gs, zs, rs) in &VOICINGS {
        let mut fg = finger_raw(midi, k);
        fg.gs = gs;
        fg.zs = zs;
        fg.rs = rs;
        let mut ok = false;
        for _ in 0..4 {
            let f = measure(inst, &fg, gamma, zeta, fs);
            if f <= 0.0 {
                ok = false;
                break;
            }
            ok = true;
            let r = f / fg.f;
            if (dmath::ln(r) * 1200.0 / std::f64::consts::LN_2).abs() < 0.2 {
                break;
            }
            fg.fb /= r;
        }
        if ok {
            return fg;
        }
    }
    finger_raw(midi, k)
}

/// How many samples of a line's noise are drawn at a time, and how many such blocks may wait drawn ahead.
const NOISE_BLOCK: usize = 2048;
const NOISE_AHEAD: usize = 16;

/// What a line needs to draw its noise ahead of it ([`render_line`]): none of it hangs on the sound, so it is drawn on
/// a thread of its own while the line plays.
struct LineNoise<'a> {
    inst: &'a Reed,
    /// The line's notes, in order.
    notes: &'a [LineNote],
    /// The line's own stream, as the knocks left it.
    line: Rng,
    seed: u64,
    fs: f64,
    len: usize,
}

impl LineNoise<'_> {
    /// For each sample, in blocks: the growl's draw (0 where the line draws none), the jitter and the slow drift
    /// (each filtered), the breath's and the air's noise (each filtered): from the streams the line would draw them
    /// (its own and the bore's) and in its order, so the line sounds the same as drawing them itself.
    fn draw(mut self, to: &std::sync::mpsc::SyncSender<Vec<[f64; 5]>>) {
        let (inst, notes, fs) = (self.inst, self.notes, self.fs);
        let nn = notes.len();
        // slow unsteadiness of breath and pitch
        let mut jit = One::new(3.0, fs);
        let mut drift = One::new(0.7, fs);
        // the bore's turbulence and the air heard directly
        let mut bore = Rng::new(self.seed);
        let mut nz = Svf::new((inst.noise_f.0 * inst.noise_f.1).sqrt(), 0.9, fs);
        let mut nz2 = Svf::new(5000.0, 0.7, fs);
        let mut cur = 0usize;
        let mut s = 0;
        while s < self.len {
            let end = (s + NOISE_BLOCK).min(self.len);
            let mut block = Vec::with_capacity(end - s);
            for i in s..end {
                let t = i as f64 / fs;
                while cur + 1 < nn && notes[cur + 1].t <= t {
                    cur += 1;
                }
                let n = &notes[cur];
                let growl = if t >= n.t && t < n.t + n.dur + 0.05 && n.growl > 0.0 {
                    self.line.gauss()
                } else {
                    0.0
                };
                let jt = jit.lp(self.line.gauss());
                let slow = drift.lp(self.line.gauss());
                let nb = nz.tick(bore.gauss()).1;
                let nh = nz2.tick(bore.gauss()).2;
                block.push([growl, jt, slow, nb, nh]);
            }
            if to.send(block).is_err() {
                return;
            }
            s = end;
        }
    }
}

/// The pads' knock: a short damped burst (the key's ring and the pad's thud).
struct Knock {
    t: f64,
    f1: f64,
    f2: f64,
    amp: f64,
}

/// How far under the line's loudest a stretch of [`QUIET_BLOCK`] must have fallen, past its ring, for the line to end
/// there ([`render_line`]'s `quiet_after`): 60 dB.
const QUIET: f64 = 1e-3;
/// The stretch the line's quiet is judged over (and its last faded over), s.
const QUIET_BLOCK: f64 = 0.01;

/// A line of notes on a reed and bore (the sketch's `render_line`), mono at the reed's own rate `fs` (twice the heard
/// rate), `len` samples long. From sample `quiet_after` on (a note's ring is over: its breath long gone, the bore
/// fallen to the floor its own flow's noise keeps up), the line ends at the first 10 ms that are 60 dB under its
/// loudest, faded over them, and is silent after; `len` or more plays the whole line.
#[allow(clippy::too_many_lines)]
fn render_line(inst: &Reed, notes: &[LineNote], seed: u64, fs: f64, len: usize, quiet_after: usize) -> Vec<f64> {
    let mut notes = notes.to_vec();
    notes.sort_by(|a, b| a.t.total_cmp(&b.t));
    if notes.is_empty() {
        return vec![0.0; len];
    }
    if inst.bag {
        for n in notes.iter_mut().skip(1) {
            n.start = Start::Slur;
        }
    }
    let nn = notes.len();
    // each note's fingering, tuned at its main pressure
    let fingers: Vec<Finger> = notes
        .iter()
        .map(|n| {
            let k = n.reg.unwrap_or_else(|| reg_of(inst, n.midi));
            let v = match n.swell {
                Some((a, b, _)) => 0.5 * (a + b),
                None => n.vel,
            };
            let zeta = inst.zeta * (1.0 - 0.45 * n.sub);
            tuned(inst, n.midi, k, gamma_of(inst, v), zeta, fs)
        })
        .collect();
    // how each note joins the next: tied (slurred into), tongued close after (legato tonguing), or apart
    let joined: Vec<bool> = (0..nn)
        .map(|i| {
            notes[i].tie
                || (i + 1 < nn
                    && notes[i + 1].start == Start::Slur
                    && notes[i + 1].t - (notes[i].t + notes[i].dur) < 0.06)
        })
        .collect();
    let close: Vec<bool> = (0..nn)
        .map(|i| {
            i + 1 < nn && notes[i + 1].start == Start::Tongue && notes[i + 1].t - (notes[i].t + notes[i].dur) < 0.12
        })
        .collect();
    // when the tongue lands before each tongued note
    let tongue_on: Vec<f64> = (0..nn)
        .map(|i| {
            let n = &notes[i];
            if i > 0 && (close[i - 1] || notes[i - 1].stop_tongue) {
                (notes[i - 1].t + notes[i - 1].dur).min(n.t - 0.004)
            } else {
                n.t - 0.03
            }
        })
        .collect();
    // when the fingers move to each note
    let finger_at: Vec<f64> = (0..nn)
        .map(|i| {
            let n = &notes[i];
            match n.start {
                Start::Slur => n.t - 0.5 * inst.key_t,
                _ => {
                    let on = tongue_on[i];
                    (on + 0.3 * (n.t - on)).min(n.t - inst.key_t)
                }
            }
        })
        .collect();
    let mut bore = Bore::new(inst, fs);
    let mut rng = Rng::new(seed ^ 0xABCD);
    let mut out = vec![0.0; len];
    let mut knocks: Vec<Knock> = Vec::new();
    for i in 1..nn {
        if (notes[i].midi - notes[i - 1].midi).abs() > 0.5 && notes[i].gliss.is_none() {
            let moved = ((notes[i].midi - notes[i - 1].midi).abs() / 3.0).clamp(0.5, 2.0);
            knocks.push(Knock {
                t: finger_at[i] + inst.key_t,
                f1: 900.0 + 700.0 * rng.uni(),
                f2: 180.0 + 120.0 * rng.uni(),
                amp: inst.keys * moved,
            });
        }
    }
    for n in &notes {
        if n.start == Start::Click {
            knocks.push(Knock {
                t: n.t,
                f1: 1200.0 + 600.0 * rng.uni(),
                f2: 220.0,
                amp: inst.keys * 4.0 * n.vel,
            });
        }
    }
    // the player's ear: the note's pitch heard (the mouthpiece's swing, its period from one upward crossing to the
    // next) and the lip and jaw slowly bringing it to the note meant
    let mut ear_bp = Svf::new(440.0, 0.35, fs);
    let mut ear_bp2 = Svf::new(440.0, 0.35, fs);
    let mut ear_corr = 0.0f64;
    let mut ear_prev = 0.0f64;
    let mut ear_hist = [-1.0f64; 4];
    let mut ear_note = usize::MAX;
    let mut x_last = 0.0f64;
    let mut cur = 0usize;
    let mut level_at_end = vec![0.0; nn];
    let mut g_prev_note = 0.0;
    let mut last_g = 0.0;
    // an accent's extra breath, and a tongued start's push (the air held behind the tongue as it lets go): the
    // larger of the two, as both at once would throw the reed into a flatter way of swinging
    let target = |n: &LineNote, t: f64| -> f64 {
        let mut v = n.vel;
        if let Some((a, b, over)) = n.swell {
            v = a + (b - a) * smooth((t - n.t) / over.max(1e-3));
        }
        let mut g = gamma_of(inst, v);
        let mut push: f64 = 0.0;
        if n.accent > 0.0 && t >= n.t {
            push = inst.acc_k * n.accent * gone((t - n.t) / 0.07);
        }
        if n.start == Start::Tongue && t >= n.t {
            push = push.max(inst.tkick * n.kick * gone((t - n.t) / 0.04));
        }
        g *= 1.0 + push;
        g * (1.0 - 0.08 * n.sub)
    };
    // what the loop works out from inputs that seldom change, kept with the inputs (the same numbers as working them
    // out each sample)
    let mut mix_at = [f64::NAN; 3];
    let mut mix_was = 0.0;
    let mut meant_at = [f64::NAN; 2];
    let mut meant_was = 0.0;
    let mut hz_at = (f64::NAN, 0.0);
    // the line's loudest so far, the loudest of the stretch being judged, and where the line ended
    let block = ((QUIET_BLOCK * fs) as usize).max(1);
    let (mut loudest, mut stretch, mut ended) = (0.0f64, 0.0f64, len);
    // the line's noise, drawn ahead on a thread of its own (the knocks above drew theirs first)
    let (tx, rx) = std::sync::mpsc::sync_channel(NOISE_AHEAD);
    std::thread::scope(|scope| {
        // (the line's own end of the noise: when the line ends early the noise stops drawing)
        let rx = rx;
        let ahead = LineNoise {
            inst,
            notes: &notes,
            line: rng,
            seed,
            fs,
            len,
        };
        scope.spawn(move || ahead.draw(&tx));
        let mut noise: Vec<[f64; 5]> = Vec::new();
        let mut next = 0;
        for (s, slot) in out.iter_mut().enumerate() {
            if next == noise.len() {
                noise = rx.recv().unwrap_or_else(|_| vec![[0.0; 5]; NOISE_BLOCK]);
                next = 0;
            }
            let [growl_noise, jt, slow, breath_noise, air_noise] = noise[next];
            next += 1;
            let t = s as f64 / fs;
            while cur + 1 < nn && notes[cur + 1].t <= t {
                g_prev_note = last_g;
                cur += 1;
            }
            let n = &notes[cur];
            let started = t >= n.t;
            let e = n.t + n.dur;
            // ------------------------------------------------ the breath
            let mut g = 0.0;
            let mut tau = 0.0;
            if started {
                let tg = target(n, t);
                let mut ramp = 1.0;
                match n.start {
                    Start::Breath => ramp = dmath::pow(smooth((t - n.t) / inst.att_breath), 0.7),
                    Start::Slur => {
                        let w = smooth((t - n.t) / 0.03);
                        g = g_prev_note * (1.0 - w) + tg * w;
                    }
                    Start::Tongue => ramp = 1.0,
                    Start::Click => ramp = 0.0,
                }
                if n.start != Start::Slur {
                    g = tg * ramp;
                }
                if n.start == Start::Click {
                    g = 0.0;
                }
                if t >= e && !joined[cur] {
                    let end_level = level_at_end[cur];
                    if close[cur] && cur + 1 < nn {
                        // legato tonguing: the breath keeps up, the tongue stops the note
                        g = end_level;
                    } else {
                        let rel = n.fall.map_or(inst.rel, |f| f.1.max(inst.rel));
                        g = end_level * (1.0 - smooth((t - e) / rel));
                    }
                } else {
                    level_at_end[cur] = g;
                }
                // the tongue leaving the reed
                if n.start == Start::Tongue {
                    let tt = inst.tongue_t * n.slow * (1.0 - 0.4 * n.accent.min(1.0));
                    tau = 1.0 - smooth((t - n.t) / tt);
                }
                // the tongue stopping this note
                if t >= e && (n.stop_tongue || close[cur]) && !joined[cur] {
                    tau = tau.max(smooth((t - e) / 0.003));
                }
                // flutter tongue: the tongue rolling against the reed
                if n.flutter > 0.0 && t < e {
                    let ph = dmath::sin(2.0 * dmath::PI * 26.0 * (t - n.t)).max(0.0);
                    tau = tau.max(n.flutter * 0.85 * ph * ph);
                }
            }
            // the next note: the tongue lands before it, the breath builds behind the tongue
            if cur + 1 < nn {
                let m = &notes[cur + 1];
                if m.start == Start::Tongue && t >= tongue_on[cur + 1] {
                    tau = tau.max(smooth((t - tongue_on[cur + 1]) / 0.003));
                    if !started || t >= e {
                        let pre = smooth((t - (m.t - 0.03)) / 0.025) * target(m, m.t);
                        g = g.max(pre);
                    }
                }
                if m.start == Start::Breath && !started {
                    g = 0.0;
                }
            }
            if !started && cur == 0 && n.start == Start::Tongue && t >= n.t - 0.03 {
                tau = 1.0;
                g = smooth((t - (n.t - 0.03)) / 0.025) * target(n, n.t);
            }
            // growl (the throat humming into the reed) and breath vibrato
            if started && t < e + 0.05 {
                if n.growl > 0.0 {
                    let ph = 2.0 * dmath::PI * n.growl_hz.unwrap_or(inst.growl_f) * (t - n.t);
                    g *= 1.0 + n.growl * (0.22 * dmath::sin(ph) + 0.08 * dmath::sin(2.0 * ph) + 0.06 * growl_noise);
                }
                if let Some((c, r, w)) = n.vib
                    && inst.breath_vibrato
                {
                    let grow = smooth((t - n.t - w) / 0.3);
                    g *= 1.0 + grow * c * 0.004 * dmath::sin(2.0 * dmath::PI * r * (t - n.t - w));
                }
            }
            g *= 1.0 + inst.jitter * jt * 9.0;
            last_g = g;
            // ------------------------------------------------ the fingers
            let fg_now = fingers[cur];
            let (mut fa, mut fbb, mut w) = (fg_now, fg_now, 0.0);
            if cur + 1 < nn && t >= finger_at[cur + 1] {
                // moving to the next note already (during the tongue's stop or the slur)
                w = smooth((t - finger_at[cur + 1]) / inst.key_t);
                fbb = fingers[cur + 1];
            } else if cur > 0 && t < finger_at[cur] + inst.key_t {
                w = smooth((t - finger_at[cur]) / inst.key_t);
                fa = fingers[cur - 1];
            }
            if [fa.fb, fbb.fb, w].map(f64::to_bits) != mix_at.map(f64::to_bits) {
                mix_at = [fa.fb, fbb.fb, w];
                mix_was = dmath::exp(dmath::ln(fa.fb) * (1.0 - w) + dmath::ln(fbb.fb) * w);
            }
            let mut fb_mix = mix_was;
            // a change of register (the register key or the octave key): the tube's length and the vent change
            // together, so the sounding pitch moves from one note to the other while the resonance that sounds changes
            // from one to the other halfway, rather than the old resonance being dragged along
            let (mut ka_now, mut kb_now, mut w_now) = (fa.k, fbb.k, w);
            if fa.k != fbb.k {
                let f_mix = dmath::exp(dmath::ln(fa.fb * fa.k) * (1.0 - w) + dmath::ln(fbb.fb * fbb.k) * w);
                let k = if w < 0.5 { fa.k } else { fbb.k };
                fb_mix = f_mix / k;
                ka_now = k;
                kb_now = k;
                w_now = 0.0;
            }
            let voice = |a: f64, b: f64| a * (1.0 - w) + b * w;
            let (gs, zs, rs) = (voice(fa.gs, fbb.gs), voice(fa.zs, fbb.zs), voice(fa.rs, fbb.rs));
            // the player's lip and jaw on the pitch, cents; the drift is the player's own slow wander, which the ear
            // then works against
            let wander = slow * inst.drift * 12.0;
            let mut cents = 0.0;
            if started {
                if let Some((c, r, w)) = n.vib {
                    let grow = smooth((t - n.t - w) / 0.3);
                    let depth = if inst.breath_vibrato { 0.3 * c } else { c };
                    cents += grow * depth * dmath::sin(2.0 * dmath::PI * r * (t - n.t - w));
                }
                if let Some((c, at, over)) = n.bend {
                    cents += c * smooth((t - n.t - at) / over.max(1e-3));
                }
                if let Some((c, at, over)) = n.scoop {
                    cents -= c * (1.0 - smooth((t - n.t - at) / over.max(1e-3)));
                }
                if let Some((c, over)) = n.fall {
                    cents -= c * dmath::pow(smooth((t - e) / over.max(1e-3)), 1.5);
                }
                if let Some(over) = n.gliss
                    && cur > 0
                {
                    let from = (notes[cur - 1].midi - n.midi) * 100.0;
                    let w = smooth((t - n.t) / over.max(1e-3));
                    cents += from * (1.0 - w);
                }
            }
            // the ear: only on a note that has settled and holds its pitch (not in a smear, scoop, fall, growl or
            // flutter, nor while the fingers move), and only for a pitch near the one meant
            if ear_note != cur {
                ear_note = cur;
                // a slurred note keeps half the last note's correction (the lip stays where it was)
                ear_corr *= if n.start == Start::Slur { 0.5 } else { 0.0 };
                ear_hist = [-1.0; 4];
            }
            // the pitch meant, Hz, worked out where it is wanted (every 8th sample and at a crossing)
            let mut meant = || {
                if n.midi.to_bits() != hz_at.0.to_bits() {
                    hz_at = (n.midi, hz(n.midi));
                }
                if [n.midi, cents].map(f64::to_bits) != meant_at.map(f64::to_bits) {
                    meant_at = [n.midi, cents];
                    meant_was = hz_at.1 * dmath::pow(2.0, cents / 1200.0);
                }
                meant_was
            };
            if s % 8 == 0 {
                ear_bp.set(meant(), 0.35, fs);
                ear_bp2.set_as(&ear_bp);
            }
            let (_, eb0, _) = ear_bp.tick(x_last);
            let (_, eb, _) = ear_bp2.tick(eb0);
            if ear_prev < 0.0 && eb >= 0.0 {
                let f_meant = meant();
                let tc = (s as f64 - 1.0 + ear_prev / (ear_prev - eb)) / fs;
                let listening = inst.ear > 0.0
                    && started
                    && t > n.t + (0.045f64).max(6.0 / f_meant)
                    && t < e
                    && w == 0.0
                    && n.gliss.is_none()
                    && n.scoop.is_none()
                    && n.fall.is_none()
                    && n.growl == 0.0
                    && n.flutter == 0.0
                    && n.start != Start::Click;
                if listening && ear_hist[0] > 0.0 {
                    // the stretch back over 1 to 4 crossings that is nearest the period meant (a strong overtone can
                    // cross more than once in a period)
                    let mut best = (f64::MAX, 0.0);
                    for &h in &ear_hist {
                        if h > 0.0 {
                            let per = tc - h;
                            let c = dmath::ln(per * f_meant).abs();
                            if c < best.0 {
                                best = (c, per);
                            }
                        }
                    }
                    let period = best.1;
                    let err = 1200.0 * dmath::log2(1.0 / (period * f_meant));
                    if err.abs() < 80.0 {
                        let step = tc - ear_hist[0];
                        let tau = (0.035f64).max(5.0 * period) / inst.ear;
                        ear_corr = (ear_corr - err * step / tau).clamp(-60.0, 60.0);
                    }
                }
                ear_hist = [tc, ear_hist[0], ear_hist[1], ear_hist[2]];
            }
            ear_prev = eb;
            if s % 4 == 0 {
                // a slur down out of a higher register: the upper resonances held back for a moment
                let mut hd = 1.0;
                for j in [cur, cur + 1] {
                    if j > 0 && j < nn && notes[j].start == Start::Slur && fingers[j].k < fingers[j - 1].k {
                        let u = (t - finger_at[j]) / 0.035;
                        if (0.0..1.0).contains(&u) {
                            hd = f64::min(hd, 0.35 + 0.65 * smooth(u));
                        }
                    }
                }
                bore.set_modes(
                    fb_mix * dmath::pow(2.0, (cents + wander + ear_corr) / 1200.0),
                    ka_now,
                    kb_now,
                    w_now,
                    hd,
                );
            }
            // ------------------------------------------------ subtone, bends and the reed
            let sub = if started { n.sub } else { 0.0 };
            let bent = n.bend.map_or(0.0, |b| (b.0.abs() / 300.0).min(1.0));
            // the reed let go by the tongue swings a little wider open for its first few cycles
            let zk = if started && n.start == Start::Tongue {
                1.0 + inst.zkick * n.kick * gone((t - n.t) / 0.03)
            } else {
                1.0
            };
            let at = if started { n.airtone } else { 0.0 };
            let zeta = inst.zeta * (1.0 - 0.45 * sub) * (1.0 - 0.2 * bent) * zs * zk * (1.0 - 0.92 * at);
            let reed_f = inst.reed_f * (1.0 - 0.3 * sub) * rs;
            let reed_d = inst.reed_d * (1.0 + 1.5 * sub);
            let breath_on = if n.start == Start::Breath {
                3.0 * (1.0 - smooth((t - n.t) / 0.15))
            } else {
                0.0
            };
            let breath = inst.breath * (1.0 + 2.0 * sub + 6.0 * at + breath_on);
            let air_on = if n.start == Start::Breath {
                4.0 * (1.0 - smooth((t - n.t) / 0.2))
            } else {
                0.0
            };
            let air = (inst.air + 0.004 * at) * (1.0 + 4.0 * sub + 10.0 * at + air_on);
            let mut kick = if n.start == Start::Click && started && t - n.t < 2.0 / fs {
                0.6 * n.vel
            } else {
                0.0
            };
            if n.slap > 0.0 && started && t - n.t < 0.005 {
                // the pop: a short puff as the reed snaps back off the tongue
                kick += 0.08 * n.slap * n.vel * dmath::exp(-(t - n.t) / 0.0008);
            }
            let (xin, y) = bore.tick(
                (g * gs).max(0.0),
                tau.clamp(0.0, 1.0),
                zeta,
                reed_f,
                reed_d,
                breath,
                air,
                kick,
                (breath_noise, air_noise),
            );
            x_last = xin;
            // an accent's loudness beyond what its extra breath gives (more breath would also pull the pitch)
            let mut og = if started && n.accent > 0.0 {
                1.0 + inst.acc_out * n.accent * gone((t - n.t) / 0.09)
            } else {
                1.0
            };
            if started && n.slap > 0.0 {
                og *= 1.0 + 0.8 * n.slap * gone((t - n.t) / 0.02);
            }
            *slot = y * og;
            if quiet_after < len {
                let a = slot.abs();
                loudest = loudest.max(a);
                stretch = stretch.max(a);
                if (s + 1) % block == 0 {
                    if s + 1 >= quiet_after && stretch < QUIET * loudest {
                        ended = s + 1;
                        break;
                    }
                    stretch = 0.0;
                }
            }
        }
        drop(rx);
    });
    if ended < len {
        let from = ended - block.min(ended);
        for (i, v) in out[from..ended].iter_mut().enumerate() {
            *v *= 1.0 - (i + 1) as f64 / (ended - from) as f64;
        }
    }
    // the knocks
    for k in &knocks {
        let s0 = (k.t * fs) as isize;
        let n = (0.03 * fs) as usize;
        for j in 0..n {
            let tt = j as f64 / fs;
            let v = k.amp
                * (dmath::sin(2.0 * dmath::PI * k.f1 * tt) * dmath::exp(-tt / 0.004)
                    + 0.7 * dmath::sin(2.0 * dmath::PI * k.f2 * tt) * dmath::exp(-tt / 0.008));
            let at = s0 + j as isize;
            if at >= 0 && (at as usize) < out.len() {
                out[at as usize] += v;
            }
        }
    }
    out
}

// =====================================================================================================================
// the free reed (the accordion)

/// The banks of reeds a register sounds: (semitones from the note, cents, gain).
fn bank_of(inst: &Reed, r: Option<Register>) -> &'static [(f64, f64, f64)] {
    match r {
        // one reed bank at 8' (the "clarinet" register)
        Some(Register::One) => &[(0.0, 0.0, 1.0)],
        // two 8' reeds, one a little sharp (the "violin")
        Some(Register::Two) => &[(0.0, 0.0, 1.0), (0.0, 9.0, 0.9)],
        // the musette: three 8' reeds, one flat, one sharp
        Some(Register::Musette) => &[(0.0, -14.0, 0.85), (0.0, 0.0, 1.0), (0.0, 14.0, 0.85)],
        // the low bank alone (16', "bassoon")
        Some(Register::Low) => &[(-12.0, 0.0, 1.0)],
        // low and middle ("bandoneon": two reeds an octave apart)
        Some(Register::Octave) => &[(-12.0, 0.0, 0.8), (0.0, 0.0, 1.0)],
        // the master: low, three middle, high
        Some(Register::Master) => &[
            (-12.0, 0.0, 0.7),
            (0.0, -12.0, 0.7),
            (0.0, 0.0, 0.8),
            (0.0, 12.0, 0.7),
            (12.0, 0.0, 0.4),
        ],
        // the bass side: a bass button (16' 8' 4') and a chord button (8' 4')
        Some(Register::Bass) => &[(0.0, 0.0, 1.0), (12.0, 0.0, 0.6), (24.0, 0.0, 0.3)],
        Some(Register::Chord) => &[(0.0, 0.0, 0.8), (12.0, 0.0, 0.5)],
        None => inst.bank,
    }
}

/// One accordion note (the sketch's `free_note`), mono at the reed's own rate `fs`, `len` samples long.
fn free_note(inst: &Reed, n: &LineNote, seed: u64, fs: f64, len: usize) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    let bank: &[(f64, f64, f64)] = if n.airtone > 0.0 { &[] } else { bank_of(inst, n.bank) };
    let mut out = vec![0.0; len];
    let mut flow_sum = vec![0.0; len];
    // the bellows' pressure over the note
    let mut press = vec![0.0; len];
    let mut dir = vec![1.0; len];
    let rise = 0.012;
    for (s, p) in press.iter_mut().enumerate() {
        let t = s as f64 / fs;
        let mut v = n.vel;
        if let Some((a, b, over)) = n.swell {
            v = a + (b - a) * smooth(t / over.max(1e-3));
        }
        let mut g = v * smooth(t / rise);
        if n.accent > 0.0 {
            g *= 1.0 + 0.35 * n.accent * dmath::exp(-t / 0.06);
        }
        if t > n.dur {
            g *= 1.0 - smooth((t - n.dur) / 0.03);
        }
        if let Some((rate, depth)) = n.shake {
            // the bellows pushed and pulled in turn: the pressure dips through nothing at each change
            let ph = dmath::sin(dmath::PI * rate * t);
            g *= 1.0 - depth * (1.0 - (ph.abs() * 3.0).min(1.0));
            dir[s] = if ph >= 0.0 { 1.0 } else { -1.0 };
        }
        if let Some((depth, rate)) = n.wobble {
            g *= 1.0 + depth * dmath::sin(2.0 * dmath::PI * rate * t);
        }
        *p = g;
    }
    // a bend: the pitch of a hard-pushed reed (cents at each moment)
    let bent: Option<Vec<f64>> = n.bend.map(|(c, at, over)| {
        (0..len)
            .map(|s| dmath::pow(2.0, c * smooth((s as f64 / fs - at) / over.max(1e-3)) / 1200.0))
            .collect()
    });
    for &(semi, cents, gain) in bank {
        let f0 = hz(n.midi + semi) * dmath::pow(2.0, cents / 1200.0);
        // each reed of a pair (push and pull) a hair apart
        let pair = 1.0 + 0.0009 * (rng.uni() - 0.5);
        // (a big low tongue is slower, but a player's bellows still gets it going within some tens of ms)
        let tau = (inst.free_tau / f0).clamp(0.003, 0.012);
        // a reed below the note's own pitch is a bigger tongue moving more air: as loud as the note's own reed
        let lowk = dmath::pow((hz(n.midi) / f0).max(1.0), 1.3);
        let mut a: f64 = 0.1 + 0.03 * rng.uni();
        let mut ph = rng.uni() * 2.0 * dmath::PI;
        let mut dr = One::new(1.2, fs);
        let th = inst.tongue_th;
        let thr_k = dmath::pow(f0 / 440.0, 0.35);
        let die = dmath::exp(-1.0 / (0.04 * fs));
        for s in 0..len {
            let p = press[s];
            // the tongue's swing grows towards what the pressure holds and dies when it stops
            let want = p.max(0.0).sqrt();
            let da = (want * want - a * a) * a / tau;
            a = (a + da / fs).clamp(0.0, 1.5);
            if p < 1e-4 {
                a *= die;
            }
            let mut f = f0
                * if dir[s] < 0.0 { pair } else { 1.0 }
                * (1.0 - 0.0012 * a * a)
                * (1.0 + 0.00025 * dr.lp(rng.gauss()) * 10.0);
            if let Some(b) = &bent {
                f *= b[s];
            }
            ph += 2.0 * dmath::PI * f / fs;
            if ph > 2.0 * dmath::PI {
                ph -= 2.0 * dmath::PI;
            }
            let y = a * (dmath::sin(ph) + 0.05 * dmath::sin(2.0 * ph));
            // the air through the slot: shut while the tongue is in it, opening as it swings clear (the slot is cut
            // through a plate of fixed thickness, so how long it stays open in a cycle changes with the swing, which
            // keeps the overtones from being the same on every note; a dying swing narrows the opening with it)
            let thr = th * thr_k * (a / 0.35).min(1.0);
            let open = 0.015 + (y - thr).max(0.0) + inst.tongue_in * (-y - thr).max(0.0);
            let u = p.max(0.0).sqrt() * open;
            flow_sum[s] += gain * lowk * u;
        }
    }
    // the sound: the puffs of air (their rate of change above the reed block's own size, themselves below it, so a
    // low reed is not much weaker than a high one), through the reed block's cavity; the steady flow is not heard
    let mut cav = Bq::peak(inst.cav_f, inst.cav_db, inst.cav_q, fs);
    let mut prev = 0.0;
    let mut nzf = Svf::new(2500.0, 0.8, fs);
    // (the sketch's 0.7071, not 1/sqrt 2)
    #[allow(clippy::approx_constant)]
    let mut dc1 = Svf::new(45.0, 0.7071, fs);
    let ksh = fs / (2.0 * dmath::PI * inst.fsh);
    let noise = inst.bellows_noise * if n.airtone > 0.0 { AIR_BUTTON } else { 1.0 };
    for s in 0..len {
        let u = flow_sum[s];
        let sh = 0.55 * (u + (u - prev) * ksh);
        prev = u;
        let (_, _, ac) = dc1.tick(sh);
        let (_, nb, _) = nzf.tick(rng.gauss());
        out[s] = cav.tick(ac) + noise * press[s].max(0.0) * nb;
    }
    // the pallet opening (the key's valve)
    let k = (0.02 * fs) as usize;
    for (j, o) in out.iter_mut().enumerate().take(k.min(len)) {
        let tt = j as f64 / fs;
        *o += inst.pallet * n.vel * dmath::sin(2.0 * dmath::PI * 520.0 * tt) * dmath::exp(-tt / 0.005);
    }
    out
}

// =====================================================================================================================
// out at the heard rate

/// From the reed's rate down to the heard rate `fs` (a windowed-sinc low-pass at 20 kHz, as the sketch's at 48 kHz),
/// `n` samples.
fn down(x: &[f64], fs: f64, n: usize) -> Vec<f64> {
    const TAPS: usize = 64;
    // (at 48 kHz the sketch's 20 kHz; at a low rate held under the heard Nyquist)
    let fc = (20_000.0 / (fs * OS as f64)).min(0.46 / OS as f64);
    let h: Vec<f64> = (0..TAPS)
        .map(|i| {
            let m = i as f64 - (TAPS - 1) as f64 / 2.0;
            let s = if m == 0.0 {
                2.0 * fc
            } else {
                dmath::sin(2.0 * dmath::PI * fc * m) / (dmath::PI * m)
            };
            let k = (TAPS - 1) as f64;
            let w = 0.42 - 0.5 * dmath::cos(2.0 * dmath::PI * i as f64 / k)
                + 0.08 * dmath::cos(4.0 * dmath::PI * i as f64 / k);
            s * w
        })
        .collect();
    // an output from its taps, those that fall inside `x`
    let edge = |j: usize| {
        let c = j * OS;
        let mut s = 0.0;
        for (k, hk) in h.iter().enumerate() {
            if c + k >= TAPS / 2 && c + k - TAPS / 2 < x.len() {
                s += hk * x[c + k - TAPS / 2];
            }
        }
        s
    };
    // inside, four outputs at a time, each its own sum in the taps' order (the same sums, four running at once)
    let first = (TAPS / 2).div_ceil(OS);
    let last = if x.len() >= TAPS / 2 {
        ((x.len() - TAPS / 2) / OS + 1).min(n)
    } else {
        0
    };
    let mut out = Vec::with_capacity(n);
    let mut j = 0;
    while j < n {
        if j >= first && j + 4 <= last {
            let c = j * OS - TAPS / 2;
            let mut s = [0.0; 4];
            for (k, hk) in h.iter().enumerate() {
                s[0] += hk * x[c + k];
                s[1] += hk * x[c + OS + k];
                s[2] += hk * x[c + 2 * OS + k];
                s[3] += hk * x[c + 3 * OS + k];
            }
            out.extend_from_slice(&s);
            j += 4;
        } else {
            let c = j * OS;
            out.push(if c >= TAPS / 2 && c + TAPS / 2 <= x.len() {
                let mut s = 0.0;
                for (hk, v) in h.iter().zip(&x[c - TAPS / 2..c + TAPS / 2]) {
                    s += hk * v;
                }
                s
            } else {
                edge(j)
            });
            j += 1;
        }
    }
    out
}

/// The sound's colour as heard: the high-pass, the formants, the darkening, the level (at the heard rate `fs`).
fn colour(inst: &Reed, x: &mut [f64], fs: f64) {
    let mut fl: Vec<Bq> = inst.form.iter().map(|&(f, db, q)| Bq::peak(f, db, q, fs)).collect();
    let mut lp = (inst.lp_out > 0.0).then(|| Bq::lowpass(inst.lp_out, 0.707, fs));
    let mut hp = Bq::highpass(inst.hp_out, 0.707, fs);
    for v in x.iter_mut() {
        let mut y = hp.tick(*v);
        for f in &mut fl {
            y = f.tick(y);
        }
        if let Some(l) = lp.as_mut() {
            y = l.tick(y);
        }
        *v = y * inst.gain;
    }
}

/// The sketch's line: `notes` on `reed` (a free reed's each on its own, seeded `seed * 1000` plus its place), with
/// noise `seed`, mono at `fs`, `secs` seconds. For measuring the port against the sketch.
pub fn line(reed: &Reed, notes: &[LineNote], seed: u64, secs: f64, fs: f64) -> Vec<f64> {
    let inner = fs * OS as f64;
    let n = (secs * fs) as usize;
    let len = n * OS;
    let x = if reed.free {
        let mut mix = vec![0.0; len];
        for (i, nt) in notes.iter().enumerate() {
            let at = (nt.t * inner) as usize;
            if at >= len {
                continue;
            }
            let k = ((nt.dur + 0.35) * inner) as usize;
            let y = free_note(reed, nt, seed * 1000 + i as u64, inner, k);
            for (o, v) in mix.iter_mut().skip(at).zip(&y) {
                *o += v;
            }
        }
        mix
    } else {
        render_line(reed, notes, seed, inner, len, len)
    };
    let mut y = down(&x, fs, n);
    colour(reed, &mut y, fs);
    y
}

// =====================================================================================================================
// a note as played

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

/// A finite number, or none.
fn fin(x: f64) -> Option<f64> {
    x.is_finite().then_some(x)
}

/// What a slurred note comes from, if it is slurred.
fn slur_of(play: &Play) -> Option<f64> {
    play.slur_from.and_then(fin).map(|m| m.clamp(0.0, 127.0))
}

/// The bagpipe's grace note before a note: high G, or high A before high G itself.
fn grace_of(m: f64) -> f64 {
    if (m - 79.0).abs() < 0.5 { 81.0 } else { 79.0 }
}

/// The note `play` as a note of the sketch's line, starting at `t`.
fn line_note(p: &ReedParts, play: &Play, t: f64) -> LineNote {
    let m = key_of(play);
    let mut n = LineNote::plain(t, m, play.dur.max(0.0), unit(play.vel));
    n.accent = unit(play.accent);
    n.tie = play.tie;
    n.stop_tongue = matches!(play.mute, Mute::Choked | Mute::Clipped);
    n.swell = play
        .swell
        .filter(|s| s.from.is_finite() && s.to.is_finite() && s.over.is_finite())
        .map(|s| (s.from.clamp(0.0, 1.2), s.to.clamp(0.0, 1.2), s.over));
    n.vib = match play.vibrato {
        // (held to 400 cents and 20 Hz: anything faster is no vibrato, and the tube cannot follow it)
        Some(v) if v.cents.is_finite() && v.hz.is_finite() && v.wait.is_finite() => {
            (v.cents != 0.0).then_some((v.cents.clamp(-400.0, 400.0), v.hz.clamp(0.0, 20.0), v.wait.max(0.0)))
        }
        _ => (p.vibrato.0 != 0.0).then_some(p.vibrato),
    };
    let slur = slur_of(play);
    if let Some(b) = play
        .bend
        .filter(|b| b.cents.is_finite() && b.at.is_finite() && b.over.is_finite())
    {
        let c = b.cents.clamp(-2400.0, 2400.0);
        if c == 0.0 {
            // a smear (a glide over the bend's time) from the note it is slurred from
            if slur.is_some() {
                n.gliss = Some(b.over.max(1e-3));
            }
        } else if b.at >= n.dur - 1e-9 {
            // a fall or a doit off the end of the note, as the breath goes
            n.fall = Some((-c, b.over.max(1e-3)));
        } else {
            n.bend = Some((c, b.at.max(0.0), b.over));
        }
    }
    if let Some(b) = play
        .scoop
        .filter(|b| b.cents.is_finite() && b.at.is_finite() && b.over.is_finite())
    {
        n.scoop = Some((-b.cents.clamp(-2400.0, 2400.0), b.at.max(0.0), b.over));
    }
    match play.technique {
        Technique::Breath => n.start = Start::Breath,
        Technique::KeyClick => n.start = Start::Click,
        Technique::BackTongue => {
            n.kick = BACK_KICK;
            n.slow = BACK_SLOW;
        }
        Technique::SoftTongue => {
            n.kick = SOFT_KICK;
            n.slow = SOFT_SLOW;
            n.stop_tongue = true;
        }
        Technique::Flutter => n.flutter = FLUTTER,
        Technique::Growl => n.growl = GROWL,
        Technique::Sung(k) => {
            n.growl = SUNG;
            n.growl_hz = Some(hz((m + f64::from(k)).clamp(0.0, 127.0)));
        }
        Technique::Subtone => {
            // (a subtone is started by the breath, the tongue hardly touching the damped reed)
            n.sub = SUBTONE;
            n.start = Start::Breath;
        }
        Technique::Slap => n.slap = SLAP,
        Technique::AirTone => n.airtone = AIR_TONE,
        Technique::Harmonic(k) => n.reg = Some(f64::from(k.max(1))),
        Technique::Shake if !p.reed.free => n.vib = Some(SHAKE),
        Technique::Tremolo if p.reed.free => n.shake = Some(BELLOWS_SHAKE),
        _ => {}
    }
    if p.reed.free {
        n.bank = play.register;
        n.bend = n.bend.or(n.fall.map(|(c, over)| (-c, n.dur, over)));
        n.wobble = n.vib.map(|(c, r, _)| ((c / 100.0).clamp(0.0, 0.6), r));
        n.vib = None;
    }
    n
}

/// The seed of repeat `variant` of a note at `m`.
fn seed(variant: u64, m: f64) -> u64 {
    variant.wrapping_mul(7919).wrapping_add(11) ^ ((m * 1000.0) as u64)
}

/// How long the note `play` rings on `p` with no length asked for, seconds, at most `longest`.
pub fn rings_for(p: &ReedParts, play: &Play, longest: f64) -> f64 {
    let dur = play.dur.max(0.0);
    let ring = if p.reed.free {
        dur + 0.35
    } else if play.tie {
        dur + JOIN
    } else {
        let fall = play
            .bend
            .filter(|b| b.at.is_finite() && b.over.is_finite() && b.at >= dur - 1e-9)
            .map_or(0.0, |b| b.over.max(0.0));
        dur + fall + RING
    };
    if ring.is_finite() {
        ring.clamp(0.05, longest.max(0.05))
    } else {
        longest.max(0.05)
    }
}

/// The note `play` on `p`, `len` samples at `fs`, as repeat `variant`. Mono.
pub fn note(p: &ReedParts, play: &Play, len: usize, variant: u64, fs: f64) -> Vec<f64> {
    if len == 0 {
        return Vec::new();
    }
    let m = key_of(play);
    let sd = seed(variant, m);
    let mut x = if p.reed.free {
        let n = line_note(p, play, 0.0);
        let inner = fs * OS as f64;
        let y = free_note(p.reed, &n, sd, inner, len * OS);
        let mut y = down(&y, fs, len);
        colour(p.reed, &mut y, fs);
        y
    } else if let (Some(d), Some(Register::Bass)) = (p.drones, play.register) {
        drones(d, play, len, sd, fs)
    } else {
        blown(p, play, len, sd, fs)
    };
    end_fade(&mut x, fs);
    x
}

/// A blown note: the sketch's line of this note alone (a slurred one led in by the note it comes from, a plain
/// bagpipe note by its grace note), heard from its start.
fn blown(p: &ReedParts, play: &Play, len: usize, sd: u64, fs: f64) -> Vec<f64> {
    let m = key_of(play);
    let graced = p.reed.bag && slur_of(play).is_none() && play.technique != Technique::Breath;
    let from = slur_of(play).or_else(|| graced.then(|| grace_of(m)));
    let lead = if from.is_some() { LEAD } else { 0.0 };
    // the note starts on a whole sample of the heard rate
    let j0 = ((PRE + lead) * fs).round() as usize;
    let t0 = j0 as f64 / fs;
    let mut this = line_note(p, play, t0);
    let mut notes = Vec::with_capacity(2);
    if let Some(f) = from {
        let mut first = LineNote::plain(PRE, f, t0 - PRE, this.vel);
        first.swell = None;
        notes.push(first);
        this.start = Start::Slur;
    }
    notes.push(this);
    // a plain bagpipe note is heard from its grace note
    let heard = if graced {
        j0 - ((GRACE * fs).round() as usize).min(j0)
    } else {
        j0
    };
    let inner = fs * OS as f64;
    let all = heard + len;
    // (past the note's own ring, as long as the part asked, the line ends once it is quiet)
    let ring = (rings_for(p, play, f64::INFINITY) * fs) as usize;
    let x = render_line(p.reed, &notes, sd, inner, all * OS, (j0 + ring) * OS);
    let mut y = down(&x, fs, all);
    colour(p.reed, &mut y, fs);
    let mut y = y.split_off(heard);
    let k = ((JOIN * fs) as usize).max(1);
    if slur_of(play).is_some() {
        for (s, v) in y.iter_mut().take(k).enumerate() {
            *v *= dmath::sin(0.5 * dmath::PI * s as f64 / k as f64);
        }
    }
    if play.tie {
        let a = (this.dur * fs) as usize;
        for (s, v) in y.iter_mut().enumerate().skip(a) {
            let u = ((s - a) as f64 / k as f64).min(1.0);
            *v *= dmath::cos(0.5 * dmath::PI * u);
        }
    }
    y
}

/// A bagpipe's drones on the note asked (the bass drone on it, the tenors over it), each a long note of its own,
/// struck in one after another and held as long as the note.
fn drones(d: &Drones, play: &Play, len: usize, sd: u64, fs: f64) -> Vec<f64> {
    let m = key_of(play);
    let inner = fs * OS as f64;
    let j0 = (PRE * fs).round() as usize;
    let mut out = vec![0.0; len];
    for (i, &(over, gain, after)) in d.lines.iter().enumerate() {
        let mut n = LineNote::plain(
            j0 as f64 / fs + after,
            (m + over).clamp(0.0, 127.0),
            play.dur.max(0.0),
            unit(play.vel),
        );
        n.swell = play
            .swell
            .filter(|s| s.from.is_finite() && s.to.is_finite() && s.over.is_finite())
            .map(|s| (s.from.clamp(0.0, 1.2), s.to.clamp(0.0, 1.2), s.over));
        let all = j0 + len;
        let x = render_line(
            &d.reed,
            &[n],
            sd.wrapping_add(7 * i as u64 + 7),
            inner,
            all * OS,
            all * OS,
        );
        let mut y = down(&x, fs, all);
        colour(&d.reed, &mut y, fs);
        for (o, v) in out.iter_mut().zip(y.iter().skip(j0)) {
            *o += gain * v;
        }
    }
    out
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

    /// How far a note near `f0` sounds from it over `a..b` seconds, cents, read on the harmonic nearest 400 Hz (a
    /// finer reading for a low note in a short stretch), within 4% either way.
    fn pitch(x: &[f64], f0: f64, a: f64, b: f64, fs: f64) -> f64 {
        let h = (400.0 / f0).round().max(1.0);
        cents(peak_in(x, h * f0 * 0.96, h * f0 * 1.04, a, b, fs), h * f0)
    }

    /// How loud a burst of highs (over 5 kHz) is at `t` against the quarter second before it, dB.
    fn burst(x: &[f64], t: f64, fs: f64) -> f64 {
        let hp = highs(x, 5000.0, fs);
        let e = |u: f64| db(&hp, u - 0.002, u + 0.002, fs);
        let around = (0..7).map(|i| e(t - 0.006 + 0.002 * i as f64)).fold(f64::MIN, f64::max);
        let mut before: Vec<f64> = (0..50).map(|i| e(t - 0.25 + 0.004 * i as f64)).collect();
        before.sort_by(f64::total_cmp);
        around - before[25]
    }

    /// No vibrato.
    const SENZA: Option<Vibrato> = Some(Vibrato {
        cents: 0.0,
        hz: 5.0,
        wait: 0.0,
    });

    /// Each of Aedan's picks with a note in the middle of its range.
    const ALL: [(ReedParts, f64); 12] = [
        (ReedParts::SS2, 72.0),
        (ReedParts::AS2, 65.0),
        (ReedParts::TS2, 57.0),
        (ReedParts::BS2, 46.0),
        (ReedParts::CL1, 60.0),
        (ReedParts::BC1, 46.0),
        (ReedParts::CC1, 34.0),
        (ReedParts::OB1, 72.0),
        (ReedParts::BN1, 46.0),
        (ReedParts::CF2, 34.0),
        (ReedParts::BP1, 71.0),
        (ReedParts::AC1, 64.0),
    ];

    fn sound(p: &ReedParts, play: &Play, secs: f64, fs: f64) -> Vec<f64> {
        note(p, play, (secs * fs) as usize, 0, fs)
    }

    fn held(m: f64, dur: f64, vel: f64) -> Play {
        Play {
            vibrato: SENZA,
            ..Play::plain(m, dur, vel)
        }
    }

    #[test]
    fn every_reed_plays_in_tune_at_48_and_44_1_khz() {
        for (p, m) in &ALL {
            for fs in [SRF, 44100.0] {
                let x = sound(p, &held(*m, 1.2, 0.7), 1.3, fs);
                let c = pitch(&x, hz_of(*m), 0.5, 1.1, fs);
                assert!(c.abs() < 12.0, "{m} at {fs}: {c:.1} cents");
            }
        }
    }

    #[test]
    fn a_harder_breath_is_louder_and_no_darker() {
        // the alto's, tenor's and bari's reeds already beat against the mouthpiece at a soft breath, so they grow
        // little louder (as in the sketch), and the bagpipe's bag holds one pressure: those are no quieter
        let grows = [
            true, false, false, false, true, true, true, true, true, true, false, true,
        ];
        for ((p, m), grows) in ALL.iter().zip(grows) {
            let soft = sound(p, &held(*m, 0.8, 0.35), 0.9, SRF);
            let hard = sound(p, &held(*m, 0.8, 0.95), 0.9, SRF);
            let louder = db(&hard, 0.2, 0.7, SRF) - db(&soft, 0.2, 0.7, SRF);
            let brighter = bright(&hard, 0.2, 0.7, 2000.0, SRF) - bright(&soft, 0.2, 0.7, 2000.0, SRF);
            assert!(louder > if grows { 1.0 } else { -0.5 }, "{m}: {louder:+.1} dB louder");
            assert!(brighter > -1.0, "{m}: {brighter:+.1} dB brighter");
        }
    }

    #[test]
    fn every_technique_and_stop_is_finite_heard_and_its_own() {
        let techniques = [
            Technique::Breath,
            Technique::KeyClick,
            Technique::BackTongue,
            Technique::SoftTongue,
            Technique::Flutter,
            Technique::Growl,
            Technique::Sung(-12),
            Technique::Subtone,
            Technique::Slap,
            Technique::AirTone,
            Technique::Harmonic(2),
            Technique::Shake,
            Technique::Tremolo,
        ];
        for (p, m) in &ALL {
            let plain = sound(p, &Play::plain(*m, 0.5, 0.7), 0.8, SRF);
            for t in techniques {
                for mute in [Mute::Open, Mute::Choked] {
                    let play = Play {
                        technique: t,
                        mute,
                        ..Play::plain(*m, 0.5, 0.7)
                    };
                    let x = sound(p, &play, 0.8, SRF);
                    assert!(x.iter().all(|v| v.is_finite()), "{m} {t:?} {mute:?}");
                    assert!(db(&x, 0.0, 0.5, SRF) > -60.0, "{m} {t:?} {mute:?}: silent");
                }
                let x = sound(
                    p,
                    &Play {
                        technique: t,
                        ..Play::plain(*m, 0.5, 0.7)
                    },
                    0.8,
                    SRF,
                );
                let apart: f64 = x.iter().zip(&plain).map(|(u, v)| (u - v) * (u - v)).sum::<f64>().sqrt();
                let size: f64 = plain.iter().map(|u| u * u).sum::<f64>().sqrt();
                let free = p.reed.free;
                // what a reed (or the accordion) has no way to play is its plain note: a note's own register asked as
                // its harmonic, and the bagpipe's chanter has no keys and is never tongued
                let none = if free {
                    !matches!(t, Technique::Tremolo | Technique::AirTone)
                } else {
                    matches!(t, Technique::Tremolo)
                        || matches!(t, Technique::Harmonic(k) if f64::from(k) == reg_of(p.reed, *m))
                        || (p.reed.bag && matches!(t, Technique::KeyClick | Technique::BackTongue))
                };
                if !none {
                    assert!(apart > 0.05 * size, "{m} {t:?}: no different from a plain note");
                }
            }
        }
    }

    #[test]
    fn a_slur_or_a_tie_joins_without_a_click_or_a_gap() {
        for (p, m) in &ALL {
            if p.reed.free {
                continue;
            }
            for glide in [None, Some(0.08)] {
                let a = Play {
                    tie: true,
                    ..held(*m, 0.5, 0.7)
                };
                let b = Play {
                    slur_from: Some(*m),
                    bend: glide.map(|over| Bend {
                        cents: 0.0,
                        at: 0.0,
                        over,
                    }),
                    ..held(*m + 2.0, 0.6, 0.7)
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
        for (p, m) in &ALL {
            let a = note(p, &Play::plain(*m, 0.5, 0.7), 30000, 0, SRF);
            assert!(
                a == note(p, &Play::plain(*m, 0.5, 0.7), 30000, 0, SRF),
                "{m}: not the same twice"
            );
            let b = note(p, &Play::plain(*m, 0.5, 0.7), 30000, 1, SRF);
            let apart: f64 = a.iter().zip(&b).map(|(u, v)| (u - v) * (u - v)).sum::<f64>().sqrt();
            let size: f64 = a.iter().map(|u| u * u).sum::<f64>().sqrt();
            assert!(apart > 0.01 * size, "{m}: repeats alike ({:.3})", apart / size);
        }
    }

    #[test]
    fn notes_at_the_edges_and_odd_asks_stay_finite() {
        for (p, m) in &ALL {
            let mut plays: Vec<Play> = [0.0, 10.0, 127.0, f64::NAN, f64::INFINITY]
                .iter()
                .map(|&k| Play::plain(k, 0.3, 0.7))
                .collect();
            let odd = [
                Play::plain(*m, 0.3, 0.0),
                Play {
                    accent: 5.0,
                    ..Play::plain(*m, 0.3, 3.0)
                },
                Play {
                    accent: f64::NAN,
                    ..Play::plain(*m, 0.3, f64::NAN)
                },
                Play {
                    swell: Some(Swell {
                        from: 0.0,
                        to: 1.0,
                        over: 0.3,
                    }),
                    ..Play::plain(*m, 0.4, 1.0)
                },
                Play::plain(*m, 0.0, 0.7),
                Play::plain(*m, f64::NAN, 0.7),
                Play {
                    tie: true,
                    slur_from: Some(*m - 7.0),
                    ..Play::plain(*m, 0.3, 0.7)
                },
                Play {
                    slur_from: Some(f64::NAN),
                    ..Play::plain(*m, 0.3, 0.7)
                },
                Play {
                    bend: Some(Bend {
                        cents: f64::INFINITY,
                        at: 0.1,
                        over: 0.1,
                    }),
                    ..Play::plain(*m, 0.3, 0.7)
                },
                Play {
                    scoop: Some(Bend {
                        cents: -2400.0,
                        at: 5.0,
                        over: 0.0,
                    }),
                    ..Play::plain(*m, 0.3, 0.7)
                },
                Play {
                    vibrato: Some(Vibrato {
                        cents: 500.0,
                        hz: 1e6,
                        wait: -1.0,
                    }),
                    ..Play::plain(*m, 0.3, 0.7)
                },
                Play {
                    technique: Technique::Harmonic(40),
                    ..Play::plain(*m, 0.3, 0.7)
                },
                Play {
                    register: Some(Register::Master),
                    ..Play::plain(*m, 0.3, 0.7)
                },
            ];
            plays.extend(odd);
            for play in &plays {
                let r = rings_for(p, play, 12.0);
                assert!(r.is_finite() && r > 0.0, "{play:?}");
                let x = note(p, play, (r.min(2.0) * SRF) as usize, 0, SRF);
                assert!(x.iter().all(|v| v.is_finite() && v.abs() < 4.0), "{m} {play:?}");
            }
        }
    }

    #[test]
    fn a_note_rings_out_and_ends_quietly() {
        for (p, m) in &ALL {
            let play = Play::plain(*m, 0.4, 0.8);
            let r = rings_for(p, &play, 12.0);
            assert!(r > 0.4 && r < 1.2, "{m}: rings {r:.2} s");
            let x = note(p, &play, (r * SRF) as usize, 0, SRF);
            let end = db(&x, r - 0.01, r, SRF) - db(&x, 0.1, 0.35, SRF);
            assert!(end < -30.0, "{m}: ends {end:+.1} dB");
        }
    }

    #[test]
    fn the_bagpipe_chanter_grace_is_high_g_and_its_drones_hold_on_a() {
        let p = ReedParts::BP1;
        // a plain chanter note starts with the high-G grace, a slurred one without it
        let graced = sound(&p, &held(69.0, 0.5, 0.8), 0.6, SRF);
        let slurred = sound(
            &p,
            &Play {
                slur_from: Some(69.0),
                ..held(69.0, 0.5, 0.8)
            },
            0.6,
            SRF,
        );
        let g = hz_of(79.0);
        let grace = at_hz(&graced, g, 0.0, 0.03, SRF) - at_hz(&slurred, g, 0.0, 0.03, SRF);
        assert!(grace > 6.0, "the grace's G {grace:+.1} dB over a slurred start");
        // the drones: A2 and A3 both held strongly for as long as they are asked
        let d = sound(
            &p,
            &Play {
                register: Some(Register::Bass),
                ..held(45.0, 3.0, 0.7)
            },
            3.0,
            SRF,
        );
        for (f, what) in [(110.0, "the bass"), (220.0, "the tenors")] {
            let c = cents(peak_in(&d, f * 0.97, f * 1.03, 1.0, 2.8, SRF), f);
            assert!(c.abs() < 12.0, "{what} drone {c:.1} cents off");
        }
        let steady = db(&d, 2.3, 2.8, SRF) - db(&d, 0.8, 1.3, SRF);
        assert!(steady.abs() < 1.5, "the drones move {steady:+.1} dB while held");
    }

    #[test]
    fn the_accordion_registers_sound_their_banks() {
        let p = ReedParts::AC1;
        let m = 64.0;
        let f = hz_of(m);
        let with = |r: Option<Register>| {
            sound(
                &p,
                &Play {
                    register: r,
                    ..held(m, 0.8, 0.7)
                },
                1.0,
                SRF,
            )
        };
        let one = with(Some(Register::One));
        let low = with(Some(Register::Low));
        let bass = with(Some(Register::Bass));
        let under = |x: &[f64]| at_hz(x, f / 2.0, 0.2, 0.7, SRF) - at_hz(x, f, 0.2, 0.7, SRF);
        let lower = under(&low) - under(&one);
        assert!(lower > 10.0, "the low bank {lower:+.1} dB an octave under");
        let over = |x: &[f64]| at_hz(x, 4.0 * f, 0.2, 0.7, SRF) - at_hz(x, f, 0.2, 0.7, SRF);
        let up = over(&bass) - over(&one);
        assert!(up > 3.0, "the bass button {up:+.1} dB two octaves over");
        // the musette's reeds beat: its level swings more than one reed's
        let swing = |x: &[f64]| {
            let w: Vec<f64> = (0..40)
                .map(|i| db(x, 0.2 + 0.0125 * i as f64, 0.2125 + 0.0125 * i as f64, SRF))
                .collect();
            w.iter().copied().fold(f64::MIN, f64::max) - w.iter().copied().fold(f64::MAX, f64::min)
        };
        let beat = swing(&with(None)) - swing(&one);
        assert!(beat > 1.0, "the musette beats {beat:+.1} dB more than one reed");
    }

    #[test]
    fn a_line_plays_its_notes_joined() {
        // the line kernel the notes are cut from: a tongued note, a slur up, a tie
        for (p, m) in &ALL {
            let mut b = LineNote::plain(0.6, *m + 2.0, 0.5, 0.7);
            b.start = Start::Slur;
            let notes = [LineNote::plain(0.1, *m, 0.5, 0.7), b];
            let x = line(p.reed, &notes, 11, 1.4, SRF);
            assert!(x.iter().all(|v| v.is_finite()), "{m}");
            let c = pitch(&x, hz_of(*m + 2.0), 0.8, 1.05, SRF);
            assert!(c.abs() < 25.0, "{m}: the slurred note {c:.1} cents off");
            let dip = db(&x, 0.58, 0.64, SRF) - db(&x, 0.35, 0.55, SRF);
            assert!(dip > -8.0, "{m}: a gap {dip:+.1} dB at the slur");
        }
    }
}
