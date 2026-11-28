//! VIOLA's part in the score: the `Sound` class of its reference kit module
//! (`tools/studio_export/older/fighters/viola.py`, #42 audio.py's VIOLA part moved into the kit's hooks).
//!
//! The Pixel Pitch viola plays a fifth (or so) under the enemy's line: a soft muted pluck on each counted rest, one
//! long bowed note per whole note (DOUBLE STOP a second string a sixth above, the leader doubled an octave down,
//! `Thinner(9)` dropping its plain notes as it loses), a crush's thud, cello crunch and rosin scratch (VIOLA JOKE's
//! also a crash and a sub drop), DOUBLE STOP's scratch, VIOLA JOKE's rimshot, the crowd's laugh (the `laugh` bus) and
//! the falling whoosh, MELODY AT LAST's lead (`VIOLA_LEAD`); the title chord on the C string, the count-in plucks, the
//! groaning slide down when it falls apart, the stinger and the solo when it wins, the pizzicato run home, the climb
//! into the title slam.
//!
//! Every bowed note is `viola(m, dur, vel, accent)`: its voice through `M.note` (a cap 0.3 s past the note, its own
//! `voice`), with a rosin scratch on the attack. The crush's cello is `M.note('CELLO', ...)`: the CELLO's voice, which
//! draws from the bed's CELLO noise stream (`violin_lowpoly.rng`) in the kit's order (ruling M4-R10). The part reads
//! nothing off its look: its note kinds come from the run's `NOTES` records.

use std::sync::{Arc, OnceLock};

use bf_core::dmath::{self, PI};
use bf_core::event::{Event, Value};
use bf_core::harmony::HarmonyError;
use bf_core::py::{fmod_py, py_max, py_min, round_half_even, round_nd};
use bf_core::recording::Array;
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{ColourArgs, MusicCtx, MusicError, Thinner, event_int, event_is_str, event_number, noise_seed};
use bf_sound::synth::{self, FILTER_O, WHOOSH_SEED, clip};
use bf_sound::traits::{Bed, Played, Sound};
use bf_sound::voice::engine2::play::Mute;

/// The name the part's errors carry.
const H: &str = "VIOLA";

/// The bus of the crowd's laugh after VIOLA JOKE's rimshot.
pub const LAUGH_BUS: &str = "laugh";

/// The bus of the crush's cello note (the bed's cellos in #42).
pub const CELLOS_BUS: &str = "cellos";

/// MELODY AT LAST: the music hands the viola the melody for its 2 bars (#42 timeline.py 166-167): `(beat, midi,
/// beats)`.
pub const VIOLA_LEAD: [(f64, i32, f64); 10] = [
    (0.0, 67, 1.0),
    (1.0, 70, 0.5),
    (1.5, 68, 0.5),
    (2.0, 67, 1.5),
    (3.5, 65, 0.5),
    (4.0, 63, 1.0),
    (5.0, 65, 0.75),
    (5.75, 67, 0.25),
    (6.0, 62, 1.0),
    (7.0, 60, 1.0),
];

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// Python's `int(x * SR)` for a length in samples, cut toward zero.
fn samples(seconds: f64) -> Result<usize, MusicError> {
    let n = seconds * SRF;
    if n.is_finite() && n > -1.0 && n < 1.0e9 {
        Ok(n as usize)
    } else {
        Err(MusicError::new(H, format!("{seconds} s is not a length")))
    }
}

/// `np.sign(x)`: -1, 0 or 1 (a NaN stays NaN).
fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

/// `np.clip(x, 0, 1)`.
fn clip01(x: f64) -> f64 {
    clip(x, 0.0, 1.0)
}

/// `scratch(dur, seed=5)`: a bow scratch, rosin noise band-limited and gritty (#28):
///
/// ```python
/// n = int(dur * A.SR)
/// x = np.random.default_rng(seed).normal(0, 1, n)
/// x = A.bp(x, 1800, 7000) * (1 + 0.8 * np.sign(np.sin(2 * np.pi * 37 * A.tt(n))))
/// return x * A.expenv(n, dur * 0.4, 0.004)
/// ```
pub fn scratch(dur: f64, seed: u64) -> Result<Vec<f64>, MusicError> {
    let n = samples(dur)?;
    let x = synth::bp(&synth::noise(seed, n), 1800.0, 7000.0, FILTER_O);
    let env = synth::expenv(n, dur * 0.4, 0.004);
    let w = 2.0 * PI * 37.0;
    Ok(x.iter()
        .zip(synth::tt(n))
        .zip(&env)
        .map(|((&v, t), &e)| v * (1.0 + 0.8 * sign(dmath::sin(w * t))) * e)
        .collect())
}

/// `pizz(m, vel=0.8, muted=False)`: a plucked string with a little wooden body (#28); `muted` is palm-muted, short
/// and dull (the rests):
///
/// ```python
/// n = int((0.25 if muted else 0.5) * SR); f = A.mtof(m); t = A.tt(n)
/// dec = 0.05 if muted else 0.16
/// x = (np.sin(2 * np.pi * f * t) + 0.5 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.05)
///      + 0.25 * np.sin(6 * np.pi * f * t) * np.exp(-t / 0.03)) * np.exp(-t / dec)
/// x += 0.15 * A.bp(np.random.default_rng(m).normal(0, 1, n), 800, 3000) * np.exp(-t / 0.01)
/// return A.lp(x, 1600 if muted else 4500) * vel
/// ```
pub fn pizz(m: i32, vel: f64, muted: bool) -> Result<Vec<f64>, MusicError> {
    let n = samples(if muted { 0.25 } else { 0.5 })?;
    let f = synth::mtof(f64::from(m));
    let dec = if muted { 0.05 } else { 0.16 };
    let seed = u64::try_from(m).map_err(|_| MusicError::new(H, format!("{m} is not a noise seed")))?;
    let body = synth::bp(&synth::noise(seed, n), 800.0, 3000.0, FILTER_O);
    let (w1, w2, w3) = (2.0 * PI * f, 4.0 * PI * f, 6.0 * PI * f);
    let x: Vec<f64> = synth::tt(n)
        .iter()
        .zip(&body)
        .map(|(&t, &b)| {
            let s = dmath::sin(w1 * t)
                + 0.5 * dmath::sin(w2 * t) * dmath::exp(-t / 0.05)
                + 0.25 * dmath::sin(w3 * t) * dmath::exp(-t / 0.03);
            s * dmath::exp(-t / dec) + 0.15 * b * dmath::exp(-t / 0.01)
        })
        .collect();
    Ok(synth::lp(&x, if muted { 1600.0 } else { 4500.0 }, FILTER_O)
        .iter()
        .map(|v| v * vel)
        .collect())
}

/// `addp(*xs)`: signals of different lengths summed (zero-padded): `sum(np.pad(x, (0, n - len(x))) for x in xs)`.
fn addp(xs: &[&[f64]]) -> Vec<f64> {
    let n = xs.iter().map(|x| x.len()).max().unwrap_or(0);
    let mut out = vec![0.0; n];
    for x in xs {
        for (o, &v) in out.iter_mut().zip(x.iter()) {
            *o += v;
        }
    }
    out
}

/// `tom(f0=200.0, dec=0.18, vel=1.0)`: a pitched tom with a stick's noise.
///
/// ```python
/// n = int(0.45 * SR); t = A.tt(n)
/// f = f0 * (1 + 0.6 * np.exp(-t / 0.03))
/// x = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / dec)
/// x += 0.3 * A.bp(np.random.default_rng(int(f0)).normal(0, 1, n), 300, 3000) * np.exp(-t / 0.02)
/// return x * vel
/// ```
fn tom(f0: f64, dec: f64, vel: f64) -> Result<Vec<f64>, MusicError> {
    let n = samples(0.45)?;
    let t = synth::tt(n);
    let f: Vec<f64> = t.iter().map(|&t| f0 * (1.0 + 0.6 * dmath::exp(-t / 0.03))).collect();
    let ph = bf_sound::dsp::cumsum(&f);
    let stick = synth::bp(&synth::noise(noise_seed(H, f0)?, n), 300.0, 3000.0, FILTER_O);
    Ok(t.iter()
        .zip(&ph)
        .zip(&stick)
        .map(|((&t, &c), &s)| {
            let x = dmath::sin(2.0 * PI * c / SRF) * dmath::exp(-t / dec);
            (x + 0.3 * s * dmath::exp(-t / 0.02)) * vel
        })
        .collect())
}

/// `rimshot(S16)`: ba-dum-tss: two toms (high, then low with the kick) and a ride / crash "tss".
///
/// ```python
/// n = int(1.6 * SR); out = np.zeros(n)
/// put(addp(A.snare(0.08) * 0.6, tom(230, 0.14)), 0.0, 0.9)
/// put(addp(tom(150, 0.22), A.kick(0.8, 0.25, 120, 50)[:int(0.4 * SR)] * 0.7), 1.5 * S16, 1.0)
/// nt = int(1.2 * SR); tt_ = A.tt(nt)
/// tss = A.hp(np.random.default_rng(77).normal(0, 1, nt), 5500) * np.exp(-tt_ / 0.35)
/// tss += 0.12 * sum(np.sin(2 * np.pi * f * tt_) for f in (5100, 6650, 8230)) * np.exp(-tt_ / 0.5)
/// put(tss, 3 * S16, 0.55)
/// ```
pub fn rimshot(s16: f64) -> Result<Vec<f64>, MusicError> {
    let n = samples(1.6)?;
    let mut out = vec![0.0; n];
    let mut put = |x: &[f64], t: f64, g: f64| -> Result<(), MusicError> {
        let a = samples(t)?;
        let b = n.min(a + x.len());
        for i in a..b {
            out[i] += x[i - a] * g;
        }
        Ok(())
    };
    let snare: Vec<f64> = synth::snare(0.08).iter().map(|v| v * 0.6).collect();
    put(&addp(&[&snare, &tom(230.0, 0.14, 1.0)?]), 0.0, 0.9)?;
    let kick = synth::kick(0.8, 0.25, 120.0, 50.0);
    let kick: Vec<f64> = kick[..kick.len().min(samples(0.4)?)].iter().map(|v| v * 0.7).collect();
    put(&addp(&[&tom(150.0, 0.22, 1.0)?, &kick]), 1.5 * s16, 1.0)?;
    let nt = samples(1.2)?;
    let tt = synth::tt(nt);
    let hiss = synth::hp(&synth::noise(77, nt), 5500.0, FILTER_O);
    let ws = [5100.0, 6650.0, 8230.0].map(|f| 2.0 * PI * f);
    let tss: Vec<f64> = tt
        .iter()
        .zip(&hiss)
        .map(|(&t, &h)| {
            let mut bell = 0.0;
            for w in ws {
                bell += dmath::sin(w * t);
            }
            h * dmath::exp(-t / 0.35) + 0.12 * bell * dmath::exp(-t / 0.5)
        })
        .collect();
    put(&tss, 3.0 * s16, 0.55)?;
    Ok(out)
}

/// `ha(f0, seed, dur=0.13, vowel=...)`: one "ha", a breathy voiced burst (a pulse train through the vowel's
/// formants) with an aspirated onset.
///
/// ```python
/// n = int(dur * A.SR); t = A.tt(n); rng = np.random.default_rng(seed)
/// f = f0 * (1 + 0.06 * np.exp(-t / 0.04)) * (1 - 0.08 * t / dur)
/// ph = np.cumsum(f) / A.SR
/// pulse = (2 * (ph % 1.0) - 1) ** 3
/// noise = rng.normal(0, 1, n)
/// src = pulse * np.clip((t - 0.015) / 0.01, 0, 1) + 0.35 * noise
/// x = sum(g * A.bp(src, fc * 0.82, fc * 1.18) for fc, g in vowel)
/// env = np.clip(t / 0.012, 0, 1) * np.exp(-np.maximum(t - 0.03, 0) / (dur * 0.45))
/// return x * env
/// ```
fn ha(f0: f64, seed: u64, dur: f64) -> Result<Vec<f64>, MusicError> {
    const VOWEL: [(f64, f64); 3] = [(800.0, 1.0), (1200.0, 0.6), (2600.0, 0.25)];
    let n = samples(dur)?;
    let t = synth::tt(n);
    let f: Vec<f64> = t
        .iter()
        .map(|&t| f0 * (1.0 + 0.06 * dmath::exp(-t / 0.04)) * (1.0 - 0.08 * t / dur))
        .collect();
    let noise = synth::noise(seed, n);
    let src: Vec<f64> = bf_sound::dsp::cumsum(&f)
        .iter()
        .zip(&t)
        .zip(&noise)
        .map(|((&c, &t), &z)| {
            let ph = c / SRF;
            let pulse = dmath::pow(2.0 * fmod_py(ph, 1.0) - 1.0, 3.0);
            pulse * clip01((t - 0.015) / 0.01) + 0.35 * z
        })
        .collect();
    let mut x = vec![0.0; n];
    for (fc, g) in VOWEL {
        for (o, v) in x.iter_mut().zip(synth::bp(&src, fc * 0.82, fc * 1.18, FILTER_O)) {
            *o += g * v;
        }
    }
    Ok(x.iter()
        .zip(&t)
        .map(|(&v, &t)| {
            let env = clip01(t / 0.012) * dmath::exp(-py_max(t - 0.03, 0.0) / (dur * 0.45));
            v * env
        })
        .collect())
}

/// `crowd_laugh(dur=2.1, n_voices=34, seed=55)`: the whole crowd bursts out laughing: many voices, each a run of
/// "ha"s (faster and higher at first, then trailing off), random onsets, pitches and pans; its two channels.
///
/// ```python
/// n = int(dur * SR); out = np.zeros((n, 2)); rng = np.random.default_rng(seed)
/// for v in range(n_voices):
///     f0 = rng.uniform(140, 330) if v % 3 else rng.uniform(260, 420)
///     t = rng.uniform(0.0, 0.28); rate = rng.uniform(0.15, 0.21); pan = rng.uniform(-0.8, 0.8)
///     amp = rng.uniform(0.5, 1.0); j = 0
///     while t < dur - 0.2:
///         x = ha(f0 * (1 - 0.03 * j), int(seed * 1000 + v * 37 + j), rng.uniform(0.10, 0.15))
///         g = amp * math.exp(-t / (dur * 0.7)) * (1.0 if j < 6 else 0.7)
///         a = int(t * SR); b = min(n, a + len(x))
///         out[a:b, 0] += x[:b - a] * g * (1 - pan) * 0.5
///         out[a:b, 1] += x[:b - a] * g * (1 + pan) * 0.5
///         t += rate * (1 + 0.04 * j) + rng.uniform(-0.02, 0.02); j += 1
/// return out / math.sqrt(n_voices) * 2.2
/// ```
pub fn crowd_laugh(dur: f64, n_voices: u64, seed: u64) -> Result<[Vec<f64>; 2], MusicError> {
    let n = samples(dur)?;
    let mut out = [vec![0.0; n], vec![0.0; n]];
    let mut rng = Generator::new(seed);
    for v in 0..n_voices {
        let f0 = if v % 3 != 0 {
            rng.uniform(140.0, 330.0)
        } else {
            rng.uniform(260.0, 420.0)
        };
        let mut t = rng.uniform(0.0, 0.28);
        let rate = rng.uniform(0.15, 0.21);
        let pan = rng.uniform(-0.8, 0.8);
        let amp = rng.uniform(0.5, 1.0);
        let mut j: u64 = 0;
        while t < dur - 0.2 {
            let f = f0 * (1.0 - 0.03 * j as f64);
            let s = seed * 1000 + v * 37 + j;
            let x = ha(f, s, rng.uniform(0.10, 0.15))?;
            let g = amp * dmath::exp(-t / (dur * 0.7)) * if j < 6 { 1.0 } else { 0.7 };
            let a = samples(t)?;
            let b = n.min(a + x.len());
            for i in a..b {
                out[0][i] += x[i - a] * g * (1.0 - pan) * 0.5;
                out[1][i] += x[i - a] * g * (1.0 + pan) * 0.5;
            }
            t += rate * (1.0 + 0.04 * j as f64) + rng.uniform(-0.02, 0.02);
            j += 1;
        }
    }
    let r = (n_voices as f64).sqrt();
    for ch in &mut out {
        for v in ch.iter_mut() {
            *v = *v / r * 2.2;
        }
    }
    Ok(out)
}

/// [`crowd_laugh`] with its defaults (2.1 s), made once: it is the same every time.
fn the_laugh() -> Result<&'static [Vec<f64>; 2], MusicError> {
    static LAUGH: OnceLock<Result<[Vec<f64>; 2], MusicError>> = OnceLock::new();
    LAUGH
        .get_or_init(|| crowd_laugh(2.1, 34, 55))
        .as_ref()
        .map_err(Clone::clone)
}

/// `sub808(m0, m1, dur, slide=0.0, vel=1.0)`: the 808, a sine sub with a punchy pitch drop on the attack,
/// saturated, gliding to `m1` over the last `slide` s (#42's bed; VIOLA JOKE's crush drops it).
///
/// ```python
/// n = int((dur + 0.05) * SR); t = A.tt(n)
/// f0, f1 = A.mtof(m0), A.mtof(m1)
/// u = np.clip((t - (dur - slide)) / max(slide, 1e-3), 0, 1) if slide > 0 else np.zeros(n)
/// f = f0 * (f1 / f0) ** (u * u * (3 - 2 * u)) * (1 + 0.5 * np.exp(-t / 0.02))
/// x = np.sin(2 * np.pi * np.cumsum(f) / SR)
/// x = np.tanh(2.2 * x) / np.tanh(2.2)
/// env = np.clip(t / 0.004, 0, 1) * np.clip((dur + 0.05 - t) / 0.05, 0, 1) * (0.75 + 0.25 * np.exp(-t / 0.15))
/// return A.lp(x, 900) * env * vel
/// ```
pub fn sub808(m0: f64, m1: f64, dur: f64, slide: f64, vel: f64) -> Result<Vec<f64>, MusicError> {
    let n = samples(dur + 0.05)?;
    let t = synth::tt(n);
    let (f0, f1) = (synth::mtof(m0), synth::mtof(m1));
    let ratio = f1 / f0;
    let f: Vec<f64> = t
        .iter()
        .map(|&t| {
            let u = if slide > 0.0 {
                clip01((t - (dur - slide)) / py_max(slide, 1e-3))
            } else {
                0.0
            };
            f0 * dmath::pow(ratio, u * u * (3.0 - 2.0 * u)) * (1.0 + 0.5 * dmath::exp(-t / 0.02))
        })
        .collect();
    let th = dmath::np_tanh(2.2);
    let x: Vec<f64> = bf_sound::dsp::cumsum(&f)
        .iter()
        .map(|&c| dmath::np_tanh(2.2 * dmath::sin(2.0 * PI * c / SRF)) / th)
        .collect();
    Ok(synth::lp(&x, 900.0, FILTER_O)
        .iter()
        .zip(&t)
        .map(|(&v, &t)| {
            let env = clip01(t / 0.004) * clip01((dur + 0.05 - t) / 0.05) * (0.75 + 0.25 * dmath::exp(-t / 0.15));
            v * env * vel
        })
        .collect())
}

/// `fit(T, m, chord, lo=48, hi=79)` (#42 timeline.py): a melody note nudged down onto the chord's scale (C harmonic
/// minor; over Ab, Fm or Cm the B becomes Bb), then into `lo..hi`.
fn fit(ctx: &MusicCtx, m: i32, chord: &str, lo: i32, hi: i32) -> i32 {
    let h = &ctx.harmony;
    let mut pcs: Vec<i32> = h.scale.iter().map(|s| (h.key_root + s).rem_euclid(12)).collect();
    if matches!(chord, "Ab" | "Fm" | "Cm") {
        for p in &mut pcs {
            if *p == 11 {
                *p = 10;
            }
        }
    }
    let mut m = m;
    if !pcs.is_empty() {
        while !pcs.contains(&m.rem_euclid(12)) {
            m -= 1;
        }
    }
    lo.max(hi.min(m))
}

/// The VIOLA's part.
#[derive(Clone, Copy, Debug)]
pub struct ViolaSound {
    k: usize,
}

/// What `setup_run` derives from the run: each of its notes' kind (by note id, the first record of it) and the
/// windows its MELODY AT LAST holds the lead (from the beat after it fires to its end or the K.O.).
struct Run {
    kinds: Vec<(i64, i64)>,
    mel_win: Vec<(f64, f64)>,
}

impl Run {
    fn kind(&self, id: i64) -> Option<i64> {
        self.kinds.iter().find(|(i, _)| *i == id).map(|&(_, k)| k)
    }

    fn in_melody(&self, t: f64) -> bool {
        self.mel_win.iter().any(|&(a, b)| a <= t && t < b)
    }
}

/// A number of a `NOTES` record.
fn num(v: &Value) -> Result<f64, MusicError> {
    match v {
        Value::Float(x) => Ok(*x),
        Value::Int(n) => Ok(*n as f64),
        other => Err(MusicError::new(
            H,
            format!("a NOTES record holds {other:?}, not a number"),
        )),
    }
}

impl ViolaSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> ViolaSound {
        ViolaSound { k }
    }

    /// `setup_run()`.
    fn run(&self, ctx: &MusicCtx) -> Result<Run, MusicError> {
        let mut kinds: Vec<(i64, i64)> = Vec::new();
        if let Some(Array::Nested(Value::List(frames))) = ctx.rec.arrays.get("NOTES") {
            for rows in frames {
                let Value::List(rows) = rows else {
                    return Err(MusicError::new(H, "the run's NOTES is not a list of records a frame"));
                };
                for row in rows {
                    let Value::List(r) = row else {
                        return Err(MusicError::new(H, "a NOTES record is not a list"));
                    };
                    if r.len() < 9 {
                        return Err(MusicError::new(H, "a NOTES record is shorter than 9 numbers"));
                    }
                    if num(&r[0])? as i64 == self.k as i64 {
                        let id = num(&r[7])? as i64;
                        if !kinds.iter().any(|(i, _)| *i == id) {
                            kinds.push((id, num(&r[8])? as i64));
                        }
                    }
                }
            }
        }
        let beat = ctx.show.grid.beat;
        let mut mel_win = Vec::new();
        for e in self.mine(ctx, "melody") {
            let s = ((e.t + 1e-6) / beat).ceil() * beat;
            mel_win.push((s, py_min(event_number(H, e, 3)?, ctx.show.t_ko)));
        }
        Ok(Run { kinds, mel_win })
    }

    /// The events of its own side: `e[1] == kind and e[2] == k`.
    fn mine<'c>(&self, ctx: &'c MusicCtx, kind: &'static str) -> impl Iterator<Item = &'c Event> + 'c {
        let k = self.k;
        ctx.rec
            .events
            .iter()
            .filter(move |e| e.kind == kind && e.who() == Some(k))
    }

    /// `viola(m, dur, vel=0.85, accent=0.0)`: a bowed viola note (its voice) with an optional heavy bow attack:
    ///
    /// ```python
    /// x = self.voice(m, dur, vel)
    /// if accent > 0:
    ///     s = scratch(0.07, int(m * 7))[:len(x)]
    ///     x = x.copy(); x[:len(s)] += accent * 0.25 * s
    /// ```
    fn viola(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64, accent: f64) -> Result<Vec<f64>, MusicError> {
        let mut x = self.voice(ctx, f64::from(m), dur, vel)?.to_vec();
        if accent > 0.0 {
            let seed = u64::try_from(m * 7).map_err(|_| MusicError::new(H, format!("{m} is not a noise seed")))?;
            let s = scratch(0.07, seed)?;
            for (v, &s) in x.iter_mut().zip(&s) {
                *v += accent * 0.25 * s;
            }
        }
        Ok(x)
    }

    /// `cello(m, dur, vel)`: `M.note('CELLO', m, round(dur, 3), round(dur + 0.2, 2), round(vel, 1))`.
    fn cello(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64) -> Result<Vec<f64>, MusicError> {
        Ok(ctx
            .note(
                "CELLO",
                f64::from(m),
                round_nd(dur, 3),
                round_nd(dur + 0.2, 2),
                round_nd(vel, 1),
            )?
            .to_vec())
    }

    /// `inner(t)`: a chord tone a fifth (or so) under the enemy's line, in the viola's warm range (C3-G4).
    fn inner(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let h = &ctx.harmony;
        let c = h.chord_at(t);
        let enemy = &ctx.show.names[self.enemy()];
        let line = ctx.line_at(enemy, t)?.ok_or_else(|| {
            MusicError::new(
                H,
                format!("{enemy}'s line has a rest at {t} s: this part plays a fifth under a note there"),
            )
        })?;
        let tgt = line - 7;
        let cands = h.chord_tones(48, 67, c).map_err(bad)?;
        cands
            .into_iter()
            .min_by_key(|&m| ((m - tgt).abs(), m))
            .ok_or_else(|| MusicError::new(H, format!("the chord {c:?} has no tone from C3 to G4")))
    }

    /// A pizzicato (`muted`: the kit's muted one, short): under the Composed arrangement the VIOLA's built
    /// instrument's (Va3), at the level of the kit's own pluck; under Classic the kit's [`pizz`].
    fn pluck(&self, ctx: &MusicCtx, m: i32, vel: f64, muted: bool) -> Result<Vec<f64>, MusicError> {
        let pp = &ctx.cfg[self.k].pp;
        match ctx.engine2_of(pp) {
            Some(inst) => {
                let mute = if muted { Mute::Choked } else { Mute::Open };
                crate::fighters::violin::sound::built_pluck(ctx, pp, inst, m, vel, mute)
            }
            None => pizz(m, vel, muted),
        }
    }

    /// The note the viola sounds at `t`. Under the Composed arrangement it is its own written line's note there,
    /// through [`MusicCtx::play_line`] (sound upgrade: every pitched fighter plays its line); under Classic, and on
    /// a rest of the line, the kit's inner voice under the enemy's line ([`ViolaSound::inner`]).
    fn sounded(&self, ctx: &MusicCtx, t: f64) -> Result<(i32, bool), MusicError> {
        if ctx.composed_on()
            && let Some(m) = ctx.play_line(self.name(ctx), t)?
        {
            return Ok((m, true));
        }
        Ok((self.inner(ctx, t)?, false))
    }

    /// `self.colour(x, t, drive_k)`.
    fn col(&self, ctx: &MusicCtx, x: &[f64], t: f64, drive_k: f64) -> Result<(Vec<f64>, f64, f64), MusicError> {
        self.colour(ctx, x, t, drive_k, &self.colour_args())
    }
}

/// `mix.place(bus, x, t, g, pan)` of a mono signal.
fn put(mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, g: f64, pan: f64) {
    mix.place(bus, &Sig::Mono(x), t, g, pan);
}

impl Sound for ViolaSound {
    fn k(&self) -> usize {
        self.k
    }

    /// #42's viola bus (its `BUS_GAIN`).
    fn bus_gain(&self) -> f64 {
        1.7
    }

    /// #42's reverb send.
    fn send(&self) -> f64 {
        0.4
    }

    /// #42's band check: the inner voice C3-G4 and its harmonics.
    fn band(&self) -> (f64, f64) {
        (120.0, 1500.0)
    }

    /// The viola keeps its low strings as it thins.
    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 70.0,
            cut_span: 300.0,
            drive: 4.5,
            ..ColourArgs::default()
        }
    }

    /// `voice(m, dur, vel=0.85)`: `M.note(M.CFG[k]['pp'], m, round(dur, 3), round(dur + 0.3, 2), round(vel, 1))`.
    fn voice(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        let name = &ctx.cfg[self.k].pp;
        ctx.note(name, m, round_nd(dur, 3), round_nd(dur + 0.3, 2), round_nd(vel, 1))
    }

    /// A dark chord on the C string (C3 G3 Eb4).
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        for (m, g) in [(48, 0.42), (55, 0.36), (63, 0.30)] {
            put(mix, &bus, self.viola(ctx, m, 0.8, 1.0, 0.5)?, t, g, -0.2);
        }
        Ok(())
    }

    /// The viola counts it in: a pluck on each of the first three counts.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        for (j, (tc, _)) in ctx.show.grid.count.iter().take(3).enumerate() {
            put(
                mix,
                &bus,
                self.pluck(ctx, [55, 59, 62][j], 0.9, false)?,
                *tc,
                0.22,
                -0.2,
            );
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        let run = self.run(ctx)?;
        let mut thin_a = Thinner::new(9.0);
        // sound upgrade: under Composed its bowed notes are one voice (a new one ends the one before it); the plucks
        // between them are plucked, and ring over the bow as they do today
        let one = ctx.one_voice(self.name(ctx));
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if e.who() != Some(k) {
                continue;
            }
            match e.kind.as_str() {
                "rest" => {
                    // the counting: a soft muted pluck on 1-2-3
                    let tq = ctx.q16(t);
                    let count = event_int(H, e, 3)?;
                    let (m, own) = self.sounded(ctx, tq)?;
                    let m = if own {
                        // its written note, the "1" an octave down where the C string reaches it
                        if count == 1 && m - 12 >= 48 { m - 12 } else { m }
                    } else {
                        48.max(m - if count == 1 { 12 } else { 0 })
                    };
                    put(
                        mix,
                        &bus,
                        self.pluck(ctx, m, 0.7, true)?,
                        tq,
                        0.22 + 0.03 * count as f64,
                        -0.2,
                    );
                }
                "note" => {
                    let nkind = event_int(H, e, 8)?;
                    let (t_land, t1) = (event_number(H, e, 9)?, event_number(H, e, 10)?);
                    let ts = ctx.q16(t_land);
                    if nkind != 2 && run.in_melody(ts) {
                        continue; // the lead line plays these (below)
                    }
                    let slot = round_half_even(ts / s16);
                    if nkind == 0 && !thin_a.keep(slot, ctx.hp_at(k, ts) / 100.0) {
                        continue;
                    }
                    let dur = py_max(0.3, py_min(bar, t1 - t_land) * 0.96);
                    // VIOLA JOKE's chord is its own: only a plain or a double-stopped note sounds the line
                    let (m, own) = if nkind == 2 {
                        (self.inner(ctx, ts)?, false)
                    } else {
                        self.sounded(ctx, ts)?
                    };
                    if nkind == 2 {
                        // VIOLA JOKE: the open C + G, fortissimo
                        for (mm, g) in [(48, 0.50), (55, 0.42), (60, 0.26)] {
                            put(mix, &bus, self.viola(ctx, mm, dur, 1.0, 0.8)?, ts, g, -0.15);
                        }
                        continue;
                    }
                    let (sig, g, lead) = self.col(ctx, &self.viola(ctx, m, dur, 0.9, 0.45)?, ts, 0.8)?;
                    let gain = if nkind == 0 { 0.52 } else { 0.38 };
                    mix.note(&bus, &Sig::Mono(sig), ts, gain * g, -0.2, one);
                    if nkind == 1 {
                        // DOUBLE STOP: a second string a sixth above
                        let tones = ctx
                            .harmony
                            .chord_tones(m + 7, m + 10, ctx.harmony.chord_at(ts))
                            .map_err(bad)?;
                        let m2 = tones.into_iter().min().unwrap_or(m + 7);
                        let x = self.col(ctx, &self.viola(ctx, m2, dur, 0.85, 0.3)?, ts, 1.0)?.0;
                        mix.note(&bus, &Sig::Mono(x), ts, 0.26 * g, -0.25, one);
                    } else if lead > 0.04 {
                        // the leader: doubled an octave down (a written note under C4: an octave up, not the open C)
                        let low = if own && m - 12 < 48 { m + 12 } else { 48.max(m - 12) };
                        let x = self.col(ctx, &self.viola(ctx, low, dur, 0.7, 0.0)?, ts, 1.0)?.0;
                        mix.note(&bus, &Sig::Mono(x), ts, 0.16 * g, -0.2, one);
                    }
                }
                "crush" => {
                    // a whole note crushes: a low crunch + thud
                    let tq = ctx.q16(t);
                    let big = run.kind(event_int(H, e, 3)?) == Some(2);
                    let thud = synth::thud(if big { 0.5 } else { 0.32 }, 110.0, 36.0, noise_seed(H, t * 13.0)?);
                    put(mix, "fx", thud, tq, if big { 0.42 } else { 0.26 }, 0.0);
                    put(
                        mix,
                        CELLOS_BUS,
                        self.cello(ctx, 36, 0.12, 1.0)?,
                        tq,
                        if big { 0.20 } else { 0.12 },
                        0.0,
                    );
                    let s = scratch(0.12, noise_seed(H, t * 17.0)?)?;
                    put(mix, &bus, s, tq, if big { 0.10 } else { 0.06 }, -0.2);
                    if big {
                        put(mix, "drums", synth::crash(2.0), tq, 0.12, 0.0);
                        put(mix, "bass808", sub808(24.0, 24.0, 0.9, 0.0, 1.0)?, tq, 0.70, 0.0);
                    }
                }
                "ability" => {
                    let tq = ctx.q16(t);
                    if event_is_str(H, e, 4, "double_stop")? {
                        // two strings dug in at once
                        put(mix, &bus, scratch(0.1, noise_seed(H, t * 29.0)?)?, tq, 0.10, -0.2);
                    } else if event_is_str(H, e, 4, "joke")? {
                        // ba-dum-tss + the crowd laughing
                        put(mix, "fx", rimshot(s16)?, tq, 0.42, 0.0);
                        let lg = the_laugh()?;
                        let tl = tq + 3.0 * s16 + 0.05;
                        put(mix, LAUGH_BUS, lg[0].clone(), tl, 0.52, -0.6);
                        put(mix, LAUGH_BUS, lg[1].clone(), tl, 0.52, 0.6);
                        // the note falling
                        let w = synth::whoosh(beat, 200.0, 3000.0, false, WHOOSH_SEED);
                        put(mix, "fx", w, tq + 0.1, 0.10, 0.0);
                    }
                }
                _ => {}
            }
        }
        // MELODY AT LAST: the viola sings the lead for its 2 bars
        for &(a, b) in &run.mel_win {
            let mut prev: Option<i32> = None;
            for (bt, m, dl) in VIOLA_LEAD {
                let ts = a + bt * beat;
                if ts >= b - 0.02 {
                    break;
                }
                let mm = fit(ctx, m, ctx.harmony.chord_at(ts + 0.01), 52, 76);
                let accent = match prev {
                    Some(p) if (mm - p).abs() <= 2 => 0.1,
                    _ => 0.3,
                };
                let x = self.viola(ctx, mm, dl * beat * 0.97, 0.95, accent)?;
                let (sig, g, _) = self.col(ctx, &x, ts, 0.5)?;
                put(mix, &bus, sig, ts, 0.50 * g, -0.1);
                let low = self
                    .col(
                        ctx,
                        &self.viola(ctx, 48.max(mm - 12), dl * beat * 0.97, 0.7, 0.0)?,
                        ts,
                        1.0,
                    )?
                    .0;
                put(mix, &bus, low, ts, 0.14, -0.2);
                prev = Some(mm);
            }
            let w = synth::whoosh(0.5, 300.0, 4000.0, true, WHOOSH_SEED);
            put(mix, "fx", w, a - 0.25, 0.06, 0.0);
        }
        Ok(())
    }

    /// A groaning slide down.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let t_hs = ctx.show.t_hs;
        for (j, m) in [67, 63, 60, 55, 51, 48].into_iter().enumerate() {
            let j = j as f64;
            put(
                mix,
                &bus,
                self.viola(ctx, m, 0.12, 0.8, 0.0)?,
                t_hs + 0.1 * j,
                0.24,
                (j - 2.5) / 6.0,
            );
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        for (m, g) in [(48, 0.44), (55, 0.36), (63, 0.30), (67, 0.26)] {
            put(mix, "solo", self.viola(ctx, m, 1.1, 1.0, 0.6)?, tw, g, 0.0);
        }
        Ok(())
    }

    /// The fight file's solo, bowed with an accent on each note.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            put(
                mix,
                "solo",
                self.viola(ctx, m, d * grid.beat * 0.95, 0.95, 0.3)?,
                ts,
                0.52,
                0.0,
            );
        }
        Ok(())
    }

    /// Its pieces fly home as a pizzicato run.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        for (j, m) in [48, 55, 60, 63, 67, 72].into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / 6.0);
            put(mix, &bus, self.pluck(ctx, m, 0.7, false)?, tj, 0.20, -0.2);
        }
        Ok(())
    }

    /// Climbing G into the C on frame 0.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let t35 = (grid.n_bars as f64 - 1.0) * grid.bar;
        for (bt, m) in [55, 59, 62, 67].into_iter().enumerate() {
            put(
                mix,
                &bus,
                self.viola(ctx, m, 0.3, 0.9, 0.0)?,
                t35 + bt as f64 * grid.beat,
                0.20,
                -0.2,
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_helpers_are_shaped_as_the_kits() {
        assert_eq!(VIOLA_LEAD.iter().map(|n| n.2).sum::<f64>(), 8.0);
        assert_eq!((sign(0.0), sign(-0.3), sign(2.0)), (0.0, -1.0, 1.0));
        assert_eq!(addp(&[&[1.0, 2.0], &[1.0]]), [2.0, 2.0]);
        // lengths: int(dur * SR)
        assert_eq!(scratch(0.07, 5).unwrap().len(), 3360);
        assert_eq!(pizz(55, 0.8, true).unwrap().len(), 12000);
        assert_eq!(pizz(55, 0.8, false).unwrap().len(), 24000);
        assert_eq!(rimshot(0.15).unwrap().len(), 76800);
        assert_eq!(sub808(24.0, 24.0, 0.9, 0.0, 1.0).unwrap().len(), 45600);
        // the first sample of a scratch is silent (the sign of sin(0) is 0, and the envelope starts at 0)
        assert_eq!(scratch(0.1, 3).unwrap()[0], 0.0);
        assert!(pizz(-1, 0.8, true).is_err());
    }

    #[test]
    fn the_laugh_is_two_channels_and_the_same_every_time() {
        let a = crowd_laugh(2.1, 34, 55).unwrap();
        assert_eq!((a[0].len(), a[1].len()), (100800, 100800));
        assert!(a[0].iter().chain(&a[1]).all(|v| v.is_finite()));
        assert!(a[0].iter().any(|v| v.abs() > 1e-3) && a[1].iter().any(|v| v.abs() > 1e-3));
        assert_eq!(the_laugh().unwrap(), &a);
    }
}
