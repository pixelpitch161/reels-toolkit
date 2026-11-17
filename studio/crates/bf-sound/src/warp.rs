//! The varispeed warp and the K.O. splice (Engine 2, milestone E5): music that slows with slow-mo.
//!
//! Before the K.O., an Engine 2 score is built on the fight clock and read back through the time map: output
//! sample `i` reads the fight-clock build at source position `src(i)`, so a 0.5x slow-mo plays the music at half
//! speed and an octave down, like tape. From the K.O. on, the score built on the video clock is used as it is;
//! [`splice`] joins the two with a short crossfade that ends on the K.O. sample.

use bf_core::dmath;

use crate::{Mix, SRF};

/// The Lanczos window's half-width: 16 taps.
pub const LANCZOS_A: usize = 8;

/// The crossfade at the K.O. seam, in samples (5 ms): it ends exactly on the seam, so the K.O. hit's attack is
/// untouched.
pub const SEAM_FADE: usize = 240;

/// A fractional position this close to a whole sample copies that sample: an identity map is bit-exact.
const EXACT: f64 = 1e-9;

const TAPS: usize = 2 * LANCZOS_A;

/// How to read one output sample: copy one source sample, or a weighted sum of 16 starting at `first`.
enum Read {
    Copy(i64),
    Taps { first: i64, w: [f64; TAPS] },
}

/// The Lanczos read at source position `p` (in samples). The weights are normalised by their sum, in a fixed
/// order, so the same position always gives the same bits.
fn read_at(p: f64) -> Read {
    let near = p.round();
    if (p - near).abs() <= EXACT {
        return Read::Copy(near as i64);
    }
    let base = p.floor();
    let first = base as i64 - (LANCZOS_A as i64 - 1);
    let a = LANCZOS_A as f64;
    let mut w = [0.0; TAPS];
    let mut sum = 0.0;
    for (k, wk) in w.iter_mut().enumerate() {
        let x = p - (first + k as i64) as f64;
        let px = dmath::PI * x;
        *wk = a * dmath::sin(px) * dmath::sin(px / a) / (px * px);
        sum += *wk;
    }
    for wk in &mut w {
        *wk /= sum;
    }
    Read::Taps { first, w }
}

fn sample(x: &[[f64; 2]], j: i64) -> [f64; 2] {
    usize::try_from(j)
        .ok()
        .and_then(|j| x.get(j))
        .copied()
        .unwrap_or([0.0; 2])
}

fn apply(x: &[[f64; 2]], r: &Read) -> [f64; 2] {
    match r {
        Read::Copy(j) => sample(x, *j),
        Read::Taps { first, w } => {
            let mut out = [0.0; 2];
            for (k, wk) in w.iter().enumerate() {
                let s = sample(x, first + k as i64);
                out[0] += wk * s[0];
                out[1] += wk * s[1];
            }
            out
        }
    }
}

/// The reads of output samples `0..n`, checking (in debug) that `src` never goes back.
fn reads(src: &dyn Fn(usize) -> f64, n: usize) -> impl Iterator<Item = Read> + '_ {
    let mut last = f64::NEG_INFINITY;
    (0..n).map(move |i| {
        let p = src(i);
        debug_assert!(
            p >= last,
            "warp: the source position went back at output sample {i} ({last} to {p})"
        );
        last = p;
        read_at(p)
    })
}

/// `x` read at source positions `src(0..n)` (in samples) with a 16-tap Lanczos (a = 8) windowed sinc. Taps
/// outside `x` read 0. A position within 1e-9 of a whole sample copies it, so `src(i) = i` gives `x` bit for bit.
/// The rates used are at most 1 (pure interpolation), so there is no anti-alias filter.
///
/// # Panics
/// In debug, when `src` decreases.
pub fn warp(x: &[[f64; 2]], src: &dyn Fn(usize) -> f64, n: usize) -> Vec<[f64; 2]> {
    reads(src, n).map(|r| apply(x, &r)).collect()
}

/// [`warp`] on every bus of `m`, in [`Mix::bus_names`] order. The result is a mix of `m`'s length whose buses
/// hold the warped samples `0..n` (cut to the bus length) and zeros after.
pub fn warp_mix(m: &Mix, src: &dyn Fn(usize) -> f64, n: usize) -> Mix {
    let names: Vec<&str> = m.bus_names();
    let buses: Vec<&[[f64; 2]]> = names.iter().map(|b| m.bus(b).expect("a listed bus")).collect();
    let mut out = Mix::new(m.dur());
    let len = buses.first().map_or(0, |b| b.len());
    let n = n.min(len);
    let mut warped: Vec<Vec<[f64; 2]>> = buses.iter().map(|_| Vec::with_capacity(n)).collect();
    // one kernel per output sample, read by every bus
    for r in reads(src, n) {
        for (w, x) in warped.iter_mut().zip(&buses) {
            w.push(apply(x, &r));
        }
    }
    for (name, w) in names.iter().zip(warped) {
        out.bus_mut(name)[..n].copy_from_slice(&w);
    }
    out
}

/// `pre` up to the K.O. seam and `post` from it, with a linear crossfade over the [`SEAM_FADE`] samples that end on
/// `seam`. The buses are the union, in `post`'s order and then `pre`'s new ones; a bus missing on one side is zeros
/// there. The result has `post`'s length.
pub fn splice(pre: &Mix, post: &Mix, seam: usize) -> Mix {
    let mut names: Vec<&str> = post.bus_names();
    for b in pre.bus_names() {
        if !names.contains(&b) {
            names.push(b);
        }
    }
    let fade0 = seam.saturating_sub(SEAM_FADE);
    let mut out = Mix::new(post.dur());
    for name in names {
        let (p, q) = (pre.bus(name), post.bus(name));
        let o = out.bus_mut(name);
        let at = |x: Option<&[[f64; 2]]>, i: usize| x.and_then(|x| x.get(i)).copied().unwrap_or([0.0; 2]);
        for (i, s) in o.iter_mut().enumerate() {
            *s = if i < fade0 {
                at(p, i)
            } else if i >= seam {
                at(q, i)
            } else {
                let w = (i - fade0) as f64 / (seam - fade0) as f64;
                let (a, b) = (at(p, i), at(q, i));
                [a[0] * (1.0 - w) + b[0] * w, a[1] * (1.0 - w) + b[1] * w]
            };
        }
    }
    out
}

/// The music clock as a source position: output sample `i` (video time `v = i / SR`) reads the fight-clock build at
/// `m(v)·SR`, where `m(v) = v` before FIGHT! and `t_fight + fight_t(v)` after (`fight_t` is the time map's fight
/// seconds from the fight start). The caller uses it only up to the K.O. seam.
pub fn music_src(t_fight: f64, fight_t: &dyn Fn(f64) -> f64) -> impl Fn(usize) -> f64 + '_ {
    move |i| {
        let v = i as f64 / SRF;
        let m = if v < t_fight { v } else { t_fight + fight_t(v) };
        m * SRF
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Mix, SR, SRF, Sig};
    use bf_core::dmath;

    fn sine(freq: f64, amp: f64, phase: f64, n: usize) -> Vec<[f64; 2]> {
        (0..n)
            .map(|i| {
                let v = amp * dmath::sin(dmath::TAU * freq * i as f64 / SRF + phase);
                [v, -0.5 * v]
            })
            .collect()
    }

    fn noise(n: usize, seed: u64) -> Vec<[f64; 2]> {
        let mut s = seed;
        let mut next = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            (s >> 11) as f64 / (1u64 << 53) as f64 - 0.5
        };
        (0..n).map(|_| [next(), next()]).collect()
    }

    fn bits(x: &[[f64; 2]]) -> Vec<u64> {
        x.iter().flatten().map(|v| v.to_bits()).collect()
    }

    /// Zero crossings of the left channel, upward, in `x[a..b]`, as a frequency.
    fn freq(x: &[[f64; 2]], a: usize, b: usize) -> f64 {
        let ups: Vec<usize> = (a + 1..b).filter(|&i| x[i - 1][0] < 0.0 && x[i][0] >= 0.0).collect();
        let (first, last) = (ups[0], *ups.last().unwrap());
        (ups.len() - 1) as f64 * SRF / (last - first) as f64
    }

    fn max_step(x: &[[f64; 2]], a: usize, b: usize) -> f64 {
        (a + 1..b)
            .flat_map(|i| [(x[i][0] - x[i - 1][0]).abs(), (x[i][1] - x[i - 1][1]).abs()])
            .fold(0.0, f64::max)
    }

    #[test]
    fn identity_warp_is_bit_exact() {
        let x = noise(20_000, 7);
        let y = warp(&x, &|i| i as f64, x.len());
        assert_eq!(bits(&y), bits(&x));
        // the music clock with no slow-mo is the identity too, to within the exact-copy tolerance
        let tf = 0.1;
        let ft = |v: f64| v - tf;
        let src = music_src(tf, &ft);
        let y = warp(&x, &src, x.len());
        assert_eq!(bits(&y), bits(&x));
    }

    #[test]
    fn half_speed_halves_pitch() {
        let x = sine(1000.0, 0.5, 0.0, SR);
        let y = warp(&x, &|i| 0.5 * i as f64, 2 * SR - 64);
        let f = freq(&y, 4_800, 2 * SR - 4_800);
        assert!((f - 500.0).abs() <= 2.0, "{f} Hz");
        // and the level holds: a sine read between its samples is still a sine of the same size
        let peak = y[4_800..2 * SR - 4_800].iter().fold(0.0f64, |m, s| m.max(s[0].abs()));
        assert!((peak - 0.5).abs() < 0.01, "peak {peak}");
    }

    #[test]
    fn warp_is_deterministic() {
        let x = noise(9_000, 3);
        let src = |i: usize| 0.37 * i as f64 + 11.25;
        assert_eq!(bits(&warp(&x, &src, 20_000)), bits(&warp(&x, &src, 20_000)));
    }

    #[test]
    fn taps_outside_read_zero() {
        let x = vec![[1.0, 1.0]; 100];
        let y = warp(&x, &|i| 200.0 + 0.5 * i as f64, 10);
        assert!(y.iter().all(|s| *s == [0.0, 0.0]));
    }

    fn mix_of(dur: f64, buses: &[(&str, Vec<[f64; 2]>)]) -> Mix {
        let mut m = Mix::new(dur);
        for (name, x) in buses {
            m.place(name, &Sig::Stereo(x.clone()), 0.0, 1.0, 0.0);
        }
        m
    }

    #[test]
    fn warp_mix_warps_each_bus() {
        let a = noise(10_000, 1);
        let b = noise(10_000, 2);
        let m = mix_of(0.5, &[("zeta", a.clone()), ("alpha", b.clone())]);
        let src = |i: usize| 0.75 * i as f64;
        let w = warp_mix(&m, &src, 12_000);
        assert_eq!(w.bus_names(), vec!["zeta", "alpha"]);
        assert_eq!(w.ns(), m.ns());
        let want = warp(m.bus("zeta").unwrap(), &src, 12_000);
        assert_eq!(bits(&w.bus("zeta").unwrap()[..12_000]), bits(&want));
        assert!(w.bus("alpha").unwrap()[12_000..].iter().all(|s| *s == [0.0, 0.0]));
    }

    #[test]
    fn splice_takes_each_side() {
        let n = SR;
        let pre = mix_of(1.0, &[("music", sine(440.0, 0.5, 0.0, n))]);
        let post = mix_of(1.0, &[("music", sine(300.0, 0.4, 1.0, n))]);
        let seam = 24_000;
        let s = splice(&pre, &post, seam);
        let (p, q, o) = (
            pre.bus("music").unwrap(),
            post.bus("music").unwrap(),
            s.bus("music").unwrap(),
        );
        assert_eq!(o.len(), q.len());
        assert_eq!(bits(&o[..seam - SEAM_FADE]), bits(&p[..seam - SEAM_FADE]));
        assert_eq!(bits(&o[seam..]), bits(&q[seam..]));
        // in the fade, between the two
        let mid = seam - SEAM_FADE / 2;
        for c in 0..2 {
            let (lo, hi) = (p[mid][c].min(q[mid][c]), p[mid][c].max(q[mid][c]));
            assert!(o[mid][c] >= lo - 1e-12 && o[mid][c] <= hi + 1e-12);
        }
    }

    #[test]
    fn splice_has_no_click() {
        let n = SR;
        let pre = mix_of(1.0, &[("music", sine(440.0, 0.5, 0.0, n))]);
        let post = mix_of(1.0, &[("music", sine(297.0, 0.5, 2.1, n))]);
        for seam in [20_000, 24_011, 30_123] {
            let o = splice(&pre, &post, seam);
            let o = o.bus("music").unwrap();
            let ms2 = SR / 500;
            let near = max_step(o, seam - ms2, seam + ms2);
            let before = max_step(o, seam - ms2 - SR / 20, seam - ms2);
            assert!(near <= 1.5 * before, "seam {seam}: {near} vs {before}");
        }
    }

    #[test]
    fn splice_unions_buses() {
        let pre = mix_of(0.5, &[("a", noise(1000, 1)), ("pre_only", noise(1000, 2))]);
        let post = mix_of(0.6, &[("post_only", noise(1000, 3)), ("a", noise(1000, 4))]);
        let s = splice(&pre, &post, 600);
        assert_eq!(s.bus_names(), vec!["post_only", "a", "pre_only"]);
        assert_eq!(s.ns(), post.ns());
        assert_eq!(s.bus("a").unwrap().len(), post.bus("a").unwrap().len());
        // a bus missing on one side is zeros there
        assert!(
            s.bus("post_only").unwrap()[..600 - SEAM_FADE]
                .iter()
                .all(|x| *x == [0.0, 0.0])
        );
        assert!(s.bus("pre_only").unwrap()[600..].iter().all(|x| *x == [0.0, 0.0]));
        assert_eq!(
            bits(&s.bus("pre_only").unwrap()[..300]),
            bits(&pre.bus("pre_only").unwrap()[..300])
        );
    }

    #[test]
    fn music_src_follows_the_fight_clock_after_fight() {
        let tf = 1.0;
        // half speed from 2 s of video on
        let ft = |v: f64| if v < 2.0 { v - tf } else { 1.0 + 0.5 * (v - 2.0) };
        let src = music_src(tf, &ft);
        assert_eq!(src(SR / 2), 0.5 * SRF);
        assert!((src(3 * SR) - 2.5 * SRF).abs() < 1e-6);
    }
}
