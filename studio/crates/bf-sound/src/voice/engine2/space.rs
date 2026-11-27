//! Where a Voice Engine 2 voice sits in the stereo picture, and its own room: applied to its bus by the Composed
//! polish (`crate::polish`).

use bf_core::rng::Generator;

use super::amp::Amp;
use crate::dsp::fft::Fft;
use crate::dsp::{self, FilterKind, butter_sos};
use crate::polish::room;
use crate::{SRF, heard};

/// A voice's place in the stereo picture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Space {
    /// How wide it spreads, 0 (a point) to about 0.5.
    pub width: f64,
    /// How far its bass leans left and its treble right, as a player hears a piano from the bench (0 for none).
    pub tilt: f64,
    /// Its own room, if it has one.
    pub home: Option<Home>,
    /// The amp its whole sound plays through (an electric's, a bass's), if it has one.
    pub amp: Option<Amp>,
}

/// A voice's own room: `length` seconds long, `under` LU under the dry sound, high-passed at `hp` Hz so it never
/// muddies the bass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Home {
    pub length: f64,
    pub under: f64,
    pub hp: f64,
}

/// Where the mid's low end and top end are split for the tilt, and where the spread starts, in Hz.
const TILT_LOW: f64 = 250.0;
const TILT_HIGH: f64 = 2000.0;
const SPREAD_FROM: f64 = 250.0;
/// The spread's four all-passes: each delay seeded between these, in seconds, each with this gain.
const ALLPASS: (f64, f64) = (0.003, 0.011);
const ALLPASS_GAIN: f64 = 0.5;

/// `x` through a Schroeder all-pass of `delay` samples and gain `g`: every frequency at the same level, each
/// arriving at its own time.
fn allpass(x: &[f64], delay: usize, g: f64) -> Vec<f64> {
    let mut y = vec![0.0; x.len()];
    for i in 0..x.len() {
        let (xd, yd) = if i >= delay {
            (x[i - delay], y[i - delay])
        } else {
            (0.0, 0.0)
        };
        y[i] = -g * x[i] + xd + g * yd;
    }
    y
}

/// `x` spread and leaned: `d` is added to the left and taken from the right, so left plus right (what a phone
/// playing one speaker sums) is unchanged. `d` is `width` times the mid made unlike itself (four seeded all-passes,
/// above 250 Hz only, so the bass stays in the middle) plus `tilt` times the mid's bass less its treble (under 250 Hz
/// against over 2 kHz): the bass a little left and the treble a little right, as from a piano's bench.
pub fn widen(x: &[[f64; 2]], width: f64, tilt: f64, seed: u64) -> Vec<[f64; 2]> {
    if (width == 0.0 && tilt == 0.0) || x.is_empty() {
        return x.to_vec();
    }
    let mid: Vec<f64> = x.iter().map(|f| 0.5 * (f[0] + f[1])).collect();
    let sos = |kind, hz: f64| butter_sos(kind, &[hz], 2, SRF);
    let mut d = vec![0.0; x.len()];
    if width != 0.0 {
        let mut r = Generator::new(seed);
        let mut side = mid.clone();
        for _ in 0..4 {
            let delay = (r.uniform(ALLPASS.0, ALLPASS.1) * SRF) as usize;
            side = allpass(&side, delay.max(1), ALLPASS_GAIN);
        }
        let side = dsp::sosfilt(&sos(FilterKind::Hp, SPREAD_FROM), &side);
        for (o, s) in d.iter_mut().zip(side) {
            *o += width * s;
        }
    }
    if tilt != 0.0 {
        let low = dsp::sosfilt(&sos(FilterKind::Lp, TILT_LOW), &mid);
        let high = dsp::sosfilt(&sos(FilterKind::Hp, TILT_HIGH), &mid);
        for ((o, l), h) in d.iter_mut().zip(low).zip(high) {
            *o += tilt * (l - h);
        }
    }
    x.iter().zip(d).map(|(f, d)| [f[0] + d, f[1] - d]).collect()
}

/// `x` with its own room added: a room `h.length` seconds long (its middle fading by 60 dB in 0.55 of that),
/// high-passed at `h.hp` Hz (fourth order: the pedal stays out of it), `h.under` LU under `x`. Silence stays silence.
pub fn home(x: &[[f64; 2]], h: &Home, seed: u64) -> Vec<[f64; 2]> {
    // the shared room's sound at this room's length, high-passed before it is used (the same as filtering what
    // comes out, to the filter's own few milliseconds past the room's end, and far cheaper than filtering a whole
    // fight's bus), then convolved by overlap-add, the two sides and the dry level at once (a fight's bus is
    // minutes long: one transform of the whole, or one side after the other, costs seconds a bus)
    let hp = butter_sos(FilterKind::Hp, &[h.hp], 4, SRF);
    let ir = room::ir_with(seed, h.length, h.length * 0.55);
    let side = |c: usize| dsp::sosfilt(&hp, &ir.iter().map(|f| f[c]).collect::<Vec<f64>>());
    let (wet, dry_level) = std::thread::scope(|s| {
        let dry_level = s.spawn(|| heard::integrated(x));
        let wet = convolve_room(x, [&side(0), &side(1)]);
        (wet, dry_level.join().expect("the dry level"))
    });
    let wet_level = heard::integrated(&wet);
    if dry_level <= -70.0 || wet_level <= -70.0 {
        return x.to_vec();
    }
    let g = heard::gain(dry_level - h.under - wet_level);
    x.iter()
        .zip(&wet)
        .map(|(f, w)| [f[0] + g * w[0], f[1] + g * w[1]])
        .collect()
}

/// The first `x.len()` samples of `x`'s two sides each convolved with its own side of the room `ir`, by overlap-add
/// in blocks: both sides in one complex transform (the left the real part, the right the imaginary, pulled apart by
/// the transform's symmetry), the blocks shared out between two threads, a block of silence passed over. The
/// convolution [`dsp::oaconvolve`] gives each side, to rounding, at a quarter of the transforms.
fn convolve_room(x: &[[f64; 2]], ir: [&[f64]; 2]) -> Vec<[f64; 2]> {
    let k = ir[0].len().max(ir[1].len());
    if x.is_empty() || k == 0 {
        return vec![[0.0; 2]; x.len()];
    }
    let size = (4 * k).next_power_of_two().max(1024);
    let step = size - k + 1;
    let fft = Fft::new(size);
    let spectrum = |h: &[f64]| {
        let (mut re, mut im) = (vec![0.0; size], vec![0.0; size]);
        re[..h.len()].copy_from_slice(h);
        fft.forward(&mut re, &mut im);
        (re, im)
    };
    let (hl, hr) = (spectrum(ir[0]), spectrum(ir[1]));
    let starts: Vec<usize> = (0..x.len()).step_by(step).collect();
    // the blocks from `from` to `to` (by number), added into their own stretch of the output from `starts[from]`
    let blocks = |from: usize, to: usize| {
        let at = starts.get(from).copied().unwrap_or(x.len());
        let end = if to > from {
            (starts[to - 1] + size).min(x.len())
        } else {
            at
        };
        let mut out = vec![[0.0; 2]; end - at];
        let (mut re, mut im) = (vec![0.0; size], vec![0.0; size]);
        let (mut zr, mut zi) = (vec![0.0; size], vec![0.0; size]);
        for &start in &starts[from..to] {
            let block = &x[start..(start + step).min(x.len())];
            if block.iter().all(|f| f[0] == 0.0 && f[1] == 0.0) {
                continue;
            }
            re.fill(0.0);
            im.fill(0.0);
            for (i, f) in block.iter().enumerate() {
                (re[i], im[i]) = (f[0], f[1]);
            }
            fft.forward(&mut re, &mut im);
            for j in 0..size {
                // Y[j] and the conjugate of Y[-j]: the left's spectrum is their mean, the right's their difference
                // over 2i
                let m = (size - j) % size;
                let (a, b, c, d) = (re[j], im[j], re[m], im[m]);
                let (lr, li) = (0.5 * (a + c), 0.5 * (b - d));
                let (rr, ri) = (0.5 * (b + d), 0.5 * (c - a));
                let (pl, ql) = (lr * hl.0[j] - li * hl.1[j], lr * hl.1[j] + li * hl.0[j]);
                let (pr, qr) = (rr * hr.0[j] - ri * hr.1[j], rr * hr.1[j] + ri * hr.0[j]);
                // the left's product plus i times the right's
                (zr[j], zi[j]) = (pl - qr, ql + pr);
            }
            fft.inverse(&mut zr, &mut zi);
            let m = (block.len() + k - 1).min(x.len() - start);
            for (o, (l, r)) in out[start - at..start - at + m].iter_mut().zip(zr.iter().zip(&zi)) {
                o[0] += l;
                o[1] += r;
            }
        }
        (at, out)
    };
    let half = starts.len().div_ceil(2);
    let (first, second) = std::thread::scope(|s| {
        let second = s.spawn(|| blocks(half, starts.len()));
        let first = blocks(0, half);
        (first, second.join().expect("the room's later blocks"))
    });
    let mut y = first.1;
    y.resize(x.len(), [0.0; 2]);
    for (o, v) in y[second.0..].iter_mut().zip(&second.1) {
        o[0] += v[0];
        o[1] += v[1];
    }
    y
}

#[cfg(test)]
mod tests {
    use super::*;
    use bf_core::dmath;

    fn noise(seed: u64, n: usize) -> Vec<f64> {
        let mut z = vec![0.0; n];
        Generator::new(seed).fill_normal(0.0, 0.3, &mut z);
        z
    }

    fn energy(x: impl Iterator<Item = f64>) -> f64 {
        x.map(|v| v * v).sum()
    }

    #[test]
    fn widening_keeps_the_mono_sum() {
        let (a, b) = (noise(1, 48_000), noise(2, 48_000));
        let x: Vec<[f64; 2]> = a.iter().zip(&b).map(|(&l, &r)| [l, 0.6 * l + 0.4 * r]).collect();
        let y = widen(&x, 0.35, 0.12, 9);
        for (f, g) in x.iter().zip(&y) {
            assert!(((f[0] + f[1]) - (g[0] + g[1])).abs() < 1e-12);
        }
        // a sound in the middle comes out with some side to it
        let mono: Vec<[f64; 2]> = a.iter().map(|&v| [v, v]).collect();
        let w = widen(&mono, 0.35, 0.0, 9);
        let side = energy(w.iter().map(|f| 0.5 * (f[0] - f[1])));
        let all = energy(w.iter().flat_map(|f| f.iter().copied())) / 2.0;
        assert!(side / all > 0.01, "{}", side / all);
        assert_eq!(widen(&x, 0.0, 0.0, 9), x);
        assert_eq!(widen(&x, 0.35, 0.12, 9), y);
    }

    #[test]
    fn the_tilt_puts_the_bass_left_and_the_treble_right() {
        let tone = |hz: f64| -> Vec<[f64; 2]> {
            (0..48_000)
                .map(|i| {
                    let v = dmath::sin(dmath::TAU * hz * f64::from(i) / SRF);
                    [v, v]
                })
                .collect()
        };
        let sides = |x: &[[f64; 2]]| {
            let x = &x[12_000..];
            (energy(x.iter().map(|f| f[0])), energy(x.iter().map(|f| f[1])))
        };
        let (l, r) = sides(&widen(&tone(100.0), 0.0, 0.12, 9));
        assert!(l > r * 1.25, "{l} {r}");
        let (l, r) = sides(&widen(&tone(4000.0), 0.0, 0.12, 9));
        assert!(r > l * 1.25, "{l} {r}");
    }

    #[test]
    fn the_home_room_is_light_and_high() {
        let h = Home {
            length: 1.8,
            under: 14.0,
            hp: 200.0,
        };
        // a held organ chord, the pedal's C2 under C4, E4, G4 and C5 (each with eight overtones), then four seconds
        // of quiet: the pedal stays out of the room
        let n = 6 * 48_000;
        let x: Vec<[f64; 2]> = (0..n)
            .map(|i| {
                let t = i as f64 / SRF;
                if t >= 2.0 {
                    return [0.0, 0.0];
                }
                let v: f64 = [36.0, 60.0, 64.0, 67.0, 72.0_f64]
                    .iter()
                    .map(|&m| {
                        let f = 440.0 * dmath::pow(2.0, (m - 69.0) / 12.0);
                        (1..=8)
                            .map(|k| dmath::sin(dmath::TAU * f * f64::from(k) * t) / f64::from(k))
                            .sum::<f64>()
                    })
                    .sum();
                [0.1 * v, 0.1 * v]
            })
            .collect();
        let y = home(&x, &h, 3);
        let room: Vec<[f64; 2]> = x.iter().zip(&y).map(|(a, b)| [b[0] - a[0], b[1] - a[1]]).collect();
        let under = heard::integrated(&x) - heard::integrated(&room);
        assert!((under - h.under).abs() <= 1.0, "{under}");
        assert!(
            heard::share(&room, heard::RUMBLE) < 0.05,
            "{}",
            heard::share(&room, heard::RUMBLE)
        );
        let rms_db = |t: f64| {
            let a = (t * SRF) as usize;
            let s = &room[a..a + 960];
            10.0 * dmath::log10(energy(s.iter().flat_map(|f| f.iter().copied())) / 1920.0 + 1e-30)
        };
        let top = (0..280).map(|j| rms_db(f64::from(j) * 0.02)).fold(f64::MIN, f64::max);
        assert!(
            rms_db(2.0 + h.length + 0.3) < top - 60.0,
            "{} {top}",
            rms_db(2.0 + h.length + 0.3)
        );
        let silent = vec![[0.0; 2]; 48_000];
        assert_eq!(home(&silent, &h, 3), silent);
    }

    #[test]
    fn the_rooms_convolution_is_each_sides_own() {
        // two unlike sides, a stretch of silence a few blocks long, and blocks enough for both threads: each side
        // as the plain overlap-add convolution gives it, to rounding, for the first x.len() samples
        let k = 300;
        let ir = [noise(7, k), noise(8, k)];
        let (l, r) = (noise(1, 12_000), noise(2, 12_000));
        let x: Vec<[f64; 2]> = (0..12_000)
            .map(|i| {
                if (3_000..7_500).contains(&i) {
                    [0.0; 2]
                } else {
                    [l[i], 0.5 * r[i]]
                }
            })
            .collect();
        let y = convolve_room(&x, [&ir[0], &ir[1]]);
        assert_eq!(y.len(), x.len());
        for c in 0..2 {
            let side: Vec<f64> = x.iter().map(|f| f[c]).collect();
            let want = dsp::oaconvolve(&side, &ir[c]);
            let worst = (0..x.len()).map(|i| (y[i][c] - want[i]).abs()).fold(0.0, f64::max);
            assert!(worst < 1e-10, "side {c}: {worst:e} off");
        }
        assert!(convolve_room(&[], [&ir[0], &ir[1]]).is_empty());
    }
}
