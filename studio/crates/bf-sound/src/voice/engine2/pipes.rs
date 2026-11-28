//! The pipe engine: an organ built stop by stop, pipe by pipe (`docs/studio/voice-engine-2-instruments.md`,
//! "Organ"; Aedan's pick O8, the sketch `organ2.py`).
//!
//! - **Pipes.** Each pipe has the steady tone of its kind (an open principal, a stopped flute with its odd
//!   overtones, a chimney flute, a narrow string, a trumpet or oboe reed), a hair out of tune with its neighbours
//!   (the same pipe the same every time), and speaks in its own time: a big pipe slower, a flue's octave flicking in
//!   first with a puff of air (the chiff) and its pitch settling from a hair flat, a reed at once and a hair sharp.
//!   It breathes a little round its pitch while it sounds, and when the key comes up it stops within a few
//!   hundredths of a second, dropping a hair in pitch.
//! - **Stops.** A stop is a row of pipes, one per key, at its own pitch (16' an octave under, 4' over); a mixture
//!   several rows that break back an octave as they climb. Notes under G2 are played on the pedal board's stops.
//! - **Wind.** Every pipe speaks on one wind: it wavers slowly, the same for every note that starts together, and a
//!   big chord sags it a hair as it starts.
//!
//! Every technique of the organ (Aedan's rule, 2026-11-26): the registration ([`Play::stops`]: from one soft flute
//! to the full organ with its reeds), the octave couplers, the crescendo ([`Play::swell`]: the stops coming in in a
//! builder's order), the swell box ([`Play::expression`]: quieter and darker shut), the tremulant
//! ([`Play::vibrato`]: the wind shaken), a quicker key ([`Play::accent`]: more chiff), and how long each key is held.
//! The engine works in blocks of a few dozen samples at any sample rate, so it could play live.

use bf_core::dmath;
use bf_core::rng::Generator;

use super::eq::{self, Band};
use super::organ::waver;
use super::play::{Mute, Play, Stops};
use crate::dsp::{self, FilterKind, butter_sos};

/// The pipe engine's settings (O8: [`PipeParts::O8`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PipeParts {
    /// How far the wind's waver moves the pitch, cents either way, and the level, dB either way.
    pub wind_cents: f64,
    pub wind_db: f64,
    /// About how fast it wavers, Hz.
    pub wind_hz: f64,
    /// How far a big chord sags the pitch at most, cents.
    pub sag_cents: f64,
    /// The lowest manual key: under it, the pedal board.
    pub pedal_top: i32,
    /// The organ's studio polish, on each note.
    pub polish: &'static [Band],
}

impl PipeParts {
    /// Aedan's pick, round 2 of the built voices: the grand chorus, polished.
    pub const O8: PipeParts = PipeParts {
        wind_cents: 0.35,
        wind_db: 0.21,
        wind_hz: 1.5,
        sag_cents: 6.0,
        pedal_top: 43,
        polish: &POLISH,
    };
}

/// O8's polish: a little warmth, body, presence and air.
pub const POLISH: [Band; 4] = [
    Band::Low {
        hz: 110.0,
        db: 2.0,
        q: 0.7,
    },
    Band::Peak {
        hz: 250.0,
        db: 1.5,
        q: 0.8,
    },
    Band::Peak {
        hz: 2800.0,
        db: 1.0,
        q: 1.0,
    },
    Band::High {
        hz: 9000.0,
        db: 1.5,
        q: 0.7,
    },
];

/// The kinds of pipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// An open principal: the organ's own tone.
    Principal,
    /// A stopped flute: its odd overtones only.
    Stopped,
    /// A chimney flute: a round tone, its odd overtones a little up.
    Chimney,
    /// A narrow string: many overtones, slow to speak.
    String,
    /// A trumpet reed (and the pedal's trombone).
    Trumpet,
    Posaune,
    /// An oboe reed: a narrower ring.
    Oboe,
}

impl Kind {
    fn reed(self) -> bool {
        matches!(self, Kind::Trumpet | Kind::Posaune | Kind::Oboe)
    }
}

/// A stop: its rows' pitches against the key's 8' pitch, its kind, its level, a detune of its own (cents), and
/// where it comes in when the stops are drawn one after another (lowest first).
struct Stop {
    ranks: &'static [f64],
    kind: Kind,
    level: f64,
    detune: f64,
    build: u8,
}

/// The manual's stops, in [`Stops::MANUAL`]'s order.
const MANUAL: [Stop; 14] = [
    stop(&[0.5], Kind::Principal, -10.0, 9),
    stop(&[0.5], Kind::Stopped, -9.0, 8),
    stop(&[1.0], Kind::Principal, 0.0, 3),
    stop(&[1.0], Kind::Stopped, -4.0, 0),
    stop(&[1.0], Kind::String, -7.0, 1),
    Stop {
        detune: 6.0,
        ..stop(&[1.0], Kind::String, -7.0, 2)
    },
    stop(&[2.0], Kind::Principal, -3.5, 4),
    stop(&[2.0], Kind::Chimney, -7.0, 1),
    stop(&[3.0], Kind::Principal, -10.0, 6),
    stop(&[4.0], Kind::Principal, -6.5, 5),
    stop(&[6.0, 8.0, 12.0, 16.0], Kind::Principal, -12.5, 7),
    stop(&[16.0, 24.0, 32.0], Kind::Principal, -17.0, 11),
    stop(&[1.0], Kind::Trumpet, -3.0, 10),
    stop(&[1.0], Kind::Oboe, -6.0, 10),
];

/// The pedal board's stops, in [`Stops::PEDAL`]'s order.
const PEDAL: [Stop; 4] = [
    stop(&[0.5], Kind::Stopped, -1.0, 0),
    stop(&[1.0], Kind::Principal, -6.0, 1),
    stop(&[2.0], Kind::Principal, -11.0, 2),
    stop(&[0.5], Kind::Posaune, -4.0, 3),
];

const fn stop(ranks: &'static [f64], kind: Kind, level: f64, build: u8) -> Stop {
    Stop {
        ranks,
        kind,
        level,
        detune: 0.0,
        build,
    }
}

/// A mixture row breaks back an octave while it is over this, Hz.
const BREAK_HZ: f64 = 2700.0;
/// No pipe over or under these, Hz.
const HIGHEST_PIPE: f64 = 9000.0;
const LOWEST_PIPE: f64 = 15.0;
/// No overtone over this, Hz (and under 0.45 of the sample rate), at most this many a pipe.
const TOP_HZ: f64 = 15_000.0;
const MOST_OVERTONES: usize = 40;
/// Overtones quieter than this are left out, dB.
const QUIETEST: f64 = -70.0;
/// The work is done in blocks of this many samples.
const BLOCK: usize = 32;
/// An accent: how much quicker the pipes speak (a fraction), and how much more chiff (times).
const ACCENT_QUICKER: f64 = 0.35;
const ACCENT_CHIFF: f64 = 2.0;
/// The organ's notes against the kit's level.
const GAIN: f64 = 0.17;
/// How far a pipe's overtone sits off the phase its pitch has on the key, radians either way.
const PHASE_SPREAD: f64 = 0.5;
/// The breath round a pipe's pitch, dB under its tone.
const BREATH_DB: f64 = -40.0;
/// The tremulant: how far it moves the level (dB) and the pitch (cents) for each cent of vibrato asked for.
const TREMULANT_DB: f64 = 0.25;
const TREMULANT_CENTS: f64 = 0.4;
/// The swell box: how much quieter it is shut (dB), and its darkening's corner shut (Hz; 32 times higher open).
const BOX_DB: f64 = 24.0;
const BOX_SHUT_HZ: f64 = 600.0;

const DB: f64 = std::f64::consts::LN_10 / 20.0;

fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn hz(m: f64) -> f64 {
    440.0 * dmath::pow(2.0, (m - 69.0) / 12.0)
}

/// A pipe's steady tone: its overtones' levels, dB, for a pipe at `f` Hz.
fn spectrum(kind: Kind, f: f64, kmax: usize) -> Vec<f64> {
    (1..=kmax)
        .map(|k| {
            let kf = k as f64;
            let l2 = dmath::log2(kf);
            let odd = k % 2 == 1;
            let mut db = match kind {
                Kind::Principal => -5.2 * l2 - 0.9 * (kf - 1.0) + if odd { 0.0 } else { 1.0 },
                Kind::Stopped => {
                    if odd {
                        -8.0 * l2 - 1.2 * (kf - 1.0)
                    } else {
                        -34.0 - 2.0 * kf
                    }
                }
                Kind::Chimney => -11.0 * l2 - 1.5 * (kf - 1.0) + if odd && k > 1 { 4.0 } else { 0.0 },
                Kind::String => -3.0 * l2 - 0.35 * (kf - 1.0) - if k == 1 { 2.0 } else { 0.0 },
                Kind::Trumpet | Kind::Posaune | Kind::Oboe => {
                    let (fc, width, under) = match kind {
                        Kind::Trumpet => (1900.0, 0.7, 5.0),
                        Kind::Posaune => (950.0, 0.7, 5.0),
                        _ => (1300.0, 0.45, 8.0),
                    };
                    let fk = kf * f;
                    let ring = dmath::log2(fk / fc) / width;
                    let top = if kind == Kind::Oboe { 3000.0 } else { 4200.0 };
                    -1.3 * (kf - 1.0) + 7.0 * dmath::exp(-0.5 * ring * ring)
                        - if k == 1 { under } else { 0.0 }
                        - 16.0 * dmath::log2((fk / top).max(1.0))
                }
            };
            db -= 15.0 * dmath::log2((kf * f / 5000.0).max(1.0));
            db
        })
        .collect()
}

/// One pipe to sound.
struct Pipe {
    kind: Kind,
    /// Its pitch against the key's 8' pitch (before its own detune), and the key.
    ratio: f64,
    key: i64,
    /// Its pitch, Hz, with its own detune.
    f: f64,
    /// Its level, dB, and which stop it belongs to (for the crescendo).
    level: f64,
    gain_of: usize,
    seed: u64,
}

/// The pipes key `m` sounds on `stops`, and each stop's place in the crescendo `(stop index, build position from
/// 0, how many stops)`.
fn pipes_of(p: &PipeParts, m: f64, stops: &Stops) -> (Vec<Pipe>, Vec<(f64, f64)>) {
    let pedal = m < f64::from(p.pedal_top);
    let (table, bits): (&[Stop], u16) = if pedal {
        (&PEDAL, u16::from(stops.pedal))
    } else {
        (&MANUAL, stops.manual)
    };
    let drawn: Vec<usize> = (0..table.len()).filter(|i| bits & (1 << i) != 0).collect();
    // the crescendo's order: by each stop's build position
    let mut order = drawn.clone();
    order.sort_by_key(|&i| (table[i].build, i));
    let entry: Vec<(f64, f64)> = order
        .iter()
        .enumerate()
        .map(|(n, &i)| (i as f64, n as f64 / order.len().max(1) as f64))
        .collect();
    let f8 = hz(m);
    let key = m.round() as i64;
    let mut out = Vec::new();
    let couplers: &[(f64, f64)] = match (pedal, stops.sub, stops.octave) {
        (true, ..) | (false, false, false) => &[(1.0, 0.0)],
        (false, true, false) => &[(1.0, 0.0), (0.5, -1.0)],
        (false, false, true) => &[(1.0, 0.0), (2.0, -1.0)],
        (false, true, true) => &[(1.0, 0.0), (0.5, -1.0), (2.0, -1.0)],
    };
    for &i in &drawn {
        let st = &table[i];
        let place = entry.iter().position(|e| e.0 as usize == i).unwrap_or(0);
        for (c, &(octave, lift)) in couplers.iter().enumerate() {
            for (j, &r) in st.ranks.iter().enumerate() {
                let mut rr = r * octave;
                if st.ranks.len() > 1 {
                    while f8 * rr > BREAK_HZ {
                        rr /= 2.0;
                    }
                }
                let f = f8 * rr;
                if !(LOWEST_PIPE..=HIGHEST_PIPE).contains(&f) {
                    continue;
                }
                let multi = if st.ranks.len() > 1 { -2.0 } else { 0.0 };
                let tilt = if st.kind == Kind::Stopped {
                    0.0
                } else {
                    2.0 * ((m - 60.0) / 36.0).clamp(-1.0, 1.0)
                };
                // the same pipe every time it plays: its seed from the stop, the row, the key it stands on
                let stands = key + (12.0 * dmath::log2(rr)).round() as i64;
                let seed = (u64::from(pedal) << 60)
                    ^ ((i as u64) << 48)
                    ^ ((j as u64) << 40)
                    ^ ((c as u64) << 36)
                    ^ (stands as u64 & 0xffff_ffff);
                let mut rng = Generator::new(seed);
                let spread = if st.kind.reed() { 1.6 } else { 1.3 };
                let cents = rng.uniform(-spread, spread) + st.detune;
                out.push(Pipe {
                    kind: st.kind,
                    ratio: rr,
                    key,
                    f: f * dmath::pow(2.0, cents / 1200.0),
                    level: st.level + multi + tilt + lift,
                    gain_of: place,
                    seed,
                });
            }
        }
    }
    (out, entry)
}

/// How a note's shared wind moves over time: its pitch (cents) and level (dB) at `t` seconds.
struct Wind {
    cents: f64,
    db: f64,
    hz: f64,
    sag: f64,
    trem: Option<(f64, f64, f64)>,
}

impl Wind {
    fn at(&self, t: f64) -> (f64, f64) {
        let mut c = self.cents * waver(t, self.hz) - self.sag * (1.0 - dmath::exp(-t / 0.02)) * dmath::exp(-t / 0.12);
        let mut d = self.db * waver(t + 0.37, 0.8 * self.hz);
        if let Some((depth, rate, wait)) = self.trem {
            let fade = smooth((t - wait) / 0.15);
            let s = dmath::sin(dmath::TAU * rate * (t - wait).max(0.0));
            c += TREMULANT_CENTS * depth * fade * s;
            d += TREMULANT_DB * depth * fade * s;
        }
        (c, d)
    }
}

/// The note `play` on the organ `p`: `len` samples at sample rate `fs`. The organ's pipes are the same every time,
/// so `variant` only moves its breath.
pub fn note(p: &PipeParts, play: &Play, len: usize, variant: u64, fs: f64) -> Vec<f64> {
    if len == 0 {
        return Vec::new();
    }
    let m = if play.midi.is_finite() {
        play.midi.clamp(12.0, 120.0)
    } else {
        60.0
    };
    let stops = play.stops.unwrap_or_else(Stops::grand);
    let (pipes, entry) = pipes_of(p, m, &stops);
    let dur = play.dur.max(0.0);
    let swell = play.swell;
    let strength = |t: f64| match swell {
        Some(s) if s.over > 0.0 => s.from + (s.to - s.from) * smooth(t / s.over),
        Some(s) => s.to,
        None => 1.0,
    };
    // each drawn stop comes in as the crescendo passes its place: fully in at n / stops
    let stops_n = entry.len().max(1) as f64;
    let gain_at = |place: usize, t: f64| {
        let at = entry.get(place).map_or(0.0, |e| e.1);
        smooth((strength(t) - at + 0.5 / stops_n) * 2.0 * stops_n)
    };
    let wind = Wind {
        cents: p.wind_cents,
        db: p.wind_db,
        hz: p.wind_hz,
        sag: if play.together >= 3 {
            p.sag_cents.min(1.2 * f64::from(play.together - 2))
        } else {
            0.0
        },
        trem: play.vibrato.map(|v| (v.cents.abs(), v.hz.max(0.1), v.wait.max(0.0))),
    };
    let clipped = matches!(play.mute, Mute::Choked | Mute::Clipped);
    let accent = play.accent.clamp(0.0, 1.0);
    let vel = 20.0 * dmath::log10(play.vel.max(0.01));
    let mut x = vec![0.0; len];
    // one breath for the note, each pipe hearing it round its own pitch
    let mut rng = Generator::new(0x0e9a_b0e5 ^ (m.round() as u64) << 8 ^ variant);
    let noise: Vec<f64> = (0..len).map(|_| rng.standard_normal()).collect();
    for pipe in &pipes {
        sound_pipe(pipe, dur, clipped, accent, vel, &wind, &gain_at, &noise, &mut x, fs);
    }
    if let Some(e) = play.expression {
        swell_box(&mut x, e, fs);
    }
    let mut y = eq::apply(&x, p.polish, fs);
    y.iter_mut().for_each(|v| *v *= GAIN);
    y
}

/// How long the note `play` sounds with no length asked for, seconds, at most `longest`: its lowest pipe's stop.
pub fn rings_for(p: &PipeParts, play: &Play, longest: f64) -> f64 {
    let m = if play.midi.is_finite() { play.midi } else { 60.0 };
    let stops = play.stops.unwrap_or_else(Stops::grand);
    let (pipes, _) = pipes_of(p, m, &stops);
    let tail = pipes.iter().map(|q| tail_of(q.kind, q.f)).fold(0.012f64, f64::max);
    (play.dur.max(0.0) + 8.0 * tail + 0.05).min(longest).max(0.05)
}

/// How quickly a pipe stops, seconds (its time constant).
fn tail_of(kind: Kind, f: f64) -> f64 {
    if kind.reed() { 0.012 } else { 0.012 + 4.0 / f }
}

/// The impulse response's energy of `sos` (what white noise through it comes out at, squared).
fn noise_gain(sos: &dsp::Sos) -> f64 {
    let mut imp = vec![0.0; 4096];
    imp[0] = 1.0;
    dsp::sosfilt(sos, &imp)
        .iter()
        .map(|v| v * v)
        .sum::<f64>()
        .sqrt()
        .max(1e-12)
}

/// One pipe into `x`.
#[allow(clippy::too_many_arguments)]
fn sound_pipe(
    pipe: &Pipe,
    dur: f64,
    clipped: bool,
    accent: f64,
    vel: f64,
    wind: &Wind,
    gain_at: &dyn Fn(usize, f64) -> f64,
    noise: &[f64],
    x: &mut [f64],
    fs: f64,
) {
    let len = x.len();
    let f = pipe.f;
    let kind = pipe.kind;
    let reed = kind.reed();
    let kmax = MOST_OVERTONES.min((TOP_HZ.min(0.45 * fs) / f) as usize).max(1);
    let db = spectrum(kind, f, kmax);
    // each overtone's phase its own, but a pipe's own pitch starts in step with the other stops' pipes at that
    // pitch on the key, each a little off it (pipes speaking together at one pitch pull into step, as on a real
    // chest: two 16' stops never cancel each other)
    let mut rng = Generator::new(pipe.seed ^ 0x5eed);
    let phase: Vec<f64> = (0..kmax)
        .map(|i| {
            if i == 0 {
                let slot = (pipe.ratio * 2.0).round() as u64;
                let mut shared = Generator::new(0x0e9a_5107 ^ (pipe.key as u64) << 20 ^ slot);
                shared.uniform(0.0, dmath::TAU) + rng.uniform(-PHASE_SPREAD, PHASE_SPREAD)
            } else {
                rng.uniform(0.0, dmath::TAU)
            }
        })
        .collect();
    let amp: Vec<f64> = db
        .iter()
        .map(|&d| if d < QUIETEST { 0.0 } else { dmath::exp(d * DB) })
        .collect();
    // speech: a big pipe speaks slower; a string slower still; a reed at once, its top a hair late
    let slow = if kind == Kind::String { 1.8 } else { 1.0 };
    // a quick key (an accent) lets the wind in at once: quicker speech, more chiff
    let quick = 1.0 - ACCENT_QUICKER * accent;
    let tau1 = quick
        * if reed {
            0.006 + 2.5 / f
        } else {
            (0.014 + 13.0 / f) * slow
        };
    let tau: Vec<f64> = (0..kmax)
        .map(|i| {
            let k = (i + 1) as f64;
            if reed {
                quick * (0.006 + 2.5 / f) * (1.0 + 0.15 * (i as f64).sqrt())
            } else if i == 0 {
                tau1
            } else {
                quick * (0.012 + 9.0 / f) / k.sqrt()
            }
        })
        .collect();
    let tail = if clipped { 0.012 } else { tail_of(kind, f) };
    let level = dmath::exp((pipe.level + vel) * DB);
    // the pipe's own pitch: from a hair flat (a flue) or sharp (a reed), settling, and a drop as it lets go
    let bend = |t: f64| {
        let speak = if reed {
            9.0 * dmath::exp(-t / 0.025)
        } else {
            -6.0 * dmath::exp(-t / (0.5 * tau1))
        };
        let off = if t > dur {
            1.0 - dmath::exp(-(t - dur) / tail)
        } else {
            0.0
        };
        speak - 5.0 * off
    };
    let release = |t: f64| if t > dur { dmath::exp(-(t - dur) / tail) } else { 1.0 };
    let end = len.min(((dur + 8.0 * tail + 0.05) * fs) as usize);
    // the overtones, turned a block at a time
    let mut z: Vec<(f64, f64)> = phase.iter().map(|&p| (dmath::cos(p), dmath::sin(p))).collect();
    let mut prev: Vec<f64> = vec![0.0; kmax];
    let mut i0 = 0;
    while i0 < end {
        let n = BLOCK.min(end - i0);
        let t = (i0 as f64 + 0.5 * n as f64) / fs;
        let (wc, wdb) = wind.at(t);
        let fi = f * dmath::pow(2.0, (wc + bend(t)) / 1200.0);
        let w = dmath::TAU * fi / fs;
        let (c1, s1) = (dmath::cos(w), dmath::sin(w));
        let g = level * dmath::exp(wdb * DB) * release(t) * gain_at(pipe.gain_of, t);
        let te = (i0 + n) as f64 / fs;
        let (mut rc, mut rs) = (1.0, 0.0);
        for k in 0..kmax {
            (rc, rs) = (rc * c1 - rs * s1, rc * s1 + rs * c1);
            if amp[k] == 0.0 || fi * (k + 1) as f64 >= 0.45 * fs {
                // keep its phase turning
                let (mut zr, mut zi) = z[k];
                for _ in 0..n {
                    (zr, zi) = (zr * rc - zi * rs, zr * rs + zi * rc);
                }
                z[k] = (zr, zi);
                prev[k] = 0.0;
                continue;
            }
            let mut rise = 1.0 - dmath::exp(-te / tau[k]);
            if k == 1 && !reed {
                // the octave flicks in first
                rise *= 1.0 + 0.9 * dmath::exp(-te / (0.6 * tau1)) * (1.0 - dmath::exp(-te / 0.003));
            }
            let target = g * amp[k] * rise;
            let from = prev[k];
            prev[k] = target;
            let (mut zr, mut zi) = z[k];
            let step = (target - from) / n as f64;
            for (j, o) in x[i0..i0 + n].iter_mut().enumerate() {
                (zr, zi) = (zr * rc - zi * rs, zr * rs + zi * rc);
                *o += (from + step * (j + 1) as f64) * zi;
            }
            let mag = (zr * zr + zi * zi).sqrt();
            z[k] = (zr / mag, zi / mag);
        }
        i0 += n;
    }
    // the puff of air as a flue pipe starts, and its breath round its pitch while it sounds
    let gain_start = gain_at(pipe.gain_of, 0.0).max(gain_at(pipe.gain_of, dur.min(0.3)));
    if !reed && gain_start > 0.0 {
        let (lo, hi) = ((1.6 * f).min(14_000.0), (8.0 * f).min(18_000.0).min(0.45 * fs));
        if hi > lo * 1.1 {
            let c = end.min((0.15 * fs) as usize);
            let sos = butter_sos(FilterKind::Bp, &[lo, hi], 2, fs);
            let k = noise_gain(&sos);
            let nz = dsp::sosfilt(&sos, &noise[..c]);
            let amount = match kind {
                Kind::Stopped => 0.16,
                Kind::Chimney => 0.13,
                Kind::String => 0.05,
                _ => 0.09,
            } * (1.0 + ACCENT_CHIFF * accent);
            let fade = 0.012 + 1.5 / f;
            for (i, (o, v)) in x.iter_mut().zip(&nz).enumerate() {
                let t = i as f64 / fs;
                *o += level * gain_start * v / k * amount * dmath::exp(-t / fade) * (1.0 - dmath::exp(-t / 0.002));
            }
        }
    }
    let bw = (0.04 * f).max(15.0);
    let (lo, hi) = ((f - bw).max(20.0), (f + bw).min(0.45 * fs));
    if hi > lo * 1.01 {
        let sos = butter_sos(FilterKind::Bp, &[lo, hi], 2, fs);
        let k = noise_gain(&sos);
        let nz = dsp::sosfilt(&sos, &noise[..end]);
        let b = dmath::exp(BREATH_DB * DB) * level / k;
        let mut i0 = 0;
        while i0 < end {
            let n = BLOCK.min(end - i0);
            let t = (i0 as f64 + 0.5 * n as f64) / fs;
            let g = b * (1.0 - dmath::exp(-t / tau1)) * release(t) * gain_at(pipe.gain_of, t);
            for (o, v) in x[i0..i0 + n].iter_mut().zip(&nz[i0..i0 + n]) {
                *o += g * v;
            }
            i0 += n;
        }
    }
}

/// The swell box over the note: open (1) as it is, shut (0) [`BOX_DB`] quieter and darker.
fn swell_box(x: &mut [f64], e: super::play::Swell, fs: f64) {
    let open = |t: f64| {
        let v = if e.over > 0.0 {
            e.from + (e.to - e.from) * smooth(t / e.over)
        } else {
            e.to
        };
        v.clamp(0.0, 1.0)
    };
    let mut lp = 0.0;
    let mut i0 = 0;
    while i0 < x.len() {
        let n = BLOCK.min(x.len() - i0);
        let o = open(i0 as f64 / fs);
        let corner = (BOX_SHUT_HZ * dmath::pow(32.0, o)).min(0.45 * fs);
        let k = dmath::exp(-dmath::TAU * corner / fs);
        let g = dmath::exp(-BOX_DB * (1.0 - o) * DB);
        for v in x[i0..i0 + n].iter_mut() {
            lp = (1.0 - k) * *v + k * lp;
            *v = g * lp;
        }
        i0 += n;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SRF;
    use crate::voice::engine2::play::{Swell, Vibrato};

    const P: PipeParts = PipeParts::O8;

    fn sound(play: &Play, secs: f64) -> Vec<f64> {
        note(&P, play, (secs * SRF) as usize, 0, SRF)
    }

    fn db(x: &[f64], a: f64, b: f64) -> f64 {
        let (i, j) = ((a * SRF) as usize, ((b * SRF) as usize).min(x.len()));
        let e = x[i..j].iter().map(|v| v * v).sum::<f64>() / (j - i).max(1) as f64;
        10.0 * dmath::log10(e.max(1e-30))
    }

    /// The level at `f` Hz over `a..b` seconds, dB.
    fn at(x: &[f64], f: f64, a: f64, b: f64) -> f64 {
        let seg = &x[(a * SRF) as usize..(b * SRF) as usize];
        let w = dmath::TAU * f / SRF;
        let (mut re, mut im) = (0.0, 0.0);
        for (i, v) in seg.iter().enumerate() {
            let hann = 0.5 - 0.5 * dmath::cos(dmath::TAU * i as f64 / seg.len() as f64);
            re += v * hann * dmath::cos(w * i as f64);
            im += v * hann * dmath::sin(w * i as f64);
        }
        10.0 * dmath::log10(re * re + im * im + 1e-30)
    }

    fn bright(x: &[f64], a: f64, b: f64) -> f64 {
        let k = dmath::exp(-dmath::TAU * 2000.0 / SRF);
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

    #[test]
    fn each_stop_sounds_its_pitch() {
        let f = hz(60.0);
        let alone = |name: &str| {
            sound(
                &Play {
                    stops: Some(Stops::of(&[name], &[])),
                    ..Play::plain(60.0, 1.0, 0.8)
                },
                1.0,
            )
        };
        // a 16' an octave under, an 8' at pitch, a 4' an octave over, the 2 2/3' a twelfth over
        for (name, ratio) in [
            ("Bourdon 16'", 0.5),
            ("Principal 8'", 1.0),
            ("Octave 4'", 2.0),
            ("Quint 2 2/3'", 3.0),
        ] {
            let x = alone(name);
            let main = at(&x, f * ratio, 0.3, 0.9);
            assert!(main > at(&x, f * ratio * 1.5, 0.3, 0.9) + 10.0, "{name}");
            assert!(main > at(&x, f * ratio * 0.75, 0.3, 0.9) + 10.0, "{name}");
        }
        // a stopped flute has (almost) no even overtones
        let x = alone("Gedackt 8'");
        assert!(at(&x, 3.0 * f, 0.3, 0.9) > at(&x, 2.0 * f, 0.3, 0.9) + 15.0);
        // under G2 the pedal board plays
        let low = sound(&Play::plain(36.0, 1.0, 0.8), 1.0);
        assert!(at(&low, hz(24.0), 0.4, 0.9) > at(&low, hz(48.0), 0.4, 0.9) - 20.0);
    }

    #[test]
    fn every_set_sounds() {
        for s in [
            Stops::soft(),
            Stops::flutes(),
            Stops::strings(),
            Stops::principal(),
            Stops::chorus(),
            Stops::grand(),
            Stops::full(),
            Stops::trumpet(),
            Stops::oboe(),
        ] {
            for m in [36.0, 60.0, 84.0] {
                let x = sound(
                    &Play {
                        stops: Some(s),
                        ..Play::plain(m, 0.5, 0.8)
                    },
                    0.8,
                );
                assert!(x.iter().all(|v| v.is_finite()) && db(&x, 0.1, 0.4) > -60.0, "{s:?} {m}");
            }
        }
        // more stops, louder
        let level = |s: Stops| {
            db(
                &sound(
                    &Play {
                        stops: Some(s),
                        ..Play::plain(60.0, 0.5, 0.8)
                    },
                    0.8,
                ),
                0.1,
                0.4,
            )
        };
        assert!(level(Stops::soft()) < level(Stops::chorus()) - 3.0);
        assert!(level(Stops::chorus()) < level(Stops::full()));
    }

    #[test]
    fn strings_beat_and_flutes_are_pure() {
        let x = sound(
            &Play {
                stops: Some(Stops::strings()),
                ..Play::plain(60.0, 2.0, 0.8)
            },
            2.0,
        );
        // the celeste a hair sharp: the level of the pitch wanders as the two rows beat (about 1.5 times a second)
        let f = hz(60.0);
        let parts: Vec<f64> = (0..8)
            .map(|i| at(&x, f, 0.4 + 0.18 * i as f64, 0.58 + 0.18 * i as f64))
            .collect();
        let swing = parts.iter().copied().fold(f64::MIN, f64::max) - parts.iter().copied().fold(f64::MAX, f64::min);
        assert!(swing > 3.0, "{swing} dB");
        // a flute is round: far less up top than a string
        let fl = sound(
            &Play {
                stops: Some(Stops::flutes()),
                ..Play::plain(60.0, 1.0, 0.8)
            },
            1.0,
        );
        assert!(bright(&fl, 0.3, 0.9) < bright(&x, 0.3, 0.9) - 6.0);
    }

    #[test]
    fn pipes_speak_quickly_and_stop_quickly() {
        let x = sound(&Play::plain(60.0, 0.5, 0.8), 1.2);
        let full = db(&x, 0.25, 0.45);
        assert!(db(&x, 0.1, 0.15) > full - 2.5);
        assert!(db(&x, 0.0, 0.004) < full - 6.0);
        assert!(db(&x, 0.7, 0.9) < full - 40.0);
        let clipped = sound(
            &Play {
                mute: Mute::Clipped,
                ..Play::plain(48.0, 0.5, 0.8)
            },
            1.2,
        );
        let open = sound(&Play::plain(48.0, 0.5, 0.8), 1.2);
        assert!(db(&clipped, 0.55, 0.65) < db(&open, 0.55, 0.65) - 6.0);
        assert!(rings_for(&P, &Play::plain(60.0, 0.5, 0.8), 12.0) < 1.5);
    }

    #[test]
    fn a_swell_brings_the_stops_in() {
        let x = sound(
            &Play {
                swell: Some(Swell {
                    from: 0.0,
                    to: 1.0,
                    over: 2.0,
                }),
                ..Play::plain(60.0, 2.5, 0.8)
            },
            2.5,
        );
        assert!(db(&x, 0.2, 0.4) < db(&x, 2.1, 2.4) - 2.0);
        assert!(bright(&x, 0.2, 0.4) < bright(&x, 2.1, 2.4) - 1.5);
        // the first stop sounds from the start
        assert!(db(&x, 0.2, 0.4) > db(&x, 2.1, 2.4) - 20.0);
        // with no time to swell, it is at its end at once; with nothing asked, the whole set
        let at_once = sound(
            &Play {
                swell: Some(Swell {
                    from: 0.0,
                    to: 1.0,
                    over: 0.0,
                }),
                ..Play::plain(60.0, 0.5, 0.8)
            },
            0.8,
        );
        assert!((db(&at_once, 0.2, 0.4) - db(&sound(&Play::plain(60.0, 0.5, 0.8), 0.8), 0.2, 0.4)).abs() < 0.5);
    }

    #[test]
    fn the_box_shut_is_quieter_and_darker() {
        let boxed = |e: f64| {
            sound(
                &Play {
                    expression: Some(Swell {
                        from: e,
                        to: e,
                        over: 0.0,
                    }),
                    ..Play::plain(60.0, 1.0, 0.8)
                },
                1.0,
            )
        };
        let (shut, open) = (boxed(0.0), boxed(1.0));
        assert!(db(&shut, 0.3, 0.9) < db(&open, 0.3, 0.9) - 15.0);
        assert!(bright(&shut, 0.3, 0.9) < bright(&open, 0.3, 0.9) - 4.0);
        assert!((db(&open, 0.3, 0.9) - db(&sound(&Play::plain(60.0, 1.0, 0.8), 1.0), 0.3, 0.9)).abs() < 1.0);
    }

    #[test]
    fn the_tremulant_shakes_the_level() {
        let x = sound(
            &Play {
                vibrato: Some(Vibrato {
                    cents: 12.0,
                    hz: 5.5,
                    wait: 0.0,
                }),
                stops: Some(Stops::flutes()),
                ..Play::plain(60.0, 2.0, 0.8)
            },
            2.0,
        );
        let still = sound(
            &Play {
                stops: Some(Stops::flutes()),
                ..Play::plain(60.0, 2.0, 0.8)
            },
            2.0,
        );
        let swing = |y: &[f64]| {
            let parts: Vec<f64> = (0..20)
                .map(|i| db(y, 0.5 + 0.045 * i as f64, 0.53 + 0.045 * i as f64))
                .collect();
            parts.iter().copied().fold(f64::MIN, f64::max) - parts.iter().copied().fold(f64::MAX, f64::min)
        };
        assert!(swing(&x) > swing(&still) + 2.0, "{} {}", swing(&x), swing(&still));
    }

    #[test]
    fn couplers_add_octaves() {
        let f = hz(60.0);
        let with = |sub: bool, octave: bool| {
            sound(
                &Play {
                    stops: Some(Stops {
                        sub,
                        octave,
                        ..Stops::principal()
                    }),
                    ..Play::plain(60.0, 1.0, 0.8)
                },
                1.0,
            )
        };
        let plain = with(false, false);
        assert!(at(&with(true, false), f / 2.0, 0.3, 0.9) > at(&plain, f / 2.0, 0.3, 0.9) + 15.0);
        assert!(at(&with(false, true), 2.0 * f, 0.3, 0.9) > at(&plain, 2.0 * f, 0.3, 0.9) + 2.0);
    }

    #[test]
    fn a_big_chord_sags_a_hair() {
        let one = sound(&Play::plain(60.0, 0.5, 0.8), 0.5);
        let chord = sound(
            &Play {
                together: 6,
                ..Play::plain(60.0, 0.5, 0.8)
            },
            0.5,
        );
        assert_ne!(one, chord);
        assert!((db(&one, 0.2, 0.45) - db(&chord, 0.2, 0.45)).abs() < 0.5);
    }

    #[test]
    fn the_same_note_is_the_same_and_an_accent_speaks_quicker() {
        let play = Play::plain(55.0, 0.4, 0.8);
        let plain = sound(&play, 0.6);
        assert_eq!(plain, sound(&play, 0.6));
        let accented = sound(
            &Play {
                accent: 1.0,
                ..play.clone()
            },
            0.6,
        );
        assert!(db(&accented, 0.0, 0.04) > db(&plain, 0.0, 0.04) + 1.0);
        assert!((db(&accented, 0.2, 0.35) - db(&plain, 0.2, 0.35)).abs() < 0.5);
    }

    #[test]
    fn edge_notes_are_finite() {
        for (m, vel, dur, len) in [
            (12.0, 0.8, 0.5, 24_000),
            (24.0, 0.8, 0.5, 24_000),
            (108.0, 0.8, 0.5, 24_000),
            (127.0, 0.8, 0.5, 24_000),
            (60.5, 0.8, 0.5, 24_000),
            (60.0, 0.0, 0.5, 24_000),
            (60.0, 1.5, 0.5, 24_000),
            (60.0, 0.8, 0.0, 24_000),
            (60.0, 0.8, -1.0, 24_000),
            (60.0, 0.8, 0.5, 1),
            (60.0, 0.8, 0.5, 30),
            (f64::NAN, 0.8, 0.5, 4_800),
        ] {
            let x = note(&P, &Play::plain(m, dur, vel), len, 0, SRF);
            assert_eq!(x.len(), len);
            assert!(
                x.iter().all(|v| v.is_finite() && v.abs() < 2.0),
                "{m} {vel} {dur} {len}"
            );
        }
        let none = Play {
            stops: Some(Stops::of(&[], &[])),
            ..Play::plain(60.0, 0.5, 0.8)
        };
        assert!(note(&P, &none, 4_800, 0, SRF).iter().all(|&v| v == 0.0));
        assert!(note(&P, &Play::plain(60.0, 0.5, 0.8), 0, 0, SRF).is_empty());
        let x = note(&P, &Play::plain(60.0, 0.5, 0.8), 44_100, 0, 44_100.0);
        assert!(x.iter().all(|v| v.is_finite()) && x.iter().any(|v| v.abs() > 1e-3));
    }
}
