//! CLARINET's instrument voice: `pp_bridge.voice('CLARINET', midi, vel, dur)`, and the reed synth behind it, which
//! its part plays directly.
//!
//! `pp_bridge` plays one note through `series/instrument-history/clarinet60_lowpoly.py`'s `synth` on the scene its
//! roster selects for the CLARINET, the module's first: the modern clarinet (key `modern`). That is `reed` with the
//! tone `TONES['modern']` (`bright=.6, vib=.003`, every other argument at its default), its even harmonics let in a
//! little more from A4 up (the clarion register: `odd` times 0.65 when `midi + 2 >= 71`), and, because the scene is
//! the modern one, two echoes of the whole note, 0.75 of a beat of that video (120 BPM) and twice that later, at
//! 0.25 and 0.1 of its level. The note is made at 44,100 Hz and resampled to 48,000 Hz.
//!
//! `reed` is a sum of up to 39 harmonics of a slightly scooped, slowly vibrating pitch: the even ones held down
//! (`odd`: a stopped pipe), a formant bump, and two spectra, a dark one and a bright one, mixed by the note's own
//! loudness. It draws no noise. Its `gl` argument (a glide) is not ported: no scene of the module has one and the
//! kit's part never passes it.
//!
//! The kit's CLARINET part does not go through `pp_bridge.voice`: it calls `reed` itself with other tones (`boehm`,
//! `goodman`) and fractional pitches ([`reed`], [`BOEHM`], [`GOODMAN`]; `sound.rs`).

use bf_core::dmath::{self, PI};
use bf_sound::dsp;
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::{PpNoise, Voice};

/// The CLARINET's register in `pp_bridge`'s roster (`('CLARINET', 'clarinet60_lowpoly', None, (55, 79), ...)`).
pub const REGISTER: (i32, i32) = (55, 79);

/// `clarinet60_lowpoly.BPM`: the tempo of the clarinet's video, which sets the echoes' delay.
const BPM: f64 = 120.0;

/// `reed`'s keyword arguments: a tone of `clarinet60_lowpoly.TONES` over the function's defaults.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tone {
    /// `bright`: how slowly the bright spectrum's harmonics fall off.
    pub bright: f64,
    /// `vib`: the vibrato's depth, as a fraction of the frequency.
    pub vib: f64,
    /// `vrate`: the vibrato's rate, in Hz.
    pub vrate: f64,
    /// `odd`: how far the even harmonics are held down (1 would leave a tenth of them).
    pub odd: f64,
    /// `scoop`: how far below pitch the note starts, as a fraction of the frequency.
    pub scoop: f64,
    /// `att`: the attack, in seconds.
    pub att: f64,
    /// `formant`: the centre of the formant bump, in Hz.
    pub formant: f64,
    /// `fboost`: the height of the formant bump.
    pub fboost: f64,
}

/// `reed`'s defaults: `bright=.6, vib=.004, vrate=5.5, odd=.8, scoop=.02, att=.03, formant=1500., fboost=.8`.
pub const DEFAULT: Tone = Tone {
    bright: 0.6,
    vib: 0.004,
    vrate: 5.5,
    odd: 0.8,
    scoop: 0.02,
    att: 0.03,
    formant: 1500.0,
    fboost: 0.8,
};

/// `TONES["modern"]`: `dict(bright=.6, vib=.003)`, the scene `pp_bridge` plays.
pub const MODERN: Tone = Tone { vib: 0.003, ..DEFAULT };

/// `TONES["boehm"]`: `dict(bright=.45, vib=.002, odd=.85)`, the part's own tone.
pub const BOEHM: Tone = Tone {
    bright: 0.45,
    vib: 0.002,
    odd: 0.85,
    ..DEFAULT
};

/// `TONES["goodman"]`: `dict(bright=.8, vib=.004, vrate=5.8, odd=.65)`, SQUEAK's squeal.
pub const GOODMAN: Tone = Tone {
    bright: 0.8,
    vib: 0.004,
    vrate: 5.8,
    odd: 0.65,
    ..DEFAULT
};

/// The CLARINET's Pixel Pitch voice.
#[derive(Clone, Copy, Debug, Default)]
pub struct ClarinetVoice;

impl Voice for ClarinetVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "CLARINET"
    }

    /// Down to -0.14 s: the tone is `int((dur + .14) * SR)` samples, and numpy refuses a negative count.
    fn can_hold(&self, dur: f64) -> bool {
        pp::has_len(dur + 0.14)
    }
}

/// `reed(f, dur, v, **tone)`: the reed tone at `f` Hz held for `dur` seconds at velocity `v`: `int((dur + .14) *
/// SR)` samples at 44,100 Hz. Above 8 kHz no harmonic is left and the samples are not numbers (0 / 0), as in Python.
///
/// # Panics
/// Where Python raises: `dur` is not a number, or below -0.14 s (a negative number of samples).
pub fn reed(f: f64, dur: f64, v: f64, tone: &Tone) -> Vec<f64> {
    let n = pp::len(dur + 0.14);
    let t = pp::tt(n);
    // env = np.minimum(1, t/att)**1.3 * np.where(t < dur, 1, np.exp(-(t - dur)/.04)) * (.9 + .1*np.exp(-t*4))
    let env: Vec<f64> = t
        .iter()
        .map(|&t| {
            let rise = if t / tone.att < 1.0 { t / tone.att } else { 1.0 };
            let hold = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.04) };
            dmath::pow(rise, 1.3) * hold * (0.9 + 0.1 * dmath::exp(-t * 4.0))
        })
        .collect();
    // fr = f*(1 - scoop*np.exp(-t/.04)) * (1 + vib*np.sin(2*np.pi*vrate*t)*np.clip((t - .15)/.25, 0, 1))
    let wobble = 2.0 * PI * tone.vrate;
    let fr: Vec<f64> = t
        .iter()
        .map(|&t| {
            let scooped = f * (1.0 - tone.scoop * dmath::exp(-t / 0.04));
            scooped * (1.0 + tone.vib * dmath::sin(wobble * t) * ((t - 0.15) / 0.25).clamp(0.0, 1.0))
        })
        .collect();
    // ph = 2*np.pi*np.cumsum(fr)/SR
    let ph: Vec<f64> = dsp::cumsum(&fr).iter().map(|&c| 2.0 * PI * c / SRF).collect();
    let (mut dark, mut brt) = (vec![0.0; n], vec![0.0; n]);
    let (mut sd, mut sb) = (0.0, 0.0);
    for k in 1..40 {
        let kf = f64::from(k);
        let fk = f * kf;
        if fk > 8000.0 {
            break;
        }
        // base = (1/k**1.1)*((1 - odd*.9) if k % 2 == 0 else 1)*(1 + fboost*math.exp(-((fk - formant)/900)**2))
        let even = if k % 2 == 0 { 1.0 - tone.odd * 0.9 } else { 1.0 };
        let q = (fk - tone.formant) / 900.0;
        let base = (1.0 / dmath::pow(kf, 1.1)) * even * (1.0 + tone.fboost * dmath::exp(-(q * q)));
        let a1 = base * dmath::exp(-kf / 2.5);
        let a2 = base * dmath::exp(-kf / (4.0 + 8.0 * tone.bright));
        for i in 0..n {
            let s = dmath::sin(kf * ph[i]);
            dark[i] += a1 * s;
            brt[i] += a2 * s;
        }
        sd += a1;
        sb += a2;
    }
    // mb = np.clip(env*v, 0, 1)**1.2; return (dark/sd*(1 - mb) + brt/sb*mb)*env*v
    (0..n)
        .map(|i| {
            let mb = dmath::pow((env[i] * v).clamp(0.0, 1.0), 1.2);
            (dark[i] / sd * (1.0 - mb) + brt[i] / sb * mb) * env[i] * v
        })
        .collect()
}

/// `clarinet60_lowpoly.synth` for the one note `pp_bridge.voice` asks of the modern scene: the note at 44,100 Hz,
/// before it is resampled. `midi` is a whole number.
///
/// # Panics
/// As [`reed`].
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    // if m + 2 >= 71 and key not in ("chal", "bass"): tone["odd"] = tone.get("odd", .8)*.65: the clarion register
    let mut tone = MODERN;
    if midi + 2.0 >= 71.0 {
        tone.odd *= 0.65;
    }
    let x = reed(pp::midi_hz(midi), dur, vel, &tone);
    // The modern scene's echoes: dl = int(.75 * BEAT_S * SR), then the note again .25 and .1 as loud.
    let dl = pp::len(0.75 * (60.0 / BPM));
    let mut y = vec![0.0; x.len() + 2 * dl];
    // y[:len(x)] += x; y[dl:dl + len(x)] += .25*x; y[2*dl:2*dl + len(x)] += .1*x
    for (delay, level) in [(0, 1.0), (dl, 0.25), (2 * dl, 0.1)] {
        for (o, &v) in y[delay..].iter_mut().zip(&x) {
            *o += level * v;
        }
    }
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_echoes_are_three_quarters_of_a_beat_apart() {
        let dl = pp::len(0.75 * (60.0 / BPM));
        assert_eq!(dl, 16537);
        let y = synth(62.0, 0.8, 0.12);
        // int((0.12 + .14) * 44100) samples of tone, and room for the two echoes
        assert_eq!(y.len(), 11466 + 2 * dl);
        assert!(y[11466..dl].iter().all(|v| *v == 0.0));
        for i in [1, 500, 11465] {
            assert_eq!(y[dl + i], 0.25 * y[i], "sample {i}");
            assert_eq!(y[2 * dl + i], 0.1 * y[i], "sample {i}");
        }
    }

    #[test]
    fn the_tone_has_its_pitch_its_attack_and_its_release() {
        let x = reed(440.0, 0.5, 1.0, &BOEHM);
        assert_eq!(x.len(), pp::len(0.64));
        // it starts from silence (the attack) and draws no noise: the same note twice is the same samples
        assert_eq!(x[0], 0.0);
        assert_eq!(x, reed(440.0, 0.5, 1.0, &BOEHM));
        // 440 Hz: 176 periods in 0.4 s after the attack, each with at least one upward zero crossing
        let ups = x[4410..22050].windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        assert!(ups >= 174 && ups % 176 < 3, "{ups}");
        // after the held length it dies away within the 0.14 s it has left
        let peak = |a: &[f64]| a.iter().fold(0.0f64, |p, v| p.max(v.abs()));
        assert!(peak(&x[x.len() - 441..]) < 0.1 * peak(&x[8820..17640]));
        // the three tones differ
        assert_ne!(reed(440.0, 0.2, 0.9, &BOEHM), reed(440.0, 0.2, 0.9, &GOODMAN));
        assert_ne!(reed(440.0, 0.2, 0.9, &BOEHM), reed(440.0, 0.2, 0.9, &MODERN));
    }

    #[test]
    fn the_clarion_register_lets_the_even_harmonics_in() {
        // midi 68: odd stays 0.8; midi 69 (m + 2 >= 71): odd is 0.8 * 0.65
        let low = synth(68.0, 0.8, 0.1);
        assert_eq!(low[..300], reed(pp::midi_hz(68.0), 0.1, 0.8, &MODERN)[..300]);
        let high = synth(69.0, 0.8, 0.1);
        let clarion = Tone {
            odd: 0.8 * 0.65,
            ..MODERN
        };
        assert_eq!(high[..300], reed(pp::midi_hz(69.0), 0.1, 0.8, &clarion)[..300]);
        assert_ne!(high[..300], reed(pp::midi_hz(69.0), 0.1, 0.8, &MODERN)[..300]);
    }

    #[test]
    fn the_voice_is_the_synth_at_48k() {
        let mut noise = PpNoise::start();
        let note = ClarinetVoice.note(&mut noise, 67.0, 0.9, 0.25);
        let raw = synth(67.0, 0.9, 0.25);
        assert_eq!(note, pp::to_48k(&raw));
        assert_eq!(note.len(), (raw.len() * 160).div_ceil(147));
        assert_eq!((ClarinetVoice.register(), ClarinetVoice.name()), (REGISTER, "CLARINET"));
        // it drew nothing from the shared noise
        assert_eq!(noise.nz(4), PpNoise::start().nz(4));
        assert!(ClarinetVoice.can_hold(0.0) && ClarinetVoice.can_hold(-0.14));
        assert!(!ClarinetVoice.can_hold(-0.15) && !ClarinetVoice.can_hold(f64::NAN));
        // far above the instrument no harmonic is under 8 kHz: the samples are not numbers (Python's are not either)
        assert!(reed(9000.0, 0.05, 0.8, &MODERN).iter().skip(1).all(|v| v.is_nan()));
    }

    #[test]
    #[should_panic(expected = "negative dimensions are not allowed")]
    fn a_note_cannot_end_before_it_starts() {
        synth(62.0, 0.8, -0.2);
    }
}
