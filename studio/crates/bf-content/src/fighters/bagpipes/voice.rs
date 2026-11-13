//! BAGPIPES's instrument: the Great Highland pipe of `series/instrument-history/bagpipes60_lowpoly.py`, its sound half
//! (`wt`, `chanter`, `drone_voice`, `synth`), at 44,100 Hz.
//!
//! Two users: [`BagpipesVoice`] is `pp_bridge.voice('BAGPIPES', midi, vel, dur)` (the roster's entry is the module's
//! first scene, the Great Highland bagpipe, key `ghb`, 9.6 s long): `synth` on one made-up event at index 0, so every
//! note is the chanter's note with its high-G grace and, as the scene's first event, the three drones striking in and
//! holding the whole scene (`drone_voice('ghb', 9.6 + 0.03)`), resampled to 48,000 Hz. The part (`sound.rs`) does not
//! play this voice: it calls [`chanter`], [`wt`] and the drones' table directly, as #30's `audio.py` did.
//!
//! `wt` keeps its tables in a cache keyed by `int(f * 4)` and the timbre, so a frequency within a quarter of a hertz
//! of one met before would get that one's table. Every frequency BAGPIPES plays is a whole MIDI note (or one of the
//! drones' three, each in a bucket of its own), so two different frequencies never share a key and the cache is a
//! pure function's: the port computes each table once per exact frequency.
//!
//! The noise is each note's own: `np.random.default_rng(seed)`, one `random()` for the chanter's phase (`seed = 7 idx
//! + m`, here `m`), three for the drones' phases (`default_rng(len('ghb'))`). The series' shared noise stream is not
//! touched.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use bf_core::dmath::{self, PI};
use bf_core::py::{fmod_py, py_max, py_min};
use bf_core::rng::Generator;
use bf_sound::dsp;
use bf_sound::voice::pp::{self, SRF};
use bf_sound::voice::{PpNoise, Voice};

/// BAGPIPES's register in `pp_bridge`'s roster (`('BAGPIPES', 'bagpipes60_lowpoly', None, (67, 81), ...)`).
pub const REGISTER: (i32, i32) = (67, 81);

/// A chanter's timbre (`TONES[key]`; the Highland pipe's has no `vib`, `detune`, `breath`, `lpf` or `stacc`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tone {
    /// The harmonics' fall, `k ** -tilt`.
    pub tilt: f64,
    /// The formant's centre (Hz).
    pub form: f64,
    /// The formant's width (Hz).
    pub fbw: f64,
    /// The formant's gain.
    pub fgain: f64,
    /// How much the even harmonics are cut.
    pub odd: f64,
    /// The attack (s).
    pub attack: f64,
    /// The release (s).
    pub rel: f64,
}

/// `HIGHLAND` (`TONES['ghb']`): the Great Highland pipe's chanter.
pub const HIGHLAND: Tone = Tone {
    tilt: 0.75,
    form: 2400.0,
    fbw: 1100.0,
    fgain: 2.2,
    odd: 0.0,
    attack: 0.006,
    rel: 0.014,
};

/// `DRONES['ghb']`: the three drones, their timbre, level and low-pass.
pub struct Drones {
    /// `(midi, amp, cents)` of each drone.
    pub p: [(f64, f64, f64); 3],
    /// The harmonics' fall.
    pub tilt: f64,
    /// The formant's centre (Hz).
    pub form: f64,
    /// The formant's width (Hz).
    pub fbw: f64,
    /// The formant's gain.
    pub fgain: f64,
    /// How much the even harmonics are cut.
    pub odd: f64,
    /// The level.
    pub v: f64,
    /// The low-pass (Hz).
    pub lpf: f64,
}

/// `DRONES['ghb']`.
pub const GHB_DRONES: Drones = Drones {
    p: [(57.0, 1.0, -1.5), (57.0, 1.0, 1.5), (45.0, 1.1, 0.0)],
    tilt: 0.9,
    form: 900.0,
    fbw: 600.0,
    fgain: 1.2,
    odd: 0.55,
    v: 0.2,
    lpf: 3600.0,
};

/// The size of a wavetable.
pub const N_WT: usize = 2048;

/// `_PH = np.random.default_rng(5).random(80) * 2 * np.pi`: each harmonic's starting phase.
fn phases() -> &'static [f64; 80] {
    static PH: OnceLock<[f64; 80]> = OnceLock::new();
    PH.get_or_init(|| {
        let mut g = Generator::new(5);
        let mut out = [0.0; 80];
        for p in &mut out {
            *p = g.random() * 2.0 * PI;
        }
        out
    })
}

type WtKey = (u64, u64, u64, u64, u64, u64);

/// `wt(f, tilt, form, fbw, fgain, odd)`: a 2,048-sample wavetable of up to 64 harmonics (none above 11 kHz), each
/// `k ** -tilt` with a formant bump, the even ones cut by `odd`, at the phases `_PH`, normalised to a peak of 1.
pub fn wt(f: f64, tilt: f64, form: f64, fbw: f64, fgain: f64, odd: f64) -> Arc<Vec<f64>> {
    static MEMO: OnceLock<Mutex<HashMap<WtKey, Arc<Vec<f64>>>>> = OnceLock::new();
    let key = (f.to_bits(), tilt.to_bits(), form.to_bits(), fbw.to_bits(), fgain.to_bits(), odd.to_bits());
    let memo = MEMO.get_or_init(Default::default);
    if let Some(tb) = memo.lock().expect("the wavetable memo").get(&key) {
        return tb.clone();
    }
    // K = int(max(1, min(64, 11000 / f)))
    let kk = py_max(1.0, py_min(64.0, 11000.0 / f)) as usize;
    let ph = phases();
    let mut tb = vec![0.0; N_WT];
    for j in 0..kk {
        let k = (j + 1) as f64;
        let fk = k * f;
        // amp = k ** (-tilt) * (1 + fgain * np.exp(-((fk - form) / fbw) ** 2))
        let z = (fk - form) / fbw;
        let mut amp = dmath::pow(k, -tilt) * (1.0 + fgain * dmath::exp(-(z * z)));
        if odd != 0.0 {
            amp *= if (j + 1) % 2 == 1 { 1.0 } else { 1.0 - odd };
        }
        // (amp[:, None] * np.sin(2 * np.pi * k[:, None] * x[None, :] + _PH[:K, None])).sum(0): the rows added in turn
        let w = 2.0 * PI * k;
        for (i, o) in tb.iter_mut().enumerate() {
            let x = i as f64 / N_WT as f64;
            let row = amp * dmath::sin(w * x + ph[j]);
            *o = if j == 0 { row } else { *o + row };
        }
    }
    let peak = tb.iter().fold(0.0f64, |m, v| if v.abs() > m { v.abs() } else { m });
    let tb = Arc::new(tb.into_iter().map(|v| v / peak).collect::<Vec<f64>>());
    memo.lock().expect("the wavetable memo").insert(key, tb.clone());
    tb
}

/// `tb[(ph * 2048).astype(int) & 2047]`.
pub fn look_up(tb: &[f64], ph: f64) -> f64 {
    tb[((ph * N_WT as f64) as i64 & (N_WT as i64 - 1)) as usize]
}

/// `chanter(f, dur, v, tn, grace, seed)` for the Highland pipe's timbre: the wavetable at `f` (the first 28 ms at
/// `grace`, if any), a phase from the note's own generator, an attack, an exponential release past the held length.
pub fn chanter(f: f64, dur: f64, v: f64, tn: &Tone, grace: Option<f64>, seed: u64) -> Vec<f64> {
    // d = max(.03, dur * tn.get('stacc', 1.)); n = int((d + rel * 6 + .02) * SR)
    let d = py_max(0.03, dur * 1.0);
    let rel = tn.rel;
    let n = pp::len(d + rel * 6.0 + 0.02);
    let t = pp::tt(n);
    let mut rg = Generator::new(seed);
    let mut fr = vec![f; n];
    if let Some(g) = grace {
        let m = pp::len(0.028).min(n);
        fr[..m].fill(g);
    }
    let tb = wt(f, tn.tilt, tn.form, tn.fbw, tn.fgain, tn.odd);
    let r0 = rg.random();
    let ph = dsp::cumsum(&fr);
    (0..n)
        .map(|i| {
            // x = tb[...]; env = np.minimum(1, t / attack) * np.where(t < d, 1., np.exp(-(t - d) / rel))
            let x = look_up(&tb, ph[i] / SRF + r0);
            let a = t[i] / tn.attack;
            let rise = if a < 1.0 { a } else { 1.0 };
            let hold = if t[i] < d { 1.0 } else { dmath::exp(-(t[i] - d) / rel) };
            x * (rise * hold) * v * 0.3
        })
        .collect()
}

/// `drone_voice('ghb', dur)`: the three drones at 44,100 Hz, `dur + 0.14` s, faded in over 50 ms and out over 80 ms.
pub fn drone_voice(dur: f64) -> Vec<f64> {
    let dc = &GHB_DRONES;
    let n = pp::len(dur + 0.14);
    let t = pp::tt(n);
    let mut x = vec![0.0; n];
    let mut rg = Generator::new(3); // len('ghb')
    for &(m, a, cents) in &dc.p {
        let f = pp::midi_hz(m) * dmath::pow(2.0, cents / 1200.0);
        let tb = wt(f, dc.tilt, dc.form, dc.fbw, dc.fgain, dc.odd);
        let r = rg.random();
        for (o, &ti) in x.iter_mut().zip(&t) {
            // ((f * t + rg.random()) % 1.) * 2048
            *o += a * look_up(&tb, fmod_py(f * ti + r, 1.0));
        }
    }
    // env = np.minimum(1, t / .05) * np.clip((dur + .12 - t) / .08, 0, 1); x = D.lp(x, lpf); x * env * v * .3
    pp::lp(&x, dc.lpf)
        .into_iter()
        .zip(&t)
        .map(|(v, &ti)| {
            let a = ti / 0.05;
            let rise = if a < 1.0 { a } else { 1.0 };
            let fall = bf_sound::synth::clip((dur + 0.12 - ti) / 0.08, 0.0, 1.0);
            v * (rise * fall) * dc.v * 0.3
        })
        .collect()
}

/// The roster's scene's length (`SCENES[0]['len']`, the Great Highland bagpipe): its drones hold it.
pub const SCENE_LEN: f64 = 9.6;

/// `grace_for('ghb', s, 0, m)`: the high-G grace (`midi_hz(G5)`) under G5, the high A on G5, none above.
fn grace_for(m: f64) -> Option<f64> {
    if m < 79.0 {
        Some(pp::midi_hz(79.0))
    } else if m == 79.0 {
        Some(pp::midi_hz(81.0))
    } else {
        None
    }
}

/// `synth(fake, 0)` as `pp_bridge.voice` calls it: the chanter's note (`seed = 0 * 7 + m`) and, the event being the
/// scene's first, the drones over the whole scene, added into the longer of the two; at 44,100 Hz.
pub fn synth(midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    let x = chanter(pp::midi_hz(midi), dur, vel, &HIGHLAND, grace_for(midi), midi as i64 as u64);
    let dr = drone_voice(SCENE_LEN - 0.0 + 0.03);
    let mut y = vec![0.0; x.len().max(dr.len())];
    for (o, v) in y.iter_mut().zip(&x) {
        *o += v;
    }
    for (o, v) in y.iter_mut().zip(&dr) {
        *o += v;
    }
    y
}

/// BAGPIPES's Pixel Pitch voice (`pp_bridge.voice('BAGPIPES', ...)`).
#[derive(Clone, Copy, Debug, Default)]
pub struct BagpipesVoice;

impl Voice for BagpipesVoice {
    fn note(&self, _noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        pp::to_48k(&synth(midi, vel, dur))
    }

    fn register(&self) -> (i32, i32) {
        REGISTER
    }

    fn name(&self) -> &str {
        "BAGPIPES"
    }

    /// The chanter holds `max(.03, dur)`: every `dur` but a NaN.
    fn can_hold(&self, dur: f64) -> bool {
        !dur.is_nan()
    }
}
