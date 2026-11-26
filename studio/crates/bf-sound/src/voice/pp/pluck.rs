//! `piano_lowpoly.ks_pluck`: the Pixel Pitch Karplus-Strong string the ACOUSTIC's voice (`acoustic_lowpoly`) and
//! the ELECTRIC's (`electric60_lowpoly`, through its amp and damper) are plucked with. Their callers, the strings
//! they double and the amp belong to their own fighters' voices.
//!
//! It is plain arithmetic on the generator's uniforms, so it gives Python's bits exactly.
//!
//! Not ported here: `piano_lowpoly.additive`, which only the PIANO's voice plays (`inst_note('grand')`); it belongs
//! to the PIANO's port.

use bf_core::rng::Generator;

use super::{SRF, len};

/// `ks_pluck(f, dur, bright, decay, seed)`: a string at `f` Hz ringing for `dur` seconds, `int(dur * SR)` samples
/// at 44,100 Hz. The string is `N = int(SR / f)` samples long and starts as `N` uniforms in [-1, 1) of
/// `default_rng(seed)`, smoothed `int((1 - bright) * 4)` times by averaging each sample with the one before it
/// (round the loop); each sample after it is `decay * 0.5 * (s[i - N] + s[i - N - 1])`. A note shorter than the
/// string is the start of the string itself. (Python's defaults: `bright=0.5`, `decay=0.996`, `seed=0`.)
///
/// # Panics
/// Where Python raises: `dur` is not a number or gives a negative number of samples, `f` gives a string that is
/// not a number of samples. And for a string of no samples (`f` above 44,100 Hz), where Python never returns (its
/// loop does not move on).
pub fn ks_pluck(f: f64, dur: f64, bright: f64, decay: f64, seed: u64) -> Vec<f64> {
    let nf = SRF / f;
    assert!(
        nf.is_finite() && nf > -1.0,
        "ks_pluck: cannot make a string of {nf} samples"
    );
    let big_n = nf as usize;
    let n = len(dur);
    assert!(
        big_n > 0 || n == 0,
        "ks_pluck: a string of no samples at {f} Hz never fills the note"
    );
    let mut r = Generator::new(seed);
    let mut buf = vec![0.0; big_n];
    r.fill_uniform(-1.0, 1.0, &mut buf);
    // for _ in range(int((1 - bright) * 4)): buf = 0.5 * (buf + np.roll(buf, 1))
    let passes = (1.0 - bright) * 4.0;
    assert!(passes.is_finite(), "ks_pluck: cannot convert float {passes} to integer");
    for _ in 0..(passes.max(0.0) as usize) {
        let prev = buf.clone();
        for i in 0..big_n {
            let before = if i == 0 { prev[big_n - 1] } else { prev[i - 1] };
            buf[i] = 0.5 * (prev[i] + before);
        }
    }
    // out = zeros(n + N + 1); out[:N] = buf
    let mut out = vec![0.0; n + big_n + 1];
    out[..big_n].copy_from_slice(&buf);
    let k = decay * 0.5;
    let mut i = big_n;
    while i < n {
        let j = n.min(i + big_n);
        // prev = out[i-N:j-N]; prev1 = out[i-N-1:j-N-1], or 0 then out[i-N:j-N-1] while i - N - 1 < 0
        for m in i..j {
            let p = out[m - big_n];
            let p1 = if m - big_n >= 1 { out[m - big_n - 1] } else { 0.0 };
            out[m] = k * (p + p1);
        }
        i = j;
    }
    out.truncate(n);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_string_starts_as_its_noise_and_decays_by_averaging() {
        // 4,410 Hz is a string of 10 samples; bright 1 leaves the noise unsmoothed.
        let x = ks_pluck(4410.0, 0.01, 1.0, 0.99, 3);
        assert_eq!(x.len(), 441);
        let mut r = Generator::new(3);
        let mut noise = vec![0.0; 10];
        r.fill_uniform(-1.0, 1.0, &mut noise);
        assert_eq!(&x[..10], &noise[..]);
        // the first sample after the string averages its first sample with the zero before it
        assert_eq!(x[10].to_bits(), (0.99 * 0.5 * (noise[0] + 0.0)).to_bits());
        assert_eq!(x[11].to_bits(), (0.99 * 0.5 * (noise[1] + noise[0])).to_bits());
        assert_eq!(x[25].to_bits(), (0.99 * 0.5 * (x[15] + x[14])).to_bits());
    }

    #[test]
    fn smoothing_and_short_notes() {
        // bright 0.35 smooths twice (int(0.65 * 4) = 2), round the loop
        let x = ks_pluck(4410.0, 0.001, 0.35, 0.996, 0);
        assert_eq!(x.len(), 44);
        let mut r = Generator::new(0);
        let mut b = vec![0.0; 10];
        r.fill_uniform(-1.0, 1.0, &mut b);
        for _ in 0..2 {
            let p = b.clone();
            for i in 0..10 {
                b[i] = 0.5 * (p[i] + p[(i + 9) % 10]);
            }
        }
        assert_eq!(&x[..10], &b[..]);
        // a note shorter than its string is the start of the string
        let low = ks_pluck(82.4, 0.001, 0.5, 0.996, 1);
        assert_eq!(low.len(), 44);
        assert_eq!(&low[..], &ks_pluck(82.4, 0.5, 0.5, 0.996, 1)[..44]);
        assert!(ks_pluck(440.0, 0.0, 0.5, 0.996, 0).is_empty());
    }

    #[test]
    #[should_panic(expected = "never fills the note")]
    fn a_string_of_no_samples_is_refused() {
        ks_pluck(50_000.0, 0.1, 0.5, 0.996, 0);
    }
}
