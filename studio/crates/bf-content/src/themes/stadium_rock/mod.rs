//! Theme: minor stadium rock, plugged / unplugged (`projects/ballfight/kit/moods/stadium_rock.py`, #44 ELECTRIC vs
//! ACOUSTIC). One original minor arena-rock song (huge drums, a big room) in two arrangements that follow the
//! leader:
//!
//! - plugged (`p_gtr`, `p_bass`, `p_drums`): a fuzz rhythm guitar (dark power chords, palm-muted chugs), a driving
//!   pick bass, the stadium kit (snare with a big room, hats, crashes, tom fills);
//! - unplugged (`u_gtr`, `u_bass`, `u_perc`): a quiet strummed shimmer up the neck (the ACOUSTIC's own Pixel Pitch
//!   voice), an upright-ish bass walking roots and fifths, hand percussion (cajon slaps, shaker 16ths, tambourine);
//! - both: the stadium kick (and its claps on 2 and 4 in the big sections), on `kick`.
//!
//! The fight file names the plugged fighter (`plugged`, the kit's `PLUGGED`; default the left corner's). While it
//! leads (the series' 5 HP hysteresis) the band is plugged in; while the other leads it goes unplugged; before
//! anyone leads it is both at once. Every lead change crossfades over one beat from the next 8th; after the K.O. it
//! follows the winner, and the build into the title slam fades back to both. Both arrangements are matched to the
//! same K-weighted loudness over the fight before the equal-power crossfade ([`lufs`], its own copy of the
//! BS.1770 filters: the loudness check's lives in `bf-export`).
//!
//! Here: the theme's harmony helpers, its instruments as the kit computes them and the loudness. The arrangement
//! that reproduces the kit's bed is [`StadiumRockClassic`], in `classic.rs`; it also holds the kit's per-bed cache
//! of sounds and reads the run (the lead changes, the story's captions, the winner) for the plugged share.
//!
//! **Voicings.** The mood has no method that voices one chord: its guitars and basses play [`root`] (a number,
//! power chords on it), and its strum plays [`strum_tones`], the harmony's own chord tones from E4 up.

mod classic;

use bf_core::dmath::{self, PI};
use bf_core::fightfile::OptionValue;
use bf_core::harmony::{Harmony, HarmonyError};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::synth::{self, FILTER_O, clip, noise};

pub use self::classic::StadiumRockClassic;
use super::{OptionKind, Preset, Theme, ThemeOption, ThemeOptions};
use crate::cards::ThemeCard;

/// The theme as the registry lists it: one option (`plugged`); its bed plays the ACOUSTIC's voice.
pub static THEME: Theme = Theme {
    name: "stadium_rock",
    ported: true,
    card: &CARD,
    options: &OPTIONS,
    voices: &["ACOUSTIC"],
    own_voice: super::no_own_voice,
    classic: |options| Ok(Box::new(StadiumRockClassic::new(plugged(options)))),
};

/// `PLUGGED`: the fighter whose lead plugs the band in (default the left corner's, as the kit's `M.NAMES[0]`).
pub static OPTIONS: [ThemeOption; 1] = [ThemeOption {
    key: "plugged",
    kit: "PLUGGED",
    kind: OptionKind::Fighter,
    default: None,
}];

/// The plugged fighter the options name, or `None` for the left corner's (an option the fight file leaves out).
fn plugged(options: &ThemeOptions) -> Option<String> {
    match options.get("plugged") {
        Some(OptionValue::Str(name)) => Some(name.clone()),
        _ => None,
    }
}

/// #44's loop: i - VI - III - VII, i - VI - iv - V7 in E minor.
const LOOP: [&str; 8] = ["Em", "C", "G", "D", "Em", "C", "Am", "B7"];

/// What the interface says about the stadium rock.
pub static CARD: ThemeCard = ThemeCard {
    display: "Stadium rock",
    text: "One minor arena-rock song in two arrangements that follow the leader: plugged (fuzz power chords, pick \
           bass, the stadium kit) while the plugged fighter leads, unplugged (a strummed acoustic shimmer, an upright \
           bass, cajon and shaker) while the other leads. The stadium kick plays in both.",
    key: "E",
    scale: "minor",
    chord_loops: &[&LOOP],
    presets: &PRESETS,
    bars: (24, 42),
};

/// The kit configs that play the stadium rock (Table S4): their music as each `episode.py` sets it.
pub static PRESETS: [Preset; 1] = [Preset {
    config: "ballfight_electric_vs_acoustic",
    fighters: ["ELECTRIC", "ACOUSTIC"],
    bpm: 126.0,
    key: "E",
    scale: "minor",
    intro: ["Em", "B7"],
    chord_loop: &LOOP,
    tail: &["Em", "C", "G", "D", "Am", "B7"],
}];

/// `PLUG` and `UNPLUG`: the two arrangements' buses.
pub const PLUG: [&str; 3] = ["p_gtr", "p_bass", "p_drums"];
pub const UNPLUG: [&str; 3] = ["u_gtr", "u_bass", "u_perc"];

/// `Bed.buses`: each bus's level and reverb send (the class's, which the lead envelope weights the arrangements by).
pub const BUSES: [(&str, (f64, f64)); 7] = [
    ("kick", (1.6, 0.12)),
    ("p_gtr", (1.3, 0.25)),
    ("p_bass", (1.6, 0.05)),
    ("p_drums", (1.45, 0.35)),
    ("u_gtr", (1.1, 0.3)),
    ("u_bass", (1.75, 0.08)),
    ("u_perc", (1.6, 0.3)),
];

/// `Bed.build_bars`: the bars of the build into the title slam.
pub const BUILD_BARS: usize = 2;

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

/// `Bed.root(ch)`: the power chords' root, `T.root_above(ch, 40)` (E2 to D#3).
///
/// # Errors
/// Where Python raises: `ch` does not parse.
pub fn root(harmony: &Harmony, ch: &str) -> Result<i32, HarmonyError> {
    harmony.root_above(ch, 40)
}

/// The strum's notes: `T.chord_tones(64, 79, ch)[:4] or T.chord_tones(60, 84, ch)[:4]` (the chord's tones from E4
/// up, at most four; a wider range when none falls there).
///
/// # Errors
/// Where Python raises: `ch` is not one of the episode's chords.
pub fn strum_tones(harmony: &Harmony, ch: &str) -> Result<Vec<i32>, HarmonyError> {
    let mut tones = harmony.chord_tones(64, 79, ch)?;
    tones.truncate(4);
    if tones.is_empty() {
        tones = harmony.chord_tones(60, 84, ch)?;
        tones.truncate(4);
    }
    Ok(tones)
}

/// `np.cumsum(np.full(n, f)) / SR`: a phase in cycles, one addition per sample.
fn phase(f: f64, n: usize) -> Vec<f64> {
    dsp::cumsum(&vec![f; n]).iter().map(|v| v / SRF).collect()
}

/// `2 * (ph % 1.0) - 1`: a naive saw (numpy's `%` of a positive phase is `fmod`).
fn saw(ph: f64) -> f64 {
    2.0 * (ph % 1.0) - 1.0
}

/// `Bed.pgtr(root, dur, mute, open_)` before the cache: the plugged rhythm guitar, a power chord (root, fifth,
/// octave; and the tenth when open), two detuned saws a note through a hard fuzz and a dark cab.
pub fn pgtr(root: i32, dur: f64, mute: bool, open: bool) -> Vec<f64> {
    let n = length((dur + 0.06) * SRF);
    let t = synth::tt(n);
    let mut x = vec![0.0; n];
    let mut notes = vec![root, root + 7, root + 12];
    if open {
        notes.push(root + 19);
    }
    for m in notes {
        let f = synth::mtof(f64::from(m));
        for (j, det) in [-0.0035, 0.0035].into_iter().enumerate() {
            let off = 0.31 * j as f64;
            for (o, p) in x.iter_mut().zip(phase(f * (1.0 + det), n)) {
                *o += saw(p + off);
            }
        }
    }
    let x: Vec<f64> = x.iter().map(|v| v / 6.0).collect();
    let x: Vec<f64> = synth::hp(&x, 90.0, FILTER_O)
        .iter()
        .map(|v| dmath::np_tanh(7.0 * v))
        .collect();
    let x = synth::lp(
        &synth::lp(&x, if mute { 850.0 } else { 2100.0 }, FILTER_O),
        2600.0,
        FILTER_O,
    );
    (0..n)
        .map(|i| {
            let t = t[i];
            let tail = if t < dur { 1.0 } else { dmath::exp(-(t - dur) / 0.03) };
            let mut env = clip(t / 0.004, 0.0, 1.0) * tail;
            if mute {
                env *= dmath::exp(-t / 0.08);
            }
            x[i] * env * 0.32
        })
        .collect()
}

/// `Bed.pbass(m, dur)` before the cache: the driving pick bass, a dark saw and a sine through a soft drive.
pub fn pbass(m: i32, dur: f64) -> Vec<f64> {
    let n = length((dur + 0.04) * SRF);
    let t = synth::tt(n);
    let ph = phase(synth::mtof(f64::from(m)), n);
    let saws: Vec<f64> = ph.iter().map(|&p| saw(p)).collect();
    let low = synth::lp(&saws, 650.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let x = low[i] * 0.7 + dmath::sin(2.0 * PI * ph[i]) * 0.7;
            let x = dmath::np_tanh(1.6 * x);
            let hold = if t < dur {
                0.85 + 0.15 * dmath::exp(-t / 0.05)
            } else {
                dmath::exp(-(t - dur) / 0.02)
            };
            x * (clip(t / 0.003, 0.0, 1.0) * hold) * 0.5
        })
        .collect()
}

/// `Bed.ubass(m, dur)` before the cache: an upright-ish bass, a round plucked tone with a finger thump (`m`'s
/// noise).
pub fn ubass(m: i32, dur: f64) -> Vec<f64> {
    let n = length((dur + 0.15) * SRF);
    let t = synth::tt(n);
    let f = synth::mtof(f64::from(m));
    let tone: Vec<f64> = t
        .iter()
        .map(|&t| {
            dmath::sin(2.0 * PI * f * t)
                + 0.45 * dmath::sin(4.0 * PI * f * t) * dmath::exp(-t / 0.12)
                + 0.18 * dmath::sin(6.0 * PI * f * t) * dmath::exp(-t / 0.06)
        })
        .collect();
    let tone = synth::lp(&tone, 700.0, FILTER_O);
    let seed = u64::try_from(m).expect("a noise seed (the note) from 0 up");
    let thump = synth::lp(&noise(seed, n), 300.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let x = tone[i] * dmath::exp(-t / 0.55) * clip(t / 0.004, 0.0, 1.0);
            let thump = thump[i] * dmath::exp(-t / 0.015) * 0.6;
            (x + thump) * 0.6
        })
        .collect()
}

/// `Bed.kick(1.0)` before the cache and its velocity: `A.hp(A.kick(1.0, 0.30, 140, 44), 35)`.
pub fn kick() -> Vec<f64> {
    synth::hp(&synth::kick(1.0, 0.30, 140.0, 44.0), 35.0, FILTER_O)
}

/// `Bed.snare(1.0)` before the cache: `A.snare(0.16)` with a big room's noise (seed 12).
pub fn snare() -> Vec<f64> {
    let x = synth::snare(0.16);
    let n = x.len();
    let t = synth::tt(n);
    let room = synth::lp(&noise(12, n), 2500.0, FILTER_O);
    (0..n)
        .map(|i| (x[i] + 0.4 * room[i] * dmath::exp(-t[i] / 0.05)) * 0.9)
        .collect()
}

/// `Bed.tom(m, 1.0)` before the cache: a tom whose pitch drops from half an octave over `m`, with a stick's noise
/// (`m`'s).
pub fn tom(m: i32) -> Vec<f64> {
    let n = length(0.5 * SRF);
    let t = synth::tt(n);
    let f0 = synth::mtof(f64::from(m));
    let f: Vec<f64> = t.iter().map(|&t| f0 * (1.0 + 0.5 * dmath::exp(-t / 0.03))).collect();
    let ph = dsp::cumsum(&f);
    let seed = u64::try_from(m).expect("a noise seed (the note) from 0 up");
    let stick = synth::bp(&noise(seed, n), 200.0, 3000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let x = dmath::sin(2.0 * PI * ph[i] / SRF) * dmath::exp(-t[i] / 0.22);
            x + 0.3 * stick[i] * dmath::exp(-t[i] / 0.01)
        })
        .collect()
}

/// `Bed.cajon(1.0)` before the cache: a slap (seed 21) over a 180 Hz body.
pub fn cajon() -> Vec<f64> {
    let n = length(0.25 * SRF);
    let t = synth::tt(n);
    let slap = synth::bp(&noise(21, n), 500.0, 4500.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let slap = slap[i] * dmath::exp(-t / 0.035);
            let body = dmath::sin(2.0 * PI * 180.0 * t) * dmath::exp(-t / 0.07);
            slap * 0.8 + body * 0.6
        })
        .collect()
}

/// `Bed.shaker(1.0)` before the cache: high-passed noise (seed 22) in a short swell.
pub fn shaker() -> Vec<f64> {
    let n = length(0.09 * SRF);
    let t = synth::tt(n);
    let hiss = synth::hp(&noise(22, n), 6000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            // np.maximum(t - 0.02, 0): the later of the two, 0 below it
            let after = t - 0.02;
            let after = if after >= 0.0 { after } else { 0.0 };
            let env = clip(t / 0.02, 0.0, 1.0) * dmath::exp(-after / 0.025);
            hiss[i] * env * 0.45
        })
        .collect()
}

/// The tambourine's jingles, in Hz.
const JINGLES: [f64; 3] = [5300.0, 7100.0, 9150.0];

/// `Bed.tamb(1.0)` before the cache: high-passed noise (seed 23) and three jingles (Python's `sum` from 0, each at
/// phase `f`).
pub fn tamb() -> Vec<f64> {
    let n = length(0.3 * SRF);
    let t = synth::tt(n);
    let hiss = synth::hp(&noise(23, n), 7000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            let mut jing = 0.0;
            for f in JINGLES {
                jing += dmath::sin(2.0 * PI * f * t + f);
            }
            let jing = jing * dmath::exp(-t / 0.07) * 0.15;
            hiss[i] * dmath::exp(-t / 0.06) * 0.5 + jing
        })
        .collect()
}

/// The count-in's stick click (seed 31), `int(0.05 * SR)` samples.
pub fn stick() -> Vec<f64> {
    let n = length(0.05 * SRF);
    let t = synth::tt(n);
    let click = synth::bp(&noise(31, n), 2000.0, 7000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let t = t[i];
            click[i] * dmath::exp(-t / 0.008) + dmath::sin(2.0 * PI * 2400.0 * t) * dmath::exp(-t / 0.01) * 0.5
        })
        .collect()
}

/// The BS.1770 K-weighting's first stage (the high shelf), `b1, a1`.
const SHELF: ([f64; 3], [f64; 3]) = (
    [1.53512485958697, -2.69169618940638, 1.19839281085285],
    [1.0, -1.69065929318241, 0.73248077421585],
);
/// Its second stage (the high-pass), `b2, a2`.
const HIGH_PASS: ([f64; 3], [f64; 3]) = ([1.0, -2.0, 1.0], [1.0, -1.99004745483398, 0.99007225036621]);

/// `Bed.lufs(x, a, b)`: the K-weighted loudness of `x[int(a * SR):int(b * SR)]`, `-0.691 + 10 * np.log10((y **
/// 2).mean(0).sum() + 1e-12)` with `y` the stretch through both stages (`scipy.signal.lfilter`, each channel on its
/// own). numpy's mean down the rows adds each channel in order. NaN for an empty stretch, as numpy's mean.
pub fn lufs(x: &[[f64; 2]], a: f64, b: f64) -> f64 {
    let i0 = length(a * SRF).min(x.len());
    let i1 = length(b * SRF).min(x.len()).max(i0);
    let y = dsp::lfilter_stereo(&SHELF.0, &SHELF.1, &x[i0..i1]);
    let y = dsp::lfilter_stereo(&HIGH_PASS.0, &HIGH_PASS.1, &y);
    let squares: Vec<[f64; 2]> = y.iter().map(|f| [f[0] * f[0], f[1] * f[1]]).collect();
    let [l, r] = dsp::sum_axis0(&squares);
    let n = y.len() as f64;
    -0.691 + 10.0 * dmath::log10((l / n + r / n) + 1e-12)
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{chords_for, scale};

    use super::*;

    fn harmony(chords: &[&str]) -> Harmony {
        let g = Grid::new(126.0);
        let all: Vec<String> = chords.iter().map(|s| s.to_string()).collect();
        let bars = chords_for(&all[..2.min(all.len())], &all, &[], g.n_bars);
        Harmony::new(&g, "E", scale("minor").unwrap(), bars, Some(all), 2).unwrap()
    }

    /// #44's chords: power-chord roots from E2 up, the strum's tones from E4 up.
    #[test]
    fn roots_and_strums_follow_the_rule() {
        let h = harmony(&["Em", "C", "G", "D", "Am", "B7"]);
        assert_eq!(root(&h, "Em").unwrap(), 40);
        assert_eq!(root(&h, "D").unwrap(), 50);
        assert_eq!(root(&h, "C").unwrap(), 48);
        assert_eq!(strum_tones(&h, "Em").unwrap(), [64, 67, 71, 76]);
        assert_eq!(strum_tones(&h, "B7").unwrap(), [66, 69, 71, 75]);
        assert_eq!(strum_tones(&h, "F#m"), Err(HarmonyError::UnknownChord("F#m".into())));
    }

    #[test]
    fn instrument_lengths_follow_pythons_int() {
        assert_eq!(pgtr(40, 0.2, true, false).len(), 12480);
        assert_eq!((pbass(28, 0.2).len(), ubass(28, 0.2).len()), (11520, 16800));
        assert_eq!((kick().len(), snare().len(), tom(50).len()), (24000, 16800, 24000));
        assert_eq!((cajon().len(), shaker().len(), tamb().len()), (12000, 4320, 14400));
        assert_eq!(stick().len(), 2400);
    }

    #[test]
    fn the_option_defaults_to_the_left_corner() {
        let mut o = ThemeOptions::new();
        assert_eq!(plugged(&o), None);
        o.insert("plugged".into(), OptionValue::Str("ACOUSTIC".into()));
        assert_eq!(plugged(&o).as_deref(), Some("ACOUSTIC"));
    }

    /// Silence is NaN-free: the floor of 1e-12.
    #[test]
    fn the_loudness_of_silence_is_the_floor() {
        let x = vec![[0.0; 2]; 48000];
        assert_eq!(lufs(&x, 0.1, 0.9), -0.691 + 10.0 * dmath::log10(1e-12));
    }
}
