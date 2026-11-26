//! Measuring a recording of a real instrument for Voice Engine 2 (`bf_sound::voice::engine2`): its pitch and string
//! stiffness, each overtone's level and pitch over time, the noise between the overtones, how fast it stops, and the
//! yardstick numbers the engine's notes are checked against. The `recipes` tool uses it on the recordings and on the
//! recipe engine's own notes, so both are measured the same way.
//!
//! An overtone is read by turning the recording down by the overtone's frequency (multiplying by `e^(-i 2 pi f t)`)
//! and averaging over a Blackman window six of the note's overtone spacings long, which keeps each overtone apart
//! from its neighbours. Only the samples of the recording count toward the average, so the first moments of a note
//! read at their own level rather than half of it. The noise is what is left in a short frame once the overtones,
//! as measured there, are taken out.

use std::path::Path;
use std::process::Command;

use bf_core::dmath;
use bf_sound::SRF;
use bf_sound::dsp::fft::Fft;
use bf_sound::voice::engine2::play::{Play, Swell};
use bf_sound::voice::engine2::recipe::{BANDS, Note, Partial, band_edges, grid};
use bf_sound::voice::engine2::{self, Instrument, Parts};
use serde_json::{Value, json};

/// The recording at `path` as mono at 48 kHz (its channels averaged), decoded by ffmpeg.
///
/// # Errors
/// ffmpeg missing or refusing the file.
pub fn decode(path: &Path, ffmpeg: &Path) -> Result<Vec<f64>, String> {
    let out = Command::new(ffmpeg)
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-f", "f64le", "-ac", "1", "-ar", "48000", "-"])
        .output()
        .map_err(|e| format!("{}: {e}", ffmpeg.display()))?;
    if !out.status.success() {
        return Err(format!(
            "ffmpeg on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(out
        .stdout
        .as_chunks::<8>()
        .0
        .iter()
        .map(|c| f64::from_le_bytes(*c))
        .collect())
}

/// Where the note starts: 1 ms before the first sample above -40 dB of the recording's peak.
pub fn onset(x: &[f64]) -> usize {
    let peak = x.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
    let at = x.iter().position(|v| v.abs() >= peak * 0.01).unwrap_or(0);
    at.saturating_sub(48)
}

fn hz(midi: f64) -> f64 {
    440.0 * dmath::pow(2.0, (midi - 69.0) / 12.0)
}

/// The Blackman window of `len` samples.
fn blackman(len: usize) -> Vec<f64> {
    let d = (len - 1).max(1) as f64;
    (0..len)
        .map(|n| {
            let a = dmath::TAU * n as f64 / d;
            0.42 - 0.5 * dmath::cos(a) + 0.08 * dmath::cos(2.0 * a)
        })
        .collect()
}

/// The window for overtones `spacing` Hz apart: six spacings long, an odd number of samples.
fn window_for(spacing: f64) -> Vec<f64> {
    blackman(((6.0 / spacing * SRF) as usize) | 1)
}

/// The overtone at `f` Hz around sample `c`: its amplitude (the windowed average, counting only the recording's
/// own samples) and the complex sum, phase referred to sample 0.
fn heterodyne(x: &[f64], c: isize, f: f64, w: &[f64]) -> (f64, (f64, f64)) {
    let h = (w.len() / 2) as isize;
    let lo = (c - h).max(0);
    let hi = (c + h + 1).min(x.len() as isize);
    if hi <= lo {
        return (0.0, (0.0, 0.0));
    }
    let step = dmath::TAU * f / SRF;
    let start = (f * lo as f64 / SRF).fract() * dmath::TAU;
    let (mut pr, mut pi) = (dmath::cos(start), -dmath::sin(start));
    let (rr, ri) = (dmath::cos(step), -dmath::sin(step));
    let (mut sr, mut si, mut ws) = (0.0, 0.0, 0.0);
    for n in lo..hi {
        let wv = w[(n - c + h) as usize];
        let v = x[n as usize] * wv;
        sr += v * pr;
        si += v * pi;
        ws += wv;
        let t = pr * rr - pi * ri;
        pi = pr * ri + pi * rr;
        pr = t;
    }
    if ws <= 0.0 {
        return (0.0, (0.0, 0.0));
    }
    (2.0 * dmath::hypot(sr, si) / ws, (2.0 * sr / ws, 2.0 * si / ws))
}

/// The note's fundamental and string stiffness: `(f0, b)`, fitted on its first 20 overtones in a 0.68 s frame from
/// 50 ms in. Partial k is taken to sit at `k f0 sqrt(1 + b k^2)`; each round looks for it within 2% / sqrt(k) of
/// where the last round put it (at least three bins), keeps the overtones within 40 dB of the strongest, and fits
/// `(f_k / k)^2 = f0^2 + f0^2 b k^2` by least squares weighted by their strength.
/// Without `stretched`, `b` is 0 and `f0` is the overtones' amplitude-weighted mean of `f_k / k`.
pub fn pitch(x: &[f64], nominal: f64, stretched: bool) -> (f64, f64) {
    const N: usize = 1 << 16;
    let from = (0.05 * SRF) as usize;
    let len = ((0.68 * SRF) as usize).min(x.len().saturating_sub(from));
    let (mut re, mut im) = (vec![0.0; N], vec![0.0; N]);
    for i in 0..len {
        let w = 0.5 - 0.5 * dmath::cos(dmath::TAU * i as f64 / len as f64);
        re[i] = x[from + i] * w;
    }
    Fft::new(N).forward(&mut re, &mut im);
    let mag: Vec<f64> = (0..N / 2)
        .map(|k| dmath::ln(re[k] * re[k] + im[k] * im[k] + 1e-30))
        .collect();
    let bin = SRF / N as f64;
    let top = mag.iter().copied().fold(f64::MIN, f64::max);
    let (mut f0, mut b) = (nominal, 0.0);
    for (round, kmax) in [8_usize, 14, 20].into_iter().enumerate() {
        let mut pts: Vec<(f64, f64, f64)> = Vec::new();
        for k in 1..=kmax {
            let kf = k as f64;
            let want = kf * f0 * (1.0 + b * kf * kf).sqrt();
            if want > 18_000.0 {
                break;
            }
            let span = (want * 0.02 / kf.sqrt() / if round == 0 { 1.0 } else { 2.0 }).max(3.0 * bin);
            let (a, z) = (((want - span) / bin) as usize, ((want + span) / bin) as usize + 1);
            let Some(p) = (a.max(1)..z.min(N / 2 - 1)).max_by(|&i, &j| mag[i].total_cmp(&mag[j])) else {
                continue;
            };
            // a peak, not the skirt of one just outside the search
            let edge = p <= a.max(1) || p + 1 >= z.min(N / 2 - 1);
            if edge || mag[p] < top - 2.0 * 40.0 * std::f64::consts::LN_10 / 20.0 {
                continue;
            }
            let (l, m, r) = (mag[p - 1], mag[p], mag[p + 1]);
            let den = l - 2.0 * m + r;
            let d = if den.abs() > 1e-12 { 0.5 * (l - r) / den } else { 0.0 };
            pts.push((kf, (p as f64 + d) * bin, dmath::exp(m / 2.0)));
        }
        if pts.is_empty() {
            break;
        }
        if stretched && pts.len() >= 3 {
            // weighted by each overtone's strength, so a weak, noisy fundamental counts for little
            let ys: Vec<(f64, f64, f64)> = pts.iter().map(|&(k, f, a)| (k * k, (f / k) * (f / k), a)).collect();
            let n: f64 = ys.iter().map(|p| p.2).sum();
            let (sx, sy) = ys.iter().fold((0.0, 0.0), |(a, c), &(x, y, w)| (a + w * x, c + w * y));
            let (mx, my) = (sx / n, sy / n);
            let (mut sxx, mut sxy) = (0.0, 0.0);
            for &(x, y, w) in &ys {
                sxx += w * (x - mx) * (x - mx);
                sxy += w * (x - mx) * (y - my);
            }
            let slope = if sxx > 0.0 { sxy / sxx } else { 0.0 };
            let a0 = my - slope * mx;
            if a0 > 0.0 {
                f0 = a0.sqrt();
                b = (slope / a0).max(0.0);
            }
        } else {
            let (num, den) = pts.iter().fold((0.0, 0.0), |(n, d), &(k, f, a)| (n + a * f / k, d + a));
            f0 = num / den;
            b = 0.0;
        }
    }
    (f0, if stretched { b } else { 0.0 })
}

/// How a recording is measured.
#[derive(Clone, Copy, Debug)]
pub struct Opts {
    /// Strings: the overtones stretch sharp (`b` fitted).
    pub stretched: bool,
    /// The overtones lie on half the note's frequency (an organ chorus with a 16' stop).
    pub sub: bool,
    /// At most this many overtones.
    pub max_partials: usize,
    /// Measure until this many seconds.
    pub end: f64,
    /// An overtone is kept only while it is within this many dB (negative) of the note's loudest.
    pub floor_db: f64,
    /// Take the recording's steady hiss out of the noise and stop each overtone where it sinks into the hiss.
    pub dehiss: bool,
    /// No overtone above this frequency, Hz.
    pub top_hz: f64,
}

/// A frame of the noise between the overtones: the band levels (dB, mean power of the band) of a 2,048-sample Hann
/// frame around sample `c`, after each overtone, as measured there, is taken out.
fn noise_frame(x: &[f64], c: isize, overtones: &[(f64, (f64, f64))], fft: &Fft, edges: &[f64]) -> [f64; BANDS] {
    const N: usize = 2048;
    let (mut re, mut im) = (vec![0.0; N], vec![0.0; N]);
    let n0 = c - (N / 2) as isize;
    for (i, r) in re.iter_mut().enumerate() {
        let n = n0 + i as isize;
        if n >= 0 && n < x.len() as isize {
            *r = x[n as usize];
        }
    }
    // each overtone as measured here, a steady tone, taken out (turned sample by sample, not recomputed)
    for &(f, (zr, zi)) in overtones {
        let step = dmath::TAU * f / SRF;
        let start = (f * n0 as f64 / SRF).rem_euclid(1.0) * dmath::TAU;
        let (mut pr, mut pi) = (dmath::cos(start), dmath::sin(start));
        let (rr, ri) = (dmath::cos(step), dmath::sin(step));
        for r in re.iter_mut() {
            *r -= zr * pr - zi * pi;
            let t = pr * rr - pi * ri;
            pi = pr * ri + pi * rr;
            pr = t;
        }
    }
    let mut wsq = 0.0;
    for (i, r) in re.iter_mut().enumerate() {
        let n = n0 + i as isize;
        let w = 0.5 - 0.5 * dmath::cos(dmath::TAU * i as f64 / N as f64);
        wsq += w * w;
        *r = if n >= 0 && n < x.len() as isize { *r * w } else { 0.0 };
    }
    fft.forward(&mut re, &mut im);
    let bin = SRF / N as f64;
    let mut out = [f64::NEG_INFINITY; BANDS];
    for (b, o) in out.iter_mut().enumerate() {
        let (lo, hi) = (
            (edges[b] / bin).ceil() as usize,
            ((edges[b + 1] / bin).floor() as usize).min(N / 2 - 1),
        );
        let p: f64 = (lo.max(1)..=hi).map(|k| re[k] * re[k] + im[k] * im[k]).sum();
        let p = 2.0 * p / (N as f64 * wsq);
        if hi >= lo && p > 0.0 {
            *o = 10.0 * dmath::log10(p);
        }
    }
    // a band too narrow to hold a bin takes its neighbours' mean
    for b in 0..BANDS {
        if out[b].is_infinite() {
            let near: Vec<f64> = [b.wrapping_sub(1), b + 1]
                .iter()
                .filter_map(|&j| out.get(j))
                .copied()
                .filter(|v| v.is_finite())
                .collect();
            if !near.is_empty() {
                out[b] = near.iter().sum::<f64>() / near.len() as f64;
            }
        }
    }
    out
}

fn db(a: f64) -> f64 {
    if a > 0.0 {
        20.0 * dmath::log10(a)
    } else {
        f64::NEG_INFINITY
    }
}

fn pow_db(p_db: f64) -> f64 {
    if p_db.is_finite() {
        dmath::pow(10.0, p_db / 10.0)
    } else {
        0.0
    }
}

/// One recorded note measured into a recipe note on [`grid`]`(o.end)` (cut at the recording's end): `x` from its
/// onset, `nominal_midi` the note it is meant to be. Its `release` is left 0 (see [`release`]).
pub fn measure(x: &[f64], nominal_midi: f64, layer: usize, o: &Opts) -> Note {
    let x = &x[onset(x)..];
    let (f0, b) = pitch(x, hz(nominal_midi), o.stretched);
    let spacing = if o.sub { f0 / 2.0 } else { f0 };
    let w = window_for(spacing);
    let dur = x.len() as f64 / SRF;
    let g: Vec<f64> = grid(o.end).into_iter().filter(|&t| t <= dur).collect();
    let edges = band_edges();
    let fft = Fft::new(2048);
    let ms = 48_isize;
    // the overtones: frequency, then per point (amplitude, complex sum, cents)
    let mut freqs = Vec::new();
    let mut numbers = Vec::new();
    for k in 1..=o.max_partials {
        let kf = k as f64;
        let f = if o.sub {
            kf * spacing
        } else {
            kf * f0 * (1.0 + b * kf * kf).sqrt()
        };
        if f > o.top_hz.min(18_000.0) {
            break;
        }
        // the sub-octave's own overtones matter only low down; above 2 kHz an organ chorus is its 8' row
        if o.sub && k % 2 == 1 && f > 2_000.0 {
            continue;
        }
        freqs.push(f);
        numbers.push(k);
    }
    let mut amp = vec![vec![0.0; g.len()]; freqs.len()];
    let mut sum = vec![vec![(0.0, 0.0); g.len()]; freqs.len()];
    let mut cents = vec![vec![0.0_f32; g.len()]; freqs.len()];
    for (k, &f) in freqs.iter().enumerate() {
        for (i, &t) in g.iter().enumerate() {
            let c = (t * SRF) as isize;
            let (a, z) = heterodyne(x, c, f, &w);
            let (_, z2) = heterodyne(x, c + ms, f, &w);
            amp[k][i] = a;
            sum[k][i] = z;
            // the sums are referred to sample 0, so an overtone off `f` by df turns by 2 pi df in a second
            let (cr, ci) = (z2.0 * z.0 + z2.1 * z.1, z2.1 * z.0 - z2.0 * z.1);
            let df = dmath::atan2(ci, cr) / dmath::TAU * SRF / ms as f64;
            cents[k][i] = if a > 0.0 {
                (1200.0 * dmath::log2((f + df) / f)) as f32
            } else {
                0.0
            };
        }
    }
    // the noise at each point
    let mut noise: Vec<[f64; BANDS]> = (0..g.len())
        .map(|i| {
            let c = (g[i] * SRF) as isize;
            let ov: Vec<(f64, (f64, f64))> = freqs.iter().enumerate().map(|(k, &f)| (f, sum[k][i])).collect();
            noise_frame(x, c, &ov, &fft, &edges)
        })
        .collect();
    let mut hiss = vec![f64::NEG_INFINITY; freqs.len()];
    if o.dehiss && dur > 1.0 {
        // the hiss: the median noise of the recording's last half second, per band, and what it reads as between
        // the overtones
        let tail: Vec<isize> = (0..10)
            .map(|j| ((dur - 0.5 + 0.05 * f64::from(j)) * SRF) as isize)
            .collect();
        let frames: Vec<[f64; BANDS]> = tail
            .iter()
            .map(|&c| {
                let ov: Vec<(f64, (f64, f64))> = freqs.iter().map(|&f| (f, heterodyne(x, c, f, &w).1)).collect();
                noise_frame(x, c, &ov, &fft, &edges)
            })
            .collect();
        let mut floor = [0.0; BANDS];
        for (bnd, fl) in floor.iter_mut().enumerate() {
            let mut v: Vec<f64> = frames.iter().map(|r| pow_db(r[bnd])).collect();
            v.sort_by(f64::total_cmp);
            *fl = v[v.len() / 2];
        }
        for row in &mut noise {
            for (v, fl) in row.iter_mut().zip(floor) {
                let p = pow_db(*v) - fl;
                *v = if p > 0.0 {
                    10.0 * dmath::log10(p)
                } else {
                    f64::NEG_INFINITY
                };
            }
        }
        for (k, &f) in freqs.iter().enumerate() {
            let mut v: Vec<f64> = tail
                .iter()
                .map(|&c| heterodyne(x, c, f + spacing / 2.0, &w).0)
                .collect();
            v.sort_by(f64::total_cmp);
            hiss[k] = db(v[v.len() / 2]);
        }
    }
    let top = amp.iter().flatten().copied().fold(0.0, f64::max);
    let top_db = db(top);
    let mut partials = Vec::new();
    for (k, &f) in freqs.iter().enumerate() {
        let lv: Vec<f64> = amp[k].iter().map(|&a| db(a)).collect();
        let keep = |i: usize| lv[i] >= top_db + o.floor_db && lv[i] >= hiss[k] + 6.0;
        let Some(first) = (0..g.len()).find(|&i| keep(i)) else {
            continue;
        };
        let last = (0..g.len()).rev().find(|&i| keep(i)).unwrap_or(first);
        partials.push(Partial {
            number: numbers[k] as u16,
            ratio: f / f0,
            first,
            level: lv[first..=last].iter().map(|&v| v as f32).collect(),
            cents: cents[k][first..=last].to_vec(),
        });
    }
    let rows = if o.dehiss {
        (0..noise.len())
            .rev()
            .find(|&i| noise[i].iter().any(|&v| v >= top_db - 90.0))
            .map_or(0, |i| i + 1)
    } else {
        noise.len()
    };
    Note {
        layer,
        midi: 69.0 + 12.0 * dmath::log2(f0 / 440.0),
        f0,
        b,
        release: 0.0,
        partials,
        noise: noise[..rows].iter().map(|r| r.map(|v| v as f32)).collect(),
    }
}

/// The fundamental's level over 20 ms windows every 5 ms, dB.
fn fundamental_track(x: &[f64], f0: f64) -> Vec<f64> {
    let w = blackman(((0.02 * SRF) as usize) | 1);
    let w = if 6.0 / f0 > 0.02 { window_for(f0) } else { w };
    (0..x.len() / 240)
        .map(|i| db(heterodyne(x, (i * 240) as isize, f0, &w).0))
        .collect()
}

/// How fast a note dies when its key comes up, as a time constant in seconds. With `from` (seconds after the onset:
/// a release recording, `Some(0.0)`), from the whole sound's level: the time it takes from 3 dB under its loudest in
/// the 30 ms after `from` to 20 dB under (or as deep as it gets above its own noise, at least 12 dB). Without, from
/// the fundamental's fall, fitted from 20 to 150 ms after the first moment past 0.3 s where it starts falling faster
/// than 40 dB a second and three times faster than over the 0.3 s before. `None` if there is no fall.
pub fn release(x: &[f64], f0: f64, from: Option<f64>) -> Option<f64> {
    let x = &x[onset(x)..];
    if let Some(t) = from {
        let env: Vec<f64> = rms_env(x).iter().map(|&v| db(v)).collect();
        let a = (t * SRF / 240.0) as usize;
        let (peak_i, peak) = env
            .iter()
            .enumerate()
            .skip(a)
            .take(6)
            .fold((a, f64::NEG_INFINITY), |m, (i, &v)| if v > m.1 { (i, v) } else { m });
        let floor = median_of(env.iter().rev().take(40).copied().collect());
        let depth = (peak - floor - 6.0).min(20.0);
        if !peak.is_finite() || depth < 12.0 {
            return None;
        }
        let cross = |d: f64| {
            env[peak_i..]
                .iter()
                .position(|&v| v < peak - d)
                .map(|i| (peak_i + i) as f64 * 240.0 / SRF)
        };
        let (t3, td) = (cross(3.0)?, cross(depth)?);
        return (td > t3).then(|| (td - t3) / ((depth - 3.0) / (20.0 * dmath::log10(std::f64::consts::E))));
    }
    let tr = fundamental_track(x, f0);
    let at = |t: f64| (t * SRF / 240.0) as usize;
    let start = match from {
        Some(t) => at(t),
        None => (60..tr.len().saturating_sub(6)).find(|&i| {
            let next = (tr[i + 6] - tr[i]) / 0.03;
            let before = (tr[i] - tr[i - 60]) / 0.3;
            next.is_finite() && before.is_finite() && next < -40.0 && next < 3.0 * before.min(0.0)
        })?,
    };
    let (a, z) = (start + 4, (start + 30).min(tr.len()));
    if z <= a + 3 {
        return None;
    }
    let pts: Vec<(f64, f64)> = (a..z)
        .filter(|&i| tr[i].is_finite())
        .map(|i| (i as f64 * 240.0 / SRF, tr[i]))
        .collect();
    let n = pts.len() as f64;
    if n < 4.0 {
        return None;
    }
    let (mx, my) = (
        pts.iter().map(|p| p.0).sum::<f64>() / n,
        pts.iter().map(|p| p.1).sum::<f64>() / n,
    );
    let sxx: f64 = pts.iter().map(|p| (p.0 - mx) * (p.0 - mx)).sum();
    let sxy: f64 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    let slope = sxy / sxx;
    // dB per second to a time constant: 20 log10(e) / tau
    (slope < -1.0).then(|| 20.0 * dmath::log10(std::f64::consts::E) / -slope)
}

/// The moments the yardstick reads a note's overtone levels at, seconds.
pub const YARD_TIMES: [f64; 5] = [0.03, 0.1, 0.3, 1.0, 2.5];

/// A note's yardstick numbers: what a real instrument's notes are checked against.
#[derive(Clone, Debug, PartialEq)]
pub struct Yard {
    /// The first 16 overtones at each of [`YARD_TIMES`], dB under the strongest at 30 ms (-120 for silence).
    pub levels: [[f64; 16]; 5],
    /// Seconds to 90% of the note's peak (5 ms RMS).
    pub start: f64,
    /// Seconds to fall 40 dB after the key comes up, when it does.
    pub stop: Option<f64>,
    /// The share of the energy that lies between the overtones, 0 to 100 ms.
    pub noise_early: f64,
    /// The same, 0.3 to 1 s.
    pub noise_steady: f64,
    /// The spectral centroid, 50 to 500 ms, Hz.
    pub bright_early: f64,
    /// The same, 0.8 to 1.2 s.
    pub bright_late: f64,
    /// The fundamental's slope from 0.3 to 2.5 s, dB per second.
    pub fall: f64,
}

fn median_of(mut v: Vec<f64>) -> f64 {
    v.retain(|x| x.is_finite());
    v.sort_by(f64::total_cmp);
    if v.is_empty() {
        f64::NEG_INFINITY
    } else {
        v[v.len() / 2]
    }
}

fn rms_env(x: &[f64]) -> Vec<f64> {
    x.chunks(240)
        .map(|c| (c.iter().map(|v| v * v).sum::<f64>() / c.len() as f64).sqrt())
        .collect()
}

fn centroid(x: &[f64], a: f64, z: f64) -> f64 {
    const N: usize = 1 << 15;
    let (s, e) = (((a * SRF) as usize).min(x.len()), ((z * SRF) as usize).min(x.len()));
    if e <= s + 64 {
        return 0.0;
    }
    let len = (e - s).min(N);
    let (mut re, mut im) = (vec![0.0; N], vec![0.0; N]);
    for i in 0..len {
        re[i] = x[s + i] * (0.5 - 0.5 * dmath::cos(dmath::TAU * i as f64 / len as f64));
    }
    Fft::new(N).forward(&mut re, &mut im);
    let bin = SRF / N as f64;
    let (mut num, mut den) = (0.0, 0.0);
    for k in ((20.0 / bin) as usize)..((16_000.0 / bin) as usize) {
        let p = re[k] * re[k] + im[k] * im[k];
        num += p * k as f64 * bin;
        den += p;
    }
    if den > 0.0 { num / den } else { 0.0 }
}

/// A note's [`Yard`]: `x` from its onset, `f0` and `b` its measured pitch and stiffness, `release_at` the moment its
/// key came up (seconds from the onset), if it did.
pub fn yard(x: &[f64], f0: f64, b: f64, release_at: Option<f64>) -> Yard {
    let x = &x[onset(x)..];
    let w = window_for(f0);
    let f = |k: usize| {
        let kf = k as f64;
        kf * f0 * (1.0 + b * kf * kf).sqrt()
    };
    let at = |t: f64| (t * SRF) as isize;
    let mut levels = [[-120.0; 16]; 5];
    let mut raw = [[0.0; 16]; 5];
    for (row, &t) in raw.iter_mut().zip(&YARD_TIMES) {
        for (k, v) in row.iter_mut().enumerate() {
            if f(k + 1) < 18_000.0 && at(t) < x.len() as isize {
                *v = heterodyne(x, at(t), f(k + 1), &w).0;
            }
        }
    }
    let top = raw[0].iter().copied().fold(0.0, f64::max);
    for j in 0..5 {
        for k in 0..16 {
            if raw[j][k] > 0.0 && top > 0.0 {
                levels[j][k] = (20.0 * dmath::log10(raw[j][k] / top)).max(-120.0);
            }
        }
    }
    let env = rms_env(x);
    let peak = env.iter().take(400).copied().fold(0.0, f64::max);
    let start = env.iter().position(|&v| v >= 0.9 * peak).unwrap_or(0) as f64 * 240.0 / SRF;
    let stop = release_at.and_then(|r| {
        let i = (r * SRF / 240.0) as usize;
        let base = *env.get(i)?;
        let j = env[i..].iter().position(|&v| v < base * 0.01)?;
        Some(j as f64 * 240.0 / SRF)
    });
    let edges = band_edges();
    let fft = Fft::new(2048);
    let share = |a: f64, z: f64| {
        let (mut nz, mut all) = (0.0, 0.0);
        let mut t = a;
        while t <= z && at(t) < x.len() as isize {
            let ks: Vec<usize> = (1..=64).take_while(|&k| f(k) < 18_000.0).collect();
            let ov: Vec<(f64, (f64, f64))> = ks.iter().map(|&k| (f(k), heterodyne(x, at(t), f(k), &w).1)).collect();
            let n: f64 = noise_frame(x, at(t), &ov, &fft, &edges)
                .iter()
                .map(|&v| pow_db(v))
                .sum();
            let s: f64 = noise_frame(x, at(t), &[], &fft, &edges)
                .iter()
                .map(|&v| pow_db(v))
                .sum();
            nz += n;
            all += s;
            t += 0.025;
        }
        if all > 0.0 { (nz / all).min(1.0) } else { 0.0 }
    };
    let tr: Vec<(f64, f64)> = grid(2.5)
        .into_iter()
        .filter(|&t| (0.3..=2.5).contains(&t) && at(t) < x.len() as isize)
        .map(|t| (t, db(heterodyne(x, at(t), f0, &w).0)))
        .filter(|p| p.1.is_finite())
        .collect();
    let fall = if tr.len() >= 3 {
        let n = tr.len() as f64;
        let (mx, my) = (
            tr.iter().map(|p| p.0).sum::<f64>() / n,
            tr.iter().map(|p| p.1).sum::<f64>() / n,
        );
        let sxx: f64 = tr.iter().map(|p| (p.0 - mx) * (p.0 - mx)).sum();
        tr.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum::<f64>() / sxx
    } else {
        0.0
    };
    Yard {
        levels,
        start,
        stop,
        noise_early: share(0.0, 0.1),
        noise_steady: share(0.3, 1.0),
        bright_early: centroid(x, 0.05, 0.5),
        bright_late: centroid(x, 0.8, 1.2),
        fall,
    }
}

impl Yard {
    /// The numbers as JSON.
    pub fn to_json(&self) -> Value {
        json!({
            "levels": self.levels.iter().map(|r| r.to_vec()).collect::<Vec<_>>(),
            "start": self.start, "stop": self.stop,
            "noise_early": self.noise_early, "noise_steady": self.noise_steady,
            "bright_early": self.bright_early, "bright_late": self.bright_late, "fall": self.fall,
        })
    }

    /// The numbers back from [`Yard::to_json`].
    pub fn from_json(v: &Value) -> Option<Yard> {
        let mut levels = [[0.0; 16]; 5];
        for (j, row) in v["levels"].as_array()?.iter().enumerate().take(5) {
            for (k, x) in row.as_array()?.iter().enumerate().take(16) {
                levels[j][k] = x.as_f64()?;
            }
        }
        Some(Yard {
            levels,
            start: v["start"].as_f64()?,
            stop: v["stop"].as_f64(),
            noise_early: v["noise_early"].as_f64()?,
            noise_steady: v["noise_steady"].as_f64()?,
            bright_early: v["bright_early"].as_f64()?,
            bright_late: v["bright_late"].as_f64()?,
            fall: v["fall"].as_f64()?,
        })
    }
}

/// The engine 2 note the yardstick measures: `inst` at `midi`, struck or registered at `strength` (a piano's
/// velocity; an organ's registration, 0.5 the quiet chorus and 1 the loud), the key held `hold` seconds, `secs` long,
/// as repeat `variant`.
pub fn engine2_note(inst: &Instrument, midi: f64, strength: f64, hold: f64, secs: f64, variant: u64) -> Vec<f64> {
    let play = match inst.parts {
        Parts::Organ(_) | Parts::Pipes(_) => Play {
            swell: Some(Swell {
                from: strength,
                to: strength,
                over: 0.0,
            }),
            ..Play::plain(midi, hold, 0.8)
        },
        _ => Play::plain(midi, hold, strength),
    };
    engine2::play(inst, &play, secs, variant)
}

/// How many of [`YARD_TIMES`] come before a piano's strings have beaten apart (30, 100 and 300 ms): a single
/// overtone is compared there; later only the overtones' median is, as two or three strings of one note beat in and
/// out of step and a beat's low point falls wherever it falls.
pub const YARD_EARLY: usize = 3;

/// How far a note's [`Yard`] is from its recording's: the tone at each of [`YARD_TIMES`], each overtone against the
/// strongest at that moment (how loud the whole note is then is the start's, the stop's and the fall's business), over
/// the overtones the recording has within 50 dB of its strongest there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct YardDiff {
    /// The farthest one overtone is from the recording's at any of the first [`YARD_EARLY`] times, dB.
    pub early_db: f64,
    /// Which (time index, overtone number) that was.
    pub early_at: (usize, usize),
    /// The farthest the overtones' median difference is from 0 at any of [`YARD_TIMES`], dB.
    pub median_db: f64,
    /// The same from 0.3 s on (the held tone), dB.
    pub late_median_db: f64,
    /// The farthest one overtone of the held tone is, its power averaged over 0.3, 1 and 2.5 s (so a beat's low point
    /// does not count as the tone), dB.
    pub steady_db: f64,
    /// The farthest one overtone is at any time (beats included), dB: for the record.
    pub any_db: f64,
    /// The start, ours less the recording's, seconds.
    pub start_s: f64,
    /// The noise shares, ours less the recording's.
    pub noise_early: f64,
    pub noise_steady: f64,
}

/// `ours` against `real` ([`YardDiff`]).
pub fn yard_diff(real: &Yard, ours: &Yard) -> YardDiff {
    let (mut early, mut at, mut median, mut any) = (0.0_f64, (0, 0), 0.0_f64, 0.0_f64);
    let mut late_median = 0.0_f64;
    let top = |row: &[f64; 16]| row.iter().copied().fold(-120.0, f64::max);
    for j in 0..YARD_TIMES.len() {
        let (rt, ot) = (top(&real.levels[j]), top(&ours.levels[j]));
        if rt <= -119.0 {
            continue;
        }
        let mut ds = Vec::new();
        for k in 0..16 {
            if real.levels[j][k] > rt - 50.0 {
                let d = ((real.levels[j][k] - rt) - (ours.levels[j][k] - ot)).abs();
                ds.push(d);
                any = any.max(d);
                if j < YARD_EARLY && d > early {
                    (early, at) = (d, (j, k + 1));
                }
            }
        }
        if !ds.is_empty() {
            let m = median_of(ds);
            median = median.max(m);
            if j >= YARD_EARLY {
                late_median = late_median.max(m);
            }
        }
    }
    // the held tone: each overtone's power averaged over the late times, against the strongest of those
    let held = |y: &Yard| -> [f64; 16] {
        let mut h = [0.0; 16];
        for (k, v) in h.iter_mut().enumerate() {
            let p: f64 = (YARD_EARLY..YARD_TIMES.len())
                .map(|j| dmath::exp(y.levels[j][k] * std::f64::consts::LN_10 / 10.0))
                .sum::<f64>()
                / (YARD_TIMES.len() - YARD_EARLY) as f64;
            *v = 10.0 * dmath::log10(p.max(1e-12));
        }
        let top = h.iter().copied().fold(-120.0, f64::max);
        h.map(|v| v - top)
    };
    let (hr, ho) = (held(real), held(ours));
    let steady = (0..16)
        .filter(|&k| hr[k] > -50.0)
        .map(|k| (hr[k] - ho[k]).abs())
        .fold(0.0, f64::max);
    YardDiff {
        early_db: early,
        early_at: at,
        median_db: median,
        late_median_db: late_median,
        steady_db: steady,
        any_db: any,
        start_s: ours.start - real.start,
        noise_early: ours.noise_early - real.noise_early,
        noise_steady: ours.noise_steady - real.noise_steady,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bf_core::rng::Generator;

    fn tone(secs: f64, f0: f64, b: f64, parts: usize, level: impl Fn(usize, f64) -> f64) -> Vec<f64> {
        let n = (secs * SRF) as usize;
        let mut x = vec![0.0; n];
        for k in 1..=parts {
            let kf = k as f64;
            let f = kf * f0 * (1.0 + b * kf * kf).sqrt();
            for (i, v) in x.iter_mut().enumerate() {
                let t = i as f64 / SRF;
                *v += level(k, t) * dmath::sin(dmath::TAU * (f * t).fract());
            }
        }
        x
    }

    fn opts() -> Opts {
        Opts {
            stretched: true,
            sub: false,
            max_partials: 20,
            end: 2.0,
            floor_db: -90.0,
            dehiss: false,
            top_hz: 18_000.0,
        }
    }

    #[test]
    fn the_pitch_and_stretch_are_found() {
        let x = tone(2.0, 261.63, 0.0004, 20, |k, _| 0.3 / k as f64);
        let (f0, b) = pitch(&x, 261.63, true);
        assert!((f0 - 261.63).abs() < 0.05, "{f0}");
        assert!((b - 0.0004).abs() < 2e-5, "{b}");
        let (f0, b) = pitch(&tone(1.0, 220.0, 0.0, 12, |k, _| 0.3 / k as f64), 221.0, false);
        assert!((f0 - 220.0).abs() < 0.05 && b == 0.0, "{f0} {b}");
    }

    #[test]
    fn a_partials_fall_is_tracked() {
        // partial 3 falls 10 dB a second; partial 5 sits 1.5 Hz sharp of the 5th harmonic
        let f0 = 220.0;
        let mut x = tone(2.0, f0, 0.0, 4, |k, t| {
            if k == 3 {
                0.2 * dmath::pow(10.0, -10.0 * t / 20.0)
            } else {
                0.1
            }
        });
        for (i, v) in x.iter_mut().enumerate() {
            let t = i as f64 / SRF;
            *v += 0.05 * dmath::sin(dmath::TAU * ((5.0 * f0 + 1.5) * t).fract());
        }
        let n = measure(
            &x,
            57.0,
            0,
            &Opts {
                stretched: false,
                ..opts()
            },
        );
        let p3 = &n.partials[2];
        let g = grid(2.0);
        let i = |t: f64| g.iter().position(|&v| v >= t).unwrap();
        let fall = p3.level_at(i(0.3)) - p3.level_at(i(1.3));
        let dt = g[i(1.3)] - g[i(0.3)];
        assert!((f64::from(fall) - 10.0 * dt).abs() < 0.3, "{fall} over {dt}");
        let want = 1200.0 * dmath::log2((5.0 * f0 + 1.5) / (5.0 * n.f0));
        let got = f64::from(n.partials[4].cents_at(i(1.0)));
        assert!((got - want).abs() < 0.3, "{got} {want}");
    }

    #[test]
    fn noise_between_the_partials_is_measured_and_the_hiss_removed() {
        let mut x = tone(2.5, 330.0, 0.0, 6, |_, _| 0.1);
        let mut g = Generator::new(5);
        for (i, v) in x.iter_mut().enumerate() {
            let burst = if i < 4800 { 0.1 } else { 0.0 };
            *v += (burst + 0.0003) * g.standard_normal();
        }
        let n = measure(
            &x,
            64.0,
            0,
            &Opts {
                stretched: false,
                dehiss: true,
                end: 2.0,
                ..opts()
            },
        );
        let g = grid(2.0);
        let early: f64 = n.noise[g.iter().position(|&t| t >= 0.05).unwrap()][12].into();
        let late = n
            .noise
            .get(g.iter().position(|&t| t >= 1.0).unwrap())
            .map_or(f64::NEG_INFINITY, |r| r[12].into());
        assert!(early - late > 30.0, "{early} {late}");
    }

    #[test]
    fn a_release_is_found() {
        let x = tone(2.5, 200.0, 0.0, 3, |_, t| {
            if t < 1.5 {
                0.2
            } else {
                0.2 * dmath::exp(-(t - 1.5) / 0.08)
            }
        });
        let tau = release(&x, 200.0, None).expect("a fall");
        assert!((tau - 0.08).abs() < 0.01, "{tau}");
    }

    #[test]
    fn the_yardstick_reads_a_known_note() {
        let mut x = tone(2.0, 196.0, 0.0, 16, |k, _| 0.3 / k as f64);
        for (i, v) in x.iter_mut().take(240).enumerate() {
            *v *= i as f64 / 240.0;
        }
        let y = yard(&x, 196.0, 0.0, None);
        for k in 0..16 {
            let want = -20.0 * dmath::log10((k + 1) as f64);
            assert!((y.levels[3][k] - want).abs() < 0.3, "{k}: {} {want}", y.levels[3][k]);
        }
        assert!(y.start < 0.015, "{}", y.start);
        assert_eq!(Yard::from_json(&y.to_json()), Some(y));
    }
}
