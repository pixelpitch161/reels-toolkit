//! The struck-string engine: a piano built from how it works (`docs/studio/voice-engine-2-instruments.md`, "Piano";
//! Aedan's pick P16, the sketch `wgpiano_main.rs`).
//!
//! - **Strings.** Each key has one string in the bass, two in the tenor and three above, a hair apart in tune (more
//!   so in the treble). A string is its own vibrating modes, each a little sharper than a whole multiple of the
//!   lowest (a stiff wire's), each losing its swing at its own rate.
//! - **Hammer.** A mass on felt thrown at the strings: the felt pushes back harder the more it is squeezed, and
//!   harder while being squeezed than while springing back. How stiff the felt is, is found per key so the hammer
//!   stays on the strings as long as a real one does in that register (2.2 ms at C3, 1.6 at C4, 1.4 at C5): that is
//!   what sets how bright a key is. A harder strike squeezes the felt more and the tone brightens.
//! - **Bridge.** A key's strings stand on one bridge: their motion in step is lost to it quickly (the loud first
//!   sound), their motion out of step slowly (the long soft aftersound).
//! - **Tension.** A big swing stretches the string a little, raising its pitch for a moment (a loud note's glide).
//! - **Dampers.** Felt lands on the strings over 20 ms when the key comes up, quicker in the middle than in the
//!   bass; the top keys have none.
//! - **Board.** The strings' push on the bridge goes out through the soundboard ([`super::board`]).
//!
//! Every technique of the piano (Aedan's rule, 2026-11-26): the sustain pedal (all the way, half way, lifted at a
//! moment; the other strings ringing along under it), the soft pedal, the middle pedal, the strings damped by hand
//! ([`Mute::Muted`]), harmonics, a string plucked inside, accents and strength. The engine runs one sample at a time
//! at any sample rate, so it could play live.

// The strings' modes are flat arrays, string after string, walked by index in the inner loops (as the sketch does).
#![allow(clippy::needless_range_loop)]

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use bf_core::dmath;

use super::board;
use super::play::{Mute, Play, Technique};

/// The struck-string engine's settings (P16: [`StringsParts::P16`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StringsParts {
    /// The felt's stiffness against the register's own (1: the contact times above).
    pub hard: f64,
    /// How far a key's strings are apart in tune, against a freshly tuned piano's (about 0.3 cents).
    pub detune: f64,
    /// How much further apart the treble's strings are, at the top of the ramp from E4 to C6.
    pub treble_detune: f64,
    /// How much faster the treble's loud first sound fades, dB a second per octave over 400 Hz (to the 1.5).
    pub treble_fade: f64,
    /// How much harder the felt pushes back while being squeezed than while springing back.
    pub felt: f64,
    /// Where the hammer strikes, as a fraction of the string from its end.
    pub strike: f64,
    /// The lowest MIDI note with no damper.
    pub undamped_from: i32,
    /// How quickly the dampers stop the strings, against a real piano's.
    pub damper: f64,
    /// How much a big swing raises the pitch, against the strings' own stretch.
    pub glide: f64,
    /// How loud the other strings ringing along under the sustain pedal are, dB.
    pub ring_along_db: f64,
}

impl StringsParts {
    /// Aedan's pick, round 4 of the built voices.
    pub const P16: StringsParts = StringsParts {
        hard: 1.0,
        detune: 0.6,
        treble_detune: 1.2,
        treble_fade: 25.0,
        felt: 0.8,
        strike: 0.118,
        undamped_from: 89,
        damper: 1.0,
        glide: 1.0,
        ring_along_db: -26.0,
    };
}

/// The strings' tension, newtons.
const TENSION: f64 = 750.0;
/// Modes up to here, Hz (and under 0.45 of the sample rate), at most [`MOST_MODES`] a string.
const MODES_TOP: f64 = 14_000.0;
const MOST_MODES: usize = 160;
/// How long the hammer can be on the strings, seconds.
const HAMMER_WINDOW: f64 = 0.025;
/// How long the dampers take to land, seconds.
const DAMPER_LANDING: f64 = 0.02;
/// The strength an accent adds.
const ACCENT: f64 = 0.12;
/// The soft pedal's felt: softer, against the hammer's usual.
const SOFT_FELT: f64 = 0.4;
/// A hand on the strings: damping in dB a second, and more for each Hz.
const HAND_DB: f64 = 120.0;
const HAND_DB_PER_HZ: f64 = 0.04;
/// A finger at a harmonic's node: how fast the other modes die, dB a second.
const NODE_DB: f64 = 400.0;
/// A pluck: where the string is pulled from (fraction), how far for a strength of 1 (metres), and the fingertip's
/// softness (the modes over this die away as a soft pluck's do), Hz.
const PLUCK_AT: f64 = 0.17;
const PLUCK_M: f64 = 3.0e-3;
const PLUCK_SOFT_HZ: f64 = 1200.0;
/// The strings' force against the kit's level: the bridge force is newtons.
const GAIN: f64 = 0.01;
/// The other strings that ring along under the sustain pedal: the keys this many either side, their first modes.
const ALONG_KEYS: i32 = 24;
const ALONG_MODES: u16 = 4;
/// The quickest end a note is given, seconds (where a note is cut short).
const END_FADE: f64 = 0.008;

/// The sketch's own random numbers (xorshift), so each key is made up exactly as in P16.
struct Rng(u64);

impl Rng {
    fn new(s: u64) -> Rng {
        Rng(s.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03)
    }
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn uni(&mut self, a: f64) -> f64 {
        (self.next() * 2.0 - 1.0) * a
    }
}

fn cents(c: f64) -> f64 {
    dmath::pow(2.0, c / 1200.0)
}

fn hz(m: f64) -> f64 {
    440.0 * dmath::pow(2.0, (m - 69.0) / 12.0)
}

/// The tuner's stretch at `m`, cents: the treble a little sharp, the bass a little flat.
pub fn stretch(m: f64) -> f64 {
    if m > 60.0 { 0.3 * (m - 60.0) } else { -0.35 * (60.0 - m) }
}

/// The stretched pitch of key `m`, Hz.
pub fn tuned(m: f64) -> f64 {
    hz(m) * cents(stretch(m))
}

/// The wire's stiffness at `m` (how much sharper each mode is than a whole multiple).
fn stiffness(m: f64) -> f64 {
    if m > 72.0 {
        3.5e-4 * dmath::exp(0.04 * (m - 72.0))
    } else {
        3.5e-4 + 5e-6 * (40.0 - m).max(0.0)
    }
}

fn strings_of(m: f64) -> usize {
    if m < 31.0 {
        1
    } else if m < 41.0 {
        2
    } else {
        3
    }
}

/// How long the hammer stays on the strings at `midi` struck at strength 0.6, seconds.
pub fn contact(midi: f64) -> f64 {
    if midi <= 60.0 {
        (1.6e-3 * dmath::pow(2.0, -(midi - 60.0) / 26.0)).min(4.5e-3)
    } else if midi <= 72.0 {
        1.6e-3 * dmath::pow(2.0, -(midi - 60.0) / 62.0)
    } else {
        1.4e-3 * dmath::pow(2.0, -(midi - 72.0) / 90.0)
    }
}

/// The hammer's speed at strength `s`, metres a second.
fn hammer_speed(s: f64) -> f64 {
    0.3 + 5.0 * dmath::pow(s.max(0.0), 1.6)
}

/// A mode's loss rates at frequency `fk` (mode number `k`) of a key at `f0`: the loud first sound's and the
/// aftersound's, dB a second.
fn losses(p: &StringsParts, f0: f64, fk: f64, k: f64) -> (f64, f64) {
    let x0 = f0 / 250.0;
    let treble = dmath::log2(f0 / 400.0).max(0.0);
    let dp0 = 22.0 * x0 / (x0 + 0.7) + p.treble_fade * dmath::pow(treble, 1.5);
    let dp = dp0 + 0.0028 * fk + 0.4 * k;
    let da = 0.22 * dp + 0.0004 * fk;
    (dp, da.min(dp))
}

const DB: f64 = std::f64::consts::LN_10 / 20.0;

/// The recurrence of one mode losing `sig` a second at angular frequency `w`, step `dt`: `(a1, a2, g, sm)`, `g`
/// the response to a push held one step, `sm` how `a1` moves with a small rise in tension.
fn coeffs(sig: f64, w: f64, dt: f64) -> [f64; 4] {
    let r = dmath::exp(-sig * dt);
    let th = w * dt;
    let (c, s) = (dmath::cos(th), dmath::sin(th));
    [2.0 * r * c, r * r, r * s * dt / w, r * s * th]
}

/// A key's make-up: its strings' modes, where the hammer meets them, its hammer.
struct Key {
    ns: usize,
    nm: usize,
    /// Per string per mode (string-major): angular frequency and own loss rate.
    w: Vec<f64>,
    sig: Vec<f64>,
    phi: Vec<f64>,
    /// Per mode: its pull on the bridge, its number squared, the bridge's loss of in-step motion.
    out: Vec<f64>,
    kk: Vec<f64>,
    rho: Vec<f64>,
    /// The modes' frequencies, Hz (string 0's), for the strings ringing along.
    fk: Vec<f64>,
    force_gain: f64,
    hammer_mass: f64,
    felt_power: f64,
    tension_k: f64,
    /// Each string's distance from the hammer's line, metres.
    off: Vec<f64>,
}

fn make_key(m: f64, p: &StringsParts, rng: &mut Rng, strike: f64, fs: f64) -> Key {
    let f0 = tuned(m);
    let ns = strings_of(m);
    let b = stiffness(m);
    let len = (0.64 * dmath::pow(2.0, -(m - 60.0) * 0.9 / 12.0)).min(2.0);
    let mu = TENSION / ((2.0 * len * f0) * (2.0 * len * f0));
    let d: f64 = if m < 41.0 { 1.3e-3 } else { 1.0e-3 };
    let es = 2.0e11 * std::f64::consts::PI * (d / 2.0) * (d / 2.0);
    let top = MODES_TOP.min(0.45 * fs);
    let mut nm = 0;
    while nm < MOST_MODES {
        let k = (nm + 1) as f64;
        if k * f0 * (1.0 + b * k * k).sqrt() > top {
            break;
        }
        nm += 1;
    }
    let base: &[f64] = match ns {
        1 => &[0.0],
        2 => &[-0.3, 0.3],
        _ => &[-0.35, 0.05, 0.4],
    };
    let spread = p.detune * (0.6 + 0.8 * rng.next()) * (1.0 + p.treble_detune * ((m - 64.0) / 20.0).clamp(0.0, 1.0));
    let apart: Vec<f64> = base.iter().map(|c| c * spread + rng.uni(0.1) * p.detune).collect();
    let mut key = Key {
        ns,
        nm,
        w: vec![0.0; ns * nm],
        sig: vec![0.0; ns * nm],
        phi: vec![0.0; ns * nm],
        out: vec![0.0; nm],
        kk: vec![0.0; nm],
        rho: vec![0.0; nm],
        fk: vec![0.0; nm],
        force_gain: 2.0 / (mu * len),
        hammer_mass: 2.97e-3 * dmath::pow(2.0, -(m - 60.0) / 48.0),
        felt_power: 2.3 + 0.7 * ((m - 21.0) / 87.0).clamp(0.0, 1.0),
        tension_k: es * std::f64::consts::PI * std::f64::consts::PI / (4.0 * len * len) / TENSION,
        off: (0..ns).map(|_| rng.uni(1.5e-5)).collect(),
    };
    for j in 0..nm {
        let k = (j + 1) as f64;
        let fk = k * f0 * (1.0 + b * k * k).sqrt();
        let (dp, da) = losses(p, f0, fk, k);
        let (sp, sa) = (dp * DB, da * DB);
        key.fk[j] = fk;
        key.out[j] = if j % 2 == 0 { -k } else { k } * TENSION * std::f64::consts::PI / len;
        key.kk[j] = k * k;
        key.rho[j] = if ns > 1 { (sp - sa) / ns as f64 } else { 0.0 };
        for (i, c) in apart.iter().enumerate() {
            let ix = i * nm + j;
            key.w[ix] = dmath::TAU * fk * cents(*c);
            key.sig[ix] = if ns > 1 { sa } else { sp };
            key.phi[ix] = dmath::sin(std::f64::consts::PI * k * strike * (1.0 + 0.03 * (i as f64 - 1.0)));
        }
    }
    key
}

/// How one strike is played out: what the hammer does, and the dampers' and fingers' schedule.
struct Plan {
    /// How many of the key's strings the hammer meets (the soft pedal misses one), and with what felt.
    struck: usize,
    felt_k: f64,
    hammer_speed: f64,
    /// No hammer: the first string plucked from this many metres aside.
    pluck: Option<f64>,
    /// Damping added from the start, dB a second, per string per mode.
    extra_db: Vec<f64>,
    /// The dampers: (from sample, damping rate dB a second) steps, each landing over [`DAMPER_LANDING`].
    dampers: Vec<(usize, f64)>,
    /// The other strings ringing along, if the pedal lets them, at this level against [`StringsParts::ring_along_db`].
    along: f64,
}

/// One string ringing along: its own mode, driven by the bridge.
struct Along {
    w: f64,
    sig: f64,
    c: [f64; 4],
    drive: f64,
    q: f64,
    qp: f64,
}

/// The strike played out, `n` samples of the strings' force on the bridge, at sample rate `fs`.
fn play_out(key: &Key, plan: &Plan, along: &mut [Along], n: usize, fs: f64, glide: f64, felt: f64) -> Vec<f64> {
    let (ns, nm) = (key.ns, key.nm);
    let dt = 1.0 / fs;
    let mut q = vec![0.0; ns * nm];
    let mut qp = vec![0.0; ns * nm];
    let mut qn = vec![0.0; ns * nm];
    let mut cf: Vec<[f64; 4]> = (0..ns * nm)
        .map(|ix| coeffs(key.sig[ix] + plan.extra_db[ix] * DB, key.w[ix], dt))
        .collect();
    if let Some(a) = plan.pluck {
        // the first string pulled aside into a triangle at PLUCK_AT and let go (no speed): each mode's share of
        // the triangle, softened over PLUCK_SOFT_HZ as a fingertip's pluck is
        let pa = PLUCK_AT;
        for j in 0..nm {
            let k = (j + 1) as f64;
            let soft = 1.0 / (1.0 + (key.fk[j] / PLUCK_SOFT_HZ) * (key.fk[j] / PLUCK_SOFT_HZ));
            let share = 2.0 * dmath::sin(std::f64::consts::PI * k * pa)
                / (k * k * std::f64::consts::PI * std::f64::consts::PI * pa * (1.0 - pa));
            q[j] = a * share * soft;
            let [a1, a2, ..] = cf[j];
            // the step before, so the mode starts at rest: q[-1] = q[0] cos(th) / r
            let r = a2.sqrt();
            qp[j] = if r > 0.0 { q[j] * (a1 / (2.0 * r)) / r } else { q[j] };
        }
    }
    let mut out = vec![0.0; n];
    let (mut yh, mut vh) = (0.0f64, plan.hammer_speed);
    let kc = plan.felt_k / ns as f64;
    let hammer_n = if plan.pluck.is_some() {
        0
    } else {
        ((HAMMER_WINDOW * fs) as usize).min(n)
    };
    let land_n = ((DAMPER_LANDING * fs) as usize).max(1);
    let mut dprev = vec![0.0; ns];
    let mut fs_ = vec![0.0; ns];
    let mut vsum = vec![0.0; nm];
    let mut applied = 0.0f64;
    let mut fb_prev = 0.0;
    let mut peak = 0.0f64;
    let last_step = plan.dampers.last().map_or(0, |d| d.0 + land_n);
    for t in 0..n {
        // the dampers land: every 64 samples the modes take the damping reached so far
        if t % 64 == 0 && !plan.dampers.is_empty() {
            let mut want = 0.0;
            let mut before = 0.0;
            for &(at, rate) in &plan.dampers {
                if t >= at {
                    let f = ((t - at) as f64 / land_n as f64).min(1.0);
                    want = before + (rate - before) * f;
                    before = rate;
                }
            }
            if want != applied {
                applied = want;
                for (ix, c) in cf.iter_mut().enumerate() {
                    *c = coeffs(key.sig[ix] + (plan.extra_db[ix] + applied) * DB, key.w[ix], dt);
                }
                // the pedal's coming up lands every damper, the other strings' too
                for a in along.iter_mut() {
                    a.c = coeffs(a.sig + applied * DB, a.w, dt);
                }
            }
        }
        if t < hammer_n {
            for i in 0..plan.struck.min(ns) {
                let mut ys = 0.0;
                for j in 0..nm {
                    ys += key.phi[i * nm + j] * q[i * nm + j];
                }
                let dl = yh - ys - key.off[i];
                let ddl = (dl - dprev[i]) * fs;
                dprev[i] = dl;
                fs_[i] = if dl > 0.0 {
                    kc * dmath::pow(dl, key.felt_power) * (1.0 + felt * ddl).max(0.0)
                } else {
                    0.0
                };
            }
            let total: f64 = fs_.iter().sum();
            vh -= total / key.hammer_mass * dt;
            yh += vh * dt;
        } else if t == hammer_n {
            fs_.iter_mut().for_each(|f| *f = 0.0);
        }
        for j in 0..nm {
            let mut s = 0.0;
            for i in 0..ns {
                s += q[i * nm + j] - qp[i * nm + j];
            }
            vsum[j] = s * fs;
        }
        let mut fb = 0.0;
        for i in 0..ns {
            let mut e = 0.0;
            if glide > 0.0 {
                for j in 0..nm {
                    let x = q[i * nm + j];
                    e += key.kk[j] * x * x;
                }
            }
            let eps = glide * key.tension_k * e;
            let fh = fs_[i] * key.force_gain;
            for j in 0..nm {
                let ix = i * nm + j;
                let [a1, a2, g, sm] = cf[ix];
                let u = fh * key.phi[ix] - 2.0 * key.rho[j] * vsum[j];
                qn[ix] = (a1 - sm * eps) * q[ix] - a2 * qp[ix] + g * u;
                fb += key.out[j] * qn[ix];
            }
        }
        std::mem::swap(&mut qp, &mut q);
        std::mem::swap(&mut q, &mut qn);
        // the other strings, driven by the bridge's push a sample ago
        let mut ring = 0.0;
        for a in along.iter_mut() {
            let qn = a.c[0] * a.q - a.c[1] * a.qp + a.c[2] * a.drive * fb_prev;
            a.qp = a.q;
            a.q = qn;
            ring += qn;
        }
        fb_prev = fb;
        out[t] = (fb + ring) * GAIN;
        peak = peak.max(out[t].abs());
        // once the dampers are down and the strings are silent, the rest is silence
        if t % 1024 == 1023
            && t > last_step
            && t > hammer_n
            && !plan.dampers.is_empty()
            && out[t + 1 - 1024..=t].iter().all(|v| v.abs() < peak * 1e-6)
        {
            break;
        }
    }
    out
}

/// The felt stiffness that keeps the hammer on key `m`'s strings for its register's [`contact`] at strength 0.6,
/// found by halving (once per key, setting and sample rate).
fn felt_stiffness(m: f64, p: &StringsParts, fs: f64) -> f64 {
    /// The stiffness found, by (settings, key, sample rate).
    type Found = Mutex<HashMap<(u64, i64, u64), f64>>;
    static FOUND: OnceLock<Found> = OnceLock::new();
    let km = m.round() as i64;
    let tag = [p.detune, p.treble_detune, p.treble_fade, p.felt, p.strike]
        .iter()
        .fold(0xcbf2_9ce4_8422_2325u64, |h, v| {
            (h ^ v.to_bits()).wrapping_mul(0x0100_0000_01b3)
        });
    let found = FOUND.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(k) = found.lock().map_or(None, |f| f.get(&(tag, km, fs.to_bits())).copied()) {
        return k;
    }
    let mut rng = Rng::new(1);
    let key = make_key(km as f64, p, &mut rng, p.strike, fs);
    let target = contact(km as f64);
    let n = (0.02 * fs) as usize;
    let (mut lo, mut hi) = (5.0f64, 16.0f64);
    for _ in 0..30 {
        let mid = 0.5 * (lo + hi);
        let ct = contact_time(&key, dmath::pow(10.0, mid), hammer_speed(0.6), n, fs, p.felt);
        if ct.is_nan() || ct > target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let k = dmath::pow(10.0, 0.5 * (lo + hi));
    if let Ok(mut f) = found.lock() {
        f.insert((tag, km, fs.to_bits()), k);
    }
    k
}

/// How long the hammer stays on `key`'s strings with felt `k` at speed `v0`, seconds (NaN if it never leaves in
/// `n` samples).
fn contact_time(key: &Key, k: f64, v0: f64, n: usize, fs: f64, felt: f64) -> f64 {
    let (ns, nm) = (key.ns, key.nm);
    let dt = 1.0 / fs;
    let cf: Vec<[f64; 4]> = (0..ns * nm).map(|ix| coeffs(key.sig[ix], key.w[ix], dt)).collect();
    let mut q = vec![0.0; ns * nm];
    let mut qp = vec![0.0; ns * nm];
    let mut qn = vec![0.0; ns * nm];
    let (mut yh, mut vh) = (0.0f64, v0);
    let kc = k / ns as f64;
    let mut dprev = vec![0.0; ns];
    let mut f = vec![0.0; ns];
    let mut vsum = vec![0.0; nm];
    let (mut c0, mut c1) = (None, None);
    for t in 0..n {
        for i in 0..ns {
            let mut ys = 0.0;
            for j in 0..nm {
                ys += key.phi[i * nm + j] * q[i * nm + j];
            }
            let dl = yh - ys - key.off[i];
            let ddl = (dl - dprev[i]) * fs;
            dprev[i] = dl;
            f[i] = if dl > 0.0 {
                kc * dmath::pow(dl, key.felt_power) * (1.0 + felt * ddl).max(0.0)
            } else {
                0.0
            };
        }
        let total: f64 = f.iter().sum();
        vh -= total / key.hammer_mass * dt;
        yh += vh * dt;
        if total > 0.0 && c0.is_none() {
            c0 = Some(t);
        }
        if total == 0.0 && c0.is_some() && c1.is_none() {
            c1 = Some(t);
            break;
        }
        for j in 0..nm {
            let mut s = 0.0;
            for i in 0..ns {
                s += q[i * nm + j] - qp[i * nm + j];
            }
            vsum[j] = s * fs;
        }
        for i in 0..ns {
            let fh = f[i] * key.force_gain;
            for j in 0..nm {
                let ix = i * nm + j;
                let [a1, a2, g, _] = cf[ix];
                let u = fh * key.phi[ix] - 2.0 * key.rho[j] * vsum[j];
                qn[ix] = a1 * q[ix] - a2 * qp[ix] + g * u;
            }
        }
        std::mem::swap(&mut qp, &mut q);
        std::mem::swap(&mut q, &mut qn);
    }
    match (c0, c1) {
        (Some(a), Some(b)) => (b - a) as f64 / fs,
        _ => f64::NAN,
    }
}

/// The strength a note is struck with: its velocity and accent.
pub fn strength(play: &Play) -> f64 {
    (play.vel + ACCENT * play.accent.clamp(0.0, 1.0)).clamp(0.0, 1.15)
}

/// The note's key, held to the 88 keys and a little past.
fn key_of(play: &Play) -> f64 {
    if play.midi.is_finite() {
        play.midi.clamp(9.0, 120.0)
    } else {
        60.0
    }
}

/// The dampers' rate at `m`, dB a second.
fn damper_rate(p: &StringsParts, m: f64) -> f64 {
    p.damper * (120.0 + 180.0 * ((m - 30.0) / 30.0).clamp(0.0, 1.0))
}

/// When the note's dampers land and how hard: `(seconds, dB a second)` steps; none for a note that rings on.
fn dampers(p: &StringsParts, play: &Play, m: f64) -> Vec<(f64, f64)> {
    if m >= f64::from(p.undamped_from) {
        return Vec::new();
    }
    let dur = play.dur.max(0.0);
    let mut rate = damper_rate(p, m);
    if matches!(play.mute, Mute::Choked | Mute::Clipped) {
        rate *= 2.0;
    }
    let ped = &play.pedals;
    let sustain = ped.sustain.clamp(0.0, 1.0);
    let up = ped.lift.map(|u| u.max(dur));
    if ped.sostenuto || sustain >= 0.9 {
        // the dampers stay up until the pedal comes up
        return up.map_or_else(Vec::new, |u| vec![(u, rate)]);
    }
    if sustain > 0.0 {
        // a half pedal: the dampers only touch the strings at the key's release, then land when it comes up
        let light = rate * (1.0 - sustain / 0.9) * (1.0 - sustain / 0.9);
        let mut d = vec![(dur, light)];
        if let Some(u) = up {
            d.push((u.max(dur), rate));
        }
        return d;
    }
    vec![(dur, rate)]
}

/// How long the note `play` rings with no length asked for, seconds (the board's ring included), at most
/// `longest`.
pub fn rings_for(p: &StringsParts, play: &Play, longest: f64) -> f64 {
    let m = key_of(play);
    let ring = match dampers(p, play, m).last() {
        Some(&(at, rate)) => at + DAMPER_LANDING + 70.0 / rate.max(1.0) + 0.25,
        None => play.dur.max(0.0) + 6.0,
    };
    let ring = if play.mute == Mute::Muted {
        ring.min(70.0 / HAND_DB + 0.25)
    } else {
        ring
    };
    ring.min(longest).max(0.05)
}

/// The other strings that ring along with key `m` under the sustain pedal: every key within two octaves, its first
/// four modes, each driven by the bridge as a real string is. The ones whose modes meet the note's swing up most (to
/// about `level` times [`StringsParts::ring_along_db`] of the mode they meet, less for a higher mode of theirs); the
/// rest pick up only the hammer's knock and a little of the note, a quiet wash at their own pitches. Each then rings
/// on at its own slow rate.
fn ring_along(p: &StringsParts, m: f64, level: f64, fs: f64) -> Vec<Along> {
    if level <= 0.0 {
        return Vec::new();
    }
    let km = m.round();
    let f_note = tuned(m);
    let gain = level * dmath::exp(p.ring_along_db * DB);
    let dt = 1.0 / fs;
    let mut along = Vec::new();
    for c in -ALONG_KEYS..=ALONG_KEYS {
        let other = km + f64::from(c);
        if c == 0 || !(21.0..=108.0).contains(&other) {
            continue;
        }
        let f0 = tuned(other);
        let b = stiffness(other);
        for kc in 1..=ALONG_MODES {
            let k = f64::from(kc);
            let f = k * f0 * (1.0 + b * k * k).sqrt();
            if f > 0.45 * fs || f > MODES_TOP {
                break;
            }
            let (_, da) = losses(p, f0, f, k);
            // the note's own loss near this frequency: a push fading at that rate builds a swing of about
            // push / (2 w rate) in a mode it meets
            let (dp, _) = losses(p, f_note, f, (f / f_note).max(1.0));
            let w = dmath::TAU * f;
            along.push(Along {
                w,
                sig: da * DB,
                c: coeffs(da * DB, w, dt),
                drive: -gain / k * 2.0 * w * dp * DB,
                q: 0.0,
                qp: 0.0,
            });
        }
    }
    along
}

/// The note `play` on the struck-string piano `p`: `len` samples at sample rate `fs`, as repeat `variant` (each
/// repeat struck a hair differently; the key itself is the same every time).
pub fn note(p: &StringsParts, play: &Play, len: usize, variant: u64, fs: f64) -> Vec<f64> {
    if len == 0 {
        return Vec::new();
    }
    let m = key_of(play);
    let km = m.round().max(0.0) as u64;
    let mut krng = Rng::new(1000 + km * 7);
    let mut hit = Rng::new(77 + km * 131 + variant.wrapping_mul(7919));
    let strike = p.strike * (1.0 + krng.uni(0.04));
    let key = make_key(m, p, &mut krng, strike, fs);
    let s = (strength(play) * (1.0 + hit.uni(0.03))).clamp(0.02, 1.15);
    let ped = &play.pedals;
    let mut felt_k = felt_stiffness(m, p, fs) * p.hard;
    let mut struck = key.ns;
    if ped.soft {
        felt_k *= SOFT_FELT;
        struck = (key.ns - 1).max(1);
    }
    let mut extra_db = vec![0.0; key.ns * key.nm];
    for i in 0..key.ns {
        for j in 0..key.nm {
            let ix = i * key.nm + j;
            if play.mute == Mute::Muted {
                extra_db[ix] += HAND_DB + HAND_DB_PER_HZ * key.fk[j];
            }
            if let Technique::Harmonic(n) = play.technique {
                let n = usize::from(n.clamp(2, 6));
                if (j + 1) % n != 0 {
                    extra_db[ix] += NODE_DB;
                }
            }
        }
    }
    let dampers: Vec<(usize, f64)> = dampers(p, play, m)
        .into_iter()
        .map(|(at, rate)| ((at.max(0.0) * fs) as usize, rate))
        .collect();
    let sustain = ped.sustain.clamp(0.0, 1.0);
    let along_level = if ped.sostenuto || play.mute == Mute::Muted {
        0.0
    } else {
        let x = ((sustain - 0.4) / 0.5).clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    };
    let plan = Plan {
        struck,
        felt_k,
        hammer_speed: hammer_speed(s),
        pluck: (play.technique == Technique::Plucked).then_some(PLUCK_M * s),
        extra_db,
        dampers,
        along: along_level,
    };
    let mut along = ring_along(p, m, plan.along, fs);
    let x = play_out(&key, &plan, &mut along, len, fs, p.glide, p.felt);
    let mut y = board::apply(&x, fs);
    end_fade(&mut y, fs);
    y
}

/// The note's last few milliseconds faded, so a note cut short ends without a click.
fn end_fade(x: &mut [f64], fs: f64) {
    let n = ((END_FADE * fs) as usize).min(x.len() / 2);
    let len = x.len();
    for i in 0..n {
        let g = 0.5 - 0.5 * dmath::cos(std::f64::consts::PI * (i as f64 + 0.5) / n as f64);
        x[len - 1 - i] *= g;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SRF;
    use crate::voice::engine2::play::Pedals;

    const P: StringsParts = StringsParts::P16;

    fn ring(play: &Play, secs: f64) -> Vec<f64> {
        note(&P, play, (secs * SRF) as usize, 0, SRF)
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

    /// The strongest frequency near `f` (within 3%), Hz, by a fine search over 0.3 to 1.3 s.
    fn peak_near(x: &[f64], f: f64) -> f64 {
        let seg = &x[(0.3 * SRF) as usize..(1.3 * SRF) as usize];
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
            g *= 1.0 + 0.1 / 1200.0;
        }
        at
    }

    #[test]
    fn the_hammer_leaves_within_its_registers_contact() {
        for m in [36.0, 60.0, 84.0] {
            let mut rng = Rng::new(1);
            let key = make_key(m, &P, &mut rng, P.strike, SRF);
            let k = felt_stiffness(m, &P, SRF);
            let ct = contact_time(&key, k, hammer_speed(0.6), (0.02 * SRF) as usize, SRF, P.felt);
            assert!((ct / contact(m) - 1.0).abs() < 0.15, "{m}: {ct} against {}", contact(m));
        }
        // a harder strike squeezes the felt more and leaves sooner
        let mut rng = Rng::new(1);
        let key = make_key(60.0, &P, &mut rng, P.strike, SRF);
        let k = felt_stiffness(60.0, &P, SRF);
        let (soft, hard) = (
            contact_time(&key, k, hammer_speed(0.3), 960, SRF, P.felt),
            contact_time(&key, k, hammer_speed(1.0), 960, SRF, P.felt),
        );
        assert!(hard < soft, "{hard} {soft}");
    }

    #[test]
    fn a_harder_strike_is_louder_and_brighter() {
        let soft = ring(&Play::plain(60.0, 1.0, 0.3), 1.0);
        let hard = ring(&Play::plain(60.0, 1.0, 0.9), 1.0);
        assert!(db(&hard, 0.0, 0.3) > db(&soft, 0.0, 0.3) + 10.0);
        assert!(bright(&hard, 0.0, 0.3, 2000.0) > bright(&soft, 0.0, 0.3, 2000.0) + 2.0);
        let accented = ring(
            &Play {
                accent: 1.0,
                ..Play::plain(60.0, 1.0, 0.3)
            },
            1.0,
        );
        assert!(db(&accented, 0.0, 0.3) > db(&soft, 0.0, 0.3) + 1.0);
    }

    #[test]
    fn notes_are_in_tune() {
        for m in [40.0, 52.0, 60.0, 69.0, 81.0, 96.0] {
            let x = ring(&Play::plain(m, 1.5, 0.5), 1.5);
            let want = tuned(m);
            let got = peak_near(&x, want);
            let off = 1200.0 * dmath::log2(got / want);
            // the stiff wire's first mode sits a hair over the stretched pitch, the strings a hair either side
            assert!(off.abs() < 3.0, "{m}: {got} Hz against {want} ({off} cents)");
        }
    }

    #[test]
    fn a_damped_key_dies_and_a_top_key_rings_on() {
        let x = ring(&Play::plain(60.0, 0.3, 0.6), 1.5);
        assert!(db(&x, 1.0, 1.4) < db(&x, 0.0, 0.25) - 50.0);
        let held = ring(&Play::plain(60.0, 1.5, 0.6), 1.5);
        assert!(db(&held, 1.0, 1.4) > db(&x, 1.0, 1.4) + 30.0);
        // the top keys have no dampers: let go early or held, they ring the same
        let (top, top_held) = (
            ring(&Play::plain(96.0, 0.2, 0.6), 1.5),
            ring(&Play::plain(96.0, 1.5, 0.6), 1.5),
        );
        assert!((db(&top, 1.0, 1.4) - db(&top_held, 1.0, 1.4)).abs() < 0.5);
        // choked, the dampers land twice as hard
        let choked = ring(
            &Play {
                mute: Mute::Choked,
                ..Play::plain(48.0, 0.3, 0.6)
            },
            0.8,
        );
        let plain = ring(&Play::plain(48.0, 0.3, 0.6), 0.8);
        assert!(db(&choked, 0.32, 0.4) < db(&plain, 0.32, 0.4) - 1.0);
    }

    #[test]
    fn repeats_differ_but_the_same_note_is_the_same() {
        let play = Play::plain(64.0, 0.5, 0.7);
        let a = note(&P, &play, 24_000, 0, SRF);
        assert_eq!(a, note(&P, &play, 24_000, 0, SRF));
        let b = note(&P, &play, 24_000, 1, SRF);
        assert_ne!(a, b);
        assert!((db(&a, 0.0, 0.3) - db(&b, 0.0, 0.3)).abs() < 1.5);
    }

    #[test]
    fn edge_notes_are_finite() {
        for (m, vel, dur, len) in [
            (21.0, 0.6, 0.5, 24_000),
            (108.0, 0.6, 0.5, 24_000),
            (60.5, 0.6, 0.5, 24_000),
            (60.0, 0.0, 0.5, 24_000),
            (60.0, 1.2, 0.5, 24_000),
            (60.0, 0.6, 0.0, 24_000),
            (60.0, 0.6, -1.0, 24_000),
            (60.0, 0.6, 0.5, 1),
            (60.0, 0.6, 0.5, 30),
            (f64::NAN, 0.6, 0.5, 4_800),
        ] {
            let x = note(&P, &Play::plain(m, dur, vel), len, 0, SRF);
            assert_eq!(x.len(), len);
            assert!(
                x.iter().all(|v| v.is_finite() && v.abs() < 4.0),
                "{m} {vel} {dur} {len}"
            );
        }
        assert!(note(&P, &Play::plain(60.0, 0.5, 0.6), 0, 0, SRF).is_empty());
        // a full accent at full strength still leaves the strings
        let x = note(
            &P,
            &Play {
                accent: 1.0,
                ..Play::plain(60.0, 0.5, 1.0)
            },
            24_000,
            0,
            SRF,
        );
        assert!(x.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn half_pedal_dies_between_damped_and_held() {
        let at = |sustain: f64| {
            let x = ring(
                &Play {
                    pedals: Pedals { sustain, ..Pedals::UP },
                    ..Play::plain(55.0, 0.2, 0.6)
                },
                1.5,
            );
            db(&x, 0.9, 1.2)
        };
        let (up, half, down) = (at(0.0), at(0.5), at(1.0));
        assert!(up < half - 10.0 && half < down - 10.0, "{up} {half} {down}");
    }

    #[test]
    fn the_pedal_lifts_when_asked() {
        let lifted = ring(
            &Play {
                pedals: Pedals {
                    lift: Some(0.6),
                    ..Pedals::SUSTAIN
                },
                ..Play::plain(55.0, 0.2, 0.6)
            },
            1.5,
        );
        let held = ring(
            &Play {
                pedals: Pedals::SUSTAIN,
                ..Play::plain(55.0, 0.2, 0.6)
            },
            1.5,
        );
        assert!((db(&lifted, 0.3, 0.55) - db(&held, 0.3, 0.55)).abs() < 1.0);
        assert!(db(&lifted, 1.1, 1.4) < db(&held, 1.1, 1.4) - 40.0);
    }

    #[test]
    fn the_other_strings_ring_along_only_under_the_pedal() {
        let pedalled = |sustain: f64, sostenuto: bool| {
            ring(
                &Play {
                    pedals: Pedals {
                        sustain,
                        sostenuto,
                        ..Pedals::UP
                    },
                    ..Play::plain(48.0, 0.2, 0.7)
                },
                2.0,
            )
        };
        let (down, held_alone) = (pedalled(1.0, false), pedalled(0.0, true));
        // the key a fifth up (none of the note's own tones) rings along under the sustain pedal, not the middle one
        let level = |x: &[f64], f: f64| {
            let seg = &x[(0.5 * SRF) as usize..(1.5 * SRF) as usize];
            let w = dmath::TAU * f / SRF;
            let (mut re, mut im) = (0.0, 0.0);
            for (i, v) in seg.iter().enumerate() {
                let hann = 0.5 - 0.5 * dmath::cos(dmath::TAU * i as f64 / seg.len() as f64);
                re += v * hann * dmath::cos(w * i as f64);
                im += v * hann * dmath::sin(w * i as f64);
            }
            10.0 * dmath::log10(re * re + im * im + 1e-30)
        };
        let fifth = peak_near(&down, tuned(55.0));
        assert!(level(&down, fifth) > level(&held_alone, fifth) + 10.0);
        assert!(ring_along(&P, 48.0, 0.0, SRF).is_empty());
        // and adds a halo, not a second note
        assert!((db(&down, 0.3, 2.0) - db(&held_alone, 0.3, 2.0)).abs() < 3.0);
    }

    #[test]
    fn soft_is_softer_and_darker() {
        let plain = ring(&Play::plain(60.0, 1.0, 0.7), 1.0);
        let soft = ring(
            &Play {
                pedals: Pedals {
                    soft: true,
                    ..Pedals::UP
                },
                ..Play::plain(60.0, 1.0, 0.7)
            },
            1.0,
        );
        assert!(db(&soft, 0.0, 0.3) < db(&plain, 0.0, 0.3) - 0.7);
        assert!(bright(&soft, 0.0, 0.3, 2000.0) < bright(&plain, 0.0, 0.3, 2000.0) - 1.2);
    }

    #[test]
    fn a_hand_damped_note_is_short() {
        let muted = ring(
            &Play {
                mute: Mute::Muted,
                ..Play::plain(52.0, 1.5, 0.7)
            },
            1.5,
        );
        let open = ring(&Play::plain(52.0, 1.5, 0.7), 1.5);
        assert!(db(&muted, 0.6, 0.9) < db(&open, 0.6, 0.9) - 30.0);
        assert!(db(&muted, 0.0, 0.05) > db(&open, 0.0, 0.05) - 6.0);
    }

    #[test]
    fn a_harmonic_sounds_an_octave_up() {
        let x = ring(
            &Play {
                technique: Technique::Harmonic(2),
                ..Play::plain(48.0, 1.5, 0.7)
            },
            1.5,
        );
        let f0 = tuned(48.0);
        let level = |f: f64| {
            let g = peak_near(&x, f);
            let w = dmath::TAU * g / SRF;
            let seg = &x[(0.4 * SRF) as usize..(1.2 * SRF) as usize];
            let (mut re, mut im) = (0.0, 0.0);
            for (i, v) in seg.iter().enumerate() {
                re += v * dmath::cos(w * i as f64);
                im += v * dmath::sin(w * i as f64);
            }
            10.0 * dmath::log10(re * re + im * im + 1e-30)
        };
        assert!(level(2.0 * f0) > level(f0) + 25.0);
    }

    #[test]
    fn a_plucked_note_rings_without_the_hammers_knock() {
        let plucked = ring(
            &Play {
                technique: Technique::Plucked,
                ..Play::plain(48.0, 1.5, 0.7)
            },
            1.5,
        );
        let struck = ring(&Play::plain(48.0, 1.5, 0.7), 1.5);
        assert!((db(&plucked, 0.3, 1.0) - db(&struck, 0.3, 1.0)).abs() < 8.0);
        assert!(bright(&plucked, 0.0, 0.05, 3000.0) < bright(&struck, 0.0, 0.05, 3000.0) - 1.5);
    }

    #[test]
    fn it_plays_at_another_sample_rate() {
        let x = note(&P, &Play::plain(60.0, 0.5, 0.6), 44_100, 0, 44_100.0);
        assert_eq!(x.len(), 44_100);
        assert!(x.iter().all(|v| v.is_finite()));
        assert!(x.iter().any(|v| v.abs() > 1e-3));
    }
}
