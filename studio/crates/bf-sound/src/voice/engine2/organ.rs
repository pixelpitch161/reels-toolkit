//! A pipe organ's working parts, around its recipe (`docs/studio/voice-engine-2-instruments.md`, "Organ").
//!
//! - **Registration**: an ordinary note plays the main chorus (the loud recording). A swell brings the stops in: the
//!   note starts on the quiet recording and moves to the loud one tone by tone, which is how an organist builds up.
//! - **Wind**: the pipes speak on wind from one bellows, which wavers a hair, slowly, for every pipe at once; a big
//!   chord draws enough wind to sag its pitch and level a little.
//! - **Stop**: when the key comes up the pallet closes and the pipes stop within a few hundredths of a second. The
//!   church's own echo after it is left out; the organ's light room of its own comes after (`space`).

use bf_core::dmath;

use super::play::{Mute, Play};
use super::player::{self, Carry, Shaping};
use super::recipe::RecipeSet;

/// The organ's settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrganParts {
    /// How far the wind's waver moves the pitch, cents either way.
    pub wind_cents: f64,
    /// How far it moves the level, dB either way.
    pub wind_db: f64,
    /// About how fast it wavers, Hz.
    pub wind_hz: f64,
    /// How far a big chord sags the pitch at most, cents.
    pub sag_cents: f64,
    /// The pipes' stop, its time constant kept between these, seconds.
    pub release: (f64, f64),
}

/// The wind's waver at `t` seconds into a note, -1 to 1: three slow sines, the same for every note, so notes that
/// start together waver together.
pub fn waver(t: f64, hz: f64) -> f64 {
    let a = dmath::sin(dmath::TAU * 0.76 * hz * t + 0.4);
    let b = dmath::sin(dmath::TAU * 1.01 * hz * t + 2.1);
    let c = dmath::sin(dmath::TAU * 1.61 * hz * t + 4.4);
    (0.5 * a + 0.3 * b + 0.2 * c).clamp(-1.0, 1.0)
}

fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The note `play` on the organ of `set`, `len` samples long.
pub fn note(set: &RecipeSet, p: &OrganParts, play: &Play, len: usize, variant: u64) -> Vec<f64> {
    let top = set.layers.last().map_or(1.0, |l| l.strength);
    let swell = play.swell;
    let strength = move |t: f64| match swell {
        Some(s) if s.over > 0.0 => s.from + (s.to - s.from) * smooth(t / s.over),
        Some(s) => s.to,
        None => top,
    };
    let sag = if play.together >= 3 {
        p.sag_cents.min(0.8 * f64::from(play.together - 2))
    } else {
        0.0
    };
    let (wc, wd, wh) = (p.wind_cents, p.wind_db, p.wind_hz);
    let pitch = move |t: f64| wc * waver(t, wh) - sag * smooth(t / 0.06);
    let vel_db = 20.0 * dmath::log10(play.vel.max(0.01));
    let sag_db = if sag > 0.0 { 0.3 } else { 0.0 };
    let level = move |t: f64| vel_db + wd * waver(t + 0.37, wh) - sag_db * smooth(t / 0.06);
    let tilt = |_: f64| 0.0;
    let noise = |_: f64| 0.0;
    let c = set.nearest(play.midi, top);
    let recorded: f64 = c.iter().map(|(i, w)| w * set.notes[i].release).sum();
    let tau = match play.mute {
        Mute::Clipped | Mute::Choked => p.release.0,
        _ => recorded.clamp(p.release.0, p.release.1),
    };
    let sh = Shaping {
        strength: &strength,
        pitch_cents: &pitch,
        level_db: &level,
        tilt_db: &tilt,
        noise_db: &noise,
        stop: Some((play.dur.max(0.0), tau)),
        carry: Carry::Steady,
    };
    let seed = 0x0e9a_0000 + (play.midi * 100.0).round() as u64;
    player::render(set, play.midi, &sh, len, variant, seed)
}

#[cfg(test)]
mod tests {
    use super::super::play::Swell;
    use super::*;
    use crate::SRF;

    fn organ() -> RecipeSet {
        RecipeSet::parse(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../bf-content/data/voices/ORGAN.ve2"
        )))
        .expect("the organ's recipe")
    }

    const PARTS: OrganParts = OrganParts {
        wind_cents: 1.5,
        wind_db: 0.25,
        wind_hz: 0.7,
        sag_cents: 4.0,
        release: (0.03, 0.12),
    };

    fn db_at(x: &[f64], t: f64) -> f64 {
        let (a, b) = (((t - 0.01) * SRF) as usize, ((t + 0.01) * SRF) as usize);
        10.0 * dmath::log10(x[a..b].iter().map(|v| v * v).sum::<f64>() / (b - a) as f64 + 1e-30)
    }

    /// The Hann-windowed sum of `x` against the frequency `f` over `c - h .. c + h`, as (cos, sin) parts.
    fn component(x: &[f64], f: f64, c: usize, h: usize) -> (f64, f64) {
        let (mut re, mut im) = (0.0, 0.0);
        for (n, v) in x.iter().enumerate().take(c + h).skip(c - h) {
            let w = 0.5 - 0.5 * dmath::cos(dmath::TAU * (n + h - c) as f64 / (2 * h) as f64);
            let ph = dmath::TAU * (f * n as f64 / SRF).fract();
            re += v * w * dmath::cos(ph);
            im += v * w * dmath::sin(ph);
        }
        (re, im)
    }

    /// The levels of the note's first eight overtones over a 0.2 s window at `t`, dB under the strongest.
    fn balance(x: &[f64], f0: f64, t: f64) -> Vec<f64> {
        let (c, h) = ((t * SRF) as usize, 4800usize);
        let lv: Vec<f64> = (1..=8)
            .map(|k| {
                let (re, im) = component(x, f0 * f64::from(k), c, h);
                10.0 * dmath::log10(re * re + im * im + 1e-30)
            })
            .collect();
        let top = lv.iter().copied().fold(f64::MIN, f64::max);
        lv.iter().map(|v| v - top).collect()
    }

    #[test]
    fn a_swell_builds_from_the_quiet_chorus_to_the_loud() {
        let set = organ();
        let f0 = 261.625_565;
        let swell = Play {
            swell: Some(Swell {
                from: 0.5,
                to: 1.0,
                over: 1.5,
            }),
            ..Play::plain(60.0, 2.0, 0.8)
        };
        let x = note(&set, &PARTS, &swell, 2 * 48_000, 0);
        let quiet = note(
            &set,
            &PARTS,
            &Play {
                swell: Some(Swell {
                    from: 0.5,
                    to: 0.5,
                    over: 0.0,
                }),
                ..swell.clone()
            },
            96_000,
            0,
        );
        let loud = note(&set, &PARTS, &Play::plain(60.0, 2.0, 0.8), 96_000, 0);
        let close = |a: &[f64], b: &[f64]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1.5 || *x < -30.0);
        assert!(close(&balance(&x, f0, 0.15), &balance(&quiet, f0, 0.15)));
        assert!(close(&balance(&x, f0, 1.75), &balance(&loud, f0, 1.75)));
    }

    #[test]
    fn the_pipes_stop_quickly() {
        let set = organ();
        let x = note(&set, &PARTS, &Play::plain(60.0, 0.5, 0.8), 48_000, 0);
        assert!(
            db_at(&x, 0.45) - db_at(&x, 0.9) >= 30.0,
            "{} {}",
            db_at(&x, 0.45),
            db_at(&x, 0.9)
        );
    }

    #[test]
    fn the_wind_wavers_slowly_and_together() {
        let t: Vec<f64> = (0..200).map(|i| f64::from(i) * 0.01).collect();
        let a: Vec<f64> = t.iter().map(|&s| waver(s, 0.7)).collect();
        assert!(a.iter().all(|v| v.abs() <= 1.0));
        // slow: it moves less than a tenth of its range in 10 ms
        assert!(a.windows(2).all(|w| (w[1] - w[0]).abs() < 0.2));
        // a hair: under 2 cents
        let cents = PARTS.wind_cents;
        assert!(cents < 2.0, "{cents}");
    }

    #[test]
    fn a_big_chord_sags_a_hair() {
        let set = organ();
        let one = Play::plain(60.0, 1.0, 0.8);
        let six = Play {
            together: 6,
            ..one.clone()
        };
        let (a, b) = (note(&set, &PARTS, &one, 48_000, 0), note(&set, &PARTS, &six, 48_000, 0));
        // the sagging note's 8th overtone lags behind: compare the phase drift over 0.2 s to 0.4 s
        let f = 261.625_565 * 8.0;
        let phase = |x: &[f64], t: f64| {
            let (re, im) = component(x, f, (t * SRF) as usize, 2400);
            dmath::atan2(im, re)
        };
        // the phase the 8th overtone gains over 10 ms: 4 cents flat at 2.09 kHz is 4.8 Hz less, 0.30 rad less
        let gain = |x: &[f64]| {
            let d = phase(x, 0.31) - phase(x, 0.3);
            (d + 3.0 * dmath::TAU) % dmath::TAU
        };
        let mut diff = gain(&b) - gain(&a);
        if diff > dmath::PI {
            diff -= dmath::TAU;
        } else if diff < -dmath::PI {
            diff += dmath::TAU;
        }
        let want = dmath::TAU * f * (dmath::pow(2.0, -4.0 / 1200.0) - 1.0) * 0.01;
        assert!((diff - want).abs() < 0.06, "{diff} {want}");
    }
}
