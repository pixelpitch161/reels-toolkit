//! CELLO's voice: `pp_bridge.voice('CELLO', midi, vel, dur)`.
//!
//! `pp_bridge` plays one note through `series/instrument-history/cello60_lowpoly.py`'s `synth` on the scene its
//! roster selects for the CELLO, the module's first: the modern cello (key `cmodern`, which has no drones). That is
//! `violin_lowpoly.bowed` (ported here: only the CELLO plays it in this milestone) with the tone
//! `cello_lowpoly.CTONE['cmodern']` that `cello_lowpoly` adds to `violin_lowpoly.TONE` on import:
//! `tilt=1.0, form=[(220, 1.0, .5), (600, .7, .4), (2000, 1.0, .45)], vib=.0055, vdel=.1, att=.04`. The note is made
//! at 44,100 Hz and resampled to 48,000 Hz.
//!
//! The voice draws one number a note from `violin_lowpoly.rng` (`default_rng(9)`, the shared `PpNoise`'s
//! `ModuleNoise::violin`): the vibrato's phase.

use bf_core::dmath::{self, PI};
use bf_core::py::py_max;
use bf_sound::dsp;
use bf_sound::voice::pp;
use bf_sound::voice::{PpNoise, Voice};

/// The CELLO's register in `pp_bridge`'s roster (`('CELLO', 'cello60_lowpoly', None, (40, 64), 0, 1.0)`).
/// `kit.music.inst_gain` measures the voice on the middle of it.
pub const REGISTER: (i32, i32) = (40, 64);

/// A tone of `violin_lowpoly.TONE`: what `bowed` reads for a tone without `slide`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tone {
    /// The harmonics' roll-off, `1 / k ** tilt`.
    pub tilt: f64,
    /// The formants: `(centre Hz, gain, width in natural-log units)`.
    pub form: [(f64, f64, f64); 3],
    /// The vibrato's depth (a fraction of the pitch).
    pub vib: f64,
    /// Seconds before the vibrato starts to fade in (over 0.25 s).
    pub vdel: f64,
    /// The attack in seconds.
    pub att: f64,
}

/// `CTONE['cmodern']`.
pub const CMODERN: Tone = Tone {
    tilt: 1.0,
    form: [(220.0, 1.0, 0.5), (600.0, 0.7, 0.4), (2000.0, 1.0, 0.45)],
    vib: 0.0055,
    vdel: 0.1,
    att: 0.04,
};

/// `np.clip(x, 0, 1)`: numpy's rule (a NaN stays NaN).
fn clip01(x: f64) -> f64 {
    let x = if x < 0.0 { 0.0 } else { x };
    if x > 1.0 { 1.0 } else { x }
}

/// `np.minimum(1, x)`: 1 where `x` is larger, `x` otherwise (a NaN stays NaN).
fn min1(x: f64) -> f64 {
    if x.is_nan() || x < 1.0 { x } else { 1.0 }
}

/// `violin_lowpoly.bowed(f, dur, vel, kind)` (not a drone), at 44,100 Hz, with `phase` the generator's draw
/// (`rng.random()`):
///
/// ```python
/// p = TONE[kind]; rel = .15
/// n = int((dur + rel)*SR); t = _t(n)
/// vib = p["vib"]*np.clip((t - p["vdel"])/.25, 0, 1)*np.sin(2*np.pi*5.7*t + rng.random()*6)
/// fm = 1 + vib
/// ph = 2*np.pi*np.cumsum(f*fm)/SR
/// sig = np.zeros(n); tot = 0
/// for k in range(1, 40):
///     fk = f*k
///     if fk > 6000: break
///     gk = (.25 + sum(a*math.exp(-(math.log(fk/fc)/w)**2) for fc, a, w in p["form"])) / k**p["tilt"]
///     sig += gk*np.sin(k*ph); tot += gk
/// sig /= tot
/// env = np.minimum(1, t/p["att"]) * np.where(t < dur, 1.0, np.exp(-(t - dur)/(rel/4)))
/// env = env*(.88 + .12*np.minimum(1, t/max(dur, .05)))
/// sig = lfilter([.35], [1, -.65], sig)
/// return sig*env*vel
/// ```
///
/// # Panics
/// Where Python raises: `dur` is not a number, or so far below -0.15 s that the note has a negative number of
/// samples.
pub fn bowed(f: f64, dur: f64, vel: f64, p: &Tone, phase: f64) -> Vec<f64> {
    let rel = 0.15;
    let n = pp::len(dur + rel);
    let t = pp::tt(n);
    let w = 2.0 * PI * 5.7;
    let off = phase * 6.0;
    // f * fm, fm = 1 + vib
    let ffm: Vec<f64> = t
        .iter()
        .map(|&t| {
            let vib = p.vib * clip01((t - p.vdel) / 0.25) * dmath::sin(w * t + off);
            f * (1.0 + vib)
        })
        .collect();
    let ph: Vec<f64> = dsp::cumsum(&ffm).iter().map(|&c| 2.0 * PI * c / pp::SRF).collect();
    let mut sig = vec![0.0; n];
    let mut tot = 0.0;
    for k in 1..40 {
        let kf = f64::from(k);
        let fk = f * kf;
        if fk > 6000.0 {
            break;
        }
        // sum(...) starts from the int 0, which adds nothing to the first term
        let mut s = 0.0;
        for &(fc, a, wd) in &p.form {
            s += a * dmath::exp(-dmath::pow(dmath::ln(fk / fc) / wd, 2.0));
        }
        let gk = (0.25 + s) / dmath::pow(kf, p.tilt);
        for (v, &x) in sig.iter_mut().zip(&ph) {
            *v += gk * dmath::sin(kf * x);
        }
        tot += gk;
    }
    for v in &mut sig {
        *v /= tot;
    }
    let sig = dsp::lfilter(&[0.35], &[1.0, -0.65], &sig);
    let hold = py_max(dur, 0.05);
    sig.iter()
        .zip(&t)
        .map(|(&s, &t)| {
            let tail = if t < dur {
                1.0
            } else {
                dmath::exp(-(t - dur) / (rel / 4.0))
            };
            let env = min1(t / p.att) * tail;
            let env = env * (0.88 + 0.12 * min1(t / hold));
            s * env * vel
        })
        .collect()
}

/// `cello60_lowpoly.synth` for the one note `pp_bridge.voice` asks of the modern scene (no drones: `cmodern` is
/// not in `DRONES`), at 44,100 Hz before it is resampled: `V.bowed(midi_hz(m), dur, v, 'cmodern')`.
pub fn synth(noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    let phase = noise.1.violin.random();
    bowed(pp::midi_hz(midi), dur, vel, &CMODERN, phase)
}

/// The CELLO's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct CelloVoice;

impl Voice for CelloVoice {
    fn note(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(noise, midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "CELLO"
    }

    /// Down to -0.15 s: the tone is `int((dur + .15) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.15)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_draws_one_number_from_the_violin_generator() {
        let mut noise = PpNoise::start();
        let note = CelloVoice.note(&mut noise, 52.0, 0.85, 0.25);
        let mut fresh = PpNoise::start();
        let phase = fresh.1.violin.random();
        assert_eq!(note, pp::to_48k(&bowed(pp::midi_hz(52.0), 0.25, 0.85, &CMODERN, phase)));
        assert_eq!(noise.1.violin.random().to_bits(), fresh.1.violin.random().to_bits());
        // the shared stream is left where it was
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        // int((dur + .15) * 44100) samples
        assert_eq!(bowed(220.0, 0.25, 0.8, &CMODERN, 0.5).len(), 17640);
        assert!(CelloVoice.can_hold(-0.15) && !CelloVoice.can_hold(-0.2) && !CelloVoice.can_hold(f64::NAN));
        assert_eq!((CelloVoice.register(), CelloVoice.name()), (REGISTER, "CELLO"));
    }

    #[test]
    fn the_first_sample_is_silent() {
        let x = bowed(110.0, 0.3, 0.9, &CMODERN, 0.25);
        assert_eq!(x[0], 0.0);
        assert!(x.iter().all(|v| v.is_finite()));
    }
}
