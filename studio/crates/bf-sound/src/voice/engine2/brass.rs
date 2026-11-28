//! Brass: the trumpet, the cornet, the flugelhorn, the horn, the mellophone, the trombone, the euphonium, the tuba
//! and the sousaphone (`docs/studio/voice-engine-2-instruments.md`; the sketch `br.rs` and `winds.py`, Aedan's picks
//! Tp1, Co1, Fh1, Hn1, Me1, Tb1, Eu1, Tu1 and Su1).
//!
//! The player's lips are a valve: a mass on a spring that the mouth's pressure squeezes shut (inward striking, which
//! plays in tune and speaks quickly where the outward-striking model did not), letting puffs of air into the horn.
//! The flow through the opening is worked out every sample against what the puffs meet: the horn's input impedance,
//! one resonance per natural note, strongest around the mouthpiece's own ring and gone above the bell's cutoff, plus
//! the plain tube's resistance over it. The valves or the slide scale every resonance by the length they add. The
//! wave that leaves the mouthpiece runs down the tube and its loud peaks run faster than its troughs, so a loud note
//! steepens into a near-shock over the tube's cylindrical length (the brassy blare); the walls soften its top, and
//! the bell lets the highs out more readily than the lows (two high-passes) with a soft top of its own. The horn runs
//! at four times the sample rate (twice for the tuba and the sousaphone) and is brought down with a windowed-sinc
//! decimator.
//!
//! The player: each note is first tried alone (trial runs of a third of a second or so) at a few lip tensions, the
//! one that sounds the right natural note cleanly, with the least lipping and a quick start, kept, and the horn's
//! length trimmed until it sounds in tune, as a player's lips and tuning slide do. The fingering is the player's: the
//! fewest valves (the shortest slide), a pedal note only when nothing else has it, another fingering when the usual
//! one will not speak. A slurred note arrives with the lips already buzzing on the note before and is tuned as it is
//! played. When the breath stops, the horn's ringing eases onto the note just played (the lips pull low notes further
//! under the horn's resonances than real lips do, and the horn rings at its own pitch once they let go). The trial
//! runs for plain notes are made once, offline, by this very code into a table ([`super::brass_tune`]); a note the
//! table lacks is tried here when it is first asked for, and kept.
//!
//! After the horn: a pulse limiter (the first puff of a hard attack can leave a pulse a millisecond long several times
//! the note's peak), the mutes and the horn player's hand in the bell as filters, and a soft limiter.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use bf_core::dmath;

use super::play::{Mute, Play, Swell, Technique};
use crate::dsp::fft::Fft;
use crate::dsp::{self, FilterKind, Sos, butter_sos};

/// A brass instrument's make-up (the sketch's settings, by its names).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrassParts {
    /// The pick's name, for its table and its tried notes (`Tp1`).
    pub name: &'static str,
    /// The open horn's nominal fundamental, MIDI (natural note n sounds at `m0` + 12 log2 n); a double horn's other
    /// side (`m0b`, 0 for none), used from `split` up.
    pub m0: f64,
    pub m0b: f64,
    pub split: f64,
    /// How far the valves or the slide lower it, semitones (`maxv`).
    pub valves: i32,
    /// The lowest and highest natural notes used (1: pedal notes allowed).
    pub minpart: usize,
    pub maxpart: usize,
    /// The first resonance as a part of the nominal fundamental (a flaring bell's is far flat).
    pub r1: f64,
    /// How many resonances.
    pub nmodes: usize,
    /// The tube's own resistance, Pa s / m^3, and the resonances' peak height.
    pub zc: f64,
    pub zp: f64,
    /// The mouthpiece's ring, Hz (the resonances are strongest around it), over so many octaves (`sig`).
    pub fmp: f64,
    pub sig: f64,
    /// The bell's cutoff, Hz.
    pub fc: f64,
    /// The first resonance's share (weak on a trumpet, fuller on a tuba).
    pub w1: f64,
    /// The resonances' sharpness at the mouthpiece's ring, and at the bottom.
    pub q0: f64,
    pub qlo: f64,
    /// The lips: mass per area (kg/m^2), damping (Q), width (m), pressed together as a part of the breath's pressure.
    pub mu: f64,
    pub ql: f64,
    pub lw: f64,
    pub lpress: f64,
    /// The lips' own ring as a part of the note, when no trial run has set it.
    pub lr: f64,
    /// The mouth's pressure, softest to loudest, Pa (at `fref`), rising with pitch as f^`pexp`.
    pub pmin: f64,
    pub pmax: f64,
    pub pexp: f64,
    pub fref: f64,
    /// The tube's cylindrical length for the steepening, m.
    pub lcyl: f64,
    /// The walls' loss round the steepened wave above this, Hz.
    pub loss_fc: f64,
    /// The bell lets the highs out: a high-pass at about its cutoff, a second, lower one (a part of it), a soft top.
    pub bell_fc: f64,
    pub bell2: f64,
    pub bell_lp: f64,
    /// The breath's turbulence (a part of the pressure) and the tongue's "t" (a puff of noise at a tongued start).
    pub noise: f64,
    pub tnoise: f64,
    /// The tongue's release time (s), a tongued start's pressure overshoot, the breath's stop (s).
    pub att: f64,
    pub over: f64,
    pub rel: f64,
    /// The player's own vibrato: cents (the sketch's depth times the pick's), Hz, the wait before it.
    pub vib_cents: f64,
    pub vib_hz: f64,
    pub vib_wait: f64,
    /// How much the air eases over a slur's change.
    pub slur_ease: f64,
    /// A half valve: how far down the valve is pressed (0 to 1), as the round's player pressed it.
    pub half_valve: f64,
    /// The trial runs: the lips' tensions tried (how many, from `amin`, or `amin_hi` from the 5th natural note up,
    /// to `amax`), from how long in the ear listens (s, x1.6 under 110 Hz), how much it minds a slow settle.
    pub nalpha: usize,
    pub amin: f64,
    pub amin_hi: f64,
    pub amax: f64,
    pub tun_a: f64,
    pub wsettle: f64,
    /// How many steps of the horn to one of the sample rate.
    pub os: usize,
    /// The horn player's hand in the bell: 0 none, 1 the usual.
    pub hand: f64,
    /// Each mute's level against the open horn, as the sketch set it on the pick's riff (the order of [`MUTES`]).
    pub mutes: [f64; 9],
    /// The pulse limiter's crest (0: none).
    pub crest: f64,
    /// The soft limiter: the horn's level that the round's page played at full scale.
    pub full: f64,
    /// The sound's level out.
    pub gain: f64,
    /// The trial runs made offline, if there are any.
    pub table: Option<&'static Table>,
    /// The highest note the player sounds reliably (MIDI): one above it is taken down the octave (the round's top
    /// notes that land between natural notes with some breaths).
    pub top: f64,
}

/// The trial runs' results for plain notes: each note from `low` to `high` at each strength step from `vels.0` to
/// `vels.1` (in twentieths), its horn's trim (cents), its lips' tension, and the fingering it took instead of the
/// usual one (natural note and lowering, 0 for the usual).
#[derive(Debug, PartialEq)]
pub struct Table {
    pub low: u8,
    pub high: u8,
    pub vels: (u8, u8),
    pub rows: &'static [(f64, f64, u8, u8)],
}

/// The mutes' shapes in the order of [`BrassParts::mutes`].
pub const MUTES: [&str; 9] = [
    "straight",
    "cup",
    "harmon",
    "harmon_stem",
    "plunger_closed",
    "plunger_open",
    "practice",
    "stopped",
    "brass",
];

/// The sketch's trumpet, on which every other is built.
const TRUMPET: BrassParts = BrassParts {
    name: "trumpet",
    m0: 46.0,
    m0b: 0.0,
    split: 200.0,
    valves: 6,
    minpart: 2,
    maxpart: 12,
    r1: 0.72,
    nmodes: 16,
    zc: 3.8e6,
    zp: 38e6,
    fmp: 800.0,
    sig: 1.3,
    fc: 1500.0,
    w1: 0.25,
    q0: 40.0,
    qlo: 24.0,
    mu: 1.5,
    ql: 15.0,
    lw: 0.009,
    lpress: 2.5,
    lr: 0.75,
    pmin: 1200.0,
    pmax: 11000.0,
    pexp: 0.4,
    fref: 466.0,
    lcyl: 1.6,
    loss_fc: 12000.0,
    bell_fc: 1100.0,
    bell2: 0.5,
    bell_lp: 9000.0,
    noise: 0.01,
    tnoise: 0.15,
    att: 0.003,
    over: 0.25,
    rel: 0.025,
    vib_cents: 10.0,
    vib_hz: 5.5,
    vib_wait: 0.25,
    slur_ease: 0.3,
    half_valve: 0.6,
    nalpha: 7,
    amin: -0.2,
    amin_hi: -0.6,
    amax: 0.0,
    tun_a: 0.30,
    wsettle: 0.3,
    os: 4,
    hand: 0.0,
    mutes: [1.0; 9],
    crest: 2.2,
    full: 1.0,
    gain: 1.0,
    table: None,
    top: 127.0,
};

impl BrassParts {
    /// Tp1, the orchestra C trumpet: clear and ringing, played straight with little vibrato.
    pub const TP1: BrassParts = BrassParts {
        name: "Tp1",
        m0: 48.0,
        fmp: 850.0,
        fc: 1600.0,
        lcyl: 1.5,
        bell_fc: 1150.0,
        bell_lp: 9500.0,
        vib_cents: 5.0,
        mutes: super::brass_tune::TP1_MUTES,
        full: super::brass_tune::TP1_FULL,
        gain: super::brass_tune::TP1_GAIN,
        table: super::brass_tune::TP1,
        ..TRUMPET
    };
    /// Co1, the brass band cornet: mellow and sweet, a gentle wavering vibrato.
    pub const CO1: BrassParts = BrassParts {
        name: "Co1",
        fmp: 700.0,
        fc: 1150.0,
        lcyl: 1.1,
        bell_fc: 950.0,
        bell_lp: 7000.0,
        zp: 34e6,
        vib_cents: 13.0,
        mutes: super::brass_tune::CO1_MUTES,
        full: super::brass_tune::CO1_FULL,
        gain: super::brass_tune::CO1_GAIN,
        table: super::brass_tune::CO1,
        ..TRUMPET
    };
    /// Fh1, the flugelhorn: soft, dark and velvety.
    pub const FH1: BrassParts = BrassParts {
        name: "Fh1",
        fmp: 600.0,
        fc: 950.0,
        lcyl: 0.7,
        bell_fc: 750.0,
        bell_lp: 5000.0,
        zp: 30e6,
        zc: 3.2e6,
        w1: 0.4,
        r1: 0.8,
        noise: 0.02,
        vib_cents: 10.0,
        half_valve: 0.55,
        mutes: super::brass_tune::FH1_MUTES,
        full: super::brass_tune::FH1_FULL,
        gain: super::brass_tune::FH1_GAIN,
        table: super::brass_tune::FH1,
        ..TRUMPET
    };
    /// Hn1, the double horn (F and B-flat), the player's hand in the bell: warm, round and noble.
    pub const HN1: BrassParts = BrassParts {
        name: "Hn1",
        m0: 29.0,
        m0b: 34.0,
        split: 64.0,
        minpart: 2,
        maxpart: 16,
        nmodes: 24,
        r1: 0.7,
        zc: 3.6e6,
        zp: 36e6,
        fmp: 500.0,
        sig: 1.4,
        fc: 1000.0,
        q0: 38.0,
        qlo: 20.0,
        mu: 1.8,
        lw: 0.010,
        lcyl: 1.4,
        bell_fc: 600.0,
        bell_lp: 4500.0,
        fref: 349.0,
        pmin: 1000.0,
        pmax: 9000.0,
        vib_cents: 6.0,
        amin: -0.6,
        nalpha: 9,
        hand: 1.0,
        mutes: super::brass_tune::HN1_MUTES,
        full: super::brass_tune::HN1_FULL,
        gain: super::brass_tune::HN1_GAIN,
        table: super::brass_tune::HN1,
        ..TRUMPET
    };
    /// Me1, the marching mellophone in F: bright, loud and direct, like a big trumpet.
    pub const ME1: BrassParts = BrassParts {
        name: "Me1",
        m0: 41.0,
        fmp: 650.0,
        fc: 1000.0,
        lcyl: 1.6,
        bell_fc: 850.0,
        bell_lp: 8000.0,
        zp: 32e6,
        zc: 3.4e6,
        maxpart: 12,
        fref: 392.0,
        amin: -0.6,
        nalpha: 9,
        vib_cents: 8.0,
        mutes: super::brass_tune::ME1_MUTES,
        full: super::brass_tune::ME1_FULL,
        gain: super::brass_tune::ME1_GAIN,
        table: super::brass_tune::ME1,
        top: 81.0,
        ..TRUMPET
    };
    /// Tb1, the orchestra tenor trombone with the F valve: full and round, then a great brassy roar when loud.
    pub const TB1: BrassParts = BrassParts {
        name: "Tb1",
        m0: 34.0,
        valves: 6,
        minpart: 1,
        maxpart: 10,
        nmodes: 16,
        r1: 0.7,
        zc: 2.4e6,
        zp: 26e6,
        fmp: 420.0,
        sig: 1.3,
        fc: 650.0,
        q0: 30.0,
        qlo: 16.0,
        mu: 2.2,
        lw: 0.013,
        lcyl: 2.0,
        bell_fc: 520.0,
        bell_lp: 7000.0,
        fref: 233.0,
        pmin: 900.0,
        pmax: 8000.0,
        vib_cents: 4.8,
        os: 4,
        mutes: super::brass_tune::TB1_MUTES,
        full: super::brass_tune::TB1_FULL,
        gain: super::brass_tune::TB1_GAIN,
        table: super::brass_tune::TB1,
        ..TRUMPET
    };
    /// Eu1, the four-valve euphonium: warm, rich and singing.
    pub const EU1: BrassParts = BrassParts {
        name: "Eu1",
        m0: 34.0,
        valves: 11,
        minpart: 1,
        maxpart: 12,
        nmodes: 16,
        r1: 0.88,
        w1: 0.5,
        zc: 2.0e6,
        zp: 22e6,
        fmp: 650.0,
        sig: 1.3,
        fc: 900.0,
        q0: 28.0,
        qlo: 14.0,
        mu: 2.4,
        lw: 0.014,
        lcyl: 1.2,
        bell_fc: 420.0,
        bell2: 0.3,
        bell_lp: 4000.0,
        fref: 233.0,
        pmin: 800.0,
        pmax: 7000.0,
        vib_cents: 9.0,
        amin: -0.6,
        nalpha: 9,
        wsettle: 0.6,
        mutes: super::brass_tune::EU1_MUTES,
        full: super::brass_tune::EU1_FULL,
        gain: super::brass_tune::EU1_GAIN,
        table: super::brass_tune::EU1,
        top: 72.0,
        ..TRUMPET
    };
    /// Tu1, the orchestra tuba in C: deep, solid and round.
    pub const TU1: BrassParts = BrassParts {
        name: "Tu1",
        m0: 24.0,
        valves: 11,
        minpart: 1,
        maxpart: 10,
        nmodes: 14,
        r1: 0.9,
        w1: 0.6,
        zc: 1.4e6,
        zp: 12e6,
        fmp: 230.0,
        sig: 1.3,
        fc: 500.0,
        q0: 26.0,
        qlo: 12.0,
        mu: 3.5,
        lw: 0.02,
        lr: 0.93,
        lcyl: 1.4,
        bell2: 0.25,
        bell_fc: 250.0,
        bell_lp: 2600.0,
        fref: 87.0,
        pmin: 600.0,
        pmax: 5500.0,
        pexp: 0.4,
        vib_cents: 6.0,
        os: 2,
        mutes: super::brass_tune::TU1_MUTES,
        full: super::brass_tune::TU1_FULL,
        gain: super::brass_tune::TU1_GAIN,
        table: super::brass_tune::TU1,
        ..TRUMPET
    };
    /// Su1, the brass sousaphone: big, punchy and buzzy, no vibrato.
    pub const SU1: BrassParts = BrassParts {
        name: "Su1",
        m0: 22.0,
        valves: 6,
        minpart: 1,
        maxpart: 10,
        nmodes: 14,
        r1: 0.88,
        w1: 0.55,
        zc: 1.4e6,
        zp: 12e6,
        fmp: 240.0,
        sig: 1.3,
        fc: 520.0,
        q0: 24.0,
        qlo: 12.0,
        mu: 3.5,
        lw: 0.02,
        lcyl: 1.6,
        bell2: 0.25,
        bell_fc: 280.0,
        bell_lp: 3200.0,
        fref: 87.0,
        pmin: 700.0,
        pmax: 6500.0,
        pexp: 0.4,
        vib_cents: 0.0,
        os: 2,
        mutes: super::brass_tune::SU1_MUTES,
        full: super::brass_tune::SU1_FULL,
        gain: super::brass_tune::SU1_GAIN,
        table: super::brass_tune::SU1,
        top: 56.0,
        ..TRUMPET
    };
    /// Every pick, for the table maker and the tests.
    pub const PICKS: [BrassParts; 9] = [
        BrassParts::TP1,
        BrassParts::CO1,
        BrassParts::FH1,
        BrassParts::HN1,
        BrassParts::ME1,
        BrassParts::TB1,
        BrassParts::EU1,
        BrassParts::TU1,
        BrassParts::SU1,
    ];

    /// Resonance n's part of the nominal fundamental.
    fn ratio(&self, n: usize) -> f64 {
        if n == 1 { self.r1 } else { n as f64 }
    }

    /// The pitch natural note n sounds at, as a multiple of the series' fundamental: the pedal (1) sounds the
    /// fundamental itself, held up by the 2nd, 3rd and 4th resonances though the bore's lowest sits well under it.
    fn prat(&self, n: usize) -> f64 {
        if n == 1 { 1.0 } else { self.ratio(n) }
    }

    /// The lips' own ring for natural note n: between it and the one under it, `alpha` of the way down.
    fn lip_ratio(&self, n: usize, alpha: f64) -> f64 {
        let below = if n > 1 { self.prat(n - 1) / self.prat(n) } else { 0.7 };
        let gap = (1.0 - below).min(0.3);
        1.0 - alpha * gap
    }

    /// The open horn's fundamental for a pitch (a double horn's side).
    fn side(&self, midi: f64) -> f64 {
        if self.m0b > 0.0 && midi >= self.split {
            self.m0b
        } else {
            self.m0
        }
    }

    /// The natural note and the valves (or slide) for a note, and the open horn's MIDI: the asked fingering if both
    /// are asked (a lowering of 100 or more the double horn's other side), else the player's (few valves, lipping a
    /// natural note's own few cents rather than reaching for a higher one with more tubing; 14 cents of lipping
    /// weighs about as much as three semitones of valves; a pedal note only when nothing else has it).
    fn fingering(&self, n: &Ln) -> (usize, i32, f64) {
        // a lowering of 100 or more: the double horn's other side (a single horn's only one), 100 under
        let (m0, want) = match n.val {
            Some(v) if v >= 100 => {
                let m0 = if self.m0b > 0.0 && n.midi < self.split {
                    self.m0b
                } else {
                    self.m0
                };
                (m0, Some(v - 100))
            }
            v => (self.side(n.midi), v),
        };
        if let (Some(p), Some(v)) = (n.part, want) {
            return (p, v, m0);
        }
        let mut best = (2usize, 0i32, 1e9f64);
        for p in self.minpart..=self.maxpart {
            if p == 7 || p == 11 || p == 13 || p == 14 || n.part.is_some_and(|pp| pp != p) {
                continue;
            }
            let mp = m0 + 12.0 * dmath::log2(self.prat(p));
            for v in 0..=self.valves {
                if want.is_some_and(|vv| vv != v) {
                    continue;
                }
                let err = (mp - f64::from(v) - n.midi).abs();
                let cost = err * 3.0 + f64::from(v) * 0.15 + if p == 1 { 3.0 } else { 0.0 };
                if err < 0.6 && cost < best.2 {
                    best = (p, v, cost);
                }
            }
        }
        (best.0, best.1, m0)
    }

    /// Every natural note and valve set that reaches the pitch, best first (the other side of a double horn with
    /// 100 added and a small cost).
    fn fingerings(&self, n: &Ln) -> Vec<(usize, i32)> {
        let m0 = self.side(n.midi);
        let mut c: Vec<(f64, usize, i32)> = Vec::new();
        let mut sides = vec![(m0, 0i32, 0.0)];
        if self.m0b > 0.0 {
            let other = if n.midi >= self.split { self.m0 } else { self.m0b };
            sides.push((other, 100, 0.5));
        }
        for (m0, tag, pen) in sides {
            for p in self.minpart..=self.maxpart {
                if p == 7 || p == 11 || p == 13 || p == 14 {
                    continue;
                }
                let mp = m0 + 12.0 * dmath::log2(self.prat(p));
                for v in 0..=self.valves {
                    let err = (mp - f64::from(v) - n.midi).abs();
                    if err < 0.6 {
                        c.push((
                            err * 3.0 + f64::from(v) * 0.15 + if p == 1 { 3.0 } else { 0.0 } + pen,
                            p,
                            v + tag,
                        ));
                    }
                }
            }
        }
        c.sort_by(|a, b| a.0.total_cmp(&b.0));
        c.into_iter().map(|x| (x.1, x.2)).collect()
    }

    /// The resonances' height at `f`: strongest round the mouthpiece's ring, gone above the bell's cutoff.
    fn peak(&self, f: f64) -> f64 {
        let o = dmath::log2(f / self.fmp) / self.sig;
        let g = dmath::exp(-0.5 * o * o);
        let x = f / self.fc;
        let x2 = x * x;
        g / (1.0 + x2 * x2)
    }

    /// The resonances' sharpness at `f`.
    fn qf(&self, f: f64) -> f64 {
        let x = (f / self.fmp).min(1.0);
        let q = self.qlo + (self.q0 - self.qlo) * x;
        let y = f / self.fc;
        let y2 = y * y;
        q / (1.0 + y2 * y2 * 0.5)
    }

    /// The mouth's pressure for a strength at a pitch, Pa.
    fn press(&self, vel: f64, f: f64) -> f64 {
        self.pmin * dmath::pow(self.pmax / self.pmin, vel.clamp(0.0, 1.2)) * dmath::pow(f / self.fref, self.pexp)
    }
}

/// The stage 1 helpers the flutes share.
pub(super) fn hz(m: f64) -> f64 {
    440.0 * dmath::pow(2.0, (m - 69.0) / 12.0)
}

pub(super) fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

pub(super) fn pow2(x: f64) -> f64 {
    dmath::pow(2.0, x)
}

/// The breath's turbulence (the sketch's generator, xorshift with a multiply, Box and Muller's normal).
pub(super) struct Breath(u64);

impl Breath {
    pub(super) fn new(seed: u64) -> Breath {
        Breath(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
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

    pub(super) fn gauss(&mut self) -> f64 {
        let u1 = self.uni().max(1e-12);
        let u2 = self.uni();
        (-2.0 * dmath::ln(u1)).sqrt() * dmath::cos(2.0 * dmath::PI * u2)
    }

    /// The generator moved on past one draw, as [`Breath::gauss`] moves it, without working it.
    fn skip(&mut self) {
        self.next();
        self.next();
    }
}

/// A resonance as a topology-preserving state-variable filter (safe to retune while it rings), band-pass with a peak
/// gain of 1.
#[derive(Clone, Copy, Default)]
pub(super) struct Svf {
    ic1: f64,
    ic2: f64,
    a1: f64,
    a2: f64,
    a3: f64,
    k: f64,
}

impl Svf {
    pub(super) fn set(&mut self, f: f64, q: f64, fs: f64) {
        let g = dmath::tan(dmath::PI * (f / fs).min(0.49));
        self.k = 1.0 / q;
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    /// The output it would give with no new input.
    fn hist(&self) -> f64 {
        self.k * (self.a1 * self.ic1 - self.a2 * self.ic2)
    }

    /// The output's part of a new input.
    fn gain(&self) -> f64 {
        self.k * self.a2
    }

    pub(super) fn tick(&mut self, x: f64) -> f64 {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        self.k * v1
    }
}

/// A one-pole low-pass or high-pass.
#[derive(Clone, Copy, Default)]
pub(super) struct OnePole {
    pub(super) a: f64,
    y: f64,
    x1: f64,
}

impl OnePole {
    pub(super) fn lp(fc: f64, fs: f64) -> OnePole {
        OnePole {
            a: dmath::exp(-2.0 * dmath::PI * fc / fs),
            ..OnePole::default()
        }
    }

    pub(super) fn lpt(&mut self, x: f64) -> f64 {
        self.y = x + self.a * (self.y - x);
        self.y
    }

    pub(super) fn hpt(&mut self, x: f64) -> f64 {
        self.y = self.a * (self.y + x - self.x1);
        self.x1 = x;
        self.y
    }
}

pub(super) fn lp1(x: &mut [f64], fc: f64, fs: f64) {
    let mut f = OnePole::lp(fc, fs);
    x.iter_mut().for_each(|v| *v = f.lpt(*v));
}

pub(super) fn hp1(x: &mut [f64], fc: f64, fs: f64) {
    let mut f = OnePole::lp(fc, fs);
    x.iter_mut().for_each(|v| *v = f.hpt(*v));
}

/// How a note starts: the tongue's "t", its back "k", a soft "d", or the breath alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tongue {
    T,
    K,
    D,
    B,
}

/// A note as the sketch takes it (its names), made from a [`Play`].
#[derive(Clone, Debug)]
pub(super) struct Ln {
    pub(super) t: f64,
    pub(super) midi: f64,
    pub(super) dur: f64,
    pub(super) vel: f64,
    pub(super) tongue: Tongue,
    pub(super) acc: f64,
    /// A sforzando: (its start over the note's strength, where it falls back to, how quickly, s).
    pub(super) sfz: Option<(f64, f64, f64)>,
    pub(super) tie: bool,
    pub(super) slur: Option<f64>,
    pub(super) glide: Option<f64>,
    /// A scoop's start, cents off the pitch.
    pub(super) from: f64,
    pub(super) bend: Option<(f64, f64, f64)>,
    pub(super) bend2: Option<(f64, f64, f64)>,
    pub(super) swell: Option<(f64, f64, f64)>,
    /// The breath over the note (`expression`, off a plunger): a second swell.
    pub(super) swell2: Option<(f64, f64, f64)>,
    pub(super) vib: Option<(f64, f64, f64)>,
    pub(super) shake: bool,
    pub(super) flutter: bool,
    pub(super) growl: bool,
    pub(super) half: f64,
    pub(super) part: Option<usize>,
    pub(super) val: Option<i32>,
    /// The breath's stop, s.
    pub(super) rel: f64,
}

/// A note held this long or longer has the player's own vibrato when none is asked, s.
pub(super) const VIB_FROM: f64 = 0.5;
/// A tongue stop's release, s; a clipped note's longest hold, s.
pub(super) const CHOKED: f64 = 0.005;
pub(super) const CLIPPED: f64 = 0.06;

fn three(s: Option<Swell>) -> Option<(f64, f64, f64)> {
    s.filter(|s| s.from.is_finite() && s.to.is_finite() && s.over.is_finite())
        .map(|s| (s.from.clamp(0.0, 2.0), s.to.clamp(0.0, 2.0), s.over.max(1e-3)))
}

/// A breath's shape over a note: from, to, over how long (s).
pub(super) type Shape = Option<(f64, f64, f64)>;

/// A sforzando, a crescendo, the breath over the note: `swell` from over 1 is a sforzando (its start over the
/// strength, where it falls back to, how quickly), else the crescendo; `expression` off a plunger the breath over
/// the note on top.
pub(super) fn swells(play: &Play) -> (Shape, Shape, Shape) {
    let s = three(play.swell);
    let e = if play.mute == Mute::Plunger {
        None
    } else {
        three(play.expression)
    };
    match s {
        Some(x) if x.0 > 1.0 => (Some(x), e, None),
        _ => (None, s, e),
    }
}

/// The asked vibrato, or the player's own on a note held long enough (`cents`, `hz`, `wait`).
pub(super) fn vib_of(play: &Play, own: (f64, f64, f64)) -> Option<(f64, f64, f64)> {
    match play.vibrato {
        Some(v) if v.cents.is_finite() && v.hz.is_finite() && v.wait.is_finite() => {
            (v.cents > 0.0).then_some((v.cents.min(200.0), v.hz.clamp(0.1, 20.0), v.wait.max(0.0)))
        }
        _ => (own.0 > 0.0 && play.dur >= VIB_FROM).then_some(own),
    }
}

/// The note's held length as played: a clipped note as short as a tongue goes.
pub(super) fn held(play: &Play) -> f64 {
    let d = if play.dur.is_finite() { play.dur.max(0.0) } else { 0.0 };
    if play.mute == Mute::Clipped { d.min(CLIPPED) } else { d }
}

pub(super) fn unit(x: f64) -> f64 {
    if x.is_finite() { x.clamp(0.0, 1.0) } else { 0.0 }
}

impl Ln {
    /// The note `play` on `p`, starting `t` seconds into its line.
    fn of(p: &BrassParts, play: &Play, t: f64) -> Ln {
        let (sfz, swell, swell2) = swells(play);
        let mut n = Ln {
            t,
            midi: play.midi.clamp(0.0, 127.0),
            dur: held(play),
            vel: if play.vel.is_finite() {
                play.vel.clamp(0.0, 1.2)
            } else {
                0.0
            },
            tongue: match play.technique {
                Technique::BackTongue => Tongue::K,
                Technique::SoftTongue => Tongue::D,
                Technique::Breath => Tongue::B,
                _ => Tongue::T,
            },
            acc: unit(play.accent),
            sfz,
            tie: play.tie,
            slur: play.slur_from.filter(|s| s.is_finite()).map(|s| s.clamp(0.0, 127.0)),
            glide: None,
            from: 0.0,
            bend: None,
            bend2: None,
            swell,
            swell2,
            vib: vib_of(play, (p.vib_cents, p.vib_hz, p.vib_wait)),
            shake: play.technique == Technique::Shake,
            flutter: play.technique == Technique::Flutter,
            growl: play.technique == Technique::Growl,
            half: if play.technique == Technique::HalfValve {
                p.half_valve
            } else {
                0.0
            },
            part: match play.technique {
                Technique::Harmonic(k) if k >= 1 => Some(usize::from(k).min(p.maxpart.max(16))),
                _ => None,
            },
            val: play.string.map(i32::from),
            rel: if matches!(play.mute, Mute::Choked | Mute::Clipped) {
                CHOKED
            } else {
                p.rel
            },
        };
        let bend = play
            .bend
            .filter(|b| b.cents.is_finite() && b.at.is_finite() && b.over.is_finite())
            .map(|b| (b.cents.clamp(-2400.0, 2400.0), b.at, b.over));
        let bend = match bend {
            Some((c, _, over)) if c == 0.0 && n.slur.is_some() => {
                n.glide = Some(over.max(0.0));
                None
            }
            b => b,
        };
        match play
            .scoop
            .filter(|b| b.cents.is_finite() && b.at.is_finite() && b.over.is_finite())
        {
            Some(s) => {
                let c = s.cents.clamp(-2400.0, 2400.0);
                n.from = c;
                n.bend = Some((-c, s.at, s.over));
                n.bend2 = bend;
            }
            None => n.bend = bend,
        }
        n
    }

    /// The note as the ear tries it: alone, held, tongued, with nothing else.
    fn plain(&self) -> Ln {
        Ln {
            t: 0.0,
            tie: false,
            slur: None,
            glide: None,
            from: 0.0,
            bend: None,
            bend2: None,
            swell: None,
            swell2: None,
            sfz: None,
            vib: None,
            shake: false,
            flutter: false,
            growl: false,
            half: 0.0,
            tongue: Tongue::T,
            ..self.clone()
        }
    }
}

/// A note's pitch at `tau` seconds in: (cents off the note, the valves half down, the shake 0 to 1).
pub(super) fn note_bend(n: &Ln, tau: f64) -> (f64, f64, f64) {
    let mut c = n.from;
    let mut half = n.half;
    let bendf = |b: (f64, f64, f64), c: &mut f64, half: &mut f64| {
        let (bc, at, over) = b;
        let x = ((tau - at) / over.max(1e-3)).clamp(0.0, 1.0);
        // a fall gathers speed
        let s = if bc < 0.0 { x * x } else { smooth(x) };
        *c += bc * s;
        // the valves half down while the pitch slides further than the lips can pull it
        if bc.abs() > 60.0 && tau > at - 0.02 && tau < at + over + 0.03 {
            *half = half.max(0.55 * (bc.abs() / 400.0).min(1.0));
        }
    };
    if let Some(b) = n.bend {
        bendf(b, &mut c, &mut half);
    }
    if let Some(b) = n.bend2 {
        bendf(b, &mut c, &mut half);
    }
    if n.from.abs() > 60.0 {
        let b = n.bend.map_or(0.08, |b| b.2);
        if tau < b + 0.02 {
            half = half.max(0.5 * (n.from.abs() / 300.0).min(1.0) * (1.0 - tau / (b + 0.02)));
        }
    }
    if let Some((vc, r, d)) = n.vib {
        let grow = smooth((tau - d) / 0.35);
        c += vc * grow * dmath::sin(2.0 * dmath::PI * r * (tau - d).max(0.0));
    }
    let sh = if n.shake {
        let x = (tau - 0.15).max(0.0);
        if x > 0.0 {
            // a lip trill: up to the next natural note and back, squarish
            let ph = (x * 6.5).fract();
            let s = 0.5 + 0.5 * dmath::sin(2.0 * dmath::PI * ph - dmath::PI / 2.0);
            smooth((s - 0.25) * 2.0)
        } else {
            0.0
        }
    } else {
        0.0
    };
    (c, half, sh)
}

/// The steps of a line between the breath's and the horn's settings, the flutes' too.
pub(super) const BLOCK: usize = 16;

/// How many of the breath's draws one batch of [`ahead`] holds.
const AHEAD: usize = 4096;

/// The breath's turbulence drawn ahead on another thread, `count` draws in the generator's own order (so the very
/// numbers drawing them in turn would give), for `f` to take in turn ([`Ahead::gauss`]): a line spends about a
/// fifth of its time on them.
pub(super) fn ahead<R>(seed: u64, count: usize, f: impl FnOnce(&mut Ahead) -> R) -> R {
    ahead_unused(seed, count, None, f)
}

/// [`ahead`], every other draw from `unused` on (when given) taken from the generator unworked (it moves on as
/// drawing it would) and handed over as 0: the tongue's puffs after the last tongued start, never heard.
fn ahead_unused<R>(seed: u64, count: usize, unused: Option<usize>, f: impl FnOnce(&mut Ahead) -> R) -> R {
    std::thread::scope(|s| {
        let (tx, rx) = std::sync::mpsc::sync_channel(8);
        s.spawn(move || {
            let mut b = Breath::new(seed);
            let mut left = count;
            let mut i = 0;
            while left > 0 {
                let n = left.min(AHEAD);
                let batch: Vec<f64> = (0..n)
                    .map(|_| {
                        i += 1;
                        match unused {
                            Some(u) if i > u && (i - 1 - u) % 2 == 0 => {
                                b.skip();
                                0.0
                            }
                            _ => b.gauss(),
                        }
                    })
                    .collect();
                left -= n;
                if tx.send(batch).is_err() {
                    break;
                }
            }
        });
        let mut a = Ahead {
            batch: Vec::new(),
            at: 0,
            rx,
        };
        let r = f(&mut a);
        // hung up: a draw still being made is let go
        drop(a);
        r
    })
}

/// The breath's draws made ahead by [`ahead`], taken in turn.
pub(super) struct Ahead {
    batch: Vec<f64>,
    at: usize,
    rx: std::sync::mpsc::Receiver<Vec<f64>>,
}

impl Ahead {
    /// The next draw: the breath's [`Breath::gauss`] as it comes.
    pub(super) fn gauss(&mut self) -> f64 {
        if self.at == self.batch.len() {
            self.batch = self.rx.recv().expect("the draws a line was counted to take");
            self.at = 0;
        }
        self.at += 1;
        self.batch[self.at - 1]
    }
}

/// One player playing a line of notes on one horn (in time order) at the horn's rate `fs`: the mouthpiece's pressure.
/// `corr` is each note's horn trim (cents) and lips' tension.
fn line(b: &BrassParts, notes: &[Ln], corr: &[(f64, f64)], n_s: usize, seed: u64, fs: f64) -> Vec<f64> {
    // two draws a step (the breath's turbulence and the tongue's puff), two more on a growl, one a block on a flutter
    let growl = notes.iter().any(|n| n.growl);
    let flutter = notes.iter().any(|n| n.flutter);
    let draws = n_s * if growl { 4 } else { 2 } + if flutter { n_s / BLOCK + 1 } else { 0 };
    // the puff is heard only over a tongued start's first 30 ms (a block at a time): from two blocks after the last
    // note's, its draws go unworked (the turbulence's, between them, drawn as ever) and it is left out
    let quiet = (!growl && !flutter).then(|| {
        let last = notes.iter().map(|n| n.t).fold(0.0, f64::max);
        ((last + 0.03) * fs) as usize + 2 * BLOCK
    });
    ahead_unused(seed, draws, quiet.map(|q| 2 * q + 1), |rng| {
        line_in(b, notes, corr, n_s, quiet, rng, fs)
    })
}

fn line_in(
    b: &BrassParts,
    notes: &[Ln],
    corr: &[(f64, f64)],
    n_s: usize,
    quiet: Option<usize>,
    rng: &mut Ahead,
    fs: f64,
) -> Vec<f64> {
    const RHO: f64 = 1.2;
    let dt = 1.0 / fs;
    let nm = b.nmodes;
    let fing: Vec<(usize, i32, f64)> = notes.iter().map(|n| b.fingering(n)).collect();
    let mut modes = vec![Svf::default(); nm];
    let mut zn = vec![0.0; nm];
    let mut hp_u = OnePole::lp(b.fc, fs);
    let mut hp_u2 = OnePole::lp(b.fc, fs);
    let mut nz_lp = OnePole::lp(1500.0, fs);
    let mut tn_lp = OnePole::lp(3000.0, fs);
    let k_mouth = 1.0 - dmath::exp(-dt / 0.002);
    let k_lips = 1.0 - dmath::exp(-dt / 1e-7);
    let (mut y, mut yv, mut dp) = (0.0f64, 0.0f64, 0.0f64);
    let mut out = vec![0.0; n_s];
    let (mut cur_f, mut cur_l) = (-1.0f64, -1.0f64);
    let mut seg_from = (0.0, 0.0);
    let (mut seg_t0, mut seg_tb, mut seg_tl) = (0.0, 0.006, 0.006);
    let mut idx: Option<usize> = None;
    let (mut pm_s, mut pm_l) = (0.0f64, 0.0f64);
    let mut gate_prev = 0.0;
    let mut growl_ph = 0.0f64;
    let mut flut_ph = 0.0;
    let (mut pm_tgt, mut lipw, mut tn_amp) = (0.0, 2.0 * dmath::PI * 400.0, 0.0);
    let mut zmodes = 0.0;
    let mut set_for = [u64::MAX; 3];
    // the resonances' memory summed as each step leaves them, for the next step (summed afresh after a retuning)
    let mut ph_next: Option<f64> = None;
    for (k, o) in out.iter_mut().enumerate() {
        let now = k as f64 * dt;
        if k % BLOCK == 0 {
            // the note sounding now: the last one started
            let mut ni = idx;
            while ni.map_or(0, |i| i + 1) < notes.len() && notes[ni.map_or(0, |i| i + 1)].t <= now {
                ni = Some(ni.map_or(0, |i| i + 1));
            }
            if ni != idx {
                let i = ni.unwrap_or(0);
                let n = &notes[i];
                let (part, val, m0) = fing[i];
                let fser = hz(m0 - f64::from(val)) * pow2(corr[i].0 / 1200.0);
                let fnote = fser * b.prat(part);
                let lrr = b.lip_ratio(part, corr[i].1);
                let tb = dmath::log2(fser);
                let tl = dmath::log2(fnote * lrr);
                let slurred = n.slur.is_some() && i > 0 && notes[i - 1].tie;
                seg_from = if cur_f < 0.0 { (tb, tl) } else { (cur_f, cur_l) };
                seg_t0 = n.t;
                if slurred {
                    // a lip slur: the lips move, the horn stays; a valve slur: the valves move in 12 ms; a slide
                    // as asked
                    seg_tb = n.glide.unwrap_or(0.012);
                    seg_tl = n.glide.unwrap_or(0.03).max(0.02);
                } else {
                    seg_tb = 0.006;
                    seg_tl = 0.004;
                    if gate_prev < 0.05 {
                        seg_from = (tb, tl);
                    }
                }
                idx = ni;
            }
            if let Some(i) = idx {
                let n = &notes[i];
                let tau = now - n.t;
                let (part, val, m0) = fing[i];
                let fser0 = hz(m0 - f64::from(val)) * pow2(corr[i].0 / 1200.0);
                let (cents, half, shake) = note_bend(n, tau);
                let slurred = n.slur.is_some() && i > 0 && notes[i - 1].tie;
                let bendk = pow2(cents / 1200.0);
                let lrr = b.lip_ratio(part, corr[i].1);
                let tb = dmath::log2(fser0 * bendk);
                let mut tl = dmath::log2(fser0 * bendk * b.prat(part) * lrr);
                if shake > 0.0 {
                    // the lips tighten to the next natural note up and back
                    let up = (b.prat(part + 1) * b.lip_ratio(part + 1, corr[i].1)) / (b.prat(part) * lrr);
                    tl += shake * dmath::log2(up);
                }
                let xb = smooth((now - seg_t0) / seg_tb);
                let xl = smooth((now - seg_t0) / seg_tl);
                cur_f = seg_from.0 + (tb - seg_from.0) * xb;
                cur_l = seg_from.1 + (tl - seg_from.1) * xl;
                // the breath
                let fnote = pow2(tb) * b.prat(part);
                let p = b.press(n.vel, fnote);
                let mut gate;
                if slurred || n.tongue == Tongue::B {
                    gate = if n.tongue == Tongue::B && !slurred {
                        smooth(tau / 0.06)
                    } else {
                        1.0
                    };
                    if slurred && tau < seg_tl * 1.5 {
                        // over a lip slur's change the player eases the air a little, so the lips cross between
                        // natural notes without a blurt
                        let x = (tau / (seg_tl * 1.5)).clamp(0.0, 1.0);
                        gate *= 1.0 - b.slur_ease * dmath::sin(dmath::PI * x);
                    }
                } else {
                    let (at, ov) = match n.tongue {
                        Tongue::K => (b.att * 1.4, b.over * 0.5),
                        Tongue::D => (b.att * 2.0, 0.0),
                        _ => (b.att, b.over),
                    };
                    let at = at * (1.0 - 0.4 * n.acc);
                    gate = smooth(tau / at);
                    let ov = ov + 0.3 * n.acc;
                    gate *= 1.0 + ov * dmath::exp(-(tau / 0.04)) * smooth(tau / at);
                    if n.tongue == Tongue::K {
                        gate *= 0.92;
                    }
                }
                if let Some((a, z, over)) = n.sfz {
                    gate *= 1.0 + (a - 1.0) * dmath::exp(-(tau / 0.05));
                    gate *= z + (1.0 - z) * dmath::exp(-(tau / over)).max(0.0);
                }
                for (a, z, over) in [n.swell, n.swell2].into_iter().flatten() {
                    gate *= a + (z - a) * smooth(tau / over);
                }
                // the stop
                let mut stopk = 1.0;
                if !n.tie && tau > n.dur {
                    stopk = 1.0 - smooth((tau - n.dur) / n.rel);
                    gate *= stopk;
                }
                if n.flutter {
                    // the tongue rolls an R: the air cut and let go 24 times a second
                    flut_ph += 24.0 * BLOCK as f64 * dt * (1.0 + 0.1 * rng.gauss());
                    let s = 0.5 + 0.5 * dmath::sin(2.0 * dmath::PI * flut_ph);
                    gate *= 1.0 - 0.75 * s * smooth(tau / 0.05);
                }
                // a fall's far end is played softer (it dies away)
                if let Some((bc, at, over)) = n.bend
                    && bc < -150.0
                {
                    let x = ((tau - at) / over).clamp(0.0, 1.0);
                    gate *= 1.0 - 0.8 * x * x;
                }
                pm_tgt = p * gate.max(0.0);
                gate_prev = gate;
                // the tongue's puff at a tongued start
                tn_amp = if !slurred && n.tongue != Tongue::B && tau < 0.03 {
                    b.tnoise * (1.0 - tau / 0.03) * (1.0 + n.acc)
                } else {
                    0.0
                };
                // the horn: once the breath stops, its tuning eases onto the note just played (our lips pull low
                // notes further under the resonances than real lips do; a pedal takes its second harmonic's)
                let fser = pow2(cur_f);
                let stop_scale = if stopk < 1.0 {
                    let rm = part.max(2);
                    let s = hz(n.midi) * (rm as f64 / part.max(1) as f64) / (fser0 * b.ratio(rm));
                    dmath::pow(s, 1.0 - stopk)
                } else {
                    1.0
                };
                // the resonances follow the horn's tuning (set afresh only when it moved since the last block)
                let tuning = [fser.to_bits(), stop_scale.to_bits(), half.to_bits()];
                if tuning != set_for {
                    set_for = tuning;
                    ph_next = None;
                    zmodes = 0.0;
                    for (m, (mode, z)) in modes.iter_mut().zip(zn.iter_mut()).enumerate() {
                        let mut f = fser * b.ratio(m + 1) * stop_scale;
                        let mut q = b.qf(f);
                        let mut zz = b.zp * b.peak(f) * if m == 0 { b.w1 } else { 1.0 };
                        if half > 0.0 {
                            q *= 1.0 - 0.7 * half;
                            zz *= 1.0 - 0.55 * half;
                            if m % 2 == 1 {
                                f *= pow2(40.0 * half / 1200.0);
                            }
                        }
                        if f > 0.45 * fs {
                            f = 0.45 * fs;
                            zz = 0.0;
                        }
                        mode.set(f, q.max(2.0), fs);
                        *z = zz;
                        zmodes += zz * mode.gain();
                    }
                }
                lipw = 2.0 * dmath::PI * pow2(cur_l);
            } else {
                pm_tgt = 0.0;
            }
        }
        // the mouth: the pressure follows the breath quickly, with turbulence and a growl
        pm_s += (pm_tgt - pm_s) * k_mouth;
        let mut pm = pm_s * (1.0 + b.noise * nz_lp.lpt(rng.gauss()) * 3.0);
        if idx.is_some_and(|i| notes[i].growl) {
            // the throat sings a rough low note into the breath
            growl_ph += dt * 92.0 * (1.0 + 0.03 * rng.gauss());
            let g = (growl_ph.fract() * 2.0 - 1.0) + 0.6 * rng.gauss();
            pm *= 1.0 + 0.22 * g;
        }
        // the horn's memory: the pressure at the mouthpiece if no new air came in
        let mut ph = ph_next.unwrap_or_else(|| {
            let mut ph = 0.0;
            for (mode, z) in modes.iter().zip(&zn) {
                ph += z * mode.hist();
            }
            ph
        });
        // the plain tube's resistance above the cutoff (two one-pole high-passes of the flow in a row)
        let ahp = hp_u.a;
        let h1 = ahp * (hp_u.y - hp_u.x1);
        let h2 = ahp * (hp_u2.y - hp_u2.x1);
        ph += b.zc * (ahp * h1 + h2);
        let zeff = zmodes + b.zc * ahp * ahp;
        // the lips (semi-implicit), inward striking: they rest apart and the mouth's pressure squeezes them shut
        let wl = lipw;
        pm_l += (pm_s - pm_l) * k_lips;
        let yr = b.lpress * pm_l / (b.mu * wl * wl);
        let acc = -(wl / b.ql) * yv - wl * wl * (y - yr) - dp / b.mu;
        yv += dt * acc;
        y += dt * yv;
        let a = b.lw * y.max(0.0);
        let d = pm - ph;
        let bq = 2.0 * a * a * zeff / RHO;
        let cq = 2.0 * a * a * d.abs() / RHO;
        let mut u = d.signum() * (-bq + (bq * bq + 4.0 * cq).sqrt()) * 0.5;
        let puff = rng.gauss();
        if quiet.is_none_or(|q| k < q) {
            u += tn_amp * 1e-4 * tn_lp.lpt(puff);
        }
        let p = ph + zeff * u;
        let mut next = 0.0;
        for (mode, z) in modes.iter_mut().zip(&zn) {
            mode.tick(u);
            next += z * mode.hist();
        }
        ph_next = Some(next);
        let y1 = hp_u.hpt(u);
        hp_u2.hpt(y1);
        dp = pm - p;
        *o = p;
    }
    out
}

/// The wave's loud peaks run ahead of its troughs over the cylindrical length: each sample arrives early by
/// L (gamma+1)/2 p / (rho c^3); where a peak would pass the trough ahead of it, a shock (a jump) forms.
fn steepen(x: &[f64], lcyl: f64, fs: f64) -> Vec<f64> {
    const RHO: f64 = 1.2;
    const C: f64 = 343.0;
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let k = lcyl * 1.2 / (RHO * C * C * C) * fs;
    let d0 = k * 40000.0;
    let mut out = vec![0.0; n];
    let mut prev_a = d0 - k * x[0];
    let mut prev_x = x[0];
    let mut m = prev_a.ceil().max(0.0) as usize;
    for (i, &xi) in x.iter().enumerate().skip(1) {
        let mut a = i as f64 + d0 - k * xi;
        if a < prev_a + 0.05 {
            a = prev_a + 0.05;
        }
        while (m as f64) <= a && m < n {
            let f = (m as f64 - prev_a) / (a - prev_a);
            out[m] = prev_x + (xi - prev_x) * f.clamp(0.0, 1.0);
            m += 1;
        }
        prev_a = a;
        prev_x = xi;
    }
    // shift back by the constant delay
    let sh = d0.round().max(0.0) as usize;
    let mut y = vec![0.0; n];
    let kept = n.saturating_sub(sh);
    y[..kept].copy_from_slice(&out[sh..sh + kept]);
    y
}

/// A windowed-sinc low-pass and every `os`-th sample kept, `n` of them.
pub(super) fn down(x: &[f64], os: usize, n: usize) -> Vec<f64> {
    if os == 1 {
        let mut y = x.to_vec();
        y.resize(n, 0.0);
        return y;
    }
    let taps = 32 * os + 1;
    let c = (taps / 2) as isize;
    let fc = 0.45 / os as f64;
    let h: Vec<f64> = (0..taps)
        .map(|k| {
            let m = k as isize - c;
            let s = if m == 0 {
                2.0 * fc
            } else {
                dmath::sin(2.0 * dmath::PI * fc * m as f64) / (dmath::PI * m as f64)
            };
            let w = 0.42 - 0.5 * dmath::cos(2.0 * dmath::PI * k as f64 / (taps - 1) as f64)
                + 0.08 * dmath::cos(4.0 * dmath::PI * k as f64 / (taps - 1) as f64);
            s * w
        })
        .collect();
    let one = |i: usize| {
        let ci = (i * os) as isize;
        let mut acc = 0.0;
        let j0 = ci - c;
        if j0 >= 0 && j0 as usize + taps <= x.len() {
            // inside: the same products summed in the same order
            for (hk, v) in h.iter().zip(&x[j0 as usize..j0 as usize + taps]) {
                acc += hk * v;
            }
            return acc;
        }
        for (k, hk) in h.iter().enumerate() {
            let j = ci + k as isize - c;
            if j >= 0 && (j as usize) < x.len() {
                acc += hk * x[j as usize];
            }
        }
        acc
    };
    let mut y = vec![0.0; n];
    let mut i = 0;
    while i < n {
        let j0 = (i * os) as isize - c;
        if i + 4 <= n && j0 >= 0 && j0 as usize + 3 * os + taps <= x.len() {
            // four outputs inside at once, each its own sum in its own order (the same numbers, four at a time)
            let j0 = j0 as usize;
            let (x0, x1) = (&x[j0..j0 + taps], &x[j0 + os..j0 + os + taps]);
            let (x2, x3) = (&x[j0 + 2 * os..j0 + 2 * os + taps], &x[j0 + 3 * os..j0 + 3 * os + taps]);
            let (mut a0, mut a1, mut a2, mut a3) = (0.0, 0.0, 0.0, 0.0);
            for k in 0..taps {
                let hk = h[k];
                a0 += hk * x0[k];
                a1 += hk * x1[k];
                a2 += hk * x2[k];
                a3 += hk * x3[k];
            }
            y[i..i + 4].copy_from_slice(&[a0, a1, a2, a3]);
            i += 4;
        } else {
            y[i] = one(i);
            i += 1;
        }
    }
    y
}

/// The horn's sound for a line: steepened, the walls' loss, the bell, brought down to `fs` (`n` samples).
fn bell(b: &BrassParts, mouth: &[f64], fs: f64, n: usize) -> Vec<f64> {
    let fs_s = fs * b.os as f64;
    let half: Vec<f64> = mouth.iter().map(|p| 0.5 * p).collect();
    let mut x = steepen(&half, b.lcyl, fs_s);
    // the wall's loss, the bell's two high-passes and its soft top, one after the other on each sample in turn
    let (mut loss, mut bell1, mut bell2) = (
        OnePole::lp(b.loss_fc.min(0.45 * fs_s), fs_s),
        OnePole::lp(b.bell_fc, fs_s),
        OnePole::lp(b.bell_fc * b.bell2, fs_s),
    );
    let mut top = OnePole::lp(b.bell_lp.min(0.45 * fs_s), fs_s);
    for v in x.iter_mut() {
        *v = top.lpt(bell2.hpt(bell1.hpt(loss.lpt(*v))));
    }
    down(&x, b.os, n)
}

/// The pitch (Hz) of `x` near `f` (searched from 0.45 f to 2.2 f) and how clearly: the normalized autocorrelation's
/// first strong peak (its sums by the transform, the sketch's to rounding).
pub(super) fn pitch_of(x: &[f64], fs: f64, f: f64) -> (f64, f64) {
    if f.is_nan() || f <= 0.0 {
        return (0.0, 0.0);
    }
    let pmin = (fs / (f * 2.2)) as usize;
    let pmax = (fs / (f * 0.45)) as usize;
    let n = x.len();
    if n < 2 * pmax + 10 || pmin < 2 {
        return (0.0, 0.0);
    }
    let w = n - pmax - 1;
    let size = n.next_power_of_two();
    let fft = Fft::new(size);
    let (mut xr, mut xi) = (vec![0.0; size], vec![0.0; size]);
    xr[..n].copy_from_slice(x);
    let (mut ar, mut ai) = (vec![0.0; size], vec![0.0; size]);
    ar[..w].copy_from_slice(&x[..w]);
    fft.forward(&mut xr, &mut xi);
    fft.forward(&mut ar, &mut ai);
    let (mut cr, mut ci) = (vec![0.0; size], vec![0.0; size]);
    for k in 0..size {
        // conj(A) X
        cr[k] = ar[k] * xr[k] + ai[k] * xi[k];
        ci[k] = ar[k] * xi[k] - ai[k] * xr[k];
    }
    fft.inverse(&mut cr, &mut ci);
    let mut pre = vec![0.0; n + 1];
    for i in 0..n {
        pre[i + 1] = pre[i] + x[i] * x[i];
    }
    let e0 = pre[w];
    let mut r = vec![0.0; pmax + 2];
    for (lag, rl) in r.iter_mut().enumerate().skip(pmin - 1) {
        let e = e0 + (pre[lag + w] - pre[lag]);
        *rl = 2.0 * cr[lag] / (e + 1e-30);
    }
    let best = r[pmin..=pmax].iter().fold(0.0f64, |m, &v| m.max(v));
    for lag in pmin..=pmax {
        if r[lag] > 0.9 * best && r[lag] >= r[lag - 1] && r[lag] >= r[lag + 1] {
            let (a, bb, c) = (r[lag - 1], r[lag], r[lag + 1]);
            let den = a - 2.0 * bb + c;
            let off = if den.abs() > 1e-12 { 0.5 * (a - c) / den } else { 0.0 };
            return (fs / (lag as f64 + off), bb);
        }
    }
    (0.0, 0.0)
}

/// What the ear settled on for a note: the horn's trim (cents), the lips' tension, and the fingering it took
/// instead of the usual one.
pub type Tuned = (f64, f64, Option<(usize, i32)>);

/// The seed of the ear's trial runs.
const TRIAL_SEED: u64 = 17;

/// The note the ear tries for `n`: tuned at the strength it holds (a sforzando's fall, a swell's end), in the
/// sketch's steps of a twentieth.
fn trial_vel(b: &BrassParts, n: &Ln) -> f64 {
    let mut g = n.sfz.map_or(1.0, |s| s.1);
    for (_, z, _) in [n.swell, n.swell2].into_iter().flatten() {
        g *= z;
    }
    let v = if (g - 1.0).abs() > 0.05 {
        (n.vel + dmath::ln(g.max(1e-6)) / dmath::ln(b.pmax / b.pmin)).max(0.05)
    } else {
        n.vel
    };
    (v * 20.0).round() / 20.0
}

/// The player's ear on one note alone, held steady: the lips' tension that centres it on its natural note with the
/// least lipping and a quick start, then the horn trimmed until it sounds in tune; another fingering if the usual one
/// gives no clean note.
fn tune(b: &BrassParts, n: &Ln, fs: f64) -> Tuned {
    let fs_s = fs * b.os as f64;
    let forced = n.part.is_some() && n.val.is_some();
    let mut t = n.plain();
    t.vel = trial_vel(b, n);
    let slow = if hz(n.midi) < 110.0 { 1.6 } else { 1.0 };
    let (wa, wb) = (b.tun_a * slow, 0.44 * slow);
    t.dur = 0.5 * slow;
    let target = hz(n.midi);
    let mut cands: Vec<Option<(usize, i32)>> = vec![None];
    if !forced {
        cands.extend(b.fingerings(n).into_iter().skip(1).take(4).map(Some));
    }
    for cand in cands {
        if let Some((p, v)) = cand {
            t.part = Some(p);
            t.val = Some(v);
        }
        let try_one = |c: f64, al: f64| -> (f64, f64, f64, f64) {
            let len = (0.46 * slow * fs_s) as usize;
            let o = line(b, std::slice::from_ref(&t), &[(c, al)], len, TRIAL_SEED, fs_s);
            let a0 = (wa * fs_s) as usize;
            let e0 = ((wb * fs_s) as usize).min(o.len());
            let (f, r) = pitch_of(&o[a0..e0], fs_s, target);
            // how soon it speaks: the first 10 ms within 3 dB of the held level
            let w = (0.01 * fs_s) as usize;
            let lv = |s: usize| (o[s..s + w].iter().map(|v| v * v).sum::<f64>() / w as f64).sqrt();
            let heldl = lv(a0);
            let mut on = 0.5;
            let mut s0 = 0;
            while s0 + w < a0 {
                if lv(s0) > heldl * 0.7 {
                    on = s0 as f64 / fs_s;
                    break;
                }
                s0 += w / 2;
            }
            if f <= 0.0 {
                return (1e9, 0.0, on, 300.0);
            }
            // how soon it settles on its pitch: two windows just after the start, against the held pitch
            let ww = (5.5 / target).max(0.04);
            let mut e = 0.0;
            for k in 0..2 {
                let s0 = ((0.03 + f64::from(k) * ww * 0.75) * fs_s) as usize;
                let s1 = s0 + (ww * fs_s) as usize;
                let (fe, re) = if s1 < a0 {
                    pitch_of(&o[s0..s1], fs_s, f)
                } else {
                    (f, 1.0)
                };
                e += if fe > 0.0 && re > 0.6 {
                    (1200.0 * dmath::log2(fe / f)).abs().min(300.0)
                } else {
                    300.0
                };
            }
            (1200.0 * dmath::log2(target / f), r, on, e * 0.5)
        };
        // the lips' tension: a few tried, the one kept that sounds the right natural note cleanly with the least
        // lipping and a quick start; high in the series, where the natural notes crowd together and the one below
        // speaks first unless the lips are set well above the note, tighter ones are tried too
        let mut best = (1e9, b.lr, 0.0);
        let part = b.fingering(&t).0;
        let (amin, nk) = if part >= 5 && b.amin_hi < b.amin {
            (b.amin_hi, b.nalpha.max(9))
        } else {
            (b.amin, b.nalpha.max(2))
        };
        for k in 0..nk {
            let al = amin + (b.amax - amin) * k as f64 / (nk - 1) as f64;
            let (off, r, on, e) = try_one(0.0, al);
            if r < 0.93 || off.abs() > 160.0 {
                continue;
            }
            let cost = off.abs() + 50.0 * on + 15.0 * al.abs() + b.wsettle * e;
            if cost < best.0 {
                best = (cost, al, off);
            }
        }
        if best.0 >= 1e9 {
            continue;
        }
        let al = best.1;
        let mut c = best.2;
        for _ in 0..3 {
            let (off, r, _, _) = try_one(c, al);
            if r < 0.9 || off.abs() > 150.0 {
                break;
            }
            c += off;
            if off.abs() < 0.7 {
                break;
            }
        }
        return (c, al, cand);
    }
    (0.0, 1.0, None)
}

/// A slurred note's ear: played from the note before (`lead`, tied), it may settle a little off the pitch it has
/// when tongued; the horn is trimmed for it, and where it lands between natural notes or warbles the lips are set
/// afresh.
fn slur_trim(b: &BrassParts, lead: &Ln, lt: Tuned, n: &Ln, nt: Tuned, fs: f64) -> Tuned {
    let fs_s = fs * b.os as f64;
    if nt.1 >= 1.0 || n.glide.unwrap_or(0.0) > 0.06 {
        return nt;
    }
    let strip = |m: &Ln, fi: Option<(usize, i32)>| -> Ln {
        let mut t = m.clone();
        t.bend = None;
        t.bend2 = None;
        t.swell = None;
        t.swell2 = None;
        t.vib = None;
        t.shake = false;
        t.flutter = false;
        t.growl = false;
        t.sfz = None;
        t.half = 0.0;
        t.from = 0.0;
        if let Some((p, v)) = fi {
            t.part = Some(p);
            t.val = Some(v);
        }
        t
    };
    let target = hz(n.midi);
    let mut first = strip(lead, lt.2);
    first.slur = None;
    let mut last = strip(n, nt.2);
    last.tie = false;
    last.dur = last.dur.max(0.2);
    let (ti, di) = (last.t, last.dur);
    let seq = [first, last];
    let s0 = ((ti + 0.06) * fs_s) as usize;
    let s1 = ((ti + (di * 0.9).max(0.15)) * fs_s) as usize;
    // the pitch over the note's body, and how far it wanders there (short windows): a slur that lands between two
    // regimes warbles for a while before it settles
    let measure = |c: f64, al: f64| -> (f64, f64) {
        let len = ((ti + di + 0.02) * fs_s) as usize;
        let o = line(b, &seq, &[(lt.0, lt.1), (c, al)], len, TRIAL_SEED, fs_s);
        let e1 = s1.min(o.len());
        if s0 >= e1 {
            return (1e9, 0.0);
        }
        let (f, r) = pitch_of(&o[s0..e1], fs_s, target);
        if f <= 0.0 {
            return (1e9, r);
        }
        let w = ((3.0 / target).max(0.02) * fs_s) as usize;
        let (mut lo, mut hi) = (1e9f64, -1e9f64);
        let mut k = s0;
        while k + w <= e1 {
            let (fk, rk) = pitch_of(&o[k..k + w], fs_s, f);
            if fk > 0.0 && rk > 0.6 {
                let ck = 1200.0 * dmath::log2(fk / f);
                lo = lo.min(ck);
                hi = hi.max(ck);
            }
            k += w / 2;
        }
        let spread = if hi > lo { hi - lo } else { 0.0 };
        let off = 1200.0 * dmath::log2(target / f);
        (off, if spread > 30.0 { r.min(0.89) } else { r })
    };
    let (c0, al0) = (nt.0, nt.1);
    let (off0, r0) = measure(c0, al0);
    if off0.abs() < 4.0 && r0 >= 0.9 {
        return nt;
    }
    // far off (the lips have pulled the horn between its natural notes): the lips set afresh for the slur
    let mut al = al0;
    let mut best = (if r0 >= 0.9 { off0.abs() } else { 1e9 }, c0, off0);
    if !(off0.abs() < 20.0 && r0 >= 0.9) {
        let part = b.fingering(&seq[1]).0;
        let (amin, nk) = if part >= 5 && b.amin_hi < b.amin {
            (b.amin_hi, b.nalpha.max(9))
        } else {
            (b.amin, b.nalpha.max(2))
        };
        for k in 0..nk {
            let a2 = amin + (b.amax - amin) * k as f64 / (nk - 1) as f64;
            let (o2, r2) = measure(c0, a2);
            if r2 >= 0.9 && o2.abs() < best.0 {
                best = (o2.abs(), c0, o2);
                al = a2;
            }
        }
    }
    if best.0 > 150.0 {
        return nt;
    }
    let (mut c, mut off) = (best.1, best.2);
    for _ in 0..2 {
        if off.abs() < 3.0 {
            break;
        }
        c += off;
        let (o2, r2) = measure(c, al);
        if r2 < 0.9 || o2.abs() > 150.0 {
            break;
        }
        off = o2;
        if o2.abs() < best.0 {
            best = (o2.abs(), c, o2);
        }
    }
    if best.1 != c0 || al != al0 {
        (best.1, al, nt.2)
    } else {
        nt
    }
}

/// The sample rate the table was made at (at another, the lips' steps differ by a hair and every note is tried).
const TABLE_RATE: f64 = 48_000.0;

/// The notes tried so far, kept for every later ask: the instrument, the note as tried (bits), the sample rate.
type TriedKey = (&'static str, Vec<u64>);

fn tried() -> &'static Mutex<HashMap<TriedKey, Tuned>> {
    static TRIED: OnceLock<Mutex<HashMap<TriedKey, Tuned>>> = OnceLock::new();
    TRIED.get_or_init(|| Mutex::new(HashMap::new()))
}

/// What the ear settles on for `n` (tongued, alone): from the table where it has the note, else tried now and kept.
fn tuned(b: &BrassParts, n: &Ln, fs: f64) -> Tuned {
    let vel = trial_vel(b, n);
    if let Some(t) = b.table
        && fs == TABLE_RATE
        && n.part.is_none()
        && n.val.is_none()
        && n.midi.fract() == 0.0
        && n.midi >= f64::from(t.low)
        && n.midi <= f64::from(t.high)
    {
        let k = (vel * 20.0).round();
        if k >= f64::from(t.vels.0) && k <= f64::from(t.vels.1) {
            let nv = usize::from(t.vels.1 - t.vels.0) + 1;
            let i = (n.midi as usize - usize::from(t.low)) * nv + (k as usize - usize::from(t.vels.0));
            let (c, a, p, v) = t.rows[i];
            return (c, a, (p > 0).then_some((usize::from(p), i32::from(v))));
        }
    }
    let key = (
        b.name,
        vec![
            n.midi.to_bits(),
            vel.to_bits(),
            n.part.map_or(u64::MAX, |p| p as u64),
            n.val.map_or(u64::MAX, |v| v as u64),
            fs.to_bits(),
        ],
    );
    if let Some(t) = tried().lock().ok().and_then(|m| m.get(&key).copied()) {
        return t;
    }
    let t = tune(b, n, fs);
    if let Ok(mut m) = tried().lock() {
        m.insert(key, t);
    }
    t
}

/// What the ear settles on for a slurred note `n` played from `lead`.
fn slur_tuned(b: &BrassParts, lead: &Ln, n: &Ln, fs: f64) -> (Tuned, Tuned) {
    let lt = tuned(b, lead, fs);
    let nt = tuned(b, n, fs);
    let key = (
        b.name,
        vec![
            u64::MAX - 1,
            lead.midi.to_bits(),
            n.midi.to_bits(),
            trial_vel(b, n).to_bits(),
            n.dur.max(0.2).to_bits(),
            n.glide.unwrap_or(-1.0).to_bits(),
            n.part.map_or(u64::MAX, |p| p as u64),
            n.val.map_or(u64::MAX, |v| v as u64),
            fs.to_bits(),
        ],
    );
    if let Some(t) = tried().lock().ok().and_then(|m| m.get(&key).copied()) {
        return (lt, t);
    }
    let t = slur_trim(b, lead, lt, n, nt, fs);
    if let Ok(mut m) = tried().lock() {
        m.insert(key, t);
    }
    (lt, t)
}

/// No clean natural note at any lip tension (the ear's mark: the lips' tension past every one tried), or one only
/// with the horn trimmed more than three quarters of a semitone (the lips between natural notes: a note that lands
/// off its pitch, or never speaks, with another breath).
fn unplayable(t: Tuned) -> bool {
    t.1 >= 1.0 || t.0.abs() > 75.0
}

/// The note as the player can sound it: over the player's top, or where the ear finds no clean natural note at any
/// lip tension and fingering (the top of the mellophone, the euphonium and the sousaphone), taken down the octave, at
/// most twice.
fn reach(b: &BrassParts, mut n: Ln, fs: f64) -> Ln {
    for _ in 0..2 {
        if n.part.is_some() || n.midi < 12.0 || (n.midi <= b.top && !unplayable(tuned(b, &n, fs))) {
            break;
        }
        n.midi -= 12.0;
        if let Some(s) = n.slur.as_mut() {
            *s = (*s - 12.0).max(0.0);
        }
    }
    n
}

/// What the player's ear settles on for the note `play` at sample rate `fs`: its horn's trim (cents), its lips'
/// tension and the fingering it takes instead of the usual one (a slurred note's as played from the note before).
pub fn ear(p: &BrassParts, play: &Play, fs: f64) -> Tuned {
    match play.slur_from.filter(|s| s.is_finite()) {
        Some(s) => {
            let t0 = (SLUR_PRE * fs).round() / fs;
            let lead = reach(
                p,
                Ln {
                    tie: true,
                    ..Ln::of(p, &Play::plain(s.clamp(0.0, 127.0), t0, play.vel), 0.0)
                },
                fs,
            );
            slur_tuned(p, &lead, &reach(p, Ln::of(p, play, t0), fs), fs).1
        }
        None => tuned(p, &reach(p, Ln::of(p, play, 0.0), fs), fs),
    }
}

/// What the ear settles on for the plain note `midi` at strength `vel` (the table maker's).
pub fn tune_plain(b: &BrassParts, midi: f64, vel: f64, fs: f64) -> Tuned {
    let n = Ln::of(b, &Play::plain(midi, 0.5, vel), 0.0);
    tune(b, &n, fs)
}

/// A slurred note's lead-in: the note it slurs from, this long before it, s.
const SLUR_PRE: f64 = 0.2;
/// A slurred note comes in as the one tied to it goes out, over this long (equal power), s.
const JOIN: f64 = 0.012;

/// How long the horn rings past the breath's stop at `midi`, s: its resonances there dying 70 dB.
fn tail(b: &BrassParts, midi: f64) -> f64 {
    let f = hz(midi);
    (8.0 * b.qf(f) / (dmath::PI * f)).clamp(0.12, 1.5) + 0.05
}

/// How long the note `play` rings on `p` with no length asked for, seconds, at most `longest`.
pub fn rings_for(p: &BrassParts, play: &Play, longest: f64) -> f64 {
    let dur = held(play);
    let ring = if play.tie {
        dur + JOIN
    } else {
        let rel = if matches!(play.mute, Mute::Choked | Mute::Clipped) {
            CHOKED
        } else {
            p.rel
        };
        dur + rel + tail(p, play.midi.clamp(0.0, 127.0))
    };
    ring.clamp(0.05, longest.max(0.05))
}

/// The seed of repeat `variant` of a note at `m`.
pub(super) fn seed(variant: u64, m: f64) -> u64 {
    variant.wrapping_mul(7919).wrapping_add(17) ^ ((m * 1000.0) as u64)
}

/// A back-tongued note's lead-in: the tongued note of its own pitch and length (at most this long, s) that the
/// back of the tongue follows, the horn still ringing, [`K_GAP`] after it stops.
const K_PRE: f64 = 0.2;
/// The breath's break between the tongued note and the back-tongued one after it, s (the round's double tonguing).
const K_GAP: f64 = 0.02;

/// Whether `play` comes in on a lead-in (slurred, or back-tongued after its tongued note).
fn led(play: &Play) -> bool {
    play.slur_from.is_some_and(f64::is_finite) || play.technique == Technique::BackTongue
}

/// The horn's sound for the note `play` (and its lead-in if slurred or back-tongued), `len` samples at `fs` from
/// its start.
fn horn(p: &BrassParts, play: &Play, len: usize, variant: u64, fs: f64) -> Vec<f64> {
    let fs_s = fs * p.os as f64;
    let rings = rings_for(p, play, f64::INFINITY);
    let out_n = len.min((rings * fs) as usize + 1);
    let slur = play.slur_from.filter(|s| s.is_finite());
    let k_lead = slur.is_none() && play.technique == Technique::BackTongue;
    let k_dur = held(play).clamp(0.02, K_PRE);
    let pre = if slur.is_some() {
        (SLUR_PRE * fs).round() as usize
    } else if k_lead {
        ((k_dur + K_GAP) * fs).round() as usize
    } else {
        0
    };
    let t0 = pre as f64 / fs;
    let n = reach(p, Ln::of(p, play, t0), fs);
    let (notes, corr) = match slur {
        None if k_lead => {
            let lead = Ln {
                midi: n.midi,
                ..Ln::of(p, &Play::plain(play.midi.clamp(0.0, 127.0), k_dur, play.vel), 0.0)
            };
            let lt = tuned(p, &lead, fs);
            let nt = tuned(p, &n, fs);
            let mut lead = lead;
            let mut n = n;
            for (x, t) in [(&mut lead, lt), (&mut n, nt)] {
                if let Some((pp, v)) = t.2 {
                    x.part = Some(pp);
                    x.val = Some(v);
                }
            }
            (vec![lead, n], vec![(lt.0, lt.1), (nt.0, nt.1)])
        }
        Some(s) => {
            let lead = reach(
                p,
                Ln {
                    tie: true,
                    ..Ln::of(p, &Play::plain(s.clamp(0.0, 127.0), t0, play.vel), 0.0)
                },
                fs,
            );
            let (lt, nt) = slur_tuned(p, &lead, &n, fs);
            let mut lead = lead;
            let mut n = n;
            for (x, t) in [(&mut lead, lt), (&mut n, nt)] {
                if let Some((pp, v)) = t.2 {
                    x.part = Some(pp);
                    x.val = Some(v);
                }
            }
            (vec![lead, n], vec![(lt.0, lt.1), (nt.0, nt.1)])
        }
        None => {
            let t = tuned(p, &n, fs);
            let mut n = n;
            if let Some((pp, v)) = t.2 {
                n.part = Some(pp);
                n.val = Some(v);
            }
            (vec![n], vec![(t.0, t.1)])
        }
    };
    let taps = 32 * p.os + 2;
    let n_s = (pre + out_n) * p.os + taps;
    let mouth = line(p, &notes, &corr, n_s, seed(variant, play.midi), fs_s);
    let x = bell(p, &mouth, fs, pre + out_n);
    let mut y = x[pre..].to_vec();
    y.resize(len, 0.0);
    y
}

/// The pulse limiter: pulses held to `crest` times the typical wave peak around them (the median of the 20 ms peaks
/// over 180 ms, the most of five such), as a desk's fast limiter would.
fn tame(x: &[f64], crest: f64, fs: f64) -> Vec<f64> {
    let n = x.len();
    let bl = ((0.02 * fs) as usize).max(1);
    let nb = n / bl;
    if crest <= 0.0 || nb == 0 {
        return x.to_vec();
    }
    let a: Vec<f64> = x.iter().map(|v| v.abs()).collect();
    let top = a.iter().fold(0.0f64, |m, &v| m.max(v));
    let bp: Vec<f64> = (0..nb)
        .map(|i| a[i * bl..(i + 1) * bl].iter().fold(0.0f64, |m, &v| m.max(v)))
        .collect();
    let med = window(&bp, 9, |w| {
        let mut w = w.to_vec();
        w.sort_by(f64::total_cmp);
        w[w.len() / 2]
    });
    let rf = window(&med, 5, |w| w.iter().fold(f64::MIN, |m, &v| m.max(v)));
    let last = rf.last().copied().unwrap_or(1.0);
    let g: Vec<f64> = (0..n)
        .map(|i| {
            let r = rf.get(i / bl).copied().unwrap_or(last);
            let thr = crest * r + 1e-9 * (top + 1e-12);
            (thr / (a[i] + 1e-12)).min(1.0)
        })
        .collect();
    let g = window_min(&g, ((0.0015 * fs) as usize).max(1));
    let size = ((0.0007 * fs) as usize).max(1);
    let g = window_mean(&g, size);
    x.iter().zip(g).map(|(v, g)| v * g).collect()
}

/// `f` over each sample's window of `size` (centred as scipy's `ndimage` filters centre an even size: from
/// `size / 2` before to the rest after), the edges reflected (`d c b a | a b c d | d c b a`).
fn window(x: &[f64], size: usize, f: impl Fn(&[f64]) -> f64) -> Vec<f64> {
    let n = x.len() as isize;
    let at = |i: isize| -> f64 {
        let mut j = i;
        loop {
            if j < 0 {
                j = -j - 1;
            } else if j >= n {
                j = 2 * n - 1 - j;
            } else {
                return x[j as usize];
            }
        }
    };
    let h = (size / 2) as isize;
    let mut buf = vec![0.0; size];
    (0..n)
        .map(|i| {
            let s = i - h;
            if s >= 0 && s + size as isize <= n {
                // inside: the samples themselves (the same values in the same order)
                return f(&x[s as usize..s as usize + size]);
            }
            for (k, b) in buf.iter_mut().enumerate() {
                *b = at(s + k as isize);
            }
            f(&buf)
        })
        .collect()
}

/// [`window`] with each window's mean (its sum in order over `size`): the same numbers, four windows summed at once.
fn window_mean(x: &[f64], size: usize) -> Vec<f64> {
    let mean = |w: &[f64]| w.iter().sum::<f64>() / size as f64;
    let n = x.len();
    let h = size / 2;
    if size == 0 || n < size + 4 {
        return window(x, size, mean);
    }
    let ni = n as isize;
    let at = |i: isize| -> f64 {
        let mut j = i;
        loop {
            if j < 0 {
                j = -j - 1;
            } else if j >= ni {
                j = 2 * ni - 1 - j;
            } else {
                return x[j as usize];
            }
        }
    };
    let mut buf = vec![0.0; size];
    let mut edge = |i: usize| {
        for (k, b) in buf.iter_mut().enumerate() {
            *b = at(i as isize - h as isize + k as isize);
        }
        mean(&buf)
    };
    // the windows wholly inside the samples: from h to `last`
    let last = n + h - size;
    let zero: f64 = std::iter::empty::<f64>().sum();
    let mut y = Vec::with_capacity(n);
    for i in 0..h {
        y.push(edge(i));
    }
    let mut i = h;
    while i + 3 <= last {
        let w = &x[i - h..i - h + size + 3];
        let (mut a0, mut a1, mut a2, mut a3) = (zero, zero, zero, zero);
        for k in 0..size {
            a0 += w[k];
            a1 += w[k + 1];
            a2 += w[k + 2];
            a3 += w[k + 3];
        }
        let z = size as f64;
        y.extend_from_slice(&[a0 / z, a1 / z, a2 / z, a3 / z]);
        i += 4;
    }
    while i <= last {
        y.push(mean(&x[i - h..i - h + size]));
        i += 1;
    }
    for i in last + 1..n {
        y.push(edge(i));
    }
    y
}

/// [`window`] with the least of each window (the same values): kept as the windows slide, not sought afresh.
fn window_min(x: &[f64], size: usize) -> Vec<f64> {
    let n = x.len() as isize;
    if n == 0 {
        return Vec::new();
    }
    let at = |i: isize| -> f64 {
        let mut j = i;
        loop {
            if j < 0 {
                j = -j - 1;
            } else if j >= n {
                j = 2 * n - 1 - j;
            } else {
                return x[j as usize];
            }
        }
    };
    let h = (size / 2) as isize;
    // the samples every window reaches, edges reflected: window i is ext[i..i + size]
    let ext: Vec<f64> = (-h..n + size as isize - 1 - h).map(at).collect();
    let mut least: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let mut out = Vec::with_capacity(x.len());
    for (j, &v) in ext.iter().enumerate() {
        while least.back().is_some_and(|&b| ext[b] >= v) {
            least.pop_back();
        }
        least.push_back(j);
        if j + 1 >= size {
            let i = j + 1 - size;
            while least.front().is_some_and(|&f| f < i) {
                least.pop_front();
            }
            out.push(ext[least[0]]);
        }
    }
    out
}

/// A peaking filter as a section.
fn peq(f0: f64, db: f64, q: f64, fs: f64) -> [f64; 6] {
    let a = dmath::pow(10.0, db / 40.0);
    let w = 2.0 * dmath::PI * f0.min(0.45 * fs) / fs;
    let (c, s) = (dmath::cos(w), dmath::sin(w));
    let al = s / (2.0 * q);
    let (b0, b1, b2) = (1.0 + al * a, -2.0 * c, 1.0 - al * a);
    let (a0, a1, a2) = (1.0 + al / a, -2.0 * c, 1.0 - al / a);
    [b0 / a0, b1 / a0, b2 / a0, 1.0, a1 / a0, a2 / a0]
}

/// Mute `k`'s shape at `fs` (`winds.py`'s): a straight mute holds the lows back with a nasal ring near 1.8 kHz; a
/// cup closes the bell round and muffled; a harmon rings nasally (stem in) or sizzles thin and close (stem out); a
/// plunger closed is dark, open nearly free; a practice mute stuffy; the hand stopping the bell, or the stopping
/// mute, thin and buzzing.
fn mute_sos(k: usize, fs: f64) -> Sos {
    let cap = |f: f64| f.min(0.45 * fs);
    let hp = |f: f64, o: usize| butter_sos(FilterKind::Hp, &[cap(f)], o, fs);
    let lp = |f: f64, o: usize| butter_sos(FilterKind::Lp, &[cap(f)], o, fs);
    let mut s: Sos = Vec::new();
    match k {
        0 => {
            s.extend(hp(700.0, 2));
            s.push(peq(1800.0, 7.0, 2.0, fs));
            s.push(peq(4500.0, 3.0, 1.5, fs));
        }
        1 => {
            s.extend(hp(250.0, 1));
            s.push(peq(900.0, 6.0, 1.8, fs));
            s.extend(lp(1800.0, 2));
        }
        2 => {
            s.extend(hp(1200.0, 2));
            s.push(peq(2500.0, 9.0, 1.5, fs));
            s.push(peq(5000.0, 4.0, 2.0, fs));
        }
        3 => {
            s.extend(hp(900.0, 2));
            s.push(peq(1500.0, 12.0, 4.0, fs));
            s.push(peq(3300.0, 6.0, 3.0, fs));
            s.extend(lp(6000.0, 2));
        }
        4 => {
            s.extend(lp(700.0, 2));
            s.push(peq(450.0, 4.0, 1.5, fs));
        }
        5 => s.push(peq(1200.0, 2.0, 1.0, fs)),
        6 => {
            s.extend(hp(500.0, 2));
            s.extend(lp(2000.0, 2));
            s.push(peq(1100.0, 5.0, 1.5, fs));
        }
        7 => {
            s.extend(hp(1100.0, 2));
            s.push(peq(3000.0, 10.0, 2.0, fs));
            s.push(peq(5500.0, 5.0, 2.0, fs));
        }
        _ => {
            s.extend(hp(1000.0, 2));
            s.push(peq(2800.0, 8.0, 1.8, fs));
            s.push(peq(5000.0, 4.0, 2.0, fs));
        }
    }
    s
}

/// `x` through mute `k`, at its level.
fn muted(p: &BrassParts, x: &[f64], k: usize, fs: f64) -> Vec<f64> {
    let g = p.mutes[k];
    dsp::sosfilt(&mute_sos(k, fs), x).into_iter().map(|v| v * g).collect()
}

/// The plunger over the note: closed to open as `expression` asks, the hand's opening going `from` to `to` over
/// `over` seconds and back, again and again (a wah-wah; one opening over a long `over`), held from the key's release
/// for 0.2 s, then closed.
fn plunger(p: &BrassParts, x: &[f64], play: &Play, fs: f64) -> Vec<f64> {
    let closed = muted(p, x, 4, fs);
    let Some((a, z, over)) = three(play.expression) else {
        return closed;
    };
    let open = muted(p, x, 5, fs);
    let dur = held(play).max(1e-3);
    closed
        .iter()
        .zip(&open)
        .enumerate()
        .map(|(i, (c, o))| {
            let t = i as f64 / fs;
            let m = if t <= dur + 0.2 {
                let u = t.min(dur);
                (a + (z - a) * (0.5 - 0.5 * dmath::cos(dmath::PI * u / over))).clamp(0.0, 1.0)
            } else {
                0.0
            };
            c * (1.0 - m) + o * m
        })
        .collect()
}

/// A horn player's right hand in the bell: darker as it closes (0 open, 1 the usual, 2 covered).
fn hand(x: &[f64], amount: f64, fs: f64) -> Vec<f64> {
    if amount <= 0.0 {
        return x.to_vec();
    }
    let fc = (4200.0 / (1.0 + 1.4 * amount)).min(0.45 * fs);
    let y = dsp::sosfilt(&butter_sos(FilterKind::Lp, &[fc], 1, fs), x);
    let a = amount.min(1.0);
    x.iter()
        .zip(y)
        .map(|(v, w)| v * (1.0 - a) * 0.3 + w * (1.0 - 0.3 * (1.0 - a)))
        .collect()
}

/// The soft limiter: only what would pass 0.75 of the page's full scale is rounded off, softly, never past 0.97.
pub(super) fn knee(x: &mut [f64], full: f64) {
    let (at, top) = (0.75 * full, 0.97 * full);
    let r = top - at;
    for v in x.iter_mut() {
        let a = v.abs();
        if a > at {
            *v = v.signum() * (at + r * dmath::tanh((a - at) / r));
        }
    }
}

/// The note `play` on the brass instrument `p`: `len` samples at sample rate `fs`, as repeat `variant` (each repeat
/// with its own breath).
pub fn note(p: &BrassParts, play: &Play, len: usize, variant: u64, fs: f64) -> Vec<f64> {
    if len == 0 {
        return Vec::new();
    }
    let x = horn(p, play, len, variant, fs);
    let x = tame(&x, p.crest, fs);
    let x = match play.mute {
        Mute::Muted | Mute::Straight => muted(p, &x, 0, fs),
        Mute::Cup => muted(p, &x, 1, fs),
        Mute::Harmon => muted(p, &x, 2, fs),
        Mute::HarmonStem => muted(p, &x, 3, fs),
        Mute::Plunger => plunger(p, &x, play, fs),
        Mute::Practice => muted(p, &x, 6, fs),
        Mute::Stopped => muted(p, &x, 7, fs),
        Mute::StopMute => muted(p, &x, 8, fs),
        _ => x,
    };
    let mut y = hand(&x, p.hand, fs);
    knee(&mut y, p.full);
    y.iter_mut().for_each(|v| *v *= p.gain);
    joins_led(&mut y, play, led(play), fs);
    y
}

/// A slurred note comes in over [`JOIN`] and a tied one goes out over it, at its held length (equal power); any
/// note's last 20 ms faded, so a note cut short ends without a click.
pub(super) fn joins(y: &mut [f64], play: &Play, fs: f64) {
    joins_led(y, play, play.slur_from.is_some_and(f64::is_finite), fs);
}

/// [`joins`], the note coming in over [`JOIN`] when `led` (on a lead-in).
pub(super) fn joins_led(y: &mut [f64], play: &Play, led: bool, fs: f64) {
    let j = ((JOIN * fs) as usize).max(1);
    if led {
        for (i, v) in y.iter_mut().take(j).enumerate() {
            *v *= dmath::sin(0.5 * dmath::PI * i as f64 / j as f64);
        }
    }
    if play.tie {
        let s = (held(play) * fs) as usize;
        for (i, v) in y.iter_mut().enumerate().skip(s) {
            let u = (i - s) as f64 / j as f64;
            *v *= if u < 1.0 { dmath::cos(0.5 * dmath::PI * u) } else { 0.0 };
        }
    }
    let len = y.len();
    let n = ((0.02 * fs) as usize).min(len / 2);
    for i in 0..n {
        y[len - 1 - i] *= i as f64 / n as f64;
    }
}

/// A line of notes in one breath, as the sketch plays a riff (for measuring the port against it): each note's start
/// and play, every note tried as the ear tries it unless `tunings` are given, the horn's sound out (before the
/// limiters and the mutes), `len` samples at `fs`.
pub fn phrase(
    p: &BrassParts,
    notes: &[(f64, Play)],
    tunings: Option<&[Tuned]>,
    seed: u64,
    fs: f64,
    len: usize,
) -> Vec<f64> {
    let mut ls: Vec<Ln> = notes.iter().map(|(t, play)| Ln::of(p, play, *t)).collect();
    let tun: Vec<Tuned> = match tunings {
        Some(t) => t.to_vec(),
        None => ls.iter().map(|n| tuned(p, n, fs)).collect(),
    };
    for (n, t) in ls.iter_mut().zip(&tun) {
        if let Some((pp, v)) = t.2 {
            n.part = Some(pp);
            n.val = Some(v);
        }
    }
    let corr: Vec<(f64, f64)> = tun.iter().map(|t| (t.0, t.1)).collect();
    let n_s = len * p.os + 32 * p.os + 2;
    let mouth = line(p, &ls, &corr, n_s, seed, fs * p.os as f64);
    bell(p, &mouth, fs, len)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::voice::engine2::play::{Bend, Swell, Vibrato};

    pub(in crate::voice::engine2) const FS: f64 = 48_000.0;

    /// A note with no vibrato (for reading its pitch).
    pub(in crate::voice::engine2) fn still(m: f64, d: f64, v: f64) -> Play {
        Play {
            vibrato: Some(Vibrato {
                cents: 0.0,
                hz: 5.0,
                wait: 0.0,
            }),
            ..Play::plain(m, d, v)
        }
    }

    fn render(p: &BrassParts, play: &Play, variant: u64, fs: f64) -> Vec<f64> {
        let n = (rings_for(p, play, 12.0) * fs) as usize;
        note(p, play, n, variant, fs)
    }

    fn pick(name: &str) -> BrassParts {
        *BrassParts::PICKS.iter().find(|p| p.name == name).expect("a pick")
    }

    pub(in crate::voice::engine2) fn seg(x: &[f64], a: f64, b: f64, fs: f64) -> &[f64] {
        let j = ((b * fs) as usize).min(x.len());
        &x[((a * fs) as usize).min(j)..j]
    }

    /// The level over `a..b` seconds, dB.
    pub(in crate::voice::engine2) fn db(x: &[f64], a: f64, b: f64, fs: f64) -> f64 {
        let s = seg(x, a, b, fs);
        let e = s.iter().map(|v| v * v).sum::<f64>() / s.len().max(1) as f64;
        10.0 * dmath::log10(e.max(1e-30))
    }

    /// The Hann-windowed power of the component at `f` over `a..b` seconds, dB.
    pub(in crate::voice::engine2) fn at_hz(x: &[f64], f: f64, a: f64, b: f64, fs: f64) -> f64 {
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

    /// How far a note near `f0` sounds from it over `a..b` seconds, cents (on the 3rd harmonic under 150 Hz, the
    /// 2nd under 300 Hz), within 6% either way.
    pub(in crate::voice::engine2) fn pitch(x: &[f64], f0: f64, a: f64, b: f64, fs: f64) -> f64 {
        let h = if f0 < 150.0 {
            3.0
        } else if f0 < 300.0 {
            2.0
        } else {
            1.0
        };
        1200.0 * dmath::log2(peak_in(x, h * f0 * 0.94, h * f0 * 1.06, a, b, fs) / (h * f0))
    }

    /// The mean level of harmonics `lo..=hi` of a note at `f0` against its fundamental over `a..b` seconds, dB
    /// (each the strongest within half a percent).
    pub(in crate::voice::engine2) fn upper(x: &[f64], f0: f64, lo: usize, hi: usize, a: f64, b: f64, fs: f64) -> f64 {
        let h = |k: usize| {
            (0..11)
                .map(|j| at_hz(x, k as f64 * f0 * (0.995 + 0.001 * j as f64), a, b, fs))
                .fold(f64::MIN, f64::max)
        };
        let base = h(1);
        (lo..=hi).map(|k| h(k) - base).sum::<f64>() / (hi - lo + 1) as f64
    }

    /// A tied note `a` and the note slurred from it, `b`, placed as a voice places them: (the sum, the join's sample).
    pub(in crate::voice::engine2) fn joined(a: &[f64], b: &[f64], at: f64, fs: f64) -> (Vec<f64>, usize) {
        let s = (at * fs) as usize;
        let mut y = vec![0.0; a.len().max(s + b.len())];
        for (o, v) in y.iter_mut().zip(a) {
            *o += v;
        }
        for (o, v) in y.iter_mut().skip(s).zip(b) {
            *o += v;
        }
        (y, s)
    }

    /// The join at sample `s` of `y`: (its largest step against the held parts' 99.9th percentile step, the lowest
    /// and the highest of its 20 ms levels from 40 ms before to 40 ms after against the held levels', dB).
    pub(in crate::voice::engine2) fn join_marks(y: &[f64], s: usize, fs: f64) -> (f64, f64, f64) {
        let steps = |a: f64, b: f64| -> Vec<f64> {
            let mut v: Vec<f64> = seg(y, a, b, fs).windows(2).map(|w| (w[1] - w[0]).abs()).collect();
            v.sort_by(f64::total_cmp);
            v
        };
        let t = s as f64 / fs;
        let p999 = |v: Vec<f64>| v[((v.len() as f64 * 0.999) as usize).min(v.len() - 1)];
        let held = p999(steps(t - 0.2, t - 0.05)).max(p999(steps(t + 0.1, t + 0.3)));
        let at = steps(t - 0.01, t + 0.01).last().copied().unwrap_or(0.0);
        let (before, after) = (db(y, t - 0.15, t - 0.04, fs), db(y, t + 0.1, t + 0.2, fs));
        let lv: Vec<f64> = (-2..2)
            .map(|k| db(y, t + 0.02 * f64::from(k), t + 0.02 * f64::from(k + 1), fs))
            .collect();
        let lo = lv.iter().copied().fold(f64::MAX, f64::min) - before.min(after);
        let hi = lv.iter().copied().fold(f64::MIN, f64::max) - before.max(after);
        (at / held, lo, hi)
    }

    /// Every pick, plain at the middle strength, sits on its note low, middle and high in its range (vibrato off),
    /// held: the table's trial runs and the ear's trims (measured within 0.6 cents).
    #[test]
    fn every_pick_plays_in_tune_across_its_range() {
        for (name, ms) in [
            ("Tp1", [60.0, 70.0, 79.0]),
            ("Co1", [58.0, 67.0, 77.0]),
            ("Fh1", [55.0, 65.0, 74.0]),
            ("Hn1", [50.0, 60.0, 69.0]),
            ("Me1", [55.0, 65.0, 77.0]),
            ("Tb1", [41.0, 53.0, 65.0]),
            ("Eu1", [43.0, 53.0, 67.0]),
            ("Tu1", [31.0, 40.0, 50.0]),
            ("Su1", [31.0, 40.0, 50.0]),
        ] {
            let p = pick(name);
            for m in ms {
                let x = render(&p, &still(m, 1.0, 0.7), 0, FS);
                let c = pitch(&x, hz(m), 0.4, 0.9, FS);
                assert!(c.abs() < 3.0, "{name} {m}: {c:.2} cents");
            }
        }
    }

    /// A note between the table's strengths takes its nearest step's (the sketch's own twentieths); one off its grid
    /// (a quarter tone) is tried at run time; both in tune.
    #[test]
    fn a_note_the_table_lacks_is_tried_and_in_tune() {
        let p = pick("Tp1");
        for play in [still(67.0, 0.8, 0.73), still(67.5, 0.8, 0.7)] {
            let x = render(&p, &play, 0, FS);
            let c = pitch(&x, hz(play.midi), 0.35, 0.75, FS);
            assert!(c.abs() < 3.0, "{} at {}: {c:.2} cents", play.midi, play.vel);
        }
    }

    /// At 44.1 kHz the horn's own rate and every filter follow: the notes stay in tune (measured within 0.2 cents).
    #[test]
    fn another_sample_rate_stays_in_tune() {
        for (name, m) in [("Tp1", 67.0), ("Tb1", 53.0), ("Hn1", 62.0), ("Tu1", 40.0)] {
            let x = render(&pick(name), &still(m, 1.0, 0.7), 0, 44_100.0);
            let c = pitch(&x, hz(m), 0.4, 0.9, 44_100.0);
            assert!(c.abs() < 3.0, "{name} {m} at 44.1 kHz: {c:.2} cents");
        }
    }

    /// Played harder, the upper harmonics (6th to 12th) climb against the fundamental: the blare (measured 12.5 dB
    /// on the trumpet, 7.6 on the trombone between strengths 0.35 and 0.95).
    #[test]
    fn louder_is_brighter() {
        for (name, m) in [("Tp1", 67.0), ("Tb1", 53.0)] {
            let p = pick(name);
            let soft = render(&p, &still(m, 0.6, 0.35), 0, FS);
            let loud = render(&p, &still(m, 0.6, 0.95), 0, FS);
            let (s, l) = (
                upper(&soft, hz(m), 6, 12, 0.2, 0.5, FS),
                upper(&loud, hz(m), 6, 12, 0.2, 0.5, FS),
            );
            assert!(l - s > 4.0, "{name}: soft {s:.1} dB, loud {l:.1} dB");
            assert!(
                db(&loud, 0.2, 0.5, FS) > db(&soft, 0.2, 0.5, FS) + 6.0,
                "{name}: louder"
            );
        }
    }

    /// Every way of playing a note is finite, within full scale, sounds, and changes the note.
    #[test]
    fn every_technique_sounds_and_changes_the_note() {
        let bend = |c: f64, at: f64, over: f64| Some(Bend { cents: c, at, over });
        let swell = |from: f64, to: f64, over: f64| Some(Swell { from, to, over });
        let base = Play::plain(67.0, 0.9, 0.7);
        let mut asks: Vec<(&str, Play)> = vec![("accent", Play { accent: 1.0, ..base })];
        for (what, t) in [
            ("back tongue", Technique::BackTongue),
            ("soft tongue", Technique::SoftTongue),
            ("breath", Technique::Breath),
            ("shake", Technique::Shake),
            ("flutter", Technique::Flutter),
            ("growl", Technique::Growl),
            ("half valve", Technique::HalfValve),
            ("4th natural note", Technique::Harmonic(4)),
        ] {
            asks.push((what, Play { technique: t, ..base }));
        }
        for (what, m) in [
            ("straight", Mute::Straight),
            ("cup", Mute::Cup),
            ("harmon", Mute::Harmon),
            ("harmon with its stem", Mute::HarmonStem),
            ("plunger", Mute::Plunger),
            ("practice", Mute::Practice),
            ("stopped", Mute::Stopped),
            ("stop mute", Mute::StopMute),
            ("muted", Mute::Muted),
            ("clipped", Mute::Clipped),
        ] {
            asks.push((what, Play { mute: m, ..base }));
        }
        asks.extend([
            (
                "valves",
                Play {
                    string: Some(2),
                    ..base
                },
            ),
            (
                "fall",
                Play {
                    bend: bend(-700.0, 0.3, 0.3),
                    ..base
                },
            ),
            (
                "scoop",
                Play {
                    scoop: bend(-200.0, 0.0, 0.08),
                    ..base
                },
            ),
            (
                "sforzando",
                Play {
                    swell: swell(1.5, 0.45, 0.12),
                    ..base
                },
            ),
            (
                "crescendo",
                Play {
                    swell: swell(0.4, 1.0, 0.5),
                    ..base
                },
            ),
            (
                "breath swell",
                Play {
                    expression: swell(1.0, 0.5, 0.4),
                    ..base
                },
            ),
            (
                "plunger wah",
                Play {
                    mute: Mute::Plunger,
                    expression: swell(0.0, 1.0, 0.3),
                    ..base
                },
            ),
        ]);
        let p = pick("Tp1");
        let plain = render(&p, &base, 0, FS);
        let top = 0.97 * p.full * p.gain + 1e-9;
        for (what, play) in asks {
            let x = render(&p, &play, 0, FS);
            assert!(x.iter().all(|v| v.is_finite()), "{what}");
            assert!(x.iter().all(|v| v.abs() <= top), "{what}: over full scale");
            assert!(db(&x, 0.0, 0.3, FS) > db(&plain, 0.0, 0.3, FS) - 40.0, "{what}: silent");
            let n = x.len().min(plain.len());
            let d: f64 = x[..n]
                .iter()
                .zip(&plain[..n])
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f64>();
            let e: f64 = plain[..n].iter().map(|v| v * v).sum();
            assert!(d / e > 0.01, "{what}: the same as plain");
        }
        // a choked note stops short: the breath cut in 5 ms, its ring after it under the plain note's (measured
        // 4.3 dB; the horn rings on either way)
        let choked = render(
            &p,
            &Play {
                mute: Mute::Choked,
                ..base
            },
            0,
            FS,
        );
        assert!(db(&choked, 0.93, 0.98, FS) < db(&plain, 0.93, 0.98, FS) - 2.5, "choked");
        // the double horn's other side, the trombone's other position: other fingerings, the same note
        for (name, m, s) in [("Hn1", 62.0, 100u8), ("Tb1", 53.0, 5)] {
            let q = pick(name);
            let a = render(&q, &still(m, 0.9, 0.7), 0, FS);
            let b = render(
                &q,
                &Play {
                    string: Some(s),
                    ..still(m, 0.9, 0.7)
                },
                0,
                FS,
            );
            assert_ne!(a, b, "{name}: another fingering");
            assert!(
                pitch(&b, hz(m), 0.3, 0.8, FS).abs() < 10.0,
                "{name}: another fingering in tune"
            );
        }
    }

    /// Each mute is quieter than the open horn (as the round set it: 3 to 15 dB on the trumpet) and changes its tone.
    #[test]
    fn mutes_are_quieter_and_shaped() {
        let p = pick("Tp1");
        let open = render(&p, &still(67.0, 0.9, 0.7), 0, FS);
        let shape = |x: &[f64]| upper(x, hz(67.0), 2, 8, 0.2, 0.7, FS);
        for m in [
            Mute::Straight,
            Mute::Cup,
            Mute::Harmon,
            Mute::HarmonStem,
            Mute::Plunger,
            Mute::Practice,
            Mute::Stopped,
            Mute::StopMute,
        ] {
            let x = render(
                &p,
                &Play {
                    mute: m,
                    ..still(67.0, 0.9, 0.7)
                },
                0,
                FS,
            );
            assert!(db(&x, 0.1, 0.8, FS) < db(&open, 0.1, 0.8, FS) - 2.0, "{m:?}: quieter");
            assert!((shape(&x) - shape(&open)).abs() > 1.0, "{m:?}: shaped");
        }
        // the horn's hand darkens it against the same horn without
        let hn = pick("Hn1");
        let bare = BrassParts { hand: 0.0, ..hn };
        let with = render(&hn, &still(62.0, 0.8, 0.7), 0, FS);
        let without = render(&bare, &still(62.0, 0.8, 0.7), 0, FS);
        assert!(upper(&with, hz(62.0), 8, 12, 0.2, 0.7, FS) < upper(&without, hz(62.0), 8, 12, 0.2, 0.7, FS) - 1.0);
    }

    /// A tied note and the note slurred from it join without a click (no step over the held notes' own), a gap (no
    /// 20 ms more than 6 dB under the quieter) or a doubled start (none 3 dB over the louder): measured at most 0.97,
    /// 3.1 dB under and 0.5 dB over.
    #[test]
    fn slurred_and_tied_notes_join_without_a_click() {
        for (name, a, b) in [
            ("Tp1", 67.0, 69.0),
            ("Tb1", 53.0, 58.0),
            ("Tu1", 36.0, 40.0),
            ("Tp1", 67.0, 67.0),
        ] {
            let p = pick(name);
            let x = render(
                &p,
                &Play {
                    tie: true,
                    ..Play::plain(a, 0.4, 0.7)
                },
                0,
                FS,
            );
            let y = render(
                &p,
                &Play {
                    slur_from: Some(a),
                    ..Play::plain(b, 0.5, 0.7)
                },
                0,
                FS,
            );
            let (sum, s) = joined(&x, &y, 0.4, FS);
            let (step, lo, hi) = join_marks(&sum, s, FS);
            assert!(step < 1.5, "{name} {a} to {b}: a click ({step:.2})");
            assert!(lo > -6.0, "{name} {a} to {b}: a gap ({lo:.1} dB)");
            assert!(hi < 3.0, "{name} {a} to {b}: a doubled start ({hi:.1} dB)");
        }
    }

    /// The same note twice is the same; another repeat has its own breath.
    #[test]
    fn the_same_note_is_the_same_and_repeats_differ() {
        for name in ["Tp1", "Tb1", "Hn1"] {
            let p = pick(name);
            let play = Play::plain(57.0, 0.5, 0.7);
            assert_eq!(render(&p, &play, 0, FS), render(&p, &play, 0, FS), "{name}");
            assert_ne!(render(&p, &play, 0, FS), render(&p, &play, 1, FS), "{name}");
        }
    }

    /// Notes the round never tried: full strength with a full accent at the top, pedal notes, a fall of two octaves,
    /// the top of every range, nearly nothing: finite, within full scale, never a long silence; the top notes in tune
    /// (or an octave down where the player cannot sound them).
    #[test]
    fn notes_the_round_never_tried_stay_sound() {
        let cases: Vec<(&str, Play)> = vec![
            (
                "Tp1",
                Play {
                    accent: 1.0,
                    ..Play::plain(86.0, 0.6, 1.2)
                },
            ),
            ("Tp1", Play::plain(55.0, 0.6, 0.9)),
            (
                "Tb1",
                Play {
                    accent: 1.0,
                    ..Play::plain(34.0, 0.8, 1.0)
                },
            ),
            ("Tu1", Play::plain(24.0, 0.8, 1.0)),
            (
                "Su1",
                Play {
                    accent: 1.0,
                    ..Play::plain(60.0, 0.6, 1.0)
                },
            ),
            ("Hn1", Play::plain(77.0, 0.6, 1.0)),
            (
                "Tp1",
                Play {
                    bend: Some(Bend {
                        cents: -2400.0,
                        at: 0.2,
                        over: 0.6,
                    }),
                    ..Play::plain(82.0, 1.0, 0.9)
                },
            ),
            ("Tp1", Play::plain(67.0, 0.6, 0.025)),
        ];
        for (name, play) in cases {
            let p = pick(name);
            let x = render(&p, &play, 0, FS);
            assert!(x.iter().all(|v| v.is_finite()), "{name} {}", play.midi);
            assert!(
                x.iter().all(|v| v.abs() <= 0.97 * p.full * p.gain + 1e-9),
                "{name} {}",
                play.midi
            );
            let lv: Vec<f64> = (0..(play.dur / 0.05) as usize)
                .map(|k| db(&x, 0.05 * k as f64, 0.05 * (k + 1) as f64, FS))
                .collect();
            let most = lv.iter().copied().fold(f64::MIN, f64::max);
            assert!(lv.iter().all(|&v| v > most - 40.0), "{name} {}: a silence", play.midi);
        }
        for (name, ms) in [
            ("Tp1", [80.0, 81.0, 82.0]),
            ("Me1", [82.0, 83.0, 84.0]),
            ("Eu1", [75.0, 76.0, 77.0]),
            ("Su1", [56.0, 57.0, 58.0]),
            ("Tu1", [53.0, 54.0, 55.0]),
        ] {
            let p = pick(name);
            for m in ms {
                for v in [0.4, 0.7, 0.95] {
                    for variant in 0..2 {
                        let x = render(&p, &still(m, 0.5, v), variant, FS);
                        assert!(db(&x, 0.05, 0.5, FS) > -45.0, "{name} {m} at {v}: silent");
                        let c = pitch(&x, hz(m), 0.15, 0.48, FS)
                            .abs()
                            .min(pitch(&x, hz(m - 12.0), 0.15, 0.48, FS).abs());
                        assert!(c < 15.0, "{name} {m} at {v}: {c:.1} cents off it and the octave below");
                    }
                }
            }
        }
    }
}
