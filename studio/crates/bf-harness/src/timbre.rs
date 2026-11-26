//! An instrument note's fingerprint (sound upgrade, stage 3, S1): what a built voice is compared with a recording of
//! the real instrument on.
//!
//! Four things, as the design names them (`docs/studio/2026-11-25-sound-upgrade-design.md`, S1):
//!
//! - **The overtones**: the levels of the first harmonics over the steady part of the note (0.15 to 0.45 s), each in
//!   dB under the strongest ([`harmonics`]). The *fingerprint distance* ([`distance`]) compares the first eight,
//!   floored at -50 dB, as a root-mean-square difference.
//! - **How long the note takes to speak** ([`attack_ms`]): from a tenth of its peak level to nine tenths.
//! - **How the note ends** ([`release_ms`]): from the note's release to a tenth of its held level.
//! - **How far its pitch moves** ([`span`]): the widest pitch travel, in semitones, of a pitch track through the note
//!   ([`pitch_track`]).
//!
//! The numbers follow the review's scratch scripts (`sound-review/tools/round2/sf.py`, `b6.py`, `b7.py`), so the
//! figures quoted in the design are this module's. The references themselves (FluidR3, development only) are never
//! shipped or committed: only their fingerprints are, in `tests/reference/fluidr3.json`, made on the owner's PC by
//! `cargo run --release -p bf-harness --example references -- render ...`.
//!
//! Development measure only: it is not in the engine crates, so it uses the platform's maths.

use serde_json::{Value, json};

/// The sample rate of every note measured here.
pub const SR: f64 = 48_000.0;
/// The steady part of a note the overtones are read over, in seconds.
pub const STEADY: (f64, f64) = (0.15, 0.45);
/// How many overtones a fingerprint keeps.
pub const OVERTONES: usize = 12;
/// How many overtones the fingerprint distance compares.
pub const COMPARED: usize = 8;
/// The floor of an overtone level in the distance, in dB under the strongest.
pub const FLOOR: f64 = -50.0;
/// The level given to an overtone above 20 kHz.
pub const ABSENT: f64 = -99.0;

/// One note's fingerprint.
#[derive(Clone, Debug, PartialEq)]
pub struct Fingerprint {
    /// The MIDI note it was asked to play.
    pub midi: i32,
    /// The first [`OVERTONES`] harmonics' levels, dB under the strongest.
    pub harmonics: Vec<f64>,
    /// Milliseconds from a tenth of the peak level to nine tenths.
    pub attack_ms: f64,
    /// Milliseconds from the release to a tenth of the held level, if it gets there.
    pub release_ms: Option<f64>,
    /// The widest pitch travel through the note, in semitones, if the note has a pitch to track.
    pub span: Option<f64>,
}

/// The frequency of MIDI note `m`, in Hz.
pub fn hz(m: f64) -> f64 {
    440.0 * 2f64.powf((m - 69.0) / 12.0)
}

/// The fingerprint of note `x` (mono, at [`SR`]), asked to play MIDI note `midi` and released at `t_off` seconds.
/// `pitched` is false for a drum, whose pitch is not tracked.
pub fn fingerprint(x: &[f64], midi: i32, t_off: f64, pitched: bool) -> Fingerprint {
    let f0 = hz(f64::from(midi));
    Fingerprint {
        midi,
        harmonics: harmonics(x, f0, OVERTONES).iter().map(|v| round2(*v)).collect(),
        attack_ms: round2(attack_ms(x)),
        release_ms: release_ms(x, t_off).map(round2),
        span: if pitched {
            span(&pitch_track(x, f0, t_off + 0.05)).map(round2)
        } else {
            None
        },
    }
}

impl Fingerprint {
    /// As JSON: `{"midi", "harmonics", "attack_ms", "release_ms", "span"}`.
    pub fn to_json(&self) -> Value {
        json!({
            "midi": self.midi,
            "harmonics": self.harmonics,
            "attack_ms": self.attack_ms,
            "release_ms": self.release_ms,
            "span": self.span,
        })
    }

    /// From [`Fingerprint::to_json`]'s JSON, or `None` where a field is missing.
    pub fn from_json(v: &Value) -> Option<Fingerprint> {
        Some(Fingerprint {
            midi: i32::try_from(v.get("midi")?.as_i64()?).ok()?,
            harmonics: v
                .get("harmonics")?
                .as_array()?
                .iter()
                .map(|h| h.as_f64())
                .collect::<Option<_>>()?,
            attack_ms: v.get("attack_ms")?.as_f64()?,
            release_ms: v.get("release_ms").and_then(Value::as_f64),
            span: v.get("span").and_then(Value::as_f64),
        })
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// The fingerprint distance between two overtone profiles: the first [`COMPARED`] levels, floored at [`FLOOR`], as a
/// root-mean-square difference in dB.
pub fn distance(a: &[f64], b: &[f64]) -> f64 {
    let n = COMPARED.min(a.len()).min(b.len());
    if n == 0 {
        return 0.0;
    }
    let s: f64 = (0..n)
        .map(|k| {
            let d = a[k].clamp(FLOOR, 0.0) - b[k].clamp(FLOOR, 0.0);
            d * d
        })
        .sum();
    (s / n as f64).sqrt()
}

/// The levels of the first `n` harmonics of `f0` over the note's steady part, dB under the strongest: a Hann window,
/// zero-padded to 2^17 points, the strongest bin within 3 % of each harmonic.
pub fn harmonics(x: &[f64], f0: f64, n: usize) -> Vec<f64> {
    let (a, b) = ((STEADY.0 * SR) as usize, (STEADY.1 * SR) as usize);
    let seg: &[f64] = if x.len() >= b && b - a >= 2048 {
        &x[a..b]
    } else {
        &x[x.len() / 4..3 * x.len() / 4]
    };
    let len = 1usize << 17;
    let m = seg.len().min(len);
    let mut re = vec![0.0; len];
    let mut im = vec![0.0; len];
    for (i, v) in seg.iter().take(m).enumerate() {
        let w = if m > 1 {
            0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (m - 1) as f64).cos()
        } else {
            1.0
        };
        re[i] = v * w;
    }
    fft(&mut re, &mut im);
    let bin = SR / len as f64;
    let mut out: Vec<f64> = (1..=n)
        .map(|k| {
            let f = f0 * k as f64;
            if f > 20_000.0 {
                return ABSENT;
            }
            let lo = ((f * 0.97 / bin).floor() as usize + 1).max(1);
            let hi = ((f * 1.03 / bin).ceil() as usize).min(len / 2);
            let peak = (lo..hi.max(lo + 1))
                .map(|i| (re[i] * re[i] + im[i] * im[i]).sqrt())
                .fold(0.0, f64::max);
            20.0 * (peak + 1e-12).log10()
        })
        .collect();
    let top = out.iter().copied().filter(|v| *v > ABSENT).fold(f64::MIN, f64::max);
    for v in &mut out {
        if *v > ABSENT {
            *v -= top;
        }
    }
    out
}

/// The note's level envelope: the root mean square over 5 ms, centred.
fn envelope(x: &[f64]) -> Vec<f64> {
    let w = 240usize;
    let mut pre = vec![0.0; x.len() + 1];
    for (i, v) in x.iter().enumerate() {
        pre[i + 1] = pre[i] + v * v;
    }
    (0..x.len())
        .map(|i| {
            let a = i.saturating_sub(w / 2);
            let b = (i + w - w / 2).min(x.len());
            ((pre[b] - pre[a]) / w as f64).max(0.0).sqrt()
        })
        .collect()
}

/// Milliseconds from a tenth of the note's peak level (within its first 0.4 s) to nine tenths.
pub fn attack_ms(x: &[f64]) -> f64 {
    let e = envelope(x);
    let head = &e[..e.len().min((0.4 * SR) as usize)];
    let pk = head.iter().copied().fold(0.0, f64::max);
    if pk <= 0.0 {
        return 0.0;
    }
    let a = head.iter().position(|v| *v > 0.1 * pk).unwrap_or(0);
    let b = head.iter().position(|v| *v > 0.9 * pk).unwrap_or(0);
    (b as f64 - a as f64) / SR * 1000.0
}

/// Milliseconds from the release at `t_off` seconds to a tenth of the level held over the 50 ms before it, or `None`
/// where the note never falls that far.
pub fn release_ms(x: &[f64], t_off: f64) -> Option<f64> {
    let e = envelope(x);
    let i = ((t_off * SR) as usize).min(e.len());
    let held = &e[i.saturating_sub(2400)..i];
    if held.is_empty() {
        return None;
    }
    let r = held.iter().sum::<f64>() / held.len() as f64;
    e[i..].iter().position(|v| *v < 0.1 * r).map(|k| k as f64 / SR * 1000.0)
}

/// A pitch track through the first `until` seconds of the note, one MIDI pitch every 10 ms (60 ms windows), `None`
/// where a window is too quiet (under 5 % of the note's peak) or has no clear period. The period is looked for
/// within a fifth of `f0` either way.
pub fn pitch_track(x: &[f64], f0: f64, until: f64) -> Vec<Option<f64>> {
    let w = (0.06 * SR) as usize;
    let hop = (0.01 * SR) as usize;
    let end = x.len().min((until * SR) as usize);
    let pk = x.iter().fold(0.0f64, |m, v| m.max(v.abs())) + 1e-12;
    let lag_lo = ((SR / (f0 * 1.5)).floor() as usize).max(2);
    let lag_hi = ((SR / (f0 / 1.5)).ceil() as usize).min(w - 2);
    let mut out = Vec::new();
    let mut s = 0;
    while s + w <= end {
        let seg = &x[s..s + w];
        let mean = seg.iter().sum::<f64>() / w as f64;
        let seg: Vec<f64> = seg.iter().map(|v| v - mean).collect();
        let rms = (seg.iter().map(|v| v * v).sum::<f64>() / w as f64).sqrt();
        s += hop;
        if rms < 0.05 * pk || lag_hi <= lag_lo + 2 {
            out.push(None);
            continue;
        }
        let ac = |k: usize| -> f64 { (0..w - k).map(|i| seg[i] * seg[i + k]).sum() };
        let a0 = ac(0);
        let vals: Vec<f64> = (lag_lo - 1..=lag_hi + 1).map(ac).collect();
        let (mut best, mut bv) = (1usize, f64::MIN);
        for (j, v) in vals.iter().enumerate().take(vals.len() - 1).skip(1) {
            if *v > bv {
                bv = *v;
                best = j;
            }
        }
        if bv < 0.3 * a0 {
            out.push(None);
            continue;
        }
        let (l, c, r) = (vals[best - 1], vals[best], vals[best + 1]);
        let d = (l - r) / (2.0 * (l - 2.0 * c + r) + 1e-12);
        let k = (lag_lo - 1 + best) as f64 + d.clamp(-1.0, 1.0);
        out.push(Some(69.0 + 12.0 * (SR / k / 440.0).log2()));
    }
    out
}

/// The widest pitch travel of a track, in semitones (highest less lowest of its pitched windows), or `None` with
/// fewer than eight of them.
pub fn span(track: &[Option<f64>]) -> Option<f64> {
    let v: Vec<f64> = track.iter().flatten().copied().collect();
    if v.len() < 8 {
        return None;
    }
    let hi = v.iter().copied().fold(f64::MIN, f64::max);
    let lo = v.iter().copied().fold(f64::MAX, f64::min);
    Some(hi - lo)
}

/// An in-place radix-2 complex FFT (the length a power of two).
fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * std::f64::consts::PI / len as f64;
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let tr = re[b] * cr - im[b] * ci;
                let ti = re[b] * ci + im[b] * cr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let nr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = nr;
            }
        }
        len <<= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A held note of `f0` with harmonics at `levels` dB, 20 ms attack, released at 1 s with a 50 ms fall.
    fn note(f0: f64, levels: &[f64]) -> Vec<f64> {
        (0..(1.5 * SR) as usize)
            .map(|i| {
                let t = i as f64 / SR;
                let env = (t / 0.02).min(1.0) * if t < 1.0 { 1.0 } else { (-(t - 1.0) / 0.05).exp() };
                levels
                    .iter()
                    .enumerate()
                    .map(|(k, db)| 10f64.powf(db / 20.0) * (2.0 * std::f64::consts::PI * f0 * (k + 1) as f64 * t).sin())
                    .sum::<f64>()
                    * env
                    * 0.2
            })
            .collect()
    }

    #[test]
    fn the_overtones_are_read_back_in_db_under_the_strongest() {
        let levels = [-20.0, -2.0, -2.0, -1.0, -14.0, 0.0, -11.0, -21.0];
        let h = harmonics(&note(hz(39.0), &levels), hz(39.0), 8);
        for (got, want) in h.iter().zip(levels) {
            assert!((got - want).abs() < 0.5, "{h:?}");
        }
        assert!(distance(&h, &levels) < 0.5);
    }

    #[test]
    fn the_distance_floors_and_compares_eight() {
        let a = [0.0, -60.0, -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, 0.0];
        let b = [0.0, -50.0, -10.0, -10.0, -10.0, -10.0, -10.0, -10.0, -40.0];
        assert_eq!(distance(&a, &b), 0.0);
        let c = [-4.0, -50.0, -10.0, -10.0, -10.0, -10.0, -10.0, -10.0];
        assert!((distance(&b, &c) - (16.0f64 / 8.0).sqrt()).abs() < 1e-12);
    }

    #[test]
    fn a_steady_note_has_a_short_attack_a_clean_release_and_no_travel() {
        let x = note(hz(57.0), &[0.0, -6.0, -12.0]);
        let f = fingerprint(&x, 57, 1.0, true);
        assert!((10.0..25.0).contains(&f.attack_ms), "{f:?}");
        let r = f.release_ms.expect("it falls");
        assert!((100.0..130.0).contains(&r), "{f:?}");
        assert!(f.span.expect("a pitch") < 0.05, "{f:?}");
        assert_eq!(Fingerprint::from_json(&f.to_json()), Some(f));
    }

    #[test]
    fn a_slide_is_measured_in_semitones() {
        // a glide up two semitones over the first half second, then held
        let f0 = hz(48.0);
        let mut ph = 0.0;
        let x: Vec<f64> = (0..SR as usize)
            .map(|i| {
                let t = i as f64 / SR;
                let m = 48.0 + 2.0 * (t / 0.5).min(1.0);
                ph += 2.0 * std::f64::consts::PI * hz(m) / SR;
                0.3 * ph.sin()
            })
            .collect();
        let s = span(&pitch_track(&x, f0, 1.0)).expect("a pitch");
        assert!((s - 2.0).abs() < 0.15, "{s}");
    }
}
