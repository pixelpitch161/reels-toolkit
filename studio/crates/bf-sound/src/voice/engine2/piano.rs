//! A grand piano's working parts, around its recipe (`docs/studio/voice-engine-2-instruments.md`, "Piano").
//!
//! - **Strength**: how hard the key is struck picks between the recorded strengths, which brighten as they get louder
//!   (the hammer's felt hardens the harder it is driven). Harder than the hardest recording, the tone goes on
//!   brightening and getting louder at the rate between the two hardest. An accent strikes harder and makes more of
//!   the hammer's knock.
//! - **Damper**: when the key comes up, the damper's felt lands on the strings and the note dies with the time
//!   constant measured from the recordings of the dampers landing. The top 18 keys have no dampers on a grand and
//!   ring on.
//! - **Pedal**: with the sustain pedal down there is no damping, and the other strings, all free, ring along softly
//!   with the note's overtones.

use bf_core::dmath;

use super::play::{Mute, Play};
use super::player::{self, Carry, Shaping, zero};
use super::recipe::RecipeSet;
use crate::SRF;

/// The piano's settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PianoParts {
    /// The lowest MIDI note with no damper (91: the top 18 keys of 88).
    pub undamped_from: i32,
    /// How loud the strings ringing along with the pedal are, dB under the note's overtone at 0.3 s.
    pub halo_db: f64,
}

/// How much harder an accent strikes, in strength.
const ACCENT: f64 = 0.12;
/// How much more of the hammer's knock a full accent makes over the note's first 50 ms, dB.
const ACCENT_KNOCK_DB: f64 = 3.0;
/// Past the hardest recording: brighter by this many dB per octave of overtone for each 0.1 of strength.
const OVER_TILT: f64 = 1.0;
/// The damper's time constant where the recipe has none, seconds.
const DAMPER: f64 = 0.07;

/// The strength a note is struck with.
pub fn strength(play: &Play) -> f64 {
    (play.vel + ACCENT * play.accent).clamp(0.2, 1.1)
}

/// The damper's time constant at `midi`, from the recorded notes nearest it.
fn damper(set: &RecipeSet, midi: f64, strength: f64) -> f64 {
    let c = set.nearest(midi, strength);
    let (mut t, mut w) = (0.0, 0.0);
    for (i, wi) in c.iter() {
        let r = set.notes[i].release;
        if r > 0.0 {
            t += wi * r;
            w += wi;
        }
    }
    if w > 0.0 { t / w } else { DAMPER }
}

/// The note `play` on the piano of `set`, `len` samples long.
pub fn note(set: &RecipeSet, p: &PianoParts, play: &Play, len: usize, variant: u64) -> Vec<f64> {
    let s = strength(play);
    let top = set.layers.last().map_or(1.0, |l| l.strength);
    let over = (s - top).max(0.0);
    // the loudness rate between the two hardest recordings, carried on past the hardest
    let rate = match set.layers.len() {
        n if n >= 2 => {
            let (a, b) = (&set.layers[n - 2], &set.layers[n - 1]);
            ((b.gain_db - a.gain_db) / (b.strength - a.strength).max(1e-6)).max(0.0)
        }
        _ => 0.0,
    };
    let (mut level, mut tilt) = (rate * over, OVER_TILT * over * 10.0);
    if play.mute == Mute::Muted {
        // the soft pedal: one string fewer struck, a softer felt
        level -= 3.0;
        tilt -= 1.5;
    }
    let accent = play.accent.clamp(0.0, 1.0);
    let st = move |_: f64| s;
    let lv = move |_: f64| level;
    let tl = move |_: f64| tilt;
    let knock = move |t: f64| if t < 0.05 { ACCENT_KNOCK_DB * accent } else { 0.0 };
    let damped = play.midi < f64::from(p.undamped_from);
    let tau = damper(set, play.midi, s);
    let stop = match play.mute {
        Mute::Choked | Mute::Clipped => Some((play.dur.max(0.0), tau * 0.5)),
        _ if play.pedal || !damped => None,
        _ => Some((play.dur.max(0.0), tau)),
    };
    let sh = Shaping {
        strength: &st,
        pitch_cents: &zero,
        level_db: &lv,
        tilt_db: &tl,
        noise_db: &knock,
        stop,
        carry: Carry::Decay,
    };
    let seed = 0x9147_0000 + (play.midi * 100.0).round() as u64;
    let mut x = player::render(set, play.midi, &sh, len, variant, seed);
    if play.pedal && stop.is_none() {
        halo(set, p, play, s, &mut x);
    }
    x
}

/// The other strings ringing along under the pedal: the note's overtones 2 to 6, each `halo_db` under the note's at
/// 0.3 s, coming up over 0.15 s and dying at the note's own late rate.
fn halo(set: &RecipeSet, p: &PianoParts, play: &Play, s: f64, x: &mut [f64]) {
    let c = set.nearest(play.midi, s);
    let Some((i, _)) = c.iter().max_by(|a, b| a.1.total_cmp(&b.1)) else {
        return;
    };
    let n = &set.notes[i];
    let g = &set.grid;
    let at = |t: f64| g.partition_point(|&v| v <= t).saturating_sub(1);
    let base = 440.0 * dmath::exp((play.midi - 69.0) / 12.0 * std::f64::consts::LN_2);
    let gain = c
        .iter()
        .map(|(j, w)| w * set.layers[set.notes[j].layer].gain_db)
        .sum::<f64>();
    for k in 2..=6_u16 {
        let Some(part) = n.partials.iter().find(|q| q.number == k) else {
            continue;
        };
        let (a, b) = (f64::from(part.level_at(at(0.3))), f64::from(part.level_at(at(2.0))));
        if !a.is_finite() {
            continue;
        }
        let fall = if b.is_finite() { ((b - a) / 1.7).min(0.0) } else { -6.0 };
        let f = base * f64::from(k);
        if f >= player::TOP_HZ {
            continue;
        }
        let w = dmath::TAU * f / SRF;
        let (rc, rs) = (dmath::cos(w), dmath::sin(w));
        let (mut zr, mut zi) = (1.0, 0.0);
        let amp0 = dmath::exp((a + gain + p.halo_db) * std::f64::consts::LN_10 / 20.0);
        let mut i0 = 0;
        while i0 < x.len() {
            let t = i0 as f64 / SRF;
            let rise = 1.0 - dmath::exp(-t / 0.05);
            let amp = amp0 * rise * dmath::exp(fall * t * std::f64::consts::LN_10 / 20.0);
            for y in x.iter_mut().skip(i0).take(player::BLOCK) {
                *y += amp * zi;
                (zr, zi) = (zr * rc - zi * rs, zr * rs + zi * rc);
            }
            let m = (zr * zr + zi * zi).sqrt();
            (zr, zi) = (zr / m, zi / m);
            i0 += player::BLOCK;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piano() -> RecipeSet {
        RecipeSet::parse(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../bf-content/data/voices/PIANO.ve2"
        )))
        .expect("the piano's recipe")
    }

    const PARTS: PianoParts = PianoParts {
        undamped_from: 91,
        halo_db: -34.0,
    };

    fn db_at(x: &[f64], t: f64) -> f64 {
        let (a, b) = (((t - 0.01) * SRF) as usize, ((t + 0.01) * SRF) as usize);
        let ms = x[a..b].iter().map(|v| v * v).sum::<f64>() / (b - a) as f64;
        10.0 * dmath::log10(ms + 1e-30)
    }

    fn centroid(x: &[f64]) -> f64 {
        let n = 16_384;
        let fft = crate::dsp::fft::Fft::new(n);
        let (mut re, mut im) = (x[2400..2400 + n].to_vec(), vec![0.0; n]);
        fft.forward(&mut re, &mut im);
        let (mut num, mut den) = (0.0, 0.0);
        for k in 1..n / 2 {
            let p = re[k] * re[k] + im[k] * im[k];
            num += p * k as f64;
            den += p;
        }
        num / den * SRF / n as f64
    }

    #[test]
    fn the_damper_stops_a_note_at_key_up() {
        let set = piano();
        let x = note(&set, &PARTS, &Play::plain(60.0, 0.3, 0.8), 48_000, 0);
        assert!(
            db_at(&x, 0.29) - db_at(&x, 0.7) > 40.0,
            "{} {}",
            db_at(&x, 0.29),
            db_at(&x, 0.7)
        );
        // a top key has no damper: let go at 0.3 s it dies exactly as if held (on its own it falls fast, about
        // 24 dB from 0.3 s to 0.7 s at C7, as a real top string does)
        let y = note(&set, &PARTS, &Play::plain(96.0, 0.3, 0.8), 48_000, 0);
        let held = note(&set, &PARTS, &Play::plain(96.0, 4.0, 0.8), 48_000, 0);
        assert!(
            (db_at(&y, 0.7) - db_at(&held, 0.7)).abs() < 0.1,
            "{} {}",
            db_at(&y, 0.7),
            db_at(&held, 0.7)
        );
    }

    #[test]
    fn the_pedal_keeps_it_ringing() {
        let set = piano();
        let pedal = note(
            &set,
            &PARTS,
            &Play {
                pedal: true,
                ..Play::plain(60.0, 0.3, 0.8)
            },
            96_000,
            0,
        );
        let held = note(&set, &PARTS, &Play::plain(60.0, 3.0, 0.8), 96_000, 0);
        assert!((db_at(&pedal, 1.5) - db_at(&held, 1.5)).abs() < 6.0);
    }

    #[test]
    fn harder_is_louder_and_brighter() {
        let set = piano();
        let soft = note(&set, &PARTS, &Play::plain(60.0, 1.0, 0.5), 48_000, 0);
        let hard = note(&set, &PARTS, &Play::plain(60.0, 1.0, 1.0), 48_000, 0);
        assert!(db_at(&hard, 0.1) - db_at(&soft, 0.1) > 6.0);
        assert!(centroid(&hard) > centroid(&soft));
    }

    #[test]
    fn an_accent_starts_harder() {
        let set = piano();
        let plain = note(&set, &PARTS, &Play::plain(60.0, 1.0, 0.7), 4800, 0);
        let accented = note(
            &set,
            &PARTS,
            &Play {
                accent: 1.0,
                ..Play::plain(60.0, 1.0, 0.7)
            },
            4800,
            0,
        );
        let e = |x: &[f64]| x[..2400].iter().map(|v| v * v).sum::<f64>();
        assert!(e(&accented) > e(&plain));
    }
}
